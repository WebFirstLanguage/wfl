//! `route` statement parsing — a natural-language "dispatch on a value" form.
//!
//! `route` is **pure syntactic sugar**. It lowers, entirely in the parser, to the
//! same `check if / otherwise check if / otherwise` chain (`Statement::IfStatement`)
//! that a hand-written dispatch would produce. Because no new AST node reaches the
//! analyzer, type checker, or interpreter, every existing WFL program is unaffected
//! and the feature needs no runtime support.
//!
//! ```text
//! route <subject>:
//!     when P1: B1
//!     when P2: B2
//!     otherwise: Bd
//! end route
//! ```
//!
//! lowers to
//!
//! ```text
//! check if <subject matches P1>:
//!     B1
//! otherwise check if <subject matches P2>:
//!     B2
//! otherwise:
//!     Bd
//! end check
//! ```
//!
//! The `<subject matches Pn>` condition is built from the pattern head:
//!
//! | Pattern head            | Condition                                   |
//! |-------------------------|---------------------------------------------|
//! | `when V`                | `subject is equal to V`                     |
//! | `when V1 or V2`         | `subject == V1 or subject == V2`            |
//! | `when contains V`       | `contains of subject and V`                 |
//! | `when one of L`         | `contains of L and subject` (membership)    |
//! | `when starts with V`    | `starts_with of subject and V`              |
//! | `when ends with V`      | `ends_with of subject and V`                |

use super::super::{ParseError, Parser, Statement};
use super::StmtParser;
use crate::lexer::token::Token;
use crate::parser::ast::{Argument, Expression, Literal, Operator};
use crate::parser::expr::{ExprParser, PrimaryExprParser};

pub(crate) trait RouteParser<'a>: ExprParser<'a> {
    /// Parse a `route … end route` block and return its desugared
    /// `IfStatement` chain.
    fn parse_route(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;
}

/// Build a two-argument builtin call expression, e.g. `contains of a and b`.
fn builtin_call(
    name: &str,
    a: Expression,
    b: Expression,
    line: usize,
    column: usize,
) -> Expression {
    Expression::FunctionCall {
        function: Box::new(Expression::Variable(name.to_string(), line, column)),
        arguments: vec![
            Argument {
                name: None,
                value: a,
            },
            Argument {
                name: None,
                value: b,
            },
        ],
        line,
        column,
    }
}

impl<'a> RouteParser<'a> for Parser<'a> {
    fn parse_route(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        let route_token = self.bump_sync().unwrap(); // Consume "route"
        let line = route_token.line;
        let column = route_token.column;

        // The subject is any expression, up to the ':' that opens the body.
        let subject = self.parse_expression()?;

        if let Some(token) = self.cursor.peek()
            && matches!(&token.token, Token::Colon)
        {
            self.bump_sync(); // Consume the colon if present
        }
        self.skip_eol();

        let mut arms: Vec<(Expression, Vec<Statement>)> = Vec::new();
        let mut default_body: Option<Vec<Statement>> = None;

        loop {
            self.skip_eol();

            let Some(token) = self.cursor.peek() else {
                return Err(self.cursor.error(
                    "Expected 'when', 'otherwise', or 'end route' in route block".to_string(),
                ));
            };

            match &token.token {
                Token::KeywordWhen => {
                    if default_body.is_some() {
                        return Err(ParseError::from_token(
                            "'otherwise' must be the last arm in a route block".to_string(),
                            token,
                        ));
                    }

                    let when_token = self.bump_sync().unwrap(); // Consume "when"
                    let condition =
                        self.parse_route_pattern(&subject, when_token.line, when_token.column)?;

                    if let Some(t) = self.cursor.peek()
                        && matches!(&t.token, Token::Colon)
                    {
                        self.bump_sync(); // Consume the colon if present
                    }
                    self.skip_eol();

                    let body = self.parse_route_arm_body()?;
                    arms.push((condition, body));
                }
                Token::KeywordOtherwise => {
                    if default_body.is_some() {
                        return Err(ParseError::from_token(
                            "A route block may only have one 'otherwise' arm".to_string(),
                            token,
                        ));
                    }

                    self.bump_sync(); // Consume "otherwise"

                    if let Some(t) = self.cursor.peek()
                        && matches!(&t.token, Token::Colon)
                    {
                        self.bump_sync(); // Consume the colon if present
                    }
                    self.skip_eol();

                    default_body = Some(self.parse_route_arm_body()?);
                }
                Token::KeywordEnd => break,
                other => {
                    return Err(ParseError::from_token(
                        format!(
                            "Expected 'when', 'otherwise', or 'end route', found {:?}",
                            other
                        ),
                        token,
                    ));
                }
            }
        }

        self.expect_token(Token::KeywordEnd, "Expected 'end' to close route block")?;
        self.expect_token(Token::KeywordRoute, "Expected 'route' after 'end'")?;

        // Fold the arms into a right-nested if/else-if chain, with the optional
        // `otherwise` block as the innermost `else`.
        let had_arms = !arms.is_empty();
        let mut else_block: Option<Vec<Statement>> = default_body;

        for (condition, body) in arms.into_iter().rev() {
            let if_stmt = Statement::IfStatement {
                condition,
                then_block: body,
                else_block: else_block.take(),
                line,
                column,
            };
            else_block = Some(vec![if_stmt]);
        }

        if had_arms {
            // After folding, `else_block` holds exactly the outermost if.
            Ok(else_block
                .expect("chain with at least one arm is non-empty")
                .pop()
                .expect("folded chain contains the outermost if"))
        } else if let Some(body) = else_block {
            // A route with only an `otherwise`: run it unconditionally.
            Ok(Statement::IfStatement {
                condition: Expression::Literal(Literal::Boolean(true), line, column),
                then_block: body,
                else_block: None,
                line,
                column,
            })
        } else {
            // An empty route (no arms, no otherwise) is a no-op, matching the
            // semantics of a `check if` with no matching branch.
            Ok(Statement::IfStatement {
                condition: Expression::Literal(Literal::Boolean(false), line, column),
                then_block: Vec::new(),
                else_block: None,
                line,
                column,
            })
        }
    }
}

impl<'a> Parser<'a> {
    /// Parse the pattern that follows a `when` head and return the boolean
    /// condition it desugars to, comparing against `subject`.
    fn parse_route_pattern(
        &mut self,
        subject: &Expression,
        line: usize,
        column: usize,
    ) -> Result<Expression, ParseError> {
        let Some(token) = self.cursor.peek() else {
            return Err(self
                .cursor
                .error("Expected a pattern after 'when'".to_string()));
        };

        match &token.token {
            // `when contains V` → contains of subject and V (text substring / list membership)
            Token::KeywordContains => {
                self.bump_sync(); // Consume "contains"
                let value = self.parse_primary_expression()?;
                Ok(builtin_call(
                    "contains",
                    subject.clone(),
                    value,
                    line,
                    column,
                ))
            }
            // `when one of L` → contains of L and subject (membership test)
            Token::KeywordOne
                if self
                    .cursor
                    .peek_next()
                    .is_some_and(|t| t.token == Token::KeywordOf) =>
            {
                self.bump_sync(); // Consume "one"
                self.bump_sync(); // Consume "of"
                let list = self.parse_primary_expression()?;
                Ok(builtin_call(
                    "contains",
                    list,
                    subject.clone(),
                    line,
                    column,
                ))
            }
            // `when starts with V` → starts_with of subject and V
            // (`starts`/`ends` are now KeywordStarts/KeywordEnds tokens — #566.)
            Token::KeywordStarts
                if self
                    .cursor
                    .peek_next()
                    .is_some_and(|t| t.token == Token::KeywordWith) =>
            {
                self.bump_sync(); // Consume "starts"
                self.bump_sync(); // Consume "with"
                let value = self.parse_primary_expression()?;
                Ok(builtin_call(
                    "starts_with",
                    subject.clone(),
                    value,
                    line,
                    column,
                ))
            }
            // `when ends with V` → ends_with of subject and V
            Token::KeywordEnds
                if self
                    .cursor
                    .peek_next()
                    .is_some_and(|t| t.token == Token::KeywordWith) =>
            {
                self.bump_sync(); // Consume "ends"
                self.bump_sync(); // Consume "with"
                let value = self.parse_primary_expression()?;
                Ok(builtin_call(
                    "ends_with",
                    subject.clone(),
                    value,
                    line,
                    column,
                ))
            }
            // `when V` or `when V1 or V2 or …` → equality, or-chained
            _ => {
                let first = self.parse_primary_expression()?;
                let mut condition = Expression::BinaryOperation {
                    left: Box::new(subject.clone()),
                    operator: Operator::Equals,
                    right: Box::new(first),
                    line,
                    column,
                };

                while self
                    .cursor
                    .peek()
                    .is_some_and(|t| t.token == Token::KeywordOr)
                {
                    self.bump_sync(); // Consume "or"
                    let next = self.parse_primary_expression()?;
                    let equality = Expression::BinaryOperation {
                        left: Box::new(subject.clone()),
                        operator: Operator::Equals,
                        right: Box::new(next),
                        line,
                        column,
                    };
                    condition = Expression::BinaryOperation {
                        left: Box::new(condition),
                        operator: Operator::Or,
                        right: Box::new(equality),
                        line,
                        column,
                    };
                }

                Ok(condition)
            }
        }
    }

    /// Collect the statements that make up a `when`/`otherwise` arm body,
    /// stopping at the next arm, the `otherwise` arm, or `end route`.
    fn parse_route_arm_body(&mut self) -> Result<Vec<Statement>, ParseError> {
        let mut body = Vec::with_capacity(4);

        while let Some(token) = self.cursor.peek() {
            match &token.token {
                Token::KeywordWhen | Token::KeywordOtherwise | Token::KeywordEnd => break,
                Token::Eol => {
                    self.bump_sync(); // Skip Eol between statements
                }
                _ => body.push(self.parse_statement()?),
            }
        }

        Ok(body)
    }
}
