//! Deterministic JSON projection — I-JSON (RFC 7493) canonicalized per RFC 8785
//! (JCS) (grammar §7.2).
//!
//! This is the lossless interop export served by `wfl manifest --json`, and the
//! byte string the `content_hash` is taken over. Two properties make it
//! footgun-free *because of decisions already locked upstream*: floats are
//! banned, so the `1`/`1.0` collision and the shortest-round-trip number path
//! never arise; and object keys are sorted by **UTF-16 code unit** exactly as
//! JCS requires (computed here for all keys, so even a non-ASCII string key
//! sorts correctly).
//!
//! A whole document projects to a JSON **array** of single-key record objects,
//! preserving document order: `[{"wflpkg":{…}}, {"package":{…}}, …]`. The single
//! kind key avoids any collision with an entry key, and array order means record
//! order is part of the content identity.

use super::{Document, Record, Scalar, Value};

/// A minimal JSON value model, just enough to render JCS.
enum Json {
    Str(String),
    Int(i64),
    Bool(bool),
    Array(Vec<Json>),
    /// Insertion order is irrelevant: [`Json::write`] sorts object keys.
    Object(Vec<(String, Json)>),
}

/// Render a document as canonical (JCS) JSON text.
pub fn to_jcs(doc: &Document) -> String {
    let json = document_to_json(doc);
    let mut out = String::new();
    json.write(&mut out);
    out
}

fn document_to_json(doc: &Document) -> Json {
    Json::Array(doc.records.iter().map(record_to_json).collect())
}

fn record_to_json(r: &Record) -> Json {
    let entries = r
        .entries
        .iter()
        .map(|e| (e.key.clone(), value_to_json(&e.value)))
        .collect();
    Json::Object(vec![(r.kind.clone(), Json::Object(entries))])
}

fn value_to_json(v: &Value) -> Json {
    match v {
        Value::String(s) => Json::Str(s.clone()),
        Value::Integer(n) => Json::Int(*n),
        Value::Boolean(b) => Json::Bool(*b),
        Value::List(items) => Json::Array(items.iter().map(scalar_to_json).collect()),
    }
}

fn scalar_to_json(s: &Scalar) -> Json {
    match s {
        Scalar::String(s) => Json::Str(s.clone()),
        Scalar::Integer(n) => Json::Int(*n),
        Scalar::Boolean(b) => Json::Bool(*b),
    }
}

impl Json {
    fn write(&self, out: &mut String) {
        match self {
            Json::Str(s) => write_json_string(out, s),
            Json::Int(n) => out.push_str(&n.to_string()),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            Json::Object(entries) => {
                // JCS: sort members by the UTF-16 code units of their keys.
                let mut sorted: Vec<&(String, Json)> = entries.iter().collect();
                sorted.sort_by(|a, b| utf16_key(&a.0).cmp(&utf16_key(&b.0)));
                out.push('{');
                for (i, (key, val)) in sorted.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_json_string(out, key);
                    out.push(':');
                    val.write(out);
                }
                out.push('}');
            }
        }
    }
}

fn utf16_key(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// JCS string escaping (RFC 8785 → ECMA-262 `JSON.stringify`): escape `"` and
/// `\`, use the short escapes for the control characters that have them, and
/// `\u00xx` for the remaining C0 controls. Every other character — including
/// all non-ASCII — is emitted as raw UTF-8.
fn write_json_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{0009}' => out.push_str("\\t"),
            '\u{000A}' => out.push_str("\\n"),
            '\u{000C}' => out.push_str("\\f"),
            '\u{000D}' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}
