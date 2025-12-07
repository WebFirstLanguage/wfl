//! File I/O and filesystem statement parsing

use super::super::{Parser, ParseError, Statement};
use crate::parser::expr::ExprParser;

pub(crate) trait IoParser<'a>: ExprParser<'a> {
    fn parse_display_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_open_file_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_open_file_read_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_close_file_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_write_to_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_create_file_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_create_directory_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_delete_statement(&mut self) -> Result<Statement, ParseError>;
}

impl<'a> IoParser<'a> for Parser<'a> {
    fn parse_display_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_open_file_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_open_file_read_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_close_file_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_write_to_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_create_file_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_create_directory_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_delete_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }
}
