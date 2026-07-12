# Configuration Reference (`.wflcfg`)

**This is the single place for everything about WFL configuration files.**  
Runtime behavior, lint/style rules, shell security, subprocess limits, and web server settings all live in `.wflcfg` (and optional global config). Topic guides still cover *how to use* those features; this page documents the file itself end-to-end.

| If you want… | Go here |
|---|---|
| Full key list, defaults, format, CLI tools | **This page** |
| Style conventions (why defaults exist) | [Code Style Guide](../06-best-practices/code-style-guide.md) |
| Project layout (where `.wflcfg` sits) | [Project Organization](../06-best-practices/project-organization.md) |
| Web bind / TLS / body size in context | [Web Servers](../04-advanced-features/web-servers.md) |
| Shell & subprocess behavior | [Subprocess Execution](../04-advanced-features/subprocess-execution.md) |

---

## Table of Contents

1. [What is `.wflcfg`?](#what-is-wflcfg)
2. [Quick start](#quick-start)
3. [CLI tools](#cli-tools)
4. [Where configuration is loaded](#where-configuration-is-loaded)
5. [File format](#file-format)
6. [Quick reference (all keys)](#quick-reference-all-keys)
7. [Configuration options (detailed)](#configuration-options-detailed)
8. [How config relates to lint, style, and servers](#how-config-relates-to-lint-style-and-servers)
9. [Example configuration files](#example-configuration-files)
10. [Precedence](#precedence)
11. [Troubleshooting](#troubleshooting)
12. [Related documentation](#related-documentation)

---

## What is `.wflcfg`?

`.wflcfg` is WFL’s **project configuration file**: a simple key-value file (not a WFL program) that controls:

- **Runtime** — timeouts, logging, debug reports
- **Code quality** — line length, indent, naming, nesting (used by `wfl --lint`)
- **Security** — shell execution allowlists and modes
- **Subprocesses** — concurrency and buffer limits
- **Web server** — bind address, TLS cert/key defaults, max request body size, request queue bound

It is **not** for application secrets or app-specific settings (ports your program chooses, API keys, business config). Put those in data files your program reads (see [Project Organization](../06-best-practices/project-organization.md)).

---

## Quick start

```bash
# Create a project config interactively (recommended)
wfl --init
# or: wfl --init /path/to/project

# Check existing config files for missing/invalid settings
wfl --configCheck

# Fix common config issues automatically
wfl --configFix

# Lint code using style settings from .wflcfg
wfl --lint my_script.wfl
wfl --lint --fix my_script.wfl --in-place
```

Minimal hand-written file in your project root:

```ini
# .wflcfg
timeout_seconds = 120
log_level = info
max_line_length = 100
indent_size = 4
```

Then run scripts from that project tree; WFL walks up from the **script’s directory** and uses the nearest `.wflcfg`.

---

## CLI tools

| Command | Purpose |
|---|---|
| `wfl --init [dir]` | Interactive wizard; writes a commented `.wflcfg` |
| `wfl --configCheck` | Validates local/global config against known settings |
| `wfl --configFix` | Checks and repairs common config problems |
| `wfl --lint <file>` | Style/quality checks driven by code-quality keys |
| `wfl --lint --fix <file> --in-place` | Auto-fix style issues when possible |
| `wfl --dump-env` | Environment dump (useful when diagnosing “config not loading”) |

The wizard prompts by category, shows defaults in `[brackets]`, validates input, and writes a well-commented file.

---

## Where configuration is loaded

WFL merges **global** then **local** configuration. Local wins.

### Global configuration

System-wide defaults:

| Platform | Default path |
|---|---|
| Linux / macOS | `/etc/wfl/wfl.cfg` |
| Windows | `C:\wfl\config` |

Override the global path with the environment variable:

```text
WFL_GLOBAL_CONFIG_PATH=/path/to/your/global.cfg
```

### Local configuration (`.wflcfg`)

WFL searches for `.wflcfg` by **walking up the directory tree from the script’s location**. The **closest** file found wins (it does not merge multiple local files).

```text
my-project/
  .wflcfg                  # Project-wide
  src/
    module1/
      script.wfl           # Uses my-project/.wflcfg
    module2/
      .wflcfg              # Module override
      script.wfl           # Uses my-project/src/module2/.wflcfg
```

**Important:** Config is resolved from the **script file’s directory**, not necessarily your shell’s current working directory. Put `.wflcfg` next to (or above) the scripts you run.

---

## File format

Simple key-value pairs. Separator is `=`. Comments start with `#`. Blank lines are fine.

```ini
# Comment
timeout_seconds = 60
logging_enabled = true
log_level = debug
allowed_shell_commands = ls, cat, grep, echo
web_server_bind_address = 127.0.0.1
```

Rules of thumb:

- Keys are lowercase with underscores
- Booleans: `true` / `false`
- Integers: unquoted numbers
- Strings / paths: usually bare after `=` (trim whitespace)
- Invalid values are typically **ignored** and the default is kept (warnings may appear when logging is on)
- Unknown keys produce a warning and are ignored

---

## Quick reference (all keys)

All keys currently loaded from config files, with defaults.

### General runtime

| Key | Type | Default | Purpose |
|---|---|---|---|
| `timeout_seconds` | integer ≥ 1 | `60` | Max script run time (seconds) |
| `logging_enabled` | bool | `false` | Write logs to `wfl.log` in the script directory |
| `debug_report_enabled` | bool | `true` | Detailed reports on runtime errors |
| `log_level` | `debug` / `info` / `warn` / `error` | `info` | Log verbosity when logging is on |

### Execution logging

| Key | Type | Default | Purpose |
|---|---|---|---|
| `execution_logging` | bool | `true` (debug builds), `false` (release) | Execution tracing |
| `verbose_execution` | bool | `false` | Per-statement logging |
| `log_loop_iterations` | bool | `false` | Log loop iterations |
| `log_throttle_factor` | integer ≥ 1 | `1000` | Log every Nth iteration when loop logging is on |

### Code quality (lint / style)

| Key | Type | Default | Purpose |
|---|---|---|---|
| `max_line_length` | integer | `100` | Soft max line length |
| `max_nesting_depth` | integer | `5` | Max nesting of control structures |
| `indent_size` | integer | `4` | Spaces per indent level |
| `snake_case_variables` | bool | `true` | Prefer snake_case for variables |
| `trailing_whitespace` | bool | `false` | `false` = trailing whitespace not allowed |
| `consistent_keyword_case` | bool | `true` | Require consistent keyword casing |

### Security (shell)

| Key | Type | Default | Purpose |
|---|---|---|---|
| `allow_shell_execution` | bool | `false` | Master switch for all process launches |
| `shell_execution_mode` | string | `forbidden` | `forbidden` / `allowlist_only` / `sanitized` / `unrestricted` |
| `allowed_shell_commands` | comma-list | *(empty)* | Program basenames allowed in `allowlist_only` mode |
| `warn_on_shell_execution` | bool | `true` | Warn whenever a shell command runs |

### Subprocess resources

| Key | Type | Default | Purpose |
|---|---|---|---|
| `max_concurrent_processes` | integer | `100` | Max simultaneous subprocesses |
| `max_buffer_size_bytes` | integer | `10485760` (10 MiB) | Max stdout/stderr buffer per process |
| `kill_on_shutdown` | bool | `false` | Kill spawned processes when the script exits |

### Web server

| Key | Type | Default | Purpose |
|---|---|---|---|
| `web_server_bind_address` | IP string | `127.0.0.1` | Bind address for `listen on port` |
| `web_server_tls_cert_file` | path | *(none)* | Default PEM cert for bare `listen … secured` |
| `web_server_tls_key_file` | path | *(none)* | Default PEM key for bare `listen … secured` |
| `web_server_max_body_size` | integer ≥ 1 | `1048576` (1 MiB) | Max HTTP request body size (bytes) |
| `web_server_max_response_size` | integer ≥ 1 | `67108864` (64 MiB) | Max HTTP response body size (bytes) |
| `web_server_request_queue_bound` | integer ≥ 1 | `256` | Max queued HTTP requests before shedding with 503 |
| `web_socket_queue_bound` | integer ≥ 1 | `1024` | Max queued frames/events per WebSocket channel before shedding |
| `web_socket_max_connections` | integer ≥ 1 | `1024` | Max simultaneous live WebSocket connections |

### Execution budget (resource limits)

A single [`ExecutionBudget`](#execution-budget-resource-limits) governs every
resource ceiling as one coherent mechanism. These keys tune it. Each is chosen
so ordinary programs never trip it while runaway behavior gets a clean,
catchable error instead of a crash or unbounded memory growth.

| Key | Type | Default | Purpose |
|---|---|---|---|
| `max_operations` | integer ≥ 0 | `0` (unlimited) | Hard ceiling on interpreter operations; `0` disables it |
| `max_call_depth` | integer ≥ 1 | `1000` | Max WFL call/recursion depth |
| `max_import_depth` | integer ≥ 1 | `64` | Max nested `load module` / `include` depth |
| `max_execute_file_depth` | integer ≥ 1 | `4` | Max `execute file` nesting depth |
| `max_pattern_steps` | integer ≥ 1 | `100000` | Max pattern-matching transitions per match (ReDoS guard) |
| `max_pattern_states` | integer ≥ 1 | `10000` | Max simultaneously-active pattern states per match |
| `max_source_size` | integer ≥ 1 | `67108864` (64 MiB) | Max WFL source-file size (bytes) |

The wall-clock deadline (`timeout_seconds`), request body/response ceilings, and
the HTTP/WebSocket queue and connection bounds above are all part of the same
budget.

---

## Configuration options (detailed)

### General runtime settings

#### `timeout_seconds`

Maximum execution time for a WFL script in seconds. The script terminates if it exceeds this limit.

- **Type:** Integer (minimum: 1)
- **Default:** `60`
- **Example:** `timeout_seconds = 300`

#### `logging_enabled`

Enables logging output to `wfl.log` in the script’s directory.

- **Type:** Boolean (`true` or `false`)
- **Default:** `false`
- **Example:** `logging_enabled = true`

#### `debug_report_enabled`

Enables detailed debug reports when runtime errors occur (stack traces, variable values, source context).

- **Type:** Boolean
- **Default:** `true`
- **Example:** `debug_report_enabled = false`

#### `log_level`

Controls verbosity of log output when logging is enabled.

- **Type:** String (`debug`, `info`, `warn`, `error`)
- **Default:** `info`
- **Example:** `log_level = debug`

### Execution logging settings

#### `execution_logging`

Enables execution logging for debugging. In debug builds this defaults to `true`; in release builds, `false`.

- **Type:** Boolean
- **Default:** `true` (debug builds), `false` (release builds)
- **Example:** `execution_logging = true`

#### `verbose_execution`

Enables detailed per-statement logging during execution.

- **Type:** Boolean
- **Default:** `false`
- **Example:** `verbose_execution = true`

#### `log_loop_iterations`

Enables logging of individual loop iterations.

- **Type:** Boolean
- **Default:** `false`
- **Example:** `log_loop_iterations = true`

#### `log_throttle_factor`

When loop iteration logging is enabled, logs every Nth iteration to reduce volume.

- **Type:** Integer (minimum: 1)
- **Default:** `1000`
- **Example:** `log_throttle_factor = 100`

### Code quality settings

These settings control the WFL linter and style enforcement (`wfl --lint`). Defaults match the [Code Style Guide](../06-best-practices/code-style-guide.md).

#### `max_line_length`

Maximum allowed line length in characters.

- **Type:** Integer
- **Default:** `100`
- **Example:** `max_line_length = 120`

#### `max_nesting_depth`

Maximum allowed nesting depth for control structures (`check if`, `repeat`, etc.).

- **Type:** Integer
- **Default:** `5`
- **Example:** `max_nesting_depth = 4`

#### `indent_size`

Number of spaces per indentation level.

- **Type:** Integer
- **Default:** `4`
- **Example:** `indent_size = 2`

#### `snake_case_variables`

Enforces snake_case naming for variables.

- **Type:** Boolean
- **Default:** `true`
- **Example:** `snake_case_variables = false`

#### `trailing_whitespace`

Controls whether trailing whitespace is allowed. When `false`, trailing whitespace triggers a warning.

- **Type:** Boolean
- **Default:** `false` (trailing whitespace not allowed)
- **Example:** `trailing_whitespace = true`

#### `consistent_keyword_case`

Requires consistent casing for keywords throughout the script.

- **Type:** Boolean
- **Default:** `true`
- **Example:** `consistent_keyword_case = false`

### Security settings

These settings control **all** subprocess execution (`execute command` and
`spawn command`), on both the shell path and the direct-exec / argv path.
Policy is always checked before any process is started.
See also [Subprocess Execution](../04-advanced-features/subprocess-execution.md).

#### `allow_shell_execution`

Master switch for subprocess execution. When `false`, **all** process launches
are blocked (shell form and `with arguments` form). This is the secure default.

- **Type:** Boolean
- **Default:** `false`
- **Example:** `allow_shell_execution = true`

#### `shell_execution_mode`

Controls how subprocesses are authorized when `allow_shell_execution = true`.
Applied to every launch, not only shell metacharacter forms.

- **Type:** String
- **Default:** `forbidden`
- **Options:**
  - `forbidden` — no process execution allowed (most secure)
  - `allowlist_only` — only programs whose basename is in `allowed_shell_commands` may run
  - `sanitized` — any program may run; shell features produce warnings
  - `unrestricted` — any program may run with shell; not recommended for production
- **Example:** `shell_execution_mode = allowlist_only`

To opt in for local tooling (after enabling the master switch):

```ini
allow_shell_execution = true
shell_execution_mode = sanitized
```

Or tighter:

```ini
allow_shell_execution = true
shell_execution_mode = allowlist_only
allowed_shell_commands = echo, ls, git
```

#### `allowed_shell_commands`

Comma-separated list of allowed **program basenames** when using
`allowlist_only` mode. Matching uses the basename of the program path
(`/bin/echo` matches `echo`). On Windows, comparison is case-insensitive.

- **Type:** Comma-separated strings
- **Default:** *(empty)*
- **Example:** `allowed_shell_commands = ls, cat, grep, echo`

#### `warn_on_shell_execution`

Emits a warning whenever a shell command is executed.

- **Type:** Boolean
- **Default:** `true`
- **Example:** `warn_on_shell_execution = false`

### Subprocess resource management

#### `max_concurrent_processes`

Maximum number of subprocesses that can run simultaneously.

- **Type:** Integer
- **Default:** `100`
- **Example:** `max_concurrent_processes = 50`

#### `max_buffer_size_bytes`

Maximum size of output buffers for subprocess stdout/stderr, in bytes.

- **Type:** Integer
- **Default:** `10485760` (10 MiB)
- **Example:** `max_buffer_size_bytes = 5242880`

#### `kill_on_shutdown`

Automatically terminates all spawned subprocesses when the WFL script exits.

- **Type:** Boolean
- **Default:** `false`
- **Example:** `kill_on_shutdown = true`

### Web server settings

These control built-in `listen on port` servers. Feature walkthrough: [Web Servers](../04-advanced-features/web-servers.md).

#### `web_server_bind_address`

IP address the web server binds to.

- **Type:** IP address string (IPv4 or IPv6)
- **Default:** `127.0.0.1` (localhost only)
- **Common values:**
  - `127.0.0.1` — localhost only (default, most secure)
  - `0.0.0.0` — all interfaces (external access; use with a firewall)
  - `::1` — IPv6 localhost
  - Any valid IP on the machine
- **Example:** `web_server_bind_address = 0.0.0.0`

**Security:** `0.0.0.0` exposes the server on the network. Only use when you intend external connections.

#### `web_server_tls_cert_file`

Default TLS certificate (PEM) for `listen on port … secured` when the statement does not name a certificate.

- **Type:** File path string
- **Default:** none
- **Example:** `web_server_tls_cert_file = /etc/wfl/tls/cert.pem`

In-language paths always win:

```wfl
// Uses paths from this statement
listen on port 8443 secured with certificate "cert.pem" and key "key.pem" as s

// Uses .wflcfg defaults
listen on port 8443 secured as s
```

A plain `listen` (without `secured`) **always** serves HTTP — putting cert paths in `.wflcfg` never silently upgrades HTTP to HTTPS.

#### `web_server_tls_key_file`

Default TLS private key (PEM) for bare `listen … secured`.

- **Type:** File path string
- **Default:** none
- **Example:** `web_server_tls_key_file = /etc/wfl/tls/key.pem`

**Security:** Restrict key permissions (e.g. `chmod 600`). WFL validates both files at `listen` time.

#### `web_server_max_body_size`

Maximum HTTP request body size accepted by `listen on port` servers, in bytes. Larger bodies are rejected before they reach your handler (DoS protection).

- **Type:** Integer (bytes, at least 1)
- **Default:** `1048576` (1 MiB)
- **Example:** `web_server_max_body_size = 10485760`  # 10 MiB for uploads

Raise for `parse_multipart` or large `body_bytes` uploads; keep as small as practical on public APIs.

#### `web_server_request_queue_bound`

Maximum number of accepted-but-not-yet-handled HTTP requests held in the queue between the transport layer and your `wait for request` loop (DoS protection).

- **Type:** Integer (at least 1)
- **Default:** `256`
- **Example:** `web_server_request_queue_bound = 512`

Because request handlers run one at a time (see [Web Servers → Limitations](../04-advanced-features/web-servers.md#limitations--notes)), a burst of traffic queues up behind the handler. Without a bound, that queue could grow until the process runs out of memory. When the queue is full, the server **sheds** further requests with a `503 Service Unavailable` (and a `Retry-After` header) and logs a warning, instead of buffering unbounded work. Raise it to absorb larger bursts at the cost of more memory; lower it to shed sooner under load. A value of `0` is rejected (the default is kept).

#### `web_server_max_response_size`

Maximum HTTP response body a handler may `respond with`, in bytes. A larger response is refused (the handler gets a runtime error) rather than streaming an unbounded payload to the client.

- **Type:** Integer (bytes, at least 1)
- **Default:** `67108864` (64 MiB)
- **Example:** `web_server_max_response_size = 5242880`  # 5 MiB

#### `web_socket_queue_bound`

Maximum number of queued frames (per outbound connection) and lifecycle events (per server) held for a WebSocket before shedding. Bounds WebSocket memory the same way `web_server_request_queue_bound` bounds HTTP requests: when a channel is full, the extra frame/event is dropped and a warning is logged, instead of growing memory without bound.

- **Type:** Integer (at least 1)
- **Default:** `1024`
- **Example:** `web_socket_queue_bound = 4096`

#### `web_socket_max_connections`

Maximum number of simultaneous live WebSocket connections. A connection attempt beyond the limit is refused (the server sends a close frame and logs a warning) instead of registering unbounded connections.

- **Type:** Integer (at least 1)
- **Default:** `1024`
- **Example:** `web_socket_max_connections = 256`

### Execution budget (resource limits)

WFL enforces every resource ceiling through a single shared **execution budget**
object that travels with a run through parsing, evaluation, pattern matching, web
handling, and module loading. Consolidating these caps in one place means they
behave consistently and are tuned from one section of `.wflcfg`. The wall-clock
deadline is `timeout_seconds` (above); the byte and queue ceilings are the
`web_server_*` / `web_socket_*` keys (above). The remaining knobs:

#### `max_operations`

Hard ceiling on the number of interpreter operations a run may execute. This is a belt-and-suspenders guard against a program that spins without ever awaiting (which the wall-clock `timeout_seconds` may not catch promptly inside a tight loop).

- **Type:** Integer (0 or more)
- **Default:** `0` (unlimited — matches historic behavior)
- **Example:** `max_operations = 500000000`

A value of `0` disables the ceiling. Like `timeout_seconds`, this ceiling is **not** enforced inside a `main loop` (a long-lived server would otherwise stop after N operations); cooperative cancellation still applies.

#### `max_call_depth`

Maximum WFL call/recursion depth. When exceeded, the run stops with a clean, catchable *“Maximum call depth (N) exceeded — possible infinite recursion”* error instead of a native stack overflow that would abort the whole process.

- **Type:** Integer (at least 1)
- **Default:** `1000`
- **Example:** `max_call_depth = 2000`

WFL runs the interpreter on a large (1 GiB) stack so this depth is reached safely; if you raise the ceiling substantially and rely on very deep recursion, prefer an iterative formulation where practical.

#### `max_import_depth`

Maximum nesting depth of `load module` / `include from`. Circular imports are already detected separately; this bounds a legitimately deep — but likely accidental — dependency chain.

- **Type:** Integer (at least 1)
- **Default:** `64`
- **Example:** `max_import_depth = 128`

#### `max_execute_file_depth`

Maximum nesting depth of `execute file` runs. Kept small because each level re-enters the whole interpreter recursively.

- **Type:** Integer (at least 1)
- **Default:** `4`
- **Example:** `max_execute_file_depth = 6`

#### `max_pattern_steps`

Maximum number of pattern-VM transitions a single match attempt may take (Regular-expression Denial-of-Service, “ReDoS”, guard). A pathological pattern that would otherwise run away stops with a pattern step-limit error.

- **Type:** Integer (at least 1)
- **Default:** `100000`
- **Example:** `max_pattern_steps = 250000`

#### `max_pattern_states`

Maximum number of simultaneously-active states a single pattern match may hold. Bounds exponential state fan-out that step-counting alone does not catch.

- **Type:** Integer (at least 1)
- **Default:** `10000`
- **Example:** `max_pattern_states = 50000`

#### `max_source_size`

Maximum size, in bytes, of a WFL source file. A larger file is refused before it is lexed or parsed.

- **Type:** Integer (bytes, at least 1)
- **Default:** `67108864` (64 MiB)
- **Example:** `max_source_size = 1048576`  # 1 MiB

Each of these positive-integer keys rejects `0` and non-numeric values, keeping the default and logging a warning.

---

## How config relates to lint, style, and servers

### Style and lint

`.wflcfg` code-quality keys are what `wfl --lint` (and team style) use. Canonical narrative: [Code Style Guide](../06-best-practices/code-style-guide.md) and [Naming Conventions](../06-best-practices/naming-conventions.md).

```ini
max_line_length = 100
max_nesting_depth = 5
indent_size = 4
snake_case_variables = true
trailing_whitespace = false
consistent_keyword_case = true
```

Share one `.wflcfg` per repo so the whole team formats the same way ([Collaboration Guide](../06-best-practices/collaboration-guide.md)).

### Project layout

Recommended layout includes `.wflcfg` at the project root:

```text
my-wfl-project/
├── src/
│   └── main.wfl
├── tests/
├── .wflcfg          # Tooling / runtime / style
└── README.md
```

Application settings (business ports, feature flags) belong in data files your program loads — not in `.wflcfg`. Details: [Project Organization](../06-best-practices/project-organization.md).

### Web servers

| Concern | Typical key |
|---|---|
| Reachable from other machines | `web_server_bind_address = 0.0.0.0` |
| HTTPS defaults without hardcoding paths | `web_server_tls_cert_file` / `web_server_tls_key_file` |
| Large uploads | `web_server_max_body_size` |
| Bound request backlog under load | `web_server_request_queue_bound` |

TLS intent always lives in the program (`secured`); config only supplies default file paths.

### Shell / subprocesses

Secure defaults block shell execution. For controlled use:

```ini
allow_shell_execution = true
shell_execution_mode = allowlist_only
allowed_shell_commands = git, echo
warn_on_shell_execution = true
kill_on_shutdown = true
```

---

## Example configuration files

### Development

```ini
# .wflcfg - Development settings
timeout_seconds = 300
logging_enabled = true
log_level = debug
debug_report_enabled = true

# Relaxed code style for local experimentation
max_line_length = 120
snake_case_variables = false

# Shell allowed with warnings
allow_shell_execution = true
shell_execution_mode = sanitized
warn_on_shell_execution = true

# Reachable on the LAN (dev only)
web_server_bind_address = 0.0.0.0
```

### Production-oriented

```ini
# .wflcfg - Production-oriented settings
timeout_seconds = 60
logging_enabled = true
log_level = warn
debug_report_enabled = false

# Strict code quality
max_line_length = 100
max_nesting_depth = 4
snake_case_variables = true

# Secure subprocess policy (default): no external processes
allow_shell_execution = false
shell_execution_mode = forbidden

# Subprocess limits
max_concurrent_processes = 50
kill_on_shutdown = true

# Bind intentionally; put a reverse proxy in front for public traffic
web_server_bind_address = 127.0.0.1

# TLS defaults for `listen ... secured`
web_server_tls_cert_file = /etc/wfl/tls/cert.pem
web_server_tls_key_file = /etc/wfl/tls/key.pem
```

### Minimal

```ini
# .wflcfg - Minimal (everything else uses defaults)
timeout_seconds = 120
log_level = info
```

---

## Precedence

Highest wins:

1. Local `.wflcfg` nearest to the script (walk up from the script directory; first found wins)
2. Global configuration (`/etc/wfl/wfl.cfg`, `C:\wfl\config`, or `WFL_GLOBAL_CONFIG_PATH`)
3. Built-in defaults

There is no merge of multiple local `.wflcfg` files along the path — only the closest one is used for local settings (after global has already been applied as a base).

---

## Troubleshooting

### Configuration not loading

- Put `.wflcfg` in the **script’s** directory tree (same folder or a parent), not only in your shell cwd if that differs
- Confirm the filename is exactly `.wflcfg` (leading dot)
- Run with logging on and check for load messages:

```ini
logging_enabled = true
log_level = debug
```

### Invalid values

Invalid values for known keys are usually **silently ignored**; the previous/default value is kept. Unknown keys log a warning. Enable debug logging to see what was loaded.

### Checking active configuration

```bash
wfl --configCheck
wfl --configFix   # if check reports fixable issues
wfl --dump-env    # environment / diagnostic context
```

### Web server still on localhost

Set `web_server_bind_address` in the `.wflcfg` that actually applies to that script, then restart the program. Bind address is read when the process starts, not mid-run.

### HTTPS not using cert paths from config

- The listen form must include `secured`
- Paths in the `listen` statement override `.wflcfg`
- Both cert and key must be set (in statement or config) and readable

### Shell commands blocked

Defaults are secure (`allow_shell_execution = false`, `shell_execution_mode = forbidden`). Explicitly enable and choose a mode before expecting shell to work.

---

## Related documentation

| Topic | Document |
|---|---|
| Formatting philosophy & examples | [Code Style Guide](../06-best-practices/code-style-guide.md) |
| Naming + `snake_case_variables` | [Naming Conventions](../06-best-practices/naming-conventions.md) |
| Where to put `.wflcfg` in a repo | [Project Organization](../06-best-practices/project-organization.md) |
| Shared team config | [Collaboration Guide](../06-best-practices/collaboration-guide.md) |
| Bind address, TLS, body size in server tutorials | [Web Servers](../04-advanced-features/web-servers.md) |
| Running external commands | [Subprocess Execution](../04-advanced-features/subprocess-execution.md) |
| Docs hub | [Documentation README](../README.md) |

---

**Previous:** [Operator Reference](operator-reference.md) | **Next:** [Error Codes](error-codes.md)
