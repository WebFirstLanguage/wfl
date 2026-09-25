# Included action and assertion warning fixes

Logbie-web's seven WFL suites produced 316 `ANALYZE-SEMANTIC` warnings for actions supplied by `include from` and 126 `ANALYZE-UNUSED` warnings. The actions existed and the tests passed. The CLI analyzed only the entry file before executing includes, while the unused-variable visitor skipped `expect` statements entirely.

The CLI now scans literal top-level includes transitively when undefined-action warnings exist. It parses each included file under the run's source and import limits, recognizes action definitions by name, and removes only matching warnings. Dynamic paths and unknown names retain their warnings. The unused-variable visitor now marks the assertion subject and expression-valued expected operands as uses. These changes do not execute includes during analysis or alter runtime include behavior.

Three focused real-binary regressions were written first. Before the fix, the literal-include and assertion-use cases failed; the dynamic-include case passed. After the fix, all three pass. Against the seven Logbie-web suites, the patched WFL binary removes all 316 false undefined-action warnings and 90 false unused-variable warnings. The other 36 unused names were ignored return values in three Logbie-web test files; those bindings were replaced with direct calls, leaving zero analyzer warnings across all seven suites. The account suite's file-backed SQLite case requires a writable fixture directory outside the sandbox.

The red test-only ancestor is commit bb103dae.
