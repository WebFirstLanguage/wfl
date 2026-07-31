# Experiment: pattern kitchen sink

Exploratory pattern-matching program (moved from `syntax_test/pattern.wfl` by
the repository hygiene migration) that throws every pattern construct at the
engine in one file.

- Owner: Brad
- Status: parked — awaiting coverage-extraction decision
- Issue: https://github.com/WebFirstLanguage/wfl/issues/669
- Created: 2026-07-31 (moved from `syntax_test/pattern.wfl`, authored earlier)
- Review-by: 2026-10-30
- Exit criteria: fold any coverage not already provided by
  `TestPrograms/patterns_comprehensive.wfl` and
  `TestPrograms/patterns_working_comprehensive.wfl` into asserted tests, then
  archive or delete the prototype and remove this directory.

Experiments are non-gating: nothing in CI executes this program.
