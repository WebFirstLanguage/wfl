//! # WFL Pattern Matching System
//!
//! This module provides a comprehensive natural language pattern matching system
//! for the WebFirst Language (WFL). It implements a bytecode-based virtual machine
//! that executes compiled patterns with full Unicode support.
//!
//! ## Overview
//!
//! The pattern system consists of three main components:
//! - **Compiler**: Converts natural language pattern AST to bytecode
//! - **VM**: Executes bytecode patterns against input text
//! - **Instructions**: Bytecode instruction set with advanced features
//!
//! ## Features
//!
//! - **Natural Language Syntax**: English-like pattern definitions
//! - **Unicode Support**: Full Unicode categories, scripts, and properties
//! - **Advanced Matching**: Lookahead/lookbehind, backreferences, named captures
//! - **Performance**: Bytecode VM with step limits to prevent ReDoS attacks
//! - **Security**: Safe execution with memory bounds checking
//!
//! ## Example Usage
//!
//! ```rust
//! use wfl::pattern::{CompiledPattern, PatternError};
//! use wfl::parser::ast::PatternExpression;
//!
//! fn example() -> Result<(), PatternError> {
//!     // Compile a pattern from AST
//!     let pattern = PatternExpression::Literal("hello".to_string());
//!     let compiled = CompiledPattern::compile(&pattern)?;
//!
//!     // Execute pattern matching
//!     assert!(compiled.matches("hello world"));
//!     assert!(!compiled.matches("goodbye world"));
//!
//!     // Find matches with positions
//!     if let Some(result) = compiled.find("say hello") {
//!         println!("Found match at {}-{}", result.start, result.end);
//!     }
//!     Ok(())
//! }
//! ```

pub mod compiler;
pub mod instruction;
pub mod vm;

pub use compiler::PatternCompiler;
pub use instruction::{Instruction, Program as PatternProgram};
pub use vm::{MatchResult, PatternVM};

use crate::parser::ast::PatternExpression;

/// Error types for pattern compilation and execution.
///
/// These errors can occur during pattern compilation or runtime execution.
/// All errors include descriptive messages to help with debugging.
#[derive(Debug, Clone)]
pub enum PatternError {
    /// Error during pattern compilation from AST to bytecode
    CompileError(String),
    /// Error during pattern execution in the VM
    RuntimeError(String),
    /// Pattern execution exceeded the maximum allowed steps (prevents ReDoS)
    StepLimitExceeded,
    /// Referenced capture group does not exist
    InvalidCapture(String),
    /// Invalid bytecode instruction encountered
    InvalidInstruction(String),
}

impl std::fmt::Display for PatternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatternError::CompileError(msg) => write!(f, "Pattern compile error: {msg}"),
            PatternError::RuntimeError(msg) => write!(f, "Pattern runtime error: {msg}"),
            PatternError::StepLimitExceeded => write!(f, "Pattern execution step limit exceeded"),
            PatternError::InvalidCapture(name) => write!(f, "Invalid capture group: {name}"),
            PatternError::InvalidInstruction(msg) => write!(f, "Invalid instruction: {msg}"),
        }
    }
}

impl std::error::Error for PatternError {}

/// A compiled pattern ready for execution.
///
/// This structure contains the bytecode program and metadata needed to execute
/// pattern matching operations. Patterns are compiled once and can be used
/// multiple times for efficient matching.
///
/// ## Thread Safety
///
/// `CompiledPattern` is thread-safe and can be shared between threads.
/// Each execution creates its own VM state, so multiple threads can
/// execute the same pattern simultaneously.
#[derive(Debug, Clone)]
pub struct CompiledPattern {
    /// The compiled bytecode program
    pub program: PatternProgram,
    /// Names of capture groups in the pattern
    pub capture_names: Vec<String>,
}

impl CompiledPattern {
    /// Create a new compiled pattern with the given program and capture names.
    ///
    /// # Arguments
    /// * `program` - The compiled bytecode program
    /// * `capture_names` - Names of capture groups in order
    pub fn new(program: PatternProgram, capture_names: Vec<String>) -> Self {
        Self {
            program,
            capture_names,
        }
    }

    /// Compile a PatternExpression AST into bytecode.
    ///
    /// This is the main entry point for converting WFL pattern syntax into
    /// executable bytecode. The compilation process validates the pattern
    /// and generates optimized instructions.
    ///
    /// # Arguments
    /// * `pattern` - The AST representation of the pattern to compile
    ///
    /// # Returns
    /// * `Ok(CompiledPattern)` - Successfully compiled pattern
    /// * `Err(PatternError)` - Compilation failed with error details
    ///
    /// # Examples
    /// ```rust
    /// use wfl::parser::ast::PatternExpression;
    /// use wfl::pattern::CompiledPattern;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let pattern = PatternExpression::Literal("hello".to_string());
    /// let compiled = CompiledPattern::compile(&pattern)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn compile(pattern: &PatternExpression) -> Result<Self, PatternError> {
        let mut compiler = PatternCompiler::new();
        let program = compiler.compile(pattern)?;
        let capture_names = compiler.capture_names();
        Ok(Self::new(program, capture_names))
    }

    /// Compile a pattern with access to environment variables for list references.
    ///
    /// This method allows patterns to reference list variables defined in the environment.
    /// List references are resolved at compile time and converted to alternative patterns.
    ///
    /// # Arguments
    /// * `pattern` - The pattern AST to compile
    /// * `env` - Environment containing variable definitions including lists
    ///
    /// # Returns
    /// * `Ok(CompiledPattern)` - Successfully compiled pattern
    /// * `Err(PatternError)` - Compilation failed due to invalid pattern or undefined list
    ///
    /// # Examples
    /// ```rust
    /// # use wfl::pattern::{CompiledPattern, PatternExpression};
    /// # use wfl::interpreter::environment::Environment;
    /// # fn example() -> Result<(), wfl::pattern::PatternError> {
    /// let env = Environment::new();
    /// let pattern = PatternExpression::ListReference("protocols".to_string());
    /// let compiled = CompiledPattern::compile_with_env(&pattern, &env)?;
    /// assert!(compiled.matches("http"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn compile_with_env(
        pattern: &PatternExpression,
        env: &crate::interpreter::environment::Environment,
    ) -> Result<Self, PatternError> {
        let mut compiler = PatternCompiler::new();
        let program = compiler.compile_with_env(pattern, env)?;
        let capture_names = compiler.capture_names();
        Ok(Self::new(program, capture_names))
    }

    /// Test if the pattern matches anywhere in the input text.
    ///
    /// This is the most efficient way to test for pattern presence.
    /// Returns `true` if the pattern matches any substring, `false` otherwise.
    ///
    /// # Arguments  
    /// * `text` - The input text to search
    ///
    /// # Returns
    /// * `true` - Pattern found in text
    /// * `false` - Pattern not found or execution error
    ///
    /// # Examples
    /// ```rust
    /// # use wfl::parser::ast::PatternExpression;
    /// # use wfl::pattern::CompiledPattern;
    /// # let pattern = PatternExpression::Literal("hello".to_string());
    /// # let pattern = CompiledPattern::compile(&pattern).unwrap();
    /// assert!(pattern.matches("hello world"));
    /// assert!(!pattern.matches("goodbye"));
    /// ```
    ///
    /// # Note
    /// Execution errors are silently converted to `false`. For error details,
    /// use the VM directly.
    pub fn matches(&self, text: &str) -> bool {
        let mut vm = PatternVM::new();
        vm.execute(&self.program, text).unwrap_or(false)
    }

    /// Find the first match in the text with position and capture information.
    ///
    /// Returns detailed information about the first match found, including
    /// start/end positions and any named capture groups.
    ///
    /// # Arguments
    /// * `text` - The input text to search
    ///
    /// # Returns
    /// * `Some(MatchResult)` - First match found with details
    /// * `None` - No match found
    ///
    /// # Examples
    /// ```rust
    /// # use wfl::parser::ast::PatternExpression;
    /// # use wfl::pattern::CompiledPattern;
    /// # let pattern = PatternExpression::Literal("hello".to_string());
    /// # let pattern = CompiledPattern::compile(&pattern).unwrap();
    /// let text = "say hello world";
    /// if let Some(m) = pattern.find(text) {
    ///     println!("Match: '{}' at {}-{}", &text[m.start..m.end], m.start, m.end);
    /// }
    /// ```
    pub fn find(&self, text: &str) -> Option<MatchResult> {
        let mut vm = PatternVM::new();
        vm.find(&self.program, text, &self.capture_names)
    }

    /// Find all non-overlapping matches in the text.
    ///
    /// Returns a vector of all matches found in the text, including position
    /// and capture information for each match.
    ///
    /// # Arguments
    /// * `text` - The input text to search
    ///
    /// # Returns
    /// * `Vec<MatchResult>` - All matches found (may be empty)
    ///
    /// # Examples
    /// ```rust
    /// # use wfl::parser::ast::PatternExpression;
    /// # use wfl::pattern::CompiledPattern;
    /// # let pattern = PatternExpression::Literal("hello".to_string());
    /// # let pattern = CompiledPattern::compile(&pattern).unwrap();
    /// let matches = pattern.find_all("hello world, hello universe");
    /// println!("Found {} matches", matches.len());
    /// ```
    ///
    /// # Performance
    /// For patterns that may match many times, consider using iterative
    /// approaches if memory usage is a concern.
    pub fn find_all(&self, text: &str) -> Vec<MatchResult> {
        let mut vm = PatternVM::new();
        vm.find_all(&self.program, text, &self.capture_names)
    }
}
