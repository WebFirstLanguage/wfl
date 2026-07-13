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
| `fuzz_pattern` | Pattern grammar + compiler + VM (ReDoS surface) | `create pattern` parse → `CompiledPattern::compile` → `find_all` |
| `fuzz_module_loading` | Module-content handling: include/load-module detection + include-aware analysis | `program_has_includes` / `program_has_load_module` / `Analyzer::analyze` |

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

# Run one target, seeding from the tracked corpus in fuzz/seeds/<target>.
# libFuzzer treats the first dir as the writable corpus and the rest as
# read-only seed inputs.
cargo +nightly fuzz run fuzz_lexer          fuzz/corpus/fuzz_lexer          fuzz/seeds/fuzz_lexer
cargo +nightly fuzz run fuzz_parser         fuzz/corpus/fuzz_parser         fuzz/seeds/fuzz_parser
cargo +nightly fuzz run fuzz_pattern        fuzz/corpus/fuzz_pattern        fuzz/seeds/fuzz_pattern
cargo +nightly fuzz run fuzz_module_loading fuzz/corpus/fuzz_module_loading fuzz/seeds/fuzz_module_loading

# Time-boxed run (the shape a CI/nightly job uses); -max_len bounds input size.
cargo +nightly fuzz run fuzz_parser -- -max_total_time=300 -max_len=65536
```

Crashes/hangs are written to `fuzz/artifacts/<target>/`; reproduce with:

```bash
cargo +nightly fuzz run fuzz_parser fuzz/artifacts/fuzz_parser/crash-<hash>
```

## Layout

```
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
