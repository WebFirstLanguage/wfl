# 2026-10-08 — Close the pattern kitchen-sink experiment (#669)

`experiments/syntax/` was parked at the repository-hygiene migration with a
`Review-by: 2026-10-30` date. The hygiene checker fails CI the day after that
date, so the experiment had to be promoted, extracted, or archived.

The prototype does not run on current `main`: it stores a variable named
`output`, which is now a reserved keyword. It is not a polished example — most
of the file is unrelated list/map/date scratch after a handful of pattern
probes, and those probes report results as `display` text rather than gated
assertions. `examples/pattern_examples.wfl` already covers the beginner email
and phone surface, so the file was not promoted.

`TestPrograms/patterns_comprehensive.wfl` and
`patterns_working_comprehensive.wfl` already exercise named patterns, literal
alternatives, `exactly N digit(s)`, and dashed phone forms. The kitchen sink's
remaining unique constructs are now gated:

- Parser tests for `any letter` / `any digit`, `one or more of (...)`, plural
  `2 to 6 letters` / `3 to 4 digits`, and optional `matches pattern`
- `TestPrograms/patterns/parenthesized_any_charset.test.wfl` — the emailme
  pattern, including that `"._-"` is a three-character literal, not a class
- `TestPrograms/patterns/parenthesized_digit_runs.test.wfl` — `one or more of
  (digit)` dashed phones
- `TestPrograms/patterns/plural_range_id.test.wfl` — plural `3 to 4 digits`
  ranges, including the original `1234-567-8901` substring match

The original `pattern_kitchen_sink.wfl` is retained byte-for-byte at
`Archive/legacy-programs/syntax/pattern_kitchen_sink.wfl`. The experiment
directory is gone.
