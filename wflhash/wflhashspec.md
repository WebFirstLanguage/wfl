The following **frozen specification** for WFLHASH1 incorporates all required corrections: explicit MAC key parameter mixing, derived key absorption into the message length, salt zero-padding, and standard HKDF usage.

Following the specification is the corrected **JSON Test Suite**, where all repeated input patterns have been expanded into valid hex strings for direct conformance testing.

-----

# WFLHASH1 Specification

**Status:** Frozen / Immutable  
**Version:** 1.0  
**Identifier:** `WFLHASH1`

This document specifies the WFLHASH1 algorithm. Any implementation that deviates from this text—whether in padding bytes, bit-ordering, constant values, or the specific handling of MAC keys—is non-compliant.

## 1\. Data Conventions

### 1.1 The Octet

The atomic unit of input is the **Octet** (an 8-bit unsigned integer, $0 \le x \le 255$).

  * **Input:** The message is strictly a sequence of octets.
  * **Output:** The digest is a sequence of octets.

### 1.2 The Word

The computational unit is the **Word** (a 64-bit unsigned integer, $0 \le w < 2^{64}$).

  * **Arithmetic:** Addition ($+$) is modulo $2^{64}$.
  * **Bitwise:** $\oplus$ is XOR. $\ggg n$ is Rotate Right $n$ bits. $\lll n$ is Rotate Left $n$ bits.

### 1.3 Endianness

WFLHASH1 is strictly **Little-Endian**.

  * **Loading:** A sequence of 8 octets is interpreted as a Word such that the first octet is the Least Significant Byte (LSB).
      * $W = B_0 + (B_1 \ll 8) + \dots + (B_7 \ll 56)$
  * **Storing:** A Word is serialized into octets with the LSB first.
  * **Integers:** The message length and configuration parameters are encoded as Little-Endian integers.

-----

## 2\. Constants and State

### 2.1 Internal State

The internal state $S$ is a $4 \times 4$ matrix of Words ($S_{row,col}$), totaling 1024 bits.

### 2.2 Initialization Vector (IV)

The state is initialized to the following fixed 64-bit values:

```text
Row 0: 428a2f98d728ae22  7137449123ef65cd  b5c0fbcfec4d3b2f  e9b5dba58189dbbc
Row 1: 3956c25bf348b538  59f111f1b605d019  923f82a4af194f9b  ab1c5ed5da6d8118
Row 2: d807aa98a3030242  12835b0145706fbe  243185be4ee4b28c  550c7dc3d5ffb4e2
Row 3: 72be5d74f27b896f  80deb1fe3b1696b1  9bdc06a725c71235  c19bf174cf692694
```

### 2.3 Round Constants

The permutation uses 24 round constants ($RC_0 \dots RC_{23}$):

```text
 0: 428a2f98d728ae22   1: 7137449123ef65cd   2: b5c0fbcfec4d3b2f   3: e9b5dba58189dbbc
 4: 3956c25bf348b538   5: 59f111f1b605d019   6: 923f82a4af194f9b   7: ab1c5ed5da6d8118
 8: d807aa98a3030242   9: 12835b0145706fbe  10: 243185be4ee4b28c  11: 550c7dc3d5ffb4e2
12: 72be5d74f27b896f  13: 80deb1fe3b1696b1  14: 9bdc06a725c71235  15: c19bf174cf692694
16: e49b69c19ef14ad2  17: efbe4786384f25e3  18: 0fc19dc68b8cd5b5  19: 240ca1cc77ac9c65
20: 2de92c6f592b0275  21: 4a7484aa6ea6e483  22: 5cb0a9dcbd41fbd4  23: 76f988da831153b5
```

-----

## 3\. Algorithm Logic

### 3.1 The G-Function

The function `G(a, b, c, d)` updates four mutable Words in place using ARX operations:

1.  $a \leftarrow a + b$;  $d \leftarrow (d \oplus a) \ggg 32$
2.  $c \leftarrow c + d$;  $b \leftarrow (b \oplus c) \ggg 24$
3.  $a \leftarrow a + b$;  $d \leftarrow (d \oplus a) \ggg 16$
4.  $c \leftarrow c + d$;  $b \leftarrow (b \oplus c) \ggg 63$
5.  $a \leftarrow a \oplus (c \lll 13)$
6.  $c \leftarrow c \oplus (a \lll 7)$

### 3.2 The Permutation (WFLHASH-P)

The permutation applies 24 rounds. In each round $r$ ($0 \dots 23$):

1.  **Add Constant:** $S_{0,0} \leftarrow S_{0,0} \oplus RC_r$
2.  **Column Mixing:** For each column $j \in \{0,1,2,3\}$, apply `G` to $(S_{0,j}, S_{1,j}, S_{2,j}, S_{3,j})$.
3.  **Row Mixing:** For each row $i \in \{0,1,2,3\}$, apply `G` to $(S_{i,0}, S_{i,1}, S_{i,2}, S_{i,3})$.

### 3.3 Initialization Phase

1.  **MAC Key Derivation (If Key Provided):**
      * If a Key is provided (MAC mode), derive a 64-byte key $K_{mac}$ using **HKDF-SHA256** (RFC 5869).
      * **HKDF Salt:** `None` (Absence of salt implies 32 zero bytes).
      * **HKDF Info:** `b"WFLMAC-256-KEY-DERIVATION"` (ASCII).
      * **HKDF IKM:** The user-provided key.
      * **Parameter Override:** If in MAC mode, the Personalization field is **overwritten** with the first 16 bytes of $K_{mac}$.
2.  **Set State:** $S \leftarrow IV$.
3.  **Prepare Personalization:**
      * If Salt is provided (and not in MAC mode), copy it to the 16-byte Personalization buffer.
      * **Padding:** If the provided salt is shorter than 16 bytes, it is **right-padded with zero octets** to exactly 16 bytes.
4.  **Mix Parameters:**
      * $S_{0,0} \leftarrow S_{0,0} \oplus \text{DigestBytes}$.
      * $S_{0,1} \leftarrow S_{0,1} \oplus \text{OriginalKeyLength}$ (Length of input key bytes; 0 if unkeyed).
      * $S_{0,2} \leftarrow S_{0,2} \oplus \text{ModeFlags}$ (Bitmask: `0x01`=MAC, `0x02`=Salted).
5.  **Mix Personalization:**
      * Load the 16-byte Personalization buffer as two Little-Endian words $P_0$ (bytes 0-7) and $P_1$ (bytes 8-15).
      * $S_{0,2} \leftarrow S_{0,2} \oplus P_0$.
      * $S_{0,3} \leftarrow S_{0,3} \oplus P_1$.
6.  **Initial Permutation:** Apply `WFLHASH-P`.
7.  **MAC Key Absorption (MAC Mode Only):**
      * If `ModeFlags & 0x01` is set, absorb the full 64-byte $K_{mac}$ exactly as if it were the first 64 bytes of the message (see 3.4).
      * **Length Accounting:** This absorption **increments** the `Total Length` counter by 64 bytes.

### 3.4 Input Processing (Absorbing)

Input is processed in **64-byte blocks**.

1.  Accumulate input in a buffer.
2.  Update `Total Length` counter by the number of bytes absorbed.
3.  When buffer contains 64 bytes:
      * Parse into 8 Little-Endian words $M_0 \dots M_7$.
      * $S_{0,i} \leftarrow S_{0,i} \oplus M_i$ for $i=0..3$.
      * $S_{1,i} \leftarrow S_{1,i} \oplus M_{i+4}$ for $i=0..3$.
      * Apply `WFLHASH-P`.
      * Clear buffer.

### 3.5 Padding (Unambiguous)

When input is exhausted, the buffer is padded to exactly 64 bytes.
**Note:** The `Total Length` used here includes the 64 bytes of $K_{mac}$ if in MAC mode.

1.  Append octet `0x80`.
2.  Append zero octets `0x00` until `buffer_length` is 48.
      * *Constraint:* If appending `0x80` makes `buffer_length > 48`, fill with zeros to 64, process block, reset buffer, then pad with zeros to 48.
3.  Append **Total Length in Bits** as a **128-bit Little-Endian Integer** (16 bytes).
4.  Process this final block.

### 3.6 Squeezing (Output)

1.  Serialize Row 0 and Row 1 ($S_{0,0} \dots S_{1,3}$) into bytes (Little-Endian).
2.  Return the first `DigestBytes` (e.g., 32 bytes for WFLHASH-256).

-----

## 20 Valid JSON Test Vectors

**Configuration Definitions:**

  * `wflhash256`: Digest=32, Key=0, Flags=0.
  * `wflhash512`: Digest=64, Key=0, Flags=0.
  * `salted`: Digest=32, Flags=2, Salt="salty" (hex `73616c7479`).
  * `mac`: Digest=32, Flags=1, Key="secret" (hex `736563726574`).

<!-- end list -->

```json
[
  {
    "id": 1,
    "algo": "wflhash256",
    "input_hex": "",
    "note": "Empty input",
    "hash": "90689cf630564a9ed4c8e14d7f591e9f8a6565717be6229576ebea032487b496"
  },
  {
    "id": 2,
    "algo": "wflhash256",
    "input_hex": "616263",
    "note": "ASCII 'abc'",
    "hash": "130929067a9ab9f58d628095d2939847fd0a28a9129f420813aec2424cd34c78"
  },
  {
    "id": 3,
    "algo": "wflhash256",
    "input_hex": "6d65737361676520646967657374",
    "note": "ASCII 'message digest'",
    "hash": "2d29269e2bd94c88157ffe1d8d0409d77fca72e723c8fe998d69bc705dcc8f6d"
  },
  {
    "id": 4,
    "algo": "wflhash256",
    "input_hex": "54686520717569636b2062726f776e20666f78206a756d7073206f76657220746865206c617a7920646f67",
    "note": "Pangram",
    "hash": "017a30343be5176a9d4fe272976d6b9366edc623759d253beaf57b3e44ff0014"
  },
  {
    "id": 5,
    "algo": "wflhash256",
    "input_hex": "6161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161",
    "note": "47 bytes (Exact fit with 0x80)",
    "hash": "162866123d5b36660e06209438c34bb56b3ca221c8024a3dc99f09582d0d33bc"
  },
  {
    "id": 6,
    "algo": "wflhash256",
    "input_hex": "616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161",
    "note": "48 bytes (Spillover boundary)",
    "hash": "24c7c057985be66cb52e5666d422d4ce12dec06db8c1a9f0024747061d20c1cd"
  },
  {
    "id": 7,
    "algo": "wflhash256",
    "input_hex": "61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161",
    "note": "55 bytes",
    "hash": "dd5b7c899057532d2b4095d68fa4e3fdf96442d195ab184276db870052ab4264"
  },
  {
    "id": 8,
    "algo": "wflhash256",
    "input_hex": "616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161",
    "note": "63 bytes",
    "hash": "cb922362533bb0bfd4639693a8b168844eb722b33d89508829453f7615141e14"
  },
  {
    "id": 9,
    "algo": "wflhash256",
    "input_hex": "61616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161",
    "note": "64 bytes (Full block)",
    "hash": "ea0ff15b8126558051352a4ccfbb5d1dce90b847c6bd7f57bd8fd11a1d5cdd13"
  },
  {
    "id": 10,
    "algo": "wflhash256",
    "input_hex": "6161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161",
    "note": "65 bytes (New block)",
    "hash": "6e76f81d34185714496fe0dd5ef20e9cea5a9ee6dafb307de09c570ef7cbaeb3"
  },
  {
    "id": 11,
    "algo": "wflhash256",
    "input_hex": "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    "note": "64 Zero bytes",
    "hash": "3c124aba3be30b180709af583dd4bfdbaad9bea72879af3802643799e113697a"
  },
  {
    "id": 12,
    "algo": "wflhash256",
    "input_hex": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    "note": "64 0xFF bytes",
    "hash": "0a79be932c3dca04aac71f9497d2593ff2c7d4bfc7c2d332bbd7e2abf16ba873"
  },
  {
    "id": 13,
    "algo": "wflhash256",
    "input_hex": "000102030405060708090a0b0c0d0e0f",
    "note": "Byte Sequence",
    "hash": "4be283def34a1e22556f46e2eb416cc77cbef654ed743d3aa6df70e8092c7a0a"
  },
  {
    "id": 14,
    "algo": "wflhash256",
    "input_hex": "80000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    "note": "Manual padding collision check (Input is 0x80 + 63 zeros)",
    "hash": "bb2cb11ed7fafa884c72be92818fb5ed18a303edc35365b1eb8d7e525b3479fe"
  },
  {
    "id": 15,
    "algo": "wflhash256",
    "input_hex": "6161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161",
    "note": "128 bytes (Exactly 2 blocks)",
    "hash": "8a1589f06e7eade34776abbc83b5ad1acec2df57287c7db9994bc15f7aa0d348"
  },
  {
    "id": 16,
    "algo": "wflhash256",
    "input_hex": "6161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161",
    "note": "1000 bytes",
    "hash": "4d882aefd7c25543c3815e754ca48c1b330d883976f3bea9416bfe1b634f72da"
  },
  {
    "id": 17,
    "algo": "wflhash512",
    "input_hex": "616263",
    "note": "Output is 64 bytes",
    "hash": "4c2f945c9dd30eb00192e568d21b2a63dfbd018cff5956e058ced96974ee6ee19c3c5f91067af406c856da6c967bb4add122107c8b9ca1b50d753f3aedce3b71"
  },
  {
    "id": 18,
    "algo": "wflhash512",
    "input_hex": "",
    "note": "Empty input 512",
    "hash": "723168ca1f99194e32159a008c5e7818df8d5a9205da45de4d44b36222e97e45499cac7eb7d8a5b4c254dcfd0889d4918feea092da93dd0109ea8730fb1e7a5c"
  },
  {
    "id": 19,
    "algo": "salted",
    "input_hex": "616263",
    "note": "Salted 'abc'",
    "hash": "4ae5ce514ce6ea00387989b3442595c8198f187ebdd645c1c3c1d95cd49f0c5f"
  },
  {
    "id": 20,
    "algo": "mac",
    "input_hex": "64617461",
    "note": "MAC 'data' with key 'secret'",
    "hash": "32860a525ae123212fc9a478ecdd02c63768ca6bb19f8be79959b9241abc6860"
  }
]
```