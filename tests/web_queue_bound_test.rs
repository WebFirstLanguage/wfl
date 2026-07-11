//! Phase 0 (PR-0c) — bounded request queue + in-flight admission.
//!
//! The transport→interpreter request queue is bounded, and requests are admitted
//! against an in-flight semaphore *before* their bodies are buffered, so a flood
//! sheds with 503 rather than growing memory without bound. These tests cover:
//!   - the `web_server_request_queue_bound` config key (default, override,
//!     zero/garbage rejection) via the public `load_config` API, and
//!   - the runtime shed decision: `overloaded_response()` is a well-formed 503,
//!     and both a full bounded channel and a drained semaphore reject
//!     deterministically (the two admission gates in the warp handler).

use std::fs;
use tokio::sync::{Semaphore, mpsc};
use wfl::config::load_config;
use wfl::interpreter::overloaded_response;

const DEFAULT_BOUND: usize = 256;

/// Write a `.wflcfg` with the given body into a fresh temp dir and load it.
fn load_with_cfg(body: &str) -> wfl::config::WflConfig {
    let dir = tempfile::tempdir().expect("create temp dir");
    fs::write(dir.path().join(".wflcfg"), body).expect("write .wflcfg");
    load_config(dir.path())
}

#[test]
fn default_queue_bound_is_256() {
    // No key present → the safe default applies.
    let cfg = load_with_cfg("# empty config\n");
    assert_eq!(cfg.web_server_request_queue_bound, DEFAULT_BOUND);
}

#[test]
fn valid_queue_bound_is_applied() {
    let cfg = load_with_cfg("web_server_request_queue_bound = 8\n");
    assert_eq!(cfg.web_server_request_queue_bound, 8);
}

#[test]
fn zero_queue_bound_is_rejected_and_default_kept() {
    // A zero bound would panic tokio's `mpsc::channel(0)` and could never accept
    // a request, so it must be rejected in favor of the default.
    let cfg = load_with_cfg("web_server_request_queue_bound = 0\n");
    assert_eq!(cfg.web_server_request_queue_bound, DEFAULT_BOUND);
}

#[test]
fn non_numeric_queue_bound_is_rejected_and_default_kept() {
    let cfg = load_with_cfg("web_server_request_queue_bound = lots\n");
    assert_eq!(cfg.web_server_request_queue_bound, DEFAULT_BOUND);
}

#[test]
fn large_queue_bound_is_applied() {
    let cfg = load_with_cfg("web_server_request_queue_bound = 4096\n");
    assert_eq!(cfg.web_server_request_queue_bound, 4096);
}

// --- runtime shed decision -------------------------------------------------

#[test]
fn overloaded_response_is_a_well_formed_503() {
    let resp = overloaded_response();
    assert_eq!(resp.status(), warp::http::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        resp.headers()
            .get("Content-Type")
            .and_then(|v| v.to_str().ok()),
        Some("text/plain; charset=utf-8")
    );
    // Content-Length matches the actual body byte count.
    let declared: usize = resp
        .headers()
        .get("Content-Length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .expect("Content-Length header present and numeric");
    assert_eq!(declared, resp.body().len());
    assert!(!resp.body().is_empty());
}

/// Queue gate: once the bounded channel is full, `try_send` reports `Full`
/// (which the warp handler maps to a 503) and never blocks or grows the queue.
#[tokio::test]
async fn full_queue_sheds_deterministically() {
    let bound = 4usize;
    let (tx, _rx) = mpsc::channel::<u32>(bound);

    for i in 0..bound {
        tx.try_send(i as u32)
            .expect("send within capacity succeeds");
    }
    assert_eq!(tx.max_capacity(), bound);

    match tx.try_send(999) {
        Err(mpsc::error::TrySendError::Full(v)) => assert_eq!(v, 999),
        other => panic!("expected Full over capacity, got {other:?}"),
    }

    assert_eq!(
        overloaded_response().status(),
        warp::http::StatusCode::SERVICE_UNAVAILABLE
    );
}

/// Admission gate: once every in-flight permit is held, `try_acquire_owned`
/// fails, which the warp filter turns into an `Overloaded` rejection → 503,
/// shedding the request before its body is buffered.
#[test]
fn full_inflight_semaphore_sheds_deterministically() {
    let bound = 4usize;
    let sem = std::sync::Arc::new(Semaphore::new(bound));

    // Hold every permit.
    let mut permits: Vec<_> = (0..bound)
        .map(|_| {
            sem.clone()
                .try_acquire_owned()
                .expect("permit within capacity")
        })
        .collect();
    assert_eq!(sem.available_permits(), 0);

    // The next admission attempt is refused deterministically.
    assert!(sem.clone().try_acquire_owned().is_err());

    // Releasing exactly one permit re-opens exactly one admission slot (the
    // other three stay held in `permits`).
    permits.pop();
    assert_eq!(sem.available_permits(), 1);
    assert!(sem.try_acquire_owned().is_ok());
}
