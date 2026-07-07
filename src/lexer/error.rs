use crate::diagnostics::Span;

/// A lexing error: an unexpected byte or sequence the lexer could not tokenize.
///
/// Lexing is non-fatal — the offending input is dropped from the token stream and
/// lexing continues — but the error is collected so the CLI and REPL can render it
/// through the uniform Elm-style diagnostic system as a Syntax Error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub span: Span,
}

impl LexError {
    pub fn new(message: String, line: usize, column: usize, span: Span) -> Self {
        LexError {
            message,
            line,
            column,
            span,
        }
    }
}
