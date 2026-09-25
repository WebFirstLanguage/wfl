# Included action and assertion warning fixes

Logbie-web's seven WFL suites produced 316 `ANALYZE-SEMANTIC` warnings for actions supplied by `include from` and 126 `ANALYZE-UNUSED` warnings. The actions existed and the tests passed. The CLI analyzed only the entry file before executing includes, while the unused-variable visitor skipped `expect` statements entirely.

The CLI now scans literal top-level includes transitively when undefined-action warnings exist. It parses each included file under the run's source and import limits, recognizes action definitions by name, and removes only matching warnings. Dynamic paths and unknown names retain their warnings. The unused-variable visitor now marks the assertion subject and expression-valued expected operands as uses. These changes do not execute includes during analysis or alter runtime include behavior.

Three focused real-binary regressions were written first. Before the fix, the literal-include and assertion-use cases failed; the dynamic-include case passed. After the fix, all three pass. Against the seven Logbie-web suites, the patched WFL binary removes all 316 false undefined-action warnings and 90 false unused-variable warnings. The other 36 unused names were ignored return values in three Logbie-web test files; those bindings were replaced with direct calls, leaving zero analyzer warnings across all seven suites. The account suite's file-backed SQLite case requires a writable fixture directory outside the sandbox.

The red test-only ancestor is commit bb103dae.

## Review follow-up

Review of the first version found four gaps, each reproduced by a failing test before the fix (red test-only commit af4fbd0):

- **Ordering.** The first version dropped a warning whenever any literal include defined the name, even for a call that runs before the include. Top-level statements run in order, and WFL defines an action only when its `define` statement runs, so the CLI now keeps a warning unless the include has run by the time of the call. For a call in sequential code, the include must be an earlier top-level statement. For a call in an action, container, or handler body, the include must come before any later statement that can run code (another include counts). The analyzer records which top-level statement holds each `Undefined action` warning, and the CLI compares statement positions. It does not compare line numbers, because the parser records some blocks at their closing line.
- **Operation budget.** The scan charged the run's shared operation budget, so a program whose `max_operations` covered its own work could fail before it started. The scan now has its own allowance with the same ceiling and the run's remaining time. Running out stops the scan and keeps the remaining warnings. A run deadline or cancellation during the scan exits with status 2 and an `Error:` line, like other front-end budget breaches. Before this change it appeared as an analyzer finding.
- **Assertion operands.** `expect items.length ...` and `expect other.size(wanted) ...` now count `items`, `other`, and `wanted` as used. The shared unused-variable walker now visits property-access receivers and method-call receivers and arguments.
- **Test strength.** The CLI tests now assert the exit status along with the diagnostic text.

Known limit: the check assumes that the included file supplying a name does not call back into a main-file action that uses the name before the file defines it. That case still loses its warning.
