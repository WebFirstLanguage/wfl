# Finite invocation execution budgets

Risk class: **R3** (resource limits, CLI compatibility and subprocess lifecycle).

The Scriptorium full Linux suite reached the existing CLI's 300-second execution
deadline on official WFL 26.9.14 after about 34 suites, then subsequent child
launches failed against the same expired budget. The configured runner timeout
could not extend this limit. The same published source completed the Windows
consumer suite in about 160 seconds. This is a real capacity failure, not a
reason to hide suite failures or exempt a batch runner with `main loop`.

The published source `8d82d785ea59300834de1c48b04a7ba0e187a1cd`
unconditionally applies `timeout_seconds.min(300)` in `src/main.rs`.
Configuration accepts larger values, but there is no existing file-entrypoint
override. The July 10 bind-address diary records retaining the historical cap
for compatibility. The new option leaves that default/config behavior intact.

## Red evidence

New scenarios, fixtures and drivers are WFL. Before implementation, the three
`TestPrograms/cli_budget/*.test.wfl` programs were run with the official Windows
26.9.14 executable (SHA-256
`da109d5926f6af4a2f24c45150140764c406c055aef2dd7e43e45b85278f1dfe`).
The argument suite failed 5/5 expectations, deadlines passed its unchanged
configuration baseline and failed the four override cases, and server-policy
failed its one override case. All programs parsed and ran as WFL tests. The new
flag is unavailable on that release; these are CLI capability failures, not
claims that the old runtime implemented the new flag incorrectly. The existing
consumer 300-second failure establishes the underlying semantic limitation.

The initial draft had an invalid reserved variable name and expected the wrong
timeout wording. Those fixture mistakes were corrected before the recorded Red
run; they are not counted as product failures. Logs are retained only under
ignored `target/reports/cli-budget/red-*.log`.

## Acceptance and design

`--execution-timeout SECONDS`, before the source filename, selects one finite
invocation deadline from 1 through 31,536,000 whole seconds. It changes only
`BudgetLimits.max_duration` before the shared budget starts. It does not change
`WflConfig`, default/config caps, request/stream limits, explicit subprocess wait
timeouts, subprocess permissions, operation/depth/size ceilings, or separately
launched WFL child configuration. Ordinary foreground operations continue to
share the invocation deadline; their duration therefore grows with an explicit
larger invocation budget. Included and executed files share that same deadline.

Fast WFL suites cover operands, routing, test mode, cumulative deadlines, child
configuration, parent-expiration cleanup, and unchanged main-loop HTTP timeout.
The cleanup test first proves the child can write under its own longer budget.
`tests/fixtures/cli_budget/long-run.test.wfl` is a real 305-second ordinary run,
to be invoked explicitly by both integration jobs with a 330-second budget.
It belongs outside the recursive 30-second program sweep and must not be skipped.

Green verification, independent review and exact-head CI will be recorded before
the change is submitted as ready to merge. No merge or release is authorized by
this evidence record itself.
