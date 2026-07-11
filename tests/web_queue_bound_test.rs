//! Phase 0 (PR-0c) — bounded request-queue configuration.
//!
//! The transport→interpreter request queue is bounded so a flood of accepted
//! requests sheds with 503 rather than growing memory without bound. These
//! tests exercise the `web_server_request_queue_bound` config key through the
//! public `load_config` API: a valid override applies, and invalid values
//! (zero, non-numeric) are rejected while keeping the safe default.
//!
//! The runtime shed decision itself (`try_send` full → `overloaded_response`
//! 503) is covered by the in-crate `queue_bound_tests` unit tests in
//! `src/interpreter/mod.rs`, since `overloaded_response` is crate-internal.

use std::fs;
use wfl::config::load_config;

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
