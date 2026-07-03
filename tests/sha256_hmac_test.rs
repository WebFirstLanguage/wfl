// TDD Tests for standard SHA-256 and HMAC-SHA256 builtins (issue #558)
// Webhook verification (e.g. Stripe) requires standard HMAC-SHA256, not just WFLHASH.

use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Test helper to run WFL code and get the result from a variable
async fn run_wfl_code(code: &str) -> Result<Value, String> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let ast = parser
        .parse()
        .map_err(|e| format!("Parse error: {:?}", e))?;

    let mut interpreter = Interpreter::new();
    let _ = interpreter
        .interpret(&ast)
        .await
        .map_err(|e| format!("Runtime error: {:?}", e))?;

    if let Some(result_value) = interpreter.global_env().borrow().get("result") {
        Ok(result_value)
    } else {
        Err("Variable 'result' not found after execution".to_string())
    }
}

fn expect_text(result: Result<Value, String>) -> String {
    match result {
        Ok(Value::Text(t)) => t.to_string(),
        other => panic!("Expected text result, got {:?}", other),
    }
}

// === SHA-256 (FIPS 180-4 / well-known vectors) ===

#[tokio::test]
async fn test_sha256_empty_string() {
    let code = r#"
        store result as sha256 of ""
    "#;
    let hash = expect_text(run_wfl_code(code).await);
    assert_eq!(
        hash, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "SHA-256 of empty string must match the standard test vector"
    );
}

#[tokio::test]
async fn test_sha256_abc() {
    let code = r#"
        store result as sha256 of "abc"
    "#;
    let hash = expect_text(run_wfl_code(code).await);
    assert_eq!(
        hash,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[tokio::test]
async fn test_sha256_hello_world() {
    let code = r#"
        store result as sha256 of "hello world"
    "#;
    let hash = expect_text(run_wfl_code(code).await);
    assert_eq!(
        hash,
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}

#[tokio::test]
async fn test_sha256_output_is_lowercase_hex() {
    let code = r#"
        store result as sha256 of "WFL"
    "#;
    let hash = expect_text(run_wfl_code(code).await);
    assert_eq!(hash.len(), 64);
    assert!(
        hash.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "Digest must be lowercase hex"
    );
}

// === HMAC-SHA256 (RFC 2104 / well-known vectors) ===

#[tokio::test]
async fn test_hmac_sha256_quick_brown_fox() {
    // Well-known vector: key = "key", message = "The quick brown fox jumps over the lazy dog"
    let code = r#"
        store result as hmac_sha256 of "The quick brown fox jumps over the lazy dog" and "key"
    "#;
    let mac = expect_text(run_wfl_code(code).await);
    assert_eq!(
        mac, "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8",
        "HMAC-SHA256 must match the standard test vector (message first, key second)"
    );
}

#[tokio::test]
async fn test_hmac_sha256_empty_key_and_message() {
    let code = r#"
        store result as hmac_sha256 of "" and ""
    "#;
    let mac = expect_text(run_wfl_code(code).await);
    assert_eq!(
        mac,
        "b613679a0814d9ec772f95d778c35fc5ff1697c493715653c6c712144292c5ad"
    );
}

#[tokio::test]
async fn test_hmac_sha256_long_key_is_hashed() {
    // Keys longer than the block size (64 bytes) are hashed first per RFC 2104.
    // key = 80 * "a", message = "test"
    let code = r#"
        store result as hmac_sha256 of "test" and "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    "#;
    let mac = expect_text(run_wfl_code(code).await);
    assert_eq!(mac.len(), 64);
    // Deterministic: computing it twice gives the same answer
    let code2 = r#"
        store result as hmac_sha256 of "test" and "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    "#;
    let mac2 = expect_text(run_wfl_code(code2).await);
    assert_eq!(mac, mac2);
}

#[tokio::test]
async fn test_hmac_sha256_stripe_style_signed_payload() {
    // Stripe webhook verification computes HMAC-SHA256(secret, "timestamp.payload")
    let code = r#"
        store event_time as "1614556800"
        store payload as "{\"id\": \"evt_123\"}"
        store signed_payload as event_time with "." with payload
        store result as hmac_sha256 of signed_payload and "whsec_test_secret"
    "#;
    let mac = expect_text(run_wfl_code(code).await);
    assert_eq!(mac.len(), 64);
    assert!(mac.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn test_hmac_sha256_different_keys_differ() {
    let code1 = r#"
        store result as hmac_sha256 of "message" and "key1"
    "#;
    let code2 = r#"
        store result as hmac_sha256 of "message" and "key2"
    "#;
    let mac1 = expect_text(run_wfl_code(code1).await);
    let mac2 = expect_text(run_wfl_code(code2).await);
    assert_ne!(mac1, mac2, "Different keys must produce different MACs");
}
