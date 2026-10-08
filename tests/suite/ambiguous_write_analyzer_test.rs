//! Analyzer coverage for the ambiguous `write line|chunk ... to <target>` shared
//! continuation across call/pattern shapes (maintainer re-review, P1).
//!
//! The two readings of an ambiguous write differ only at the leftmost leaf; every
//! other sub-expression is shared continuation. When the continuation desugars to
//! a call/pattern shape (`starts with`, `contains pattern`, ...), the analyzer must
//! still walk the shared operands so an undefined name there is reported at analysis
//! time instead of surfacing only at runtime — without falsely rejecting either
//! valid reading of the lead.

use wfl::analyzer::Analyzer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

fn analyze(code: &str) -> Result<(), String> {
    let tokens = lex_wfl_with_positions(code);
    let program = Parser::new(&tokens).parse().expect("parse");
    Analyzer::new()
        .analyze(&program)
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}

#[test]
fn undefined_name_in_a_starts_with_continuation_is_reported() {
    // Both leads are defined (`greeting` for the stream reading, `line greeting` for
    // the classic file reading), so the ONLY undefined name is the shared
    // `starts with` operand. It must be reported rather than deferred to runtime.
    let code = "store greeting as \"hello\"\n\
                store line greeting as \"world\"\n\
                write line greeting starts with missing_operand to \"/tmp/wfl_analyzer_out\"";
    let err = analyze(code)
        .expect_err("an undefined name in the shared `starts with` continuation must be reported");
    assert!(
        err.contains("missing_operand"),
        "expected the undefined shared-continuation name to be reported, got: {err}"
    );
}

#[test]
fn defined_name_in_a_starts_with_continuation_is_not_a_false_positive() {
    // Same shape, but the shared operand is defined: neither reading is broken, so
    // analysis must pass (the parallel walk must not over-report).
    let code = "store greeting as \"hello\"\n\
                store line greeting as \"world\"\n\
                store suffix as \"lo\"\n\
                write line greeting starts with suffix to \"/tmp/wfl_analyzer_out\"";
    assert!(
        analyze(code).is_ok(),
        "a fully-defined ambiguous write must analyze cleanly: {:?}",
        analyze(code).err()
    );
}
