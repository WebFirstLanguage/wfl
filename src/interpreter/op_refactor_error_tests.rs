#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::lexer::lex_wfl_with_positions;
    use crate::parser::Parser;

    async fn run_expr_expect_error(source: &str) -> String {
        let mut interpreter = Interpreter::new();
        let tokens = lex_wfl_with_positions(source);
        let mut parser = Parser::new(&tokens);
        let program = parser.parse().expect("Failed to parse");

        match interpreter.interpret(&program).await {
            Ok(_) => panic!("Expected error but got success"),
            Err(errors) => errors[0].message.clone(),
        }
    }

    #[tokio::test]
    async fn test_binary_op_errors() {
        // Subtraction
        let err = run_expr_expect_error("5 minus \"text\"").await;
        assert_eq!(err, "Cannot subtract Text from Number");

        // Multiplication
        let err = run_expr_expect_error("5 times \"text\"").await;
        assert_eq!(err, "Cannot multiply Number and Text");

        // Division
        let err = run_expr_expect_error("10 divided by \"text\"").await;
        assert_eq!(err, "Cannot divide Number by Text");

        // Division by zero
        let err = run_expr_expect_error("10 divided by 0").await;
        assert_eq!(err, "Division by zero");

        // Modulo
        let err = run_expr_expect_error("10 % \"text\"").await;
        assert_eq!(err, "Cannot compute modulo of Number by Text");

        // Modulo by zero
        let err = run_expr_expect_error("10 % 0").await;
        assert_eq!(err, "Modulo by zero");

        // Greater than (incompatible types)
        let err = run_expr_expect_error("5 is greater than \"text\"").await;
        assert_eq!(err, "Cannot compare Number and Text with >");

        // Less than (incompatible types)
        let err = run_expr_expect_error("5 is less than \"text\"").await;
        assert_eq!(err, "Cannot compare Number and Text with <");
    }
}
