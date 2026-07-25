# Dev Diary — 2026-07-25: Parser streaming compatibility regressions

This change repairs four backward-compatibility regressions introduced by the
response-streaming grammar: classic `write line`/`write chunk` expressions with
continuations, exact zero-argument actions named `flush`, display folds after
property access, and JavaScript transpilation of ambiguous classic file writes.

## Risk and compatibility contract

- **Risk class:** R3.
- **Triggers:** backward-compatible parsing, streaming dispatch, file writes,
  action dispatch, and silent output changes.
- **Compatibility contract:** existing classic programs keep their pre-streaming
  interpretation while unambiguous response-stream operations remain available.
- **External state:** none beyond temporary files created and removed by tests.

## Acceptance criteria and coverage

| Acceptance criterion | Regression coverage |
|---|---|
| A variable literally named `line` or `chunk` remains a classic file-write expression when followed by an operator, `at`, property access, or indexing | `tests/write_web_postfix_test.rs`; `TestPrograms/parser_streaming_compat_regression.test.wfl` |
| Direct integer stream targets such as `write line 0` remain streaming syntax | `tests/write_web_postfix_test.rs` and existing streaming parser tests |
| `flush (expr)` and same-line `flush call ...` invoke an exact zero-argument action named `flush` when one exists | `tests/flush_action_backcompat_test.rs`; `TestPrograms/parser_streaming_compat_regression.test.wfl` |
| Spaced values after property access remain display folds, while adjacent postfix indexing remains indexing | `tests/write_web_postfix_test.rs`; existing `tests/property_index_access_test.rs` |
| JavaScript transpilation uses the valid classic fallback for ambiguous `write line ... to ...`, but still rejects an unambiguous response-stream write | `tests/transpiler_test.rs` |

## Red → Green record

The tests were added and run before the corresponding production changes.
The observed Red failures were:

- `cargo test --test flush_action_backcompat_test`: both new tests failed
  because the parser/typechecker treated the target as a response stream
  (`Expected a server response stream`).
- `cargo test --test write_web_postfix_test`: six new regressions failed,
  reproducing the reported stream-target error, undefined `at`, bracket
  misparse, display parse error, and runtime text-indexing error.
- `cargo test --test transpiler_test ambiguous_write_line_uses_classic_file_fallback`:
  the transpiler rejected the statement as unsupported streaming.
- The standalone WFL E2E program failed on the same write and flush dispatch
  errors.
- Additional direct-binary tests were observed Red for `line + ...` and direct
  integer-target discrimination before their parser changes.

The final Green focused command was:

```text
cargo test --test write_web_postfix_test --test flush_action_backcompat_test --test transpiler_test --test property_index_access_test --test http_server_streaming_test
```

It passed all 123 focused tests. The standalone E2E command
`target\debug\wfl.exe --test TestPrograms\parser_streaming_compat_regression.test.wfl`
passed 4/4 tests.

This working-tree session records the Red observations but does not claim a
test-only Red commit ancestor; preserving commit-level Red ancestry remains a
maintainer integration responsibility if these changes are committed.

## Implementation

The write parser now classifies exact bare `line`/`chunk` markers followed by a
classic expression continuation as classic file writes. For the still
ambiguous direct-integer form, it keeps a span-matched classic fallback while
preserving the response-stream interpretation.

Exact `flush` forms with a parenthesized target or explicit same-line call now
carry the same exact-binding action fallback as the merged-identifier form.
The existing analyzer/runtime binding check decides between that legacy action
and response-stream flushing.

Property-origin postfix parsing now requires source adjacency for brackets and
does not consume a spaced integer as an index. Direct and chained adjacent
property indexes continue to work. The JavaScript transpiler emits the classic
file-write fallback when the parser supplied one and continues rejecting
unambiguous streaming writes.

## Verification

Completed successfully:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all --jobs 2` (complete workspace and doctests)
- `cargo build --release`
- Focused 123-test parser, streaming, property, and transpiler command
- Standalone WFL E2E: 4 passed, 0 failed

The official integration wrapper's split layer passed 11/11. Its unconstrained
parallel `cargo test --test '*'` then failed during compilation with Windows
error 1455 (paging file too small); no product test failed. The equivalent
complete workspace suite passed with compiler parallelism bounded to two jobs.
The wrapper also emits an existing read-only Cargo cache warning on this host.

## Residual risk

The discrimination is deliberately narrow: only exact marker/action bindings
receive compatibility fallbacks, and adjacent postfix syntax remains available.
CI should rerun the official integration and platform matrix on the final
committed candidate. No coverage percentage is claimed.
