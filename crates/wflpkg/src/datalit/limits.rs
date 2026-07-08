//! Normative resource limits for the frozen data-literal grammar (§7).
//!
//! Every limit here is a fixed part of `wflpkg-data-literal-grammar 1.0.0`.
//! They exist to make parsing terminate in bounded steps on adversarial input
//! (grammar spec §10.3) and to keep manifests reviewable by humans and agents.
//! Raising any of these is a grammar-version decision, not a silent change.

/// Maximum size of a whole manifest/lockfile document, in bytes (`MG-B06`).
/// 256 KiB is enormous for supply-chain metadata; anything larger is either a
/// mistake or an exhaustion attempt. This bound (together with the large parse
/// stack in [`super::parse`]) guarantees the §10.3 "termination in bounded
/// steps" property even though the shared `logos` lexer recurses per byte of a
/// single token.
pub const MAX_DOCUMENT_BYTES: usize = 256 * 1024;

/// Maximum decoded length of a single string value, in bytes (`MG-L12`).
pub const MAX_STRING_BYTES: usize = 64 * 1024;

/// Maximum length of a record key, in bytes (`MG-L12`).
pub const MAX_KEY_BYTES: usize = 256;

/// Maximum number of elements in a list value (`MG-S03`/`MG-S07`, N8).
pub const MAX_LIST_ELEMENTS: usize = 4096;

/// Maximum number of entries in one record (`MG-S07`).
pub const MAX_ENTRIES_PER_RECORD: usize = 4096;

/// Maximum number of records in a document (`MG-S07`).
pub const MAX_RECORDS: usize = 4096;

/// Largest admissible integer value, `2^53 − 1` (`MG-L11`, N6).
/// This is the I-JSON safe-integer bound: lossless for any JSON consumer.
pub const MAX_INT: i64 = 9_007_199_254_740_991; // 2^53 - 1

/// Longest identity field (`scope`, `name`), in bytes (§8, `MG-I01`).
pub const MAX_IDENTITY_LEN: usize = 64;
