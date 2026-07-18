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
//! * **Source, file-read, body, and response bytes** —
//!   [`ExecutionBudget::check_source_bytes`],
//!   [`ExecutionBudget::check_file_read_bytes`],
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
    /// Maximum bytes buffered by one text or binary file read. Mapped from
    /// `.wflcfg` `max_file_read_size`.
    pub max_file_read_bytes: usize,
    /// Maximum accepted HTTP request body size in bytes. Mapped from `.wflcfg`
    /// `web_server_max_body_size`.
    pub max_request_body_bytes: usize,
    /// Maximum HTTP response body size in bytes, for both handler responses and
    /// bodies read by outbound `open url` statements. Mapped from `.wflcfg`
    /// `web_server_max_response_size`.
    pub max_response_bytes: usize,
    /// Maximum accepted-but-unhandled HTTP requests held in the transport
    /// queue. Mapped from `.wflcfg` `web_server_request_queue_bound`.
    pub max_pending_requests: usize,
    /// Maximum wall-clock time the transport waits for a handler to answer an
    /// accepted request before shedding it with 504 and releasing its in-flight
    /// slot. `None` disables the timeout. Mapped from `.wflcfg`
    /// `web_server_response_timeout_seconds` (`0` = disabled).
    pub max_request_duration: Option<Duration>,
    /// Maximum queued frames/events per WebSocket channel. Mapped from
    /// `.wflcfg` `web_socket_queue_bound`.
    pub max_ws_queue: usize,
    /// Maximum simultaneous live WebSocket connections. Mapped from `.wflcfg`
    /// `web_socket_max_connections`.
    pub max_ws_connections: usize,
    /// Maximum size in bytes of a single WebSocket text message (inbound or
    /// outbound). Mapped from `.wflcfg` `web_socket_max_message_size`.
    pub max_ws_message_bytes: usize,
    /// Global ceiling in bytes on WebSocket payloads queued across every
    /// connection. Mapped from `.wflcfg` `web_socket_max_queued_bytes`.
    pub max_ws_queued_bytes: usize,
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
            // Preserve the file-I/O guide's existing 50 MiB per-read policy,
            // now enforced for both text and binary reads while streaming.
            max_file_read_bytes: 50 * 1024 * 1024,
            // Preserves the previous `web_server_max_body_size` default (1 MiB).
            max_request_body_bytes: 1_048_576,
            // Response size was unchecked before; 64 MiB clears any real payload.
            max_response_bytes: 64 * 1024 * 1024,
            // Preserves the previous `web_server_request_queue_bound` default.
            max_pending_requests: 256,
            // Bound how long an accepted request may await its handler, so a
            // dequeued-but-unanswered request cannot pin its in-flight slot
            // forever. 300s is far longer than any serial handler needs.
            max_request_duration: Some(Duration::from_secs(300)),
            // WebSocket channels were unbounded before; 1_024 clears normal use.
            max_ws_queue: 1_024,
            // Connection count was uncapped before; 1_024 clears normal use.
            max_ws_connections: 1_024,
            // Per-message size was unbounded (only frame count was capped); 1 MiB
            // clears normal chat/JSON traffic while bounding a single frame.
            max_ws_message_bytes: 1_048_576,
            // Global queued-byte ceiling across all WS channels; 16 MiB bounds
            // total buffered payload regardless of connection/frame counts.
            max_ws_queued_bytes: 16 * 1_048_576,
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
            max_file_read_bytes: config.max_file_read_size,
            max_request_body_bytes: config.web_server_max_body_size,
            max_response_bytes: config.web_server_max_response_size,
            max_pending_requests: config.web_server_request_queue_bound.max(1),
            max_request_duration: match config.web_server_response_timeout_seconds {
                0 => None,
                secs => Some(Duration::from_secs(secs)),
            },
            max_ws_queue: config.web_socket_queue_bound.max(1),
            max_ws_connections: config.web_socket_max_connections.max(1),
            max_ws_message_bytes: config.web_socket_max_message_size.max(1),
            max_ws_queued_bytes: config.web_socket_max_queued_bytes.max(1),
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
            max_file_read_bytes: usize::MAX,
            max_request_body_bytes: usize::MAX,
            max_response_bytes: usize::MAX,
            max_pending_requests: usize::MAX,
            max_request_duration: None,
            max_ws_queue: usize::MAX,
            max_ws_connections: usize::MAX,
            max_ws_message_bytes: usize::MAX,
            max_ws_queued_bytes: usize::MAX,
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
    /// A text or binary file read exceeded its per-operation byte ceiling.
    FileReadBytes { limit: usize, actual: usize },
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
            BudgetExceeded::FileReadBytes { limit, actual } => {
                format!("File read too large: {actual} bytes (limit: {limit} bytes)")
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
    /// WebSocket payload bytes currently queued across every connection's
    /// inbound event and outbound frame channels. Bounded by
    /// `limits.max_ws_queued_bytes`; each queued frame holds a [`WsBytePermit`]
    /// that releases its bytes when the frame is consumed or shed.
    ws_queued_bytes: AtomicUsize,
    /// Number of `main loop`s currently active (a *depth*, not a flag). The
    /// wall-clock deadline is exempt while this is `> 0` — a long-lived server
    /// must not time out on its own uptime. A **depth counter** (rather than a
    /// bool) makes the exemption nestable and correct under `execute file`: a
    /// child interpreter that shares this budget inherits the parent's active
    /// main loop, and an [`MainLoopGuard`] restores the depth on *every* exit,
    /// including a caught error. Pattern matching reads this live so a match
    /// launched inside a `main loop` gets the same exemption as ordinary
    /// operations (see [`ExecutionBudget::charge_operation`]).
    main_loop_depth: AtomicUsize,
}

// NOTE: per-match pattern accounting (transitions + active states) lives on a
// separate per-match [`PatternMeter`], *not* on the shared run budget. Two
// matches that share one `Arc<ExecutionBudget>` (e.g. concurrent web handlers)
// must never reset or share a single transition counter, or one could grant the
// other unbounded extra quota. The budget owns only the *limits* and the
// cross-cutting deadline/cancellation the meter samples.

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
            ws_queued_bytes: AtomicUsize::new(0),
            main_loop_depth: AtomicUsize::new(0),
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

    /// Enter a `main loop`: bump the main-loop depth and return an
    /// [`MainLoopGuard`] that restores it on drop — on *every* exit path,
    /// including a caught error or a nested loop, so the wall-clock exemption is
    /// never leaked or cleared early. While the depth is `> 0` the deadline is
    /// exempt (a long-lived server must not time out on its own uptime).
    pub fn enter_main_loop(self: &Arc<Self>) -> MainLoopGuard {
        self.main_loop_depth.fetch_add(1, Ordering::AcqRel);
        MainLoopGuard {
            budget: Arc::clone(self),
        }
    }

    /// Whether the wall-clock deadline is currently exempt — i.e. at least one
    /// `main loop` is active on this (shared) budget. Read live, so a match or
    /// operation launched inside a `main loop` is exempt and one launched after
    /// it exits is not.
    pub fn is_deadline_exempt(&self) -> bool {
        self.main_loop_depth.load(Ordering::Acquire) > 0
    }

    /// The number of `main loop`s currently active on this budget.
    pub fn main_loop_depth(&self) -> usize {
        self.main_loop_depth.load(Ordering::Acquire)
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

    /// The per-operation ceiling for buffered text and binary file reads.
    pub fn max_file_read_bytes(&self) -> usize {
        self.limits.max_file_read_bytes
    }

    /// Fail if a text or binary file read exceeds its byte ceiling.
    pub fn check_file_read_bytes(&self, len: usize) -> Result<(), BudgetExceeded> {
        if len > self.limits.max_file_read_bytes {
            Err(BudgetExceeded::FileReadBytes {
                limit: self.limits.max_file_read_bytes,
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

    /// The maximum time the transport waits for a handler to answer an accepted
    /// request before shedding it (504) and freeing its slot. `None` = no limit.
    pub fn max_request_duration(&self) -> Option<Duration> {
        self.limits.max_request_duration
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

    /// The maximum size in bytes of a single WebSocket text message; larger
    /// frames are dropped rather than queued.
    pub fn max_ws_message_bytes(&self) -> usize {
        self.limits.max_ws_message_bytes
    }

    /// Try to reserve `bytes` of the global WebSocket queued-byte budget for one
    /// frame, returning an RAII [`WsBytePermit`] that releases them when the
    /// frame is consumed or shed. `None` when the frame alone exceeds
    /// `max_ws_message_bytes`, or when reserving would exceed the global
    /// `max_ws_queued_bytes` ceiling — in which case the transport sheds it.
    pub fn try_reserve_ws_bytes(self: &Arc<Self>, bytes: usize) -> Option<WsBytePermit> {
        if bytes > self.limits.max_ws_message_bytes {
            return None;
        }
        let limit = self.limits.max_ws_queued_bytes;
        let mut current = self.ws_queued_bytes.load(Ordering::Acquire);
        loop {
            if current.saturating_add(bytes) > limit {
                return None;
            }
            match self.ws_queued_bytes.compare_exchange_weak(
                current,
                current + bytes,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    return Some(WsBytePermit {
                        budget: Arc::clone(self),
                        bytes,
                    });
                }
                Err(observed) => current = observed,
            }
        }
    }

    /// WebSocket payload bytes currently queued across every connection.
    pub fn ws_queued_bytes(&self) -> usize {
        self.ws_queued_bytes.load(Ordering::Relaxed)
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

tokio::task_local! {
    /// The budget in effect for the current async **task** — an interpreter run
    /// or a REPL command. Task-local, NOT thread-local, so two interpreter
    /// futures interleaved on one thread (a library embedder that `join!`s or
    /// `spawn_local`s two `Interpreter`s — both are re-exported from the crate
    /// root and are `!Send`, so this is legal) never observe each other's budget
    /// or restore stale state across an `.await`. Any async run establishes this
    /// scope, so it always takes precedence over the synchronous fallback below.
    static CURRENT_BUDGET_TASK: Arc<ExecutionBudget>;
}

thread_local! {
    /// Synchronous fallback current budget, consulted only when no task-local
    /// scope is active. It exists for code that runs to completion **without
    /// awaiting** and cannot interleave — specifically the CLI front-end
    /// (lex/parse/analyze/type-check) installed by `main`, which runs on a
    /// single-future runtime. Because every async run (`interpret`, REPL
    /// `process_line`) wraps itself in a [`ExecutionBudget::scope`] that shadows
    /// this, the fallback can never cross-contaminate an interleaved run.
    static CURRENT_BUDGET_THREAD: std::cell::RefCell<Option<Arc<ExecutionBudget>>> =
        const { std::cell::RefCell::new(None) };
}

impl ExecutionBudget {
    /// Run `future` with `budget` installed as the task-local current budget,
    /// restoring the previous task-local (if any) when it completes. This is the
    /// interleaving-safe way to scope a run; prefer it for every async run.
    /// Nesting (e.g. an `execute file` child) is supported.
    pub async fn scope<F>(budget: Arc<ExecutionBudget>, future: F) -> F::Output
    where
        F: std::future::Future,
    {
        CURRENT_BUDGET_TASK.scope(budget, future).await
    }

    /// Install `budget` as the synchronous thread-local fallback for the
    /// lifetime of the returned guard (restoring the previous one on drop). Use
    /// this ONLY for synchronous, non-interleaving contexts (the CLI front-end);
    /// async runs must use [`ExecutionBudget::scope`], which takes precedence.
    pub fn enter(budget: Arc<ExecutionBudget>) -> CurrentBudgetGuard {
        let previous = CURRENT_BUDGET_THREAD.with(|c| c.borrow_mut().replace(budget));
        CurrentBudgetGuard { previous }
    }

    /// The current budget: the task-local scope if one is active (an async run),
    /// otherwise the synchronous thread-local fallback (the CLI front-end).
    pub fn current() -> Option<Arc<ExecutionBudget>> {
        CURRENT_BUDGET_TASK
            .try_with(Arc::clone)
            .ok()
            .or_else(|| CURRENT_BUDGET_THREAD.with(|c| c.borrow().clone()))
    }

    /// The current budget, or a fresh unlimited one (which still carries the
    /// pattern ReDoS ceilings) when no run is active — so a bare
    /// [`crate::pattern::PatternVM::new`] is always bounded.
    pub fn current_or_default() -> Arc<ExecutionBudget> {
        Self::current().unwrap_or_else(|| Arc::new(Self::unlimited()))
    }
}

/// Restores the previous synchronous thread-local fallback budget when dropped.
#[must_use]
pub struct CurrentBudgetGuard {
    previous: Option<Arc<ExecutionBudget>>,
}

impl Drop for CurrentBudgetGuard {
    fn drop(&mut self) {
        CURRENT_BUDGET_THREAD.with(|c| *c.borrow_mut() = self.previous.take());
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

/// RAII reservation of global WebSocket queued bytes for one queued frame;
/// releases the bytes when the frame is consumed (dequeued and dropped) or shed.
#[derive(Debug)]
pub struct WsBytePermit {
    budget: Arc<ExecutionBudget>,
    bytes: usize,
}

impl Drop for WsBytePermit {
    fn drop(&mut self) {
        self.budget
            .ws_queued_bytes
            .fetch_sub(self.bytes, Ordering::AcqRel);
    }
}

/// RAII marker for one active `main loop`; decrements the shared main-loop depth
/// on drop. Because it restores on *every* exit — normal, early `return`, a
/// caught error unwinding through the loop, or a nested loop — the wall-clock
/// exemption can never leak past the loop or be cleared while an outer loop is
/// still active.
#[derive(Debug)]
#[must_use]
pub struct MainLoopGuard {
    budget: Arc<ExecutionBudget>,
}

impl Drop for MainLoopGuard {
    fn drop(&mut self) {
        self.budget.main_loop_depth.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Per-top-level-match pattern metering.
///
/// A fresh `PatternMeter` is created for each top-level pattern operation
/// (`matches`/`find`/`find_all`) and cloned (as an `Arc`) **only** into nested
/// lookaround/lookbehind VMs, so their transitions and active states count
/// against the *same* per-match ceilings as the enclosing match. It borrows the
/// run's limits, wall-clock deadline, and cancellation flag from the shared
/// [`ExecutionBudget`], but keeps its own transition counter and active-state
/// accounting — so two matches sharing one run budget (e.g. concurrent web
/// handlers) never reset or share each other's meter.
///
/// Kept atomic (rather than `Cell`) so a [`crate::pattern::PatternVM`] stays
/// `Send`; a single match runs on one thread, so the atomics are uncontended.
#[derive(Debug)]
pub struct PatternMeter {
    budget: Arc<ExecutionBudget>,
    /// Transitions charged for this match (all frontiers, all nested VMs).
    steps: AtomicU64,
    /// State slots reserved live across every frontier (current + next
    /// generation + any suspended nested lookaround/lookbehind frontiers).
    active_states: AtomicUsize,
}

impl PatternMeter {
    /// A fresh per-match meter bound to `budget`.
    pub fn new(budget: Arc<ExecutionBudget>) -> Arc<Self> {
        Arc::new(Self {
            budget,
            steps: AtomicU64::new(0),
            active_states: AtomicUsize::new(0),
        })
    }

    /// The shared run budget this meter borrows limits/deadline/cancellation from.
    pub fn budget(&self) -> &Arc<ExecutionBudget> {
        &self.budget
    }

    /// Reset the per-match counters. Called once at the start of each *direct*
    /// top-level VM operation so reusing one VM does not accumulate transitions
    /// from an unrelated prior match; nested lookaround VMs share the meter and
    /// deliberately do **not** reset it.
    pub fn reset(&self) {
        self.steps.store(0, Ordering::Relaxed);
        self.active_states.store(0, Ordering::Relaxed);
    }

    /// Charge one pattern-VM transition (one dispatched instruction). Fails once
    /// the per-match transition ceiling is exceeded, and — on a throttled stride
    /// — honours cancellation and (unless exempt) the wall-clock deadline, so a
    /// single synchronous match cannot run past `timeout_seconds`. A deadline
    /// breach surfaces as [`BudgetExceeded::Deadline`] (a timeout), not a step
    /// limit.
    ///
    /// The deadline exemption is read **live** from the shared budget's
    /// main-loop depth on each sampled stride (not snapshotted at construction),
    /// so reusing one `PatternVM` across a `main loop` boundary is always correct
    /// — a match that enters a `main loop` region stops enforcing the deadline
    /// and one that leaves it resumes enforcing.
    pub fn charge_step(&self) -> Result<(), BudgetExceeded> {
        let n = self.steps.fetch_add(1, Ordering::Relaxed);
        if n >= self.budget.limits.max_pattern_steps as u64 {
            return Err(BudgetExceeded::PatternSteps {
                limit: self.budget.limits.max_pattern_steps,
            });
        }
        if n & (CLOCK_SAMPLE_STRIDE - 1) == 0 {
            if self.budget.cancelled.load(Ordering::Relaxed) {
                return Err(BudgetExceeded::Cancelled);
            }
            if !self.budget.is_deadline_exempt()
                && let Some(limit) = self.budget.limits.max_duration
                && self.budget.started.elapsed() > limit
            {
                return Err(BudgetExceeded::Deadline {
                    limit_secs: limit.as_secs(),
                });
            }
        }
        Ok(())
    }

    /// Reserve `n` active-state slots, returning an RAII [`StateReservation`]
    /// that releases them on drop. Fails if the total live reservation (this
    /// frontier plus every other frontier currently holding slots, including
    /// nested VMs) would exceed the per-match active-state ceiling.
    pub fn reserve_states(self: &Arc<Self>, n: usize) -> Result<StateReservation, BudgetExceeded> {
        let prev = self.active_states.fetch_add(n, Ordering::Relaxed);
        if prev + n > self.budget.limits.max_pattern_states {
            self.active_states.fetch_sub(n, Ordering::Relaxed);
            return Err(BudgetExceeded::PatternStates {
                limit: self.budget.limits.max_pattern_states,
            });
        }
        Ok(StateReservation {
            meter: Arc::clone(self),
            held: n,
        })
    }
}

/// An RAII reservation of active-state slots on a [`PatternMeter`]. Holds a
/// count of slots and releases exactly that many when dropped, so every exit
/// path (including `?` early-returns) restores the live-state accounting.
#[must_use]
pub struct StateReservation {
    meter: Arc<PatternMeter>,
    held: usize,
}

impl StateReservation {
    /// Grow this reservation by `extra` slots, failing if that would exceed the
    /// per-match active-state ceiling (across all live frontiers). Used as a
    /// frontier is built incrementally so runaway fan-out fails fast.
    pub fn grow(&mut self, extra: usize) -> Result<(), BudgetExceeded> {
        let prev = self.meter.active_states.fetch_add(extra, Ordering::Relaxed);
        if prev + extra > self.meter.budget.limits.max_pattern_states {
            self.meter.active_states.fetch_sub(extra, Ordering::Relaxed);
            return Err(BudgetExceeded::PatternStates {
                limit: self.meter.budget.limits.max_pattern_states,
            });
        }
        self.held += extra;
        Ok(())
    }

    /// Release `n` previously-reserved slots (e.g. when a generation is fully
    /// consumed), keeping the remainder reserved. Saturates at the amount held.
    pub fn release(&mut self, n: usize) {
        let n = n.min(self.held);
        self.held -= n;
        self.meter.active_states.fetch_sub(n, Ordering::Relaxed);
    }
}

impl Drop for StateReservation {
    fn drop(&mut self) {
        self.meter
            .active_states
            .fetch_sub(self.held, Ordering::Relaxed);
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
            max_file_read_bytes: 8,
            max_request_body_bytes: 8,
            max_response_bytes: 8,
            max_pending_requests: 2,
            max_request_duration: None,
            max_ws_queue: 2,
            max_ws_connections: 2,
            max_ws_message_bytes: 8,
            max_ws_queued_bytes: 16,
        }
    }

    /// Spin until the budget's monotonic clock has advanced past creation, so a
    /// zero-second deadline is genuinely elapsed (`elapsed() > 0`). `Instant`
    /// resolution is coarse on some platforms (notably Windows), where the first
    /// charge can otherwise land within the same tick and read `elapsed() == 0`.
    /// The wait is bounded by an iteration cap — not a wall-clock timeout, which
    /// would depend on the very clock we suspect — so a pathologically frozen
    /// clock fails loudly instead of hanging the suite.
    fn wait_for_clock_to_advance(budget: &ExecutionBudget) {
        const MAX_SPINS: u64 = 100_000_000;
        for _ in 0..MAX_SPINS {
            if budget.elapsed() != Duration::ZERO {
                return;
            }
            std::hint::spin_loop();
        }
        panic!(
            "monotonic clock did not advance past budget creation within {MAX_SPINS} spins; \
             cannot exercise the zero-second deadline"
        );
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
        // The deadline trips when `elapsed() > limit`, so a zero-second deadline
        // is only "elapsed" once the monotonic clock has advanced past creation.
        // Wait for that (bounded) so the assertion is deterministic rather than
        // racing the timer resolution.
        wait_for_clock_to_advance(&budget);
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
    fn pattern_meter_is_per_match_and_charges_transitions() {
        let budget = Arc::new(ExecutionBudget::new(tiny_limits())); // max_pattern_steps = 5
        let meter = PatternMeter::new(Arc::clone(&budget));
        // Five transitions (indices 0..=4) fit; the sixth (index 5 >= 5) trips.
        for _ in 0..5 {
            meter.charge_step().expect("within pattern budget");
        }
        assert_eq!(
            meter.charge_step(),
            Err(BudgetExceeded::PatternSteps { limit: 5 })
        );
        // A direct top-level VM op resets *its own* meter for the next match.
        meter.reset();
        assert!(meter.charge_step().is_ok());

        // A second match sharing the same run budget gets an INDEPENDENT meter,
        // so it cannot reset or borrow the first meter's transition count.
        let other = PatternMeter::new(Arc::clone(&budget));
        for _ in 0..5 {
            other.charge_step().expect("independent per-match quota");
        }
        assert_eq!(
            other.charge_step(),
            Err(BudgetExceeded::PatternSteps { limit: 5 })
        );
    }

    #[test]
    fn pattern_meter_reserves_states_across_frontiers() {
        let budget = Arc::new(ExecutionBudget::new(tiny_limits())); // max_pattern_states = 4
        let meter = PatternMeter::new(Arc::clone(&budget));
        // A "current" frontier of 3 plus a "next" frontier growing to 1 = 4 fits.
        let _current = meter.reserve_states(3).expect("current frontier");
        let mut next = meter.reserve_states(0).expect("next frontier");
        next.grow(1).expect("one more still fits (total 4)");
        // A fifth simultaneously-live slot (e.g. a nested lookaround frontier)
        // exceeds the ceiling even though no single frontier does.
        assert_eq!(
            meter.reserve_states(1).map(|_| ()),
            Err(BudgetExceeded::PatternStates { limit: 4 })
        );
        // Releasing a frontier frees its slots for reuse.
        drop(_current);
        let _reused = meter.reserve_states(3).expect("slots freed on drop");
    }

    #[test]
    fn pattern_meter_deadline_exemption_is_read_live() {
        let mut limits = tiny_limits();
        limits.max_duration = Some(Duration::from_secs(0));
        let budget = Arc::new(ExecutionBudget::new(limits));
        // See `deadline_trips_when_elapsed`: the zero-second deadline only trips
        // once the monotonic clock has moved past creation. Wait (bounded) so the
        // post-main-loop assertion below is deterministic on coarse-resolution
        // timers (e.g. Windows).
        wait_for_clock_to_advance(&budget);
        // ONE meter reused across a main-loop boundary (the VM-reuse case);
        // `reset()` runs per top-level op, so each op's first charge (index 0) is
        // a sample point.
        let meter = PatternMeter::new(Arc::clone(&budget));
        // Exempt (inside a `main loop`): a zero-second deadline is not enforced.
        let guard = budget.enter_main_loop();
        assert!(budget.is_deadline_exempt());
        meter.reset();
        assert!(meter.charge_step().is_ok());
        // Leaving the main loop: the same meter now enforces the elapsed
        // deadline (the exemption is read live, not snapshotted at creation).
        drop(guard);
        assert!(!budget.is_deadline_exempt());
        meter.reset();
        assert_eq!(
            meter.charge_step(),
            Err(BudgetExceeded::Deadline { limit_secs: 0 })
        );
    }

    #[test]
    fn main_loop_guard_nests_and_restores_on_drop() {
        let budget = Arc::new(ExecutionBudget::new(tiny_limits()));
        assert!(!budget.is_deadline_exempt());
        let outer = budget.enter_main_loop();
        assert_eq!(budget.main_loop_depth(), 1);
        {
            let _inner = budget.enter_main_loop();
            assert_eq!(budget.main_loop_depth(), 2);
        }
        // Inner dropped: still exempt because the outer loop is active.
        assert_eq!(budget.main_loop_depth(), 1);
        assert!(budget.is_deadline_exempt());
        drop(outer);
        assert_eq!(budget.main_loop_depth(), 0);
        assert!(!budget.is_deadline_exempt());
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
        assert!(budget.check_file_read_bytes(8).is_ok());
        assert_eq!(
            budget.check_file_read_bytes(9),
            Err(BudgetExceeded::FileReadBytes {
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
    fn ws_byte_permit_bounds_queued_payload() {
        // tiny_limits: max_ws_message_bytes = 8, max_ws_queued_bytes = 16.
        let budget = Arc::new(ExecutionBudget::new(tiny_limits()));
        // A single frame larger than the per-message cap is refused outright.
        assert!(budget.try_reserve_ws_bytes(9).is_none());
        // Two 8-byte frames fill the 16-byte global ceiling.
        let a = budget.try_reserve_ws_bytes(8).expect("first frame");
        let b = budget.try_reserve_ws_bytes(8).expect("second frame");
        assert_eq!(budget.ws_queued_bytes(), 16);
        // A third frame exceeds the global ceiling and is shed.
        assert!(budget.try_reserve_ws_bytes(1).is_none());
        // Consuming a frame frees its bytes for reuse.
        drop(a);
        assert_eq!(budget.ws_queued_bytes(), 8);
        let _c = budget.try_reserve_ws_bytes(8).expect("bytes freed on drop");
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
            max_file_read_size: 123,
            ..Default::default()
        };
        let limits = BudgetLimits::from_config(&config);
        assert_eq!(limits.max_duration, Some(Duration::from_secs(42)));
        assert_eq!(limits.max_request_body_bytes, 4096);
        assert_eq!(limits.max_pending_requests, 7);
        assert_eq!(limits.max_file_read_bytes, 123);
    }

    #[test]
    fn response_timeout_zero_disables() {
        let config = WflConfig {
            web_server_response_timeout_seconds: 0,
            ..Default::default()
        };
        assert_eq!(
            BudgetLimits::from_config(&config).max_request_duration,
            None
        );
        let config = WflConfig {
            web_server_response_timeout_seconds: 45,
            ..Default::default()
        };
        assert_eq!(
            BudgetLimits::from_config(&config).max_request_duration,
            Some(Duration::from_secs(45))
        );
    }

    #[test]
    fn budget_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ExecutionBudget>();
        assert_send_sync::<Arc<ExecutionBudget>>();
    }
}
