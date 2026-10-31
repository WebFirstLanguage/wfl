//! `random_seed` is an error in any file that also does crypto or auth
//! (`ANALYZE-SECURITY`). "Any file" has two halves, and these tests pin both:
//!
//! 1. **Anywhere in the file.** The lint's call collector must reach every
//!    expression a program can hold. Each walker test below hides the only
//!    crypto/auth call — or the only seed — inside one construct, so it fails
//!    if the collector skips that construct. Before this suite the collector
//!    ended in a catch-all that silently ignored container creation, method
//!    calls, map/list creation, file writes, outbound HTTP, `expect`,
//!    `wait for`, property defaults, and more.
//! 2. **Any file that runs.** A file brought in with `include from`,
//!    `load module from`, or `execute file at` is held to the same rule as
//!    the file named on the command line, so moving the code into a second
//!    file does not evade it. These tests spawn the real `wfl` binary.

use std::fs;
use std::process::Command;
use tempfile::TempDir;
use wfl::analyzer::Analyzer;
use wfl::analyzer::static_analyzer::{StaticAnalyzer, rng_security_ingredients};
use wfl::diagnostics::Severity;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

use crate::common::wfl_exe;

fn parse(source: &str) -> wfl::parser::ast::Program {
    let tokens = lex_wfl_with_positions(source);
    Parser::new(&tokens)
        .parse()
        .unwrap_or_else(|errors| panic!("test program must parse: {errors:?}\n{source}"))
}

/// The file lint must report exactly one `ANALYZE-SECURITY` error: the single
/// `random_seed` call in `source`. `construct` names where the hidden call sits.
fn assert_flagged(construct: &str, source: &str) {
    let program = parse(source);
    let found = rng_security_ingredients(&program);
    assert!(
        found.seeds_rng(),
        "{construct}: the random_seed call was not collected"
    );
    assert!(
        found.uses_security_builtin(),
        "{construct}: the crypto/auth call was not collected"
    );

    let diagnostics = Analyzer::new().check_insecure_rng_seeding(&program, 0);
    assert_eq!(
        diagnostics.len(),
        1,
        "{construct}: expected one ANALYZE-SECURITY error, got {diagnostics:?}"
    );
    assert_eq!(diagnostics[0].code, "ANALYZE-SECURITY", "{construct}");
    assert_eq!(diagnostics[0].severity, Severity::Error, "{construct}");
}

// ---------------------------------------------------------------------------
// 1. The collector reaches every construct (crypto/auth call hidden inside).
// ---------------------------------------------------------------------------

#[test]
fn crypto_call_in_a_container_property_initializer_is_seen() {
    assert_flagged(
        "create new … property initializer",
        r#"
create container Account:
    property password_hash: Text
end
store seeded as random_seed of 42
create new Account as alice:
    password_hash is hash_password of "hunter2"
end
"#,
    );
}

#[test]
fn crypto_call_in_a_container_constructor_argument_is_seen() {
    assert_flagged(
        "create new … with <argument>",
        r#"
create container Account:
    property password_hash: Text
    action initialize needs digest: Text:
        change password_hash to digest
    end
end
store seeded as random_seed of 42
create new Account with (hash_password of "hunter2") as alice:
end
"#,
    );
}

#[test]
fn crypto_call_in_a_container_property_default_is_seen() {
    assert_flagged(
        "property … defaults <expr>",
        r#"
create container Session:
    property token: Text defaults secure_random_bytes of 16
end
store seeded as random_seed of 42
"#,
    );
}

#[test]
fn crypto_call_in_a_method_call_argument_is_seen() {
    assert_flagged(
        "object.method(<argument>)",
        r#"
create container Vault:
    property secret: Text
    action keep needs value: Text:
        change secret to value
    end
end
create new Vault as vault:
    secret is ""
end
store seeded as random_seed of 42
vault.keep(secure_random_bytes of 16)
"#,
    );
}

#[test]
fn crypto_call_in_a_map_entry_is_seen() {
    assert_flagged(
        "create map … entry",
        r#"
store seeded as random_seed of 42
create map session_info:
    token is secure_random_bytes of 16
end map
"#,
    );
}

#[test]
fn crypto_call_in_a_list_element_is_seen() {
    assert_flagged(
        "create list … element",
        r#"
store seeded as random_seed of 42
create list tokens:
    add secure_random_bytes of 16
end list
"#,
    );
}

#[test]
fn crypto_call_added_to_a_list_is_seen() {
    assert_flagged(
        "add <value> to <list>",
        r#"
store seeded as random_seed of 42
create list hashes:
end list
add hash_password of "hunter2" to hashes
"#,
    );
}

#[test]
fn crypto_call_in_file_write_content_is_seen() {
    assert_flagged(
        "write content <value> into <file>",
        r#"
store seeded as random_seed of 42
open file at "token.txt" for writing as token_file
write content secure_random_bytes of 16 into token_file
close file token_file
"#,
    );
}

#[test]
fn crypto_call_in_created_file_content_is_seen() {
    assert_flagged(
        "create file at <path> with <content>",
        r#"
store seeded as random_seed of 42
create file at "token.txt" with secure_random_bytes of 16
"#,
    );
}

#[test]
fn crypto_call_inside_wait_for_is_seen() {
    assert_flagged(
        "wait for <statement>",
        r#"
store seeded as random_seed of 42
open file at "token.txt" for writing as token_file
wait for write content secure_random_bytes of 16 into token_file
close file token_file
"#,
    );
}

#[test]
fn crypto_call_in_an_outbound_http_body_is_seen() {
    // Webhook signing is the textbook place an HMAC appears.
    assert_flagged(
        "open url … and body <expr>",
        r#"
store seeded as random_seed of 42
open url at "http://127.0.0.1:9/hook" with method "POST" and body (hmac_sha256 of "payload" and "key") and read response as resp
"#,
    );
}

#[test]
fn crypto_call_in_an_expect_assertion_is_seen() {
    // A test file that seeds for reproducibility and checks a password.
    assert_flagged(
        "expect <subject> to …",
        r#"
describe "login":
    test "accepts the right password":
        store seeded as random_seed of 42
        store stored as "not-a-real-hash"
        expect verify_password of "hunter2" and stored to be yes
    end test
end describe
"#,
    );
}

#[test]
fn crypto_call_in_a_database_query_expression_is_seen() {
    // The statement form was fixed in July; this is the expression form.
    assert_flagged(
        "give back query … and parameters [<expr>]",
        r#"
open database at "sqlite::memory:" as db
store seeded as random_seed of 42
define action called save_secret:
    give back query db with "SELECT ?" and parameters [seal of "secret" and "key"]
end action
"#,
    );
}

#[test]
fn crypto_call_in_a_websocket_message_is_seen() {
    assert_flagged(
        "send websocket message <expr> to <connection>",
        r#"
listen for websockets on port 9 as sockets
store seeded as random_seed of 42
on websocket connect to sockets as conn:
    send websocket message secure_random_bytes of 16 to conn
end on
"#,
    );
}

// ---------------------------------------------------------------------------
// 1b. The seed half is collected from the same constructs.
// ---------------------------------------------------------------------------

#[test]
fn random_seed_in_a_map_entry_is_seen() {
    assert_flagged(
        "random_seed inside create map",
        r#"
create map run_config:
    seed is random_seed of 42
end map
store digest as hash_password of "hunter2"
"#,
    );
}

#[test]
fn random_seed_in_a_container_property_initializer_is_seen() {
    assert_flagged(
        "random_seed inside create new … property initializer",
        r#"
create container Simulation:
    property seed: Number
end
create new Simulation as sim:
    seed is random_seed of 42
end
store digest as hash_password of "hunter2"
"#,
    );
}

// ---------------------------------------------------------------------------
// 1c. Renaming a builtin does not hide it. WFL lets a program store a builtin
// in a variable and call it through that name, so a bare mention counts.
// ---------------------------------------------------------------------------

#[test]
fn random_seed_called_through_an_alias_is_seen() {
    assert_flagged(
        "store f as random_seed, then f of 42",
        r#"
store reseed as random_seed
store seeded as reseed of 42
store digest as hash_password of "hunter2"
"#,
    );
}

#[test]
fn a_crypto_builtin_called_through_an_alias_is_seen() {
    assert_flagged(
        "store h as hash_password, then h of …",
        r#"
store seeded as random_seed of 42
store hasher as hash_password
store digest as hasher of "hunter2"
"#,
    );
}

#[test]
fn a_program_variable_named_random_seed_is_not_the_builtin() {
    // Negative control: WFL allows `store random_seed as 5`. After that the
    // name is the program's own number, so mentioning it seeds nothing.
    let program = parse(
        r#"
store random_seed as 5
display random_seed
store digest as hash_password of "hunter2"
"#,
    );
    let diagnostics = Analyzer::new().check_insecure_rng_seeding(&program, 0);
    assert!(
        diagnostics.is_empty(),
        "the program's own variable is not a seed call: {diagnostics:?}"
    );
}

#[test]
fn a_program_action_that_replaces_a_builtin_is_not_the_builtin() {
    // Negative control: WFL lets a program define its own action under some
    // builtin names (EXPLICIT_CALL_BUILTIN_FUNCTIONS, e.g. `session_cookie`).
    // A bare mention then refers to that action, so no session code runs.
    let program = parse(
        r#"
define action called session_cookie with parameters name:
    give back "cookie for " with name
end action
store make_cookie as session_cookie
store seeded as random_seed of 42
"#,
    );
    let diagnostics = Analyzer::new().check_insecure_rng_seeding(&program, 0);
    assert!(
        diagnostics.is_empty(),
        "a mention of the program's own action is not a session builtin: {diagnostics:?}"
    );
}

// ---------------------------------------------------------------------------
// Boundary: general-purpose hashes stay excluded (deliberate design, see
// SECURITY_SENSITIVE_BUILTINS), even in the newly walked constructs.
// ---------------------------------------------------------------------------

#[test]
fn a_checksum_in_a_newly_walked_construct_is_still_not_crypto_or_auth() {
    let program = parse(
        r#"
store seeded as random_seed of 42
create map manifest:
    checksum is sha256 of "file contents"
end map
"#,
    );
    let diagnostics = Analyzer::new().check_insecure_rng_seeding(&program, 0);
    assert!(
        diagnostics.is_empty(),
        "sha256 is a checksum, not crypto/auth, for this rule: {diagnostics:?}"
    );
}

// ---------------------------------------------------------------------------
// 2. Every file that runs is held to the rule (real binary, real files).
// ---------------------------------------------------------------------------

/// An auth helper file that seeds the RNG and hashes a password: the exact
/// combination the rule forbids.
const INSECURE_AUTH_FILE: &str = r#"store seeded as random_seed of 42
store digest as hash_password of "hunter2"
display "INSECURE FILE RAN"
"#;

/// The same file with the crypto removed: seeding is legal on its own.
const SEED_ONLY_FILE: &str = r#"store seeded as random_seed of 42
store roll as random_int of 1 and 6
display "SEED-ONLY FILE RAN"
"#;

/// Write `helper.wfl` and `main.wfl` into a fresh directory and run `main.wfl`
/// with the real binary. Returns (combined stdout+stderr, exit code).
fn run_main_with_helper(main_source: &str, helper_source: &str) -> (String, Option<i32>) {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("helper.wfl"), helper_source).unwrap();
    let main_path = dir.path().join("main.wfl");
    fs::write(&main_path, main_source).unwrap();
    let output = Command::new(wfl_exe())
        .arg(&main_path)
        .current_dir(dir.path())
        .output()
        .expect("failed to run wfl");
    (
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
        output.status.code(),
    )
}

/// The rejected helper must not run, the main program must stop at the load
/// site, and the message must name both the rule and the offending file.
fn assert_loaded_file_rejected(how: &str, main_source: &str) {
    let (out, code) = run_main_with_helper(main_source, INSECURE_AUTH_FILE);
    assert_ne!(code, Some(0), "{how}: the run must fail\n{out}");
    assert!(
        out.contains(
            "random_seed must not be used in authentication, session, or cryptographic code"
        ),
        "{how}: the error must state the rule\n{out}"
    );
    assert!(
        out.contains("helper.wfl"),
        "{how}: the error must name the offending file\n{out}"
    );
    assert!(
        !out.contains("INSECURE FILE RAN"),
        "{how}: no statement of the rejected file may execute\n{out}"
    );
    assert!(
        !out.contains("MAIN CONTINUED"),
        "{how}: the main program must not continue past the load\n{out}"
    );
}

#[test]
fn an_included_file_that_seeds_and_hashes_is_rejected() {
    assert_loaded_file_rejected(
        "include from",
        "include from \"helper.wfl\"\ndisplay \"MAIN CONTINUED\"\n",
    );
}

#[test]
fn a_loaded_module_that_seeds_and_hashes_is_rejected() {
    assert_loaded_file_rejected(
        "load module from",
        "load module from \"helper.wfl\"\ndisplay \"MAIN CONTINUED\"\n",
    );
}

#[test]
fn an_executed_file_that_seeds_and_hashes_is_rejected() {
    assert_loaded_file_rejected(
        "execute file at",
        "execute file at \"helper.wfl\"\ndisplay \"MAIN CONTINUED\"\n",
    );
}

#[test]
fn a_seed_only_helper_still_runs_through_every_loader() {
    // Negative control: the rule is about the combination, so a helper that
    // only seeds (a reproducible simulation) must keep working.
    for (how, main_source) in [
        (
            "include from",
            "include from \"helper.wfl\"\ndisplay \"MAIN CONTINUED\"\n",
        ),
        (
            "load module from",
            "load module from \"helper.wfl\"\ndisplay \"MAIN CONTINUED\"\n",
        ),
        (
            "execute file at",
            "execute file at \"helper.wfl\"\ndisplay \"MAIN CONTINUED\"\n",
        ),
    ] {
        let (out, code) = run_main_with_helper(main_source, SEED_ONLY_FILE);
        assert_eq!(code, Some(0), "{how}: a seed-only helper must run\n{out}");
        assert!(out.contains("SEED-ONLY FILE RAN"), "{how}\n{out}");
        assert!(out.contains("MAIN CONTINUED"), "{how}\n{out}");
    }
}
