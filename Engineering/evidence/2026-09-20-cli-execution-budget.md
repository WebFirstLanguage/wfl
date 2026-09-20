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

## Review follow-up Red

Automated review of `68c46650` exposed three additional semantic cases, verified
locally before their fixes in `TestPrograms/cli_budget/review-boundaries.test.wfl`.
The WFL suite passed its existing main-loop wait exemption baseline and failed
three real expectations: an ordinary five-second wait outlived a one-second
deadline, a one-second invocation override shortened a ten-second server HTTP
policy, and dump modes accepted a misplaced timeout after the source filename.
The log is `target/reports/cli-budget/red-review-boundaries.log` (1/4 passed).
The wait gap predates the CLI option; it matters to an explicit finite deadline
and cannot be hidden by adding a later operation checkpoint to the assertion.
These findings supersede the earlier source-review verdict until corrected.
An additional last-statement wait case then confirmed successful exit after an
expired deadline without any following checkpoint; the expanded Red was 1/5
(`red-review-boundaries-final-wait.log`). Existing embedded-runtime tests also
establish that a manually supplied 250ms budget bounds main-loop HTTP requests,
so the remedy must distinguish a CLI invocation override without changing that
existing API contract.

Additional WFL streamed-response tests failed 0/2 on the frozen `68c46650`
binary, covering delayed headers and delayed body reads under config 10s / CLI
1s. Their first draft used a reserved parameter name and an ungrouped list
expression; those fixture parse errors were corrected before the recorded
semantic Red. An independently authored resource suite also failed its ordinary
wait and active WebSocket wait cases while passing the main-loop exemption
baseline (1/3). Red commits are `01416f4d`, `370ec22a`, `a5988f13`, and `028ef59e`.

The final remedy retains the original main-loop operation duration privately in
`ExecutionBudget`. Only `with_invocation_timeout` replaces run lifetime; existing
explicit-budget constructors retain their behavior. HTTP continues choosing the
minimum of that operation duration and the configured/remaining stream limit.
Duration waits now check eagerly before/after the wait and each WebSocket pump
iteration. Passive sleeping and receiving use at most 10ms intervals; handler
dispatch is awaited normally so WFL finally blocks and interpreter state unwind.
A reviewed intermediate outer-select design was rejected because dropping an
arbitrary running handler could skip that cleanup. Its successful checks are
not final-source acceptance evidence. Dump modes scan only for a misplaced new
option, preserving the handling of unrelated trailing arguments.

The resource regression was strengthened in `5d781336` after review found that
driver-side process reaping/closing could mask a surviving descendant. The
fixture now establishes writer readiness, releases it after the owner's
deadline, and observes the marker before any parent poll/reap. Exact-port
WebSocket rebind also happens before the driver's cleanup. The final frozen
old-binary result is 1/3: the late-write assertion fails with an actual pre-reap
write, the ordinary WebSocket wait outlives the deadline, and the exempt server
case passes. The unchanged final-source suite passes 3/3 in about eight seconds.
This covers an idle active WebSocket receiver; queued handler traffic remains
covered by the existing Rust WebSocket suite, not by this new WFL fixture.

The final reviewed no-drop candidate SHA-256 is
`c0619c544b09551ce7988f58a564f50ff04e48a5e94a6f903c08a141b911c516`.
At this source, all seven fast WFL suites pass 25/25. `cargo test --all --locked`
passes 2,414 tests, with 27 existing ignored tests across 175 result records;
this includes the unchanged custom-250ms main-loop HTTP contract, stream tests,
WebSocket tests and CLI compatibility. Release build, fmt, strict Clippy,
fuzz-target compilation, and documentation validation (36/36) pass. Local logs
are the `*-final*` and `wait-resources-green.log` files under the same ignored
report directory. Final source review found no remaining blocking issue.

The first remote run for `68c46650`,
[CI 35509162373](https://github.com/WebFirstLanguage/wfl/actions/runs/35509162373),
is not acceptance evidence: Windows integration failed in unchanged
`trusted_proxy_test::default_and_untrusted_peers_ignore_forged_forwarding` when
binding port 53684 reported Windows address-in-use error 10048. Its Linux
integration sibling was then cancelled, so neither long-boundary step ran.
Both ordinary program sweeps and the other completed jobs passed. The fixture
obtains a free port before spawning the server, which leaves a reuse window;
the observed log does not establish who occupied it. The final local full Rust
run passes that test unchanged. A new exact-head CI run must pass all gates;
the earlier failure is retained rather than treated as a waiver.

The final no-drop candidate also passes the actual 305-second WFL boundary
1/1 with `--execution-timeout 330`; `green-final-long.log` records the result.
