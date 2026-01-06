# WFL MCP Server Configuration

This guide details how to configure the WFL Language Server Protocol (LSP) server when running in Model Context Protocol (MCP) mode.

## Running the Server

To start the server in MCP mode, use the `--mcp` flag:

```bash
wfl-lsp --mcp
```

### Environment Variables

The server respects the following environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Controls internal server logging level (e.g., `info`, `debug`, `error`). | `info` |
| `WFL_GLOBAL_CONFIG_PATH` | Path to a global configuration file. | Windows: `C:\wfl\config`<br>Linux/Mac: `/etc/wfl/wfl.cfg` |

## Workspace Configuration (`.wflcfg`)

The server looks for a `.wflcfg` file in the root of your workspace. This file uses a simple `key = value` format.

### Example `.wflcfg`

```ini
# General Settings
timeout_seconds = 60
logging_enabled = true
log_level = info
debug_report_enabled = true

# Code Quality
max_line_length = 100
indent_size = 4
snake_case_variables = true

# Security
allow_shell_execution = false
```

### Configuration Options

#### General Settings

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `timeout_seconds` | Integer | `60` | Global timeout for operations in seconds. |
| `logging_enabled` | Boolean | `false` | Enable/disable WFL execution logging. |
| `log_level` | Enum | `info` | Logging level (`debug`, `info`, `warn`, `error`). |
| `debug_report_enabled` | Boolean | `true` | Generate debug reports on failure. |

#### Execution Logging

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `execution_logging` | Boolean | `false` | Master switch for execution logging (defaults `true` in debug builds). |
| `verbose_execution` | Boolean | `false` | Log every statement execution. |
| `log_loop_iterations` | Boolean | `false` | Log each iteration of loops. |
| `log_throttle_factor` | Integer | `1000` | Log every Nth iteration if loop logging is active. |

#### Code Quality (Linter)

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_line_length` | Integer | `100` | Maximum characters per line before warning. |
| `max_nesting_depth` | Integer | `5` | Maximum block nesting level. |
| `indent_size` | Integer | `4` | Number of spaces for indentation. |
| `snake_case_variables` | Boolean | `true` | Enforce snake_case for variable names. |
| `trailing_whitespace` | Boolean | `false` | Allow trailing whitespace (false = warn). |
| `consistent_keyword_case` | Boolean | `true` | Enforce consistent casing for keywords. |

#### Process Security

These settings control the `exec` capability within WFL scripts.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `allow_shell_execution` | Boolean | `false` | Allow scripts to execute system shell commands. |
| `shell_execution_mode` | Enum | `Forbidden` | `Forbidden`, `AllowlistOnly`, `Sanitized`, `Unrestricted`. |
| `allowed_shell_commands` | List | `[]` | Comma-separated list of allowed commands (e.g. `ls,echo`). |
| `warn_on_shell_execution` | Boolean | `true` | Log a warning when a shell command is executed. |

#### Subprocess Resource Management

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `max_concurrent_processes` | Integer | `100` | Max simultaneous subprocesses. |
| `max_buffer_size_bytes` | Integer | `10MB` | Max output buffer for captured stdout/stderr. |
| `kill_on_shutdown` | Boolean | `false` | Kill child processes when parent exits. |

## Client Configuration (Claude Desktop)

To interact with the WFL MCP server using Claude Desktop, add the following to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "wfl": {
      "command": "wfl-lsp",
      "args": ["--mcp"],
      "cwd": "/absolute/path/to/your/workspace"
    }
  }
}
```

> **Note:** The `cwd` (Current Working Directory) is important because the server uses it to locate the `.wflcfg` file and the workspace root.

## Troubleshooting

- **Server fails to start**: Check if `wfl-lsp` is in your PATH. Try running `wfl-lsp --mcp` manually in a terminal.
- **Config not loading**: Ensure `.wflcfg` is in the directory specified by `cwd` in your client config.
- **Logs**: Set `RUST_LOG=debug` in the `env` section of your client configuration to see detailed server logs.

```json
"env": {
  "RUST_LOG": "debug"
}
```
