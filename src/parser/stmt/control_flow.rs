//! Control flow statement parsing (if, for, loops, repeat)

use super::super::{Parser, ParseError, Statement};
use super::StmtParser;
use crate::parser::expr::ExprParser;

pub(crate) trait ControlFlowParser<'a>: ExprParser<'a> {
    fn parse_if_statement(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;

    fn parse_single_line_if(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;

    fn parse_for_each_loop(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;

    fn parse_count_loop(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;

    fn parse_main_loop(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;

    fn parse_repeat_statement(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;
}

impl<'a> ControlFlowParser<'a> for Parser<'a> {
    fn parse_if_statement(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        todo!("To be extracted from mod.rs")
    }

    fn parse_single_line_if(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        todo!("To be extracted from mod.rs")
    }

    fn parse_for_each_loop(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        todo!("To be extracted from mod.rs")
    }

    fn parse_count_loop(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        todo!("To be extracted from mod.rs")
    }

    fn parse_main_loop(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        todo!("To be extracted from mod.rs")
    }

    fn parse_repeat_statement(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        todo!("To be extracted from mod.rs")
    }
}
