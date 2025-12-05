use wfl::parser::Parser;
use wfl::lexer::lex_wfl_with_positions;
use wfl::analyzer::Analyzer;
use wfl::typechecker::TypeChecker;
use wfl::interpreter::Interpreter;
use wfl::parser::ast::Statement;

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
    let count_loops: Vec<_> = program.statements.iter().filter_map(|stmt| {
        if let Statement::CountLoop { .. } = stmt {
            Some(stmt)
        } else {
            None
        }
    }).collect();

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
    analyzer.analyze(&program).expect("Should analyze successfully");

    assert!(analyzer.get_errors().is_empty(), "Should have no analysis errors");
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
    interpreter.interpret(&program).await.expect("Should execute successfully");

    let total = interpreter.global_env().borrow().get("total").expect("total should exist");
    assert_eq!(total.to_string(), "6", "Should have 6 total iterations (3 * 2)");
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
    interpreter.interpret(&program).await.expect("Should execute successfully");

    let sum = interpreter.global_env().borrow().get("sum").expect("sum should exist");
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
    interpreter.interpret(&program).await.expect("Should execute successfully");

    let results = interpreter.global_env().borrow().get("results").expect("results should exist");
    let results_str = results.to_string();

    // Should have values: 11, 12, 21, 22
    assert!(results_str.contains("11"), "Should contain 11");
    assert!(results_str.contains("12"), "Should contain 12");
    assert!(results_str.contains("21"), "Should contain 21");
    assert!(results_str.contains("22"), "Should contain 22");
}
