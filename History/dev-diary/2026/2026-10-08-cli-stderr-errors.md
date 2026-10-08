# CLI diagnostics on stderr

Issue #703 described three missing ways to report an intentional failure.
Since it was filed, `raise_error` gained catchable application messages and
`exit program with code` gained process status control. The remaining gap was
writing a clean diagnostic to stderr without causing a runtime error.

The new explicit-call `print_error` core function accepts text and writes it
to process stderr. It does not change control flow. A CLI can compose a message
with `with`, call `print_error`, then `exit program with code 1`. A library can
continue using `raise_error` when its caller must catch the failure.

The real-binary CLI regression was observed failing before implementation in
test-only commits `e4f6e4cb` and `8dd47987`: `print_error` was an undefined
action. It now checks exact stderr output, stdout, exit status, absence of a
debug report, the successful standalone call, and rejection of a dynamic
nontext message. The documentation example uses the same CLI program and
expects status 1.

On the release binary, that example exited 1 with empty stdout and exactly
`jshrink: Unclosed string at position: 42` plus one newline on stderr. After
rebasing onto main, the three focused CLI tests, full workspace Rust suite,
formatting, Clippy, and repository hygiene checks passed. The full documentation
validator passed all 39 examples. The release integration runner passed its
Rust tests and 165 WFL programs (24 skipped by the suite); the web flow passed
all three server tests.
