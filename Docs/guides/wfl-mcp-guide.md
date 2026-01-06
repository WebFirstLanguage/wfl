# WFL Model Context Protocol (MCP) Guide

## Overview

The WFL Language Server (`wfl-lsp`) now supports the Model Context Protocol (MCP), enabling AI assistants like Claude to analyze, understand, and interact with WFL codebases. This guide covers everything you need to know to use WFL with AI-powered development tools.

## What is MCP?

The Model Context Protocol (MCP) is a standard protocol that allows AI assistants to access tools and resources from external applications. With MCP support, AI assistants can:

- Parse and analyze WFL code
- Type check and lint WFL programs
- Provide code completions and symbol information
- Explore entire WFL workspaces
- Read configuration files
- Identify issues across multiple files

## Getting Started

### Prerequisites

- WFL installed (`wfl-lsp` executable available)
- An MCP-compatible client (e.g., Claude Desktop, or custom integration)

### Running the MCP Server

The WFL LSP server can run in two modes:

**LSP Mode (default)** - For IDE integration:
```bash
wfl-lsp
```

**MCP Mode** - For AI assistant integration:
```bash
wfl-lsp --mcp
```

The MCP server communicates via JSON-RPC 2.0 over stdin/stdout.

## Using with Claude Desktop

Claude Desktop is the official MCP client from Anthropic. To integrate WFL with Claude Desktop:

### 1. Configure Claude Desktop

Add the following to your Claude Desktop MCP configuration file:

**On Windows:** `%APPDATA%\Claude\claude_desktop_config.json`
**On macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
**On Linux:** `~/.config/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "wfl": {
      "command": "wfl-lsp",
      "args": ["--mcp"],
      "cwd": "/path/to/your/wfl/project"
    }
  }
}
```

### 2. Restart Claude Desktop

After updating the configuration, restart Claude Desktop to load the WFL MCP server.

### 3. Verify Connection

You should see the WFL server listed in Claude Desktop's MCP servers panel. Claude can now:

- Analyze your WFL code
- Provide intelligent suggestions
- Find and fix errors
- Explain code functionality

## Available Tools

The WFL MCP server provides 6 powerful tools:

### 1. `parse_wfl`

Parse WFL source code and return the Abstract Syntax Tree (AST).

**Input:**
```json
{
  "source": "store x as 5\ndisplay x",
  "include_positions": true
}
```

**Use Cases:**
- Understanding code structure
- Validating syntax
- AST analysis

### 2. `analyze_wfl`

Run semantic analysis and return all diagnostics (errors, warnings).

**Input:**
```json
{
  "source": "store x as 5\ndisplay y"
}
```

**Returns:**
- Parse errors
- Semantic errors (undefined variables, etc.)
- Type errors
- Line/column positions

**Use Cases:**
- Finding bugs
- Code validation
- Error explanation

### 3. `typecheck_wfl`

Run the type checker and return type errors.

**Input:**
```json
{
  "source": "store x as 5\nstore y as x + \"text\""
}
```

**Use Cases:**
- Type safety verification
- Type error detection
- Type mismatch identification

### 4. `lint_wfl`

Lint WFL code and suggest improvements.

**Input:**
```json
{
  "source": "store unused_var as 5"
}
```

**Use Cases:**
- Code style checking
- Best practice enforcement
- Warning detection

### 5. `get_completions`

Get code completion suggestions at a specific position.

**Input:**
```json
{
  "source": "store ",
  "line": 0,
  "column": 6
}
```

**Returns:**
- WFL keyword completions
- Context-aware suggestions
- 28+ completion items

**Use Cases:**
- Code generation assistance
- Syntax guidance
- Learning WFL syntax

### 6. `get_symbol_info`

Get information about symbols at a specific position.

**Input:**
```json
{
  "source": "store x as 5",
  "line": 0,
  "column": 7
}
```

**Use Cases:**
- Understanding code elements
- Symbol information
- Code navigation

## Available Resources

The WFL MCP server provides 5 workspace-level resources:

### 1. `workspace://files`

List all WFL files in the workspace.

**Returns:**
```json
{
  "files": [
    {
      "uri": "file:///path/to/file.wfl",
      "name": "file.wfl",
      "mimeType": "text/x-wfl"
    }
  ],
  "count": 1
}
```

**Use Cases:**
- Workspace exploration
- Project structure understanding
- File discovery

### 2. `file:///{path}`

Read the contents of a specific WFL file.

**Example:** `file:///G:/Projects/myapp/main.wfl`

**Use Cases:**
- Reading source code
- Multi-file analysis
- Code review

### 3. `workspace://symbols`

Get all symbols across the workspace.

**Returns:**
```json
{
  "symbols": [
    {
      "file": "/path/to/file.wfl",
      "statement_count": 10
    }
  ],
  "file_count": 1
}
```

**Use Cases:**
- Project-wide symbol search
- Code navigation
- Understanding project structure

### 4. `workspace://config`

Read the WFL workspace configuration (`.wflcfg`).

**Returns:**
```
timeout_seconds = 60
logging_enabled = false
debug_report_enabled = true
log_level = info
```

**Use Cases:**
- Understanding project settings
- Configuration review
- Debugging configuration issues

### 5. `workspace://diagnostics`

Get all diagnostics across the entire workspace.

**Returns:**
```json
{
  "files_with_issues": [
    {
      "file": "/path/to/file.wfl",
      "diagnostic_count": 2,
      "diagnostics": [
        {
          "message": "Undefined variable 'x'",
          "severity": "Error"
        }
      ]
    }
  ],
  "total_files_with_issues": 1
}
```

**Use Cases:**
- Project health check
- Finding all errors at once
- Code quality assessment

## Example Workflows

### Workflow 1: Understanding a WFL Project

Ask Claude (with WFL MCP enabled):

> "What WFL files are in this workspace and what do they do?"

Claude will:
1. Use `workspace://files` to list all WFL files
2. Use `file:///{path}` to read each file
3. Use `parse_wfl` to understand structure
4. Provide a comprehensive overview

### Workflow 2: Finding and Fixing Errors

Ask Claude:

> "Find all errors in my WFL project and suggest fixes"

Claude will:
1. Use `workspace://diagnostics` to find all issues
2. Read affected files using `file:///{path}`
3. Use `analyze_wfl` for detailed error info
4. Suggest specific fixes for each error

### Workflow 3: Code Completion

Ask Claude:

> "Help me complete this WFL code: 'store x as 5\ncheck if x'"

Claude will:
1. Use `get_completions` to see available keywords
2. Understand context with `parse_wfl`
3. Suggest appropriate completions

### Workflow 4: Type Checking

Ask Claude:

> "Check if this code has any type errors: [paste code]"

Claude will:
1. Use `typecheck_wfl` to check types
2. Use `analyze_wfl` for additional diagnostics
3. Explain any type mismatches

## Advanced Usage

### Custom MCP Clients

You can build custom MCP clients to integrate WFL with your own tools:

```javascript
// Example: Node.js MCP client
const { spawn } = require('child_process');

const wflServer = spawn('wfl-lsp', ['--mcp'], {
  cwd: '/path/to/workspace'
});

// Send JSON-RPC request
const request = {
  jsonrpc: '2.0',
  id: 1,
  method: 'tools/call',
  params: {
    name: 'parse_wfl',
    arguments: {
      source: 'store x as 5'
    }
  }
};

wflServer.stdin.write(JSON.stringify(request) + '\n');

// Read response
wflServer.stdout.on('data', (data) => {
  const response = JSON.parse(data.toString());
  console.log(response);
});
```

### Programmatic Integration

See the [MCP Architecture Documentation](../technical/wfl-mcp-architecture.md) for details on implementing custom integrations.

## Troubleshooting

### Server Not Starting

**Issue:** `wfl-lsp --mcp` doesn't start

**Solutions:**
- Ensure `wfl-lsp` is in your PATH
- Check that you're running from a valid workspace directory
- Verify no other process is using stdin/stdout

### No Resources Found

**Issue:** `workspace://files` returns empty

**Solutions:**
- Ensure you're running from a directory with `.wfl` files
- Check file permissions
- Verify workspace path is correct

### Tools Not Working

**Issue:** Tools return errors

**Solutions:**
- Validate your WFL source code syntax
- Check that required parameters are provided
- Review error messages in the response

### Claude Desktop Not Detecting Server

**Issue:** WFL server not appearing in Claude Desktop

**Solutions:**
- Verify `claude_desktop_config.json` is in the correct location
- Check JSON syntax in configuration file
- Ensure `wfl-lsp` path is correct and executable
- Restart Claude Desktop after configuration changes

## Best Practices

### 1. Use Workspace Root

Always run `wfl-lsp --mcp` from your project root directory for best results with workspace resources.

### 2. Provide Context

When asking Claude for help, provide context:
- Mention what you're trying to achieve
- Share relevant error messages
- Describe the expected behavior

### 3. Leverage Resources

Use workspace resources for project-wide operations:
- Use `workspace://diagnostics` before starting work
- Check `workspace://config` to understand settings
- Use `workspace://symbols` for project navigation

### 4. Iterate with Tools

Combine tools for complex analysis:
1. `parse_wfl` to understand structure
2. `analyze_wfl` to find issues
3. `typecheck_wfl` to verify types
4. `lint_wfl` to improve code quality

## Limitations

Current limitations of the WFL MCP server:

- **Single Directory**: Only scans immediate workspace directory (not subdirectories)
- **No Watch Mode**: Doesn't auto-refresh when files change
- **Basic Symbol Info**: Symbol extraction is currently basic (statement counts only)
- **No Formatting**: `format_wfl` tool is placeholder (WFL formatter coming soon)

## Future Enhancements

Planned improvements:

- **MCP Prompts**: Code templates and snippets
- **Recursive Workspace Scan**: Support for nested directories
- **Enhanced Symbol Info**: Full symbol table with types and locations
- **Code Actions**: Quick fixes and refactorings
- **Real-time Updates**: Resource subscriptions for live updates
- **Code Execution**: Safe execution of WFL code with results

## Getting Help

- **GitHub Issues**: [WFL Repository](https://github.com/your-repo/wfl)
- **Documentation**: [WFL Docs](../README.md)
- **MCP Specification**: [Model Context Protocol](https://modelcontextprotocol.io/)

## See Also

- [WFL MCP Configuration](wfl-mcp-configuration.md)
- [WFL MCP API Reference](wfl-mcp-api-reference.md)
- [WFL MCP Architecture](../technical/wfl-mcp-architecture.md)
- [Claude Desktop MCP Integration](https://docs.anthropic.com/claude/mcp)
- [Model Context Protocol Specification](https://modelcontextprotocol.io/specification)

---

**Version:** WFL LSP v0.1.0 with MCP Support
**Last Updated:** January 2026
**Protocol Version:** MCP 2024-11-05
