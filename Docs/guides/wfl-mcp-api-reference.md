# WFL MCP API Reference

Complete API reference for the WFL Model Context Protocol (MCP) server.

## Table of Contents

- [Protocol Information](#protocol-information)
- [Connection](#connection)
- [Tools API](#tools-api)
- [Resources API](#resources-api)
- [Error Codes](#error-codes)
- [Examples](#examples)

## Protocol Information

- **Protocol:** JSON-RPC 2.0
- **Transport:** stdio (stdin/stdout)
- **MCP Version:** 2024-11-05
- **Server Version:** wfl-lsp v0.1.0

## Connection

### Initialize

Establish connection and retrieve server capabilities.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {}
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "tools": {},
      "resources": {}
    },
    "serverInfo": {
      "name": "wfl-lsp",
      "version": "0.1.0"
    }
  }
}
```

---

## Tools API

### List Tools

Get all available tools.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list"
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
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
      }
      // ... 5 more tools
    ]
  }
}
```

### Tool: parse_wfl

Parse WFL source code and return the Abstract Syntax Tree.

**Parameters:**

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `source` | string | Yes | - | WFL source code to parse |
| `include_positions` | boolean | No | true | Include position information in AST |

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "parse_wfl",
    "arguments": {
      "source": "store x as 5\ndisplay x",
      "include_positions": true
    }
  }
}
```

**Success Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"success\": true,\n  \"statement_count\": 2,\n  \"ast\": \"Program {...}\",\n  \"message\": \"Successfully parsed 2 statement(s)\"\n}"
      }
    ]
  }
}
```

**Error Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"success\": false,\n  \"errors\": [...],\n  \"error_count\": 1,\n  \"message\": \"Parse failed with 1 error(s)\"\n}"
      }
    ],
    "isError": true
  }
}
```

### Tool: analyze_wfl

Run semantic analysis and return diagnostics.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `source` | string | Yes | WFL source code to analyze |

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "analyze_wfl",
    "arguments": {
      "source": "store x as 5\ndisplay y"
    }
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"success\": true,\n  \"diagnostic_count\": 1,\n  \"diagnostics\": [\n    {\n      \"message\": \"Undefined variable 'y'\",\n      \"severity\": \"Some(Error)\",\n      \"range\": {\n        \"start\": {\"line\": 1, \"character\": 8},\n        \"end\": {\"line\": 1, \"character\": 9}\n      }\n    }\n  ],\n  \"message\": \"Found 1 diagnostic(s)\"\n}"
      }
    ]
  }
}
```

### Tool: typecheck_wfl

Run type checker and return type errors.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `source` | string | Yes | WFL source code to type check |

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "tools/call",
  "params": {
    "name": "typecheck_wfl",
    "arguments": {
      "source": "store x as 5\nstore y as x + \"text\""
    }
  }
}
```

**Success Response (No Errors):**
```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"success\": true,\n  \"message\": \"Type checking passed - no type errors found\",\n  \"type_errors\": []\n}"
      }
    ]
  }
}
```

**Error Response (Type Errors):**
```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"success\": false,\n  \"message\": \"Found 1 type error(s)\",\n  \"type_errors\": [\"TypeError {...}\"]\n}"
      }
    ],
    "isError": true
  }
}
```

### Tool: lint_wfl

Lint WFL code and suggest improvements.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `source` | string | Yes | WFL source code to lint |

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "tools/call",
  "params": {
    "name": "lint_wfl",
    "arguments": {
      "source": "store x as 5"
    }
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"success\": true,\n  \"lint_issue_count\": 0,\n  \"lint_issues\": [],\n  \"message\": \"No linting issues found\"\n}"
      }
    ]
  }
}
```

### Tool: get_completions

Get code completion suggestions at a position.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `source` | string | Yes | WFL source code |
| `line` | number | Yes | Line number (0-based) |
| `column` | number | Yes | Column number (0-based) |

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "tools/call",
  "params": {
    "name": "get_completions",
    "arguments": {
      "source": "store ",
      "line": 0,
      "column": 6
    }
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"success\": true,\n  \"position\": {\"line\": 0, \"column\": 6},\n  \"completion_count\": 28,\n  \"completions\": [\n    {\n      \"label\": \"store\",\n      \"kind\": \"Keyword\",\n      \"detail\": \"WFL keyword: store\"\n    },\n    ...\n  ],\n  \"message\": \"Found 28 completion(s) at line 0, column 6\"\n}"
      }
    ]
  }
}
```

### Tool: get_symbol_info

Get information about a symbol at a position.

**Parameters:**

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `source` | string | Yes | WFL source code |
| `line` | number | Yes | Line number (0-based) |
| `column` | number | Yes | Column number (0-based) |

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "method": "tools/call",
  "params": {
    "name": "get_symbol_info",
    "arguments": {
      "source": "store x as 5",
      "line": 0,
      "column": 7
    }
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"success\": true,\n  \"position\": {\"line\": 0, \"column\": 7},\n  \"symbol_info\": {\n    \"type\": \"Program\",\n    \"statement_count\": 1,\n    \"description\": \"WFL program with 1 statement(s)\"\n  },\n  \"message\": \"Symbol info at line 0, column 7\"\n}"
      }
    ]
  }
}
```

---

## Resources API

### List Resources

Get all available resources.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 9,
  "method": "resources/list"
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 9,
  "result": {
    "resources": [
      {
        "uri": "workspace://files",
        "name": "WFL Files",
        "description": "List all WFL files in the workspace",
        "mimeType": "application/json"
      },
      {
        "uri": "workspace://symbols",
        "name": "Workspace Symbols",
        "description": "Get all symbols across the workspace",
        "mimeType": "application/json"
      },
      {
        "uri": "workspace://diagnostics",
        "name": "Workspace Diagnostics",
        "description": "Get all diagnostics across the workspace",
        "mimeType": "application/json"
      },
      {
        "uri": "workspace://config",
        "name": "WFL Configuration",
        "description": "Get WFL workspace configuration (.wflcfg)",
        "mimeType": "application/json"
      }
    ]
  }
}
```

### Read Resource

Read a specific resource by URI.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "method": "resources/read",
  "params": {
    "uri": "workspace://files"
  }
}
```

### Resource: workspace://files

List all WFL files in the workspace.

**URI:** `workspace://files`

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "result": {
    "contents": [
      {
        "uri": "workspace://files",
        "mimeType": "application/json",
        "text": "{\n  \"files\": [\n    {\n      \"uri\": \"file:///path/to/file.wfl\",\n      \"name\": \"file.wfl\",\n      \"mimeType\": \"text/x-wfl\"\n    }\n  ],\n  \"count\": 1\n}"
      }
    ]
  }
}
```

### Resource: file:///{path}

Read contents of a specific WFL file.

**URI:** `file:///{absolute_path}`

**Example:** `file:///G:/Projects/myapp/main.wfl`

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 11,
  "result": {
    "contents": [
      {
        "uri": "file:///G:/Projects/myapp/main.wfl",
        "mimeType": "text/x-wfl",
        "text": "store x as 5\ndisplay x"
      }
    ]
  }
}
```

### Resource: workspace://symbols

Get all symbols across the workspace.

**URI:** `workspace://symbols`

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 12,
  "result": {
    "contents": [
      {
        "uri": "workspace://symbols",
        "mimeType": "application/json",
        "text": "{\n  \"symbols\": [\n    {\n      \"file\": \"/path/to/file.wfl\",\n      \"statement_count\": 10\n    }\n  ],\n  \"file_count\": 1\n}"
      }
    ]
  }
}
```

### Resource: workspace://config

Read the WFL workspace configuration.

**URI:** `workspace://config`

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 13,
  "result": {
    "contents": [
      {
        "uri": "workspace://config",
        "mimeType": "application/json",
        "text": "timeout_seconds = 60\nlogging_enabled = false\n..."
      }
    ]
  }
}
```

### Resource: workspace://diagnostics

Get all diagnostics across the workspace.

**URI:** `workspace://diagnostics`

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 14,
  "result": {
    "contents": [
      {
        "uri": "workspace://diagnostics",
        "mimeType": "application/json",
        "text": "{\n  \"files_with_issues\": [\n    {\n      \"file\": \"/path/to/file.wfl\",\n      \"diagnostic_count\": 2,\n      \"diagnostics\": [\n        {\n          \"message\": \"Undefined variable 'x'\",\n          \"severity\": \"Error\"\n        }\n      ]\n    }\n  ],\n  \"total_files_with_issues\": 1\n}"
      }
    ]
  }
}
```

---

## Error Codes

Standard JSON-RPC 2.0 error codes:

| Code | Message | Description |
|------|---------|-------------|
| -32700 | Parse error | Invalid JSON received |
| -32600 | Invalid Request | JSON-RPC request is invalid |
| -32601 | Method not found | Method does not exist |
| -32602 | Invalid params | Invalid method parameters |
| -32603 | Internal error | Internal server error |

**Example Error Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 15,
  "error": {
    "code": -32602,
    "message": "Missing 'source' parameter",
    "data": null
  }
}
```

---

## Examples

### Complete Workflow: Parse and Analyze

```json
// 1. Initialize
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {}
}

// 2. Parse code
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "parse_wfl",
    "arguments": {
      "source": "store x as 5\ndisplay x"
    }
  }
}

// 3. Analyze code
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "analyze_wfl",
    "arguments": {
      "source": "store x as 5\ndisplay x"
    }
  }
}

// 4. Type check
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "typecheck_wfl",
    "arguments": {
      "source": "store x as 5\ndisplay x"
    }
  }
}
```

### Workspace Exploration

```json
// 1. List all resources
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "resources/list"
}

// 2. Get all WFL files
{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "resources/read",
  "params": {
    "uri": "workspace://files"
  }
}

// 3. Get workspace diagnostics
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "resources/read",
  "params": {
    "uri": "workspace://diagnostics"
  }
}

// 4. Read specific file
{
  "jsonrpc": "2.0",
  "id": 8,
  "method": "resources/read",
  "params": {
    "uri": "file:///G:/Projects/myapp/main.wfl"
  }
}
```

---

## Notes

- All line and column numbers are **0-based**
- URIs use forward slashes (`/`) even on Windows
- File URIs must be absolute paths
- Resources require workspace root to be set (run from project directory)
- All responses include proper JSON-RPC 2.0 formatting

## See Also

- [WFL MCP User Guide](wfl-mcp-guide.md)
- [WFL MCP Architecture](../technical/wfl-mcp-architecture.md)
- [MCP Specification](https://modelcontextprotocol.io/specification)

---

**Version:** WFL LSP v0.1.0
**Protocol:** JSON-RPC 2.0
**MCP Version:** 2024-11-05
