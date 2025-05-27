use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    General,
    EnvDropped,
    Timeout,
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
        };
        write!(
            f,
            "Runtime error at line {}, column {}: {}{}",
            self.line, self.column, kind_str, self.message
        )
    }
}

impl std::error::Error for RuntimeError {}
