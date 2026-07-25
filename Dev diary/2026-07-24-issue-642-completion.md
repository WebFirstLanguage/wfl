# Dev Diary - 2026-07-24: issue #642 completion pass

Issue [#642](https://github.com/WebFirstLanguage/wfl/issues/642) re-reviewed
PR #641 at `b25aed57` and identified five P1 groups plus missing R3
lifecycle evidence. The completion pass fixes the remaining behavior, hardens
the tests so they prove the promised boundaries, and records unrelated
platform defects discovered by the full presubmit.

## Risk and compatibility

- **Risk class:** R3.
- **Issue triggers:** concurrency, cancellation, HTTP lifecycle, streaming,
  resource ownership, arbitrary duration configuration, and backward
  compatibility of existing WFL expressions.
- **Additional gate trigger:** untrusted archive paths and path containment.
  This was a pre-existing Windows portability defect found by the full gate,
  not a sixth issue #642 requirement.
- **Public contract:** no existing WFL syntax is intentionally removed or
  changed. The fixes restore classic `write` and `flush` expression behavior
  and preserve typed timeout/cancellation outcomes.
- **External state:** none. Rollback is a source revert; there is no data
  migration or persistent-state recovery step.

## Selected design

The completion pass keeps the existing architecture and hardens each boundary:

1. **Concurrent server:** handler state records sticky request acceptance.
   A centralized classifier separates request-local outcomes from structural
   pre-request failures. Pending-response ownership is checked before
   expression evaluation and consumed atomically only at response commit.
2. **Outbound streams:** each handle keeps a stable slot and a first-wins
   `watch` terminal reason. Expiry records a `Timeout` tombstone, wakes an
   active reader, drops a parked upstream body, and refuses reinsertion. EOF,
   error, and close abort the per-stream reaper.
3. **Ambiguous writes:** type checking follows the branch selected by a
   concrete target and checks every viable branch for a gradual target.
   Typechecker-local and inherited container context participates in
   definedness and property typing.
4. **Merged operands:** one seeded-expression continuation parser is shared by
   write operands, response content type, response headers, and legacy flush
   candidates. Response-clause boundaries propagate through recursive primary
   wrappers and explicit-call argument lists.
5. **Flush compatibility:** the parser records the original merged binding as
   `legacy_binding` metadata before split/find/replace rewrites. Analyzer,
   static unused-variable analysis, typechecker, and runtime consult the same
   metadata, then dispatch the full fallback AST through ordinary
   expression-statement semantics.

This removes the reported races and restores grammar parity without changing
public `ErrorKind` variants or adding a second request-lease subsystem.

## Acceptance criteria to regression coverage

| Issue requirement | Regression evidence |
|---|---|
| Request-local failures cannot stop the server | `concurrent_disconnect_paths_burst_test`: two serialized 256-client waves per disconnect path (512 disconnects total), exact checkpoint/ack synchronization, handler-start ordinal 768 (the initial 256 plus one replacement for every consumed disconnect result), then `/ping`; every socket operation and join is bounded |
| Owned missing pending entry is cancellation; duplicate remains an error | `concurrent_handler_classification_tests::missing_pending_entry_is_cancelled_only_while_the_handler_owns_it` |
| Repeated finite request waits survive | 257 direct classifier observations plus a real server using 1 ms waits until handler-start ordinal 512 |
| Expiry/read ownership is atomic and preserves `Timeout` | active-read real-socket test, unread-expiry test, and deterministic `take -> expire -> ready result -> put` unit ordering |
| Stream EOF is stable | final unterminated line, then exactly one EOF `nothing`, then the documented closed-handle result |
| Reaper resources remain bounded | retained-runtime live-reaper counter across rapid close, clean EOF, truncated-body error, and opened-but-unread expiry |
| Extreme duration cannot panic or disable the cap | deadline unit test proves `u64::MAX` becomes a finite one-year cap and diagnostics use that effective duration |
| Concrete and gradual write branches are sound | one-sided leads in both directions, property roots, handler/action scopes, direct and inherited container properties, wrapped file/stream/list payloads, gradual definedness, and gradual payload tests |
| Merged operands match ordinary expression grammar | 3 x 7 parser matrix plus clause boundaries in concatenation, `at` indexes, nested `of` calls, builtins, unary operands, explicit `call ... with ...` arguments, and `file exists at <indexed expression>` |
| Full flush fallback is preserved | non-callable, overload, nested postfix, binary continuation, `of` calls, split/find/replace rewrites, invalid container-property types, and unused-variable accounting for `legacy_binding` and fallback operands |
| Backpressure test proves the intended path | client confirms the 200 head, stays connected without reading, then asserts the exact typed stall timeout and lower bound |
| Dropped pending request sends the explicit 500 | exact dequeue/drop/release synchronization followed by exact status, content type, body, prompt completion, and clean interpreter/server joins |

## Red evidence observed

The following focused regressions failed before their matching implementation
changes:

- GitHub Actions run `30106107011` and the local focused run:
  `http_stream_test::test_next_line_returns_final_unterminated_line` returned
  `Unknown or already-closed stream handle 'httpstream1'` instead of EOF
  `nothing`.
- Active hard-expiry cancellation produced `ErrorKind::General` with a closed
  stream message instead of `ErrorKind::Timeout`.
- Reading an unread expired stream produced an unknown-handle General error
  instead of a typed timeout.
- Container method `write line value to "C:/tmp/out"` falsely reported
  `Variable 'line value' is not defined`.
- Wrapped write operands skipped undefined names in concrete-file,
  concrete-stream, and gradual-target branches.
- Unmerged response content type `"text/" with subtype` stopped parsing at
  `with`.
- Response clauses were swallowed by concatenation operands, `at` indexes, and
  recursively nested `of`/builtin/unary expressions.
- `call render with value and headers h` swallowed `headers h` as a second
  explicit-call argument.
- `file exists at paths at kind and headers h` absorbed the headers clause
  into the nested index/Boolean-And expression.
- `flush cache plus 1` left `plus` dangling, and `flush cache[0][0]` selected
  the short `cache` root at runtime.
- Split/find/replace flush rewrites discarded the original legacy binding, and
  an invalid live container property was not rejected by the typechecker.
- Static unused-variable analysis reported both `flush cache` and `dead`; only
  `dead` should have been unused.
- The original lifecycle tests could finish without proving the post-
  disconnect checkpoint, exact drop point, bounded joins, or admission beyond
  the breaker threshold. Deterministic synchronization was added before the
  production behavior was accepted as Green.

The original server-breaker defect is anchored to issue head `b25aed57`. Only
GitHub Actions run `30106107011` is retained policy-compliant CI Red evidence.
The other Reds were observed locally but are not preserved as Red commits or
durable CI artifacts.

A pre-existing zero-byte `.git/objects/maintenance.lock` (dated
2026-07-18 03:36 local time) prevented Git object writes; its owning process
was not established. Consequently there is no committed Red-to-Green ancestry
for the local regressions, including the existing archive reproduction test.
This is a testing-policy handoff limitation and must not be overstated as
formal Red evidence.

## Unrelated full-gate defects repaired

These changes are not issue #642 acceptance items, but each blocked or weakened
the repository's required verification:

- `file_io_performance_test::test_directory_listing_performance` recursively
  scanned the repository and `target` (about 65,000 files), exceeded its
  10-second bound at 11.28 seconds, and left 30 fixtures. It now uses an
  auto-cleaned temporary directory; focused Green was 1/1 in 0.02 seconds.
- On Windows, `Path::is_absolute()` did not classify the portable archive entry
  `/etc/shadow` as absolute. The existing containment guard still rejected it
  as escaping the destination, but with the wrong classification and message.
  `Path::has_root()` now performs portable rooted-path rejection; the
  `wflpkg` security suite is 31/31.
- `execute_file_test` reserved port 58123. It now uses `free_tcp_port()` to
  avoid unrelated local collisions.
- Windows PowerShell 5.1 rejected inherited environments containing identical
  `Path` and `PATH` keys before `Start-Process` could run. Both official
  scripts now canonicalize only identical duplicates and fail closed on
  conflicting values. The integration runner also retains the child process
  handle before a timed wait so a real exit code is available.
- `subprocess_comprehensive.wfl` treated shell-only `echo` as a Windows
  executable. It now uses `cargo --version`, an existing runner prerequisite,
  explicitly waits after output capture so shutdown is orphan-free, and uses a
  repo-owned blocking WFL helper to prove the child is running before kill and
  absent afterward. The helper carries the first-line `CI-SKIP` directive used
  by both platform runners so it is never treated as a standalone test.

## Green evidence

Focused final results:

- Language-focused combined run: 68/68.
- `write_web_postfix_test`: 21/21.
- Static-analyzer focused units: 15/15.
- Parser units: 110/110.
- Strengthened disconnect binary: 4/4 in about 11.5 seconds; the handler-entry
  barrier case also passed focused 1/1.
- Directory-performance fixture: 1/1 in 0.02 seconds.
- `cargo test -p wflpkg --test security_tests --verbose`: 31/31.
- Final subprocess fixture: exit 0 with a live-child kill assertion and no
  orphan warning.

Final-tree gates:

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `git diff --check` | pass |
| `cargo clippy --all-targets --all-features -- -D warnings` | pass |
| `cargo build --release` | pass |
| `cargo test --all --verbose --jobs 2` | pass; core 627 passed / 6 ignored, all workspace integration packages passed, WFL doctests 28 passed / 11 ignored |
| `scripts/run_integration_tests.ps1 -TestOnly` | pass; Rust integration binaries passed and TestPrograms finished 110 passed / 0 failed / 24 explicit skips |
| `scripts/run_web_tests.ps1` | pass; 2/2 HTTP tests, TLS script case explicitly skipped because OpenSSL was unavailable |
| Git Bash syntax + first-line skip probe | pass; the Unix runner parses and recognizes the helper's `CI-SKIP` directive |
| `python scripts/validate_docs_examples.py --ci --force` | pass; 18/18 examples across validation layers |

The first unbounded `cargo test --all --verbose` attempt hit a pre-test Windows
linker fan-out failure, `LNK1104: cannot open msvcrt.lib`. The library was
present and readable, and the exact failed target linked immediately
afterward. `--jobs 2` preserved the complete test selection while bounding
concurrent linkers.

The Cargo cache also reported a read-only last-use database in this sandbox.
That warning did not affect dependency resolution, compilation, or test
selection.

## Residual risk and recovery

- Each disconnect path covers 512 clients in two 256-client waves, with at
  most 256 simultaneous handlers. This proves every disconnected result is
  consumed before the post-check while staying within the configured admission
  bound.
- The repeated finite-timeout proof is separate and uses handler-start ordinal
  512.
- `0` continues to disable the outbound absolute cap. Positive values above
  one year use the documented one-year effective cap.
- The script-level TLS case was not run because OpenSSL was unavailable, but
  the Rust TLS integration suite passed 8/8 in the workspace test gate.
- No deployment or external state changed. Reverting this source/test set is
  the rollback; forward repair is preferred if a platform timing or socket-
  limit issue appears.
