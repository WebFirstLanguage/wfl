//! Process spawning and management statement parsing

use super::super::{Expression, Literal, ParseError, Parser, Statement, WriteMode};
use super::StmtParser;
use crate::lexer::token::{Token, TokenWithPosition};
use crate::parser::expr::{ExprParser, PrimaryExprParser};

pub(crate) trait ProcessParser<'a>: ExprParser<'a> {
    fn parse_execute_command_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_spawn_process_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_kill_process_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_read_process_output_statement(&mut self) -> Result<Statement, ParseError>;

    fn parse_wait_for_statement(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;
}

impl<'a> ProcessParser<'a> for Parser<'a> {
    fn parse_execute_command_statement(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.bump_sync().unwrap(); // Consume "execute"
        self.expect_token(Token::KeywordCommand, "Expected 'command' after 'execute'")?;

        let command = self.parse_primary_expression()?;

        // Check for optional "with arguments"
        let arguments = if let Some(token) = self.cursor.peek()
            && matches!(token.token, Token::KeywordWith)
        {
            self.bump_sync(); // Consume "with"
            self.expect_token(Token::KeywordArguments, "Expected 'arguments' after 'with'")?;
            Some(self.parse_primary_expression()?)
        } else {
            None
        };

        // Check for optional "using shell"
        let use_shell = if let Some(token) = self.cursor.peek()
            && matches!(token.token, Token::KeywordUsing)
        {
            self.bump_sync(); // Consume "using"
            self.expect_token(Token::KeywordShell, "Expected 'shell' after 'using'")?;
            true
        } else {
            false
        };

        // Check for optional "as variable"
        let variable_name = if let Some(token) = self.cursor.peek()
            && matches!(token.token, Token::KeywordAs)
        {
            self.bump_sync(); // Consume "as"
            let var_token = self.bump_sync().ok_or_else(|| {
                ParseError::new(
                    "Expected identifier after 'as'".to_string(),
                    token_pos.line,
                    token_pos.column,
                )
            })?;

            if let Token::Identifier(name) = &var_token.token {
                Some(name.clone())
            } else {
                return Err(ParseError::new(
                    format!("Expected identifier, found {:?}", var_token.token),
                    var_token.line,
                    var_token.column,
                ));
            }
        } else {
            None
        };

        Ok(Statement::ExecuteCommandStatement {
            command,
            arguments,
            variable_name,
            use_shell,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_spawn_process_statement(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.bump_sync().unwrap(); // Consume "spawn"
        self.expect_token(Token::KeywordCommand, "Expected 'command' after 'spawn'")?;

        let command = self.parse_primary_expression()?;

        // Check for optional "with arguments"
        let arguments = if let Some(token) = self.cursor.peek()
            && matches!(token.token, Token::KeywordWith)
        {
            self.bump_sync(); // Consume "with"
            self.expect_token(Token::KeywordArguments, "Expected 'arguments' after 'with'")?;
            Some(self.parse_primary_expression()?)
        } else {
            None
        };

        // Check for optional "using shell"
        let use_shell = if let Some(token) = self.cursor.peek()
            && matches!(token.token, Token::KeywordUsing)
        {
            self.bump_sync(); // Consume "using"
            self.expect_token(Token::KeywordShell, "Expected 'shell' after 'using'")?;
            true
        } else {
            false
        };

        // "as" is required for spawn (need to store process ID)
        self.expect_token(Token::KeywordAs, "Expected 'as' after spawn command")?;

        let var_token = self.bump_sync().ok_or_else(|| {
            ParseError::new(
                "Expected identifier after 'as'".to_string(),
                token_pos.line,
                token_pos.column,
            )
        })?;

        let variable_name = if let Token::Identifier(name) = &var_token.token {
            name.clone()
        } else {
            return Err(ParseError::new(
                format!("Expected identifier, found {:?}", var_token.token),
                var_token.line,
                var_token.column,
            ));
        };

        Ok(Statement::SpawnProcessStatement {
            command,
            arguments,
            variable_name,
            use_shell,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_kill_process_statement(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.bump_sync().unwrap(); // Consume "kill"
        self.expect_token(Token::KeywordProcess, "Expected 'process' after 'kill'")?;

        let process_id = self.parse_primary_expression()?;

        Ok(Statement::KillProcessStatement {
            process_id,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_read_process_output_statement(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.bump_sync().unwrap(); // Consume "read"
        self.expect_token(Token::KeywordOutput, "Expected 'output' after 'read'")?;
        self.expect_token(Token::KeywordFrom, "Expected 'from' after 'read output'")?;
        self.expect_token(Token::KeywordProcess, "Expected 'process' after 'from'")?;

        let process_id = self.parse_primary_expression()?;

        self.expect_token(Token::KeywordAs, "Expected 'as' after process ID")?;

        let var_token = self.bump_sync().ok_or_else(|| {
            ParseError::new(
                "Expected identifier after 'as'".to_string(),
                token_pos.line,
                token_pos.column,
            )
        })?;

        let variable_name = if let Token::Identifier(name) = &var_token.token {
            name.clone()
        } else {
            return Err(ParseError::new(
                format!("Expected identifier, found {:?}", var_token.token),
                var_token.line,
                var_token.column,
            ));
        };

        Ok(Statement::ReadProcessOutputStatement {
            process_id,
            variable_name,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_wait_for_statement(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        let wait_token_pos = self.cursor.peek().map_or(
            &TokenWithPosition {
                token: Token::KeywordWait,
                line: 0,
                column: 0,
                length: 0,
                byte_start: 0,
                byte_end: 0,
            },
            |v| v,
        );

        self.bump_sync(); // Consume "wait"
        self.expect_token(Token::KeywordFor, "Expected 'for' after 'wait'")?;

        // Check for write mode (append or write)
        let write_mode = if let Some(token) = self.cursor.peek() {
            match &token.token {
                Token::KeywordAppend => {
                    self.bump_sync(); // Consume "append"
                    WriteMode::Append
                }
                Token::KeywordWrite => {
                    self.bump_sync(); // Consume "write"
                    WriteMode::Overwrite
                }
                Token::Identifier(id) if id == "write" => {
                    self.bump_sync(); // Consume "write" identifier
                    WriteMode::Overwrite
                }
                Token::KeywordProcess => {
                    // Handle "wait for process X to complete as exit_code"
                    self.bump_sync(); // Consume "process"

                    let process_id = self.parse_primary_expression()?;

                    // Expect "to" and "complete"
                    if let Some(token) = self.cursor.peek() {
                        if matches!(token.token, Token::KeywordTo) {
                            self.bump_sync(); // Consume "to"
                        } else {
                            return Err(ParseError::new(
                                "Expected 'to' after process ID".to_string(),
                                token.line,
                                token.column,
                            ));
                        }
                    }

                    if let Some(token) = self.cursor.peek() {
                        if let Token::Identifier(id) = &token.token {
                            if id == "complete" {
                                self.bump_sync(); // Consume "complete"
                            } else {
                                return Err(ParseError::new(
                                    "Expected 'complete' after 'to'".to_string(),
                                    token.line,
                                    token.column,
                                ));
                            }
                        } else {
                            return Err(ParseError::new(
                                "Expected 'complete' after 'to'".to_string(),
                                token.line,
                                token.column,
                            ));
                        }
                    }

                    // Check for optional "as variable_name"
                    let variable_name = if let Some(token) = self.cursor.peek() {
                        if matches!(token.token, Token::KeywordAs) {
                            self.bump_sync(); // Consume "as"
                            Some(self.parse_variable_name_simple()?)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    return Ok(Statement::WaitForProcessStatement {
                        process_id,
                        variable_name,
                        line: wait_token_pos.line,
                        column: wait_token_pos.column,
                    });
                }
                Token::KeywordRequest => {
                    // Handle "wait for request comes in on server as request_name"
                    self.bump_sync(); // Consume "request"

                    // Expect "comes"
                    if let Some(token) = self.cursor.peek() {
                        if token.token == Token::KeywordComes {
                            self.bump_sync(); // Consume "comes"
                        } else {
                            return Err(ParseError::new(
                                "Expected 'comes' after 'request'".to_string(),
                                token.line,
                                token.column,
                            ));
                        }
                    }

                    // Expect "in"
                    self.expect_token(Token::KeywordIn, "Expected 'in' after 'comes'")?;

                    // Expect "on"
                    self.expect_token(Token::KeywordOn, "Expected 'on' after 'in'")?;

                    // Parse server expression
                    let server = self.parse_expression()?;

                    // Expect "as"
                    self.expect_token(Token::KeywordAs, "Expected 'as' after server")?;

                    // Parse request name
                    let request_name = self.parse_variable_name_simple()?;

                    // Check for optional timeout
                    let timeout = if let Some(token) = self.cursor.peek() {
                        if matches!(token.token, Token::KeywordWith) {
                            self.bump_sync(); // Consume "with"
                            self.expect_token(
                                Token::KeywordTimeout,
                                "Expected 'timeout' after 'with'",
                            )?;
                            Some(self.parse_expression()?)
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    return Ok(Statement::WaitForRequestStatement {
                        server,
                        request_name,
                        timeout,
                        line: wait_token_pos.line,
                        column: wait_token_pos.column,
                    });
                }
                _ => {
                    // Try to parse as "wait for X milliseconds/seconds"
                    let checkpoint = self.cursor.checkpoint();

                    // Try to parse a duration expression
                    if let Ok(duration_expr) = self.parse_expression() {
                        // Check if next token is a time unit
                        if let Some(token) = self.cursor.peek() {
                            match &token.token {
                                Token::KeywordMilliseconds => {
                                    self.bump_sync(); // Consume "milliseconds"
                                    return Ok(Statement::WaitForDurationStatement {
                                        duration: duration_expr,
                                        unit: "milliseconds".to_string(),
                                        line: wait_token_pos.line,
                                        column: wait_token_pos.column,
                                    });
                                }
                                Token::Identifier(id) if id == "seconds" => {
                                    self.bump_sync(); // Consume "seconds"
                                    return Ok(Statement::WaitForDurationStatement {
                                        duration: duration_expr,
                                        unit: "seconds".to_string(),
                                        line: wait_token_pos.line,
                                        column: wait_token_pos.column,
                                    });
                                }
                                _ => {
                                    // Not a duration, restore checkpoint and parse as statement
                                    self.cursor.rewind(checkpoint);
                                }
                            }
                        } else {
                            // No more tokens, restore checkpoint
                            self.cursor.rewind(checkpoint);
                        }
                    } else {
                        // Failed to parse expression, restore checkpoint
                        self.cursor.rewind(checkpoint);
                    }

                    // Fall back to parsing as a statement
                    let inner = Box::new(self.parse_statement()?);
                    return Ok(Statement::WaitForStatement {
                        inner,
                        line: wait_token_pos.line,
                        column: wait_token_pos.column,
                    });
                }
            }
        } else {
            return Err(ParseError::new("Unexpected end of input".to_string(), 0, 0));
        };

        if let Some(token) = self.cursor.peek() {
            // Check for "content" keyword
            if matches!(token.token, Token::KeywordContent)
                || matches!(token.token, Token::Identifier(ref id) if id == "content")
            {
                self.bump_sync(); // Consume "content"

                let content = self.parse_expression()?;

                self.expect_token(
                    Token::KeywordInto,
                    "Expected 'into' after content expression",
                )?;

                let file = self.parse_expression()?;

                let write_stmt = Statement::WriteFileStatement {
                    file,
                    content,
                    mode: write_mode,
                    line: wait_token_pos.line,
                    column: wait_token_pos.column,
                };

                return Ok(Statement::WaitForStatement {
                    inner: Box::new(write_stmt),
                    line: wait_token_pos.line,
                    column: wait_token_pos.column,
                });
            }
        }

        Err(ParseError::new(
            "Expected 'content' after 'write' or 'append'".to_string(),
            wait_token_pos.line,
            wait_token_pos.column,
        ))
    }
}
