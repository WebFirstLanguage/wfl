# MCP Integration

Model Context Protocol (MCP) lets AI assistants call WFL tools—parse, analyze, type-check, and lint—so help stays grounded in the real language, not guesses.

This is the **user and developer guide** for WFL’s MCP server (served by `wfl-lsp --mcp`). For editor features without AI, see [LSP Integration](lsp-integration.md).

## What is MCP?

MCP connects an assistant to tools and resources from an application.

**For WFL:** the assistant can validate syntax, report semantic and type issues, and suggest style fixes using the same pipeline as the compiler and linter—aligned with clear, actionable error reporting.

## Quick Setup (Claude Desktop)

### 1. Locate Config File

**Windows:** `%APPDATA%\Claude\claude_desktop_config.json`
**macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
**Linux:** `~/.config/Claude/claude_desktop_config.json`

### 2. Add WFL MCP Server

```json
{
  "mcpServers": {
    "wfl": {
      "command": "wfl-lsp",
      "args": ["--mcp"],
      "cwd": "C:\\path\\to\\your\\wfl\\project"
    }
  }
}
```

**For MSI installation:**
```json
{
  "mcpServers": {
    "wfl": {
      "command": "C:\\Program Files\\WFL\\bin\\wfl-lsp.exe",
      "args": ["--mcp"],
      "cwd": "C:\\Users\\YourName\\Documents\\wfl-projects"
    }
  }
}
```

### 3. Restart Claude Desktop

Close completely and restart.

### 4. Test Integration

Ask Claude:
> "Parse this WFL code and find errors"

Claude now has WFL tools!

## Available Tools

### 1. parse_wfl

**Purpose:** Validate syntax

**Usage:**
```
Claude, parse this WFL code:
<code>
```

**Returns:** AST or parse errors

### 2. analyze_wfl

**Purpose:** Semantic analysis

**Usage:**
```
Claude, analyze this WFL code for errors
```

**Returns:** Diagnostics (undefined variables, etc.)

### 3. typecheck_wfl

**Purpose:** Type checking

**Usage:**
```
Claude, check for type errors in this code
```

**Returns:** Type errors and warnings

### 4. lint_wfl

**Purpose:** Code quality checks

**Usage:**
```
Claude, lint this WFL code
```

**Returns:** Style issues and suggestions

### 5. get_completions

**Purpose:** Code completion

**Usage:**
```
Claude, what can I type after "display"?
```

**Returns:** Completion suggestions

### 6. get_symbol_info

**Purpose:** Symbol information

**Usage:**
```
Claude, what is this variable's type?
```

**Returns:** Symbol details

## Available Resources

### 1. workspace://files

**Purpose:** List project files

### 2. workspace://symbols

**Purpose:** Code symbols (functions, variables)

### 3. workspace://diagnostics

**Purpose:** Current errors/warnings

### 4. workspace://config

**Purpose:** Project configuration

### 5. file:///

**Purpose:** Read specific files

## Example Workflows

**Find all errors:**
> "Claude, find all errors in my WFL project"

**Understand code:**
> "What does this WFL program do?" (paste code)

**Get help:**
> "Help me write a WFL function that reads a CSV file"

**Refactor:**
> "Suggest improvements for this WFL code"

## Troubleshooting

### MCP Not Working

1. **Check config path** - Absolute path to wfl-lsp
2. **Check cwd** - Points to your project directory
3. **Restart Claude** - Completely quit and restart
4. **Check binary** - `wfl-lsp --version`

### Tools Not Appearing

1. Verify MCP server shows in Claude Desktop settings
2. Check for error messages in Claude
3. Test wfl-lsp manually: `wfl-lsp --mcp`

---

**Previous:** [← LSP Integration](lsp-integration.md) | **Next:** [Compiler Internals →](compiler-internals.md)
