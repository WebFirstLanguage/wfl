# Dev Diary — 2026-07-10: Native image resize + TTL cache primitive

## Context (13.3 Stretch, WFL runtime)

Two runtime stretch goals landed together:

1. **A native image-resize builtin**, binding the Rust `image` crate. The point
   is to *erase the media boundary*: before this, any WFL program that wanted to
   make a thumbnail had to shell out to an external tool (ImageMagick, `sips`,
   etc.), which drags a process boundary — and its failure modes, escaping, and
   platform assumptions — into an otherwise self-contained program. Image
   handling now lives on the WFL side of that boundary.
2. **A simple TTL cache primitive.** The page-cache pattern (cache-or-compute a
   rendered response) already works with plain objects, but doing it *idiomatically*
   — with real time-to-live expiry — deserved a first-class primitive.

## Media module (`src/stdlib/media.rs`)

Images travel through WFL as `Value::Binary`, which is exactly what
`read binary content` already produces and what a binary web response already
consumes — so the new functions slot into the existing binary I/O story with no
new value type.

- `image_dimensions of bytes -> { width, height }` reads only the header
  (`ImageReader::into_dimensions`), so it's cheap even for large images.
- `resize_image of bytes and width and height -> bytes` decodes (with resource
  limits), scales to **fit inside** the box while **preserving aspect ratio**
  (Lanczos3), and re-encodes to the **same format** it read. Format is
  auto-detected — never passed in.

Design decisions worth recording:

- **Fit-inside, aspect-preserving** was chosen as the single default rather than
  exact-scaling. It never distorts, which is the beginner-safe behaviour (No-
  Unlearning invariant): the naive call does the sensible thing, and a program
  that genuinely needs exact dimensions can read `image_dimensions` and compute
  the box. One function, no distortion footgun.
- **Preserve source format.** Silently changing a `.jpg` into a `.png` would be a
  nasty surprise for anyone writing the bytes back to disk or serving them, so
  the encoder targets the detected format and errors clearly if that format has
  no encoder. JPEG is the one special case: it has no alpha channel, so an RGBA
  image is flattened to RGB before encoding.
- **Security / DoS bounds.** Decoding applies `image::Limits` (max width/height
  30k, max alloc 512 MiB) to defuse decompression bombs, and resize targets are
  capped (each axis ≤ 30k, ≤ 100 megapixels total) so a tiny request can't ask
  for a giant allocation.
- Enabled `image` codec features are scoped to the common web formats — `png`,
  `jpeg`, `gif`, `bmp`, `webp` — with default features off to keep the
  dependency footprint down.

## Cache module (`src/stdlib/cache.rs`)

`create_cache`, `cache_set` (with per-entry TTL in seconds), `cache_get`,
`cache_has`, `cache_delete`, `cache_clear`, `cache_size`.

- A cache is an opaque numeric **handle**. Native functions in WFL are plain `fn`
  pointers and can't capture state, so the caches live in a `thread_local`
  registry. That's sound here because the interpreter runs single-threaded — its
  `Value`s hold `Rc` (`!Send`), so the whole evaluation stays on one thread. (A
  `static` was a non-starter: `Value` is `!Sync`, so it can't live in one.)
- **Lazy expiry.** An expired entry is removed on the next `cache_get` / `cache_has`
  lookup, and `cache_size` sweeps the whole cache. No background thread, no
  surprise pauses. Calling `cache_size` periodically keeps memory bounded even
  for keys that are never read again.
- **Monotonic time.** Expiry deadlines use `Instant`, so entries expire on
  schedule regardless of wall-clock (NTP) adjustments.
- **TTL of 0 means "never expires."** This gives a single primitive that covers
  both transient and permanent entries without a second function.

## Testing (TDD)

- Rust unit tests: 8 in `media.rs` (aspect ratio, format preservation, RGBA→JPEG
  flattening, no-upscale, non-integer / zero / non-image / non-binary rejection)
  and 12 in `cache.rs` (set/get, miss, ttl=0 permanence, real timed expiry +
  sweep, has/delete/clear/size, overwrite, cache independence, bad-handle errors).
- WFL end-to-end suites: `TestPrograms/cache_ttl.test.wfl` and
  `TestPrograms/image_resize.test.wfl` (the latter resizes the repo's 128×128
  `icons/wfl.png` and round-trips the thumbnail through the filesystem).

## Docs

New `Docs/05-standard-library/media-module.md` and `cache-module.md`, linked from
the standard-library `index.md` and `overview.md`.

## Wiring notes

Builtins are registered in three places, all updated: the interpreter environment
(`stdlib/mod.rs` via each module's `register_*`), the analyzer's builtin
signatures (`stdlib/typechecker.rs`), and the type checker's return-type map
(`typechecker/mod.rs`). Added a reusable `expect_binary` extractor to
`stdlib/helpers.rs` alongside the existing `expect_*` family.
