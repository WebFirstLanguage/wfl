//! Control flow statement parsing (if, for, loops, repeat)

use super::super::{ParseError, Parser, Statement};
use super::StmtParser;
use crate::lexer::token::{Token, TokenWithPosition};
use crate::parser::expr::ExprParser;

pub(crate) trait ControlFlowParser<'a>: ExprParser<'a> {
    fn parse_if_statement(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;

    fn parse_single_line_if(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;

    fn parse_for_each_loop(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;

    fn parse_count_loop(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;

    fn parse_main_loop(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;

    fn parse_repeat_statement(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;
}

impl<'a> ControlFlowParser<'a> for Parser<'a> {
    fn parse_if_statement(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        let check_token = self.bump_sync().unwrap(); // Consume "check" and store for line/column info

        self.expect_token(Token::KeywordIf, "Expected 'if' after 'check'")?;

        let condition = self.parse_expression()?;

        if let Some(token) = self.cursor.peek()
            && matches!(token.token, Token::Colon)
        {
            self.bump_sync(); // Consume the colon if present
        }

        // Skip any Eol tokens after the colon
        self.skip_eol();

        let mut then_block = Vec::with_capacity(8);

        while let Some(token) = self.cursor.peek().cloned() {
            match &token.token {
                Token::KeywordOtherwise | Token::KeywordEnd => {
                    break;
                }
                Token::Eol => {
                    self.bump_sync(); // Skip Eol between statements
                    continue;
                }
                _ => match self.parse_statement() {
                    Ok(stmt) => then_block.push(stmt),
                    Err(e) => return Err(e),
                },
            }
        }

        // Handle the "otherwise" clause (else block)
        let else_block = if let Some(token) = self.cursor.peek() {
            if matches!(token.token, Token::KeywordOtherwise) {
                self.bump_sync(); // Consume "otherwise"

                if let Some(token) = self.cursor.peek()
                    && matches!(token.token, Token::Colon)
                {
                    self.bump_sync(); // Consume the colon if present
                }

                // Skip any Eol tokens after the colon
                self.skip_eol();

                let mut else_stmts = Vec::with_capacity(8);

                while let Some(token) = self.cursor.peek().cloned() {
                    if matches!(token.token, Token::KeywordEnd) {
                        break;
                    }
                    if matches!(token.token, Token::Eol) {
                        self.bump_sync(); // Skip Eol between statements
                        continue;
                    }

                    match self.parse_statement() {
                        Ok(stmt) => else_stmts.push(stmt),
                        Err(e) => return Err(e),
                    }
                }

                Some(else_stmts)
            } else {
                None
            }
        } else {
            None
        };

        // Handle the "end check" part
        if let Some(token) = self.cursor.peek() {
            if matches!(token.token, Token::KeywordEnd) {
                self.bump_sync(); // Consume "end"

                // Look for the "check" after "end"
                if let Some(next_token) = self.cursor.peek() {
                    if matches!(next_token.token, Token::KeywordCheck) {
                        self.bump_sync(); // Consume "check"
                    } else {
                        return Err(ParseError::from_token(
                            format!("Expected 'check' after 'end', found {:?}", next_token.token),
                            next_token,
                        ));
                    }
                } else {
                    return Err(ParseError::from_token(
                        "Expected 'check' after 'end', found end of input".to_string(),
                        token,
                    ));
                }
            } else {
                return Err(ParseError::from_token(
                    format!("Expected 'end' after if block, found {:?}", token.token),
                    token,
                ));
            }
        } else {
            return Err(ParseError::from_token(
                "Expected 'end' after if block, found end of input".to_string(),
                check_token,
            ));
        }

        Ok(Statement::IfStatement {
            condition,
            then_block,
            else_block,
            line: check_token.line,
            column: check_token.column,
        })
    }

    fn parse_single_line_if(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        let if_token = self.bump_sync().unwrap(); // Consume "if"

        let condition = self.parse_expression()?;

        self.expect_token(Token::KeywordThen, "Expected 'then' after if condition")?;

        // Check if this is a multi-line if by looking ahead for newlines or multiple statements
        let mut then_block = Vec::new();
        let mut is_multiline = false;

        // Parse then block - could be single statement or multiple statements
        while let Some(token) = self.cursor.peek() {
            match &token.token {
                Token::KeywordOtherwise | Token::KeywordEnd => {
                    is_multiline = true;
                    break;
                }
                Token::Eol => {
                    self.bump_sync(); // Consume newline
                    is_multiline = true;
                    continue;
                }
                _ => {
                    let stmt = self.parse_statement()?;
                    then_block.push(stmt);

                    // Check if there's more content after this statement
                    if let Some(next_token) = self.cursor.peek() {
                        if matches!(
                            next_token.token,
                            Token::KeywordOtherwise | Token::KeywordEnd
                        ) {
                            is_multiline = true;
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        // Handle else block
        let else_block = if let Some(token) = self.cursor.peek() {
            if matches!(token.token, Token::KeywordOtherwise) {
                self.bump_sync(); // Consume "otherwise"

                let mut else_stmts = Vec::new();
                while let Some(token) = self.cursor.peek() {
                    match &token.token {
                        Token::KeywordEnd => break,
                        Token::Eol => {
                            self.bump_sync(); // Consume newline
                            continue;
                        }
                        _ => {
                            let stmt = self.parse_statement()?;
                            else_stmts.push(stmt);
                        }
                    }
                }
                Some(else_stmts)
            } else {
                None
            }
        } else {
            None
        };

        if is_multiline {
            self.expect_token(Token::KeywordEnd, "Expected 'end' after if block")?;
            self.expect_token(Token::KeywordIf, "Expected 'if' after 'end'")?;
        }

        if is_multiline {
            Ok(Statement::IfStatement {
                condition,
                then_block,
                else_block,
                line: if_token.line,
                column: if_token.column,
            })
        } else {
            let then_stmt = if then_block.is_empty() {
                return Err(ParseError::from_token(
                    "Expected statement after 'then'".to_string(),
                    if_token,
                ));
            } else {
                Box::new(then_block.into_iter().next().unwrap())
            };

            let else_stmt = else_block.and_then(|stmts| {
                if stmts.is_empty() {
                    None
                } else {
                    Some(Box::new(stmts.into_iter().next().unwrap()))
                }
            });

            Ok(Statement::SingleLineIf {
                condition,
                then_stmt,
                else_stmt,
                line: if_token.line,
                column: if_token.column,
            })
        }
    }

    fn parse_for_each_loop(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        self.bump_sync(); // Consume "for"

        self.expect_token(Token::KeywordEach, "Expected 'each' after 'for'")?;

        let item_name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                self.bump_sync();
                id.clone()
            } else {
                return Err(ParseError::from_token(
                    format!("Expected identifier after 'each', found {:?}", token.token),
                    token,
                ));
            }
        } else {
            return Err(ParseError::from_span(
                "Unexpected end of input after 'each'".to_string(),
                crate::diagnostics::Span { start: 0, end: 0 },
                0,
                0,
            ));
        };

        self.expect_token(Token::KeywordIn, "Expected 'in' after item name")?;

        let reversed = if let Some(token) = self.cursor.peek() {
            if matches!(token.token, Token::KeywordReversed) {
                self.bump_sync(); // Consume "reversed"
                true
            } else {
                false
            }
        } else {
            false
        };

        let collection = self.parse_expression()?;

        if let Some(token) = self.cursor.peek()
            && matches!(token.token, Token::Colon)
        {
            self.bump_sync(); // Consume the colon if present
        }

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

        self.expect_token(Token::KeywordEnd, "Expected 'end' after for-each loop body")?;
        self.expect_token(Token::KeywordFor, "Expected 'for' after 'end'")?;

        let token_pos = self.cursor.peek().map_or(
            &TokenWithPosition {
                token: Token::KeywordFor,
                line: 0,
                column: 0,
                length: 0,
                byte_start: 0,
                byte_end: 0,
            },
            |v| v,
        );
        Ok(Statement::ForEachLoop {
            item_name,
            collection,
            reversed,
            body,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_count_loop(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        let count_token = self.bump_sync().unwrap(); // Consume "count"

        self.expect_token(Token::KeywordFrom, "Expected 'from' after 'count'")?;

        let start = self.parse_expression()?;

        let downward = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                if id.to_lowercase() == "down" {
                    self.bump_sync(); // Consume "down"
                    self.expect_token(Token::KeywordTo, "Expected 'to' after 'down'")?;
                    true
                } else if matches!(token.token, Token::KeywordTo) {
                    self.bump_sync(); // Consume "to"
                    false
                } else {
                    return Err(ParseError::from_token(
                        format!("Expected 'to' or 'down to', found {:?}", token.token),
                        token,
                    ));
                }
            } else if matches!(token.token, Token::KeywordTo) {
                self.bump_sync(); // Consume "to"
                false
            } else {
                return Err(ParseError::from_token(
                    format!("Expected 'to' or 'down to', found {:?}", token.token),
                    token,
                ));
            }
        } else {
            return Err(ParseError::from_token(
                "Unexpected end of input after count from expression".to_string(),
                count_token,
            ));
        };

        let end = self.parse_expression()?;

        let step = if let Some(token) = self.cursor.peek() {
            if matches!(token.token, Token::KeywordBy) {
                self.bump_sync(); // Consume "by"
                Some(self.parse_expression()?)
            } else {
                None
            }
        } else {
            None
        };

        // Parse optional "as <variable_name>" clause
        let variable_name = if let Some(token) = self.cursor.peek() {
            if matches!(token.token, Token::KeywordAs) {
                self.bump_sync(); // Consume "as"

                // Expect an identifier for the variable name
                if let Some(name_token) = self.cursor.peek() {
                    if let Token::Identifier(name) = &name_token.token {
                        let var_name = name.clone();
                        self.bump_sync(); // Consume the identifier
                        Some(var_name)
                    } else {
                        return Err(ParseError::from_token(
                            format!(
                                "Expected identifier after 'as', found {:?}",
                                name_token.token
                            ),
                            name_token,
                        ));
                    }
                } else {
                    return Err(ParseError::from_token(
                        "Unexpected end of input after 'as'".to_string(),
                        count_token,
                    ));
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some(token) = self.cursor.peek()
            && matches!(token.token, Token::Colon)
        {
            self.bump_sync(); // Consume the colon if present
        }

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

        self.expect_token(Token::KeywordEnd, "Expected 'end' after count loop body")?;
        self.expect_token(Token::KeywordCount, "Expected 'count' after 'end'")?;

        let token_pos = self.cursor.peek().map_or(
            &TokenWithPosition {
                token: Token::KeywordCount,
                line: 0,
                column: 0,
                length: 0,
                byte_start: 0,
                byte_end: 0,
            },
            |v| v,
        );
        Ok(Statement::CountLoop {
            start,
            end,
            step,
            downward,
            variable_name,
            body,
            line: token_pos.line,
            column: token_pos.column,
        })
    }

    fn parse_main_loop(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        let main_token = self.bump_sync().unwrap(); // Consume "main"
        self.expect_token(Token::KeywordLoop, "Expected 'loop' after 'main'")?;
        self.expect_token(Token::Colon, "Expected ':' after 'main loop'")?;

        // Skip any Eol tokens after the colon
        self.skip_eol();

        let mut body = Vec::new();
        while let Some(token) = self.cursor.peek().cloned() {
            if matches!(token.token, Token::KeywordEnd) {
                break;
            }
            body.push(self.parse_statement()?);
        }

        self.expect_token(Token::KeywordEnd, "Expected 'end' after main loop body")?;
        self.expect_token(Token::KeywordLoop, "Expected 'loop' after 'end'")?;

        Ok(Statement::MainLoop {
            body,
            line: main_token.line,
            column: main_token.column,
        })
    }

    fn parse_repeat_statement(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        let repeat_token = self.bump_sync().unwrap(); // Consume "repeat"

        if let Some(token) = self.cursor.peek().cloned() {
            match token.token {
                Token::KeywordWhile => {
                    self.bump_sync(); // Consume "while"
                    let condition = self.parse_expression()?;
                    if let Some(token) = self.cursor.peek()
                        && matches!(token.token, Token::Colon)
                    {
                        self.bump_sync(); // Consume the colon if present
                    }

                    // Skip any Eol tokens after the colon
                    self.skip_eol();

                    let mut body = Vec::new();
                    while let Some(token) = self.cursor.peek().cloned() {
                        if matches!(token.token, Token::KeywordEnd) {
                            break;
                        }
                        if matches!(token.token, Token::Eol) {
                            self.bump_sync(); // Skip Eol between statements
                            continue;
                        }
                        body.push(self.parse_statement()?);
                    }

                    self.expect_token(Token::KeywordEnd, "Expected 'end' after repeat while body")?;
                    self.expect_token(Token::KeywordRepeat, "Expected 'repeat' after 'end'")?;

                    Ok(Statement::RepeatWhileLoop {
                        condition,
                        body,
                        line: repeat_token.line,
                        column: repeat_token.column,
                    })
                }
                Token::KeywordUntil => {
                    self.bump_sync(); // Consume "until"
                    let condition = self.parse_expression()?;
                    if let Some(token) = self.cursor.peek()
                        && matches!(token.token, Token::Colon)
                    {
                        self.bump_sync(); // Consume the colon if present
                    }

                    // Skip any Eol tokens after the colon
                    self.skip_eol();

                    let mut body = Vec::new();
                    while let Some(token) = self.cursor.peek().cloned() {
                        if matches!(token.token, Token::KeywordEnd) {
                            break;
                        }
                        if matches!(token.token, Token::Eol) {
                            self.bump_sync(); // Skip Eol between statements
                            continue;
                        }
                        body.push(self.parse_statement()?);
                    }

                    self.expect_token(Token::KeywordEnd, "Expected 'end' after repeat until body")?;
                    self.expect_token(Token::KeywordRepeat, "Expected 'repeat' after 'end'")?;

                    Ok(Statement::RepeatUntilLoop {
                        condition,
                        body,
                        line: repeat_token.line,
                        column: repeat_token.column,
                    })
                }
                Token::KeywordForever => {
                    self.bump_sync(); // Consume "forever"
                    self.expect_token(Token::Colon, "Expected ':' after 'forever'")?;

                    // Skip any Eol tokens after the colon
                    self.skip_eol();

                    let mut body = Vec::new();
                    while let Some(token) = self.cursor.peek().cloned() {
                        if matches!(token.token, Token::KeywordEnd) {
                            break;
                        }
                        if matches!(token.token, Token::Eol) {
                            self.bump_sync(); // Skip Eol between statements
                            continue;
                        }
                        body.push(self.parse_statement()?);
                    }

                    self.expect_token(Token::KeywordEnd, "Expected 'end' after forever body")?;
                    self.expect_token(Token::KeywordRepeat, "Expected 'repeat' after 'end'")?;

                    Ok(Statement::ForeverLoop {
                        body,
                        line: repeat_token.line,
                        column: repeat_token.column,
                    })
                }
                Token::Colon => {
                    self.bump_sync(); // Consume ":"

                    // Skip any Eol tokens after the colon
                    self.skip_eol();

                    let mut body = Vec::new();
                    while let Some(token) = self.cursor.peek().cloned() {
                        if matches!(token.token, Token::KeywordUntil) {
                            break;
                        }
                        body.push(self.parse_statement()?);
                    }

                    self.expect_token(Token::KeywordUntil, "Expected 'until' after repeat body")?;
                    let condition = self.parse_expression()?;

                    Ok(Statement::RepeatUntilLoop {
                        condition,
                        body,
                        line: repeat_token.line,
                        column: repeat_token.column,
                    })
                }
                _ => Err(ParseError::from_token(
                    format!(
                        "Expected 'while', 'until', 'forever', or ':' after 'repeat', found {:?}",
                        token.token
                    ),
                    &token,
                )),
            }
        } else {
            Err(ParseError::from_token(
                "Unexpected end of input after 'repeat'".to_string(),
                repeat_token,
            ))
        }
    }
}
