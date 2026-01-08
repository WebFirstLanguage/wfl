//! Module and import statement parsing

use super::super::{Expression, Literal, ParseError, Parser, Statement};
use crate::lexer::token::Token;
use crate::parser::expr::{ExprParser, PrimaryExprParser};

pub(crate) trait ModuleParser<'a>: ExprParser<'a> {
    fn parse_load_statement(&mut self) -> Result<Statement, ParseError>;
}

impl<'a> ModuleParser<'a> for Parser<'a> {
    fn parse_load_statement(&mut self) -> Result<Statement, ParseError> {
        let load_token = self.bump_sync().unwrap(); // Consume "load"
        let line = load_token.line;
        let column = load_token.column;

        // Check for "load module from" or simplified "load" syntax
        let path = if let Some(tok) = self.cursor.peek() {
            if tok.token == Token::KeywordModule {
                // "load module from 'path'" syntax
                self.bump_sync(); // Consume "module"

                // Expect "from"
                if let Some(from_tok) = self.cursor.peek() {
                    if from_tok.token == Token::KeywordFrom {
                        self.bump_sync(); // Consume "from"
                    } else {
                        return Err(ParseError::from_token(
                            format!("Expected 'from' after 'module', found {:?}", from_tok.token),
                            from_tok,
                        ));
                    }
                } else {
                    return Err(ParseError::from_token(
                        "Expected 'from' after 'module'".to_string(),
                        &load_token,
                    ));
                }

                // Parse the path expression (should be a string literal)
                self.parse_primary_expression()?
            } else {
                // Simplified "load 'path'" syntax
                self.parse_primary_expression()?
            }
        } else {
            return Err(ParseError::from_token(
                "Expected path after 'load'".to_string(),
                &load_token,
            ));
        };

        // Validate that path is a string literal
        match &path {
            Expression::Literal(Literal::String(_), _, _) => {
                Ok(Statement::ImportStatement { path, line, column })
            }
            _ => Err(ParseError::from_token(
                "Import path must be a string literal".to_string(),
                &load_token,
            )),
        }
    }
}
