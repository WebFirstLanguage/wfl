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
