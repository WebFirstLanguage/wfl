//! Binary expression parsing with operator precedence
//!
//! This module handles parsing of binary operations, including arithmetic, logical,
//! comparison, pattern matching, and custom language constructs.

use super::super::{Argument, Expression, Operator, ParseError, Parser};
use super::{ExprParser, PrimaryExprParser};
use crate::lexer::token::Token;

/// Trait for parsing binary expressions with operator precedence
pub(crate) trait BinaryExprParser<'a> {
    /// Parses a binary expression with operator precedence.
    ///
    /// This method parses binary operations, handling operator precedence and associativity for a wide range of operators, including arithmetic, logical, comparison, pattern matching, and custom language constructs. It supports multi-token operators (such as "divided by", "is equal to", "is less than or equal to"), action calls, and special pattern-related expressions. The parser ensures correct grouping and evaluation order by recursively parsing sub-expressions with increasing precedence.
    ///
    /// # Parameters
    /// - `precedence`: The minimum precedence level to consider when parsing operators. Operators with lower precedence will not be parsed at this level.
    ///
    /// # Returns
    /// Returns an `Expression` representing the parsed binary expression, or a `ParseError` if the syntax is invalid.
    fn parse_binary_expression(&mut self, precedence: u8) -> Result<Expression, ParseError>;

    /// Parses a function/action call expression.
    ///
    /// # Parameters
    /// - `call_line`: Line number where the 'call' keyword was found
    /// - `call_column`: Column number where the 'call' keyword was found
    fn parse_call_expression(
        &mut self,
        call_line: usize,
        call_column: usize,
    ) -> Result<Expression, ParseError>;

    /// Parses a comma-separated or 'and'-separated argument list for action calls.
    fn parse_argument_list(&mut self) -> Result<Vec<Argument>, ParseError>;
}

impl<'a> BinaryExprParser<'a> for Parser<'a> {
    fn parse_binary_expression(&mut self, precedence: u8) -> Result<Expression, ParseError> {
        let mut left = self.parse_primary_expression()?;

        while let Some(token_pos) = self.cursor.peek() {
            let token = &token_pos.token;
            let line = token_pos.line;
            let column = token_pos.column;

            // Stop at Eol (statement boundary) or statement starter
            if matches!(token, Token::Eol) || Parser::is_statement_starter(token) {
                break;
            }

            let op = match token {
                Token::Plus => Some((Operator::Plus, 1)),
                Token::KeywordPlus => Some((Operator::Plus, 1)),
                Token::Minus => Some((Operator::Minus, 1)),
                Token::KeywordMinus => Some((Operator::Minus, 1)),
                Token::KeywordTimes => Some((Operator::Multiply, 2)),
                Token::KeywordDividedBy => Some((Operator::Divide, 2)),
                Token::Percent => Some((Operator::Modulo, 2)),
                Token::KeywordDivided => {
                    // Check if next token is "by" more efficiently
                    if self.peek_divided_by() {
                        Some((Operator::Divide, 2))
                    } else {
                        return Err(ParseError::new(
                            "Expected 'by' after 'divided'".to_string(),
                            line,
                            column,
                        ));
                    }
                }
                Token::Equals => Some((Operator::Equals, 0)),
                Token::KeywordIs => {
                    self.bump_sync(); // Consume "is"

                    if let Some(next_token) = self.cursor.peek().cloned() {
                        match &next_token.token {
                            Token::KeywordEqual => {
                                self.bump_sync(); // Consume "equal"

                                if let Some(to_token) = self.cursor.peek().cloned() {
                                    if matches!(to_token.token, Token::KeywordTo) {
                                        self.bump_sync(); // Consume "to"
                                        Some((Operator::Equals, 0))
                                    } else {
                                        Some((Operator::Equals, 0)) // "is equal" without "to" is valid too
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
                                self.bump_sync(); // Consume "not"
                                Some((Operator::NotEquals, 0))
                            }
                            Token::KeywordGreater => {
                                self.bump_sync(); // Consume "greater"

                                if let Some(than_token) = self.cursor.peek().cloned() {
                                    if matches!(than_token.token, Token::KeywordThan) {
                                        self.bump_sync(); // Consume "than"

                                        // Check for "or equal to" after "greater than"
                                        if let Some(or_token) = self.cursor.peek().cloned() {
                                            if matches!(or_token.token, Token::KeywordOr) {
                                                self.bump_sync(); // Consume "or"
                                                if let Some(equal_token) =
                                                    self.cursor.peek().cloned()
                                                {
                                                    if matches!(
                                                        equal_token.token,
                                                        Token::KeywordEqual
                                                    ) {
                                                        self.bump_sync(); // Consume "equal"
                                                        // Optional "to"
                                                        if let Some(to_token) =
                                                            self.cursor.peek().cloned()
                                                        {
                                                            if matches!(
                                                                to_token.token,
                                                                Token::KeywordTo
                                                            ) {
                                                                self.bump_sync(); // Consume "to"
                                                                Some((
                                                                    Operator::GreaterThanOrEqual,
                                                                    0,
                                                                ))
                                                            } else {
                                                                Some((
                                                                    Operator::GreaterThanOrEqual,
                                                                    0,
                                                                )) // "or equal" without "to" is valid too
                                                            }
                                                        } else {
                                                            Some((Operator::GreaterThanOrEqual, 0)) // "or equal" without "to" is valid too
                                                        }
                                                    } else {
                                                        Some((Operator::GreaterThan, 0)) // Just "greater than or" without "equal" is treated as "greater than"
                                                    }
                                                } else {
                                                    Some((Operator::GreaterThan, 0)) // Just "greater than or" without "equal" is treated as "greater than"
                                                }
                                            } else {
                                                Some((Operator::GreaterThan, 0)) // Just "greater than" without "or"
                                            }
                                        } else {
                                            Some((Operator::GreaterThan, 0)) // Just "greater than" without "or"
                                        }
                                    } else {
                                        Some((Operator::GreaterThan, 0)) // "is greater" without "than" is valid too
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
                                self.bump_sync(); // Consume "less"

                                if let Some(than_token) = self.cursor.peek().cloned() {
                                    if matches!(than_token.token, Token::KeywordThan) {
                                        self.bump_sync(); // Consume "than"

                                        // Check for "or equal to" after "less than"
                                        if let Some(or_token) = self.cursor.peek().cloned() {
                                            if matches!(or_token.token, Token::KeywordOr) {
                                                self.bump_sync(); // Consume "or"

                                                if let Some(equal_token) =
                                                    self.cursor.peek().cloned()
                                                {
                                                    if matches!(
                                                        equal_token.token,
                                                        Token::KeywordEqual
                                                    ) {
                                                        self.bump_sync(); // Consume "equal"

                                                        if let Some(to_token) =
                                                            self.cursor.peek().cloned()
                                                        {
                                                            if matches!(
                                                                to_token.token,
                                                                Token::KeywordTo
                                                            ) {
                                                                self.bump_sync(); // Consume "to"
                                                                Some((Operator::LessThanOrEqual, 0))
                                                            } else {
                                                                Some((Operator::LessThanOrEqual, 0)) // "or equal" without "to" is valid too
                                                            }
                                                        } else {
                                                            Some((Operator::LessThanOrEqual, 0)) // "or equal" without "to" is valid too
                                                        }
                                                    } else {
                                                        Some((Operator::LessThan, 0)) // Just "less than or" without "equal" is treated as "less than"
                                                    }
                                                } else {
                                                    Some((Operator::LessThan, 0)) // Just "less than or" without "equal" is treated as "less than"
                                                }
                                            } else {
                                                Some((Operator::LessThan, 0)) // Just "less than" without "or equal to"
                                            }
                                        } else {
                                            Some((Operator::LessThan, 0)) // Just "less than" without "or equal to"
                                        }
                                    } else {
                                        Some((Operator::LessThan, 0)) // "is less" without "than" is valid too
                                    }
                                } else {
                                    return Err(ParseError::new(
                                        "Unexpected end of input after 'is less'".into(),
                                        line,
                                        column,
                                    ));
                                }
                            }
                            _ => Some((Operator::Equals, 0)), // Simple "is" means equals
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
                    // With the introduction of 'call' keyword, we still support
                    // legacy syntax for builtin functions: `builtinName with args`
                    // For user-defined actions, require `call actionName with args`
                    if let Expression::Variable(ref name, var_line, var_column) = left {
                        // Check if this is a builtin function
                        if crate::builtins::is_builtin_function(name) {
                            // Builtin function - keep legacy syntax
                            self.bump_sync(); // Consume "with"
                            let arguments = self.parse_argument_list()?;

                            left = Expression::ActionCall {
                                name: name.clone(),
                                arguments,
                                line: var_line,
                                column: var_column,
                            };
                            continue;
                        }
                    }

                    // For all other cases (including user-defined actions),
                    // treat 'with' as concatenation
                    self.bump_sync(); // Consume "with"
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
                    // DON'T consume here - let precedence check happen first
                    // Token will be consumed in the block after precedence check
                    Some((Operator::And, 0))
                }
                Token::KeywordOr => {
                    self.bump_sync(); // Consume "or"

                    // Handle "or equal to" as a special case
                    if let Some(equal_token) = self.cursor.peek().cloned()
                        && matches!(equal_token.token, Token::KeywordEqual)
                    {
                        self.bump_sync(); // Consume "equal"

                        if let Some(to_token) = self.cursor.peek().cloned()
                            && matches!(to_token.token, Token::KeywordTo)
                        {
                            self.bump_sync(); // Consume "to"

                            if let Expression::BinaryOperation {
                                operator,
                                left: left_expr,
                                right: right_expr,
                                line: op_line,
                                column: op_column,
                            } = &left
                            {
                                if operator == &Operator::LessThan {
                                    left = Expression::BinaryOperation {
                                        left: left_expr.clone(),
                                        operator: Operator::LessThanOrEqual,
                                        right: right_expr.clone(),
                                        line: *op_line,
                                        column: *op_column,
                                    };
                                    continue;
                                } else if operator == &Operator::GreaterThan {
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
                    self.bump_sync(); // Consume "matches"

                    // Check if next token is "pattern" keyword (optional)
                    if let Some(pattern_token) = self.cursor.peek().cloned()
                        && matches!(pattern_token.token, Token::KeywordPattern)
                    {
                        self.bump_sync(); // Consume "pattern"
                    }

                    let pattern_expr = self.parse_binary_expression(precedence + 1)?;

                    left = Expression::PatternMatch {
                        text: Box::new(left),
                        pattern: Box::new(pattern_expr),
                        line,
                        column,
                    };
                    continue; // Skip the rest of the loop since we've already updated left
                }
                Token::KeywordFind => {
                    self.bump_sync(); // Consume "find"

                    // Check if next token is "pattern" keyword (optional)
                    if let Some(pattern_token) = self.cursor.peek().cloned()
                        && matches!(pattern_token.token, Token::KeywordPattern)
                    {
                        self.bump_sync(); // Consume "pattern"
                    }

                    let pattern_expr = self.parse_binary_expression(precedence + 1)?;

                    if let Some(in_token) = self.cursor.peek().cloned()
                        && matches!(in_token.token, Token::KeywordIn)
                    {
                        self.bump_sync(); // Consume "in"

                        let text_expr = self.parse_binary_expression(precedence + 1)?;

                        left = Expression::PatternFind {
                            text: Box::new(text_expr),
                            pattern: Box::new(pattern_expr),
                            line,
                            column,
                        };
                        continue; // Skip the rest of the loop since we've already updated left
                    }

                    left = Expression::PatternFind {
                        text: Box::new(left),
                        pattern: Box::new(pattern_expr),
                        line,
                        column,
                    };
                    continue; // Skip the rest of the loop since we've already updated left
                }
                Token::KeywordReplace => {
                    self.bump_sync(); // Consume "replace"

                    // Check if next token is "pattern" keyword (optional)
                    if let Some(pattern_token) = self.cursor.peek().cloned()
                        && matches!(pattern_token.token, Token::KeywordPattern)
                    {
                        self.bump_sync(); // Consume "pattern"
                    }

                    let pattern_expr = self.parse_binary_expression(precedence + 1)?;

                    if let Some(with_token) = self.cursor.peek().cloned()
                        && matches!(with_token.token, Token::KeywordWith)
                    {
                        self.bump_sync(); // Consume "with"

                        let replacement_expr = self.parse_binary_expression(precedence + 1)?;

                        if let Some(in_token) = self.cursor.peek().cloned()
                            && matches!(in_token.token, Token::KeywordIn)
                        {
                            self.bump_sync(); // Consume "in"

                            let text_expr = self.parse_binary_expression(precedence + 1)?;

                            left = Expression::PatternReplace {
                                text: Box::new(text_expr),
                                pattern: Box::new(pattern_expr),
                                replacement: Box::new(replacement_expr),
                                line,
                                column,
                            };
                            continue; // Skip the rest of the loop since we've already updated left
                        }

                        left = Expression::PatternReplace {
                            text: Box::new(left),
                            pattern: Box::new(pattern_expr),
                            replacement: Box::new(replacement_expr),
                            line,
                            column,
                        };
                        continue; // Skip the rest of the loop since we've already updated left
                    }

                    return Err(ParseError::new(
                        "Expected 'with' after pattern in replace operation".to_string(),
                        line,
                        column,
                    ));
                }
                Token::KeywordSplit => {
                    self.bump_sync(); // Consume "split"

                    // Parse the text expression to split
                    let text_expr = self.parse_binary_expression(precedence + 1)?;

                    // Check for "by" (string split) or "on" (pattern split)
                    if let Some(next_token) = self.cursor.peek().cloned() {
                        match next_token.token {
                            Token::KeywordBy => {
                                // Handle "split text by delimiter" syntax
                                self.bump_sync(); // Consume "by"
                                let delimiter_expr =
                                    self.parse_binary_expression(precedence + 1)?;

                                left = Expression::StringSplit {
                                    text: Box::new(text_expr),
                                    delimiter: Box::new(delimiter_expr),
                                    line,
                                    column,
                                };
                                continue;
                            }
                            Token::KeywordOn => {
                                // Handle "split text on pattern name" syntax
                                self.bump_sync(); // Consume "on"

                                // Check if next token is "pattern" keyword (optional)
                                if let Some(pattern_token) = self.cursor.peek().cloned()
                                    && matches!(pattern_token.token, Token::KeywordPattern)
                                {
                                    self.bump_sync(); // Consume "pattern"
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
                            _ => {
                                return Err(ParseError::new(
                                    "Expected 'by' or 'on' after text in split operation"
                                        .to_string(),
                                    line,
                                    column,
                                ));
                            }
                        }
                    } else {
                        return Err(ParseError::new(
                            "Expected 'by' or 'on' after text in split operation".to_string(),
                            line,
                            column,
                        ));
                    }
                }
                Token::KeywordContains => {
                    self.bump_sync(); // Consume "contains"

                    if let Some(pattern_token) = self.cursor.peek().cloned()
                        && matches!(pattern_token.token, Token::KeywordPattern)
                    {
                        self.bump_sync(); // Consume "pattern"

                        let pattern_expr = self.parse_binary_expression(precedence + 1)?;

                        left = Expression::PatternMatch {
                            text: Box::new(left),
                            pattern: Box::new(pattern_expr),
                            line,
                            column,
                        };
                        continue; // Skip the rest of the loop since we've already updated left
                    }

                    Some((Operator::Contains, 0))
                }
                Token::Colon => {
                    self.bump_sync(); // Consume ":"
                    continue;
                }
                _ => None,
            };

            if let Some((operator, op_precedence)) = op {
                if op_precedence < precedence {
                    break;
                }

                // Now consume the operator token(s) since the precedence check passed
                match token {
                    Token::Plus => {
                        self.bump_sync(); // Consume "+"
                    }
                    Token::KeywordPlus => {
                        self.bump_sync(); // Consume "plus"
                    }
                    Token::KeywordMinus => {
                        self.bump_sync(); // Consume "minus"
                    }
                    Token::Minus => {
                        self.bump_sync(); // Consume "-"
                    }
                    Token::KeywordTimes => {
                        self.bump_sync(); // Consume "times"
                    }
                    Token::KeywordDividedBy => {
                        self.bump_sync(); // Consume "divided by"
                    }
                    Token::KeywordDivided => {
                        self.bump_sync(); // Consume "divided"
                        self.expect_token(Token::KeywordBy, "Expected 'by' after 'divided'")?;
                        self.bump_sync(); // Consume "by"
                    }
                    Token::Percent => {
                        self.bump_sync(); // Consume "%"
                    }
                    Token::Equals => {
                        self.bump_sync(); // Consume "="
                    }
                    Token::KeywordAnd => {
                        self.bump_sync(); // Consume "and"
                    }
                    _ => {
                        // For operators like "is" that have already consumed tokens in their detection
                        // No additional consumption needed
                    }
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

    fn parse_call_expression(
        &mut self,
        call_line: usize,
        call_column: usize,
    ) -> Result<Expression, ParseError> {
        // We've already consumed Token::KeywordCall in the caller

        // Expect identifier for action name
        let name = if let Some(token_pos) = self.cursor.peek().cloned() {
            if let Token::Identifier(id) = &token_pos.token {
                let name = id.clone();
                self.bump_sync(); // Consume identifier
                name
            } else {
                let error = ParseError::from_token(
                    "Expected action name after 'call'".to_string(),
                    &token_pos,
                );
                self.errors.push(error.clone());
                return Err(error);
            }
        } else {
            let error = ParseError::new(
                "Expected action name after 'call'".to_string(),
                call_line,
                call_column,
            );
            self.errors.push(error.clone());
            return Err(error);
        };

        // Check for 'with' keyword (optional for zero-argument calls)
        if let Some(token_pos) = self.cursor.peek() {
            if matches!(token_pos.token, Token::KeywordWith) {
                self.bump_sync(); // Consume 'with'
            } else {
                // 'with' is optional if action takes no arguments
                // Return zero-argument action call
                return Ok(Expression::ActionCall {
                    name,
                    arguments: vec![],
                    line: call_line,
                    column: call_column,
                });
            }
        } else {
            // End of input after action name - zero arguments
            return Ok(Expression::ActionCall {
                name,
                arguments: vec![],
                line: call_line,
                column: call_column,
            });
        }

        // Parse argument list
        let arguments = self.parse_argument_list()?;

        Ok(Expression::ActionCall {
            name,
            arguments,
            line: call_line,
            column: call_column,
        })
    }

    fn parse_argument_list(&mut self) -> Result<Vec<Argument>, ParseError> {
        let mut arguments = Vec::with_capacity(4);

        let start_pos = self.cursor.pos();

        loop {
            // Check for named arguments (name: value)
            let arg_name = if let Some(name_token) = self.cursor.peek().cloned() {
                if let Token::Identifier(id) = &name_token.token {
                    // Check if next token is colon (named argument syntax)
                    if self
                        .cursor
                        .peek_next()
                        .is_some_and(|t| matches!(t.token, Token::Colon))
                    {
                        self.bump_sync(); // Consume name
                        self.bump_sync(); // Consume ":"
                        Some(id.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };

            // FIX: Parse expressions with precedence >= 1 (arithmetic operators)
            // This stops at 'and' (precedence 0), which is then used as argument separator
            let arg_value = self.parse_binary_expression(1)?;

            arguments.push(Argument {
                name: arg_name,
                value: arg_value,
            });

            if let Some(token) = self.cursor.peek().cloned() {
                if matches!(token.token, Token::KeywordAnd) {
                    self.bump_sync(); // Consume "and"
                    continue; // Continue parsing next argument
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        assert!(
            self.cursor.pos() > start_pos,
            "Parser made no progress while parsing argument list at line {}",
            self.cursor.current_line()
        );

        Ok(arguments)
    }
}
