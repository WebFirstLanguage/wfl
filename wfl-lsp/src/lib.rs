use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use wfl::analyzer::Analyzer;
use wfl::diagnostics::{DiagnosticReporter, WflDiagnostic};
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::{Parser, ast::Program};
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
        position: Position,
    ) -> Option<Vec<CompletionItem>> {
        let mut items = Vec::new();

        let tokens = lex_wfl_with_positions(document_text);
        let mut parser = Parser::new(&tokens);

        match parser.parse() {
            Ok(program) => {
                // Collect variables from the program
                self.collect_variables_from_program(&program, &mut items);

                // Collect functions from the program
                self.collect_functions_from_program(&program, &mut items);

                // Add standard library functions
                self.add_stdlib_completions(&mut items);

                // Add context-aware completions based on cursor position
                self.add_context_aware_completions(document_text, position, &mut items);
            }
            Err(_) => {
                // Even if parsing fails, provide basic completions
                self.add_stdlib_completions(&mut items);
            }
        }

        Some(items)
    }

    fn collect_variables_from_program(&self, program: &Program, items: &mut Vec<CompletionItem>) {
        use wfl::parser::ast::Statement;

        for statement in &program.statements {
            match statement {
                Statement::VariableDeclaration { name, .. } => {
                    items.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: Some(format!("Variable: {}", name)),
                        insert_text: Some(name.clone()),
                        ..CompletionItem::default()
                    });
                }
                Statement::CreateListStatement { name, .. } => {
                    items.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: Some(format!("List variable: {}", name)),
                        insert_text: Some(name.clone()),
                        ..CompletionItem::default()
                    });
                }
                Statement::MapCreation { name, .. } => {
                    items.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: Some(format!("Map variable: {}", name)),
                        insert_text: Some(name.clone()),
                        ..CompletionItem::default()
                    });
                }
                Statement::CreateDateStatement { name, .. } => {
                    items.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: Some(format!("Date variable: {}", name)),
                        insert_text: Some(name.clone()),
                        ..CompletionItem::default()
                    });
                }
                Statement::CreateTimeStatement { name, .. } => {
                    items.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: Some(format!("Time variable: {}", name)),
                        insert_text: Some(name.clone()),
                        ..CompletionItem::default()
                    });
                }
                _ => {}
            }
        }
    }

    fn collect_functions_from_program(&self, program: &Program, items: &mut Vec<CompletionItem>) {
        use wfl::parser::ast::Statement;

        for statement in &program.statements {
            if let Statement::ActionDefinition { name, parameters, .. } = statement {
                let param_list = parameters.iter()
                    .map(|p| p.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ");

                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some(format!("Function: {}({})", name, param_list)),
                    insert_text: Some(format!("{}({})", name,
                        parameters.iter()
                            .enumerate()
                            .map(|(i, p)| format!("${{{}:{}}}", i + 1, p.name))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    ..CompletionItem::default()
                });
            }
        }
    }

    fn add_stdlib_completions(&self, items: &mut Vec<CompletionItem>) {
        let stdlib_functions = [
            // Core functions
            ("length of", "length of ${1:collection}", "Get the length of a collection"),
            ("first of", "first of ${1:collection}", "Get the first item of a collection"),
            ("last of", "last of ${1:collection}", "Get the last item of a collection"),
            ("add", "add ${1:item} to ${2:collection}", "Add an item to a collection"),
            ("remove", "remove ${1:item} from ${2:collection}", "Remove an item from a collection"),
            ("contains", "${1:collection} contains ${2:item}", "Check if collection contains item"),

            // Text functions
            ("uppercase", "uppercase ${1:text}", "Convert text to uppercase"),
            ("lowercase", "lowercase ${1:text}", "Convert text to lowercase"),
            ("trim", "trim ${1:text}", "Remove whitespace from text"),
            ("replace", "replace ${1:old} with ${2:new} in ${3:text}", "Replace text"),
            ("substring", "substring of ${1:text} from ${2:start} to ${3:end}", "Extract substring"),
            ("join", "join ${1:collection} with ${2:separator}", "Join collection with separator"),
            ("split", "split ${1:text} by ${2:separator}", "Split text by separator"),

            // Math functions
            ("random", "random number", "Generate random number"),
            ("random between", "random number between ${1:min} and ${2:max}", "Random number in range"),
            ("round", "round ${1:number}", "Round number to nearest integer"),
            ("floor", "floor ${1:number}", "Round number down"),
            ("ceiling", "ceiling ${1:number}", "Round number up"),
            ("absolute", "absolute value of ${1:number}", "Get absolute value"),

            // List functions
            ("sort", "sort ${1:list}", "Sort a list"),
            ("reverse", "reverse ${1:list}", "Reverse a list"),
            ("clear", "clear ${1:list}", "Clear all items from list"),

            // Time functions
            ("today", "today", "Get current date"),
            ("now", "now", "Get current time"),
            ("format date", "format ${1:date} as ${2:format}", "Format date"),
            ("format time", "format ${1:time} as ${2:format}", "Format time"),
        ];

        for (label, snippet, description) in &stdlib_functions {
            items.push(CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(format!("WFL stdlib: {}", description)),
                insert_text: Some(snippet.to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..CompletionItem::default()
            });
        }
    }

    fn add_context_aware_completions(&self, document_text: &str, position: Position, items: &mut Vec<CompletionItem>) {
        // Get the line where completion is requested
        let lines: Vec<&str> = document_text.lines().collect();
        if position.line as usize >= lines.len() {
            return;
        }

        let current_line = lines[position.line as usize];
        let line_prefix = &current_line[..position.character.min(current_line.len() as u32) as usize];

        // Context-aware completions based on what comes before the cursor
        if line_prefix.trim_end().ends_with("if") || line_prefix.contains("if ") {
            // After 'if', suggest comparison patterns
            let comparisons = [
                ("is equal to", "${1:variable} is equal to ${2:value}"),
                ("is greater than", "${1:variable} is greater than ${2:value}"),
                ("is less than", "${1:variable} is less than ${2:value}"),
                ("is not equal to", "${1:variable} is not equal to ${2:value}"),
                ("contains", "${1:collection} contains ${2:item}"),
                ("is empty", "${1:collection} is empty"),
                ("is not empty", "${1:collection} is not empty"),
            ];

            for (label, snippet) in &comparisons {
                items.push(CompletionItem {
                    label: label.to_string(),
                    kind: Some(CompletionItemKind::OPERATOR),
                    detail: Some("Comparison operator".to_string()),
                    insert_text: Some(snippet.to_string()),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    ..CompletionItem::default()
                });
            }
        }

        if line_prefix.trim_end().ends_with("store") || line_prefix.contains("store ") {
            // After 'store', suggest 'as' keyword
            items.push(CompletionItem {
                label: "as".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("WFL keyword: as".to_string()),
                insert_text: Some("${1:variable_name} as ${2:value}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..CompletionItem::default()
            });
        }

        if line_prefix.trim_end().ends_with("count") {
            // After 'count', suggest count loop patterns
            items.push(CompletionItem {
                label: "from".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Count loop: from".to_string()),
                insert_text: Some("from ${1:variable} as ${2:start} to ${3:end}".to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..CompletionItem::default()
            });
        }
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
