//! Action definition and call statement parsing

use super::super::{Parser, ParseError, Statement, Parameter};
use super::StmtParser;
use crate::parser::expr::ExprParser;

pub(crate) trait ActionParser<'a>: ExprParser<'a> {
    fn parse_action_definition(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;

    fn parse_container_action_definition(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>;

    fn parse_parameter_list(&mut self) -> Result<Vec<Parameter>, ParseError>;
    fn parse_return_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_exit_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_parent_method_call(&mut self) -> Result<Statement, ParseError>;
}

impl<'a> ActionParser<'a> for Parser<'a> {
    fn parse_action_definition(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        todo!("To be extracted from mod.rs")
    }

    fn parse_container_action_definition(&mut self) -> Result<Statement, ParseError>
    where
        Self: StmtParser<'a>,
    {
        todo!("To be extracted from mod.rs")
    }

    fn parse_parameter_list(&mut self) -> Result<Vec<Parameter>, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_return_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_exit_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_parent_method_call(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }
}
