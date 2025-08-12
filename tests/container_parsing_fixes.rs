// TDD tests for container parsing fixes
// These tests MUST fail first, then implementation will be written to make them pass

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

#[test]
fn test_container_action_without_return_type_should_parse() {
    let source = r#"
create container Test:
    action greet:
        display "Hello"
    end
end
"#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();
    
    // This should pass once the bug is fixed
    assert!(result.is_ok(), "Parser should handle action without return type: {:?}", result.err());
}

#[test]
fn test_container_action_with_parameters_should_parse() {
    let source = r#"
create container Test:
    action set_name needs new_name: Text:
        display "Setting name"
    end
end
"#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();
    
    // This should pass once the 'needs' keyword bug is fixed
    assert!(result.is_ok(), "Parser should handle 'needs' parameters: {:?}", result.err());
}

#[test]
fn test_nested_end_tokens_should_parse() {
    let source = r#"
create container Test:
    action greet:
        display "Hello"
    end
end
"#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();
    
    // This should pass once the nested end token handling is fixed
    assert!(result.is_ok(), "Parser should handle nested end tokens: {:?}", result.err());
}

#[test]
fn test_all_container_parsing_issues_combined() {
    let source = r#"
create container Person:
    property name: Text
    
    action greet:
        display "Hello, I am " with name
    end
    
    action set_name needs new_name: Text:
        store name as new_name
    end
end
"#;

    let tokens = lex_wfl_with_positions(source);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();
    
    // This comprehensive test should pass once all bugs are fixed
    assert!(result.is_ok(), "Parser should handle complete container with all features: {:?}", result.err());
}