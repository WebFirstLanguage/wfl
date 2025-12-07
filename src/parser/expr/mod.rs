//! Expression parsing module
//!
//! This module organizes expression parsing into logical sub-modules:
//! - `binary`: Binary expressions with operator precedence
//! - `primary`: Primary/atomic expressions (literals, variables, etc.)

mod binary;
mod primary;

pub(crate) use binary::BinaryExprParser;
pub(crate) use primary::PrimaryExprParser;

use super::{Expression, ParseError, Parser};

/// Main trait for expression parsing
///
/// This trait combines primary and binary expression parsing capabilities.
pub(crate) trait ExprParser<'a>: PrimaryExprParser<'a> + BinaryExprParser<'a> {
    /// Parses an expression, starting with the lowest precedence.
    ///
    /// Returns the parsed expression or a parse error if the expression is invalid.
    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_binary_expression(0) // Start with lowest precedence
    }
}

// Blanket implementation: any type that implements both PrimaryExprParser and BinaryExprParser
// automatically implements ExprParser
impl<'a> ExprParser<'a> for Parser<'a> {}
