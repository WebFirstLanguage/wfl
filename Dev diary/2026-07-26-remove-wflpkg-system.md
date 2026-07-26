# 2026-07-26 — Removing the `wflpkg` package manager

## Why

WFL is heading into its first release candidate. The `wflpkg` package manager —
a ~7,700-line crate, twelve `wfl` subcommands, a `package:` import protocol, and
seven design documents — is being rethought from scratch.

The deciding argument was irreversibility. Most of what ships in an RC can be
revised in the next release. A package manager cannot: manifest formats,
lockfile formats, archive formats, and above all the registry trust root are
things other people build against the moment they exist. Shipping a
half-formed one and then redesigning it would mean either breaking every early
package or carrying a design nobody wanted. Withdrawing it before the RC costs
nothing, because nothing depends on it yet.

The docs had already drifted ahead of this decision: `Docs/guides/faq.md` and
`Docs/01-introduction/key-features.md` both told users WFL has no package
manager while the code shipped one. This change makes the code agree with the
docs rather than the other way around.

## What was removed

**Implementation.** The whole `crates/wflpkg` crate: manifest and lockfile
parsers, the version-constraint resolver, the `.wflpkg` tar.gz archive format,
the `wflhash:v2:` integrity transcript, the download cache, the `wflhub.org`
registry client with its credential store, the permissions model, and the
standalone `wflpkg` binary. With it went five dependencies the rest of the tree
never used — `rpassword`, `flate2`, `tar`, `ignore`, and unix `libc` — plus
`reqwest`'s `multipart` feature.

**CLI.** The positional-subcommand dispatch block in `src/main.rs` (`create`,
`add`, `remove`, `update`, `build`, `run`, `share`, `search`, `info`, `login`,
`logout`, `check`), the `DEFAULT_REGISTRY` constant, the
`parse_create_project_args` helper, and the `PACKAGE MANAGEMENT` section of
`wfl --help`.

**Language surface.** The `package:` prefix in
`Interpreter::resolve_module_path`, along with `resolve_package_path` and
`find_project_root`. `load module from "package:my-lib"` is now an ordinary
relative path and fails as one.

**Docs.** The `### Package System (V4)` block in
`Docs/04-advanced-features/modules.md`, and the eight `[Unreleased] Security`
CHANGELOG bullets describing package publishing, registry credentials, and
archive integrity — all of which described code that will never ship a release.

## What was deliberately kept

**The module system.** `load module from "path.wfl"`, `include from "path.wfl"`,
and `export` are a general file-based feature that `package:` was bolted onto at
runtime. The parser (`src/parser/stmt/module.rs`) treats the path string as
opaque, so nothing above the interpreter needed to change. Two regression tests
pin this.

**The design documents.** Moved with `git mv` to `Docs/Archive/wflpkg/` so the
redesign starts with the prior art rather than a blank page, and given a README
that says in its first line that nothing in the folder describes shipping
behavior. Archived aspirational specs are exactly the kind of thing that gets
mistaken for documentation later.

`wflhub_language_gaps_prd.md` needed the most thought. It is nominally a
registry document, but what it actually specifies is a list of WFL language
capabilities — HTTP header access, response streaming — that a registry
*happened* to need. Several have since been implemented independently
(`tests/header_access_runtime_test.rs`, `tests/http_server_streaming_test.rs`).
The archive README calls this out so the language wishlist isn't assumed dead
along with the package manager.

**The historical record.** Dev diary entries and the PR-641 red-chronology doc
that mention `wflpkg` were left untouched. They are records of work as it
happened; editing them to hide a since-removed subsystem would falsify the audit
trail.

## The one backward-compatibility break

`wfl run <file>.wfl` and `wfl test <file>.wfl` lived *inside* the package
subcommand block — they stripped the subcommand and fell through to normal file
handling. Removing the block removed them.

This was raised with the Maintainer before implementation and the removal was
confirmed. The mitigating facts: neither alias appeared in `wfl --help`, in
`CLAUDE.md`'s CLI list, or anywhere in `Docs/`; the documented spellings
`wfl <file.wfl>` and `wfl --test <file.wfl>` are unchanged and verified working.
Post-removal, `wfl run x.wfl` exits 1 with the same not-found error any missing
path produces — loud, not silent.

The `package:` protocol is a second nominal break, but not a practical one:
resolving it required a `packages/` tree that only the removed `wfl add` could
produce.

## Testing (R3)

Classified R3 — it touches backward compatibility, so `testing.md` §5 puts it in
the highest class regardless of how mechanical the diff looks.

Red first, in a test-only commit (`afe8c05`) that is an ancestor of the removal.
`tests/package_protocol_removed_test.rs`, 8 tests, of which 6 failed against the
old tree for the intended reasons and 2 were green regression guards:

| Test | Red behavior |
|---|---|
| `package_protocol_no_longer_resolves` | the package resolved and executed |
| `package_import_failure_mentions_no_package_manager` | no failure existed to inspect |
| `bare_package_prefix_is_not_special_cased` | `"requires a package name"` diagnostic |
| `package_subcommands_are_removed` | `wfl logout` exited 0 |
| `run_and_test_positional_aliases_are_removed` | `wfl run main.wfl` ran the program |
| `help_has_no_package_management_section` | `PACKAGE MANAGEMENT` in `--help` |
| `relative_module_paths_still_work` | green (guard) |
| `relative_load_module_still_works` | green (guard) |

One detail worth recording. The first draft of the package tests invoked the
script as a bare relative `main.wfl` and got a *different* failure than expected:
`Could not verify project directory ""`. The removed `find_project_root` walked
up from the source file's **parent**, and a relative argument gave it an empty
parent, so it bailed before ever consulting `packages/`. That would have been a
weak Red — the test would have passed for the wrong reason. Switching to an
absolute script path exercises the real resolver, which is what makes the Red
meaningful. The helper carries a comment explaining this.

Two negative assertions guard the removal specifically: the failure text must
not contain `project.wfl`, `wfl create project`, `wfl add`, `is not installed`,
or `packages directory` (their presence would mean a wflpkg error branch
survived), and `--help` must not mention `PACKAGE MANAGEMENT`, `wflhub`, or
publishing. `package_import_failure_mentions_no_package_manager` also asserts a
non-zero exit *before* checking the message, so it can never pass vacuously by
the import succeeding again.

### Layers run

| Layer | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `cargo test --workspace` | 130 test binaries, **0 failures** |
| `TestPrograms/` against `target/release/wfl` | **110 passed, 0 failed**, 24 skipped |
| `python scripts/validate_docs_examples.py` | 18/18 pass |
| `python scripts/test_docs_code_blocks.py` | ran clean (survey report, not a gate) |
| `cargo check --manifest-path fuzz/Cargo.toml` | clean; `fuzz/Cargo.lock` regenerated |
| Manual CLI | `--help`, `--version`, `--test`, program run, `wfl run` break |

### A note on the environment, not the change

`cargo test --workspace` and `scripts/run_integration_tests.sh` both died
mid-link with `No space left on device` / `ld terminated with signal 7 [Bus
error]`. This is the `target/` growth problem `CLAUDE.md` warns about: 114
integration-test binaries at `debuginfo=2` push `target/debug` to ~27 GB, past
this container's allowance.

The workaround was to run the suite with `CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_DEBUG=0`, which fits comfortably and changes nothing about
which tests run or how they assert — only the richness of panic backtraces. The
`TestPrograms/` gate was then run directly against the already-built release
binary, mirroring `run_test_programs()`'s skip and expected-fail lists exactly.
Both layers are genuinely green; neither was skipped or relaxed. CI, which has
room for the full-debuginfo build, runs them unmodified.

## Files touched

- **Deleted:** `crates/wflpkg/` (26 files)
- **Moved:** `wflpkg/*.md`, `wflpkg_prd.md`, `wflhub_language_gaps_prd.md` →
  `Docs/Archive/wflpkg/`
- **Code:** `Cargo.toml`, `Cargo.lock`, `fuzz/Cargo.lock`, `src/main.rs`,
  `src/interpreter/mod.rs`
- **Tests:** `tests/package_protocol_removed_test.rs` (new)
- **Docs:** `CHANGELOG.md`, `CLAUDE.md`, `AGENTS.md`, `GOVERNANCE.md`,
  `Docs/04-advanced-features/modules.md`, `Docs/Archive/wflpkg/README.md` (new)
- **Tooling:** `.github/workflows/ci.yml` (comment),
  `scripts/test_docs_code_blocks.py` (dead error-string classifiers)
