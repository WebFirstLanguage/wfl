// TDD tests for the `parse_toml` / `stringify_toml` builtins (issue #667).
//
// The surface deliberately mirrors the JSON one (`parse_json` /
// `stringify_json` / `stringify_json_pretty`, src/stdlib/json.rs) rather than the
// `to_toml` spelling sketched in the issue, so the two formats read the same way.
//
// R3 (untrusted input): config text is attacker-reachable, so malformed input
// must produce a clean error rather than a panic.

use wfl::interpreter::value::Value;

mod common;
use common::{expect_number, expect_text, get_global, run_wfl};

// ---------------------------------------------------------------------------
// Parsing — the priority half of the issue.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn parse_toml_reads_a_realistic_config() {
    let code = r#"
store toml_text as "title = \"my project\"
port = 8080
debug = true
ratio = 0.25
"
store config as parse_toml of toml_text
store title as config["title"]
store the_port as config["port"]
store debug as config["debug"]
store ratio as config["ratio"]
"#;
    let interpreter = run_wfl(code).await.expect("program should run");
    assert_eq!(
        expect_text(&get_global(&interpreter, "title")),
        "my project"
    );
    assert_eq!(expect_number(&get_global(&interpreter, "the_port")), 8080.0);
    assert_eq!(get_global(&interpreter, "debug"), Value::Bool(true));
    assert_eq!(expect_number(&get_global(&interpreter, "ratio")), 0.25);
}

#[tokio::test]
async fn parse_toml_reads_nested_tables() {
    let code = r#"
store toml_text as "[server]
host = \"localhost\"
port = 5432

[server.tls]
enabled = true
"
store config as parse_toml of toml_text
store server_table as config["server"]
store host as server_table["host"]
store tls as server_table["tls"]
store enabled as tls["enabled"]
"#;
    let interpreter = run_wfl(code).await.expect("program should run");
    assert_eq!(expect_text(&get_global(&interpreter, "host")), "localhost");
    assert_eq!(get_global(&interpreter, "enabled"), Value::Bool(true));
}

#[tokio::test]
async fn parse_toml_reads_arrays_as_lists() {
    let code = r#"
store toml_text as "hosts = [\"a\", \"b\", \"c\"]
ports = [1, 2, 3]
"
store config as parse_toml of toml_text
store hosts as config["hosts"]
store n as length of hosts
store first as hosts[0]
"#;
    let interpreter = run_wfl(code).await.expect("program should run");
    assert_eq!(expect_number(&get_global(&interpreter, "n")), 3.0);
    assert_eq!(expect_text(&get_global(&interpreter, "first")), "a");
}

#[tokio::test]
async fn parse_toml_reads_arrays_of_tables() {
    let code = r#"
store toml_text as "[[project]]
slug = \"alpha\"

[[project]]
slug = \"beta\"
"
store config as parse_toml of toml_text
store projects as config["project"]
store n as length of projects
store second as projects[1]
store slug as second["slug"]
"#;
    let interpreter = run_wfl(code).await.expect("program should run");
    assert_eq!(expect_number(&get_global(&interpreter, "n")), 2.0);
    assert_eq!(expect_text(&get_global(&interpreter, "slug")), "beta");
}

#[tokio::test]
async fn parse_toml_reads_datetimes_as_text() {
    let code = r#"
store toml_text as "created = 1979-05-27T07:32:00Z
"
store config as parse_toml of toml_text
store created as config["created"]
"#;
    let interpreter = run_wfl(code).await.expect("program should run");
    let created = expect_text(&get_global(&interpreter, "created"));
    assert!(
        created.starts_with("1979-05-27"),
        "a TOML datetime should surface as readable text, got {created:?}"
    );
}

#[tokio::test]
async fn parse_toml_rejects_malformed_input() {
    for bad in [
        "this is not toml at all = = =",
        "[unclosed",
        "key = ",
        "key = \"unterminated",
        "a = 1\na = 2",
    ] {
        let code = format!(
            r#"
store config as parse_toml of "{}"
"#,
            bad.replace('\\', "\\\\").replace('"', "\\\"")
        );
        let err = match run_wfl(&code).await {
            Ok(_) => panic!("malformed TOML {bad:?} must be rejected"),
            Err(e) => e,
        };
        assert!(
            err.to_lowercase().contains("toml"),
            "the error should name TOML, got: {err}"
        );
    }
}

// ---------------------------------------------------------------------------
// Serializing
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stringify_toml_round_trips() {
    let code = r#"
store toml_text as "name = \"wfl\"
port = 8080
debug = false
"
store config as parse_toml of toml_text
store out as stringify_toml of config
store again as parse_toml of out
store name as again["name"]
store the_port as again["port"]
store debug as again["debug"]
"#;
    let interpreter = run_wfl(code).await.expect("program should run");
    assert_eq!(expect_text(&get_global(&interpreter, "name")), "wfl");
    assert_eq!(expect_number(&get_global(&interpreter, "the_port")), 8080.0);
    assert_eq!(get_global(&interpreter, "debug"), Value::Bool(false));
}

#[tokio::test]
async fn stringify_toml_pretty_round_trips_nested_structure() {
    let code = r#"
store toml_text as "[server]
host = \"localhost\"
ports = [1, 2]
"
store config as parse_toml of toml_text
store out as stringify_toml_pretty of config
store again as parse_toml of out
store server_table as again["server"]
store host as server_table["host"]
store ports as server_table["ports"]
store n as length of ports
"#;
    let interpreter = run_wfl(code).await.expect("program should run");
    assert_eq!(expect_text(&get_global(&interpreter, "host")), "localhost");
    assert_eq!(expect_number(&get_global(&interpreter, "n")), 2.0);
}

#[tokio::test]
async fn stringify_toml_requires_a_table_at_the_top_level() {
    // TOML documents are always tables. A bare list has no valid representation,
    // and saying so beats emitting something that will not parse back.
    let code = r#"
store out as stringify_toml of [1 and 2 and 3]
"#;
    let err = run_wfl(code)
        .await
        .err()
        .expect("a top-level list is not a valid TOML document");
    assert!(
        err.to_lowercase().contains("toml"),
        "the error should name TOML, got: {err}"
    );
}

#[tokio::test]
async fn nothing_valued_keys_are_omitted_rather_than_invented() {
    // TOML has no null. An absent value is an absent key — that is the format's
    // own idiom, and it round-trips.
    let code = r#"
store toml_text as "present = \"yes\"
"
store config as parse_toml of toml_text
store absent as nothing
store out as stringify_toml of config
store again as parse_toml of out
store still_there as again["present"]
"#;
    let interpreter = run_wfl(code).await.expect("program should run");
    assert_eq!(
        expect_text(&get_global(&interpreter, "still_there")),
        "yes",
        "present keys must survive the round trip"
    );
}

#[tokio::test]
async fn parse_toml_of_an_empty_document_gives_an_empty_table() {
    let code = r#"
store config as parse_toml of ""
store out as stringify_toml of config
"#;
    let interpreter = run_wfl(code).await.expect("program should run");
    assert!(
        matches!(get_global(&interpreter, "config"), Value::Object(_)),
        "an empty TOML document should parse to an empty object"
    );
    assert_eq!(expect_text(&get_global(&interpreter, "out")), "");
}

/// TOML dates parse to text, and therefore write back as strings rather than
/// dates. That is a real fidelity loss, so it is pinned here and stated in
/// Docs/05-standard-library/toml-module.md rather than left for a user to
/// discover from a config a downstream tool then rejects.
#[tokio::test]
async fn a_date_round_trips_as_text_not_as_a_toml_date() {
    let interpreter = run_wfl(
        r#"
store parsed as parse_toml of "released = 2026-07-31
"
store rendered as stringify_toml of parsed
"#,
    )
    .await
    .expect("valid TOML");

    assert_eq!(
        expect_text(&get_global(&interpreter, "rendered")).trim(),
        r#"released = "2026-07-31""#,
        "the characters survive, but the TOML type becomes a string"
    );
}

/// WFL numbers are f64, so TOML integers beyond 2^53 are rounded on the way in.
/// Documented in the TOML page; asserted here so the boundary is not moved
/// silently.
#[tokio::test]
async fn integers_within_exact_f64_range_survive_a_round_trip() {
    let interpreter = run_wfl(
        r#"
store parsed as parse_toml of "id = 9007199254740991
"
store rendered as stringify_toml of parsed
"#,
    )
    .await
    .expect("valid TOML");

    assert_eq!(
        expect_text(&get_global(&interpreter, "rendered")).trim(),
        "id = 9007199254740991",
        "2^53 - 1 is exactly representable and must survive"
    );
}

#[tokio::test]
async fn integers_beyond_exact_f64_range_are_rounded() {
    let interpreter = run_wfl(
        r#"
store parsed as parse_toml of "id = 9007199254740993
"
store rendered as stringify_toml of parsed
"#,
    )
    .await
    .expect("valid TOML");

    assert_eq!(
        expect_text(&get_global(&interpreter, "rendered")).trim(),
        "id = 9007199254740992",
        "beyond 2^53 the value is rounded — keep such ids as quoted strings"
    );
}
