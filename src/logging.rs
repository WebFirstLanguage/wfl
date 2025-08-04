use crate::config::{LogLevel, WflConfig};
use chrono::Local;
use log::{LevelFilter, SetLoggerError};
use once_cell::sync::Lazy;
use simplelog::{
    ColorChoice, CombinedLogger, ConfigBuilder, TermLogger, TerminalMode, WriteLogger,
};
use std::cell::RefCell;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;
use time;
use time::format_description::FormatItem;

static LOGGER_INITIALIZED: AtomicBool = AtomicBool::new(false);
static EXEC_LOGGER_INITIALIZED: AtomicBool = AtomicBool::new(false);
static START_TIME: Lazy<Instant> = Lazy::new(Instant::now);
static TIME_FORMAT: Lazy<Vec<FormatItem>> =
    Lazy::new(|| time::format_description::parse("[hour]:[minute]:[second].[subsecond]").unwrap());
static INDENTATION_LEVEL: AtomicUsize = AtomicUsize::new(0);
thread_local! {
    static EXECUTION_LOG_FILE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

// Helper for execution log indentation
pub struct IndentGuard;

impl Default for IndentGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl IndentGuard {
    pub fn new() -> Self {
        INDENTATION_LEVEL.fetch_add(1, Ordering::SeqCst);
        IndentGuard
    }
}

impl Drop for IndentGuard {
    fn drop(&mut self) {
        INDENTATION_LEVEL.fetch_sub(1, Ordering::SeqCst);
    }
}

pub fn init_logger(log_level: LogLevel, file_path: &Path) -> Result<(), SetLoggerError> {
    if LOGGER_INITIALIZED.load(Ordering::Relaxed) {
        return Ok(());
    }

    let level_filter = log_level.to_level_filter();

    let config = ConfigBuilder::new()
        .set_time_format_custom(&TIME_FORMAT)
        .set_location_level(LevelFilter::Debug) // Include file:line for all levels
        .build();

    let file_logger_result = File::create(file_path).map(|file| {
        WriteLogger::new(
            LevelFilter::Debug, // Log everything to file
            config.clone(),
            file,
        )
    });

    if let Err(e) = &file_logger_result {
        eprintln!(
            "Warning: Could not create log file at {}: {}",
            file_path.display(),
            e
        );

        let term_logger = TermLogger::new(
            level_filter.min(LevelFilter::Info), // Terminal gets info and above
            config.clone(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        );

        CombinedLogger::init(vec![term_logger])?;
        LOGGER_INITIALIZED.store(true, Ordering::Relaxed);

        log::info!(
            "WFL logging initialized at {} (console-only mode)",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        );
        return Ok(());
    }

    let file_logger = file_logger_result.unwrap();

    let term_logger = TermLogger::new(
        level_filter.min(LevelFilter::Info), // Terminal gets info and above
        config,
        TerminalMode::Mixed,
        ColorChoice::Auto,
    );

    CombinedLogger::init(vec![file_logger, term_logger])?;
    LOGGER_INITIALIZED.store(true, Ordering::Relaxed);

    log::info!(
        "WFL logging initialized at {}",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    Ok(())
}

// Initialize a separate logger for execution trace
pub fn init_execution_logger(
    config: &WflConfig,
    base_log_path: &Path,
) -> Result<(), SetLoggerError> {
    if !config.execution_logging || EXEC_LOGGER_INITIALIZED.load(Ordering::Relaxed) {
        return Ok(());
    }

    // Create execution log file path (append _exec to the base filename)
    let file_name = base_log_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("wfl.log");

    let exec_file_name = if let Some(pos) = file_name.rfind('.') {
        format!("{}_exec{}", &file_name[..pos], &file_name[pos..])
    } else {
        format!("{file_name}_exec.log")
    };

    let exec_log_path = base_log_path.with_file_name(exec_file_name);

    // Store the execution log path for later use
    EXECUTION_LOG_FILE.with(|path| {
        *path.borrow_mut() = Some(exec_log_path.clone());
    });

    // Create the execution log file
    let file_result = File::create(&exec_log_path);

    if let Err(e) = &file_result {
        eprintln!(
            "Warning: Could not create execution log file at {}: {}",
            exec_log_path.display(),
            e
        );
        return Ok(());
    }

    let config = ConfigBuilder::new()
        .set_time_format_custom(&TIME_FORMAT)
        .set_location_level(LevelFilter::Debug)
        .build();

    let file_logger = WriteLogger::new(LevelFilter::Debug, config, file_result.unwrap());

    // Only the file logger for execution tracing, no terminal output
    CombinedLogger::init(vec![file_logger])?;
    EXEC_LOGGER_INITIALIZED.store(true, Ordering::Relaxed);

    log::info!(
        "WFL execution logging initialized at {} - {}",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        exec_log_path.display()
    );

    Ok(())
}

pub fn elapsed_time() -> std::time::Duration {
    START_TIME.elapsed()
}

pub fn current_indent() -> String {
    let level = INDENTATION_LEVEL.load(Ordering::Relaxed);
    "  ".repeat(level)
}

// Standard logging macros
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        log::debug!("[{:?}] {}", $crate::logging::elapsed_time(), format!($($arg)*))
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        log::info!("[{:?}] {}", $crate::logging::elapsed_time(), format!($($arg)*))
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        log::warn!("[{:?}] {}", $crate::logging::elapsed_time(), format!($($arg)*))
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        log::error!("[{:?}] {}", $crate::logging::elapsed_time(), format!($($arg)*))
    };
}

// Execution logging macros - these only compile in debug builds
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! exec_trace {
    ($($arg:tt)*) => {
        if let Ok(config) = $crate::CONFIG.read() {
            if let Some(cfg) = config.as_ref() {
                if cfg.execution_logging && cfg.verbose_execution {
                    log::debug!("EXEC: {}{}", $crate::logging::current_indent(), format!($($arg)*))
                }
            }
        }
    };
}

// High-level execution logging (always logged when execution_logging is on)
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! exec_trace_always {
    ($($arg:tt)*) => {
        if let Ok(config) = $crate::CONFIG.read() {
            if let Some(cfg) = config.as_ref() {
                if cfg.execution_logging {
                    log::debug!("EXEC: {}{}", $crate::logging::current_indent(), format!($($arg)*))
                }
            }
        }
    };
}

// Loop-aware execution logging with throttling
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! exec_trace_loop {
    ($iteration:expr, $($arg:tt)*) => {
        if let Ok(config) = $crate::CONFIG.read() {
            if let Some(cfg) = config.as_ref() {
                if cfg.execution_logging && cfg.log_loop_iterations {
                    let iter = $iteration;
                    let throttle = cfg.log_throttle_factor;
                    // Log first 3, every Nth, and approach to end
                    if iter < 3 || iter % throttle == 0 {
                        log::debug!("EXEC: {}[Loop {}] {}", $crate::logging::current_indent(), iter, format!($($arg)*))
                    }
                }
            }
        }
    };
}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! exec_var_declare {
    ($name:expr, $value:expr) => {
        if let Ok(config) = $crate::CONFIG.read() {
            if let Some(cfg) = config.as_ref() {
                if cfg.execution_logging {
                    log::debug!(
                        "EXEC: {}Declaration: '{}' = {:?}",
                        $crate::logging::current_indent(),
                        $name,
                        $value
                    )
                }
            }
        }
    };
}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! exec_var_assign {
    ($name:expr, $value:expr) => {
        if let Ok(config) = $crate::CONFIG.read() {
            if let Some(cfg) = config.as_ref() {
                if cfg.execution_logging {
                    log::debug!(
                        "EXEC: {}Assignment: '{}' = {:?}",
                        $crate::logging::current_indent(),
                        $name,
                        $value
                    )
                }
            }
        }
    };
}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! exec_expr_eval {
    ($expr_desc:expr, $value:expr) => {
        if let Ok(config) = $crate::CONFIG.read() {
            if let Some(cfg) = config.as_ref() {
                if cfg.execution_logging {
                    log::debug!(
                        "EXEC: {}Expression: {} = {:?}",
                        $crate::logging::current_indent(),
                        $expr_desc,
                        $value
                    )
                }
            }
        }
    };
}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! exec_control_flow {
    ($desc:expr, $result:expr) => {
        if let Ok(config) = $crate::CONFIG.read() {
            if let Some(cfg) = config.as_ref() {
                if cfg.execution_logging {
                    log::debug!(
                        "EXEC: {}Control flow: {} = {:?}",
                        $crate::logging::current_indent(),
                        $desc,
                        $result
                    )
                }
            }
        }
    };
}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! exec_function_call {
    ($name:expr, $args:expr) => {
        if let Ok(config) = $crate::CONFIG.read() {
            if let Some(cfg) = config.as_ref() {
                if cfg.execution_logging {
                    log::debug!(
                        "EXEC: {}Function call: {}({})",
                        $crate::logging::current_indent(),
                        $name,
                        $args
                            .iter()
                            .map(|arg| format!("{:?}", arg))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
        }
    };
}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! exec_function_return {
    ($name:expr, $value:expr) => {
        if let Ok(config) = $crate::CONFIG.read() {
            if let Some(cfg) = config.as_ref() {
                if cfg.execution_logging {
                    log::debug!(
                        "EXEC: {}Function return: {} = {:?}",
                        $crate::logging::current_indent(),
                        $name,
                        $value
                    )
                }
            }
        }
    };
}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! exec_block_enter {
    ($desc:expr) => {
        if let Ok(config) = $crate::CONFIG.read() {
            if let Some(cfg) = config.as_ref() {
                if cfg.execution_logging {
                    log::debug!(
                        "EXEC: {}┌─ Block entry: {}",
                        $crate::logging::current_indent(),
                        $desc
                    )
                }
            }
        }
    };
}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! exec_block_exit {
    ($desc:expr) => {
        if let Ok(config) = $crate::CONFIG.read() {
            if let Some(cfg) = config.as_ref() {
                if cfg.execution_logging {
                    log::debug!(
                        "EXEC: {}└─ Block exit: {}",
                        $crate::logging::current_indent(),
                        $desc
                    )
                }
            }
        }
    };
}

// No-op versions for release builds
#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! exec_trace {
    ($($arg:tt)*) => {};
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! exec_var_declare {
    ($name:expr, $value:expr) => {};
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! exec_var_assign {
    ($name:expr, $value:expr) => {};
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! exec_expr_eval {
    ($expr_desc:expr, $value:expr) => {};
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! exec_control_flow {
    ($desc:expr, $result:expr) => {};
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! exec_function_call {
    ($name:expr, $args:expr) => {};
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! exec_function_return {
    ($name:expr, $value:expr) => {};
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! exec_block_enter {
    ($desc:expr) => {};
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! exec_block_exit {
    ($desc:expr) => {};
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    #[ignore = "Cannot reset logger state between tests"]
    fn test_logger_initialization() {
        let temp_dir = tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");

        let result = init_logger(LogLevel::Debug, &log_path);
        assert!(result.is_ok());

        log::info!("Test log message");
    }

    #[test]
    fn test_logger_fallback_when_file_creation_fails() {
        let temp_dir = tempdir().unwrap();
        #[cfg(unix)]
        let dir_path = temp_dir.path();

        #[cfg(not(unix))]
        let _dir_path = temp_dir.path();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(dir_path, std::fs::Permissions::from_mode(0o555)).unwrap();
        }

        #[cfg(not(unix))]
        let dir_path = temp_dir.path().join("non_existent_directory");

        let log_path = dir_path.join("test.log");

        let result = init_logger(LogLevel::Debug, &log_path);
        assert!(result.is_ok());

        log::info!("This should not panic");
    }
}
