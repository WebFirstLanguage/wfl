use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::{Parser, ast::Program};

pub mod core;
pub mod mcp_server;

pub use core::WflLanguageCore;

/// Converts UTF-16 column position to UTF-8 byte offset
/// LSP Position.character uses UTF-16 code units, but Rust strings use UTF-8 bytes
pub fn byte_offset_for_utf16_col(s: &str, utf16_col: u32) -> usize {
    let mut utf16_pos = 0usize;
    for (byte_idx, ch) in s.char_indices() {
        if utf16_pos >= utf16_col as usize {
            return byte_idx;
        }
        // Count UTF-16 code units (BMP=1, non-BMP=2)
        utf16_pos += if (ch as u32) < 0x10000 { 1 } else { 2 };
    }
    s.len()
}

#[derive(Debug)]
pub struct WflLanguageServer {
    client: Client,
    core: WflLanguageCore,
    document_map: DashMap<String, String>,
}

impl WflLanguageServer {
    pub fn new(client: Client) -> Self {
        WflLanguageServer {
            client,
            core: WflLanguageCore::new(),
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
        // Use the shared core for analysis
        self.core.analyze_document(document_text)
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
            if let Statement::ActionDefinition {
                name, parameters, ..
            } = statement
            {
                let param_list = parameters
                    .iter()
                    .map(|p| p.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ");

                items.push(CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some(format!("Function: {}({})", name, param_list)),
                    insert_text: Some(format!(
                        "{}({})",
                        name,
                        parameters
                            .iter()
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
            (
                "length of",
                "length of ${1:collection}",
                "Get the length of a collection",
            ),
            (
                "first of",
                "first of ${1:collection}",
                "Get the first item of a collection",
            ),
            (
                "last of",
                "last of ${1:collection}",
                "Get the last item of a collection",
            ),
            (
                "add",
                "add ${1:item} to ${2:collection}",
                "Add an item to a collection",
            ),
            (
                "remove",
                "remove ${1:item} from ${2:collection}",
                "Remove an item from a collection",
            ),
            (
                "contains",
                "${1:collection} contains ${2:item}",
                "Check if collection contains item",
            ),
            // Text functions
            (
                "uppercase",
                "uppercase ${1:text}",
                "Convert text to uppercase",
            ),
            (
                "lowercase",
                "lowercase ${1:text}",
                "Convert text to lowercase",
            ),
            ("trim", "trim ${1:text}", "Remove whitespace from text"),
            (
                "replace",
                "replace ${1:old} with ${2:new} in ${3:text}",
                "Replace text",
            ),
            (
                "substring",
                "substring of ${1:text} from ${2:start} to ${3:end}",
                "Extract substring",
            ),
            (
                "join",
                "join ${1:collection} with ${2:separator}",
                "Join collection with separator",
            ),
            (
                "split",
                "split ${1:text} by ${2:separator}",
                "Split text by separator",
            ),
            // Math functions
            ("random", "random number", "Generate random number"),
            (
                "random between",
                "random number between ${1:min} and ${2:max}",
                "Random number in range",
            ),
            (
                "round",
                "round ${1:number}",
                "Round number to nearest integer",
            ),
            ("floor", "floor ${1:number}", "Round number down"),
            ("ceiling", "ceiling ${1:number}", "Round number up"),
            (
                "absolute",
                "absolute value of ${1:number}",
                "Get absolute value",
            ),
            // List functions
            ("sort", "sort ${1:list}", "Sort a list"),
            ("reverse", "reverse ${1:list}", "Reverse a list"),
            ("clear", "clear ${1:list}", "Clear all items from list"),
            // Time functions
            ("today", "today", "Get current date"),
            ("now", "now", "Get current time"),
            (
                "format date",
                "format ${1:date} as ${2:format}",
                "Format date",
            ),
            (
                "format time",
                "format ${1:time} as ${2:format}",
                "Format time",
            ),
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

    fn add_context_aware_completions(
        &self,
        document_text: &str,
        position: Position,
        items: &mut Vec<CompletionItem>,
    ) {
        // Get the line where completion is requested
        let lines: Vec<&str> = document_text.lines().collect();
        if position.line as usize >= lines.len() {
            return;
        }

        let current_line = lines[position.line as usize];
        let end = byte_offset_for_utf16_col(current_line, position.character);
        let line_prefix = &current_line[..end];

        // Context-aware completions based on what comes before the cursor
        if line_prefix.trim_end().ends_with("if") || line_prefix.contains("if ") {
            // After 'if', suggest comparison patterns
            let comparisons = [
                ("is equal to", "${1:variable} is equal to ${2:value}"),
                (
                    "is greater than",
                    "${1:variable} is greater than ${2:value}",
                ),
                ("is less than", "${1:variable} is less than ${2:value}"),
                (
                    "is not equal to",
                    "${1:variable} is not equal to ${2:value}",
                ),
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

    fn get_hover_info(&self, document_text: &str, position: Position) -> Option<String> {
        // Try to find symbol information at the given position
        let tokens = lex_wfl_with_positions(document_text);
        let mut parser = Parser::new(&tokens);

        match parser.parse() {
            Ok(program) => {
                // Look for symbols at the position
                if let Some(symbol_info) =
                    self.find_symbol_at_position(&program, document_text, position)
                {
                    Some(self.format_hover_info(&symbol_info))
                } else {
                    // Check for keywords or stdlib functions at position
                    self.get_keyword_or_stdlib_hover(document_text, position)
                }
            }
            Err(_) => {
                // Even if parsing fails, try to provide keyword/stdlib hover
                self.get_keyword_or_stdlib_hover(document_text, position)
            }
        }
    }

    fn find_symbol_at_position(
        &self,
        program: &Program,
        document_text: &str,
        position: Position,
    ) -> Option<SymbolInfo> {
        use wfl::parser::ast::Statement;

        // Get the word at the cursor position
        let word_at_position = self.get_word_at_position(document_text, position)?;

        // Search for variables
        for statement in &program.statements {
            match statement {
                Statement::VariableDeclaration { name, .. } if name == &word_at_position => {
                    return Some(SymbolInfo::Variable {
                        name: name.clone(),
                        var_type: self.infer_variable_type(statement),
                        value: self.get_variable_value(statement),
                    });
                }
                Statement::ActionDefinition {
                    name, parameters, ..
                } if name == &word_at_position => {
                    let param_names: Vec<String> =
                        parameters.iter().map(|p| p.name.clone()).collect();
                    return Some(SymbolInfo::Function {
                        name: name.clone(),
                        parameters: param_names,
                        return_type: Some("any".to_string()), // Could be enhanced with type analysis
                    });
                }
                _ => {}
            }
        }

        None
    }

    fn get_word_at_position(&self, document_text: &str, position: Position) -> Option<String> {
        let lines: Vec<&str> = document_text.lines().collect();
        if position.line as usize >= lines.len() {
            return None;
        }

        let line = lines[position.line as usize];
        let byte_pos = byte_offset_for_utf16_col(line, position.character);

        if byte_pos >= line.len() {
            return None;
        }

        // Find word boundaries around the cursor position
        let chars: Vec<char> = line.chars().collect();

        // Convert byte position to character index
        let char_pos = line[..byte_pos].chars().count();

        // Find start of word
        let mut start = char_pos;
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }

        // Find end of word
        let mut end = char_pos;
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }

        if start < end {
            Some(chars[start..end].iter().collect())
        } else {
            None
        }
    }

    fn infer_variable_type(&self, statement: &wfl::parser::ast::Statement) -> String {
        use wfl::parser::ast::{Expression, Literal, Statement};

        match statement {
            Statement::VariableDeclaration { value, .. } => match value {
                Expression::Literal(Literal::String(_), _, _) => "text".to_string(),
                Expression::Literal(Literal::Integer(_), _, _) => "number".to_string(),
                Expression::Literal(Literal::Float(_), _, _) => "number".to_string(),
                Expression::Literal(Literal::Boolean(_), _, _) => "boolean".to_string(),
                Expression::Literal(Literal::Nothing, _, _) => "nothing".to_string(),
                _ => "any".to_string(),
            },
            _ => "unknown".to_string(),
        }
    }

    fn get_variable_value(&self, statement: &wfl::parser::ast::Statement) -> Option<String> {
        use wfl::parser::ast::{Expression, Literal, Statement};

        match statement {
            Statement::VariableDeclaration { value, .. } => match value {
                Expression::Literal(Literal::String(s), _, _) => Some(format!("\"{}\"", s)),
                Expression::Literal(Literal::Integer(i), _, _) => Some(i.to_string()),
                Expression::Literal(Literal::Float(f), _, _) => Some(f.to_string()),
                Expression::Literal(Literal::Boolean(b), _, _) => Some(if *b {
                    "yes".to_string()
                } else {
                    "no".to_string()
                }),
                Expression::Literal(Literal::Nothing, _, _) => Some("nothing".to_string()),
                _ => None,
            },
            _ => None,
        }
    }

    fn get_keyword_or_stdlib_hover(
        &self,
        document_text: &str,
        position: Position,
    ) -> Option<String> {
        let word = self.get_word_at_position(document_text, position)?;

        // Check for WFL keywords
        if let Some(keyword_info) = self.get_keyword_info(&word) {
            return Some(self.format_hover_info(&keyword_info));
        }

        // Check for stdlib functions (including multi-word ones like "length of")
        if let Some(stdlib_info) = self.get_stdlib_function_info(document_text, position) {
            return Some(self.format_hover_info(&stdlib_info));
        }

        None
    }

    fn get_keyword_info(&self, word: &str) -> Option<SymbolInfo> {
        let keywords = [
            (
                "if",
                "Conditional statement - executes code block if condition is true",
            ),
            ("then", "Marks the beginning of the if block"),
            ("otherwise", "Alternative block for if statement (else)"),
            ("end", "Marks the end of a code block"),
            (
                "count",
                "Loop statement - repeats code block for a range of values",
            ),
            ("from", "Specifies the start of a count loop"),
            ("to", "Specifies the end of a count loop"),
            ("store", "Creates a new variable and assigns a value"),
            ("create", "Creates a new variable (alias for store)"),
            ("display", "Outputs text or values to the console"),
            ("call", "Invokes a function or action"),
            ("define", "Defines a new function or action"),
            ("action", "Keyword used in function definitions"),
            ("called", "Keyword used in function definitions"),
            ("with", "Specifies parameters or arguments"),
            ("and", "Logical AND operator or parameter separator"),
            ("or", "Logical OR operator"),
            ("not", "Logical NOT operator"),
            ("is", "Comparison operator"),
            ("equal", "Part of equality comparison"),
            ("greater", "Part of greater than comparison"),
            ("less", "Part of less than comparison"),
            ("than", "Part of comparison operators"),
            ("return", "Returns a value from a function"),
            ("try", "Begins error handling block"),
            ("catch", "Handles errors in try block"),
            ("when", "Conditional error handling"),
            ("error", "Error handling keyword"),
        ];

        for (keyword, description) in &keywords {
            if word == *keyword {
                return Some(SymbolInfo::Keyword {
                    name: keyword.to_string(),
                    description: description.to_string(),
                });
            }
        }

        None
    }

    fn get_stdlib_function_info(
        &self,
        document_text: &str,
        position: Position,
    ) -> Option<SymbolInfo> {
        let lines: Vec<&str> = document_text.lines().collect();
        if position.line as usize >= lines.len() {
            return None;
        }

        let line = lines[position.line as usize];
        let byte_pos = byte_offset_for_utf16_col(line, position.character);
        let char_pos = line[..byte_pos].chars().count();

        // Look for multi-word stdlib functions around the cursor
        let stdlib_functions = [
            (
                "length of",
                "length of collection",
                "Returns the number of items in a collection",
            ),
            (
                "first of",
                "first of collection",
                "Returns the first item in a collection",
            ),
            (
                "last of",
                "last of collection",
                "Returns the last item in a collection",
            ),
            ("uppercase", "uppercase text", "Converts text to uppercase"),
            ("lowercase", "lowercase text", "Converts text to lowercase"),
            (
                "trim",
                "trim text",
                "Removes whitespace from the beginning and end of text",
            ),
            (
                "random",
                "random number",
                "Generates a random number between 0 and 1",
            ),
            (
                "round",
                "round number",
                "Rounds a number to the nearest integer",
            ),
            (
                "floor",
                "floor number",
                "Rounds a number down to the nearest integer",
            ),
            (
                "ceiling",
                "ceiling number",
                "Rounds a number up to the nearest integer",
            ),
            (
                "absolute",
                "absolute value of number",
                "Returns the absolute value of a number",
            ),
            (
                "join",
                "join collection with separator",
                "Joins collection items with a separator",
            ),
            (
                "split",
                "split text by separator",
                "Splits text into a collection using a separator",
            ),
            (
                "replace",
                "replace old with new in text",
                "Replaces occurrences of old text with new text",
            ),
            (
                "substring",
                "substring of text from start to end",
                "Extracts a portion of text",
            ),
            (
                "contains",
                "collection contains item",
                "Checks if a collection contains an item",
            ),
            (
                "add",
                "add item to collection",
                "Adds an item to a collection",
            ),
            (
                "remove",
                "remove item from collection",
                "Removes an item from a collection",
            ),
            (
                "sort",
                "sort collection",
                "Sorts a collection in ascending order",
            ),
            (
                "reverse",
                "reverse collection",
                "Reverses the order of items in a collection",
            ),
            (
                "clear",
                "clear collection",
                "Removes all items from a collection",
            ),
            ("today", "today", "Returns the current date"),
            ("now", "now", "Returns the current time"),
        ];

        // Check if any stdlib function appears around the cursor position
        for (func_name, signature, description) in &stdlib_functions {
            if line.contains(func_name) {
                // Check if cursor is within the function name
                if let Some(func_pos) = line.find(func_name) {
                    let func_end = func_pos + func_name.len();
                    if char_pos >= func_pos && char_pos <= func_end {
                        return Some(SymbolInfo::StdlibFunction {
                            name: func_name.to_string(),
                            description: description.to_string(),
                            signature: signature.to_string(),
                        });
                    }
                }
            }
        }

        None
    }

    fn format_hover_info(&self, symbol_info: &SymbolInfo) -> String {
        match symbol_info {
            SymbolInfo::Variable {
                name,
                var_type,
                value,
            } => {
                let mut info = format!("**Variable:** `{}`\n\n**Type:** `{}`", name, var_type);
                if let Some(val) = value {
                    info.push_str(&format!("\n\n**Value:** `{}`", val));
                }
                info
            }
            SymbolInfo::Function {
                name,
                parameters,
                return_type,
            } => {
                let params = parameters.join(", ");
                let mut info = format!("**Function:** `{}({})`", name, params);
                if let Some(ret_type) = return_type {
                    info.push_str(&format!("\n\n**Returns:** `{}`", ret_type));
                }
                info.push_str("\n\n*User-defined function*");
                info
            }
            SymbolInfo::Keyword { name, description } => {
                format!("**WFL Keyword:** `{}`\n\n{}", name, description)
            }
            SymbolInfo::StdlibFunction {
                name: _,
                description,
                signature,
            } => {
                format!(
                    "**WFL Standard Library**\n\n`{}`\n\n{}",
                    signature, description
                )
            }
        }
    }
}

#[derive(Debug, Clone)]
enum SymbolInfo {
    Variable {
        name: String,
        var_type: String,
        value: Option<String>,
    },
    Function {
        name: String,
        parameters: Vec<String>,
        return_type: Option<String>,
    },
    Keyword {
        name: String,
        description: String,
    },
    StdlibFunction {
        #[allow(dead_code)]
        name: String,
        description: String,
        signature: String,
    },
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
        let position = params.text_document_position_params.position;

        if let Some(document) = self.document_map.get(&uri.to_string()) {
            if let Some(hover_info) = self.get_hover_info(&document, position) {
                Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: hover_info,
                    }),
                    range: None,
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_offset_for_utf16_col_ascii() {
        let s = "hello world";
        assert_eq!(byte_offset_for_utf16_col(s, 0), 0);
        assert_eq!(byte_offset_for_utf16_col(s, 5), 5);
        assert_eq!(byte_offset_for_utf16_col(s, 11), 11);
        assert_eq!(byte_offset_for_utf16_col(s, 15), 11); // Beyond end
    }

    #[test]
    fn test_byte_offset_for_utf16_col_non_ascii() {
        let s = "hello 世界"; // "hello " is 6 bytes, "世" is 3 bytes, "界" is 3 bytes
        assert_eq!(byte_offset_for_utf16_col(s, 0), 0);
        assert_eq!(byte_offset_for_utf16_col(s, 6), 6); // At start of "世"
        assert_eq!(byte_offset_for_utf16_col(s, 7), 9); // At start of "界" (世 is 1 UTF-16 code unit)
        assert_eq!(byte_offset_for_utf16_col(s, 8), 12); // At end
    }

    #[test]
    fn test_byte_offset_for_utf16_col_emoji() {
        let s = "hi 🦀 rust"; // "hi " is 3 bytes, "🦀" is 4 bytes (2 UTF-16 code units), " rust" is 5 bytes
        assert_eq!(byte_offset_for_utf16_col(s, 0), 0);
        assert_eq!(byte_offset_for_utf16_col(s, 3), 3); // At start of crab emoji
        assert_eq!(byte_offset_for_utf16_col(s, 5), 7); // After crab emoji (it uses 2 UTF-16 code units)
        assert_eq!(byte_offset_for_utf16_col(s, 10), 12); // At end
    }

    #[test]
    fn test_byte_offset_for_utf16_col_surrogate_pairs() {
        let s = "test 𝕏 end"; // "test " is 5 bytes, "𝕏" is 4 bytes (2 UTF-16 code units), " end" is 4 bytes
        assert_eq!(byte_offset_for_utf16_col(s, 0), 0);
        assert_eq!(byte_offset_for_utf16_col(s, 5), 5); // At start of "𝕏"
        assert_eq!(byte_offset_for_utf16_col(s, 7), 9); // After "𝕏" (it uses 2 UTF-16 code units)
        assert_eq!(byte_offset_for_utf16_col(s, 11), 13); // At end
    }
}
