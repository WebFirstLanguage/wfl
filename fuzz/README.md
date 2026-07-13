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
| `fuzz_frontend` | Compiler **frontend** on arbitrary source: checked lex → parse → include/load-module detection → analyze → type check | `lex_wfl_with_positions_checked` → `Parser::parse` → `program_has_includes`/`program_has_load_module` → `Analyzer::analyze` → `TypeChecker::check_types` |

Each target's invariant is the same: **no arbitrary input may panic, overflow
the stack, or hang.** Controlled `Err`/diagnostic results are expected outcomes,
not failures.

### Not yet covered: module *loading* (open Phase 1 follow-up)

> **Naming note:** an earlier revision named the frontend target
> `fuzz_module_loading`; it was **renamed to `fuzz_frontend`** because it does
> not actually fuzz module loading. If you are cross-referencing older PR text
> that says `fuzz_module_loading`, it means `fuzz_frontend`.

Phase 1 ([#610](https://github.com/WebFirstLanguage/wfl/issues/610)) lists a
**module-loading** fuzz surface. There is no such target yet. `fuzz_frontend`
fuzzes the static pipeline a module's *content* passes through, but it does
**not** invoke the interpreter's `LoadModuleStatement` / `IncludeStatement`
paths, so it never reaches filesystem path resolution/canonicalization, bounded
reads, cross-file circular/import-depth enforcement, parent-scope construction,
or module execution.

Fuzzing the *real* loader means driving the async interpreter against on-disk
modules. Doing that **safely** is the hard part: executing fuzzer-generated WFL
would also exercise subprocess spawning, networking, the web server, and
filesystem writes, so a naive interpreter-in-libFuzzer harness is unsafe. A
proper module-loading target needs a sandboxed harness (benign module bodies +
fuzzed include-graph structure/paths, or an execution-disabled load path). This
is tracked as remaining Phase 1 work — the module-loading fuzz item is **not**
complete.

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
for t in fuzz_lexer fuzz_parser fuzz_pattern fuzz_frontend; do
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

- **Baseline (Phase 1):** three of the four required surfaces (lexer, parser,
  pattern engine) plus the compiler frontend are established and type-checked; no
  sustained run recorded yet. The agreed sustained run and corpus retention are
  **Phase 3** work (issue #610, *"Establish continuous or scheduled fuzzing with
  corpus retention"*). Record the duration and any findings in the score history
  when that run completes.
- **Open Phase 1 item — module-loading fuzzing:** not done (see *"Not yet
  covered"* above). Filesystem- and Tokio-backed module *resolution*
  (`resolve_module_path`, cross-file circular-include detection, import-depth) and
  module execution need a *sandboxed* async harness — full interpreter execution
  of fuzzer WFL is unsafe. Until that lands, the module-loading fuzz surface
  remains uncovered and the Phase 1 fuzz-target task is only partially complete.
