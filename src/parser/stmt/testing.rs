//! Test framework statement parsing

use super::super::{ParseError, Parser, Statement};
use super::StmtParser;
use crate::lexer::token::Token;
use crate::parser::ast::Assertion;
use crate::parser::expr::ExprParser;

pub(crate) trait TestingParser<'a>: ExprParser<'a> {
    fn parse_describe_block(&mut self) -> Result<Statement, ParseError>;
    fn parse_test_block(&mut self) -> Result<Statement, ParseError>;
    fn parse_expect_statement(&mut self) -> Result<Statement, ParseError>;
}

impl<'a> TestingParser<'a> for Parser<'a> {
    fn parse_describe_block(&mut self) -> Result<Statement, ParseError> {
        // Parse: describe "Description":
        //            [setup: ... end setup]
        //            test "...": ... end test
        //            [teardown: ... end teardown]
        //        end describe

        // Capture position
        let describe_token = self.cursor.peek().ok_or_else(|| {
            ParseError::from_span(
                "Unexpected end of input while parsing describe block".to_string(),
                self.cursor.current_span(),
                self.cursor.current_line(),
                1,
            )
        })?;
        let (line, column) = (describe_token.line, describe_token.column);

        // Consume 'describe' token
        self.expect_token(Token::KeywordDescribe, "Expected 'describe' keyword")?;

        // Parse description string
        let description = if let Some(token) = self.cursor.peek() {
            if let Token::StringLiteral(s) = &token.token {
                let desc = s.clone();
                self.cursor.bump(); // Consume the string
                desc
            } else {
                return Err(ParseError::from_token(
                    "Expected string literal after 'describe'".to_string(),
                    token,
                ));
            }
        } else {
            return Err(ParseError::from_span(
                "Expected description string after 'describe'".to_string(),
                self.cursor.current_span(),
                self.cursor.current_line(),
                1,
            ));
        };

        // Expect colon
        self.expect_token(Token::Colon, "Expected ':' after describe description")?;

        // Consume newline after colon
        self.skip_eol();

        // Parse body (setup, tests, teardown)
        let mut setup: Option<Vec<Statement>> = None;
        let mut teardown: Option<Vec<Statement>> = None;
        let mut tests: Vec<Statement> = Vec::new();

        loop {
            self.skip_eol();

            let token = self.cursor.peek();
            match token {
                Some(t) => match &t.token {
                    Token::KeywordSetup => {
                        // Parse setup block
                        self.cursor.bump(); // Consume 'setup'
                        self.expect_token(Token::Colon, "Expected ':' after 'setup'")?;
                        self.skip_eol();

                        let mut setup_stmts = Vec::new();
                        loop {
                            self.skip_eol();
                            if let Some(token) = self.cursor.peek() {
                                if matches!(token.token, Token::KeywordEnd) {
                                    break;
                                }
                            }
                            setup_stmts.push(self.parse_statement()?);
                        }

                        self.expect_token(
                            Token::KeywordEnd,
                            "Expected 'end' to close setup block",
                        )?;
                        self.expect_token(
                            Token::KeywordSetup,
                            "Expected 'setup' after 'end' in setup block",
                        )?;
                        setup = Some(setup_stmts);
                    }
                    Token::KeywordTeardown => {
                        // Parse teardown block
                        self.cursor.bump(); // Consume 'teardown'
                        self.expect_token(Token::Colon, "Expected ':' after 'teardown'")?;
                        self.skip_eol();

                        let mut teardown_stmts = Vec::new();
                        loop {
                            self.skip_eol();
                            if let Some(token) = self.cursor.peek() {
                                if matches!(token.token, Token::KeywordEnd) {
                                    break;
                                }
                            }
                            teardown_stmts.push(self.parse_statement()?);
                        }

                        self.expect_token(
                            Token::KeywordEnd,
                            "Expected 'end' to close teardown block",
                        )?;
                        self.expect_token(
                            Token::KeywordTeardown,
                            "Expected 'teardown' after 'end' in teardown block",
                        )?;
                        teardown = Some(teardown_stmts);
                    }
                    Token::KeywordTest => {
                        // Parse test block
                        tests.push(self.parse_test_block()?);
                    }
                    Token::KeywordDescribe => {
                        // Nested describe block
                        tests.push(self.parse_describe_block()?);
                    }
                    Token::KeywordEnd => {
                        // End of describe block
                        break;
                    }
                    _ => {
                        return Err(ParseError::from_token(
                            "Expected 'setup', 'test', 'teardown', or 'end' in describe block"
                                .to_string(),
                            t,
                        ));
                    }
                },
                None => {
                    return Err(ParseError::from_span(
                        "Unexpected end of input in describe block".to_string(),
                        self.cursor.current_span(),
                        self.cursor.current_line(),
                        1,
                    ));
                }
            }
        }

        // Expect 'end describe'
        self.expect_token(Token::KeywordEnd, "Expected 'end' to close describe block")?;
        self.expect_token(
            Token::KeywordDescribe,
            "Expected 'describe' after 'end' in describe block",
        )?;

        Ok(Statement::DescribeBlock {
            description,
            setup,
            teardown,
            tests,
            line,
            column,
        })
    }

    fn parse_test_block(&mut self) -> Result<Statement, ParseError> {
        // Parse: test "description":
        //            [statements]
        //        end test

        // Capture position
        let test_token = self.cursor.peek().ok_or_else(|| {
            ParseError::from_span(
                "Unexpected end of input while parsing test block".to_string(),
                self.cursor.current_span(),
                self.cursor.current_line(),
                1,
            )
        })?;
        let (line, column) = (test_token.line, test_token.column);

        // Consume 'test' token
        self.expect_token(Token::KeywordTest, "Expected 'test' keyword")?;

        // Parse description string
        let description = if let Some(token) = self.cursor.peek() {
            if let Token::StringLiteral(s) = &token.token {
                let desc = s.clone();
                self.cursor.bump(); // Consume the string
                desc
            } else {
                return Err(ParseError::from_token(
                    "Expected string literal after 'test'".to_string(),
                    token,
                ));
            }
        } else {
            return Err(ParseError::from_span(
                "Expected description string after 'test'".to_string(),
                self.cursor.current_span(),
                self.cursor.current_line(),
                1,
            ));
        };

        // Expect colon
        self.expect_token(Token::Colon, "Expected ':' after test description")?;

        // Consume newline after colon
        self.skip_eol();

        // Parse body statements
        let mut body = Vec::new();
        loop {
            self.skip_eol();

            if let Some(token) = self.cursor.peek() {
                if matches!(token.token, Token::KeywordEnd) {
                    break;
                }
            }

            body.push(self.parse_statement()?);
        }

        // Expect 'end test'
        self.expect_token(Token::KeywordEnd, "Expected 'end' to close test block")?;
        self.expect_token(
            Token::KeywordTest,
            "Expected 'test' after 'end' in test block",
        )?;

        Ok(Statement::TestBlock {
            description,
            body,
            line,
            column,
        })
    }

    fn parse_expect_statement(&mut self) -> Result<Statement, ParseError> {
        // Parse: expect <expression> to <assertion>
        //
        // Examples:
        // expect result to equal 5
        // expect list to contain "item"
        // expect value to be greater than 10

        // Capture position
        let expect_token = self.cursor.peek().ok_or_else(|| {
            ParseError::from_span(
                "Unexpected end of input while parsing expect statement".to_string(),
                self.cursor.current_span(),
                self.cursor.current_line(),
                1,
            )
        })?;
        let (line, column) = (expect_token.line, expect_token.column);

        // Consume 'expect' token
        self.expect_token(Token::KeywordExpect, "Expected 'expect' keyword")?;

        // Parse subject expression
        let subject = self.parse_expression()?;

        // Expect 'to' keyword
        self.expect_token(Token::KeywordTo, "Expected 'to' after expect subject")?;

        // Parse assertion type based on next token
        let assertion = self.parse_assertion()?;

        Ok(Statement::ExpectStatement {
            subject,
            assertion,
            line,
            column,
        })
    }
}

impl<'a> Parser<'a> {
    /// Helper method to parse assertion types for expect statements
    fn parse_assertion(&mut self) -> Result<Assertion, ParseError> {
        let token = self.cursor.peek().ok_or_else(|| {
            ParseError::from_span(
                "Expected assertion type after 'to'".to_string(),
                self.cursor.current_span(),
                self.cursor.current_line(),
                1,
            )
        })?;

        match &token.token {
            Token::KeywordEqual => {
                self.cursor.bump(); // Consume 'equal'
                let value = self.parse_expression()?;
                Ok(Assertion::Equal(value))
            }
            Token::KeywordBe => {
                self.cursor.bump(); // Consume 'be'

                // Check what comes after 'be'
                if let Some(next_token) = self.cursor.peek() {
                    match &next_token.token {
                        Token::BooleanLiteral(true) => {
                            self.cursor.bump();
                            Ok(Assertion::BeYes)
                        }
                        Token::BooleanLiteral(false) => {
                            self.cursor.bump();
                            Ok(Assertion::BeNo)
                        }
                        Token::KeywordGreater => {
                            self.cursor.bump(); // Consume 'greater'
                            self.expect_token(
                                Token::KeywordThan,
                                "Expected 'than' after 'greater'",
                            )?;
                            let value = self.parse_expression()?;
                            Ok(Assertion::GreaterThan(value))
                        }
                        Token::KeywordLess => {
                            self.cursor.bump(); // Consume 'less'
                            self.expect_token(Token::KeywordThan, "Expected 'than' after 'less'")?;
                            let value = self.parse_expression()?;
                            Ok(Assertion::LessThan(value))
                        }
                        Token::Identifier(id) if id == "empty" => {
                            self.cursor.bump(); // Consume 'empty'
                            Ok(Assertion::BeEmpty)
                        }
                        Token::KeywordOf => {
                            self.cursor.bump(); // Consume 'of'

                            // Check if next token is "type" identifier
                            if let Some(token) = self.cursor.peek() {
                                if let Token::Identifier(id) = &token.token {
                                    if id == "type" {
                                        self.cursor.bump(); // Consume 'type'

                                        // Expect a string literal for the type name
                                        if let Some(token) = self.cursor.peek() {
                                            if let Token::StringLiteral(type_name) = &token.token {
                                                let tn = type_name.clone();
                                                self.cursor.bump();
                                                return Ok(Assertion::BeOfType(tn));
                                            }
                                        }
                                    }
                                }
                            }

                            Err(ParseError::from_span(
                                "Expected 'type' after 'of'".to_string(),
                                self.cursor.current_span(),
                                self.cursor.current_line(),
                                1,
                            ))
                        }
                        _ => {
                            // Default: treat as 'be' synonym for 'equal'
                            let value = self.parse_expression()?;
                            Ok(Assertion::Be(value))
                        }
                    }
                } else {
                    Err(ParseError::from_span(
                        "Expected value or condition after 'be'".to_string(),
                        self.cursor.current_span(),
                        self.cursor.current_line(),
                        1,
                    ))
                }
            }
            Token::KeywordGreater => {
                self.cursor.bump(); // Consume 'greater'
                self.expect_token(Token::KeywordThan, "Expected 'than' after 'greater'")?;
                let value = self.parse_expression()?;
                Ok(Assertion::GreaterThan(value))
            }
            Token::KeywordLess => {
                self.cursor.bump(); // Consume 'less'
                self.expect_token(Token::KeywordThan, "Expected 'than' after 'less'")?;
                let value = self.parse_expression()?;
                Ok(Assertion::LessThan(value))
            }
            Token::KeywordContain => {
                self.cursor.bump(); // Consume 'contain'
                let value = self.parse_expression()?;
                Ok(Assertion::Contain(value))
            }
            Token::KeywordExist => {
                self.cursor.bump(); // Consume 'exist'
                Ok(Assertion::Exist)
            }
            Token::KeywordHave => {
                self.cursor.bump(); // Consume 'have'

                // Check if next token is "length" identifier
                if let Some(token) = self.cursor.peek() {
                    if let Token::Identifier(id) = &token.token {
                        if id == "length" {
                            self.cursor.bump(); // Consume 'length'
                            let value = self.parse_expression()?;
                            return Ok(Assertion::HaveLength(value));
                        }
                    }
                }

                Err(ParseError::from_span(
                    "Expected 'length' after 'have'".to_string(),
                    self.cursor.current_span(),
                    self.cursor.current_line(),
                    1,
                ))
            }
            _ => Err(ParseError::from_token(
                format!(
                    "Unknown assertion type: expected 'equal', 'be', 'contain', etc. Got: {:?}",
                    token.token
                ),
                token,
            )),
        }
    }
}
