use wfl::parser::Parser;
use wfl::lexer::Lexer;

#[test]
fn test_basic_container_definition() {
    let input = r#"
create container Person:
end
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse program");
    
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_container_with_properties() {
    let input = r#"
create container Person:
    property name: Text
    property age: Number
end
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse program");
    
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_container_with_methods() {
    let input = r#"
create container Person:
    action greet:
        display "Hello"
    end
end
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse program");
    
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_container_instantiation() {
    let input = r#"
create container Person:
end

create new Person as alice:
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse program");
    
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn test_container_with_inheritance() {
    let input = r#"
create container Animal:
end

create container Dog extends Animal:
end
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse program");
    
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn test_container_with_interface_implementation() {
    let input = r#"
create interface Drawable:
end

create container Shape implements Drawable:
end
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse program");
    
    assert_eq!(program.statements.len(), 2);
}

#[test]
fn test_container_with_property_initializers() {
    let input = r#"
create container Person:
    property name: Text
end

create new Person as alice:
    name = "Alice"
"#;
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse program");
    
    assert_eq!(program.statements.len(), 2);
}
