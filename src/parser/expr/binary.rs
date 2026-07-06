//! Binary expression parsing with operator precedence
//!
//! This module handles parsing of binary operations, including arithmetic, logical,
//! comparison, pattern matching, and custom language constructs.

use super::super::{Argument, Expression, Operator, ParseError, Parser};
use super::{ExprParser, PrimaryExprParser};
use crate::diagnostics::Span;
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

    /// Parses a single argument of an `of`-call (e.g. `fibonacci of n minus 1`).
    ///
    /// Unlike the full binary-expression parser, this absorbs *only* arithmetic
    /// (`plus`/`minus`/`times`/`divided by`/`/`/`%`/`modulo`), so that a call
    /// argument binds the surrounding arithmetic — `fib of n minus 1` means
    /// `fib of (n minus 1)`. Crucially it stops at `and` (the argument
    /// separator), `with` (concatenation), `from`/`by`/`length` (stdlib call
    /// separators), comparisons, and pattern keywords, leaving those for the
    /// caller so multi-argument and postfix forms keep working.
    fn parse_of_call_argument(&mut self) -> Result<Expression, ParseError>;

    /// Multiplicative level of an `of`-call argument: times / divided by / `/`
    /// / `%` / modulo (precedence 3).
    fn parse_of_call_arg_term(&mut self) -> Result<Expression, ParseError>;
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

            // Precedence ladder (higher binds tighter):
            //   0: and, or
            //   1: comparisons (is, equals, greater/less than, contains)
            //   2: plus, minus
            //   3: times, divided by, modulo
            let op = match token {
                Token::Plus => Some((Operator::Plus, 2)),
                Token::KeywordPlus => Some((Operator::Plus, 2)),
                Token::Minus => Some((Operator::Minus, 2)),
                Token::KeywordMinus => Some((Operator::Minus, 2)),
                Token::KeywordTimes => Some((Operator::Multiply, 3)),
                Token::KeywordDividedBy => Some((Operator::Divide, 3)),
                Token::Slash => Some((Operator::Divide, 3)),
                Token::Percent => Some((Operator::Modulo, 3)),
                Token::KeywordModulo => Some((Operator::Modulo, 3)),
                Token::KeywordDivided => {
                    // Check if next token is "by" more efficiently
                    if self.peek_divided_by() {
                        Some((Operator::Divide, 3))
                    } else {
                        return Err(ParseError::from_span(
                            "Expected 'by' after 'divided'".to_string(),
                            Span { start: 0, end: 0 },
                            line,
                            column,
                        ));
                    }
                }
                Token::Equals => Some((Operator::Equals, 1)),
                Token::KeywordIs => {
                    // Comparisons live at precedence 1. When we are parsing the
                    // right-hand side of a tighter operator (arithmetic, prec 2/3),
                    // we must stop *before* consuming any comparison tokens so the
                    // comparison binds around the whole arithmetic result. Without
                    // this guard `y plus 1 is equal to 4` eagerly eats "is equal to"
                    // while parsing the RHS of `plus`, corrupting the parse.
                    if 1 < precedence {
                        break;
                    }

                    self.bump_sync(); // Consume "is"

                    if let Some(next_token) = self.cursor.peek() {
                        match &next_token.token {
                            // "is between A and B" — inclusive range check that
                            // desugars to `X >= A and X <= B`.
                            Token::KeywordBetween => {
                                self.bump_sync(); // Consume "between"
                                let lower = self.parse_binary_expression(2)?;
                                self.expect_token(
                                    Token::KeywordAnd,
                                    "Expected 'and' between the bounds of 'is between'",
                                )?;
                                let upper = self.parse_binary_expression(2)?;

                                let lower_bound = Expression::BinaryOperation {
                                    left: Box::new(left.clone()),
                                    operator: Operator::GreaterThanOrEqual,
                                    right: Box::new(lower),
                                    line,
                                    column,
                                };
                                let upper_bound = Expression::BinaryOperation {
                                    left: Box::new(left),
                                    operator: Operator::LessThanOrEqual,
                                    right: Box::new(upper),
                                    line,
                                    column,
                                };
                                left = Expression::BinaryOperation {
                                    left: Box::new(lower_bound),
                                    operator: Operator::And,
                                    right: Box::new(upper_bound),
                                    line,
                                    column,
                                };
                                continue;
                            }
                            // "is above N" / "is below N" — natural synonyms for
                            // greater-than / less-than.
                            Token::KeywordAbove => {
                                self.bump_sync(); // Consume "above"
                                Some((Operator::GreaterThan, 1))
                            }
                            Token::KeywordBelow => {
                                self.bump_sync(); // Consume "below"
                                Some((Operator::LessThan, 1))
                            }
                            Token::KeywordEqual => {
                                self.bump_sync(); // Consume "equal"

                                if let Some(to_token) = self.cursor.peek() {
                                    if matches!(&to_token.token, Token::KeywordTo) {
                                        self.bump_sync(); // Consume "to"
                                        Some((Operator::Equals, 1))
                                    } else {
                                        Some((Operator::Equals, 1)) // "is equal" without "to" is valid too
                                    }
                                } else {
                                    return Err(ParseError::from_span(
                                        "Unexpected end of input after 'is equal'".into(),
                                        Span { start: 0, end: 0 },
                                        line,
                                        column,
                                    ));
                                }
                            }
                            Token::KeywordNot => {
                                self.bump_sync(); // Consume "not"

                                // Support the natural "is not equal to" form by
                                // consuming an optional "equal" and an optional "to"
                                // (mirroring the "is equal to" branch above).
                                if let Some(equal_token) = self.cursor.peek()
                                    && matches!(&equal_token.token, Token::KeywordEqual)
                                {
                                    self.bump_sync(); // Consume "equal"
                                    if let Some(to_token) = self.cursor.peek()
                                        && matches!(&to_token.token, Token::KeywordTo)
                                    {
                                        self.bump_sync(); // Consume "to"
                                    }
                                }
                                Some((Operator::NotEquals, 1))
                            }
                            Token::KeywordGreater => {
                                self.bump_sync(); // Consume "greater"

                                if let Some(than_token) = self.cursor.peek() {
                                    if matches!(&than_token.token, Token::KeywordThan) {
                                        self.bump_sync(); // Consume "than"

                                        // Check for "or equal to" after "greater than"
                                        if let Some(or_token) = self.cursor.peek() {
                                            if matches!(&or_token.token, Token::KeywordOr) {
                                                self.bump_sync(); // Consume "or"
                                                if let Some(equal_token) = self.cursor.peek() {
                                                    if matches!(
                                                        equal_token.token,
                                                        Token::KeywordEqual
                                                    ) {
                                                        self.bump_sync(); // Consume "equal"
                                                        // Optional "to"
                                                        if let Some(to_token) = self.cursor.peek() {
                                                            if matches!(
                                                                to_token.token,
                                                                Token::KeywordTo
                                                            ) {
                                                                self.bump_sync(); // Consume "to"
                                                                Some((
                                                                    Operator::GreaterThanOrEqual,
                                                                    1,
                                                                ))
                                                            } else {
                                                                Some((
                                                                    Operator::GreaterThanOrEqual,
                                                                    1,
                                                                )) // "or equal" without "to" is valid too
                                                            }
                                                        } else {
                                                            Some((Operator::GreaterThanOrEqual, 1)) // "or equal" without "to" is valid too
                                                        }
                                                    } else {
                                                        Some((Operator::GreaterThan, 1)) // Just "greater than or" without "equal" is treated as "greater than"
                                                    }
                                                } else {
                                                    Some((Operator::GreaterThan, 1)) // Just "greater than or" without "equal" is treated as "greater than"
                                                }
                                            } else {
                                                Some((Operator::GreaterThan, 1)) // Just "greater than" without "or"
                                            }
                                        } else {
                                            Some((Operator::GreaterThan, 1)) // Just "greater than" without "or"
                                        }
                                    } else {
                                        Some((Operator::GreaterThan, 1)) // "is greater" without "than" is valid too
                                    }
                                } else {
                                    return Err(ParseError::from_span(
                                        "Unexpected end of input after 'is greater'".into(),
                                        Span { start: 0, end: 0 },
                                        line,
                                        column,
                                    ));
                                }
                            }
                            Token::KeywordLess => {
                                self.bump_sync(); // Consume "less"

                                if let Some(than_token) = self.cursor.peek() {
                                    if matches!(&than_token.token, Token::KeywordThan) {
                                        self.bump_sync(); // Consume "than"

                                        // Check for "or equal to" after "less than"
                                        if let Some(or_token) = self.cursor.peek() {
                                            if matches!(&or_token.token, Token::KeywordOr) {
                                                self.bump_sync(); // Consume "or"

                                                if let Some(equal_token) = self.cursor.peek() {
                                                    if matches!(
                                                        equal_token.token,
                                                        Token::KeywordEqual
                                                    ) {
                                                        self.bump_sync(); // Consume "equal"

                                                        if let Some(to_token) = self.cursor.peek() {
                                                            if matches!(
                                                                to_token.token,
                                                                Token::KeywordTo
                                                            ) {
                                                                self.bump_sync(); // Consume "to"
                                                                Some((Operator::LessThanOrEqual, 1))
                                                            } else {
                                                                Some((Operator::LessThanOrEqual, 1)) // "or equal" without "to" is valid too
                                                            }
                                                        } else {
                                                            Some((Operator::LessThanOrEqual, 1)) // "or equal" without "to" is valid too
                                                        }
                                                    } else {
                                                        Some((Operator::LessThan, 1)) // Just "less than or" without "equal" is treated as "less than"
                                                    }
                                                } else {
                                                    Some((Operator::LessThan, 1)) // Just "less than or" without "equal" is treated as "less than"
                                                }
                                            } else {
                                                Some((Operator::LessThan, 1)) // Just "less than" without "or equal to"
                                            }
                                        } else {
                                            Some((Operator::LessThan, 1)) // Just "less than" without "or equal to"
                                        }
                                    } else {
                                        Some((Operator::LessThan, 1)) // "is less" without "than" is valid too
                                    }
                                } else {
                                    return Err(ParseError::from_span(
                                        "Unexpected end of input after 'is less'".into(),
                                        Span { start: 0, end: 0 },
                                        line,
                                        column,
                                    ));
                                }
                            }
                            _ => Some((Operator::Equals, 1)), // Simple "is" means equals
                        }
                    } else {
                        return Err(ParseError::from_span(
                            "Unexpected end of input after 'is'".into(),
                            Span { start: 0, end: 0 },
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
                        // Check if this is a builtin function. `count` is
                        // excluded: it is the implicit count-loop variable and
                        // the documented idiom `display "..." with count with
                        // "..."` is concatenation, not a call to the list
                        // builtin (use `count of <list> and <value>` for that).
                        if name != "count" && crate::builtins::is_builtin_function(name) {
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
                    // Logical 'or' lives at precedence 0; do not consume it while
                    // parsing a tighter sub-expression.
                    if 0 < precedence {
                        break;
                    }
                    self.bump_sync(); // Consume "or"

                    // Handle "or equal to" as a special case
                    if let Some(equal_token) = self.cursor.peek()
                        && matches!(&equal_token.token, Token::KeywordEqual)
                    {
                        self.bump_sync(); // Consume "equal"

                        if let Some(to_token) = self.cursor.peek()
                            && matches!(&to_token.token, Token::KeywordTo)
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
                    if let Some(pattern_token) = self.cursor.peek()
                        && matches!(&pattern_token.token, Token::KeywordPattern)
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
                    if let Some(pattern_token) = self.cursor.peek()
                        && matches!(&pattern_token.token, Token::KeywordPattern)
                    {
                        self.bump_sync(); // Consume "pattern"
                    }

                    let pattern_expr = self.parse_binary_expression(precedence + 1)?;

                    if let Some(in_token) = self.cursor.peek()
                        && matches!(&in_token.token, Token::KeywordIn)
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
                    if let Some(pattern_token) = self.cursor.peek()
                        && matches!(&pattern_token.token, Token::KeywordPattern)
                    {
                        self.bump_sync(); // Consume "pattern"
                    }

                    let pattern_expr = self.parse_binary_expression(precedence + 1)?;

                    if let Some(with_token) = self.cursor.peek()
                        && matches!(&with_token.token, Token::KeywordWith)
                    {
                        self.bump_sync(); // Consume "with"

                        let replacement_expr = self.parse_binary_expression(precedence + 1)?;

                        if let Some(in_token) = self.cursor.peek()
                            && matches!(&in_token.token, Token::KeywordIn)
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

                    return Err(ParseError::from_span(
                        "Expected 'with' after pattern in replace operation".to_string(),
                        Span { start: 0, end: 0 },
                        line,
                        column,
                    ));
                }
                Token::KeywordSplit => {
                    self.bump_sync(); // Consume "split"

                    // Accept the documented `split of X by DELIM` form by
                    // optionally consuming "of" (equivalent to `split X by DELIM`).
                    self.consume_optional_of();

                    // Parse the text expression to split
                    let text_expr = self.parse_binary_expression(precedence + 1)?;

                    // Check for "by" (string split) or "on" (pattern split)
                    if let Some(next_token) = self.cursor.peek() {
                        match &next_token.token {
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
                                if let Some(pattern_token) = self.cursor.peek()
                                    && matches!(&pattern_token.token, Token::KeywordPattern)
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
                                return Err(ParseError::from_span(
                                    "Expected 'by' or 'on' after text in split operation"
                                        .to_string(),
                                    Span { start: 0, end: 0 },
                                    line,
                                    column,
                                ));
                            }
                        }
                    } else {
                        return Err(ParseError::from_span(
                            "Expected 'by' or 'on' after text in split operation".to_string(),
                            Span { start: 0, end: 0 },
                            line,
                            column,
                        ));
                    }
                }
                Token::KeywordStarts | Token::KeywordEnds => {
                    // `X starts with Y` / `X ends with Y` are substring predicates
                    // at comparison precedence (1). They desugar to the
                    // `starts_with` / `ends_with` builtins so no new interpreter
                    // or type-checker operator is needed (#566). Before this,
                    // `starts`/`ends` had no token and the lexer's multi-word
                    // identifier accumulator swallowed `path ends` into one name.
                    if 1 < precedence {
                        break;
                    }
                    let is_starts = matches!(token, Token::KeywordStarts);
                    // Only an operator when directly followed by `with`; otherwise
                    // it is a plain (contextual) identifier — leave it in place.
                    if !self
                        .cursor
                        .peek_next()
                        .is_some_and(|t| t.token == Token::KeywordWith)
                    {
                        break;
                    }
                    self.bump_sync(); // Consume "starts"/"ends"
                    self.bump_sync(); // Consume "with"
                    // RHS binds at precedence 2 (tighter than comparison), matching
                    // how `contains`/`is` parse their right-hand side.
                    let right = self.parse_binary_expression(2)?;
                    let fn_name = if is_starts {
                        "starts_with"
                    } else {
                        "ends_with"
                    };
                    left = Expression::FunctionCall {
                        function: Box::new(Expression::Variable(fn_name.to_string(), line, column)),
                        arguments: vec![
                            Argument {
                                name: None,
                                value: left,
                            },
                            Argument {
                                name: None,
                                value: right,
                            },
                        ],
                        line,
                        column,
                    };
                    continue;
                }
                Token::KeywordContains => {
                    // 'contains' is a comparison operator at precedence 1.
                    if 1 < precedence {
                        break;
                    }
                    self.bump_sync(); // Consume "contains"

                    if let Some(pattern_token) = self.cursor.peek()
                        && matches!(&pattern_token.token, Token::KeywordPattern)
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

                    Some((Operator::Contains, 1))
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
                    }
                    Token::Slash => {
                        self.bump_sync(); // Consume "/"
                    }
                    Token::Percent => {
                        self.bump_sync(); // Consume "%"
                    }
                    Token::KeywordModulo => {
                        self.bump_sync(); // Consume "modulo"
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
        let name = if let Some(token_pos) = self.cursor.peek() {
            if let Token::Identifier(id) = &token_pos.token {
                let name = id.clone();
                self.bump_sync(); // Consume identifier
                name
            } else {
                let error = ParseError::from_token(
                    "Expected action name after 'call'".to_string(),
                    token_pos,
                );
                self.errors.push(error.clone());
                return Err(error);
            }
        } else {
            let error = ParseError::from_span(
                "Expected action name after 'call'".to_string(),
                crate::diagnostics::Span { start: 0, end: 0 },
                call_line,
                call_column,
            );
            self.errors.push(error.clone());
            return Err(error);
        };

        // Check for 'with' keyword (optional for zero-argument calls)
        if let Some(token_pos) = self.cursor.peek() {
            if matches!(&token_pos.token, Token::KeywordWith) {
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

    fn parse_of_call_argument(&mut self) -> Result<Expression, ParseError> {
        // Additive level: plus / minus (precedence 2).
        let mut left = self.parse_of_call_arg_term()?;

        while let Some(token_pos) = self.cursor.peek() {
            let (operator, line, column) = match &token_pos.token {
                Token::Plus | Token::KeywordPlus => {
                    (Operator::Plus, token_pos.line, token_pos.column)
                }
                Token::Minus | Token::KeywordMinus => {
                    (Operator::Minus, token_pos.line, token_pos.column)
                }
                _ => break,
            };
            self.bump_sync(); // Consume the additive operator
            let right = self.parse_of_call_arg_term()?;
            left = Expression::BinaryOperation {
                left: Box::new(left),
                operator,
                right: Box::new(right),
                line,
                column,
            };
        }

        Ok(left)
    }

    fn parse_of_call_arg_term(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_primary_expression()?;

        while let Some(token_pos) = self.cursor.peek() {
            let line = token_pos.line;
            let column = token_pos.column;
            let operator = match &token_pos.token {
                Token::KeywordTimes => Operator::Multiply,
                Token::KeywordDividedBy | Token::Slash => Operator::Divide,
                Token::Percent | Token::KeywordModulo => Operator::Modulo,
                Token::KeywordDivided => {
                    if self.peek_divided_by() {
                        Operator::Divide
                    } else {
                        break;
                    }
                }
                _ => break,
            };

            match &token_pos.token {
                Token::KeywordDivided => {
                    self.bump_sync(); // Consume "divided"
                    self.expect_token(Token::KeywordBy, "Expected 'by' after 'divided'")?;
                }
                _ => {
                    self.bump_sync(); // Consume the multiplicative operator
                }
            }

            let right = self.parse_primary_expression()?;
            left = Expression::BinaryOperation {
                left: Box::new(left),
                operator,
                right: Box::new(right),
                line,
                column,
            };
        }

        Ok(left)
    }

    fn parse_argument_list(&mut self) -> Result<Vec<Argument>, ParseError> {
        let mut arguments = Vec::with_capacity(4);

        let start_pos = self.cursor.pos();

        loop {
            // Check for named arguments (name: value)
            let arg_name = if let Some(name_token) = self.cursor.peek() {
                if let Token::Identifier(id) = &name_token.token {
                    // Check if next token is colon (named argument syntax)
                    if self
                        .cursor
                        .peek_next()
                        .is_some_and(|t| matches!(&t.token, Token::Colon))
                    {
                        let name = id.to_string();
                        self.bump_sync(); // Consume name
                        self.bump_sync(); // Consume ":"
                        Some(name)
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

            if let Some(token) = self.cursor.peek() {
                if matches!(&token.token, Token::KeywordAnd) {
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
