//! Canonical on-disk form — `wfl fmt` (grammar §7.1).
//!
//! `wfl fmt` is the **only** sanctioned writer and it operates on author intent
//! *before* signing, never on the verification path. The canonical form is
//! UTF-8, NFC, LF line endings, no BOM, no comments; one record block per
//! record separated by a single blank line; exactly one space around `is` and
//! after `:`; one entry per line; list elements separated by `", "`.
//!
//! Because every non-canonical form is *rejected* (not collapsed), the
//! concrete→node mapping is injective over accepted inputs, so this writer is
//! idempotent (`fmt(fmt(x)) == fmt(x)`) and round-trips
//! (`parse(fmt(x)) == parse(x)`). Record and entry order are preserved from the
//! node tree; schema-canonical *key* order is the schema writer's job.

use super::{Document, Record, Scalar, Value};

/// Serialize a [`Document`] to its canonical byte form (returned as a `String`;
/// it is pure ASCII structure plus already-NFC string contents).
pub fn to_canonical(doc: &Document) -> String {
    doc.records
        .iter()
        .map(render_record)
        .collect::<Vec<_>>()
        .join("\n") // a single blank line between record blocks
}

fn render_record(r: &Record) -> String {
    let mut s = String::new();
    s.push_str("create map ");
    s.push_str(&r.kind);
    s.push_str(":\n");
    for e in &r.entries {
        render_key(&mut s, &e.key);
        s.push_str(" is ");
        render_value(&mut s, &e.value);
        s.push('\n');
    }
    s.push_str("end map\n");
    s
}

/// A key is emitted bare when it is a lowercase identifier, quoted otherwise
/// (e.g. a key that collides with a keyword, or holds non-identifier bytes).
fn render_key(out: &mut String, key: &str) {
    if is_bare_key(key) {
        out.push_str(key);
    } else {
        render_string(out, key);
    }
}

fn is_bare_key(key: &str) -> bool {
    let mut bytes = key.bytes();
    match bytes.next() {
        Some(b) if b.is_ascii_lowercase() => {}
        _ => return false,
    }
    bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn render_value(out: &mut String, v: &Value) {
    match v {
        Value::String(s) => render_string(out, s),
        Value::Integer(n) => out.push_str(&n.to_string()),
        Value::Boolean(b) => out.push_str(if *b { "yes" } else { "no" }),
        Value::List(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                render_scalar(out, item);
            }
            out.push(']');
        }
    }
}

fn render_scalar(out: &mut String, s: &Scalar) {
    match s {
        Scalar::String(s) => render_string(out, s),
        Scalar::Integer(n) => out.push_str(&n.to_string()),
        Scalar::Boolean(b) => out.push_str(if *b { "yes" } else { "no" }),
    }
}

/// Emit a string with the canonical, minimal escape set (`\n \t \\ \"`). All
/// other characters are already validated to be printable NFC UTF-8.
fn render_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
}
