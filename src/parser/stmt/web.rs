//! Web server statement parsing

use super::super::{Parser, ParseError, Statement};
use crate::parser::expr::ExprParser;

pub(crate) trait WebParser<'a>: ExprParser<'a> {
    fn parse_listen_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_respond_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_register_signal_handler_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_stop_accepting_connections_statement(&mut self) -> Result<Statement, ParseError>;
    fn parse_close_server_statement(&mut self) -> Result<Statement, ParseError>;
}

impl<'a> WebParser<'a> for Parser<'a> {
    fn parse_listen_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_respond_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_register_signal_handler_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_stop_accepting_connections_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_close_server_statement(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }
}
