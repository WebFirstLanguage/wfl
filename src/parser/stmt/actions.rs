//! Action definition and call statement parsing

use super::super::{Parameter, ParseError, Parser, Statement, Type};
use super::StmtParser;
use crate::exec_trace;
use crate::lexer::token::{Token, TokenWithPosition};
use crate::parser::expr::ExprParser;

pub(crate) trait ActionParser<'a>: ExprParser<'a> {
    fn parse_action_definition(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;

    fn parse_container_action_definition(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;

    fn parse_parameter_list(&mut self) -> Result<Vec<Parameter>, ParseError>;
    fn parse_return_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_exit_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_parent_method_call(&mut self) -> Result<Statement, ParseError>;
}

impl<'a> ActionParser<'a> for Parser<'a> {
    fn parse_action_definition(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        exec_trace!("Parsing action definition");
        self.bump_sync(); // Consume "define"

        exec_trace!("Expecting 'action' after 'define'");
        self.expect_token(Token::KeywordAction, "Expected 'action' after 'define'")?;

        exec_trace!("Expecting 'called' after 'action'");
        self.expect_token(Token::KeywordCalled, "Expected 'called' after 'action'")?;

        exec_trace!("Expecting identifier after 'called'");
        let name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                exec_trace!("Found action name: {}", id);
                self.bump_sync();
                id.clone()
            } else {
                exec_trace!(
                    "Expected identifier after 'called', found {:?}",
                    token.token
                );
                let err_token = token.clone();
                return Err(ParseError::from_token(
                    format!(
                        "Expected identifier after 'called', found {:?}",
                        err_token.token
                    ),
                    &err_token,
                ));
            }
        } else {
            exec_trace!("Unexpected end of input after 'called'");
            return Err(ParseError::from_span(
                "Unexpected end of input after 'called'".to_string(),
                crate::diagnostics::Span { start: 0, end: 0 },
                0,
                0,
            ));
        };

        exec_trace!("Action name parsed: {}", name);
        let mut parameters = Vec::with_capacity(4);

        if let Some(token) = self.cursor.peek().cloned()
            && (matches!(token.token, Token::KeywordNeeds)
                || matches!(token.token, Token::KeywordWith))
        {
            let _keyword = if matches!(token.token, Token::KeywordNeeds) {
                "needs"
            } else {
                "with"
            };
            exec_trace!("Found '{}' keyword, parsing parameters", _keyword);
            self.bump_sync(); // Consume "needs" or "with"

            // Skip optional "parameters" keyword for readability
            if let Some(token) = self.cursor.peek()
                && matches!(token.token, Token::KeywordParameters)
            {
                self.bump_sync(); // Consume "parameters"
                exec_trace!("Skipped 'parameters' keyword");
            }

            while let Some(token) = self.cursor.peek().cloned() {
                exec_trace!("Checking token for parameter: {:?}", token.token);
                let (param_name, param_line, param_column) =
                    if let Token::Identifier(id) = &token.token {
                        exec_trace!("Found parameter: {}", id);
                        let line = token.line;
                        let column = token.column;
                        self.bump_sync();

                        (id.clone(), line, column)
                    } else {
                        exec_trace!("Not an identifier, breaking parameter parsing");
                        break;
                    };

                let param_type = if let Some(token) = self.cursor.peek() {
                    if matches!(token.token, Token::KeywordAs) {
                        self.bump_sync(); // Consume "as"

                        if let Some(type_token) = self.cursor.peek() {
                            if let Token::Identifier(type_name) = &type_token.token {
                                self.bump_sync();

                                let typ = match type_name.as_str() {
                                    "text" => Type::Text,
                                    "number" => Type::Number,
                                    "boolean" => Type::Boolean,
                                    "nothing" => Type::Nothing,
                                    _ => Type::Custom(type_name.clone()),
                                };

                                Some(typ)
                            } else {
                                let err_token = type_token.clone();
                                return Err(ParseError::from_token(
                                    format!(
                                        "Expected type name after 'as', found {:?}",
                                        err_token.token
                                    ),
                                    &err_token,
                                ));
                            }
                        } else {
                            return Err(ParseError::from_span(
                                "Unexpected end of input after 'as'".to_string(),
                                crate::diagnostics::Span { start: 0, end: 0 },
                                0,
                                0,
                            ));
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                let default_value = if let Some(token) = self.cursor.peek() {
                    if let Token::Identifier(id) = &token.token {
                        if id.to_lowercase() == "default" {
                            self.bump_sync(); // Consume "default"

                            Some(self.parse_expression()?)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                parameters.push(Parameter {
                    name: param_name,
                    param_type,
                    default_value,
                    line: param_line,
                    column: param_column,
                });

                if let Some(token) = self.cursor.peek().cloned() {
                    if matches!(token.token, Token::KeywordAnd)
                        || matches!(token.token, Token::Identifier(ref id) if id.to_lowercase() == "and")
                    {
                        self.bump_sync(); // Consume "and"
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        let return_type = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                if id.to_lowercase() == "returns" {
                    self.bump_sync(); // Consume "returns"

                    if let Some(type_token) = self.cursor.peek() {
                        if let Token::Identifier(type_name) = &type_token.token {
                            self.bump_sync();

                            let typ = match type_name.as_str() {
                                "text" => Type::Text,
                                "number" => Type::Number,
                                "boolean" => Type::Boolean,
                                "nothing" => Type::Nothing,
                                _ => Type::Custom(type_name.clone()),
                            };

                            Some(typ)
                        } else {
                            let err_token = type_token.clone();
                            return Err(ParseError::from_token(
                                format!(
                                    "Expected type name after 'returns', found {:?}",
                                    err_token.token
                                ),
                                &err_token,
                            ));
                        }
                    } else {
                        return Err(ParseError::from_span(
                            "Unexpected end of input after 'returns'".to_string(),
                            crate::diagnostics::Span { start: 0, end: 0 },
                            0,
                            0,
                        ));
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Check for KeywordAnd that might be mistakenly present after the last parameter
        if let Some(token) = self.cursor.peek().cloned()
            && let Token::Identifier(id) = &token.token
            && id == "and"
        {
            self.bump_sync(); // Consume the extra "and"
        }

        self.expect_token(Token::Colon, "Expected ':' after action definition")?;

        // Skip any Eol tokens after the colon
        self.skip_eol();

        let mut body = Vec::with_capacity(10);

        while let Some(token) = self.cursor.peek().cloned() {
            if matches!(token.token, Token::KeywordEnd) {
                break;
            }
            if matches!(token.token, Token::Eol) {
                self.bump_sync(); // Skip Eol between statements
                continue;
            }

            match self.parse_statement() {
                Ok(stmt) => body.push(stmt),
                Err(e) => return Err(e),
            }
        }

        let start_pos = self.cursor.pos();

        if let Some(token) = self.cursor.peek() {
            if matches!(token.token, Token::KeywordEnd) {
                self.bump_sync(); // Consume "end"

                if let Some(token) = self.cursor.peek() {
                    if matches!(token.token, Token::KeywordAction) {
                        self.bump_sync(); // Consume "action"
                    } else {
                        let err_token = token.clone();
                        return Err(ParseError::from_token(
                            "Expected 'action' after 'end'".to_string(),
                            &err_token,
                        ));
                    }
                } else {
                    return Err(ParseError::from_span(
                        "Expected 'action' after 'end'".to_string(),
                        crate::diagnostics::Span { start: 0, end: 0 },
                        0,
                        0,
                    ));
                }
            } else {
                let err_token = token.clone();
                return Err(ParseError::from_token(
                    "Expected 'end' after action body".to_string(),
                    &err_token,
                ));
            }
        } else {
            return Err(ParseError::from_span(
                "Expected 'end' after action body".to_string(),
                crate::diagnostics::Span { start: 0, end: 0 },
                0,
                0,
            ));
        }

        assert!(
            self.cursor.pos() > start_pos,
            "Parser made no progress while parsing end action tokens at line {}",
            self.cursor.current_line()
        );

        let token_pos = self.cursor.peek().map_or(
            &TokenWithPosition {
                token: Token::KeywordDefine,
                line: 0,
                column: 0,
                length: 0,
                byte_start: 0,
                byte_end: 0,
            },
            |v| v,
        );
        Ok(Statement::ActionDefinition {
            name,
            parameters,
            body,
            return_type,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_container_action_definition(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        self.bump_sync(); // Consume "action"

        let name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                self.bump_sync();
                id.clone()
            } else {
                let err_token = token.clone();
                return Err(ParseError::from_token(
                    format!(
                        "Expected identifier after 'action', found {:?}",
                        err_token.token
                    ),
                    &err_token,
                ));
            }
        } else {
            return Err(ParseError::from_span(
                "Expected identifier after 'action'".to_string(),
                crate::diagnostics::Span { start: 0, end: 0 },
                0,
                0,
            ));
        };

        let mut parameters = Vec::new();

        // Check for parameters
        if let Some(token) = self.cursor.peek().cloned()
            && matches!(token.token, Token::KeywordNeeds)
        {
            self.bump_sync(); // Consume "needs"
            parameters = self.parse_parameter_list()?;
        }

        // Parse return type if present (after parameters or action name)
        let return_type = if let Some(token) = self.cursor.peek().cloned()
            && matches!(token.token, Token::Colon)
        {
            self.bump_sync(); // Consume ':'

            // Check if the next token is actually a type identifier
            // If it's not, this colon just marks the start of the action body (no return type)
            if let Some(type_token) = self.cursor.peek().cloned() {
                if let Token::Identifier(type_name) = &type_token.token {
                    // Check if this identifier is a valid type name
                    let is_type = matches!(
                        type_name.as_str(),
                        "Text" | "Number" | "Boolean" | "Nothing" | "Pattern"
                    ) || type_name.chars().next().is_some_and(|c| c.is_uppercase());

                    if is_type {
                        self.bump_sync(); // Consume type name
                        Some(match type_name.as_str() {
                            "Text" => Type::Text,
                            "Number" => Type::Number,
                            "Boolean" => Type::Boolean,
                            "Nothing" => Type::Nothing,
                            "Pattern" => Type::Pattern,
                            _ => Type::Custom(type_name.clone()),
                        })
                    } else {
                        // This identifier is not a type, so no return type specified
                        None
                    }
                } else {
                    // Next token after ':' is not an identifier, so no return type
                    None
                }
            } else {
                // End of input after ':', so no return type
                None
            }
        } else {
            None
        };

        // Skip any Eol tokens before the body
        self.skip_eol();

        let mut body = Vec::new();

        // Parse action body until 'end'
        loop {
            if let Some(token) = self.cursor.peek() {
                if token.token == Token::KeywordEnd {
                    self.bump_sync(); // Consume 'end'
                    break;
                }
                if matches!(token.token, Token::Eol) {
                    self.bump_sync(); // Skip Eol between statements
                    continue;
                }
                body.push(self.parse_statement()?);
            } else {
                return Err(ParseError::from_span(
                    "Unexpected end of input in action body".to_string(),
                    crate::diagnostics::Span { start: 0, end: 0 },
                    0,
                    0,
                ));
            }
        }

        Ok(Statement::ActionDefinition {
            name,
            parameters,
            body,
            return_type,
            line: 0,
            column: 0,
        })
    }

    fn parse_parameter_list(&mut self) -> Result<Vec<Parameter>, ParseError> {
        let mut parameters = Vec::new();

        while let Some(token) = self.cursor.peek() {
            if let Token::Identifier(param_name) = &token.token {
                let name = param_name.clone();
                let param_line = token.line;
                let param_column = token.column;
                self.bump_sync(); // Consume parameter name

                let param_type = if let Some(type_token) = self.cursor.peek() {
                    if type_token.token == Token::Colon {
                        self.bump_sync(); // Consume ':'

                        if let Some(type_name_token) = self.cursor.peek() {
                            if let Token::Identifier(type_name) = &type_name_token.token {
                                self.bump_sync(); // Consume type name
                                Some(match type_name.as_str() {
                                    "Text" => Type::Text,
                                    "Number" => Type::Number,
                                    "Boolean" => Type::Boolean,
                                    "Nothing" => Type::Nothing,
                                    "Pattern" => Type::Pattern,
                                    _ => Type::Custom(type_name.clone()),
                                })
                            } else {
                                let err_token = type_name_token.clone();
                                return Err(ParseError::from_token(
                                    "Expected type name after ':'".to_string(),
                                    &err_token,
                                ));
                            }
                        } else {
                            return Err(ParseError::from_span(
                                "Expected type name after ':'".to_string(),
                                crate::diagnostics::Span { start: 0, end: 0 },
                                param_line,
                                param_column,
                            ));
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                parameters.push(Parameter {
                    name,
                    param_type,
                    default_value: None,
                    line: param_line,
                    column: param_column,
                });

                // Check for comma to continue or break
                if let Some(next_token) = self.cursor.peek() {
                    if next_token.token == Token::Comma {
                        self.bump_sync(); // Consume comma
                        continue;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        Ok(parameters)
    }

    fn parse_return_statement(&mut self) -> Result<Statement, ParseError> {
        let return_token = self.bump_sync().unwrap(); // Consume "give" or "return"

        if matches!(return_token.token, Token::KeywordGive) {
            self.expect_token(Token::KeywordBack, "Expected 'back' after 'give'")?;
        }

        let value = if let Some(token) = self.cursor.peek().cloned() {
            if matches!(token.token, Token::NothingLiteral) {
                self.bump_sync(); // Consume "nothing"
                None
            } else {
                Some(self.parse_expression()?)
            }
        } else {
            None
        };

        Ok(Statement::ReturnStatement {
            value,
            line: return_token.line,
            column: return_token.column,
        })
    }

    fn parse_exit_statement(&mut self) -> Result<Statement, ParseError> {
        let exit_token = self.bump_sync().unwrap(); // Consume "exit"

        // Check for "loop" after "exit"
        if let Some(token) = self.cursor.peek().cloned()
            && let Token::Identifier(id) = &token.token
            && id.to_lowercase() == "loop"
        {
            self.bump_sync(); // Consume "loop"
        }

        Ok(Statement::ExitStatement {
            line: exit_token.line,
            column: exit_token.column,
        })
    }

    fn parse_parent_method_call(&mut self) -> Result<Statement, ParseError> {
        let start_token = self.bump_sync().unwrap(); // Consume 'parent'
        let line = start_token.line;
        let column = start_token.column;

        // Parse method name
        let method_name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                self.bump_sync(); // Consume the identifier
                id.clone()
            } else {
                let err_token = token.clone();
                return Err(ParseError::from_token(
                    format!(
                        "Expected identifier for method name, found {:?}",
                        err_token.token
                    ),
                    &err_token,
                ));
            }
        } else {
            return Err(ParseError::from_span(
                "Expected identifier for method name, found end of input".to_string(),
                crate::diagnostics::Span { start: 0, end: 0 },
                line,
                column,
            ));
        };

        // For now, just create a simple parent method call
        Ok(Statement::ParentMethodCall {
            method_name,
            arguments: Vec::new(),
            line,
            column,
        })
    }
}
