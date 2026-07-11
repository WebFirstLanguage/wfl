//! Phase 0 (PR-0b) — off-thread crypto routing, exercised through the public
//! `crypto_async::route` seam. Lives under `tests/` per the repo convention;
//! round-trips go through routing (hash then verify) rather than reaching into
//! crate-internal helpers.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wfl::interpreter::value::Value;
use wfl::stdlib::crypto_async::route;

fn text(s: &str) -> Value {
    Value::Text(Arc::from(s))
}

fn as_text(v: Value) -> String {
    match v {
        Value::Text(t) => t.to_string(),
        other => panic!("expected text, got {other:?}"),
    }
}

/// Route a two-argument verify builtin and unwrap its boolean result.
async fn verify(name: &str, password: &str, stored: &str) -> bool {
    let out = route(name, &[text(password), text(stored)])
        .expect("verify routes")
        .await
        .expect("verify succeeds");
    match out {
        Value::Bool(b) => b,
        other => panic!("expected bool, got {other:?}"),
    }
}

/// Only the deliberately-slow crypto builtins route; everything else falls
/// through to the synchronous native. Guards against silent scope creep.
#[test]
fn route_map_covers_exactly_the_heavy_crypto_builtins() {
    let one = [text("x")];
    let two = [text("x"), text("y")];
    let four = [
        text("x"),
        text("y"),
        Value::Number(1.0),
        Value::Number(16.0),
    ];

    for name in [
        "argon2_hash",
        "scrypt_hash",
        "bcrypt_hash",
        "pbkdf2_hash",
        "hash_password",
    ] {
        assert!(route(name, &one).is_some(), "{name} should route");
    }
    for name in [
        "argon2_verify",
        "scrypt_verify",
        "bcrypt_verify",
        "pbkdf2_verify",
        "verify_password",
    ] {
        assert!(route(name, &two).is_some(), "{name} should route");
    }
    assert!(route("pbkdf2_hmac_sha256", &four).is_some());

    // Fast / non-crypto builtins must NOT route.
    for name in [
        "constant_time_equals",
        "secure_random_bytes",
        "sha256",
        "wflhash256",
        "print",
        "length",
    ] {
        assert!(route(name, &two).is_none(), "{name} must not route");
    }
}

/// The core Phase 0 property (plan U4): the heavy work runs off the interpreter
/// thread. On a single-threaded runtime, a concurrently-spawned ticker can only
/// make progress *during* the crypto `.await` if that work was offloaded via
/// `spawn_blocking`. Had the hash run inline, it would have monopolized the one
/// thread and the ticker would still read zero when the await returns. This is
/// deterministic and independent of core count.
#[tokio::test(flavor = "current_thread")]
async fn routed_crypto_frees_the_interpreter_thread() {
    let counter = Arc::new(AtomicUsize::new(0));
    let ticker_counter = counter.clone();
    let ticker = tokio::spawn(async move {
        for _ in 0..5 {
            tokio::task::yield_now().await;
            ticker_counter.fetch_add(1, Ordering::SeqCst);
        }
    });

    let out = route("argon2_hash", &[text("a-password-to-hash")])
        .expect("argon2_hash routes")
        .await
        .expect("hash succeeds");

    // Read the ticker's progress the instant the await returns, before we yield
    // again — so a passing result can only mean the runtime polled the ticker
    // while the blocking pool computed the hash.
    let progressed = counter.load(Ordering::SeqCst);
    ticker.await.expect("ticker joins");

    assert!(
        progressed > 0,
        "ticker never advanced during the crypto await — work was not offloaded"
    );
    assert!(matches!(out, Value::Text(_)));
}

/// The routed hash still produces a valid, verifiable credential, and
/// `verify_password` auto-detects the algorithm.
#[tokio::test]
async fn routed_argon2_hash_roundtrips() {
    let hash = as_text(
        route("argon2_hash", &[text("correct horse")])
            .unwrap()
            .await
            .unwrap(),
    );
    assert!(hash.starts_with("$argon2"));
    assert!(verify("argon2_verify", "correct horse", &hash).await);
    assert!(!verify("argon2_verify", "wrong password", &hash).await);
    assert!(verify("verify_password", "correct horse", &hash).await);
}

/// bcrypt uses a distinct MCF (`$2b$`) format, exercising a different path.
#[tokio::test]
async fn routed_bcrypt_hash_roundtrips() {
    let hash = as_text(
        route("bcrypt_hash", &[text("hunter2")])
            .unwrap()
            .await
            .unwrap(),
    );
    assert!(hash.starts_with("$2"));
    assert!(verify("bcrypt_verify", "hunter2", &hash).await);
    assert!(!verify("bcrypt_verify", "nope", &hash).await);
}

/// The routed KDF is deterministic (no salt) and correctly shaped: two routed
/// calls with identical inputs produce identical 32-byte (64-hex) output.
#[tokio::test]
async fn routed_pbkdf2_is_deterministic() {
    let args = [
        text("password"),
        text("salt"),
        Value::Number(4096.0),
        Value::Number(32.0),
    ];
    let a = as_text(route("pbkdf2_hmac_sha256", &args).unwrap().await.unwrap());
    let b = as_text(route("pbkdf2_hmac_sha256", &args).unwrap().await.unwrap());
    assert_eq!(a, b);
    assert_eq!(a.len(), 64, "32 bytes rendered as lowercase hex");
}

/// Argument errors still surface (as the routed future's error), so routing
/// does not mask misuse.
#[tokio::test]
async fn routed_argerror_is_reported() {
    // argon2_hash expects exactly one argument.
    let err = route("argon2_hash", &[text("a"), text("b")])
        .unwrap()
        .await
        .unwrap_err();
    assert!(err.message.contains("argon2_hash"));
}
