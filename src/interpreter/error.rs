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
    /// Not a failure: an `exit program` statement asking the run to stop where
    /// it stands. It travels as an error so it unwinds blocks, loops and
    /// action calls alike, but it is deliberately **not** catchable by
    /// `when error` and never reaches the user as a diagnostic — the top of
    /// the run turns it back into a successful finish.
    ExitProgram,
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

    /// The sentinel raised by `exit program`. See [`ErrorKind::ExitProgram`]:
    /// it unwinds like an error but finishes the run successfully.
    pub fn exit_program(line: usize, column: usize) -> Self {
        RuntimeError {
            message: "exit program".to_string(),
            line,
            column,
            kind: ErrorKind::ExitProgram,
        }
    }

    /// True for the `exit program` sentinel, which must never be caught by
    /// `when error`, rewrapped by an include/module frame, or reported.
    pub fn is_exit_program(&self) -> bool {
        matches!(self.kind, ErrorKind::ExitProgram)
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
            ErrorKind::ExitProgram => "[Exit program] ",
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
