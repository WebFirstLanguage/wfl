use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::WflLanguageCore;
use wfl::analyzer::Analyzer;
use wfl::diagnostics::DiagnosticReporter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

/// JSON-RPC 2.0 Request
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// MCP Server implementation
pub struct WflMcpServer {
    core: Arc<WflLanguageCore>,
    workspace_root: Option<PathBuf>,
}

impl WflMcpServer {
    pub fn new() -> Self {
        // Try to get workspace from current directory
        let workspace_root = std::env::current_dir().ok();
        WflMcpServer {
            core: Arc::new(WflLanguageCore::new()),
            workspace_root,
        }
    }

    pub fn with_workspace(workspace_path: PathBuf) -> Self {
        WflMcpServer {
            core: Arc::new(WflLanguageCore::with_workspace(workspace_path.clone())),
            workspace_root: Some(workspace_path),
        }
    }

    /// Handle MCP initialize request
    fn handle_initialize(&self, id: Option<Value>) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "resources": {},
                },
                "serverInfo": {
                    "name": "wfl-lsp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
            error: None,
        }
    }

    /// Handle tools/list request
    fn handle_tools_list(&self, id: Option<Value>) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "tools": [
                    {
                        "name": "parse_wfl",
                        "description": "Parse WFL source code and return the Abstract Syntax Tree (AST)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "source": {
                                    "type": "string",
                                    "description": "WFL source code to parse"
                                },
                                "include_positions": {
                                    "type": "boolean",
                                    "description": "Whether to include position information in the AST",
                                    "default": true
                                }
                            },
                            "required": ["source"]
                        }
                    },
                    {
                        "name": "analyze_wfl",
                        "description": "Run semantic analysis on WFL code and return diagnostics",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "source": {
                                    "type": "string",
                                    "description": "WFL source code to analyze"
                                }
                            },
                            "required": ["source"]
                        }
                    },
                    {
                        "name": "typecheck_wfl",
                        "description": "Run type checker on WFL code and return type errors",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "source": {
                                    "type": "string",
                                    "description": "WFL source code to type check"
                                }
                            },
                            "required": ["source"]
                        }
                    },
                    {
                        "name": "lint_wfl",
                        "description": "Lint WFL code and suggest improvements",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "source": {
                                    "type": "string",
                                    "description": "WFL source code to lint"
                                }
                            },
                            "required": ["source"]
                        }
                    },
                    {
                        "name": "get_completions",
                        "description": "Get code completion suggestions at a specific position",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "source": {
                                    "type": "string",
                                    "description": "WFL source code"
                                },
                                "line": {
                                    "type": "number",
                                    "description": "Line number (0-based)"
                                },
                                "column": {
                                    "type": "number",
                                    "description": "Column number (0-based)"
                                }
                            },
                            "required": ["source", "line", "column"]
                        }
                    },
                    {
                        "name": "get_symbol_info",
                        "description": "Get information about a symbol at a specific position",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "source": {
                                    "type": "string",
                                    "description": "WFL source code"
                                },
                                "line": {
                                    "type": "number",
                                    "description": "Line number (0-based)"
                                },
                                "column": {
                                    "type": "number",
                                    "description": "Column number (0-based)"
                                }
                            },
                            "required": ["source", "line", "column"]
                        }
                    }
                ]
            })),
            error: None,
        }
    }

    /// Handle tools/call request
    fn handle_tools_call(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        // Extract tool name and arguments
        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match tool_name {
            "parse_wfl" => self.handle_parse_wfl(id, params),
            "analyze_wfl" => self.handle_analyze_wfl(id, params),
            "typecheck_wfl" => self.handle_typecheck_wfl(id, params),
            "lint_wfl" => self.handle_lint_wfl(id, params),
            "get_completions" => self.handle_get_completions(id, params),
            "get_symbol_info" => self.handle_get_symbol_info(id, params),
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Unknown tool: {}", tool_name),
                    data: None,
                }),
            },
        }
    }

    /// Handle parse_wfl tool
    fn handle_parse_wfl(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let arguments = match params.get("arguments") {
            Some(args) => args,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing 'arguments' in tool call".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let source = match arguments.get("source").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing or invalid 'source' parameter".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let include_positions = arguments
            .get("include_positions")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        // Parse the WFL code
        let tokens = lex_wfl_with_positions(source);
        let mut parser = Parser::new(&tokens);

        match parser.parse() {
            Ok(program) => {
                let statement_count = program.statements.len();
                let ast_representation = if include_positions {
                    format!("{:#?}", program)
                } else {
                    program
                        .statements
                        .iter()
                        .enumerate()
                        .map(|(i, stmt)| format!("Statement {}: {:?}", i + 1, stmt))
                        .collect::<Vec<_>>()
                        .join("\n")
                };

                let result = json!({
                    "success": true,
                    "statement_count": statement_count,
                    "ast": ast_representation,
                    "message": format!("Successfully parsed {} statement(s)", statement_count)
                });

                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string_pretty(&result).unwrap()
                            }
                        ]
                    })),
                    error: None,
                }
            }
            Err(errors) => {
                let error_messages: Vec<String> =
                    errors.iter().map(|e| format!("{:?}", e)).collect();

                let result = json!({
                    "success": false,
                    "errors": error_messages,
                    "error_count": errors.len(),
                    "message": format!("Parse failed with {} error(s)", errors.len())
                });

                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string_pretty(&result).unwrap()
                            }
                        ],
                        "isError": true
                    })),
                    error: None,
                }
            }
        }
    }

    /// Handle analyze_wfl tool - semantic analysis
    fn handle_analyze_wfl(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let arguments = match params.get("arguments") {
            Some(args) => args,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing 'arguments' in tool call".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let source = match arguments.get("source").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing or invalid 'source' parameter".to_string(),
                        data: None,
                    }),
                };
            }
        };

        // Use the shared core for analysis
        let diagnostics = self.core.analyze_document(source);

        let result = json!({
            "success": true,
            "diagnostic_count": diagnostics.len(),
            "diagnostics": diagnostics.iter().map(|d| {
                json!({
                    "message": d.message,
                    "severity": format!("{:?}", d.severity),
                    "range": {
                        "start": {"line": d.range.start.line, "character": d.range.start.character},
                        "end": {"line": d.range.end.line, "character": d.range.end.character}
                    }
                })
            }).collect::<Vec<_>>(),
            "message": if diagnostics.is_empty() {
                "No issues found".to_string()
            } else {
                format!("Found {} diagnostic(s)", diagnostics.len())
            }
        });

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&result).unwrap()
                }]
            })),
            error: None,
        }
    }

    /// Handle typecheck_wfl tool - type checking
    fn handle_typecheck_wfl(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let arguments = match params.get("arguments") {
            Some(args) => args,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing 'arguments' in tool call".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let source = match arguments.get("source").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing or invalid 'source' parameter".to_string(),
                        data: None,
                    }),
                };
            }
        };

        // Parse and type check
        let tokens = lex_wfl_with_positions(source);
        let mut parser = Parser::new(&tokens);

        match parser.parse() {
            Ok(program) => {
                let mut type_checker = TypeChecker::new();
                match type_checker.check_types(&program) {
                    Ok(_) => {
                        let result = json!({
                            "success": true,
                            "message": "Type checking passed - no type errors found",
                            "type_errors": []
                        });

                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: Some(json!({
                                "content": [{
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&result).unwrap()
                                }]
                            })),
                            error: None,
                        }
                    }
                    Err(errors) => {
                        let error_messages: Vec<String> =
                            errors.iter().map(|e| format!("{:?}", e)).collect();

                        let result = json!({
                            "success": false,
                            "message": format!("Found {} type error(s)", errors.len()),
                            "type_errors": error_messages
                        });

                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: Some(json!({
                                "content": [{
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&result).unwrap()
                                }],
                                "isError": true
                            })),
                            error: None,
                        }
                    }
                }
            }
            Err(parse_errors) => {
                let error_messages: Vec<String> =
                    parse_errors.iter().map(|e| format!("{:?}", e)).collect();

                let result = json!({
                    "success": false,
                    "message": "Cannot type check - parse errors present",
                    "parse_errors": error_messages
                });

                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&result).unwrap()
                        }],
                        "isError": true
                    })),
                    error: None,
                }
            }
        }
    }

    /// Handle lint_wfl tool - linting (uses analyzer for now)
    fn handle_lint_wfl(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let arguments = match params.get("arguments") {
            Some(args) => args,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing 'arguments' in tool call".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let source = match arguments.get("source").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing or invalid 'source' parameter".to_string(),
                        data: None,
                    }),
                };
            }
        };

        // For now, use analyze_document which includes semantic and type checking
        let diagnostics = self.core.analyze_document(source);

        // Filter to warnings and suggestions (linting)
        let lint_issues: Vec<_> = diagnostics
            .iter()
            .filter(|d| {
                matches!(
                    d.severity,
                    Some(tower_lsp::lsp_types::DiagnosticSeverity::WARNING)
                        | Some(tower_lsp::lsp_types::DiagnosticSeverity::INFORMATION)
                        | Some(tower_lsp::lsp_types::DiagnosticSeverity::HINT)
                )
            })
            .collect();

        let result = json!({
            "success": true,
            "lint_issue_count": lint_issues.len(),
            "lint_issues": lint_issues.iter().map(|d| {
                json!({
                    "message": d.message,
                    "severity": format!("{:?}", d.severity),
                    "category": "style"
                })
            }).collect::<Vec<_>>(),
            "message": if lint_issues.is_empty() {
                "No linting issues found".to_string()
            } else {
                format!("Found {} linting issue(s)", lint_issues.len())
            }
        });

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&result).unwrap()
                }]
            })),
            error: None,
        }
    }

    /// Handle get_completions tool
    fn handle_get_completions(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let arguments = match params.get("arguments") {
            Some(args) => args,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing 'arguments' in tool call".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let source = match arguments.get("source").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing or invalid 'source' parameter".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let line = arguments.get("line").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let column = arguments.get("column").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        // Basic keyword completions (can be enhanced)
        let keywords = vec![
            "store", "create", "display", "check if", "count from", "for each",
            "define action", "give back", "try", "when", "otherwise", "repeat while",
            "repeat until", "open file", "and", "or", "not", "is", "greater", "less",
            "than", "equal", "to", "as", "called", "with", "in", "end",
        ];

        let completions: Vec<_> = keywords
            .iter()
            .map(|kw| {
                json!({
                    "label": kw,
                    "kind": "Keyword",
                    "detail": format!("WFL keyword: {}", kw)
                })
            })
            .collect();

        let result = json!({
            "success": true,
            "position": {"line": line, "column": column},
            "completion_count": completions.len(),
            "completions": completions,
            "message": format!("Found {} completion(s) at line {}, column {}", completions.len(), line, column)
        });

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&result).unwrap()
                }]
            })),
            error: None,
        }
    }

    /// Handle get_symbol_info tool
    fn handle_get_symbol_info(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let arguments = match params.get("arguments") {
            Some(args) => args,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing 'arguments' in tool call".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let source = match arguments.get("source").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing or invalid 'source' parameter".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let line = arguments.get("line").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let column = arguments.get("column").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        // Parse the code to extract symbols
        let tokens = lex_wfl_with_positions(source);
        let mut parser = Parser::new(&tokens);

        match parser.parse() {
            Ok(program) => {
                // For now, return basic info about the program structure
                let symbol_count = program.statements.len();

                let result = json!({
                    "success": true,
                    "position": {"line": line, "column": column},
                    "symbol_info": {
                        "type": "Program",
                        "statement_count": symbol_count,
                        "description": format!("WFL program with {} statement(s)", symbol_count)
                    },
                    "message": format!("Symbol info at line {}, column {}", line, column)
                });

                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&result).unwrap()
                        }]
                    })),
                    error: None,
                }
            }
            Err(_) => {
                let result = json!({
                    "success": false,
                    "message": "Cannot get symbol info - parse errors present"
                });

                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&result).unwrap()
                        }],
                        "isError": true
                    })),
                    error: None,
                }
            }
        }
    }

    /// Handle resources/list request
    fn handle_resources_list(&self, id: Option<Value>) -> JsonRpcResponse {
        let mut resources = vec![
            json!({
                "uri": "workspace://files",
                "name": "WFL Files",
                "description": "List all WFL files in the workspace",
                "mimeType": "application/json"
            }),
            json!({
                "uri": "workspace://symbols",
                "name": "Workspace Symbols",
                "description": "Get all symbols across the workspace",
                "mimeType": "application/json"
            }),
            json!({
                "uri": "workspace://diagnostics",
                "name": "Workspace Diagnostics",
                "description": "Get all diagnostics across the workspace",
                "mimeType": "application/json"
            }),
        ];

        // Add workspace config if workspace is available
        if self.workspace_root.is_some() {
            resources.push(json!({
                "uri": "workspace://config",
                "name": "WFL Configuration",
                "description": "Get WFL workspace configuration (.wflcfg)",
                "mimeType": "application/json"
            }));
        }

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "resources": resources
            })),
            error: None,
        }
    }

    /// Handle resources/read request
    fn handle_resources_read(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let uri = match params.get("uri").and_then(|v| v.as_str()) {
            Some(u) => u,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32602,
                        message: "Missing 'uri' parameter".to_string(),
                        data: None,
                    }),
                };
            }
        };

        // Route to appropriate handler based on URI
        if uri == "workspace://files" {
            self.handle_workspace_files(id)
        } else if uri == "workspace://symbols" {
            self.handle_workspace_symbols(id)
        } else if uri == "workspace://config" {
            self.handle_workspace_config(id)
        } else if uri == "workspace://diagnostics" {
            self.handle_workspace_diagnostics(id)
        } else if uri.starts_with("file:///") {
            self.handle_file_resource(id, uri)
        } else {
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32602,
                    message: format!("Unknown resource URI: {}", uri),
                    data: None,
                }),
            }
        }
    }

    /// Handle workspace://files resource
    fn handle_workspace_files(&self, id: Option<Value>) -> JsonRpcResponse {
        let workspace_root = match &self.workspace_root {
            Some(path) => path,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: "No workspace root configured".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let mut wfl_files = Vec::new();
        if let Ok(entries) = fs::read_dir(workspace_root) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        if let Some(path) = entry.path().to_str() {
                            if path.ends_with(".wfl") {
                                let file_name = entry.file_name();
                                wfl_files.push(json!({
                                    "uri": format!("file:///{}", path.replace("\\", "/")),
                                    "name": file_name.to_string_lossy(),
                                    "mimeType": "text/x-wfl"
                                }));
                            }
                        }
                    }
                }
            }
        }

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "contents": [{
                    "uri": "workspace://files",
                    "mimeType": "application/json",
                    "text": serde_json::to_string_pretty(&json!({
                        "files": wfl_files,
                        "count": wfl_files.len()
                    })).unwrap()
                }]
            })),
            error: None,
        }
    }

    /// Handle file:///{path} resource
    fn handle_file_resource(&self, id: Option<Value>, uri: &str) -> JsonRpcResponse {
        // Extract path from file:/// URI
        let path_str = uri.strip_prefix("file:///").unwrap_or(uri);
        let path = Path::new(path_str);

        match fs::read_to_string(path) {
            Ok(content) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "text/x-wfl",
                        "text": content
                    }]
                })),
                error: None,
            },
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32603,
                    message: format!("Failed to read file: {}", e),
                    data: None,
                }),
            },
        }
    }

    /// Handle workspace://symbols resource
    fn handle_workspace_symbols(&self, id: Option<Value>) -> JsonRpcResponse {
        let workspace_root = match &self.workspace_root {
            Some(path) => path,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: "No workspace root configured".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let mut all_symbols = Vec::new();

        // Scan for .wfl files and extract symbols
        if let Ok(entries) = fs::read_dir(workspace_root) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("wfl") {
                            if let Ok(content) = fs::read_to_string(&path) {
                                let tokens = lex_wfl_with_positions(&content);
                                let mut parser = Parser::new(&tokens);
                                if let Ok(program) = parser.parse() {
                                    all_symbols.push(json!({
                                        "file": path.to_string_lossy(),
                                        "statement_count": program.statements.len()
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "contents": [{
                    "uri": "workspace://symbols",
                    "mimeType": "application/json",
                    "text": serde_json::to_string_pretty(&json!({
                        "symbols": all_symbols,
                        "file_count": all_symbols.len()
                    })).unwrap()
                }]
            })),
            error: None,
        }
    }

    /// Handle workspace://config resource
    fn handle_workspace_config(&self, id: Option<Value>) -> JsonRpcResponse {
        let workspace_root = match &self.workspace_root {
            Some(path) => path,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: "No workspace root configured".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let config_path = workspace_root.join(".wflcfg");
        let config_content = if config_path.exists() {
            fs::read_to_string(&config_path).unwrap_or_else(|_| "{}".to_string())
        } else {
            json!({
                "message": "No .wflcfg file found in workspace",
                "using_defaults": true
            })
            .to_string()
        };

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "contents": [{
                    "uri": "workspace://config",
                    "mimeType": "application/json",
                    "text": config_content
                }]
            })),
            error: None,
        }
    }

    /// Handle workspace://diagnostics resource
    fn handle_workspace_diagnostics(&self, id: Option<Value>) -> JsonRpcResponse {
        let workspace_root = match &self.workspace_root {
            Some(path) => path,
            None => {
                return JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: "No workspace root configured".to_string(),
                        data: None,
                    }),
                };
            }
        };

        let mut all_diagnostics = Vec::new();

        // Scan for .wfl files and collect diagnostics
        if let Ok(entries) = fs::read_dir(workspace_root) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("wfl") {
                            if let Ok(content) = fs::read_to_string(&path) {
                                let diagnostics = self.core.analyze_document(&content);
                                if !diagnostics.is_empty() {
                                    all_diagnostics.push(json!({
                                        "file": path.to_string_lossy(),
                                        "diagnostic_count": diagnostics.len(),
                                        "diagnostics": diagnostics.iter().map(|d| {
                                            json!({
                                                "message": d.message,
                                                "severity": format!("{:?}", d.severity)
                                            })
                                        }).collect::<Vec<_>>()
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(json!({
                "contents": [{
                    "uri": "workspace://diagnostics",
                    "mimeType": "application/json",
                    "text": serde_json::to_string_pretty(&json!({
                        "files_with_issues": all_diagnostics,
                        "total_files_with_issues": all_diagnostics.len()
                    })).unwrap()
                }]
            })),
            error: None,
        }
    }

    /// Process a single JSON-RPC request
    fn process_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(request.id),
            "tools/list" => self.handle_tools_list(request.id),
            "resources/list" => self.handle_resources_list(request.id),
            "tools/call" => {
                if let Some(params) = request.params {
                    self.handle_tools_call(request.id, params)
                } else {
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: "Missing params for tools/call".to_string(),
                            data: None,
                        }),
                    }
                }
            }
            "resources/read" => {
                if let Some(params) = request.params {
                    self.handle_resources_read(request.id, params)
                } else {
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: request.id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32602,
                            message: "Missing params for resources/read".to_string(),
                            data: None,
                        }),
                    }
                }
            }
            _ => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                }),
            },
        }
    }
}

impl Default for WflMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Run the MCP server on stdin/stdout
pub async fn run_server() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("[MCP] WFL MCP Server starting...");
    eprintln!("[MCP] Version: {}", env!("CARGO_PKG_VERSION"));
    eprintln!("[MCP] Protocol: JSON-RPC 2.0");
    eprintln!("[MCP] Capabilities: Tools (parse_wfl)");
    eprintln!("[MCP] Listening on stdin/stdout...");

    let server = WflMcpServer::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    // Read JSON-RPC messages from stdin, one per line
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[MCP] Error reading stdin: {}", e);
                continue;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        eprintln!("[MCP] Received request: {}", line);

        // Parse JSON-RPC request
        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                eprintln!("[MCP] Error parsing JSON-RPC request: {}", e);
                let error_response = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: "Parse error".to_string(),
                        data: Some(json!({ "details": e.to_string() })),
                    }),
                };
                let response_json = serde_json::to_string(&error_response)?;
                writeln!(stdout, "{}", response_json)?;
                stdout.flush()?;
                continue;
            }
        };

        // Process request and send response
        let response = server.process_request(request);
        let response_json = serde_json::to_string(&response)?;

        eprintln!("[MCP] Sending response: {}", response_json);
        writeln!(stdout, "{}", response_json)?;
        stdout.flush()?;
    }

    eprintln!("[MCP] Server shutting down");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = WflMcpServer::new();
        assert!(Arc::strong_count(&server.core) > 0);
    }

    #[test]
    fn test_handle_initialize() {
        let server = WflMcpServer::new();
        let response = server.handle_initialize(Some(json!(1)));

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_some());
        assert!(response.error.is_none());

        let result = response.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert!(result["capabilities"]["tools"].is_object());
    }

    #[test]
    fn test_handle_tools_list() {
        let server = WflMcpServer::new();
        let response = server.handle_tools_list(Some(json!(2)));

        assert!(response.result.is_some());
        let result = response.result.unwrap();
        let tools = result["tools"].as_array().unwrap();

        assert_eq!(tools.len(), 6);
        assert_eq!(tools[0]["name"], "parse_wfl");
        assert_eq!(tools[1]["name"], "analyze_wfl");
        assert_eq!(tools[2]["name"], "typecheck_wfl");
        assert_eq!(tools[3]["name"], "lint_wfl");
        assert_eq!(tools[4]["name"], "get_completions");
        assert_eq!(tools[5]["name"], "get_symbol_info");
    }

    #[test]
    fn test_parse_wfl_valid_code() {
        let server = WflMcpServer::new();
        let params = json!({
            "name": "parse_wfl",
            "arguments": {
                "source": "store x as 5",
                "include_positions": true
            }
        });

        let response = server.handle_parse_wfl(Some(json!(3)), params);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_parse_wfl_invalid_code() {
        let server = WflMcpServer::new();
        let params = json!({
            "name": "parse_wfl",
            "arguments": {
                "source": "store x as",
                "include_positions": true
            }
        });

        let response = server.handle_parse_wfl(Some(json!(4)), params);
        assert!(response.result.is_some());

        let result = response.result.unwrap();
        assert_eq!(result["isError"], true);
    }
}
