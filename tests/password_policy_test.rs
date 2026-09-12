//! Password maintenance contracts through registered natives, async routing,
//! and interpreted WFL. This suite was run red before adding the implementation.

mod common;

use argon2::password_hash::PasswordHash;
use std::sync::Arc;
use wfl::interpreter::Interpreter;
use wfl::interpreter::error::RuntimeError;
use wfl::interpreter::value::Value;
use wfl::stdlib::crypto_async::route;

fn text(value: &str) -> Value {
    Value::Text(Arc::from(value))
}

fn call(name: &str, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let interpreter = Interpreter::new();
    let function = interpreter.global_env().borrow().get(name);
    match function {
        Some(Value::NativeFunction(_, function)) => function(args),
        other => panic!("{name} must be registered as a native, got {other:?}"),
    }
}

fn policy(memory: f64, iterations: f64, parallelism: f64) -> Value {
    call(
        "password_hash_policy",
        vec![
            Value::Number(memory),
            Value::Number(iterations),
            Value::Number(parallelism),
        ],
    )
    .expect("valid policy")
}

fn needs_rehash(stored: &str, policy: Value) -> Result<Value, RuntimeError> {
    call("password_needs_rehash", vec![text(stored), policy])
}

fn phc(cost: &str) -> String {
    // Metadata inspection does not verify the digest or need the password.
    format!(
        "$argon2id$v=19${cost}$c29tZXNhbHQxMjM0NTY3OA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    )
}

#[test]
fn policy_bounds_reject_fractional_nonfinite_small_large_and_excessive_work() {
    for values in [
        [19455.0, 2.0, 1.0],
        [262145.0, 2.0, 1.0],
        [19456.0, 1.0, 1.0],
        [19456.0, 11.0, 1.0],
        [19456.0, 2.0, 0.0],
        [19456.0, 2.0, 17.0],
        [19456.5, 2.0, 1.0],
        [f64::NAN, 2.0, 1.0],
        [f64::INFINITY, 2.0, 1.0],
        [262144.0, 10.0, 1.0],
    ] {
        assert!(
            call("password_hash_policy", values.map(Value::Number).to_vec()).is_err(),
            "accepted invalid policy {values:?}"
        );
    }
    assert!(matches!(policy(19456.0, 2.0, 1.0), Value::Object(_)));
    assert!(matches!(policy(262144.0, 4.0, 16.0), Value::Object(_)));
}

#[test]
fn policy_revalidated_after_mutation_and_never_leaks_password_in_errors() {
    let candidate = policy(19456.0, 2.0, 1.0);
    if let Value::Object(fields) = &candidate {
        fields
            .borrow_mut()
            .insert("memory_kib".into(), Value::Number(1.0));
    }
    let error = call(
        "hash_password_with_policy",
        vec![text("private-value"), candidate],
    )
    .expect_err("tampering must not bypass bounds");
    assert!(!error.message.contains("private-value"));
}

#[tokio::test]
async fn policy_hash_is_salted_argon2id_with_exact_costs_and_roundtrips() {
    let args = [text("maintain-me"), policy(19456.0, 2.0, 1.0)];
    let first = common::expect_text_result(
        route("hash_password_with_policy", &args)
            .expect("configured hashing must route off thread")
            .await
            .map_err(|error| error.to_string()),
    );
    let second = common::expect_text_result(
        call("hash_password_with_policy", args.to_vec()).map_err(|error| error.to_string()),
    );
    assert_ne!(first, second);
    let parsed = PasswordHash::new(&first).unwrap();
    assert_eq!(parsed.algorithm.as_str(), "argon2id");
    assert_eq!(parsed.version, Some(19));
    assert_eq!(parsed.params.get_decimal("m"), Some(19456));
    assert_eq!(parsed.params.get_decimal("t"), Some(2));
    assert_eq!(parsed.params.get_decimal("p"), Some(1));
    assert_eq!(parsed.hash.unwrap().len(), 32);
    assert!(matches!(
        call("verify_password", vec![text("maintain-me"), text(&first)]),
        Ok(Value::Bool(true))
    ));
    assert!(matches!(
        call("verify_password", vec![text("wrong"), text(&first)]),
        Ok(Value::Bool(false))
    ));
    assert!(matches!(
        needs_rehash(&first, args[1].clone()),
        Ok(Value::Bool(false))
    ));
}

#[test]
fn rehash_upgrades_weak_hashes_and_never_silently_lowers_other_costs() {
    let desired = policy(32768.0, 3.0, 1.0);
    assert!(matches!(
        needs_rehash(&phc("m=19456,t=2,p=1"), desired.clone()),
        Ok(Value::Bool(true))
    ));
    assert!(matches!(
        needs_rehash(&phc("m=32768,t=3,p=1"), desired.clone()),
        Ok(Value::Bool(false))
    ));
    assert!(matches!(
        needs_rehash(&phc("m=65536,t=4,p=2"), desired.clone()),
        Ok(Value::Bool(false))
    ));
    assert!(needs_rehash(&phc("m=65536,t=2,p=1"), desired).is_err());
}

#[test]
fn rehash_checks_algorithm_version_salt_and_output() {
    let desired = policy(19456.0, 2.0, 1.0);
    let stored = phc("m=19456,t=2,p=1");
    for old in [
        stored.replace("argon2id", "argon2i"),
        stored.replace("v=19", "v=16"),
        stored.replace("c29tZXNhbHQxMjM0NTY3OA", "c29tZXNhbHQ"),
        stored.replace(
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            "AAAAAAAAAAAAAAAAAAAAAA",
        ),
    ] {
        assert!(
            matches!(needs_rehash(&old, desired.clone()), Ok(Value::Bool(true))),
            "did not identify legacy parameters: {old}"
        );
    }
}

#[test]
fn rehash_rejects_missing_unknown_malformed_and_oversized_metadata() {
    let desired = policy(19456.0, 2.0, 1.0);
    for stored in [
        "garbage".into(),
        "$argon2id$v=19$m=19456,t=2,p=1".into(),
        phc("m=19456,t=2"),
        phc("m=1,t=2,p=1"),
        phc("m=19456,t=2,p=0"),
        phc("m=19456,t=2,p=1,unknown=2"),
        phc("m=19456,t=2,p=1").replace("v=19", "v=999"),
        "x".repeat(1025),
    ] {
        assert!(
            needs_rehash(&stored, desired.clone()).is_err(),
            "accepted malformed stored hash"
        );
    }
}

#[test]
fn rehash_migrates_supported_legacy_algorithms_without_running_them() {
    let desired = policy(19456.0, 2.0, 1.0);
    let bcrypt = "$2y$12$L6Bc/AlTQHyd9liGgGEZyOFLPHNgyxeEPfgYfBCVxJ7JIlwxyVU3u";
    let suffix = "$c29tZXNhbHQxMjM0NTY3OA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    for old in [
        bcrypt.to_string(),
        format!("$scrypt$ln=17,r=8,p=1{suffix}"),
        format!("$pbkdf2-sha256$i=600000,l=32{suffix}"),
        format!("$pbkdf2-sha512$i=600000,l=32{suffix}"),
    ] {
        assert!(matches!(
            needs_rehash(&old, desired.clone()),
            Ok(Value::Bool(true))
        ));
    }
    for malformed in [
        bcrypt.replace("$12$", "$03$"),
        bcrypt.replace("$12$", "$99$"),
        format!("$scrypt$ln=17,r=0,p=1{suffix}"),
        format!("$pbkdf2-sha256$i=0,l=32{suffix}"),
        format!("$pbkdf2-sha256$i=600000,l=64{suffix}"),
        format!("$scrypt$v=19$ln=17,r=8,p=1{suffix}"),
    ] {
        assert!(
            needs_rehash(&malformed, desired.clone()).is_err(),
            "accepted malformed legacy metadata"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn policy_hash_leaves_interpreter_thread_available_and_validates_input() {
    use std::sync::atomic::{AtomicBool, Ordering};
    let progressed = Arc::new(AtomicBool::new(false));
    let marker = progressed.clone();
    let sibling = tokio::spawn(async move {
        marker.store(true, Ordering::SeqCst);
    });
    let args = [text("password"), policy(19456.0, 2.0, 1.0)];
    route("hash_password_with_policy", &args)
        .expect("hash routes")
        .await
        .unwrap();
    assert!(progressed.load(Ordering::SeqCst));
    sibling.await.unwrap();

    let long_args = [text(&"x".repeat(4097)), args[1].clone()];
    assert!(
        route("hash_password_with_policy", &long_args)
            .unwrap()
            .await
            .is_err()
    );
    assert!(
        route("hash_password_with_policy", &[])
            .unwrap()
            .await
            .is_err()
    );
    assert!(route("password_needs_rehash", &args).is_none());
}

#[tokio::test]
async fn password_policy_is_available_in_wfl_and_default_hashing_stays_compatible() {
    assert!(common::expect_bool_result(
        common::run_wfl_code(
            r#"
        store policy as password_hash_policy of 19456 and 2 and 1
        store stored as hash_password_with_policy of "account password" and policy
        store valid as verify_password of "account password" and stored
        store outdated as password_needs_rehash of stored and policy
        store result as valid and not outdated
    "#
        )
        .await
    ));
    let original = call("hash_password", vec![text("compatible")]).unwrap();
    let original = common::expect_text(&original);
    assert!(matches!(
        needs_rehash(&original, policy(19456.0, 2.0, 1.0)),
        Ok(Value::Bool(false))
    ));
    assert!(
        call(
            "hash_password",
            vec![text("compatible"), policy(19456.0, 2.0, 1.0)]
        )
        .is_err()
    );
}
