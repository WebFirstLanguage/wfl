// Performance tests for analyzer scope cloning
// These tests verify that the Rc<Scope> optimization prevents quadratic complexity
// in deeply nested code structures.

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::analyzer::Analyzer;
use std::time::Instant;

#[test]
fn test_deeply_nested_if_statements_performance() {
    // Generate WFL program with 20 levels of nested if statements
    // This tests the worst case for scope cloning in conditional branches
    let mut program = String::new();

    for i in 0..20 {
        program.push_str(&format!("{}check if yes:\n", "    ".repeat(i)));
    }
    program.push_str(&format!("{}store deeply_nested_var as 42\n", "    ".repeat(20)));
    for i in (0..20).rev() {
        program.push_str(&format!("{}end check\n", "    ".repeat(i)));
    }

    let start = Instant::now();
    let tokens = lex_wfl_with_positions(&program);
    let ast = Parser::new(&tokens).parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);
    let duration = start.elapsed();

    // Verify analysis succeeded
    assert!(result.is_ok(), "Analysis should succeed: {:?}", result.err());

    // With Box<Scope> cloning, this would take >1s or timeout
    // With Rc<Scope> optimization, should complete in under 100ms
    println!("Deeply nested if statements (20 levels) analyzed in {}ms", duration.as_millis());
    assert!(
        duration.as_millis() < 500,
        "Analysis took {}ms, expected <500ms (may indicate O(N²) cloning issue)",
        duration.as_millis()
    );
}

#[test]
fn test_deeply_nested_loops_performance() {
    // Generate WFL program with 15 levels of nested loops
    // Use unique variable names to avoid shadowing errors
    let mut program = String::new();

    program.push_str("store items0 as [1, 2]\n");
    for i in 0..15 {
        let items_var = format!("items{}", i);
        let item_var = format!("item{}", i);
        let next_items_var = format!("items{}", i + 1);

        program.push_str(&format!("{}for each {} in {}:\n", "    ".repeat(i), item_var, items_var));
        program.push_str(&format!("{}store {} as [1, 2]\n", "    ".repeat(i + 1), next_items_var));
    }
    program.push_str(&format!("{}store nested_loop_var as 42\n", "    ".repeat(15)));
    for i in (0..15).rev() {
        program.push_str(&format!("{}end for\n", "    ".repeat(i)));
    }

    let start = Instant::now();
    let tokens = lex_wfl_with_positions(&program);
    let ast = Parser::new(&tokens).parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);
    let duration = start.elapsed();

    // Verify analysis succeeded
    assert!(result.is_ok(), "Analysis should succeed: {:?}", result.err());

    println!("Deeply nested loops (15 levels) analyzed in {}ms", duration.as_millis());
    assert!(
        duration.as_millis() < 500,
        "Analysis took {}ms, expected <500ms",
        duration.as_millis()
    );
}

#[test]
fn test_mixed_nested_control_flow_performance() {
    // Mix different control flow structures to test realistic nesting
    let mut program = String::new();

    program.push_str("store items as [1, 2, 3]\n");
    program.push_str("store condition as yes\n\n");

    // Level 1: for loop
    program.push_str("for each item in items:\n");
    // Level 2: if statement
    program.push_str("    check if condition:\n");
    // Level 3: try block
    program.push_str("        try:\n");
    // Level 4: another for loop (count)
    program.push_str("            count from 1 to 10:\n");
    // Level 5: nested if
    program.push_str("                check if count is greater than 5:\n");
    // Level 6: another try
    program.push_str("                    try:\n");
    // Level 7: nested if instead of while loop (which has complex syntax)
    program.push_str("                        check if condition:\n");
    // Level 8: deep if
    program.push_str("                            check if item is greater than 0:\n");
    program.push_str("                                store result as item\n");
    program.push_str("                                change condition to no\n");
    program.push_str("                            end check\n");
    program.push_str("                        end check\n");
    program.push_str("                    when error:\n");
    program.push_str("                        display error\n");
    program.push_str("                    end try\n");
    program.push_str("                end check\n");
    program.push_str("            end count\n");
    program.push_str("        when error:\n");
    program.push_str("            display error\n");
    program.push_str("        end try\n");
    program.push_str("    end check\n");
    program.push_str("end for\n");

    let start = Instant::now();
    let tokens = lex_wfl_with_positions(&program);
    let ast = Parser::new(&tokens).parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);
    let duration = start.elapsed();

    // Verify analysis succeeded
    assert!(result.is_ok(), "Analysis should succeed: {:?}", result.err());

    println!("Mixed nested control flow (8 levels) analyzed in {}ms", duration.as_millis());
    assert!(
        duration.as_millis() < 200,
        "Analysis took {}ms, expected <200ms",
        duration.as_millis()
    );
}

#[test]
fn test_if_else_branches_performance() {
    // Test if-else branches which create multiple child scopes from the same parent
    // This specifically tests the cloning pattern at lines 483 and 505
    let mut program = String::new();

    program.push_str("store outer as 1\n");
    for i in 0..12 {
        let indent = "    ".repeat(i);
        program.push_str(&format!("{}check if yes:\n", indent));
        program.push_str(&format!("{}    store then_{} as {}\n", indent, i, i));
        program.push_str(&format!("{}otherwise:\n", indent));
        program.push_str(&format!("{}    store else_{} as {}\n", indent, i, i));
    }
    program.push_str(&format!("{}end check\n", "    ".repeat(11)));
    for i in (0..11).rev() {
        program.push_str(&format!("{}end check\n", "    ".repeat(i)));
    }

    let start = Instant::now();
    let tokens = lex_wfl_with_positions(&program);
    let ast = Parser::new(&tokens).parse().expect("Parse should succeed");
    let mut analyzer = Analyzer::new();
    let result = analyzer.analyze(&ast);
    let duration = start.elapsed();

    // Verify analysis succeeded
    assert!(result.is_ok(), "Analysis should succeed: {:?}", result.err());

    println!("Nested if-else branches (12 levels) analyzed in {}ms", duration.as_millis());
    assert!(
        duration.as_millis() < 300,
        "Analysis took {}ms, expected <300ms",
        duration.as_millis()
    );
}
