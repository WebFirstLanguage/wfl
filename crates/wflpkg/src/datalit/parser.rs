//! Gate L + Gate S — the restricted manifest parser (grammar §6).
//!
//! A single pass over the **shared lexer's** `(token, span)` stream. The tokens
//! come from `wfl_core::lexer::token::Token` — the exact `logos` tokenizer the
//! compiler uses (condition 5) — so this layer is purely *subtractive*: it
//! rejects token sequences, it never asks for a token the lexer does not emit.
//!
//! Because the lexer erases surface spelling (boolean case, integer leading
//! zeros) before the token payload exists, every spelling-sensitive rejection
//! reads the **raw span bytes** via [`Lexed::slice`], not the token value
//! (grammar §2.2).

use logos::Logos;
use wfl_core::lexer::token::Token;

use super::error::{Code, GrammarError, GrammarResult};
use super::limits::{
    MAX_ENTRIES_PER_RECORD, MAX_INT, MAX_KEY_BYTES, MAX_LIST_ELEMENTS, MAX_RECORDS,
    MAX_STRING_BYTES,
};
use super::{Document, Entry, Record, Scalar, Value};

/// One raw lexer token with its byte span. `token` is `None` for a lexical
/// error (e.g. an integer that overflows `i64`); the span still points at the
/// offending bytes so we can classify it.
struct Lexed {
    token: Option<Token>,
    start: usize,
    end: usize,
}

impl Lexed {
    fn slice<'a>(&self, text: &'a str) -> &'a str {
        &text[self.start..self.end]
    }
    fn is(&self, t: &Token) -> bool {
        self.token.as_ref() == Some(t)
    }
}

/// Entry point: tokenize with the shared lexer, check inter-token gaps
/// (comments / form-feeds), then parse the record/entry/value structure.
pub fn parse_tokens(text: &str) -> GrammarResult<Document> {
    let toks = lex_and_check_gaps(text)?;
    let mut p = Parser { text, toks, pos: 0 };
    p.parse_document()
}

/// Tokenize and enforce Gate-L gap coverage (`MG-L01`, `MG-L02`). Every span of
/// bytes the lexer *skipped* between tokens must be horizontal whitespace only;
/// a skipped comment or form-feed turns up here as a non-`[ \t]` gap.
fn lex_and_check_gaps(text: &str) -> GrammarResult<Vec<Lexed>> {
    let mut lexer = Token::lexer(text);
    let mut toks = Vec::new();
    let mut prev_end = 0usize;
    while let Some(res) = lexer.next() {
        let span = lexer.span();
        check_gap(text, prev_end, span.start)?;
        toks.push(Lexed {
            token: res.ok(),
            start: span.start,
            end: span.end,
        });
        prev_end = span.end;
    }
    // Trailing gap (after the last token to end-of-file).
    check_gap(text, prev_end, text.len())?;
    Ok(toks)
}

/// Every byte in an inter-token gap must be a space or tab. A form-feed is
/// `MG-L02`; the start of a `//`/`#` comment is `MG-L01`. (A `//` or `#` inside
/// a string is inside the string's span, not a gap, so URLs and `#tags` in
/// string values are unaffected.)
fn check_gap(text: &str, from: usize, to: usize) -> GrammarResult<()> {
    let bytes = text.as_bytes();
    let mut i = from;
    while i < to {
        match bytes[i] {
            b' ' | b'\t' => {}
            0x0C => {
                return Err(GrammarError::new(
                    Code::MgL02,
                    i,
                    "A form-feed character appears between tokens. Use only spaces and newlines.",
                ));
            }
            b'/' | b'#' => {
                return Err(GrammarError::new(
                    Code::MgL01,
                    i,
                    "Comments are not allowed in a manifest. Put human notes in a `notes` field \
                     (it is hashed and reviewed); `//` and `#` are not.",
                ));
            }
            _ => {
                // Only spaces, tabs, form-feeds and comments are ever skipped by
                // the lexer, so this is unreachable in practice; treat any other
                // skipped byte as a stray control (fail closed).
                return Err(GrammarError::new(
                    Code::MgL02,
                    i,
                    "An unexpected character appears between tokens.",
                ));
            }
        }
        i += 1;
    }
    Ok(())
}

struct Parser<'a> {
    text: &'a str,
    toks: Vec<Lexed>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Lexed> {
        self.toks.get(self.pos)
    }

    fn eof_offset(&self) -> usize {
        self.text.len()
    }

    /// Byte offset of the current token, or end-of-file.
    fn here(&self) -> usize {
        self.peek()
            .map(|t| t.start)
            .unwrap_or_else(|| self.eof_offset())
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Some(l) if l.is(&Token::Newline)) {
            self.pos += 1;
        }
    }

    fn parse_document(&mut self) -> GrammarResult<Document> {
        let mut records = Vec::new();
        self.skip_newlines();
        while self.pos < self.toks.len() {
            let rec = self.parse_record()?;
            records.push(rec);
            if records.len() > MAX_RECORDS {
                return Err(GrammarError::new(
                    Code::MgS07,
                    self.here(),
                    format!("This document has more than {MAX_RECORDS} records."),
                ));
            }
            self.skip_newlines();
        }
        if records.is_empty() {
            return Err(GrammarError::new(
                Code::MgS04,
                0,
                "A manifest must contain at least one `create map … end map` record.",
            ));
        }
        Ok(Document { records })
    }

    fn parse_record(&mut self) -> GrammarResult<Record> {
        // MG-S04 — the only admitted top-level node is `create map`.
        let create = self.peek().ok_or_else(|| {
            GrammarError::new(
                Code::MgS04,
                self.eof_offset(),
                "Expected a `create map` record.",
            )
        })?;
        if !create.is(&Token::KeywordCreate) {
            return Err(GrammarError::new(
                Code::MgS04,
                create.start,
                "Only `create map … end map` records may appear at the top level of a manifest.",
            ));
        }
        let offset = create.start;
        self.pos += 1;

        self.expect(&Token::KeywordMap, "Expected `map` after `create`.")?;

        // Record kind: exactly one non-reserved lowercase-ASCII identifier.
        let kind = self.parse_name()?;

        self.expect(&Token::Colon, "Expected `:` after the record name.")?;
        self.expect(
            &Token::Newline,
            "Expected a line break after `:`; each entry goes on its own line.",
        )?;

        let entries = self.parse_entries()?;
        Ok(Record {
            kind,
            entries,
            offset,
        })
    }

    /// A record-kind name (`MG-S05`): a single lowercase-ASCII identifier, not a
    /// reserved keyword.
    fn parse_name(&mut self) -> GrammarResult<String> {
        let l = self.peek().ok_or_else(|| {
            GrammarError::new(Code::MgS05, self.eof_offset(), "Expected a record name.")
        })?;
        match &l.token {
            Some(Token::Identifier(_)) => {
                let raw = l.slice(self.text);
                let start = l.start;
                if !is_lower_ident(raw) {
                    return Err(GrammarError::new(
                        Code::MgS05,
                        start,
                        format!(
                            "Record name `{raw}` must be lowercase letters, digits and \
                             underscores, starting with a letter."
                        ),
                    ));
                }
                self.pos += 1;
                Ok(raw.to_string())
            }
            Some(tok) if tok.is_keyword() => Err(GrammarError::new(
                Code::MgS05,
                l.start,
                format!(
                    "`{}` is a reserved WFL word and cannot be a bare record name.",
                    l.slice(self.text)
                ),
            )),
            _ => Err(GrammarError::new(
                Code::MgS05,
                l.start,
                "Expected a record name (a lowercase identifier).",
            )),
        }
    }

    fn parse_entries(&mut self) -> GrammarResult<Vec<Entry>> {
        let mut entries: Vec<Entry> = Vec::new();
        loop {
            self.skip_newlines();
            let l = self.peek().ok_or_else(|| {
                GrammarError::new(
                    Code::MgS05,
                    self.eof_offset(),
                    "This record is missing its `end map`.",
                )
            })?;

            if l.is(&Token::KeywordEnd) {
                self.pos += 1;
                self.expect(&Token::KeywordMap, "Expected `map` after `end`.")?;
                return Ok(entries);
            }

            let entry = self.parse_entry()?;
            // MG-S02 — duplicate key within one record.
            if entries.iter().any(|e| e.key == entry.key) {
                return Err(GrammarError::new(
                    Code::MgS02,
                    entry.offset,
                    format!("Duplicate key `{}` in this record.", entry.key),
                ));
            }
            entries.push(entry);
            if entries.len() > MAX_ENTRIES_PER_RECORD {
                return Err(GrammarError::new(
                    Code::MgS07,
                    self.here(),
                    format!("This record has more than {MAX_ENTRIES_PER_RECORD} entries."),
                ));
            }
        }
    }

    fn parse_entry(&mut self) -> GrammarResult<Entry> {
        let key_offset = self.here();
        let key = self.parse_key()?;
        self.expect(&Token::KeywordIs, "Expected `is` after the key.")?;

        // The value occupies exactly the tokens between `is` and the line break.
        let value_start = self.pos;
        let mut i = value_start;
        while i < self.toks.len() && !self.toks[i].is(&Token::Newline) {
            i += 1;
        }
        if i >= self.toks.len() {
            return Err(GrammarError::new(
                Code::MgS05,
                self.here(),
                "This entry is not terminated by a line break.",
            ));
        }
        let value = parse_value(self.text, &self.toks[value_start..i], self.here())?;
        self.pos = i; // land on the Newline
        self.pos += 1; // consume it
        Ok(Entry {
            key,
            value,
            offset: key_offset,
        })
    }

    /// An entry key (`MG-S05`): a lowercase-ASCII identifier, or a quoted string
    /// (which may hold characters an identifier cannot — including a name that
    /// collides with a keyword).
    fn parse_key(&mut self) -> GrammarResult<String> {
        let l = self
            .peek()
            .ok_or_else(|| GrammarError::new(Code::MgS05, self.eof_offset(), "Expected a key."))?;
        match &l.token {
            Some(Token::Identifier(_)) => {
                let raw = l.slice(self.text);
                if raw.len() > MAX_KEY_BYTES {
                    return Err(GrammarError::new(
                        Code::MgL12,
                        l.start,
                        format!("This key is longer than {MAX_KEY_BYTES} bytes."),
                    ));
                }
                if !is_lower_ident(raw) {
                    return Err(GrammarError::new(
                        Code::MgS05,
                        l.start,
                        format!("Key `{raw}` must be a lowercase identifier, or a quoted string."),
                    ));
                }
                self.pos += 1;
                Ok(raw.to_string())
            }
            Some(Token::StringLiteral(_)) => {
                let start = l.start;
                let raw = l.slice(self.text);
                let s = validate_string(raw, start)?;
                if s.len() > MAX_KEY_BYTES {
                    return Err(GrammarError::new(
                        Code::MgL12,
                        start,
                        format!("This key is longer than {MAX_KEY_BYTES} bytes."),
                    ));
                }
                self.pos += 1;
                Ok(s)
            }
            Some(tok) if tok.is_keyword() => Err(GrammarError::new(
                Code::MgS05,
                l.start,
                format!(
                    "`{}` is a reserved WFL word; to use it as a key, quote it (\"{}\").",
                    l.slice(self.text),
                    l.slice(self.text)
                ),
            )),
            _ => Err(GrammarError::new(
                Code::MgS05,
                l.start,
                "Expected a key (a lowercase identifier or a quoted string).",
            )),
        }
    }

    fn expect(&mut self, want: &Token, msg: &str) -> GrammarResult<()> {
        match self.peek() {
            Some(l) if l.is(want) => {
                self.pos += 1;
                Ok(())
            }
            Some(l) => Err(GrammarError::new(Code::MgS05, l.start, msg.to_string())),
            None => Err(GrammarError::new(
                Code::MgS05,
                self.eof_offset(),
                msg.to_string(),
            )),
        }
    }
}

/// Parse the token slice that forms a single value (between `is` and the line
/// break). A value is exactly one literal, or one bracketed list — anything the
/// full WFL expression grammar would accept beyond that (references, operators,
/// calls) is `MG-S01`.
fn parse_value(text: &str, toks: &[Lexed], line_offset: usize) -> GrammarResult<Value> {
    let first = toks.first().ok_or_else(|| {
        GrammarError::new(Code::MgS05, line_offset, "Expected a value after `is`.")
    })?;

    if first.is(&Token::LeftBracket) {
        return parse_list(text, toks);
    }

    if toks.len() == 1 {
        return parse_scalar_value(text, first).map(Value::from);
    }

    // More than one token: an expression, a bare version, or a signed/float
    // number — none of which are admitted values.
    if matches!(first.token, Some(Token::Minus) | Some(Token::Plus)) {
        return Err(GrammarError::new(
            Code::MgL10,
            first.start,
            "Signed numbers are not allowed. Manifest integers are non-negative.",
        ));
    }
    if toks
        .iter()
        .any(|t| matches!(t.token, Some(Token::FloatLiteral(_))))
    {
        return Err(GrammarError::new(
            Code::MgL10,
            first.start,
            "Floating-point / multi-part numbers are not allowed. Quote versions as strings \
             (\"26.2.1\") and use whole numbers elsewhere.",
        ));
    }
    Err(GrammarError::new(
        Code::MgS01,
        first.start,
        "A value must be a single string, whole number, `yes`/`no`, or list — not an \
         expression, reference, or call.",
    ))
}

/// A scalar (used both as a standalone value and as a list element). Returns the
/// scalar; `MG-*` on any non-scalar or malformed literal.
fn parse_scalar_value(text: &str, l: &Lexed) -> GrammarResult<Scalar> {
    match &l.token {
        Some(Token::StringLiteral(_)) => {
            let s = validate_string(l.slice(text), l.start)?;
            Ok(Scalar::String(s))
        }
        Some(Token::IntLiteral(n)) => {
            validate_integer(l.slice(text), *n, l.start)?;
            Ok(Scalar::Integer(*n))
        }
        Some(Token::BooleanLiteral(_)) => {
            let b = validate_boolean(l.slice(text), l.start)?;
            Ok(Scalar::Boolean(b))
        }
        Some(Token::NothingLiteral) => Err(GrammarError::new(
            Code::MgL09,
            l.start,
            "There is no null value. To leave something out, omit the key entirely.",
        )),
        Some(Token::FloatLiteral(_)) => Err(GrammarError::new(
            Code::MgL10,
            l.start,
            "Floating-point numbers are not allowed.",
        )),
        Some(Token::Identifier(_)) => Err(GrammarError::new(
            Code::MgS01,
            l.start,
            format!(
                "`{}` looks like a reference. A value must be a literal; quote text in double \
                 quotes.",
                l.slice(text)
            ),
        )),
        None => {
            // A lexical error token. Classify it precisely rather than emitting a
            // generic code:
            let raw = l.slice(text);
            if raw.starts_with('"') && raw.len() >= 2 && raw.ends_with('"') {
                // The lexer rejected this *string* (e.g. an unknown escape the
                // lexer's own decoder refuses). Re-scan the raw span so the user
                // gets the exact MG-L0x reason (e.g. MG-L08) rather than MG-S01.
                validate_string(raw, l.start)?;
                // validate_string should have returned an error; if not, fail closed.
                Err(GrammarError::new(
                    Code::MgS01,
                    l.start,
                    "This is not a valid manifest value.",
                ))
            } else if !raw.is_empty() && raw.bytes().all(|b| b.is_ascii_digit()) {
                // An all-digit slice is an integer that overflowed i64.
                Err(GrammarError::new(
                    Code::MgL11,
                    l.start,
                    format!("The integer `{raw}` is too large (max {MAX_INT})."),
                ))
            } else {
                Err(GrammarError::new(
                    Code::MgS01,
                    l.start,
                    "This is not a valid manifest value.",
                ))
            }
        }
        _ => Err(GrammarError::new(
            Code::MgS01,
            l.start,
            "A value must be a string, whole number, `yes`/`no`, or list.",
        )),
    }
}

/// Parse a bracketed list (`MG-S03`): comma-separated scalars, no trailing
/// comma, no nesting.
fn parse_list(text: &str, toks: &[Lexed]) -> GrammarResult<Value> {
    debug_assert!(toks[0].is(&Token::LeftBracket));
    let open = &toks[0];
    let last = toks.last().unwrap();
    if !last.is(&Token::RightBracket) {
        return Err(GrammarError::new(
            Code::MgS05,
            open.start,
            "This list is missing its closing `]`.",
        ));
    }
    let inner = &toks[1..toks.len() - 1];
    let mut elements: Vec<Scalar> = Vec::new();
    if inner.is_empty() {
        return Ok(Value::List(elements)); // `[]`
    }

    // Alternating scalar, comma, scalar, … no trailing comma.
    let mut expect_scalar = true;
    for l in inner {
        if expect_scalar {
            if l.is(&Token::LeftBracket) || l.is(&Token::RightBracket) {
                return Err(GrammarError::new(
                    Code::MgS03,
                    l.start,
                    "Lists may not contain other lists; elements are scalars only.",
                ));
            }
            let scalar = parse_scalar_value(text, l)?;
            elements.push(scalar);
            if elements.len() > MAX_LIST_ELEMENTS {
                return Err(GrammarError::new(
                    Code::MgS07,
                    l.start,
                    format!("This list has more than {MAX_LIST_ELEMENTS} elements."),
                ));
            }
            expect_scalar = false;
        } else {
            // Separator position: comma only.
            if l.is(&Token::Comma) {
                expect_scalar = true;
            } else if l.is(&Token::KeywordAnd) || l.is(&Token::Colon) {
                return Err(GrammarError::new(
                    Code::MgS03,
                    l.start,
                    "Use a comma to separate list elements — not `and` or `:`.",
                ));
            } else {
                return Err(GrammarError::new(
                    Code::MgS03,
                    l.start,
                    "Expected a comma between list elements.",
                ));
            }
        }
    }
    if expect_scalar {
        // Ended on a separator → trailing comma.
        return Err(GrammarError::new(
            Code::MgS03,
            last.start,
            "A list may not end with a trailing comma.",
        ));
    }
    Ok(Value::List(elements))
}

impl From<Scalar> for Value {
    fn from(s: Scalar) -> Self {
        match s {
            Scalar::String(s) => Value::String(s),
            Scalar::Integer(n) => Value::Integer(n),
            Scalar::Boolean(b) => Value::Boolean(b),
        }
    }
}

/// Validate a string literal's **raw span** against the manifest string rules
/// and return the decoded value (grammar N5, §5). This never trusts the lexer's
/// lenient decode: it re-scans the raw bytes so `\r`, `\0`, raw controls, raw
/// newlines, bidi and zero-width characters are all caught here.
pub(crate) fn validate_string(raw_quoted: &str, start: usize) -> GrammarResult<String> {
    // `raw_quoted` includes the surrounding quotes.
    let inner = &raw_quoted[1..raw_quoted.len() - 1];
    let content_start = start + 1;
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.char_indices();
    while let Some((i, c)) = chars.next() {
        let off = content_start + i;
        if c == '\\' {
            match chars.next() {
                Some((_, 'n')) => out.push('\n'),
                Some((_, 't')) => out.push('\t'),
                Some((_, '\\')) => out.push('\\'),
                Some((_, '"')) => out.push('"'),
                Some((eoff, other)) => {
                    return Err(GrammarError::new(
                        Code::MgL08,
                        content_start + eoff,
                        format!(
                            "`\\{other}` is not an allowed escape. Only \\n \\t \\\\ and \\\" \
                             are permitted."
                        ),
                    ));
                }
                None => {
                    return Err(GrammarError::new(
                        Code::MgL08,
                        off,
                        "A string ends with a lone backslash.",
                    ));
                }
            }
            continue;
        }

        let cp = c as u32;
        // MG-L04 — raw newline (CR is already rejected at Gate B).
        if c == '\n' {
            return Err(GrammarError::new(
                Code::MgL04,
                off,
                "Strings are single-line. Use \\n for a line break inside a string.",
            ));
        }
        // MG-L03 — raw C0 / DEL / C1 control.
        if cp <= 0x1F || cp == 0x7F || (0x80..=0x9F).contains(&cp) {
            return Err(GrammarError::new(
                Code::MgL03,
                off,
                "This string contains a raw control character.",
            ));
        }
        // MG-L05 — bidirectional controls (Trojan Source).
        if matches!(cp, 0x202A..=0x202E | 0x2066..=0x2069 | 0x200E | 0x200F) {
            return Err(GrammarError::new(
                Code::MgL05,
                off,
                "This string contains a bidirectional control character (a Trojan-Source vector).",
            ));
        }
        // MG-L06 — zero-width / invisible characters.
        if matches!(cp, 0x200B | 0x200C | 0x200D | 0x2060 | 0xFEFF) {
            return Err(GrammarError::new(
                Code::MgL06,
                off,
                "This string contains a zero-width / invisible character.",
            ));
        }
        out.push(c);
        // MG-L12 — decoded length limit.
        if out.len() > MAX_STRING_BYTES {
            return Err(GrammarError::new(
                Code::MgL12,
                start,
                format!("This string is longer than {MAX_STRING_BYTES} bytes."),
            ));
        }
    }
    Ok(out)
}

/// Validate an integer's raw span (grammar N6, `MG-L11`): no leading zero,
/// within the safe-integer range.
pub(crate) fn validate_integer(raw: &str, n: i64, start: usize) -> GrammarResult<()> {
    if raw.len() > 1 && raw.starts_with('0') {
        return Err(GrammarError::new(
            Code::MgL11,
            start,
            format!("`{raw}` has a leading zero. Write integers without leading zeros."),
        ));
    }
    if n > MAX_INT {
        return Err(GrammarError::new(
            Code::MgL11,
            start,
            format!("The integer `{raw}` is too large (max {MAX_INT})."),
        ));
    }
    Ok(())
}

/// Validate a boolean's raw span (grammar N7, `MG-L07`): exactly `yes` or `no`,
/// lowercase. The lexer collapses `YES`/`true`/`False`/… to one `bool`, so the
/// single-spelling rule is enforced here on the raw bytes.
pub(crate) fn validate_boolean(raw: &str, start: usize) -> GrammarResult<bool> {
    match raw {
        "yes" => Ok(true),
        "no" => Ok(false),
        other => Err(GrammarError::new(
            Code::MgL07,
            start,
            format!("Booleans are written `yes` or `no` (lowercase); found `{other}`."),
        )),
    }
}

/// The `[a-z][a-z0-9_]*` identifier surface used for record names and bare keys.
fn is_lower_ident(s: &str) -> bool {
    let mut bytes = s.bytes();
    match bytes.next() {
        Some(b) if b.is_ascii_lowercase() => {}
        _ => return false,
    }
    bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
