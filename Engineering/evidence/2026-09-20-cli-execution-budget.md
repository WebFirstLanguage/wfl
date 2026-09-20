# Finite invocation execution budgets

Risk class: **R3** (resource limits, CLI compatibility and subprocess lifecycle).

The Scriptorium full Linux suite reached the existing CLI's 300-second execution
deadline on official WFL 26.9.14 after about 34 suites, then subsequent child
launches failed against the same expired budget. The configured runner timeout
could not extend this limit. The same published runtime completed the Windows
consumer suite at `07e8adcb` in about 160 seconds: 41 suites passed and its one
deliberately failing gate test correctly made the invocation exit 1. That local
consumer log is `target/full-official-windows-red.log` in Scriptorium. This is a
real capacity failure, not a
reason to hide suite failures or exempt a batch runner with `main loop`.
The consumer Red is [Scriptorium CI35507634634](https://github.com/WebFirstLanguage/Scriptorium/actions/runs/35507634634)
at `07e8adcb`, using the published runtime rather than a development build.

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
configuration, parent-expiration cleanup, and unchanged main-loop HTTP timeout,
operation limits and subprocess permissions.
The cleanup test first proves the child can write under its own longer budget.
`tests/fixtures/cli_budget/long-run.test.wfl` is a real 305-second ordinary run,
invoked explicitly by both integration jobs with a 330-second budget.
It belongs outside the recursive 30-second program sweep and must not be skipped.

## Green verification and review

Red commit `84cb272c` preserves the original executable capability tests before
the product implementation. Green adds the CLI parsing and budget selection in
`src/main.rs`; no interpreter or configuration policy implementation changes.
Further cases cover disallowed CLI modes and unchanged operation/permission
limits. While bringing the fixtures to Green, the shared-file fixture gained
the required `execute file at` syntax and the child assertion fixture gained
`--test`. These fixture corrections are not product defects or semantic Red
evidence. The later resource-policy cases also fail against the official runtime
because the flag is absent, rather than because the old policies are incorrect.

The release candidate reports version 26.9.15 and has SHA-256
`1185f150c8214d982a27431d4b88b94f7f20d6ce6c718161c35120deb817650f`.
Local Windows verification at the reviewed source:

- `cargo build --release --locked`: passed.
- Four fast WFL suites: arguments 6/6, deadlines 5/5, server policy 1/1,
  resource policy 3/3; all 15 passed.
- `wfl --execution-timeout 330 --test tests/fixtures/cli_budget/long-run.test.wfl`:
  passed 1/1 after the actual 305-second wait and checkpoint. This run was not
  shortened, skipped or made lifetime-exempt.
- Existing Rust compatibility suites: 139/139 passed across execution budget,
  CLI help/version, lint CLI, bind-address CLI, outbound HTTP budget, subprocess,
  subprocess cleanup and subprocess security.
- `cargo fmt --all -- --check` and
  `cargo clippy --all-targets --all-features -- -D warnings`: passed.
- Existing documentation validation: 36/36 passed.
- Static repository hygiene after staging all new files, and `git diff --check`:
  passed.
- `cargo check --locked --manifest-path fuzz/Cargo.toml -j 1 --verbose`:
  passed, including `fuzz_frontend`, in 5m22s. This is compilation evidence,
  not a sustained fuzz campaign.

The first default-parallel fuzz compilation failed while compiling unchanged
`crypto-common 0.2.2`: rustc exited 1 without an explaining compiler diagnostic.
The successful single-worker diagnostic build establishes that the candidate
fuzz workspace compiles, but does not identify the initial failure's cause.
Both logs are retained and the unresolved observation is tracked in
[issue #740](https://github.com/WebFirstLanguage/wfl/issues/740). No dependency,
lockfile or source change was made between those builds; the issue does not
waive any check. The host used stable x86_64-pc-windows-msvc Rust 1.98.1.

Logs remain under ignored `target/reports/cli-budget/`. An independent reviewer
checked flag operands and placement, CLI modes and child argv, shared deadlines,
configured HTTP/operation/permission policies and cancellation cleanup. The
review's two fixture findings were fixed: the owned child now has its own longer
budget plus a positive control, and the child assertion program is launched in
test mode. Final source and fixture review identified no blocking finding.

Exact-head CI must still complete, including both real five-minute boundary
steps and the ordinary Linux/Windows program sweeps. No merge or release is
authorized by this evidence record itself.
