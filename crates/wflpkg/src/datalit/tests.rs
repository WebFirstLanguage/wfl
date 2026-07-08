//! Conformance tests for the frozen data-literal grammar.
//!
//! Covers the happy path and value forms, the determinism / idempotence /
//! round-trip / collapse-freedom properties, the drift oracle (§9), and the
//! full §10 malicious-input reject corpus (the `corpus` submodule). The
//! subset-equivalence guard (`L(manifest) ⊂ L(WFL)`) lives in the root crate at
//! `tests/manifest_subset_equivalence.rs`, since it needs the full WFL parser.

use super::*;

/// Parse helper over a `&str`.
fn parse_str(s: &str) -> GrammarResult<Document> {
    parse(s.as_bytes())
}

/// Assert that an input is rejected with exactly the given code.
fn assert_code(input: &str, expected: Code) {
    match parse_str(input) {
        Ok(_) => panic!("expected {expected} for input {input:?}, but it was accepted"),
        Err(e) => assert_eq!(
            e.code, expected,
            "input {input:?}: expected {expected}, got {} ({})",
            e.code, e.message
        ),
    }
}

const MANIFEST: &str = "\
create map wflpkg:
    grammar is \"1.0.0\"
end map

create map package:
    name is \"greeting\"
    version is \"26.2.1\"
    description is \"A friendly command-line greeter\"
    authors is [\"Alice Smith\", \"Bob Jones\"]
    license is \"MIT\"
    keywords is [\"cli\", \"greeting\"]
    notes is \"Human-readable annotations live here, inside the hash.\"
end map

create map dependency:
    name is \"http-client\"
    version is \"26.1 or newer\"
end map

create map dependency:
    name is \"json-parser\"
    version is \"25.12 or newer\"
    scope is \"dev\"
end map
";

const LOCKFILE: &str = "\
create map wflpkg:
    grammar is \"1.0.0\"
end map

create map locked:
    name is \"http-client\"
    version is \"26.1.3\"
    hash is \"sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08\"
    deps is [\"text-utils\"]
end map

create map locked:
    name is \"text-utils\"
    version is \"25.11.2\"
    hash is \"sha256:2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae\"
    deps is []
end map
";

#[test]
fn accepts_the_illustrative_manifest() {
    let doc = parse_str(MANIFEST).expect("manifest should parse");
    assert_eq!(doc.records.len(), 4);
    assert_eq!(doc.records[0].kind, "wflpkg");
    let package = doc.single("package").unwrap();
    assert_eq!(package.get_string("name"), Some("greeting"));
    assert_eq!(package.get_string("version"), Some("26.2.1"));
    // Repeated `dependency` kind is allowed.
    assert_eq!(doc.records_of("dependency").count(), 2);
    // List value.
    match package.get("authors") {
        Some(Value::List(items)) => assert_eq!(items.len(), 2),
        other => panic!("authors should be a list, got {other:?}"),
    }
}

#[test]
fn accepts_the_illustrative_lockfile() {
    let doc = parse_str(LOCKFILE).expect("lockfile should parse");
    assert_eq!(doc.records_of("locked").count(), 2);
    // Empty list.
    let text_utils = doc.records_of("locked").nth(1).unwrap();
    assert_eq!(text_utils.get("deps"), Some(&Value::List(vec![])));
}

#[test]
fn value_forms() {
    let doc = parse_str(
        "create map t:\n\
         s is \"hi\"\n\
         n is 42\n\
         z is 0\n\
         b is yes\n\
         c is no\n\
         nums is [1, 2, 3]\n\
         end map\n",
    )
    .unwrap();
    let r = &doc.records[0];
    assert_eq!(r.get("s"), Some(&Value::String("hi".to_string())));
    assert_eq!(r.get("n"), Some(&Value::Integer(42)));
    assert_eq!(r.get("z"), Some(&Value::Integer(0)));
    assert_eq!(r.get("b"), Some(&Value::Boolean(true)));
    assert_eq!(r.get("c"), Some(&Value::Boolean(false)));
    assert_eq!(
        r.get("nums"),
        Some(&Value::List(vec![
            Scalar::Integer(1),
            Scalar::Integer(2),
            Scalar::Integer(3)
        ]))
    );
}

#[test]
fn quoted_keyword_key_is_allowed() {
    // `requires` is a reserved keyword; as a *quoted* key it is fine.
    let doc = parse_str("create map t:\n\"requires\" is \"x\"\nend map\n").unwrap();
    assert_eq!(doc.records[0].get_string("requires"), Some("x"));
}

#[test]
fn string_escapes() {
    let doc = parse_str("create map t:\ns is \"a\\nb\\t\\\"c\\\\d\"\nend map\n").unwrap();
    assert_eq!(
        doc.records[0].get("s"),
        Some(&Value::String("a\nb\t\"c\\d".to_string()))
    );
}

// ---- A sampling of rejections; the full corpus is in tests/ ----

#[test]
fn rejects_bom() {
    assert_code("\u{FEFF}create map t:\nx is 1\nend map\n", Code::MgB04);
}

#[test]
fn rejects_crlf() {
    assert_code("create map t:\r\nx is 1\r\nend map\r\n", Code::MgB07);
}

#[test]
fn rejects_comment() {
    assert_code("create map t: // hi\nx is 1\nend map\n", Code::MgL01);
    assert_code("create map t:\n# hi\nx is 1\nend map\n", Code::MgL01);
}

#[test]
fn rejects_boolean_spelling() {
    assert_code("create map t:\nx is true\nend map\n", Code::MgL07);
    assert_code("create map t:\nx is YES\nend map\n", Code::MgL07);
}

#[test]
fn rejects_null() {
    assert_code("create map t:\nx is nothing\nend map\n", Code::MgL09);
    assert_code("create map t:\nx is missing\nend map\n", Code::MgL09);
}

#[test]
fn rejects_float() {
    assert_code("create map t:\nx is 1.5\nend map\n", Code::MgL10);
}

#[test]
fn rejects_leading_zero_and_overflow() {
    assert_code("create map t:\nx is 007\nend map\n", Code::MgL11);
    assert_code(
        "create map t:\nx is 99999999999999999999\nend map\n",
        Code::MgL11,
    );
}

#[test]
fn rejects_duplicate_key() {
    assert_code("create map t:\nx is 1\nx is 2\nend map\n", Code::MgS02);
}

#[test]
fn rejects_bare_keyword_key() {
    // `port` is a reserved keyword; as a bare key it is rejected (quote it).
    assert_code("create map t:\nport is 1\nend map\n", Code::MgS05);
}

#[test]
fn rejects_reference_value() {
    assert_code("create map t:\nname is greeting\nend map\n", Code::MgS01);
}

#[test]
fn rejects_arithmetic_value() {
    // A non-keyword key so we reach the value: `1 plus 1` is an expression.
    assert_code("create map t:\nretries is 1 plus 1\nend map\n", Code::MgS01);
}

#[test]
fn rejects_bad_list_separator() {
    assert_code("create map t:\nx is [1 and 2]\nend map\n", Code::MgS03);
    assert_code("create map t:\nx is [1 : 2]\nend map\n", Code::MgS03);
    assert_code("create map t:\nx is [1, 2,]\nend map\n", Code::MgS03);
}

#[test]
fn rejects_top_level_non_record() {
    assert_code("store x as 5\n", Code::MgS04);
}

#[test]
fn rejects_missing_end_map() {
    assert_code("create map t:\nx is 1\n", Code::MgS05);
}

#[test]
fn rejects_confusable_and_underscore_names() {
    assert_code(
        "create map package:\nname is \"my_pkg\"\nend map\n",
        Code::MgI01,
    );
}

#[test]
fn rejects_bad_version() {
    assert_code(
        "create map package:\nname is \"x\"\nversion is \"26.2\"\nend map\n",
        Code::MgI02,
    );
}

// ---- Determinism / idempotence / round-trip / collapse-freedom ----

#[test]
fn fmt_round_trips_and_is_idempotent() {
    let doc = parse_str(MANIFEST).unwrap();
    let once = fmt::to_canonical(&doc);
    let reparsed = parse_str(&once).unwrap();
    assert_eq!(doc, reparsed, "parse(fmt(x)) must equal parse(x)");
    let twice = fmt::to_canonical(&reparsed);
    assert_eq!(once, twice, "fmt must be idempotent");
}

#[test]
fn hashes_are_format_insensitive_for_content_but_not_for_file() {
    let compact = "create map t:\nx is yes\ny is [1,2]\nend map\n";
    let spaced = "create map t:\n    x is yes\n    y is [1, 2]\n\nend map\n";
    let a = parse_str(compact).unwrap();
    let b = parse_str(spaced).unwrap();
    // Same spec → same content_hash.
    assert_eq!(hash::content_hash(&a), hash::content_hash(&b));
    // The two canonical byte forms are identical too (fmt normalizes both).
    assert_eq!(fmt::to_canonical(&a), fmt::to_canonical(&b));
    assert_eq!(hash::file_hash(&a), hash::file_hash(&b));
}

#[test]
fn json_projection_is_canonical() {
    let doc = parse_str("create map t:\nb is no\na is yes\nn is 7\nend map\n").unwrap();
    // Keys sorted (a,b,n); yes/no → true/false; array-of-record shape.
    assert_eq!(
        json::to_jcs(&doc),
        "[{\"t\":{\"a\":true,\"b\":false,\"n\":7}}]"
    );
}

#[test]
fn parse_is_pure_and_deterministic() {
    // Same bytes → identical result, twice (grammar §10.7).
    assert_eq!(parse_str(MANIFEST).unwrap(), parse_str(MANIFEST).unwrap());
}

#[test]
fn collapse_freedom_reject_cases() {
    // Two byte-strings that a *naive* parser would collapse to the same node
    // must not both be accepted — the surface-spelling one is rejected
    // (grammar §10.6). `7` is accepted; `007` is rejected (not silently == 7).
    assert!(parse_str("create map t:\nx is 7\nend map\n").is_ok());
    assert_code("create map t:\nx is 007\nend map\n", Code::MgL11);
    // `yes` accepted; `YES`/`true` rejected (not silently == true).
    assert!(parse_str("create map t:\nx is yes\nend map\n").is_ok());
    assert_code("create map t:\nx is YES\nend map\n", Code::MgL07);
    assert_code("create map t:\nx is true\nend map\n", Code::MgL07);
}

/// The §10 malicious-input reject corpus. Each input MUST reject with the stated
/// code; adding a bypass is the first step of any incident response. This corpus
/// is versioned with the grammar.
mod corpus {
    use super::*;

    fn assert_bytes(bytes: &[u8], expected: Code) {
        match parse(bytes) {
            Ok(_) => panic!("expected {expected} for bytes {bytes:x?}, but accepted"),
            Err(e) => assert_eq!(
                e.code, expected,
                "bytes {bytes:x?}: expected {expected}, got {} ({})",
                e.code, e.message
            ),
        }
    }

    // A minimal well-formed record we can wrap malformed values in.
    fn record(body: &str) -> String {
        format!("create map t:\n{body}\nend map\n")
    }

    #[test]
    fn gate_b_byte_level() {
        // MG-B04 — BOM anywhere.
        assert_bytes(
            &[&[0xEF, 0xBB, 0xBF], record("x is 1").as_bytes()].concat(),
            Code::MgB04,
        );
        // MG-B02 — overlong UTF-8.
        assert_bytes(&[0xC0, 0x80], Code::MgB02);
        assert_bytes(&[0xE0, 0x80, 0x80], Code::MgB02);
        assert_bytes(&[0xF0, 0x80, 0x80, 0x80], Code::MgB02);
        // MG-B03 — surrogate.
        assert_bytes(&[0xED, 0xA0, 0x80], Code::MgB03);
        // MG-B05 — non-NFC (decomposed é = 'e' + U+0301).
        assert_bytes(record("x is \"e\u{0301}\"").as_bytes(), Code::MgB05);
        // MG-B07 — bare CR / CRLF.
        assert_bytes(b"create map t:\rx is 1\rend map\r", Code::MgB07);
        assert_bytes(b"create map t:\r\nx is 1\r\nend map\r\n", Code::MgB07);
    }

    #[test]
    fn gate_l_lexical() {
        // MG-L01 — comments (// and #), including a bidi-in-comment payload.
        assert_code("create map t: // hi\nx is 1\nend map\n", Code::MgL01);
        assert_code("create map t:\n# hi\nx is 1\nend map\n", Code::MgL01);
        assert_code(
            "create map t:\nx is 1 // \u{202E}evil\nend map\n",
            Code::MgL01,
        );
        // MG-L02 — form-feed between tokens.
        assert_bytes(b"create map t:\nx\x0Cis 1\nend map\n", Code::MgL02);
        // MG-L03 — raw NUL / C0 / DEL in a string.
        assert_bytes(b"create map t:\nx is \"a\x00b\"\nend map\n", Code::MgL03);
        assert_bytes(b"create map t:\nx is \"a\x7Fb\"\nend map\n", Code::MgL03);
        // MG-L04 — raw newline in a string.
        assert_bytes(b"create map t:\nx is \"a\nb\"\nend map\n", Code::MgL04);
        // MG-L05 — bidi control in a string (Trojan Source).
        assert_code(&record("x is \"a\u{202E}b\""), Code::MgL05);
        // MG-L06 — zero-width / invisible.
        assert_code(&record("x is \"a\u{200B}b\""), Code::MgL06);
        assert_code(&record("x is \"a\u{200D}b\""), Code::MgL06);
        // MG-L07 — non-canonical boolean spelling.
        assert_code(&record("x is true"), Code::MgL07);
        assert_code(&record("x is Yes"), Code::MgL07);
        // MG-L08 — disallowed escapes.
        assert_code(&record("x is \"a\\rb\""), Code::MgL08);
        assert_code(&record("x is \"a\\0b\""), Code::MgL08);
        assert_code(&record("x is \"a\\qb\""), Code::MgL08);
        // MG-L09 — null literal in value position.
        assert_code(&record("x is nothing"), Code::MgL09);
        assert_code(&record("x is missing"), Code::MgL09);
        assert_code(&record("x is undefined"), Code::MgL09);
        // MG-L10 — float / signed.
        assert_code(&record("x is 1.5"), Code::MgL10);
        assert_code(&record("x is -5"), Code::MgL10);
        // MG-L11 — leading zero, and overflow.
        assert_code(&record("x is 007"), Code::MgL11);
        assert_code(&record("x is 99999999999999999999"), Code::MgL11);
    }

    #[test]
    fn gate_l_over_length_string() {
        assert_code(
            &record(&format!("x is \"{}\"", "a".repeat(70_000))),
            Code::MgL12,
        );
    }

    #[test]
    fn gate_l_over_length_key() {
        assert_code(&record(&format!("{} is 1", "k".repeat(300))), Code::MgL12);
    }

    /// §10.3: a very long single token must terminate in bounded steps with a
    /// typed code — never a stack-overflow abort. This exercises the large-stack
    /// parse worker against a token far larger than any legitimate manifest.
    #[test]
    fn bounded_termination_on_huge_token() {
        // ~200 KiB string, under MG-B06 (256 KiB) but far over MG-L12 (64 KiB).
        assert_code(
            &record(&format!("x is \"{}\"", "a".repeat(200_000))),
            Code::MgL12,
        );
    }

    /// A document over the size limit is rejected at Gate B, never lexed.
    #[test]
    fn gate_b_document_too_large() {
        let huge = "a".repeat(300 * 1024);
        assert_code(&record(&format!("x is \"{huge}\"")), Code::MgB06);
    }

    #[test]
    fn gate_s_structural() {
        // MG-S01 — reference / expression as value.
        assert_code(&record("name is greeting"), Code::MgS01);
        assert_code(&record("retries is 1 plus 1"), Code::MgS01);
        // MG-S02 — duplicate key.
        assert_code(&record("x is 1\nx is 2"), Code::MgS02);
        // MG-S03 — bad list separator / trailing comma / nested list.
        assert_code(&record("x is [1 and 2]"), Code::MgS03);
        assert_code(&record("x is [1 : 2]"), Code::MgS03);
        assert_code(&record("x is [1, 2,]"), Code::MgS03);
        assert_code(&record("x is [[1]]"), Code::MgS03);
        // MG-S04 — top-level non-record, incl. old-dialect regressions.
        assert_code("store x as 5\n", Code::MgS04);
        assert_code("version is 26.2.1\n", Code::MgS04);
        assert_code("requires http-client 26.1 or newer\n", Code::MgS04);
        // MG-S05 — missing end map, bare keyword key.
        assert_code("create map t:\nx is 1\n", Code::MgS05);
        assert_code(&record("port is 1"), Code::MgS05);
    }

    #[test]
    fn gate_i_identity() {
        // MG-I01 — Cyrillic homoglyph name (CVE-2021-42694), underscore name.
        assert_code(
            "create map package:\nname is \"ра\"\nend map\n",
            Code::MgI01,
        );
        assert_code(
            "create map package:\nname is \"my_pkg\"\nend map\n",
            Code::MgI01,
        );
        // MG-I02 — bad version (unquoted decomposes to a float first).
        assert_code(&record("version is 26.2.1"), Code::MgL10);
        assert_code(
            "create map package:\nname is \"x\"\nversion is \"26.2\"\nend map\n",
            Code::MgI02,
        );
    }
}

/// Drift oracle (grammar §9): the manifest grammar pins the exact byte-surface
/// of the small set of tokens it relies on. If the shared lexer ever changes how
/// one of these lexes, this test turns the build red and forces a conscious
/// grammar-version decision, rather than silently changing the manifest format.
#[test]
fn drift_oracle_pinned_token_surface() {
    use logos::Logos;
    use wfl_core::lexer::token::Token;

    fn lex_one(s: &str) -> Token {
        Token::lexer(s).next().unwrap().unwrap()
    }

    // The manifest's structural keyword surface: {create, map, is, end}.
    assert_eq!(lex_one("create"), Token::KeywordCreate);
    assert_eq!(lex_one("map"), Token::KeywordMap);
    assert_eq!(lex_one("is"), Token::KeywordIs);
    assert_eq!(lex_one("end"), Token::KeywordEnd);
    // Booleans: exactly `yes`/`no` carry a bool; case is erased by the lexer
    // (which is *why* MG-L07 reads the raw span).
    assert_eq!(lex_one("yes"), Token::BooleanLiteral(true));
    assert_eq!(lex_one("no"), Token::BooleanLiteral(false));
    // The null spellings collapse to one token (why absence must be a key).
    assert_eq!(lex_one("nothing"), Token::NothingLiteral);
    assert_eq!(lex_one("missing"), Token::NothingLiteral);
    // Punctuation and literals.
    assert_eq!(lex_one("["), Token::LeftBracket);
    assert_eq!(lex_one("]"), Token::RightBracket);
    assert_eq!(lex_one(","), Token::Comma);
    assert_eq!(lex_one(":"), Token::Colon);
    assert_eq!(lex_one("7"), Token::IntLiteral(7));
    assert_eq!(lex_one("\"hi\""), Token::StringLiteral("hi".to_string()));
    // Leading zeros and case are collapsed by the lexer payload — the manifest
    // relies on reading raw spans to catch them.
    assert_eq!(lex_one("007"), Token::IntLiteral(7));
    assert_eq!(lex_one("TRUE"), Token::BooleanLiteral(true));
}
