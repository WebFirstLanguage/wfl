#![no_main]
//! Fuzz target: the module-loading static surface.
//!
//! When WFL loads a module (`include from` / `load module`), it lexes, parses,
//! and analyzes the module's untrusted content, and keys behaviour off the
//! include/load-module detection helpers. This target drives exactly that
//! content-handling path on arbitrary bytes:
//!   1. lex + parse the bytes as a module,
//!   2. `program_has_includes` / `program_has_load_module` (the loader's
//!      dispatch predicates),
//!   3. `Analyzer::analyze`, which runs the include-aware name resolution
//!      (`warn_undefined_callee_if_includes`, the #548/#580/#592 path) that a
//!      freshly-loaded module AST is put through.
//!
//! Filesystem- and async-backed module *resolution* (`resolve_module_path`,
//! circular-include detection across real files) is intentionally out of scope
//! here — it belongs to a harness that can drive the Tokio interpreter — and is
//! documented as follow-up work in fuzz/README.md.
use libfuzzer_sys::fuzz_target;
use wfl::analyzer::{self, Analyzer};
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let Ok(program) = parser.parse() else {
        return;
    };
    let _ = analyzer::program_has_includes(&program);
    let _ = analyzer::program_has_load_module(&program);
    let mut analyzer = Analyzer::new();
    let _ = analyzer.analyze(&program);
});
