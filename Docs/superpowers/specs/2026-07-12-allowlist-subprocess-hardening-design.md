# Allowlist-only subprocess hardening design

**Date:** 2026-07-12
**Status:** Approved for implementation
**Security reference:** WFL-SEC-001 (CWE-78, CWE-426)

## Context

WFL applies subprocess policy before both `execute command` and `spawn command`.
In `allowlist_only` mode, the current policy reduces the requested program and
every configured entry to a basename. When a command needs a shell, policy
checks only the first token and then execution passes the complete original
string to `sh -c` or `cmd.exe /C`.

Those two behaviors violate the intended allowlist boundary:

- An allowed first token can authorize additional shell syntax and programs
  contained later in the command string.
- A relative or alternate executable path can impersonate an allowed basename.
- Authorization and execution do not bind to the same executable identity.

This change is a fail-closed security correction. It preserves the existing
configuration keys and WFL subprocess syntax, but intentionally rejects
previously accepted configurations and commands that cannot enforce a strict
allowlist.

## Goals

- Make `allowlist_only` incapable of launching a WFL-created shell command.
- Require allowlisted programs to be identified by canonical absolute paths.
- Execute the exact canonical path that passed authorization.
- Apply identical policy to foreground execution and background spawning.
- Keep literal argv execution available, including arguments containing shell
  metacharacters when supplied through `with arguments`.
- Provide actionable errors and migration documentation.
- Preserve the behavior of `forbidden`, `sanitized`, and `unrestricted` except
  where shared preparation code is refactored without semantic change.

## Non-goals

- Building a shell parser or attempting to sanitize arbitrary shell grammar.
- Treating an executable allowlist as an argument-capability sandbox. If an
  operator explicitly allowlists a general-purpose interpreter such as a shell,
  Python, or PowerShell, that executable retains its normal capabilities.
- Verifying executable ownership, signatures, package provenance, or hashes.
- Eliminating filesystem replacement races when an attacker can modify an
  already allowlisted executable path. Operators must protect allowlisted files
  and their parent directories.
- Publishing or releasing a security advisory. Disclosure and release timing
  remain Maintainer decisions under `SECURITY.md` and `GOVERNANCE.md`.

## Policy contract

| Mode | Direct program launch | WFL-created shell launch |
|---|---|---|
| `forbidden` | Blocked | Blocked |
| `allowlist_only` | Allowed only for a canonical absolute executable path present in `allowed_shell_commands` | Blocked |
| `sanitized` | Allowed under existing behavior | Allowed with existing warnings |
| `unrestricted` | Allowed under existing behavior | Allowed with existing warnings |

For `allowlist_only`:

- Every entry capable of authorizing a process is an absolute path.
- The requested program is also an absolute path.
- Bare names such as `git`, relative paths such as `./git`, and alternate paths
  sharing an allowed basename do not authorize a launch.
- Shell need may be inferred from metacharacters or requested explicitly with
  `using shell`; both forms are blocked before a shell process is constructed.
- A direct launch may use either the parsed no-shell command-string form or the
  explicit `with arguments` form. The latter remains the recommended form for
  untrusted argument data.

## Architecture and data flow

### 1. Prepare the launch once

The shared subprocess gate in `src/interpreter/mod.rs` will return a launch
decision rather than a Boolean. The decision distinguishes:

- a permitted shell launch for modes that support it; and
- a permitted direct launch carrying the exact executable path to use.

Both `execute_command` and `spawn_process` will consume this decision. Neither
path may independently re-resolve an allowlisted program after authorization.

### 2. Reject shell use before allowlist matching

When policy mode is `allowlist_only` and `needs_shell` is true, authorization
returns `Blocked` immediately. It does not parse or compare the first token and
does not return `RequiresShell`.

This removes the vulnerable transition from “the first token is allowed” to
“execute the entire string in a shell.” Intentional shell functionality
requires an operator to select `sanitized` or `unrestricted` explicitly.

### 3. Resolve a canonical executable identity

For a direct `allowlist_only` launch, the sanitizer performs this sequence:

1. Parse the requested program without shell fallback. A parse failure blocks
   the launch.
2. Remove only optional surrounding quotes used to represent a configuration
   path; do not reduce either side to a basename.
3. Require the requested program path to be absolute.
4. Canonicalize the requested path and require it to resolve to a file.
5. Examine configured allowlist entries. Relative entries and entries that do
   not resolve on the current host cannot authorize a launch. Their presence
   does not invalidate a separate, valid entry that matches the requested
   executable, but configuration checking reports them for correction.
6. Canonicalize each usable allowlist entry and compare the resulting executable
   identities. Unix comparison is case-sensitive. Windows comparison remains
   case-insensitive.
7. On a match, return the canonical requested path in the authorization result.
8. Construct `Command` with that returned path, not the original token.

Canonical identity comparison means a symlink may authorize only when it
resolves to the same configured executable target. A different file with the
same basename never matches.

### 4. Preserve argv boundaries

For no-shell command strings, WFL continues to parse the program and arguments
with `CommandSanitizer::parse_command`. For `with arguments`, each WFL list item
continues to become one argv element. In either case, direct arguments are not
interpreted as shell syntax.

An operator who allowlists an absolute path to `sh`, `cmd.exe`, PowerShell,
Python, or another interpreter is granting that executable's full argument-level
capabilities. Documentation will state this explicitly and will no longer use
`cmd.exe /C` as an example of safe untrusted-argument handling.

## Error handling

Policy failures remain runtime errors reported before any `Command::new` call.
Messages will distinguish these cases:

- `allowlist_only` rejected shell features or explicit `using shell`;
- the requested executable path was not absolute;
- the requested path did not resolve to a file;
- no canonical configured path matched the requested executable; and
- configured entries were unusable because they were relative or unresolved.

Each message will recommend the relevant remedy: use an absolute executable
path in both the WFL program and `allowed_shell_commands`, use `with arguments`
for data, or deliberately select a shell-capable mode when shell semantics are
required.

The configuration checker will describe `allowed_shell_commands` as a list of
absolute executable paths and, when `allowlist_only` is selected, report
non-absolute entries as invalid for the strict allowlist contract. Runtime
authorization remains fail closed even when configuration checking was not run.

## Compatibility and migration

The existing keys remain:

```ini
allow_shell_execution = true
shell_execution_mode = allowlist_only
allowed_shell_commands = C:\Windows\System32\where.exe
```

Unix configurations use their host's canonical absolute executable paths, for
example `/usr/bin/git`. Operators must update both the configuration entry and
the WFL command expression to use the absolute path.

Behavior changes:

- `allowed_shell_commands = echo, git` no longer authorizes a process.
- `/bin/echo` no longer matches an `echo` entry by basename.
- `./echo` and any alternate same-basename path remain blocked.
- Commands with shell syntax no longer run in `allowlist_only`, even when their
  first token names an allowed executable.
- Direct literal arguments continue to work when an approved absolute
  executable path is used.

The changelog, subprocess guide, configuration reference, security guide, and
configuration-help text will explain this corrective break and its migration.
A Dev Diary entry will record the rationale, behavior, tests, and residual
boundary around explicitly allowlisted interpreters.

## Test strategy

Implementation follows mandatory TDD. Each regression test is observed failing
against the vulnerable implementation before production code changes.

### Unit policy tests

`src/interpreter/command_sanitizer.rs` will cover:

- an allowed executable plus inferred shell syntax is blocked;
- explicit shell use is blocked in `allowlist_only`;
- bare and relative requested programs are blocked;
- an alternate path with the same basename is blocked;
- a configured relative basename cannot authorize a launch;
- a canonical exact executable path is authorized and returned;
- symlink/canonical identity behavior where supported;
- Unix case-sensitive and Windows case-insensitive identity comparison; and
- sanitized and unrestricted shell decisions retain their existing behavior.

### Integration security tests

`tests/subprocess_security_test.rs` will cover both `execute command` and
`spawn command`:

- shell chaining, pipes, redirection, substitution, and explicit `using shell`
  are denied under `allowlist_only` without producing a secondary marker;
- an attacker-controlled executable with an allowed basename at another path is
  denied;
- an exact absolute allowlisted executable succeeds through direct argv;
- shell metacharacters passed as a literal argv item remain literal;
- non-allowlisted direct execution remains denied; and
- intentional shell execution in a shell-capable mode remains a regression
  control.

Tests will use harmless temporary files and executables or the WFL test binary;
they will not execute destructive payloads. Platform-specific fixtures will not
use `cmd.exe` as the benign Windows allowlisted executable.

### Verification gates

After targeted red/green cycles, verification includes:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all --verbose`
- a release build followed by the repository integration-test script
- documentation example validation

## Files expected to change

- `src/interpreter/command_sanitizer.rs`
- `src/interpreter/mod.rs`
- `src/wfl_config/checker.rs`
- `tests/subprocess_security_test.rs`
- `Docs/04-advanced-features/subprocess-execution.md`
- `Docs/06-best-practices/security-guidelines.md`
- `Docs/reference/configuration-reference.md`
- `CHANGELOG.md`
- `Dev diary/2026-07-12-allowlist-subprocess-hardening.md`

`SECURITY.md` will not contain vulnerability details. Maintainers may separately
use the implementation evidence to prepare coordinated advisory and release
materials.

## Decision

WFL will make `allowlist_only` a direct-execution, canonical-absolute-path
policy. Shell-mediated launches are forbidden in that mode, and the executable
path returned by authorization is the path used for process creation. This
chooses a clear security boundary over basename and PATH compatibility.
