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

    /// Returns `true` if `token` has a dedicated arm in
    /// `parse_primary_expression` (`src/parser/expr/primary.rs`) that attempts
    /// to parse a standalone value, rather than falling through to that
    /// function's final `_ => Err("Unexpected token in expression")` arm.
    ///
    /// This is the single source of truth for "can this token begin a primary
    /// expression" — `is_value_start` below is defined in terms of it (primary
    /// starters minus an explicit, documented exclusion list) instead of
    /// maintaining its own independent token list, so the two can no longer
    /// silently drift apart as `parse_primary_expression` grows new arms.
    /// `parser/tests.rs` has a coupling test
    /// (`can_start_primary_expression_matches_parse_primary_expression`) that
    /// feeds a representative token of every `Token` variant through both
    /// this predicate and an actual `parse_primary_expression` call and
    /// asserts they agree — so an arm added to one without the other fails a
    /// test instead of quietly drifting.
    ///
    /// Mirrors, in match order: the literal/bracket/paren/identifier arms; the
    /// explicit keyword-led arms (`call`, `not`, `-` unary, `with`, `count`,
    /// `pattern`, `loop`, `output`, `repeat`, `exit`, `back`, `try`, `when`,
    /// `error`, `file`, `directory`, `process`, `header`, `current`, `list`,
    /// `read`, `find`, `replace`, `split`); and finally the contextual-keyword
    /// catch-all (`_ if token.is_contextual_keyword()`), which covers every
    /// other contextual keyword (e.g. `text`, `map`, `contains`, `create`,
    /// `new`, ...) as a bare variable reference. `Token::Eol` has its own arm
    /// but it always errors, so it is excluded here.
    pub(crate) fn can_start_primary_expression(token: &Token) -> bool {
        matches!(
            token,
            Token::LeftBracket
                | Token::LeftParen
                | Token::StringLiteral(_)
                | Token::IntLiteral(_)
                | Token::FloatLiteral(_)
                | Token::BooleanLiteral(_)
                | Token::NothingLiteral
                | Token::KeywordCall
                | Token::Identifier(_)
                | Token::KeywordNot
                | Token::Minus
                | Token::KeywordWith
                | Token::KeywordCount
                | Token::KeywordPattern
                | Token::KeywordLoop
                | Token::KeywordOutput
                | Token::KeywordRepeat
                | Token::KeywordExit
                | Token::KeywordBack
                | Token::KeywordTry
                | Token::KeywordWhen
                | Token::KeywordError
                | Token::KeywordFile
                | Token::KeywordDirectory
                | Token::KeywordProcess
                | Token::KeywordHeader
                | Token::KeywordCurrent
                | Token::KeywordList
                | Token::KeywordRead
                | Token::KeywordFind
                | Token::KeywordReplace
                | Token::KeywordSplit
        ) || token.is_contextual_keyword()
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
    /// Defined as `can_start_primary_expression` minus an explicit exclusion
    /// list, so it can never accept a token the expression parser itself
    /// rejects, and any *new* primary-expression starter is included by
    /// default rather than requiring a second, easy-to-forget edit — the
    /// opposite failure mode of the token list this replaced. Each included
    /// keyword starter is exercised end-to-end (parse *and* fold) by a
    /// `test_display_folds_keyword_*` test in `parser/tests.rs`.
    ///
    /// Excluded, with reasons:
    /// - `loop`, `exit`, `repeat`, `try`, `when` — these double as
    ///   statement/block openers elsewhere in the grammar (the first four are
    ///   literal entries in `is_statement_starter` above, and `when` opens
    ///   route-arm blocks); folding them into a preceding `display` on parse
    ///   would swallow what is far more likely to be a genuine syntax error (a
    ///   missing line break) than an intended display value, so they are left
    ///   to surface that error instead of being silently absorbed. `back` and
    ///   `error` have no such conflict — each is *only* ever a bare-variable
    ///   arm in `parse_primary_expression`, is not a dedicated arm of
    ///   `parse_statement`'s top-level dispatch (`src/parser/mod.rs`), and
    ///   (unlike `count`/`read`, see below) never leads a longer statement
    ///   form — so they are included.
    /// - `create`, `change`, `push`, `parent`, `skip`, `give` — each is
    ///   contextual (`Token::is_contextual_keyword`), so in expression
    ///   position it is just a bare-variable reference, but each is *also* a
    ///   dedicated arm of `parse_statement`'s top-level dispatch leading a
    ///   statement form with no expression-position equivalent: `create
    ///   container/list/new/pattern/directory/file/map/date/time/... as`,
    ///   `change X to Y` (assignment), `push with LIST and VALUE`, `parent
    ///   method(...)` (parent method call), and `give back EXPR` (return).
    ///   Centralizing on `can_start_primary_expression` surfaced these the
    ///   same way it surfaced `count`/`read`; unlike those two, none has a
    ///   single, unambiguous continuation token to guard on (`create` alone
    ///   forks more than half a dozen ways), so — same reasoning as
    ///   `loop`/`exit`/`repeat`/`try`/`when` — they are excluded outright
    ///   rather than guarded. `skip` is the sharpest case: as a bare
    ///   statement it's a *control-flow* effect (`continue`, one token,
    ///   see `Token::KeywordContinue | Token::KeywordSkip` in
    ///   `parse_statement`) with no visible syntax difference from folding it
    ///   as a value — both consume exactly one token — so an unguarded
    ///   inclusion would silently turn a loop's `continue` into inert display
    ///   output instead of a parse-time error. See the
    ///   `*_after_display_stays_a_separate_statement` tests in
    ///   `parser/tests.rs` for `create`, `change`, `push`, `parent`, `skip`,
    ///   and `give`.
    /// - `with`, `find`, `replace`, `split` — `parse_binary_expression`
    ///   already consumes these as continuations of the *first* value inside
    ///   the initial `parse_expression()` call above, so they never reach this
    ///   check as the head of a fresh value. (`find`, `replace`, and `split`
    ///   have a separate, pre-existing bug where that continuation silently
    ///   discards the value that precedes them — e.g.
    ///   `display "parts: " split "a,b" by ","` prints only the split result —
    ///   but that lives in the general binary-expression grammar, not here;
    ///   see the Dev Diary entry for this feature for the follow-up scope
    ///   note.)
    /// - Unary `-` (`Minus`) can never be the head of a fresh display value:
    ///   after an operand, `parse_binary_expression` consumes a `-` as the
    ///   binary subtraction operator, so a leading `-` after the first value is
    ///   always folded into that first `parse_expression()` call as arithmetic
    ///   (`display "n: " -5` parses as `"n: " minus 5`, not as two values) —
    ///   adding it here would be dead code. `not` has no such conflict (it is
    ///   never a binary continuation), so it is included.
    /// - `LeftBracket` (`[`) — postfix indexing (`Token::LeftBracket` in
    ///   `parse_primary_expression`'s postfix loop) unconditionally attaches a
    ///   following `[...]` to whatever expression was just parsed, so a `[` can
    ///   never be the head of a *fresh* value here either; it is always already
    ///   consumed as an index into the previous value.
    ///
    /// `count` and `read` are included (the count-loop variable, and
    /// `read content/binary/N bytes from ...`), but each also leads a longer,
    /// statement-only form on the same line — `count from ... to ...:` (a
    /// count loop) and `read output from process ...`
    /// (`parse_read_process_output_statement`) — that has no expression-position
    /// parse of its own. Folding either as a bare value would truncate the
    /// `display` at just the keyword and strand the rest as unparsable
    /// leftover tokens. `parse_display_statement`'s fold loop therefore pairs
    /// this predicate with `is_display_fold_statement_boundary`, which peeks
    /// one token further to keep those two forms as their own statement,
    /// exactly as they parsed before this feature existed.
    pub(crate) fn is_value_start(token: &Token) -> bool {
        Self::can_start_primary_expression(token)
            && !matches!(
                token,
                Token::KeywordLoop
                    | Token::KeywordExit
                    | Token::KeywordRepeat
                    | Token::KeywordTry
                    | Token::KeywordWhen
                    | Token::KeywordCreate
                    | Token::KeywordChange
                    | Token::KeywordPush
                    | Token::KeywordParent
                    | Token::KeywordSkip
                    | Token::KeywordGive
                    | Token::KeywordWith
                    | Token::KeywordFind
                    | Token::KeywordReplace
                    | Token::KeywordSplit
                    | Token::Minus
                    | Token::LeftBracket
            )
    }

    /// Returns `true` when the upcoming tokens begin a new statement rather
    /// than a fresh `display` value, even though the leading token alone
    /// passes `is_value_start`.
    ///
    /// `count` and `read` are each folded as expression starters (the
    /// count-loop variable, and `read content`/`read binary`/`read N bytes
    /// from`), but `count from ...` opens a count loop and
    /// `read output from process ...` is statement-only
    /// (`parse_read_process_output_statement`) — neither has an
    /// expression-position parse, so folding them would silently truncate the
    /// `display` at just the keyword and leave the rest of the statement
    /// dangling as unparsable leftover tokens. This lookahead preserves the
    /// pre-existing same-line behavior for both: `display "x" count from 1 to
    /// 3:` and `display "x" read output from process p` still end the
    /// `display` after `"x"` and parse the second statement normally.
    pub(crate) fn is_display_fold_statement_boundary(&self) -> bool {
        let Some(next) = self.cursor.peek() else {
            return false;
        };
        match &next.token {
            Token::KeywordCount => self
                .cursor
                .peek_next()
                .is_some_and(|t| t.token == Token::KeywordFrom),
            Token::KeywordRead => self
                .cursor
                .peek_next()
                .is_some_and(|t| t.token == Token::KeywordOutput),
            _ => false,
        }
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
