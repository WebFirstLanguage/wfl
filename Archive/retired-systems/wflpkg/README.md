# Archive: the `wflpkg` package manager (removed)

> **Nothing in this folder describes shipping WFL behavior.**
>
> The `wflpkg` package manager was **removed from the code base in July 2026**, during
> preparation for WFL's first release candidate. The system is being rethought from
> scratch. These documents are kept as historical prior art for that redesign — they
> are **not** a specification of anything WFL does today, and they are **not** a
> commitment to what the redesign will look like.

## What was removed

- The `crates/wflpkg` crate (~7,700 lines): manifest and lockfile parsing, the
  dependency resolver, the `.wflpkg` archive format, the download cache, the registry
  client and credential store, and the `wflpkg` binary.
- The positional `wfl` subcommands: `create`, `add`, `remove`, `update`, `build`,
  `run`, `share`, `search`, `info`, `login`, `logout`, and `check`.
- The `package:` import protocol — `load module from "package:my-lib"` no longer
  resolves through `packages/`. It is now an ordinary, unresolvable relative path.

The file-based module system (`load module from "path.wfl"`, `include from "path.wfl"`,
`export`) is **unaffected** and remains fully supported.

## What is in here

| Document | What it was |
|---|---|
| `wflpkgdesign.md` | Master design document for the package manager |
| `wflpkg-manifest-grammar-1.0.md` | Formal grammar for `project.wfl` manifests and `project.lock` |
| `wflpkg-brainstorm-results.md` | Multi-agent brainstorm output that fed the design |
| `wflpkg-open-decisions-resolved.md` | Log of resolved open design questions |
| `wflpkg-adr-001-binary-and-crate-structure.md` | ADR: separate `wflpkg` binary vs. `wfl` subcommands |
| `wflpkg_prd.md` | PRD for **WFLHub**, the proposed registry at `wflhub.org` |
| `wflhub_language_gaps_prd.md` | PRD for language features needed to build WFLHub *in WFL* |

## A note on `wflhub_language_gaps_prd.md`

That document is the one file here that is only partly about packaging. It specifies
general WFL language capabilities — HTTP header access, response streaming, and
similar — that happened to be motivated by building a registry. **Those language
requirements are not cancelled** by the removal of the package manager; several have
been implemented independently since. Read it as a language wishlist that happens to
have a registry-shaped rationale, not as a dead registry document.

## Governance

Per `GOVERNANCE.md` §8, package and registry design — including any future revival of
this work — remains a Maintainer-only decision area, because supply-chain and trust-root
choices are not reversible once published.
