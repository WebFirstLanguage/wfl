//! Module import and loading statement parsing

use super::super::{ParseError, Parser, Statement};
use crate::lexer::token::Token;
use crate::parser::expr::{ExprParser, PrimaryExprParser};

pub(crate) trait ModuleParser<'a>: ExprParser<'a> {
    fn parse_load_module_statement(&mut self) -> Result<Statement, ParseError>;
}

impl<'a> ModuleParser<'a> for Parser<'a> {
    fn parse_load_module_statement(&mut self) -> Result<Statement, ParseError> {
        let load_token = match self.bump_sync() {
            Some(token) => token,
            None => {
                return Err(ParseError::from_span(
                    "Unexpected end of input while parsing load statement".to_string(),
                    self.cursor.current_span(),
                    self.cursor.current_line(),
                    self.cursor.peek().map_or(0, |t| t.column),
                ));
            }
        };

        self.expect_token(Token::KeywordModule, "Expected 'module' after 'load'")?;
        self.expect_token(Token::KeywordFrom, "Expected 'from' after 'module'")?;

        let path = self.parse_primary_expression()?;

        // V1: Reject unsupported alias syntax with helpful error
        if let Some(next_token) = self.cursor.peek()
            && matches!(next_token.token, Token::KeywordAs)
        {
            return Err(ParseError::from_token(
                "Module aliases are not yet supported. Use: load module from \"path\"".to_string(),
                next_token,
            ));
        }

        Ok(Statement::LoadModuleStatement {
            path,
            alias: None,
            line: load_token.line,
            column: load_token.column,
        })
    }
}
