# Configuration wizard moves to `wfl config`

The configuration wizard now runs as `wfl config`, with no directory argument.
It configures global defaults at `C:\wfl\config` on Windows or
`/etc/wfl/wfl.cfg` on Linux/macOS, with `WFL_GLOBAL_CONFIG_PATH` overriding the
destination. Project `.wflcfg` files remain unchanged. The wizard asks before
replacing an existing file and creates missing parent directories only when
saving after all answers are complete. Invalid command arguments are rejected
before any prompts or writes.

The command takes precedence over an extensionless program named `config`;
that program can still run through an explicit path such as `wfl ./config`.

Optional TLS certificate/key settings previously had no defaults but could not
be left blank: the wizard ignored their `required` metadata. Enter now leaves
optional settings without defaults absent from the generated configuration.
Required values still reject blank input; settings with defaults retain those
defaults, including the empty shell-command list. Prompts explain how to skip
optional settings, and generated files name `wfl config` in their header.

CLI/configuration documentation and the changelog describe global setup and
project overrides. The archived package-wizard comparison now refers generically
to WFL's configuration wizard, with its archive checksum updated.

Tests were added and observed failing before implementation. Verification of
the initial wizard changes, including Red evidence and platform limitations,
is recorded in [the initial change evidence](../../../Engineering/evidence/2026-09-17-config-command.md).
The [global-setup verification](../../../Engineering/evidence/2026-09-17-global-system-config.md)
records the final contract, parent-directory failure paths, and project-file
preservation checks.

## 2026-09-17 addendum: review fixes and scope authorization

Review of PR #731 identified two remaining gaps: saving could truncate an
existing configuration before all bytes were written, and the global wizard
did not expose `outbound_stream_max_seconds`. Saving now prepares the complete
configuration in a temporary file beside the destination and atomically replaces
the target only after preparation succeeds. Write or replacement failures
preserve the existing file; this does not promise durability after power loss.
The stream-lifetime prompt accepts Enter for `300` seconds, `60` or another
non-negative duration, and `0` to disable the limit. The documentation index now
describes both global and project configuration.

The maintainer explicitly authorized this breaking CLI contract: `wfl config`
takes no directory argument, configures global defaults, and takes precedence
over a bare filename matching the command. The maintainer also directed removal
of obsolete command terminology throughout the documentation, including the
archived package-wizard comparison and its checksum update. These are scoped
exceptions to the usual compatibility and archive-preservation policies, not
changes to those policies for future work.

## 2026-09-17 addendum: CI portability

Linux CI exposed a platform difference in rustyline's handling of piped input:
supported terminals suppress prompts, while the line-oriented mode prints them.
The CLI test helper now explicitly selects that mode with `TERM=dumb`, retaining
all prompt and configuration-value assertions. The standalone fuzz workspace
lockfile also includes the atomic-save implementation's `tempfile` dependency,
without changing existing dependency versions. Targeted CLI tests and the locked
fuzz compilation pass locally; the [review evidence](../../../Engineering/evidence/2026-09-17-config-review.md)
records the original CI failures and the remaining platform verification.
