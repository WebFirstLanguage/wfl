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
            Token::KeywordOutput => "output".to_string(),
            Token::KeywordBinary => "binary".to_string(),
            Token::KeywordBytes => "bytes".to_string(),
            Token::KeywordThat => "that".to_string(),
            Token::KeywordRoute => "route".to_string(),
            Token::KeywordWhen => "when".to_string(),
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
                | Token::KeywordRoute
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
                | Token::KeywordInclude
                | Token::KeywordExport
                | Token::KeywordGive
                | Token::KeywordReturn
        )
    }

    /// Returns `true` if `token` can begin a fresh standalone value in a
    /// `display` list.
    ///
    /// Used by `display` to fold multiple space-separated values into a single
    /// concatenation: quoted text is a string literal, everything else is a
    /// variable/expression. Only tokens that start a fresh value trigger the
    /// fold — statement boundaries (`Eol`) and binary operators do not — so
    /// `display numbers 0` stays a direct index access (the `0` is absorbed by
    /// the preceding expression) and `display x\n0` keeps the `0` as its own
    /// statement across the line break.
    ///
    /// This list tracks the keyword-led arms of `parse_primary_expression`'s
    /// top-level match (`src/parser/expr/primary.rs`) that produce a
    /// standalone value with no ambiguity against statement boundaries: `not`,
    /// `pattern`, `output`, `file`, `directory`, `process`, `header`, `list`,
    /// and `read`, alongside the pre-existing `call`/`count`/`current`. Each is
    /// exercised end-to-end (parse *and* fold) by a
    /// `test_display_folds_keyword_*` test in `parser/tests.rs`, so a
    /// regression here — the token stops folding, or silently changes what it
    /// parses to — fails loudly instead of drifting unnoticed.
    ///
    /// Not every keyword-led primary-expression arm is included. Several are
    /// deliberately left out because they double as statement/block openers
    /// elsewhere in the grammar (`loop`, `exit`, `repeat`, `try`, `when` — the
    /// first four are literal entries in `is_statement_starter` above, and
    /// `when` opens route-arm blocks); folding them into a preceding `display`
    /// on parse would swallow what is far more likely to be a genuine syntax
    /// error (a missing line break) than an intended display value, so they
    /// are left to surface that error instead of being silently absorbed.
    /// `back` and `error` were also left out of this pass — they are
    /// unambiguous by the same reasoning as `not`/`pattern`, but weren't
    /// flagged by review, so they're a candidate for a follow-up rather than
    /// bundled in here without an explicit ask.
    ///
    /// Deliberately excluded:
    /// - `with`, `find`, `replace`, `split`, `matches`, `contains`,
    ///   `starts`/`ends with`, `is`, `and`, `or`, and arithmetic keywords —
    ///   `parse_binary_expression` already consumes these as continuations of
    ///   the *first* value inside the initial `parse_expression()` call above,
    ///   so they never reach this check as the head of a fresh value. (`find`,
    ///   `replace`, and `split` have a separate, pre-existing bug where that
    ///   continuation silently discards the value that precedes them — e.g.
    ///   `display "parts: " split "a,b" by ","` prints only the split result —
    ///   but that lives in the general binary-expression grammar, not here; see
    ///   the Dev Diary entry for this feature for the follow-up scope note.)
    /// - `not` and unary `-` (`Minus`) as *binary*-operator continuations don't
    ///   apply here, but `Minus` specifically can never be the head of a fresh
    ///   display value either way: `parse_binary_expression` has no precedence
    ///   guard on subtraction, so a leading `-` after the first value is always
    ///   consumed as arithmetic *inside* that first `parse_expression()` call
    ///   (`display "n: " -5` parses as `"n: " minus 5`, not as two values) —
    ///   adding it here would be dead code. `not` has no such conflict (it is
    ///   never a binary continuation), so it is included below.
    /// - `LeftBracket` (`[`) — postfix indexing (`Token::LeftBracket` in
    ///   `parse_primary_expression`'s postfix loop) unconditionally attaches a
    ///   following `[...]` to whatever expression was just parsed, so a `[` can
    ///   never be the head of a *fresh* value here either; it is always already
    ///   consumed as an index into the previous value.
    pub(crate) fn is_value_start(token: &Token) -> bool {
        matches!(
            token,
            Token::StringLiteral(_)
                | Token::IntLiteral(_)
                | Token::FloatLiteral(_)
                | Token::BooleanLiteral(_)
                | Token::NothingLiteral
                | Token::Identifier(_)
                | Token::LeftParen
                | Token::KeywordCall
                | Token::KeywordCount
                | Token::KeywordCurrent
                | Token::KeywordNot
                | Token::KeywordPattern
                | Token::KeywordOutput
                | Token::KeywordFile
                | Token::KeywordDirectory
                | Token::KeywordProcess
                | Token::KeywordHeader
                | Token::KeywordList
                | Token::KeywordRead
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
                    #[allow(unused_variables)]
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

    /// Consumes an optional leading `of` keyword if present.
    ///
    /// Used by the `split` handlers to accept both `split X by DELIM` and the
    /// documented `split of X by DELIM` spelling from a single place.
    pub(crate) fn consume_optional_of(&mut self) {
        if let Some(of_token) = self.cursor.peek()
            && matches!(&of_token.token, Token::KeywordOf)
        {
            self.bump_sync(); // Consume "of"
        }
    }
}
