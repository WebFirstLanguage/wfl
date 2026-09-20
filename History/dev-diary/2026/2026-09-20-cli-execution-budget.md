# A finite execution budget for long batch invocations

The complete Scriptorium WFL suite exceeded the CLI's historical 300-second
cap on Linux. Increasing `.wflcfg` could not help because the cap was applied
after loading configuration. Making the test runner a lifetime-exempt server
loop would hide the boundary instead of supporting legitimate batch work.

`wfl --execution-timeout 1200 batch.wfl` now grants one explicit finite deadline
to that invocation. Only `BudgetLimits.max_duration` changes; the interpreter
still receives its original capped configuration. This separation preserves
HTTP and streaming timeouts, main-loop process waits, explicit process-wait
deadlines, permissions and memory/operation/depth limits. Ordinary foreground
operations remain part of the same invocation budget, and executed files share
the same budget rather than resetting it. Child WFL processes retain their own
configuration unless the caller explicitly passes an override to them.

The option accepts whole seconds from one through one year, rejects duplicate
or malformed operands, and must precede the source filename. Normal scripts and
test-mode scripts use the same option. Configuration maintenance, editor launch
and environment dumps refuse an irrelevant override instead of ignoring it.

All new scenarios and drivers are WFL. Fast tests cover CLI routing, the existing
configuration baseline, longer/shorter deadlines, child cleanup and isolation,
and unchanged HTTP limits. A separate 305-second WFL case in both integration
jobs proves execution past the old ceiling with an explicit 330-second bound.
The existing outer CI bound remains finite. See the matching engineering
evidence record for Red/Green results, review and exact-head CI.

Review exposed two policy boundaries that needed stronger tests: a shorter
invocation override must not shorten main-loop HTTP or streamed responses, and
a duration wait must observe the deadline even when it is the final statement.
The budget now retains its original operation duration separately when the CLI
overrides invocation lifetime, preserving existing embedded custom budgets.
Duration waits check eagerly and poll passive sleeping/receiving in bounded
intervals. WebSocket handlers are awaited normally so errors run their cleanup
and unwind interpreter state; the runtime does not cancel a handler future to
interrupt a duration wait. Dump modes reject the misplaced new option while
preserving their handling of unrelated trailing arguments.
