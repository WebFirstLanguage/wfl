# Designing a Package Manager for WFL

## 1. Why WFL Needs a Package Manager

WFL is built on the belief that programming should read like natural language. The compiler, the syntax, the error messages — everything is designed so that a developer can look at WFL code and understand it without a manual. But right now, sharing WFL code between projects means copying `.wfl` files by hand or committing them into a monorepo. That does not scale.

A package manager solves this by giving developers a standard way to share libraries, reuse code across projects, and build on each other's work. But here is the constraint that makes WFL different from every other ecosystem: **the tools around WFL must feel like WFL itself.** If WFL code reads like English, then the manifest file, the CLI commands, and the error messages should read like English too.

This is not a cosmetic preference. It is a direct consequence of Principle 19 (Avoidance of Unnecessary Conventions): we should not borrow TOML from Rust or JSON from JavaScript just because that is what other ecosystems do. We should not name commands `init` or `publish` just because Cargo does. Every design decision in this document traces back to one or more of WFL's 19 guiding principles.

---

## 2. Design Philosophy

The package manager is governed by six principles above all others:

| Principle | How It Shapes the Design |
|---|---|
| **P1 — Natural-Language Syntax** | The manifest reads like English sentences, not key-value pairs. CLI commands use WFL keywords (`add`, `remove`, `create`). |
| **P2 — Minimize Special Characters** | No brackets, braces, or quotes in the manifest. No `=` for assignment — we use `is`, just like WFL itself. |
| **P3 — Readability and Clarity** | A developer who has never seen a package manifest should be able to read `project.wfl` and understand every line. |
| **P4 — Clear Error Reporting** | Every error message explains the problem in first person, shows what went wrong, and gives the exact command to fix it. |
| **P9 — Accessibility for Beginners** | `wfl create project` launches an interactive wizard. No flags to memorize, no config files to write by hand. |
| **P19 — Avoidance of Unnecessary Conventions** | We do not use TOML because Cargo does. We do not call it `publish` because npm does. We design from first principles. |

The remaining principles (P5 through P18) inform specific decisions throughout the document and are called out where they apply.

---

## 3. The Manifest: `project.wfl`

Every WFL package has a `project.wfl` file at its root. This is a flat file with natural-language syntax, inspired by WFL's own `.wflcfg` configuration format.

### Example

```
// project.wfl - Package manifest for a web application

name is greeting
version is 26.2.1
description is A web application that greets visitors
author is Alice Smith
license is MIT

entry is src/main.wfl

requires http-client 26.1 or newer
requires json-parser 25.12 or newer
requires text-utils any version

requires test-runner 26.1 or newer for development
```

### Design Choices

**`is` instead of `=`** — WFL uses `is` for property assignment (`name is "Alice"`, `age is 30`). The manifest follows the same convention. A WFL developer already knows what `is` means. *(P1, P19)*

**No quotes around values** — Names, versions, and descriptions are plain text. The parser treats everything after `is` as the value, trimmed of whitespace. If a value needs to span multiple lines, that is a sign it should be a separate file (like a README). *(P2)*

**`//` comments** — WFL uses `//` for comments. So does the manifest. *(P1, P3)*

**`requires` keyword** — `requires` is already a reserved keyword in WFL's lexer (`KeywordRequires` in `src/lexer/token.rs`). Using it in the manifest means the same word has the same meaning in code and in configuration. *(P1)*

**`for development` suffix** — Development-only dependencies are marked with a natural-language suffix rather than a separate section like `[dev-dependencies]`. This reads as a complete English sentence: "requires test-runner 26.1 or newer for development." *(P3)*

### Full Field Reference

| Field | Required | Example | Purpose |
|---|---|---|---|
| `name` | Yes | `name is my-app` | Package identifier (lowercase, hyphens allowed) |
| `version` | Yes | `version is 26.2.1` | Package version (YY.MM.BUILD) |
| `description` | Yes | `description is A math library` | One-line summary |
| `author` | No | `author is Alice Smith` | Package author |
| `authors` | No | `authors are Alice Smith and Bob Jones` | Multiple authors |
| `license` | No | `license is MIT` | SPDX license identifier |
| `entry` | No | `entry is src/main.wfl` | Main entry point (default: `src/main.wfl`) |
| `repository` | No | `repository is github.com/user/repo` | Source repository URL |
| `requires` | No | `requires http-client 26.1 or newer` | Dependency declaration |
| `needs` | No | `needs file-access` | Permission declaration |

---

## 4. Version Constraints

Version constraints use natural language built from existing WFL keywords. The words `or`, `and`, `between`, `any`, `exactly`, `below`, and `above` are all reserved keywords in WFL's lexer, so they carry consistent meaning.

| Constraint | Meaning | Example |
|---|---|---|
| `26.1 or newer` | >= 26.1.0 | `requires http-client 26.1 or newer` |
| `26.1.3 exactly` | == 26.1.3 | `requires http-client 26.1.3 exactly` |
| `between 25.12 and 26.2` | >= 25.12.0, <= 26.2.x | `requires http-client between 25.12 and 26.2` |
| `any version` | No constraint | `requires text-utils any version` |
| `26.1 or newer but below 27` | >= 26.1.0, < 27.0.0 | `requires http-client 26.1 or newer but below 27` |
| `above 25.6` | > 25.6.x | `requires json-parser above 25.6` |
| `below 27` | < 27.0.0 | `requires json-parser below 27` |

WFL uses a calendar-based version scheme (YY.MM.BUILD). The package manager adopts the same scheme. When a constraint omits the BUILD number (e.g., `26.1`), it matches any build within that month (`26.1.0`, `26.1.1`, `26.1.15`, etc.).

---

## 5. CLI Commands

Users interact with the package manager through the `wfl` command. Under the hood, `wfl` delegates package management operations to `wflpkg`. Advanced users can invoke `wflpkg` directly with the same subcommands.

### Project Creation

| Command | Purpose | Principle |
|---|---|---|
| `wfl create project` | Interactive wizard — asks for name, description, author, license | P9 (Beginners), P16 (Gradual learning) |
| `wfl create project called my-app` | Create a named project non-interactively | P1 (Natural language) |

The interactive wizard follows the same pattern as `wfl --init` for `.wflcfg` files: it prompts for each field with sensible defaults, explains what each field means, and generates the `project.wfl` and directory structure.

### Dependency Management

| Command | Purpose | Principle |
|---|---|---|
| `wfl add http-client` | Add a dependency (latest version) | P1 (`add` is a WFL keyword) |
| `wfl add http-client 26.1 or newer` | Add with a version constraint | P3 (Readable) |
| `wfl add test-runner for development` | Add a dev-only dependency | P3 (Readable) |
| `wfl remove http-client` | Remove a dependency | P1 (`remove` is a WFL keyword) |
| `wfl update` | Update all dependencies to latest allowed | P3 |
| `wfl update http-client` | Update a specific dependency | P3 |

### Building and Running

| Command | Purpose | Principle |
|---|---|---|
| `wfl build` | Build the project and all dependencies | P3 |
| `wfl run` | Run the project's entry point | P3 |
| `wfl test` | Run tests (`wfl --test` on test files) | P18 (Best practices) |

### Registry and Sharing

| Command | Purpose | Principle |
|---|---|---|
| `wfl share` | Publish the package to the registry | P12 (Community) |
| `wfl search http` | Search the registry for packages | P3 |
| `wfl info http-client` | Show details about a package | P3 |
| `wfl login` | Authenticate with the registry | P3 |
| `wfl logout` | Log out from the registry | P3 |

### Security and Maintenance

| Command | Purpose | Principle |
|---|---|---|
| `wfl check security` | Audit dependencies for known vulnerabilities | P8 (Built-in Security) |
| `wfl check compatibility` | Verify the package has not broken its API | P15 (Scalability) |

### Why `wfl share` Instead of `wfl publish`?

The word "publish" is borrowed from npm and Cargo. It carries connotations of formal academic or media publishing. The word "share" is what WFL developers actually do — they share their code with the community. It is simpler, friendlier, and aligns with P12 (Community and Collaboration). *(P19)*

---

## 6. Using Packages in Code

Packages integrate with WFL's existing module system through the planned V4 `package:` protocol:

```wfl
load module from "package:http-client"
include from "package:json-parser"
```

When the interpreter encounters a `package:` prefix, it resolves the package from the local cache (installed via `wfl add`). If the package is not cached, it reports a clear error with the exact command to install it.

No new syntax is required. The same `load module from` and `include from` statements that work with local files work with packages. *(P1, P16)*

---

## 7. The Lock File: `project.lock`

The lock file records the exact versions and checksums of every resolved dependency. It ensures that every developer on a team, and every CI server, uses identical dependency versions.

```
// Auto-generated by WFL. Do not edit.
// Records exact versions for reproducible builds.

package http-client
  version is 26.1.3
  checksum is wflhash:a3f8b2c9d4e5f6a7

package json-parser
  version is 25.12.8
  checksum is wflhash:b4c5d6e7f8a9b0c1
  requires text-utils 25.11.2

package text-utils
  version is 25.11.2
  checksum is wflhash:c5d6e7f8a9b0c1d2
```

The lock file uses the same flat-file format as `project.wfl` — `is` for assignment, `//` for comments, indentation for nesting. It is human-readable (P3) even though developers rarely need to read it. *(P2, P19)*

The lock file should be committed to version control. Running `wfl build` or `wfl update` regenerates it when dependencies change.

---

## 8. Package Registry: wflhub.org

The central registry hosts published WFL packages. It provides:

- **Search and discovery** via `wfl search <keywords>` and a web interface
- **Package pages** showing description, version history, dependencies, and download counts
- **Publishing** via `wfl share` after authenticating with `wfl login`
- **Private registries** for organizations that need to host internal packages

### Publishing Flow

```
wfl login
  → Authenticates with wflhub.org (browser-based OAuth)

wfl share
  → Validates project.wfl (name, version, description, entry point)
  → Runs wfl check compatibility (warns about breaking changes)
  → Packages source files into a .wflpkg archive
  → Computes WFLHASH checksum
  → Uploads to wflhub.org
  → Displays: "Shared greeting 26.2.1 to wflhub.org"
```

### Private Registries

Organizations can host their own registry. The `project.wfl` file supports a `registry` field:

```
registry is packages.mycompany.com
```

When set, `wfl share` and `wfl add` use the specified registry instead of wflhub.org.

---

## 9. Security Model

Security is not an afterthought — it is built into the package manager from the start. *(P8 — Built-in Security)*

### Checksum Verification

Every package in the registry includes a WFLHASH checksum (WFL's custom hash function). On installation, the package manager verifies the checksum and refuses to install packages that do not match. The lock file records checksums so that subsequent builds can verify integrity without contacting the registry.

### No Install Scripts

Unlike npm, WFL packages cannot run arbitrary code during installation. Packages are pure WFL source code. There are no `postinstall` scripts, no shell commands, no binary downloads. This eliminates an entire class of supply-chain attacks. *(P8)*

### Permission Declarations

Packages that need access to sensitive capabilities must declare their requirements in `project.wfl`:

```
needs file-access
needs network-access
```

The `needs` keyword is already reserved in WFL's lexer (`KeywordNeeds` in `src/lexer/token.rs`). When a developer runs `wfl add` for a package that declares permissions, the package manager displays a clear summary and asks for confirmation:

```
The package "file-manager" needs the following permissions:
  - file-access: Can read and write files on disk
  - network-access: Can make HTTP requests

Do you want to add this package? (yes/no)
```

### Security Auditing

`wfl check security` scans all dependencies for:
- Known vulnerabilities reported to the wflhub.org advisory database
- Packages that have been yanked or deprecated
- Permission escalation (a dependency requesting more permissions than its parent)

---

## 10. Error Messages

Every error message follows WFL's Elm-inspired error reporting philosophy (P4): first person voice, a clear explanation of the problem, and an exact command to fix it.

### Package Not Found

```
I could not find a package called "htpp-client" in the registry.

Did you mean one of these?
  - http-client (26.1.3) — An HTTP client library for WFL
  - http-server (26.1.1) — A lightweight HTTP server

To add http-client, run:
  wfl add http-client
```

### Version Conflict

```
I found a version conflict while resolving dependencies.

The package "web-framework" requires http-client 26.1 or newer,
but "legacy-tools" requires http-client below 26.

These two constraints cannot be satisfied at the same time.

You can:
  1. Update "legacy-tools" to a version that supports http-client 26.1:
       wfl update legacy-tools
  2. Remove "legacy-tools" if you no longer need it:
       wfl remove legacy-tools
```

### Security Warning

```
I found 2 security advisories affecting your dependencies.

  http-client 26.1.2 — HIGH: Request smuggling vulnerability
    Fixed in 26.1.3. Run: wfl update http-client

  json-parser 25.12.5 — LOW: Excessive memory use on malformed input
    Fixed in 25.12.8. Run: wfl update json-parser

To update all affected packages at once, run:
  wfl update
```

### Registry Unreachable

```
I could not connect to the registry at wflhub.org.

This might be a network issue, or the registry might be temporarily
unavailable.

Your project can still build using cached packages. To build offline:
  wfl build

To retry connecting:
  wfl update
```

### Missing Manifest

```
I could not find a project.wfl file in this directory.

Every WFL project needs a project.wfl file to manage dependencies.
To create one interactively, run:
  wfl create project

Or create one manually — here is a minimal example:
  name is my-project
  version is 26.1.1
  description is A new WFL project
```

### Permission Denied During Share

```
I could not share "greeting" because you are not logged in.

To log in to wflhub.org, run:
  wfl login

Then try sharing again:
  wfl share
```

---

## 11. Workspace Support

For monorepo projects that contain multiple packages, a `workspace.wfl` file at the repository root defines the workspace:

```
// workspace.wfl

name is my-organization

member is packages/core
member is packages/web-server
member is packages/cli-tool
```

### How Workspaces Work

- All members share a single `project.lock` at the workspace root
- Running `wfl build` at the root builds all members
- Running `wfl build` inside a member builds only that member (and its dependencies)
- Members can depend on each other: `requires core any version` resolves to the local workspace member first *(P15 — Scalability)*
- Each member has its own `project.wfl` with its own name, version, and dependencies

### Example Structure

```
my-organization/
  workspace.wfl
  project.lock            // shared across all members
  packages/
    core/
      project.wfl
      src/
        main.wfl
    web-server/
      project.wfl
      src/
        main.wfl
    cli-tool/
      project.wfl
      src/
        main.wfl
```

---

## 12. Project Structure

### What `wfl create project` Generates

```
my-app/
  project.wfl             // package manifest
  .wflcfg                 // local project settings (already supported by WFL)
  src/
    main.wfl              // entry point
```

The generated `project.wfl`:

```
// project.wfl

name is my-app
version is 26.1.1
description is A new WFL project
author is Alice Smith
license is MIT

entry is src/main.wfl
```

The generated `src/main.wfl`:

```wfl
// my-app - A new WFL project

display "Hello from my-app!"
```

### After Adding Dependencies

```
my-app/
  project.wfl
  project.lock            // auto-generated after first wfl add
  .wflcfg
  src/
    main.wfl
  packages/               // local dependency cache
    http-client/
    json-parser/
```

The `packages/` directory is the local cache of installed dependencies. It should be added to `.gitignore` — the `project.lock` file is what ensures reproducibility, not the cached files themselves.

---

## 13. Architecture

```
User runs: wfl add http-client
               |
               v
         wfl CLI (detects package command)
               |
               v
         wflpkg (package manager core)
               |
     +---------+---------+
     |                   |
     v                   v
project.wfl parser    Registry client (wflhub.org)
     |                   |
     v                   v
Dependency resolver   Package cache (~/.wfl/packages/)
     |
     v
project.lock writer
     |
     v
Local packages/ directory (symlinks or copies from cache)
```

### Components

1. **wfl CLI layer** — Detects package-management subcommands (`add`, `remove`, `create project`, `share`, `search`, `build`, `update`) and delegates to wflpkg. Non-package commands (`wfl run`, `wfl --lint`) continue to work as before.

2. **wflpkg core** — The package manager library. Parses `project.wfl` and `project.lock`, resolves dependencies, manages the cache, and communicates with the registry. Can also be invoked directly as `wflpkg` for advanced use.

3. **project.wfl parser** — Reads the natural-language manifest format. Validates field names, version constraints, and permission declarations.

4. **Dependency resolver** — Takes the dependency graph from `project.wfl`, consults the registry for available versions, and produces a resolved set of exact versions. Implements a deterministic algorithm: the same inputs always produce the same lock file.

5. **Package cache** — Global cache at `~/.wfl/packages/`. Stores downloaded package archives and extracted source. Uses WFLHASH checksums to detect corruption or tampering.

6. **Registry client** — HTTP client for wflhub.org (or a configured private registry). Handles authentication, search, download, and publishing.

7. **Compiler integration** — After resolving dependencies, `wfl build` invokes the WFL compiler with the appropriate module paths. The `package:` protocol in the module loader resolves imports from the local `packages/` directory.

---

## 14. Avoiding Common Pitfalls

### Single Version Per Package

The resolver enforces a single version of each package within a build. No nested dependency trees, no "works on my machine" version conflicts. If two dependencies require incompatible versions of the same package, the resolver reports a clear error with actionable solutions (see Section 10). *(P4)*

### Project-Local Dependencies

All dependencies live in the project's `packages/` directory and are defined in `project.wfl`. There is no global package installation that silently affects other projects. A separate `wfl add --global` command may be added in the future for CLI tools, but library code is always project-local. *(P17 — Error Transparency)*

### No Install Scripts

WFL packages are pure source code. No arbitrary scripts run during installation. If a package needs to generate files or perform setup, it does so at runtime through normal WFL code, not through hidden install hooks. *(P8)*

### Lock File Ensures Consistency

The `project.lock` file captures exact versions and checksums. Every developer and every CI server resolves to the same dependency tree. Running `wfl update` is the only way to change locked versions — it never happens silently. *(P17)*

### Backward Compatibility Checking

WFL promises backward compatibility across releases. The `wfl check compatibility` command analyzes the public API of a package (exported actions, containers, and constants) and warns if a new version removes or changes them. This catches breaking changes before they reach the registry. *(P15)*

---

## 15. Implementation Roadmap

### Phase 1: Manifest and Project Creation
- Implement `project.wfl` parser (natural-language flat file)
- Implement `wfl create project` interactive wizard (following the `.wflcfg` wizard pattern from `src/wfl_config/wizard.rs`)
- Implement `wfl create project called <name>` non-interactive mode
- Generate `project.wfl`, `.wflcfg`, and `src/main.wfl`

### Phase 2: Lock File and Local Resolution
- Implement `project.lock` generation and parsing
- Implement deterministic dependency resolver (local packages only)
- Implement WFLHASH checksum computation and verification

### Phase 3: Dependency Management Commands
- Implement `wfl add <package>` with version constraint parsing
- Implement `wfl remove <package>`
- Implement `wfl update` and `wfl update <package>`
- Implement local package cache at `~/.wfl/packages/`

### Phase 4: Registry
- Design and implement wflhub.org registry API
- Implement `wfl share` (publish to registry)
- Implement `wfl search <keywords>` and `wfl info <package>`
- Implement `wfl login` / `wfl logout` (browser-based OAuth)

### Phase 5: Compiler Integration
- Implement `package:` protocol in the WFL module loader
- Wire `wfl build` to resolve dependencies before compilation
- Ensure `load module from "package:name"` and `include from "package:name"` work

### Phase 6: Security and Workspaces
- Implement `wfl check security` (advisory database integration)
- Implement `wfl check compatibility` (API diff analysis)
- Implement `workspace.wfl` parsing and multi-package builds
- Implement permission declaration validation and user prompts

---

## 16. Glossary

These terms are defined in plain language for developers who may be new to package management. *(P9 — Accessibility for Beginners, P16 — Gradual Learning Curve)*

| Term | Definition |
|---|---|
| **Package** | A reusable collection of WFL code that other projects can depend on. A package has a name, a version, and a `project.wfl` manifest. |
| **Dependency** | A package that your project needs in order to work. Listed in `project.wfl` with `requires`. |
| **Dev dependency** | A dependency only needed during development (e.g., a testing framework). Marked with `for development`. |
| **Manifest** | The `project.wfl` file that describes your package — its name, version, dependencies, and permissions. |
| **Lock file** | The `project.lock` file that records the exact version of every dependency. Ensures everyone on the team uses the same versions. |
| **Registry** | A server that hosts published packages. The default registry is wflhub.org. |
| **Version constraint** | A rule that says which versions of a dependency are acceptable. For example, `26.1 or newer` means any version from 26.1.0 onward. |
| **Workspace** | A collection of related packages in a single repository, defined by a `workspace.wfl` file. |
| **Cache** | A local copy of downloaded packages stored at `~/.wfl/packages/`. Prevents re-downloading packages you already have. |
| **Checksum** | A unique fingerprint computed from a package's contents using WFLHASH. Used to verify that a downloaded package has not been modified or corrupted. |
| **Sharing** | Publishing a package to the registry so other developers can use it. Done with `wfl share`. |
| **Permission** | A capability that a package declares it needs, such as `file-access` or `network-access`. The package manager asks for your approval before installing packages with special permissions. |
