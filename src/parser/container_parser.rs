use crate::lexer::token::Token;
use crate::parser::error::ParseError;

use super::Parser;

impl<'a> Parser<'a> {
    pub(crate) fn parse_inheritance2(
        &mut self,
    ) -> Result<(Option<String>, Vec<String>), ParseError> {
        let mut extends = None;
        let mut implements = Vec::new();

        if let Some(token) = self.tokens.peek()
            && token.token == Token::KeywordExtends
        {
            self.tokens.next();

            if let Some(token) = self.tokens.peek() {
                if let Token::Identifier(id) = &token.token {
                    extends = Some(id.clone());
                    self.tokens.next();
                } else {
                    return Err(ParseError::new(
                        "Expected identifier after 'extends'".to_string(),
                        token.line,
                        token.column,
                    ));
                }
            } else {
                return Err(ParseError::new(
                    "Expected identifier after 'extends'".to_string(),
                    0,
                    0,
                ));
            }
        }

        if let Some(token) = self.tokens.peek()
            && token.token == Token::KeywordImplements
        {
            self.tokens.next();

            loop {
                if let Some(token) = self.tokens.peek() {
                    if let Token::Identifier(id) = &token.token {
                        implements.push(id.clone());
                        self.tokens.next();

                        if let Some(next_token) = self.tokens.peek() {
                            if next_token.token == Token::Comma {
                                self.tokens.next();
                                continue;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    } else {
                        return Err(ParseError::new(
                            "Expected identifier in implements list".to_string(),
                            token.line,
                            token.column,
                        ));
                    }
                } else {
                    return Err(ParseError::new(
                        "Expected identifier in implements list".to_string(),
                        0,
                        0,
                    ));
                }
            }
        }

        Ok((extends, implements))
    }
}
