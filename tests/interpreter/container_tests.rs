use wfl::interpreter::Interpreter;
use wfl::parser::Parser;
use wfl::lexer::Lexer;
use wfl::interpreter::value::Value;
use tokio;

#[tokio::test]
async fn test_container_instantiation() {
    let input = r#"
create container Person:
    property name: Text
    property age: Number
end

create new Person as alice:
    name = "Alice"
    age = 28
"#;
    
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse program");
    
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&program).await;
    
    assert!(result.is_ok(), "Container instantiation should succeed");
}

#[tokio::test]
async fn test_container_method_call() {
    let input = r#"
create container Person:
    property name: Text
    property age: Number
    
    action greet:
        display "Hello, I am " + this.name + " and I am " + this.age + "."
    end
end

create new Person as alice:
    name = "Alice"
    age = 28

alice.greet()
"#;
    
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse program");
    
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&program).await;
    
    assert!(result.is_ok(), "Container method call should succeed");
}

#[tokio::test]
async fn test_container_property_access() {
    let input = r#"
create container Person:
    property name: Text
    property age: Number
end

create new Person as alice:
    name = "Alice"
    age = 28

display alice.name
display alice.age
"#;
    
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse program");
    
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&program).await;
    
    assert!(result.is_ok(), "Container property access should succeed");
}

#[tokio::test]
async fn test_container_inheritance() {
    let input = r#"
create container Animal:
    property species: Text
    
    action speak:
        display "Animal sound"
    end
end

create container Dog extends Animal:
    property breed: Text
    
    action speak:
        display "Woof!"
    end
end

create new Dog as buddy:
    species = "Canine"
    breed = "Golden Retriever"

buddy.speak()
display buddy.species
"#;
    
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse program");
    
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&program).await;
    
    assert!(result.is_ok(), "Container inheritance should work");
}

#[tokio::test]
async fn test_undefined_method_call_failure() {
    let input = r#"
create container Person:
    property name: Text
end

create new Person as alice:
    name = "Alice"

alice.undefined_method()
"#;
    
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse program");
    
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&program).await;
    
    assert!(result.is_err(), "Calling undefined method should fail");
}

#[tokio::test]
async fn test_undefined_property_access_failure() {
    let input = r#"
create container Person:
    property name: Text
end

create new Person as alice:
    name = "Alice"

display alice.undefined_property
"#;
    
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse program");
    
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&program).await;
    
    assert!(result.is_err(), "Accessing undefined property should fail");
}

#[tokio::test]
async fn test_static_member_access() {
    let input = r#"
create container Math:
    static property PI: Number = 3.14159
    
    static action square needs value: Number: Number
        return value * value
    end
end

display Math.PI
store Math.square(5) in result
display result
"#;
    
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse program");
    
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&program).await;
    
    assert!(result.is_ok(), "Static member access should work");
}

#[tokio::test]
async fn test_interface_implementation() {
    let input = r#"
create interface Drawable:
    action draw:
end

create container Circle implements Drawable:
    property radius: Number
    
    action draw:
        display "Drawing a circle with radius " + this.radius
    end
end

create new Circle as c:
    radius = 5

c.draw()
"#;
    
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().expect("Failed to tokenize");
    let mut parser = Parser::new(tokens);
    let program = parser.parse().expect("Failed to parse program");
    
    let mut interpreter = Interpreter::new();
    let result = interpreter.interpret(&program).await;
    
    assert!(result.is_ok(), "Interface implementation should work");
}
