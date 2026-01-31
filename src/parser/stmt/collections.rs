//! Collection and data structure statement parsing

use super::super::{Expression, Literal, Operator, ParseError, Parser, Statement};
use crate::lexer::token::Token;
use crate::parser::expr::{BinaryExprParser, ExprParser, PrimaryExprParser};

pub(crate) trait CollectionParser<'a>: ExprParser<'a> {
    fn parse_create_list_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_push_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_add_operation(&mut self) -> Result<Statement, ParseError>;
    fn parse_remove_from_list_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_clear_list_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_map_creation(&mut self) -> Result<Statement, ParseError>;
    fn parse_create_date_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_create_time_statement(&mut self) -> Result<Statement, ParseError>;
}

impl<'a> CollectionParser<'a> for Parser<'a> {
    fn parse_create_list_statement(&mut self) -> Result<Statement, ParseError> {
        let create_token = self.bump_sync().unwrap(); // Consume "create"
        self.expect_token(Token::KeywordList, "Expected 'list' after 'create'")?;

        // Parse list name
        let name = if let Some(token) = self.cursor.peek() {
            match &token.token {
                Token::Identifier(n) => {
                    let name = n.clone();
                    self.bump_sync(); // Consume the identifier
                    name
                }
                _ => {
                    return Err(ParseError::from_token(
                        format!("Expected identifier for list name, found {:?}", token.token),
                        token,
                    ));
                }
            }
        } else {
            return Err(ParseError::from_token(
                "Expected list name after 'create list'".to_string(),
                create_token,
            ));
        };

        // Expect colon
        self.expect_token(Token::Colon, "Expected ':' after list name")?;

        // Skip any Eol tokens after the colon
        self.skip_eol();

        // Parse list items
        let mut initial_values = Vec::new();

        while let Some(token) = self.cursor.peek() {
            match &token.token {
                Token::KeywordEnd => {
                    self.bump_sync(); // Consume "end"
                    self.expect_token(Token::KeywordList, "Expected 'list' after 'end'")?;
                    break;
                }
                Token::KeywordAdd => {
                    self.bump_sync(); // Consume "add"
                    let value = self.parse_expression()?;
                    initial_values.push(value);
                }
                Token::Eol => {
                    self.bump_sync(); // Skip Eol between items
                    continue;
                }
                _ => {
                    return Err(ParseError::from_token(
                        format!(
                            "Expected 'add' or 'end list' in list creation, found {:?}",
                            token.token
                        ),
                        token,
                    ));
                }
            }
        }

        Ok(Statement::CreateListStatement {
            name,
            initial_values,
            line: create_token.line,
            column: create_token.column,
        })
    }

    fn parse_push_statement(&mut self) -> Result<Statement, ParseError> {
        let push_token = self.bump_sync().unwrap(); // Consume "push"

        self.expect_token(Token::KeywordWith, "Expected 'with' after 'push'")?;

        // Parse the list expression but limit it to just the primary expression
        let list_expr = self.parse_primary_expression()?;

        self.expect_token(Token::KeywordAnd, "Expected 'and' after list expression")?;

        // Parse as binary expression if not at Eol or statement starter, otherwise just primary
        let value_expr = if let Some(token) = self.cursor.peek()
            && !matches!(&token.token, Token::Eol)
            && !Parser::is_statement_starter(&token.token)
        {
            self.parse_binary_expression(0)?
        } else {
            self.parse_primary_expression()?
        };

        let stmt = Statement::PushStatement {
            list: list_expr,
            value: value_expr,
            line: push_token.line,
            column: push_token.column,
        };

        Ok(stmt)
    }

    fn parse_add_operation(&mut self) -> Result<Statement, ParseError> {
        // We need to determine if this is:
        // 1. Arithmetic: "add 5 to variable" (adds 5 to a numeric variable)
        // 2. List operation: "add "item" to list" (appends to a list)

        // Save the position to potentially backtrack
        // Position saved in cursor - use checkpoint if needed
        let add_token = self.bump_sync().unwrap(); // Consume "add"

        // Parse the value to add
        let value = self.parse_expression()?;

        // Check for "to" keyword
        if let Some(token) = self.cursor.peek() {
            if token.token == Token::KeywordTo {
                self.bump_sync(); // Consume "to"

                // Parse the target name
                let target_name = self.parse_variable_name_simple()?;

                // Try to determine the operation type
                // For now, we'll check if the value is numeric to decide
                // The interpreter will handle the actual type checking
                match &value {
                    Expression::Literal(Literal::Integer(_), _, _)
                    | Expression::Literal(Literal::Float(_), _, _) => {
                        // Likely arithmetic operation
                        let operator = Operator::Plus;
                        Ok(Statement::Assignment {
                            name: target_name.clone(),
                            value: Expression::BinaryOperation {
                                left: Box::new(Expression::Variable(
                                    target_name,
                                    add_token.line,
                                    add_token.column,
                                )),
                                operator,
                                right: Box::new(value),
                                line: add_token.line,
                                column: add_token.column,
                            },
                            line: add_token.line,
                            column: add_token.column,
                        })
                    }
                    _ => {
                        // Treat as list operation
                        Ok(Statement::AddToListStatement {
                            value,
                            list_name: target_name,
                            line: add_token.line,
                            column: add_token.column,
                        })
                    }
                }
            } else {
                // No "to" keyword, this is an error
                Err(ParseError::from_token(
                    "Expected 'to' after value in add statement".to_string(),
                    add_token,
                ))
            }
        } else {
            Err(ParseError::from_token(
                "Unexpected end of input after add value".to_string(),
                add_token,
            ))
        }
    }

    fn parse_remove_from_list_statement(&mut self) -> Result<Statement, ParseError> {
        let remove_token = self.bump_sync().unwrap(); // Consume "remove"

        // Parse the value to remove
        let value = self.parse_expression()?;

        // Expect "from"
        self.expect_token(Token::KeywordFrom, "Expected 'from' after value in remove")?;

        // Parse the list name
        let list_name = self.parse_variable_name_simple()?;

        Ok(Statement::RemoveFromListStatement {
            value,
            list_name,
            line: remove_token.line,
            column: remove_token.column,
        })
    }

    fn parse_clear_list_statement(&mut self) -> Result<Statement, ParseError> {
        let clear_token = self.bump_sync().unwrap(); // Consume "clear"

        // Parse the list name
        let list_name = self.parse_variable_name_simple()?;

        // Optionally consume "list" keyword if present
        if let Some(token) = self.cursor.peek()
            && token.token == Token::KeywordList
        {
            self.bump_sync(); // Consume "list"
        }

        Ok(Statement::ClearListStatement {
            list_name,
            line: clear_token.line,
            column: clear_token.column,
        })
    }

    fn parse_map_creation(&mut self) -> Result<Statement, ParseError> {
        let create_token = self.bump_sync().unwrap(); // Consume "create"
        self.expect_token(Token::KeywordMap, "Expected 'map' after 'create'")?;

        // Parse map name
        let name = if let Some(token) = self.cursor.peek() {
            match &token.token {
                Token::Identifier(n) => {
                    let name = n.clone();
                    self.bump_sync(); // Consume the identifier
                    name
                }
                t if t.is_contextual_keyword() => {
                    let name = self.get_token_text(t);
                    self.bump_sync(); // Consume the keyword
                    name
                }
                _ => {
                    return Err(ParseError::from_token(
                        format!("Expected identifier for map name, found {:?}", token.token),
                        token,
                    ));
                }
            }
        } else {
            return Err(ParseError::from_token(
                "Expected map name after 'create map'".to_string(),
                create_token,
            ));
        };

        // Expect colon
        self.expect_token(Token::Colon, "Expected ':' after map name")?;

        // Skip any Eol tokens after the colon
        self.skip_eol();

        // Parse map entries
        let mut entries = Vec::new();

        while let Some(token) = self.cursor.peek() {
            match &token.token {
                Token::KeywordEnd => {
                    self.bump_sync(); // Consume "end"
                    self.expect_token(Token::KeywordMap, "Expected 'map' after 'end'")?;
                    break;
                }
                Token::Eol => {
                    self.bump_sync(); // Skip Eol between items
                    continue;
                }
                Token::Identifier(key) => {
                    let key = key.clone();
                    self.bump_sync(); // Consume the key

                    // Expect "is"
                    self.expect_token(Token::KeywordIs, "Expected 'is' after map key")?;

                    // Parse the value expression
                    let value = self.parse_expression()?;

                    entries.push((key, value));
                }
                Token::StringLiteral(key) => {
                    let key = key.clone();
                    self.bump_sync(); // Consume the key

                    // Expect "is"
                    self.expect_token(Token::KeywordIs, "Expected 'is' after map key")?;

                    // Parse the value expression
                    let value = self.parse_expression()?;

                    entries.push((key, value));
                }
                _ => {
                    return Err(ParseError::from_token(
                        format!(
                            "Expected map key (identifier or string) or 'end map', found {:?}",
                            token.token
                        ),
                        token,
                    ));
                }
            }
        }

        Ok(Statement::MapCreation {
            name,
            entries,
            line: create_token.line,
            column: create_token.column,
        })
    }

    fn parse_create_date_statement(&mut self) -> Result<Statement, ParseError> {
        let create_token = self.bump_sync().unwrap(); // Consume "create"
        self.expect_token(Token::KeywordDate, "Expected 'date' after 'create'")?;

        // Parse the date variable name
        let name = self.parse_variable_name_simple()?;

        // Check if there's an "as" clause for a custom date value
        let value = if let Some(token) = self.cursor.peek() {
            if token.token == Token::KeywordAs {
                self.bump_sync(); // Consume "as"
                Some(self.parse_expression()?)
            } else {
                None // Default to "today"
            }
        } else {
            None
        };

        Ok(Statement::CreateDateStatement {
            name,
            value,
            line: create_token.line,
            column: create_token.column,
        })
    }

    fn parse_create_time_statement(&mut self) -> Result<Statement, ParseError> {
        let create_token = self.bump_sync().unwrap(); // Consume "create"
        self.expect_token(Token::KeywordTime, "Expected 'time' after 'create'")?;

        // Parse the time variable name
        let name = self.parse_variable_name_simple()?;

        // Check if there's an "as" clause for a custom time value
        let value = if let Some(token) = self.cursor.peek() {
            if token.token == Token::KeywordAs {
                self.bump_sync(); // Consume "as"
                Some(self.parse_expression()?)
            } else {
                None // Default to "now"
            }
        } else {
            None
        };

        Ok(Statement::CreateTimeStatement {
            name,
            value,
            line: create_token.line,
            column: create_token.column,
        })
    }
}
