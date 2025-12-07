//! Statement parsing module
//!
//! This module organizes statement parsing into logical sub-modules by functionality.
//! Each module defines a trait with parsing methods for a specific category of statements,
//! and the main StmtParser trait combines all of them to provide the complete statement
//! parsing interface.

mod actions;
mod collections;
mod containers;
mod control_flow;
mod errors;
mod io;
mod patterns;
mod processes;
mod variables;
mod web;

pub(crate) use actions::ActionParser;
pub(crate) use collections::CollectionParser;
pub(crate) use containers::ContainerParser;
pub(crate) use control_flow::ControlFlowParser;
pub(crate) use errors::ErrorHandlingParser;
pub(crate) use io::IoParser;
pub(crate) use patterns::PatternParser;
pub(crate) use processes::ProcessParser;
pub(crate) use variables::VariableParser;
pub(crate) use web::WebParser;

use super::{ParseError, Statement};
use crate::lexer::token::Token;

/// Main trait for statement parsing
///
/// This trait combines all statement parsing capabilities and provides the central
/// parse_statement() dispatcher that routes to appropriate sub-parsers based on
/// the current token.
///
/// The implementations of parse_statement() and parse_expression_statement() are
/// provided in the Parser impl block in mod.rs, as they need direct access to the cursor.
pub(crate) trait StmtParser<'a>:
    VariableParser<'a>
    + CollectionParser<'a>
    + IoParser<'a>
    + ProcessParser<'a>
    + WebParser<'a>
    + ActionParser<'a>
    + ErrorHandlingParser<'a>
    + ControlFlowParser<'a>
    + PatternParser<'a>
    + ContainerParser<'a>
{
    /// Parses a statement by dispatching to the appropriate parser based on the current token.
    ///
    /// This is the main entry point for statement parsing. It examines the current token
    /// and routes to the specialized parser for that statement type.
    fn parse_statement(&mut self) -> Result<Statement, ParseError>;

    /// Parses an expression as a statement.
    ///
    /// This is used as a fallback when no specific statement keyword is matched,
    /// allowing bare expressions to be treated as statements.
    fn parse_expression_statement(&mut self) -> Result<Statement, ParseError>;
}
