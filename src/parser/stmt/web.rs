//! Web server statement parsing

use super::super::{ParseError, Parser, Statement};
use crate::lexer::token::Token;
use crate::parser::expr::{ExprParser, PrimaryExprParser};

pub(crate) trait WebParser<'a>: ExprParser<'a> + PrimaryExprParser<'a> {
    fn parse_listen_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_respond_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_register_signal_handler_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_stop_accepting_connections_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_close_server_statement(&mut self) -> Result<Statement, ParseError>;
}

impl<'a> WebParser<'a> for Parser<'a> {
    fn parse_listen_statement(&mut self) -> Result<Statement, ParseError> {
        let listen_token = self.bump_sync().unwrap(); // Consume "listen"

        // Expect "on"
        self.expect_token(Token::KeywordOn, "Expected 'on' after 'listen'")?;

        // Expect "port"
        self.expect_token(Token::KeywordPort, "Expected 'port' after 'listen on'")?;

        // Parse port expression
        let port = self.parse_expression()?;

        // Expect "as"
        self.expect_token(Token::KeywordAs, "Expected 'as' after port")?;

        // Parse server name
        let server_name = self.parse_variable_name_simple()?;

        Ok(Statement::ListenStatement {
            port,
            server_name,
            line: listen_token.line,
            column: listen_token.column,
        })
    }

    fn parse_respond_statement(&mut self) -> Result<Statement, ParseError> {
        let respond_token = self.bump_sync().unwrap(); // Consume "respond"

        // Expect "to"
        self.expect_token(Token::KeywordTo, "Expected 'to' after 'respond'")?;

        // Parse request expression (use primary to avoid consuming "with")
        let request = self.parse_primary_expression()?;

        // Expect "with"
        self.expect_token(Token::KeywordWith, "Expected 'with' after request")?;

        // Parse content expression (use primary to avoid consuming "and")
        let content = self.parse_primary_expression()?;

        // Optional status, content_type, and headers
        let mut status = None;
        let mut content_type = None;
        let mut headers = None;

        // Check for optional "and" clauses (status and/or content_type and/or headers)
        loop {
            if let Some(token) = self.cursor.peek()
                && token.token == Token::KeywordAnd
            {
                // Look ahead to see what comes after "and"
                if let Some(next_token) = self.cursor.peek_next() {
                    if next_token.token == Token::KeywordStatus {
                        self.bump_sync(); // Consume "and"
                        self.bump_sync(); // Consume "status"
                        status = Some(self.parse_expression()?);
                        continue;
                    } else if let Token::Identifier(id) = &next_token.token
                        && (id == "content_type" || id == "content")
                    {
                        self.bump_sync(); // Consume "and"
                        self.bump_sync(); // Consume "content_type" or "content"

                        // If it was "content", expect "type" next
                        if id == "content"
                            && let Some(type_token) = self.cursor.peek()
                            && let Token::Identifier(type_id) = &type_token.token
                            && type_id == "type"
                        {
                            self.bump_sync(); // Consume "type"
                        }

                        content_type = Some(self.parse_expression()?);
                        continue;
                    } else if matches!(next_token.token, Token::KeywordHeaders | Token::KeywordHeader) {
                        self.bump_sync(); // Consume "and"
                        self.bump_sync(); // Consume "headers" or "header"
                        headers = Some(self.parse_expression()?);
                        continue;
                    }
                }
            }
            break;
        }

        Ok(Statement::RespondStatement {
            request,
            content,
            status,
            content_type,
            headers,
            line: respond_token.line,
            column: respond_token.column,
        })
    }

    fn parse_register_signal_handler_statement(&mut self) -> Result<Statement, ParseError> {
        let register_token = self.bump_sync().unwrap(); // Consume "register"

        // Expect "signal"
        self.expect_token(Token::KeywordSignal, "Expected 'signal' after 'register'")?;

        // Expect "handler"
        self.expect_token(Token::KeywordHandler, "Expected 'handler' after 'signal'")?;

        // Expect "for"
        self.expect_token(Token::KeywordFor, "Expected 'for' after 'handler'")?;

        // Parse signal type (SIGINT, SIGTERM, etc.)
        let signal_type = match self.bump_sync() {
            Some(token) => match &token.token {
                Token::Identifier(signal) => signal.clone(),
                _ => {
                    return Err(ParseError::from_token(
                        "Expected signal type (SIGINT, SIGTERM, etc.)".to_string(),
                        token,
                    ));
                }
            },
            None => {
                return Err(ParseError::from_token(
                    "Expected signal type".to_string(),
                    register_token,
                ));
            }
        };

        // Expect "as"
        self.expect_token(Token::KeywordAs, "Expected 'as' after signal type")?;

        // Parse handler name
        let handler_name = self.parse_variable_name_simple()?;

        Ok(Statement::RegisterSignalHandlerStatement {
            signal_type,
            handler_name,
            line: register_token.line,
            column: register_token.column,
        })
    }

    fn parse_stop_accepting_connections_statement(&mut self) -> Result<Statement, ParseError> {
        let stop_token = self.bump_sync().unwrap(); // Consume "stop"

        // Expect "accepting"
        self.expect_token(Token::KeywordAccepting, "Expected 'accepting' after 'stop'")?;

        // Expect "connections"
        self.expect_token(
            Token::KeywordConnections,
            "Expected 'connections' after 'accepting'",
        )?;

        // Expect "on"
        self.expect_token(Token::KeywordOn, "Expected 'on' after 'connections'")?;

        // Parse server expression
        let server = self.parse_expression()?;

        Ok(Statement::StopAcceptingConnectionsStatement {
            server,
            line: stop_token.line,
            column: stop_token.column,
        })
    }

    fn parse_close_server_statement(&mut self) -> Result<Statement, ParseError> {
        let close_token = self.bump_sync().unwrap(); // Consume "close"

        // Expect "server"
        self.expect_token(Token::KeywordServer, "Expected 'server' after 'close'")?;

        // Parse server expression
        let server = self.parse_expression()?;

        Ok(Statement::CloseServerStatement {
            server,
            line: close_token.line,
            column: close_token.column,
        })
    }
}
