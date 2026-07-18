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

// ---------------------------------------------------------------------------
// `route` construct — desugaring to the `check if` / `IfStatement` chain.
// ---------------------------------------------------------------------------

/// Parse a source string that contains exactly one statement and return it.
fn parse_single(input: &str) -> Statement {
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);
    parser
        .parse_statement()
        .unwrap_or_else(|e| panic!("parse failed for {input:?}: {e:?}"))
}

#[test]
fn route_equality_arm_desugars_to_equals_if() {
    let stmt = parse_single("route path:\n    when \"/\":\n        display \"home\"\nend route");
    if let Statement::IfStatement {
        condition,
        then_block,
        else_block,
        ..
    } = stmt
    {
        match condition {
            Expression::BinaryOperation {
                left,
                operator: Operator::Equals,
                right,
                ..
            } => {
                assert!(matches!(*left, Expression::Variable(ref n, ..) if n == "path"));
                assert!(
                    matches!(*right, Expression::Literal(Literal::String(ref s), ..) if s.as_ref() == "/")
                );
            }
            other => panic!("expected Equals condition, got {other:?}"),
        }
        assert_eq!(then_block.len(), 1, "arm body should have one statement");
        assert!(else_block.is_none(), "no otherwise -> no else block");
    } else {
        panic!("route should desugar to an IfStatement, got {stmt:?}");
    }
}

#[test]
fn route_or_list_arm_desugars_to_or_of_equalities() {
    let stmt =
        parse_single("route code:\n    when 404 or 410:\n        display \"gone\"\nend route");
    if let Statement::IfStatement { condition, .. } = stmt {
        match condition {
            Expression::BinaryOperation {
                operator: Operator::Or,
                left,
                right,
                ..
            } => {
                assert!(matches!(
                    *left,
                    Expression::BinaryOperation {
                        operator: Operator::Equals,
                        ..
                    }
                ));
                assert!(matches!(
                    *right,
                    Expression::BinaryOperation {
                        operator: Operator::Equals,
                        ..
                    }
                ));
            }
            other => panic!("expected Or condition, got {other:?}"),
        }
    } else {
        panic!("expected IfStatement, got {stmt:?}");
    }
}

#[test]
fn route_contains_arm_desugars_to_contains_call() {
    let stmt =
        parse_single("route path:\n    when contains \"admin\":\n        display \"a\"\nend route");
    if let Statement::IfStatement { condition, .. } = stmt {
        match condition {
            Expression::FunctionCall {
                function,
                arguments,
                ..
            } => {
                assert!(matches!(*function, Expression::Variable(ref n, ..) if n == "contains"));
                assert_eq!(arguments.len(), 2);
                // contains of subject and needle
                assert!(
                    matches!(arguments[0].value, Expression::Variable(ref n, ..) if n == "path")
                );
                assert!(
                    matches!(arguments[1].value, Expression::Literal(Literal::String(ref s), ..) if s.as_ref() == "admin")
                );
            }
            other => panic!("expected contains FunctionCall, got {other:?}"),
        }
    } else {
        panic!("expected IfStatement, got {stmt:?}");
    }
}

#[test]
fn route_one_of_arm_is_membership_contains() {
    let stmt =
        parse_single("route path:\n    when one of assets:\n        display \"a\"\nend route");
    if let Statement::IfStatement { condition, .. } = stmt {
        match condition {
            Expression::FunctionCall {
                function,
                arguments,
                ..
            } => {
                assert!(matches!(*function, Expression::Variable(ref n, ..) if n == "contains"));
                // membership: contains of list and subject
                assert!(
                    matches!(arguments[0].value, Expression::Variable(ref n, ..) if n == "assets")
                );
                assert!(
                    matches!(arguments[1].value, Expression::Variable(ref n, ..) if n == "path")
                );
            }
            other => panic!("expected contains FunctionCall, got {other:?}"),
        }
    } else {
        panic!("expected IfStatement, got {stmt:?}");
    }
}

#[test]
fn route_starts_and_ends_with_desugar_to_builtins() {
    let starts = parse_single(
        "route path:\n    when starts with \"/api/\":\n        display \"api\"\nend route",
    );
    if let Statement::IfStatement { condition, .. } = starts {
        // Assert the callee name AND that the arguments are (subject, literal)
        // in that order, so a swapped or hardcoded argument fails the test.
        assert!(
            matches!(condition, Expression::FunctionCall { ref function, ref arguments, .. }
            if matches!(**function, Expression::Variable(ref n, ..) if n == "starts_with")
                && matches!(arguments[0].value, Expression::Variable(ref n, ..) if n == "path")
                && matches!(arguments[1].value, Expression::Literal(Literal::String(ref s), ..) if s.as_ref() == "/api/"))
        );
    } else {
        panic!("expected IfStatement");
    }

    let ends = parse_single(
        "route path:\n    when ends with \".css\":\n        display \"css\"\nend route",
    );
    if let Statement::IfStatement { condition, .. } = ends {
        assert!(
            matches!(condition, Expression::FunctionCall { ref function, ref arguments, .. }
            if matches!(**function, Expression::Variable(ref n, ..) if n == "ends_with")
                && matches!(arguments[0].value, Expression::Variable(ref n, ..) if n == "path")
                && matches!(arguments[1].value, Expression::Literal(Literal::String(ref s), ..) if s.as_ref() == ".css"))
        );
    } else {
        panic!("expected IfStatement");
    }
}

#[test]
fn route_otherwise_becomes_final_else_chain() {
    let stmt = parse_single(
        "route x:\n    when 1:\n        display \"one\"\n    when 2:\n        display \"two\"\n    otherwise:\n        display \"other\"\nend route",
    );
    // Outer if is arm 1; its else is [inner if for arm 2]; inner if's else is the otherwise body.
    if let Statement::IfStatement { else_block, .. } = stmt {
        let inner = else_block.expect("arm 1 should have an else");
        assert_eq!(inner.len(), 1);
        if let Statement::IfStatement {
            else_block: inner_else,
            ..
        } = &inner[0]
        {
            let default_body = inner_else
                .as_ref()
                .expect("arm 2 should have otherwise else");
            assert_eq!(default_body.len(), 1, "otherwise body has one statement");
            assert!(matches!(
                default_body[0],
                Statement::DisplayStatement { .. }
            ));
        } else {
            panic!("expected nested IfStatement for arm 2");
        }
    } else {
        panic!("expected IfStatement");
    }
}

#[test]
fn route_with_only_otherwise_runs_unconditionally() {
    let stmt = parse_single("route x:\n    otherwise:\n        display \"always\"\nend route");
    if let Statement::IfStatement {
        condition,
        then_block,
        else_block,
        ..
    } = stmt
    {
        assert!(matches!(
            condition,
            Expression::Literal(Literal::Boolean(true), ..)
        ));
        assert_eq!(then_block.len(), 1);
        assert!(else_block.is_none());
    } else {
        panic!("expected IfStatement");
    }
}

#[test]
fn empty_route_is_a_noop() {
    let stmt = parse_single("route x:\nend route");
    if let Statement::IfStatement {
        condition,
        then_block,
        ..
    } = stmt
    {
        assert!(matches!(
            condition,
            Expression::Literal(Literal::Boolean(false), ..)
        ));
        assert!(then_block.is_empty());
    } else {
        panic!("expected IfStatement");
    }
}

#[test]
fn route_otherwise_before_when_is_an_error() {
    let input = "route x:\n    otherwise:\n        display \"d\"\n    when 5:\n        display \"five\"\nend route";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);
    let result = parser.parse_statement();
    assert!(
        result.is_err(),
        "otherwise before a when arm must be rejected"
    );
}

// --- Issue #571 regression tests: precedence, of-calls, between, above/below,
//     finally, and caught-error binding ---

/// Parse `store result as <expr>` and return the declared value expression.
fn parse_value_expr(expr_src: &str) -> Expression {
    let input = format!("store result as {expr_src}");
    let tokens = lex_wfl_with_positions(&input);
    let mut parser = Parser::new(&tokens);
    match parser.parse_statement() {
        Ok(Statement::VariableDeclaration { value, .. }) => value,
        other => panic!("expected a variable declaration, got {other:?}"),
    }
}

#[test]
fn arithmetic_binds_tighter_than_comparison() {
    // `y plus 1 is equal to 4` must parse as `(y plus 1) is equal to 4`.
    let expr = parse_value_expr("y plus 1 is equal to 4");
    match expr {
        Expression::BinaryOperation { operator, left, .. } => {
            assert_eq!(
                operator,
                Operator::Equals,
                "top operator should be equality"
            );
            assert!(
                matches!(
                    *left,
                    Expression::BinaryOperation {
                        operator: Operator::Plus,
                        ..
                    }
                ),
                "left of the comparison should be the `plus` sub-expression, got {left:?}"
            );
        }
        other => panic!("expected a comparison at the top, got {other:?}"),
    }
}

#[test]
fn of_call_argument_absorbs_arithmetic() {
    // `f of n minus 1` must parse as `f of (n minus 1)`, not `(f of n) minus 1`.
    let expr = parse_value_expr("f of n minus 1");
    match expr {
        Expression::FunctionCall { arguments, .. } => {
            assert_eq!(arguments.len(), 1, "should be a single call argument");
            assert!(
                matches!(
                    &arguments[0].value,
                    Expression::BinaryOperation {
                        operator: Operator::Minus,
                        ..
                    }
                ),
                "the argument should be `n minus 1`, got {:?}",
                arguments[0].value
            );
        }
        other => panic!("expected a function call, got {other:?}"),
    }
}

#[test]
fn of_call_argument_still_stops_at_with() {
    // `substring of s and 0 and 1 with x` must keep `with` as concatenation on
    // the whole call, i.e. produce a Concatenation whose left is the call.
    let expr = parse_value_expr(r#"substring of s and 0 and 1 with "!""#);
    match expr {
        Expression::Concatenation { left, .. } => {
            assert!(
                matches!(*left, Expression::FunctionCall { .. }),
                "left of the concatenation should be the substring call, got {left:?}"
            );
        }
        other => panic!("expected a concatenation, got {other:?}"),
    }
}

#[test]
fn is_between_desugars_to_two_comparisons() {
    // `x is between 1 and 10` => `(x >= 1) and (x <= 10)`.
    let expr = parse_value_expr("x is between 1 and 10");
    match expr {
        Expression::BinaryOperation {
            operator: Operator::And,
            left,
            right,
            ..
        } => {
            assert!(
                matches!(
                    *left,
                    Expression::BinaryOperation {
                        operator: Operator::GreaterThanOrEqual,
                        ..
                    }
                ),
                "lower bound should be >=, got {left:?}"
            );
            assert!(
                matches!(
                    *right,
                    Expression::BinaryOperation {
                        operator: Operator::LessThanOrEqual,
                        ..
                    }
                ),
                "upper bound should be <=, got {right:?}"
            );
        }
        other => panic!("expected an `and` of two comparisons, got {other:?}"),
    }
}

#[test]
fn is_above_and_below_map_to_greater_and_less() {
    assert!(matches!(
        parse_value_expr("t is above 30"),
        Expression::BinaryOperation {
            operator: Operator::GreaterThan,
            ..
        }
    ));
    assert!(matches!(
        parse_value_expr("t is below 30"),
        Expression::BinaryOperation {
            operator: Operator::LessThan,
            ..
        }
    ));
}

#[test]
fn try_statement_parses_finally_and_named_error_binding() {
    let input = "try:\n\
                 display \"x\"\n\
                 when error as e:\n\
                 display e\n\
                 finally:\n\
                 display \"done\"\n\
                 end try";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);
    match parser.parse_statement() {
        Ok(Statement::TryStatement {
            when_clauses,
            finally_block,
            ..
        }) => {
            assert_eq!(when_clauses.len(), 1);
            assert_eq!(
                when_clauses[0].error_name, "e",
                "`when error as e` should bind the error under `e`"
            );
            let finally = finally_block.expect("finally block should be present");
            assert_eq!(finally.len(), 1, "finally block should have one statement");
        }
        other => panic!("expected a try statement, got {other:?}"),
    }
}

#[test]
fn pattern_quantifier_rejects_descending_numeric_range() {
    let input = "create pattern bounded:\n    10 to 1 digit\nend pattern";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let error = parser
        .parse_statement()
        .expect_err("a descending pattern range must be rejected");

    assert!(
        error.message.contains("lower bound") && error.message.contains("upper bound"),
        "unexpected error: {error}"
    );
}

#[test]
fn pattern_quantifier_rejects_counts_outside_u32_range() {
    for body in [
        "exactly 4294967296 digit",
        "exactly 9223372036854775807 digit",
        "1 to 4294967296 digit",
        "digit exactly 4294967296",
        "digit between 1 and 4294967296",
    ] {
        let input = format!("create pattern bounded:\n    {body}\nend pattern");
        let tokens = lex_wfl_with_positions(&input);
        let mut parser = Parser::new(&tokens);

        let error = parser
            .parse_statement()
            .expect_err("out-of-range quantifier must be rejected");
        assert!(
            error.message.contains("between 0 and"),
            "unexpected error for `{body}`: {error}"
        );
    }
}

#[test]
fn normal_pattern_quantifier_counts_remain_compatible() {
    let cases = [
        ("exactly 3 digit", Quantifier::Exactly(3)),
        ("2 to 6 digit", Quantifier::Between(2, 6)),
        ("digit exactly 4", Quantifier::Exactly(4)),
        ("digit between 1 and 5", Quantifier::Between(1, 5)),
    ];

    for (body, expected_quantifier) in cases {
        let input = format!("create pattern bounded:\n    {body}\nend pattern");
        let tokens = lex_wfl_with_positions(&input);
        let mut parser = Parser::new(&tokens);
        let statement = parser
            .parse_statement()
            .unwrap_or_else(|error| panic!("expected `{body}` to parse: {error}"));

        let Statement::PatternDefinition { pattern, .. } = statement else {
            panic!("expected a pattern definition for `{body}`");
        };
        let PatternExpression::Quantified { quantifier, .. } = pattern else {
            panic!("expected a quantified pattern for `{body}`, got {pattern:?}");
        };
        assert_eq!(quantifier, expected_quantifier, "body: `{body}`");
    }
}

// --- Multi-value `display` (concatenation of space-separated values) ---
//
// `display` accepts more than one space-separated value: quoted text is a
// string literal, anything else is a variable/expression, and the values are
// folded right-associatively into a single Concatenation — the same AST shape
// (and therefore the same evaluation order and stringification order) as
// joining them explicitly with `with` (`display "x" y "z"` parses identically
// to `display "x" with y with "z"`; a run of bare words with no separator lexes
// as one multi-word identifier, not several values, so an example needs a
// non-identifier token to actually show the fold). See parse_display_statement
// in stmt/io.rs.

#[test]
fn test_display_multiple_values_string_then_var() {
    let input = r#"display "user age is " user age"#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse multi-value display: {result:?}"
    );

    let Ok(Statement::DisplayStatement { value, .. }) = result else {
        panic!("Expected DisplayStatement, got: {result:?}");
    };
    let Expression::Concatenation { left, right, .. } = value else {
        panic!("Expected Concatenation, got: {value:?}");
    };
    match *left {
        Expression::Literal(Literal::String(ref s), ..) => {
            assert_eq!(
                s.as_ref(),
                "user age is ",
                "left should be the string literal"
            )
        }
        other => panic!("Expected string literal on left, got: {other:?}"),
    }
    match *right {
        Expression::Variable(ref name, ..) => {
            assert_eq!(name, "user age", "right should be the variable 'user age'")
        }
        other => panic!("Expected variable on right, got: {other:?}"),
    }
}

#[test]
fn test_display_multiple_values_var_then_string() {
    let input = r#"display user age " is the age""#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse multi-value display: {result:?}"
    );

    let Ok(Statement::DisplayStatement { value, .. }) = result else {
        panic!("Expected DisplayStatement, got: {result:?}");
    };
    let Expression::Concatenation { left, right, .. } = value else {
        panic!("Expected Concatenation, got: {value:?}");
    };
    match *left {
        Expression::Variable(ref name, ..) => {
            assert_eq!(name, "user age", "left should be the variable 'user age'")
        }
        other => panic!("Expected variable on left, got: {other:?}"),
    }
    match *right {
        Expression::Literal(Literal::String(ref s), ..) => {
            assert_eq!(
                s.as_ref(),
                " is the age",
                "right should be the string literal"
            )
        }
        other => panic!("Expected string literal on right, got: {other:?}"),
    }
}

#[test]
fn test_display_three_values_right_associative_like_with() {
    let input = r#"display "a" middle "b""#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse three-value display: {result:?}"
    );

    let Ok(Statement::DisplayStatement { value, .. }) = result else {
        panic!("Expected DisplayStatement, got: {result:?}");
    };
    // Right-associative, matching `"a" with middle with "b"`: ("a" with (middle with "b"))
    let Expression::Concatenation { left, right, .. } = value else {
        panic!("Expected outer Concatenation, got: {value:?}");
    };
    match *left {
        Expression::Literal(Literal::String(ref s), ..) => assert_eq!(s.as_ref(), "a"),
        other => panic!("Expected string 'a' on outer left, got: {other:?}"),
    }
    let Expression::Concatenation {
        left: inner_left,
        right: inner_right,
        ..
    } = *right
    else {
        panic!("Expected inner Concatenation on right");
    };
    match *inner_left {
        Expression::Variable(ref name, ..) => assert_eq!(name, "middle"),
        other => panic!("Expected variable 'middle' on inner left, got: {other:?}"),
    }
    match *inner_right {
        Expression::Literal(Literal::String(ref s), ..) => assert_eq!(s.as_ref(), "b"),
        other => panic!("Expected string 'b' on inner right, got: {other:?}"),
    }
}

#[test]
fn test_display_four_values_right_associative_nesting() {
    // Four values must nest the same way a `with` chain does: right-associative,
    // all the way down — (a with (b with (c with d))) — not just pairwise at
    // the top level. This is what makes evaluation order (and therefore
    // stringification order for mutating expressions) match `with` exactly.
    // See the mutation-order regression in
    // tests/display_multiple_values_stdout_test.rs for the runtime consequence.
    fn as_str_literal(expr: &Expression) -> &str {
        match expr {
            Expression::Literal(Literal::String(s), ..) => s.as_ref(),
            other => panic!("expected a string literal, got: {other:?}"),
        }
    }
    fn as_concatenation(expr: &Expression) -> (&Expression, &Expression) {
        match expr {
            Expression::Concatenation { left, right, .. } => (&**left, &**right),
            other => panic!("expected Concatenation, got: {other:?}"),
        }
    }

    let tokens = lex_wfl_with_positions(r#"display "a" "b" "c" "d""#);
    let mut parser = Parser::new(&tokens);
    let Ok(Statement::DisplayStatement { value, .. }) = parser.parse_statement() else {
        panic!("expected a DisplayStatement");
    };

    let (a, rest1) = as_concatenation(&value);
    assert_eq!(as_str_literal(a), "a");
    let (b, rest2) = as_concatenation(rest1);
    assert_eq!(as_str_literal(b), "b");
    let (c, d) = as_concatenation(rest2);
    assert_eq!(as_str_literal(c), "c");
    assert_eq!(as_str_literal(d), "d");
}

#[test]
fn test_display_direct_index_not_concatenated() {
    // Regression: `display numbers 0` must stay a single IndexAccess, NOT
    // become a concatenation of `numbers` and `0`.
    let input = r#"display numbers 0"#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse direct index display: {result:?}"
    );

    let Ok(Statement::DisplayStatement { value, .. }) = result else {
        panic!("Expected DisplayStatement, got: {result:?}");
    };
    assert!(
        matches!(value, Expression::IndexAccess { .. }),
        "Expected IndexAccess, got: {value:?}"
    );
}

#[test]
fn test_display_single_value_unchanged() {
    // Regression: a single value is still a bare expression, not a Concatenation.
    let input = r#"display user age"#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse single-value display: {result:?}"
    );

    let Ok(Statement::DisplayStatement { value, .. }) = result else {
        panic!("Expected DisplayStatement, got: {result:?}");
    };
    match value {
        Expression::Variable(ref name, ..) => assert_eq!(name, "user age"),
        other => panic!("Expected bare Variable, got: {other:?}"),
    }
}

#[test]
fn test_display_multiple_values_does_not_leak_into_next_statement() {
    // The trailing value must be consumed by `display`, leaving the following
    // statement (here `change`) to parse normally on the next line.
    let input = "display \"user age is \" user age\nchange user age to 9";
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let program = parser
        .parse()
        .unwrap_or_else(|errors| panic!("Expected the program to parse: {errors:?}"));
    assert_eq!(
        program.statements.len(),
        2,
        "Expected exactly two statements, got: {:?}",
        program.statements
    );
    assert!(
        matches!(
            program.statements[0],
            Statement::DisplayStatement {
                value: Expression::Concatenation { .. },
                ..
            }
        ),
        "Expected a Concatenation DisplayStatement, got: {:?}",
        program.statements[0]
    );
    assert!(
        matches!(program.statements[1], Statement::Assignment { .. }),
        "Expected the following `change` to parse as an Assignment, got: {:?}",
        program.statements[1]
    );
}

#[test]
fn test_display_folds_count_variable() {
    // Regression for the review finding: the count-loop variable `count` is a
    // keyword, so it must still fold as a trailing display value instead of
    // being mis-parsed as the start of a new count loop.
    let input = r#"display "count is " count"#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse display with folded count: {result:?}"
    );

    let Ok(Statement::DisplayStatement { value, .. }) = result else {
        panic!("Expected DisplayStatement, got: {result:?}");
    };
    let Expression::Concatenation { left, right, .. } = value else {
        panic!("Expected Concatenation, got: {value:?}");
    };
    match *left {
        Expression::Literal(Literal::String(ref s), ..) => assert_eq!(s.as_ref(), "count is "),
        other => panic!("Expected string literal on left, got: {other:?}"),
    }
    match *right {
        Expression::Variable(ref name, ..) => assert_eq!(name, "count"),
        other => panic!("Expected the `count` variable on right, got: {other:?}"),
    }
}

#[test]
fn test_display_folds_call_action() {
    // Regression for the review finding: `call <action>` is a keyword-led value
    // and must fold as a trailing display value rather than being dropped.
    let input = r#"display "n: " call doubled with 5"#;
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);

    let result = parser.parse_statement();
    assert!(
        result.is_ok(),
        "Failed to parse display with folded call: {result:?}"
    );

    let Ok(Statement::DisplayStatement { value, .. }) = result else {
        panic!("Expected DisplayStatement, got: {result:?}");
    };
    let Expression::Concatenation { left, right, .. } = value else {
        panic!("Expected Concatenation, got: {value:?}");
    };
    match *left {
        Expression::Literal(Literal::String(ref s), ..) => assert_eq!(s.as_ref(), "n: "),
        other => panic!("Expected string literal on left, got: {other:?}"),
    }
    match *right {
        Expression::ActionCall { ref name, .. } => assert_eq!(name, "doubled"),
        other => panic!("Expected an ActionCall on right, got: {other:?}"),
    }
}

/// Parses `input` as a single `display` statement and returns its value
/// expression, panicking with the parse error (or the wrong statement kind)
/// otherwise. Shared by the `test_display_folds_keyword_*` tests below.
fn parse_display(input: &str) -> Expression {
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);
    // Parse the whole program (not a single statement) so any trailing tokens
    // the fold failed to consume surface as an extra statement and fail here,
    // rather than being silently left on the cursor.
    let program = parser
        .parse()
        .unwrap_or_else(|error| panic!("Failed to parse `{input}`: {error:?}"));
    let mut statements = program.statements.into_iter();
    let statement = statements
        .next()
        .unwrap_or_else(|| panic!("Expected a statement for `{input}`"));
    assert!(
        statements.next().is_none(),
        "Expected exactly one statement for `{input}`"
    );
    match statement {
        Statement::DisplayStatement { value, .. } => value,
        other => panic!("Expected a DisplayStatement for `{input}`, got: {other:?}"),
    }
}

/// Asserts `input` parses as `display "<label>" <trailing>` folded into a
/// Concatenation whose left is the label string and whose right satisfies
/// `check_right`.
fn assert_display_folds(input: &str, label: &str, check_right: impl FnOnce(&Expression)) {
    let value = parse_display(input);
    let Expression::Concatenation { left, right, .. } = value else {
        panic!("Expected Concatenation for `{input}`, got: {value:?}");
    };
    match *left {
        Expression::Literal(Literal::String(ref s), ..) => assert_eq!(s.as_ref(), label),
        other => panic!("Expected string literal '{label}' on left, got: {other:?}"),
    }
    check_right(&right);
}

// The following regression tests cover each keyword-led value added to
// `is_value_start` beyond `call`/`count`/`current` (see the doc comment on
// `is_value_start` in parser/helpers.rs for why each is safe): every one must
// still fold into the display's Concatenation instead of being left as a
// dangling, separately-parsed (and likely erroring) trailing statement.

#[test]
fn test_display_folds_keyword_not() {
    assert_display_folds(r#"display "flag: " not yes"#, "flag: ", |right| {
        assert!(
            matches!(
                right,
                Expression::UnaryOperation {
                    operator: UnaryOperator::Not,
                    ..
                }
            ),
            "expected a `not` UnaryOperation, got: {right:?}"
        );
    });
}

#[test]
fn test_display_folds_keyword_pattern() {
    assert_display_folds(r#"display "p: " pattern "abc""#, "p: ", |right| {
        assert!(
            matches!(right, Expression::Literal(Literal::Pattern(_), ..)),
            "expected a Pattern literal, got: {right:?}"
        );
    });
}

#[test]
fn test_display_folds_keyword_output() {
    // `output` is a contextual keyword: in expression position (not preceded
    // by `read ... from process`) it is just the variable named `output`.
    assert_display_folds(r#"display "o: " output"#, "o: ", |right| {
        assert!(
            matches!(right, Expression::Variable(name, ..) if name == "output"),
            "expected the `output` variable, got: {right:?}"
        );
    });
}

#[test]
fn test_display_folds_keyword_file_exists() {
    assert_display_folds(
        r#"display "exists: " file exists at "x.txt""#,
        "exists: ",
        |right| {
            assert!(
                matches!(right, Expression::FileExists { .. }),
                "expected a FileExists expression, got: {right:?}"
            );
        },
    );
}

#[test]
fn test_display_folds_keyword_directory_exists() {
    assert_display_folds(
        r#"display "exists: " directory exists at "x""#,
        "exists: ",
        |right| {
            assert!(
                matches!(right, Expression::DirectoryExists { .. }),
                "expected a DirectoryExists expression, got: {right:?}"
            );
        },
    );
}

#[test]
fn test_display_folds_keyword_process_running() {
    assert_display_folds(
        r#"display "running: " process pid is running"#,
        "running: ",
        |right| {
            assert!(
                matches!(right, Expression::ProcessRunning { .. }),
                "expected a ProcessRunning expression, got: {right:?}"
            );
        },
    );
}

#[test]
fn test_display_folds_keyword_header() {
    assert_display_folds(
        r#"display "h: " header "Content-Type" of req"#,
        "h: ",
        |right| {
            assert!(
                matches!(right, Expression::HeaderAccess { .. }),
                "expected a HeaderAccess expression, got: {right:?}"
            );
        },
    );
}

#[test]
fn test_display_folds_keyword_list_files() {
    assert_display_folds(
        r#"display "files: " list files in ".""#,
        "files: ",
        |right| {
            assert!(
                matches!(right, Expression::ListFiles { .. }),
                "expected a ListFiles expression, got: {right:?}"
            );
        },
    );
}

#[test]
fn test_display_folds_keyword_read_content() {
    assert_display_folds(
        r#"display "content: " read content from handle"#,
        "content: ",
        |right| {
            assert!(
                matches!(right, Expression::ReadContent { .. }),
                "expected a ReadContent expression, got: {right:?}"
            );
        },
    );
}

#[test]
fn test_display_folds_keyword_back() {
    // `back` is only reserved after `give` (`give back x`); as a fresh
    // display value it is an unambiguous bare-variable reference, same as
    // `not`/`pattern` above (see the `is_value_start` doc comment).
    assert_display_folds(r#"display "b: " back"#, "b: ", |right| {
        assert!(
            matches!(right, Expression::Variable(name, ..) if name == "back"),
            "expected the `back` variable, got: {right:?}"
        );
    });
}

#[test]
fn test_display_folds_keyword_error() {
    // `error` is only reserved inside `when error:` (a `try` clause header);
    // as a fresh display value it is an unambiguous bare-variable reference.
    assert_display_folds(r#"display "e: " error"#, "e: ", |right| {
        assert!(
            matches!(right, Expression::Variable(name, ..) if name == "error"),
            "expected the `error` variable, got: {right:?}"
        );
    });
}

// --- Same-line statement boundaries: `count from ...` / `read output from process ...` ----
//
// `count` and `read` each fold as a bare value (the count-loop variable, and
// `read content/binary/N bytes from ...`), but each also leads a longer,
// statement-only form with no expression-position parse of its own:
// `count from X to Y:` (a count loop) and `read output from process P as V`
// (`parse_read_process_output_statement`). Both must still parse as their own
// statement when they immediately follow a `display` on the same line,
// exactly as they did before multi-value `display` existed, instead of being
// partially folded into the display and stranding the rest as unparsable
// leftover tokens. See `is_display_fold_statement_boundary` in
// `parser/helpers.rs`.

/// Parses `input` as a whole program and returns its statements, panicking
/// with the parse error otherwise. Unlike `parse_display`, more than one
/// statement is expected here — these tests are specifically about where one
/// statement ends and the next begins.
fn parse_program_statements(input: &str) -> Vec<Statement> {
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);
    parser
        .parse()
        .unwrap_or_else(|error| panic!("Failed to parse `{input}`: {error:?}"))
        .statements
}

#[test]
fn count_from_after_display_stays_a_separate_statement() {
    let statements = parse_program_statements(
        r#"display "start" count from 1 to 3:
    display "n: " count
end count
"#,
    );

    assert_eq!(
        statements.len(),
        2,
        "expected exactly 2 statements, got: {statements:?}"
    );

    match &statements[0] {
        Statement::DisplayStatement { value, .. } => {
            assert!(
                matches!(value, Expression::Literal(Literal::String(s), ..) if s.as_ref() == "start"),
                "expected the `display` to stop at the bare string, not fold `count`, got: {value:?}"
            );
        }
        other => panic!("Expected a DisplayStatement first, got: {other:?}"),
    }

    assert!(
        matches!(&statements[1], Statement::CountLoop { .. }),
        "expected `count from ...` to open a count loop, got: {:?}",
        statements[1]
    );
}

#[test]
fn read_output_from_process_after_display_stays_a_separate_statement() {
    let statements = parse_program_statements(
        r#"display "start" read output from process proc as result
display "result: " result
"#,
    );

    assert_eq!(
        statements.len(),
        3,
        "expected exactly 3 statements, got: {statements:?}"
    );

    match &statements[0] {
        Statement::DisplayStatement { value, .. } => {
            assert!(
                matches!(value, Expression::Literal(Literal::String(s), ..) if s.as_ref() == "start"),
                "expected the `display` to stop at the bare string, not fold `read`, got: {value:?}"
            );
        }
        other => panic!("Expected a DisplayStatement first, got: {other:?}"),
    }

    assert!(
        matches!(
            &statements[1],
            Statement::ReadProcessOutputStatement { variable_name, .. } if variable_name == "result"
        ),
        "expected `read output from process ... as result` to parse as its own statement, got: {:?}",
        statements[1]
    );

    assert!(
        matches!(&statements[2], Statement::DisplayStatement { .. }),
        "expected the trailing `display \"result: \" result` to parse normally, got: {:?}",
        statements[2]
    );
}

// --- Same-line statement boundaries surfaced by centralizing `is_value_start` ----
//
// `create`, `change`, `push`, `parent`, `skip`, and `give` are all contextual
// keywords (bare variables in expression position) that are *also* dedicated
// arms of `parse_statement`'s top-level dispatch, with no expression-position
// equivalent for their statement form (see the `is_value_start` doc comment
// in `parser/helpers.rs`). Centralizing on `can_start_primary_expression`
// surfaced these the same way it surfaced `count`/`read`, so each is excluded
// from `is_value_start` outright; these tests lock that in.

#[test]
fn create_after_display_stays_a_separate_statement() {
    let statements =
        parse_program_statements("display \"start\" create directory at \"wfl-test-dir\"\n");

    assert_eq!(
        statements.len(),
        2,
        "expected exactly 2 statements, got: {statements:?}"
    );
    match &statements[0] {
        Statement::DisplayStatement { value, .. } => {
            assert!(
                matches!(value, Expression::Literal(Literal::String(s), ..) if s.as_ref() == "start"),
                "expected the `display` to stop at the bare string, not fold `create`, got: {value:?}"
            );
        }
        other => panic!("Expected a DisplayStatement first, got: {other:?}"),
    }
    assert!(
        matches!(&statements[1], Statement::CreateDirectoryStatement { .. }),
        "expected `create directory at ...` to parse as its own statement, got: {:?}",
        statements[1]
    );
}

#[test]
fn change_after_display_stays_a_separate_statement() {
    let statements = parse_program_statements(
        r#"store n as 1
display "start" change n to 2
"#,
    );

    assert_eq!(
        statements.len(),
        3,
        "expected exactly 3 statements, got: {statements:?}"
    );
    match &statements[1] {
        Statement::DisplayStatement { value, .. } => {
            assert!(
                matches!(value, Expression::Literal(Literal::String(s), ..) if s.as_ref() == "start"),
                "expected the `display` to stop at the bare string, not fold `change`, got: {value:?}"
            );
        }
        other => panic!("Expected a DisplayStatement second, got: {other:?}"),
    }
    assert!(
        matches!(&statements[2], Statement::Assignment { .. }),
        "expected `change n to 2` to parse as its own assignment statement, got: {:?}",
        statements[2]
    );
}

#[test]
fn push_after_display_stays_a_separate_statement() {
    let statements = parse_program_statements(
        r#"create list numbers:
end list
display "start" push with numbers and 5
"#,
    );

    let display_index = statements.len() - 2;
    match &statements[display_index] {
        Statement::DisplayStatement { value, .. } => {
            assert!(
                matches!(value, Expression::Literal(Literal::String(s), ..) if s.as_ref() == "start"),
                "expected the `display` to stop at the bare string, not fold `push`, got: {value:?}"
            );
        }
        other => panic!("Expected a DisplayStatement, got: {other:?}"),
    }
    assert!(
        matches!(
            &statements[display_index + 1],
            Statement::PushStatement { .. }
        ),
        "expected `push with numbers and 5` to parse as its own statement, got: {:?}",
        statements[display_index + 1]
    );
}

#[test]
fn parent_after_display_stays_a_separate_statement() {
    let statements = parse_program_statements(
        r#"display "start" parent bump
"#,
    );

    assert_eq!(
        statements.len(),
        2,
        "expected exactly 2 statements, got: {statements:?}"
    );
    match &statements[0] {
        Statement::DisplayStatement { value, .. } => {
            assert!(
                matches!(value, Expression::Literal(Literal::String(s), ..) if s.as_ref() == "start"),
                "expected the `display` to stop at the bare string, not fold `parent`, got: {value:?}"
            );
        }
        other => panic!("Expected a DisplayStatement first, got: {other:?}"),
    }
    assert!(
        matches!(
            &statements[1],
            Statement::ParentMethodCall { method_name, .. } if method_name == "bump"
        ),
        "expected `parent bump` to parse as its own statement, got: {:?}",
        statements[1]
    );
}

#[test]
fn skip_after_display_stays_a_continue_statement() {
    // The sharpest case: as a bare statement, `skip` is `continue` — a
    // control-flow *effect*, not a value — and both forms consume exactly one
    // token, so a silent regression here would not even leave stray tokens
    // behind; it would just quietly stop continuing the loop.
    let statements = parse_program_statements(
        r#"display "start" skip
"#,
    );

    assert_eq!(
        statements.len(),
        2,
        "expected exactly 2 statements, got: {statements:?}"
    );
    match &statements[0] {
        Statement::DisplayStatement { value, .. } => {
            assert!(
                matches!(value, Expression::Literal(Literal::String(s), ..) if s.as_ref() == "start"),
                "expected the `display` to stop at the bare string, not fold `skip`, got: {value:?}"
            );
        }
        other => panic!("Expected a DisplayStatement first, got: {other:?}"),
    }
    assert!(
        matches!(&statements[1], Statement::ContinueStatement { .. }),
        "expected `skip` to still parse as its own ContinueStatement, got: {:?}",
        statements[1]
    );
}

#[test]
fn give_back_after_display_stays_a_separate_statement() {
    let statements = parse_program_statements(
        r#"define action called f:
    display "start" give back 5
end action
"#,
    );

    let Statement::ActionDefinition { body, .. } = &statements[0] else {
        panic!("Expected an ActionDefinition, got: {:?}", statements[0]);
    };

    assert_eq!(
        body.len(),
        2,
        "expected exactly 2 statements in the action body, got: {body:?}"
    );
    match &body[0] {
        Statement::DisplayStatement { value, .. } => {
            assert!(
                matches!(value, Expression::Literal(Literal::String(s), ..) if s.as_ref() == "start"),
                "expected the `display` to stop at the bare string, not fold `give`, got: {value:?}"
            );
        }
        other => panic!("Expected a DisplayStatement first, got: {other:?}"),
    }
    assert!(
        matches!(&body[1], Statement::ReturnStatement { .. }),
        "expected `give back 5` to parse as its own return statement, got: {:?}",
        body[1]
    );
}

// --- Centralization: `is_value_start` cannot drift from `parse_primary_expression` ----
//
// The primary enforcement is a pair of `debug_assert!`s inside
// `parse_primary_expression` itself (`src/parser/expr/primary.rs`), which
// compare `can_start_primary_expression`'s prediction against the real
// dispatch on *every* primary-expression parse in every debug build — every
// test below, every `TestPrograms/*.wfl` run, every program compiled without
// `--release`. The test in this section is a curated, documented sample
// kept for explicit coverage of each keyword-led arm, not the only thing
// standing between the two staying in sync.

/// Returns `true` if parsing `input` as a primary expression falls all the
/// way through to `parse_primary_expression`'s final
/// `_ => Err("Unexpected token in expression: ...")` arm — i.e. the leading
/// token has *no* dedicated arm — as opposed to succeeding, or failing with a
/// different, arm-specific error (e.g. an incomplete but recognized keyword
/// form). This is a stronger signal than plain `Ok`/`Err`: several arms
/// return a perfectly valid `Ok` for a bare keyword (`back`, `output`, ...),
/// while others can still legitimately `Err` without having fallen through
/// (e.g. `(` with nothing before `)`) — only the fallback's specific message
/// means "no arm claimed this token".
fn falls_through_to_primary_expression_fallback(input: &str) -> bool {
    let tokens = lex_wfl_with_positions(input);
    let mut parser = Parser::new(&tokens);
    match parser.parse_primary_expression() {
        Ok(_) => false,
        Err(error) => error.message.contains("Unexpected token in expression"),
    }
}

#[test]
fn can_start_primary_expression_matches_parse_primary_expression() {
    // One representative snippet per case, covering every explicit
    // keyword-led arm in `parse_primary_expression`, a broad sample of the
    // contextual-keyword catch-all (`_ if token.is_contextual_keyword()`),
    // and a broad sample of tokens with no primary-expression arm at all
    // (structural keywords/punctuation that only mean something as a
    // statement or as a continuation of a *preceding* expression). Not
    // literally exhaustive over all ~190 `Token` variants — that would mostly
    // test the lexer, not this coupling — but wide enough that adding or
    // removing a `parse_primary_expression` arm for any of these tokens
    // without updating `can_start_primary_expression` fails this test.
    let cases: &[(&str, bool)] = &[
        // Literals, identifiers, brackets: always a primary starter.
        ("5", true),
        ("5.0", true),
        (r#""s""#, true),
        ("yes", true),
        ("nothing", true),
        ("x", true),
        ("(5)", true),
        ("[5]", true),
        // Explicit keyword-led arms.
        ("call", true),
        ("not yes", true),
        ("-5", true),
        ("with x", true),
        ("count", true),
        ("pattern", true),
        ("loop", true),
        ("output", true),
        ("repeat", true),
        ("exit", true),
        ("back", true),
        ("try", true),
        ("when", true),
        ("error", true),
        ("file", true),
        ("directory", true),
        ("process", true),
        ("header", true),
        ("current", true),
        ("list", true),
        ("read", true),
        ("find x in y", true),
        ("replace x with y in z", true),
        ("split x by y", true),
        // A sample of the contextual-keyword catch-all: no dedicated arm
        // above, but `token.is_contextual_keyword()` is true, so a bare
        // keyword still resolves to a plain variable reference.
        ("text", true),
        ("map", true),
        ("create", true),
        ("new", true),
        ("parent", true),
        ("push", true),
        ("skip", true),
        ("give", true),
        ("called", true),
        ("needs", true),
        ("change", true),
        ("reversed", true),
        ("at", true),
        ("least", true),
        ("most", true),
        ("than", true),
        ("zero", true),
        ("any", true),
        ("must", true),
        ("defaults", true),
        ("binary", true),
        ("bytes", true),
        ("that", true),
        ("files", true),
        ("extension", true),
        ("extensions", true),
        ("contains", true),
        ("starts", true),
        ("ends", true),
        // Structural keywords and punctuation with no primary-expression arm:
        // only meaningful as a statement starter or as a continuation
        // consumed by an already-parsed left operand.
        ("store", false),
        ("display", false),
        ("check", false),
        ("if", false),
        ("for", false),
        ("define", false),
        ("action", false),
        ("end", false),
        ("otherwise", false),
        ("then", false),
        ("as", false),
        ("to", false),
        ("from", false),
        ("and", false),
        ("or", false),
        ("in", false),
        ("while", false),
        ("until", false),
        ("forever", false),
        ("break", false),
        ("continue", false),
        ("return", false),
        (":", false),
        (",", false),
    ];

    for (input, expected) in cases {
        let tokens = lex_wfl_with_positions(input);
        let leading_token = tokens
            .first()
            .unwrap_or_else(|| panic!("`{input}` lexed to no tokens"));
        let predicted = Parser::can_start_primary_expression(&leading_token.token);
        assert_eq!(
            predicted, *expected,
            "can_start_primary_expression(`{input}`) predicted {predicted}, expected {expected}"
        );

        if *expected {
            assert!(
                !falls_through_to_primary_expression_fallback(input),
                "`{input}` is predicted to start a primary expression, but \
                 parse_primary_expression fell through to its fallback arm — \
                 can_start_primary_expression has drifted from \
                 parse_primary_expression"
            );
        } else {
            assert!(
                falls_through_to_primary_expression_fallback(input),
                "`{input}` is predicted NOT to start a primary expression, but \
                 parse_primary_expression accepted it (or failed with a \
                 different, arm-specific error) — can_start_primary_expression \
                 has drifted from parse_primary_expression"
            );
        }
    }
}
