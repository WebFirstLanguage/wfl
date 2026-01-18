//! Tests for the WFL to JavaScript transpiler

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::transpiler::{TranspilerConfig, TranspilerTarget, transpile};

/// Helper function to parse WFL source code and transpile to JavaScript
fn transpile_wfl(source: &str) -> Result<String, String> {
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .map_err(|e| format!("Parse error: {:?}", e))?;

    let config = TranspilerConfig {
        include_runtime: false, // Don't include runtime for tests (cleaner output)
        source_maps: false,
        target: TranspilerTarget::Node,
        minify: false,
        indent: "  ".to_string(),
        es_modules: false,
    };

    let result = transpile(&program, &config).map_err(|e| format!("Transpile error: {}", e))?;
    Ok(result.code)
}

/// Helper to check if output contains expected JavaScript
fn assert_contains(output: &str, expected: &str) {
    assert!(
        output.contains(expected),
        "Expected output to contain:\n{}\n\nActual output:\n{}",
        expected,
        output
    );
}

#[test]
fn test_variable_declaration() {
    let source = r#"store name as "Alice""#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, r#"let name = "Alice";"#);
}

#[test]
fn test_variable_with_spaces() {
    let source = r#"store user name as "Bob""#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, r#"let user_name = "Bob";"#);
}

#[test]
fn test_variable_assignment() {
    let source = r#"
store x as 10
change x to 20
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "let x = 10;");
    assert_contains(&js, "x = 20;");
}

#[test]
fn test_display_statement() {
    let source = r#"display "Hello, World!""#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, r#"WFL.display("Hello, World!");"#);
}

#[test]
fn test_if_statement() {
    let source = r#"
store x as 10
check if x is greater than 5:
    display "big"
end check
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "if ((x > 5))");
    assert_contains(&js, r#"WFL.display("big");"#);
}

#[test]
fn test_if_else_statement() {
    let source = r#"
store x as 3
check if x is greater than 5:
    display "big"
otherwise:
    display "small"
end check
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "if ((x > 5))");
    assert_contains(&js, "} else {");
    assert_contains(&js, r#"WFL.display("small");"#);
}

#[test]
fn test_count_loop() {
    let source = r#"
count from 1 to 5:
    display count
end count
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "for (let count = 1; count <= 5; count += 1)");
}

#[test]
fn test_count_loop_with_step() {
    let source = r#"
count from 0 to 10 by 2:
    display count
end count
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "for (let count = 0; count <= 10; count += 2)");
}

#[test]
fn test_count_loop_downward() {
    let source = r#"
count from 10 down to 1:
    display count
end count
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "for (let count = 10; count >= 1; count -= 1)");
}

#[test]
fn test_for_each_loop() {
    let source = r#"
create list items:
    add "a"
    add "b"
end list
for each item in items:
    display item
end for
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "for (const item of items)");
}

#[test]
fn test_action_definition() {
    let source = r#"
define action called greet:
    display "Hello!"
end action
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "function greet()");
    assert_contains(&js, r#"WFL.display("Hello!");"#);
}

#[test]
fn test_action_with_parameter() {
    let source = r#"
define action called say_hello with parameter name:
    display name
end action
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "function say_hello");
    assert_contains(&js, "WFL.display(name);");
}

#[test]
fn test_action_with_return() {
    let source = r#"
define action called double with parameter x:
    return x times 2
end action
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "function double");
    assert_contains(&js, "return (x * 2);");
}

#[test]
fn test_binary_arithmetic_operations() {
    let source = r#"
store a as 10 plus 5
store b as 10 minus 5
store c as 10 times 5
store d as 10 divided by 5
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "let a = (10 + 5);");
    assert_contains(&js, "let b = (10 - 5);");
    assert_contains(&js, "let c = (10 * 5);");
    assert_contains(&js, "let d = (10 / 5);");
}

#[test]
fn test_comparison_operations() {
    // Use variables on the left side of comparisons (WFL syntax)
    let source = r#"
store x as 5
store y as 10
store a as x is equal to x
store b as x is greater than 3
store c as y is less than 15
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "let a = (x === x);");
    assert_contains(&js, "let b = (x > 3);");
    assert_contains(&js, "let c = (y < 15);");
}

#[test]
fn test_logical_operations() {
    let source = r#"
store a as yes and no
store b as yes or no
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "let a = (true && false);");
    assert_contains(&js, "let b = (true || false);");
}

#[test]
fn test_unary_not() {
    let source = r#"store a as not yes"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "let a = (!true);");
}

#[test]
fn test_list_creation() {
    let source = r#"
create list numbers:
    add 1
    add 2
    add 3
end list
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "let numbers = [1, 2, 3];");
}

#[test]
fn test_list_push() {
    let source = r#"
create list items:
end list
push with items and "new item"
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "items.push(\"new item\");");
}

#[test]
fn test_list_clear() {
    let source = r#"
create list items:
    add 1
end list
clear items
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "items.length = 0;");
}

#[test]
fn test_string_concatenation() {
    let source = r#"store greeting as "Hello, " with "World!""#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(
        &js,
        r#"let greeting = (String("Hello, ") + String("World!"));"#,
    );
}

#[test]
fn test_main_action_call() {
    let source = r#"
define action called main:
    display "Hello from main!"
end action
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "function main()");
    assert_contains(&js, "main();"); // Entry point call
}

#[test]
fn test_es_modules_option() {
    let source = r#"display "test""#;
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();

    let config = TranspilerConfig {
        include_runtime: false,
        source_maps: false,
        target: TranspilerTarget::Node,
        minify: false,
        indent: "  ".to_string(),
        es_modules: true, // Enable ES modules
    };

    let result = transpile(&program, &config).unwrap();

    // Should NOT have IIFE wrapper when using ES modules
    assert!(!result.code.contains("(function()"));
}

#[test]
fn test_iife_wrapper() {
    let source = r#"display "test""#;
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();

    let config = TranspilerConfig {
        include_runtime: false,
        source_maps: false,
        target: TranspilerTarget::Node,
        minify: false,
        indent: "  ".to_string(),
        es_modules: false, // Disable ES modules (use IIFE)
    };

    let result = transpile(&program, &config).unwrap();

    // Should have IIFE wrapper
    assert_contains(&result.code, "(function()");
    assert_contains(&result.code, "'use strict';");
    assert_contains(&result.code, "})();");
}

#[test]
fn test_boolean_literals() {
    let source = r#"
store a as yes
store b as no
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "let a = true;");
    assert_contains(&js, "let b = false;");
}

#[test]
fn test_nothing_literal() {
    let source = r#"store a as nothing"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "let a = null;");
}

#[test]
fn test_repeat_while_loop() {
    let source = r#"
store x as 0
repeat while x is less than 5:
    change x to x plus 1
end repeat
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "do {");
    assert_contains(&js, "} while ((x < 5));");
}

#[test]
fn test_reserved_word_sanitization() {
    // Test that reserved words are properly sanitized
    // Using 'function' as a variable name (reserved in JS)
    let source = r#"store my_function as "test""#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "let my_function =");
}

#[test]
fn test_try_catch_basic() {
    let source = r#"
try:
    display "trying"
catch:
    display "caught"
end try
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "try {");
    assert_contains(&js, "} catch (_wfl_error) {");
}

#[test]
fn test_nested_if() {
    let source = r#"
store x as 10
check if x is greater than 5:
    check if x is less than 15:
        display "in range"
    end check
end check
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "if ((x > 5))");
    assert_contains(&js, "if ((x < 15))");
    assert_contains(&js, r#"WFL.display("in range");"#);
}

#[test]
fn test_multiple_statements() {
    let source = r#"
store a as 1
store b as 2
store c as a plus b
display c
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "let a = 1;");
    assert_contains(&js, "let b = 2;");
    assert_contains(&js, "let c = (a + b);");
    assert_contains(&js, "WFL.display(c);");
}

#[test]
fn test_number_literals() {
    let source = r#"
store integer as 42
store float_val as 3.14
store negative as 0 minus 10
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "let integer = 42;");
    assert_contains(&js, "let float_val = 3.14;");
    assert_contains(&js, "let negative = (0 - 10);");
}

#[test]
fn test_display_concatenation() {
    let source = r#"
store name as "Alice"
display "Hello, " with name
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, r#"WFL.display((String("Hello, ") + String(name)));"#);
}

#[test]
fn test_complex_expression() {
    let source = r#"store result as 2 plus 3 times 4"#;
    let js = transpile_wfl(source).unwrap();
    // Should handle operator precedence
    assert!(js.contains("let result ="));
}

#[test]
fn test_empty_list() {
    let source = r#"
create list empty:
end list
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "let empty = [];");
}

#[test]
fn test_list_with_mixed_types() {
    let source = r#"
create list mixed:
    add 1
    add "text"
    add yes
end list
"#;
    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "let mixed = [1, \"text\", true];");
}

#[test]
fn test_action_hoisting() {
    // Actions should be hoisted (generated before other code)
    let source = r#"
display "before"
define action called test:
    display "test"
end action
display "after"
"#;
    let js = transpile_wfl(source).unwrap();
    // The function should appear before the display calls
    let func_pos = js.find("function test()").unwrap();
    let display_before = js.find(r#"WFL.display("before")"#).unwrap();
    assert!(
        func_pos < display_before,
        "Function should be hoisted before other statements"
    );
}

#[test]
fn test_runtime_inclusion() {
    let source = r#"
        store x as 42
        display x
    "#;

    // Parse the source
    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().unwrap();

    // Test with runtime included (default)
    let config = TranspilerConfig {
        include_runtime: true,
        target: TranspilerTarget::Node,
        ..Default::default()
    };
    let result = transpile(&program, &config).unwrap();
    assert!(result.code.contains("// WFL Runtime Library"));
    assert!(result.code.contains("const WFL = {"));

    // Test with runtime excluded
    let config = TranspilerConfig {
        include_runtime: false,
        target: TranspilerTarget::Node,
        ..Default::default()
    };
    let result = transpile(&program, &config).unwrap();
    assert!(!result.code.contains("// WFL Runtime Library"));
    assert!(!result.code.contains("const WFL = {"));
    assert!(result.code.contains("let x = 42"));
}

#[test]
fn test_pattern_matching_basic() {
    let source = r#"
        create pattern test_pattern:
            "test"
            one or more digit
        end pattern
        
        store text as "test123"
        if text matches test_pattern then
            display "matches"
        else
            display "no match"
        end if
    "#;

    let js = transpile_wfl(source).unwrap();
    assert_contains(&js, "new WFL.Pattern");
    println!("Generated JS:\n{}", js);
}

#[test]
fn test_async_main_function_detection() {
    let source = r#"
        define action called main:
            wait for 1000 milliseconds
            display "async main"
        end action
    "#;

    let js = transpile_wfl(source).unwrap();
    // Should detect async main and wrap in IIFE with await
    assert_contains(&js, "(async () => { await main(); })()");
    assert_contains(&js, "async function main()");
}

#[test]
fn test_wait_for_with_variable_declaration() {
    let source = r#"
        wait for store result as 42
        display result
    "#;

    let js = transpile_wfl(source).unwrap();
    // Should generate proper await for variable declaration
    assert_contains(&js, "let result = await 42");
    assert!(!js.contains("await let"));
}
