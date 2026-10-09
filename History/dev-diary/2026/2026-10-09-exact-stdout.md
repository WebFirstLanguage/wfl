# Exact stdout output — 2026-10-09

Issue: [#704](https://github.com/WebFirstLanguage/wfl/issues/704).

The approved scope adds `call write_stdout with text`, also available as
`write_stdout of text`. It writes exact UTF-8 text without a separator or
terminating newline, flushes stdout before returning, and returns nothing.
Invalid argument counts/types and stdout failures raise ordinary runtime errors.

The existing thread-local capture buffers already store strings. The writer
appends directly to the innermost capture, preserving final partial lines and
the existing nesting, error cleanup and per-handler capture context. No parser
or keyword change is needed; the builtin uses only the explicit-call inventory
so a variable named `write_stdout` retains its `with` concatenation behavior.
`display`, `print` and file writes preserve their existing behavior.

Testing risk is R3 because concurrent capture and language compatibility are
affected. The test-only Red commit is `c6237b41`; the real CLI tests cover exact
bytes, empty/Unicode/control text, mixed output order, nested capture, cleanup
after child failure, validation, name compatibility, immediate flushing and a
closed stdout pipe. Static contracts and native validation have direct tests;
the existing concurrent capture tests now assert exact output including partial
writes and sibling output isolation. The gated WFL test checks the return value
and catchable dynamic argument failure, and the docs example produces `abcdef`.
The closed-pipe regression catches the I/O failure, writes the error to a
temporary marker file and exits successfully; an interpreter test checks
catchable arity errors when static checking is bypassed.

The initial local regression run found the missing builtin. Two capture fixtures
were corrected to avoid the reserved word `captured`. Subsequent runtime
verification uses the repository's approved GitHub Actions environments, per
the current contribution policy. The [test-only Actions run](https://github.com/WebFirstLanguage/wfl/actions/runs/37937122463/job/113842100263)
confirmed eight intended missing-builtin failures on the Red commit. Passing CI and final review evidence belongs
on the linked PR; this entry does not claim those gates have passed.
