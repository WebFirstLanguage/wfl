use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
// use std::fmt;
use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

impl From<Severity> for codespan_reporting::diagnostic::Severity {
    fn from(severity: Severity) -> Self {
        match severity {
            Severity::Error => Self::Error,
            Severity::Warning => Self::Warning,
            Severity::Note => Self::Note,
            Severity::Help => Self::Help,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct WflDiagnostic {
    pub severity: Severity,
    #[allow(clippy::too_many_arguments)]
    pub message: String,
    pub labels: Vec<(Span, String)>,
    pub notes: Vec<String>,
    pub code: String,
    pub file_id: usize,
    pub line: usize,
    pub column: usize,
}

#[allow(clippy::too_many_arguments)]
impl WflDiagnostic {
    pub fn new(
        severity: Severity,
        message: impl Into<String>,
        note: Option<impl Into<String>>,
        code: impl Into<String>,
        file_id: usize,
        line: usize,
        column: usize,
        span: Option<Span>,
    ) -> Self {
        let mut diagnostic = WflDiagnostic {
            severity,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
            code: code.into(),
            file_id,
            line,
            column,
        };

        if let Some(note) = note {
            diagnostic.notes.push(note.into());
        }

        if let Some(span) = span {
            diagnostic.labels.push((span, "Here".to_string()));
        }

        diagnostic
    }

    pub fn error(message: impl Into<String>) -> Self {
        WflDiagnostic {
            severity: Severity::Error,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
            code: "ERROR".to_string(),
            file_id: 0,
            line: 0,
            column: 0,
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        WflDiagnostic {
            severity: Severity::Warning,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
            code: "WARNING".to_string(),
            file_id: 0,
            line: 0,
            column: 0,
        }
    }

    pub fn with_primary_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push((span, message.into()));
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn to_codespan_diagnostic(&self, file_id: usize) -> Diagnostic<usize> {
        let mut diag = Diagnostic::new(self.severity.into()).with_message(self.message.clone());

        for (span, message) in &self.labels {
            diag = diag.with_labels(vec![
                Label::primary(file_id, span.start..span.end).with_message(message.clone()),
            ]);
        }

        for note in &self.notes {
            diag = diag.with_notes(vec![note.clone()]);
        }

        diag
    }
}

pub struct DiagnosticReporter {
    pub files: SimpleFiles<String, String>,
}

impl Default for DiagnosticReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticReporter {
    pub fn new() -> Self {
        DiagnosticReporter {
            files: SimpleFiles::new(),
        }
    }

    pub fn add_file(&mut self, name: impl Into<String>, source: impl Into<String>) -> usize {
        self.files.add(name.into(), source.into())
    }

    pub fn report_diagnostic(&self, file_id: usize, diagnostic: &WflDiagnostic) -> io::Result<()> {
        let mut diag =
            Diagnostic::new(diagnostic.severity.into()).with_message(diagnostic.message.clone());

        if !diagnostic.code.is_empty() {
            diag = diag.with_code(diagnostic.code.clone());
        }

        for (span, message) in &diagnostic.labels {
            diag = diag.with_labels(vec![
                Label::primary(file_id, span.start..span.end).with_message(message.clone()),
            ]);
        }

        for note in &diagnostic.notes {
            diag = diag.with_notes(vec![note.clone()]);
        }

        let writer = StandardStream::stderr(ColorChoice::Always);
        let config = term::Config::default();

        term::emit(&mut writer.lock(), &config, &self.files, &diag)
            .map_err(|_| io::Error::other("Failed to emit diagnostic"))
    }

    pub fn line_col_to_offset(&self, file_id: usize, line: usize, column: usize) -> Option<usize> {
        let line = line.saturating_sub(1);
        let column = column.saturating_sub(1);

        if let Ok(file) = self.files.get(file_id) {
            let source = file.source();
            let lines: Vec<&str> = source.lines().collect();

            if line < lines.len() {
                let line_start = source
                    .lines()
                    .take(line)
                    .map(|l| l.len() + 1)
                    .sum::<usize>();
                if column <= lines[line].len() {
                    return Some(line_start + column);
                }
            }
        }
        None
    }

    pub fn convert_parse_error(
        &self,
        file_id: usize,
        error: &crate::parser::ast::ParseError,
    ) -> WflDiagnostic {
        let message = error.message.clone();

        let start_offset = self
            .line_col_to_offset(file_id, error.line, error.column)
            .unwrap_or(0);
        let end_offset = start_offset + 1;

        let mut diag = WflDiagnostic::error(message.clone()).with_primary_label(
            Span {
                start: start_offset,
                end: end_offset,
            },
            "Error occurred here",
        );

        if message.contains("Expected 'as' after variable name") {
            diag = diag.with_note(
                "Did you forget to use 'as' before assigning a value? For example: `store a as 4`",
            );
        } else if message.contains("Expected 'to' after identifier") {
            diag = diag.with_note(
                "Did you forget to use 'to' before assigning a value? For example: `change a to 4`",
            );
        } else if message.contains("Expected a variable name before 'as'") {
            diag = diag.with_note(
                "You must provide a variable name before 'as'. For example: `store x as 3`",
            );
        } else if message.contains("Expected variable name but found end of input") {
            diag = diag.with_note(
                "The 'store' statement requires a variable name and value. For example: `store x as 3`",
            );
        } else if message.contains("Cannot use a number as a variable name") {
            diag = diag.with_note(
                "Variable names must start with a letter, not a number. For example: `store count as 1`",
            );
        } else if message.contains("Cannot use keyword") {
            diag = diag.with_note(
                "Reserved keywords cannot be used as variable names. Choose a different name that is not a reserved word.",
            );
        } else if message.contains("Expected ':' after container name") {
            diag = diag.with_note(
                "Container definitions require a colon after the name. For example: `create container Person:`",
            );
        } else if message.contains("Expected 'container' after 'create'") {
            diag = diag.with_note(
                "Use 'create container' to define a new container. For example: `create container Person:`",
            );
        } else if message.contains("Expected identifier for container name") {
            diag = diag.with_note(
                "Container names must be valid identifiers. For example: `create container Person:`",
            );
        } else if message.contains("Expected 'as' after container type") {
            diag = diag.with_note(
                "Container instantiation requires 'as' before the instance name. For example: `create new Person as alice:`",
            );
        } else if message.contains("Expected 'new' after 'create'") {
            diag = diag.with_note(
                "Use 'create new' to instantiate a container. For example: `create new Person as alice:`",
            );
        } else if message.contains("Expected identifier for container type") {
            diag = diag.with_note(
                "Specify a valid container type name. For example: `create new Person as alice:`",
            );
        } else if message.contains("Expected property name after 'property'") {
            diag = diag.with_note(
                "Property definitions require a name. For example: `property name: Text`",
            );
        } else if message.contains("Expected 'interface' after 'create'") {
            diag = diag.with_note(
                "Use 'create interface' to define a new interface. For example: `create interface Drawable:`",
            );
        } else if message.contains("Expected identifier for interface name") {
            diag = diag.with_note(
                "Interface names must be valid identifiers. For example: `create interface Drawable:`",
            );
        }

        diag
    }

    pub fn convert_type_error(
        &self,
        file_id: usize,
        error: &crate::typechecker::TypeError,
    ) -> WflDiagnostic {
        let mut message_text = error.message.clone();

        if let (Some(expected), Some(found)) = (&error.expected, &error.found) {
            message_text = format!(
                "{} - Expected {} but found {}",
                message_text, expected, found
            );
        }

        let start_offset = self
            .line_col_to_offset(file_id, error.line, error.column)
            .unwrap_or(0);
        let end_offset = start_offset + 1;

        let mut diag = WflDiagnostic::error(message_text.clone()).with_primary_label(
            Span {
                start: start_offset,
                end: end_offset,
            },
            "Type error occurred here",
        );

        if message_text.contains("undefined") || message_text.contains("not defined") {
            diag = diag.with_note("Did you misspell the variable name or forget to declare it?");
        } else if let (Some(expected), Some(found)) = (&error.expected, &error.found) {
            if expected.to_string() == "Number" && found.to_string() == "Text" {
                diag =
                    diag.with_note("Try converting the text to a number using 'convert to number'");
            } else if expected.to_string() == "Text" && found.to_string() == "Number" {
                diag = diag.with_note("Try converting the number to text using 'convert to text'");
            }
        }

        diag
    }

    pub fn convert_semantic_error(
        &self,
        file_id: usize,
        error: &crate::analyzer::SemanticError,
    ) -> WflDiagnostic {
        let message = error.message.clone();

        let start_offset = self
            .line_col_to_offset(file_id, error.line, error.column)
            .unwrap_or(0);
        let end_offset = start_offset
            + (if error.message.contains("not defined") {
                error
                    .message
                    .split_whitespace()
                    .find(|word| word.starts_with('\'') && word.ends_with('\''))
                    .map(|word| word.len() - 2)
                    .unwrap_or(1)
            } else {
                1
            });

        let span = Span {
            start: start_offset,
            end: end_offset,
        };

        if error.message.contains("unused variable") || error.message.contains("Unused variable") {
            return WflDiagnostic::new(
                Severity::Warning,
                message,
                Some("Consider removing this variable if it's not needed".to_string()),
                "ANALYZE-UNUSED".to_string(),
                file_id,
                error.line,
                error.column,
                Some(span),
            );
        } else if error.message.contains("unreachable code")
            || error.message.contains("Unreachable code")
        {
            return WflDiagnostic::new(
                Severity::Warning,
                message,
                Some("This code will never be executed".to_string()),
                "ANALYZE-UNREACHABLE".to_string(),
                file_id,
                error.line,
                error.column,
                Some(span),
            );
        } else if error.message.contains("dead branch") || error.message.contains("Dead branch") {
            return WflDiagnostic::new(
                Severity::Warning,
                message,
                Some("This branch will never be taken".to_string()),
                "ANALYZE-DEADBRANCH".to_string(),
                file_id,
                error.line,
                error.column,
                Some(span),
            );
        } else if error.message.contains("shadows") {
            return WflDiagnostic::new(
                Severity::Warning,
                message,
                Some("Variable shadowing can lead to confusion and bugs".to_string()),
                "ANALYZE-SHADOW".to_string(),
                file_id,
                error.line,
                error.column,
                Some(span),
            );
        } else if error.message.contains("inconsistent return")
            || error.message.contains("return paths")
        {
            return WflDiagnostic::new(
                Severity::Warning,
                message,
                Some("Ensure all code paths return a value".to_string()),
                "ANALYZE-RETURN".to_string(),
                file_id,
                error.line,
                error.column,
                Some(span),
            );
        }

        let mut diag = WflDiagnostic::new(
            Severity::Error,
            message,
            None::<String>,
            "SEMANTIC".to_string(),
            file_id,
            error.line,
            error.column,
            Some(span),
        );

        if error.message.contains("already defined") {
            diag = diag.with_note("Variables must have unique names within the same scope");
        } else if error.message.contains("not defined") {
            diag = diag.with_note("Did you misspell the variable name or forget to declare it?");
        }

        diag
    }

    pub fn convert_runtime_error(
        &self,
        file_id: usize,
        error: &crate::interpreter::error::RuntimeError,
    ) -> WflDiagnostic {
        let message = error.message.clone();

        let start_offset = self
            .line_col_to_offset(file_id, error.line, error.column)
            .unwrap_or(0);
        let span = Span {
            start: start_offset,
            end: start_offset + 1,
        };

        let mut diag = WflDiagnostic::error(message.clone())
            .with_primary_label(span, "Runtime error occurred here");

        if error.message.contains("Pattern compilation error") {
            if error.message.contains("invalid range") || error.message.contains("Invalid range") {
                return WflDiagnostic::new(
                    Severity::Error,
                    message,
                    Some(
                        "Check that quantifier ranges are valid (e.g., 'between 1 and 5')"
                            .to_string(),
                    ),
                    "PATTERN-SYNTAX-INVALID-RANGE".to_string(),
                    file_id,
                    error.line,
                    error.column,
                    Some(span),
                );
            } else if error.message.contains("unclosed group")
                || error.message.contains("Unclosed group")
            {
                return WflDiagnostic::new(
                    Severity::Error,
                    message,
                    Some(
                        "Make sure all pattern groups are properly closed with matching delimiters"
                            .to_string(),
                    ),
                    "PATTERN-SYNTAX-UNCLOSED-GROUP".to_string(),
                    file_id,
                    error.line,
                    error.column,
                    Some(span),
                );
            } else {
                return WflDiagnostic::new(
                    Severity::Error,
                    message,
                    Some("Check pattern syntax for errors in quantifiers, groups, or character classes".to_string()),
                    "PATTERN-SYNTAX-ERROR".to_string(),
                    file_id,
                    error.line,
                    error.column,
                    Some(span),
                );
            }
        } else if error
            .message
            .contains("Pattern execution depth limit exceeded")
        {
            return WflDiagnostic::new(
                Severity::Error,
                message,
                Some("Pattern matching was stopped to prevent infinite loops. Simplify your pattern or reduce input size".to_string()),
                "PATTERN-RUNTIME-DEPTH".to_string(),
                file_id,
                error.line,
                error.column,
                Some(span),
            );
        }

        if error.message.contains("division by zero") {
            diag = diag.with_note("Check your divisor to ensure it's never zero");
        } else if error.message.contains("index out of bounds") {
            diag = diag.with_note("Make sure your index is within the valid range of the list");
        } else if error.message.contains("file not found") {
            diag = diag.with_note("Verify that the file exists and the path is correct");
        } else if error.message.contains("Feature not implemented") {
            diag = diag.with_note("This feature is not implemented in the current build");
        }

        if matches!(error.kind, crate::interpreter::error::ErrorKind::EnvDropped) {
            diag = diag.with_note(
                "This usually means a closure outlived its defining scope. \
                Re-check lifetime of returned actions.",
            );
        }

        diag
    }

    pub fn convert_pattern_error(
        &self,
        file_id: usize,
        error_message: &str,
        pattern_name: Option<&str>,
        input_preview: Option<&str>,
        line: usize,
        column: usize,
    ) -> WflDiagnostic {
        let start_offset = self.line_col_to_offset(file_id, line, column).unwrap_or(0);
        let span = Span {
            start: start_offset,
            end: start_offset + 1,
        };

        let mut message = error_message.to_string();

        if let Some(name) = pattern_name {
            message = format!("Pattern '{}': {}", name, message);
        }

        if let Some(input) = input_preview {
            let preview = if input.len() > 30 {
                format!("{}...", &input[..30])
            } else {
                input.to_string()
            };
            message = format!("{} (input: \"{}\")", message, preview);
        }

        if error_message.contains("invalid range") || error_message.contains("Invalid range") {
            WflDiagnostic::new(
                Severity::Error,
                message,
                Some(
                    "Check that quantifier ranges are valid (e.g., 'between 1 and 5')".to_string(),
                ),
                "PATTERN-SYNTAX-INVALID-RANGE".to_string(),
                file_id,
                line,
                column,
                Some(span),
            )
        } else if error_message.contains("unclosed group")
            || error_message.contains("Unclosed group")
        {
            WflDiagnostic::new(
                Severity::Error,
                message,
                Some(
                    "Make sure all pattern groups are properly closed with matching delimiters"
                        .to_string(),
                ),
                "PATTERN-SYNTAX-UNCLOSED-GROUP".to_string(),
                file_id,
                line,
                column,
                Some(span),
            )
        } else if error_message.contains("depth limit exceeded")
            || error_message.contains("performance limit")
        {
            WflDiagnostic::new(
                Severity::Error,
                message,
                Some("Pattern matching was stopped to prevent infinite loops. Simplify your pattern or reduce input size".to_string()),
                "PATTERN-RUNTIME-DEPTH".to_string(),
                file_id,
                line,
                column,
                Some(span),
            )
        } else {
            WflDiagnostic::new(
                Severity::Error,
                message,
                Some(
                    "Check pattern syntax for errors in quantifiers, groups, or character classes"
                        .to_string(),
                ),
                "PATTERN-SYNTAX-ERROR".to_string(),
                file_id,
                line,
                column,
                Some(span),
            )
        }
    }
}

#[cfg(test)]
mod tests;
