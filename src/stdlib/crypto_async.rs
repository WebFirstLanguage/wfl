//! Off-thread routing for CPU-heavy crypto builtins (WFL concurrency Phase 0,
//! PR-0b).
//!
//! The password-hashing / KDF builtins (`argon2_hash`, `bcrypt_hash`,
//! `scrypt_hash`, `pbkdf2_hash`, `pbkdf2_hmac_sha256`, `hash_password`, and the
//! verify counterparts) are *deliberately* slow — Argon2 is memory-hard, PBKDF2
//! runs 600k rounds. Run inline on the single interpreter thread, one login-like
//! call stalls the whole process (a cooperative-scheduling DoS).
//!
//! This module hops that work onto Tokio's blocking pool with
//! [`tokio::task::spawn_blocking`], following the libuv / Node pattern. The
//! interpreter core stays `!Send`: arguments are extracted into owned plain data
//! (`String`, `u64`, `usize`) on the interpreter thread *before* the hop, only
//! that plain data crosses the boundary, and the resulting `Value` is built back
//! on the interpreter thread after the `.await`. No `Rc`/`RefCell`/`Value`/
//! `Environment` ever crosses threads.
//!
//! The returned future is a `!Send` [`LocalBoxFuture`]; it is awaited from the
//! interpreter future, which runs via `block_on` (never `tokio::spawn`), so
//! awaiting a `Send` `JoinHandle` from inside it is sound.

use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use crate::stdlib::crypto;
use crate::stdlib::helpers::{check_arg_count, expect_text};
use futures_util::future::{FutureExt, LocalBoxFuture};
use std::sync::Arc;

/// Route a CPU-heavy crypto builtin onto the blocking pool.
///
/// Returns `None` for any name that is not a heavy crypto builtin — the caller
/// then invokes the plain synchronous native as before. For the routed set
/// (see below), returns a future that performs argument validation on the
/// interpreter thread and the heavy computation on `spawn_blocking`.
///
/// Routed set (11): `argon2_hash`, `argon2_verify`, `scrypt_hash`,
/// `scrypt_verify`, `bcrypt_hash`, `bcrypt_verify`, `pbkdf2_hash`,
/// `pbkdf2_verify`, `hash_password`, `verify_password`, `pbkdf2_hmac_sha256`.
///
/// `constant_time_equals` and `secure_random_bytes` are intentionally *not*
/// routed: they are fast and routing them would only add a scheduling hop.
pub(crate) fn route(
    name: &str,
    args: &[Value],
) -> Option<LocalBoxFuture<'static, Result<Value, RuntimeError>>> {
    let fut = match name {
        "argon2_hash" => hash_route("argon2_hash", args, crypto::argon2_hash_str),
        "scrypt_hash" => hash_route("scrypt_hash", args, crypto::scrypt_hash_str),
        "pbkdf2_hash" => hash_route("pbkdf2_hash", args, crypto::pbkdf2_hash_str),
        "bcrypt_hash" => hash_route("bcrypt_hash", args, crypto::bcrypt_hash_str),
        // hash_password is Argon2id under the hood (see native_hash_password).
        "hash_password" => hash_route("hash_password", args, crypto::argon2_hash_str),
        "argon2_verify" => verify_route("argon2_verify", args, crypto::argon2_verify_str),
        "scrypt_verify" => verify_route("scrypt_verify", args, crypto::scrypt_verify_str),
        "pbkdf2_verify" => verify_route("pbkdf2_verify", args, crypto::pbkdf2_verify_str),
        "bcrypt_verify" => verify_route("bcrypt_verify", args, crypto::bcrypt_verify_str),
        "verify_password" => verify_route("verify_password", args, crypto::verify_any_password),
        "pbkdf2_hmac_sha256" => pbkdf2_hmac_route(args),
        _ => return None,
    };
    Some(fut)
}

/// One-argument password-hash builtins: `<func> of "password"` → PHC/MCF string.
/// `compute` takes `(func_name, password)` to match the crypto helper shape.
fn hash_route(
    func: &'static str,
    args: &[Value],
    compute: fn(&str, &str) -> Result<String, RuntimeError>,
) -> LocalBoxFuture<'static, Result<Value, RuntimeError>> {
    // Extract on the interpreter thread; only the owned `String` crosses the
    // boundary below.
    let extracted = extract_password(func, args);
    async move {
        let password = extracted?;
        let hash = tokio::task::spawn_blocking(move || compute(func, &password))
            .await
            .map_err(|e| join_error(func, e))??;
        Ok(Value::Text(Arc::from(hash)))
    }
    .boxed_local()
}

/// Two-argument verify builtins: `<func> of "password" and stored` → boolean.
fn verify_route(
    func: &'static str,
    args: &[Value],
    compute: fn(&str, &str) -> bool,
) -> LocalBoxFuture<'static, Result<Value, RuntimeError>> {
    let extracted = extract_password_and_stored(func, args);
    async move {
        let (password, stored) = extracted?;
        let ok = tokio::task::spawn_blocking(move || compute(&password, &stored))
            .await
            .map_err(|e| join_error(func, e))?;
        Ok(Value::Bool(ok))
    }
    .boxed_local()
}

/// Raw PBKDF2-HMAC-SHA256 KDF: `pbkdf2_hmac_sha256 of password and salt and
/// iterations and length` → hex string.
fn pbkdf2_hmac_route(args: &[Value]) -> LocalBoxFuture<'static, Result<Value, RuntimeError>> {
    const FUNC: &str = "pbkdf2_hmac_sha256";
    let extracted = (|| {
        check_arg_count(FUNC, args, 4)?;
        let password = expect_text(&args[0])?.to_string();
        let salt = expect_text(&args[1])?.to_string();
        let iterations = crypto::expect_count(FUNC, "iterations", &args[2])?;
        let length = crypto::expect_count(FUNC, "length", &args[3])? as usize;
        Ok::<_, RuntimeError>((password, salt, iterations, length))
    })();
    async move {
        let (password, salt, iterations, length) = extracted?;
        let hex = tokio::task::spawn_blocking(move || {
            crypto::pbkdf2_hmac_sha256_str(&password, &salt, iterations, length)
        })
        .await
        .map_err(|e| join_error(FUNC, e))??;
        Ok(Value::Text(Arc::from(hex)))
    }
    .boxed_local()
}

/// Validate arity and pull out an owned password `String` on the interpreter
/// thread. Kept as a plain (non-async) fn so extraction errors surface before
/// any thread hop.
fn extract_password(func: &str, args: &[Value]) -> Result<String, RuntimeError> {
    check_arg_count(func, args, 1)?;
    Ok(expect_text(&args[0])?.to_string())
}

fn extract_password_and_stored(
    func: &str,
    args: &[Value],
) -> Result<(String, String), RuntimeError> {
    check_arg_count(func, args, 2)?;
    let password = expect_text(&args[0])?.to_string();
    let stored = expect_text(&args[1])?.to_string();
    Ok((password, stored))
}

/// Map a `spawn_blocking` join failure (panic/cancel in the blocking task) to a
/// `RuntimeError`. Panics inside the crypto helpers are not expected, but a
/// join error must not be silently swallowed.
fn join_error(func: &str, e: tokio::task::JoinError) -> RuntimeError {
    RuntimeError::new(format!("{func}: crypto task failed: {e}"), 0, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn text(s: &str) -> Value {
        Value::Text(Arc::from(s))
    }

    /// Only the deliberately-slow crypto builtins route; everything else falls
    /// through to the synchronous native. Guards against silent scope creep in
    /// the routed set.
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

    /// The core Phase 0 property (plan U4): the heavy work runs off the
    /// interpreter thread. On a single-threaded runtime, a concurrently-spawned
    /// ticker can only make progress *during* the crypto `.await` if that work
    /// was offloaded via `spawn_blocking`. Had the hash run inline, it would
    /// have monopolized the one thread and the ticker would still read zero when
    /// the await returns. This is deterministic and independent of core count.
    #[tokio::test(flavor = "current_thread")]
    async fn routed_crypto_frees_the_interpreter_thread() {
        let counter = std::sync::Arc::new(AtomicUsize::new(0));
        let ticker_counter = counter.clone();
        let ticker = tokio::spawn(async move {
            for _ in 0..5 {
                tokio::task::yield_now().await;
                ticker_counter.fetch_add(1, Ordering::SeqCst);
            }
        });

        let args = [text("a-password-to-hash")];
        let out = route("argon2_hash", &args)
            .expect("argon2_hash routes")
            .await
            .expect("hash succeeds");

        // Read the ticker's progress the instant the await returns, before we
        // yield again — so a passing result can only mean the runtime polled the
        // ticker while the blocking pool computed the hash.
        let progressed = counter.load(Ordering::SeqCst);
        ticker.await.expect("ticker joins");

        assert!(
            progressed > 0,
            "ticker never advanced during the crypto await — work was not offloaded"
        );
        assert!(matches!(out, Value::Text(_)));
    }

    /// The routed hash still produces a valid, verifiable credential.
    #[tokio::test]
    async fn routed_argon2_hash_roundtrips() {
        let out = route("argon2_hash", &[text("correct horse")])
            .unwrap()
            .await
            .unwrap();
        let hash = match out {
            Value::Text(h) => h,
            other => panic!("expected text, got {other:?}"),
        };
        assert!(crypto::argon2_verify_str("correct horse", &hash));
        assert!(!crypto::argon2_verify_str("wrong password", &hash));
    }

    /// bcrypt uses a distinct MCF (`$2b$`) format, exercising a different helper.
    #[tokio::test]
    async fn routed_bcrypt_hash_roundtrips() {
        let out = route("bcrypt_hash", &[text("hunter2")])
            .unwrap()
            .await
            .unwrap();
        let hash = match out {
            Value::Text(h) => h,
            other => panic!("expected text, got {other:?}"),
        };
        assert!(hash.starts_with("$2"));
        assert!(crypto::bcrypt_verify_str("hunter2", &hash));
        assert!(!crypto::bcrypt_verify_str("nope", &hash));
    }

    /// The routed KDF output is byte-identical to the direct plain-data helper —
    /// the thread hop changes nothing about the result (deterministic: no salt).
    #[tokio::test]
    async fn routed_pbkdf2_matches_direct() {
        let args = [
            text("password"),
            text("salt"),
            Value::Number(4096.0),
            Value::Number(32.0),
        ];
        let out = route("pbkdf2_hmac_sha256", &args).unwrap().await.unwrap();
        let routed = match out {
            Value::Text(h) => h.to_string(),
            other => panic!("expected text, got {other:?}"),
        };
        let direct = crypto::pbkdf2_hmac_sha256_str("password", "salt", 4096, 32).unwrap();
        assert_eq!(routed, direct);
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
}
