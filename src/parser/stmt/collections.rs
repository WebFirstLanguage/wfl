//! Collection and data structure statement parsing

use super::super::{Parser, ParseError, Statement};
use crate::parser::expr::ExprParser;

pub(crate) trait CollectionParser<'a>: ExprParser<'a> {
    fn parse_create_list_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_push_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_add_operation(&mut self) -> Result<Statement, ParseError>;
    fn parse_remove_from_list_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_clear_list_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_map_creation(&mut self) -> Result<Statement, ParseError>;
    fn parse_create_date_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_create_time_statement(&mut self) -> Result<Statement, ParseError>;
}

impl<'a> CollectionParser<'a> for Parser<'a> {
    fn parse_create_list_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_push_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_add_operation(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_remove_from_list_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_clear_list_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_map_creation(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_create_date_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_create_time_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }
}
