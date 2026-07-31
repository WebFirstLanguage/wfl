# Tool Examples

Full WFL programs that do real work — kept as validated examples of practical
WFL (file I/O, directory walking, text assembly).

- `rust_loc_counter.wfl` — counts Rust lines of code across `src/`
  (WFL port of `scripts/metrics/generate_rust_loc_report.py`; superseded
  earlier variants are archived under `Archive/legacy-programs/tools/`).
- `combine_markdown.wfl` — combines `Docs/` markdown into one document
  (WFL port of `scripts/docs/combine_markdown.py`). Writes under
  `target/reports/combined/`.

Per `REPOSITORY_HYGIENE.md`, examples are current, documented, and validated —
and must write any output under `target/`, never into the source tree.
