# Container Constructor Arguments Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let `create new K with 3 as k: ... end` pass positional arguments to `initialize`.

**Architecture:** Populate the existing `ContainerInstantiation.arguments` AST field. Reuse PR #785's receiver-aware runtime dispatch, add inherited static lookup, and expose the source form through the existing LSP server.

**Tech Stack:** Rust 2024, WFL parser/analyzer/type checker/interpreter, tower-lsp.

**Spec:** `Engineering/designs/2026-10-11-container-constructor-arguments-design.md`

## Global Constraints

- Base the work on `dev` after PR #785; Rust >= 1.94; add no dependency.
- Preserve `create new K as k: ... end` and do not invoke zero-argument `initialize` implicitly.
- Property initializers run before `initialize`; arguments are positional; no named arguments or container overloads.
- Keep every source behavior change additive and follow `testing.md` R3 red-to-green evidence, independent review, and end-to-end gates.
- Put user docs under `Docs/`, executable examples under `TestPrograms/`, and test output under `target/` or a temp directory.

## Review Focus

- A text argument containing `and` or `as` must remain one argument (Task 1 parser test).
- A `with` concatenation inside an argument must keep its expression value (Task 1 CLI test).
- A child override must win over an inherited `initialize` (Task 2 CLI test).
- A missing parent supplied through an include must not produce a false static missing-method error (Task 2 checker test).
- Signature help on an unfinished `with` clause must still identify the right container and active parameter (Task 3 LSP test).

---

### Task 1: Parse the constructor clause

**Files:** Modify `src/parser/stmt/containers.rs`; create `tests/container_constructor_arguments_test.rs` and `TestPrograms/containers/constructor_arguments.wfl`; inspect `src/fixer/source.rs` and `src/linter/layout.rs` only if round-trip tests fail.

**Interfaces:** Produce `Statement::ContainerInstantiation.arguments: Vec<Argument>` using the existing positional `Argument { name: None, value }` form. Consume `with <expression> [and <expression> ...]` before `as`; preserve the required property body.

- [ ] Write parser and real-binary tests: `create new K with 3 and 4 as k:` yields two arguments plus body initializers; a text literal containing `and`/`as` stays one argument; a concatenated argument evaluates correctly; no-argument syntax stays valid; named arguments and `K(3)` / `as k with 3` report the accepted form. Add a gated program with `describe`/`test` assertions that `initialize(3)` calling `this.configure()` leaves `value` equal to 6.
- [ ] Run `cargo test --test container_constructor_arguments_test` and the gated program against the pre-change binary. Record the intended parse failures, then commit the test-only red state.
- [ ] Update `parse_container_instantiation` to read the optional clause with an `as` boundary, reject named arguments, and pass arguments to the AST. Remove the always-empty argument vector from `parse_instantiation_body`; retain its property parsing. Add targeted diagnostics for the two wrong forms.
- [ ] Run `cargo test --test container_constructor_arguments_test` and the gated program. Check `wfl --lint` and `wfl --lint --fix --in-place` round trips on the new form; change fixer/linter code only if those checks fail.
- [ ] Commit the parser, tests, and gated program as the green step.

### Task 2: Check inherited constructor contracts

**Files:** Modify `src/typechecker/mod.rs`; extend `tests/container_constructor_arguments_test.rs`.

**Interfaces:** Resolve `initialize` from the named container outward through `ContainerInfo.extends`, with child override first. Distinguish a known missing action from an unresolved ancestor. Keep the existing argument type compatibility rules and diagnostic style.

- [ ] Add tests for inherited success, child override, missing action, wrong count, wrong type, and an unresolved included ancestor. Assert static diagnostics name both the container and `initialize`; assert real-binary output for inheritance and override.
- [ ] Run `cargo test --test container_constructor_arguments_test`; confirm inherited lookup and any missing failure-path cases fail for the intended reason. Commit the test-only red state.
- [ ] Replace the direct `container.methods.get("initialize")` check with a hierarchy lookup; validate the selected action's parameter count/types and report a missing action only for a fully known chain. Do not change interpreter dispatch.
- [ ] Re-run the focused test and `cargo test --test container_this_receiver_test`; commit the green step.

### Task 3: Expose constructors in the LSP

**Files:** Modify `wfl-lsp/src/lib.rs` and its existing unit tests there.

**Interfaces:** Add `get_constructor_signature_help(&self, document_text: &str, position: Position) -> Option<SignatureHelp>`, register `signature_help_provider`, and implement the `LanguageServer::signature_help` handler. Use compiler diagnostics; do not duplicate type-checking rules.

- [ ] Write failing LSP tests for a `create new` completion snippet, hover on the container type with the `initialize` parameters, signature help and active parameter in incomplete and multi-argument `with` clauses, inherited signatures, and a string containing `and`.
- [ ] Run `cargo test -p wfl-lsp --lib`; record the focused failures and commit the test-only red state.
- [ ] Extend completion/hover using the existing document parse and add focused token-based context detection for signature help, including incomplete source. Return `None` outside constructor clauses.
- [ ] Re-run `cargo test -p wfl-lsp --lib`; commit the green step.

### Task 4: Publish the language contract and verify the branch

**Files:** Update `Docs/04-advanced-features/containers-oop.md`, `Docs/reference/keyword-reference.md`, `Docs/reference/reserved-keywords.md`, `Docs/guides/migration-from-javascript.md`, `CHANGELOG.md`, and `History/dev-diary/2026/2026-10-11-container-constructors.md`. Add a docs-example fixture and manifest entry only if the documented snippet is not already covered by the gated program.

**Interfaces:** Document the exact Task 1 syntax and Task 2 diagnostics; correct the JavaScript guide's current claim that constructors are unavailable. No new reserved keyword.

- [ ] Write the documentation and Dev Diary entry. Validate every new WFL example with the available parse/analyze/typecheck/lint tools and `python scripts/validate_docs_examples.py`.
- [ ] Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --workspace`, `cargo check --locked --manifest-path fuzz/Cargo.toml`, and `python scripts/check_repo_hygiene.py --mode static`. Build release, then run `scripts/run_integration_tests.ps1` and `scripts/run_web_tests.ps1` on Windows.
- [ ] Record R3 acceptance criteria, red/green commits, commands, failures, and residual risk in the PR description. Commit docs, request independent review, and inspect required CI on the pushed PR head. Do not merge without the repository's review and branch gates.
