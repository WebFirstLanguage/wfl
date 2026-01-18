# Configuration Reference

WFL uses configuration files to control runtime behavior, code quality settings, security policies, and web server options. This reference documents all available configuration options.

## Configuration File Locations

WFL loads configuration from two locations, with local settings overriding global ones:

### Global Configuration

The global configuration file provides system-wide defaults:

- **Linux/macOS:** `/etc/wfl/wfl.cfg`
- **Windows:** `C:\wfl\config`

You can override the global config path by setting the `WFL_GLOBAL_CONFIG_PATH` environment variable.

### Local Configuration

WFL searches for `.wflcfg` files by walking up the directory tree from your script's location. The closest `.wflcfg` file found takes precedence over any parent or global configuration.

```
my-project/
  .wflcfg                  # Applies to entire project
  src/
    module1/
      script.wfl           # Uses my-project/.wflcfg
    module2/
      .wflcfg              # Overrides parent config
      script.wfl           # Uses my-project/src/module2/.wflcfg
```

This allows project-wide configuration with per-module overrides as needed.

## Creating Configuration Files

### Interactive Wizard (Recommended)

Use the `--init` wizard to create a `.wflcfg` file interactively:

```bash
wfl --init              # Create in current directory
wfl --init /path/to/dir # Create in specific directory
```

The wizard will:
1. Prompt for all configuration options, grouped by category
2. Show defaults in brackets `[value]` - press Enter to accept
3. Validate input in real-time
4. Generate a well-formatted `.wflcfg` file with comments

### Manual Creation

You can also create configuration files manually using the format described below.

## Configuration File Format

Configuration files use a simple key-value format with `=` as the separator. Comments start with `#`.

```ini
# This is a comment
timeout_seconds = 60
logging_enabled = true
log_level = debug
```

## Configuration Options

### General Runtime Settings

#### timeout_seconds

Maximum execution time for a WFL script in seconds. The script will terminate if it exceeds this limit.

- **Type:** Integer (minimum: 1)
- **Default:** `60`
- **Example:** `timeout_seconds = 300`

#### logging_enabled

Enables logging output to `wfl.log` file in the script's directory.

- **Type:** Boolean (`true` or `false`)
- **Default:** `false`
- **Example:** `logging_enabled = true`

#### debug_report_enabled

Enables detailed debug reports when runtime errors occur. Reports include stack traces, variable values, and source context.

- **Type:** Boolean
- **Default:** `true`
- **Example:** `debug_report_enabled = false`

#### log_level

Controls the verbosity of log output when logging is enabled.

- **Type:** String (`debug`, `info`, `warn`, `error`)
- **Default:** `info`
- **Example:** `log_level = debug`

### Execution Logging Settings

These settings control detailed execution tracing for debugging purposes.

#### execution_logging

Enables execution logging for debugging. In debug builds, this defaults to `true`.

- **Type:** Boolean
- **Default:** `true` (debug builds), `false` (release builds)
- **Example:** `execution_logging = true`

#### verbose_execution

Enables detailed per-statement logging during execution.

- **Type:** Boolean
- **Default:** `false`
- **Example:** `verbose_execution = true`

#### log_loop_iterations

Enables logging of individual loop iterations.

- **Type:** Boolean
- **Default:** `false`
- **Example:** `log_loop_iterations = true`

#### log_throttle_factor

When loop iteration logging is enabled, logs every Nth iteration to reduce output volume.

- **Type:** Integer (minimum: 1)
- **Default:** `1000`
- **Example:** `log_throttle_factor = 100`

### Code Quality Settings

These settings control the WFL linter and code style enforcement.

#### max_line_length

Maximum allowed line length in characters.

- **Type:** Integer
- **Default:** `100`
- **Example:** `max_line_length = 120`

#### max_nesting_depth

Maximum allowed nesting depth for control structures (if, repeat, etc.).

- **Type:** Integer
- **Default:** `5`
- **Example:** `max_nesting_depth = 4`

#### indent_size

Number of spaces per indentation level.

- **Type:** Integer
- **Default:** `4`
- **Example:** `indent_size = 2`

#### snake_case_variables

Enforces snake_case naming convention for variables.

- **Type:** Boolean
- **Default:** `true`
- **Example:** `snake_case_variables = false`

#### trailing_whitespace

Controls whether trailing whitespace is allowed. When `false`, trailing whitespace triggers a warning.

- **Type:** Boolean
- **Default:** `false` (trailing whitespace not allowed)
- **Example:** `trailing_whitespace = true`

#### consistent_keyword_case

Requires consistent casing for keywords throughout the script.

- **Type:** Boolean
- **Default:** `true`
- **Example:** `consistent_keyword_case = false`

### Security Settings

These settings control subprocess execution and shell command security.

#### allow_shell_execution

Master switch for shell command execution. When `false`, all shell commands are blocked.

- **Type:** Boolean
- **Default:** `false`
- **Example:** `allow_shell_execution = true`

#### shell_execution_mode

Controls how shell commands are validated and executed.

- **Type:** String
- **Default:** `forbidden`
- **Options:**
  - `forbidden` - No shell execution allowed (most secure)
  - `allowlist_only` - Only commands in `allowed_shell_commands` can run
  - `sanitized` - Shell execution with validation and warnings
  - `unrestricted` - Legacy mode, not recommended for production

- **Example:** `shell_execution_mode = allowlist_only`

#### allowed_shell_commands

Comma-separated list of allowed shell commands when using `allowlist_only` mode.

- **Type:** Comma-separated strings
- **Default:** (empty)
- **Example:** `allowed_shell_commands = ls, cat, grep, echo`

#### warn_on_shell_execution

Emits a warning whenever a shell command is executed.

- **Type:** Boolean
- **Default:** `true`
- **Example:** `warn_on_shell_execution = false`

### Subprocess Resource Management

These settings control resource limits for spawned subprocesses.

#### max_concurrent_processes

Maximum number of subprocesses that can run simultaneously.

- **Type:** Integer
- **Default:** `100`
- **Example:** `max_concurrent_processes = 50`

#### max_buffer_size_bytes

Maximum size of output buffers for subprocess stdout/stderr in bytes.

- **Type:** Integer
- **Default:** `10485760` (10 MB)
- **Example:** `max_buffer_size_bytes = 5242880`

#### kill_on_shutdown

Automatically terminates all spawned subprocesses when the WFL script exits.

- **Type:** Boolean
- **Default:** `false`
- **Example:** `kill_on_shutdown = true`

### Web Server Settings

These settings control the built-in web server functionality.

#### web_server_bind_address

The IP address the web server binds to when using `listen on port` statements.

- **Type:** IP address string (IPv4 or IPv6)
- **Default:** `127.0.0.1` (localhost only)
- **Options:**
  - `127.0.0.1` - Listen on localhost only (default, most secure)
  - `0.0.0.0` - Listen on all network interfaces (allows external access)
  - `::1` - IPv6 localhost
  - Any valid IP address assigned to the machine

- **Example:** `web_server_bind_address = 0.0.0.0`

**Security Note:** Setting this to `0.0.0.0` exposes your web server to the network. Only use this when you intend to accept external connections.

## Example Configuration Files

### Development Configuration

```ini
# .wflcfg - Development settings
timeout_seconds = 300
logging_enabled = true
log_level = debug
debug_report_enabled = true

# Relaxed code style for development
max_line_length = 120
snake_case_variables = false

# Allow shell commands for development
allow_shell_execution = true
shell_execution_mode = sanitized
warn_on_shell_execution = true

# Local web server accessible from network
web_server_bind_address = 0.0.0.0
```

### Production Configuration

```ini
# .wflcfg - Production settings
timeout_seconds = 60
logging_enabled = true
log_level = warn
debug_report_enabled = false

# Strict code quality
max_line_length = 100
max_nesting_depth = 4
snake_case_variables = true

# Secure shell execution
allow_shell_execution = false
shell_execution_mode = forbidden

# Subprocess limits
max_concurrent_processes = 50
kill_on_shutdown = true

# Localhost only web server
web_server_bind_address = 127.0.0.1
```

### Minimal Configuration

```ini
# .wflcfg - Minimal settings (uses defaults for everything else)
timeout_seconds = 120
log_level = info
```

## Configuration Precedence

When the same setting appears in multiple locations, the following precedence applies (highest to lowest):

1. Local `.wflcfg` in the script's directory
2. Global configuration file (`/etc/wfl/wfl.cfg` or `C:\wfl\config`)
3. Built-in defaults

## Troubleshooting

### Configuration Not Loading

Ensure your `.wflcfg` file is in the same directory as the WFL script you're running, not in your current working directory.

### Invalid Values

Invalid values for configuration options are silently ignored, and the default value is used instead. Enable debug logging to see configuration loading messages:

```ini
logging_enabled = true
log_level = debug
```

### Checking Active Configuration

Run your script with debug logging enabled to see which configuration values are being loaded and from which files.

---

**Previous:** [Operator Reference](operator-reference.md) | **Next:** [Error Codes](error-codes.md)
