# 2026-10-08 — Close the Nexus prototype experiment (#668)

`experiments/nexus/` was parked at the repository-hygiene migration with a
`Review-by: 2026-10-30` date. The hygiene checker fails CI the day after that
date, so the experiment had to be promoted, extracted, or archived.

The prototype still runs end-to-end on current `main` (every logged probe
printed `PASS`). It is not a polished example: it writes `nexus.log` and
scratch files next to the program and reports results as log text rather than
gated assertions. `examples/` already covers the same beginner surface, so it
was not promoted.

The useful micro-probes were already asserted under `TestPrograms/nexus/`.
Three remaining unique loop forms from `nexus.wfl` were folded in as
`describe`/`expect` programs:

- `nested_loop_exit.wfl` — `exit loop` leaves every enclosing loop (the
  counterpart of `nested_loop_control.wfl`, which pins `break`)
- `forever_break.wfl` — `break` stops a `repeat forever`
- `repeat_until.wfl` — body-first `repeat until` sums 1 through 5

The original `nexus.wfl` is retained byte-for-byte at
`Archive/legacy-programs/nexus/nexus.wfl`. The experiment directory is gone.
