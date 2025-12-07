//! Container (OOP) statement parsing

use super::super::{Parser, ParseError, Statement};
use super::StmtParser;
use crate::parser::expr::ExprParser;

pub(crate) trait ContainerParser<'a>: ExprParser<'a> {
    fn parse_container_definition(&mut self) -> Result<Statement, ParseError>;
    fn parse_interface_definition(&mut self) -> Result<Statement, ParseError>;
    fn parse_container_instantiation(&mut self) -> Result<Statement, ParseError>;
    fn parse_event_definition(&mut self) -> Result<Statement, ParseError>;
    fn parse_event_trigger(&mut self) -> Result<Statement, ParseError>;
    fn parse_event_handler(&mut self) -> Result<Statement, ParseError>;

    fn parse_inheritance(&mut self) -> Result<Vec<String>, ParseError>;

    fn parse_container_body(&mut self) -> Result<Vec<Statement>, ParseError>
    where
        Self: StmtParser<'a>;

    fn parse_property_definition(&mut self) -> Result<Statement, ParseError>;
    fn parse_event_definition_full(&mut self) -> Result<Statement, ParseError>;

    fn parse_instantiation_body(&mut self) -> Result<Vec<Statement>, ParseError>
    where
        Self: StmtParser<'a>;
}

impl<'a> ContainerParser<'a> for Parser<'a> {
    fn parse_container_definition(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_interface_definition(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_container_instantiation(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_event_definition(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_event_trigger(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_event_handler(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_inheritance(&mut self) -> Result<Vec<String>, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_container_body(&mut self) -> Result<Vec<Statement>, ParseError>
    where
        Self: StmtParser<'a>,
    {
        todo!("To be extracted from mod.rs")
    }

    fn parse_property_definition(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_event_definition_full(&mut self) -> Result<Statement, ParseError> {
        todo!("To be extracted from mod.rs")
    }

    fn parse_instantiation_body(&mut self) -> Result<Vec<Statement>, ParseError>
    where
        Self: StmtParser<'a>,
    {
        todo!("To be extracted from mod.rs")
    }
}
