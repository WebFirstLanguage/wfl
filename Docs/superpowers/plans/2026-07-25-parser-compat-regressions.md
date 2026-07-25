# Parser Compatibility Regressions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore legacy file-write, zero-argument `flush` action, display-fold, and JavaScript transpilation behavior without weakening the new HTTP-stream syntax.

**Architecture:** Keep ambiguous syntax represented explicitly in the AST and defer branch selection to the existing analyzer/typechecker/runtime or transpiler boundary. Restrict trailing postfix parsing to postfix forms that were historically owned by the preceding expression, using source adjacency where whitespace distinguishes a display fold from an index.

**Tech Stack:** Rust 2024, WFL lexer/parser/analyzer/interpreter, JavaScript transpiler, Cargo integration tests, WFL end-to-end programs.

## Global Constraints

- Risk class is R3 because this changes language backward compatibility and streaming syntax.
- Preserve every previously valid WFL program; do not broaden streaming dispatch over legacy expression syntax.
- Follow Red → Green → Refactor → Broaden → Record with observed failing tests before production edits.
- Add coverage at the parser/transpiler layer and through the real `wfl` binary.

---

### Task 1: Record compatibility regressions as failing tests

**Files:**
- Modify: `tests/write_web_postfix_test.rs`
- Modify: `tests/transpiler_test.rs`
- Create: `TestPrograms/parser_streaming_compat_regression.wfl`

**Interfaces:**
- Consumes: public lexer/parser, `wfl::transpiler::JavaScriptTranspiler`, and the built `wfl` binary.
- Produces: regression tests for classic `write line|chunk` continuations, exact `flush` action fallback, display folding, and transpiler fallback.

- [ ] Add parser/runtime tests for `write line with "!"`, `write line at 0`, and `write line[0]` targeting a file.
- [ ] Add runtime tests proving zero-argument action `flush` still runs for `flush (expr)` and same-line `flush call ...`.
- [ ] Add parser/runtime tests proving `display alice.name [1, 2]` and `display alice.name 5` remain display folds.
- [ ] Add a transpiler test proving `write line note to "f.txt"` uses its classic file-write fallback.
- [ ] Add a real WFL end-to-end program that asserts the compatible runtime results.
- [ ] Run the focused tests and retain their expected failures as Red evidence.

### Task 2: Restore classic write ownership for bare marker continuations

**Files:**
- Modify: `src/parser/stmt/io.rs`
- Test: `tests/write_web_postfix_test.rs`

**Interfaces:**
- Consumes: the token immediately following an exact `line`/`chunk` contextual marker.
- Produces: `WriteToStatement` for legacy continuations and `StreamWriteStatement` for genuine stream operands.

- [ ] Extend the bare-marker guard to recognize legacy continuation starters (`with`, `at`, `[`, and `.`) rather than only `to`.
- [ ] Run the focused write parser/runtime tests and confirm Green.

### Task 3: Preserve exact `flush` action fallback

**Files:**
- Modify: `src/parser/stmt/web.rs`
- Test: `tests/write_web_postfix_test.rs`

**Interfaces:**
- Consumes: exact `flush` dispatch followed by a parenthesized or explicit-call stream target.
- Produces: `FlushStreamStatement.action_fallback` containing the legacy zero-argument `flush` action expression.

- [ ] Build the exact-token legacy fallback from the `flush` binding while independently parsing the stream target.
- [ ] Run the focused flush parser/runtime tests and confirm Green.

### Task 4: Stop postfix parsing from stealing display folds

**Files:**
- Modify: `src/parser/expr/primary.rs`
- Test: `tests/write_web_postfix_test.rs`

**Interfaces:**
- Consumes: a property/method expression followed by possible postfix tokens.
- Produces: adjacent `property[index]` chaining while leaving whitespace-separated list and scalar expressions to the display fold.

- [ ] Require bracket adjacency after a property/method expression before treating `[` as its postfix.
- [ ] Do not reinterpret a trailing bare integer as direct indexing after property/method access.
- [ ] Run the focused display and existing property-index tests and confirm Green.

### Task 5: Transpile ambiguous classic writes through their fallback

**Files:**
- Modify: `src/transpiler/javascript.rs`
- Test: `tests/transpiler_test.rs`

**Interfaces:**
- Consumes: `StreamWriteStatement` with `fallback_content: Some`.
- Produces: the same `WFL.file.write(...)` JavaScript emitted for the legacy file-write reading; unambiguous streaming statements remain unsupported.

- [ ] Split the transpiler match arm so ambiguous writes use `fallback_content` and `target`.
- [ ] Keep unambiguous HTTP stream writes as clear transpilation errors.
- [ ] Run transpiler tests and confirm Green.

### Task 6: Broaden verification and record evidence

**Files:**
- Modify: `Dev diary/2026-07-25-parser-streaming-compat-regressions.md`

**Interfaces:**
- Consumes: final implementation and test output.
- Produces: durable R3 acceptance criteria, Red/Green commands, and residual-risk record.

- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run focused parser, transpiler, analyzer/typechecker, and property-index suites.
- [ ] Run `cargo test --all`.
- [ ] Run `cargo build --release` followed by `scripts/run_integration_tests.ps1`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Record exact evidence and any residual risk in the Dev Diary.
