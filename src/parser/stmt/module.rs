//! Module import and loading statement parsing

use super::super::{ParseError, Parser, Statement};
use crate::lexer::token::Token;
use crate::parser::expr::ExprParser;

pub(crate) trait ModuleParser<'a>: ExprParser<'a> {
    fn parse_load_module_statement(&mut self) -> Result<Statement, ParseError>;
}

impl<'a> ModuleParser<'a> for Parser<'a> {
    fn parse_load_module_statement(&mut self) -> Result<Statement, ParseError> {
<<<<<<< HEAD
        // Safely consume "load" token and capture its position
        let load_token = if let Some(token) = self.cursor.peek() {
            let token = token.clone();
            self.bump_sync();
            token
        } else {
            return Err(self
                .cursor
                .error("Expected 'load' keyword but found end of input".to_string()));
=======
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
>>>>>>> origin/includes
        };

        self.expect_token(Token::KeywordModule, "Expected 'module' after 'load'")?;
        self.expect_token(Token::KeywordFrom, "Expected 'from' after 'module'")?;

        let path = self.parse_expression()?;

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
