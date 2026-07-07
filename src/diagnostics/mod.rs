use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
// use std::fmt;
use std::collections::HashMap;
use std::io;

pub mod render;

#[cfg(test)]
mod render_tests;

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

impl Severity {
    /// The glyph shown before the title in the Elm-style renderer.
    pub fn glyph(&self) -> &'static str {
        match self {
            Severity::Error => "✕",
            Severity::Warning => "▲",
            Severity::Note => "•",
            Severity::Help => "💡",
        }
    }
}

/// The category of a diagnostic. It determines the title shown on the first line
/// of the Elm-style output (e.g. "Type Error") and the default severity. Every
/// diagnostic produced by any WFL stage should carry a kind so all errors and
/// warnings read uniformly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    SyntaxError,     // lexer
    ParseError,      // parser
    TypeError,       // type checker
    NameError,       // semantic: undefined / already-defined names
    RuntimeError,    // interpreter
    PatternError,    // pattern engine
    LintWarning,     // linter
    AnalysisWarning, // static analyzer warnings (unused, unreachable, ...)
}

impl DiagnosticKind {
    /// Human-readable title shown on the diagnostic's first line, e.g. "Type Error".
    pub fn title(&self) -> &'static str {
        match self {
            DiagnosticKind::SyntaxError => "Syntax Error",
            DiagnosticKind::ParseError => "Parse Error",
            DiagnosticKind::TypeError => "Type Error",
            DiagnosticKind::NameError => "Name Error",
            DiagnosticKind::RuntimeError => "Runtime Error",
            DiagnosticKind::PatternError => "Pattern Error",
            DiagnosticKind::LintWarning => "Lint Warning",
            DiagnosticKind::AnalysisWarning => "Warning",
        }
    }

    /// The severity this kind maps to when none is otherwise specified.
    pub fn default_severity(&self) -> Severity {
        match self {
            DiagnosticKind::LintWarning | DiagnosticKind::AnalysisWarning => Severity::Warning,
            _ => Severity::Error,
        }
    }
}

/// A structured "expected type vs. found type" pair, rendered as the aligned
/// Expected/Found block. `found` may include the offending literal, e.g.
/// `Text ("hello")`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeMismatch {
    pub expected: String,
    pub found: String,
}

/// An actionable suggestion ("💡 Try ..."). `examples` are concrete fix snippets
/// rendered one per line, separated by `joiner` (default "— or —").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub message: String,
    pub examples: Vec<String>,
    pub joiner: Option<String>,
}

impl Suggestion {
    pub fn new(message: impl Into<String>) -> Self {
        Suggestion {
            message: message.into(),
            examples: Vec::new(),
            joiner: None,
        }
    }

    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.examples.push(example.into());
        self
    }

    /// The separator rendered between examples.
    pub fn joiner_str(&self) -> &str {
        self.joiner.as_deref().unwrap_or("— or —")
    }
}

/// Actionable guidance carried at the error's origin, so converters no longer
/// need to guess suggestions by substring-matching the message. Fold into a
/// diagnostic with [`WflDiagnostic::apply_hint`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiagnosticHint {
    pub explanation: Option<String>,
    pub suggestion: Option<Suggestion>,
    pub type_info: Option<TypeMismatch>,
    pub kind: Option<DiagnosticKind>,
}

impl DiagnosticHint {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.explanation.is_none()
            && self.suggestion.is_none()
            && self.type_info.is_none()
            && self.kind.is_none()
    }

    pub fn with_explanation(mut self, explanation: impl Into<String>) -> Self {
        self.explanation = Some(explanation.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: Suggestion) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    pub fn with_type_mismatch(
        mut self,
        expected: impl Into<String>,
        found: impl Into<String>,
    ) -> Self {
        self.type_info = Some(TypeMismatch {
            expected: expected.into(),
            found: found.into(),
        });
        self
    }

    pub fn with_kind(mut self, kind: DiagnosticKind) -> Self {
        self.kind = Some(kind);
        self
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
    // --- Elm-style enrichment (all optional; default None) ---
    /// The category, driving the rendered title and glyph.
    pub kind: Option<DiagnosticKind>,
    /// Structured Expected/Found block (type errors).
    pub type_info: Option<TypeMismatch>,
    /// Plain-English explanation paragraph shown below the source frame.
    pub explanation: Option<String>,
    /// Actionable "💡 Try ..." suggestion with example fixes.
    pub suggestion: Option<Suggestion>,
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
            kind: None,
            type_info: None,
            explanation: None,
            suggestion: None,
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
            kind: None,
            type_info: None,
            explanation: None,
            suggestion: None,
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
            kind: None,
            type_info: None,
            explanation: None,
            suggestion: None,
        }
    }

    pub fn with_primary_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push((span, message.into()));
        self
    }

    pub fn with_location(mut self, line: usize, column: usize) -> Self {
        self.line = line;
        self.column = column;
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_kind(mut self, kind: DiagnosticKind) -> Self {
        self.kind = Some(kind);
        self
    }

    pub fn with_type_mismatch(
        mut self,
        expected: impl Into<String>,
        found: impl Into<String>,
    ) -> Self {
        self.type_info = Some(TypeMismatch {
            expected: expected.into(),
            found: found.into(),
        });
        self
    }

    pub fn with_explanation(mut self, explanation: impl Into<String>) -> Self {
        self.explanation = Some(explanation.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: Suggestion) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    /// Fold an origin-carried [`DiagnosticHint`] into this diagnostic. Only fields
    /// the hint actually sets are overwritten, so it composes with builder calls
    /// and with the legacy substring-derived notes.
    pub fn apply_hint(mut self, hint: DiagnosticHint) -> Self {
        if let Some(kind) = hint.kind {
            self.kind = Some(kind);
        }
        if let Some(type_info) = hint.type_info {
            self.type_info = Some(type_info);
        }
        if let Some(explanation) = hint.explanation {
            self.explanation = Some(explanation);
        }
        if let Some(suggestion) = hint.suggestion {
            self.suggestion = Some(suggestion);
        }
        self
    }

    /// The title shown on the diagnostic's first line: the kind's title when set,
    /// otherwise a generic severity-based fallback so unkinded (legacy)
    /// diagnostics still render sensibly.
    pub fn title(&self) -> &'static str {
        match self.kind {
            Some(kind) => kind.title(),
            None => match self.severity {
                Severity::Error => "Error",
                Severity::Warning => "Warning",
                Severity::Note => "Note",
                Severity::Help => "Help",
            },
        }
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
    /// Cache of line start positions for each file to avoid recomputing them
    /// Key: file_id, Value: vector of byte offsets where each line starts
    line_starts_cache: HashMap<usize, Vec<usize>>,
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
            line_starts_cache: HashMap::new(),
        }
    }

    pub fn add_file(&mut self, name: impl Into<String>, source: impl Into<String>) -> usize {
        let file_id = self.files.add(name.into(), source.into());
        // Clear any cached line starts for this file since we're adding/updating it
        self.line_starts_cache.remove(&file_id);
        file_id
    }

    /// The text of a single (1-based) source line, without its trailing newline.
    /// Used by the Elm-style renderer to draw the source frame.
    pub fn line_text(&self, file_id: usize, line: usize) -> Option<String> {
        let file = self.files.get(file_id).ok()?;
        file.source()
            .lines()
            .nth(line.saturating_sub(1))
            .map(|s| s.to_string())
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

    /// Get or compute line start positions for a file, using cache when available.
    /// Returns a vector where each element is the byte offset where a line starts.
    /// The first element is always 0 (start of file).
    fn get_line_starts(&mut self, file_id: usize) -> Option<&Vec<usize>> {
        // Check if we already have cached line starts for this file
        if !self.line_starts_cache.contains_key(&file_id) {
            // Need to compute and cache line starts
            if let Ok(file) = self.files.get(file_id) {
                let source = file.source();

                // Build line start positions by scanning for newlines
                let mut line_starts = vec![0];
                for (i, c) in source.char_indices() {
                    if c == '\n' {
                        line_starts.push(i + 1);
                    }
                }

                self.line_starts_cache.insert(file_id, line_starts);
            } else {
                return None;
            }
        }

        self.line_starts_cache.get(&file_id)
    }

    pub fn offset_to_line_col(&mut self, file_id: usize, offset: usize) -> Option<(usize, usize)> {
        let source_len = if let Ok(file) = self.files.get(file_id) {
            file.source().len()
        } else {
            return None;
        };

        // Get cached line starts (this will compute and cache them if needed)
        let line_starts = self.get_line_starts(file_id)?;

        // Use binary search to find which line this offset belongs to.
        // binary_search returns Ok(idx) if the offset is exactly at line_starts[idx],
        // or Err(idx) if the offset would be inserted at position idx.
        let line_idx = match line_starts.binary_search(&offset) {
            Ok(idx) => idx, // Offset is exactly at the start of a line
            Err(idx) => {
                if idx == 0 {
                    0 // Offset is before the first line (shouldn't happen with valid offsets)
                } else {
                    // Offset falls within the line that started at line_starts[idx-1].
                    // We use idx-1 because binary_search returns the insertion point,
                    // which is one past the line where this offset actually belongs.
                    idx - 1
                }
            }
        };

        if line_idx < line_starts.len() {
            let line = line_idx + 1; // Convert to 1-based line numbering for WFL
            let column = offset - line_starts[line_idx] + 1; // Convert to 1-based column numbering for WFL

            // Ensure the offset is within the file bounds
            if offset <= source_len {
                return Some((line, column));
            }
        }

        None
    }

    pub fn line_col_to_offset(
        &mut self,
        file_id: usize,
        line: usize,
        column: usize,
    ) -> Option<usize> {
        // Convert from 1-based to 0-based indexing
        let line = line.saturating_sub(1);
        let column = column.saturating_sub(1);

        let source_len = if let Ok(file) = self.files.get(file_id) {
            file.source().len()
        } else {
            return None;
        };

        // Get cached line starts (this will compute and cache them if needed)
        let line_starts = self.get_line_starts(file_id)?;

        // Check if the line number is valid
        if line < line_starts.len() {
            let line_start_offset = line_starts[line];

            // Get the actual line content to check column bounds
            let line_end = if line + 1 < line_starts.len() {
                line_starts[line + 1] - 1 // Exclude the newline
            } else {
                source_len
            };

            let line_length = line_end - line_start_offset;
            if column <= line_length {
                return Some(line_start_offset + column);
            }
        }

        None
    }

    pub fn convert_parse_error(
        &mut self,
        file_id: usize,
        error: &crate::parser::ast::ParseError,
    ) -> WflDiagnostic {
        let message = error.message.clone();

        // Use span directly if available (start != 0 or end != 0)
        let span = if error.span.start != 0 || error.span.end != 0 {
            error.span
        } else {
            // Fallback for old-style errors or errors without proper spans
            let start_offset = self
                .line_col_to_offset(file_id, error.line, error.column)
                .unwrap_or(0);
            Span {
                start: start_offset,
                end: start_offset + 1,
            }
        };

        let mut diag =
            WflDiagnostic::error(message.clone()).with_primary_label(span, "Error occurred here");

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
        &mut self,
        file_id: usize,
        error: &crate::typechecker::TypeError,
    ) -> WflDiagnostic {
        let mut message_text = error.message.clone();

        if let (Some(expected), Some(found)) = (&error.expected, &error.found) {
            message_text = format!("{message_text} - Expected {expected} but found {found}");
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
        &mut self,
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
        &mut self,
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
        &mut self,
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
            message = format!("Pattern '{name}': {message}");
        }

        if let Some(input) = input_preview {
            let preview = if input.len() > 30 {
                format!("{}...", &input[..30])
            } else {
                input.to_string()
            };
            message = format!("{message} (input: \"{preview}\")");
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
