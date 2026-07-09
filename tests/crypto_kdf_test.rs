// TDD tests for launch-blocking crypto builtins (section 13.1):
//   - pbkdf2_hmac_sha256 of password and salt and iterations and length
//   - constant_time_equals of a and b
//   - secure_random_bytes of n
//
// These move the auth hot-path (KDF), the timing-safe comparison, and CSPRNG
// byte generation into native Rust so WFL auth/session code has a correct,
// non-DoS-prone primitive instead of hand-rolling them in interpreted WFL.

use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Run WFL source and return the value stored in the `result` variable.
async fn run_wfl_code(code: &str) -> Result<Value, String> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let ast = parser
        .parse()
        .map_err(|e| format!("Parse error: {:?}", e))?;

    let mut interpreter = Interpreter::new();
    interpreter
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

fn expect_bool(result: Result<Value, String>) -> bool {
    match result {
        Ok(Value::Bool(b)) => b,
        other => panic!("Expected boolean result, got {:?}", other),
    }
}

// === PBKDF2-HMAC-SHA256 (well-known / RFC-style vectors) ===
// Vectors: P="password", S="salt". These are the canonical PBKDF2-HMAC-SHA256
// test vectors published alongside RFC 6070's PBKDF2-HMAC-SHA1 vectors.

#[tokio::test]
async fn test_pbkdf2_hmac_sha256_c1_dklen32() {
    let code = r#"
        store result as pbkdf2_hmac_sha256 of "password" and "salt" and 1 and 32
    "#;
    let dk = expect_text(run_wfl_code(code).await);
    assert_eq!(
        dk, "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b",
        "PBKDF2-HMAC-SHA256(password, salt, c=1, dkLen=32) must match the standard vector"
    );
}

#[tokio::test]
async fn test_pbkdf2_hmac_sha256_c2_dklen32() {
    let code = r#"
        store result as pbkdf2_hmac_sha256 of "password" and "salt" and 2 and 32
    "#;
    let dk = expect_text(run_wfl_code(code).await);
    assert_eq!(
        dk,
        "ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43"
    );
}

#[tokio::test]
async fn test_pbkdf2_hmac_sha256_c4096_dklen32() {
    let code = r#"
        store result as pbkdf2_hmac_sha256 of "password" and "salt" and 4096 and 32
    "#;
    let dk = expect_text(run_wfl_code(code).await);
    assert_eq!(
        dk,
        "c5e478d59288c841aa530db6845c4c8d962893a001ce4e11a4963873aa98134a"
    );
}

#[tokio::test]
async fn test_pbkdf2_hmac_sha256_long_vector_dklen40() {
    // P="passwordPASSWORDpassword", S="saltSALTsaltSALTsaltSALTsaltSALTsalt",
    // c=4096, dkLen=40.
    let code = r#"
        store result as pbkdf2_hmac_sha256 of "passwordPASSWORDpassword" and "saltSALTsaltSALTsaltSALTsaltSALTsalt" and 4096 and 40
    "#;
    let dk = expect_text(run_wfl_code(code).await);
    assert_eq!(
        dk,
        "348c89dbcbd32b2f32d814b8116e84cf2b17347ebc1800181c4e2a1fb8dd53e1c635518c7dac47e9"
    );
}

#[tokio::test]
async fn test_pbkdf2_hmac_sha256_output_length_matches_request() {
    // dkLen=16 bytes -> 32 hex chars
    let code = r#"
        store result as pbkdf2_hmac_sha256 of "pw" and "sa" and 1000 and 16
    "#;
    let dk = expect_text(run_wfl_code(code).await);
    assert_eq!(dk.len(), 32);
    assert!(
        dk.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
}

#[tokio::test]
async fn test_pbkdf2_hmac_sha256_rejects_zero_iterations() {
    let code = r#"
        store result as pbkdf2_hmac_sha256 of "pw" and "salt" and 0 and 32
    "#;
    assert!(run_wfl_code(code).await.is_err());
}

#[tokio::test]
async fn test_pbkdf2_hmac_sha256_rejects_zero_length() {
    let code = r#"
        store result as pbkdf2_hmac_sha256 of "pw" and "salt" and 1000 and 0
    "#;
    assert!(run_wfl_code(code).await.is_err());
}

// === constant_time_equals ===

#[tokio::test]
async fn test_constant_time_equals_equal_strings() {
    let code = r#"
        store result as constant_time_equals of "abc123" and "abc123"
    "#;
    assert!(expect_bool(run_wfl_code(code).await));
}

#[tokio::test]
async fn test_constant_time_equals_different_strings() {
    let code = r#"
        store result as constant_time_equals of "abc123" and "abc124"
    "#;
    assert!(!expect_bool(run_wfl_code(code).await));
}

#[tokio::test]
async fn test_constant_time_equals_different_lengths() {
    let code = r#"
        store result as constant_time_equals of "short" and "longer string"
    "#;
    assert!(!expect_bool(run_wfl_code(code).await));
}

#[tokio::test]
async fn test_constant_time_equals_empty_strings() {
    let code = r#"
        store result as constant_time_equals of "" and ""
    "#;
    assert!(expect_bool(run_wfl_code(code).await));
}

#[tokio::test]
async fn test_constant_time_equals_verifies_hmac_signature() {
    // The intended use: compare a computed MAC against a received one safely.
    let code = r#"
        store expected as hmac_sha256 of "payload" and "secret"
        store received as hmac_sha256 of "payload" and "secret"
        store result as constant_time_equals of expected and received
    "#;
    assert!(expect_bool(run_wfl_code(code).await));
}

// === secure_random_bytes ===

#[tokio::test]
async fn test_secure_random_bytes_length() {
    // 16 bytes -> 32 hex chars
    let code = r#"
        store result as secure_random_bytes of 16
    "#;
    let bytes = expect_text(run_wfl_code(code).await);
    assert_eq!(bytes.len(), 32);
    assert!(
        bytes
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
}

#[tokio::test]
async fn test_secure_random_bytes_are_unpredictable() {
    let a = expect_text(run_wfl_code("store result as secure_random_bytes of 32").await);
    let b = expect_text(run_wfl_code("store result as secure_random_bytes of 32").await);
    assert_ne!(
        a, b,
        "Two calls must (with overwhelming probability) differ"
    );
}

#[tokio::test]
async fn test_secure_random_bytes_rejects_zero() {
    assert!(
        run_wfl_code("store result as secure_random_bytes of 0")
            .await
            .is_err()
    );
}

#[tokio::test]
async fn test_secure_random_bytes_rejects_excessive() {
    assert!(
        run_wfl_code("store result as secure_random_bytes of 100000")
            .await
            .is_err()
    );
}
