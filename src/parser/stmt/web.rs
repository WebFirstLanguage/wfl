//! Web server statement parsing

use super::super::{Expression, ParseError, Parser, Statement};
use super::StmtParser;
use crate::lexer::token::Token;
use crate::parser::ast::{Argument, TlsListenConfig, WsHandlerEvent};
use crate::parser::expr::{ExprParser, PrimaryExprParser};

pub(crate) trait WebParser<'a>: ExprParser<'a> + PrimaryExprParser<'a> {
    fn parse_listen_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_tls_path_value(&mut self, marker: &str) -> Result<Expression, ParseError>;
    fn parse_respond_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_start_streaming_response(&mut self) -> Result<Statement, ParseError>;
    fn parse_flush_stream(&mut self) -> Result<Statement, ParseError>;
    fn parse_register_signal_handler_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_stop_accepting_connections_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_close_server_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_websocket_handler(&mut self) -> Result<Statement, ParseError>;
    fn parse_send_websocket_message(&mut self) -> Result<Statement, ParseError>;
    fn parse_broadcast_websocket_message(&mut self) -> Result<Statement, ParseError>;
    fn parse_ws_message_operand(
        &mut self,
        rest: &str,
        line: usize,
        column: usize,
    ) -> Result<Expression, ParseError>;
}

impl<'a> WebParser<'a> for Parser<'a> {
    fn parse_listen_statement(&mut self) -> Result<Statement, ParseError> {
        let listen_token = self.bump_sync().unwrap(); // Consume "listen"

        // `listen for websockets on port <expr> as <name>` — WebSocket listener.
        // The `websockets` marker lexes as a plain identifier (keywords `on`/`port`
        // flush it), so detect it before the HTTP `listen on port` path.
        if let Some(token) = self.cursor.peek()
            && token.token == Token::KeywordFor
        {
            self.bump_sync(); // Consume "for"

            match self.cursor.peek() {
                Some(t) => match &t.token {
                    Token::Identifier(id) if id == "websockets" || id == "websocket" => {
                        self.bump_sync(); // Consume "websockets"
                    }
                    _ => {
                        return Err(ParseError::from_token(
                            "Expected 'websockets' after 'listen for'".to_string(),
                            t,
                        ));
                    }
                },
                None => {
                    return Err(ParseError::from_token(
                        "Expected 'websockets' after 'listen for'".to_string(),
                        listen_token,
                    ));
                }
            }

            self.expect_token(
                Token::KeywordOn,
                "Expected 'on' after 'listen for websockets'",
            )?;
            self.expect_token(
                Token::KeywordPort,
                "Expected 'port' after 'listen for websockets on'",
            )?;
            let port = self.parse_expression()?;
            self.expect_token(Token::KeywordAs, "Expected 'as' after websocket port")?;
            let server_name = self.parse_variable_name_simple()?;

            return Ok(Statement::ListenWebSocketStatement {
                port,
                server_name,
                line: listen_token.line,
                column: listen_token.column,
            });
        }

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

    fn parse_start_streaming_response(&mut self) -> Result<Statement, ParseError> {
        // Consume the `start` keyword, then the `streaming` contextual identifier.
        let start_token = self.bump_sync().unwrap();
        let (line, column) = (start_token.line, start_token.column);

        match self.cursor.peek() {
            Some(t) => match &t.token {
                Token::Identifier(id) if id == "streaming" => {
                    self.bump_sync(); // Consume "streaming"
                }
                _ => {
                    return Err(ParseError::from_token(
                        "Expected 'streaming' after 'start'".to_string(),
                        t,
                    ));
                }
            },
            None => {
                return Err(ParseError::from_token(
                    "Expected 'streaming response' after 'start'".to_string(),
                    start_token,
                ));
            }
        }

        self.expect_token(
            Token::KeywordResponse,
            "Expected 'response' after 'start streaming'",
        )?;
        self.expect_token(
            Token::KeywordTo,
            "Expected 'to <request>' after 'start streaming response'",
        )?;
        let request = self.parse_primary_expression()?;

        let mut status = None;
        let mut content_type = None;
        let mut headers = None;

        // Optional clauses joined by `with`/`and`, in any order: `status <e>`,
        // `content type <e>`, `headers <e>`. Mirrors the `respond` clause loop.
        // A connective directly before `as` is the end-of-clauses join and is
        // consumed so the `as <name>` binding parses; a connective before any
        // other unrecognized token ends the loop WITHOUT being consumed, so the
        // trailing `expect_token(as)` reports the malformed clause.
        loop {
            let connective = matches!(
                self.cursor.peek(),
                Some(t) if t.token == Token::KeywordWith || t.token == Token::KeywordAnd
            );
            if !connective {
                break;
            }
            let Some(next_token) = self.cursor.peek_next() else {
                break;
            };

            match &next_token.token {
                Token::KeywordStatus => {
                    self.bump_sync(); // with/and
                    self.bump_sync(); // status
                    status = Some(self.parse_primary_expression()?);
                }
                // `content type <e>` — `content` keyword then optional `type`.
                // When `<e>` is a bare identifier the lexer merges it into the
                // `type` token (`type ct` -> Identifier("type ct")), so split the
                // value off rather than binding the whole thing as the variable.
                Token::KeywordContent => {
                    self.bump_sync(); // with/and
                    self.bump_sync(); // content
                    let merged_rest = if let Some(t) = self.cursor.peek()
                        && let Token::Identifier(id) = &t.token
                        && (id == "type" || id.starts_with("type "))
                    {
                        let id = id.clone();
                        let pos = (t.line, t.column);
                        self.bump_sync(); // (possibly merged) type
                        let rest = id.strip_prefix("type").map(str::trim_start).unwrap_or("");
                        (!rest.is_empty()).then(|| (rest.to_string(), pos))
                    } else {
                        None
                    };
                    content_type = Some(match merged_rest {
                        Some((rest, (l, c))) => {
                            // Compose any dangling postfix accessors
                            // (`content type upstream.headers["content-type"]`).
                            let lead = Expression::Variable(rest, l, c);
                            self.parse_trailing_postfix(lead)?
                        }
                        None => self.parse_primary_expression()?,
                    });
                }
                // Merged `content_type <var>` / `content type <var>` form.
                Token::Identifier(id)
                    if id == "content_type"
                        || id.starts_with("content_type ")
                        || id.starts_with("content type") =>
                {
                    let id = id.clone();
                    let (id_line, id_column) = (next_token.line, next_token.column);
                    self.bump_sync(); // with/and
                    self.bump_sync(); // merged marker
                    let rest = id
                        .strip_prefix("content_type")
                        .map(str::trim_start)
                        .unwrap_or_else(|| {
                            id.strip_prefix("content type")
                                .map(str::trim_start)
                                .unwrap_or("")
                        });
                    if rest.is_empty() {
                        content_type = Some(self.parse_primary_expression()?);
                    } else {
                        let lead = Expression::Variable(rest.to_string(), id_line, id_column);
                        content_type = Some(self.parse_trailing_postfix(lead)?);
                    }
                }
                // `headers <map>` (bare or merged `headers <var>`).
                Token::Identifier(id) if id == "headers" || id.starts_with("headers ") => {
                    let id = id.clone();
                    let (id_line, id_column) = (next_token.line, next_token.column);
                    self.bump_sync(); // with/and
                    self.bump_sync(); // merged marker
                    let rest = id
                        .strip_prefix("headers")
                        .map(str::trim_start)
                        .unwrap_or("");
                    if rest.is_empty() {
                        headers = Some(self.parse_primary_expression()?);
                    } else {
                        // Compose any dangling postfix accessors so direct
                        // forwarding like `headers upstream.headers` binds fully.
                        let lead = Expression::Variable(rest.to_string(), id_line, id_column);
                        headers = Some(self.parse_trailing_postfix(lead)?);
                    }
                }
                // A connective directly before `as` just joins the clause list to
                // the binding; consume it so `as <name>` parses cleanly instead of
                // `expect_token(as)` tripping over the leftover `and`/`with`.
                Token::KeywordAs => {
                    self.bump_sync(); // consume the connective; `as` stays next
                    break;
                }
                _ => break,
            }
        }

        self.expect_token(
            Token::KeywordAs,
            "Expected 'as <name>' after 'start streaming response ...'",
        )?;
        let variable_name = self.parse_variable_name_simple()?;

        Ok(Statement::StartStreamingResponseStatement {
            request,
            status,
            content_type,
            headers,
            variable_name,
            line,
            column,
        })
    }

    fn parse_flush_stream(&mut self) -> Result<Statement, ParseError> {
        // `flush <out>` — the lexer merges a bare-identifier target into the
        // command token (`flush out` -> Identifier("flush out")).
        let token = self.bump_sync().unwrap();
        let (line, column) = (token.line, token.column);
        let phrase = match &token.token {
            Token::Identifier(id) => id.clone(),
            _ => {
                return Err(ParseError::from_token(
                    "Expected 'flush <stream>'".to_string(),
                    token,
                ));
            }
        };
        let rest = phrase
            .strip_prefix("flush")
            .map(str::trim_start)
            .unwrap_or("");
        let (target, action_fallback) = if rest.is_empty() {
            (self.parse_primary_expression()?, None)
        } else {
            // The lexer merged `flush` with the operand identifier, so any postfix
            // accessors (`flush streams["a"]`, `flush obj.out`) are left as separate
            // tokens. Compose them onto the split-off lead so the operand parses
            // consistently with a normal expression instead of dangling.
            let lead = Expression::Variable(rest.to_string(), line, column);
            let composed = self.parse_trailing_postfix(lead)?;
            // Backward compatibility: `flush <ident>` used to auto-invoke a
            // zero-argument action named "flush <ident>". Carry the full phrase so
            // the interpreter can prefer that action when it exists. Only a
            // bare-identifier operand can collide with such an action name; a
            // postfix operand (`flush obj.out`) cannot, so it carries no fallback
            // (else a defined `flush obj` action would wrongly swallow `.out`).
            let fallback = if matches!(composed, Expression::Variable(..)) {
                Some(phrase.clone())
            } else {
                None
            };
            (composed, fallback)
        };

        Ok(Statement::FlushStreamStatement {
            target,
            action_fallback,
            line,
            column,
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

    /// Parses a WebSocket event handler block:
    /// `on websocket connect|message|disconnect to|from <server> as <binding>: ... end on`.
    ///
    /// The lexer merges the two identifiers after `on` into a single token
    /// (`websocket connect`), so the event kind is read off that merged phrase.
    /// `connect` reads naturally with `to`; `message`/`disconnect` with `from` —
    /// both connective words are accepted for any event so phrasing stays free.
    fn parse_websocket_handler(&mut self) -> Result<Statement, ParseError> {
        let on_token = self.bump_sync().unwrap(); // Consume "on"

        let phrase = match self.cursor.peek() {
            Some(t) => match &t.token {
                Token::Identifier(id) => id.clone(),
                _ => {
                    return Err(ParseError::from_token(
                        "Expected 'websocket <event>' after 'on'".to_string(),
                        t,
                    ));
                }
            },
            None => {
                return Err(ParseError::from_token(
                    "Expected 'websocket <event>' after 'on'".to_string(),
                    on_token,
                ));
            }
        };

        let event = match phrase.as_str() {
            "websocket connect" => WsHandlerEvent::Connect,
            "websocket message" => WsHandlerEvent::Message,
            "websocket disconnect" => WsHandlerEvent::Disconnect,
            other => {
                let t = self.cursor.peek().unwrap();
                return Err(ParseError::from_token(
                    format!(
                        "Unknown websocket event '{other}'. Expected 'websocket connect', 'websocket message', or 'websocket disconnect'"
                    ),
                    t,
                ));
            }
        };
        self.bump_sync(); // Consume the event phrase identifier

        // Accept `to` or `from` interchangeably as the connective word.
        match self.cursor.peek() {
            Some(t) if t.token == Token::KeywordTo || t.token == Token::KeywordFrom => {
                self.bump_sync();
            }
            Some(t) => {
                return Err(ParseError::from_token(
                    "Expected 'to' or 'from' after the websocket event".to_string(),
                    t,
                ));
            }
            None => {
                return Err(ParseError::from_token(
                    "Expected 'to' or 'from' after the websocket event".to_string(),
                    on_token,
                ));
            }
        }

        // Primary expression only, so the following `as` stays available.
        let server = self.parse_primary_expression()?;

        self.expect_token(
            Token::KeywordAs,
            "Expected 'as <name>' after the websocket server",
        )?;
        let binding = self.parse_variable_name_simple()?;
        self.expect_token(
            Token::Colon,
            "Expected ':' after the websocket handler binding",
        )?;

        // Parse the handler body until `end on`.
        self.skip_eol();
        let mut body = Vec::new();
        loop {
            self.skip_eol();
            match self.cursor.peek() {
                Some(t) if t.token == Token::KeywordEnd => break,
                None => {
                    return Err(ParseError::from_token(
                        "Expected 'end on' to close the websocket handler".to_string(),
                        on_token,
                    ));
                }
                _ => body.push(self.parse_statement()?),
            }
        }
        self.expect_token(
            Token::KeywordEnd,
            "Expected 'end' to close the websocket handler",
        )?;
        self.expect_token(
            Token::KeywordOn,
            "Expected 'on' after 'end' in the websocket handler",
        )?;

        Ok(Statement::WebSocketHandlerStatement {
            event,
            server,
            binding,
            body,
            line: on_token.line,
            column: on_token.column,
        })
    }

    /// Parses `send websocket message <message> to <connection>`.
    ///
    /// The lexer glues the command words — and a bare identifier message — into
    /// one token (`send websocket message reply`), so the message is split off
    /// the phrase prefix; a literal message follows as its own token instead.
    fn parse_send_websocket_message(&mut self) -> Result<Statement, ParseError> {
        let token = self.bump_sync().unwrap(); // Consume the merged command phrase
        let (line, column) = (token.line, token.column);
        let phrase = match &token.token {
            Token::Identifier(id) => id.clone(),
            _ => {
                return Err(ParseError::from_token(
                    "Expected 'send websocket message ...'".to_string(),
                    token,
                ));
            }
        };

        let rest = phrase
            .strip_prefix("send websocket message")
            .map(str::trim_start)
            .unwrap_or("");
        let message = self.parse_ws_message_operand(rest, line, column)?;

        self.expect_token(
            Token::KeywordTo,
            "Expected 'to <connection>' after the websocket message",
        )?;
        let target = self.parse_expression()?;

        Ok(Statement::SendWebSocketMessageStatement {
            message,
            target,
            line,
            column,
        })
    }

    /// Parses `broadcast websocket message <message> to <server>`.
    fn parse_broadcast_websocket_message(&mut self) -> Result<Statement, ParseError> {
        let token = self.bump_sync().unwrap(); // Consume the merged command phrase
        let (line, column) = (token.line, token.column);
        let phrase = match &token.token {
            Token::Identifier(id) => id.clone(),
            _ => {
                return Err(ParseError::from_token(
                    "Expected 'broadcast websocket message ...'".to_string(),
                    token,
                ));
            }
        };

        let rest = phrase
            .strip_prefix("broadcast websocket message")
            .map(str::trim_start)
            .unwrap_or("");
        let message = self.parse_ws_message_operand(rest, line, column)?;

        self.expect_token(
            Token::KeywordTo,
            "Expected 'to <server>' after the websocket message",
        )?;
        let server = self.parse_expression()?;

        Ok(Statement::BroadcastWebSocketMessageStatement {
            message,
            server,
            line,
            column,
        })
    }

    /// Parses the message operand of a `send`/`broadcast websocket message`
    /// statement. The lexer glues the command words — and a leading identifier
    /// message — into one token, so `rest` is whatever trailed the command
    /// prefix. Handles the natural forms:
    ///
    ///   - empty `rest`: a literal/number-led expression (`"Echo: " with body of msg`)
    ///   - `<name> of <object>`: a property read (`body of msg`)
    ///   - a bare (possibly multi-word) variable (`reply`, `my reply`)
    ///
    /// A more complex inline expression starting with an identifier must be
    /// stored in a variable first, which the error message explains.
    fn parse_ws_message_operand(
        &mut self,
        rest: &str,
        line: usize,
        column: usize,
    ) -> Result<Expression, ParseError> {
        if rest.is_empty() {
            // The message begins with a non-identifier token (string/number), so
            // the whole expression — including any `with`-concatenation — parses
            // cleanly from here.
            return self.parse_expression();
        }

        let left = Expression::Variable(rest.to_string(), line, column);
        match self.cursor.peek().map(|t| &t.token) {
            // `<field> of <object>`, e.g. `body of msg`. Built as an `of`-call so
            // it resolves through the interpreter's property-of-object access.
            Some(Token::KeywordOf) => {
                self.bump_sync(); // Consume "of"
                let object = self.parse_primary_expression()?;
                Ok(Expression::FunctionCall {
                    function: Box::new(left),
                    arguments: vec![Argument {
                        name: None,
                        value: object,
                    }],
                    line,
                    column,
                })
            }
            // A bare variable message: the next token starts the `to ...` clause.
            Some(Token::KeywordTo) => Ok(left),
            _ => Err(ParseError::from_span(
                "A computed websocket message must be stored in a variable first, e.g. `store reply as \"Echo: \" with body of msg` then `send websocket message reply to ...`.".to_string(),
                self.cursor.current_span(),
                self.cursor.current_line(),
                column,
            )),
        }
    }
}
