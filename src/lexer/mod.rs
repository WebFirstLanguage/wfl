#[cfg(test)]
mod column_tests;
#[cfg(test)]
mod position_tests;
#[cfg(test)]
mod string_line_ending_tests;
#[cfg(test)]
mod tests;

pub mod token;
use logos::Logos;
use token::{Token, TokenWithPosition};

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
                // Check if this keyword/token is actually part of a longer identifier
                // e.g., "content_type" should be an identifier, not KeywordContent + error
                let span = lexer.span();
                let is_keyword_in_identifier = other.is_keyword()
                    && span.end < input.len()
                    && input.as_bytes().get(span.end) == Some(&b'_');

                if is_keyword_in_identifier {
                    // This keyword is followed by underscore - treat as identifier start
                    let keyword_str = &input[span.start..span.end];
                    if let Some(ref mut id) = current_id {
                        id.push(' ');
                        id.push_str(keyword_str);
                    } else {
                        current_id = Some(keyword_str.to_string());
                    }
                } else {
                    if let Some(id) = current_id.take() {
                        tokens.push(Token::Identifier(id));
                    }
                    // Bolt: Optimized to avoid cloning StringLiteral.
                    // The token `other` is owned here, so we can consume it directly
                    // without borrowing and cloning the string content.
                    tokens.push(other);
                }
            }
            Err(_) => {
                let slice = lexer.slice();
                // Check if this is an underscore that's part of an identifier being accumulated
                // This handles cases like "content_type" where "content" was a keyword
                if slice == "_" && current_id.is_some() {
                    // Append underscore to current identifier
                    if let Some(ref mut id) = current_id {
                        id.push('_');
                    }
                } else {
                    eprintln!(
                        "Lexing error at position {}: unexpected input `{}`",
                        lexer.span().start,
                        slice
                    );
                }
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
                }
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
                            if i + 1 < len && bytes[i + 1] == b'\n' {
                                // Handle \r\n as a single newline (2-byte sequence)
                                newline_count += 1;
                                // Distance from end is calculated after BOTH bytes
                                last_nl_end_dist = len - (i + 2);
                                // Skip the \n byte on next iteration since we processed it here
                                i += 1;
                            } else {
                                // Standalone \r (Mac-style line ending)
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
                // Check if this keyword/token is actually part of a longer identifier
                // e.g., "content_type" should be an identifier, not KeywordContent + error
                let is_keyword_in_identifier = other.is_keyword()
                    && span.end < input.len()
                    && input.as_bytes().get(span.end) == Some(&b'_');

                if is_keyword_in_identifier {
                    // This keyword is followed by underscore - treat as identifier start
                    let keyword_str = &input[span.start..span.end];
                    if let Some(ref mut id) = current_id {
                        id.push(' ');
                        id.push_str(keyword_str);
                        current_id_length += 1 + token_length;
                        current_id_byte_end = span.end;
                    } else {
                        current_id = Some(keyword_str.to_string());
                        current_id_start_line = token_line;
                        current_id_start_column = token_column;
                        current_id_length = token_length;
                        current_id_byte_start = span.start;
                        current_id_byte_end = span.end;
                    }
                } else {
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

                    // Bolt: Optimized to avoid cloning StringLiteral.
                    // The token `other` is owned here, so we can consume it directly
                    // without borrowing and cloning the string content.
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
                let slice = lexer.slice();
                // Check if this is an underscore that's part of an identifier being accumulated
                // This handles cases like "content_type" where "content" was a keyword
                if slice == "_" && current_id.is_some() {
                    // Append underscore to current identifier
                    if let Some(ref mut id) = current_id {
                        id.push('_');
                        current_id_length += 1;
                        current_id_byte_end = span.end;
                    }
                } else {
                    eprintln!(
                        "Lexing error at position {}: unexpected input `{}`",
                        span.start,
                        slice
                    );
                }
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
