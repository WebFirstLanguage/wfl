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

    #[allow(dead_code)]
    pub fn synchronize(&mut self) {
        while let Some(token) = self.tokens.peek().cloned() {
            match &token.token {
                Token::KeywordStore
                | Token::KeywordCreate
                | Token::KeywordDisplay
                | Token::KeywordCheck
                | Token::KeywordCount
                | Token::KeywordFor
                | Token::KeywordDefine
                | Token::KeywordIf
                | Token::KeywordPush => {
                    break;
                }
                Token::KeywordEnd => {
                    self.tokens.next();
                    if let Some(next_token) = self.tokens.peek() {
                        match &next_token.token {
                            Token::KeywordAction
                            | Token::KeywordCheck
                            | Token::KeywordFor
                            | Token::KeywordCount
                            | Token::KeywordRepeat
                            | Token::KeywordTry
                            | Token::KeywordLoop
                            | Token::KeywordWhile => {
                                self.tokens.next();
                            }
                            _ => {}
                        }
                    }
                    break;
                }
                _ => {
                    self.tokens.next();
                }
            }
        }
    }
}
