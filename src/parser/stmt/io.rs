//! File I/O and filesystem statement parsing

use super::super::{Expression, FileOpenMode, Literal, ParseError, Parser, Statement};
use super::database::DatabaseParser;
use crate::lexer::token::Token;
use crate::parser::expr::{BinaryExprParser, ExprParser, PrimaryExprParser};
use std::sync::Arc;

impl<'a> Parser<'a> {
    /// Parse a `write line|chunk` value from an already-chosen leading operand:
    /// an optional `<field> of <object>` postfix, then any `with`/operator
    /// continuation, exactly as a normal expression value would parse.
    ///
    /// The ambiguous merged `write line|chunk <ident> ...` form has two readings
    /// (stream: split-off `<ident>`; classic file write: whole `line <ident>`)
    /// that differ only in the leading operand. They are parsed independently —
    /// same tokens, via a cursor rewind between the two calls — because a
    /// continuation can desugar differently per operand (a builtin name becomes
    /// an `ActionCall`, `is between` duplicates the left, `starts/ends with` and
    /// the pattern operators build calls), so deriving one AST from the other by
    /// leaf-swapping silently corrupted the classic reading.
    fn parse_write_value_from_lead(&mut self, lead: Expression) -> Result<Expression, ParseError> {
        // The lexer merges `write` with the operand identifier and leaves any
        // bracket-index / dotted-property accessors as following tokens, so compose
        // them onto the lead (`write line chunks[0] to out`,
        // `write line upstream.status to out`, classic `write line values[0] to
        // "/tmp/out"`) instead of leaving them to dangle after the statement.
        let lead = self.parse_trailing_postfix(lead)?;
        let lead = if matches!(self.cursor.peek().map(|t| &t.token), Some(Token::KeywordOf)) {
            // Anchor the `<field> of <object>` call to the `of` keyword itself,
            // matching how the rest of the parser positions FunctionCall nodes so
            // error spans point at the operator, not the lead (review feedback).
            let (of_line, of_column) = self
                .bump_sync()
                .map(|t| (t.line, t.column))
                .expect("peeked `of` immediately above");
            let object = self.parse_primary_expression()?;
            Expression::FunctionCall {
                function: Box::new(lead),
                arguments: vec![crate::parser::ast::Argument {
                    name: None,
                    value: object,
                }],
                line: of_line,
                column: of_column,
            }
        } else {
            lead
        };
        self.parse_binary_continuation(lead, 0)
    }
}

pub(crate) trait IoParser<'a>: ExprParser<'a> {
    fn parse_display_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_open_file_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_open_url_statement(
        &mut self,
        open_token: &'a crate::lexer::token::TokenWithPosition,
    ) -> Result<Statement, ParseError>;
    fn parse_http_value_expression(&mut self) -> Result<Expression, ParseError>;
    fn parse_http_clause_value(
        &mut self,
        merged: &str,
        clause: &str,
        line: usize,
        column: usize,
    ) -> Result<Expression, ParseError>;
    fn continue_http_value_expression(
        &mut self,
        expr: Expression,
    ) -> Result<Expression, ParseError>;
    #[allow(dead_code)]
    fn parse_open_file_read_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_close_file_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_write_to_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_create_file_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_create_directory_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_delete_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_path_expression(&mut self) -> Result<Expression, ParseError>;
}

impl<'a> IoParser<'a> for Parser<'a> {
    /// Parses a file path expression: a primary expression optionally
    /// concatenated with further primaries via `with`, e.g.
    /// `open file at base_dir with "/index.html" for reading as f`.
    /// Keywords like `for`, `and`, and `as` still terminate the path.
    fn parse_path_expression(&mut self) -> Result<Expression, ParseError> {
        let mut path = self.parse_primary_expression()?;

        while let Some(token) = self.cursor.peek() {
            if token.token != Token::KeywordWith {
                break;
            }
            let (line, column) = (token.line, token.column);
            self.bump_sync(); // Consume "with"
            let right = self.parse_primary_expression()?;
            path = Expression::Concatenation {
                left: Box::new(path),
                right: Box::new(right),
                line,
                column,
            };
        }

        Ok(path)
    }

    /// Parses the remainder of `open url at <url> ...` after `url` was consumed.
    ///
    /// Grammar (clauses optional, joined by `and`/`with`, any order):
    ///   open url at <url>
    ///       [with method <expr>] [and headers <expr>] [and body <expr>]
    ///       ( and read content as <name>   -- binds the response body text
    ///       | and read response as <name>  -- binds status/ok/body/headers object
    ///       | as <name> )                  -- binds the response body text
    ///
    /// Plain GET forms keep producing `HttpGetStatement` for backward
    /// compatibility; anything using method/headers/body or `read response`
    /// produces `HttpRequestStatement`.
    fn parse_open_url_statement(
        &mut self,
        open_token: &'a crate::lexer::token::TokenWithPosition,
    ) -> Result<Statement, ParseError> {
        if !matches!(self.cursor.peek(), Some(t) if t.token == Token::KeywordAt) {
            return Err(ParseError::from_token(
                "Expected 'at' after 'url'".to_string(),
                open_token,
            ));
        }
        self.bump_sync(); // Consume "at"

        let url_expr = self.parse_primary_expression()?;

        let mut method: Option<Expression> = None;
        let mut headers: Option<Expression> = None;
        let mut body: Option<Expression> = None;

        let parse_variable_name =
            |parser: &mut Self, open_token: &'a crate::lexer::token::TokenWithPosition| {
                if let Some(token) = parser.cursor.peek() {
                    if let Token::Identifier(name) = &token.token {
                        parser.bump_sync(); // Consume the identifier
                        Ok(name.clone())
                    } else if token.token == Token::KeywordContent {
                        // Special case for "content" as a variable name
                        parser.bump_sync();
                        Ok("content".to_string())
                    } else {
                        Err(ParseError::from_token(
                            format!(
                                "Expected identifier for variable name, found {:?}",
                                token.token
                            ),
                            token,
                        ))
                    }
                } else {
                    Err(ParseError::from_token(
                        "Unexpected end of input".to_string(),
                        open_token,
                    ))
                }
            };

        loop {
            // Long requests may continue clauses on the following lines:
            //   open url at "https://..."
            //       with method "POST"
            //       and read response as resp
            // Skip line breaks only when a connector follows, so a statement
            // that simply ends without its terminator still errors here.
            if matches!(self.cursor.peek(), Some(t) if t.token == Token::Eol) {
                let mut lookahead = 1;
                while matches!(self.cursor.peek_kind_n(lookahead), Some(Token::Eol)) {
                    lookahead += 1;
                }
                if matches!(
                    self.cursor.peek_kind_n(lookahead),
                    Some(Token::KeywordAnd) | Some(Token::KeywordWith)
                ) {
                    for _ in 0..lookahead {
                        self.bump_sync(); // Consume the line breaks
                    }
                }
            }

            let Some(next_token) = self.cursor.peek() else {
                return Err(ParseError::from_token(
                    "Unexpected end of input after URL: expected 'and read content as <name>', 'and read response as <name>', or 'as <name>'"
                        .to_string(),
                    open_token,
                ));
            };

            match &next_token.token {
                Token::KeywordAs => {
                    // "open url at <url> ... as <name>" binds the body text
                    self.bump_sync(); // Consume "as"
                    let variable_name = parse_variable_name(self, open_token)?;

                    return Ok(if method.is_some() || headers.is_some() || body.is_some() {
                        Statement::HttpRequestStatement {
                            url: url_expr,
                            method,
                            headers,
                            body,
                            variable_name,
                            full_response: false,
                            line: open_token.line,
                            column: open_token.column,
                        }
                    } else {
                        Statement::HttpGetStatement {
                            url: url_expr,
                            variable_name,
                            line: open_token.line,
                            column: open_token.column,
                        }
                    });
                }
                Token::KeywordAnd | Token::KeywordWith => {
                    self.bump_sync(); // Consume the connector
                }
                _ => {
                    return Err(ParseError::from_token(
                        format!(
                            "Expected 'and', 'with', or 'as' after URL, found {:?}",
                            next_token.token
                        ),
                        next_token,
                    ));
                }
            }

            // A connector was consumed; dispatch on the clause keyword
            let Some(clause_token) = self.cursor.peek() else {
                return Err(ParseError::from_token(
                    "Unexpected end of input: expected 'method', 'headers', 'body', or 'read' clause"
                        .to_string(),
                    open_token,
                ));
            };

            match &clause_token.token {
                Token::KeywordRead => {
                    self.bump_sync(); // Consume "read"

                    let full_response = if let Some(token) = self.cursor.peek() {
                        match &token.token {
                            Token::KeywordContent => {
                                self.bump_sync(); // Consume "content"
                                false
                            }
                            Token::KeywordResponse => {
                                self.bump_sync(); // Consume "response"
                                true
                            }
                            _ => {
                                return Err(ParseError::from_token(
                                    format!(
                                        "Expected 'content' or 'response' after 'read', found {:?}",
                                        token.token
                                    ),
                                    token,
                                ));
                            }
                        }
                    } else {
                        return Err(ParseError::from_token(
                            "Unexpected end of input after 'read'".to_string(),
                            open_token,
                        ));
                    };

                    self.expect_token(
                        Token::KeywordAs,
                        "Expected 'as' after 'content' or 'response'",
                    )?;
                    let variable_name = parse_variable_name(self, open_token)?;

                    let is_plain_get = method.is_none() && headers.is_none() && body.is_none();
                    return Ok(if is_plain_get && !full_response {
                        Statement::HttpGetStatement {
                            url: url_expr,
                            variable_name,
                            line: open_token.line,
                            column: open_token.column,
                        }
                    } else {
                        Statement::HttpRequestStatement {
                            url: url_expr,
                            method,
                            headers,
                            body,
                            variable_name,
                            full_response,
                            line: open_token.line,
                            column: open_token.column,
                        }
                    });
                }
                // `stream response as <name>` — return the status/headers
                // immediately and bind a streaming handle instead of buffering
                // the body. `stream` is a contextual identifier (not a
                // keyword), so match it as one; `response` is a keyword.
                Token::Identifier(name) if name == "stream" => {
                    self.bump_sync(); // Consume "stream"
                    self.expect_token(
                        Token::KeywordResponse,
                        "Expected 'response' after 'stream'",
                    )?;
                    self.expect_token(Token::KeywordAs, "Expected 'as' after 'stream response'")?;
                    let variable_name = parse_variable_name(self, open_token)?;
                    return Ok(Statement::HttpStreamStatement {
                        url: url_expr,
                        method,
                        headers,
                        body,
                        variable_name,
                        line: open_token.line,
                        column: open_token.column,
                    });
                }
                // The lexer merges consecutive identifiers into multi-word
                // names, so `headers auth_headers` arrives as the single
                // token Identifier("headers auth_headers"). Match both the
                // bare clause keyword and the merged form.
                Token::Identifier(name) if name == "method" || name.starts_with("method ") => {
                    if method.is_some() {
                        return Err(ParseError::from_token(
                            "Duplicate 'method' clause in open url statement".to_string(),
                            clause_token,
                        ));
                    }
                    let merged = name.clone();
                    let (clause_line, clause_column) = (clause_token.line, clause_token.column);
                    self.bump_sync(); // Consume "method" (or the merged identifier)
                    method = Some(self.parse_http_clause_value(
                        &merged,
                        "method",
                        clause_line,
                        clause_column,
                    )?);
                }
                Token::Identifier(name) if name == "headers" || name.starts_with("headers ") => {
                    if headers.is_some() {
                        return Err(ParseError::from_token(
                            "Duplicate 'headers' clause in open url statement".to_string(),
                            clause_token,
                        ));
                    }
                    let merged = name.clone();
                    let (clause_line, clause_column) = (clause_token.line, clause_token.column);
                    self.bump_sync(); // Consume "headers" (or the merged identifier)
                    headers = Some(self.parse_http_clause_value(
                        &merged,
                        "headers",
                        clause_line,
                        clause_column,
                    )?);
                }
                Token::Identifier(name) if name == "body" || name.starts_with("body ") => {
                    if body.is_some() {
                        return Err(ParseError::from_token(
                            "Duplicate 'body' clause in open url statement".to_string(),
                            clause_token,
                        ));
                    }
                    let merged = name.clone();
                    let (clause_line, clause_column) = (clause_token.line, clause_token.column);
                    self.bump_sync(); // Consume "body" (or the merged identifier)
                    body = Some(self.parse_http_clause_value(
                        &merged,
                        "body",
                        clause_line,
                        clause_column,
                    )?);
                }
                _ => {
                    return Err(ParseError::from_token(
                        format!(
                            "Expected 'method', 'headers', 'body', or 'read' after 'and'/'with', found {:?}",
                            clause_token.token
                        ),
                        clause_token,
                    ));
                }
            }
        }
    }

    /// Parses a clause value in an `open url` statement: a primary expression
    /// optionally concatenated with further primaries via `with`. A `with`
    /// that introduces the next request clause (method/headers/body/read)
    /// terminates the value instead of concatenating.
    fn parse_http_value_expression(&mut self) -> Result<Expression, ParseError> {
        let expr = self.parse_primary_expression()?;
        self.continue_http_value_expression(expr)
    }

    /// Parses the value of a `method`/`headers`/`body` clause when the clause
    /// keyword has already been consumed. `merged` is the identifier token
    /// text: if the lexer merged the clause keyword with a following variable
    /// name (`headers auth_headers`), the remainder is the variable reference.
    fn parse_http_clause_value(
        &mut self,
        merged: &str,
        clause: &str,
        line: usize,
        column: usize,
    ) -> Result<Expression, ParseError> {
        if merged.len() > clause.len() {
            let rest = merged[clause.len() + 1..].to_string();
            let base = Expression::Variable(rest, line, column);
            self.continue_http_value_expression(base)
        } else {
            self.parse_http_value_expression()
        }
    }

    /// Continues an already-parsed clause value with `with`-concatenations,
    /// stopping before a `with` that introduces the next request clause.
    fn continue_http_value_expression(
        &mut self,
        mut expr: Expression,
    ) -> Result<Expression, ParseError> {
        while let Some(token) = self.cursor.peek() {
            if token.token != Token::KeywordWith {
                break;
            }
            // Lookahead: stop if this 'with' starts the next request clause
            match self.cursor.peek_kind_n(1) {
                Some(Token::KeywordRead) => break,
                Some(Token::Identifier(name))
                    if name == "method"
                        || name.starts_with("method ")
                        || name == "headers"
                        || name.starts_with("headers ")
                        || name == "body"
                        || name.starts_with("body ") =>
                {
                    break;
                }
                _ => {}
            }
            let (line, column) = (token.line, token.column);
            self.bump_sync(); // Consume "with"
            let right = self.parse_primary_expression()?;
            expr = Expression::Concatenation {
                left: Box::new(expr),
                right: Box::new(right),
                line,
                column,
            };
        }

        Ok(expr)
    }

    fn parse_display_statement(&mut self) -> Result<Statement, ParseError> {
        // Anchor the statement at the `display` keyword itself. This parser is
        // only dispatched on `Token::KeywordDisplay`, so `bump_sync` is always
        // `Some` here — expect rather than defaulting to a misleading (0, 0).
        let display_token = self
            .bump_sync() // Consume "display"
            .expect("parse_display_statement is only called on the `display` keyword");
        let (line, column) = (display_token.line, display_token.column);

        // Parse the first value.
        let mut values = vec![self.parse_expression()?];
        let mut join_positions = Vec::new();

        // `display` accepts more than one space-separated value: quoted text is
        // a string literal and anything else is a variable/expression. Collect
        // every additional value here — only tokens that begin a fresh value
        // continue the loop (see `is_value_start`), and only when they aren't
        // actually the start of a same-line statement in disguise (see
        // `is_display_fold_statement_boundary`, which keeps `count from ...`
        // and `read output from process ...` as their own statement). Direct
        // index access such as `display numbers 0` is already absorbed by
        // `parse_expression` above — the trailing `0` never reaches this loop
        // — and a line break ends the statement because `Eol` is not a value
        // start, so both keep working unchanged.
        loop {
            let is_boundary = self.is_display_fold_statement_boundary();
            let (cat_line, cat_column) = match self.cursor.peek() {
                Some(token) if !is_boundary && Self::is_value_start(&token.token) => {
                    (token.line, token.column)
                }
                _ => break,
            };

            values.push(self.parse_expression()?);
            join_positions.push((cat_line, cat_column));
        }

        // Fold right-associatively — the same tree shape, evaluation order, and
        // stringification order as explicit `with` (`a with b with c` parses as
        // `a with (b with c)`, see the `with` handling in expr/binary.rs). This
        // matters beyond cosmetics: `Concatenation` evaluates left, then right,
        // then stringifies both, so a left-associative fold would stringify an
        // earlier value *before* a later value's side effects (e.g. a list
        // mutation) run, while this matches `with` exactly, value for value.
        let mut value = values.pop().expect("at least one value was parsed");
        while let Some(left) = values.pop() {
            let (cat_line, cat_column) = join_positions
                .pop()
                .expect("one join position per folded value");
            value = Expression::Concatenation {
                left: Box::new(left),
                right: Box::new(value),
                line: cat_line,
                column: cat_column,
            };
        }

        Ok(Statement::DisplayStatement {
            value,
            line,
            column,
        })
    }

    fn parse_open_file_statement(&mut self) -> Result<Statement, ParseError> {
        let open_token = self.bump_sync().unwrap(); // Consume "open"

        // Check if the next token is "file", "url", or "database"
        if let Some(next_token) = self.cursor.peek() {
            match next_token.token {
                Token::KeywordFile => {
                    // Existing file handling
                    self.bump_sync(); // Consume "file"
                }
                Token::KeywordDatabase => {
                    // "open database at <url> as <name>"
                    return self.parse_open_database_statement(open_token.line, open_token.column);
                }
                Token::KeywordUrl => {
                    // New URL handling
                    self.bump_sync(); // Consume "url"
                    return self.parse_open_url_statement(open_token);
                }
                _ => {
                    return Err(ParseError::from_token(
                        format!(
                            "Expected 'file', 'url', or 'database' after 'open', found {:?}",
                            next_token.token
                        ),
                        next_token,
                    ));
                }
            }
        } else {
            return Err(ParseError::from_token(
                "Unexpected end of input after 'open'".to_string(),
                open_token,
            ));
        }

        if let Some(token) = self.cursor.peek()
            && token.token == Token::KeywordAt
        {
            self.bump_sync(); // Consume "at"

            let path_expr = self.parse_path_expression()?;

            // Check for "for append", "and read content as" pattern AND direct "as" pattern
            if let Some(next_token) = self.cursor.peek() {
                if next_token.token == Token::KeywordFor {
                    // Check for "for [mode] as" pattern where mode can be append, reading, or writing
                    self.bump_sync(); // Consume "for"

                    let mode = if let Some(token) = self.cursor.peek() {
                        match &token.token {
                            Token::KeywordAppend => {
                                self.bump_sync(); // Consume "append"
                                FileOpenMode::Append
                            }
                            Token::KeywordAppending => {
                                self.bump_sync(); // Consume "appending"
                                FileOpenMode::Append
                            }
                            Token::Identifier(mode_str) if mode_str == "reading" => {
                                self.bump_sync(); // Consume "reading"
                                // Check for optional "binary" keyword
                                if let Some(next) = self.cursor.peek()
                                    && next.token == Token::KeywordBinary
                                {
                                    self.bump_sync(); // Consume "binary"
                                    FileOpenMode::ReadBinary
                                } else {
                                    FileOpenMode::Read
                                }
                            }
                            Token::Identifier(mode_str) if mode_str == "writing" => {
                                self.bump_sync(); // Consume "writing"
                                // Check for optional "binary" keyword
                                if let Some(next) = self.cursor.peek()
                                    && next.token == Token::KeywordBinary
                                {
                                    self.bump_sync(); // Consume "binary"
                                    FileOpenMode::WriteBinary
                                } else {
                                    FileOpenMode::Write
                                }
                            }
                            _ => {
                                return Err(ParseError::from_token(
                                    "Expected 'append', 'appending', 'reading', or 'writing' after 'for'"
                                        .to_string(),
                                    token,
                                ));
                            }
                        }
                    } else {
                        return Err(ParseError::from_token(
                            "Expected mode after 'for'".to_string(),
                            next_token,
                        ));
                    };

                    self.expect_token(Token::KeywordAs, "Expected 'as' after file mode")?;

                    let variable_name = if let Some(token) = self.cursor.peek() {
                        if let Token::Identifier(name) = &token.token {
                            self.bump_sync(); // Consume the identifier
                            name.clone()
                        } else {
                            return Err(ParseError::from_token(
                                format!("Expected identifier after 'as', found {:?}", token.token),
                                token,
                            ));
                        }
                    } else {
                        return Err(ParseError::from_token(
                            "Unexpected end of input after 'as'".to_string(),
                            open_token,
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

                    let variable_name = if let Some(token) = self.cursor.peek() {
                        if let Token::Identifier(name) = &token.token {
                            self.bump_sync(); // Consume the identifier
                            name.clone()
                        } else if let Token::KeywordContent = &token.token {
                            // Special case for "content" as an identifier
                            self.bump_sync(); // Consume the "content" keyword
                            "content".to_string()
                        } else {
                            return Err(ParseError::from_token(
                                format!(
                                    "Expected identifier for variable name, found {:?}",
                                    token.token
                                ),
                                token,
                            ));
                        }
                    } else {
                        return Err(self.cursor.error("Unexpected end of input".to_string()));
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

                    let variable_name = if let Some(token) = self.cursor.peek() {
                        if let Token::Identifier(id) = &token.token {
                            self.bump_sync();
                            id.clone()
                        } else {
                            return Err(ParseError::from_token(
                                format!("Expected identifier after 'as', found {:?}", token.token),
                                token,
                            ));
                        }
                    } else {
                        return Err(ParseError::from_token(
                            "Unexpected end of input after 'as'".to_string(),
                            open_token,
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
                    return Err(ParseError::from_token(
                        format!(
                            "Expected 'and' or 'as' after file path, found {:?}",
                            next_token.token
                        ),
                        next_token,
                    ));
                }
            } else {
                return Err(ParseError::from_token(
                    "Unexpected end of input after file path".to_string(),
                    open_token,
                ));
            }
        }

        let path = self.parse_path_expression()?;

        self.expect_token(Token::KeywordAs, "Expected 'as' after file path")?;

        let variable_name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(id) = &token.token {
                self.bump_sync();
                id.clone()
            } else {
                return Err(ParseError::from_token(
                    format!("Expected identifier after 'as', found {:?}", token.token),
                    token,
                ));
            }
        } else {
            return Err(ParseError::from_token(
                "Unexpected end of input after 'as'".to_string(),
                open_token,
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
                Expression::Literal(Literal::String(Arc::from(path)), line, column)
            } else {
                return Err(ParseError::from_token(
                    format!(
                        "Expected string literal for file path, found {:?}",
                        token.token
                    ),
                    token,
                ));
            }
        } else {
            return Err(self.cursor.error("Unexpected end of input".to_string()));
        };

        self.expect_token(Token::KeywordAnd, "Expected 'and' after file path")?;
        self.expect_token(Token::KeywordRead, "Expected 'read' after 'and'")?;
        self.expect_token(Token::KeywordContent, "Expected 'content' after 'read'")?;
        self.expect_token(Token::KeywordAs, "Expected 'as' after 'content'")?;

        let variable_name = if let Some(token) = self.cursor.peek() {
            if let Token::Identifier(name) = &token.token {
                self.bump_sync(); // Consume the identifier
                name.clone()
            } else if let Token::KeywordContent = &token.token {
                self.bump_sync(); // Consume the "content" keyword
                "content".to_string()
            } else {
                return Err(ParseError::from_token(
                    format!(
                        "Expected identifier for variable name, found {:?}",
                        token.token
                    ),
                    token,
                ));
            }
        } else {
            return Err(self.cursor.error("Unexpected end of input".to_string()));
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

        // `write line <value> to <out>` / `write chunk <value> to <out>` —
        // append to a server response stream. `line`/`chunk` are contextual
        // identifiers; the lexer merges a following bare-identifier value into
        // the same token (`line payload` -> Identifier("line payload")), so
        // split the value off the marker, mirroring the websocket-message form.
        //
        // Do NOT intercept a bare `line`/`chunk` that is immediately followed by
        // `to`: that is the classic `write <var> to <file>` form using a
        // variable literally named `line`/`chunk` (common in line-by-line file
        // processing). The streaming form always has a value between the marker
        // and `to`, so a bare marker directly before `to` is not a stream write.
        let bare_marker_before_to = matches!(
            self.cursor.peek(),
            Some(t) if matches!(&t.token, Token::Identifier(id) if id == "line" || id == "chunk")
        ) && matches!(
            self.cursor.peek_kind_n(1),
            Some(Token::KeywordTo)
        );

        if !bare_marker_before_to
            && let Some(next_token) = self.cursor.peek()
            && let Token::Identifier(id) = &next_token.token
            && (id == "line"
                || id == "chunk"
                || id.starts_with("line ")
                || id.starts_with("chunk "))
        {
            let id = id.clone();
            let (marker_line, marker_column) = (next_token.line, next_token.column);
            let is_line = id.starts_with("line");
            let marker = if is_line { "line" } else { "chunk" };
            let rest = id
                .strip_prefix(marker)
                .map(str::trim_start)
                .unwrap_or("")
                .to_string();
            self.bump_sync(); // Consume the (possibly merged) marker

            // Build the stream-write `value` and, for the ambiguous merged-
            // identifier form (`write line <ident> to <target>`), the classic
            // file-write `fallback_content`. The merged token `line <ident>` could
            // equally be a variable literally named `line <ident>` (WFL allows
            // space-separated names), so we carry the file-write interpretation
            // and let the interpreter pick based on whether `target` is a stream.
            let (value, fallback_content) = if rest.is_empty() {
                // Value begins with a non-identifier (string/number), so the
                // whole expression — including `with` concatenation — parses
                // cleanly from here. This form was never a valid classic file
                // write (`write line "x" to f` did not parse), so no fallback.
                (self.parse_expression()?, None)
            } else {
                // Ambiguous merged form: `<ident>` alone (stream) vs the full
                // merged `line <ident>` (classic file write of that variable).
                // Parse the two readings INDEPENDENTLY from the same continuation
                // tokens via cursor rewind — NOT by deriving one AST from the
                // other. A trailing `with`/operator continuation desugars
                // differently per leading operand: a builtin name becomes an
                // `ActionCall`, `is between` duplicates the left operand,
                // `starts/ends with` and the pattern operators build calls — none
                // of which survive a leftmost-leaf swap, which silently dropped or
                // mangled the continuation for the classic file-write reading.
                let value_start = self.cursor.checkpoint();

                // Stream reading: split-off `<rest>` as the leading operand.
                let stream_left = Expression::Variable(rest, marker_line, marker_column);
                let value = self.parse_write_value_from_lead(stream_left)?;
                let after_stream = self.cursor.checkpoint();

                // Rewind and parse the classic file-write reading with the whole
                // merged `line <ident>` as the leading operand, over the very same
                // tokens. This alternate interpretation is only USED at runtime
                // when the target turns out to be a file, so it must not be
                // REQUIRED to parse: a value whose stream reading uses grammar the
                // classic reading can't (e.g. a builtin call with named arguments,
                // `write line substring with text: "x" and start: 1 to out`) still
                // has a valid stream reading. If the classic reading fails to
                // parse, drop the fallback rather than failing the statement.
                self.cursor.rewind(value_start);
                let file_left = Expression::Variable(id, marker_line, marker_column);
                let fallback = self.parse_write_value_from_lead(file_left).ok();
                // Only keep the classic fallback when it consumed EXACTLY the same
                // continuation span as the stream reading. A fallback that parses
                // a shorter (or longer) span is a different interpretation of the
                // tokens — e.g. `write line min with a: 1 and b: 2 to <target>`,
                // where the stream reading is the builtin call `min` with named
                // args but `line min with a` only parses up to the `:`. Retaining
                // that partial parse and pairing it with the SAME trailing
                // `to <target>` would silently corrupt a file write, so require the
                // spans to match before trusting the fallback.
                let fallback_end = self.cursor.checkpoint();
                let fallback = fallback.filter(|_| fallback_end == after_stream);
                // Always resume right after the stream value, whatever the
                // (speculative) fallback parse consumed, so `to <target>` follows.
                self.cursor.rewind(after_stream);

                (value, fallback.map(Box::new))
            };

            self.expect_token(
                Token::KeywordTo,
                "Expected 'to <stream>' after the value in a 'write line'/'write chunk' statement",
            )?;
            let target = self.parse_primary_expression()?;
            return Ok(Statement::StreamWriteStatement {
                value,
                target,
                is_line,
                fallback_content,
                line: token_pos.line,
                column: token_pos.column,
            });
        }

        // Check if next token is "binary" for "write binary X into Y" syntax
        if let Some(next_token) = self.cursor.peek()
            && matches!(&next_token.token, Token::KeywordBinary)
        {
            self.bump_sync(); // Consume "binary"

            let content = self.parse_expression()?;

            self.expect_token(
                Token::KeywordInto,
                "Expected 'into' after content in write binary statement",
            )?;

            let target = self.parse_primary_expression()?;

            return Ok(Statement::WriteBinaryStatement {
                content,
                target,
                line: token_pos.line,
                column: token_pos.column,
            });
        }

        // Check if next token is "content" for "write content X into Y" syntax
        if let Some(next_token) = self.cursor.peek()
            && matches!(&next_token.token, Token::KeywordContent)
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
                _ => Err(ParseError::from_token(
                    format!(
                        "Expected 'file' or 'directory' after 'delete', found {:?}",
                        next_token.token
                    ),
                    next_token,
                )),
            }
        } else {
            Err(ParseError::from_token(
                "Expected 'file' or 'directory' after 'delete'".to_string(),
                token_pos,
            ))
        }
    }
}
