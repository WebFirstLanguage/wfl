//! Process spawning and management statement parsing

use super::super::{Parser, ParseError, Statement};
use crate::parser::expr::ExprParser;

pub(crate) trait ProcessParser<'a>: ExprParser<'a> {
    fn parse_execute_command_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_spawn_process_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_kill_process_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_read_process_output_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_wait_for_statement(&mut self) -> Result<Statement, ParseError>;
}

impl<'a> ProcessParser<'a> for Parser<'a> {
    fn parse_execute_command_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_spawn_process_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_kill_process_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_read_process_output_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_wait_for_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }
}
