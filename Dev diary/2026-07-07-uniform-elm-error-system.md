# Uniform Elm-Style Error System

**Date:** 2026-07-07

## What Changed

WFL now renders **every** error and warning through one uniform, Elm-inspired
format across the whole toolchain — lexer, parser, analyzer, type checker,
interpreter, pattern engine, and linter, in both the CLI and the REPL, plus the
LSP. This delivers on Fundamental #4 ("Clear and Actionable Error Reporting").

Before, output went through `codespan-reporting`'s default Rust-compiler style
(`error[CODE]: msg` / `┌─ file:line:col` / `^`), suggestions were attached by
fragile `message.contains("…")` matching, ~10 copy-pasted report blocks in
`main.rs` printed inconsistent headers, lint warnings had no source frame, and the
lexer bypassed the diagnostic system entirely with raw `eprintln!`.

Now a diagnostic reads:

```
✕ Type Error   line 5, column 8

    Expected: Number
    Found:    Text ("hello")

The expression
    age plus "hello"
cannot add a Number and Text.

💡 Try converting first:
    age plus 5
    — or —
    string of age with "hello"
```

## How It Works

- **Enriched model** (`src/diagnostics/mod.rs`): `WflDiagnostic` gained optional
  `kind` (`DiagnosticKind`), `type_info` (`TypeMismatch`), `explanation`, and
  `suggestion` (`Suggestion`) fields, plus a `DiagnosticHint` for carrying
  guidance. All additive — existing constructors and the LSP mapping are
  unchanged.
- **Custom renderer** (`src/diagnostics/render.rs`): `render_diagnostic` draws the
  title, Expected/Found block, source frame, explanation, and 💡 suggestion to any
  `termcolor::WriteColor` sink, so the CLI (stderr) and the REPL (an in-memory
  buffer) share one code path.
- **Single entry point**: `report_to_stderr` / `report_all` / `render_to_string`
  on `DiagnosticReporter` replaced the scattered per-stage report blocks and their
  bespoke headers.
- **Per-stage kinds**: parse → Parse Error, type → Type Error (with structured
  Expected/Found + conversion suggestion), undefined names → Name Error, runtime →
  Runtime Error, lint → Lint Warning (now with a source frame), lexer → Syntax
  Error (routed via a new `LexError` type and `lex_wfl_with_positions_reporting`;
  the frozen `lex_wfl_with_positions` still serves its ~297 callers).
- **Color correctness**: `ColorChoice::Always` was replaced with real TTY
  detection + `NO_COLOR`, and a `--color=auto|always|never` flag was added.
  (termcolor's `Auto` only checks env vars, not TTY status, so we detect it
  ourselves.)
- **LSP**: `convert_to_lsp_diagnostic` folds the new fields into
  `related_information` so editor diagnostics carry the same guidance.

## Cleanup

Removed stale `src/linter/mod.rs.orig`/`.rej`, the unused `RuleSeverity` enum, the
linter's `println!` progress noise, and the now-vestigial `to_codespan_diagnostic`
method + `From<Severity>` impl. `codespan-reporting` is retained only for
`SimpleFiles` (source storage) and its termcolor re-export.

## Notes / Follow-ups

- WFL's type checker is lenient on some expressions (e.g. `age plus "hello"`
  concatenates rather than erroring), so the flagship Expected/Found layout shows
  up whenever a real static `TypeError` carries typed `expected`/`found`.
- Parse/semantic/runtime converters still construct their (now structured or
  note-based) guidance in `DiagnosticReporter`; the `DiagnosticHint`/`apply_hint`
  infrastructure is in place to migrate that guidance to each error's origin
  later.
- Tests: added renderer snapshot tests in `src/diagnostics/render_tests.rs`
  (including the mockup reproduced verbatim), ANSI/`NO_COLOR`, and ASCII-fallback
  coverage. All existing unit + integration tests and `TestPrograms/` remain green.
