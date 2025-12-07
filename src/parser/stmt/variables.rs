//! Variable declaration and assignment statement parsing

use super::super::{Parser, ParseError, Statement, Expression, Literal, Operator};
use crate::lexer::token::{Token, TokenWithPosition};
use crate::parser::expr::ExprParser;

pub(crate) trait VariableParser<'a>: ExprParser<'a> {
    fn parse_variable_declaration(&mut self) -> Result<Statement, ParseError>;
    fn parse_assignment(&mut self) -> Result<Statement, ParseError>;
    fn parse_arithmetic_operation(&mut self) -> Result<Statement, ParseError>;
}

impl<'a> VariableParser<'a> for Parser<'a> {
    fn parse_variable_declaration(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.bump_sync().unwrap();
        let is_store = matches!(token_pos.token, Token::KeywordStore);
        let _keyword = if is_store { "store" } else { "create" };

        // Check for "store new constant" syntax
        let mut is_constant = false;
        if is_store
            && let Some(next_token) = self.cursor.peek()
            && matches!(next_token.token, Token::KeywordNew)
        {
            self.bump_sync(); // Consume "new"
            if let Some(const_token) = self.cursor.peek() {
                if matches!(const_token.token, Token::KeywordConstant) {
                    self.bump_sync(); // Consume "constant"
                    is_constant = true;
                } else {
                    return Err(ParseError::new(
                        format!(
                            "Expected 'constant' after 'new', found {:?}",
                            const_token.token
                        ),
                        const_token.line,
                        const_token.column,
                    ));
                }
            } else {
                return Err(ParseError::new(
                    "Expected 'constant' after 'new'".to_string(),
                    token_pos.line,
                    token_pos.column,
                ));
            }
        }

        let name = self.parse_variable_name_list()?;

        // Handle special case: "create list as name"
        if !is_store && name == "list" {
            self.expect_token(Token::KeywordAs, "Expected 'as' after 'list'")?;

            let list_name = if let Some(token) = self.cursor.peek() {
                if let Token::Identifier(id) = &token.token {
                    self.bump_sync();
                    id.clone()
                } else {
                    return Err(ParseError::new(
                        format!("Expected identifier after 'as', found {:?}", token.token),
                        token.line,
                        token.column,
                    ));
                }
            } else {
                return Err(ParseError::new(
                    "Expected identifier after 'as'".to_string(),
                    token_pos.line,
                    token_pos.column,
                ));
            };

            let empty_list =
                Expression::Literal(Literal::List(Vec::new()), token_pos.line, token_pos.column);

            return Ok(Statement::VariableDeclaration {
                name: list_name,
                value: empty_list,
                is_constant: false,
                line: token_pos.line,
                column: token_pos.column,
            });
        }

        if let Some(token) = self.cursor.peek().cloned() {
            if !matches!(token.token, Token::KeywordAs) {
                return Err(ParseError::new(
                    format!(
                        "Expected 'as' after variable name '{}', but found {:?}",
                        name, token.token
                    ),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new(
                format!("Expected 'as' after variable name '{name}', but found end of input"),
                token_pos.line,
                token_pos.column,
            ));
        }

        self.bump_sync(); // Consume the 'as' token

        let value = self.parse_expression()?;

        Ok(Statement::VariableDeclaration {
            name,
            value,
            is_constant,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_assignment(&mut self) -> Result<Statement, ParseError> {
        self.bump_sync(); // Consume "change"

        let mut name = String::new();
        let mut has_identifier = false;

        while let Some(token) = self.cursor.peek().cloned() {
            if let Token::Identifier(id) = &token.token {
                has_identifier = true;
                if !name.is_empty() {
                    name.push(' ');
                }
                name.push_str(id);
                self.bump_sync();
            } else if let Token::KeywordTo = &token.token {
                break;
            } else {
                // Provide a more specific error message if we've seen at least one identifier
                if has_identifier {
                    return Err(ParseError::new(
                        format!(
                            "Expected 'to' after identifier(s), but found {:?}",
                            token.token
                        ),
                        token.line,
                        token.column,
                    ));
                } else {
                    return Err(ParseError::new(
                        format!("Expected identifier or 'to', found {:?}", token.token),
                        token.line,
                        token.column,
                    ));
                }
            }
        }

        self.expect_token(
            Token::KeywordTo,
            "Expected 'to' after variable name in change statement",
        )?;

        let value = self.parse_expression()?;

        let token_pos = self.cursor.peek().map_or(
            &TokenWithPosition {
                token: Token::KeywordChange,
                line: 0,
                column: 0,
                length: 0,
                byte_start: 0,
                byte_end: 0,
            },
            |v| v,
        );
        Ok(Statement::Assignment {
            name,
            value,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_arithmetic_operation(&mut self) -> Result<Statement, ParseError> {
        let op_token = self.bump_sync().unwrap(); // Consume "add", "subtract", "multiply", or "divide"

        // For multiply and divide: variable comes first, then "by", then value
        // For add: value comes first, then "to", then variable
        // For subtract: value comes first, then "from", then variable

        let (name, value) = match op_token.token {
            Token::KeywordAdd => {
                // add 5 to cn1
                let value = self.parse_expression()?;
                self.expect_token(Token::KeywordTo, "Expected 'to' after value in add")?;
                let name = self.parse_variable_name_simple()?;
                (name, value)
            }
            Token::KeywordSubtract => {
                // subtract 2 from cn1
                let value = self.parse_expression()?;
                self.expect_token(
                    Token::KeywordFrom,
                    "Expected 'from' after value in subtract",
                )?;
                let name = self.parse_variable_name_simple()?;
                (name, value)
            }
            Token::KeywordMultiply | Token::KeywordDivide => {
                // multiply cn1 by 3 or divide cn1 by 2
                let name = self.parse_variable_name_simple()?;
                self.expect_token(Token::KeywordBy, "Expected 'by' after variable name")?;
                let value = self.parse_expression()?;
                (name, value)
            }
            _ => unreachable!(),
        };

        // Create the appropriate operation
        let operator = match op_token.token {
            Token::KeywordAdd => Operator::Plus,
            Token::KeywordSubtract => Operator::Minus,
            Token::KeywordMultiply => Operator::Multiply,
            Token::KeywordDivide => Operator::Divide,
            _ => unreachable!(),
        };

        // Create a binary operation expression
        let var_expr = Expression::Variable(name.clone(), op_token.line, op_token.column);
        let binary_expr = Expression::BinaryOperation {
            left: Box::new(var_expr),
            operator,
            right: Box::new(value),
            line: op_token.line,
            column: op_token.column,
        };

        // Return an assignment statement
        Ok(Statement::Assignment {
            name,
            value: binary_expr,
            line: op_token.line,
            column: op_token.column,
        })
    }
}
