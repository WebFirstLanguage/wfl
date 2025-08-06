use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use wfl::analyzer::Analyzer;
use wfl::diagnostics::{DiagnosticReporter, WflDiagnostic};
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

#[derive(Debug)]
pub struct WflLanguageServer {
    client: Client,
    document_map: DashMap<String, String>,
}

impl WflLanguageServer {
    pub fn new(client: Client) -> Self {
        WflLanguageServer {
            client,
            document_map: DashMap::new(),
        }
    }

    async fn validate_document(&self, uri: Url) {
        let diagnostics = if let Some(document) = self.document_map.get(&uri.to_string()) {
            self.analyze_document(&document)
        } else {
            Vec::new()
        };

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    fn analyze_document(&self, document_text: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut diagnostic_reporter = DiagnosticReporter::new();
        let file_id = diagnostic_reporter.add_file("document.wfl", document_text.to_string());

        let tokens = lex_wfl_with_positions(document_text);

        let mut parser = Parser::new(&tokens);
        match parser.parse() {
            Ok(program) => {
                let mut analyzer = Analyzer::new();
                if let Err(errors) = analyzer.analyze(&program) {
                    for error in errors {
                        let wfl_diag = diagnostic_reporter.convert_semantic_error(file_id, &error);
                        diagnostics.push(self.convert_to_lsp_diagnostic(
                            &wfl_diag,
                            &mut diagnostic_reporter,
                            file_id,
                        ));
                    }
                }

                let mut type_checker = TypeChecker::new();
                if let Err(errors) = type_checker.check_types(&program) {
                    for error in errors {
                        let wfl_diag = diagnostic_reporter.convert_type_error(file_id, &error);
                        diagnostics.push(self.convert_to_lsp_diagnostic(
                            &wfl_diag,
                            &mut diagnostic_reporter,
                            file_id,
                        ));
                    }
                }
            }
            Err(errors) => {
                for error in errors {
                    let wfl_diag = diagnostic_reporter.convert_parse_error(file_id, &error);
                    diagnostics.push(self.convert_to_lsp_diagnostic(
                        &wfl_diag,
                        &mut diagnostic_reporter,
                        file_id,
                    ));
                }
            }
        }

        diagnostics
    }

    fn convert_to_lsp_diagnostic(
        &self,
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

    fn collect_completion_items(
        &self,
        document_text: &str,
        position: Position,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        self.add_keyword_completions(&mut items);

        if let Some(scope_items) = self.get_scope_items(document_text, position) {
            items.extend(scope_items);
        }

        items
    }

    fn add_keyword_completions(&self, items: &mut Vec<CompletionItem>) {
        let keywords = [
            ("store", "store ${1:variable_name} as ${2:value}"),
            ("create", "create ${1:variable_name} as ${2:value}"),
            ("display", "display ${1:expression}"),
            (
                "check if",
                "check if ${1:condition}:\n\t${2:statements}\nend check",
            ),
            (
                "count from",
                "count from ${1:start} to ${2:end}:\n\t${3:statements}\nend count",
            ),
            (
                "for each",
                "for each ${1:item} in ${2:collection}:\n\t${3:statements}\nend for each",
            ),
            (
                "define action",
                "define action called ${1:name}:\n\t${2:statements}\nend action",
            ),
            ("open file", "open file at \"${1:path}\" and read content"),
            (
                "repeat while",
                "repeat while ${1:condition}:\n\t${2:statements}\nend repeat",
            ),
            (
                "repeat until",
                "repeat until ${1:condition}:\n\t${2:statements}\nend repeat",
            ),
            ("give back", "give back ${1:value}"),
            (
                "try",
                "try:\n\t${1:statements}\nwhen error:\n\t${2:error_handling}\nend try",
            ),
        ];

        for (keyword, snippet) in keywords {
            items.push(CompletionItem {
                label: keyword.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(format!("WFL keyword: {}", keyword)),
                insert_text: Some(snippet.to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..CompletionItem::default()
            });
        }
    }

    fn get_scope_items(
        &self,
        document_text: &str,
        _position: Position,
    ) -> Option<Vec<CompletionItem>> {
        let mut items = Vec::new();

        let tokens = lex_wfl_with_positions(document_text);
        let mut parser = Parser::new(&tokens);

        match parser.parse() {
            Ok(program) => {
                let mut analyzer = Analyzer::new();
                if analyzer.analyze(&program).is_ok() {
                    items.push(CompletionItem {
                        label: "example_variable".to_string(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: Some("Example variable (placeholder)".to_string()),
                        ..CompletionItem::default()
                    });

                    items.push(CompletionItem {
                        label: "example_function".to_string(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some("Example function (placeholder)".to_string()),
                        ..CompletionItem::default()
                    });
                }
            }
            Err(_) => {}
        }

        Some(items)
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for WflLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![" ".to_string()]),
                    ..CompletionOptions::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "WFL Language Server".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "WFL language server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;

        self.document_map.insert(uri.to_string(), text);
        self.validate_document(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        if let Some(change) = params.content_changes.last() {
            self.document_map
                .insert(uri.to_string(), change.text.clone());
            self.validate_document(uri).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.document_map.remove(&uri.to_string());

        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        if let Some(document) = self.document_map.get(&uri.to_string()) {
            let items = self.collect_completion_items(&document, position);
            Ok(Some(CompletionResponse::Array(items)))
        } else {
            Ok(None)
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let _position = params.text_document_position_params.position;

        if let Some(_document) = self.document_map.get(&uri.to_string()) {
            Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: "WFL symbol information would appear here.".to_string(),
                }),
                range: None,
            }))
        } else {
            Ok(None)
        }
    }
}
