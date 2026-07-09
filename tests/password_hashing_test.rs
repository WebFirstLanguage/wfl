// TDD Tests for password-hashing builtins: Argon2id, bcrypt, scrypt, PBKDF2.
// WFL previously had only fast hashes (sha256/wflhash) which are unsuitable for
// storing passwords. These builtins add slow, salted, memory/CPU-hard password
// hashes that produce self-describing PHC/MCF strings and verify in constant time.

use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

/// Run WFL code and return the value stored in `result`.
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
        other => panic!("Expected bool result, got {:?}", other),
    }
}

// === Generic password hashing (Argon2id default) ===

#[tokio::test]
async fn test_hash_password_produces_argon2id_phc_string() {
    let hash = expect_text(
        run_wfl_code(r#"store result as hash_password of "correct horse battery staple""#).await,
    );
    assert!(
        hash.starts_with("$argon2id$"),
        "hash_password should default to Argon2id, got: {hash}"
    );
}

#[tokio::test]
async fn test_hash_password_is_salted_and_unique() {
    let h1 = expect_text(run_wfl_code(r#"store result as hash_password of "same password""#).await);
    let h2 = expect_text(run_wfl_code(r#"store result as hash_password of "same password""#).await);
    assert_ne!(
        h1, h2,
        "A random salt must make two hashes of the same password differ"
    );
}

#[tokio::test]
async fn test_verify_password_accepts_correct_password() {
    let code = r#"
        store stored as hash_password of "s3cret!"
        store result as verify_password of "s3cret!" and stored
    "#;
    assert!(
        expect_bool(run_wfl_code(code).await),
        "verify_password must accept the correct password"
    );
}

#[tokio::test]
async fn test_verify_password_rejects_wrong_password() {
    let code = r#"
        store stored as hash_password of "s3cret!"
        store result as verify_password of "wrong" and stored
    "#;
    assert!(
        !expect_bool(run_wfl_code(code).await),
        "verify_password must reject an incorrect password"
    );
}

#[tokio::test]
async fn test_verify_password_returns_false_on_garbage_hash() {
    let code = r#"store result as verify_password of "anything" and "not-a-valid-hash""#;
    assert!(
        !expect_bool(run_wfl_code(code).await),
        "A malformed stored hash must verify to false, not error"
    );
}

// === Argon2 ===

#[tokio::test]
async fn test_argon2_hash_and_verify_roundtrip() {
    let code = r#"
        store stored as argon2_hash of "pw-argon2"
        store result as argon2_verify of "pw-argon2" and stored
    "#;
    assert!(expect_bool(run_wfl_code(code).await));
}

#[tokio::test]
async fn test_argon2_hash_prefix() {
    let hash = expect_text(run_wfl_code(r#"store result as argon2_hash of "x""#).await);
    assert!(hash.starts_with("$argon2id$"), "got: {hash}");
}

#[tokio::test]
async fn test_argon2_verify_rejects_wrong() {
    let code = r#"
        store stored as argon2_hash of "right"
        store result as argon2_verify of "wrong" and stored
    "#;
    assert!(!expect_bool(run_wfl_code(code).await));
}

// === bcrypt ===

#[tokio::test]
async fn test_bcrypt_hash_and_verify_roundtrip() {
    let code = r#"
        store stored as bcrypt_hash of "pw-bcrypt"
        store result as bcrypt_verify of "pw-bcrypt" and stored
    "#;
    assert!(expect_bool(run_wfl_code(code).await));
}

#[tokio::test]
async fn test_bcrypt_hash_prefix() {
    let hash = expect_text(run_wfl_code(r#"store result as bcrypt_hash of "x""#).await);
    assert!(
        hash.starts_with("$2b$") || hash.starts_with("$2a$") || hash.starts_with("$2y$"),
        "bcrypt output should be an MCF string, got: {hash}"
    );
}

#[tokio::test]
async fn test_bcrypt_verify_rejects_wrong() {
    let code = r#"
        store stored as bcrypt_hash of "right"
        store result as bcrypt_verify of "wrong" and stored
    "#;
    assert!(!expect_bool(run_wfl_code(code).await));
}

// === scrypt ===

#[tokio::test]
async fn test_scrypt_hash_and_verify_roundtrip() {
    let code = r#"
        store stored as scrypt_hash of "pw-scrypt"
        store result as scrypt_verify of "pw-scrypt" and stored
    "#;
    assert!(expect_bool(run_wfl_code(code).await));
}

#[tokio::test]
async fn test_scrypt_hash_prefix() {
    let hash = expect_text(run_wfl_code(r#"store result as scrypt_hash of "x""#).await);
    assert!(hash.starts_with("$scrypt$"), "got: {hash}");
}

#[tokio::test]
async fn test_scrypt_verify_rejects_wrong() {
    let code = r#"
        store stored as scrypt_hash of "right"
        store result as scrypt_verify of "wrong" and stored
    "#;
    assert!(!expect_bool(run_wfl_code(code).await));
}

// === PBKDF2 ===

#[tokio::test]
async fn test_pbkdf2_hash_and_verify_roundtrip() {
    let code = r#"
        store stored as pbkdf2_hash of "pw-pbkdf2"
        store result as pbkdf2_verify of "pw-pbkdf2" and stored
    "#;
    assert!(expect_bool(run_wfl_code(code).await));
}

#[tokio::test]
async fn test_pbkdf2_hash_prefix() {
    let hash = expect_text(run_wfl_code(r#"store result as pbkdf2_hash of "x""#).await);
    assert!(hash.starts_with("$pbkdf2-sha256$"), "got: {hash}");
}

#[tokio::test]
async fn test_pbkdf2_verify_rejects_wrong() {
    let code = r#"
        store stored as pbkdf2_hash of "right"
        store result as pbkdf2_verify of "wrong" and stored
    "#;
    assert!(!expect_bool(run_wfl_code(code).await));
}

// === Cross-algorithm: verify_password auto-detects the algorithm from the hash ===

#[tokio::test]
async fn test_verify_password_detects_bcrypt() {
    let code = r#"
        store stored as bcrypt_hash of "cross"
        store result as verify_password of "cross" and stored
    "#;
    assert!(
        expect_bool(run_wfl_code(code).await),
        "verify_password should auto-detect and verify a bcrypt hash"
    );
}

#[tokio::test]
async fn test_verify_password_detects_scrypt() {
    let code = r#"
        store stored as scrypt_hash of "cross"
        store result as verify_password of "cross" and stored
    "#;
    assert!(expect_bool(run_wfl_code(code).await));
}

#[tokio::test]
async fn test_verify_password_detects_pbkdf2() {
    let code = r#"
        store stored as pbkdf2_hash of "cross"
        store result as verify_password of "cross" and stored
    "#;
    assert!(expect_bool(run_wfl_code(code).await));
}

#[tokio::test]
async fn test_verify_is_algorithm_specific() {
    // An scrypt verifier must not accept a bcrypt hash (wrong format -> false, no panic).
    let code = r#"
        store stored as bcrypt_hash of "cross"
        store result as scrypt_verify of "cross" and stored
    "#;
    assert!(
        !expect_bool(run_wfl_code(code).await),
        "scrypt_verify must reject a hash produced by a different algorithm"
    );
}
