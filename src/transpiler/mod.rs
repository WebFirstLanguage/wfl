//! WFL to JavaScript Transpiler
//!
//! This module provides functionality to transpile WFL (Web First Language) code
//! into JavaScript, allowing WFL programs to run in browsers or Node.js environments.

mod javascript;
mod runtime;

pub use javascript::JavaScriptTranspiler;

use crate::parser::ast::Program;

/// Configuration options for the transpiler
#[derive(Debug, Clone)]
pub struct TranspilerConfig {
    /// Whether to include the runtime library in the output
    pub include_runtime: bool,
    /// Whether to generate source maps (future feature)
    pub source_maps: bool,
    /// Target environment (browser or node)
    pub target: TranspilerTarget,
    /// Whether to minify the output (future feature)
    pub minify: bool,
    /// Indentation string (e.g., "  " or "\t")
    pub indent: String,
    /// Whether to generate ES modules (export/import) or IIFE
    pub es_modules: bool,
}

impl Default for TranspilerConfig {
    fn default() -> Self {
        Self {
            include_runtime: true,
            source_maps: false,
            target: TranspilerTarget::Node,
            minify: false,
            indent: "  ".to_string(),
            es_modules: false,
        }
    }
}

/// Target environment for the transpiled code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranspilerTarget {
    /// Node.js environment
    Node,
    /// Browser environment
    Browser,
    /// Universal (works in both)
    Universal,
}

/// Result of a transpilation operation
#[derive(Debug)]
pub struct TranspileResult {
    /// The generated JavaScript code
    pub code: String,
    /// Any warnings generated during transpilation
    pub warnings: Vec<TranspileWarning>,
}

/// A warning generated during transpilation
#[derive(Debug)]
pub struct TranspileWarning {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

/// Error that can occur during transpilation
#[derive(Debug)]
pub struct TranspileError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for TranspileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Transpile error at line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for TranspileError {}

/// Main entry point for transpiling WFL to JavaScript
pub fn transpile(program: &Program, config: &TranspilerConfig) -> Result<TranspileResult, TranspileError> {
    let transpiler = JavaScriptTranspiler::new(config.clone());
    transpiler.transpile(program)
}

/// Convenience function to transpile with default configuration
pub fn transpile_default(program: &Program) -> Result<TranspileResult, TranspileError> {
    transpile(program, &TranspilerConfig::default())
}
