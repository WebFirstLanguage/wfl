use crate::lexer::token::Token;
use crate::parser::error::ParseError;

use super::Parser;

impl<'a> Parser<'a> {
    pub fn expect_token(&mut self, expected: Token, error_message: &str) -> Result<(), ParseError> {
        if let Some(token) = self.tokens.peek().cloned() {
            if token.token == expected {
                self.tokens.next();
                Ok(())
            } else {
                Err(ParseError::new(
                    format!(
                        "{}: expected {:?}, found {:?}",
                        error_message, expected, token.token
                    ),
                    token.line,
                    token.column,
                ))
            }
        } else {
            Err(ParseError::new(
                format!("{error_message}: unexpected end of input"),
                0,
                0,
            ))
        }
    }
}
