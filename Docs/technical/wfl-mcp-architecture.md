# WFL MCP Architecture Documentation

Technical documentation for the WFL Model Context Protocol (MCP) server implementation.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Implementation Details](#implementation-details)
- [Design Decisions](#design-decisions)
- [Code Structure](#code-structure)
- [Extension Guide](#extension-guide)

## Overview

The WFL Language Server (`wfl-lsp`) implements the Model Context Protocol (MCP) using a manual JSON-RPC 2.0 approach, providing AI assistants with comprehensive access to WFL language features.

### Key Features

- **Dual-mode operation**: LSP and MCP from single binary
- **Manual JSON-RPC**: No complex SDK dependencies
- **Shared core**: LSP and MCP use common analysis pipeline
- **Full MCP support**: Tools, Resources, and future Prompts
- **Backward compatible**: Zero breaking changes to LSP

## Architecture

### High-Level Design

```
┌──────────────────────────────────────────────────────────────┐
│                      wfl-lsp Binary                          │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌────────────┐                    ┌────────────┐           │
│  │  main.rs   │                    │  main.rs   │           │
│  │            │                    │            │           │
│  │ (default)  │                    │  (--mcp)   │           │
│  └─────┬──────┘                    └─────┬──────┘           │
│        │                                 │                  │
│        ▼                                 ▼                  │
│  ┌────────────┐                    ┌────────────┐           │
│  │ LSP Server │                    │ MCP Server │           │
│  │ (tower-lsp)│                    │(JSON-RPC)  │           │
│  └─────┬──────┘                    └─────┬──────┘           │
│        │                                 │                  │
│        └────────────┬────────────────────┘                  │
│                     ▼                                       │
│         ┌──────────────────────┐                            │
│         │  WflLanguageCore     │                            │
│         │  (Shared Foundation) │                            │
│         └──────────┬───────────┘                            │
│                    │                                        │
│                    ▼                                        │
│         ┌──────────────────────┐                            │
│         │  WFL Compiler        │                            │
│         │  Lexer → Parser →    │                            │
│         │  Analyzer → TypeChecker                           │
│         └──────────────────────┘                            │
└──────────────────────────────────────────────────────────────┘
```

### Component Breakdown

#### 1. Main Entry Point (`src/main.rs`)

```rust
#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("--mcp") => run_mcp_server().await,
        _ => run_lsp_server().await,  // Default
    }
}
```

**Responsibilities:**
- Command-line argument parsing
- Mode selection (LSP vs MCP)
- Server initialization

#### 2. Shared Core (`src/core.rs`)

```rust
pub struct WflLanguageCore {
    documents: Arc<DashMap<String, DocumentState>>,
    workspace_path: Option<PathBuf>,
}
```

**Responsibilities:**
- Document management (thread-safe)
- WFL compiler integration
- Diagnostic conversion
- Analysis pipeline execution

**Key Methods:**
- `analyze_document()`: Complete analysis (parse + semantic + type check)
- `analyze_source()`: Raw analysis with custom reporter
- `convert_to_lsp_diagnostic()`: WFL → LSP diagnostic conversion

#### 3. MCP Server (`src/mcp_server.rs`)

```rust
pub struct WflMcpServer {
    core: Arc<WflLanguageCore>,
    workspace_root: Option<PathBuf>,
}
```

**Responsibilities:**
- JSON-RPC 2.0 message handling
- Tool execution
- Resource management
- Workspace operations

## Implementation Details

### JSON-RPC 2.0 Protocol

The MCP server implements JSON-RPC 2.0 manually without external SDKs:

```rust
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}
```

**Message Flow:**

1. Read line from stdin
2. Deserialize to `JsonRpcRequest`
3. Route to appropriate handler based on `method`
4. Execute handler
5. Serialize `JsonRpcResponse`
6. Write to stdout

### Tool Implementation Pattern

Each tool follows a consistent pattern:

```rust
fn handle_tool_name(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
    // 1. Extract and validate arguments
    let arguments = params.get("arguments")?;
    let source = arguments.get("source")?.as_str()?;

    // 2. Execute WFL compiler operations
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse()?;

    // 3. Format results
    let result = json!({
        "success": true,
        "data": "..."
    });

    // 4. Return JSON-RPC response
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&result)?
            }]
        })),
        error: None,
    }
}
```

### Resource Implementation Pattern

Resources provide workspace-level data:

```rust
fn handle_workspace_resource(&self, id: Option<Value>) -> JsonRpcResponse {
    // 1. Verify workspace root exists
    let workspace_root = self.workspace_root.as_ref()?;

    // 2. Scan workspace
    let mut data = Vec::new();
    for entry in fs::read_dir(workspace_root)? {
        if entry.path().extension() == Some("wfl") {
            // Collect data
        }
    }

    // 3. Format and return
    JsonRpcResponse {
        result: Some(json!({
            "contents": [{
                "uri": "workspace://resource",
                "mimeType": "application/json",
                "text": serde_json::to_string_pretty(&data)?
            }]
        })),
        error: None,
    }
}
```

## Design Decisions

### 1. Manual JSON-RPC vs SDK

**Decision:** Implement JSON-RPC 2.0 manually

**Rationale:**
- **Simplicity**: <100 lines of protocol code
- **Control**: Full understanding of message flow
- **Stability**: No dependency on evolving SDK APIs
- **Maintainability**: Easy to debug and modify

**Trade-offs:**
- ✅ No external dependencies
- ✅ Clear, readable code
- ✅ Easy to extend
- ❌ Must manually handle protocol details
- ❌ No automatic validation

### 2. Shared Core Architecture

**Decision:** Extract common logic to `WflLanguageCore`

**Rationale:**
- **DRY**: Single source of truth for analysis
- **Consistency**: LSP and MCP produce identical results
- **Maintainability**: Update once, applies everywhere
- **Performance**: Shared document cache

**Implementation:**
```rust
// LSP uses core
impl WflLanguageServer {
    fn analyze_document(&self, text: &str) -> Vec<Diagnostic> {
        self.core.analyze_document(text)
    }
}

// MCP uses same core
impl WflMcpServer {
    fn handle_analyze_wfl(&self, ...) -> JsonRpcResponse {
        let diagnostics = self.core.analyze_document(source);
        // Format for MCP
    }
}
```

### 3. Tool vs Resource Design

**Tools**: Active operations on code
- Parse, analyze, type check
- Take code as input
- Return computed results

**Resources**: Passive data access
- File listings, configuration
- Read-only workspace data
- Return existing information

This separation aligns with MCP specification and user expectations.

### 4. Synchronous Processing

**Decision:** Process requests synchronously (no async tools)

**Rationale:**
- **Simplicity**: Easier to reason about
- **WFL Compiler**: Synchronous by design
- **Performance**: Analysis is fast (<100ms typically)
- **Reliability**: No concurrency bugs

**Future:** Could add async for long-running operations if needed.

### 5. Document Management

**Decision:** Thread-safe `Arc<DashMap>` for document storage

**Rationale:**
- **Thread Safety**: Multiple protocol handlers can access
- **Performance**: Lock-free reads in most cases
- **LSP Compatibility**: Already used in LSP server

```rust
documents: Arc<DashMap<String, DocumentState>>
```

## Code Structure

### File Organization

```
wfl-lsp/
├── src/
│   ├── main.rs          # Entry point, mode selection (50 lines)
│   ├── core.rs          # Shared core (300 lines)
│   ├── mcp_server.rs    # MCP implementation (1,200 lines)
│   └── lib.rs           # LSP server (1,100 lines)
├── tests/
│   └── mcp_*.rs         # MCP integration tests
└── Cargo.toml           # Dependencies
```

### Dependencies

```toml
[dependencies]
# LSP
tower-lsp = "0.20.0"
tokio = { version = "1.35.1", features = ["full"] }

# Shared
dashmap = "5.5.3"
serde_json = "1.0.114"
serde = { version = "1.0", features = ["derive"] }

# MCP (none! Manual implementation)
```

### Key Types

```rust
// MCP Server
pub struct WflMcpServer {
    core: Arc<WflLanguageCore>,
    workspace_root: Option<PathBuf>,
}

// Shared Core
pub struct WflLanguageCore {
    documents: Arc<DashMap<String, DocumentState>>,
    workspace_path: Option<PathBuf>,
}

// Document State
pub struct DocumentState {
    uri: String,
    text: String,
    version: i32,
    diagnostics: Vec<WflDiagnostic>,
    last_analysis: Option<AnalysisResult>,
}
```

## Extension Guide

### Adding a New Tool

1. **Define the tool in `tools/list`:**

```rust
fn handle_tools_list(&self, id: Option<Value>) -> JsonRpcResponse {
    // Add to tools array
    {
        "name": "new_tool",
        "description": "What it does",
        "inputSchema": {
            "type": "object",
            "properties": {
                "param": {"type": "string"}
            },
            "required": ["param"]
        }
    }
}
```

2. **Implement the handler:**

```rust
fn handle_new_tool(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
    // Extract arguments
    let arguments = params.get("arguments")?;
    let param = arguments.get("param")?.as_str()?;

    // Do the work
    let result = do_something(param);

    // Return response
    JsonRpcResponse {
        jsonrpc: "2.0".to_string(),
        id,
        result: Some(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&result)?
            }]
        })),
        error: None,
    }
}
```

3. **Add to router:**

```rust
fn handle_tools_call(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
    match tool_name {
        "parse_wfl" => self.handle_parse_wfl(id, params),
        "new_tool" => self.handle_new_tool(id, params),  // Add here
        _ => error_response("Unknown tool")
    }
}
```

4. **Write tests:**

```rust
#[tokio::test]
async fn test_new_tool() {
    let server = WflMcpServer::new();
    let result = server.handle_new_tool(
        Some(json!(1)),
        json!({"arguments": {"param": "test"}})
    );
    assert!(result.result.is_some());
}
```

### Adding a New Resource

1. **Add to `resources/list`:**

```rust
{
    "uri": "workspace://newresource",
    "name": "New Resource",
    "description": "What it provides",
    "mimeType": "application/json"
}
```

2. **Implement handler:**

```rust
fn handle_new_resource(&self, id: Option<Value>) -> JsonRpcResponse {
    let workspace_root = self.workspace_root.as_ref()?;

    // Gather data
    let data = collect_data(workspace_root);

    // Return resource
    JsonRpcResponse {
        result: Some(json!({
            "contents": [{
                "uri": "workspace://newresource",
                "mimeType": "application/json",
                "text": serde_json::to_string_pretty(&data)?
            }]
        })),
        error: None,
    }
}
```

3. **Add to router:**

```rust
fn handle_resources_read(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
    match uri {
        "workspace://files" => self.handle_workspace_files(id),
        "workspace://newresource" => self.handle_new_resource(id),  // Add
        _ => error_response("Unknown resource")
    }
}
```

### Extending WflLanguageCore

To add new shared functionality:

```rust
impl WflLanguageCore {
    pub fn new_analysis_method(&self, source: &str) -> Result<Data, Error> {
        // 1. Lex and parse
        let tokens = lex_wfl_with_positions(source);
        let mut parser = Parser::new(&tokens);
        let program = parser.parse()?;

        // 2. Perform analysis
        let result = custom_analysis(&program);

        // 3. Return results
        Ok(result)
    }
}
```

Both LSP and MCP can now use this method.

## Performance Considerations

### Parsing Performance

- **Typical**: <10ms for small files (<100 lines)
- **Large files**: ~100ms for files with 1000+ lines
- **Optimization**: Consider caching parsed ASTs

### Resource Scanning

- **workspace://files**: O(n) where n = files in directory
- **workspace://symbols**: O(n*m) where m = parse time per file
- **Optimization**: Consider incremental updates

### Memory Usage

- **Document Map**: ~1KB per document in cache
- **AST**: ~10KB per parsed program (not cached)
- **Typical**: <10MB for workspace with 100 files

## Testing Strategy

### Unit Tests

Test individual handlers:

```rust
#[test]
fn test_parse_wfl_valid() {
    let server = WflMcpServer::new();
    let result = server.handle_parse_wfl(...);
    assert!(result.result.is_some());
}
```

### Integration Tests

Test complete JSON-RPC flow:

```rust
#[tokio::test]
async fn test_complete_workflow() {
    // 1. Spawn server
    let server = spawn_mcp_server();

    // 2. Send initialize
    let init_response = server.send(initialize_request()).await;
    assert_eq!(init_response.protocol_version, "2024-11-05");

    // 3. Call tool
    let tool_response = server.send(parse_request()).await;
    assert!(tool_response.success);
}
```

### Manual Testing

Use command-line for quick verification:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | wfl-lsp --mcp
```

## Future Enhancements

### Planned Features

1. **MCP Prompts**: Code templates and snippets
2. **Recursive Workspace Scan**: Support subdirectories
3. **Resource Subscriptions**: Real-time updates
4. **Streaming Responses**: For large results
5. **Enhanced Symbol Info**: Full symbol tables

### Optimization Opportunities

1. **AST Caching**: Cache parsed programs
2. **Incremental Parsing**: Re-parse only changed portions
3. **Parallel Resource Scanning**: Use rayon for workspace ops
4. **Lazy Resource Loading**: Load resources on-demand

## Troubleshooting

### Common Issues

**Problem:** Tools return errors
**Solution:** Check argument validation, ensure required params provided

**Problem:** Resources return empty
**Solution:** Verify workspace_root is set correctly

**Problem:** Performance degradation
**Solution:** Profile with `cargo flamegraph`, check for N+1 queries

## See Also

- [WFL MCP User Guide](../guides/wfl-mcp-guide.md)
- [WFL MCP API Reference](../guides/wfl-mcp-api-reference.md)
- [MCP Specification](https://modelcontextprotocol.io/specification)
- [JSON-RPC 2.0 Spec](https://www.jsonrpc.org/specification)

---

**Version:** WFL LSP v0.1.0
**Last Updated:** January 2026
**Maintainer:** WFL Team
