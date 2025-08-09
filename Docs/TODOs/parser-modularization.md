# TODOs - Parser Modularization Follow-ups

These items were noted during/after the modular refactor of the WebFirst Language (WFL) parser. Capturing here for future handling.

- Verify and tighten handling of stray "end X" tokens within nested constructs:
  - We added defensive skipping of unexpected `end <keyword>` pairs inside action bodies to preserve old behavior and avoid desynchronization.
  - Revisit this approach for a more principled synchronization strategy in `util::synchronize()` that can be shared across contexts (not just actions).

- Re-validate control-flow edge cases inside action definitions:
  - Specifically, `count ... end count` within `define action ... end action`, and early returns (`give back` / `return`) inside nested loops.
  - Ensure token consumption invariants hold and that inner constructs fully consume their own `end X` tokens.

- Complete doc polish for parser module responsibilities:
  - Confirm Docs/technical/wfl-parser.md references the final modules and that examples reflect the orchestrator-only `mod.rs`.
  - Add a brief section describing the lookahead pattern used to avoid borrow checker issues and unintended consumption.

- Consolidate shared helpers:
  - Audit helpers across `statements.rs`, `expressions.rs`, and `container_parser.rs` to ensure anything reusable is in `util.rs`.
  - Consider general utilities like identifier sequence parsing and argument-list parsing to reduce duplication.

- Testing additions:
  - Add parser unit tests specifically targeting nested structures and stray `end` recovery cases.
  - Add token-consumption progress tests to prevent infinite-loop regressions.

- Performance/memory:
  - Optionally run `scripts/run_heaptrack.sh` on larger programs and document peak usage targets in Docs/technical if not already present.

- Known Actions:
  - Double-check all action-call resolution paths remain confined to `expressions.rs` and that no new call sites were added outside expressions.
