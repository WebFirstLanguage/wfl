//! Cursor-based token navigation for the WFL parser.
//!
//! This module provides an efficient, index-based abstraction for navigating
//! through tokens during parsing, replacing the previous `Peekable<Iter>` approach.
//!
//! # Key Features
//!
//! - **Efficient lookahead**: O(1) multi-token lookahead via indexed access
//! - **Zero cloning**: No iterator cloning for progress tracking or lookahead
//! - **Checkpointing**: Cheap position save/restore for backtracking
//! - **Clear API**: Explicit methods for common parsing patterns

use crate::lexer::token::{Token, TokenWithPosition};

/// Cursor for efficient token stream navigation.
///
/// Provides index-based access to a token slice, enabling efficient
/// lookahead, backtracking, and progress tracking without iterator cloning.
///
/// # Examples
///
/// ```
/// # use wfl::lexer::token::{Token, TokenWithPosition};
/// # use wfl::parser::cursor::Cursor;
/// let tokens = vec![
///     TokenWithPosition::new(Token::KeywordStore, 1, 1, 5),
///     TokenWithPosition::new(Token::Identifier("x".to_string()), 1, 7, 1),
/// ];
///
/// let mut cursor = Cursor::new(&tokens);
/// assert_eq!(cursor.peek().unwrap().token, Token::KeywordStore);
/// cursor.bump();
/// assert_eq!(cursor.peek().unwrap().token, Token::Identifier("x".to_string()));
/// ```
#[derive(Clone)]
pub struct Cursor<'a> {
    /// All tokens for the current parse unit
    tokens: &'a [TokenWithPosition],
    /// Current position in token stream (0-based index)
    pos: usize,
}

impl<'a> Cursor<'a> {
    /// Create a new cursor from a token slice.
    ///
    /// The cursor starts at position 0 (first token).
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::TokenWithPosition;
    /// let tokens: Vec<TokenWithPosition> = vec![];
    /// let cursor = Cursor::new(&tokens);
    /// assert_eq!(cursor.pos(), 0);
    /// ```
    pub fn new(tokens: &'a [TokenWithPosition]) -> Self {
        Self { tokens, pos: 0 }
    }

    /// Get current position in token stream.
    ///
    /// Returns the 0-based index of the next token to consume.
    /// This is useful for progress tracking and debugging.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::{Token, TokenWithPosition};
    /// # let tokens = vec![TokenWithPosition::new(Token::KeywordStore, 1, 1, 5)];
    /// let mut cursor = Cursor::new(&tokens);
    /// assert_eq!(cursor.pos(), 0);
    /// cursor.bump();
    /// assert_eq!(cursor.pos(), 1);
    /// ```
    #[inline]
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Check if at end of token stream.
    ///
    /// Returns `true` if no more tokens are available for consumption.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::TokenWithPosition;
    /// let tokens: Vec<TokenWithPosition> = vec![];
    /// let cursor = Cursor::new(&tokens);
    /// assert!(cursor.is_eof());
    /// ```
    #[inline]
    pub fn is_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    /// Peek at current token without consuming it.
    ///
    /// Returns a reference to the token at the current position,
    /// or `None` if at end of stream.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::{Token, TokenWithPosition};
    /// # let tokens = vec![TokenWithPosition::new(Token::KeywordStore, 1, 1, 5)];
    /// let cursor = Cursor::new(&tokens);
    /// let token = cursor.peek().unwrap();
    /// assert_eq!(token.token, Token::KeywordStore);
    /// assert_eq!(cursor.pos(), 0); // Position unchanged
    /// ```
    #[inline]
    pub fn peek(&self) -> Option<&'a TokenWithPosition> {
        self.tokens.get(self.pos)
    }

    /// Peek at token N positions ahead without consuming.
    ///
    /// Returns a reference to the token at position `pos + n`,
    /// or `None` if beyond end of stream.
    ///
    /// # Arguments
    ///
    /// * `n` - Positions ahead to look (0 = current, 1 = next, etc.)
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::{Token, TokenWithPosition};
    /// # let tokens = vec![
    /// #     TokenWithPosition::new(Token::KeywordStore, 1, 1, 5),
    /// #     TokenWithPosition::new(Token::Identifier("x".to_string()), 1, 7, 1),
    /// # ];
    /// let cursor = Cursor::new(&tokens);
    /// assert_eq!(cursor.peek_n(0).unwrap().token, Token::KeywordStore);
    /// assert_eq!(cursor.peek_n(1).unwrap().token, Token::Identifier("x".to_string()));
    /// assert!(cursor.peek_n(10).is_none());
    /// ```
    #[inline]
    pub fn peek_n(&self, n: usize) -> Option<&'a TokenWithPosition> {
        self.tokens.get(self.pos + n)
    }

    /// Peek at next token (position + 1).
    ///
    /// Convenience method equivalent to `peek_n(1)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::{Token, TokenWithPosition};
    /// # let tokens = vec![
    /// #     TokenWithPosition::new(Token::KeywordStore, 1, 1, 5),
    /// #     TokenWithPosition::new(Token::Identifier("x".to_string()), 1, 7, 1),
    /// # ];
    /// let cursor = Cursor::new(&tokens);
    /// assert_eq!(cursor.peek_next(), cursor.peek_n(1));
    /// ```
    #[inline]
    pub fn peek_next(&self) -> Option<&'a TokenWithPosition> {
        self.peek_n(1)
    }

    /// Peek at token kind (enum variant) without position info.
    ///
    /// Returns a reference to the `Token` enum, useful for pattern matching.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::{Token, TokenWithPosition};
    /// # let tokens = vec![TokenWithPosition::new(Token::KeywordStore, 1, 1, 5)];
    /// let cursor = Cursor::new(&tokens);
    /// assert_eq!(cursor.peek_kind(), Some(&Token::KeywordStore));
    /// ```
    #[inline]
    #[allow(dead_code)]
    pub fn peek_kind(&self) -> Option<&Token> {
        self.peek().map(|twp| &twp.token)
    }

    /// Peek at token kind N positions ahead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::{Token, TokenWithPosition};
    /// # let tokens = vec![
    /// #     TokenWithPosition::new(Token::KeywordStore, 1, 1, 5),
    /// #     TokenWithPosition::new(Token::Identifier("x".to_string()), 1, 7, 1),
    /// # ];
    /// let cursor = Cursor::new(&tokens);
    /// assert_eq!(cursor.peek_kind_n(1), Some(&Token::Identifier("x".to_string())));
    /// ```
    #[inline]
    #[allow(dead_code)]
    pub fn peek_kind_n(&self, n: usize) -> Option<&Token> {
        self.peek_n(n).map(|twp| &twp.token)
    }

    /// Consume current token and advance position.
    ///
    /// Returns a reference to the consumed token, or `None` if at EOF.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::{Token, TokenWithPosition};
    /// # let tokens = vec![
    /// #     TokenWithPosition::new(Token::KeywordStore, 1, 1, 5),
    /// #     TokenWithPosition::new(Token::Identifier("x".to_string()), 1, 7, 1),
    /// # ];
    /// let mut cursor = Cursor::new(&tokens);
    /// let first = cursor.bump().unwrap();
    /// assert_eq!(first.token, Token::KeywordStore);
    /// assert_eq!(cursor.pos(), 1);
    /// ```
    #[inline]
    pub fn bump(&mut self) -> Option<&'a TokenWithPosition> {
        if self.is_eof() {
            None
        } else {
            let token = &self.tokens[self.pos];
            self.pos += 1;
            Some(token)
        }
    }

    /// Check if current token matches expected type.
    ///
    /// Compares using discriminant equality (ignores enum payload).
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::{Token, TokenWithPosition};
    /// # let tokens = vec![TokenWithPosition::new(Token::KeywordStore, 1, 1, 5)];
    /// let cursor = Cursor::new(&tokens);
    /// assert!(cursor.at(Token::KeywordStore));
    /// assert!(!cursor.at(Token::KeywordDisplay));
    /// ```
    #[inline]
    #[allow(dead_code)]
    pub fn at(&self, expected: Token) -> bool {
        self.peek_kind()
            .is_some_and(|t| std::mem::discriminant(t) == std::mem::discriminant(&expected))
    }

    /// Consume token if it matches expected type.
    ///
    /// Returns `true` if token was consumed, `false` otherwise.
    /// Position advances only on match.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::{Token, TokenWithPosition};
    /// # let tokens = vec![
    /// #     TokenWithPosition::new(Token::KeywordStore, 1, 1, 5),
    /// #     TokenWithPosition::new(Token::Identifier("x".to_string()), 1, 7, 1),
    /// # ];
    /// let mut cursor = Cursor::new(&tokens);
    /// assert!(cursor.eat(Token::KeywordStore));
    /// assert_eq!(cursor.pos(), 1);
    /// assert!(!cursor.eat(Token::KeywordStore)); // No longer at store
    /// assert_eq!(cursor.pos(), 1); // Position unchanged
    /// ```
    #[inline]
    #[allow(dead_code)]
    pub fn eat(&mut self, expected: Token) -> bool {
        if self.at(expected) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Create checkpoint for potential backtracking.
    ///
    /// Returns current position that can be restored later via `rewind()`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::{Token, TokenWithPosition};
    /// # let tokens = vec![
    /// #     TokenWithPosition::new(Token::KeywordStore, 1, 1, 5),
    /// #     TokenWithPosition::new(Token::Identifier("x".to_string()), 1, 7, 1),
    /// # ];
    /// let mut cursor = Cursor::new(&tokens);
    /// let checkpoint = cursor.checkpoint();
    /// cursor.bump();
    /// cursor.bump();
    /// cursor.rewind(checkpoint);
    /// assert_eq!(cursor.pos(), 0);
    /// ```
    #[inline]
    pub fn checkpoint(&self) -> usize {
        self.pos
    }

    /// Restore position to previous checkpoint.
    ///
    /// # Arguments
    ///
    /// * `checkpoint` - Position from previous `checkpoint()` call
    ///
    /// # Panics
    ///
    /// In debug builds, panics if checkpoint is beyond token stream length.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::{Token, TokenWithPosition};
    /// # let tokens = vec![
    /// #     TokenWithPosition::new(Token::KeywordStore, 1, 1, 5),
    /// #     TokenWithPosition::new(Token::Identifier("x".to_string()), 1, 7, 1),
    /// # ];
    /// let mut cursor = Cursor::new(&tokens);
    /// cursor.bump();
    /// let cp = cursor.checkpoint();
    /// cursor.bump();
    /// cursor.rewind(cp);
    /// assert_eq!(cursor.pos(), 1);
    /// ```
    #[inline]
    pub fn rewind(&mut self, checkpoint: usize) {
        debug_assert!(
            checkpoint <= self.tokens.len(),
            "Invalid checkpoint: {} > {}",
            checkpoint,
            self.tokens.len()
        );
        self.pos = checkpoint;
    }

    /// Get number of remaining tokens.
    ///
    /// Returns count of unconsumed tokens in the stream.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::{Token, TokenWithPosition};
    /// # let tokens = vec![
    /// #     TokenWithPosition::new(Token::KeywordStore, 1, 1, 5),
    /// #     TokenWithPosition::new(Token::Identifier("x".to_string()), 1, 7, 1),
    /// # ];
    /// let mut cursor = Cursor::new(&tokens);
    /// assert_eq!(cursor.remaining(), 2);
    /// cursor.bump();
    /// assert_eq!(cursor.remaining(), 1);
    /// ```
    #[inline]
    pub fn remaining(&self) -> usize {
        self.tokens.len().saturating_sub(self.pos)
    }

    /// Get line number of current token.
    ///
    /// Returns line number, or 0 if at EOF.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::{Token, TokenWithPosition};
    /// # let tokens = vec![TokenWithPosition::new(Token::KeywordStore, 42, 1, 5)];
    /// let cursor = Cursor::new(&tokens);
    /// assert_eq!(cursor.current_line(), 42);
    /// ```
    #[inline]
    pub fn current_line(&self) -> usize {
        self.peek().map_or(0, |t| t.line)
    }

    /// Get column number of current token.
    ///
    /// Returns column number, or 0 if at EOF.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::{Token, TokenWithPosition};
    /// # let tokens = vec![TokenWithPosition::new(Token::KeywordStore, 1, 10, 5)];
    /// let cursor = Cursor::new(&tokens);
    /// assert_eq!(cursor.current_column(), 10);
    /// ```
    #[inline]
    pub fn current_column(&self) -> usize {
        self.peek().map_or(0, |t| t.column)
    }

    /// Get span of current token (for error reporting).
    ///
    /// Returns a Span covering the current token's byte range.
    /// If at EOF, returns a zero-length span.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::{Token, TokenWithPosition};
    /// # use wfl::diagnostics::Span;
    /// # let tokens = vec![TokenWithPosition::with_span(Token::KeywordStore, 1, 1, 5, 0, 5)];
    /// let cursor = Cursor::new(&tokens);
    /// let span = cursor.current_span();
    /// assert_eq!(span.start, 0);
    /// assert_eq!(span.end, 5);
    /// ```
    #[allow(dead_code)]
    pub fn current_span(&self) -> crate::diagnostics::Span {
        use crate::diagnostics::Span;
        self.peek()
            .map(|t| Span {
                start: t.byte_start,
                end: t.byte_end,
            })
            .unwrap_or(Span { start: 0, end: 0 })
    }

    /// Create ParseError from current token position.
    ///
    /// Convenience method that creates a ParseError with proper span
    /// information from the current token.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wfl::parser::cursor::Cursor;
    /// # use wfl::lexer::token::{Token, TokenWithPosition};
    /// # let tokens = vec![TokenWithPosition::with_span(Token::KeywordStore, 1, 1, 5, 0, 5)];
    /// let cursor = Cursor::new(&tokens);
    /// let error = cursor.error("Test error".to_string());
    /// assert_eq!(error.line, 1);
    /// assert_eq!(error.column, 1);
    /// assert_eq!(error.span.start, 0);
    /// assert_eq!(error.span.end, 5);
    /// ```
    #[allow(dead_code)]
    pub fn error(&self, message: String) -> crate::parser::ast::ParseError {
        use crate::parser::ast::ParseError;
        if let Some(token) = self.peek() {
            ParseError::from_token(message, token)
        } else {
            ParseError::from_span(message, crate::diagnostics::Span { start: 0, end: 0 }, 0, 0)
        }
    }
}

impl<'a> std::fmt::Debug for Cursor<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cursor")
            .field("pos", &self.pos)
            .field("remaining", &self.remaining())
            .field("current", &self.peek())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tokens() -> Vec<TokenWithPosition> {
        vec![
            TokenWithPosition::new(Token::KeywordStore, 1, 1, 5),
            TokenWithPosition::new(Token::Identifier("x".to_string()), 1, 7, 1),
            TokenWithPosition::new(Token::KeywordAs, 1, 9, 2),
            TokenWithPosition::new(Token::FloatLiteral(42.0), 1, 12, 2),
        ]
    }

    #[test]
    fn new_cursor_starts_at_zero() {
        let tokens = make_tokens();
        let cursor = Cursor::new(&tokens);
        assert_eq!(cursor.pos(), 0);
        assert!(!cursor.is_eof());
    }

    #[test]
    fn peek_returns_current_without_advancing() {
        let tokens = make_tokens();
        let cursor = Cursor::new(&tokens);

        let first = cursor.peek().unwrap();
        assert_eq!(first.token, Token::KeywordStore);
        assert_eq!(cursor.pos(), 0); // Position unchanged
    }

    #[test]
    fn bump_consumes_and_advances() {
        let tokens = make_tokens();
        let mut cursor = Cursor::new(&tokens);

        let first = cursor.bump().unwrap();
        assert_eq!(first.token, Token::KeywordStore);
        assert_eq!(cursor.pos(), 1);

        let second = cursor.peek().unwrap();
        assert_eq!(second.token, Token::Identifier("x".to_string()));
    }

    #[test]
    fn peek_n_lookahead() {
        let tokens = make_tokens();
        let cursor = Cursor::new(&tokens);

        assert_eq!(cursor.peek_n(0).unwrap().token, Token::KeywordStore);
        assert_eq!(
            cursor.peek_n(1).unwrap().token,
            Token::Identifier("x".to_string())
        );
        assert_eq!(cursor.peek_n(2).unwrap().token, Token::KeywordAs);
        assert!(cursor.peek_n(10).is_none());
    }

    #[test]
    fn peek_next_is_peek_n_1() {
        let tokens = make_tokens();
        let cursor = Cursor::new(&tokens);

        assert_eq!(cursor.peek_next(), cursor.peek_n(1));
    }

    #[test]
    fn at_checks_token_type() {
        let tokens = make_tokens();
        let cursor = Cursor::new(&tokens);

        assert!(cursor.at(Token::KeywordStore));
        assert!(!cursor.at(Token::KeywordAs));
    }

    #[test]
    fn eat_consumes_if_matches() {
        let tokens = make_tokens();
        let mut cursor = Cursor::new(&tokens);

        assert!(cursor.eat(Token::KeywordStore));
        assert_eq!(cursor.pos(), 1);

        assert!(!cursor.eat(Token::KeywordStore)); // No longer at store
        assert_eq!(cursor.pos(), 1); // Position unchanged
    }

    #[test]
    fn checkpoint_and_rewind() {
        let tokens = make_tokens();
        let mut cursor = Cursor::new(&tokens);

        cursor.bump(); // Advance to pos 1
        cursor.bump(); // Advance to pos 2
        let cp = cursor.checkpoint();
        assert_eq!(cp, 2);

        cursor.bump(); // Advance to pos 3
        assert_eq!(cursor.pos(), 3);

        cursor.rewind(cp);
        assert_eq!(cursor.pos(), 2);
        assert_eq!(cursor.peek().unwrap().token, Token::KeywordAs);
    }

    #[test]
    fn is_eof_detection() {
        let tokens = make_tokens();
        let mut cursor = Cursor::new(&tokens);

        assert!(!cursor.is_eof());

        for _ in 0..4 {
            cursor.bump();
        }

        assert!(cursor.is_eof());
        assert_eq!(cursor.peek(), None);
    }

    #[test]
    fn remaining_count() {
        let tokens = make_tokens();
        let mut cursor = Cursor::new(&tokens);

        assert_eq!(cursor.remaining(), 4);
        cursor.bump();
        assert_eq!(cursor.remaining(), 3);
        cursor.bump();
        cursor.bump();
        cursor.bump();
        assert_eq!(cursor.remaining(), 0);
    }

    #[test]
    fn peek_kind_extracts_token() {
        let tokens = make_tokens();
        let cursor = Cursor::new(&tokens);

        assert_eq!(cursor.peek_kind(), Some(&Token::KeywordStore));
    }

    #[test]
    fn current_line_and_column() {
        let tokens = make_tokens();
        let cursor = Cursor::new(&tokens);

        assert_eq!(cursor.current_line(), 1);
        assert_eq!(cursor.current_column(), 1);
    }

    #[test]
    fn empty_token_stream() {
        let tokens: Vec<TokenWithPosition> = vec![];
        let cursor = Cursor::new(&tokens);

        assert!(cursor.is_eof());
        assert_eq!(cursor.peek(), None);
        assert_eq!(cursor.remaining(), 0);
        assert_eq!(cursor.current_line(), 0);
        assert_eq!(cursor.current_column(), 0);
    }

    #[test]
    fn peek_kind_n_lookahead() {
        let tokens = make_tokens();
        let cursor = Cursor::new(&tokens);

        assert_eq!(cursor.peek_kind_n(0), Some(&Token::KeywordStore));
        assert_eq!(
            cursor.peek_kind_n(1),
            Some(&Token::Identifier("x".to_string()))
        );
        assert_eq!(cursor.peek_kind_n(2), Some(&Token::KeywordAs));
        assert_eq!(cursor.peek_kind_n(10), None);
    }
}
