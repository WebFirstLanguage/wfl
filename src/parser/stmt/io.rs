//! File I/O and filesystem statement parsing

use super::super::{Expression, FileOpenMode, Literal, ParseError, Parser, Statement};
use crate::lexer::token::Token;
use crate::parser::expr::{ExprParser, PrimaryExprParser};

pub(crate) trait IoParser<'a>: ExprParser<'a> {
    fn parse_display_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_open_file_statement(&mut self) -> Result<Statement, ParseError>;
    #[allow(dead_code)]
    fn parse_open_file_read_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_close_file_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_write_to_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_create_file_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_create_directory_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_delete_statement(&mut self) -> Result<Statement, ParseError>;
}

impl<'a> IoParser<'a> for Parser<'a> {
    fn parse_display_statement(&mut self) -> Result<Statement, ParseError> {
        self.bump_sync(); // Consume "display"

        let expr = self.parse_expression()?;

        let token_pos = if let Some(token) = self.cursor.peek() {
            token
        } else {
            return match expr {
                Expression::Literal(_, line, column) => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::Variable(_, line, column) => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::BinaryOperation { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::UnaryOperation { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::FunctionCall { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::MemberAccess { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::IndexAccess { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::Concatenation { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::PatternMatch { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::PatternFind { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::PatternReplace { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::PatternSplit { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::StringSplit { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::AwaitExpression { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::ActionCall { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::StaticMemberAccess { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::MethodCall { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::PropertyAccess { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::FileExists { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::DirectoryExists { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::ListFiles { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::ReadContent { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::ListFilesRecursive { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::ListFilesFiltered { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::HeaderAccess { line, column, .. } => Ok(Statement::DisplayStatement {
                    value: expr,
                    line,
                    column,
                }),
                Expression::CurrentTimeMilliseconds { line, column } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::CurrentTimeFormatted { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
                Expression::ProcessRunning { line, column, .. } => {
                    Ok(Statement::DisplayStatement {
                        value: expr,
                        line,
                        column,
                    })
                }
            };
        };

        Ok(Statement::DisplayStatement {
            value: expr,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_open_file_statement(&mut self) -> Result<Statement, ParseError> {
        let open_token = self.bump_sync().unwrap(); // Consume "open"

        // Check if the next token is "file" or "url"
        if let Some(next_token) = self.cursor.peek() {
            match next_token.token {
                Token::KeywordFile => {
                    // Existing file handling
                    self.bump_sync(); // Consume "file"
                }
                Token::KeywordUrl => {
                    // New URL handling
                    self.bump_sync(); // Consume "url"

                    // Continue with URL-specific parsing
                    if let Some(token) = self.cursor.peek().cloned()
                        && token.token == Token::KeywordAt
                    {
                        self.bump_sync(); // Consume "at"

                        let url_expr = self.parse_primary_expression()?;

                        // Check for "and read content as" pattern
                        if let Some(next_token) = self.cursor.peek().cloned() {
                            if next_token.token == Token::KeywordAnd {
                                self.bump_sync(); // Consume "and"
                                self.expect_token(
                                    Token::KeywordRead,
                                    "Expected 'read' after 'and'",
                                )?;
                                self.expect_token(
                                    Token::KeywordContent,
                                    "Expected 'content' after 'read'",
                                )?;
                                self.expect_token(
                                    Token::KeywordAs,
                                    "Expected 'as' after 'content'",
                                )?;

                                let variable_name = if let Some(token) = self.cursor.peek().cloned()
                                {
                                    if let Token::Identifier(name) = &token.token {
                                        self.bump_sync(); // Consume the identifier
                                        name.clone()
                                    } else {
                                        return Err(ParseError::new(
                                            format!(
                                                "Expected identifier for variable name, found {:?}",
                                                token.token
                                            ),
                                            token.line,
                                            token.column,
                                        ));
                                    }
                                } else {
                                    return Err(ParseError::new(
                                        "Unexpected end of input".to_string(),
                                        0,
                                        0,
                                    ));
                                };

                                // Use HttpGetStatement for URL handling
                                return Ok(Statement::HttpGetStatement {
                                    url: url_expr,
                                    variable_name,
                                    line: open_token.line,
                                    column: open_token.column,
                                });
                            } else if next_token.token == Token::KeywordAs {
                                // Handle "open url at "..." as variable" syntax
                                self.bump_sync(); // Consume "as"

                                let variable_name = if let Some(token) = self.cursor.peek().cloned()
                                {
                                    if let Token::Identifier(name) = &token.token {
                                        self.bump_sync(); // Consume the identifier
                                        name.clone()
                                    } else {
                                        return Err(ParseError::new(
                                            format!(
                                                "Expected identifier for variable name, found {:?}",
                                                token.token
                                            ),
                                            token.line,
                                            token.column,
                                        ));
                                    }
                                } else {
                                    return Err(ParseError::new(
                                        "Unexpected end of input".to_string(),
                                        0,
                                        0,
                                    ));
                                };

                                // Use HttpGetStatement for URL handling with direct "as" syntax
                                return Ok(Statement::HttpGetStatement {
                                    url: url_expr,
                                    variable_name,
                                    line: open_token.line,
                                    column: open_token.column,
                                });
                            } else {
                                return Err(ParseError::new(
                                    format!(
                                        "Expected 'and' or 'as' after URL, found {:?}",
                                        next_token.token
                                    ),
                                    next_token.line,
                                    next_token.column,
                                ));
                            }
                        }
                    }

                    return Err(ParseError::new(
                        "Expected 'at' after 'url'".to_string(),
                        open_token.line,
                        open_token.column + 5, // Approximate position after "open url"
                    ));
                }
                _ => {
                    return Err(ParseError::new(
                        format!(
                            "Expected 'file' or 'url' after 'open', found {:?}",
                            next_token.token
                        ),
                        next_token.line,
                        next_token.column,
                    ));
                }
            }
        } else {
            return Err(ParseError::new(
                "Unexpected end of input after 'open'".to_string(),
                open_token.line,
                open_token.column + 4, // Approximate position after "open"
            ));
        }

        if let Some(token) = self.cursor.peek().cloned()
            && token.token == Token::KeywordAt
        {
            self.bump_sync(); // Consume "at"

            let path_expr = self.parse_primary_expression()?;

            // Check for "for append", "and read content as" pattern AND direct "as" pattern
            if let Some(next_token) = self.cursor.peek().cloned() {
                if next_token.token == Token::KeywordFor {
                    // Check for "for [mode] as" pattern where mode can be append, reading, or writing
                    self.bump_sync(); // Consume "for"

                    let mode = if let Some(token) = self.cursor.peek().cloned() {
                        match token.token {
                            Token::KeywordAppend => {
                                self.bump_sync(); // Consume "append"
                                FileOpenMode::Append
                            }
                            Token::KeywordAppending => {
                                self.bump_sync(); // Consume "appending"
                                FileOpenMode::Append
                            }
                            Token::Identifier(ref mode_str) if mode_str == "reading" => {
                                self.bump_sync(); // Consume "reading"
                                FileOpenMode::Read
                            }
                            Token::Identifier(ref mode_str) if mode_str == "writing" => {
                                self.bump_sync(); // Consume "writing"
                                FileOpenMode::Write
                            }
                            _ => {
                                return Err(ParseError::new(
                                    "Expected 'append', 'appending', 'reading', or 'writing' after 'for'"
                                        .to_string(),
                                    token.line,
                                    token.column,
                                ));
                            }
                        }
                    } else {
                        return Err(ParseError::new(
                            "Expected mode after 'for'".to_string(),
                            next_token.line,
                            next_token.column,
                        ));
                    };

                    self.expect_token(Token::KeywordAs, "Expected 'as' after file mode")?;

                    let variable_name = if let Some(token) = self.cursor.peek().cloned() {
                        if let Token::Identifier(name) = &token.token {
                            self.bump_sync(); // Consume the identifier
                            name.clone()
                        } else {
                            return Err(ParseError::new(
                                format!("Expected identifier after 'as', found {:?}", token.token),
                                token.line,
                                token.column,
                            ));
                        }
                    } else {
                        return Err(ParseError::new(
                            "Unexpected end of input after 'as'".to_string(),
                            0,
                            0,
                        ));
                    };

                    return Ok(Statement::OpenFileStatement {
                        path: path_expr,
                        variable_name,
                        mode,
                        line: open_token.line,
                        column: open_token.column,
                    });
                } else if next_token.token == Token::KeywordAnd {
                    // Original pattern: "open file at "path" and read content as variable"
                    self.bump_sync(); // Consume "and"
                    self.expect_token(Token::KeywordRead, "Expected 'read' after 'and'")?;
                    self.expect_token(Token::KeywordContent, "Expected 'content' after 'read'")?;
                    self.expect_token(Token::KeywordAs, "Expected 'as' after 'content'")?;

                    let variable_name = if let Some(token) = self.cursor.peek().cloned() {
                        if let Token::Identifier(name) = &token.token {
                            self.bump_sync(); // Consume the identifier
                            name.clone()
                        } else if let Token::KeywordContent = &token.token {
                            // Special case for "content" as an identifier
                            self.bump_sync(); // Consume the "content" keyword
                            "content".to_string()
                        } else {
                            return Err(ParseError::new(
                                format!(
                                    "Expected identifier for variable name, found {:?}",
                                    token.token
                                ),
                                token.line,
                                token.column,
                            ));
                        }
                    } else {
                        return Err(ParseError::new("Unexpected end of input".to_string(), 0, 0));
                    };

                    return Ok(Statement::ReadFileStatement {
                        path: path_expr,
                        variable_name,
                        line: open_token.line,
                        column: open_token.column,
                    });
                } else if next_token.token == Token::KeywordAs {
                    // NEW pattern: "open file at "path" as variable"
                    self.bump_sync(); // Consume "as"

                    let variable_name = if let Some(token) = self.cursor.peek().cloned() {
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
                            "Unexpected end of input after 'as'".to_string(),
                            0,
                            0,
                        ));
                    };

                    return Ok(Statement::OpenFileStatement {
                        path: path_expr,
                        variable_name,
                        mode: FileOpenMode::Read,
                        line: open_token.line,
                        column: open_token.column,
                    });
                } else {
                    return Err(ParseError::new(
                        format!(
                            "Expected 'and' or 'as' after file path, found {:?}",
                            next_token.token
                        ),
                        next_token.line,
                        next_token.column,
                    ));
                }
            } else {
                return Err(ParseError::new(
                    "Unexpected end of input after file path".to_string(),
                    0,
                    0,
                ));
            }
        }

        let path = self.parse_primary_expression()?;

        self.expect_token(Token::KeywordAs, "Expected 'as' after file path")?;

        let variable_name = if let Some(token) = self.cursor.peek().cloned() {
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
                "Unexpected end of input after 'as'".to_string(),
                0,
                0,
            ));
        };

        Ok(Statement::OpenFileStatement {
            path,
            variable_name,
            mode: FileOpenMode::Read,
            line: open_token.line,
            column: open_token.column,
        })
    }

    #[allow(dead_code)]
    fn parse_open_file_read_statement(&mut self) -> Result<Statement, ParseError> {
        let open_token = self.bump_sync().unwrap(); // Consume "open"

        self.expect_token(Token::KeywordFile, "Expected 'file' after 'open'")?;
        self.expect_token(Token::KeywordAt, "Expected 'at' after 'file'")?;

        let path_expr = if let Some(token) = self.cursor.peek() {
            if let Token::StringLiteral(path_str) = &token.token {
                let line = token.line;
                let column = token.column;
                let path = path_str.clone();
                self.bump_sync(); // Consume the string literal
                Expression::Literal(Literal::String(path), line, column)
            } else {
                return Err(ParseError::new(
                    format!(
                        "Expected string literal for file path, found {:?}",
                        token.token
                    ),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new("Unexpected end of input".to_string(), 0, 0));
        };

        self.expect_token(Token::KeywordAnd, "Expected 'and' after file path")?;
        self.expect_token(Token::KeywordRead, "Expected 'read' after 'and'")?;
        self.expect_token(Token::KeywordContent, "Expected 'content' after 'read'")?;
        self.expect_token(Token::KeywordAs, "Expected 'as' after 'content'")?;

        let variable_name = if let Some(token) = self.cursor.peek().cloned() {
            if let Token::Identifier(name) = &token.token {
                self.bump_sync(); // Consume the identifier
                name.clone()
            } else if let Token::KeywordContent = &token.token {
                self.bump_sync(); // Consume the "content" keyword
                "content".to_string()
            } else {
                return Err(ParseError::new(
                    format!(
                        "Expected identifier for variable name, found {:?}",
                        token.token
                    ),
                    token.line,
                    token.column,
                ));
            }
        } else {
            return Err(ParseError::new("Unexpected end of input".to_string(), 0, 0));
        };

        Ok(Statement::ReadFileStatement {
            path: path_expr,
            variable_name,
            line: open_token.line,
            column: open_token.column,
        })
    }

    fn parse_close_file_statement(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.bump_sync().unwrap(); // Consume "close"

        // Check if the next token is "file" (for "close file file_handle" syntax)
        // Otherwise, parse the expression directly (for "close file_handle" syntax)
        if let Some(next_token) = self.cursor.peek()
            && next_token.token == Token::KeywordFile
        {
            self.bump_sync(); // Consume "file"
        }

        let file = self.parse_expression()?;

        Ok(Statement::CloseFileStatement {
            file,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_write_to_statement(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.bump_sync().unwrap(); // Consume "write"

        // Check if next token is "content" for "write content X into Y" syntax
        if let Some(next_token) = self.cursor.peek()
            && matches!(next_token.token, Token::KeywordContent)
        {
            self.bump_sync(); // Consume "content"

            let content = self.parse_expression()?;

            self.expect_token(
                Token::KeywordInto,
                "Expected 'into' after content in write content statement",
            )?;

            let target = self.parse_primary_expression()?;

            return Ok(Statement::WriteContentStatement {
                content,
                target,
                line: token_pos.line,
                column: token_pos.column,
            });
        }

        // Original "write X to Y" syntax
        let content = self.parse_expression()?;

        self.expect_token(
            Token::KeywordTo,
            "Expected 'to' after content in write statement",
        )?;

        let file = self.parse_primary_expression()?;

        Ok(Statement::WriteToStatement {
            content,
            file,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_create_file_statement(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.bump_sync().unwrap(); // Consume "create"
        self.expect_token(Token::KeywordFile, "Expected 'file' after 'create'")?;
        self.expect_token(Token::KeywordAt, "Expected 'at' after 'create file'")?;

        let path = self.parse_primary_expression()?;

        self.expect_token(Token::KeywordWith, "Expected 'with' after file path")?;
        let content = self.parse_expression()?;

        Ok(Statement::CreateFileStatement {
            path,
            content,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_create_directory_statement(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.bump_sync().unwrap(); // Consume "create"
        self.expect_token(
            Token::KeywordDirectory,
            "Expected 'directory' after 'create'",
        )?;
        self.expect_token(Token::KeywordAt, "Expected 'at' after 'create directory'")?;

        let path = self.parse_primary_expression()?;

        Ok(Statement::CreateDirectoryStatement {
            path,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_delete_statement(&mut self) -> Result<Statement, ParseError> {
        let token_pos = self.bump_sync().unwrap(); // Consume "delete"

        // Check if next token is "file" or "directory"
        if let Some(next_token) = self.cursor.peek() {
            match next_token.token {
                Token::KeywordFile => {
                    self.bump_sync(); // Consume "file"
                    self.expect_token(Token::KeywordAt, "Expected 'at' after 'delete file'")?;
                    let path = self.parse_primary_expression()?;

                    Ok(Statement::DeleteFileStatement {
                        path,
                        line: token_pos.line,
                        column: token_pos.column,
                    })
                }
                Token::KeywordDirectory => {
                    self.bump_sync(); // Consume "directory"
                    self.expect_token(Token::KeywordAt, "Expected 'at' after 'delete directory'")?;
                    let path = self.parse_primary_expression()?;

                    Ok(Statement::DeleteDirectoryStatement {
                        path,
                        line: token_pos.line,
                        column: token_pos.column,
                    })
                }
                _ => Err(ParseError::new(
                    format!(
                        "Expected 'file' or 'directory' after 'delete', found {:?}",
                        next_token.token
                    ),
                    next_token.line,
                    next_token.column,
                )),
            }
        } else {
            Err(ParseError::new(
                "Expected 'file' or 'directory' after 'delete'".to_string(),
                token_pos.line,
                token_pos.column,
            ))
        }
    }
}
