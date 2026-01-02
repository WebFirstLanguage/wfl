//! Primary expression parsing
//!
//! This module handles parsing of atomic expressions (literals, variables, etc.)

use super::super::{Argument, Expression, Literal, ParseError, Parser, UnaryOperator};
use super::{BinaryExprParser, ExprParser};
use crate::exec_trace;
use crate::lexer::token::Token;
use crate::parser::stmt::PatternParser;

/// Trait for parsing primary (atomic) expressions
pub(crate) trait PrimaryExprParser<'a> {
    /// Parses a primary expression (atomic expression like literals, variables, etc.)
    fn parse_primary_expression(&mut self) -> Result<Expression, ParseError>;

    /// Parses a single list element without parsing binary operators
    fn parse_list_element(&mut self) -> Result<Expression, ParseError>;
}

impl<'a> PrimaryExprParser<'a> for Parser<'a> {
    fn parse_primary_expression(&mut self) -> Result<Expression, ParseError> {
        if let Some(token) = self.cursor.peek().cloned() {
            let result = match &token.token {
                Token::LeftBracket => {
                    let bracket_token = self.bump_sync().unwrap(); // Consume '['

                    // Check for empty list
                    if let Some(next_token) = self.cursor.peek()
                        && next_token.token == Token::RightBracket
                    {
                        self.bump_sync(); // Consume ']'
                        return Ok(Expression::Literal(
                            Literal::List(Vec::new()),
                            bracket_token.line,
                            bracket_token.column,
                        ));
                    }

                    let mut elements = Vec::new();

                    // Parse first element - use a special method that doesn't parse operators
                    elements.push(self.parse_list_element()?);

                    while let Some(next_token) = self.cursor.peek() {
                        if next_token.token == Token::RightBracket {
                            self.bump_sync(); // Consume ']'
                            return Ok(Expression::Literal(
                                Literal::List(elements),
                                bracket_token.line,
                                bracket_token.column,
                            ));
                        } else if next_token.token == Token::KeywordAnd
                            || next_token.token == Token::Colon
                            || next_token.token == Token::Comma
                        {
                            self.bump_sync(); // Consume separator
                            elements.push(self.parse_list_element()?);
                        } else {
                            let err_token = next_token.clone();
                            return Err(ParseError::from_token(
                                format!(
                                    "Expected ']', ',' or 'and' in list literal, found {:?}",
                                    err_token.token
                                ),
                                &err_token,
                            ));
                        }
                    }

                    return Err(ParseError::from_token(
                        "Unexpected end of input while parsing list literal".into(),
                        bracket_token,
                    ));
                }
                Token::LeftParen => {
                    self.bump_sync(); // Consume '('
                    let expr = self.parse_expression()?;

                    if let Some(token) = self.cursor.peek().cloned() {
                        if token.token == Token::RightParen {
                            self.bump_sync(); // Consume ')'
                            return Ok(expr);
                        } else {
                            return Err(ParseError::from_token(
                                format!("Expected closing parenthesis, found {:?}", token.token),
                                &token,
                            ));
                        }
                    } else {
                        return Err(ParseError::from_token(
                            "Expected closing parenthesis, found end of input".into(),
                            &token,
                        ));
                    }
                }
                Token::StringLiteral(s) => {
                    let token_pos = self.bump_sync().unwrap();
                    Ok(Expression::Literal(
                        Literal::String(s.to_string()),
                        token_pos.line,
                        token_pos.column,
                    ))
                }
                Token::IntLiteral(n) => {
                    let token_pos = self.bump_sync().unwrap();
                    Ok(Expression::Literal(
                        Literal::Integer(*n),
                        token_pos.line,
                        token_pos.column,
                    ))
                }
                Token::FloatLiteral(f) => {
                    let token_pos = self.bump_sync().unwrap();
                    Ok(Expression::Literal(
                        Literal::Float(*f),
                        token_pos.line,
                        token_pos.column,
                    ))
                }
                Token::BooleanLiteral(b) => {
                    let token_pos = self.bump_sync().unwrap();
                    Ok(Expression::Literal(
                        Literal::Boolean(*b),
                        token_pos.line,
                        token_pos.column,
                    ))
                }
                Token::NothingLiteral => {
                    let token_pos = self.bump_sync().unwrap();
                    Ok(Expression::Literal(
                        Literal::Nothing,
                        token_pos.line,
                        token_pos.column,
                    ))
                }
                Token::KeywordCall => {
                    let call_line = token.line;
                    let call_column = token.column;
                    self.bump_sync(); // Consume 'call'
                    return self.parse_call_expression(call_line, call_column);
                }
                Token::Identifier(name) => {
                    self.bump_sync();
                    let token_line = token.line;
                    let token_column = token.column;

                    // Check for property access (dot notation)
                    if let Some(next_token) = self.cursor.peek().cloned() {
                        if next_token.token == Token::Dot {
                            self.bump_sync(); // Consume '.'

                            if let Some(property_token) = self.cursor.peek().cloned() {
                                if let Token::Identifier(property_name) = &property_token.token {
                                    self.bump_sync(); // Consume property name

                                    // Check for method call with parentheses
                                    if let Some(paren_token) = self.cursor.peek().cloned()
                                        && paren_token.token == Token::LeftParen
                                    {
                                        self.bump_sync(); // Consume '('

                                        let mut arguments = Vec::new();

                                        if let Some(next_token) = self.cursor.peek()
                                            && next_token.token != Token::RightParen
                                        {
                                            let expr = self.parse_expression()?;
                                            arguments.push(Argument {
                                                name: None,
                                                value: expr,
                                            });

                                            while let Some(comma_token) = self.cursor.peek() {
                                                if comma_token.token == Token::Comma {
                                                    self.bump_sync(); // Consume ','
                                                    let expr = self.parse_expression()?;
                                                    arguments.push(Argument {
                                                        name: None,
                                                        value: expr,
                                                    });
                                                } else {
                                                    break;
                                                }
                                            }
                                        }

                                        self.expect_token(
                                            Token::RightParen,
                                            "Expected ')' after method arguments",
                                        )?;

                                        return Ok(Expression::MethodCall {
                                            object: Box::new(Expression::Variable(
                                                name.clone(),
                                                token_line,
                                                token_column,
                                            )),
                                            method: property_name.clone(),
                                            arguments,
                                            line: token_line,
                                            column: token_column,
                                        });
                                    }

                                    // Property access without method call
                                    return Ok(Expression::PropertyAccess {
                                        object: Box::new(Expression::Variable(
                                            name.clone(),
                                            token_line,
                                            token_column,
                                        )),
                                        property: property_name.clone(),
                                        line: token_line,
                                        column: token_column,
                                    });
                                } else {
                                    return Err(ParseError::from_token(
                                        "Expected property name after '.'".to_string(),
                                        &property_token,
                                    ));
                                }
                            } else {
                                return Err(ParseError::from_token(
                                    "Expected property name after '.'".to_string(),
                                    &token,
                                ));
                            }
                        } else if let Token::Identifier(id) = &next_token.token
                            && id.to_lowercase() == "with"
                        {
                            self.bump_sync(); // Consume "with"

                            let arguments = self.parse_argument_list()?;

                            return Ok(Expression::ActionCall {
                                name: name.clone(),
                                arguments,
                                line: token_line,
                                column: token_column,
                            });
                        }
                    }

                    let is_standalone = false;

                    if is_standalone {
                        exec_trace!(
                            "Found standalone identifier '{}', treating as function call",
                            name
                        );
                        Ok(Expression::ActionCall {
                            name: name.clone(),
                            arguments: Vec::new(),
                            line: token_line,
                            column: token_column,
                        })
                    } else {
                        Ok(Expression::Variable(name.clone(), token_line, token_column))
                    }
                }
                Token::KeywordNot => {
                    self.bump_sync(); // Consume "not"
                    let expr = self.parse_primary_expression()?;
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::UnaryOperation {
                        operator: UnaryOperator::Not,
                        expression: Box::new(expr),
                        line: token_line,
                        column: token_column,
                    })
                }
                Token::Minus => {
                    self.bump_sync(); // Consume "-"
                    let expr = self.parse_primary_expression()?;
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::UnaryOperation {
                        operator: UnaryOperator::Minus,
                        expression: Box::new(expr),
                        line: token_line,
                        column: token_column,
                    })
                }
                Token::KeywordWith => {
                    self.bump_sync(); // Consume "with"
                    let expr = self.parse_expression()?;
                    Ok(expr)
                }
                Token::KeywordCount => {
                    self.bump_sync(); // Consume "count"
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "count".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordPattern => {
                    self.bump_sync(); // Consume "pattern"

                    if let Some(pattern_token) = self.cursor.peek().cloned() {
                        if let Token::StringLiteral(pattern) = &pattern_token.token {
                            let token_pos = self.bump_sync().unwrap();
                            return Ok(Expression::Literal(
                                Literal::Pattern(pattern.clone()),
                                token_pos.line,
                                token_pos.column,
                            ));
                        } else {
                            return Err(ParseError::from_token(
                                format!(
                                    "Expected string literal after 'pattern', found {:?}",
                                    pattern_token.token
                                ),
                                &pattern_token,
                            ));
                        }
                    } else {
                        return Err(ParseError::from_token(
                            "Unexpected end of input after 'pattern'".to_string(),
                            &token,
                        ));
                    }
                }
                Token::KeywordLoop => {
                    self.bump_sync(); // Consume "loop"
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "loop".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordRepeat => {
                    self.bump_sync(); // Consume "repeat"
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "repeat".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordExit => {
                    self.bump_sync(); // Consume "exit"
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "exit".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordBack => {
                    self.bump_sync(); // Consume "back"
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "back".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordTry => {
                    self.bump_sync(); // Consume "try"
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "try".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordWhen => {
                    self.bump_sync(); // Consume "when"
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "when".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordError => {
                    self.bump_sync(); // Consume "error"
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "error".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordFile => {
                    self.bump_sync(); // Consume "file"
                    let token_line = token.line;
                    let token_column = token.column;

                    // Check if it's "file exists at"
                    if let Some(next_token) = self.cursor.peek()
                        && next_token.token == Token::KeywordExists
                    {
                        self.bump_sync(); // Consume "exists"
                        self.expect_token(Token::KeywordAt, "Expected 'at' after 'file exists'")?;
                        let path = self.parse_primary_expression()?;
                        return Ok(Expression::FileExists {
                            path: Box::new(path),
                            line: token_line,
                            column: token_column,
                        });
                    }

                    // Otherwise treat "file" as a variable
                    Ok(Expression::Variable(
                        "file".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordDirectory => {
                    self.bump_sync(); // Consume "directory"
                    let token_line = token.line;
                    let token_column = token.column;

                    // Check if it's "directory exists at"
                    if let Some(next_token) = self.cursor.peek()
                        && next_token.token == Token::KeywordExists
                    {
                        self.bump_sync(); // Consume "exists"
                        self.expect_token(
                            Token::KeywordAt,
                            "Expected 'at' after 'directory exists'",
                        )?;
                        let path = self.parse_primary_expression()?;
                        return Ok(Expression::DirectoryExists {
                            path: Box::new(path),
                            line: token_line,
                            column: token_column,
                        });
                    }

                    // Otherwise treat "directory" as a variable
                    Ok(Expression::Variable(
                        "directory".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordProcess => {
                    self.bump_sync(); // Consume "process"
                    let token_line = token.line;
                    let token_column = token.column;

                    // Parse process ID expression
                    let process_id = self.parse_primary_expression()?;

                    // Check if followed by "is running"
                    if let Some(next_token) = self.cursor.peek()
                        && next_token.token == Token::KeywordIs
                    {
                        self.bump_sync(); // Consume "is"
                        self.expect_token(Token::KeywordRunning, "Expected 'running' after 'is'")?;

                        return Ok(Expression::ProcessRunning {
                            process_id: Box::new(process_id),
                            line: token_line,
                            column: token_column,
                        });
                    }

                    // Otherwise, this is an error - "process" without "is running" is not valid
                    Err(ParseError::from_token(
                        "Expected 'is running' after process ID".to_string(),
                        &token,
                    ))
                }
                Token::KeywordHeader => {
                    self.bump_sync(); // Consume "header"
                    let token_line = token.line;
                    let token_column = token.column;

                    // Parse header name (expect a string literal directly)
                    let header_name = if let Some(name_token) = self.bump_sync() {
                        match &name_token.token {
                            Token::StringLiteral(name) => name.clone(),
                            _ => {
                                return Err(ParseError::from_token(
                                    "Expected string literal for header name".to_string(),
                                    name_token,
                                ));
                            }
                        }
                    } else {
                        return Err(ParseError::from_token(
                            "Expected header name after 'header'".to_string(),
                            &token,
                        ));
                    };

                    // Expect "of"
                    self.expect_token(Token::KeywordOf, "Expected 'of' after header name")?;

                    // Parse request expression
                    let request = self.parse_primary_expression()?;

                    Ok(Expression::HeaderAccess {
                        header_name,
                        request: Box::new(request),
                        line: token_line,
                        column: token_column,
                    })
                }
                Token::KeywordCurrent => {
                    self.bump_sync(); // Consume "current"
                    let token_line = token.line;
                    let token_column = token.column;

                    // Expect "time"
                    self.expect_token(Token::KeywordTime, "Expected 'time' after 'current'")?;

                    // Check for "in milliseconds" or "formatted as"
                    if let Some(next_token) = self.cursor.peek() {
                        match next_token.token {
                            Token::KeywordIn => {
                                self.bump_sync(); // Consume "in"
                                self.expect_token(
                                    Token::KeywordMilliseconds,
                                    "Expected 'milliseconds' after 'in'",
                                )?;
                                Ok(Expression::CurrentTimeMilliseconds {
                                    line: token_line,
                                    column: token_column,
                                })
                            }
                            Token::KeywordFormatted => {
                                self.bump_sync(); // Consume "formatted"
                                self.expect_token(
                                    Token::KeywordAs,
                                    "Expected 'as' after 'formatted'",
                                )?;

                                // Parse format string
                                let format_token = self.bump_sync().ok_or_else(|| {
                                    ParseError::from_token(
                                        "Expected format string after 'as'".to_string(),
                                        &token,
                                    )
                                })?;

                                let format = match &format_token.token {
                                    Token::StringLiteral(fmt) => fmt.clone(),
                                    _ => {
                                        return Err(ParseError::from_token(
                                            "Expected string literal for time format".to_string(),
                                            format_token,
                                        ));
                                    }
                                };

                                Ok(Expression::CurrentTimeFormatted {
                                    format,
                                    line: token_line,
                                    column: token_column,
                                })
                            }
                            _ => {
                                let err_token = next_token.clone();
                                Err(ParseError::from_token(
                                    "Expected 'in milliseconds' or 'formatted as' after 'current time'"
                                        .to_string(),
                                    &err_token,
                                ))
                            }
                        }
                    } else {
                        Err(ParseError::from_token(
                            "Expected 'in milliseconds' or 'formatted as' after 'current time'"
                                .to_string(),
                            &token,
                        ))
                    }
                }
                Token::KeywordList => {
                    self.bump_sync(); // Consume "list"
                    let token_line = token.line;
                    let token_column = token.column;

                    // Check if it's "list files [recursively] in"
                    if let Some(next_token) = self.cursor.peek()
                        && next_token.token == Token::KeywordFiles
                    {
                        self.bump_sync(); // Consume "files"

                        // Check if "recursively" comes before "in"
                        let mut is_recursive = false;
                        if let Some(token) = self.cursor.peek()
                            && token.token == Token::KeywordRecursively
                        {
                            self.bump_sync(); // Consume "recursively"
                            is_recursive = true;
                        }

                        self.expect_token(
                            Token::KeywordIn,
                            "Expected 'in' after 'list files [recursively]'",
                        )?;
                        let path = self.parse_primary_expression()?;

                        // Handle recursive listing (if not already handled)
                        if is_recursive {
                            // Check for "with extension/extensions" after recursive
                            if let Some(with_token) = self.cursor.peek()
                                && with_token.token == Token::KeywordWith
                            {
                                self.bump_sync(); // Consume "with"
                                let extensions = self.parse_extension_filter()?;
                                return Ok(Expression::ListFilesRecursive {
                                    path: Box::new(path),
                                    extensions: Some(extensions),
                                    line: token_line,
                                    column: token_column,
                                });
                            }

                            // Just recursively, no filter
                            return Ok(Expression::ListFilesRecursive {
                                path: Box::new(path),
                                extensions: None,
                                line: token_line,
                                column: token_column,
                            });
                        }

                        // Check for "recursively" or "with" after the path
                        if let Some(next) = self.cursor.peek() {
                            match &next.token {
                                Token::KeywordRecursively => {
                                    self.bump_sync(); // Consume "recursively"

                                    // Check for "with extension/extensions"
                                    if let Some(with_token) = self.cursor.peek()
                                        && with_token.token == Token::KeywordWith
                                    {
                                        self.bump_sync(); // Consume "with"
                                        let extensions = self.parse_extension_filter()?;
                                        return Ok(Expression::ListFilesRecursive {
                                            path: Box::new(path),
                                            extensions: Some(extensions),
                                            line: token_line,
                                            column: token_column,
                                        });
                                    }

                                    // Just recursively, no filter
                                    return Ok(Expression::ListFilesRecursive {
                                        path: Box::new(path),
                                        extensions: None,
                                        line: token_line,
                                        column: token_column,
                                    });
                                }
                                Token::KeywordWith => {
                                    self.bump_sync(); // Consume "with"
                                    let extensions = self.parse_extension_filter()?;
                                    return Ok(Expression::ListFilesFiltered {
                                        path: Box::new(path),
                                        extensions,
                                        line: token_line,
                                        column: token_column,
                                    });
                                }
                                _ => {}
                            }
                        }

                        // Basic list files
                        return Ok(Expression::ListFiles {
                            path: Box::new(path),
                            line: token_line,
                            column: token_column,
                        });
                    }

                    // Otherwise treat "list" as a variable
                    Ok(Expression::Variable(
                        "list".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordRead => {
                    self.bump_sync(); // Consume "read"
                    let token_line = token.line;
                    let token_column = token.column;

                    // Check if it's "read content from"
                    if let Some(next_token) = self.cursor.peek()
                        && next_token.token == Token::KeywordContent
                    {
                        self.bump_sync(); // Consume "content"
                        self.expect_token(
                            Token::KeywordFrom,
                            "Expected 'from' after 'read content'",
                        )?;
                        let file_handle = self.parse_primary_expression()?;
                        return Ok(Expression::ReadContent {
                            file_handle: Box::new(file_handle),
                            line: token_line,
                            column: token_column,
                        });
                    }

                    // Otherwise treat "read" as a variable
                    Ok(Expression::Variable(
                        "read".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordFind => {
                    self.bump_sync(); // Consume "find"
                    let pattern_expr = self.parse_expression()?;
                    self.expect_token(
                        Token::KeywordIn,
                        "Expected 'in' after pattern in find expression",
                    )?;
                    let text_expr = self.parse_expression()?;
                    Ok(Expression::PatternFind {
                        pattern: Box::new(pattern_expr),
                        text: Box::new(text_expr),
                        line: token.line,
                        column: token.column,
                    })
                }
                Token::KeywordReplace => {
                    self.bump_sync(); // Consume "replace"
                    let pattern_expr = self.parse_primary_expression()?;
                    self.expect_token(
                        Token::KeywordWith,
                        "Expected 'with' after pattern in replace expression",
                    )?;
                    let replacement_expr = self.parse_expression()?;
                    self.expect_token(
                        Token::KeywordIn,
                        "Expected 'in' after replacement in replace expression",
                    )?;
                    let text_expr = self.parse_expression()?;
                    Ok(Expression::PatternReplace {
                        pattern: Box::new(pattern_expr),
                        replacement: Box::new(replacement_expr),
                        text: Box::new(text_expr),
                        line: token.line,
                        column: token.column,
                    })
                }
                Token::KeywordSplit => {
                    self.bump_sync(); // Consume "split"
                    let text_expr = self.parse_expression()?;

                    // Check for "by" (string split) or "on" (pattern split)
                    if let Some(next_token) = self.cursor.peek().cloned() {
                        match next_token.token {
                            Token::KeywordBy => {
                                // Handle "split text by delimiter" syntax
                                self.bump_sync(); // Consume "by"
                                let delimiter_expr = self.parse_expression()?;
                                Ok(Expression::StringSplit {
                                    text: Box::new(text_expr),
                                    delimiter: Box::new(delimiter_expr),
                                    line: token.line,
                                    column: token.column,
                                })
                            }
                            Token::KeywordOn => {
                                // Handle "split text on pattern name" syntax
                                self.bump_sync(); // Consume "on"
                                self.expect_token(
                                    Token::KeywordPattern,
                                    "Expected 'pattern' after 'on' in split expression",
                                )?;
                                let pattern_expr = self.parse_expression()?;
                                Ok(Expression::PatternSplit {
                                    text: Box::new(text_expr),
                                    pattern: Box::new(pattern_expr),
                                    line: token.line,
                                    column: token.column,
                                })
                            }
                            _ => Err(ParseError::from_token(
                                "Expected 'by' or 'on' after text in split expression".to_string(),
                                &token,
                            )),
                        }
                    } else {
                        Err(ParseError::from_token(
                            "Expected 'by' or 'on' after text in split expression".to_string(),
                            &token,
                        ))
                    }
                }
                _ if token.token.is_contextual_keyword() => {
                    // Special handling for "create list" expression
                    if token.token == Token::KeywordCreate {
                        self.bump_sync(); // Consume "create"
                        let token_line = token.line;
                        let token_column = token.column;

                        // Check if next token is "list"
                        if let Some(next_token) = self.cursor.peek()
                            && next_token.token == Token::KeywordList
                        {
                            self.bump_sync(); // Consume "list"
                            // Return an empty list literal
                            return Ok(Expression::Literal(
                                Literal::List(Vec::new()),
                                token_line,
                                token_column,
                            ));
                        } else {
                            // Not "create list", treat "create" as a variable
                            return Ok(Expression::Variable(
                                "create".to_string(),
                                token_line,
                                token_column,
                            ));
                        }
                    }

                    // Special handling for "contains X in Y" syntax or "contains of X and Y"
                    if token.token == Token::KeywordContains {
                        self.bump_sync(); // Consume "contains"
                        let token_line = token.line;
                        let token_column = token.column;

                        // Check if next token is "of" for old syntax
                        if let Some(next_token) = self.cursor.peek()
                            && next_token.token == Token::KeywordOf
                        {
                            // Old syntax: "contains of X and Y"
                            // Set as variable and let postfix operators handle "of"
                            Ok(Expression::Variable(
                                "contains".to_string(),
                                token_line,
                                token_column,
                            ))
                        } else {
                            // Try to parse as "contains X in Y"
                            // Parse the needle expression
                            let needle = self.parse_primary_expression()?;

                            // Check if next token is "in"
                            if let Some(in_token) = self.cursor.peek()
                                && in_token.token == Token::KeywordIn
                            {
                                self.bump_sync(); // Consume "in"

                                // Parse the haystack expression
                                let haystack = self.parse_primary_expression()?;

                                // Create a function call expression for contains
                                Ok(Expression::FunctionCall {
                                    function: Box::new(Expression::Variable(
                                        "contains".to_string(),
                                        token_line,
                                        token_column,
                                    )),
                                    arguments: vec![
                                        Argument {
                                            name: None,
                                            value: haystack,
                                        },
                                        Argument {
                                            name: None,
                                            value: needle,
                                        },
                                    ],
                                    line: token_line,
                                    column: token_column,
                                })
                            } else {
                                // Not "contains X in Y", treat as error
                                // We already parsed an expression after contains
                                Err(ParseError::from_token(
                                    "Expected 'in' after expression in contains".to_string(),
                                    &token,
                                ))
                            }
                        }
                    } else {
                        // Handle other contextual keywords as variables/identifiers
                        let name = self.get_token_text(&token.token);
                        self.bump_sync(); // Consume the contextual keyword
                        let token_line = token.line;
                        let token_column = token.column;

                        // Check for property access (dot notation) - same as identifier handling
                        if let Some(next_token) = self.cursor.peek().cloned() {
                            if next_token.token == Token::Dot {
                                self.bump_sync(); // Consume '.'

                                if let Some(property_token) = self.cursor.peek().cloned()
                                    && let Token::Identifier(property_name) = &property_token.token
                                {
                                    self.bump_sync(); // Consume property name

                                    // Check for method call with parentheses
                                    if let Some(paren_token) = self.cursor.peek().cloned()
                                        && paren_token.token == Token::LeftParen
                                    {
                                        // Handle method call - similar to identifier method handling
                                        // For now, just return property access
                                        return Ok(Expression::PropertyAccess {
                                            object: Box::new(Expression::Variable(
                                                name,
                                                token_line,
                                                token_column,
                                            )),
                                            property: property_name.clone(),
                                            line: token_line,
                                            column: token_column,
                                        });
                                    }

                                    return Ok(Expression::PropertyAccess {
                                        object: Box::new(Expression::Variable(
                                            name,
                                            token_line,
                                            token_column,
                                        )),
                                        property: property_name.clone(),
                                        line: token_line,
                                        column: token_column,
                                    });
                                }
                            } else if next_token.token == Token::LeftBracket {
                                // Handle array indexing
                                self.bump_sync(); // Consume '['
                                let index = self.parse_expression()?;

                                self.expect_token(
                                    Token::RightBracket,
                                    "Expected ']' after array index",
                                )?;

                                return Ok(Expression::IndexAccess {
                                    collection: Box::new(Expression::Variable(
                                        name,
                                        token_line,
                                        token_column,
                                    )),
                                    index: Box::new(index),
                                    line: token_line,
                                    column: token_column,
                                });
                            }
                        }

                        // Return as a simple variable
                        Ok(Expression::Variable(name, token_line, token_column))
                    }
                }
                Token::Eol => Err(ParseError::from_token(
                    "Unexpected end of line in expression".to_string(),
                    &token,
                )),
                _ => Err(ParseError::from_token(
                    format!("Unexpected token in expression: {:?}", token.token),
                    &token,
                )),
            };

            if let Ok(mut expr) = result {
                while let Some(token) = self.cursor.peek().cloned() {
                    match &token.token {
                        // Support direct index access: listName index (e.g., states 1)
                        Token::IntLiteral(index) => {
                            // Only treat as index access for specific base kinds and when on the same source line
                            if matches!(
                                expr,
                                Expression::Variable(_, _, _)
                                    | Expression::IndexAccess { .. }
                                    | Expression::FunctionCall { .. }
                                    | Expression::PropertyAccess { .. }
                                    | Expression::MethodCall { .. }
                            ) {
                                // Extract base expr span for anchoring
                                let (base_line, base_col) = match &expr {
                                    Expression::Variable(_, line, col)
                                    | Expression::IndexAccess {
                                        line, column: col, ..
                                    }
                                    | Expression::FunctionCall {
                                        line, column: col, ..
                                    }
                                    | Expression::PropertyAccess {
                                        line, column: col, ..
                                    }
                                    | Expression::MethodCall {
                                        line, column: col, ..
                                    } => (*line, *col),
                                    _ => (token.line, token.column),
                                };
                                // Since Eol terminates expressions, we can safely proceed
                                // The index literal must come immediately after the base expression
                                self.bump_sync(); // Consume the number
                                expr = Expression::IndexAccess {
                                    collection: Box::new(expr),
                                    index: Box::new(Expression::Literal(
                                        Literal::Integer(*index),
                                        token.line,
                                        token.column,
                                    )),
                                    line: base_line,
                                    column: base_col,
                                };
                            } else {
                                break; // Not an index access; stop parsing postfix operators
                            }
                        }
                        Token::KeywordOf => {
                            self.bump_sync(); // Consume "of"

                            // Parse the first argument after "of"
                            // Use parse_primary_expression to avoid treating "and" as a binary operator
                            let first_arg = self.parse_primary_expression()?;

                            let is_function_call = matches!(
                                expr,
                                Expression::Variable(_, _, _) | Expression::FunctionCall { .. }
                            );

                            if is_function_call {
                                let mut arguments = Vec::with_capacity(4);

                                arguments.push(Argument {
                                    name: None,
                                    value: first_arg,
                                });

                                while let Some(and_token) = self.cursor.peek().cloned() {
                                    if let Token::KeywordAnd = &and_token.token {
                                        self.bump_sync(); // Consume "and"

                                        // Use parse_primary_expression to avoid treating next "and" as binary operator
                                        let arg_value = self.parse_primary_expression()?;

                                        arguments.push(Argument {
                                            name: None,
                                            value: arg_value,
                                        });
                                    } else {
                                        break;
                                    }
                                }

                                expr = Expression::FunctionCall {
                                    function: Box::new(expr),
                                    arguments,
                                    line: token.line,
                                    column: token.column,
                                };
                            } else {
                                return Err(ParseError::from_token(
                                    "Member access not supported with expression arguments"
                                        .to_string(),
                                    &token,
                                ));
                            }
                        }
                        Token::KeywordAt => {
                            self.bump_sync(); // Consume "at"

                            let index = self.parse_expression()?;

                            expr = Expression::IndexAccess {
                                collection: Box::new(expr),
                                index: Box::new(index),
                                line: token.line,
                                column: token.column,
                            };
                        }
                        Token::LeftBracket => {
                            self.bump_sync(); // Consume "["

                            let index = self.parse_expression()?;

                            // Expect closing bracket
                            if let Some(closing_token) = self.cursor.peek().cloned() {
                                if closing_token.token == Token::RightBracket {
                                    self.bump_sync(); // Consume "]"
                                    expr = Expression::IndexAccess {
                                        collection: Box::new(expr),
                                        index: Box::new(index),
                                        line: token.line,
                                        column: token.column,
                                    };
                                } else {
                                    return Err(ParseError::from_token(
                                        format!(
                                            "Expected ']' after array index, found {:?}",
                                            closing_token.token
                                        ),
                                        &closing_token,
                                    ));
                                }
                            } else {
                                return Err(ParseError::from_token(
                                    "Expected ']' after array index, found end of input"
                                        .to_string(),
                                    &token,
                                ));
                            }
                        }
                        // Handle function call with parentheses: "function(args)"
                        Token::LeftParen => {
                            self.bump_sync(); // Consume '('

                            let mut arguments = Vec::new();

                            // Check for empty argument list
                            if let Some(next_token) = self.cursor.peek()
                                && next_token.token != Token::RightParen
                            {
                                // Parse first argument
                                let arg_expr = self.parse_expression()?;
                                arguments.push(Argument {
                                    name: None,
                                    value: arg_expr,
                                });

                                // Parse additional arguments separated by commas
                                while let Some(comma_token) = self.cursor.peek() {
                                    if comma_token.token == Token::Comma {
                                        self.bump_sync(); // Consume ','
                                        let arg_expr = self.parse_expression()?;
                                        arguments.push(Argument {
                                            name: None,
                                            value: arg_expr,
                                        });
                                    } else {
                                        break;
                                    }
                                }
                            }

                            self.expect_token(
                                Token::RightParen,
                                "Expected ')' after function arguments",
                            )?;

                            // Get line/column from the base expression
                            let (base_line, base_col) = match &expr {
                                Expression::Variable(_, line, col)
                                | Expression::FunctionCall { line, column: col, .. }
                                | Expression::PropertyAccess { line, column: col, .. }
                                | Expression::StaticMemberAccess { line, column: col, .. } => (*line, *col),
                                _ => (token.line, token.column),
                            };

                            expr = Expression::FunctionCall {
                                function: Box::new(expr),
                                arguments,
                                line: base_line,
                                column: base_col,
                            };
                        }
                        // Handle static member access: "Container.staticMember"
                        Token::Dot => {
                            self.bump_sync(); // Consume "."

                            if let Some(member_token) = self.cursor.peek().cloned() {
                                if let Token::Identifier(member) = &member_token.token {
                                    self.bump_sync(); // Consume member name

                                    // Extract container name from expression
                                    let container = if let Expression::Variable(name, _, _) = &expr
                                    {
                                        name.clone()
                                    } else {
                                        return Err(ParseError::from_token(
                                            "Static member access requires a container name"
                                                .to_string(),
                                            &token,
                                        ));
                                    };

                                    expr = Expression::StaticMemberAccess {
                                        container,
                                        member: member.clone(),
                                        line: token.line,
                                        column: token.column,
                                    };
                                } else {
                                    return Err(ParseError::from_token(
                                        format!(
                                            "Expected identifier after '.', found {:?}",
                                            member_token.token
                                        ),
                                        &member_token,
                                    ));
                                }
                            } else {
                                return Err(ParseError::from_token(
                                    "Unexpected end of input after '.'".to_string(),
                                    &token,
                                ));
                            }
                        }
                        _ => break,
                    }
                }

                Ok(expr)
            } else {
                result
            }
        } else {
            Err(ParseError::from_span(
                "Unexpected end of input while parsing expression".to_string(),
                crate::diagnostics::Span { start: 0, end: 0 },
                0,
                0,
            ))
        }
    }

    fn parse_list_element(&mut self) -> Result<Expression, ParseError> {
        // Parse a single list element without parsing binary operators
        // This prevents "and" from being interpreted as a boolean operator
        self.parse_primary_expression()
    }
}
