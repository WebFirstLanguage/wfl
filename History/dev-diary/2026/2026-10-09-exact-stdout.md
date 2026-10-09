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
`display` and `print` retain their formatting; file writes are unchanged.

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

Review identified synchronous stdout backpressure: an open pipe whose consumer
stops reading blocked the interpreter thread, including unrelated handlers and
cooperative timeouts. Brad explicitly requested fixing it before review. The
corrected test-only ancestor `7f859ebe` reproduced six intended failures in the
[Actions run](https://github.com/WebFirstLanguage/wfl/actions/runs/37947338000/job/113877172085):
stalled sibling requests/queued handlers and a timeout unable to interrupt output.
Earlier test-only revisions exposed invalid marker-file fixtures, which were
corrected before establishing this Red evidence.

All three WFL output forms now use one process-wide writer and a queue with one
pending slot. Admission yields before formatting or copying a payload; each
caller awaits its own write/flush result. Capture decisions and value formatting
stay on the interpreter thread. Only plain text crosses to the writer. The four
server startup notices use the same writer but retain their uncaptured stdout
destination. Native fn-pointer calls made directly by embedders remain synchronous;
WFL interpreter dispatch routes the output natives asynchronously.

The writer uses a dedicated detached thread rather than Tokio's blocking pool,
and duplicates pipe/file stdout without holding Rust's global stdout lock. This
lets the CLI/runtime exit while a physical pipe write is stalled. Windows console
output retains Rust's UTF-8 conversion. Interpreter waits check the shared budget
for deadlines/cancellation. Cancelled queued messages are skipped; an active OS
write may continue after cancellation and can leave partial output. An embedded
process retains this single worker/descriptor until process exit; cancellation
does not promise to terminate the OS write or let subsequent stdout overtake it.

The real CLI/pipe/TCP regressions cover sibling progress, resumption/order for
`write_stdout`, `print` and `display`, reader disconnect/error catchability,
shutdown with active/pending output, and deadline interruption with no later
file write. Controlled writer unit tests cover bounded admission, live
cancellation/resumption and flush-error delivery. The shared scheduling finding
is tracked in [#784](https://github.com/WebFirstLanguage/wfl/issues/784).

Brad confirmed in the Codex chat that Yomi does not exist and explicitly
requested removal of that named-review requirement. The governance suite,
current contributor guides and canonical PR checklist now require independent
review without naming Yomi. Required GitHub approval, current-revision CI,
handled bot feedback and maintainer merge authority continue to apply.

The initial local regression run found the missing builtin. Two capture fixtures
were corrected to avoid the reserved word `captured`. Subsequent runtime
verification uses the repository's approved GitHub Actions environments, per
the current contribution policy. The [test-only Actions run](https://github.com/WebFirstLanguage/wfl/actions/runs/37937122463/job/113842100263)
confirmed eight intended missing-builtin failures on the Red commit. Passing CI and final review evidence belongs
on the linked PR; this entry does not claim those gates have passed.
