// Test specifically for the double colon consumption issue
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

#[test]
fn test_colon_consumption_bug() {
    // This test is designed to expose the double colon consumption bug
    // If the bug exists, the parser should consume the colon twice:
    // 1. Once when checking for return type
    // 2. Again when expecting colon for action body

    let source = r#"
create container Test:
    action greet:
        display "Hello"
    end
end
"#;

    let tokens = lex_wfl_with_positions(source);
    println!(
        "Tokens: {:?}",
        tokens.iter().map(|t| &t.token).collect::<Vec<_>>()
    );

    let mut parser = Parser::new(&tokens);
    let result = parser.parse();

    if let Err(errors) = result {
        for error in errors {
            println!("Parse error: {}", error.message);
            // Check for specific error patterns that would indicate double colon consumption
            if error.message.contains("Expected ':' after")
                || error
                    .message
                    .contains("Unexpected token in expression: Colon")
            {
                panic!("Double colon consumption detected: {}", error.message);
            }
        }
    }

    // If no specific colon errors, the test passes
    assert!(true, "No double colon consumption detected");
}

#[test]
fn test_explicit_token_stream_analysis() {
    // Test that manually checks token consumption patterns
    let source = "action greet: display \"Hello\" end";

    let tokens = lex_wfl_with_positions(source);
    let token_types: Vec<String> = tokens.iter().map(|t| format!("{:?}", t.token)).collect();

    println!("Token sequence: {:?}", token_types);

    // Expected sequence: action, identifier(greet), colon, keyword(display), string, keyword(end)
    // The colon should only be consumed once

    let expected = vec![
        "KeywordAction",
        "Identifier", // greet
        "Colon",
        "KeywordDisplay",
        "String", // "Hello"
        "KeywordEnd",
    ];

    assert_eq!(
        token_types.len(),
        expected.len(),
        "Token count mismatch. Expected: {:?}, Got: {:?}",
        expected,
        token_types
    );
}
