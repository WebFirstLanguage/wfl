//! Gate B — byte-level checks that run **before** the shared lexer (grammar §6).
//!
//! These operate on raw bytes and constrain the accepted domain to already-NFC,
//! BOM-free, LF-only, well-formed UTF-8 within the size limit. Constraining the
//! domain this way is what lets us hash raw bytes *and* guarantee NFC without a
//! transform to diverge on: every consumer runs the identical boolean
//! `NFC(bytes) == bytes` (grammar §6, NFC ruling).

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::is_nfc;

use super::error::{Code, GrammarError, GrammarResult};
use super::limits::MAX_DOCUMENT_BYTES;

/// Run Gate B and return the validated text. On success the caller is
/// guaranteed: within size limit, well-formed UTF-8, no BOM, no CR byte, NFC.
pub fn check(bytes: &[u8]) -> GrammarResult<&str> {
    // MG-B06 — size limit, checked first to bound all downstream work.
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(GrammarError::new(
            Code::MgB06,
            MAX_DOCUMENT_BYTES,
            format!(
                "This file is {} bytes, larger than the {}-byte manifest limit.",
                bytes.len(),
                MAX_DOCUMENT_BYTES
            ),
        ));
    }

    // MG-B01 / MG-B02 / MG-B03 — well-formed UTF-8. Rust's decoder already
    // rejects overlong encodings, surrogates, and code points above U+10FFFF;
    // we classify *which* so the error code is precise.
    let text = match std::str::from_utf8(bytes) {
        Ok(t) => t,
        Err(e) => {
            let pos = e.valid_up_to();
            let code = classify_bad_utf8(bytes, pos);
            let what = match code {
                Code::MgB02 => "an overlong UTF-8 encoding",
                Code::MgB03 => "a surrogate or out-of-range code point",
                _ => "invalid UTF-8",
            };
            return Err(GrammarError::new(
                code,
                pos,
                format!("This file contains {what} at byte {pos}; manifests must be UTF-8."),
            ));
        }
    };

    // MG-B04 — Byte-Order Mark anywhere (including offset 0). A BOM is an
    // invisible byte and a Trojan-Source vector.
    if let Some(rel) = text.find('\u{FEFF}') {
        return Err(GrammarError::new(
            Code::MgB04,
            rel,
            "This file contains a Byte-Order Mark (U+FEFF). Remove it; manifests are plain UTF-8.",
        ));
    }

    // MG-B07 — LF is the sole line ending. Any CR byte (bare CR or the CR of a
    // CRLF) is a canonicalization differential and is rejected outright.
    if let Some(rel) = text.find('\r') {
        return Err(GrammarError::new(
            Code::MgB07,
            rel,
            "This file uses a CR line ending. Manifests must use LF (\\n) only.",
        ));
    }

    // MG-B05 — content must already be in NFC. We reject, we never normalize:
    // silent normalization is itself a differential.
    if !is_nfc(text) {
        let offset = first_non_nfc_offset(text);
        return Err(GrammarError::new(
            Code::MgB05,
            offset,
            "This file is not in Unicode NFC form. Re-save it normalized (NFC); \
             the manifest parser rejects, rather than silently repairs, non-NFC text.",
        ));
    }

    Ok(text)
}

/// Classify an invalid-UTF-8 position into the precise Gate-B code. `pos` is the
/// first invalid byte (from [`std::str::Utf8Error::valid_up_to`]).
fn classify_bad_utf8(bytes: &[u8], pos: usize) -> Code {
    let b = bytes[pos];
    let b1 = bytes.get(pos + 1).copied();

    // C0/C1 lead bytes only ever encode overlong 2-byte forms of ASCII.
    if b == 0xC0 || b == 0xC1 {
        return Code::MgB02;
    }
    // E0 followed by 0x80..=0x9F is an overlong 3-byte form.
    if b == 0xE0 && matches!(b1, Some(0x80..=0x9F)) {
        return Code::MgB02;
    }
    // F0 followed by 0x80..=0x8F is an overlong 4-byte form.
    if b == 0xF0 && matches!(b1, Some(0x80..=0x8F)) {
        return Code::MgB02;
    }
    // ED followed by 0xA0..=0xBF encodes a UTF-16 surrogate.
    if b == 0xED && matches!(b1, Some(0xA0..=0xBF)) {
        return Code::MgB03;
    }
    // F4 followed by 0x90..=0xBF, or any F5..=FF lead, is above U+10FFFF.
    if b == 0xF4 && matches!(b1, Some(0x90..=0xBF)) {
        return Code::MgB03;
    }
    if (0xF5..=0xFF).contains(&b) {
        return Code::MgB03;
    }
    Code::MgB01
}

/// Byte offset of the first character that would move under NFC. Used only to
/// point the error at a useful place; the accept/reject decision is [`is_nfc`].
fn first_non_nfc_offset(text: &str) -> usize {
    let normalized: String = text.nfc().collect();
    text.bytes()
        .zip(normalized.bytes())
        .position(|(a, b)| a != b)
        .unwrap_or(0)
}
