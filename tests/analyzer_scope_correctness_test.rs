// Correctness tests for analyzer scope semantics
// These tests ensure that the Rc<Scope> refactoring doesn't break
// existing scope behavior: isolation, lookup, and resolution.

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::analyzer::Analyzer;

#[test]
fn test_scope_isolation() {
    // Variables defined ONLY in then-block of if-else should NOT be visible outside
    // (WFL propagates variables from single-branch if-statements, but not from if-else)
    let input = r#"
        check if yes:
            store inner_var as 42
        otherwise:
            store other_var as 43
        end check
        display inner_var
    "#;

    let tokens = lex_wfl_with_positions(input);
    let ast = Parser::new(&tokens).parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);

    // Should produce semantic error for undefined 'inner_var'
    assert!(result.is_err(), "Should error on undefined variable");
    let errors = result.unwrap_err();
    assert!(!errors.is_empty(), "Should have at least one error");
    assert!(
        errors[0].message.contains("inner_var") && errors[0].message.contains("not defined"),
        "Error should mention undefined variable: {}",
        errors[0].message
    );
}

#[test]
fn test_parent_scope_lookup() {
    // Variables defined in outer scopes SHOULD be visible in inner scopes
    let input = r#"
        store outer_var as 10
        check if yes:
            display outer_var
        end check
    "#;

    let tokens = lex_wfl_with_positions(input);
    let ast = Parser::new(&tokens).parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);

    // Should analyze without errors
    assert!(result.is_ok(), "Should succeed: {:?}", result.err());
}

#[test]
fn test_multiple_children_from_same_parent() {
    // Both if and else branches should see parent variables
    // but NOT each other's variables
    let input = r#"
        store parent_var as 1
        check if yes:
            store in_then as 2
            display parent_var
        otherwise:
            store in_else as 3
            display parent_var
        end check
    "#;

    let tokens = lex_wfl_with_positions(input);
    let ast = Parser::new(&tokens).parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);

    // Should analyze without errors - both branches can see parent_var
    assert!(result.is_ok(), "Should succeed: {:?}", result.err());
}

#[test]
fn test_branch_variable_isolation() {
    // Variables defined in then-branch should NOT be visible in else-branch
    let input = r#"
        check if yes:
            store in_then as 2
        otherwise:
            display in_then
        end check
    "#;

    let tokens = lex_wfl_with_positions(input);
    let ast = Parser::new(&tokens).parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);

    // Should error - in_then not defined in else branch
    assert!(result.is_err(), "Should error on undefined variable");
}

#[test]
fn test_deep_parent_chain_resolution() {
    // Variables should resolve through multiple parent levels
    let input = r#"
        store level0 as "root"
        check if yes:
            store level1 as "first"
            check if yes:
                store level2 as "second"
                check if yes:
                    display level0
                    display level1
                    display level2
                end check
            end check
        end check
    "#;

    let tokens = lex_wfl_with_positions(input);
    let ast = Parser::new(&tokens).parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);

    // Should analyze without errors - all variables resolve through parent chain
    assert!(result.is_ok(), "Should succeed: {:?}", result.err());
}

#[test]
fn test_loop_variable_scoping() {
    // Loop variables should be scoped to the loop body
    let input = r#"
        store items as [1, 2, 3]
        for each item in items:
            display item
        end for
        display item
    "#;

    let tokens = lex_wfl_with_positions(input);
    let ast = Parser::new(&tokens).parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);

    // Should error - 'item' not defined outside loop
    assert!(result.is_err(), "Should error on undefined loop variable");
    let errors = result.unwrap_err();
    assert!(
        errors[0].message.contains("item") && errors[0].message.contains("not defined"),
        "Error should mention undefined variable: {}",
        errors[0].message
    );
}

#[test]
fn test_try_when_error_variable_scoping() {
    // Error variables should be scoped to their when clause
    let input = r#"
        try:
            display "trying"
        when error:
            display error
        end try
        display error
    "#;

    let tokens = lex_wfl_with_positions(input);
    let ast = Parser::new(&tokens).parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);

    // Should error - 'error' not defined outside when clause
    assert!(result.is_err(), "Should error on undefined error variable");
}

#[test]
fn test_no_variable_redefinition_in_same_scope() {
    // Cannot redefine a variable in the same scope
    let input = r#"
        store x as 10
        store x as 20
    "#;

    let tokens = lex_wfl_with_positions(input);
    let ast = Parser::new(&tokens).parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);

    // Should error - cannot redefine x
    assert!(result.is_err(), "Should error on variable redefinition");
    let errors = result.unwrap_err();
    assert!(
        errors[0].message.contains("already been defined"),
        "Error should mention variable already defined: {}",
        errors[0].message
    );
}

#[test]
fn test_no_variable_shadowing_parent_scope() {
    // Cannot define a variable that shadows a parent scope variable
    let input = r#"
        store outer as 10
        check if yes:
            store outer as 20
        end check
    "#;

    let tokens = lex_wfl_with_positions(input);
    let ast = Parser::new(&tokens).parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);

    // Should error - cannot shadow parent variable
    assert!(result.is_err(), "Should error on variable shadowing");
    let errors = result.unwrap_err();
    assert!(
        errors[0].message.contains("already been defined in an outer scope"),
        "Error should mention outer scope: {}",
        errors[0].message
    );
}

#[test]
fn test_action_parameter_scoping() {
    // Action parameters should be visible throughout the action body
    let input = r#"
        define action called process_item with parameter input_value:
            check if input_value is greater than 0:
                display input_value
            end check
        end action
    "#;

    let tokens = lex_wfl_with_positions(input);
    let ast = Parser::new(&tokens).parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);

    // Should analyze without errors - parameter visible in nested scopes
    assert!(result.is_ok(), "Should succeed: {:?}", result.err());
}

#[test]
fn test_count_loop_variable() {
    // Count loop creates an implicit 'count' variable scoped to the loop
    // Note: WFL may propagate the variable outside in some cases, so we test
    // that it's at least defined and accessible WITHIN the loop
    let input = r#"
        count from 1 to 5:
            display count
        end count
    "#;

    let tokens = lex_wfl_with_positions(input);
    let ast = Parser::new(&tokens).parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);

    // Should succeed - count is defined within the loop
    assert!(result.is_ok(), "Should succeed: {:?}", result.err());
}

#[test]
fn test_nested_loops_with_same_variable_name() {
    // Each loop should have its own scope for the loop variable
    let input = r#"
        store outer_items as [1, 2]
        for each item in outer_items:
            store inner_items as [3, 4]
            for each item in inner_items:
                display item
            end for
        end for
    "#;

    let tokens = lex_wfl_with_positions(input);
    let ast = Parser::new(&tokens).parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);

    // Should error - inner loop tries to redefine 'item' from outer loop
    assert!(result.is_err(), "Should error on loop variable shadowing");
}
