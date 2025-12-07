//! Error handling statement parsing (try/when/otherwise)

use super::super::{ParseError, Parser, Statement, ast};
use super::StmtParser;
use crate::lexer::token::Token;
use crate::parser::expr::ExprParser;

pub(crate) trait ErrorHandlingParser<'a>: ExprParser<'a> {
    fn parse_try_statement(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;
}

impl<'a> ErrorHandlingParser<'a> for Parser<'a> {
    fn parse_try_statement(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        let try_token = self.bump_sync().unwrap(); // Consume "try"
        self.expect_token(Token::Colon, "Expected ':' after 'try'")?;

        // Skip any Eol tokens after the colon
        self.skip_eol();

        let mut body = Vec::new();
        while let Some(token) = self.cursor.peek().cloned() {
            if matches!(
                token.token,
                Token::KeywordWhen
                    | Token::KeywordCatch
                    | Token::KeywordOtherwise
                    | Token::KeywordEnd
            ) {
                break;
            }
            if matches!(token.token, Token::Eol) {
                self.bump_sync(); // Skip Eol between statements
                continue;
            }
            body.push(self.parse_statement()?);
        }

        let mut when_clauses = Vec::new();
        let mut otherwise_block = None;

        // Parse when clauses
        while let Some(token) = self.cursor.peek().cloned() {
            match &token.token {
                Token::KeywordWhen => {
                    self.bump_sync(); // Consume "when"

                    // Parse error type
                    let (error_type, error_name) = if let Some(next_token) =
                        self.cursor.peek().cloned()
                    {
                        match &next_token.token {
                            Token::KeywordError => {
                                self.bump_sync(); // Consume "error"
                                (ast::ErrorType::General, "error".to_string())
                            }
                            Token::KeywordFile => {
                                self.bump_sync(); // Consume "file"
                                self.expect_token(
                                    Token::KeywordNot,
                                    "Expected 'not' after 'file'",
                                )?;
                                self.expect_token(
                                    Token::KeywordFound,
                                    "Expected 'found' after 'not'",
                                )?;
                                (ast::ErrorType::FileNotFound, "error".to_string())
                            }
                            Token::KeywordPermission => {
                                self.bump_sync(); // Consume "permission"
                                self.expect_token(
                                    Token::KeywordDenied,
                                    "Expected 'denied' after 'permission'",
                                )?;
                                (ast::ErrorType::PermissionDenied, "error".to_string())
                            }
                            Token::KeywordProcess => {
                                self.bump_sync(); // Consume "process"

                                // Check what comes next to determine error type
                                if let Some(next) = self.cursor.peek().cloned() {
                                    match &next.token {
                                        Token::KeywordNot => {
                                            self.bump_sync(); // Consume "not"
                                            self.expect_token(
                                                Token::KeywordFound,
                                                "Expected 'found' after 'not'",
                                            )?;
                                            (ast::ErrorType::ProcessNotFound, "error".to_string())
                                        }
                                        Token::Identifier(id) if id == "spawn" => {
                                            self.bump_sync(); // Consume "spawn"
                                            if let Some(failed) = self.cursor.peek().cloned() {
                                                if let Token::Identifier(fid) = &failed.token {
                                                    if fid == "failed" {
                                                        self.bump_sync(); // Consume "failed"
                                                        (
                                                            ast::ErrorType::ProcessSpawnFailed,
                                                            "error".to_string(),
                                                        )
                                                    } else {
                                                        return Err(ParseError::new(
                                                            "Expected 'failed' after 'spawn'"
                                                                .to_string(),
                                                            failed.line,
                                                            failed.column,
                                                        ));
                                                    }
                                                } else {
                                                    return Err(ParseError::new(
                                                        "Expected 'failed' after 'spawn'"
                                                            .to_string(),
                                                        failed.line,
                                                        failed.column,
                                                    ));
                                                }
                                            } else {
                                                return Err(ParseError::new(
                                                    "Expected 'failed' after 'spawn'".to_string(),
                                                    next.line,
                                                    next.column,
                                                ));
                                            }
                                        }
                                        Token::Identifier(id) if id == "kill" => {
                                            self.bump_sync(); // Consume "kill"
                                            if let Some(failed) = self.cursor.peek().cloned() {
                                                if let Token::Identifier(fid) = &failed.token {
                                                    if fid == "failed" {
                                                        self.bump_sync(); // Consume "failed"
                                                        (
                                                            ast::ErrorType::ProcessKillFailed,
                                                            "error".to_string(),
                                                        )
                                                    } else {
                                                        return Err(ParseError::new(
                                                            "Expected 'failed' after 'kill'"
                                                                .to_string(),
                                                            failed.line,
                                                            failed.column,
                                                        ));
                                                    }
                                                } else {
                                                    return Err(ParseError::new(
                                                        "Expected 'failed' after 'kill'"
                                                            .to_string(),
                                                        failed.line,
                                                        failed.column,
                                                    ));
                                                }
                                            } else {
                                                return Err(ParseError::new(
                                                    "Expected 'failed' after 'kill'".to_string(),
                                                    next.line,
                                                    next.column,
                                                ));
                                            }
                                        }
                                        _ => {
                                            return Err(ParseError::new(
                                                "Expected 'not found', 'spawn failed', or 'kill failed' after 'process'".to_string(),
                                                next.line,
                                                next.column,
                                            ));
                                        }
                                    }
                                } else {
                                    return Err(ParseError::new(
                                        "Unexpected end after 'process'".to_string(),
                                        next_token.line,
                                        next_token.column,
                                    ));
                                }
                            }
                            Token::KeywordCommand => {
                                self.bump_sync(); // Consume "command"
                                self.expect_token(
                                    Token::KeywordNot,
                                    "Expected 'not' after 'command'",
                                )?;
                                self.expect_token(
                                    Token::KeywordFound,
                                    "Expected 'found' after 'not'",
                                )?;
                                (ast::ErrorType::CommandNotFound, "error".to_string())
                            }
                            _ => {
                                return Err(ParseError::new(
                                    format!(
                                        "Expected 'error', 'file', 'permission', 'process', or 'command' after 'when', found {:?}",
                                        next_token.token
                                    ),
                                    next_token.line,
                                    next_token.column,
                                ));
                            }
                        }
                    } else {
                        return Err(ParseError::new(
                            "Unexpected end of input after 'when'".to_string(),
                            token.line,
                            token.column,
                        ));
                    };

                    self.expect_token(Token::Colon, "Expected ':' after error type")?;

                    // Skip any Eol tokens after the colon
                    self.skip_eol();

                    let mut when_body = Vec::new();
                    while let Some(token) = self.cursor.peek().cloned() {
                        if matches!(
                            token.token,
                            Token::KeywordWhen
                                | Token::KeywordCatch
                                | Token::KeywordOtherwise
                                | Token::KeywordEnd
                        ) {
                            break;
                        }
                        if matches!(token.token, Token::Eol) {
                            self.bump_sync(); // Skip Eol between statements
                            continue;
                        }
                        when_body.push(self.parse_statement()?);
                    }

                    when_clauses.push(ast::WhenClause {
                        error_type,
                        error_name,
                        body: when_body,
                    });
                }
                Token::KeywordCatch => {
                    self.bump_sync(); // Consume "catch"

                    // Check for optional "with error_name" syntax
                    let error_name = if let Some(next_token) = self.cursor.peek() {
                        if matches!(next_token.token, Token::KeywordWith) {
                            self.bump_sync(); // Consume "with"
                            if let Some(name_token) = self.cursor.peek() {
                                if let Token::Identifier(name) = &name_token.token {
                                    let name = name.clone();
                                    self.bump_sync(); // Consume identifier
                                    name
                                } else {
                                    "error".to_string()
                                }
                            } else {
                                "error".to_string()
                            }
                        } else {
                            "error".to_string()
                        }
                    } else {
                        "error".to_string()
                    };

                    self.expect_token(Token::Colon, "Expected ':' after 'catch'")?;

                    // Skip any Eol tokens after the colon
                    self.skip_eol();

                    let mut catch_body = Vec::new();
                    while let Some(token) = self.cursor.peek().cloned() {
                        if matches!(
                            token.token,
                            Token::KeywordWhen
                                | Token::KeywordCatch
                                | Token::KeywordOtherwise
                                | Token::KeywordEnd
                        ) {
                            break;
                        }
                        if matches!(token.token, Token::Eol) {
                            self.bump_sync(); // Skip Eol between statements
                            continue;
                        }
                        catch_body.push(self.parse_statement()?);
                    }

                    // Add catch as a general error when clause
                    when_clauses.push(ast::WhenClause {
                        error_type: ast::ErrorType::General,
                        error_name,
                        body: catch_body,
                    });
                }
                Token::KeywordOtherwise => {
                    self.bump_sync(); // Consume "otherwise"
                    self.expect_token(Token::Colon, "Expected ':' after 'otherwise'")?;

                    // Skip any Eol tokens after the colon
                    self.skip_eol();

                    let mut otherwise_body = Vec::new();
                    while let Some(token) = self.cursor.peek().cloned() {
                        if matches!(token.token, Token::KeywordEnd) {
                            break;
                        }
                        if matches!(token.token, Token::Eol) {
                            self.bump_sync(); // Skip Eol between statements
                            continue;
                        }
                        otherwise_body.push(self.parse_statement()?);
                    }
                    otherwise_block = Some(otherwise_body);
                    break;
                }
                Token::KeywordEnd => {
                    break;
                }
                _ => {
                    return Err(ParseError::new(
                        format!(
                            "Expected 'when', 'catch', 'otherwise', or 'end', found {:?}",
                            token.token
                        ),
                        token.line,
                        token.column,
                    ));
                }
            }
        }

        // Ensure at least one when or catch clause
        if when_clauses.is_empty() {
            return Err(ParseError::new(
                "Try statement must have at least one 'when' or 'catch' clause".to_string(),
                try_token.line,
                try_token.column,
            ));
        }

        self.expect_token(Token::KeywordEnd, "Expected 'end' after try block")?;
        self.expect_token(Token::KeywordTry, "Expected 'try' after 'end'")?;

        Ok(Statement::TryStatement {
            body,
            when_clauses,
            otherwise_block,
            line: try_token.line,
            column: try_token.column,
        })
    }
}
