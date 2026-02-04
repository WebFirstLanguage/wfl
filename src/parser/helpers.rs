//! Helper functions and utilities for the parser
//!
//! This module contains common helper functions used throughout the parser,
//! including token validation, error recovery, and pattern name checking.

use super::{ParseError, Parser};
use crate::exec_trace;
use crate::lexer::token::Token;

/// Checks if a pattern name conflicts with reserved keywords in WFL
pub(crate) fn is_reserved_pattern_name(name: &str) -> bool {
    matches!(
        name,
        "url"
            | "digit"
            | "letter"
            | "file"
            | "database"
            | "data"
            | "date"
            | "time"
            | "text"
            | "pattern"
            | "character"
            | "whitespace"
            | "unicode"
            | "category"
            | "script"
            | "greedy"
            | "lazy"
            | "zero"
            | "one"
            | "any"
            | "optional"
            | "between"
            | "start"
            | "ahead"
            | "behind"
            | "not"
            | "is"
            | "than"
            | "same"
            | "greater"
            | "less"
            | "equal"
            | "above"
            | "below"
            | "contains"
            | "matches"
            | "find"
            | "replace"
            | "split"
            | "capture"
            | "captured"
            | "more"
            | "exactly"
            | "push"
            | "add"
            | "subtract"
            | "multiply"
            | "divide"
            | "plus"
            | "minus"
            | "times"
            | "divided"
            | "by"
            | "open"
            | "close"
            | "read"
            | "write"
            | "append"
            | "content"
            | "wait"
            | "try"
            | "error"
            | "exists"
            | "list"
            | "map"
            | "remove"
            | "clear"
            | "files"
            | "found"
            | "permission"
            | "denied"
            | "recursively"
            | "extension"
            | "extensions"
            | "at"
            | "least"
            | "most"
            | "into"
            | "when"
            | "store"
            | "create"
            | "display"
            | "change"
            | "if"
            | "check"
            | "otherwise"
            | "then"
            | "end"
            | "as"
            | "to"
            | "from"
            | "with"
            | "and"
            | "or"
            | "count"
            | "for"
            | "each"
            | "in"
            | "reversed"
            | "repeat"
            | "while"
            | "until"
            | "forever"
            | "skip"
            | "continue"
            | "break"
            | "exit"
            | "loop"
            | "load"
            | "module"
            | "define"
            | "action"
            | "called"
            | "needs"
            | "give"
            | "back"
            | "return"
            | "directory"
            | "delete"
            | "container"
            | "property"
            | "extends"
            | "implements"
            | "interface"
            | "requires"
            | "event"
            | "trigger"
            | "on"
            | "static"
            | "public"
            | "private"
            | "parent"
            | "new"
            | "constant"
            | "must"
            | "defaults"
            | "of"
    )
}

impl<'a> Parser<'a> {
    /// Skip any Eol tokens at the current position
    /// Useful when parsing block bodies (if/check/loop) where Eol tokens separate statements
    pub(crate) fn skip_eol(&mut self) {
        while let Some(token) = self.cursor.peek() {
            if matches!(token.token, Token::Eol) {
                self.bump_sync();
            } else {
                break;
            }
        }
    }

    /// Helper method to get text representation of contextual keywords
    pub(crate) fn get_token_text(&self, token: &Token) -> String {
        match token {
            Token::KeywordCount => "count".to_string(),
            Token::KeywordPattern => "pattern".to_string(),
            Token::KeywordFiles => "files".to_string(),
            Token::KeywordExtension => "extension".to_string(),
            Token::KeywordExtensions => "extensions".to_string(),
            Token::KeywordContains => "contains".to_string(),
            Token::KeywordList => "list".to_string(),
            Token::KeywordMap => "map".to_string(),
            Token::KeywordText => "text".to_string(),
            Token::KeywordCreate => "create".to_string(),
            Token::KeywordNew => "new".to_string(),
            Token::KeywordParent => "parent".to_string(),
            Token::KeywordRead => "read".to_string(),
            Token::KeywordPush => "push".to_string(),
            Token::KeywordSkip => "skip".to_string(),
            Token::KeywordGive => "give".to_string(),
            Token::KeywordBack => "back".to_string(),
            Token::KeywordCalled => "called".to_string(),
            Token::KeywordNeeds => "needs".to_string(),
            Token::KeywordChange => "change".to_string(),
            Token::KeywordReversed => "reversed".to_string(),
            Token::KeywordAt => "at".to_string(),
            Token::KeywordLeast => "least".to_string(),
            Token::KeywordMost => "most".to_string(),
            Token::KeywordThan => "than".to_string(),
            Token::KeywordZero => "zero".to_string(),
            Token::KeywordAny => "any".to_string(),
            Token::KeywordMust => "must".to_string(),
            Token::KeywordDefaults => "defaults".to_string(),
            Token::KeywordHeaders => "headers".to_string(),
            _ => format!("{:?}", token),
        }
    }

    /// Checks if a token can start a statement
    pub(crate) fn is_statement_starter(token: &Token) -> bool {
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
                | Token::KeywordLoad
                | Token::KeywordGive
                | Token::KeywordReturn
        )
    }

    /// Synchronize parser state after an error by advancing to the next statement starter
    #[allow(dead_code)]
    pub(crate) fn synchronize(&mut self) {
        while let Some(token) = self.cursor.peek() {
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
                    // Handle orphaned "end" tokens during error recovery
                    let line = token.line;
                    exec_trace!("Synchronizing: found 'end' token at line {}", line);
                    self.bump_sync();
                    if let Some(next_token) = self.cursor.peek() {
                        match &next_token.token {
                            Token::KeywordAction
                            | Token::KeywordCheck
                            | Token::KeywordFor
                            | Token::KeywordCount
                            | Token::KeywordRepeat
                            | Token::KeywordTry
                            | Token::KeywordLoop
                            | Token::KeywordWhile => {
                                exec_trace!(
                                    "Synchronizing: consuming {:?} after 'end'",
                                    next_token.token
                                );
                                self.bump_sync();
                            }
                            _ => {} // Just consumed "end", continue
                        }
                    }
                    break; // After handling orphaned end, continue with recovery
                }
                _ => {
                    self.bump_sync();
                }
            }
        }
    }

    /// Expect a specific token and consume it, or return an error
    pub(crate) fn expect_token(
        &mut self,
        expected: Token,
        error_message: &str,
    ) -> Result<(), ParseError> {
        if let Some(token) = self.cursor.peek() {
            if token.token == expected {
                self.bump_sync();
                Ok(())
            } else {
                Err(ParseError::from_token(
                    format!(
                        "{}: expected {:?}, found {:?}",
                        error_message, expected, token.token
                    ),
                    token,
                ))
            }
        } else {
            Err(self
                .cursor
                .error(format!("{error_message}: unexpected end of input")))
        }
    }

    /// Consumes all tokens until "end pattern" is found to prevent cascading errors
    /// when a pattern definition fails early (e.g., due to reserved name)
    pub(crate) fn consume_pattern_body_on_error(&mut self) {
        // First, skip the colon if present
        if let Some(token) = self.cursor.peek()
            && token.token == Token::Colon
        {
            self.bump_sync();
        }

        let mut depth = 1; // We're inside one pattern block

        while let Some(token) = self.bump_sync() {
            match token.token {
                Token::KeywordEnd => {
                    // Check if this is "end pattern"
                    if let Some(next_token) = self.cursor.peek()
                        && next_token.token == Token::KeywordPattern
                    {
                        depth -= 1;
                        if depth == 0 {
                            self.bump_sync(); // Consume "pattern"
                            break;
                        }
                    }
                }
                Token::KeywordCreate => {
                    // Check if this is nested "create pattern"
                    if let Some(next_token) = self.cursor.peek()
                        && next_token.token == Token::KeywordPattern
                    {
                        depth += 1;
                    }
                }
                _ => {
                    // Continue consuming tokens
                }
            }
        }
    }

    /// Returns true if the token following the current position is the keyword "by".
    ///
    /// This method is typically used to detect the "divided by" operator sequence without advancing the main token iterator.
    pub(crate) fn peek_divided_by(&mut self) -> bool {
        // Look ahead one token to check for "by" keyword
        self.cursor
            .peek_next()
            .is_some_and(|tok| matches!(tok.token, Token::KeywordBy))
    }
}
