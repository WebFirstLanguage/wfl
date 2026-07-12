//! A single [`ExecutionBudget`] shared across parsing, evaluation, pattern
//! matching, web handling, and module loading.
//!
//! # Why one object
//!
//! Before this, the runtime enforced a dozen unrelated ceilings from a dozen
//! unrelated places: the interpreter's wall-clock timeout (`max_duration` +
//! `op_count`), the pattern VM's `MAX_STEPS`, the web server's
//! `web_server_max_body_size` and `web_server_request_queue_bound`, an
//! `execute file` depth constant, and several things that were simply
//! *unbounded* (recursion depth, WebSocket queues and connection counts, HTTP
//! response size, source-file size). Each was a separate audit finding.
//!
//! `ExecutionBudget` replaces that scatter with one coherent object. It carries
//! the immutable [`BudgetLimits`] for a run plus the small amount of shared
//! mutable accounting (operations charged, live pending requests, live
//! WebSocket connections) needed to enforce them. Every dimension the task
//! enumerates lives here:
//!
//! * **Deadline and cancellation** — [`ExecutionBudget::charge_operation`] /
//!   [`ExecutionBudget::check_deadline`] / [`ExecutionBudget::cancel`].
//! * **Remaining interpreter operations** — the operation counter and its
//!   optional ceiling.
//! * **Recursion and import depth** — [`ExecutionBudget::check_call_depth`],
//!   [`ExecutionBudget::check_import_depth`],
//!   [`ExecutionBudget::check_execute_file_depth`].
//! * **Pattern transitions and active states** —
//!   [`ExecutionBudget::check_pattern_steps`] /
//!   [`ExecutionBudget::check_pattern_states`].
//! * **Source, body, and response bytes** —
//!   [`ExecutionBudget::check_source_bytes`],
//!   [`ExecutionBudget::check_request_body_bytes`],
//!   [`ExecutionBudget::check_response_bytes`].
//! * **Pending HTTP requests** — [`ExecutionBudget::max_pending_requests`].
//! * **WebSocket queue and connection limits** —
//!   [`ExecutionBudget::ws_queue_bound`] /
//!   [`ExecutionBudget::try_acquire_ws_connection`].
//!
//! # Thread-safety
//!
//! The interpreter core stays `!Send` (`Rc`/`RefCell`), but the budget must
//! also be readable from the multi-threaded web transport (warp accept tasks,
//! per-connection WebSocket tasks). It is therefore `Send + Sync`: every
//! mutable field is an atomic, so an `Arc<ExecutionBudget>` can be cloned into a
//! transport task without any `Rc`/`RefCell` crossing a thread boundary. Sharing
//! a small atomic-only object this way does not violate the "no `Rc`→`Arc`
//! rewrite of the interpreter" rule — it is exactly how `Arc<WflConfig>` is
//! already shared.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crate::config::WflConfig;

/// How often (in charged operations) the wall-clock deadline and cancellation
/// flag are sampled on the interpreter hot path. Reading the clock on every
/// operation is a measurable cost in tight loops, so those checks run only on
/// this stride — preserving the interpreter's historic `op_count & 1023`
/// throttle. Must stay a power of two so `index & (STRIDE - 1)` is exact.
const CLOCK_SAMPLE_STRIDE: u64 = 1024;

/// Immutable per-run ceilings. Construct via [`BudgetLimits::from_config`] (maps
/// the existing `.wflcfg` keys) or [`BudgetLimits::default`].
///
/// The two "opt-in" fields ([`BudgetLimits::max_duration`] and
/// [`BudgetLimits::max_operations`]) are `Option`; `None` means "no limit".
/// Every other field is a concrete ceiling that is always enforced — its
/// default is chosen generously so existing programs never trip it while
/// runaway behaviour still gets a clean error instead of a crash or unbounded
/// growth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetLimits {
    /// Wall-clock deadline for non-`main loop` execution. `None` disables it.
    /// Mapped from `.wflcfg` `timeout_seconds`.
    pub max_duration: Option<Duration>,
    /// Hard ceiling on charged interpreter operations. `None` (the default)
    /// disables it, matching the historic behaviour where the operation counter
    /// only throttled clock reads. Mapped from `.wflcfg` `max_operations`
    /// (`0` = unlimited).
    pub max_operations: Option<u64>,
    /// Maximum WFL call/recursion depth. Mapped from `.wflcfg` `max_call_depth`.
    pub max_call_depth: usize,
    /// Maximum nested `load module` / `include` depth. Mapped from `.wflcfg`
    /// `max_import_depth`.
    pub max_import_depth: usize,
    /// Maximum `execute file` nesting depth. Kept small because each level
    /// re-enters the whole interpreter recursively. Mapped from `.wflcfg`
    /// `max_execute_file_depth`.
    pub max_execute_file_depth: usize,
    /// Maximum pattern-VM transitions per match attempt (ReDoS guard). Mapped
    /// from `.wflcfg` `max_pattern_steps`.
    pub max_pattern_steps: usize,
    /// Maximum simultaneously-active pattern-VM states per match attempt.
    /// Mapped from `.wflcfg` `max_pattern_states`.
    pub max_pattern_states: usize,
    /// Maximum WFL source-file size in bytes. Mapped from `.wflcfg`
    /// `max_source_size`.
    pub max_source_bytes: usize,
    /// Maximum accepted HTTP request body size in bytes. Mapped from `.wflcfg`
    /// `web_server_max_body_size`.
    pub max_request_body_bytes: usize,
    /// Maximum HTTP response body size in bytes. Mapped from `.wflcfg`
    /// `web_server_max_response_size`.
    pub max_response_bytes: usize,
    /// Maximum accepted-but-unhandled HTTP requests held in the transport
    /// queue. Mapped from `.wflcfg` `web_server_request_queue_bound`.
    pub max_pending_requests: usize,
    /// Maximum queued frames/events per WebSocket channel. Mapped from
    /// `.wflcfg` `web_socket_queue_bound`.
    pub max_ws_queue: usize,
    /// Maximum simultaneous live WebSocket connections. Mapped from `.wflcfg`
    /// `web_socket_max_connections`.
    pub max_ws_connections: usize,
}

impl Default for BudgetLimits {
    fn default() -> Self {
        Self {
            // Matches the historic interpreter default (`timeout_seconds: 60`).
            max_duration: Some(Duration::from_secs(60)),
            // Off by default: no operation ceiling existed before.
            max_operations: None,
            // No runtime recursion guard existed before (only a debug assert at
            // 10_000). WFL runs on a dedicated 1 GiB stack (see main.rs), and
            // 1_000 frames fit comfortably within it in both debug and release,
            // so runaway recursion gets a clean error well before the stack
            // overflows — while clearing any realistic program's depth.
            max_call_depth: 1_000,
            // Module/include nesting was unbounded before; 64 is far beyond any
            // real dependency chain.
            max_import_depth: 64,
            // Preserves the previous `MAX_EXECUTE_FILE_DEPTH` constant exactly.
            max_execute_file_depth: 4,
            // Per-instruction charging (not per-wave), so this is far above the
            // old per-wave `MAX_STEPS` (100_000) while still catching runaway
            // (e.g. ReDoS) matches that blow past millions of transitions.
            max_pattern_steps: 5_000_000,
            // Active-state fan-out was unbounded before; 10_000 is generous for
            // any non-pathological pattern.
            max_pattern_states: 10_000,
            // Source size was unchecked before; 64 MiB clears any real program.
            max_source_bytes: 64 * 1024 * 1024,
            // Preserves the previous `web_server_max_body_size` default (1 MiB).
            max_request_body_bytes: 1_048_576,
            // Response size was unchecked before; 64 MiB clears any real payload.
            max_response_bytes: 64 * 1024 * 1024,
            // Preserves the previous `web_server_request_queue_bound` default.
            max_pending_requests: 256,
            // WebSocket channels were unbounded before; 1_024 clears normal use.
            max_ws_queue: 1_024,
            // Connection count was uncapped before; 1_024 clears normal use.
            max_ws_connections: 1_024,
        }
    }
}

impl BudgetLimits {
    /// Derive the limits from a loaded [`WflConfig`], mapping the existing
    /// `.wflcfg` keys (`timeout_seconds`, `web_server_max_body_size`,
    /// `web_server_request_queue_bound`) and the budget-specific keys onto their
    /// budget fields. Any field the config does not carry keeps its default.
    pub fn from_config(config: &WflConfig) -> Self {
        Self {
            max_duration: Some(Duration::from_secs(config.timeout_seconds)),
            max_operations: config.max_operations,
            max_call_depth: config.max_call_depth,
            max_import_depth: config.max_import_depth,
            max_execute_file_depth: config.max_execute_file_depth,
            max_pattern_steps: config.max_pattern_steps,
            max_pattern_states: config.max_pattern_states,
            max_source_bytes: config.max_source_size,
            max_request_body_bytes: config.web_server_max_body_size,
            max_response_bytes: config.web_server_max_response_size,
            max_pending_requests: config.web_server_request_queue_bound.max(1),
            max_ws_queue: config.web_socket_queue_bound.max(1),
            max_ws_connections: config.web_socket_max_connections.max(1),
        }
    }

    /// Limits with every ceiling effectively disabled. Used by standalone
    /// pattern helpers and tests that must not be constrained by a run budget.
    /// Pattern limits keep the historic `MAX_STEPS`/state defaults so a
    /// bare [`crate::pattern::PatternVM::new`] still resists ReDoS.
    pub fn unlimited() -> Self {
        Self {
            max_duration: None,
            max_operations: None,
            max_call_depth: usize::MAX,
            max_import_depth: usize::MAX,
            max_execute_file_depth: usize::MAX,
            max_pattern_steps: 5_000_000,
            max_pattern_states: 10_000,
            max_source_bytes: usize::MAX,
            max_request_body_bytes: usize::MAX,
            max_response_bytes: usize::MAX,
            max_pending_requests: usize::MAX,
            max_ws_queue: usize::MAX,
            max_ws_connections: usize::MAX,
        }
    }
}

/// The specific ceiling a run tripped. Callers map this onto their own error
/// type (the interpreter to `RuntimeError`, the pattern VM to `PatternError`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetExceeded {
    /// The wall-clock deadline elapsed.
    Deadline { limit_secs: u64 },
    /// The run was cancelled cooperatively via [`ExecutionBudget::cancel`].
    Cancelled,
    /// The interpreter-operation ceiling was reached.
    Operations { limit: u64 },
    /// Call/recursion depth would exceed the ceiling.
    CallDepth { limit: usize },
    /// `load module` / `include` nesting would exceed the ceiling.
    ImportDepth { limit: usize },
    /// `execute file` nesting would exceed the ceiling.
    ExecuteFileDepth { limit: usize },
    /// A pattern match exceeded its transition ceiling.
    PatternSteps { limit: usize },
    /// A pattern match exceeded its active-state ceiling.
    PatternStates { limit: usize },
    /// A source file exceeded the byte ceiling.
    SourceBytes { limit: usize, actual: usize },
    /// An HTTP request body exceeded the byte ceiling.
    RequestBodyBytes { limit: usize, actual: usize },
    /// An HTTP response body exceeded the byte ceiling.
    ResponseBytes { limit: usize, actual: usize },
    /// The in-flight/pending HTTP request ceiling was reached.
    PendingRequests { limit: usize },
    /// The WebSocket connection ceiling was reached.
    WsConnections { limit: usize },
}

impl BudgetExceeded {
    /// A human-facing, Elm-style message describing the breach.
    pub fn message(&self) -> String {
        match self {
            // Preserves the historic interpreter timeout wording verbatim so
            // existing timeout diagnostics/tests keep matching.
            BudgetExceeded::Deadline { limit_secs } => {
                format!("Execution exceeded timeout ({limit_secs}s)")
            }
            BudgetExceeded::Cancelled => "Execution was cancelled".to_string(),
            BudgetExceeded::Operations { limit } => {
                format!("Execution exceeded the operation budget ({limit} operations)")
            }
            BudgetExceeded::CallDepth { limit } => {
                format!("Maximum call depth ({limit}) exceeded - possible infinite recursion")
            }
            BudgetExceeded::ImportDepth { limit } => {
                format!("Maximum import depth ({limit}) exceeded - possible circular imports")
            }
            BudgetExceeded::ExecuteFileDepth { limit } => format!(
                "Maximum execute file nesting depth ({limit}) exceeded - possible circular execution"
            ),
            BudgetExceeded::PatternSteps { limit } => {
                format!("Pattern execution step limit exceeded ({limit} steps)")
            }
            BudgetExceeded::PatternStates { limit } => {
                format!("Pattern active-state limit exceeded ({limit} states)")
            }
            BudgetExceeded::SourceBytes { limit, actual } => {
                format!("Source file too large: {actual} bytes (limit: {limit} bytes)")
            }
            BudgetExceeded::RequestBodyBytes { limit, actual } => {
                format!("Request body too large: {actual} bytes (limit: {limit} bytes)")
            }
            BudgetExceeded::ResponseBytes { limit, actual } => {
                format!("Response body too large: {actual} bytes (limit: {limit} bytes)")
            }
            BudgetExceeded::PendingRequests { limit } => {
                format!("Pending request limit reached ({limit} in flight)")
            }
            BudgetExceeded::WsConnections { limit } => {
                format!("WebSocket connection limit reached ({limit} connections)")
            }
        }
    }
}

impl std::fmt::Display for BudgetExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message())
    }
}

impl std::error::Error for BudgetExceeded {}

/// The shared runtime budget. Cheap to clone as an `Arc`; every method takes
/// `&self` and mutates only atomics.
#[derive(Debug)]
pub struct ExecutionBudget {
    limits: BudgetLimits,
    started: Instant,
    cancelled: AtomicBool,
    /// Total interpreter operations charged. Also drives the clock-sampling
    /// stride, exactly as the old `op_count` field did.
    operations: AtomicU64,
    /// Accepted-but-unfinished HTTP requests currently in flight.
    pending_requests: AtomicUsize,
    /// Live WebSocket connections currently registered.
    ws_connections: AtomicUsize,
    /// Pattern-VM transitions charged for the *current* match operation. One
    /// shared meter so nested lookaround VMs count against the same budget
    /// (they don't reset it); the top-level operation resets it to zero.
    pattern_steps: AtomicU64,
}

impl ExecutionBudget {
    /// Build a budget from explicit limits, starting the deadline clock now.
    pub fn new(limits: BudgetLimits) -> Self {
        Self {
            limits,
            started: Instant::now(),
            cancelled: AtomicBool::new(false),
            operations: AtomicU64::new(0),
            pending_requests: AtomicUsize::new(0),
            ws_connections: AtomicUsize::new(0),
            pattern_steps: AtomicU64::new(0),
        }
    }

    /// Build a budget from a loaded configuration. See
    /// [`BudgetLimits::from_config`].
    pub fn from_config(config: &WflConfig) -> Self {
        Self::new(BudgetLimits::from_config(config))
    }

    /// A budget with no effective ceilings (pattern ReDoS guards aside). For
    /// standalone pattern helpers and tests.
    pub fn unlimited() -> Self {
        Self::new(BudgetLimits::unlimited())
    }

    /// The immutable limits backing this budget.
    pub fn limits(&self) -> &BudgetLimits {
        &self.limits
    }

    /// Time elapsed since the budget was created.
    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }

    // ----- Deadline & cancellation -----------------------------------------

    /// Request cooperative cancellation. The next sampled checkpoint (and any
    /// [`ExecutionBudget::check_cancelled`] call) observes it.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Fail if cancellation has been requested. Cheap; safe to call anywhere.
    pub fn check_cancelled(&self) -> Result<(), BudgetExceeded> {
        if self.is_cancelled() {
            Err(BudgetExceeded::Cancelled)
        } else {
            Ok(())
        }
    }

    /// Fail if the wall-clock deadline has elapsed. Reads the clock every call;
    /// prefer [`ExecutionBudget::charge_operation`] on hot paths, which samples.
    pub fn check_deadline(&self) -> Result<(), BudgetExceeded> {
        if let Some(limit) = self.limits.max_duration
            && self.started.elapsed() > limit
        {
            return Err(BudgetExceeded::Deadline {
                limit_secs: limit.as_secs(),
            });
        }
        Ok(())
    }

    // ----- Interpreter operations ------------------------------------------

    /// Charge one interpreter operation.
    ///
    /// Always: increments the operation counter and, on a throttled stride,
    /// honours cancellation. When `enforce_limits` is true it also enforces the
    /// operation ceiling (every call) and the wall-clock deadline (on the
    /// stride). `enforce_limits` is set to `false` while inside a `main loop`,
    /// preserving the historic rule that a long-lived server loop is exempt from
    /// the timeout — cancellation still applies so a server can be stopped.
    pub fn charge_operation(&self, enforce_limits: bool) -> Result<(), BudgetExceeded> {
        // `main loop` exemption: do not consume the operation budget or read the
        // clock (a long-lived server would otherwise exhaust the ceiling), but
        // still honour cooperative cancellation so the loop can be stopped. The
        // operation counter is left untouched so exempt work cannot later push a
        // post-loop `Operations` breach.
        if !enforce_limits {
            if self.cancelled.load(Ordering::Relaxed) {
                return Err(BudgetExceeded::Cancelled);
            }
            return Ok(());
        }

        // `fetch_add` returns the previous value; use it as this op's index so
        // the very first op (index 0) is a sample point, matching the old code.
        let index = self.operations.fetch_add(1, Ordering::Relaxed);
        let sample = index & (CLOCK_SAMPLE_STRIDE - 1) == 0;

        if let Some(limit) = self.limits.max_operations
            && index >= limit
        {
            return Err(BudgetExceeded::Operations { limit });
        }

        if sample {
            if self.cancelled.load(Ordering::Relaxed) {
                return Err(BudgetExceeded::Cancelled);
            }
            if let Some(limit) = self.limits.max_duration
                && self.started.elapsed() > limit
            {
                return Err(BudgetExceeded::Deadline {
                    limit_secs: limit.as_secs(),
                });
            }
        }

        Ok(())
    }

    /// Operations charged so far.
    pub fn operations_charged(&self) -> u64 {
        self.operations.load(Ordering::Relaxed)
    }

    // ----- Depth guards -----------------------------------------------------

    /// Fail if entering another call frame would exceed the recursion ceiling.
    /// `current_depth` is the number of frames already on the stack.
    pub fn check_call_depth(&self, current_depth: usize) -> Result<(), BudgetExceeded> {
        if current_depth >= self.limits.max_call_depth {
            Err(BudgetExceeded::CallDepth {
                limit: self.limits.max_call_depth,
            })
        } else {
            Ok(())
        }
    }

    /// Fail if entering another import would exceed the import ceiling.
    /// `current_depth` is the number of modules already loading.
    pub fn check_import_depth(&self, current_depth: usize) -> Result<(), BudgetExceeded> {
        if current_depth >= self.limits.max_import_depth {
            Err(BudgetExceeded::ImportDepth {
                limit: self.limits.max_import_depth,
            })
        } else {
            Ok(())
        }
    }

    /// Fail if entering another `execute file` level would exceed the ceiling.
    pub fn check_execute_file_depth(&self, current_depth: usize) -> Result<(), BudgetExceeded> {
        if current_depth >= self.limits.max_execute_file_depth {
            Err(BudgetExceeded::ExecuteFileDepth {
                limit: self.limits.max_execute_file_depth,
            })
        } else {
            Ok(())
        }
    }

    // ----- Pattern matching -------------------------------------------------

    /// The per-match transition ceiling for the pattern VM.
    pub fn pattern_step_limit(&self) -> usize {
        self.limits.max_pattern_steps
    }

    /// The per-match active-state ceiling for the pattern VM.
    pub fn pattern_state_limit(&self) -> usize {
        self.limits.max_pattern_states
    }

    /// Reset the shared pattern-transition meter. Called once at the start of a
    /// top-level pattern operation (`matches`/`find`/`find_all`/…); nested
    /// lookaround VMs deliberately do **not** reset it, so their work counts
    /// against the same budget as the enclosing match.
    pub fn reset_pattern_steps(&self) {
        self.pattern_steps.store(0, Ordering::Relaxed);
    }

    /// Charge one pattern-VM transition (one instruction dispatched, or one
    /// negative-lookahead loop iteration) against the shared meter, failing once
    /// the per-operation ceiling is exceeded. Also honours cancellation on a
    /// throttled stride so a runaway match can be aborted.
    pub fn charge_pattern_step(&self) -> Result<(), BudgetExceeded> {
        let n = self.pattern_steps.fetch_add(1, Ordering::Relaxed);
        if n >= self.limits.max_pattern_steps as u64 {
            return Err(BudgetExceeded::PatternSteps {
                limit: self.limits.max_pattern_steps,
            });
        }
        if n & (CLOCK_SAMPLE_STRIDE - 1) == 0 && self.cancelled.load(Ordering::Relaxed) {
            return Err(BudgetExceeded::Cancelled);
        }
        Ok(())
    }

    /// Fail if a pattern match has taken more transitions than allowed.
    pub fn check_pattern_steps(&self, steps: usize) -> Result<(), BudgetExceeded> {
        if steps > self.limits.max_pattern_steps {
            Err(BudgetExceeded::PatternSteps {
                limit: self.limits.max_pattern_steps,
            })
        } else {
            Ok(())
        }
    }

    /// Fail if a pattern match holds more active states than allowed.
    pub fn check_pattern_states(&self, states: usize) -> Result<(), BudgetExceeded> {
        if states > self.limits.max_pattern_states {
            Err(BudgetExceeded::PatternStates {
                limit: self.limits.max_pattern_states,
            })
        } else {
            Ok(())
        }
    }

    // ----- Byte ceilings ----------------------------------------------------

    /// The source-file byte ceiling. A bounded loader reads at most this many
    /// bytes (plus one) so an oversized file is refused without allocating it.
    pub fn max_source_bytes(&self) -> usize {
        self.limits.max_source_bytes
    }

    /// Fail if a source file exceeds the byte ceiling. `len` is a raw file
    /// length (`u64`); a value that does not fit in `usize` (huge file on a
    /// 32-bit target) is treated as over the limit rather than truncated.
    pub fn check_source_len(&self, len: u64) -> Result<(), BudgetExceeded> {
        match usize::try_from(len) {
            Ok(len) => self.check_source_bytes(len),
            Err(_) => Err(BudgetExceeded::SourceBytes {
                limit: self.limits.max_source_bytes,
                actual: usize::MAX,
            }),
        }
    }

    /// Fail if a source file exceeds the byte ceiling.
    pub fn check_source_bytes(&self, len: usize) -> Result<(), BudgetExceeded> {
        if len > self.limits.max_source_bytes {
            Err(BudgetExceeded::SourceBytes {
                limit: self.limits.max_source_bytes,
                actual: len,
            })
        } else {
            Ok(())
        }
    }

    /// Fail if an HTTP request body exceeds the byte ceiling.
    pub fn check_request_body_bytes(&self, len: usize) -> Result<(), BudgetExceeded> {
        if len > self.limits.max_request_body_bytes {
            Err(BudgetExceeded::RequestBodyBytes {
                limit: self.limits.max_request_body_bytes,
                actual: len,
            })
        } else {
            Ok(())
        }
    }

    /// Fail if an HTTP response body exceeds the byte ceiling.
    pub fn check_response_bytes(&self, len: usize) -> Result<(), BudgetExceeded> {
        if len > self.limits.max_response_bytes {
            Err(BudgetExceeded::ResponseBytes {
                limit: self.limits.max_response_bytes,
                actual: len,
            })
        } else {
            Ok(())
        }
    }

    /// The accepted HTTP request body ceiling in bytes.
    pub fn max_request_body_bytes(&self) -> usize {
        self.limits.max_request_body_bytes
    }

    // ----- Pending HTTP requests -------------------------------------------

    /// The pending/in-flight HTTP request ceiling. The web transport sizes its
    /// bounded queue and admission semaphore from this value.
    pub fn max_pending_requests(&self) -> usize {
        self.limits.max_pending_requests
    }

    /// Try to reserve a pending-request slot, returning an RAII guard that
    /// releases it on drop. `None` when already at the ceiling. Provided for
    /// callers that want to account pending requests directly; the web server's
    /// bounded queue is the primary enforcement path.
    pub fn try_acquire_request(self: &Arc<Self>) -> Option<RequestGuard> {
        let limit = self.limits.max_pending_requests;
        let mut current = self.pending_requests.load(Ordering::Acquire);
        loop {
            if current >= limit {
                return None;
            }
            match self.pending_requests.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Some(RequestGuard {
                        budget: Arc::clone(self),
                    });
                }
                Err(observed) => current = observed,
            }
        }
    }

    /// Pending HTTP requests currently reserved.
    pub fn pending_requests(&self) -> usize {
        self.pending_requests.load(Ordering::Relaxed)
    }

    // ----- WebSocket --------------------------------------------------------

    /// The per-channel WebSocket queue bound. Transport tasks size their
    /// bounded channels from this value and shed on `Full`.
    pub fn ws_queue_bound(&self) -> usize {
        self.limits.max_ws_queue
    }

    /// Try to reserve a WebSocket connection slot, returning an RAII guard that
    /// releases it when the connection ends. `None` when already at the
    /// ceiling, in which case the transport should refuse the connection.
    pub fn try_acquire_ws_connection(self: &Arc<Self>) -> Option<WsConnectionGuard> {
        let limit = self.limits.max_ws_connections;
        let mut current = self.ws_connections.load(Ordering::Acquire);
        loop {
            if current >= limit {
                return None;
            }
            match self.ws_connections.compare_exchange_weak(
                current,
                current + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Some(WsConnectionGuard {
                        budget: Arc::clone(self),
                    });
                }
                Err(observed) => current = observed,
            }
        }
    }

    /// Live WebSocket connections currently reserved.
    pub fn ws_connections(&self) -> usize {
        self.ws_connections.load(Ordering::Relaxed)
    }
}

impl Default for ExecutionBudget {
    fn default() -> Self {
        Self::new(BudgetLimits::default())
    }
}

thread_local! {
    /// The budget in effect on this thread, set for the duration of a run so
    /// leaf helpers with no budget parameter (notably the stdlib pattern
    /// builtins, whose native signature is `fn(Vec<Value>) -> ...`) still match
    /// under the run's configured ceilings and shared meters. `None` outside a
    /// run.
    static CURRENT_BUDGET: std::cell::RefCell<Option<Arc<ExecutionBudget>>> =
        const { std::cell::RefCell::new(None) };
}

impl ExecutionBudget {
    /// Install `budget` as the current-thread budget for the lifetime of the
    /// returned guard (restoring the previous one on drop). Nesting is
    /// supported. Used by the interpreter to scope a run.
    pub fn enter(budget: Arc<ExecutionBudget>) -> CurrentBudgetGuard {
        let previous = CURRENT_BUDGET.with(|c| c.borrow_mut().replace(budget));
        CurrentBudgetGuard { previous }
    }

    /// The current-thread budget, if a run has installed one via
    /// [`ExecutionBudget::enter`].
    pub fn current() -> Option<Arc<ExecutionBudget>> {
        CURRENT_BUDGET.with(|c| c.borrow().clone())
    }

    /// The current-thread budget, or a fresh unlimited one (which still carries
    /// the pattern ReDoS ceilings) when no run is active — so a bare
    /// [`crate::pattern::PatternVM::new`] is always bounded.
    pub fn current_or_default() -> Arc<ExecutionBudget> {
        Self::current().unwrap_or_else(|| Arc::new(Self::unlimited()))
    }
}

/// Restores the previous current-thread budget when dropped.
#[must_use]
pub struct CurrentBudgetGuard {
    previous: Option<Arc<ExecutionBudget>>,
}

impl Drop for CurrentBudgetGuard {
    fn drop(&mut self) {
        CURRENT_BUDGET.with(|c| *c.borrow_mut() = self.previous.take());
    }
}

/// RAII slot for one in-flight HTTP request; releases on drop.
#[derive(Debug)]
pub struct RequestGuard {
    budget: Arc<ExecutionBudget>,
}

impl Drop for RequestGuard {
    fn drop(&mut self) {
        self.budget.pending_requests.fetch_sub(1, Ordering::AcqRel);
    }
}

/// RAII slot for one live WebSocket connection; releases on drop.
#[derive(Debug)]
pub struct WsConnectionGuard {
    budget: Arc<ExecutionBudget>,
}

impl Drop for WsConnectionGuard {
    fn drop(&mut self) {
        self.budget.ws_connections.fetch_sub(1, Ordering::AcqRel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_limits() -> BudgetLimits {
        BudgetLimits {
            max_duration: None,
            max_operations: Some(10),
            max_call_depth: 3,
            max_import_depth: 2,
            max_execute_file_depth: 2,
            max_pattern_steps: 5,
            max_pattern_states: 4,
            max_source_bytes: 8,
            max_request_body_bytes: 8,
            max_response_bytes: 8,
            max_pending_requests: 2,
            max_ws_queue: 2,
            max_ws_connections: 2,
        }
    }

    #[test]
    fn operation_ceiling_trips_after_limit() {
        let budget = ExecutionBudget::new(tiny_limits());
        // Ten operations (indices 0..=9) are allowed; the eleventh (index 10)
        // reaches the ceiling.
        for _ in 0..10 {
            budget.charge_operation(true).expect("within budget");
        }
        assert_eq!(
            budget.charge_operation(true),
            Err(BudgetExceeded::Operations { limit: 10 })
        );
    }

    #[test]
    fn main_loop_exemption_skips_operation_ceiling() {
        let budget = ExecutionBudget::new(tiny_limits());
        // `enforce_limits = false` (inside a main loop) never trips the ceiling.
        for _ in 0..100 {
            budget.charge_operation(false).expect("exempt from ceiling");
        }
    }

    #[test]
    fn cancellation_is_honored_even_when_exempt() {
        let budget = ExecutionBudget::new(tiny_limits());
        budget.cancel();
        assert!(budget.is_cancelled());
        // Index 0 is a sample point, so cancellation is observed immediately
        // even with `enforce_limits = false`.
        assert_eq!(
            budget.charge_operation(false),
            Err(BudgetExceeded::Cancelled)
        );
    }

    #[test]
    fn deadline_trips_when_elapsed() {
        let mut limits = tiny_limits();
        limits.max_duration = Some(Duration::from_secs(0));
        let budget = ExecutionBudget::new(limits);
        // A zero-second deadline is already elapsed at the first sample point.
        assert_eq!(
            budget.charge_operation(true),
            Err(BudgetExceeded::Deadline { limit_secs: 0 })
        );
    }

    #[test]
    fn depth_guards_use_ge_semantics() {
        let budget = ExecutionBudget::new(tiny_limits());
        // Depths 0,1,2 fit under a limit of 3; depth 3 is refused.
        assert!(budget.check_call_depth(2).is_ok());
        assert_eq!(
            budget.check_call_depth(3),
            Err(BudgetExceeded::CallDepth { limit: 3 })
        );
        assert!(budget.check_import_depth(1).is_ok());
        assert_eq!(
            budget.check_import_depth(2),
            Err(BudgetExceeded::ImportDepth { limit: 2 })
        );
        assert!(budget.check_execute_file_depth(1).is_ok());
        assert_eq!(
            budget.check_execute_file_depth(2),
            Err(BudgetExceeded::ExecuteFileDepth { limit: 2 })
        );
    }

    #[test]
    fn pattern_checks_use_gt_semantics() {
        let budget = ExecutionBudget::new(tiny_limits());
        // Steps equal to the limit are still fine; exceeding it fails.
        assert!(budget.check_pattern_steps(5).is_ok());
        assert_eq!(
            budget.check_pattern_steps(6),
            Err(BudgetExceeded::PatternSteps { limit: 5 })
        );
        assert!(budget.check_pattern_states(4).is_ok());
        assert_eq!(
            budget.check_pattern_states(5),
            Err(BudgetExceeded::PatternStates { limit: 4 })
        );
    }

    #[test]
    fn pattern_step_meter_charges_and_resets() {
        let budget = ExecutionBudget::new(tiny_limits()); // max_pattern_steps = 5
        // Charging is shared across (simulated) nested VMs without resetting; the
        // sixth charge (index 5 >= limit 5) trips.
        for _ in 0..5 {
            budget.charge_pattern_step().expect("within pattern budget");
        }
        assert_eq!(
            budget.charge_pattern_step(),
            Err(BudgetExceeded::PatternSteps { limit: 5 })
        );
        // A top-level operation resets the shared meter for the next match.
        budget.reset_pattern_steps();
        assert!(budget.charge_pattern_step().is_ok());
    }

    #[test]
    fn current_budget_scope_is_restored() {
        assert!(ExecutionBudget::current().is_none());
        let outer = Arc::new(ExecutionBudget::new(tiny_limits()));
        {
            let _g = ExecutionBudget::enter(Arc::clone(&outer));
            assert!(Arc::ptr_eq(&ExecutionBudget::current().unwrap(), &outer));
        }
        // Guard dropped: the thread-local is cleared again.
        assert!(ExecutionBudget::current().is_none());
    }

    #[test]
    fn byte_ceilings_report_actual_and_limit() {
        let budget = ExecutionBudget::new(tiny_limits());
        assert!(budget.check_source_bytes(8).is_ok());
        assert_eq!(
            budget.check_source_bytes(9),
            Err(BudgetExceeded::SourceBytes {
                limit: 8,
                actual: 9
            })
        );
        assert_eq!(
            budget.check_request_body_bytes(100),
            Err(BudgetExceeded::RequestBodyBytes {
                limit: 8,
                actual: 100
            })
        );
        assert_eq!(
            budget.check_response_bytes(100),
            Err(BudgetExceeded::ResponseBytes {
                limit: 8,
                actual: 100
            })
        );
    }

    #[test]
    fn request_guard_releases_slot_on_drop() {
        let budget = Arc::new(ExecutionBudget::new(tiny_limits()));
        let a = budget.try_acquire_request().expect("slot 1");
        let b = budget.try_acquire_request().expect("slot 2");
        assert_eq!(budget.pending_requests(), 2);
        // At the ceiling of 2, a third acquisition is refused.
        assert!(budget.try_acquire_request().is_none());
        drop(a);
        assert_eq!(budget.pending_requests(), 1);
        // A freed slot can be reacquired.
        let _c = budget.try_acquire_request().expect("slot reused");
        drop(b);
    }

    #[test]
    fn ws_connection_guard_bounds_live_connections() {
        let budget = Arc::new(ExecutionBudget::new(tiny_limits()));
        let a = budget.try_acquire_ws_connection().expect("conn 1");
        let _b = budget.try_acquire_ws_connection().expect("conn 2");
        assert_eq!(budget.ws_connections(), 2);
        assert!(budget.try_acquire_ws_connection().is_none());
        drop(a);
        assert_eq!(budget.ws_connections(), 1);
    }

    #[test]
    fn from_config_maps_existing_keys() {
        let config = WflConfig {
            timeout_seconds: 42,
            web_server_max_body_size: 4096,
            web_server_request_queue_bound: 7,
            ..Default::default()
        };
        let limits = BudgetLimits::from_config(&config);
        assert_eq!(limits.max_duration, Some(Duration::from_secs(42)));
        assert_eq!(limits.max_request_body_bytes, 4096);
        assert_eq!(limits.max_pending_requests, 7);
    }

    #[test]
    fn budget_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ExecutionBudget>();
        assert_send_sync::<Arc<ExecutionBudget>>();
    }
}
