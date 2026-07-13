#![no_main]
//! Fuzz target: the compiler **frontend** (lexer → parser → analyzer → type
//! checker) on arbitrary source.
//!
//! This drives the static pipeline every program — including the *content* of a
//! module — passes through before execution:
//!   1. **checked** lexing (`lex_wfl_with_positions_checked`),
//!   2. parse,
//!   3. include/load-module detection predicates (`program_has_includes` /
//!      `program_has_load_module`),
//!   4. include-aware semantic analysis (`Analyzer::analyze`),
//!   5. type checking (`TypeChecker::check_types`).
//!
//! Invariant: no arbitrary input may panic, overflow the stack, or hang the
//! frontend. Controlled `Err`/diagnostic results are expected.
//!
//! ## This is NOT a module-loading fuzz target
//!
//! It does **not** invoke the interpreter's `LoadModuleStatement` /
//! `IncludeStatement` paths, so it never reaches filesystem path
//! resolution/canonicalization, bounded reads, cross-file circular/import-depth
//! enforcement, parent-scope construction, or module execution. Fuzzing the real
//! loader requires driving the async interpreter against on-disk modules — and
//! doing that *safely* is non-trivial, because executing fuzzer-generated WFL
//! would also exercise subprocess spawning, networking, the web server, and
//! filesystem writes. That is tracked as follow-up in `fuzz/README.md`; the
//! Phase 1 "module loading" fuzz surface is therefore **not** covered here.
use libfuzzer_sys::fuzz_target;
use wfl::analyzer::{self, Analyzer};
use wfl::lexer::lex_wfl_with_positions_checked;
use wfl::parser::Parser;
use wfl::typechecker::TypeChecker;

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
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
