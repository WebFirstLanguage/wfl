pub mod ast;
mod container_parser;
pub mod error;
mod expressions;
mod pattern_parser;
mod statements;
#[cfg(test)]
mod tests;
mod util;

use crate::lexer::token::{Token, TokenWithPosition};
use ast::*;
use error::ParseError;
use std::collections::HashSet;
use std::iter::Peekable;
use std::slice::Iter;

pub struct Parser<'a> {
    tokens: Peekable<Iter<'a, TokenWithPosition>>,
    errors: Vec<ParseError>,
    known_actions: HashSet<String>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [TokenWithPosition]) -> Self {
        Parser {
            tokens: tokens.iter().peekable(),
            errors: Vec::with_capacity(4),
            known_actions: HashSet::new(),
        }
    }

    pub fn parse(&mut self) -> Result<Program, Vec<ParseError>> {
        let mut program = Program::new();
        program.statements.reserve(self.tokens.clone().count() / 5);

        while self.tokens.peek().is_some() {
            let start_len = self.tokens.clone().count();

            match self.parse_statement() {
                Ok(stmt) => program.statements.push(stmt),
                Err(err) => {
                    self.errors.push(err);
                    let current_line = if let Some(tok) = self.tokens.peek() {
                        tok.line
                    } else {
                        0
                    };
                    while let Some(tok) = self.tokens.peek() {
                        if tok.line > current_line || Parser::is_statement_starter(&tok.token) {
                            break;
                        }
                        if matches!(tok.token, Token::KeywordEnd) {
                            let mut la = self.tokens.clone();
                            let _ = la.next();
                            if let Some(next_tok) = la.peek() {
                                match next_tok.token {
                                    Token::KeywordAction
                                    | Token::KeywordCheck
                                    | Token::KeywordFor
                                    | Token::KeywordCount
                                    | Token::KeywordRepeat
                                    | Token::KeywordTry
                                    | Token::KeywordLoop
                                    | Token::KeywordMap
                                    | Token::KeywordWhile
                                    | Token::KeywordPattern => {
                                        self.tokens.next();
                                        self.tokens.next();
                                        continue;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        self.tokens.next();
                    }
                }
            }

            let end_len = self.tokens.clone().count();
            if let Some(tok) = self.tokens.peek()
                && tok.token == Token::KeywordEnd
                && start_len <= 2
            {
                while self.tokens.next().is_some() {}
                break;
            }

            assert!(
                end_len < start_len,
                "Parser made no progress - token {:?} caused infinite loop",
                self.tokens.peek()
            );
        }

        if self.errors.is_empty() {
            Ok(program)
        } else {
            Err(self.errors.clone())
        }
    }

    fn is_statement_starter(token: &Token) -> bool {
        matches!(
            token,
            Token::KeywordStore
                | Token::KeywordCreate
                | Token::KeywordDisplay
                | Token::KeywordCheck
                | Token::KeywordIf
                | Token::KeywordCount
                | Token::KeywordFor
                | Token::KeywordDefine
                | Token::KeywordChange
                | Token::KeywordTry
                | Token::KeywordRepeat
                | Token::KeywordExit
                | Token::KeywordPush
                | Token::KeywordBreak
                | Token::KeywordContinue
                | Token::KeywordSkip
                | Token::KeywordOpen
                | Token::KeywordClose
                | Token::KeywordWait
                | Token::KeywordGive
                | Token::KeywordReturn
        )
    }
}
