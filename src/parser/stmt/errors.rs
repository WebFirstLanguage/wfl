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
        while let Some(token) = self.cursor.peek() {
            if matches!(
                token.token,
                Token::KeywordWhen
                    | Token::KeywordCatch
                    | Token::KeywordOtherwise
                    | Token::KeywordFinally
                    | Token::KeywordEnd
            ) {
                break;
            }
            if matches!(&token.token, Token::Eol) {
                self.bump_sync(); // Skip Eol between statements
                continue;
            }
            body.push(self.parse_statement()?);
        }

        let mut when_clauses = Vec::new();
        let mut otherwise_block = None;
        let mut finally_block = None;

        // Parse when clauses
        while let Some(token) = self.cursor.peek() {
            match &token.token {
                Token::KeywordWhen => {
                    self.bump_sync(); // Consume "when"

                    // Parse error type
                    let (error_type, mut error_name) = if let Some(next_token) = self.cursor.peek()
                    {
                        match &next_token.token {
                            Token::KeywordError => {
                                self.bump_sync(); // Consume "error"
                                (ast::ErrorType::General, "error".to_string())
                            }
                            // Bare "when:" is shorthand for "when error:"
                            Token::Colon => (ast::ErrorType::General, "error".to_string()),
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
                                if let Some(next) = self.cursor.peek() {
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
                                            if let Some(failed) = self.cursor.peek() {
                                                if let Token::Identifier(fid) = &failed.token {
                                                    if fid == "failed" {
                                                        self.bump_sync(); // Consume "failed"
                                                        (
                                                            ast::ErrorType::ProcessSpawnFailed,
                                                            "error".to_string(),
                                                        )
                                                    } else {
                                                        return Err(ParseError::from_token(
                                                            "Expected 'failed' after 'spawn'"
                                                                .to_string(),
                                                            failed,
                                                        ));
                                                    }
                                                } else {
                                                    return Err(ParseError::from_token(
                                                        "Expected 'failed' after 'spawn'"
                                                            .to_string(),
                                                        failed,
                                                    ));
                                                }
                                            } else {
                                                return Err(ParseError::from_token(
                                                    "Expected 'failed' after 'spawn'".to_string(),
                                                    next,
                                                ));
                                            }
                                        }
                                        Token::Identifier(id) if id == "kill" => {
                                            self.bump_sync(); // Consume "kill"
                                            if let Some(failed) = self.cursor.peek() {
                                                if let Token::Identifier(fid) = &failed.token {
                                                    if fid == "failed" {
                                                        self.bump_sync(); // Consume "failed"
                                                        (
                                                            ast::ErrorType::ProcessKillFailed,
                                                            "error".to_string(),
                                                        )
                                                    } else {
                                                        return Err(ParseError::from_token(
                                                            "Expected 'failed' after 'kill'"
                                                                .to_string(),
                                                            failed,
                                                        ));
                                                    }
                                                } else {
                                                    return Err(ParseError::from_token(
                                                        "Expected 'failed' after 'kill'"
                                                            .to_string(),
                                                        failed,
                                                    ));
                                                }
                                            } else {
                                                return Err(ParseError::from_token(
                                                    "Expected 'failed' after 'kill'".to_string(),
                                                    next,
                                                ));
                                            }
                                        }
                                        _ => {
                                            return Err(ParseError::from_token(
                                                "Expected 'not found', 'spawn failed', or 'kill failed' after 'process'".to_string(),
                                                next,
                                            ));
                                        }
                                    }
                                } else {
                                    return Err(ParseError::from_token(
                                        "Unexpected end after 'process'".to_string(),
                                        next_token,
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
                                return Err(ParseError::from_token(
                                    format!(
                                        "Expected 'error', 'file', 'permission', 'process', or 'command' after 'when', found {:?}",
                                        next_token.token
                                    ),
                                    next_token,
                                ));
                            }
                        }
                    } else {
                        return Err(ParseError::from_token(
                            "Unexpected end of input after 'when'".to_string(),
                            token,
                        ));
                    };

                    // Optional "as <name>" to bind the caught error under a
                    // custom name, e.g. `when error as e:`. Without it the error
                    // is still available via the implicit `error_message` alias.
                    if let Some(as_token) = self.cursor.peek()
                        && matches!(&as_token.token, Token::KeywordAs)
                    {
                        self.bump_sync(); // Consume "as"
                        if let Some(name_token) = self.cursor.peek() {
                            if let Token::Identifier(name) = &name_token.token {
                                error_name = name.clone();
                                self.bump_sync(); // Consume the binding name
                            } else {
                                return Err(ParseError::from_token(
                                    "Expected a name after 'as' in error handler".to_string(),
                                    name_token,
                                ));
                            }
                        } else {
                            return Err(ParseError::from_token(
                                "Expected a name after 'as' in error handler".to_string(),
                                as_token,
                            ));
                        }
                    }

                    self.expect_token(Token::Colon, "Expected ':' after error type")?;

                    // Skip any Eol tokens after the colon
                    self.skip_eol();

                    let mut when_body = Vec::new();
                    while let Some(token) = self.cursor.peek() {
                        if matches!(
                            token.token,
                            Token::KeywordWhen
                                | Token::KeywordCatch
                                | Token::KeywordOtherwise
                                | Token::KeywordFinally
                                | Token::KeywordEnd
                        ) {
                            break;
                        }
                        if matches!(&token.token, Token::Eol) {
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
                        if matches!(&next_token.token, Token::KeywordWith) {
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
                    while let Some(token) = self.cursor.peek() {
                        if matches!(
                            token.token,
                            Token::KeywordWhen
                                | Token::KeywordCatch
                                | Token::KeywordOtherwise
                                | Token::KeywordFinally
                                | Token::KeywordEnd
                        ) {
                            break;
                        }
                        if matches!(&token.token, Token::Eol) {
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
                    while let Some(token) = self.cursor.peek() {
                        if matches!(&token.token, Token::KeywordFinally | Token::KeywordEnd) {
                            break;
                        }
                        if matches!(&token.token, Token::Eol) {
                            self.bump_sync(); // Skip Eol between statements
                            continue;
                        }
                        otherwise_body.push(self.parse_statement()?);
                    }
                    otherwise_block = Some(otherwise_body);
                    // Do not break: a `finally:` clause may still follow.
                }
                Token::KeywordFinally => {
                    self.bump_sync(); // Consume "finally"
                    self.expect_token(Token::Colon, "Expected ':' after 'finally'")?;

                    // Skip any Eol tokens after the colon
                    self.skip_eol();

                    let mut finally_body = Vec::new();
                    while let Some(token) = self.cursor.peek() {
                        if matches!(&token.token, Token::KeywordEnd) {
                            break;
                        }
                        if matches!(&token.token, Token::Eol) {
                            self.bump_sync(); // Skip Eol between statements
                            continue;
                        }
                        finally_body.push(self.parse_statement()?);
                    }
                    finally_block = Some(finally_body);
                    break;
                }
                Token::KeywordEnd => {
                    break;
                }
                _ => {
                    return Err(ParseError::from_token(
                        format!(
                            "Expected 'when', 'catch', 'otherwise', or 'end', found {:?}",
                            token.token
                        ),
                        token,
                    ));
                }
            }
        }

        // A try must catch something or clean up: at least one 'when'/'catch'
        // clause, or a 'finally' block.
        if when_clauses.is_empty() && finally_block.is_none() {
            return Err(ParseError::from_token(
                "Try statement must have at least one 'when', 'catch', or 'finally' clause"
                    .to_string(),
                try_token,
            ));
        }

        self.expect_token(Token::KeywordEnd, "Expected 'end' after try block")?;
        self.expect_token(Token::KeywordTry, "Expected 'try' after 'end'")?;

        Ok(Statement::TryStatement {
            body,
            when_clauses,
            otherwise_block,
            finally_block,
            line: try_token.line,
            column: try_token.column,
        })
    }
}
