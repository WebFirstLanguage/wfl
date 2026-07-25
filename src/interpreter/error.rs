use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    General,
    EnvDropped,
    Timeout,
    /// A shared `ExecutionBudget` ceiling other than the deadline was reached
    /// (operation count, recursion/import/execute-file depth, byte caps, etc.).
    ResourceLimit,
    /// A cooperative cancellation of an in-flight operation triggered by an
    /// expected external event rather than a fault — currently a downstream
    /// (browser) disconnect cancelling a proxy handler's blocked upstream read.
    /// Catchable like any other error, but the concurrent `main loop` treats it
    /// as a normal handler outcome, not a structural failure.
    Cancelled,
    FileNotFound,
    PermissionDenied,
    ProcessNotFound,
    ProcessSpawnFailed,
    ProcessKillFailed,
    CommandNotFound,
}

#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub kind: ErrorKind,
}

impl RuntimeError {
    pub fn new(message: String, line: usize, column: usize) -> Self {
        RuntimeError {
            message,
            line,
            column,
            kind: ErrorKind::General,
        }
    }

    pub fn with_kind(message: String, line: usize, column: usize, kind: ErrorKind) -> Self {
        RuntimeError {
            message,
            line,
            column,
            kind,
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let kind_str = match self.kind {
            ErrorKind::General => "",
            ErrorKind::EnvDropped => "[Environment dropped] ",
            ErrorKind::Timeout => "[Timeout] ",
            ErrorKind::ResourceLimit => "[Resource limit] ",
            ErrorKind::Cancelled => "[Cancelled] ",
            ErrorKind::FileNotFound => "[File not found] ",
            ErrorKind::PermissionDenied => "[Permission denied] ",
            ErrorKind::ProcessNotFound => "[Process not found] ",
            ErrorKind::ProcessSpawnFailed => "[Process spawn failed] ",
            ErrorKind::ProcessKillFailed => "[Process kill failed] ",
            ErrorKind::CommandNotFound => "[Command not found] ",
        };
        write!(
            f,
            "Runtime error at line {}, column {}: {}{}",
            self.line, self.column, kind_str, self.message
        )
    }
}

impl std::error::Error for RuntimeError {}
