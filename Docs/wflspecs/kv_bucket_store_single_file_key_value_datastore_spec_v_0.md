# KV Bucket Store — Single‑File Key/Value Datastore Specification (v0.1)

*A compact, crash‑resilient, append‑optimized key/value store that packs data, index, and metadata into one binary **bucket** file.*

This spec uses RFC 2119 keywords (**MUST**, **SHOULD**, **MAY**).

---

## 1. Scope & Design Goals

**Primary goals**
- Single binary file (the **bucket**) contains data, index, and metadata.
- Fast point lookups by key.
- Append‑only writes with background compaction.
- Crash safety with bounded recovery time.
- Cross‑platform (POSIX + Windows), little‑endian on disk.

**Secondary goals**
- Optional compression and at‑rest authenticated encryption.
- TTL/expiry and deletes via tombstones.
- Prefix/range scans (lexicographic by key bytes).
- Single‑writer, multi‑reader concurrency.

**Non‑goals (v0.1)**
- Distributed replication.
- Multi‑writer concurrency to the same file.
- Transactional multi‑key atomicity (beyond batched durability).

---

## 2. Terminology
- **Bucket**: the datastore file (`.kvb`).
- **Page**: fixed‑size block; default 4096 bytes (configurable at init, power of two). All on‑disk offsets are **page‑relative** + **in‑page offset**.
- **Segment**: contiguous range of pages into which log records are appended.
- **Record**: a single mutation (PUT/DELETE) of a key.
- **Index page**: hash table entries that map a key fingerprint to the latest record location.
- **FSM**: free‑space map tracking free extents.
- **WAL/Journal**: tiny intent log for index & checkpoint pointers.

---

## 3. Consistency & Durability Model
- **Write visibility**: A successful `put/delete` is visible to the writer immediately; readers see it after the **commit pointer** is advanced.
- **Durability**: Two modes:
  - **Sync** (default): fsync data record, then index journal, then commit pointer. Crash yields either old or new value, never torn.
  - **Relaxed**: rely on OS flush; crash may lose most recent ops.
- **Atomicity**: single key update is atomic.
- **Ordering**: writer issues a monotonically increasing 64‑bit `seqno` per record.

---

## 4. On‑Disk Layout (High‑Level)
```
+----------------------+  Page 0
| Superblock A         |
+----------------------+  Page 1..N1
| Free Space Map       |
+----------------------+
| Index Region         |  (pool of index pages; extent list in superblock)
+----------------------+
| Journal (small ring) |
+----------------------+
| Segment Region(s)    |  append‑only data log (one or more active segments)
+----------------------+
| Checkout / Trailer   |  Superblock B (redundant), recent checkpoints
+----------------------+
```

**Redundancy**: Two superblocks (A at start, B near end). The newer one is found by `epoch` with valid checksum.

---

## 5. Primitive Types (On Disk)
- **u16/u32/u64**: little‑endian.
- **varint**: unsigned LEB128.
- **paddr**: 64‑bit physical byte offset from file start.
- **pageid**: 64‑bit page number.
- **hash64**: 64‑bit key hash (XXH3‑64 default; configurable).

**Alignment**: Records start at arbitrary byte offsets; segment headers are page‑aligned; structures **SHOULD** align to 8 bytes for speed.

---

## 6. Superblock
```
struct Superblock {
  u32   magic;          // "KVBK" (0x4B56424B)
  u16   version_major;  // 1
  u16   version_minor;  // 0
  u32   page_size;      // e.g., 4096 (power of two)
  u64   file_uuid_hi;   // random at init
  u64   file_uuid_lo;
  u64   created_unix_ns;
  u64   flags;          // bitfield: compression, encryption, etc.
  u64   index_region_start_page;
  u64   index_region_page_count;
  u64   fsm_start_page;
  u64   fsm_page_count;
  u64   journal_start_page;
  u64   journal_page_count;   // small ring (e.g., 8–64 pages)
  u64   segment_start_page;   // first data segment page
  u64   segment_end_page;     // current high‑water mark (exclusive)
  u64   commit_seqno;         // highest durable seqno
  u64   commit_paddr;         // durable end‑of‑log byte offset
  u64   epoch;                // incremented per checkpoint
  u32   crc32c;               // over all prior fields
  // pad to page_size
}
```
Two copies exist: **Superblock A** (page 0) and **Superblock B** (trailer). The valid one has the greater `epoch` and a valid `crc32c`.

---

## 7. Journal (Index/Checkpoint Intent Log)
- Ring buffer of fixed‑size entries with `crc32c`.
- Used to persist **index mutations** and **commit pointer** advances safely.
- On recovery, replay entries newer than the durable `superblock.commit_seqno`.

```
struct JournalEntry {
  u8    kind;          // 1=IndexInsert, 2=IndexUpdate, 3=IndexDelete, 4=Commit
  u8    reserved[7];
  u64   seqno;         // mutation sequence
  u64   hash64;        // key hash
  u64   index_pageid;  // where the slot lives (for idempotent replay)
  u64   slot_idx;      // slot within page (Robin Hood position)
  u64   record_paddr;  // location of record start (or 0 for tombstone)
  u32   crc32c;        // of the entry up to here
}
```

**Crash rule**: Data record is written & fsynced **before** its corresponding journal entry; commit entries advance visibility.

---

## 8. Index Region (Hash Table Pages)
- Open‑addressed hash table with **Robin Hood** probing for stable lookup cost.
- Fixed‑size slots; power‑of‑two slots per page.
- Resizing accomplished by allocating new index pages and flipping a pointer table (v0.1 supports a single pool; v0.2 MAY adopt extendible hashing directory).

**Index Page**
```
struct IndexPageHeader {
  u64 page_uuid;      // random; assists torn‑write detection
  u32 slots;          // number of slots (power of two)
  u32 used;           // occupied slots
  u64 pageid;         // self id
  u32 crc32c;         // header only
}

struct IndexSlot {    // 16 bytes
  u64 hash64;         // 0 == empty; 1 == tombstone sentinel
  u64 record_paddr;   // points to latest committed record
}
```

**Lookup**: start at `hash64 & (slots-1)` and probe forward until match (`slot.hash64 == hash64`) or empty sentinel (`0`). On collision, Robin Hood invariants apply.

**Integrity**: Readers validate `record_paddr` by checking the record header’s `hash64` and CRC.

---

## 9. Segment Region (Append‑Only Log)
- One or more **segments**, each page‑aligned.
- Writer appends records to the current **active segment**; when full or after compaction, a new segment is allocated.

**Segment Header** (page‑aligned)
```
struct SegmentHeader {
  u64  seg_id;         // monotonic
  u64  base_seqno;     // first record seqno in this segment
  u64  start_unix_ns;
  u64  flags;          // compression/encryption hints
  u32  writer_pid;     // optional
  u32  reserved;
  u64  segment_size;   // bytes including header + payload + footer (filled at seal)
  u32  header_crc32c;  // over fields above
  // payload starts immediately after
}
```

**Record** (variable length)
```
struct RecordHeader {
  u32  header_crc32c;  // over fields below up to key bytes
  u64  seqno;          // global monocounter
  u64  hash64;         // of key bytes (XXH3‑64)
  u64  timestamp_ns;   // wall clock when written
  u8   type;           // 0=PUT, 1=DELETE
  u8   enc;            // 0=raw, 1=zstd, 2=… | +128 means AEAD encrypted
  u16  reserved16;
  u32  key_len;        // bytes
  u32  val_len;        // bytes (0 for DELETE)
  u32  ttl_secs;       // 0 = no expiry
  // key bytes [key_len]
  // value bytes [val_len] (optionally compressed/encrypted)
  u32  record_crc32c;  // over RecordHeader..value bytes
}
```

**Segment Footer** (written when sealing the segment; assists fast scans/compaction)
```
struct SegmentFooter {
  u64  seg_id;
  u64  last_seqno;     // last seqno sealed in this segment
  u64  live_bytes;     // best‑effort at seal time
  u32  sparse_index_n; // number of (hash64 -> paddr) entries
  // repeated: { u64 hash64; u64 paddr; }
  u32  footer_crc32c;
}
```

**Sparse Mini‑Index**: Every Nth record (e.g., 64) emits a `(hash64, paddr)` pair recorded in the footer; used during recovery/compaction to avoid full scans.

---

## 10. Free Space Map (FSM)
- Bitmap of pages plus an **extent list** for large free runs.
- Writer allocates from FSM for new index pages and segments.
- Compaction returns reclaimed segments to the FSM.

**FSM Consistency**: Alloc/free operations are journaled; on recovery, recompute from journal replay + segment seals.

---

## 11. Compression & Encryption (Optional)
- **Compression**: Zstandard (*enc=1*). `val_len` is the post‑compression size; original length is stored as a varint prefix in value payload (first varint = `orig_len`).
- **Encryption**: XChaCha20‑Poly1305 AEAD; per‑segment 192‑bit nonce = `H(file_uuid || seg_id)`. AAD covers `seg_id`, `seqno`, `hash64`. Key management is out of scope; bucket stores key id in superblock flags/metadata.

---

## 12. Operations

### 12.1 PUT(key, value, ttl=0)
1. Compute `hash64 = H(key)`.
2. Append `RecordHeader + key + value + CRC` to active segment; fsync (Sync mode) or defer (Relaxed).
3. Upsert index slot on in‑memory view; emit `JournalEntry(IndexInsert|Update)` and fsync journal.
4. Optionally emit `JournalEntry(Commit)` to advance `commit_paddr/seqno` (batched per N records or M ms).

**Idempotence**: Replaying an identical PUT with same `seqno` is ignored (seqno monotonicity).

### 12.2 GET(key)
1. Compute `hash64` and probe index pages.
2. Read `record_paddr`; validate `RecordHeader.hash64`, CRCs, TTL.
3. If expired or tombstoned, treat as not found.

### 12.3 DELETE(key)
- Append tombstone record (`type=DELETE`, `val_len=0`), then index update (`record_paddr=0` in journal causes index slot to become tombstone sentinel `1`).

### 12.4 SCAN(prefix)
- Full scan reads segments in seq order; skip non‑matching keys. For production use, enable an optional **ordered key table** (v0.2) or maintain a prefix Bloom filter per segment to prune.

### 12.5 BATCH(ops)
- Writer may coalesce multiple PUT/DELETE operations before fsync; emits a single `Commit` journal entry.

---

## 13. Compaction & GC
- **Trigger**: segment live ratio < threshold (e.g., 60%) or file size > soft quota.
- **Process**:
  1. Allocate a new segment.
  2. For each (hash64 → paddr) in victim segments, copy **latest** live records into the new segment.
  3. Update index slots to new `paddr` and journal these updates.
  4. Seal victim segments; return their pages to FSM.

- **Expiry**: Keys with `ttl_secs > 0` and `timestamp + ttl < now` are omitted during compaction.

Recovery is bounded by: sealed segments + sparse footer indices + last active segment scan (amortized by mini‑index density).

---

## 14. Concurrency
- **Single writer** per bucket enforced via advisory file lock.
- **Multiple readers** open same file concurrently; readers use `commit_paddr` to bound visible log.
- **Memory‑mapping**: Readers **MAY** mmap segments for faster GET; writer uses normal buffered I/O to avoid page‑cache coherency pitfalls.

---

## 15. Error Handling & Integrity
- Every superblock, journal entry, index header, and record carries a `crc32c`.
- Readers validate on fetch; bad CRC causes a soft error and optional fallback to scan.
- **Torn write handling**: journal and records are written length‑prefixed (implicit via struct sizes) and verified by CRC; partial tails are discarded at mount.

---

## 16. API Surface (v0.1)
```
put(key: bytes, value: bytes, ttl_secs: int = 0) -> None
get(key: bytes) -> Optional[bytes]
delete(key: bytes) -> bool
exists(key: bytes) -> bool
scan(prefix: bytes, limit: int = 1000) -> Iterator[(key, value)]
flush() -> None            // force checkpoint/commit
compact(kind: str = "auto" | "full") -> Stats
stats() -> Stats
repair() -> Report         // offline, if corruption detected
```

**Return/Exception semantics** are implementation‑defined but **MUST** never expose torn/partial records to callers.

---

## 17. CLI (Reference)
```
kvb init bucket.kvb --page-size 4096 --index-mb 64 --compression zstd
kvb put bucket.kvb key.bin value.bin --ttl 3600
kvb get bucket.kvb key.bin > out
kvb del bucket.kvb key.bin
kvb scan bucket.kvb --prefix 0xDEADBEEF --limit 100
kvb compact bucket.kvb --full
kvb stats bucket.kvb
kvb repair bucket.kvb --dry-run
```

---

## 18. Limits & Tuning Knobs
- Max file size: 2^63‑1 bytes (paddr is u64; practical limits are OS/filesystem).
- Page size: 4 KiB default; 8–64 KiB recommended for large values.
- Index load factor: target 0.80; triggers growth when exceeded.
- Journal ring: ≥ 8 pages; larger rings reduce fsync churn under heavy write bursts.
- Sparse mini‑index stride: every 64–256 records (tradeoff: recovery vs write amp).

---

## 19. Observability
- Built‑in counters: `puts`, `gets`, `deletes`, `hit_rate`, `bytes_written`, `bytes_live`, `segments_active`, `segments_garbage_ratio`, `compaction_seconds`.
- Optional tracepoints: `journal_fsync`, `segment_seal`, `rehash_start/stop`.

---

## 20. Recovery Algorithm (Mount)
1. Read both superblocks; choose the one with highest `epoch` and valid CRC.
2. Rebuild in‑memory index pointers by scanning the **journal** forward from `commit_seqno` and applying idempotently.
3. Validate the active segment tail via CRC; discard partial tail.
4. If index pages are inconsistent (CRC mismatch), rebuild from sealed segments using **sparse mini‑indices** then scan only the last unsealed tail.
5. Write a new checkpoint (advance `epoch`), fsync superblock B, then A.

Bounded by journal size + density of mini‑indices; typical mount time is proportional to the last few hundred MB, not whole file.

---

## 21. Security Notes
- AEAD covers key bytes and values; metadata (e.g., index positions, key lengths) are not hidden in v0.1.
- Consider per‑bucket key rotation by rewriting live records into a new encrypted segment and dropping old ones.

---

## 22. Compatibility & Versioning
- `version_major` changes denote incompatible on‑disk changes.
- Minor versions **MAY** add optional fields guarded by flags; readers **MUST** ignore unknown flags.

---

## 23. Gotchas (Field Notes)
- **Hot keys**: Robin Hood probing evens out cost, but keep load factor ≤ 0.85.
- **Huge values**: Prefer larger page/segment sizes to reduce fragmentation.
- **Power loss**: Keep journal fsync after record fsync. Never advance `commit_paddr` before both land.
- **Clock skew**: TTL uses wall time; a bad clock can resurface expired keys. Consider monotonic stamps + grace.
- **Memory use**: In‑memory index mirrors on‑disk slots for speed; with 16‑byte slots at 0.8 LF, budget ~20 bytes per key.

---

## 24. Future Work (v0.2+)
- Extendible hashing directory for online index growth without large rehash.
- Prefix Bloom filters per segment to accelerate `scan(prefix)`.
- Multi‑writer with page‑level intent locks.
- Secondary indexes and range trees (e.g., B+ sidecar) for ordered scans.
- Checksumming upgrade: BLAKE3 option for long‑tail corruption detection.

---

## Appendix A — Byte Layout Examples

**Index Slot (16 bytes)**
```
Offset Size Field
0x00   8    hash64
0x08   8    record_paddr
```

**Record (PUT) — header fields only (before key/value)**
```
Offset Size Field
0x00   4    header_crc32c
0x04   8    seqno
0x0C   8    hash64
0x14   8    timestamp_ns
0x1C   1    type (0)
0x1D   1    enc
0x1E   2    reserved16
0x20   4    key_len
0x24   4    val_len
0x28   4    ttl_secs
0x2C   ...  key bytes
...    ...  value bytes (maybe compressed/encrypted)
...    4    record_crc32c
```

**Superblock — magic and CRC coverage**
- `crc32c` is computed over the struct excluding the `crc32c` field itself and any page padding.

---

## Appendix B — Fsync Order (Sync Mode)
1. Append record bytes → `fdatasync(data)`
2. Append index journal entry → `fdatasync(journal)`
3. Update `commit_paddr/seqno` (journal `Commit`) → `fdatasync(journal)`
4. Periodic checkpoint: write & fsync **Superblock B**, then **A**

---

## Appendix C — Reference Heuristics
- **Compaction picking**: score = `(1 - live_ratio)*size_bytes + age_weight*segment_age`.
- **Mini‑index stride**: choose `max(64, 1 << log2(avg_records_per_segment)/8)`.
- **Journal batching**: commit every 4 ms or 256 ops, whichever first.

---

*End of v0.1.*

