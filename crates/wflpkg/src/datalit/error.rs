//! Typed, byte-offset-carrying errors for the frozen data-literal grammar.
//!
//! Every code below is a **stable part of the versioned spec** (grammar §6):
//! the error codes are an API. Identical input MUST yield the identical code in
//! every conforming implementation, so the string form of each code
//! (`"MG-B01"`, …) never changes within a grammar major version.
//!
//! The three ingest gates are ordered — a byte fails at the *first* gate that
//! catches it (Gate B → Gate L → Gate S), then Gate I runs post-parse on
//! identity fields. See the module docs for the gate pipeline.

use std::fmt;

/// A stable rejection code. The discriminant name mirrors the spec code:
/// `MgB01` prints as `"MG-B01"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Code {
    // ---- Gate B: byte-level, before the shared lexer ----
    /// Bytes are not well-formed UTF-8.
    MgB01,
    /// Overlong UTF-8 encoding.
    MgB02,
    /// Surrogate-range or > U+10FFFF byte sequence.
    MgB03,
    /// Byte-Order Mark U+FEFF anywhere.
    MgB04,
    /// Content is not in NFC — reject, do not normalize.
    MgB05,
    /// Document size exceeds the limit.
    MgB06,
    /// A line ending other than LF (CR or CRLF) used as structure.
    MgB07,

    // ---- Gate L: lexical / span ----
    /// A comment (`//` or `#`) is present.
    MgL01,
    /// Inter-token whitespace contains form-feed or a stray C0 control.
    MgL02,
    /// A string contains a raw C0/DEL/C1 control character.
    MgL03,
    /// A string contains a raw newline.
    MgL04,
    /// A string contains a bidirectional control character (Trojan Source).
    MgL05,
    /// A string contains a zero-width / invisible character.
    MgL06,
    /// Boolean bytes are not exactly `yes` or `no`.
    MgL07,
    /// A string escape other than `\n \t \\ \"`.
    MgL08,
    /// A null literal (`nothing` / `missing` / `undefined`) in value position.
    MgL09,
    /// A float literal, or any signed/hex/exponent numeric form.
    MgL10,
    /// An integer with a leading zero, or exceeding 2^53−1 / overflowing i64.
    MgL11,
    /// A string decoded length or key length over the limit.
    MgL12,

    // ---- Gate S: structural ----
    /// A value node outside {String, Integer, Boolean, List}.
    MgS01,
    /// A duplicate key within one record.
    MgS02,
    /// A list separator other than comma, a trailing comma, or a nested list.
    MgS03,
    /// A top-level node that is not a `create map … end map` record block.
    MgS04,
    /// An unterminated or malformed block, or a reserved keyword used as a bare
    /// record name or bare entry key.
    MgS05,
    /// A `create map` value the full expression grammar accepts but this
    /// grammar does not (belt-and-suspenders subset assertion).
    MgS06,
    /// A resource limit exceeded (entries, list length, record count).
    MgS07,

    // ---- Gate I: identity / semantic ----
    /// A `scope`/`name` field violates the identity allowlist.
    MgI01,
    /// A `version` field fails the exact-version or constraint grammar.
    MgI02,
    /// An identity field fails the UTS #39 tripwire (mixed-script /
    /// restriction-level) under the pinned Unicode data.
    MgI03,
}

impl Code {
    /// The stable wire form of the code, e.g. `"MG-B01"`. This string is part
    /// of the grammar's public contract and never changes within a major.
    pub const fn as_str(self) -> &'static str {
        match self {
            Code::MgB01 => "MG-B01",
            Code::MgB02 => "MG-B02",
            Code::MgB03 => "MG-B03",
            Code::MgB04 => "MG-B04",
            Code::MgB05 => "MG-B05",
            Code::MgB06 => "MG-B06",
            Code::MgB07 => "MG-B07",
            Code::MgL01 => "MG-L01",
            Code::MgL02 => "MG-L02",
            Code::MgL03 => "MG-L03",
            Code::MgL04 => "MG-L04",
            Code::MgL05 => "MG-L05",
            Code::MgL06 => "MG-L06",
            Code::MgL07 => "MG-L07",
            Code::MgL08 => "MG-L08",
            Code::MgL09 => "MG-L09",
            Code::MgL10 => "MG-L10",
            Code::MgL11 => "MG-L11",
            Code::MgL12 => "MG-L12",
            Code::MgS01 => "MG-S01",
            Code::MgS02 => "MG-S02",
            Code::MgS03 => "MG-S03",
            Code::MgS04 => "MG-S04",
            Code::MgS05 => "MG-S05",
            Code::MgS06 => "MG-S06",
            Code::MgS07 => "MG-S07",
            Code::MgI01 => "MG-I01",
            Code::MgI02 => "MG-I02",
            Code::MgI03 => "MG-I03",
        }
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A rejection: a stable code, the byte offset where it was caught, and a
/// human-readable, Elm-flavoured explanation. Rejections never repair, never
/// panic, never hang (grammar §2.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarError {
    pub code: Code,
    /// Byte offset into the original document where the problem was caught.
    pub offset: usize,
    /// A first-person, actionable message. The code + offset are the machine
    /// contract; this is for the human reading the terminal.
    pub message: String,
}

impl GrammarError {
    pub fn new(code: Code, offset: usize, message: impl Into<String>) -> Self {
        Self {
            code,
            offset,
            message: message.into(),
        }
    }
}

impl fmt::Display for GrammarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (at byte {}): {}",
            self.code, self.offset, self.message
        )
    }
}

impl std::error::Error for GrammarError {}

/// Convenience alias for grammar results.
pub type GrammarResult<T> = Result<T, GrammarError>;
