#![deny(clippy::await_holding_refcell_ref)]

// Concurrency Phase 0 (PR-0a) — panic-strategy gate.
//
// The runtime relies on `std::panic::catch_unwind` to contain a panicking
// request handler so its siblings survive (concurrency Phase 1). That fault
// isolation is UNSOUND under `panic = "abort"`: an abort tears the whole
// process down before the catch can run, turning it into a phantom control.
// Fail the build rather than ship that. `cfg(panic = ...)` reflects the panic
// strategy actually compiled into this crate; Cargo force-unwinds test/bench
// harnesses, so this never trips `cargo test`.
#[cfg(panic = "abort")]
compile_error!(
    "WFL requires panic = \"unwind\"; the runtime's catch_unwind-based request-handler \
     fault isolation (concurrency Phase 1) is unsound under panic = \"abort\". \
     Remove the panic = \"abort\" override."
);

// Global allocator for dhat heap profiling
#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

pub mod analyzer;
pub mod builtins;
pub mod config;
pub mod debug_report;
pub mod diagnostics;
pub mod env_dump;
pub mod exec;
pub mod fixer;
pub mod interpreter;
pub mod lexer;
pub mod linter;
pub mod logging;
pub mod parser;
pub mod pattern;
pub mod repl;
pub mod stdlib;
pub mod transpiler;
pub mod typechecker;
pub mod version;
pub mod wfl_config;

use crate::config::WflConfig;
use once_cell::sync::Lazy;
use std::path::Path;
use std::sync::RwLock;

// Global configuration accessible throughout the codebase
pub static CONFIG: Lazy<RwLock<Option<WflConfig>>> = Lazy::new(|| RwLock::new(None));

// Initialize both loggers - regular and execution logger
pub fn init_loggers(log_path: &Path, script_dir: &Path) {
    // Load the configuration
    let config = config::load_config_with_global(script_dir);

    // Initialize the main logger
    if config.logging_enabled
        && let Err(e) = logging::init_logger(config.log_level, log_path)
    {
        eprintln!("Failed to initialize logger: {e}");
    }

    // Initialize the execution logger if enabled
    if config.execution_logging
        && let Err(e) = logging::init_execution_logger(&config, log_path)
    {
        eprintln!("Failed to initialize execution logger: {e}");
    }

    // Store config globally
    if let Ok(mut global_config) = CONFIG.write() {
        *global_config = Some(config);
    }
}

/// Install a process-level rustls [`CryptoProvider`](rustls::crypto::CryptoProvider).
///
/// This crate links two rustls crypto providers into one `rustls 0.23`:
/// aws-lc-rs (via reqwest) and ring (via sqlx). rustls refuses to auto-select a
/// default when more than one is present, so any code path that builds a TLS
/// config from the ambient default panics at runtime. Every binary that links
/// this crate — and any embedder — should call this once at startup, before
/// creating a TLS client or connection pool.
///
/// The guarantee is only that *a* process-level provider is installed once this
/// returns, not that it is ring specifically: `install_default` fails if a
/// default is already set (by an earlier call to this function *or* by any other
/// code), and that `Err` is intentionally ignored so the call is always safe and
/// idempotent. Callers must not assume which provider ends up installed. If you
/// need a specific provider, install it yourself before calling any WFL code.
pub fn init_rustls_crypto_provider() {
    // Ignore the `Err`: it means a default is already installed, which satisfies
    // the "some provider is set" contract regardless of which one it is.
    let _ = rustls::crypto::ring::default_provider().install_default();
}

pub use interpreter::{Interpreter, TestFailure, TestResults};

/// Dedicated interpreter thread stack size (1 GiB), reserved virtually and
/// committed lazily. See [`run_with_interpreter_stack`].
pub const INTERPRETER_STACK_SIZE: usize = 1024 * 1024 * 1024;

/// Run `work` on a dedicated large-stack thread ([`INTERPRETER_STACK_SIZE`]) and
/// return its result.
///
/// WFL's async tree-walking interpreter recurses through several frames per WFL
/// call, so deep WFL recursion is very stack-heavy — a debug build overflows an
/// ordinary 8 MiB thread stack near depth ~40, long before the shared
/// [`exec::budget::ExecutionBudget`]'s `max_call_depth` (default 1000) can turn
/// runaway recursion into a clean, catchable `ResourceLimit` error. The CLI runs
/// its whole runtime on such a thread; **a library embedder that drives
/// [`Interpreter`] directly should do the same** — otherwise a deep WFL program
/// can crash the host process with a native stack overflow that no depth limit
/// can catch, because the limit only protects a stack large enough to reach it.
///
/// Wrap the runtime + interpreter call in this helper, e.g.:
///
/// ```no_run
/// let exit = wfl::run_with_interpreter_stack(|| {
///     let rt = tokio::runtime::Builder::new_current_thread()
///         .enable_all()
///         .build()
///         .expect("runtime");
///     rt.block_on(async {
///         // build an Interpreter and call `interpret(...)` here
///     })
/// })
/// .expect("reserve interpreter stack");
/// # let _ = exit;
/// ```
///
/// Returns `Err` only if the large-stack thread cannot be spawned (e.g. a tight
/// `RLIMIT_AS` or a 32-bit target); a caller may then fall back to running the
/// work on the current stack (accepting that deep recursion may hit the OS limit
/// before `max_call_depth`), or set a conservatively low `max_call_depth`. A
/// panic inside `work` is propagated to the caller.
pub fn run_with_interpreter_stack<F, T>(work: F) -> std::io::Result<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let handle = std::thread::Builder::new()
        .name("wfl-interpreter".to_string())
        .stack_size(INTERPRETER_STACK_SIZE)
        .spawn(work)?;
    Ok(handle
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload)))
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
