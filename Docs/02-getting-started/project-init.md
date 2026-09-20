# Initialize a WFL project

Run `wfl init` in your project's root directory to create a minimal WFL
configuration and give coding agents a starting point for working on the
application.

```bash
mkdir my-wfl-project
cd my-wfl-project
wfl init
```

For an existing project, change into its root and run the same command.
`wfl init` takes no directory argument and does not prompt for input.

## Files created

| File | Purpose |
|---|---|
| `.wflcfg` | Simple project settings for WFL tooling and runtime behavior |
| `AGENTS.md` | A short adapter directing agents to `CLAUDE.md` |
| `CLAUDE.md` | Shared application guidance, WFL syntax, commands, and documentation links |

The configuration filename is exactly `.wflcfg`, including the leading dot.
WFL discovers that name automatically; a file named `simple.wflcfg` is not
the project configuration file. The agent filenames use the conventional
uppercase names `AGENTS.md` and `CLAUDE.md`.

Initialization creates these three files only. It does not generate a starter
program, application directory structure, Git repository, or editor settings.
Create your first `.wfl` source file using the
[Hello, World! tutorial](hello-world.md), then run it with `wfl hello.wfl`.

## Existing files and repeated runs

Existing regular files keep their contents and permissions. On Unix, newly
created files use normal file permissions (`0666` restricted by your `umask`),
so project members can read or edit them when your settings allow it.
Running `wfl init` again
fills in missing files and reports which files were created or kept. It does
not merge new instructions into existing agent files or refresh an existing
configuration.

If `.wflcfg`, `AGENTS.md`, or `CLAUDE.md` is a directory, symbolic link, or other
non-regular file, initialization reports an error before creating any files.
Resolve the conflicting entry, then run the command again. A later filesystem
error, such as running out of space while writing, can leave files that were
already created; rerunning fills in missing files without replacing existing
ones.

Initialization uses bundled templates. It works offline, does not download
documentation, and does not change global WFL configuration. For global
defaults, use `wfl config`; see the
[Configuration Reference](../reference/configuration-reference.md).

## What agents receive

`CLAUDE.md` is a starting guide for an application written in WFL. It explains
how WFL executes programs, gives syntax examples and common mistakes to avoid,
and lists commands for running, analyzing, linting, and testing code. Customize
it with your application's entry points, test commands, layout, and conventions.
Keep `AGENTS.md` as the pointer so both files lead to the same guidance.

The generated guide also explains how to find and use:

- The [documentation hub](../README.md),
  [syntax reference](../reference/syntax-reference.md), language guides, and
  standard-library references for details beyond the introductory examples.
- The [editor and LSP setup guide](editor-setup.md), including the WFL language
  server and the separate MCP integration used by compatible agents.
- The [Docker testing guide](../guides/docker-testing.md), with commands for
  running application tests using the published WFL runtime on Linux and
  PowerShell.
- The [WFL Context7 library](https://context7.com/webfirstlanguage/wfl) for
  searching indexed WFL documentation when the agent has Context7 available.

These are navigation links, not installed integrations. Check `wfl --version`
and confirm examples against your installed runtime; upstream documentation
and the nightly Docker image can describe a newer version.

## Help and command names

```bash
wfl init --help
wfl init -h
```

Both help forms display usage without creating files. Unsupported options or
extra arguments also leave the project unchanged.

`init` is a CLI command, written as `wfl init`. The bare name takes precedence
over an extensionless program named `init`. Run such a program through an
explicit path, for example `wfl ./init`; `wfl init.wfl` still runs a source file
normally.
