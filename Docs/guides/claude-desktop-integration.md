# Integrating WFL with Claude Desktop

Complete guide for using WFL with Claude Desktop through the Model Context Protocol (MCP).

## Overview

Claude Desktop is the official MCP client from Anthropic that allows Claude to interact with external tools and resources. This guide shows you how to integrate WFL with Claude Desktop for AI-powered WFL development.

## Prerequisites

1. **Claude Desktop** installed ([Download](https://claude.ai/download))
2. **WFL** installed with `wfl-lsp` executable in PATH
3. **A WFL project** to work with

## Quick Start (5 Minutes)

### Step 1: Locate Configuration File

Find your Claude Desktop configuration file:

- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Linux**: `~/.config/Claude/claude_desktop_config.json`

### Step 2: Add WFL MCP Server

Open the configuration file in your text editor and add:

```json
{
  "mcpServers": {
    "wfl": {
      "command": "wfl-lsp",
      "args": ["--mcp"],
      "cwd": "G:/Projects/my-wfl-project"
    }
  }
}
```

**Important:** Replace `"G:/Projects/my-wfl-project"` with the **absolute path** to your WFL workspace.

### Step 3: Restart Claude Desktop

1. Quit Claude Desktop completely
2. Restart the application
3. The WFL MCP server will be loaded automatically

### Step 4: Verify Connection

In Claude Desktop, you should see:
- WFL server in the MCP servers list
- Available tools: parse_wfl, analyze_wfl, typecheck_wfl, lint_wfl, get_completions, get_symbol_info
- Available resources: workspace://files, workspace://symbols, workspace://diagnostics, workspace://config

### Step 5: Start Using!

Try asking Claude:

> "What WFL files are in this workspace?"

> "Analyze my WFL code and find any errors"

> "Help me understand what this WFL program does: [paste code]"

## Configuration Options

### Basic Configuration

Minimal configuration for single project:

```json
{
  "mcpServers": {
    "wfl": {
      "command": "wfl-lsp",
      "args": ["--mcp"],
      "cwd": "/path/to/project"
    }
  }
}
```

### Advanced Configuration

Multiple projects with custom paths:

```json
{
  "mcpServers": {
    "wfl-main": {
      "command": "C:/Program Files/wfl/bin/wfl-lsp.exe",
      "args": ["--mcp"],
      "cwd": "G:/Projects/main-project",
      "env": {
        "RUST_LOG": "info"
      }
    },
    "wfl-experiments": {
      "command": "wfl-lsp",
      "args": ["--mcp"],
      "cwd": "G:/Projects/experiments"
    }
  }
}
```

**Configuration Fields:**

| Field | Required | Description |
|-------|----------|-------------|
| `command` | Yes | Path to wfl-lsp executable |
| `args` | Yes | Must include `["--mcp"]` |
| `cwd` | Recommended | Workspace directory (absolute path) |
| `env` | No | Environment variables (e.g., RUST_LOG) |

## Example Conversations

### Example 1: Code Analysis

**You:**
> "Can you analyze this WFL code and tell me if there are any issues?"
> ```wfl
> store x as 5
> store y as x + 10
> display z
> ```

**Claude (using analyze_wfl):**
> I found an issue in your code:
> - Line 3: Undefined variable 'z' (Error)
>
> The variable 'z' is used but never defined. You probably meant to write:
> ```wfl
> display y
> ```

### Example 2: Project Overview

**You:**
> "What WFL files are in my workspace and what do they do?"

**Claude (using workspace://files and file:/// resources):**
> Your workspace contains 3 WFL files:
>
> 1. **debug_split.wfl** (5 statements) - Implements string splitting functionality
> 2. **generate_hash.wfl** (12 statements) - Generates cryptographic hashes
> 3. **rust_loc_counter.wfl** (incomplete) - Has 1 parse error that needs fixing
>
> Would you like me to help fix the error in rust_loc_counter.wfl?

### Example 3: Type Checking

**You:**
> "Is this code type-safe?"
> ```wfl
> store name as "Alice"
> store age as 25
> store greeting as name + age
> ```

**Claude (using typecheck_wfl):**
> This code has a type error on line 3:
>
> Cannot add text and number types. The expression `name + age` tries to concatenate a text value ("Alice") with a number value (25).
>
> You need to convert the age to text first:
> ```wfl
> store greeting as name + " is " + (age as text)
> ```

### Example 4: Code Completion

**You:**
> "I'm writing a loop in WFL but forgot the syntax. Help me complete: 'count from 1'"

**Claude (using get_completions):**
> The WFL loop syntax is:
> ```wfl
> count from 1 to 10:
>     display "iteration"
> end count
> ```
>
> Key keywords for loops:
> - `count from` - Start of count loop
> - `to` - End value
> - `end count` - Close the loop

## Troubleshooting

### WFL Server Not Appearing

**Symptoms:**
- WFL not listed in Claude Desktop's MCP servers
- Tools/resources not available

**Solutions:**

1. **Check configuration file location**
   ```bash
   # Windows (PowerShell)
   echo $env:APPDATA\Claude\claude_desktop_config.json

   # macOS/Linux
   echo ~/.config/Claude/claude_desktop_config.json
   ```

2. **Validate JSON syntax**
   - Use a JSON validator
   - Check for missing commas, quotes
   - Ensure proper escaping of paths (use forward slashes)

3. **Verify wfl-lsp executable**
   ```bash
   wfl-lsp --mcp
   # Should start server and wait for input
   ```

4. **Check Claude Desktop logs**
   - **Windows**: `%APPDATA%\Claude\logs\`
   - **macOS**: `~/Library/Logs/Claude/`
   - Look for MCP connection errors

### Server Starting But Not Working

**Symptoms:**
- Server shows in list but tools fail
- Error messages about missing workspace

**Solutions:**

1. **Verify workspace path**
   - Must be absolute path
   - Must exist and be readable
   - Should contain `.wfl` files

2. **Check permissions**
   - Ensure Claude Desktop can execute wfl-lsp
   - Verify read permissions on workspace files

3. **Test manually**
   ```bash
   cd /your/workspace/path
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | wfl-lsp --mcp
   ```

### Tools Return Errors

**Symptoms:**
- parse_wfl fails with valid code
- analyze_wfl returns internal errors

**Solutions:**

1. **Check WFL version**
   ```bash
   wfl --version
   wfl-lsp --version  # When LSP has version command
   ```

2. **Verify source code**
   - Ensure code is valid WFL syntax
   - Check for special characters or encoding issues

3. **Review error messages**
   - Error responses include detailed information
   - Ask Claude to explain the specific error

## Best Practices

### 1. One MCP Server Per Project

Configure separate MCP server entries for each WFL project:

```json
{
  "mcpServers": {
    "wfl-project-a": {
      "command": "wfl-lsp",
      "args": ["--mcp"],
      "cwd": "/path/to/project-a"
    },
    "wfl-project-b": {
      "command": "wfl-lsp",
      "args": ["--mcp"],
      "cwd": "/path/to/project-b"
    }
  }
}
```

### 2. Specify Working Directory

Always set `cwd` to your project root for proper workspace resource access.

### 3. Use Absolute Paths

Use absolute paths in configuration to avoid path resolution issues:

```json
// Good
"cwd": "G:/Projects/my-wfl-app"

// Bad
"cwd": "../my-wfl-app"
```

### 4. Keep Configuration Updated

When moving or renaming projects, update your Claude Desktop configuration.

## Security Considerations

The WFL MCP server:

- **Read-only by default**: Cannot modify files (only reads)
- **Workspace-scoped**: Can only access files in configured workspace
- **No execution**: Does not execute WFL code (analysis only)
- **Local only**: Runs on your machine, no external network access

Safe to use with proprietary codebases.

## Example Use Cases

### Use Case 1: Code Review

Ask Claude to review your WFL code:

> "Review my WFL codebase and suggest improvements"

Claude will:
- Scan all files with workspace://files
- Analyze each file with analyze_wfl
- Check types with typecheck_wfl
- Provide comprehensive review

### Use Case 2: Debugging

Ask Claude to help debug:

> "I have an error in my WFL code but I can't figure out why"

Claude will:
- Use workspace://diagnostics to find errors
- Use analyze_wfl for detailed error info
- Explain the root cause
- Suggest fixes

### Use Case 3: Learning WFL

Ask Claude to teach you:

> "I'm new to WFL. Can you explain what this code does and suggest improvements?"

Claude will:
- Use parse_wfl to understand structure
- Explain each statement
- Suggest WFL best practices
- Provide learning resources

### Use Case 4: Refactoring

Ask Claude to help refactor:

> "Help me refactor this WFL code to be more maintainable"

Claude will:
- Analyze current structure with parse_wfl
- Check for issues with analyze_wfl
- Suggest improvements
- Verify refactored code with typecheck_wfl

## Advanced Topics

### Custom Workflows

Build custom automations using Claude + WFL MCP:

1. **Automated Testing**: Ask Claude to check all files before commit
2. **Code Generation**: Use completions to generate boilerplate
3. **Documentation**: Have Claude document your WFL code
4. **Migration**: Get help migrating between WFL versions

### Integration with CI/CD

While Claude Desktop is interactive, you can build custom MCP clients for automation:

```bash
# Example: CI/CD script
wfl-lsp --mcp < check-all-files.json
```

## FAQ

**Q: Can Claude modify my WFL files?**
A: No, the MCP server only provides read and analysis capabilities. Claude can suggest changes but cannot modify files directly.

**Q: Does this work with VS Code?**
A: No, VS Code uses the LSP server (default mode). MCP is for AI assistants like Claude Desktop.

**Q: Can I use this with other AI tools?**
A: Yes! Any MCP-compatible client can use the WFL MCP server.

**Q: Does the server need to be running constantly?**
A: No, Claude Desktop spawns the server when needed and stops it when done.

**Q: Can I have both LSP and MCP running?**
A: Yes! They're separate processes. VS Code uses LSP, Claude Desktop uses MCP.

**Q: What if my workspace has subdirectories?**
A: Currently, only the root directory is scanned. Subdirectory support is planned.

## Next Steps

- Read the [API Reference](wfl-mcp-api-reference.md) for detailed tool documentation
- Check the [Architecture Guide](../technical/wfl-mcp-architecture.md) to understand how it works
- Join the WFL community to share your experience

---

**Need Help?**
- [WFL GitHub Issues](https://github.com/your-repo/wfl/issues)
- [Claude Desktop Support](https://support.anthropic.com/)
- [MCP Community](https://modelcontextprotocol.io/community)
