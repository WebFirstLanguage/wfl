//! Error handling statement parsing (try/when/otherwise)

use super::super::{Parser, ParseError, Statement};
use super::StmtParser;
use crate::parser::expr::ExprParser;

pub(crate) trait ErrorHandlingParser<'a>: ExprParser<'a> {
    fn parse_try_statement(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;
}

impl<'a> ErrorHandlingParser<'a> for Parser<'a> {
    fn parse_try_statement(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        todo!("To be extracted from mod.rs")
    }
}
