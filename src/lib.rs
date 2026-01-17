#![deny(clippy::await_holding_refcell_ref)]

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
pub mod fixer;
pub mod interpreter;
pub mod lexer;
pub mod linter;
pub mod logging;
pub mod parser;
pub mod pattern;
pub mod repl;
pub mod stdlib;
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

pub use interpreter::{Interpreter, TestFailure, TestResults};

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
