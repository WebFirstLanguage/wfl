# Project initialization with application agent guidance

`wfl init` initializes the current directory with `.wflcfg`, `AGENTS.md`, and
`CLAUDE.md`. The command has no directory argument, works noninteractively from
bundled templates, and leaves global configuration unchanged. The exact
`.wflcfg` name matches WFL's existing project-configuration discovery.

The generated `AGENTS.md` points to `CLAUDE.md` as the shared application guide.
The latter introduces WFL execution, syntax, validation commands, LSP and MCP
setup, Docker testing, and documentation navigation, including the Context7
WFL library. It gives agents useful starting context without copying the WFL
compiler repository's contributor policies into application projects. The
command creates no starter program or application layout.

Existing regular files are preserved, so repeated runs fill in missing files
without replacing project-specific instructions or configuration. Conflicting
directories, symbolic links, and other non-regular entries are rejected during
preflight. Help and invalid argument handling perform no initialization.

The bare command name `init` is reserved for this CLI operation. An
extensionless source file with that name can still run through an explicit
path such as `wfl ./init`.

The Getting Started guide, documentation hub, resources, configuration reference,
changelog, and repository CLI instructions now link or describe the command.
The [change evidence](../../../Engineering/evidence/2026-09-19-project-init.md)
records Red-to-Green results, independent review, and verification limits as
checks complete.
