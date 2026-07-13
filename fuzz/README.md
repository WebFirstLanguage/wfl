# WFL fuzz targets

Coverage-guided fuzz targets for WFL's untrusted-input surfaces, built with
[`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) / libFuzzer. Established
under Phase 1 of the production-readiness plan
([issue #610](https://github.com/WebFirstLanguage/wfl/issues/610)) to satisfy the
mandatory gate *"Fuzz targets complete the agreed sustained run without an
unresolved crash or hang."*

## Targets

| Target | Surface under test | Entry points |
|---|---|---|
| `fuzz_lexer` | Tokenization | `lexer::lex_wfl_with_positions_checked` |
| `fuzz_parser` | Recursive-descent parser (with error recovery) | `lexer::lex_wfl_with_positions` → `Parser::parse` |
| `fuzz_pattern` | Pattern grammar + compiler + VM (ReDoS surface) | `pattern\0haystack` pair → `create pattern` parse → `CompiledPattern::compile` → `find_all(haystack)` |
| `fuzz_module_loading` | Module-content handling (static half of loading): checked lex → parse → include/load-module detection → include-aware analysis → type check | `lex_wfl_with_positions_checked` → `Parser::parse` → `program_has_includes`/`program_has_load_module` → `Analyzer::analyze` → `TypeChecker::check_types` |

Each target's invariant is the same: **no arbitrary input may panic, overflow
the stack, or hang.** Controlled `Err`/diagnostic results are expected outcomes,
not failures.

## Why this is a separate workspace

`fuzz/Cargo.toml` declares its own empty `[workspace]` and the repo root lists
`fuzz` under `[workspace] exclude`, so the stable-toolchain `cargo build` /
`cargo test` / `cargo clippy` at the root never descend into it. libFuzzer
targets need a **nightly** toolchain and the sanitizer runtime; keeping them out
of the default build means the mandatory CI checks stay on stable.

You can still *type-check* the targets on stable to catch API drift:

```bash
cargo check --manifest-path fuzz/Cargo.toml
```

## Running

Requires a nightly toolchain and `cargo-fuzz`:

```bash
rustup toolchain install nightly
cargo install cargo-fuzz

# List targets
cargo +nightly fuzz list

# The live corpus dir (fuzz/corpus/<target>) is gitignored and does NOT exist on
# a fresh clone, so seed it from the tracked seeds first. `cargo fuzz run` then
# uses fuzz/corpus/<target> as the writable corpus by default.
for t in fuzz_lexer fuzz_parser fuzz_pattern fuzz_module_loading; do
  mkdir -p "fuzz/corpus/$t"
  cp -n fuzz/seeds/$t/* "fuzz/corpus/$t/" 2>/dev/null || true
done

# Run one target (writable corpus defaults to fuzz/corpus/<target>).
# -timeout=10 flags any single input that takes >10s as a hang.
cargo +nightly fuzz run fuzz_parser -- -timeout=10 -max_len=65536

# Time-boxed run (the shape a CI/nightly job uses); bounds total time + input.
cargo +nightly fuzz run fuzz_parser -- -max_total_time=300 -timeout=10 -max_len=65536
```

Crashes/hangs are written to `fuzz/artifacts/<target>/`; reproduce with:

```bash
cargo +nightly fuzz run fuzz_parser fuzz/artifacts/fuzz_parser/crash-<hash>
```

## Layout

```text
fuzz/
  Cargo.toml            # standalone cargo-fuzz workspace
  fuzz_targets/*.rs     # one libFuzzer target per surface
  seeds/<target>/       # tracked seed inputs (committed)
  corpus/<target>/      # live/evolving corpus (gitignored)
  artifacts/<target>/   # crash reproducers (gitignored)
```

## Baseline & follow-up

- **Baseline (Phase 1):** targets established and type-checked; no sustained run
  recorded yet. The agreed sustained run and corpus retention are **Phase 3**
  work (issue #610, *"Establish continuous or scheduled fuzzing with corpus
  retention"*). Record the duration and any findings in the score history when
  that run completes.
- **Out of scope here:** filesystem- and Tokio-backed module *resolution*
  (`resolve_module_path`, cross-file circular-include detection) and full
  interpreter execution need a harness that can drive the async runtime; they
  are deliberately not fuzzed by `fuzz_module_loading`, which targets the pure
  content-handling path. Tracked as follow-up for the Phase 3 fuzzing workstream.
