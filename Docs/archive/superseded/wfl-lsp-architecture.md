# WFL LSP Architecture Documentation

Technical documentation for the WFL Language Server Protocol implementation.

## Overview

The WFL LSP server (`wfl-lsp`) is built using the `tower-lsp` crate and integrates directly with WFL's compiler components to provide rich IDE features.

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    LSP Client (VS Code, Vim, etc.)         │
└─────────────────────┬───────────────────────────────────────┘
                      │ JSON-RPC over stdio/TCP
┌─────────────────────▼───────────────────────────────────────┐
│                 WFL Language Server                         │
│  ┌─────────────────────────────────────────────────────────┐│
│  │              tower-lsp Framework                       ││
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────────┐  ││
│  │  │ Completion  │ │    Hover    │ │   Diagnostics   │  ││
│  │  │   Handler   │ │   Handler   │ │     Handler     │  ││
│  │  └─────────────┘ └─────────────┘ └─────────────────┘  ││
│  └─────────────────────────────────────────────────────────┘│
│  ┌─────────────────────────────────────────────────────────┐│
│  │                WFL Integration Layer                    ││
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────────┐  ││
│  │  │   Symbol    │ │  Document   │ │     Error       │  ││
│  │  │ Management  │ │   Manager   │ │   Converter     │  ││
│  │  └─────────────┘ └─────────────┘ └─────────────────┘  ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────┬───────────────────────────────────────┘
                      │ Direct Function Calls
┌─────────────────────▼───────────────────────────────────────┐
│                 WFL Compiler Components                     │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐│
│  │    Lexer    │ │   Parser    │ │       Analyzer          ││
│  │ (Tokenizer) │ │ (AST Gen)   │ │  (Semantic Analysis)    ││
│  └─────────────┘ └─────────────┘ └─────────────────────────┘│
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐│
│  │Type Checker │ │ Diagnostics │ │    Standard Library     ││
│  │(Type Valid) │ │  Reporter   │ │      Metadata           ││
│  └─────────────┘ └─────────────┘ └─────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. WflLanguageServer

The main LSP server implementation in `wfl-lsp/src/lib.rs`:

```rust
pub struct WflLanguageServer {
    client: Client,
    document_map: DashMap<Url, String>,
}

impl LanguageServer for WflLanguageServer {
    // LSP protocol method implementations
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult>;
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>>;
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>>;
    // ... other LSP methods
}
```

### 2. Document Management

- **Document Storage**: Uses `DashMap<Url, String>` for thread-safe document storage
- **Change Tracking**: Handles `textDocument/didChange` notifications
- **Lifecycle Management**: Manages document open/close/save events

### 3. Symbol Analysis

#### Variable Collection
```rust
fn collect_variables_from_program(program: &Program) -> Vec<CompletionItem> {
    // Traverses AST to find:
    // - VariableDeclaration statements
    // - CreateListStatement declarations  
    // - MapCreation statements
    // - ActionDefinition parameters
}
```

#### Function Collection
```rust
fn collect_functions_from_program(program: &Program) -> Vec<CompletionItem> {
    // Finds ActionDefinition statements
    // Extracts function names and parameters
    // Generates completion items with signatures
}
```

### 4. Standard Library Integration

The LSP server includes comprehensive standard library metadata:

```rust
fn add_stdlib_completions(items: &mut Vec<CompletionItem>) {
    // Text functions: length of, uppercase, lowercase, etc.
    // List functions: first of, last of, sum of, etc.
    // Math functions: random number, round, floor, etc.
    // I/O functions: display, input, file operations
}
```

## LSP Protocol Implementation

### Initialization

```rust
async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
    Ok(InitializeResult {
        capabilities: ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::FULL
            )),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(false),
                trigger_characters: Some(vec![" ".to_string(), ".".to_string()]),
                ..Default::default()
            }),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            ..Default::default()
        },
        ..Default::default()
    })
}
```

### Completion Implementation

```rust
async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
    let uri = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;
    
    if let Some(document_text) = self.document_map.get(&uri) {
        let completion_items = self.collect_completion_items(&document_text, position);
        Ok(Some(CompletionResponse::Array(completion_items)))
    } else {
        Ok(None)
    }
}
```

### Hover Implementation

```rust
async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;
    
    if let Some(document_text) = self.document_map.get(&uri) {
        if let Some(hover_info) = self.get_hover_info(&document_text, position) {
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
```

### Diagnostics Implementation

```rust
async fn did_save(&self, params: DidSaveTextDocumentParams) {
    let uri = params.text_document.uri;
    
    if let Some(document_text) = self.document_map.get(&uri) {
        let diagnostics = self.analyze_document(&document_text);
        
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }
}

fn analyze_document(&self, document_text: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    
    // Lexical analysis
    let tokens = lex_wfl_with_positions(document_text);
    
    // Parse AST
    let mut parser = Parser::new(&tokens);
    match parser.parse() {
        Ok(program) => {
            // Semantic analysis
            let mut analyzer = Analyzer::new();
            if let Err(errors) = analyzer.analyze(&program) {
                diagnostics.extend(convert_analyzer_errors_to_diagnostics(errors));
            }
            
            // Type checking
            let mut type_checker = TypeChecker::new();
            if let Err(errors) = type_checker.check_types(&program) {
                diagnostics.extend(convert_type_errors_to_diagnostics(errors));
            }
        }
        Err(parse_errors) => {
            diagnostics.extend(convert_parse_errors_to_diagnostics(parse_errors));
        }
    }
    
    diagnostics
}
```

## Context-Aware Features

### Completion Context Analysis

```rust
fn get_completion_context(document_text: &str, position: Position) -> CompletionContext {
    let lines: Vec<&str> = document_text.lines().collect();
    let current_line = lines.get(position.line as usize).unwrap_or(&"");
    let before_cursor = &current_line[..position.character as usize.min(current_line.len())];
    
    if before_cursor.ends_with("if ") {
        CompletionContext::Condition
    } else if before_cursor.ends_with("store ") {
        CompletionContext::VariableName
    } else if before_cursor.ends_with(" as ") {
        CompletionContext::Value
    } else if before_cursor.ends_with("length of ") {
        CompletionContext::ListOrText
    } else {
        CompletionContext::General
    }
}
```

### Symbol Resolution

```rust
fn find_symbol_at_position(program: &Program, line: u32, character: u32) -> Option<SymbolInfo> {
    // Traverse AST to find symbol at specific position
    // Returns variable, function, or keyword information
    for statement in &program.statements {
        if let Some(symbol) = find_symbol_in_statement(statement, line, character) {
            return Some(symbol);
        }
    }
    None
}
```

## Performance Optimizations

### 1. Incremental Parsing

```rust
// Cache parsed ASTs for unchanged documents
struct DocumentCache {
    content_hash: u64,
    parsed_program: Program,
    symbols: Vec<Symbol>,
}
```

### 2. Completion Filtering

```rust
fn filter_completions(items: Vec<CompletionItem>, prefix: &str, max_items: usize) -> Vec<CompletionItem> {
    items
        .into_iter()
        .filter(|item| item.label.starts_with(prefix))
        .take(max_items)
        .collect()
}
```

### 3. Async Processing

All LSP operations are async to prevent blocking:

```rust
#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    
    let (service, socket) = LspService::new(|client| WflLanguageServer {
        client,
        document_map: DashMap::new(),
    });
    
    Server::new(stdin, stdout, socket).serve(service).await;
}
```

## Error Handling

### Graceful Degradation

```rust
fn safe_parse_document(document_text: &str) -> ParseResult {
    match parse_wfl_document(document_text) {
        Ok(program) => ParseResult::Success(program),
        Err(errors) => {
            // Continue with partial AST for better user experience
            if let Some(partial_program) = try_partial_parse(document_text) {
                ParseResult::Partial(partial_program, errors)
            } else {
                ParseResult::Failed(errors)
            }
        }
    }
}
```

### Error Recovery

```rust
impl Parser {
    fn recover_from_error(&mut self) -> Result<(), ParseError> {
        // Skip tokens until we find a recovery point
        while let Some(token) = self.peek_token() {
            match token.token {
                Token::EndIf | Token::EndAction | Token::EndCount => break,
                Token::Newline => {
                    self.next_token();
                    break;
                }
                _ => self.next_token(),
            }
        }
        Ok(())
    }
}
```

## Testing Architecture

### Test Structure

```
wfl-lsp/tests/
├── lsp_server_test.rs              # Core LSP functionality
├── lsp_completion_test.rs          # Completion features
├── lsp_hover_test.rs               # Hover information
├── lsp_diagnostics_test.rs         # Error reporting
├── lsp_performance_stability_test.rs # Performance & stability
└── lsp_end_to_end_validation_test.rs # Integration tests
```

### Mock Testing Framework

```rust
struct MockLspServer {
    server: WflLanguageServer,
}

impl MockLspServer {
    async fn send_completion_request(&self, uri: &str, line: u32, character: u32) -> Vec<CompletionItem> {
        // Simulate LSP completion request
    }
    
    async fn send_hover_request(&self, uri: &str, line: u32, character: u32) -> Option<String> {
        // Simulate LSP hover request
    }
}
```

## Configuration System

### Server Configuration

```rust
#[derive(Debug, Clone)]
pub struct LspConfig {
    pub max_completion_items: usize,
    pub hover_timeout_ms: u64,
    pub log_level: LogLevel,
    pub enable_diagnostics: bool,
}

impl Default for LspConfig {
    fn default() -> Self {
        Self {
            max_completion_items: 100,
            hover_timeout_ms: 1000,
            log_level: LogLevel::Warn,
            enable_diagnostics: true,
        }
    }
}
```

### Runtime Configuration Updates

```rust
async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
    if let Some(settings) = params.settings.as_object() {
        if let Some(wfl_settings) = settings.get("wfl") {
            self.update_config_from_json(wfl_settings).await;
        }
    }
}
```

## Future Enhancements

### Planned Features

1. **Go to Definition**: Navigate to symbol definitions
2. **Find References**: Find all symbol usages
3. **Document Symbols**: Outline view for documents
4. **Workspace Symbols**: Project-wide symbol search
5. **Signature Help**: Function parameter hints
6. **Code Actions**: Quick fixes and refactoring
7. **Formatting**: Document and range formatting

### Architecture Extensions

1. **Multi-file Analysis**: Cross-file symbol resolution
2. **Incremental Updates**: Delta-based document updates
3. **Caching Layer**: Persistent symbol cache
4. **Plugin System**: Extensible LSP features

---

For implementation details, see the source code in `wfl-lsp/src/lib.rs` and related test files.
