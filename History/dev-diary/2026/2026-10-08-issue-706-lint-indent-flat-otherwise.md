# 2026-10-08 — LINT-INDENT on flat `otherwise check if` (#706)

## Symptom

A correctly indented, correctly running program:

```wfl
store x as 1
check if x is equal to 1:
    display "a"
otherwise check if x is equal to 2:
    display "b"
end check
```

used to get a `LINT-INDENT` warning on every remaining line. `--lint` exited 1
while execution and `--analyze` were clean. A 780-line file with ten 5-arm
ladders produced 776 false warnings.

The issue report attributed this to an exact-string dedent on `"otherwise:"`.
That matcher did exist in the old line scanner (`trimmed == "otherwise:"`, then
`ends_with(':')` to indent). It is no longer the implementation.

## Current cause

`IndentationRule` now uses the shared token `SourceLayout` scanner
(`src/linter/layout.rs`). After a check-owned `otherwise`, the scanner must
consume the following `check if` *without* opening a new `Check` block. Those
three words share the original terminator.

If that skip is missing, `otherwise` is treated as a same-level branch (so the
header itself is not warned), then `check` is a body header and opens a new
nest. The issue program then reports:

- line 5 `display "b"`: expected 8 spaces, found 4
- line 6 `end check`: expected 4 spaces, found 0

and every later line stays permanently offset. Comparison conditions such as
`x is equal to 2` are not special; they simply appear on the same physical line
as the `:` that used to fool the old scanner.

The skip is already present (`is_chained_check` advances past `otherwise check
if`). What #706 still lacked was the exact comparison-condition repro, a CLI
`--lint` exit-0 contract, a mis-indent still-flagged control, and docs that
still called the flat form nonexistent.

## Verification

Temporarily forcing the scanner to consume only `otherwise` reproduced the
desync on the issue program (library test failed with the two warnings above).
Restoring the three-token skip made the same test pass. `--lint` exits 0 on the
conforming file and 1 when the else-if body is at column 0.
