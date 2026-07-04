//! Web server statement parsing

use super::super::{Expression, ParseError, Parser, Statement};
use crate::lexer::token::Token;
use crate::parser::ast::TlsListenConfig;
use crate::parser::expr::{ExprParser, PrimaryExprParser};

pub(crate) trait WebParser<'a>: ExprParser<'a> + PrimaryExprParser<'a> {
    fn parse_listen_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_tls_path_value(&mut self, marker: &str) -> Result<Expression, ParseError>;
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

        // Optional clause markers. `secured` and `redirecting` are contextual
        // identifiers, not keywords, so existing programs can still use them
        // as variable names. The lexer merges adjacent identifiers, so a
        // variable port arrives glued to the marker (`my_port secured` lexes
        // as Identifier("my_port secured")). Detect that BEFORE parsing the
        // port expression: `with` is a concatenation operator, so
        // parse_expression would otherwise swallow a following
        // `with certificate ...` clause into the port expression.
        let mut marker: Option<&str> = None;
        let mut merged_port: Option<Expression> = None;
        if let Some(token) = self.cursor.peek()
            && let Token::Identifier(id) = &token.token
        {
            for candidate in ["secured", "redirecting"] {
                if let Some(stripped) = id.strip_suffix(&format!(" {candidate}")) {
                    merged_port = Some(Expression::Variable(
                        stripped.to_string(),
                        token.line,
                        token.column,
                    ));
                    marker = Some(candidate);
                    break;
                }
            }
            if marker.is_some() {
                self.bump_sync(); // Consume the merged port variable + marker
            }
        }

        // Parse port expression (unless already extracted from a merged token)
        let port = match merged_port {
            Some(port) => port,
            None => self.parse_expression()?,
        };
        if marker.is_none()
            && let Some(token) = self.cursor.peek()
            && let Token::Identifier(id) = &token.token
        {
            match id.as_str() {
                "secured" => {
                    self.bump_sync();
                    marker = Some("secured");
                }
                "redirecting" => {
                    self.bump_sync();
                    marker = Some("redirecting");
                }
                _ => {}
            }
        }

        let mut tls: Option<TlsListenConfig> = None;
        let mut redirect_to_port: Option<Expression> = None;

        match marker {
            Some("secured") => {
                if let Some(token) = self.cursor.peek()
                    && token.token == Token::KeywordWith
                {
                    // `secured with certificate <expr> and key <expr>`
                    self.bump_sync(); // Consume "with"
                    let cert_path = self.parse_tls_path_value("certificate")?;
                    self.expect_token(
                        Token::KeywordAnd,
                        "Expected 'and key ...' after certificate path",
                    )?;
                    let key_path = self.parse_tls_path_value("key")?;
                    tls = Some(TlsListenConfig {
                        cert_path: Some(cert_path),
                        key_path: Some(key_path),
                    });
                } else {
                    // Bare `secured` — certificate and key paths come from
                    // .wflcfg at runtime.
                    tls = Some(TlsListenConfig {
                        cert_path: None,
                        key_path: None,
                    });
                }

                // Reject `secured ... redirecting ...` explicitly: a listener
                // either terminates TLS or redirects to one, never both.
                if let Some(token) = self.cursor.peek()
                    && let Token::Identifier(id) = &token.token
                    && (id == "redirecting" || id.starts_with("redirecting "))
                {
                    return Err(ParseError::from_token(
                        "'secured' and 'redirecting to port' cannot be combined on one listen statement"
                            .to_string(),
                        token,
                    ));
                }
            }
            Some("redirecting") => {
                self.expect_token(Token::KeywordTo, "Expected 'to' after 'redirecting'")?;
                self.expect_token(Token::KeywordPort, "Expected 'port' after 'redirecting to'")?;
                // Primary expression only, so the following "as" stays available.
                redirect_to_port = Some(self.parse_primary_expression()?);
            }
            _ => {}
        }

        // Expect "as"
        self.expect_token(Token::KeywordAs, "Expected 'as' after port")?;

        // Parse server name
        let server_name = self.parse_variable_name_simple()?;

        Ok(Statement::ListenStatement {
            port,
            server_name,
            tls,
            redirect_to_port,
            line: listen_token.line,
            column: listen_token.column,
        })
    }

    /// Parses `<marker> <expr>` inside a `secured with ...` clause, where
    /// marker is `certificate` or `key`. The marker is a contextual
    /// identifier, so a variable value arrives merged with it
    /// (`certificate cert_path` lexes as Identifier("certificate cert_path"))
    /// while a string literal follows a lone marker token.
    fn parse_tls_path_value(&mut self, marker: &str) -> Result<Expression, ParseError> {
        let Some(token) = self.cursor.peek() else {
            return Err(ParseError::from_span(
                format!("Expected '{marker}' followed by a file path, found end of input"),
                crate::diagnostics::Span { start: 0, end: 0 },
                0,
                0,
            ));
        };

        if let Token::Identifier(id) = &token.token {
            if id == marker {
                self.bump_sync(); // Consume the marker
                // Primary expression only, so a following "and"/"as" clause
                // stays available.
                self.parse_primary_expression()
            } else if let Some(rest) = id.strip_prefix(&format!("{marker} ")) {
                let rest = rest.to_string();
                let (line, column) = (token.line, token.column);
                self.bump_sync(); // Consume the merged marker + variable
                Ok(Expression::Variable(rest, line, column))
            } else {
                Err(ParseError::from_token(
                    format!("Expected '{marker}' before file path, found '{id}'"),
                    token,
                ))
            }
        } else {
            Err(ParseError::from_token(
                format!(
                    "Expected '{marker}' before file path, found {:?}",
                    token.token
                ),
                token,
            ))
        }
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

        // Check for optional "and" clauses (status, content_type, and/or headers)
        loop {
            if let Some(token) = self.cursor.peek()
                && token.token == Token::KeywordAnd
            {
                // Look ahead to see what comes after "and"
                if let Some(next_token) = self.cursor.peek_next() {
                    if next_token.token == Token::KeywordStatus {
                        self.bump_sync(); // Consume "and"
                        self.bump_sync(); // Consume "status"
                        // Primary expression only: a full expression would
                        // swallow a following "and content_type ..." clause
                        // as a boolean operation.
                        status = Some(self.parse_primary_expression()?);
                        continue;
                    } else if let Token::Identifier(id) = &next_token.token
                        && (id == "content_type"
                            || id == "content"
                            || id.starts_with("content_type ")
                            || id.starts_with("content type"))
                    {
                        // The lexer merges adjacent identifiers into one token,
                        // so a variable value can arrive glued to the marker
                        // (e.g. `and content_type my_type` lexes as
                        // Identifier("content_type my_type")). Split the marker
                        // off and treat the remainder as the value variable.
                        let id = id.clone();
                        let (id_line, id_column) = (next_token.line, next_token.column);
                        self.bump_sync(); // Consume "and"
                        self.bump_sync(); // Consume the (possibly merged) marker

                        let rest = if let Some(stripped) = id.strip_prefix("content_type") {
                            stripped.trim_start()
                        } else {
                            id.strip_prefix("content")
                                .map(|s| s.trim_start())
                                .map(|s| s.strip_prefix("type").map(str::trim_start).unwrap_or(s))
                                .unwrap_or("")
                        };

                        if rest.is_empty() {
                            // If it was a bare "content", expect "type" next
                            if id == "content"
                                && let Some(type_token) = self.cursor.peek()
                                && let Token::Identifier(type_id) = &type_token.token
                                && type_id == "type"
                            {
                                self.bump_sync(); // Consume "type"
                            }

                            // Primary expression only, so a following "and
                            // status ..." clause stays available.
                            content_type = Some(self.parse_primary_expression()?);
                        } else {
                            content_type =
                                Some(Expression::Variable(rest.to_string(), id_line, id_column));
                        }
                        continue;
                    } else if let Token::Identifier(id) = &next_token.token
                        && (id == "headers" || id.starts_with("headers "))
                    {
                        // Mirrors the outbound client's `with headers <map>` form.
                        // The lexer merges adjacent identifiers, so a variable
                        // value can arrive glued to the marker
                        // (`and headers h` lexes as Identifier("headers h")).
                        let id = id.clone();
                        let (id_line, id_column) = (next_token.line, next_token.column);
                        self.bump_sync(); // Consume "and"
                        self.bump_sync(); // Consume the (possibly merged) "headers" marker
                        let rest = id
                            .strip_prefix("headers")
                            .map(str::trim_start)
                            .unwrap_or("");
                        if rest.is_empty() {
                            // Primary expression only, so a following "and
                            // status ..." clause stays available.
                            headers = Some(self.parse_primary_expression()?);
                        } else {
                            headers =
                                Some(Expression::Variable(rest.to_string(), id_line, id_column));
                        }
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
