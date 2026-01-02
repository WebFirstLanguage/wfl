pub mod token;

#[cfg(test)]
mod tests;

use logos::Logos;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use token::{Token, TokenWithPosition};

static STRING_POOL: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn intern_string(s: String) -> String {
    let pool = STRING_POOL.get_or_init(|| Mutex::new(HashMap::new()));
    let mut pool_guard = pool.lock().unwrap();

    if let Some(interned) = pool_guard.get(&s) {
        interned.clone()
    } else {
        pool_guard.insert(s.clone(), s.clone());
        s
    }
}

pub fn normalize_line_endings(input: &str) -> String {
    input.replace("\r\n", "\n")
}

pub fn lex_wfl(input: &str) -> Vec<Token> {
    let input = normalize_line_endings(input);
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
                    current_id = Some(intern_string(word));
                }
            }
            Ok(Token::Newline) => {
                // Flush multi-word identifier if any
                if let Some(id) = current_id.take() {
                    tokens.push(Token::Identifier(intern_string(id)));
                }

                // NEW: Emit Eol token
                tokens.push(Token::Eol);
            }
            Ok(other) => {
                if let Some(id) = current_id.take() {
                    tokens.push(Token::Identifier(intern_string(id)));
                }
                if let Token::StringLiteral(s) = &other {
                    tokens.push(Token::StringLiteral(intern_string(s.clone())));
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
        tokens.push(Token::Identifier(intern_string(id)));
    }
    tokens
}

pub fn lex_wfl_with_positions(input: &str) -> Vec<TokenWithPosition> {
    let input = normalize_line_endings(input);
    let mut lexer = Token::lexer(&input);
    let mut tokens = Vec::new();
    let mut current_id: Option<String> = None;
    let mut current_id_start_line = 0;
    let mut current_id_start_column = 0;
    let mut current_id_length = 0;
    // NEW: Track byte positions for multi-word identifiers
    let mut current_id_byte_start = 0;
    let mut current_id_byte_end = 0;

    let mut _line = 1;
    let mut _column = 1;
    let mut line_starts = vec![0];

    for (i, c) in input.char_indices() {
        if c == '\n' {
            _line += 1;
            _column = 1;
            line_starts.push(i + 1);
        } else {
            _column += 1;
        }
    }

    let position = |offset: usize| -> (usize, usize) {
        let line_idx = line_starts.binary_search(&offset).unwrap_or_else(|i| i - 1);
        let line = line_idx + 1;
        let column = offset - line_starts[line_idx] + 1;
        (line, column)
    };

    while let Some(token_result) = lexer.next() {
        let span = lexer.span();
        let (token_line, token_column) = position(span.start);
        let token_length = span.end - span.start;

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
                    current_id = Some(intern_string(word));
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
                        Token::Identifier(intern_string(id)),
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
                        Token::Identifier(intern_string(id)),
                        current_id_start_line,
                        current_id_start_column,
                        current_id_length,
                        current_id_byte_start,
                        current_id_byte_end,
                    ));
                }

                if let Token::StringLiteral(s) = &other {
                    tokens.push(TokenWithPosition::with_span(
                        Token::StringLiteral(intern_string(s.clone())),
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
            Token::Identifier(intern_string(id)),
            current_id_start_line,
            current_id_start_column,
            current_id_length,
            current_id_byte_start,
            current_id_byte_end,
        ));
    }
    tokens
}
