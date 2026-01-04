use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, Position, Range, Url,
};
use wfl::analyzer::Analyzer;
use wfl::diagnostics::{DiagnosticReporter, WflDiagnostic};
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::{Parser, ast::Program};
use wfl::typechecker::TypeChecker;

/// Represents the state of a document in the workspace
#[derive(Debug, Clone)]
pub struct DocumentState {
    pub uri: String,
    pub text: String,
    pub version: i32,
    pub diagnostics: Vec<WflDiagnostic>,
    pub last_analysis: Option<AnalysisResult>,
}

/// Result of analyzing WFL source code
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub program: Program,
    pub diagnostics: Vec<WflDiagnostic>,
}

/// Shared core for both LSP and MCP servers
/// Provides document management and WFL compiler integration
#[derive(Debug, Clone)]
pub struct WflLanguageCore {
    /// Thread-safe document storage
    documents: Arc<DashMap<String, DocumentState>>,
    /// Optional workspace root path
    workspace_path: Option<PathBuf>,
}

impl WflLanguageCore {
    /// Create a new WflLanguageCore instance
    pub fn new() -> Self {
        WflLanguageCore {
            documents: Arc::new(DashMap::new()),
            workspace_path: None,
        }
    }

    /// Create a new WflLanguageCore with a workspace path
    pub fn with_workspace(workspace_path: PathBuf) -> Self {
        WflLanguageCore {
            documents: Arc::new(DashMap::new()),
            workspace_path: Some(workspace_path),
        }
    }

    /// Get the workspace path, if set
    pub fn workspace_path(&self) -> Option<&PathBuf> {
        self.workspace_path.as_ref()
    }

    /// Add or update a document in the document map
    pub fn add_document(&self, uri: String, text: String, version: i32) {
        let doc_state = DocumentState {
            uri: uri.clone(),
            text,
            version,
            diagnostics: Vec::new(),
            last_analysis: None,
        };
        self.documents.insert(uri, doc_state);
    }

    /// Get a document by URI
    pub fn get_document(&self, uri: &str) -> Option<DocumentState> {
        self.documents.get(uri).map(|entry| entry.value().clone())
    }

    /// Remove a document by URI
    pub fn remove_document(&self, uri: &str) {
        self.documents.remove(uri);
    }

    /// Update document text and version
    pub fn update_document(&self, uri: &str, text: String, version: i32) -> bool {
        if let Some(mut doc) = self.documents.get_mut(uri) {
            doc.text = text;
            doc.version = version;
            doc.last_analysis = None; // Invalidate cached analysis
            true
        } else {
            false
        }
    }

    /// Analyze WFL source code and return diagnostics
    /// This is the core analysis pipeline shared by LSP and MCP
    pub fn analyze_source(
        &self,
        source: &str,
        file_id: usize,
        diagnostic_reporter: &mut DiagnosticReporter,
    ) -> (Vec<WflDiagnostic>, Option<Program>) {
        let mut diagnostics = Vec::new();

        let tokens = lex_wfl_with_positions(source);
        let mut parser = Parser::new(&tokens);

        match parser.parse() {
            Ok(program) => {
                // Run semantic analysis
                let mut analyzer = Analyzer::new();
                if let Err(errors) = analyzer.analyze(&program) {
                    for error in errors {
                        let wfl_diag = diagnostic_reporter.convert_semantic_error(file_id, &error);
                        diagnostics.push(wfl_diag);
                    }
                }

                // Run type checking
                let mut type_checker = TypeChecker::new();
                if let Err(errors) = type_checker.check_types(&program) {
                    for error in errors {
                        let wfl_diag = diagnostic_reporter.convert_type_error(file_id, &error);
                        diagnostics.push(wfl_diag);
                    }
                }

                (diagnostics, Some(program))
            }
            Err(errors) => {
                // Parse errors
                for error in errors {
                    let wfl_diag = diagnostic_reporter.convert_parse_error(file_id, &error);
                    diagnostics.push(wfl_diag);
                }
                (diagnostics, None)
            }
        }
    }

    /// Analyze a document and return LSP diagnostics
    pub fn analyze_document(&self, document_text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut diagnostic_reporter = DiagnosticReporter::new();
        let file_id = diagnostic_reporter.add_file("document.wfl", document_text.to_string());

        let (wfl_diagnostics, _program) =
            self.analyze_source(document_text, file_id, &mut diagnostic_reporter);

        for wfl_diag in wfl_diagnostics {
            diagnostics.push(Self::convert_to_lsp_diagnostic(
                &wfl_diag,
                &mut diagnostic_reporter,
                file_id,
            ));
        }

        diagnostics
    }

    /// Convert WFL diagnostic to LSP diagnostic
    pub fn convert_to_lsp_diagnostic(
        wfl_diag: &WflDiagnostic,
        diagnostic_reporter: &mut DiagnosticReporter,
        file_id: usize,
    ) -> Diagnostic {
        let severity = match wfl_diag.severity {
            wfl::diagnostics::Severity::Error => Some(DiagnosticSeverity::ERROR),
            wfl::diagnostics::Severity::Warning => Some(DiagnosticSeverity::WARNING),
            wfl::diagnostics::Severity::Note => Some(DiagnosticSeverity::INFORMATION),
            wfl::diagnostics::Severity::Help => Some(DiagnosticSeverity::HINT),
        };

        let mut related_information = None;
        if !wfl_diag.notes.is_empty() {
            let related = wfl_diag
                .notes
                .iter()
                .map(|note| DiagnosticRelatedInformation {
                    location: Location {
                        uri: Url::parse("file:///document.wfl").unwrap(),
                        range: Range {
                            start: Position {
                                line: 0,
                                character: 0,
                            },
                            end: Position {
                                line: 0,
                                character: 0,
                            },
                        },
                    },
                    message: note.clone(),
                })
                .collect();
            related_information = Some(related);
        }

        let mut range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 1,
            },
        };

        if let Some((span, _)) = wfl_diag.labels.first() {
            // Use proper line/column conversion instead of rough estimation
            if let Some((start_line, start_character)) =
                diagnostic_reporter.offset_to_line_col(file_id, span.start)
            {
                let (end_line, end_character) = diagnostic_reporter
                    .offset_to_line_col(file_id, span.end)
                    .unwrap_or((start_line, start_character + 1)); // Default to start + 1 if end conversion fails

                range = Range {
                    start: Position {
                        line: (start_line.saturating_sub(1)) as u32, // Convert to 0-based line numbering for LSP
                        character: (start_character.saturating_sub(1)) as u32, // Convert to 0-based column numbering for LSP
                    },
                    end: Position {
                        line: (end_line.saturating_sub(1)) as u32,
                        character: (end_character.saturating_sub(1)) as u32,
                    },
                };
            }
        }

        Diagnostic {
            range,
            severity,
            code: None,
            code_description: None,
            source: Some("wfl".to_string()),
            message: wfl_diag.message.clone(),
            related_information,
            tags: None,
            data: None,
        }
    }
}

impl Default for WflLanguageCore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_creation() {
        let core = WflLanguageCore::new();
        assert!(core.workspace_path().is_none());
    }

    #[test]
    fn test_core_with_workspace() {
        let path = PathBuf::from("/test/workspace");
        let core = WflLanguageCore::with_workspace(path.clone());
        assert_eq!(core.workspace_path(), Some(&path));
    }

    #[test]
    fn test_document_management() {
        let core = WflLanguageCore::new();

        // Add document
        core.add_document(
            "file:///test.wfl".to_string(),
            "store x as 5".to_string(),
            1,
        );

        // Get document
        let doc = core.get_document("file:///test.wfl");
        assert!(doc.is_some());
        assert_eq!(doc.unwrap().text, "store x as 5");

        // Update document
        core.update_document("file:///test.wfl", "store x as 10".to_string(), 2);
        let updated_doc = core.get_document("file:///test.wfl");
        assert_eq!(updated_doc.unwrap().text, "store x as 10");

        // Remove document
        core.remove_document("file:///test.wfl");
        assert!(core.get_document("file:///test.wfl").is_none());
    }

    #[test]
    fn test_analyze_valid_code() {
        let core = WflLanguageCore::new();
        let diagnostics = core.analyze_document("store x as 5");
        assert!(
            diagnostics.is_empty(),
            "Valid code should have no diagnostics"
        );
    }

    #[test]
    fn test_analyze_invalid_code() {
        let core = WflLanguageCore::new();
        let diagnostics = core.analyze_document("store x as");
        assert!(
            !diagnostics.is_empty(),
            "Invalid code should have diagnostics"
        );
    }
}
