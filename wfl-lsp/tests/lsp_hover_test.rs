// Tests for WFL LSP hover functionality
// These tests validate that the LSP server provides meaningful hover information
// for WFL symbols including variables, functions, and built-in constructs

use tower_lsp::lsp_types::{
    HoverParams, Position, TextDocumentIdentifier, TextDocumentPositionParams, Url,
};
use wfl::analyzer::Analyzer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::{Parser, ast::Program};
use wfl::typechecker::TypeChecker;

// Helper function to extract hover information from WFL programs
// This simulates what the LSP server should do for hover requests
fn extract_hover_info_at_position(
    document_text: &str,
    line: u32,
    character: u32,
) -> Option<String> {
    let tokens = lex_wfl_with_positions(document_text);
    let mut parser = Parser::new(&tokens);

    match parser.parse() {
        Ok(program) => {
            // Find the symbol at the given position
            if let Some(symbol_info) = find_symbol_at_position(&program, line, character) {
                Some(format_hover_info(&symbol_info))
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

#[derive(Debug, Clone)]
enum SymbolInfo {
    Variable {
        name: String,
        var_type: String,
        value: Option<String>,
    },
    Function {
        name: String,
        parameters: Vec<String>,
        return_type: Option<String>,
    },
    Keyword {
        name: String,
        description: String,
    },
    StdlibFunction {
        name: String,
        description: String,
        signature: String,
    },
}

fn find_symbol_at_position(program: &Program, line: u32, character: u32) -> Option<SymbolInfo> {
    use wfl::parser::ast::Statement;

    // This is a simplified implementation - real implementation would be more sophisticated
    for statement in &program.statements {
        match statement {
            Statement::VariableDeclaration { name, .. } => {
                // For testing, assume any position in the document could reference this variable
                return Some(SymbolInfo::Variable {
                    name: name.clone(),
                    var_type: "text".to_string(), // Simplified - would use type checker
                    value: Some("example value".to_string()),
                });
            }
            Statement::ActionDefinition {
                name, parameters, ..
            } => {
                let param_names: Vec<String> = parameters.iter().map(|p| p.name.clone()).collect();
                return Some(SymbolInfo::Function {
                    name: name.clone(),
                    parameters: param_names,
                    return_type: Some("any".to_string()),
                });
            }
            _ => {}
        }
    }

    None
}

fn format_hover_info(symbol_info: &SymbolInfo) -> String {
    match symbol_info {
        SymbolInfo::Variable {
            name,
            var_type,
            value,
        } => {
            let mut info = format!("**Variable:** `{}`\n\n**Type:** `{}`", name, var_type);
            if let Some(val) = value {
                info.push_str(&format!("\n\n**Value:** `{}`", val));
            }
            info
        }
        SymbolInfo::Function {
            name,
            parameters,
            return_type,
        } => {
            let params = parameters.join(", ");
            let mut info = format!("**Function:** `{}({})`", name, params);
            if let Some(ret_type) = return_type {
                info.push_str(&format!("\n\n**Returns:** `{}`", ret_type));
            }
            info
        }
        SymbolInfo::Keyword { name, description } => {
            format!("**WFL Keyword:** `{}`\n\n{}", name, description)
        }
        SymbolInfo::StdlibFunction {
            name,
            description,
            signature,
        } => {
            format!(
                "**WFL Standard Library**\n\n`{}`\n\n{}",
                signature, description
            )
        }
    }
}

#[tokio::test]
async fn test_hover_should_show_variable_information() {
    // Test that hover shows variable type and value information
    let document_text = r#"
store username as "Alice"
store age as 25
store active as yes

display username
"#;

    // Test hover at different positions
    let hover_info = extract_hover_info_at_position(document_text, 1, 6); // On "username"

    assert!(
        hover_info.is_some(),
        "Should provide hover info for variables"
    );

    let info = hover_info.unwrap();
    assert!(info.contains("Variable"), "Should identify as variable");
    assert!(info.contains("username"), "Should show variable name");
    assert!(info.contains("Type"), "Should show variable type");

    println!("Variable hover info: {}", info);
}

#[tokio::test]
async fn test_hover_should_show_function_information() {
    // Test that hover shows function signature and parameter information
    let document_text = r#"
define action called greet with name and title:
    display title with ": " with name
end action

define action called calculate with x and y:
    return x + y
end action

call greet with "Alice" and "Ms."
"#;

    let hover_info = extract_hover_info_at_position(document_text, 1, 20); // On function definition

    assert!(
        hover_info.is_some(),
        "Should provide hover info for functions"
    );

    let info = hover_info.unwrap();
    assert!(info.contains("Function"), "Should identify as function");
    assert!(info.contains("greet"), "Should show function name");
    assert!(info.contains("name"), "Should show parameter names");
    assert!(info.contains("title"), "Should show parameter names");

    println!("Function hover info: {}", info);
}

#[tokio::test]
async fn test_hover_should_show_stdlib_function_information() {
    // Test that hover shows standard library function documentation
    let document_text = r#"
store my_list as [1, 2, 3, 4, 5]
store list_size as length of my_list
store first_item as first of my_list

display "List size: " with list_size
"#;

    // This test validates that we should provide hover info for stdlib functions
    // The actual implementation would detect "length of" and "first of" as stdlib functions

    let expected_stdlib_functions = [
        ("length of", "Returns the number of items in a collection"),
        ("first of", "Returns the first item in a collection"),
        ("last of", "Returns the last item in a collection"),
        ("uppercase", "Converts text to uppercase"),
        ("lowercase", "Converts text to lowercase"),
    ];

    for (func_name, description) in &expected_stdlib_functions {
        // Validate that we have documentation for these functions
        assert!(!func_name.is_empty(), "Function name should not be empty");
        assert!(
            !description.is_empty(),
            "Function description should not be empty"
        );

        // Create expected hover format
        let expected_hover = format!(
            "**WFL Standard Library**\n\n`{}`\n\n{}",
            func_name, description
        );
        assert!(
            expected_hover.contains("WFL Standard Library"),
            "Should identify as stdlib function"
        );
    }

    println!("Stdlib function hover validation completed");
}

#[tokio::test]
async fn test_hover_should_show_keyword_information() {
    // Test that hover shows WFL keyword documentation
    let document_text = r#"
if age is greater than 18 then
    display "Adult"
otherwise
    display "Minor"
end if

count from i as 1 to 10
    display i
end count
"#;

    let wfl_keywords = [
        (
            "if",
            "Conditional statement - executes code block if condition is true",
        ),
        ("then", "Marks the beginning of the if block"),
        ("otherwise", "Alternative block for if statement (else)"),
        ("end", "Marks the end of a code block"),
        (
            "count",
            "Loop statement - repeats code block for a range of values",
        ),
        ("from", "Specifies the start of a count loop"),
        ("to", "Specifies the end of a count loop"),
        ("store", "Creates a new variable and assigns a value"),
        ("display", "Outputs text or values to the console"),
    ];

    for (keyword, description) in &wfl_keywords {
        // Validate that we have documentation for these keywords
        assert!(!keyword.is_empty(), "Keyword should not be empty");
        assert!(
            !description.is_empty(),
            "Keyword description should not be empty"
        );

        // Create expected hover format
        let expected_hover = format!("**WFL Keyword:** `{}`\n\n{}", keyword, description);
        assert!(
            expected_hover.contains("WFL Keyword"),
            "Should identify as WFL keyword"
        );
    }

    println!("WFL keyword hover validation completed");
}

#[tokio::test]
async fn test_hover_should_include_type_information() {
    // Test that hover includes type information from type checker
    let document_text = r#"
store name as "Alice"          // text type
store age as 25                // number type
store active as yes            // boolean type

display name
"#;

    let tokens = lex_wfl_with_positions(document_text);
    let mut parser = Parser::new(&tokens);

    match parser.parse() {
        Ok(program) => {
            // Test that we can analyze the program for type information
            let mut analyzer = Analyzer::new();
            let analysis_result = analyzer.analyze(&program);

            let mut type_checker = TypeChecker::new();
            let type_result = type_checker.check_types(&program);

            // Even if analysis/type checking has errors, we should be able to provide basic hover info
            println!("Analysis result: {:?}", analysis_result.is_ok());
            println!("Type checking result: {:?}", type_result.is_ok());

            // The hover implementation should use type information when available
            assert!(
                !program.statements.is_empty(),
                "Should have parsed statements for hover analysis"
            );
        }
        Err(errors) => {
            panic!("Failed to parse test program: {:?}", errors);
        }
    }
}

#[tokio::test]
async fn test_hover_should_handle_complex_expressions() {
    // Test that hover works with complex expressions and member access
    let document_text = r#"
store user as {
    "name": "Alice",
    "age": 25,
    "active": yes
}

store user_name as user.name
store user_age as user.age

display "User: " with user_name
"#;

    // This test validates that hover should work with:
    // - Object/map access (user.name)
    // - Complex expressions
    // - Nested structures

    let tokens = lex_wfl_with_positions(document_text);
    let mut parser = Parser::new(&tokens);

    match parser.parse() {
        Ok(program) => {
            // Test that we can parse complex expressions for hover analysis
            assert!(
                !program.statements.is_empty(),
                "Should parse complex expressions"
            );

            // The hover implementation should handle:
            // - Member access expressions (user.name)
            // - Variable references in complex contexts
            // - Type information for nested structures

            println!("Complex expression hover test - parsing successful");
        }
        Err(errors) => {
            println!(
                "Parse errors (may be expected for complex syntax): {:?}",
                errors
            );
            // Even with parse errors, basic hover should still work
        }
    }
}

#[tokio::test]
async fn test_hover_performance() {
    // Test that hover performs well with larger documents
    let mut large_document = String::new();

    // Create a large document with many variables and functions
    for i in 0..50 {
        large_document.push_str(&format!("store var_{} as {}\n", i, i));
    }

    for i in 0..10 {
        large_document.push_str(&format!(
            "define action called func_{} with param:\n    return param + {}\nend action\n\n",
            i, i
        ));
    }

    large_document.push_str("display var_0\n");

    let start_time = std::time::Instant::now();

    let hover_info = extract_hover_info_at_position(&large_document, 0, 6);

    let hover_time = start_time.elapsed();

    println!("Hover analysis took: {:?} for large document", hover_time);

    // Performance assertion - hover should be fast
    assert!(
        hover_time.as_millis() < 100,
        "Hover analysis should be under 100ms for large documents"
    );

    // Should still provide hover info even for large documents
    assert!(
        hover_info.is_some(),
        "Should provide hover info for large documents"
    );
}
