//! The two SHA-256 hashes (grammar §7.3).
//!
//! These are **different layers, not competitors**:
//!
//! | Hash | Over | Answers |
//! |------|------|---------|
//! | [`file_hash`] | the canonical on-disk bytes (`fmt` output) | "did these exact bytes arrive intact / is this what was signed on disk?" |
//! | [`content_hash`] | the JCS projection of the parsed value | "are these two manifests the same package spec regardless of formatting?" |
//!
//! Both use SHA-256 — the cross-ecosystem interop hash (crates.io, OCI, Git's
//! SHA-256 transition, TUF, Sigstore/Rekor, SPDX/CycloneDX). Digests are stored
//! **algorithm-tagged** (`sha256:…`) so the ecosystem is rotation-ready.
//! **WFLHASH is deliberately excluded** from this integrity/identity path — an
//! unaudited custom primitive on a cross-trust-boundary channel has no external
//! interop and no payoff here.

use sha2::{Digest, Sha256};

use super::{Document, fmt, json};

/// The algorithm tag prefixed to every digest this module emits.
pub const ALGORITHM: &str = "sha256";

/// SHA-256 of arbitrary bytes as lowercase hex (no algorithm tag).
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// An algorithm-tagged digest, e.g. `sha256:9f86d0…`.
fn tagged(bytes: &[u8]) -> String {
    format!("{ALGORITHM}:{}", sha256_hex(bytes))
}

/// `file_hash`: SHA-256 over the canonical on-disk bytes (the `fmt` output).
/// This is the byte-integrity anchor; signatures cover these bytes and
/// verification does **no** canonicalization step.
pub fn file_hash(doc: &Document) -> String {
    let canonical = fmt::to_canonical(doc);
    tagged(canonical.as_bytes())
}

/// `file_hash` computed directly over already-canonical bytes (e.g. the exact
/// file that was read/downloaded), without re-serializing.
pub fn file_hash_of_bytes(canonical_bytes: &[u8]) -> String {
    tagged(canonical_bytes)
}

/// `content_hash`: SHA-256 over the JCS projection of the parsed value. Two
/// manifests with identical structure but different formatting share this hash.
pub fn content_hash(doc: &Document) -> String {
    let jcs = json::to_jcs(doc);
    tagged(jcs.as_bytes())
}
