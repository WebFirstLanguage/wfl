# Claude Code Hooks

This directory contains hook scripts that run automatically during Claude Code operations.

## format-rust Hook

Automatically runs `cargo fmt --all` after any Edit or Write operation on Rust files.

### Files

- **format-rust.ps1** - PowerShell version (Windows/PowerShell Core)
- **format-rust.sh** - Bash version (Unix/Linux/macOS/Git Bash)

### Prerequisites

The hook configured in `../.claude/settings.json` currently uses PowerShell:

- **Windows PowerShell**: Built into Windows (default configuration)
  - Verify: `powershell --version`
- **PowerShell Core (pwsh)**: Optional cross-platform alternative (Windows/macOS/Linux)
  - Install: https://github.com/PowerShell/PowerShell#get-powershell
  - Verify: `pwsh --version`

### Alternative Configurations

For PowerShell Core (cross-platform Windows/macOS/Linux):

```json
{
  "type": "command",
  "command": "pwsh -File .claude/hooks/format-rust.ps1",
  "timeout": 120
}
```

For bash (Unix/macOS/Git Bash on Windows):

```json
{
  "type": "command",
  "command": "bash .claude/hooks/format-rust.sh",
  "timeout": 120
}
```

Current configuration (Windows PowerShell):

```json
{
  "type": "command",
  "command": "powershell -File .claude/hooks/format-rust.ps1",
  "timeout": 120
}
```

### How It Works

1. Claude Code triggers the hook after Edit/Write operations
2. Hook receives JSON with file path via stdin
3. Script checks if file ends with `.rs`
4. If true, runs `cargo fmt --all` to format all Rust code
5. Ensures consistent formatting across the codebase

### Troubleshooting

If the hook fails:
- Verify `pwsh` is installed and in PATH: `pwsh --version`
- Verify `cargo` is available: `cargo --version`
- Check hook execution permissions (Unix: `chmod +x .claude/hooks/format-rust.sh`)
- Review error messages in Claude Code output
