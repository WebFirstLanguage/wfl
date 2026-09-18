# Configuration wizard moves to `wfl config`

The configuration wizard now runs as `wfl config [dir]`, using the current
directory when no directory is supplied. The destination must already exist.
The wizard still creates only `.wflcfg` and asks before replacing an existing
file. Invalid command arguments are rejected before any prompts or writes.

The maintainer explicitly requested removing `--init` now, rather than keeping
a deprecated alias. That flag and bare `init` report the replacement command.
Project initialization is reserved for separate work. Existing extensionless
programs named `config` or `init` continue to execute, as do explicit paths and
script arguments containing these words.

Optional TLS certificate/key settings previously had no defaults but could not
be left blank: the wizard ignored their `required` metadata. Enter now leaves
optional settings without defaults absent from the generated configuration.
Required values still reject blank input; settings with defaults retain those
defaults, including the empty shell-command list. Prompts explain how to skip
optional settings, and generated files name `wfl config` in their header.

Maintained CLI/configuration documentation and the changelog describe the new
command and migration. Historical archived descriptions remain unchanged.

Tests were added and observed failing before implementation. Eleven wizard
unit tests and thirteen real CLI tests now pass. Detailed acceptance criteria,
Red evidence, broader verification, and remaining platform limits are recorded
in [the change evidence](../../../Engineering/evidence/2026-09-17-config-command.md).
