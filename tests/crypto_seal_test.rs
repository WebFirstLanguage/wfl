// TDD tests for the `seal` / `unseal` AEAD builtins (issue #665).
//
// R3 (crypto/secrets): the point of an AEAD is that it fails *closed*. Most of
// these are therefore negative tests — a wrong key, a flipped byte, a truncated
// blob, or a mismatched context must all be rejected, and rejected the same way,
// so `unseal` never becomes an oracle that distinguishes one failure from another.

use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

async fn run_wfl(code: &str) -> Result<Interpreter, String> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let ast = parser.parse().map_err(|e| format!("Parse error: {e:?}"))?;

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&ast)
        .await
        .map_err(|e| format!("Runtime error: {e:?}"))?;
    Ok(interpreter)
}

fn get_global(interpreter: &Interpreter, name: &str) -> Value {
    interpreter
        .global_env()
        .borrow()
        .get(name)
        .unwrap_or_else(|| panic!("Variable '{name}' not found"))
}

fn expect_text(value: &Value) -> String {
    match value {
        Value::Text(t) => t.to_string(),
        other => panic!("Expected text, got {other:?}"),
    }
}

/// A valid 32-byte key as 64 hex characters — the shape `secure_random_bytes of 32` returns.
const KEY: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
const OTHER_KEY: &str = "ffeeddccbbaa99887766554433221100ffeeddccbbaa99887766554433221100";

// ---------------------------------------------------------------------------
// The happy path from the issue: mint a key, seal a secret, get it back.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn seal_then_unseal_round_trips() {
    let code = format!(
        r#"
store key as "{KEY}"
store secret as "sk-live-abc123-my-provider-token"
store sealed as seal of secret and key
store plain as unseal of sealed and key
"#
    );
    let interpreter = run_wfl(&code).await.expect("program should run");
    assert_eq!(
        expect_text(&get_global(&interpreter, "plain")),
        "sk-live-abc123-my-provider-token"
    );
}

#[tokio::test]
async fn sealed_output_does_not_leak_the_plaintext() {
    let code = format!(
        r#"
store key as "{KEY}"
store sealed as seal of "correct-horse-battery-staple" and key
"#
    );
    let interpreter = run_wfl(&code).await.expect("program should run");
    let sealed = expect_text(&get_global(&interpreter, "sealed"));
    assert!(
        !sealed.contains("correct-horse"),
        "ciphertext must not contain the plaintext: {sealed}"
    );
    assert!(
        sealed.starts_with("wflseal1:"),
        "sealed output should be self-describing and versioned, got: {sealed}"
    );
}

#[tokio::test]
async fn a_key_from_secure_random_bytes_works_end_to_end() {
    // The exact flow issue #665 describes: `secure_random_bytes of 32` mints a
    // key, and that key must be directly usable for sealing.
    let code = r#"
store project_key as secure_random_bytes of 32
store sealed as seal of "project secret" and project_key
store plain as unseal of sealed and project_key
"#;
    let interpreter = run_wfl(code).await.expect("program should run");
    assert_eq!(
        expect_text(&get_global(&interpreter, "plain")),
        "project secret"
    );
}

#[tokio::test]
async fn sealing_the_same_plaintext_twice_gives_different_ciphertexts() {
    // Nonce freshness. If these ever match, the nonce is being reused.
    let code = format!(
        r#"
store key as "{KEY}"
store a as seal of "same message" and key
store b as seal of "same message" and key
"#
    );
    let interpreter = run_wfl(&code).await.expect("program should run");
    let a = expect_text(&get_global(&interpreter, "a"));
    let b = expect_text(&get_global(&interpreter, "b"));
    assert_ne!(
        a, b,
        "each seal must use a fresh nonce, so identical plaintexts differ"
    );
}

#[tokio::test]
async fn empty_and_unicode_plaintexts_round_trip() {
    let code = format!(
        r#"
store key as "{KEY}"
store empty_sealed as seal of "" and key
store empty_plain as unseal of empty_sealed and key
store uni_sealed as seal of "こんにちは 🌍 café" and key
store uni_plain as unseal of uni_sealed and key
"#
    );
    let interpreter = run_wfl(&code).await.expect("program should run");
    assert_eq!(expect_text(&get_global(&interpreter, "empty_plain")), "");
    assert_eq!(
        expect_text(&get_global(&interpreter, "uni_plain")),
        "こんにちは 🌍 café"
    );
}

// ---------------------------------------------------------------------------
// Fails closed
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unseal_with_the_wrong_key_fails() {
    let code = format!(
        r#"
store key as "{KEY}"
store other as "{OTHER_KEY}"
store sealed as seal of "top secret" and key
store plain as unseal of sealed and other
"#
    );
    let err = run_wfl(&code)
        .await
        .err()
        .expect("unsealing with the wrong key must fail, not return garbage");
    assert!(
        err.to_lowercase().contains("unseal"),
        "error should name the operation, got: {err}"
    );
}

/// Seal a message in one program run and hand the blob back to Rust, so the
/// tamper tests can mutate exact byte positions instead of doing string surgery
/// in WFL.
async fn seal_once(plaintext: &str) -> String {
    let code = format!(
        r#"
store key as "{KEY}"
store sealed as seal of "{plaintext}" and key
"#
    );
    let interpreter = run_wfl(&code).await.expect("sealing should succeed");
    expect_text(&get_global(&interpreter, "sealed"))
}

/// Try to unseal a literal blob, returning the error.
async fn expect_unseal_failure(blob: &str, why: &str) -> String {
    let code = format!(
        r#"
store key as "{KEY}"
store plain as unseal of "{blob}" and key
"#
    );
    match run_wfl(&code).await {
        Ok(_) => panic!("{why}"),
        Err(e) => e,
    }
}

/// Flip one hex digit at `index` within the hex body of a sealed blob.
fn flip_hex_digit(sealed: &str, index: usize) -> String {
    let (prefix, body) = sealed
        .split_once(':')
        .expect("sealed blob should be prefixed");
    let mut chars: Vec<char> = body.chars().collect();
    assert!(index < chars.len(), "index must be inside the blob body");
    // Map the digit to a definitely-different one.
    chars[index] = if chars[index] == '0' { '1' } else { '0' };
    format!("{prefix}:{}", chars.into_iter().collect::<String>())
}

#[tokio::test]
async fn tampered_ciphertext_fails() {
    let sealed = seal_once("top secret").await;
    // Position 60 is past the 48-hex-char (24-byte) nonce, so this lands in the
    // ciphertext body proper.
    let tampered = flip_hex_digit(&sealed, 60);
    assert_ne!(
        tampered, sealed,
        "the tamper helper must actually change it"
    );
    let err =
        expect_unseal_failure(&tampered, "a modified ciphertext must fail authentication").await;
    assert!(
        err.to_lowercase().contains("unseal"),
        "error should name the operation, got: {err}"
    );
}

#[tokio::test]
async fn tampered_nonce_fails() {
    let sealed = seal_once("top secret").await;
    // Position 0 is inside the nonce.
    let tampered = flip_hex_digit(&sealed, 0);
    expect_unseal_failure(&tampered, "a modified nonce must fail authentication").await;
}

#[tokio::test]
async fn tampered_tag_fails() {
    let sealed = seal_once("top secret").await;
    let body_len = sealed.split_once(':').unwrap().1.len();
    // The last 32 hex chars are the 16-byte Poly1305 tag.
    let tampered = flip_hex_digit(&sealed, body_len - 1);
    expect_unseal_failure(&tampered, "a modified tag must fail authentication").await;
}

#[tokio::test]
async fn truncated_ciphertext_fails() {
    let sealed = seal_once("top secret").await;
    let truncated = &sealed[..sealed.len() - 8];
    expect_unseal_failure(
        truncated,
        "a truncated blob must fail rather than partially decrypt",
    )
    .await;
}

#[tokio::test]
async fn blob_with_the_wrong_version_prefix_fails() {
    let sealed = seal_once("top secret").await;
    let body = sealed.split_once(':').unwrap().1;
    let relabelled = format!("wflseal9:{body}");
    let err = expect_unseal_failure(
        &relabelled,
        "an unknown format version must be rejected, not guessed at",
    )
    .await;
    assert!(
        err.to_lowercase().contains("unseal"),
        "error should name the operation, got: {err}"
    );
}

#[tokio::test]
async fn unseal_rejects_input_that_is_not_a_sealed_blob() {
    let code = format!(
        r#"
store key as "{KEY}"
store plain as unseal of "just some text a user typed" and key
"#
    );
    let err = run_wfl(&code)
        .await
        .err()
        .expect("unseal must reject input that was never sealed");
    assert!(
        err.to_lowercase().contains("unseal"),
        "error should name the operation, got: {err}"
    );
}

#[tokio::test]
async fn seal_rejects_a_key_of_the_wrong_length() {
    let code = r#"
store sealed as seal of "secret" and "tooshort"
"#;
    let err = run_wfl(code)
        .await
        .err()
        .expect("a short key must be rejected outright");
    assert!(
        err.contains("secure_random_bytes"),
        "the error should tell the user how to make a valid key, got: {err}"
    );
}

#[tokio::test]
async fn seal_rejects_a_non_hex_key() {
    let code = r#"
store sealed as seal of "secret" and "zzzz02030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
"#;
    let err = run_wfl(code)
        .await
        .err()
        .expect("a key that is the right length but not hex must be rejected");
    assert!(
        err.contains("secure_random_bytes"),
        "the error should tell the user how to make a valid key, got: {err}"
    );
}

// ---------------------------------------------------------------------------
// Associated data — binds a ciphertext to its context.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn associated_data_round_trips() {
    let code = format!(
        r#"
store key as "{KEY}"
store sealed as seal of "provider token" and key and "project:acme/api_key"
store plain as unseal of sealed and key and "project:acme/api_key"
"#
    );
    let interpreter = run_wfl(&code).await.expect("program should run");
    assert_eq!(
        expect_text(&get_global(&interpreter, "plain")),
        "provider token"
    );
}

#[tokio::test]
async fn mismatched_associated_data_fails() {
    // The whole point: a ciphertext sealed for one context must not unseal in another.
    let code = format!(
        r#"
store key as "{KEY}"
store sealed as seal of "provider token" and key and "project:acme/api_key"
store plain as unseal of sealed and key and "project:evil/api_key"
"#
    );
    run_wfl(&code)
        .await
        .err()
        .expect("a ciphertext must not unseal under a different context");
}

#[tokio::test]
async fn context_is_required_on_unseal_when_it_was_used_on_seal() {
    let code = format!(
        r#"
store key as "{KEY}"
store sealed as seal of "provider token" and key and "project:acme/api_key"
store plain as unseal of sealed and key
"#
    );
    run_wfl(&code)
        .await
        .err()
        .expect("dropping the context on unseal must fail, not silently succeed");
}

/// A key or sealed value is untrusted text — it can come from a config file, a
/// database row, or an HTTP request. Every malformed input must produce a
/// catchable error, never a panic that takes down the process (and any server
/// it is hosting) with it.
///
/// Multi-byte UTF-8 is the case that escaped: decoding hex by slicing the string
/// at fixed two-byte offsets lands mid-character and panics on a `str` boundary
/// check rather than failing closed like every other bad input.
#[tokio::test]
async fn a_non_ascii_key_is_refused_rather_than_panicking() {
    // "€a" is 4 bytes, so an even-length check passes, but offset 2 is inside
    // the 3-byte '€'.
    let code = r#"
store sealed as seal of "secret" and "€a"
"#;
    run_wfl(code)
        .await
        .err()
        .expect("a non-ASCII key must be reported as a bad key, not panic");
}

#[tokio::test]
async fn a_non_ascii_sealed_value_is_refused_rather_than_panicking() {
    let code = format!(
        r#"
store key as "{KEY}"
store plain as unseal of "wflseal1:€a" and key
"#
    );
    run_wfl(&code)
        .await
        .err()
        .expect("a non-ASCII sealed value must be reported, not panic");
}

/// Hex is ASCII by definition, so a full-width or accented character anywhere in
/// an otherwise well-formed value must be rejected too.
#[tokio::test]
async fn a_multibyte_character_inside_a_full_length_key_is_refused() {
    // 62 ASCII hex characters plus a 2-byte 'é' — 64 bytes, 63 characters.
    let key: String = "a".repeat(62) + "é";
    let code = format!(
        r#"
store sealed as seal of "secret" and "{key}"
"#
    );
    run_wfl(&code)
        .await
        .err()
        .expect("a multi-byte character inside a key must be reported, not panic");
}
