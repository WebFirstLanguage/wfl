# LSP Integration

WFL Language Server Protocol implementation provides IDE features for WFL development.

## What is LSP?

Language Server Protocol standardizes IDE features:
- Syntax highlighting
- Error checking
- Auto-completion
- Go-to definition
- Hover documentation

One server works with multiple editors (VS Code, Vim, Emacs, etc.).

## WFL LSP Server

**Location:** `wfl-lsp/` directory

**Technology:** tower-lsp crate

**Capabilities:**
- Real-time diagnostics
- Semantic tokens (highlighting)
- Completion suggestions
- Hover information
- Document symbols
- MCP server mode

## Building

```bash
# Build LSP server
cargo build --release -p wfl-lsp

# Binary location:
# Windows: target/release/wfl-lsp.exe
# Linux/macOS: target/release/wfl-lsp
```

## Running

### Standard LSP Mode

```bash
wfl-lsp
```

Communicates via stdin/stdout (JSON-RPC).

### MCP Server Mode

```bash
wfl-lsp --mcp
```

Provides tools for AI assistants (Claude Desktop).

### Debug Mode

```bash
# Verbose logging
RUST_LOG=trace wfl-lsp
```

## VS Code Integration

**Automatic:** Install VS Code extension

```powershell
scripts/install_vscode_extension.ps1
```

**Manual configuration:**

```json
{
  "wfl.lspPath": "C:\\path\\to\\wfl-lsp.exe",
  "wfl.enableLSP": true
}
```

## Other Editors

### Vim/Neovim

**With nvim-lspconfig:**

```lua
require'lspconfig'.wfl_lsp.setup{
  cmd = { 'wfl-lsp' },
  filetypes = { 'wfl' },
}
```

### Emacs

**With lsp-mode:**

```elisp
(lsp-register-client
 (make-lsp-client :new-connection (lsp-stdio-connection "wfl-lsp")
                  :major-modes '(wfl-mode)
                  :server-id 'wfl-lsp))
```

## Troubleshooting

### LSP Not Starting

1. Check binary exists: `wfl-lsp --version`
2. Check logs in editor output panel
3. Verify file association (`.wfl` → wfl)

### No Diagnostics

1. Verify LSP enabled in settings
2. Check file is valid WFL
3. Restart editor
4. Check LSP logs

### Performance Issues

1. Large files may be slow
2. Check RUST_LOG not set to trace in production
3. Update to latest version

---

**Previous:** [← Architecture Overview](architecture-overview.md) | **Next:** [MCP Integration →](mcp-integration.md)
