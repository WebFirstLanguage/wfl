use wfl::parser::Parser;
use wfl::lexer::Lexer;

#[test]
fn test_missing_colon_after_container_name() {
    let input = r#"
create container Person
end
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    
    assert!(result.is_err(), "Expected parse error for missing colon");
}

#[test]
fn test_missing_end_keyword() {
    let input = r#"
create container Person:
    property name: Text
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    
    assert!(result.is_err(), "Expected parse error for missing end keyword");
}

#[test]
fn test_invalid_property_syntax() {
    let input = r#"
create container Person:
    property name
end
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    
    assert!(result.is_err(), "Expected parse error for invalid property syntax");
}

#[test]
fn test_undefined_parent_class() {
    let input = r#"
create container Dog extends UndefinedAnimal:
end
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    
    assert!(result.is_ok(), "Parser should succeed, typechecker should catch error");
}

#[test]
fn test_invalid_method_syntax() {
    let input = r#"
create container Person:
    action greet
        display "Hello"
    end
end
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    
    assert!(result.is_err(), "Expected parse error for missing colon after action");
}

#[test]
fn test_invalid_instantiation_syntax() {
    let input = r#"
create new Person alice:
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    
    assert!(result.is_err(), "Expected parse error for missing 'as' keyword");
}

#[test]
fn test_empty_container_body() {
    let input = r#"
create container Person:
end
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    
    assert!(result.is_ok(), "Empty container body should be valid");
}
