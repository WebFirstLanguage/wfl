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

                // Inner right should be a string literal
                if let Expression::Literal(Literal::String(s), ..) = *inner_right {
                    assert_eq!(s, "\\n", "Right side should be string '\\n'");
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
            assert_eq!(s, "Hello, World!");
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
                assert_eq!(s, "x is 10");
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
                assert_eq!(s, "x is not 10");
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
                assert_eq!(s, "nexus.log");
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
                    assert_eq!(s, "data.txt");
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
            e[0].message.contains("Cannot use keyword"),
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
