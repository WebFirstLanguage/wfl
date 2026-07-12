use std::path::Path;
use std::str::FromStr;

#[cfg(windows)]
const DEFAULT_GLOBAL_CONFIG_PATH: &str = "C:\\wfl\\config";

#[cfg(not(windows))]
const DEFAULT_GLOBAL_CONFIG_PATH: &str = "/etc/wfl/wfl.cfg";

fn get_global_config_path() -> &'static str {
    std::env::var("WFL_GLOBAL_CONFIG_PATH")
        .ok()
        .map(|path| Box::leak(path.into_boxed_str()))
        .map_or(DEFAULT_GLOBAL_CONFIG_PATH, |v| v)
}

#[derive(Debug, Clone)]
pub struct WflConfig {
    pub timeout_seconds: u64,
    pub logging_enabled: bool,
    pub debug_report_enabled: bool,
    pub log_level: LogLevel,
    pub execution_logging: bool,
    // Enhanced execution logging controls
    pub verbose_execution: bool, // Controls detailed per-statement logging
    pub log_loop_iterations: bool, // Whether to log loop iterations
    pub log_throttle_factor: usize, // Log every Nth iteration in loops
    // Code quality suite settings
    pub max_line_length: usize,
    pub max_nesting_depth: usize,
    pub indent_size: usize,
    pub snake_case_variables: bool,
    pub trailing_whitespace: bool,
    pub consistent_keyword_case: bool,
    // Subprocess security settings
    pub allow_shell_execution: bool,
    pub shell_execution_mode: ShellExecutionMode,
    pub allowed_shell_commands: Vec<String>,
    pub warn_on_shell_execution: bool,
    // Subprocess resource management
    pub subprocess_config: SubprocessConfig,
    // Web server network binding
    pub web_server_bind_address: String,
    // Web server TLS defaults: used by `listen ... secured` when the listen
    // statement does not name certificate/key files itself
    pub web_server_tls_cert_file: Option<String>,
    pub web_server_tls_key_file: Option<String>,
    /// Maximum accepted HTTP request body size in bytes (DoS protection).
    /// Default 1 MiB; raise for media uploads via `.wflcfg`.
    pub web_server_max_body_size: usize,
    /// Maximum number of accepted-but-not-yet-handled HTTP requests held in the
    /// transport→interpreter queue (DoS protection). When the queue is full the
    /// server sheds new requests with a 503 instead of growing memory without
    /// bound. Default 256; must be at least 1.
    pub web_server_request_queue_bound: usize,
    /// Maximum HTTP response body size in bytes. A handler that tries to send a
    /// larger body is refused with a 500 rather than streaming an unbounded
    /// payload. Feeds `ExecutionBudget`. Default 64 MiB.
    pub web_server_max_response_size: usize,
    /// Maximum seconds the transport waits for a handler to answer an accepted
    /// HTTP request before shedding it with 504 and releasing its in-flight
    /// slot. `0` disables the timeout. Feeds `ExecutionBudget`. Default 300.
    pub web_server_response_timeout_seconds: u64,
    // --- Shared ExecutionBudget limits (see src/exec/budget.rs) ---
    /// Hard ceiling on charged interpreter operations. `None`/`0` = unlimited
    /// (the default, matching historic behavior). Feeds `ExecutionBudget`.
    pub max_operations: Option<u64>,
    /// Maximum WFL call/recursion depth before a clean error replaces a native
    /// stack overflow. Feeds `ExecutionBudget`. Default 1000.
    pub max_call_depth: usize,
    /// Maximum nested `load module` / `include` depth. Feeds `ExecutionBudget`.
    /// Default 64.
    pub max_import_depth: usize,
    /// Maximum `execute file` nesting depth. Kept small because each level
    /// re-enters the whole interpreter recursively. Feeds `ExecutionBudget`.
    /// Default 4.
    pub max_execute_file_depth: usize,
    /// Maximum pattern-VM transitions (instructions) per match operation (ReDoS
    /// guard). Feeds `ExecutionBudget`. Default 5000000.
    pub max_pattern_steps: usize,
    /// Maximum simultaneously-active pattern-VM states per match attempt. Feeds
    /// `ExecutionBudget`. Default 10000.
    pub max_pattern_states: usize,
    /// Maximum WFL source-file size in bytes. Feeds `ExecutionBudget`.
    /// Default 64 MiB.
    pub max_source_size: usize,
    /// Maximum queued frames/events per WebSocket channel before shedding.
    /// Feeds `ExecutionBudget`. Default 1024; must be at least 1.
    pub web_socket_queue_bound: usize,
    /// Maximum simultaneous live WebSocket connections. Feeds `ExecutionBudget`.
    /// Default 1024; must be at least 1.
    pub web_socket_max_connections: usize,
    /// Maximum size in bytes of a single WebSocket text message (inbound or
    /// outbound); larger frames are dropped rather than queued. Feeds
    /// `ExecutionBudget`. Default 1 MiB; must be at least 1.
    pub web_socket_max_message_size: usize,
    /// Global ceiling in bytes on WebSocket payloads queued across every
    /// connection's inbound event and outbound frame channels; reservations are
    /// released as frames are consumed or shed, so a slow/absent consumer cannot
    /// buffer without bound. Feeds `ExecutionBudget`. Default 16 MiB; must be at
    /// least 1.
    pub web_socket_max_queued_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellExecutionMode {
    Forbidden,     // No shell execution allowed
    AllowlistOnly, // Only allowlisted commands
    Sanitized,     // Shell with validation/warnings
    Unrestricted,  // Legacy mode (not recommended)
}

#[derive(Debug, Clone)]
pub struct SubprocessConfig {
    pub max_concurrent_processes: usize,
    pub max_buffer_size_bytes: usize,
    pub enable_auto_cleanup: bool,
    pub enable_reaper: bool,
    pub reaper_interval_secs: u64,
    pub kill_on_shutdown: bool,
    pub warn_on_orphan: bool,
}

impl Default for SubprocessConfig {
    fn default() -> Self {
        Self {
            max_concurrent_processes: 100,
            max_buffer_size_bytes: 10 * 1024 * 1024, // 10 MB
            enable_auto_cleanup: true,
            enable_reaper: false,
            reaper_interval_secs: 30,
            kill_on_shutdown: false,
            warn_on_orphan: true,
        }
    }
}

impl Default for WflConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 60,
            logging_enabled: false,
            debug_report_enabled: true,
            log_level: LogLevel::Info,
            #[cfg(debug_assertions)]
            execution_logging: true, // Enable by default in debug builds
            #[cfg(not(debug_assertions))]
            execution_logging: false, // Disable by default in release builds
            // Enhanced execution logging defaults - less verbose by default
            verbose_execution: false, // Disable verbose per-statement logging
            log_loop_iterations: false, // Disable loop iteration logging by default
            log_throttle_factor: 1000, // Log every 1000th iteration when enabled
            // Code quality suite defaults - strict by default
            max_line_length: 100,
            max_nesting_depth: 5,
            indent_size: 4,
            snake_case_variables: true,
            trailing_whitespace: false, // false means no trailing whitespace allowed
            consistent_keyword_case: true,
            // Subprocess security defaults (secure by default)
            allow_shell_execution: false,
            shell_execution_mode: ShellExecutionMode::Forbidden,
            allowed_shell_commands: Vec::new(),
            warn_on_shell_execution: true,
            // Subprocess resource management defaults
            subprocess_config: SubprocessConfig::default(),
            // Web server network binding default
            web_server_bind_address: "127.0.0.1".to_string(),
            // No TLS defaults: a `listen ... secured` statement must either
            // name its certificate/key files or these must be set in .wflcfg
            web_server_tls_cert_file: None,
            web_server_tls_key_file: None,
            // 1 MiB default body limit (DoS protection); raise for uploads
            web_server_max_body_size: 1_048_576,
            // Bound the accept queue so a flood sheds with 503 rather than
            // growing memory without bound. Aligns with the Phase 1 in-flight cap.
            web_server_request_queue_bound: 256,
            // 64 MiB default response body limit (DoS protection).
            web_server_max_response_size: 64 * 1024 * 1024,
            // Free an accepted request's in-flight slot if its handler does not
            // answer within 5 minutes (far longer than any serial handler needs).
            web_server_response_timeout_seconds: 300,
            // Shared ExecutionBudget limits (see src/exec/budget.rs). Defaults
            // are chosen so existing programs never trip them while runaway
            // behavior gets a clean error instead of a crash or OOM.
            max_operations: None,
            max_call_depth: 1_000,
            max_import_depth: 64,
            max_execute_file_depth: 4,
            max_pattern_steps: 5_000_000,
            max_pattern_states: 10_000,
            max_source_size: 64 * 1024 * 1024,
            web_socket_queue_bound: 1_024,
            web_socket_max_connections: 1_024,
            web_socket_max_message_size: 1_048_576,
            web_socket_max_queued_bytes: 16 * 1_048_576,
        }
    }
}

// For the FromStr trait implementation
#[derive(Debug)]
pub struct ParseLogLevelError;

impl std::fmt::Display for ParseLogLevelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "failed to parse log level")
    }
}

impl std::error::Error for ParseLogLevelError {}

impl FromStr for LogLevel {
    type Err = ParseLogLevelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.trim().to_lowercase().as_str() {
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" | "warning" => LogLevel::Warn,
            "error" => LogLevel::Error,
            _ => LogLevel::Info, // Default to Info for unrecognized values
        })
    }
}

impl LogLevel {
    // Keep this for backward compatibility
    pub fn parse_str(s: &str) -> Self {
        s.parse().unwrap_or(LogLevel::Info)
    }

    pub fn to_level_filter(&self) -> log::LevelFilter {
        match self {
            LogLevel::Debug => log::LevelFilter::Debug,
            LogLevel::Info => log::LevelFilter::Info,
            LogLevel::Warn => log::LevelFilter::Warn,
            LogLevel::Error => log::LevelFilter::Error,
        }
    }
}

/// Parse a positive-integer `.wflcfg` value into `field`. Rejects zero and
/// non-numeric input with a warning, leaving `field` at its previous value.
/// Shared by the `ExecutionBudget` limit keys, which all require `>= 1`.
fn set_positive_usize(field: &mut usize, key: &str, value: &str, file: &Path) {
    match value.parse::<usize>() {
        Ok(0) | Err(_) => log::warn!(
            "Invalid {key} '{value}' in {}: expected a positive integer",
            file.display()
        ),
        Ok(parsed) => {
            *field = parsed;
            log::debug!("Loaded {key}: {parsed} from {}", file.display());
        }
    }
}

fn parse_config_text(config: &mut WflConfig, text: &str, file: &Path) {
    log::debug!("Parsing config from {}", file.display());
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, rest)) = line.split_once('=') {
            let key = key.trim();
            let value = rest.trim();
            log::debug!("Found config key: {key}, value: {value}");

            match key {
                "timeout_seconds" => {
                    if let Ok(timeout) = value.parse::<u64>() {
                        if config.timeout_seconds != WflConfig::default().timeout_seconds {
                            log::debug!(
                                "Overriding timeout_seconds: {} -> {} from {}",
                                config.timeout_seconds,
                                timeout.max(1),
                                file.display()
                            );
                        }
                        config.timeout_seconds = timeout.max(1);
                        log::debug!(
                            "Loaded timeout override: {} s from {}",
                            config.timeout_seconds,
                            file.display()
                        );
                    }
                }
                "logging_enabled" => {
                    if let Ok(enabled) = value.parse::<bool>() {
                        if config.logging_enabled != WflConfig::default().logging_enabled {
                            log::debug!(
                                "Overriding logging_enabled: {} -> {} from {}",
                                config.logging_enabled,
                                enabled,
                                file.display()
                            );
                        }
                        config.logging_enabled = enabled;
                        log::debug!(
                            "Loaded logging_enabled: {} from {}",
                            config.logging_enabled,
                            file.display()
                        );
                    }
                }
                "debug_report_enabled" => {
                    if let Ok(enabled) = value.parse::<bool>() {
                        if config.debug_report_enabled != WflConfig::default().debug_report_enabled
                        {
                            log::debug!(
                                "Overriding debug_report_enabled: {} -> {} from {}",
                                config.debug_report_enabled,
                                enabled,
                                file.display()
                            );
                        }
                        config.debug_report_enabled = enabled;
                        log::debug!(
                            "Loaded debug_report_enabled: {} from {}",
                            config.debug_report_enabled,
                            file.display()
                        );
                    }
                }
                "execution_logging" => {
                    if let Ok(enabled) = value.parse::<bool>() {
                        if config.execution_logging != WflConfig::default().execution_logging {
                            log::debug!(
                                "Overriding execution_logging: {} -> {} from {}",
                                config.execution_logging,
                                enabled,
                                file.display()
                            );
                        }
                        config.execution_logging = enabled;
                        log::debug!(
                            "Loaded execution_logging: {} from {}",
                            config.execution_logging,
                            file.display()
                        );
                    }
                }
                "verbose_execution" => {
                    if let Ok(enabled) = value.parse::<bool>() {
                        if config.verbose_execution != WflConfig::default().verbose_execution {
                            log::debug!(
                                "Overriding verbose_execution: {} -> {} from {}",
                                config.verbose_execution,
                                enabled,
                                file.display()
                            );
                        }
                        config.verbose_execution = enabled;
                        log::debug!(
                            "Loaded verbose_execution: {} from {}",
                            config.verbose_execution,
                            file.display()
                        );
                    }
                }
                "log_loop_iterations" => {
                    if let Ok(enabled) = value.parse::<bool>() {
                        if config.log_loop_iterations != WflConfig::default().log_loop_iterations {
                            log::debug!(
                                "Overriding log_loop_iterations: {} -> {} from {}",
                                config.log_loop_iterations,
                                enabled,
                                file.display()
                            );
                        }
                        config.log_loop_iterations = enabled;
                        log::debug!(
                            "Loaded log_loop_iterations: {} from {}",
                            config.log_loop_iterations,
                            file.display()
                        );
                    }
                }
                "log_throttle_factor" => {
                    if let Ok(factor) = value.parse::<usize>() {
                        if config.log_throttle_factor != WflConfig::default().log_throttle_factor {
                            log::debug!(
                                "Overriding log_throttle_factor: {} -> {} from {}",
                                config.log_throttle_factor,
                                factor.max(1),
                                file.display()
                            );
                        }
                        config.log_throttle_factor = factor.max(1);
                        log::debug!(
                            "Loaded log_throttle_factor: {} from {}",
                            config.log_throttle_factor,
                            file.display()
                        );
                    }
                }
                "log_level" => {
                    if config.log_level != WflConfig::default().log_level {
                        log::debug!(
                            "Overriding log_level: {:?} -> {:?} from {}",
                            config.log_level,
                            LogLevel::parse_str(value),
                            file.display()
                        );
                    }
                    config.log_level = LogLevel::parse_str(value);
                    log::debug!(
                        "Loaded log_level: {:?} from {}",
                        config.log_level,
                        file.display()
                    );
                }
                // Code quality suite settings
                "max_line_length" => {
                    if let Ok(length) = value.parse::<usize>() {
                        if config.max_line_length != WflConfig::default().max_line_length {
                            log::debug!(
                                "Overriding max_line_length: {} -> {} from {}",
                                config.max_line_length,
                                length,
                                file.display()
                            );
                        }
                        config.max_line_length = length;
                        log::debug!(
                            "Loaded max_line_length: {} from {}",
                            config.max_line_length,
                            file.display()
                        );
                    }
                }
                "max_nesting_depth" => {
                    if let Ok(depth) = value.parse::<usize>() {
                        if config.max_nesting_depth != WflConfig::default().max_nesting_depth {
                            log::debug!(
                                "Overriding max_nesting_depth: {} -> {} from {}",
                                config.max_nesting_depth,
                                depth,
                                file.display()
                            );
                        }
                        config.max_nesting_depth = depth;
                        log::debug!(
                            "Loaded max_nesting_depth: {} from {}",
                            config.max_nesting_depth,
                            file.display()
                        );
                    }
                }
                "indent_size" => {
                    if let Ok(size) = value.parse::<usize>() {
                        if config.indent_size != WflConfig::default().indent_size {
                            log::debug!(
                                "Overriding indent_size: {} -> {} from {}",
                                config.indent_size,
                                size,
                                file.display()
                            );
                        }
                        config.indent_size = size;
                        log::debug!(
                            "Loaded indent_size: {} from {}",
                            config.indent_size,
                            file.display()
                        );
                    }
                }
                "snake_case_variables" => {
                    if let Ok(enabled) = value.parse::<bool>() {
                        if config.snake_case_variables != WflConfig::default().snake_case_variables
                        {
                            log::debug!(
                                "Overriding snake_case_variables: {} -> {} from {}",
                                config.snake_case_variables,
                                enabled,
                                file.display()
                            );
                        }
                        config.snake_case_variables = enabled;
                        log::debug!(
                            "Loaded snake_case_variables: {} from {}",
                            config.snake_case_variables,
                            file.display()
                        );
                    }
                }
                "trailing_whitespace" => {
                    if let Ok(enabled) = value.parse::<bool>() {
                        if config.trailing_whitespace != WflConfig::default().trailing_whitespace {
                            log::debug!(
                                "Overriding trailing_whitespace: {} -> {} from {}",
                                config.trailing_whitespace,
                                enabled,
                                file.display()
                            );
                        }
                        config.trailing_whitespace = enabled;
                        log::debug!(
                            "Loaded trailing_whitespace: {} from {}",
                            config.trailing_whitespace,
                            file.display()
                        );
                    }
                }
                "consistent_keyword_case" => {
                    if let Ok(enabled) = value.parse::<bool>() {
                        if config.consistent_keyword_case
                            != WflConfig::default().consistent_keyword_case
                        {
                            log::debug!(
                                "Overriding consistent_keyword_case: {} -> {} from {}",
                                config.consistent_keyword_case,
                                enabled,
                                file.display()
                            );
                        }
                        config.consistent_keyword_case = enabled;
                        log::debug!(
                            "Loaded consistent_keyword_case: {} from {}",
                            config.consistent_keyword_case,
                            file.display()
                        );
                    }
                }
                // Subprocess security settings
                "allow_shell_execution" => {
                    if let Ok(enabled) = value.parse::<bool>() {
                        if config.allow_shell_execution
                            != WflConfig::default().allow_shell_execution
                        {
                            log::debug!(
                                "Overriding allow_shell_execution: {} -> {} from {}",
                                config.allow_shell_execution,
                                enabled,
                                file.display()
                            );
                        }
                        config.allow_shell_execution = enabled;
                        log::debug!(
                            "Loaded allow_shell_execution: {} from {}",
                            config.allow_shell_execution,
                            file.display()
                        );
                    }
                }
                "shell_execution_mode" => {
                    let mode = match value.trim().to_lowercase().as_str() {
                        "forbidden" => ShellExecutionMode::Forbidden,
                        "allowlist_only" | "allowlistonly" => ShellExecutionMode::AllowlistOnly,
                        "sanitized" => ShellExecutionMode::Sanitized,
                        "unrestricted" => ShellExecutionMode::Unrestricted,
                        _ => {
                            log::warn!("Unknown shell_execution_mode: {}, using default", value);
                            ShellExecutionMode::Forbidden
                        }
                    };
                    if config.shell_execution_mode != WflConfig::default().shell_execution_mode {
                        log::debug!(
                            "Overriding shell_execution_mode: {:?} -> {:?} from {}",
                            config.shell_execution_mode,
                            mode,
                            file.display()
                        );
                    }
                    config.shell_execution_mode = mode;
                    log::debug!(
                        "Loaded shell_execution_mode: {:?} from {}",
                        config.shell_execution_mode,
                        file.display()
                    );
                }
                "allowed_shell_commands" => {
                    // Parse comma-separated list of allowed commands
                    let commands: Vec<String> = value
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();

                    if !config.allowed_shell_commands.is_empty() {
                        log::debug!("Overriding allowed_shell_commands from {}", file.display());
                    }
                    config.allowed_shell_commands = commands;
                    log::debug!(
                        "Loaded allowed_shell_commands: {:?} from {}",
                        config.allowed_shell_commands,
                        file.display()
                    );
                }
                "warn_on_shell_execution" => {
                    if let Ok(enabled) = value.parse::<bool>() {
                        if config.warn_on_shell_execution
                            != WflConfig::default().warn_on_shell_execution
                        {
                            log::debug!(
                                "Overriding warn_on_shell_execution: {} -> {} from {}",
                                config.warn_on_shell_execution,
                                enabled,
                                file.display()
                            );
                        }
                        config.warn_on_shell_execution = enabled;
                        log::debug!(
                            "Loaded warn_on_shell_execution: {} from {}",
                            config.warn_on_shell_execution,
                            file.display()
                        );
                    }
                }
                // Subprocess resource management settings
                "max_concurrent_processes" => {
                    if let Ok(limit) = value.parse::<usize>() {
                        config.subprocess_config.max_concurrent_processes = limit;
                        log::debug!(
                            "Loaded max_concurrent_processes: {} from {}",
                            limit,
                            file.display()
                        );
                    }
                }
                "max_buffer_size_bytes" => {
                    if let Ok(size) = value.parse::<usize>() {
                        config.subprocess_config.max_buffer_size_bytes = size;
                        log::debug!(
                            "Loaded max_buffer_size_bytes: {} from {}",
                            size,
                            file.display()
                        );
                    }
                }
                "kill_on_shutdown" => {
                    if let Ok(enabled) = value.parse::<bool>() {
                        config.subprocess_config.kill_on_shutdown = enabled;
                        log::debug!(
                            "Loaded kill_on_shutdown: {} from {}",
                            enabled,
                            file.display()
                        );
                    }
                }
                "web_server_bind_address" => {
                    let addr = value.trim().to_string();

                    // Validate IP address
                    if is_valid_ip_address(&addr) {
                        if config.web_server_bind_address
                            != WflConfig::default().web_server_bind_address
                        {
                            log::debug!(
                                "Overriding web_server_bind_address: {} -> {} from {}",
                                config.web_server_bind_address,
                                addr,
                                file.display()
                            );
                        }
                        config.web_server_bind_address = addr;
                        log::debug!(
                            "Loaded web_server_bind_address: {} from {}",
                            config.web_server_bind_address,
                            file.display()
                        );
                    } else {
                        log::warn!(
                            "Invalid web_server_bind_address '{}' in {}: not a valid IP address. Using default '127.0.0.1'",
                            addr,
                            file.display()
                        );
                    }
                }
                "web_server_tls_cert_file" => {
                    let path = value.trim().to_string();
                    if path.is_empty() {
                        log::warn!(
                            "Empty web_server_tls_cert_file in {}: ignoring",
                            file.display()
                        );
                    } else {
                        // Existence is checked at listen time so relative
                        // paths resolve against the script's working directory
                        config.web_server_tls_cert_file = Some(path);
                        log::debug!(
                            "Loaded web_server_tls_cert_file: {:?} from {}",
                            config.web_server_tls_cert_file,
                            file.display()
                        );
                    }
                }
                "web_server_tls_key_file" => {
                    let path = value.trim().to_string();
                    if path.is_empty() {
                        log::warn!(
                            "Empty web_server_tls_key_file in {}: ignoring",
                            file.display()
                        );
                    } else {
                        config.web_server_tls_key_file = Some(path);
                        log::debug!(
                            "Loaded web_server_tls_key_file: {:?} from {}",
                            config.web_server_tls_key_file,
                            file.display()
                        );
                    }
                }
                "web_server_max_body_size" => {
                    if let Ok(size) = value.parse::<usize>() {
                        // Reject zero so a misconfigured limit cannot lock out all POSTs
                        if size == 0 {
                            log::warn!(
                                "Invalid web_server_max_body_size '0' in {}: must be at least 1. Keeping {}",
                                file.display(),
                                config.web_server_max_body_size
                            );
                        } else {
                            if config.web_server_max_body_size
                                != WflConfig::default().web_server_max_body_size
                            {
                                log::debug!(
                                    "Overriding web_server_max_body_size: {} -> {} from {}",
                                    config.web_server_max_body_size,
                                    size,
                                    file.display()
                                );
                            }
                            config.web_server_max_body_size = size;
                            log::debug!(
                                "Loaded web_server_max_body_size: {} from {}",
                                config.web_server_max_body_size,
                                file.display()
                            );
                        }
                    } else {
                        log::warn!(
                            "Invalid web_server_max_body_size '{}' in {}: expected a positive integer",
                            value,
                            file.display()
                        );
                    }
                }
                "web_server_request_queue_bound" => {
                    if let Ok(bound) = value.parse::<usize>() {
                        // Reject zero: `tokio::sync::mpsc::channel(0)` panics, and a
                        // zero-length queue could never accept a request.
                        if bound == 0 {
                            log::warn!(
                                "Invalid web_server_request_queue_bound '0' in {}: must be at least 1. Keeping {}",
                                file.display(),
                                config.web_server_request_queue_bound
                            );
                        } else {
                            if config.web_server_request_queue_bound
                                != WflConfig::default().web_server_request_queue_bound
                            {
                                log::debug!(
                                    "Overriding web_server_request_queue_bound: {} -> {} from {}",
                                    config.web_server_request_queue_bound,
                                    bound,
                                    file.display()
                                );
                            }
                            config.web_server_request_queue_bound = bound;
                            log::debug!(
                                "Loaded web_server_request_queue_bound: {} from {}",
                                config.web_server_request_queue_bound,
                                file.display()
                            );
                        }
                    } else {
                        log::warn!(
                            "Invalid web_server_request_queue_bound '{}' in {}: expected a positive integer",
                            value,
                            file.display()
                        );
                    }
                }
                "web_server_max_response_size" => match value.parse::<usize>() {
                    Ok(0) | Err(_) => log::warn!(
                        "Invalid web_server_max_response_size '{}' in {}: expected a positive integer",
                        value,
                        file.display()
                    ),
                    Ok(size) => {
                        config.web_server_max_response_size = size;
                        log::debug!(
                            "Loaded web_server_max_response_size: {size} from {}",
                            file.display()
                        );
                    }
                },
                "web_server_response_timeout_seconds" => match value.parse::<u64>() {
                    Ok(secs) => {
                        // 0 disables the timeout (the documented sentinel).
                        config.web_server_response_timeout_seconds = secs;
                        log::debug!(
                            "Loaded web_server_response_timeout_seconds: {secs} from {}",
                            file.display()
                        );
                    }
                    Err(_) => log::warn!(
                        "Invalid web_server_response_timeout_seconds '{}' in {}: expected a non-negative integer",
                        value,
                        file.display()
                    ),
                },
                "max_operations" => match value.parse::<u64>() {
                    Ok(n) => {
                        // 0 means "no operation ceiling" (the default).
                        config.max_operations = if n == 0 { None } else { Some(n) };
                        log::debug!(
                            "Loaded max_operations: {:?} from {}",
                            config.max_operations,
                            file.display()
                        );
                    }
                    Err(_) => log::warn!(
                        "Invalid max_operations '{}' in {}: expected a non-negative integer",
                        value,
                        file.display()
                    ),
                },
                "max_call_depth" => {
                    set_positive_usize(&mut config.max_call_depth, "max_call_depth", value, file)
                }
                "max_import_depth" => set_positive_usize(
                    &mut config.max_import_depth,
                    "max_import_depth",
                    value,
                    file,
                ),
                "max_execute_file_depth" => set_positive_usize(
                    &mut config.max_execute_file_depth,
                    "max_execute_file_depth",
                    value,
                    file,
                ),
                "max_pattern_steps" => set_positive_usize(
                    &mut config.max_pattern_steps,
                    "max_pattern_steps",
                    value,
                    file,
                ),
                "max_pattern_states" => set_positive_usize(
                    &mut config.max_pattern_states,
                    "max_pattern_states",
                    value,
                    file,
                ),
                "max_source_size" => {
                    set_positive_usize(&mut config.max_source_size, "max_source_size", value, file)
                }
                "web_socket_queue_bound" => set_positive_usize(
                    &mut config.web_socket_queue_bound,
                    "web_socket_queue_bound",
                    value,
                    file,
                ),
                "web_socket_max_connections" => set_positive_usize(
                    &mut config.web_socket_max_connections,
                    "web_socket_max_connections",
                    value,
                    file,
                ),
                "web_socket_max_message_size" => set_positive_usize(
                    &mut config.web_socket_max_message_size,
                    "web_socket_max_message_size",
                    value,
                    file,
                ),
                "web_socket_max_queued_bytes" => set_positive_usize(
                    &mut config.web_socket_max_queued_bytes,
                    "web_socket_max_queued_bytes",
                    value,
                    file,
                ),
                _ => {
                    log::warn!("Unknown configuration key: {} in {}", key, file.display());
                }
            }
        }
    }
}

pub fn load_config(dir: &Path) -> WflConfig {
    // Start with default configuration
    let mut config = WflConfig::default();

    // Try to load global configuration
    let global_config = Path::new(get_global_config_path());
    let mut loaded_global = false;

    if global_config.exists()
        && let Ok(text) = std::fs::read_to_string(global_config)
    {
        loaded_global = true;
        log::debug!(
            "Loading global configuration from {}",
            global_config.display()
        );
        parse_config_text(&mut config, &text, global_config);
    }

    if !loaded_global {
        let old_system_config = Path::new("/etc/wfl/.wflcfg");
        if old_system_config.exists()
            && let Ok(text) = std::fs::read_to_string(old_system_config)
        {
            log::debug!(
                "Loading global configuration from {} (legacy path)",
                old_system_config.display()
            );
            parse_config_text(&mut config, &text, old_system_config);
        }
    }

    // Walk up directory tree looking for .wflcfg (closest wins)
    // Canonicalize the starting directory to resolve relative paths
    let mut current_dir = Some(std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf()));
    let mut found_local_config = None;

    while let Some(dir_path) = current_dir {
        let config_path = dir_path.join(".wflcfg");
        if config_path.exists() {
            found_local_config = Some(config_path);
            break; // Stop at first .wflcfg found
        }
        current_dir = dir_path.parent().map(Path::to_path_buf);
    }

    if let Some(local_config) = found_local_config
        && let Ok(text) = std::fs::read_to_string(&local_config)
    {
        log::debug!(
            "Loading local configuration from {}",
            local_config.display()
        );
        parse_config_text(&mut config, &text, &local_config);
    }

    config
}

pub fn load_config_with_global(script_dir: &Path) -> WflConfig {
    // Start with default configuration
    let mut config = WflConfig::default();

    // Try to load global configuration
    let global_config = Path::new(get_global_config_path());
    let mut loaded_global = false;

    if global_config.exists()
        && let Ok(text) = std::fs::read_to_string(global_config)
    {
        loaded_global = true;
        log::debug!(
            "Loading global configuration from {}",
            global_config.display()
        );
        parse_config_text(&mut config, &text, global_config);
    }

    if !loaded_global {
        let old_system_config = Path::new("/etc/wfl/.wflcfg");
        if old_system_config.exists()
            && let Ok(text) = std::fs::read_to_string(old_system_config)
        {
            log::debug!(
                "Loading global configuration from {} (legacy path)",
                old_system_config.display()
            );
            parse_config_text(&mut config, &text, old_system_config);
        }
    }

    // Walk up directory tree looking for .wflcfg (closest wins)
    // Canonicalize the starting directory to resolve relative paths
    let mut current_dir =
        Some(std::fs::canonicalize(script_dir).unwrap_or_else(|_| script_dir.to_path_buf()));
    let mut found_local_config = None;

    while let Some(dir_path) = current_dir {
        let config_path = dir_path.join(".wflcfg");
        if config_path.exists() {
            found_local_config = Some(config_path);
            break; // Stop at first .wflcfg found
        }
        current_dir = dir_path.parent().map(Path::to_path_buf);
    }

    if let Some(local_config) = found_local_config
        && let Ok(text) = std::fs::read_to_string(&local_config)
    {
        log::debug!(
            "Loading local configuration from {}",
            local_config.display()
        );
        parse_config_text(&mut config, &text, &local_config);
    }

    config
}

pub fn load_timeout(dir: &Path) -> u64 {
    load_config(dir).timeout_seconds
}

/// Validates that a string is a valid IPv4 or IPv6 address
fn is_valid_ip_address(addr: &str) -> bool {
    use std::net::IpAddr;
    addr.parse::<IpAddr>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::sync::Mutex;
    use tempfile::tempdir;

    // Mutex to serialize config tests that modify environment variables
    // This prevents test interference when tests run in parallel
    static TEST_ENV_LOCK: Mutex<()> = Mutex::new(());

    #[cfg(test)]
    fn set_test_env_var(val: Option<&str>) {
        match val {
            Some(v) => unsafe { ::std::env::set_var("WFL_GLOBAL_CONFIG_PATH", v) },
            None => unsafe { ::std::env::remove_var("WFL_GLOBAL_CONFIG_PATH") },
        }
    }

    fn with_test_global_path<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        // Acquire lock to serialize tests that modify environment variables
        let _guard = TEST_ENV_LOCK.lock().unwrap();

        let original = std::env::var("WFL_GLOBAL_CONFIG_PATH").ok();

        let result = f();

        #[cfg(test)]
        match original {
            Some(val) => set_test_env_var(Some(&val)),
            None => set_test_env_var(None),
        }

        result
    }

    #[test]
    fn test_load_timeout_default() {
        let temp_dir = tempfile::tempdir().unwrap();
        let timeout = with_test_global_path(|| {
            // Explicitly set a non-existent path to ensure we don't pick up any global config
            set_test_env_var(Some("/non/existent/path"));
            load_timeout(temp_dir.path())
        });
        assert_eq!(timeout, 60);
    }

    #[test]
    fn test_load_timeout_custom() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(".wflcfg");

        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(b"timeout_seconds = 120").unwrap();

        let timeout = load_timeout(temp_dir.path());
        assert_eq!(timeout, 120);
    }

    #[test]
    fn test_load_timeout_with_comments() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(".wflcfg");

        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(b"# This is a comment\ntimeout_seconds = 45\n# Another comment")
            .unwrap();

        let timeout = load_timeout(temp_dir.path());
        assert_eq!(timeout, 45);
    }

    #[test]
    fn test_load_timeout_invalid() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(".wflcfg");

        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(b"timeout_seconds = invalid").unwrap();

        let timeout = with_test_global_path(|| {
            set_test_env_var(Some("/non/existent/path"));
            load_timeout(temp_dir.path())
        });
        assert_eq!(timeout, 60); // Should fall back to default
    }

    #[test]
    fn test_load_config_defaults() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Use with_test_global_path to ensure we don't pick up any global config
        let config = with_test_global_path(|| {
            // Explicitly set a non-existent path to ensure we don't pick up any global config
            set_test_env_var(Some("/non/existent/path"));
            load_config(temp_dir.path())
        });

        // Verify default configuration values
        assert_eq!(config.timeout_seconds, 60);
        assert!(!config.logging_enabled);
        assert!(config.debug_report_enabled);
        assert_eq!(config.log_level, LogLevel::Info);
    }

    #[test]
    fn test_load_config_custom() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(".wflcfg");

        let config_content = r#"
        # WFL Configuration
        timeout_seconds = 120
        logging_enabled = true
        debug_report_enabled = false
        log_level = debug
        "#;

        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = load_config(temp_dir.path());

        assert_eq!(config.timeout_seconds, 120);
        assert!(config.logging_enabled);
        assert!(!config.debug_report_enabled);
        assert_eq!(config.log_level, LogLevel::Debug);
    }

    #[test]
    fn test_load_config_partial() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(".wflcfg");

        let config_content = r#"
        # Only specify some settings
        timeout_seconds = 30
        log_level = error
        "#;

        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = with_test_global_path(|| {
            set_test_env_var(Some("/non/existent/path"));
            load_config(temp_dir.path())
        });

        assert_eq!(config.timeout_seconds, 30);
        assert!(!config.logging_enabled); // Default
        assert!(config.debug_report_enabled); // Default
        assert_eq!(config.log_level, LogLevel::Error);
    }

    #[test]
    fn test_log_level_parsing() {
        assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("INFO".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("Warning".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("ERROR".parse::<LogLevel>().unwrap(), LogLevel::Error);
        assert_eq!("unknown".parse::<LogLevel>().unwrap(), LogLevel::Info); // Default
    }

    #[test]
    fn test_load_config_global_only() {
        // Create a temporary directory to hold our global config file
        let temp_dir = tempdir().unwrap();
        let global_config_path = temp_dir.path().join("wfl.cfg");

        let global_config_content = r#"
        # Global WFL Configuration
        timeout_seconds = 180
        logging_enabled = true
        max_line_length = 120
        "#;

        let mut file = fs::File::create(&global_config_path).unwrap();
        file.write_all(global_config_content.as_bytes()).unwrap();

        // Create a separate directory for the "script" with no local config
        let script_dir = tempdir().unwrap();

        // Ensure we use the specific global config and don't have local config
        let config = with_test_global_path(|| {
            // Explicitly set the path to our test global config
            set_test_env_var(Some(global_config_path.to_str().unwrap()));
            // Use load_config_with_global to load the global config
            load_config_with_global(script_dir.path())
        });

        // Verify the global config values were properly loaded
        assert_eq!(config.timeout_seconds, 180); // From global config
        assert!(config.logging_enabled);
        assert_eq!(config.max_line_length, 120);
        assert!(config.debug_report_enabled); // Default
    }

    #[test]
    fn test_load_config_local_only() {
        let script_dir = tempdir().unwrap();
        let local_config_path = script_dir.path().join(".wflcfg");

        let local_config_content = r#"
        # Local WFL Configuration
        timeout_seconds = 90
        log_level = debug
        snake_case_variables = false
        "#;

        let mut file = fs::File::create(&local_config_path).unwrap();
        file.write_all(local_config_content.as_bytes()).unwrap();

        let config = with_test_global_path(|| {
            // Explicitly set a non-existent path to ensure we don't pick up any global config
            set_test_env_var(Some("/non/existent/path"));
            load_config(script_dir.path())
        });

        assert_eq!(config.timeout_seconds, 90);
        assert!(!config.logging_enabled); // Default value since global config path is non-existent
        assert_eq!(config.log_level, LogLevel::Debug);
        assert!(!config.snake_case_variables);
    }

    #[test]
    fn test_load_config_local_override() {
        let temp_dir = tempdir().unwrap();
        let global_config_path = temp_dir.path().join("wfl.cfg");

        let global_config_content = r#"
        # Global WFL Configuration
        timeout_seconds = 180
        logging_enabled = true
        max_line_length = 120
        snake_case_variables = true
        "#;

        let mut file = fs::File::create(&global_config_path).unwrap();
        file.write_all(global_config_content.as_bytes()).unwrap();

        let script_dir = tempdir().unwrap();
        let local_config_path = script_dir.path().join(".wflcfg");

        let local_config_content = r#"
        # Local WFL Configuration (overrides global)
        timeout_seconds = 60
        log_level = debug
        snake_case_variables = false
        "#;

        let mut file = fs::File::create(&local_config_path).unwrap();
        file.write_all(local_config_content.as_bytes()).unwrap();

        let config = with_test_global_path(|| {
            set_test_env_var(Some(global_config_path.to_str().unwrap()));
            load_config_with_global(script_dir.path())
        });

        assert_eq!(config.timeout_seconds, 60); // Local override
        assert!(config.logging_enabled); // From global
        assert_eq!(config.max_line_length, 120); // From global
        assert_eq!(config.log_level, LogLevel::Debug); // Local override
        assert!(!config.snake_case_variables); // Local override
    }

    #[test]
    fn test_web_server_bind_address_default() {
        let temp_dir = tempfile::tempdir().unwrap();

        let config = with_test_global_path(|| {
            set_test_env_var(Some("/non/existent/path"));
            load_config(temp_dir.path())
        });

        // Verify default bind address is localhost
        assert_eq!(
            config.web_server_bind_address, "127.0.0.1",
            "Default web_server_bind_address should be 127.0.0.1"
        );
    }

    #[test]
    fn test_web_server_tls_files_default_none() {
        let temp_dir = tempfile::tempdir().unwrap();

        let config = with_test_global_path(|| {
            set_test_env_var(Some("/non/existent/path"));
            load_config(temp_dir.path())
        });

        assert_eq!(
            config.web_server_tls_cert_file, None,
            "Default web_server_tls_cert_file should be None"
        );
        assert_eq!(
            config.web_server_tls_key_file, None,
            "Default web_server_tls_key_file should be None"
        );
    }

    #[test]
    fn test_web_server_max_body_size_default() {
        let temp_dir = tempfile::tempdir().unwrap();

        let config = with_test_global_path(|| {
            set_test_env_var(Some("/non/existent/path"));
            load_config(temp_dir.path())
        });

        assert_eq!(
            config.web_server_max_body_size, 1_048_576,
            "Default web_server_max_body_size should be 1 MiB"
        );
    }

    #[test]
    fn test_web_server_max_body_size_custom() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(".wflcfg");

        let config_content = "web_server_max_body_size = 10485760\n";

        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = with_test_global_path(|| {
            set_test_env_var(Some("/non/existent/path"));
            load_config(temp_dir.path())
        });

        assert_eq!(
            config.web_server_max_body_size, 10_485_760,
            "web_server_max_body_size should load from .wflcfg"
        );
    }

    #[test]
    fn test_web_server_tls_files_custom() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(".wflcfg");

        let config_content = r#"
        web_server_tls_cert_file = /etc/wfl/cert.pem
        web_server_tls_key_file = /etc/wfl/key.pem
        "#;

        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = with_test_global_path(|| {
            set_test_env_var(Some("/non/existent/path"));
            load_config(temp_dir.path())
        });

        assert_eq!(
            config.web_server_tls_cert_file.as_deref(),
            Some("/etc/wfl/cert.pem"),
            "web_server_tls_cert_file should be loaded"
        );
        assert_eq!(
            config.web_server_tls_key_file.as_deref(),
            Some("/etc/wfl/key.pem"),
            "web_server_tls_key_file should be loaded"
        );
    }

    #[test]
    fn test_web_server_tls_files_empty_values_ignored() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(".wflcfg");

        let config_content = r#"
        web_server_tls_cert_file =
        web_server_tls_key_file =
        "#;

        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = with_test_global_path(|| {
            set_test_env_var(Some("/non/existent/path"));
            load_config(temp_dir.path())
        });

        assert_eq!(
            config.web_server_tls_cert_file, None,
            "Empty web_server_tls_cert_file should stay None"
        );
        assert_eq!(
            config.web_server_tls_key_file, None,
            "Empty web_server_tls_key_file should stay None"
        );
    }

    #[test]
    fn test_web_server_bind_address_custom_ipv4() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(".wflcfg");

        let config_content = r#"
        # Test custom IPv4 bind address
        web_server_bind_address = 0.0.0.0
        "#;

        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = with_test_global_path(|| {
            set_test_env_var(Some("/non/existent/path"));
            load_config(temp_dir.path())
        });

        assert_eq!(
            config.web_server_bind_address, "0.0.0.0",
            "web_server_bind_address should be set to 0.0.0.0"
        );
    }

    #[test]
    fn test_web_server_bind_address_custom_ipv6() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(".wflcfg");

        let config_content = r#"
        # Test IPv6 localhost bind address
        web_server_bind_address = ::1
        "#;

        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = with_test_global_path(|| {
            set_test_env_var(Some("/non/existent/path"));
            load_config(temp_dir.path())
        });

        assert_eq!(
            config.web_server_bind_address, "::1",
            "web_server_bind_address should support IPv6 addresses"
        );
    }

    #[test]
    fn test_web_server_bind_address_specific_ip() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(".wflcfg");

        let config_content = r#"
        # Test specific IP address
        web_server_bind_address = 192.168.1.100
        "#;

        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = with_test_global_path(|| {
            set_test_env_var(Some("/non/existent/path"));
            load_config(temp_dir.path())
        });

        assert_eq!(
            config.web_server_bind_address, "192.168.1.100",
            "web_server_bind_address should accept specific IP addresses"
        );
    }

    #[test]
    fn test_web_server_bind_address_local_override() {
        let temp_dir = tempdir().unwrap();
        let global_config_path = temp_dir.path().join("wfl.cfg");

        let global_config_content = r#"
        # Global config with default bind address
        web_server_bind_address = 127.0.0.1
        "#;

        let mut file = fs::File::create(&global_config_path).unwrap();
        file.write_all(global_config_content.as_bytes()).unwrap();

        let script_dir = tempdir().unwrap();
        let local_config_path = script_dir.path().join(".wflcfg");

        let local_config_content = r#"
        # Local config overrides to all interfaces
        web_server_bind_address = 0.0.0.0
        "#;

        let mut file = fs::File::create(&local_config_path).unwrap();
        file.write_all(local_config_content.as_bytes()).unwrap();

        let config = with_test_global_path(|| {
            set_test_env_var(Some(global_config_path.to_str().unwrap()));
            load_config_with_global(script_dir.path())
        });

        assert_eq!(
            config.web_server_bind_address, "0.0.0.0",
            "Local config should override global web_server_bind_address"
        );
    }

    #[test]
    fn test_web_server_bind_address_empty_value_uses_default() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(".wflcfg");

        // Empty value should be ignored, keeping the default
        let config_content = r#"
        web_server_bind_address =
        "#;

        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(config_content.as_bytes()).unwrap();

        let config = with_test_global_path(|| {
            set_test_env_var(Some("/non/existent/path"));
            load_config(temp_dir.path())
        });

        assert_eq!(
            config.web_server_bind_address, "127.0.0.1",
            "Empty web_server_bind_address should keep default value"
        );
    }
}
