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
//! The returned future is a boxed `Pin<Box<dyn Future>>` ([`RoutedFuture`]); it
//! is awaited from the interpreter future, which runs via `block_on` (never
//! `tokio::spawn`), so awaiting a `Send` `JoinHandle` from inside it is sound.

use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use crate::stdlib::crypto;
use crate::stdlib::helpers::{check_arg_count, expect_text};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use zeroize::Zeroizing;

/// Boxed future produced by [`route`]. A plain `std` type (rather than a
/// `futures_util` alias) so the public signature doesn't leak a third-party type
/// into the crate's API. It is awaited only on the interpreter thread.
type RoutedFuture = Pin<Box<dyn Future<Output = Result<Value, RuntimeError>>>>;

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
///
/// Public so the interpreter dispatch (and the `tests/crypto_async_test.rs`
/// integration tests) can drive it; not part of the stable language surface.
pub fn route(name: &str, args: &[Value]) -> Option<RoutedFuture> {
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
) -> RoutedFuture {
    // Extract on the interpreter thread; only the owned `String` crosses the
    // boundary below.
    let extracted = extract_password(func, args);
    Box::pin(async move {
        let password = extracted?;
        let hash = tokio::task::spawn_blocking(move || compute(func, password.as_str()))
            .await
            .map_err(|e| join_error(func, e))??;
        Ok(Value::Text(Arc::from(hash)))
    })
}

/// Two-argument verify builtins: `<func> of "password" and stored` → boolean.
fn verify_route(
    func: &'static str,
    args: &[Value],
    compute: fn(&str, &str) -> bool,
) -> RoutedFuture {
    let extracted = extract_password_and_stored(func, args);
    Box::pin(async move {
        let (password, stored) = extracted?;
        let ok = tokio::task::spawn_blocking(move || compute(password.as_str(), &stored))
            .await
            .map_err(|e| join_error(func, e))?;
        Ok(Value::Bool(ok))
    })
}

/// Raw PBKDF2-HMAC-SHA256 KDF: `pbkdf2_hmac_sha256 of password and salt and
/// iterations and length` → hex string.
fn pbkdf2_hmac_route(args: &[Value]) -> RoutedFuture {
    const FUNC: &str = "pbkdf2_hmac_sha256";
    let extracted = (|| {
        check_arg_count(FUNC, args, 4)?;
        let password = Zeroizing::new(expect_text(&args[0])?.to_string());
        let salt = expect_text(&args[1])?.to_string();
        let iterations = crypto::expect_count(FUNC, "iterations", &args[2])?;
        let length = crypto::expect_count(FUNC, "length", &args[3])? as usize;
        Ok::<_, RuntimeError>((password, salt, iterations, length))
    })();
    Box::pin(async move {
        let (password, salt, iterations, length) = extracted?;
        let hex = tokio::task::spawn_blocking(move || {
            crypto::pbkdf2_hmac_sha256_str(password.as_str(), &salt, iterations, length)
        })
        .await
        .map_err(|e| join_error(FUNC, e))??;
        Ok(Value::Text(Arc::from(hex)))
    })
}

/// Validate arity and pull out an owned password on the interpreter thread.
/// Kept as a plain (non-async) fn so extraction errors surface before any thread
/// hop. The password copy is wrapped in `Zeroizing` so it is wiped when the
/// blocking closure that owns it is dropped.
fn extract_password(func: &str, args: &[Value]) -> Result<Zeroizing<String>, RuntimeError> {
    check_arg_count(func, args, 1)?;
    Ok(Zeroizing::new(expect_text(&args[0])?.to_string()))
}

/// Like [`extract_password`] but also returns the stored hash. Only the password
/// is secret, so only it is zeroized; the stored hash is left as a plain `String`.
fn extract_password_and_stored(
    func: &str,
    args: &[Value],
) -> Result<(Zeroizing<String>, String), RuntimeError> {
    check_arg_count(func, args, 2)?;
    let password = Zeroizing::new(expect_text(&args[0])?.to_string());
    let stored = expect_text(&args[1])?.to_string();
    Ok((password, stored))
}

/// Map a `spawn_blocking` join failure (panic/cancel in the blocking task) to a
/// `RuntimeError`. Panics inside the crypto helpers are not expected, but a
/// join error must not be silently swallowed.
fn join_error(func: &str, e: tokio::task::JoinError) -> RuntimeError {
    RuntimeError::new(format!("{func}: crypto task failed: {e}"), 0, 0)
}
