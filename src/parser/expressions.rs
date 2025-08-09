use super::Parser;
use super::ast::*;
use super::error::ParseError;
use crate::exec_trace;
use crate::lexer::token::Token;

impl<'a> Parser<'a> {
    pub(crate) fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_binary_expression(0)
    }

    pub(crate) fn parse_binary_expression(
        &mut self,
        precedence: u8,
    ) -> Result<Expression, ParseError> {
        let mut left = self.parse_primary_expression()?;

        let left_line = if let Some(token) = self.tokens.peek() {
            token.line
        } else {
            0
        };

        while let Some(token_pos) = self.tokens.peek().cloned() {
            let token = token_pos.token.clone();
            let line = token_pos.line;
            let column = token_pos.column;

            if line > left_line || Parser::is_statement_starter(&token) {
                break;
            }

            let op = match token {
                Token::Plus => {
                    self.tokens.next();
                    Some((Operator::Plus, 1))
                }
                Token::KeywordPlus => {
                    self.tokens.next();
                    Some((Operator::Plus, 1))
                }
                Token::KeywordMinus => {
                    self.tokens.next();
                    Some((Operator::Minus, 1))
                }
                Token::KeywordTimes => {
                    self.tokens.next();
                    Some((Operator::Multiply, 2))
                }
                Token::KeywordDividedBy => {
                    self.tokens.next();
                    Some((Operator::Divide, 2))
                }
                Token::KeywordDivided => {
                    self.tokens.next();
                    self.expect_token(Token::KeywordBy, "Expected 'by' after 'divided'")?;
                    self.tokens.next();
                    Some((Operator::Divide, 2))
                }
                Token::Equals => {
                    self.tokens.next();
                    Some((Operator::Equals, 0))
                }
                Token::KeywordIs => {
                    self.tokens.next();

                    if let Some(next_token) = self.tokens.peek().cloned() {
                        match &next_token.token {
                            Token::KeywordEqual => {
                                self.tokens.next();
                                if let Some(to_token) = self.tokens.peek().cloned() {
                                    if matches!(to_token.token, Token::KeywordTo) {
                                        self.tokens.next();
                                        Some((Operator::Equals, 0))
                                    } else {
                                        Some((Operator::Equals, 0))
                                    }
                                } else {
                                    return Err(ParseError::new(
                                        "Unexpected end of input after 'is equal'".into(),
                                        line,
                                        column,
                                    ));
                                }
                            }
                            Token::KeywordNot => {
                                self.tokens.next();
                                Some((Operator::NotEquals, 0))
                            }
                            Token::KeywordGreater => {
                                self.tokens.next();
                                if let Some(than_token) = self.tokens.peek().cloned() {
                                    if matches!(than_token.token, Token::KeywordThan) {
                                        self.tokens.next();
                                        Some((Operator::GreaterThan, 0))
                                    } else {
                                        Some((Operator::GreaterThan, 0))
                                    }
                                } else {
                                    return Err(ParseError::new(
                                        "Unexpected end of input after 'is greater'".into(),
                                        line,
                                        column,
                                    ));
                                }
                            }
                            Token::KeywordLess => {
                                self.tokens.next();
                                if let Some(than_token) = self.tokens.peek().cloned() {
                                    if matches!(than_token.token, Token::KeywordThan) {
                                        self.tokens.next();
                                        if let Some(or_token) = self.tokens.peek().cloned() {
                                            if matches!(or_token.token, Token::KeywordOr) {
                                                self.tokens.next();
                                                if let Some(equal_token) =
                                                    self.tokens.peek().cloned()
                                                {
                                                    if matches!(
                                                        equal_token.token,
                                                        Token::KeywordEqual
                                                    ) {
                                                        self.tokens.next();
                                                        if let Some(to_token) =
                                                            self.tokens.peek().cloned()
                                                        {
                                                            if matches!(
                                                                to_token.token,
                                                                Token::KeywordTo
                                                            ) {
                                                                self.tokens.next();
                                                                Some((Operator::LessThanOrEqual, 0))
                                                            } else {
                                                                Some((Operator::LessThanOrEqual, 0))
                                                            }
                                                        } else {
                                                            Some((Operator::LessThanOrEqual, 0))
                                                        }
                                                    } else {
                                                        Some((Operator::LessThan, 0))
                                                    }
                                                } else {
                                                    Some((Operator::LessThan, 0))
                                                }
                                            } else {
                                                Some((Operator::LessThan, 0))
                                            }
                                        } else {
                                            Some((Operator::LessThan, 0))
                                        }
                                    } else {
                                        Some((Operator::LessThan, 0))
                                    }
                                } else {
                                    return Err(ParseError::new(
                                        "Unexpected end of input after 'is less'".into(),
                                        line,
                                        column,
                                    ));
                                }
                            }
                            _ => Some((Operator::Equals, 0)),
                        }
                    } else {
                        return Err(ParseError::new(
                            "Unexpected end of input after 'is'".into(),
                            line,
                            column,
                        ));
                    }
                }
                Token::KeywordWith => {
                    if let Expression::Variable(ref name, var_line, var_column) = left
                        && self.known_actions.contains(name)
                    {
                        self.tokens.next();

                        let mut arguments = Vec::with_capacity(4);
                        if let Some(peek) = self.tokens.peek() {
                            if peek.token == Token::LeftParen {
                                arguments = self.parse_argument_list()?;
                            } else {
                                let first = self.parse_expression()?;
                                arguments.push(Argument {
                                    name: None,
                                    value: first,
                                });
                                while let Some(and_tok) = self.tokens.peek() {
                                    if matches!(and_tok.token, Token::KeywordAnd) {
                                        self.tokens.next();
                                        let val = self.parse_expression()?;
                                        arguments.push(Argument {
                                            name: None,
                                            value: val,
                                        });
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }

                        left = Expression::ActionCall {
                            name: name.clone(),
                            arguments,
                            line: var_line,
                            column: var_column,
                        };
                        continue;
                    }

                    self.tokens.next();
                    let right = self.parse_expression()?;
                    left = Expression::Concatenation {
                        left: Box::new(left),
                        right: Box::new(right),
                        line: token_pos.line,
                        column: token_pos.column,
                    };
                    continue;
                }
                Token::KeywordAnd => {
                    self.tokens.next();
                    Some((Operator::And, 0))
                }
                Token::KeywordOr => {
                    self.tokens.next();

                    if let Some(equal_token) = self.tokens.peek().cloned()
                        && matches!(equal_token.token, Token::KeywordEqual)
                    {
                        self.tokens.next();
                        if let Some(to_token) = self.tokens.peek().cloned()
                            && matches!(to_token.token, Token::KeywordTo)
                        {
                            self.tokens.next();
                            if let Expression::BinaryOperation {
                                operator,
                                left: left_expr,
                                right: right_expr,
                                line: op_line,
                                column: op_column,
                            } = &left
                            {
                                if *operator == Operator::LessThan {
                                    left = Expression::BinaryOperation {
                                        left: left_expr.clone(),
                                        operator: Operator::LessThanOrEqual,
                                        right: right_expr.clone(),
                                        line: *op_line,
                                        column: *op_column,
                                    };
                                    continue;
                                } else if *operator == Operator::GreaterThan {
                                    left = Expression::BinaryOperation {
                                        left: left_expr.clone(),
                                        operator: Operator::GreaterThanOrEqual,
                                        right: right_expr.clone(),
                                        line: *op_line,
                                        column: *op_column,
                                    };
                                    continue;
                                }
                            }
                        }
                    }

                    Some((Operator::Or, 0))
                }
                Token::KeywordMatches => {
                    self.tokens.next();
                    if let Some(pattern_token) = self.tokens.peek().cloned()
                        && matches!(pattern_token.token, Token::KeywordPattern)
                    {
                        self.tokens.next();
                    }
                    let pattern_expr = self.parse_binary_expression(precedence + 1)?;
                    left = Expression::PatternMatch {
                        text: Box::new(left),
                        pattern: Box::new(pattern_expr),
                        line,
                        column,
                    };
                    continue;
                }
                Token::KeywordFind => {
                    self.tokens.next();
                    if let Some(pattern_token) = self.tokens.peek().cloned()
                        && matches!(pattern_token.token, Token::KeywordPattern)
                    {
                        self.tokens.next();
                    }
                    let pattern_expr = self.parse_binary_expression(precedence + 1)?;
                    if let Some(in_token) = self.tokens.peek().cloned()
                        && matches!(in_token.token, Token::KeywordIn)
                    {
                        self.tokens.next();
                        let text_expr = self.parse_binary_expression(precedence + 1)?;
                        left = Expression::PatternFind {
                            text: Box::new(text_expr),
                            pattern: Box::new(pattern_expr),
                            line,
                            column,
                        };
                        continue;
                    }
                    left = Expression::PatternFind {
                        text: Box::new(left),
                        pattern: Box::new(pattern_expr),
                        line,
                        column,
                    };
                    continue;
                }
                Token::KeywordReplace => {
                    self.tokens.next();
                    if let Some(pattern_token) = self.tokens.peek().cloned()
                        && matches!(pattern_token.token, Token::KeywordPattern)
                    {
                        self.tokens.next();
                    }
                    let pattern_expr = self.parse_binary_expression(precedence + 1)?;
                    if let Some(with_token) = self.tokens.peek().cloned()
                        && matches!(with_token.token, Token::KeywordWith)
                    {
                        self.tokens.next();
                        let replacement_expr = self.parse_binary_expression(precedence + 1)?;
                        if let Some(in_token) = self.tokens.peek().cloned()
                            && matches!(in_token.token, Token::KeywordIn)
                        {
                            self.tokens.next();
                            let text_expr = self.parse_binary_expression(precedence + 1)?;
                            left = Expression::PatternReplace {
                                text: Box::new(text_expr),
                                pattern: Box::new(pattern_expr),
                                replacement: Box::new(replacement_expr),
                                line,
                                column,
                            };
                            continue;
                        }
                        left = Expression::PatternReplace {
                            text: Box::new(left),
                            pattern: Box::new(pattern_expr),
                            replacement: Box::new(replacement_expr),
                            line,
                            column,
                        };
                        continue;
                    }
                    return Err(ParseError::new(
                        "Expected 'with' after pattern in replace operation".to_string(),
                        line,
                        column,
                    ));
                }
                Token::KeywordSplit => {
                    self.tokens.next();
                    let text_expr = self.parse_binary_expression(precedence + 1)?;
                    if let Some(on_token) = self.tokens.peek().cloned()
                        && matches!(on_token.token, Token::KeywordOn)
                    {
                        self.tokens.next();
                        if let Some(pattern_token) = self.tokens.peek().cloned()
                            && matches!(pattern_token.token, Token::KeywordPattern)
                        {
                            self.tokens.next();
                        }
                        let pattern_expr = self.parse_binary_expression(precedence + 1)?;
                        left = Expression::PatternSplit {
                            text: Box::new(text_expr),
                            pattern: Box::new(pattern_expr),
                            line,
                            column,
                        };
                        continue;
                    }
                    return Err(ParseError::new(
                        "Expected 'on' after text in split operation".to_string(),
                        line,
                        column,
                    ));
                }
                Token::KeywordContains => {
                    self.tokens.next();
                    if let Some(pattern_token) = self.tokens.peek().cloned()
                        && matches!(pattern_token.token, Token::KeywordPattern)
                    {
                        self.tokens.next();
                        let pattern_expr = self.parse_binary_expression(precedence + 1)?;
                        left = Expression::PatternMatch {
                            text: Box::new(left),
                            pattern: Box::new(pattern_expr),
                            line,
                            column,
                        };
                        continue;
                    }
                    Some((Operator::Contains, 0))
                }
                Token::Colon => {
                    self.tokens.next();
                    continue;
                }
                _ => None,
            };

            if let Some((operator, op_precedence)) = op {
                if op_precedence < precedence {
                    break;
                }
                let right = self.parse_binary_expression(op_precedence + 1)?;
                left = Expression::BinaryOperation {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right),
                    line: token_pos.line,
                    column: token_pos.column,
                };
            } else {
                break;
            }
        }

        Ok(left)
    }

    pub(crate) fn parse_primary_expression(&mut self) -> Result<Expression, ParseError> {
        if let Some(token) = self.tokens.peek().cloned() {
            let result = match &token.token {
                Token::LeftBracket => {
                    let bracket_token = self.tokens.next().unwrap();
                    if let Some(next_token) = self.tokens.peek()
                        && next_token.token == Token::RightBracket
                    {
                        self.tokens.next();
                        return Ok(Expression::Literal(
                            Literal::List(Vec::new()),
                            bracket_token.line,
                            bracket_token.column,
                        ));
                    }
                    let mut elements = Vec::new();
                    elements.push(self.parse_list_element()?);
                    while let Some(next_token) = self.tokens.peek() {
                        if next_token.token == Token::RightBracket {
                            self.tokens.next();
                            return Ok(Expression::Literal(
                                Literal::List(elements),
                                bracket_token.line,
                                bracket_token.column,
                            ));
                        } else if next_token.token == Token::KeywordAnd
                            || next_token.token == Token::Colon
                        {
                            self.tokens.next();
                            elements.push(self.parse_list_element()?);
                        } else {
                            return Err(ParseError::new(
                                format!(
                                    "Expected ']' or 'and' in list literal, found {:?}",
                                    next_token.token
                                ),
                                next_token.line,
                                next_token.column,
                            ));
                        }
                    }
                    return Err(ParseError::new(
                        "Unexpected end of input while parsing list literal".into(),
                        bracket_token.line,
                        bracket_token.column,
                    ));
                }
                Token::LeftParen => {
                    self.tokens.next();
                    let expr = self.parse_expression()?;
                    if let Some(token) = self.tokens.peek().cloned() {
                        if token.token == Token::RightParen {
                            self.tokens.next();
                            return Ok(expr);
                        } else {
                            return Err(ParseError::new(
                                format!("Expected closing parenthesis, found {:?}", token.token),
                                token.line,
                                token.column,
                            ));
                        }
                    } else {
                        return Err(ParseError::new(
                            "Expected closing parenthesis, found end of input".into(),
                            token.line,
                            token.column,
                        ));
                    }
                }
                Token::StringLiteral(s) => {
                    let token_pos = self.tokens.next().unwrap();
                    Ok(Expression::Literal(
                        Literal::String(s.to_string()),
                        token_pos.line,
                        token_pos.column,
                    ))
                }
                Token::IntLiteral(n) => {
                    let token_pos = self.tokens.next().unwrap();
                    Ok(Expression::Literal(
                        Literal::Integer(*n),
                        token_pos.line,
                        token_pos.column,
                    ))
                }
                Token::FloatLiteral(f) => {
                    let token_pos = self.tokens.next().unwrap();
                    Ok(Expression::Literal(
                        Literal::Float(*f),
                        token_pos.line,
                        token_pos.column,
                    ))
                }
                Token::BooleanLiteral(b) => {
                    let token_pos = self.tokens.next().unwrap();
                    Ok(Expression::Literal(
                        Literal::Boolean(*b),
                        token_pos.line,
                        token_pos.column,
                    ))
                }
                Token::NothingLiteral => {
                    let token_pos = self.tokens.next().unwrap();
                    Ok(Expression::Literal(
                        Literal::Nothing,
                        token_pos.line,
                        token_pos.column,
                    ))
                }
                Token::Identifier(name) => {
                    self.tokens.next();
                    if let Some(next_token) = self.tokens.peek().cloned() {
                        if next_token.token == Token::Dot {
                            self.tokens.next();
                            if let Some(property_token) = self.tokens.peek().cloned() {
                                if let Token::Identifier(property_name) = &property_token.token {
                                    self.tokens.next();
                                    if let Some(paren_token) = self.tokens.peek().cloned()
                                        && paren_token.token == Token::LeftParen
                                    {
                                        self.tokens.next();
                                        let mut arguments = Vec::new();
                                        if let Some(next_token) = self.tokens.peek()
                                            && next_token.token != Token::RightParen
                                        {
                                            let expr = self.parse_expression()?;
                                            arguments.push(Argument {
                                                name: None,
                                                value: expr,
                                            });
                                            while let Some(comma_token) = self.tokens.peek() {
                                                if comma_token.token == Token::Comma {
                                                    self.tokens.next();
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
                                                token.line,
                                                token.column,
                                            )),
                                            method: property_name.clone(),
                                            arguments,
                                            line: token.line,
                                            column: token.column,
                                        });
                                    }
                                    return Ok(Expression::PropertyAccess {
                                        object: Box::new(Expression::Variable(
                                            name.clone(),
                                            token.line,
                                            token.column,
                                        )),
                                        property: property_name.clone(),
                                        line: token.line,
                                        column: token.column,
                                    });
                                } else {
                                    return Err(ParseError::new(
                                        "Expected property name after '.'".to_string(),
                                        property_token.line,
                                        property_token.column,
                                    ));
                                }
                            } else {
                                return Err(ParseError::new(
                                    "Expected property name after '.'".to_string(),
                                    token.line,
                                    token.column,
                                ));
                            }
                        } else if let Token::Identifier(id) = &next_token.token
                            && id.to_lowercase() == "with"
                        {
                            self.tokens.next();
                            let arguments = self.parse_argument_list()?;
                            let token_line = token.line;
                            let token_column = token.column;
                            return Ok(Expression::ActionCall {
                                name: name.clone(),
                                arguments,
                                line: token_line,
                                column: token_column,
                            });
                        }
                    }
                    let is_standalone = false;
                    let token_line = token.line;
                    let token_column = token.column;
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
                    self.tokens.next();
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
                Token::KeywordWith => Err(ParseError::new(
                    "Unexpected 'with' in primary expression".to_string(),
                    token.line,
                    token.column,
                )),
                Token::KeywordCount => {
                    self.tokens.next();
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "count".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordPattern => {
                    self.tokens.next();
                    if let Some(pattern_token) = self.tokens.peek().cloned() {
                        if let Token::StringLiteral(pattern) = &pattern_token.token {
                            let token_pos = self.tokens.next().unwrap();
                            return Ok(Expression::Literal(
                                Literal::Pattern(pattern.clone()),
                                token_pos.line,
                                token_pos.column,
                            ));
                        } else {
                            return Err(ParseError::new(
                                format!(
                                    "Expected string literal after 'pattern', found {:?}",
                                    pattern_token.token
                                ),
                                pattern_token.line,
                                pattern_token.column,
                            ));
                        }
                    } else {
                        return Err(ParseError::new(
                            "Unexpected end of input after 'pattern'".to_string(),
                            token.line,
                            token.column,
                        ));
                    }
                }
                Token::KeywordLoop => {
                    self.tokens.next();
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "loop".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordRepeat => {
                    self.tokens.next();
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "repeat".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordExit => {
                    self.tokens.next();
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "exit".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordBack => {
                    self.tokens.next();
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "back".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordTry => {
                    self.tokens.next();
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "try".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordWhen => {
                    self.tokens.next();
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "when".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordError => {
                    self.tokens.next();
                    let token_line = token.line;
                    let token_column = token.column;
                    Ok(Expression::Variable(
                        "error".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordFile => {
                    self.tokens.next();
                    let token_line = token.line;
                    let token_column = token.column;
                    if let Some(next_token) = self.tokens.peek()
                        && next_token.token == Token::KeywordExists
                    {
                        self.tokens.next();
                        self.expect_token(Token::KeywordAt, "Expected 'at' after 'file exists'")?;
                        let path = self.parse_primary_expression()?;
                        return Ok(Expression::FileExists {
                            path: Box::new(path),
                            line: token_line,
                            column: token_column,
                        });
                    }
                    Ok(Expression::Variable(
                        "file".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordDirectory => {
                    self.tokens.next();
                    let token_line = token.line;
                    let token_column = token.column;
                    if let Some(next_token) = self.tokens.peek()
                        && next_token.token == Token::KeywordExists
                    {
                        self.tokens.next();
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
                    Ok(Expression::Variable(
                        "directory".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordList => {
                    self.tokens.next();
                    let token_line = token.line;
                    let token_column = token.column;
                    if let Some(next_token) = self.tokens.peek()
                        && next_token.token == Token::KeywordFiles
                    {
                        self.tokens.next();
                        self.expect_token(Token::KeywordIn, "Expected 'in' after 'list files'")?;
                        let path = self.parse_primary_expression()?;
                        if let Some(next) = self.tokens.peek() {
                            match &next.token {
                                Token::KeywordRecursively => {
                                    self.tokens.next();
                                    if let Some(with_token) = self.tokens.peek()
                                        && with_token.token == Token::KeywordWith
                                    {
                                        self.tokens.next();
                                        let extensions = self.parse_extension_filter()?;
                                        return Ok(Expression::ListFilesRecursive {
                                            path: Box::new(path),
                                            extensions: Some(extensions),
                                            line: token_line,
                                            column: token_column,
                                        });
                                    }
                                    return Ok(Expression::ListFilesRecursive {
                                        path: Box::new(path),
                                        extensions: None,
                                        line: token_line,
                                        column: token_column,
                                    });
                                }
                                Token::KeywordWith => {
                                    self.tokens.next();
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
                        return Ok(Expression::ListFiles {
                            path: Box::new(path),
                            line: token_line,
                            column: token_column,
                        });
                    }
                    Ok(Expression::Variable(
                        "list".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordRead => {
                    self.tokens.next();
                    let token_line = token.line;
                    let token_column = token.column;
                    if let Some(next_token) = self.tokens.peek()
                        && next_token.token == Token::KeywordContent
                    {
                        let mut lookahead = self.tokens.clone();
                        lookahead.next();
                        if let Some(after_content) = lookahead.peek()
                            && after_content.token == Token::KeywordFrom
                        {
                            self.tokens.next();
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
                    }
                    Ok(Expression::Variable(
                        "read".to_string(),
                        token_line,
                        token_column,
                    ))
                }
                Token::KeywordFind => {
                    self.tokens.next();
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
                    self.tokens.next();
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
                    self.tokens.next();
                    let text_expr = self.parse_expression()?;
                    self.expect_token(
                        Token::KeywordOn,
                        "Expected 'on' after text in split expression",
                    )?;
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
                _ => Err(ParseError::new(
                    format!("Unexpected token in expression: {:?}", token.token),
                    token.line,
                    token.column,
                )),
            };

            let mut expr = result?;
            while let Some(token) = self.tokens.peek().cloned() {
                match &token.token {
                    Token::KeywordOf => {
                        self.tokens.next();
                        let first_arg = self.parse_expression()?;
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
                            while let Some(and_token) = self.tokens.peek().cloned() {
                                if let Token::KeywordAnd = &and_token.token {
                                    self.tokens.next();
                                    let arg_value = self.parse_expression()?;
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
                            return Err(ParseError::new(
                                "Member access not supported with expression arguments".to_string(),
                                token.line,
                                token.column,
                            ));
                        }
                    }
                    Token::KeywordAt => {
                        self.tokens.next();
                        let index = self.parse_expression()?;
                        expr = Expression::IndexAccess {
                            collection: Box::new(expr),
                            index: Box::new(index),
                            line: token.line,
                            column: token.column,
                        };
                    }
                    Token::Identifier(id) if id == "." => {
                        self.tokens.next();
                        if let Some(member_token) = self.tokens.peek().cloned() {
                            if let Token::Identifier(member) = &member_token.token {
                                self.tokens.next();
                                if let Expression::Variable(container_name, _, _) = expr {
                                    expr = Expression::StaticMemberAccess {
                                        container: container_name,
                                        member: member.clone(),
                                        line: token.line,
                                        column: token.column,
                                    };
                                } else {
                                    return Err(ParseError::new(
                                        "Static access must start with a container identifier"
                                            .to_string(),
                                        token.line,
                                        token.column,
                                    ));
                                }
                            } else {
                                return Err(ParseError::new(
                                    "Expected member name after '.'".to_string(),
                                    member_token.line,
                                    member_token.column,
                                ));
                            }
                        } else {
                            return Err(ParseError::new(
                                "Expected member name after '.'".to_string(),
                                token.line,
                                token.column,
                            ));
                        }
                    }
                    _ => break,
                }
            }
            return Ok(expr);
        }
        Err(ParseError::new(
            "Unexpected end of input in expression".to_string(),
            0,
            0,
        ))
    }
    pub(crate) fn parse_argument_list(&mut self) -> Result<Vec<Argument>, ParseError> {
        let mut arguments = Vec::with_capacity(4);

        if let Some(token) = self.tokens.peek()
            && token.token == Token::LeftParen
        {
            let before_count = self.tokens.clone().count();

            self.tokens.next();

            if let Some(next_token) = self.tokens.peek()
                && next_token.token != Token::RightParen
            {
                loop {
                    let name = if let Some(peek_token) = self.tokens.peek() {
                        if let Token::Identifier(id) = &peek_token.token {
                            let mut tokens_clone = self.tokens.clone();
                            tokens_clone.next();
                            if let Some(eq_token) = tokens_clone.peek()
                                && eq_token.token == Token::Equals
                            {
                                self.tokens.next();
                                self.tokens.next();
                                Some(id.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    let expr = self.parse_expression()?;
                    arguments.push(Argument { name, value: expr });

                    if let Some(comma_token) = self.tokens.peek()
                        && comma_token.token == Token::Comma
                    {
                        self.tokens.next();
                        continue;
                    }

                    break;
                }
            }

            self.expect_token(Token::RightParen, "Expected ')' after arguments")?;

            let after_count = self.tokens.clone().count();
            assert!(
                after_count < before_count,
                "No progress in parse_argument_list"
            );
        }

        Ok(arguments)
    }

    pub(crate) fn parse_extension_filter(&mut self) -> Result<Vec<Expression>, ParseError> {
        if let Some(token) = self.tokens.peek() {
            match &token.token {
                Token::Identifier(id)
                    if id.to_lowercase() == "extension" || id.to_lowercase() == "extensions" =>
                {
                    self.tokens.next();
                }
                _ => {
                    return Err(ParseError::new(
                        "Expected 'extension' or 'extensions'".to_string(),
                        token.line,
                        token.column,
                    ));
                }
            }
        } else {
            return Err(ParseError::new(
                "Expected 'extension' or 'extensions'".to_string(),
                0,
                0,
            ));
        }

        let mut extensions = Vec::new();

        loop {
            let ext_expr = self.parse_primary_expression()?;
            extensions.push(ext_expr);

            if let Some(and_token) = self.tokens.peek()
                && (matches!(and_token.token, Token::KeywordAnd)
                    || matches!(and_token.token, Token::Comma))
            {
                self.tokens.next();
                continue;
            }
            break;
        }

        Ok(extensions)
    }

    pub(crate) fn parse_list_element(&mut self) -> Result<Expression, ParseError> {
        self.parse_primary_expression()
    }
}
