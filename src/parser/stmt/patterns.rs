//! Pattern matching and pattern definition statement parsing

use super::super::{Parser, ParseError, Statement, Expression, PatternExpression};
use crate::lexer::token::TokenWithPosition;
use crate::parser::expr::ExprParser;

pub(crate) trait PatternParser<'a>: ExprParser<'a> {
    fn parse_create_pattern_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_pattern_tokens(tokens: &[TokenWithPosition]) -> Result<PatternExpression, ParseError>;
    fn parse_pattern_sequence(
        tokens: &[TokenWithPosition],
        pos: &mut usize,
    ) -> Result<PatternExpression, ParseError>;
    fn parse_pattern_concatenation(
        tokens: &[TokenWithPosition],
        pos: &mut usize,
    ) -> Result<PatternExpression, ParseError>;
    fn parse_pattern_element(
        tokens: &[TokenWithPosition],
        pos: &mut usize,
    ) -> Result<PatternExpression, ParseError>;
    fn parse_quantifier(
        tokens: &[TokenWithPosition],
        pos: &mut usize,
    ) -> Result<(usize, Option<usize>), ParseError>;
    fn parse_extension_filter(&mut self) -> Result<Vec<Expression>, ParseError>;
}

impl<'a> PatternParser<'a> for Parser<'a> {
    fn parse_create_pattern_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_pattern_tokens(tokens: &[TokenWithPosition]) -> Result<PatternExpression, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_pattern_sequence(
        tokens: &[TokenWithPosition],
        pos: &mut usize,
    ) -> Result<PatternExpression, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_pattern_concatenation(
        tokens: &[TokenWithPosition],
        pos: &mut usize,
    ) -> Result<PatternExpression, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_pattern_element(
        tokens: &[TokenWithPosition],
        pos: &mut usize,
    ) -> Result<PatternExpression, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_quantifier(
        tokens: &[TokenWithPosition],
        pos: &mut usize,
    ) -> Result<(usize, Option<usize>), ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_extension_filter(&mut self) -> Result<Vec<Expression>, ParseError> {
        todo!("To be extracted from mod.rs")
    }
}
