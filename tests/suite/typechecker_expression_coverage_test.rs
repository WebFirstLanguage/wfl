use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::typechecker::{TypeCheckError, TypeChecker};

fn typecheck(source: &str) -> Result<(), TypeCheckError> {
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("test program should parse");
    TypeChecker::new().check_types(&program)
}

fn diagnostics(source: &str) -> Vec<String> {
    typecheck(source)
        .expect_err("program should be rejected")
        .into_diagnostics()
        .into_iter()
        .map(|error| error.message)
        .collect()
}

#[test]
fn list_literal_elements_are_analyzed_and_typechecked() {
    let errors = diagnostics("store values as [missing_value]\n");
    assert!(
        errors
            .iter()
            .any(|message| message.contains("missing_value") && message.contains("not defined")),
        "undefined list elements must be analyzed: {errors:?}"
    );

    let errors = diagnostics("store values as [(1 minus \"text\")]\n");
    assert!(
        errors
            .iter()
            .any(|message| message.contains("Cannot perform Minus")),
        "invalid operations inside list elements must be typechecked: {errors:?}"
    );
}

#[test]
fn fixed_result_expressions_validate_and_visit_their_operands() {
    for source in [
        "store result as file exists at 1\n",
        "store result as directory exists at no\n",
        "store result as list files in 1\n",
        "store result as read binary from 1\n",
        "store byte_count as \"many\"\nstore result as read byte_count bytes from \"handle\"\n",
        "store result as file size of no\n",
        "store result as process 1 is running\n",
    ] {
        assert!(
            typecheck(source).is_err(),
            "a fixed return type must not hide an invalid operand: {source}"
        );
    }

    let errors = diagnostics("store result as file exists at (1 minus \"text\")\n");
    assert!(
        errors
            .iter()
            .any(|message| message.contains("Cannot perform Minus")),
        "operand expressions must be traversed before applying the fixed result type: {errors:?}"
    );
}

#[test]
fn pattern_find_uses_the_runtime_pattern_and_optional_match_contract() {
    let source = r#"
create pattern letter_a:
    "a"
end pattern
store found_match as find letter_a in "abc"
check if found_match is not nothing:
    store adjusted as found_match["start"] minus 1
end check
"#;
    typecheck(source).unwrap_or_else(|failure| {
        panic!(
            "a guarded match object has dynamic fields: {:?}",
            failure.into_diagnostics()
        )
    });

    let errors = diagnostics(r#"store found_match as find "a" in "abc""#);
    assert!(
        errors
            .iter()
            .any(|message| message.contains("Expected Pattern")),
        "runtime pattern-find requires a compiled Pattern, got: {errors:?}"
    );
}
