pub mod token;

#[cfg(test)]
mod tests;

use logos::Logos;
use std::borrow::Cow;
use token::{Token, TokenWithPosition};

#[deprecated(
    since = "0.1.0",
    note = "Use normalize_line_endings_cow for better performance"
)]
pub fn normalize_line_endings(input: &str) -> String {
    normalize_line_endings_cow(input).into_owned()
}

pub fn normalize_line_endings_cow(input: &str) -> Cow<'_, str> {
    if !input.contains('\r') {
        return Cow::Borrowed(input);
    }
    // Handle CRLF first as it's the most common case requiring normalization
    if input.contains("\r\n") {
        return Cow::Owned(input.replace("\r\n", "\n"));
    }
    // Handle standalone CR (Mac Classic)
    Cow::Owned(input.replace('\r', "\n"))
}

pub fn lex_wfl(input: &str) -> Vec<Token> {
    let input = normalize_line_endings_cow(input);
    let mut lexer = Token::lexer(&input);
    let mut tokens = Vec::new();
    let mut current_id: Option<String> = None;

    while let Some(token_result) = lexer.next() {
        match token_result {
            Ok(Token::Error) => {
                eprintln!(
                    "Lexing error at position {}: unexpected input `{}`",
                    lexer.span().start,
                    lexer.slice()
                );
            }
            Ok(Token::Identifier(word)) => {
                if let Some(ref mut id) = current_id {
                    id.push(' ');
                    id.push_str(&word);
                } else {
                    current_id = Some(word);
                }
            }
            Ok(Token::Newline) => {
                // Flush multi-word identifier if any
                if let Some(id) = current_id.take() {
                    tokens.push(Token::Identifier(id));
                }

                // NEW: Emit Eol token
                tokens.push(Token::Eol);
            }
            Ok(other) => {
                if let Some(id) = current_id.take() {
                    tokens.push(Token::Identifier(id));
                }
                if let Token::StringLiteral(s) = &other {
                    tokens.push(Token::StringLiteral(s.clone()));
                } else {
                    tokens.push(other);
                }
            }
            Err(_) => {
                eprintln!(
                    "Lexing error at position {}: unexpected input `{}`",
                    lexer.span().start,
                    lexer.slice()
                );
            }
        }
    }

    if let Some(id) = current_id.take() {
        tokens.push(Token::Identifier(id));
    }
    tokens
}

pub fn lex_wfl_with_positions(input: &str) -> Vec<TokenWithPosition> {
    let input = normalize_line_endings_cow(input);
    let mut lexer = Token::lexer(&input);
    let mut tokens = Vec::new();
    let mut current_id: Option<String> = None;
    let mut current_id_start_line = 0;
    let mut current_id_start_column = 0;
    let mut current_id_length = 0;
    // NEW: Track byte positions for multi-word identifiers
    let mut current_id_byte_start = 0;
    let mut current_id_byte_end = 0;

    // Track position incrementally to avoid O(N) pre-scan and O(log N) lookup
    let mut current_line = 1;
    let mut current_column = 1;
    let mut last_span_end = 0;

    while let Some(token_result) = lexer.next() {
        let span = lexer.span();

        // Calculate skipped whitespace/comments length
        // Logos is configured to skip [ \t\f\r] and comments.
        // Since we normalize \r\n to \n and \r to \n, and \n is a Token,
        // the skipped content does not contain newlines.
        let skipped_len = span.start - last_span_end;
        current_column += skipped_len;

        let token_line = current_line;
        let token_column = current_column;
        let token_length = span.end - span.start;

        // Update position for the next token based on current token content
        let slice = lexer.slice();
        let newline_count = slice.as_bytes().iter().filter(|&&b| b == b'\n').count();
        if newline_count > 0 {
            current_line += newline_count;
            // Guaranteed to exist if newline_count > 0
            let last_nl_pos = slice.rfind('\n').unwrap();
            current_column = slice.len() - last_nl_pos;
        } else {
            current_column += slice.len();
        }

        last_span_end = span.end;

        match token_result {
            Ok(Token::Error) => {
                eprintln!(
                    "Lexing error at position {}: unexpected input `{}`",
                    span.start,
                    lexer.slice()
                );
            }
            Ok(Token::Identifier(word)) => {
                if let Some(ref mut id) = current_id {
                    id.push(' ');
                    id.push_str(&word);
                    // For multi-word identifiers, we need to account for the space and additional word
                    current_id_length += 1 + token_length; // +1 for the space
                    current_id_byte_end = span.end; // NEW: Update end byte position
                } else {
                    current_id = Some(word);
                    current_id_start_line = token_line;
                    current_id_start_column = token_column;
                    current_id_length = token_length;
                    current_id_byte_start = span.start; // NEW: Track start byte position
                    current_id_byte_end = span.end; // NEW: Track end byte position
                }
            }
            Ok(Token::Newline) => {
                // Flush multi-word identifier if any
                if let Some(id) = current_id.take() {
                    tokens.push(TokenWithPosition::with_span(
                        Token::Identifier(id),
                        current_id_start_line,
                        current_id_start_column,
                        current_id_length,
                        current_id_byte_start,
                        current_id_byte_end,
                    ));
                }

                // NEW: Emit Eol token to mark statement boundary
                tokens.push(TokenWithPosition::with_span(
                    Token::Eol,
                    token_line,
                    token_column,
                    token_length, // Length of '\n' = 1
                    span.start,
                    span.end,
                ));
            }
            Ok(other) => {
                if let Some(id) = current_id.take() {
                    tokens.push(TokenWithPosition::with_span(
                        Token::Identifier(id),
                        current_id_start_line,
                        current_id_start_column,
                        current_id_length,
                        current_id_byte_start,
                        current_id_byte_end,
                    ));
                }

                if let Token::StringLiteral(s) = &other {
                    tokens.push(TokenWithPosition::with_span(
                        Token::StringLiteral(s.clone()),
                        token_line,
                        token_column,
                        token_length,
                        span.start,
                        span.end,
                    ));
                } else {
                    tokens.push(TokenWithPosition::with_span(
                        other,
                        token_line,
                        token_column,
                        token_length,
                        span.start,
                        span.end,
                    ));
                }
            }
            Err(_) => {
                eprintln!(
                    "Lexing error at position {}: unexpected input `{}`",
                    span.start,
                    lexer.slice()
                );
            }
        }
    }

    if let Some(id) = current_id.take() {
        tokens.push(TokenWithPosition::with_span(
            Token::Identifier(id),
            current_id_start_line,
            current_id_start_column,
            current_id_length,
            current_id_byte_start,
            current_id_byte_end,
        ));
    }
    tokens
}
