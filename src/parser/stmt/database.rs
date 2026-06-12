//! Database statement parsing
//!
//! Syntax:
//! - `open database at "<url>" as <name>` (also `connect to database at ... as ...`)
//! - `store <name> as query <db> with <sql> [and parameters <list>]`
//! - `store <name> as execute <db> with <sql> [and parameters <list>]`
//! - `close database <db>`

use super::super::{ParseError, Parser, Statement};
use crate::lexer::token::Token;
use crate::parser::ast::DatabaseQueryKind;
use crate::parser::expr::{ExprParser, PrimaryExprParser};

pub(crate) trait DatabaseParser<'a>: ExprParser<'a> {
    /// Parse `open database at <url> as <name>` with the `open` token already consumed
    /// and the cursor positioned on `database`. Also used by `connect to database`,
    /// which shares everything after the `database` keyword.
    fn parse_open_database_statement(
        &mut self,
        line: usize,
        column: usize,
    ) -> Result<Statement, ParseError>;

    /// Parse `connect to database at <url> as <name>` from the `connect` identifier.
    fn parse_connect_to_database_statement(&mut self) -> Result<Statement, ParseError>;

    /// Parse `close database <db>` from the `close` keyword.
    fn parse_close_database_statement(&mut self) -> Result<Statement, ParseError>;

    /// Detect whether the cursor is positioned on the value side of a database
    /// query/execute form: `query|execute <handle> with ...`. The three-token
    /// lookahead keeps ordinary variables named `query` parsing as expressions.
    fn peek_database_query_kind(&self) -> Option<DatabaseQueryKind>;

    /// Parse the value side of `store <name> as query/execute <db> with <sql>
    /// [and parameters <list>]` with the cursor positioned on `query`/`execute`.
    fn parse_database_query_value(
        &mut self,
        name: String,
        kind: DatabaseQueryKind,
        line: usize,
        column: usize,
    ) -> Result<Statement, ParseError>;
}

impl<'a> DatabaseParser<'a> for Parser<'a> {
    fn peek_database_query_kind(&self) -> Option<DatabaseQueryKind> {
        match self.cursor.peek().map(|t| &t.token) {
            // The lexer merges adjacent identifiers, so `query db` arrives as the
            // single token Identifier("query db"). A plain variable named `query`
            // (no handle) deliberately does not match.
            Some(Token::Identifier(id)) if id.starts_with("query ") => {
                let with_ok = self
                    .cursor
                    .peek_next()
                    .is_some_and(|t| t.token == Token::KeywordWith);
                if with_ok {
                    Some(DatabaseQueryKind::Query)
                } else {
                    None
                }
            }
            Some(Token::KeywordExecute) => {
                let handle_ok = matches!(
                    self.cursor.peek_next().map(|t| &t.token),
                    Some(Token::Identifier(_))
                );
                let with_ok = self
                    .cursor
                    .peek_n(2)
                    .is_some_and(|t| t.token == Token::KeywordWith);
                if handle_ok && with_ok {
                    Some(DatabaseQueryKind::Execute)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn parse_open_database_statement(
        &mut self,
        line: usize,
        column: usize,
    ) -> Result<Statement, ParseError> {
        self.bump_sync(); // Consume "database"
        self.expect_token(Token::KeywordAt, "Expected 'at' after 'database'")?;

        let url = self.parse_primary_expression()?;

        self.expect_token(Token::KeywordAs, "Expected 'as' after database URL")?;

        let variable_name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(name) = &token.token {
                self.bump_sync();
                name.clone()
            } else {
                return Err(ParseError::from_token(
                    format!("Expected identifier after 'as', found {:?}", token.token),
                    token,
                ));
            }
        } else {
            return Err(self
                .cursor
                .error("Unexpected end of input after 'as'".to_string()));
        };

        Ok(Statement::OpenDatabaseStatement {
            url,
            variable_name,
            line,
            column,
        })
    }

    fn parse_connect_to_database_statement(&mut self) -> Result<Statement, ParseError> {
        let connect_token = self.bump_sync().unwrap(); // Consume "connect"
        self.expect_token(Token::KeywordTo, "Expected 'to' after 'connect'")?;

        if let Some(token) = self.cursor.peek() {
            if token.token != Token::KeywordDatabase {
                return Err(ParseError::from_token(
                    format!(
                        "Expected 'database' after 'connect to', found {:?}",
                        token.token
                    ),
                    token,
                ));
            }
        } else {
            return Err(self
                .cursor
                .error("Unexpected end of input after 'connect to'".to_string()));
        }

        self.parse_open_database_statement(connect_token.line, connect_token.column)
    }

    fn parse_close_database_statement(&mut self) -> Result<Statement, ParseError> {
        let close_token = self.bump_sync().unwrap(); // Consume "close"
        self.bump_sync(); // Consume "database"

        let db = self.parse_expression()?;

        Ok(Statement::CloseDatabaseStatement {
            db,
            line: close_token.line,
            column: close_token.column,
        })
    }

    fn parse_database_query_value(
        &mut self,
        name: String,
        kind: DatabaseQueryKind,
        line: usize,
        column: usize,
    ) -> Result<Statement, ParseError> {
        let db = match self.cursor.peek() {
            Some(token) => match &token.token {
                // Merged token form: Identifier("query <handle>")
                Token::Identifier(id) if id.starts_with("query ") => {
                    let handle = id["query ".len()..].to_string();
                    let (handle_line, handle_column) = (token.line, token.column);
                    self.bump_sync(); // Consume "query <handle>"
                    super::super::Expression::Variable(handle, handle_line, handle_column)
                }
                Token::KeywordExecute => {
                    self.bump_sync(); // Consume "execute"
                    self.parse_primary_expression()?
                }
                _ => {
                    return Err(ParseError::from_token(
                        format!("Expected 'query' or 'execute', found {:?}", token.token),
                        token,
                    ));
                }
            },
            None => {
                return Err(self
                    .cursor
                    .error("Unexpected end of input in database statement".to_string()));
            }
        };

        self.expect_token(Token::KeywordWith, "Expected 'with' after database handle")?;

        let sql = self.parse_primary_expression()?;

        // Optional "and parameters <expression>"
        let parameters = if self.cursor.peek().map(|t| &t.token) == Some(&Token::KeywordAnd)
            && self.cursor.peek_next().map(|t| &t.token) == Some(&Token::KeywordParameters)
        {
            self.bump_sync(); // Consume "and"
            self.bump_sync(); // Consume "parameters"
            Some(self.parse_primary_expression()?)
        } else {
            None
        };

        Ok(Statement::DatabaseQueryStatement {
            db,
            sql,
            parameters,
            variable_name: name,
            kind,
            line,
            column,
        })
    }
}
