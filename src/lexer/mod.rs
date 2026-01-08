#[cfg(test)]
mod column_tests;
#[cfg(test)]
mod position_tests;
#[cfg(test)]
mod tests;

pub mod token;
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
    // Bolt: We no longer normalize line endings globally to avoid allocation.
    // Token::Newline now matches \r\n, \n, and \r.
    let mut lexer = Token::lexer(input);
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
    // Bolt: We no longer normalize line endings globally to avoid allocation.
    // Token::Newline now matches \r\n, \n, and \r.
    let mut lexer = Token::lexer(input);
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
        let skipped_len = span.start - last_span_end;
        current_column += skipped_len;

        let token_line = current_line;
        let token_column = current_column;
        let token_length = span.end - span.start;

        // Update position for the next token based on current token content
        let slice = lexer.slice();

        let mut newline_count = 0;
        let mut last_nl_end_dist = 0;

        // Optimization: Only scan tokens that CAN contain newlines.
        // Most tokens (Identifiers, Keywords, IntLiterals, etc.) do not.
        let needs_scan = match &token_result {
             Ok(Token::Newline) => true,
             Ok(Token::StringLiteral(_)) => true,
             Ok(Token::Error) => true, // Errors might include newlines depending on logos config
             _ => false,
        };

        if needs_scan {
             match &token_result {
                 Ok(Token::Newline) => {
                     // We know Newline token is exactly one newline sequence
                     newline_count = 1;
                     // Column resets to 1 (distance 0 from end of newline)
                     last_nl_end_dist = 0;
                 },
                 _ => {
                     // Scan bytes for StringLiteral or Error
                     let bytes = slice.as_bytes();
                     let mut i = 0;
                     let len = bytes.len();
                     while i < len {
                         if bytes[i] == b'\n' {
                             newline_count += 1;
                             last_nl_end_dist = len - (i + 1);
                         } else if bytes[i] == b'\r' {
                             if i + 1 < len && bytes[i+1] == b'\n' {
                                 // \r\n. Count it when we hit \n (next iteration) or skip \r?
                                 // If we don't count here, next iter sees \n and counts.
                                 // Dist will be updated then.
                             } else {
                                 // Standalone \r
                                 newline_count += 1;
                                 last_nl_end_dist = len - (i + 1);
                             }
                         }
                         i += 1;
                     }
                 }
             }
        }

        if newline_count > 0 {
            current_line += newline_count;
            current_column = 1 + last_nl_end_dist;
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
                    token_length,
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
