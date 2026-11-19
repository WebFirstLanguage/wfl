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
- **DNS** — which DNS servers outbound HTTP requests ask, in which order (see [DNS settings](#dns-settings))

It is **not** for application secrets or app-specific settings (ports your program chooses, API keys, business config). Put those in data files your program reads (see [Project Organization](../06-best-practices/project-organization.md)).

---

## Quick start

```bash
# Create project configuration and agent guidance in the current directory
wfl init

# Configure global defaults interactively
wfl config

# Check existing config files for missing/invalid settings
wfl --configCheck

# Fix common config issues automatically
wfl --configFix

# Lint code using style settings from .wflcfg
wfl --lint my_script.wfl
wfl --lint --fix my_script.wfl --in-place
```

For project-specific overrides, edit the `.wflcfg` created by `wfl init`, or
write one in your project root:

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
| `wfl init` | Creates missing `.wflcfg`, `AGENTS.md`, and `CLAUDE.md` in the current directory |
| `wfl config` | Interactive wizard; writes the global configuration file |
| `wfl --configCheck` | Validates local/global config against known settings |
| `wfl --configFix` | Checks and repairs common config problems |
| `wfl --lint <file>` | Style/quality checks driven by code-quality keys |
| `wfl --lint --fix <file>` | Print fixed source without changing the file |
| `wfl --lint --fix <file> --diff` | Preview fixes as a unified diff |
| `wfl --lint --fix <file> --in-place` | Atomically apply validated fixes to the file |
| `wfl --dump-env` | Environment dump (useful when diagnosing “config not loading”) |

`--fix` requires `--lint`. `--diff` and `--in-place` require `--fix` and are
mutually exclusive. Plain lint exits with `0` for clean source, `1` for lint
warnings, and `2` for invalid options, unreadable input, or invalid source.
A completed fix or preview exits with `0`; run plain lint again to detect
warnings that need manual changes. Neither operation executes the program.
See [Code Style Guide: Commands](../06-best-practices/code-style-guide.md#commands)
for output modes and preservation guarantees.

`wfl init` takes no directory argument and preserves existing regular files.
It uses bundled templates without prompts, network access, or changes to global
configuration. See [Initialize a Project](../02-getting-started/project-init.md)
for generated agent guidance, conflict handling, and help commands.

`wfl config` takes no directory argument. It writes global defaults to `C:\wfl\config` on Windows or `/etc/wfl/wfl.cfg` on Linux/macOS. Set `WFL_GLOBAL_CONFIG_PATH` to use another global configuration file. Project `.wflcfg` files are not created or changed.

If the global configuration file already exists, the command asks before replacing it. Press Enter at that confirmation to cancel. Missing parent directories are created only when saving, after all answers are complete. You need permission to write to the selected global location.

Saving first writes the complete configuration to a temporary file in the destination directory, then atomically replaces the target. If writing or replacement fails, the existing configuration remains unchanged. This prevents partial saves; it does not guarantee durability after power loss.

An existing symbolic link is preserved and its target is updated. Dangling links, directories, and read-only targets are rejected. Existing Unix permission bits are retained; preservation of ownership or custom access-control entries is not guaranteed.

The wizard prompts by category, shows defaults in `[brackets]`, validates input, and writes a well-commented file. Press Enter to accept each default. For optional settings without a default, including the TLS certificate and key paths, press Enter to leave the setting out of the generated file. Skipped TLS paths do not create empty assignments; TLS paths can still be supplied by a project's `.wflcfg` or the program's `secured` statement.

`wfl config` always starts the configuration command. To run a program file named exactly `config`, use an explicit path such as `wfl ./config`.

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
| `allowed_shell_commands` | comma-list | *(empty)* | Program names or explicit paths allowed in `allowlist_only` mode |
| `warn_on_shell_execution` | bool | `true` | Warn whenever a shell command runs |

### Subprocess resources

| Key | Type | Default | Purpose |
|---|---|---|---|
| `max_concurrent_processes` | integer | `100` | Max owned, unconsumed background process handles |
| `max_buffer_size_bytes` | integer | `10485760` (10 MiB) | Max stdout/stderr buffer per process |
| `kill_on_shutdown` | bool | `false` | Own process groups/jobs and close remaining children on shutdown or completion |

### Web server

| Key | Type | Default | Purpose |
|---|---|---|---|
| `web_server_bind_address` | IP string | `127.0.0.1` | Bind address for `listen on port` |
| `web_server_trusted_proxies` | Comma-separated IPs/CIDRs | empty | Trusted peers for validated `originating_ip`; `client_ip` stays the socket peer |
| `web_server_tls_cert_file` | path | *(none)* | Default PEM cert for bare `listen … secured` |
| `web_server_tls_key_file` | path | *(none)* | Default PEM key for bare `listen … secured` |
| `web_server_max_body_size` | integer ≥ 1 | `1048576` (1 MiB) | Max HTTP request body size (bytes); enforced while streaming (chunked-safe) |
| `web_server_max_response_size` | integer ≥ 1 | `67108864` (64 MiB) | Max handler or outbound HTTP response body size (bytes) |
| `web_server_request_queue_bound` | integer ≥ 1 | `256` | Max queued HTTP requests before shedding with 503 |
| `web_server_response_timeout_seconds` | integer ≥ 0 | `300` | Seconds to await a handler before shedding with 504; `0` disables |
| `outbound_stream_max_seconds` | integer ≥ 0 | `300` | Absolute total lifetime (seconds) of one outbound streaming response, distinct from the per-read idle timeout; `0` disables; values above one year use the one-year safety cap |
| `web_socket_queue_bound` | integer ≥ 1 | `1024` | Max queued frames/events per WebSocket channel before shedding |
| `web_socket_max_connections` | integer ≥ 1 | `1024` | Max simultaneous live WebSocket connections |
| `web_socket_max_message_size` | integer ≥ 1 | `1048576` (1 MiB) | Max size of a single WebSocket text message (bytes); larger frames are dropped |
| `web_socket_max_queued_bytes` | integer ≥ 1 | `16777216` (16 MiB) | Global ceiling on queued WebSocket payload bytes across all connections |

### DNS (name lookups for outbound HTTP)

| Key | Type | Default | Purpose |
|---|---|---|---|
| `dns_servers` | Comma-separated IP addresses, optional `:port` | *(not set)* | DNS servers WFL asks, one at a time, in this order |
| `dns_servers_mode` | `only` / `before_system` / `after_system` | `only` | Whether `dns_servers` replace the system's servers or come before/after them |
| `dns_timeout_seconds` | integer 1–30 | `timeout` in `/etc/resolv.conf`, else `5` | Seconds to wait for one server before asking the next |
| `dns_resolver` | `wfl` / `system` | `wfl` on Linux, `system` elsewhere | `system` switches back to the operating system's resolver (the off switch) |

### Execution budget keys (summary)

A single [`ExecutionBudget`](#execution-budget-resource-limits) governs every
resource ceiling as one coherent mechanism. These keys tune it (detailed below).
Each is chosen so ordinary programs never trip it while runaway behavior gets a
clean, catchable error instead of a crash or unbounded memory growth.

| Key | Type | Default | Purpose |
|---|---|---|---|
| `max_operations` | integer ≥ 0 | `0` (unlimited) | Hard ceiling on interpreter operations; `0` disables it |
| `max_call_depth` | integer ≥ 1 | `1000` | Max WFL call/recursion depth |
| `max_import_depth` | integer ≥ 1 | `64` | Max nested `load module` / `include` depth |
| `max_execute_file_depth` | integer ≥ 1 | `4` | Max `execute file` nesting depth |
| `max_pattern_steps` | integer ≥ 1 | `5000000` | Max pattern-matching transitions per match (ReDoS guard) |
| `max_pattern_states` | integer ≥ 1 | `10000` | Max simultaneously-active pattern states per match |
| `max_source_size` | integer ≥ 1 | `67108864` (64 MiB) | Max WFL source-file size (bytes) |
| `max_file_read_size` | integer ≥ 1 | `52428800` (50 MiB) | Max bytes buffered by one text or binary file read |

The wall-clock deadline (`timeout_seconds`), request body/response ceilings, and
the HTTP/WebSocket queue and connection bounds above are all part of the same
budget.

---

## Configuration options (detailed)

### General runtime settings

#### `timeout_seconds`

Maximum execution time for a WFL script in seconds. Outside a `main loop`, an
outbound `open url` request (connection, headers, and response body) consumes
the run's remaining time. A `main loop` remains exempt from the lifetime limit,
but each outbound request inside it gets this duration as a fresh finite timeout
so a stalled remote peer cannot wedge the server indefinitely.

- **Type:** Integer (minimum: 1)
- **Default:** `60`
- **Example:** `timeout_seconds = 300`

The file CLI preserves a maximum of 300 seconds for this configuration setting.
For a trusted batch job that needs a longer finite invocation, place
`--execution-timeout SECONDS` before its filename:

```bash
wfl --execution-timeout 1200 batch.wfl
wfl --execution-timeout 1200 --test batch.test.wfl
```

The option accepts whole seconds from 1 through 31,536,000 (one year). Zero,
negative or fractional values, nonnumeric values and duplicate options are
errors; there is no unlimited value. Without the option, the default remains
60 seconds and configured values retain the 300-second cap. The option may also
bound source analysis, lexing and parsing. It cannot accompany configuration
maintenance, environment dumps or editor launch. After an executable source
filename, arguments are passed literally to that program instead.

This is an invocation-only override of the shared execution deadline, starting
before source loading and covering the front end, interpreter, includes and
executed files. It does not reset between tests or nested files, and it does not
implicitly change the configuration of separately launched WFL children.
Ordinary foreground subprocess operations continue sharing the invocation
deadline, while explicit process-wait timeouts remain independently enforced.
Duration waits observe cancellation and the invocation deadline while waiting,
including a final wait with no following statement. Waits within an active
`main loop` retain the server lifetime exemption.

The override does not change other resource limits, subprocess permissions,
request/stream limits, or configured per-operation timeouts. A server's `main
loop` retains its existing lifetime exemption; its outbound requests and
implicit subprocess waits keep their finite configured timeouts. Use the option
only when the caller deliberately grants the job more execution time; it does
not establish a sandbox for untrusted programs.

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

Maximum allowed line length in Unicode characters. Long lines receive a lint
warning; the fixer does not automatically wrap them.

- **Type:** Integer
- **Default:** `100`
- **Example:** `max_line_length = 120`

#### `max_nesting_depth`

Maximum allowed nesting depth for control structures (`check if`, `repeat`, etc.).
Excessive nesting receives a lint warning and requires a manual change.

- **Type:** Integer
- **Default:** `5`
- **Example:** `max_nesting_depth = 4`

#### `indent_size`

Number of spaces per indentation level, used by both the linter and fixer.

- **Type:** Integer
- **Default:** `4`
- **Example:** `indent_size = 2`

#### `snake_case_variables`

Enables snake_case warnings for variables and actions, and safe local naming
fixes. Set to `false` to disable both. Conflicting names and public API/data
names are preserved even when this setting is enabled.

- **Type:** Boolean
- **Default:** `true`
- **Example:** `snake_case_variables = false`

#### `trailing_whitespace`

Controls whether trailing whitespace is allowed. When `false`, trailing
whitespace outside string literals triggers a warning and is removed by
`--fix`. When `true`, it is preserved and does not trigger a warning.

- **Type:** Boolean
- **Default:** `false` (trailing whitespace not allowed)
- **Example:** `trailing_whitespace = true`

#### `consistent_keyword_case`

Enables warnings for keyword casing and lowercase fixes for boolean literals
such as `YES` or `False`. Set to `false` to disable these warnings and fixes.
This setting does not change the language's case-sensitive keyword syntax;
the fixer does not convert identifiers into keywords.

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
  - `allowlist_only` — only direct-exec programs in `allowed_shell_commands` may run; shell features are rejected
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

Comma-separated list of allowed program names or explicit executable paths when
using `allowlist_only` mode. A name such as `echo` authorizes only a name-only
invocation resolved through the host process's `PATH`; it does not authorize
`./echo`, `/tmp/echo`, or another caller-selected path with the same basename.
Path-bearing commands require a path-bearing allowlist entry resolving to the
same executable. On Windows, comparison is case-insensitive.

`allowlist_only` never invokes a shell. Commands containing pipes, redirects,
expansion, command chaining, or other shell features are rejected even when
their first program is allowlisted. Pass data through `with arguments`; opt in
to `sanitized` or `unrestricted` only when shell syntax is genuinely required.
Do not allowlist a shell or interpreter (`sh`, `cmd.exe`, PowerShell, Python,
and similar) unless you intend its arguments to be able to execute code.

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

Maximum number of owned background process handles. Completed children count
until `wait for process` consumes their result or `close process` releases them.

- **Type:** Integer
- **Default:** `100`
- **Example:** `max_concurrent_processes = 50`

#### `max_buffer_size_bytes`

Maximum number of raw stream bytes retained in each stdout/stderr output
buffer. This limit applies to both foreground `execute command` capture and
background `spawn command` capture. If a stream exceeds the limit, WFL drains
it without growing memory, keeps the most recent bytes, and emits a truncation
warning. A value of `0` discards all captured output. Converting malformed
UTF-8 to WFL text may expand the returned text, but remains a bounded multiple
of this raw-byte ceiling.

- **Type:** Integer
- **Default:** `10485760` (10 MiB)
- **Example:** `max_buffer_size_bytes = 5242880`

#### `kill_on_shutdown`

Own launched processes in Unix groups or Windows Job Objects and terminate
remaining children when the direct child completes, is closed or times out, or
the interpreter shuts down. Linux also uses parent-death signalling for nested
WFL drivers. Enable this option in a test runner and its nested WFL fixtures.
The default preserves historical direct-child behavior without promising tree
cleanup. See [subprocess ownership](../04-advanced-features/subprocess-execution.md#own-the-process-lifetime).

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

#### `web_server_trusted_proxies`

Explicit proxy IP addresses or CIDR networks allowed to supply an HTTP
`X-Forwarded-For` chain. Applies to both HTTP and HTTPS listeners.

- **Type:** comma-separated IPv4/IPv6 addresses or CIDRs
- **Default:** empty (trust no proxies)
- **Example:** `web_server_trusted_proxies = 127.0.0.1, 10.0.0.0/8, 2001:db8:1::/48`
- **Limits:** 128 entries and 8192 bytes, including entry separators
- **Invalid values:** clear the whole list, including previously inherited trust;
  a warning explains the failure. An empty value also clears inherited trust.

Only configure addresses belonging to proxies you control. A trusted proxy must
overwrite client-supplied forwarding metadata or append the address it observed
to the right of the chain. Broad ranges grant that authority to every address
they contain. Configuration is read at server startup.

The request's `client_ip` remains the socket peer. `originating_ip` uses the
rightmost untrusted address in a validated `X-Forwarded-For` chain, starting
with the trusted socket peer and walking right to left. If every address is
trusted, the leftmost address is returned. IPv4-mapped IPv6 addresses are
normalized to IPv4 for matching and originating identity; mapped CIDRs require
prefixes of at least 96. Ordinary IPv4 and IPv6 networks match their own family.

No forwarding metadata is used when the socket peer is untrusted. Missing,
malformed, or duplicate `X-Forwarded-For` fields fall back to the socket peer.
The entire field must contain at most 32 plain IP addresses within 4096 bytes;
empty elements, ports, brackets, zone identifiers, and non-IP tokens are
rejected. `Forwarded` and `X-Real-IP` are not consulted. If no socket peer is
available, both identity fields are `"unknown"`.

See [request identity](../04-advanced-features/web-servers.md#request-identity-behind-a-proxy)
for application usage. Continue enforcing connection and real-client limits
at the proxy; application account limits complement those transport limits.

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

For multiple certificates on the same port, use the named certificate clauses
on `listen` described in [Multiple domains on one HTTPS port](../04-advanced-features/web-servers.md#multiple-domains-on-one-https-port).
These default configuration paths apply to bare `secured` statements only;
named-only listeners do not inherit a fallback certificate from configuration.

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

Maximum HTTP response body size, in bytes, for both directions: content a
handler may `respond with`, and content an outbound `open url` statement may
read. A larger handler response is refused; a larger outbound response is
stopped when either its received bytes or decoded UTF-8 text reaches this
limit. This applies even when the remote server uses chunked transfer encoding
or omits `Content-Length`.

- **Type:** Integer (bytes, at least 1)
- **Default:** `67108864` (64 MiB)
- **Example:** `web_server_max_response_size = 5242880`  # 5 MiB

#### `web_server_response_timeout_seconds`

Maximum time, in seconds, the transport waits for a handler to answer an accepted request before shedding it with a `504 Gateway Timeout` and freeing its in-flight slot. This bounds a dequeued-but-never-answered request so it cannot pin an in-flight slot indefinitely. It also bounds a single **streaming-response** `write` (`write line|chunk ... to <out>`): when a connected client stops reading, the bounded body channel fills and the write applies backpressure, so this timeout caps how long that write parks before failing — a stalled client can slow a handler but not pin it forever.

- **Type:** Integer (0 or more)
- **Default:** `300`
- **Example:** `web_server_response_timeout_seconds = 30`

A value of `0` disables the timeout (including the streaming-write bound above). It does **not** control how long `close out` (or the automatic end-of-handler stream drain) waits for the transport to finish the body: that finish wait is a short, fixed runtime ceiling independent of this setting, so a non-reading client cannot hold a serial `main loop` for the full write-timeout duration just by stalling at close. The in-flight request cap (`web_server_request_queue_bound`) is enforced globally across every `listen` server via one shared budget, and a request's slot is held from the moment its body starts streaming until the handler responds, this timeout fires, or the client disconnects.

#### `outbound_stream_max_seconds`

Absolute total lifetime, in seconds, of a single **outbound** streaming response opened with `open url ... and stream response as <name>`, measured from when the stream is opened. This is distinct from `timeout_seconds`, which is the per-read **idle** timeout: an upstream that trickles one byte just before every idle timeout would otherwise run forever, but it can never live past this hard cap. The cap is enforced in **real time**, not only on the next read: each incremental read (`wait for next line/chunk`) is bounded by the time remaining to this deadline, AND a background reaper closes the stream (cancelling the upstream request) when the deadline elapses — so even a stream that is opened and then never read cannot outlive the cap.

- **Type:** Integer (0 or more)
- **Default:** `300`
- **Example:** `outbound_stream_max_seconds = 60`

The `wfl config` wizard prompts for this global default. Press Enter to accept `300` seconds, enter `60` for one minute or another non-negative integer for a custom duration, or enter `0` to disable the limit.

A value of `0` disables the absolute cap (the idle timeout still applies per
read). Positive values above 31,536,000 seconds (one year) are safely clamped to
one year when the runtime creates the deadline. This keeps extreme configuration
values finite and prevents platform `Instant` overflow; timeout diagnostics
report the effective clamped duration.

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

#### `web_socket_max_message_size`

Maximum size in bytes of a single WebSocket text message, applied to both inbound frames and outbound `send`/`broadcast` frames. A larger frame is dropped (with a warning) rather than queued, so the per-message memory a connection can pin is bounded — the frame-count bound (`web_socket_queue_bound`) alone does not bound the *size* of each queued frame.

- **Type:** Integer (at least 1)
- **Default:** `1048576` (1 MiB)
- **Example:** `web_socket_max_message_size = 262144`

#### `web_socket_max_queued_bytes`

Global ceiling in bytes on all WebSocket payloads queued across every connection's inbound event and outbound frame channels at once. Each queued frame reserves its byte length against this ceiling and releases it when the frame is delivered, consumed, or shed, so a slow or absent consumer cannot buffer WebSocket memory without bound even under the per-message and per-channel count limits.

- **Type:** Integer (at least 1)
- **Default:** `16777216` (16 MiB)
- **Example:** `web_socket_max_queued_bytes = 8388608`

### DNS settings

These keys control how WFL turns a hostname (like `knowledge.lan`) into an IP
address when a program makes an **outbound HTTP request** (`open url ...`). They
do **not** affect database connections (`open database at "postgres://..."`),
which still use the operating system's resolver.

#### How lookups work

On **Linux**, WFL uses its own resolver by default. It asks DNS servers **one at
a time, in order**, the same way glibc does:

1. If the name is `localhost` or ends in `.localhost`, the answer is the local
   machine (`127.0.0.1`, `::1`). No DNS server is asked.
2. If the name is in the hosts file (`/etc/hosts`), that answer is used. The
   file is re-read when it changes.
3. If the same name was looked up recently, the remembered answer is used (see
   [Answer cache](#answer-cache)).
4. Otherwise WFL asks the first server. **Its answer is final** if it gives
   addresses, says "no such name", or says the name has no addresses. A later
   server can never override it.
5. WFL asks the **next** server only if the current one does not answer within
   the timeout, or answers "server failure", "refused", "not implemented",
   "format error", or something WFL cannot read.
6. The whole list is tried up to `attempts` times (from `options attempts:N` in
   `/etc/resolv.conf`, default 2). Within one try, WFL may re-send the same
   question to the same server over UDP if no reply has arrived yet. That never
   contacts another server.

WFL asks for IPv4 (A) and IPv6 (AAAA) addresses from each server at the same
time. If one family fails while the other comes back empty, that server could
not help, so WFL asks the next one. This keeps an IPv4-only internal host
working when a server is unreliable for one record type.

**Why this matters:** the Linux release binary is statically linked against
musl, whose own resolver sends every question to *all* listed servers at once and
takes the first reply. With `nameserver 10.0.0.53`, `nameserver 10.0.0.54` and
`nameserver 8.8.8.8`, a public server's "no such name" for an internal host often
arrived first, so internal names failed at random, and every internal name was
sent to the public server.

**What WFL reads from `/etc/resolv.conf`** (glibc's rules): the first three
usable `nameserver` lines (a repeated line still uses up one of the three, as in
glibc); the last `search` or `domain` line, using at most six domains; and
`options ndots:N`, `timeout:N` and `attempts:N` (capped at 15, 30 and 5). Other
options are ignored. When the file changes, WFL reads it again and forgets the
answers it had remembered, so a VPN or DHCP change takes effect at once.
A name with fewer dots than `ndots` is tried with each search domain first and
as written last; otherwise it is tried as written first. A name that ends in a
dot is tried only as written.

**Linux builds that use glibc** (built from source, or packaged by a
distribution) used to ask glibc, which can also consult other name sources set
in `/etc/nsswitch.conf`: mDNS (`.local` names), `myhostname`, LDAP or SSSD
host lists. WFL's resolver reads only the hosts file and DNS. If you rely on
those other sources, set `dns_resolver = system`. The released Linux tarball
is a static musl binary and never used those sources.

The hosts file may be UTF-8 (with or without a byte-order mark) or UTF-16 with
a byte-order mark, and a stray non-UTF-8 byte (for example in a comment) does
not stop WFL from reading the other lines.

**On Windows and macOS** the operating system's resolver stays the default.
Setting `dns_servers` switches that program to WFL's resolver (with
`dns_servers_mode = only`; the two mixing modes are Linux-only for now). The
hosts file is still honored on every platform, including
`%SystemRoot%\System32\drivers\etc\hosts` on Windows.

#### `dns_servers`

The DNS servers WFL asks, in the order to ask them. Use this when the machine's
own DNS settings are wrong and you cannot change them, or to make a program use
specific servers wherever it runs.

- **Type:** Comma-separated IP addresses, each with an optional port
- **Default:** not set (the system's servers are used)
- **Examples:** `10.0.0.53`, `10.0.0.53:5353`, `fd00::1`, `[fd00::1]:53`

```ini
# Ask the internal DNS servers, in this order, and nothing else.
dns_servers = 10.0.0.53, 10.0.0.54
```

Rules:

- **IP addresses only.** A hostname is rejected, because looking up a DNS
  server's own name would itself need DNS.
- IPv6 link-local addresses with a zone ID (`fe80::1%eth0`) are not supported
  yet. Use a global or unique-local IPv6 address instead.
- The default port is 53. A server listed twice is asked once, at its first
  position. At most 32 servers.
- An empty value (`dns_servers =`) means "not set". In a local `.wflcfg`, that
  clears a value inherited from the global configuration file.
- **An invalid value fails closed.** WFL does not fall back to other DNS
  servers. Every lookup fails with a message that names the file and the bad
  entry, and no DNS question is sent anywhere. Programs that never use the
  network are not affected.

#### `dns_servers_mode`

How `dns_servers` combine with the servers in `/etc/resolv.conf`.

- **Values:** `only` (default), `before_system`, `after_system`
- **`only`:** ask only `dns_servers`. The system's servers are never contacted.
- **`before_system`:** ask `dns_servers` first, then the system's servers.
- **`after_system`:** ask the system's servers first, then `dns_servers`.

```ini
# Prefer the internal servers, but keep the system's servers as a backup.
dns_servers = 10.0.0.53, 10.0.0.54
dns_servers_mode = before_system
```

> **Warning about `after_system`:** a final answer is final. If a system server
> answers "no such name" for an internal host, `dns_servers` are never asked.
> To make internal names work, put your internal servers **first**
> (`only` or `before_system`).

The mixing modes are Linux-only for now. On Windows and macOS they are reported
as "not supported on this platform yet". If the system has no usable servers,
`before_system` and `after_system` use `dns_servers` alone and log a warning.
Setting this key without `dns_servers` has no effect (`wfl --dump-env` says so).

#### `dns_timeout_seconds`

How long to wait for one DNS server before asking the next one.

- **Type:** Whole seconds, 1 to 30
- **Default:** `options timeout:N` in `/etc/resolv.conf`, otherwise `5`
- **Example:** `dns_timeout_seconds = 2`

An invalid value logs a warning and the default is used.

The worst case for one lookup, when every server is silent, is
`names tried × attempts × servers × timeout`. For example: three servers, two
attempts and a five-second timeout take 30 seconds for one name. A lookup still
counts toward the program's overall time limit (`timeout_seconds`), so it can
never wait longer than that.

#### `dns_resolver`

The off switch.

- **Values:** `wfl`, `system`
- **Default:** `wfl` on Linux, `system` on Windows and macOS
- **`system`:** use the operating system's resolver exactly as WFL did before
  this feature. `dns_servers` is then ignored, and `wfl --dump-env` says so.

```ini
# Go back to the operating system's resolver on this machine.
dns_resolver = system
```

#### Answer cache

WFL remembers answers in memory while the program runs, so a dead first server
does not slow down every request:

- Addresses are kept for as long as the DNS answer allows (its TTL), at most one
  day. When an unqualified name is expanded through the search list, a later
  candidate's addresses are kept only until the earliest earlier "no such name"
  TTL, so a preferred name is asked again once it can exist.
- "No such name" and "no addresses" answers are kept for at most 30 seconds.
- At most 1,024 names are kept; the least recently used one is dropped first.
- Failures (no server answered) are never cached. Nothing is written to disk.
- When `/etc/resolv.conf` changes, everything remembered is forgotten. A lookup
  that was already reading the old file cannot put that old answer back.

#### When a lookup fails

The error names every server WFL asked, in order, where each one came from, and
what it said, then gives a hint. For example:

```text
Failed to send HTTP GET request: Could not find the address of "knowledge.lan".

I asked these DNS servers, one at a time, in this order:
  1. 10.0.0.53:53  (from /etc/resolv.conf)  no reply within 5 seconds
  2. 10.0.0.54:53  (from /etc/resolv.conf)  answered "server failure"
  3. 8.8.8.8:53    (from /etc/resolv.conf)  answered "no such name" (final)

Hint: if this is an internal name, list your internal DNS servers first in .wflcfg, for example:
    dns_servers = 10.0.0.53, 10.0.0.54
```

The message contains only the hostname, never the URL's path or query string.
If WFL finds no DNS servers at all (no `dns_servers`, and `/etc/resolv.conf` is
missing, unreadable, or has no `nameserver` lines), the lookup fails at once and
says where it looked. It never falls back to a public resolver or to
`127.0.0.1`. To use a resolver on the same machine, say so:
`dns_servers = 127.0.0.1`.

#### Checking and troubleshooting DNS

- `wfl --dump-env` has a `=== DNS ===` section. It shows the resolver in use,
  the mode, every server in the order WFL asks them and which file it came from,
  the search list, the timeout and the attempts.
- `wfl --configCheck` validates the four DNS keys with the same rules as the
  loader. `wfl --configFix` never adds or changes DNS servers.
- With `logging_enabled = true` and `log_level = debug`, the WFL log records each
  server asked and its answer. Configuration warnings about DNS keys are written
  to the same log.

#### Security notes

- **Plain DNS can be forged** by someone on the same network. That is true for
  every resolver that uses plain DNS, including glibc and musl. WFL uses random
  query IDs and source ports and ignores replies that do not match, but the real
  protection is HTTPS: a forged answer then fails the certificate check. **Do not
  send secrets over plain `http://`.**
- **DNS rebinding is not blocked.** A DNS server can answer a public name with a
  private or loopback address. If a WFL program fetches URLs chosen by its users,
  check the destination yourself.
- **A project's `.wflcfg` can change DNS servers.** Review the `.wflcfg` of a
  downloaded project as you would its code (it can also enable shell
  execution).
- WFL's resolver never contacts a server that is not in the list shown by
  `wfl --dump-env`.

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

Maximum nesting depth of `execute file` runs. Kept small because each level re-enters the whole interpreter pipeline (lex → parse → analyze → interpret). The CLI runs the interpreter on a dedicated large stack so the depth guard can fire as a clean error before a native stack overflow; library embedders should use `wfl::run_with_interpreter_stack` (or an equivalent large stack) for the same guarantee.

- **Type:** Integer (at least 1)
- **Default:** `4`
- **Example:** `max_execute_file_depth = 6`

#### `max_pattern_steps`

Maximum number of pattern-VM transitions a single match attempt may take (Regular-expression Denial-of-Service, “ReDoS”, guard). A pathological pattern that would otherwise run away stops with a pattern step-limit error.

- **Type:** Integer (at least 1)
- **Default:** `5000000`
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

#### `max_file_read_size`

Maximum bytes one `read content`, `read binary`, or `read N bytes` operation may
buffer. The ceiling is enforced while the file is read, so streams and special
files whose metadata has no useful length cannot grow memory without bound. A
larger read fails with a catchable resource-limit error; exact-limit reads are
accepted.

- **Type:** Integer (bytes, at least 1)
- **Default:** `52428800` (50 MiB)
- **Example:** `max_file_read_size = 10485760`  # 10 MiB

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

The linter and fixer load configuration relative to the source file, including
the nearest ancestor `.wflcfg`. Fixes preserve comments, string contents and
escapes, expression grouping, and line endings. Only safe local naming changes
are automatic; public API/data names, collisions, and names in files that
import or export modules are preserved. Long lines and excessive nesting need
manual changes. A successful fix therefore does not guarantee a clean lint run.

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

**Exception:** invalid `dns_servers`, `dns_servers_mode` or `dns_resolver` values **fail closed**: every outbound HTTP lookup fails with a message naming the file and the bad value, so a typo can never silently send your hostnames to some other DNS server. See [DNS settings](#dns-settings).

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

### Internal hostnames fail at random, or resolve to the wrong address

The machine's DNS list probably mixes internal and public servers. Run
`wfl --dump-env` and check the `=== DNS ===` section: WFL asks the servers in
the order shown, and the first final answer wins. Put your internal servers
first, either in `/etc/resolv.conf` or, if you cannot change it, in `.wflcfg`:

```ini
dns_servers = 10.0.0.53, 10.0.0.54
```

See [DNS settings](#dns-settings).

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
