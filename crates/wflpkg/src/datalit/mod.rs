//! The frozen data-literal grammar — `wflpkg-data-literal-grammar 1.0.0`.
//!
//! A wflpkg manifest (`project.wfl`) and lockfile (`project.lock`) are
//! **supply-chain metadata**: a *value, never a program*. It is *deserialized*,
//! never *executed*. This module implements the small, restricted, **frozen**
//! data-literal dialect of WFL specified in `wflpkg-manifest-grammar-1.0.md`.
//!
//! ## The design law (grammar §2)
//!
//! * **Subtractive over the shared lexer.** There is one lexer — the `logos`
//!   tokenizer in [`wfl_core::lexer::token`] — used byte-identically by the
//!   compiler and by this parser (condition 5). This module is an *acceptance
//!   predicate layered on top of that token stream*: it may only ever **reject**
//!   token sequences the lexer produces; it never requires a new token. Hence
//!   `L(manifest) ⊂ L(WFL)`.
//! * **The acceptance layer reads `(token, source-span)`, not just tokens.**
//!   The lexer erases three security-relevant distinctions before the token
//!   stream exists — boolean case/spelling, null spelling, and integer leading
//!   zeros. So every rejection that depends on surface spelling reads the *raw
//!   span bytes*, not the token payload.
//! * **Reject, don't repair (and never panic, never hang).** On any ambiguity,
//!   duplicate key, non-canonical spelling, disallowed byte, overlong encoding,
//!   or limit breach, [`parse`] returns a typed [`GrammarError`] with a byte
//!   offset. Silent repair is itself a differential.
//!
//! ## The three gates (grammar §6)
//!
//! Ingest is ordered gates; a byte fails at the first that catches it.
//!
//! 1. **Gate B** ([`bytegate`]) — byte-level, before the lexer: UTF-8
//!    well-formedness, no BOM, NFC, size, LF-only line endings.
//! 2. **Gate L + Gate S** ([`parser`]) — a single pass over the shared lexer's
//!    `(token, span)` stream enforcing every lexical/span rejection and the
//!    record/entry/value structure.
//! 3. **Gate I** ([`identity`]) — post-parse, on the identity-bearing fields
//!    `scope`, `name`, `version`.
//!
//! The canonical byte form, JSON projection, and the two SHA-256 hashes live in
//! [`fmt`], [`json`], and [`hash`].

pub mod bytegate;
pub mod error;
pub mod fmt;
pub mod hash;
pub mod identity;
pub mod json;
pub mod limits;
pub mod parser;
pub mod version;

#[cfg(test)]
mod tests;

pub use error::{Code, GrammarError, GrammarResult};

/// The SemVer of the grammar this implementation conforms to. Versioned
/// **independently of the WFL language** (grammar §9).
pub const GRAMMAR_VERSION: &str = "1.0.0";

/// A parsed manifest/lockfile document: an ordered sequence of record blocks.
///
/// Record order is preserved from the source. Two documents that differ only in
/// record/entry order are distinct byte forms (distinct `file_hash`) but the
/// same package spec (same `content_hash`, since the JSON projection sorts
/// keys) — see [`hash`].
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub records: Vec<Record>,
}

/// A `create map <kind>: … end map` block. The block name is read as the
/// record's *kind* tag (grammar §3); repetition of a kind is permitted (three
/// `dependency` blocks are three records).
///
/// Equality is **semantic**: two records are equal when their kind and entries
/// match. The `offset` is source provenance for error reporting and is
/// deliberately excluded, so `parse(fmt(x)) == parse(x)` holds across the
/// whitespace changes `fmt` makes (grammar §10.4).
#[derive(Debug, Clone)]
pub struct Record {
    /// The record-kind tag: a non-reserved lowercase-ASCII identifier.
    pub kind: String,
    /// Ordered entries. Keys are unique within a record (`MG-S02`).
    pub entries: Vec<Entry>,
    /// Byte offset of the `create` keyword that opened this record.
    pub offset: usize,
}

impl PartialEq for Record {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.entries == other.entries
    }
}

/// A `KEY is VALUE` binding inside a record. Equality ignores `offset` (see
/// [`Record`]).
#[derive(Debug, Clone)]
pub struct Entry {
    pub key: String,
    pub value: Value,
    /// Byte offset of the key.
    pub offset: usize,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.value == other.value
    }
}

/// A manifest value: exactly one of the four admitted kinds (grammar N4).
/// There is no null (absence = omit the key) and no float.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    /// Unsigned; `0 ..= 2^53−1`.
    Integer(i64),
    Boolean(bool),
    List(Vec<Scalar>),
}

/// A list element: lists hold scalars only, never lists or records (grammar N8).
#[derive(Debug, Clone, PartialEq)]
pub enum Scalar {
    String(String),
    Integer(i64),
    Boolean(bool),
}

impl Record {
    /// The first entry with the given key, if any. Keys are unique per record,
    /// so "first" is "the" entry.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.iter().find(|e| e.key == key).map(|e| &e.value)
    }

    /// The string value for a key, if the key exists and is a string.
    pub fn get_string(&self, key: &str) -> Option<&str> {
        match self.get(key) {
            Some(Value::String(s)) => Some(s.as_str()),
            _ => None,
        }
    }
}

impl Document {
    /// Parse and validate a document through all four gates. This is the single
    /// entry point; it is a pure function of the input bytes (grammar §10.7).
    pub fn parse(bytes: &[u8]) -> GrammarResult<Document> {
        parse(bytes)
    }

    /// All records of a given kind, in document order.
    pub fn records_of<'a>(&'a self, kind: &str) -> impl Iterator<Item = &'a Record> {
        self.records.iter().filter(move |r| r.kind == kind)
    }

    /// The single record of a given kind, or `None` if there are zero or more
    /// than one.
    pub fn single(&self, kind: &str) -> Option<&Record> {
        let mut it = self.records_of(kind);
        let first = it.next()?;
        if it.next().is_some() {
            return None;
        }
        Some(first)
    }
}

/// Stack size for the manifest-parse worker thread. The shared `logos` lexer
/// recurses roughly once per byte of a single token (a pre-existing lexer
/// characteristic — a long token in *any* WFL source stresses it). Combined
/// with [`limits::MAX_DOCUMENT_BYTES`] (which bounds the longest possible
/// token), this generous fixed stack guarantees the parse never overflows,
/// regardless of the caller's own stack size.
const PARSE_STACK_BYTES: usize = 64 * 1024 * 1024;

/// Parse and validate a document through the full gate pipeline (grammar §6).
///
/// Gate B runs on the raw bytes; Gates L+S run in a single pass over the shared
/// lexer's token stream; Gate I runs on identity fields. Returns a validated
/// [`Document`] or the first [`GrammarError`], fail-closed.
///
/// The pipeline runs on a dedicated large-stack thread so that a manifest — even
/// a maximally sized one — parses in bounded steps without any risk of a
/// stack-overflow abort (grammar §10.3: "termination in bounded steps, never a
/// hang"). This is defense-in-depth on top of the Gate-B size limit.
pub fn parse(bytes: &[u8]) -> GrammarResult<Document> {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("wflpkg-manifest-parse".to_string())
            .stack_size(PARSE_STACK_BYTES)
            .spawn_scoped(scope, || parse_pipeline(bytes))
            .expect("failed to spawn manifest parse thread")
            .join()
            .unwrap_or_else(|_| {
                // A panic on the worker (should be impossible for a bounded,
                // size-limited document) is reported, never propagated.
                Err(GrammarError::new(
                    error::Code::MgB06,
                    0,
                    "The manifest parser exceeded its resource limits.",
                ))
            })
    })
}

/// The gate pipeline proper. Kept separate so [`parse`] can host it on a
/// large-stack thread.
fn parse_pipeline(bytes: &[u8]) -> GrammarResult<Document> {
    // Gate B — byte-level checks, returns a validated &str.
    let text = bytegate::check(bytes)?;

    // Gate L + Gate S — one pass over the shared lexer's (token, span) stream.
    let document = parser::parse_tokens(text)?;

    // Gate I — identity/semantic checks on scope/name/version fields.
    identity::check(&document)?;

    Ok(document)
}
