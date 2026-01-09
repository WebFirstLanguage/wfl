# Editor Setup

Get the best WFL development experience with syntax highlighting, real-time error checking, and AI assistance. This guide covers VS Code (recommended) and other editors.

## Quick Start (VS Code)

**For Windows users with MSI installer:**

The VS Code extension is automatically installed if you selected it during installation. You're done!

**For everyone else:**

Run the installation script from the WFL repository:

```powershell
# Windows PowerShell
.\scripts\install_vscode_extension.ps1
```

```bash
# Linux/macOS
./scripts/install_vscode_extension.sh
```

**That's it!** Open a `.wfl` file in VS Code and you'll have full IDE support.

---

## VS Code Extension Features

The WFL extension provides:

### 🎨 Syntax Highlighting

Beautiful color-coding for WFL syntax:

- **Keywords** - `store`, `check`, `display`, `action`
- **Strings** - `"text in quotes"`
- **Numbers** - `42`, `3.14`
- **Comments** - `// comments in gray`
- **Operators** - `is greater than`, `with`, `plus`

### ⚡ Real-Time Error Checking

Errors appear as you type:

```wfl
store age as "twenty"  // ⚠️ Warning appears here
display age plus 5     // ❌ Error: Cannot add Text and Number
```

Hover over the error to see the full message.

### 💡 Auto-Completion

Press `Ctrl+Space` to trigger suggestions:

- **Keywords** - Type `sto` → suggests `store`
- **Functions** - Type `di` → suggests `display`
- **Variables** - Shows variables you've defined
- **Snippets** - Quick templates for common code

### 📍 Go-to Definition

`Ctrl+Click` (or `F12`) on a variable or function to jump to its definition.

### 📖 Hover Documentation

Hover over built-in functions to see documentation:

```wfl
display "Hello"  // Hover over 'display' to see what it does
```

### 🔍 Find All References

Right-click a variable → "Find All References" to see everywhere it's used.

---

## LSP Server Setup

The Language Server Protocol (LSP) powers all the IDE features. Here's how to set it up manually if needed.

### Prerequisites

You need the `wfl-lsp` binary:

**Windows MSI users:** Already have it at `C:\Program Files\WFL\bin\wfl-lsp.exe`

**Building from source:**
```bash
cargo build --release -p wfl-lsp
```

The binary will be at:
- **Windows:** `target\release\wfl-lsp.exe`
- **Linux/macOS:** `target/release/wfl-lsp`

### VS Code LSP Configuration

The extension handles this automatically, but if you need manual configuration:

1. Open VS Code settings (`Ctrl+,`)
2. Search for "WFL"
3. Verify these settings:

```json
{
  "wfl.lspPath": "C:\\Program Files\\WFL\\bin\\wfl-lsp.exe",  // Adjust path
  "wfl.enableLSP": true
}
```

### Test LSP Server

Verify the LSP server works:

```bash
wfl-lsp --version
```

**Expected output:**
```
wfl-lsp v0.1.0
```

### Debug LSP Issues

Enable LSP logging:

```bash
# Windows (PowerShell)
$env:RUST_LOG="trace"
wfl-lsp

# Linux/macOS
RUST_LOG=trace wfl-lsp
```

This shows detailed logs for troubleshooting.

---

## MCP Integration (AI Assistance)

WFL supports the Model Context Protocol (MCP), letting AI assistants like Claude Desktop analyze and help with your code.

### What is MCP?

MCP lets AI assistants:
- Parse and analyze your WFL code
- Find errors and suggest fixes
- Explain what your code does
- Help write new code
- Provide code completions

Think of it as having an AI pair programmer who understands WFL.

### Setup for Claude Desktop

**Step 1: Locate your Claude Desktop config**

- **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`
- **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Linux:** `~/.config/Claude/claude_desktop_config.json`

**Step 2: Add WFL MCP server**

Open the config file and add:

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

**Important:** Replace `C:\\path\\to\\your\\wfl\\project` with your actual project path.

**For Windows MSI users:**
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

**For source builds (adjust paths):**
```json
{
  "mcpServers": {
    "wfl": {
      "command": "C:\\path\\to\\wfl\\target\\release\\wfl-lsp.exe",
      "args": ["--mcp"],
      "cwd": "C:\\path\\to\\your\\project"
    }
  }
}
```

**Step 3: Restart Claude Desktop**

Close Claude Desktop completely and restart it.

**Step 4: Test the integration**

In Claude Desktop, try:

> "Analyze my WFL code"

> "Find all errors in my WFL project"

> "What does this WFL function do?"

Claude now has access to WFL-specific tools!

### Available MCP Capabilities

**6 Tools:**
1. **parse_wfl** - Check syntax
2. **analyze_wfl** - Semantic analysis
3. **typecheck_wfl** - Type checking
4. **lint_wfl** - Code quality checks
5. **get_completions** - Code suggestions
6. **get_symbol_info** - Symbol information

**5 Resources:**
1. **workspace://files** - Project files
2. **workspace://symbols** - Code symbols
3. **workspace://diagnostics** - Error reports
4. **workspace://config** - Configuration
5. **file:///** - Individual file access

### Example Workflows with MCP

**Find errors:**
> "Claude, analyze test.wfl and find any errors"

**Explain code:**
> "What does this WFL program do?" (paste code)

**Get help coding:**
> "Help me write a WFL function that reads a file and counts lines"

**Type checking:**
> "Check if my WFL code has any type errors"

**Refactoring:**
> "Can you suggest improvements for this WFL code?"

For complete MCP documentation:
**[MCP User Guide →](../guides/wfl-mcp-guide.md)** *(coming soon)*

---

## Other Editors

While VS Code has the best support, you can use WFL with any editor.

### Vim / Neovim

**Syntax Highlighting:**

1. Copy syntax file:
   ```bash
   mkdir -p ~/.vim/syntax
   cp vscode-extension/syntaxes/wfl.tmLanguage.json ~/.vim/syntax/wfl.vim
   ```

2. Add to `.vimrc`:
   ```vim
   au BufRead,BufNewFile *.wfl set filetype=wfl
   ```

**LSP Support (Neovim with nvim-lspconfig):**

```lua
-- In your init.lua or init.vim
require'lspconfig'.wfl_lsp.setup{
  cmd = { 'wfl-lsp' },
  filetypes = { 'wfl' },
}
```

### Emacs

**Syntax Highlighting:**

Create `wfl-mode.el`:

```elisp
(define-derived-mode wfl-mode prog-mode "WFL"
  "Major mode for editing WFL files."
  (setq-local comment-start "// ")
  (setq-local comment-end ""))

(add-to-list 'auto-mode-alist '("\\.wfl\\'" . wfl-mode))
```

**LSP Support (with lsp-mode):**

```elisp
(add-to-list 'lsp-language-id-configuration '(wfl-mode . "wfl"))
(lsp-register-client
 (make-lsp-client :new-connection (lsp-stdio-connection "wfl-lsp")
                  :major-modes '(wfl-mode)
                  :server-id 'wfl-lsp))
```

### Sublime Text

**Syntax Highlighting:**

1. Navigate to `Packages/User/`
2. Create `WFL.sublime-syntax` based on `vscode-extension/syntaxes/`

**LSP Support (with LSP package):**

Install LSP package, then add to settings:

```json
{
  "clients": {
    "wfl-lsp": {
      "enabled": true,
      "command": ["wfl-lsp"],
      "selector": "source.wfl"
    }
  }
}
```

### Any Editor

**Minimum setup for any editor:**

1. **File association** - Open `.wfl` files as text
2. **Syntax highlighting** - Optional but recommended
3. **Run command** - Configure `wfl filename.wfl` as run command

You'll still get the benefits of WFL's clear syntax, even without IDE features!

---

## Recommended VS Code Settings

For the best WFL experience, add these to your VS Code settings:

```json
{
  // Enable format on save
  "editor.formatOnSave": true,

  // WFL-specific settings
  "wfl.enableLSP": true,
  "wfl.lintOnSave": true,
  "wfl.showHints": true,

  // Helpful general settings
  "editor.minimap.enabled": true,
  "editor.rulers": [100],  // Match WFL's max line length
  "files.associations": {
    "*.wfl": "wfl"
  }
}
```

---

## Troubleshooting

### "Extension not found" in VS Code

**Solution:**
1. Check VS Code extension is installed: `Ctrl+Shift+X` → Search "WFL"
2. If not found, run install script again
3. Restart VS Code

### Syntax highlighting not working

**Solution:**
1. Verify file extension is `.wfl`
2. Reload VS Code: `Ctrl+Shift+P` → "Reload Window"
3. Check extension is enabled: `Ctrl+Shift+X`

### LSP not providing suggestions

**Solution:**
1. Check `wfl-lsp` is installed: `wfl-lsp --version`
2. Enable LSP in settings: `"wfl.enableLSP": true`
3. Check VS Code output: View → Output → Select "WFL Language Server"
4. Restart VS Code

### MCP not working with Claude Desktop

**Solution:**
1. Verify `claude_desktop_config.json` path is correct
2. Check `wfl-lsp` command path is absolute
3. Verify `cwd` points to your project
4. Restart Claude Desktop completely
5. Check for MCP errors in Claude Desktop settings

### LSP crashes on startup

**Solution:**
1. Check RUST_LOG output: `RUST_LOG=trace wfl-lsp`
2. Verify WFL project is valid (no corrupted files)
3. Update wfl-lsp: Rebuild from source or reinstall MSI
4. Report bug on GitHub with logs

---

## Development Workflow

With everything set up, here's a recommended workflow:

### 1. Write Code in VS Code
- Get syntax highlighting
- See errors in real-time
- Use auto-completion

### 2. Test in REPL
- Quick experiments
- Test expressions
- Prototype functions

### 3. Run from Terminal
- Execute complete programs
- See full output
- Automated testing

### 4. Get AI Help (MCP)
- Ask Claude for explanations
- Get refactoring suggestions
- Find bugs

This combination gives you the best development experience!

---

## Next Steps

Now that your editor is configured:

**[Language Basics →](../03-language-basics/index.md)**
- Learn all WFL features
- Variables, loops, functions
- Error handling

**[Resources →](resources.md)**
- Find documentation
- Community links
- Learning resources

**[Standard Library →](../05-standard-library/index.md)**
- Explore built-in functions
- Try them in your editor with auto-completion!

---

## Editor Setup Checklist

Before moving on, verify:

- ✅ Syntax highlighting works (open a `.wfl` file)
- ✅ Error checking works (create a type error)
- ✅ Auto-completion works (press `Ctrl+Space`)
- ✅ Can run programs (`wfl filename.wfl`)
- ✅ (Optional) MCP works with Claude Desktop

**Happy coding!** Your editor is now optimized for WFL development.

---

**Previous:** [← REPL Guide](repl-guide.md) | **Next:** [Resources →](resources.md)
