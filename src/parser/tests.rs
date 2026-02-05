use super::*;
use crate::lexer::lex_wfl_with_positions;

#[test]
fn parses_concatenation_correctly() {
    let input = r#"store updatedLog as currentLog with message_text with "\n""#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(result.is_ok());

    if let Ok(Statement::VariableDeclaration { value, .. }) = result {
        // The outer expression should be a Concatenation
        if let Expression::Concatenation { left, right, .. } = value {
            // Left side of the outer concatenation should be a variable
            if let Expression::Variable(var_name, ..) = *left {
                assert_eq!(
                    var_name, "currentLog",
                    "Left side should be variable 'currentLog'"
                );
            } else {
                panic!("Left side of concatenation should be a Variable, not {left:?}");
            }

            // Right side of the outer concatenation should be another concatenation
            if let Expression::Concatenation {
                left: inner_left,
                right: inner_right,
                ..
            } = *right
            {
                // Inner left should be a variable
                if let Expression::Variable(var_name, ..) = *inner_left {
                    assert_eq!(
                        var_name, "message_text",
                        "Left side should be variable 'message_text'"
                    );
                } else {
                    panic!("Inner left side should be a Variable, not {inner_left:?}");
                }

                // Inner right should be a string literal with actual newline
                if let Expression::Literal(Literal::String(s), ..) = *inner_right {
                    assert_eq!(
                        s.as_ref(),
                        "\n",
                        "Right side should be string with actual newline"
                    );
                } else {
                    panic!("Inner right side should be a String literal, not {inner_right:?}");
                }
            } else if let Expression::Variable(var_name, ..) = *right {
                // For simple concatenation, right side could be just the variable
                assert_eq!(
                    var_name, "message_text",
                    "Right side should be variable 'message_text'"
                );
            } else {
                panic!("Right side should be a Variable or Concatenation, not {right:?}");
            }
        } else {
            panic!("Expected Concatenation expression, got: {value:?}");
        }
    } else {
        panic!("Expected VariableDeclaration, got: {result:?}");
    }
}

#[test]
fn test_parse_variable_declaration() {
    let input = "store greeting as \"Hello, World!\"";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(result.is_ok());

    if let Ok(Statement::VariableDeclaration { name, value, .. }) = result {
        assert_eq!(name, "greeting");
        if let Expression::Literal(Literal::String(s), ..) = value {
            assert_eq!(s.as_ref(), "Hello, World!");
        } else {
            panic!("Expected string literal");
        }
    } else {
        panic!("Expected variable declaration");
    }
}

#[test]
fn test_parse_if_statement() {
    let input = "check if x is equal to 10:\n  display \"x is 10\"\notherwise:\n  display \"x is not 10\"\nend check";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(result.is_ok());

    if let Ok(Statement::IfStatement {
        condition,
        then_block,
        else_block,
        ..
    }) = result
    {
        if let Expression::BinaryOperation {
            left,
            operator,
            right,
            ..
        } = condition
        {
            if let Expression::Variable(name, ..) = *left {
                assert_eq!(name, "x");
            } else {
                panic!("Expected variable in condition");
            }

            assert_eq!(operator, Operator::Equals);

            if let Expression::Literal(Literal::Integer(n), ..) = *right {
                assert_eq!(n, 10);
            } else {
                panic!("Expected integer literal in condition");
            }
        } else {
            panic!("Expected binary operation in condition");
        }

        assert_eq!(then_block.len(), 1);
        if let Statement::DisplayStatement { value, .. } = &then_block[0] {
            if let Expression::Literal(Literal::String(s), ..) = value {
                assert_eq!(s.as_ref(), "x is 10");
            } else {
                panic!("Expected string literal in then block");
            }
        } else {
            panic!("Expected display statement in then block");
        }

        assert!(else_block.is_some());
        let else_stmts = else_block.as_ref().unwrap();
        assert_eq!(else_stmts.len(), 1);
        if let Statement::DisplayStatement { value, .. } = &else_stmts[0] {
            if let Expression::Literal(Literal::String(s), ..) = value {
                assert_eq!(s.as_ref(), "x is not 10");
            } else {
                panic!("Expected string literal in else block");
            }
        } else {
            panic!("Expected display statement in else block");
        }
    } else {
        panic!("Expected if statement");
    }
}

#[test]
fn test_parse_expression() {
    let input = "5 plus 3 times 2";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_expression();
    assert!(result.is_ok());

    if let Ok(Expression::BinaryOperation {
        left,
        operator,
        right,
        ..
    }) = result
    {
        if let Expression::Literal(Literal::Integer(n), ..) = *left {
            assert_eq!(n, 5);
        } else {
            panic!("Expected integer literal");
        }

        assert_eq!(operator, Operator::Plus);

        if let Expression::BinaryOperation {
            left,
            operator,
            right,
            ..
        } = *right
        {
            if let Expression::Literal(Literal::Integer(n), ..) = *left {
                assert_eq!(n, 3);
            } else {
                panic!("Expected integer literal");
            }

            assert_eq!(operator, Operator::Multiply);

            if let Expression::Literal(Literal::Integer(n), ..) = *right {
                assert_eq!(n, 2);
            } else {
                panic!("Expected integer literal");
            }
        } else {
            panic!("Expected binary operation");
        }
    } else {
        panic!("Expected binary operation");
    }
}

#[test]
fn test_parse_wait_for_open_file() {
    {
        let input = r#"open file at "data.txt" and read content as content"#;
        let tokens = lex_wfl_with_positions(input);
        let mut parser = Parser::new(&tokens);

        println!("Testing open file statement:");
        for (i, token) in tokens.iter().enumerate() {
            println!("{i}: {token:?}");
        }

        let result = parser.parse_statement();
        if let Err(ref e) = result {
            println!("Parse error for open file: {e:?}");
        } else {
            println!("Successfully parsed open file statement");
        }
        assert!(result.is_ok());
    }

    // Test the new syntax: "open file at "path" as variable"
    {
        let input = r#"open file at "nexus.log" as logHandle"#;
        let tokens = lex_wfl_with_positions(input);
        let mut parser = Parser::new(&tokens);

        println!("\nTesting new open file syntax:");
        for (i, token) in tokens.iter().enumerate() {
            println!("{i}: {token:?}");
        }

        let result = parser.parse_statement();
        if let Err(ref e) = result {
            println!("Parse error for new open file syntax: {e:?}");
        } else {
            println!("Successfully parsed new open file syntax");
        }
        assert!(result.is_ok());

        if let Ok(Statement::OpenFileStatement {
            path,
            variable_name,
            ..
        }) = result
        {
            if let Expression::Literal(Literal::String(s), ..) = path {
                assert_eq!(s.as_ref(), "nexus.log");
            } else {
                panic!("Expected string literal for path");
            }
            assert_eq!(variable_name, "logHandle");
        } else {
            panic!("Expected OpenFileStatement");
        }
    }

    {
        let input = r#"wait for open file at "data.txt" and read content as content"#;
        let tokens = lex_wfl_with_positions(input);
        let mut parser = Parser::new(&tokens);

        println!("\nTesting wait for statement:");
        for (i, token) in tokens.iter().enumerate() {
            println!("{i}: {token:?}");
        }

        let result = parser.parse_statement();
        if let Err(ref e) = result {
            println!("Parse error for wait for: {e:?}");
        } else {
            println!("Successfully parsed wait for statement");
        }
        assert!(result.is_ok());

        if let Ok(Statement::WaitForStatement { inner, .. }) = result {
            if let Statement::ReadFileStatement {
                path,
                variable_name,
                ..
            } = *inner
            {
                if let Expression::Literal(Literal::String(s), ..) = path {
                    assert_eq!(s.as_ref(), "data.txt");
                } else {
                    panic!("Expected string literal for path");
                }
                assert_eq!(variable_name, "content");
            } else {
                panic!("Expected ReadFileStatement");
            }
        } else {
            panic!("Expected WaitForStatement");
        }
    }
}

#[test]
fn test_missing_as_in_store_statement() {
    let input = "store greeting 42";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(result.is_err());

    if let Err(error) = result {
        assert!(error.message.contains("Expected 'as' after variable name"));
        assert!(error.message.contains("42"));
    }
}

#[test]
fn test_missing_as_in_create_statement() {
    let input = "create user \"John\"";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(result.is_err());

    if let Err(error) = result {
        assert!(error.message.contains("Expected 'as' after variable name"));
        assert!(error.message.contains("StringLiteral"));
    }
}

#[test]
fn test_missing_to_in_change_statement() {
    let input = "change counter 10";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(result.is_err());

    if let Err(error) = result {
        assert!(error.message.contains("Expected 'to' after identifier(s)"));
        assert!(error.message.contains("10"));
    }
}

#[test]
fn test_valid_store_statements() {
    let input = "store x as 1";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();
    assert!(result.is_ok());

    let input = "store first name as \"Alice\"";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();
    assert!(result.is_ok());
}

#[test]
fn test_store_without_variable_name() {
    let input = "store";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(
            e[0].message.contains("Expected variable name"),
            "Got error: {}",
            e[0]
        );
    }
}

#[test]
fn test_store_with_incomplete_statement() {
    let input = "store a";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(
            e[0].message.contains("Expected 'as'"),
            "Got error: {}",
            e[0]
        );
    }
}

#[test]
fn test_store_with_missing_as() {
    let input = "store a a";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(
            e[0].message.contains("Expected 'as'"),
            "Got error: {}",
            e[0]
        );
    }
}

#[test]
fn test_store_with_number_as_variable_name() {
    let input = "store 1 as 1";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(
            e[0].message
                .contains("Cannot use a number as a variable name"),
            "Got error: {}",
            e[0]
        );
    }
}

#[test]
fn test_store_with_number_as_variable_name_without_as() {
    let input = "store 1 b";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(
            e[0].message
                .contains("Cannot use a number as a variable name"),
            "Got error: {}",
            e[0]
        );
    }
}

#[test]
fn test_store_with_keyword_as_variable_name() {
    let input = "store if as 1";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse();
    assert!(result.is_err());

    if let Err(e) = result {
        assert!(
            e[0].message.contains("Cannot use reserved keyword"),
            "Got error: {}",
            e[0]
        );
    }
}

#[test]
fn test_than_keyword_parsing() {
    let input = "check if x is greater than 5:\n  display \"x is greater than 5\"\nend check";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(result.is_ok());

    if let Ok(Statement::IfStatement { condition, .. }) = result {
        if let Expression::BinaryOperation {
            operator, right, ..
        } = condition
        {
            assert_eq!(operator, Operator::GreaterThan);

            if let Expression::Literal(Literal::Integer(n), ..) = *right {
                assert_eq!(n, 5);
            } else {
                panic!("Expected integer literal in condition");
            }
        } else {
            panic!("Expected binary operation in condition");
        }
    } else {
        panic!("Expected if statement");
    }
}

#[test]
fn test_parse_simple_pattern_definition() {
    let input = r#"create pattern greeting:
    "hello"
end pattern"#;

    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(result.is_ok(), "Failed to parse simple pattern: {result:?}");

    if let Ok(Statement::PatternDefinition { name, pattern, .. }) = result {
        assert_eq!(name, "greeting");
        if let PatternExpression::Literal(s) = pattern {
            assert_eq!(s, "hello");
        } else {
            panic!("Expected literal pattern, got {pattern:?}");
        }
    } else {
        panic!("Expected PatternDefinition, got {result:?}");
    }
}

#[test]
fn test_parse_character_class_pattern() {
    let input = r#"create pattern phone:
    digit digit digit
end pattern"#;

    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse character class pattern: {result:?}"
    );

    if let Ok(Statement::PatternDefinition { name, pattern, .. }) = result {
        assert_eq!(name, "phone");
        if let PatternExpression::Sequence(elements) = pattern {
            assert_eq!(elements.len(), 3);
            for element in elements {
                if let PatternExpression::CharacterClass(CharClass::Digit) = element {
                    // Correct
                } else {
                    panic!("Expected digit character class, got {element:?}");
                }
            }
        } else {
            panic!("Expected sequence pattern, got {pattern:?}");
        }
    } else {
        panic!("Expected PatternDefinition, got {result:?}");
    }
}

#[test]
fn test_parse_quantified_pattern() {
    let input = r#"create pattern flexible:
    one or more digit
end pattern"#;

    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse quantified pattern: {result:?}"
    );

    if let Ok(Statement::PatternDefinition { name, pattern, .. }) = result {
        assert_eq!(name, "flexible");
        if let PatternExpression::Quantified {
            pattern: inner,
            quantifier,
        } = pattern
        {
            if let PatternExpression::CharacterClass(CharClass::Digit) = inner.as_ref() {
                // Correct
            } else {
                panic!("Expected digit character class, got {inner:?}");
            }
            if let Quantifier::OneOrMore = quantifier {
                // Correct
            } else {
                panic!("Expected OneOrMore quantifier, got {quantifier:?}");
            }
        } else {
            panic!("Expected quantified pattern, got {pattern:?}");
        }
    } else {
        panic!("Expected PatternDefinition, got {result:?}");
    }
}

/// Tests parsing of a pattern definition with alternative patterns.
///
/// Verifies that the parser correctly recognizes a pattern definition containing an alternative between two string literals, ensuring the resulting AST represents an `Alternative` with the expected literals.
#[allow(dead_code)]
fn test_parse_alternative_pattern() {
    let input = r#"create pattern greeting:
    "hello" or "hi"
end pattern"#;

    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse alternative pattern: {result:?}"
    );

    if let Ok(Statement::PatternDefinition { name, pattern, .. }) = result {
        assert_eq!(name, "greeting");
        if let PatternExpression::Alternative(alternatives) = pattern {
            assert_eq!(alternatives.len(), 2);
            if let PatternExpression::Literal(s1) = &alternatives[0] {
                assert_eq!(s1, "hello");
            } else {
                let alt = &alternatives[0];
                panic!("Expected first alternative to be 'hello', got {alt:?}");
            }
            if let PatternExpression::Literal(s2) = &alternatives[1] {
                assert_eq!(s2, "hi");
            } else {
                let alt = &alternatives[1];
                panic!("Expected second alternative to be 'hi', got {alt:?}");
            }
        } else {
            panic!("Expected alternative pattern, got {pattern:?}");
        }
    } else {
        panic!("Expected PatternDefinition, got {result:?}");
    }
}

/// Tests that chained binary operations are parsed as left-associative.
///
/// Parses the statement `"store result as 1 plus 2 plus 3"` and asserts that the resulting AST represents the expression as `(1 + 2) + 3`, confirming correct left-associativity of binary operations.
///
/// # Examples
///
/// ```
/// test_chained_binary_operations_parsing();
/// // Should not panic; verifies AST structure for left-associativity.
/// ```
#[test]
fn test_chained_binary_operations_parsing() {
    let input = "store result as 1 plus 2 plus 3";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let program = parser.parse().unwrap();
    assert_eq!(program.statements.len(), 1);

    if let Statement::VariableDeclaration { value, .. } = &program.statements[0] {
        // The AST should be: BinaryOperation(BinaryOperation(1, Plus, 2), Plus, 3)
        // This represents (1 + 2) + 3, which is correct for left-associativity
        if let Expression::BinaryOperation {
            left,
            operator,
            right,
            ..
        } = value
        {
            assert_eq!(operator, &Operator::Plus);

            // Left should be another binary operation: (1 + 2)
            if let Expression::BinaryOperation {
                left: inner_left,
                operator: inner_op,
                right: inner_right,
                ..
            } = left.as_ref()
            {
                assert_eq!(inner_op, &Operator::Plus);

                if let (
                    Expression::Literal(Literal::Integer(1), _, _),
                    Expression::Literal(Literal::Integer(2), _, _),
                ) = (inner_left.as_ref(), inner_right.as_ref())
                {
                    // Good, left side is (1 + 2)
                } else {
                    panic!(
                        "Expected inner binary operation to be (1 + 2), got: {inner_left:?} {inner_op:?} {inner_right:?}"
                    );
                }

                // Right should be 3
                if let Expression::Literal(Literal::Integer(3), _, _) = right.as_ref() {
                    // Perfect! This is the correct AST structure for left-associativity
                } else {
                    panic!("Expected right operand to be 3, got: {right:?}");
                }
            } else {
                panic!("Expected left operand to be a binary operation, got: {left:?}");
            }
        } else {
            panic!("Expected variable value to be a binary operation, got: {value:?}");
        }
    } else {
        panic!(
            "Expected variable declaration, got: {:?}",
            program.statements[0]
        );
    }

    println!("✅ Parsing test passed - AST structure is correct for left-associativity");
}

/// Prints the token sequence generated from lexing a sample input string for debugging purposes.
///
/// This test lexes the input `"store result as 1 plus 2 plus 3"` and outputs each token with its index to the console.
///
/// # Examples
///
/// ```
/// debug_token_sequence(); // Prints the token sequence for inspection
/// ```
#[allow(dead_code)]
fn debug_token_sequence() {
    let input = "store result as 1 plus 2 plus 3";
    let tokens = lex_wfl_with_positions(input);

    println!("Input: '{input}'");
    println!("Tokens:");
    for (i, token) in tokens.iter().enumerate() {
        println!("{i}: {token:?}");
    }
}

#[test]
fn test_subtraction_basic() {
    let input = "display 5 - 3";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse 'display 5 - 3': {result:?}"
    );

    if let Ok(Statement::DisplayStatement { value, .. }) = result {
        if let Expression::BinaryOperation {
            left,
            operator,
            right,
            ..
        } = value
        {
            if let Expression::Literal(Literal::Integer(n), ..) = *left {
                assert_eq!(n, 5, "Expected left operand to be 5");
            } else {
                panic!("Expected integer literal 5, got: {left:?}");
            }

            assert_eq!(operator, Operator::Minus, "Expected Minus operator");

            if let Expression::Literal(Literal::Integer(n), ..) = *right {
                assert_eq!(n, 3, "Expected right operand to be 3");
            } else {
                panic!("Expected integer literal 3, got: {right:?}");
            }
        } else {
            panic!("Expected binary operation, got: {value:?}");
        }
    } else {
        panic!("Expected display statement, got: {result:?}");
    }
}

#[test]
fn test_subtraction_with_negative() {
    let input = "display 5 - -3";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse 'display 5 - -3': {result:?}"
    );

    if let Ok(Statement::DisplayStatement { value, .. }) = result {
        if let Expression::BinaryOperation {
            left,
            operator,
            right,
            ..
        } = value
        {
            if let Expression::Literal(Literal::Integer(n), ..) = *left {
                assert_eq!(n, 5, "Expected left operand to be 5");
            } else {
                panic!("Expected integer literal 5, got: {left:?}");
            }

            assert_eq!(operator, Operator::Minus, "Expected Minus operator");

            // Right side should be unary minus with 3
            if let Expression::UnaryOperation {
                operator: unary_op,
                expression,
                ..
            } = *right
            {
                assert_eq!(
                    unary_op,
                    UnaryOperator::Minus,
                    "Expected unary minus operator"
                );

                if let Expression::Literal(Literal::Integer(n), ..) = *expression {
                    assert_eq!(n, 3, "Expected operand to be 3");
                } else {
                    panic!("Expected integer literal 3, got: {expression:?}");
                }
            } else {
                panic!("Expected unary operation, got: {right:?}");
            }
        } else {
            panic!("Expected binary operation, got: {value:?}");
        }
    } else {
        panic!("Expected display statement, got: {result:?}");
    }
}

#[test]
fn test_unary_minus_with_complex_expression() {
    let input = "display -(1 + 2) times 3";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse 'display -(1 + 2) times 3': {result:?}"
    );

    if let Ok(Statement::DisplayStatement { value, .. }) = result {
        // Should be: (-(1 + 2)) * 3
        if let Expression::BinaryOperation {
            left,
            operator,
            right,
            ..
        } = value
        {
            assert_eq!(operator, Operator::Multiply, "Expected Multiply operator");

            // Left side should be unary minus with (1 + 2)
            if let Expression::UnaryOperation {
                operator: unary_op,
                expression,
                ..
            } = *left
            {
                assert_eq!(
                    unary_op,
                    UnaryOperator::Minus,
                    "Expected unary minus operator"
                );

                // expression should be (1 + 2)
                if let Expression::BinaryOperation {
                    left: inner_left,
                    operator: inner_op,
                    right: inner_right,
                    ..
                } = *expression
                {
                    assert_eq!(inner_op, Operator::Plus, "Expected Plus operator");

                    if let Expression::Literal(Literal::Integer(n), ..) = *inner_left {
                        assert_eq!(n, 1, "Expected left operand to be 1");
                    } else {
                        panic!("Expected integer literal 1, got: {inner_left:?}");
                    }

                    if let Expression::Literal(Literal::Integer(n), ..) = *inner_right {
                        assert_eq!(n, 2, "Expected right operand to be 2");
                    } else {
                        panic!("Expected integer literal 2, got: {inner_right:?}");
                    }
                } else {
                    panic!("Expected binary operation (1 + 2), got: {expression:?}");
                }
            } else {
                panic!("Expected unary operation, got: {left:?}");
            }

            // Right side should be 3
            if let Expression::Literal(Literal::Integer(n), ..) = *right {
                assert_eq!(n, 3, "Expected right operand to be 3");
            } else {
                panic!("Expected integer literal 3, got: {right:?}");
            }
        } else {
            panic!("Expected binary operation, got: {value:?}");
        }
    } else {
        panic!("Expected display statement, got: {result:?}");
    }
}
#[test]
fn test_bracket_array_indexing() {
    // Test basic bracket indexing with integer literal
    let input = r#"display args[0]"#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse basic bracket indexing: {result:?}"
    );

    if let Ok(Statement::DisplayStatement { value, .. }) = result {
        if let Expression::IndexAccess {
            collection, index, ..
        } = value
        {
            // Collection should be a variable "args"
            if let Expression::Variable(var_name, ..) = *collection {
                assert_eq!(var_name, "args", "Expected collection to be 'args'");
            } else {
                panic!("Expected collection to be Variable, got: {collection:?}");
            }

            // Index should be integer literal 0
            if let Expression::Literal(Literal::Integer(n), ..) = *index {
                assert_eq!(n, 0, "Expected index to be 0");
            } else {
                panic!("Expected index to be integer literal 0, got: {index:?}");
            }
        } else {
            panic!("Expected IndexAccess expression, got: {value:?}");
        }
    } else {
        panic!("Expected DisplayStatement, got: {result:?}");
    }
}

#[test]
fn test_bracket_array_indexing_with_variable() {
    // Test bracket indexing with variable as index
    let input = r#"display args[last_index]"#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse bracket indexing with variable: {result:?}"
    );

    if let Ok(Statement::DisplayStatement { value, .. }) = result {
        if let Expression::IndexAccess {
            collection, index, ..
        } = value
        {
            // Collection should be a variable "args"
            if let Expression::Variable(var_name, ..) = *collection {
                assert_eq!(var_name, "args", "Expected collection to be 'args'");
            } else {
                panic!("Expected collection to be Variable, got: {collection:?}");
            }

            // Index should be variable "last_index"
            if let Expression::Variable(var_name, ..) = *index {
                assert_eq!(var_name, "last_index", "Expected index to be 'last_index'");
            } else {
                panic!("Expected index to be variable 'last_index', got: {index:?}");
            }
        } else {
            panic!("Expected IndexAccess expression, got: {value:?}");
        }
    } else {
        panic!("Expected DisplayStatement, got: {result:?}");
    }
}

#[test]
fn test_bracket_array_indexing_with_expression() {
    // Test bracket indexing with expression as index
    let input = r#"display my_list[count minus 1]"#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse bracket indexing with expression: {result:?}"
    );

    if let Ok(Statement::DisplayStatement { value, .. }) = result {
        if let Expression::IndexAccess {
            collection, index, ..
        } = value
        {
            // Collection should be a variable "my_list"
            if let Expression::Variable(var_name, ..) = *collection {
                assert_eq!(var_name, "my_list", "Expected collection to be 'my_list'");
            } else {
                panic!("Expected collection to be Variable, got: {collection:?}");
            }

            // Index should be a binary operation "count minus 1"
            if let Expression::BinaryOperation {
                left,
                operator,
                right,
                ..
            } = *index
            {
                // Left should be variable "count"
                if let Expression::Variable(var_name, ..) = *left {
                    assert_eq!(var_name, "count", "Expected left operand to be 'count'");
                } else {
                    panic!("Expected left operand to be Variable, got: {left:?}");
                }

                // Operator should be Minus
                assert_eq!(operator, Operator::Minus, "Expected operator to be Minus");

                // Right should be integer literal 1
                if let Expression::Literal(Literal::Integer(n), ..) = *right {
                    assert_eq!(n, 1, "Expected right operand to be 1");
                } else {
                    panic!("Expected right operand to be integer literal 1, got: {right:?}");
                }
            } else {
                panic!("Expected index to be BinaryOperation, got: {index:?}");
            }
        } else {
            panic!("Expected IndexAccess expression, got: {value:?}");
        }
    } else {
        panic!("Expected DisplayStatement, got: {result:?}");
    }
}

/// Tests that reserved pattern names show helpful error messages instead of generic parsing errors.
///
/// Verifies that when a user tries to create a pattern with a reserved keyword name like 'url',
/// the parser returns a specific error message indicating that the name is predefined.
#[test]
fn test_reserved_pattern_name_error_message() {
    // Test with 'url' - should show helpful error
    let input_url = r#"create pattern url:
    "test"
end pattern"#;

    let tokens = lex_wfl_with_positions(input_url);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_err(),
        "Expected parsing to fail for reserved name 'url'"
    );

    let error = result.unwrap_err();
    assert!(
        error
            .message
            .contains("'url' is a predefined pattern in WFL"),
        "Expected helpful error message, got: {}",
        error.message
    );

    // Test with 'digit' - should show helpful error
    let input_digit = r#"create pattern digit:
    "0" or "1"
end pattern"#;

    let tokens = lex_wfl_with_positions(input_digit);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_err(),
        "Expected parsing to fail for reserved name 'digit'"
    );

    let error = result.unwrap_err();
    assert!(
        error
            .message
            .contains("'digit' is a predefined pattern in WFL"),
        "Expected helpful error message, got: {}",
        error.message
    );

    // Test with valid name - should work fine
    let input_valid = r#"create pattern my_url:
    "test"
end pattern"#;

    let tokens = lex_wfl_with_positions(input_valid);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Expected parsing to succeed for valid name 'my_url'"
    );
}

/// Tests that reserved pattern names only produce one error, not cascading errors
/// from the pattern body being parsed as regular statements.
#[test]
fn test_reserved_pattern_single_error() {
    let input = r#"create pattern url:
    "test"
    "://"
    one or more letter or digit or "-" or "."
    "/"
end pattern

display "This should work""#;

    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse();
    assert!(
        result.is_err(),
        "Expected parsing to fail for reserved name 'url'"
    );

    let errors = result.unwrap_err();
    // Should only have one error (the reserved pattern name), not multiple
    assert_eq!(
        errors.len(),
        1,
        "Expected exactly one error, got: {:?}",
        errors
    );

    assert!(
        errors[0]
            .message
            .contains("'url' is a predefined pattern in WFL"),
        "Expected helpful error message, got: {}",
        errors[0].message
    );
}

/// Tests that list references in patterns are parsed correctly as ListReference AST nodes.
#[test]
fn test_pattern_list_reference_parsing() {
    let input = r#"create pattern my_pattern:
    my_list
    "://"
    another_list
end pattern"#;

    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Expected parsing to succeed for list references"
    );

    if let Ok(Statement::PatternDefinition { pattern, .. }) = result {
        // The pattern should be a sequence containing list references and a literal
        if let PatternExpression::Sequence(elements) = pattern {
            assert_eq!(elements.len(), 3, "Expected 3 elements in pattern sequence");

            // First element should be a list reference
            if let PatternExpression::ListReference(name) = &elements[0] {
                assert_eq!(name, "my_list", "Expected list reference to 'my_list'");
            } else {
                panic!(
                    "Expected first element to be ListReference, got: {:?}",
                    elements[0]
                );
            }

            // Second element should be a literal
            if let PatternExpression::Literal(text) = &elements[1] {
                assert_eq!(text, "://", "Expected literal ':://'");
            } else {
                panic!(
                    "Expected second element to be Literal, got: {:?}",
                    elements[1]
                );
            }

            // Third element should be another list reference
            if let PatternExpression::ListReference(name) = &elements[2] {
                assert_eq!(
                    name, "another_list",
                    "Expected list reference to 'another_list'"
                );
            } else {
                panic!(
                    "Expected third element to be ListReference, got: {:?}",
                    elements[2]
                );
            }
        } else {
            panic!("Expected pattern to be a Sequence, got: {:?}", pattern);
        }
    } else {
        panic!("Expected PatternDefinition statement, got: {:?}", result);
    }
}

// Phase 2: Eol Token Parser Tests

#[test]
fn test_eol_prevents_multiline_expression() {
    use crate::lexer::lex_wfl_with_positions;

    // This should NOT parse as x = 1 + 2
    let input = "store x as 1 plus\n2";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse();

    // Should either error or parse as incomplete
    // NOT as a valid x = 1 + 2 expression
    assert!(
        result.is_err() || !parser.errors.is_empty(),
        "Multi-line expression should not be allowed"
    );
}

#[test]
fn test_sameline_index_access() {
    use crate::lexer::lex_wfl_with_positions;

    // Same line - should parse as index access
    let input = "store x as numbers 0";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let program = parser.parse().expect("Should parse successfully");

    // Verify AST contains index access
    assert_eq!(program.statements.len(), 1);
    // The statement should be a variable declaration with an index access expression
}

#[test]
fn test_crossline_not_index_access() {
    use crate::lexer::lex_wfl_with_positions;

    // Different lines - should NOT parse as index access
    let input = "display numbers\n0";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let program = parser.parse().expect("Should parse successfully");

    // Should parse as two separate statements
    assert_eq!(
        program.statements.len(),
        2,
        "Should be 2 statements, not index access"
    );
}

#[test]
fn test_blank_lines_allowed() {
    use crate::lexer::lex_wfl_with_positions;

    let input = "store x as 5\n\n\nstore y as 10\n";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let program = parser.parse().expect("Blank lines should be allowed");
    assert_eq!(program.statements.len(), 2);
}

// ===== Phase 4: Tests for 'call' keyword syntax =====

#[test]
fn test_call_syntax_basic() {
    let input = r#"call greet with "Alice""#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(result.is_ok(), "Should parse call statement successfully");

    if let Ok(Statement::ExpressionStatement { expression, .. }) = result {
        if let Expression::ActionCall {
            name, arguments, ..
        } = expression
        {
            assert_eq!(name, "greet", "Action name should be 'greet'");
            assert_eq!(arguments.len(), 1, "Should have 1 argument");

            if let Expression::Literal(Literal::String(s), ..) = &arguments[0].value {
                assert_eq!(s.as_ref(), "Alice", "Argument should be 'Alice'");
            } else {
                panic!("Argument should be a string literal");
            }
        } else {
            panic!("Expected ActionCall expression, got: {expression:?}");
        }
    } else {
        panic!("Expected ExpressionStatement, got: {result:?}");
    }
}

#[test]
fn test_call_syntax_multiple_args() {
    let input = r#"call calculate with 10 and 20 and 30"#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(result.is_ok(), "Should parse call with multiple args");

    if let Ok(Statement::ExpressionStatement { expression, .. }) = result {
        if let Expression::ActionCall {
            name, arguments, ..
        } = expression
        {
            assert_eq!(name, "calculate", "Action name should be 'calculate'");
            assert_eq!(arguments.len(), 3, "Should have 3 arguments");

            // Verify argument values
            for (i, expected) in [10, 20, 30].iter().enumerate() {
                if let Expression::Literal(Literal::Integer(n), ..) = arguments[i].value {
                    assert_eq!(n, *expected, "Argument {} should be {}", i, expected);
                } else {
                    panic!("Argument {} should be an integer literal", i);
                }
            }
        } else {
            panic!("Expected ActionCall expression, got: {expression:?}");
        }
    } else {
        panic!("Expected ExpressionStatement, got: {result:?}");
    }
}

#[test]
fn test_call_syntax_zero_args() {
    let input = r#"call initialize"#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(result.is_ok(), "Should parse call with zero args");

    if let Ok(Statement::ExpressionStatement { expression, .. }) = result {
        if let Expression::ActionCall {
            name, arguments, ..
        } = expression
        {
            assert_eq!(name, "initialize", "Action name should be 'initialize'");
            assert_eq!(arguments.len(), 0, "Should have 0 arguments");
        } else {
            panic!("Expected ActionCall expression, got: {expression:?}");
        }
    } else {
        panic!("Expected ExpressionStatement, got: {result:?}");
    }
}

#[test]
fn test_legacy_builtin_syntax_still_works() {
    let input = r#"print with "Hello""#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Legacy builtin syntax should still work for backward compatibility"
    );

    if let Ok(Statement::ExpressionStatement { expression, .. }) = result {
        if let Expression::ActionCall {
            name, arguments, ..
        } = expression
        {
            assert_eq!(name, "print", "Should parse as ActionCall to 'print'");
            assert_eq!(arguments.len(), 1, "Should have 1 argument");
        } else {
            panic!("Expected ActionCall for builtin function, got: {expression:?}");
        }
    } else {
        panic!("Expected ExpressionStatement, got: {result:?}");
    }
}

#[test]
fn test_concatenation_unchanged() {
    let input = r#"store msg as "Hello" with " World""#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "String concatenation should still work with 'with'"
    );

    if let Ok(Statement::VariableDeclaration { value, .. }) = result {
        if let Expression::Concatenation { left, right, .. } = value {
            // Left should be string "Hello"
            if let Expression::Literal(Literal::String(s), ..) = *left {
                assert_eq!(s.as_ref(), "Hello", "Left side should be 'Hello'");
            } else {
                panic!("Left side should be string literal");
            }

            // Right should be string " World"
            if let Expression::Literal(Literal::String(s), ..) = *right {
                assert_eq!(s.as_ref(), " World", "Right side should be ' World'");
            } else {
                panic!("Right side should be string literal");
            }
        } else {
            panic!("Expected Concatenation expression, got: {value:?}");
        }
    } else {
        panic!("Expected VariableDeclaration, got: {result:?}");
    }
}

#[test]
fn test_call_in_variable_declaration() {
    let input = r#"store result as call factorial with 5"#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(result.is_ok(), "Should parse call in variable declaration");

    if let Ok(Statement::VariableDeclaration { name, value, .. }) = result {
        assert_eq!(name, "result", "Variable name should be 'result'");

        if let Expression::ActionCall {
            name: action_name,
            arguments,
            ..
        } = value
        {
            assert_eq!(
                action_name, "factorial",
                "Action name should be 'factorial'"
            );
            assert_eq!(arguments.len(), 1, "Should have 1 argument");
        } else {
            panic!("Expected ActionCall in value, got: {value:?}");
        }
    } else {
        panic!("Expected VariableDeclaration, got: {result:?}");
    }
}

#[test]
fn test_variable_concatenation_not_action_call() {
    // This tests that variable concatenation doesn't get parsed as action call
    let input = r#"store x as "test"
store y as x with " more""#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let program = parser.parse().expect("Should parse successfully");
    assert_eq!(program.statements.len(), 2, "Should have 2 statements");

    // Second statement should be concatenation (since 'x' is not a builtin)
    if let Statement::VariableDeclaration { value, .. } = &program.statements[1] {
        if let Expression::Concatenation { left, right, .. } = value {
            // Left should be variable 'x'
            if let Expression::Variable(var_name, ..) = &**left {
                assert_eq!(var_name, "x", "Left side should be variable 'x'");
            } else {
                panic!("Left side should be variable 'x'");
            }

            // Right should be string " more"
            if let Expression::Literal(Literal::String(s), ..) = &**right {
                assert_eq!(s.as_ref(), " more", "Right side should be ' more'");
            } else {
                panic!("Right side should be string literal");
            }
        } else {
            panic!("Expected Concatenation (not ActionCall) for variable 'x', got: {value:?}");
        }
    } else {
        panic!("Second statement should be VariableDeclaration");
    }
}

// ===== Tests for "otherwise check if" else-if chain pattern =====

#[test]
fn test_otherwise_check_if_chain() {
    // Test the "otherwise check if" pattern (else-if chain without colon)
    let input = r#"check if x is equal to 1:
    display "one"
otherwise check if x is equal to 2:
    display "two"
otherwise:
    display "other"
end check"#;

    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Should parse 'otherwise check if' chain successfully: {:?}",
        result.err()
    );

    if let Ok(Statement::IfStatement {
        then_block,
        else_block,
        ..
    }) = result
    {
        // Then block should have one statement
        assert_eq!(then_block.len(), 1, "Then block should have 1 statement");

        // Else block should exist and contain a nested if
        let else_stmts = else_block.expect("Should have else block");
        assert_eq!(else_stmts.len(), 1, "Else block should have 1 statement");

        // The else statement should be another IfStatement (the chained else-if)
        if let Statement::IfStatement {
            then_block: inner_then,
            else_block: inner_else,
            ..
        } = &else_stmts[0]
        {
            assert_eq!(
                inner_then.len(),
                1,
                "Inner then block should have 1 statement"
            );

            // Inner else block should exist (the final "otherwise:" clause)
            let inner_else_stmts = inner_else.as_ref().expect("Should have inner else block");
            assert_eq!(
                inner_else_stmts.len(),
                1,
                "Inner else block should have 1 statement"
            );
        } else {
            panic!(
                "Expected nested IfStatement in else block, got: {:?}",
                else_stmts[0]
            );
        }
    } else {
        panic!("Expected IfStatement, got: {:?}", result);
    }
}

#[test]
fn test_otherwise_check_if_in_action() {
    // Test that "otherwise check if" works inside action definitions
    let input = r#"define action called classify with value:
    check if value is less than 0:
        give back "negative"
    otherwise check if value is equal to 0:
        give back "zero"
    otherwise:
        give back "positive"
    end check
end action"#;

    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Should parse 'otherwise check if' inside action: {:?}",
        result.err()
    );

    if let Ok(Statement::ActionDefinition { body, .. }) = result {
        assert_eq!(body.len(), 1, "Action body should have 1 statement");

        // The body should contain an IfStatement with chained else-if
        if let Statement::IfStatement { else_block, .. } = &body[0] {
            let else_stmts = else_block.as_ref().expect("Should have else block");
            assert_eq!(else_stmts.len(), 1, "Else block should have 1 statement");

            // Should be a nested IfStatement
            assert!(
                matches!(else_stmts[0], Statement::IfStatement { .. }),
                "Else block should contain nested IfStatement"
            );
        } else {
            panic!("Expected IfStatement in action body, got: {:?}", body[0]);
        }
    } else {
        panic!("Expected ActionDefinition, got: {:?}", result);
    }
}

#[test]
fn test_otherwise_colon_check_if_still_works() {
    // Test that the traditional nested syntax still works
    let input = r#"check if x is equal to 1:
    display "one"
otherwise:
    check if x is equal to 2:
        display "two"
    end check
end check"#;

    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Traditional 'otherwise: check if' syntax should still work: {:?}",
        result.err()
    );
}

#[test]
fn test_deep_else_if_chain() {
    // Test multiple chained else-if statements
    let input = r#"check if x is equal to 1:
    display "one"
otherwise check if x is equal to 2:
    display "two"
otherwise check if x is equal to 3:
    display "three"
otherwise check if x is equal to 4:
    display "four"
otherwise:
    display "other"
end check"#;

    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Should parse deep else-if chain: {:?}",
        result.err()
    );

    // Verify the structure: each else block should contain a single nested IfStatement
    if let Ok(Statement::IfStatement { else_block, .. }) = result {
        let mut current_else = else_block;
        let mut depth = 0;

        while let Some(stmts) = current_else {
            depth += 1;
            if depth > 4 {
                break; // Prevent infinite loop
            }

            if stmts.len() == 1 {
                if let Statement::IfStatement { else_block, .. } = &stmts[0] {
                    current_else = else_block.clone();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        assert_eq!(depth, 4, "Should have 4 levels of else-if chaining");
    }
}

/// Regression test for character class union parsing.
/// Verifies that "letter or digit" is parsed as a single Alternative pattern
/// rather than being split across top-level alternatives.
/// See: https://github.com/WebFirstLanguage/wfl/pull/270
#[test]
fn test_character_class_union_parsing() {
    let input = r#"create pattern alphanum:
    one or more letter or digit
end pattern"#;

    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse character class union pattern: {result:?}"
    );

    if let Ok(Statement::PatternDefinition { name, pattern, .. }) = result {
        assert_eq!(name, "alphanum");
        // The pattern should be: Quantified { OneOrMore, Alternative([Letter, Digit]) }
        if let PatternExpression::Quantified {
            pattern: inner,
            quantifier,
        } = pattern
        {
            assert!(
                matches!(quantifier, Quantifier::OneOrMore),
                "Expected OneOrMore quantifier, got {quantifier:?}"
            );
            if let PatternExpression::Alternative(alternatives) = inner.as_ref() {
                assert_eq!(
                    alternatives.len(),
                    2,
                    "Expected 2 alternatives in character class union"
                );
                assert!(
                    matches!(
                        &alternatives[0],
                        PatternExpression::CharacterClass(CharClass::Letter)
                    ),
                    "First alternative should be Letter, got {:?}",
                    alternatives[0]
                );
                assert!(
                    matches!(
                        &alternatives[1],
                        PatternExpression::CharacterClass(CharClass::Digit)
                    ),
                    "Second alternative should be Digit, got {:?}",
                    alternatives[1]
                );
            } else {
                panic!("Expected Alternative pattern inside quantifier, got {inner:?}");
            }
        } else {
            panic!("Expected Quantified pattern, got {pattern:?}");
        }
    } else {
        panic!("Expected PatternDefinition, got {result:?}");
    }
}

/// Regression test for Unicode character class union parsing.
/// Verifies that "unicode letter or unicode digit" is parsed as a single Alternative pattern.
#[test]
fn test_unicode_character_class_union_parsing() {
    let input = r#"create pattern unicode_alphanum:
    one or more unicode letter or unicode digit
end pattern"#;

    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse unicode character class union pattern: {result:?}"
    );

    if let Ok(Statement::PatternDefinition { name, pattern, .. }) = result {
        assert_eq!(name, "unicode_alphanum");
        // The pattern should be: Quantified { OneOrMore, Alternative([UnicodeProperty("Alphabetic"), UnicodeProperty("Numeric")]) }
        if let PatternExpression::Quantified {
            pattern: inner,
            quantifier,
        } = pattern
        {
            assert!(
                matches!(quantifier, Quantifier::OneOrMore),
                "Expected OneOrMore quantifier, got {quantifier:?}"
            );
            if let PatternExpression::Alternative(alternatives) = inner.as_ref() {
                assert_eq!(
                    alternatives.len(),
                    2,
                    "Expected 2 alternatives in unicode character class union"
                );
                if let PatternExpression::CharacterClass(CharClass::UnicodeProperty(prop)) =
                    &alternatives[0]
                {
                    assert_eq!(prop, "Alphabetic", "First alternative should be Alphabetic");
                } else {
                    panic!(
                        "First alternative should be UnicodeProperty, got {:?}",
                        alternatives[0]
                    );
                }
                if let PatternExpression::CharacterClass(CharClass::UnicodeProperty(prop)) =
                    &alternatives[1]
                {
                    assert_eq!(prop, "Numeric", "Second alternative should be Numeric");
                } else {
                    panic!(
                        "Second alternative should be UnicodeProperty, got {:?}",
                        alternatives[1]
                    );
                }
            } else {
                panic!("Expected Alternative pattern inside quantifier, got {inner:?}");
            }
        } else {
            panic!("Expected Quantified pattern, got {pattern:?}");
        }
    } else {
        panic!("Expected PatternDefinition, got {result:?}");
    }
}
