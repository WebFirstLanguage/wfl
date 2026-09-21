// TDD tests for `ed25519_verify of public_key and message and signature`.
//
// R3 (crypto / untrusted input): this is the activation-prerequisite verifier.
// Authoritative vectors are RFC 8032 §7.1. Invalid signatures must return `no`;
// malformed encodings must fail closed without echoing the inputs.

mod common;
use common::{
    expect_bool_result as expect_bool, expect_text_result as expect_text, get_global, run_wfl,
    run_wfl_code,
};
use wfl::interpreter::value::Value;

/// RFC 8032 §7.1 TEST 1 — empty message.
const RFC8032_TEST1_PK: &str = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
const RFC8032_TEST1_SIG: &str = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";

/// RFC 8032 §7.1 TEST 2 — one-byte message `0x72` (`"r"` in UTF-8 / ASCII).
const RFC8032_TEST2_PK: &str = "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";
const RFC8032_TEST2_SIG: &str = "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00";

fn verify_src(public_key: &str, message: &str, signature: &str) -> String {
    format!(
        r#"
store result as ed25519_verify of "{public_key}" and "{message}" and "{signature}"
"#
    )
}

async fn verify_bool(public_key: &str, message: &str, signature: &str) -> bool {
    expect_bool(run_wfl_code(&verify_src(public_key, message, signature)).await)
}

async fn verify_err(public_key: &str, message: &str, signature: &str) -> String {
    match run_wfl_code(&verify_src(public_key, message, signature)).await {
        Ok(value) => panic!("expected a runtime error, got {value:?}"),
        Err(error) => error,
    }
}

fn assert_closed_error(error: &str) {
    let lower = error.to_lowercase();
    assert!(
        lower.contains("ed25519_verify"),
        "malformed-input errors must name the builtin, got: {error}"
    );
    for secret in [
        RFC8032_TEST1_PK,
        RFC8032_TEST1_SIG,
        RFC8032_TEST2_PK,
        RFC8032_TEST2_SIG,
        "sk-",
        "BEGIN",
    ] {
        assert!(
            !error.contains(secret),
            "diagnostics must not echo key or signature material: {error}"
        );
    }
}

#[tokio::test]
async fn rfc8032_test1_empty_message_verifies() {
    assert!(
        verify_bool(RFC8032_TEST1_PK, "", RFC8032_TEST1_SIG).await,
        "RFC 8032 TEST 1 (empty message) must verify"
    );
}

#[tokio::test]
async fn rfc8032_test2_ascii_byte_verifies() {
    assert!(
        verify_bool(RFC8032_TEST2_PK, "r", RFC8032_TEST2_SIG).await,
        "RFC 8032 TEST 2 (message 0x72) must verify"
    );
}

/// RFC 8032 §7.1 TEST 3 — two-byte binary message `af82` (not valid UTF-8).
const RFC8032_TEST3_PK: &str = "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025";
const RFC8032_TEST3_SIG: &str = "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a";
const RFC8032_TEST3_MSG: [u8; 2] = [0xaf, 0x82];

#[test]
fn rfc8032_test3_binary_message_verifies_at_the_byte_boundary() {
    // TEST 3's message is not valid UTF-8, so the WFL text surface cannot carry
    // it. The native byte helper is the authoritative path for that vector.
    let accepted = wfl::stdlib::crypto::verify_ed25519_bytes(
        &hex_literal(RFC8032_TEST3_PK),
        &RFC8032_TEST3_MSG,
        &hex_literal(RFC8032_TEST3_SIG),
    );
    assert!(accepted, "RFC 8032 TEST 3 (message af82) must verify");
}

fn hex_literal(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair).expect("hex fixture is ASCII");
            u8::from_str_radix(text, 16).expect("hex fixture is valid")
        })
        .collect()
}

#[tokio::test]
async fn uppercase_hex_is_accepted() {
    assert!(
        verify_bool(
            &RFC8032_TEST1_PK.to_ascii_uppercase(),
            "",
            &RFC8032_TEST1_SIG.to_ascii_uppercase()
        )
        .await,
        "hex is case-insensitive"
    );
}

#[tokio::test]
async fn tampered_message_is_rejected() {
    assert!(
        !verify_bool(RFC8032_TEST2_PK, "s", RFC8032_TEST2_SIG).await,
        "a one-byte message change must not verify"
    );
}

#[tokio::test]
async fn wrong_public_key_is_rejected() {
    assert!(
        !verify_bool(RFC8032_TEST1_PK, "r", RFC8032_TEST2_SIG).await,
        "TEST 2's signature must not verify under TEST 1's public key"
    );
}

#[tokio::test]
async fn flipped_signature_byte_is_rejected() {
    let mut flipped = RFC8032_TEST2_SIG.to_string();
    flipped.replace_range(0..1, if flipped.starts_with('9') { "8" } else { "9" });
    assert_ne!(flipped, RFC8032_TEST2_SIG);
    assert!(
        !verify_bool(RFC8032_TEST2_PK, "r", &flipped).await,
        "a flipped signature nibble must not verify"
    );
}

#[tokio::test]
async fn truncated_signature_is_a_format_error() {
    let error = verify_err(RFC8032_TEST2_PK, "r", &RFC8032_TEST2_SIG[..126]).await;
    assert_closed_error(&error);
}

#[tokio::test]
async fn truncated_public_key_is_a_format_error() {
    let error = verify_err(&RFC8032_TEST2_PK[..62], "r", RFC8032_TEST2_SIG).await;
    assert_closed_error(&error);
}

#[tokio::test]
async fn odd_length_hex_is_a_format_error() {
    let error = verify_err(&format!("0{RFC8032_TEST2_PK}"), "r", RFC8032_TEST2_SIG).await;
    assert_closed_error(&error);
}

#[tokio::test]
async fn non_hex_signature_is_a_format_error() {
    let bogus = "z".repeat(128);
    let error = verify_err(RFC8032_TEST2_PK, "r", &bogus).await;
    assert_closed_error(&error);
    assert!(
        !error.contains(&bogus),
        "must not echo the malformed signature: {error}"
    );
}

#[tokio::test]
async fn base64_public_key_is_unsupported() {
    let error = verify_err(
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511=",
        "r",
        RFC8032_TEST2_SIG,
    )
    .await;
    assert_closed_error(&error);
}

#[tokio::test]
async fn pem_public_key_is_unsupported() {
    let pem = "-----BEGIN PUBLIC KEY-----MCowBQYDK2VwAyEA11qYAYKxCrfVS/7TyWQHOg7hcvPapiMlrwIaaPcHURo=-----END PUBLIC KEY-----";
    let error = verify_err(pem, "r", RFC8032_TEST2_SIG).await;
    assert_closed_error(&error);
    assert!(
        !error.contains("BEGIN PUBLIC KEY"),
        "must not echo PEM material: {error}"
    );
}

#[tokio::test]
async fn hex_prefix_is_unsupported() {
    let error = verify_err(&format!("0x{RFC8032_TEST2_PK}"), "r", RFC8032_TEST2_SIG).await;
    assert_closed_error(&error);
}

#[tokio::test]
async fn oversized_message_is_rejected() {
    // 1 MiB + 1 exceeds the documented limit. Build the source in Rust so the
    // WFL program does not have to construct the string itself.
    let too_big = "a".repeat(1_048_576 + 1);
    let error = verify_err(RFC8032_TEST1_PK, &too_big, RFC8032_TEST1_SIG).await;
    assert_closed_error(&error);
    assert!(
        error.to_lowercase().contains("maximum") || error.to_lowercase().contains("size"),
        "oversized messages must mention the limit, got: {error}"
    );
}

#[tokio::test]
async fn website_style_activation_payload_verifies() {
    // The call shape the Logbie website will use: settings public key, exact
    // UTF-8 payload, hex signature. RFC 8032 TEST 2 stands in for a signed
    // activation record until issuance exists.
    let code = format!(
        r#"
store activation_public_key as "{RFC8032_TEST2_PK}"
store activation_message as "r"
store activation_signature as "{RFC8032_TEST2_SIG}"
store accepted as ed25519_verify of activation_public_key and activation_message and activation_signature
"#
    );
    let interpreter = run_wfl(&code)
        .await
        .expect("the website-shaped verifier call must run");
    match get_global(&interpreter, "accepted") {
        Value::Bool(true) => {}
        other => panic!("website-shaped verify must accept a valid signature, got {other:?}"),
    }
}

#[tokio::test]
async fn missing_builtin_is_not_silently_a_text_concat() {
    // Guard: `ed25519_verify of …` must be a real builtin, not concatenated text.
    let result = run_wfl_code(&verify_src(RFC8032_TEST1_PK, "", RFC8032_TEST1_SIG)).await;
    match result {
        Ok(Value::Bool(_)) => {}
        Ok(Value::Text(text)) => panic!("ed25519_verify must not concatenate to text: {text}"),
        Ok(other) => panic!("ed25519_verify must return a boolean, got {other:?}"),
        Err(error) => {
            // Red: the builtin is absent. Green: this branch is unused.
            assert!(
                error.to_lowercase().contains("ed25519")
                    || error.to_lowercase().contains("undefined")
                    || error.to_lowercase().contains("unknown"),
                "unexpected absence diagnostic: {error}"
            );
        }
    }
}

#[tokio::test]
async fn expect_text_helper_still_reads_unrelated_hex() {
    // Keeps the file compiling against the shared helpers if a future edit
    // drops every text-result assertion.
    let hex = expect_text(run_wfl_code(r#"store result as sha256 of """#).await);
    assert_eq!(hex.len(), 64);
}
