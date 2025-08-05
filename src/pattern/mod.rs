pub mod compiler;
pub mod instruction;
pub mod vm;

pub use compiler::PatternCompiler;
pub use instruction::{Instruction, Program as PatternProgram};
pub use vm::{MatchResult, PatternVM};

use crate::parser::ast::PatternExpression;

/// Error types for pattern compilation and execution
#[derive(Debug, Clone)]
pub enum PatternError {
    CompileError(String),
    RuntimeError(String),
    StepLimitExceeded,
    InvalidCapture(String),
    InvalidInstruction(String),
}

impl std::fmt::Display for PatternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatternError::CompileError(msg) => write!(f, "Pattern compile error: {}", msg),
            PatternError::RuntimeError(msg) => write!(f, "Pattern runtime error: {}", msg),
            PatternError::StepLimitExceeded => write!(f, "Pattern execution step limit exceeded"),
            PatternError::InvalidCapture(name) => write!(f, "Invalid capture group: {}", name),
            PatternError::InvalidInstruction(msg) => write!(f, "Invalid instruction: {}", msg),
        }
    }
}

impl std::error::Error for PatternError {}

/// Compiled pattern ready for execution
#[derive(Debug, Clone)]
pub struct CompiledPattern {
    pub program: PatternProgram,
    pub capture_names: Vec<String>,
}

impl CompiledPattern {
    pub fn new(program: PatternProgram, capture_names: Vec<String>) -> Self {
        Self {
            program,
            capture_names,
        }
    }

    /// Compile a PatternExpression AST into bytecode
    pub fn compile(pattern: &PatternExpression) -> Result<Self, PatternError> {
        let mut compiler = PatternCompiler::new();
        let program = compiler.compile(pattern)?;
        let capture_names = compiler.capture_names();
        Ok(Self::new(program, capture_names))
    }

    /// Execute the pattern against input text
    pub fn matches(&self, text: &str) -> bool {
        let mut vm = PatternVM::new();
        vm.execute(&self.program, text).unwrap_or(false)
    }

    /// Find the first match in the text
    pub fn find(&self, text: &str) -> Option<MatchResult> {
        let mut vm = PatternVM::new();
        vm.find(&self.program, text, &self.capture_names)
    }

    /// Find all matches in the text
    pub fn find_all(&self, text: &str) -> Vec<MatchResult> {
        let mut vm = PatternVM::new();
        vm.find_all(&self.program, text, &self.capture_names)
    }
}
