# Dev Diary — 2026-07-22 — `write line/chunk` preserves the classic file write

## Context

The new streamed-response verbs `write line <value> to <out>` /
`write chunk <value> to <out>` share a surface with the pre-existing file write
`write <content> to <file>`. Because WFL identifiers can be space-separated, the
lexer merges `line payload` into a single `Identifier("line payload")` token. The
first cut of the parser always split such a token into a `line` marker plus a
value, so `write line payload to out` was unconditionally parsed as a stream
write — silently breaking any pre-existing program that wrote a variable literally
named `line payload` to a file. Review (Copilot, twice) flagged this as a
backward-compatibility break.

Backward compatibility is sacred, and the two readings genuinely cannot be told
apart at parse time: `write line <var> to <stream>` (the primary NDJSON use case)
and `write line <var> to <file>` (a variable named `line <var>`) both use a bare
variable. The only correct disambiguation is on the **runtime target type**.

## Fix — carry both readings, decide at runtime

- **AST/parser.** `StreamWriteStatement` gained `fallback_content:
  Option<Box<Expression>>`. For the ambiguous merged form the parser now records
  both the stream value (`Variable("payload")`) and the classic file-write
  content (`Variable("line payload")`). Unambiguous forms — a literal value, or a
  bare marker directly before `to` — set `None` (they were never valid file
  writes).
- **Interpreter.** `StreamWriteStatement` evaluates the target first. If it is a
  server response stream, it does the stream write. Otherwise, if a
  `fallback_content` is present, it performs the classic `write <fallback> to
  <target>` file write; if not, it errors as before.
- **Static analysis.** For the ambiguous form the live reading (and thus which
  variable must exist) is unknown until runtime, so semantic analysis defers
  definedness for it instead of rejecting the file-write reading. The
  unused-variable pass counts **both** candidate variables as used, so a variable
  named `line <ident>` written to a file is not falsely reported unused.

The other statements (`analyze`, typechecker, transpiler) already matched with
`..`; the transpiler still rejects streaming statements outright.

## Testing (Red → Green)

`tests/write_line_backcompat_test.rs`:

1. `test_write_line_multiword_variable_parses_with_fallback` — the merged form
   parses with `fallback_content: Some`; the literal form with `None`.
2. `test_write_multiword_line_variable_to_file_still_works` — runs the full
   analyzer + interpreter on `store line note as "…"` / `write line note to
   "<file>"`, asserting analysis accepts it and the file receives the **variable's
   value**, not the token `note`.

- **Red** (analyzer analyzing the stream value unconditionally): semantic analysis
  rejects the program with `Variable 'note' is not defined`.
- **Green**: analysis accepts it and the file contains the variable's value.

Risk class **R3** (backward compatibility). The existing streaming tests
(`write line "alpha" to out`, bare `write line to out`) continue to pass, so the
stream write and the classic file write both work through the shared surface.
