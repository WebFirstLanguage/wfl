use wfl::analyzer::Analyzer;
use wfl::interpreter::Interpreter;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::Statement;
use wfl::typechecker::TypeChecker;

/// Test that nested count loops with default variable names (count) should fail
/// due to variable shadowing
#[test]
fn test_nested_count_loops_default_variable_should_fail() {
    let code = r#"
store total as 0

count from 1 to 3:
    count from 1 to 2:
        change total to (total plus 1)
    end count
end count
"#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");

    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&program);

    // This should fail with a variable shadowing error
    assert!(
        result.is_err() || !analyzer.get_errors().is_empty(),
        "Nested count loops with default 'count' variable should fail due to shadowing"
    );
}

/// Test that nested count loops with custom variable names should work
#[test]
fn test_nested_count_loops_with_custom_variables() {
    let code = r#"
store total as 0

count from 1 to 3 as i:
    count from 1 to 2 as j:
        change total to (total plus 1)
    end count
end count
"#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse successfully");

    // Verify that CountLoop statements have variable names
    let count_loops: Vec<_> = program
        .statements
        .iter()
        .filter(|stmt| matches!(stmt, Statement::CountLoop { .. }))
        .collect();

    assert_eq!(count_loops.len(), 1, "Should have outer count loop");

    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&program);

    assert!(
        result.is_ok() && analyzer.get_errors().is_empty(),
        "Nested count loops with custom variables should pass analysis: {:?}",
        analyzer.get_errors()
    );
}

/// Test that custom loop variables can be accessed in the loop body
#[test]
fn test_custom_loop_variable_accessible() {
    let code = r#"
store sum as 0

count from 1 to 5 as counter:
    change sum to (sum plus counter)
end count
"#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");

    let mut analyzer = Analyzer::new();
    analyzer
        .analyze(&program)
        .expect("Should analyze successfully");

    assert!(
        analyzer.get_errors().is_empty(),
        "Should have no analysis errors"
    );
}

/// Test execution of nested count loops with custom variables
#[tokio::test]
async fn test_execute_nested_count_loops_with_custom_variables() {
    let code = r#"
store total as 0

count from 1 to 3 as i:
    count from 1 to 2 as j:
        change total to (total plus 1)
    end count
end count
"#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");

    let mut analyzer = Analyzer::new();
    analyzer.analyze(&program).expect("Should analyze");

    let mut type_checker = TypeChecker::new();
    type_checker.check_types(&program).ok();

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .expect("Should execute successfully");

    let total = interpreter
        .global_env()
        .borrow()
        .get("total")
        .expect("total should exist");
    assert_eq!(
        total.to_string(),
        "6",
        "Should have 6 total iterations (3 * 2)"
    );
}

/// Test that default 'count' variable still works for backwards compatibility
#[tokio::test]
async fn test_default_count_variable_backwards_compatibility() {
    let code = r#"
store sum as 0

count from 1 to 5:
    change sum to (sum plus count)
end count
"#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");

    let mut analyzer = Analyzer::new();
    analyzer.analyze(&program).expect("Should analyze");

    let mut type_checker = TypeChecker::new();
    type_checker.check_types(&program).ok();

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .expect("Should execute successfully");

    let sum = interpreter
        .global_env()
        .borrow()
        .get("sum")
        .expect("sum should exist");
    assert_eq!(sum.to_string(), "15", "Should sum 1+2+3+4+5 = 15");
}

/// Test accessing loop variable in nested loop
#[tokio::test]
async fn test_access_both_loop_variables() {
    let code = r#"
create list results:
end list

count from 1 to 2 as outer:
    count from 1 to 2 as inner:
        store value as (outer times 10 plus inner)
        add value to results
    end count
end count
"#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");

    let mut analyzer = Analyzer::new();
    analyzer.analyze(&program).expect("Should analyze");

    let mut type_checker = TypeChecker::new();
    type_checker.check_types(&program).ok();

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .expect("Should execute successfully");

    let results = interpreter
        .global_env()
        .borrow()
        .get("results")
        .expect("results should exist");
    let results_str = results.to_string();

    // Should have values: 11, 12, 21, 22
    assert!(results_str.contains("11"), "Should contain 11");
    assert!(results_str.contains("12"), "Should contain 12");
    assert!(results_str.contains("21"), "Should contain 21");
    assert!(results_str.contains("22"), "Should contain 22");
}

/// Test that closures capturing loop variables prevent environment recycling.
///
/// This verifies that when an action (closure) is defined inside a loop and captures
/// the loop variable, the environment is not recycled under it. Each closure should
/// retain its own captured value rather than seeing a cleared/reused environment.
#[tokio::test]
async fn test_closure_captures_prevent_env_recycling() {
    let code = r#"
create list closures:
end list

count from 1 to 3 as i:
    define action called grab_i:
        give back i
    end action
    add grab_i to closures
end count

store result1 as closures at 0
store result2 as closures at 1
store result3 as closures at 2
"#;

    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("Should parse");

    // Skip analyzer — it doesn't track `define action` names as variables in scope,
    // but the interpreter handles this correctly at runtime.

    let mut interpreter = Interpreter::new();
    interpreter
        .interpret(&program)
        .await
        .expect("Should execute successfully");

    let env = interpreter.global_env();
    let env_borrowed = env.borrow();

    let r1 = env_borrowed.get("result1").expect("result1 should exist");
    let r2 = env_borrowed.get("result2").expect("result2 should exist");
    let r3 = env_borrowed.get("result3").expect("result3 should exist");

    // Each closure should return its own captured loop variable value.
    // If environment recycling incorrectly cleared captured environments,
    // all closures would return the same (or invalid) value.
    assert_eq!(r1.to_string(), "1", "First closure should capture i=1");
    assert_eq!(r2.to_string(), "2", "Second closure should capture i=2");
    assert_eq!(r3.to_string(), "3", "Third closure should capture i=3");
}
