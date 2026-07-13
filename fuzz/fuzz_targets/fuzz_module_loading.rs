#![no_main]
//! Fuzz target: the module-content handling pipeline (the static half of module
//! loading).
//!
//! When WFL loads a module (`include from` / `load module`), it lexes, parses,
//! analyzes, and type-checks the module's untrusted content, and keys behaviour
//! off the include/load-module detection helpers. This target drives that
//! content-handling pipeline on arbitrary bytes, end-to-end through the static
//! stages a freshly-loaded module AST is put through:
//!   1. **checked** lexing (`lex_wfl_with_positions_checked`) — the same
//!      budget-aware tokenizer the loader uses,
//!   2. parse,
//!   3. `program_has_includes` / `program_has_load_module` (the loader's
//!      dispatch predicates),
//!   4. include-aware semantic analysis (`Analyzer::analyze` runs
//!      `warn_undefined_callee_if_includes`, the #548/#580/#592 path),
//!   5. type checking (`TypeChecker::check_types`).
//!
//! **Scope / known gap (honest):** this does *not* run the Tokio interpreter, so
//! it does not exercise filesystem path resolution/canonicalization, bounded
//! reads, cross-file circular/import-depth enforcement, or module *execution* —
//! those live in the async `resolve_module_path` + interpreter loop and need a
//! harness that can drive the runtime on its large-stack thread. Driving the
//! real FS/async loader is tracked as follow-up in `fuzz/README.md`; this target
//! covers the untrusted-content parsing/analysis surface that runs on every
//! loaded module.
use libfuzzer_sys::fuzz_target;
use wfl::analyzer::{self, Analyzer};
use wfl::lexer::lex_wfl_with_positions_checked;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    // Checked lexer: with no ExecutionBudget installed it lexes fully; the point
    // is to exercise the same entry point the loader calls.
    let Ok(tokens) = lex_wfl_with_positions_checked(source) else {
        return;
    };
    let mut parser = Parser::new(&tokens);
    let Ok(program) = parser.parse() else {
        return;
    };
    let _ = analyzer::program_has_includes(&program);
    let _ = analyzer::program_has_load_module(&program);
    let mut analyzer = Analyzer::new();
    let _ = analyzer.analyze(&program);
    let mut type_checker = TypeChecker::new();
    let _ = type_checker.check_types(&program);
});
