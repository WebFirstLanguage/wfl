# Authentication PR review follow-up

The PR review exposed a request-projection gap: an executed page received the
flattened header map but not the transport's ambiguity flag. A real HTTP test
reproduced a duplicate Cookie request being accepted after delegation. The
projection now forwards the boolean unchanged, supplies `no` for older
caller-created contexts, and rejects malformed values before running the page.
Tests cover normal requests, repeated Cookie and CSRF headers, invalid header
encoding, legacy contexts, and recovery after denied requests. The initial
test-only revision is `df7d1db8`.

Adding native names to the parser's legacy `name with arguments` inventory also
changed how existing scalar variables were parsed. Source-local binding
inference cannot preserve imports, injected interpreter values, later REPL
input, or assignments in branches that never execute. The legacy grammar is
therefore kept separate from the additive builtin registry. New helpers use
the already documented `name of arguments` or explicit `call name with
arguments` forms. Existing legacy calls retain their original grammar. The
final test-only compatibility revision `3df2230b` has eight expected failures
and three passing legacy controls before the fix.

The process-wide configured-hash admission and worker limits are intentional:
allocating the same memory allowance per embedded interpreter would multiply
the host's memory ceiling. Documentation now states that embedded programs
share capacity and receive no tenant-fairness guarantee. Existing cancellation
tests exercise a second interpreter encountering occupied admission and then
recovering. Native authentication, password policy, and proxy helpers gained
Rustdoc explaining their contracts.

CI's repository-hygiene failure came from the newly committed audit report at
an unapproved root path. It was moved byte-for-byte into `Engineering/evidence/`;
the report's historical findings were not rewritten.

The Claude review action separately returned an immediate error without
printing its underlying message. Its workflow remains unchanged because the
action validates workflow identity before starting the review. The bounded
`scripts/report_claude_failure.py` utility can summarize a retained execution
file using fixed error categories and typed metadata; it never prints the
transcript and does not change a failing gate into a pass. Synthetic tests
verify redaction, malformed data, and input bounds. The account-side cause
cannot be inferred from the masked job log alone.
