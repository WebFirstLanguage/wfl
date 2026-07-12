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

/// Poll the run budget every this many lexed tokens. A power of two so the
/// stride test is a mask.
const LEX_CHECKPOINT_STRIDE: u64 = 4096;

pub fn lex_wfl(input: &str) -> Vec<Token> {
    // Bolt: We no longer normalize line endings globally to avoid allocation.
    // Token::Newline now matches \r\n, \n, and \r.
    let mut lexer = Token::lexer(input);
    // Estimate token count to avoid reallocations.
    // Heuristic: Average token length + whitespace ~ 10 bytes.
    // Reduces reallocations by ~23% for dense code while maintaining stable performance for string-heavy code.
    // Ensure at least 1 capacity to handle very small inputs.
    let estimated_tokens = (input.len() / 10).max(1);
    let mut tokens = Vec::with_capacity(estimated_tokens);
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

/// Tokenize `input` (with positions), consulting `checkpoint` every
/// [`LEX_CHECKPOINT_STRIDE`] tokens. The checkpoint returns `Err(BudgetExceeded)`
/// to abort tokenization with a typed, fatal outcome — the lexer never returns a
/// silently truncated token stream as a success. The two public entry points
/// below supply either a no-op checkpoint (non-budgeted callers: the LSP,
/// tooling, and tests) or a budget-enforcing one (production execution paths).
fn lex_positions_core<F>(
    input: &str,
    mut checkpoint: F,
) -> Result<Vec<TokenWithPosition>, crate::exec::budget::BudgetExceeded>
where
    F: FnMut() -> Result<(), crate::exec::budget::BudgetExceeded>,
{
    // Bolt: We no longer normalize line endings globally to avoid allocation.
    // Token::Newline now matches \r\n, \n, and \r.
    let mut lexer = Token::lexer(input);
    // Estimate token count to avoid reallocations.
    // Heuristic: Average token length + whitespace ~ 10 bytes.
    // Reduces reallocations by ~23% for dense code while maintaining stable performance for string-heavy code.
    // Ensure at least 1 capacity to handle very small inputs.
    let estimated_tokens = (input.len() / 10).max(1);
    let mut tokens = Vec::with_capacity(estimated_tokens);
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
    // Strided run-budget checkpoint counter. The lexer is otherwise a tight loop
    // with no budget call, so a large (source-size-capped) input would tokenize
    // fully even after the run's deadline, and a same-task Ctrl-C could not
    // interrupt lexing.
    let mut lexed_tokens: u64 = 0;

    while let Some(token_result) = lexer.next() {
        // Every `LEX_CHECKPOINT_STRIDE` tokens, consult the checkpoint. On a
        // budget breach it returns `Err`, and `?` aborts tokenization with a
        // typed, fatal outcome — the lexer never returns a truncated stream as a
        // success that a later phase could mistake for a complete program.
        lexed_tokens = lexed_tokens.wrapping_add(1);
        if lexed_tokens & (LEX_CHECKPOINT_STRIDE - 1) == 0 {
            checkpoint()?;
        }
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
                    let len = bytes.len();

                    // Bolt Optimization: Fast path for single-line strings.
                    // Use find() to locate the first newline, which avoids double scanning the entire string
                    // when checking for both \n and \r, and allows skipping the clean prefix in the manual loop.
                    let first_nl = slice.find('\n');
                    let limit = first_nl.unwrap_or(len);
                    // Search for \r only up to the first \n (or end if no \n)
                    let first_cr = slice[..limit].find('\r');

                    let mut i = if let Some(r) = first_cr {
                        r
                    } else if let Some(n) = first_nl {
                        n
                    } else {
                        len
                    };

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
                        span.start, slice
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
    Ok(tokens)
}

/// Tokenize `input` **without** budget enforcement. Used by the LSP, tooling,
/// and tests that run outside a run budget. Its checkpoint is a no-op, so it
/// never truncates and cannot fail — the silent-truncation footgun that a
/// budget breach used to create in this path no longer exists here.
pub fn lex_wfl_with_positions(input: &str) -> Vec<TokenWithPosition> {
    match lex_positions_core(input, || Ok(())) {
        Ok(tokens) => tokens,
        // The no-op checkpoint never returns `Err`, so this arm is unreachable.
        Err(_) => unreachable!("the no-op lexer checkpoint never breaches the budget"),
    }
}

/// Tokenize `input` under the current [`crate::exec::budget::ExecutionBudget`],
/// returning a typed [`crate::exec::budget::BudgetExceeded`] on a deadline /
/// cancellation / operation-ceiling breach instead of a silently truncated
/// token stream. Production execution paths (the CLI run and `--lex`, nested
/// `execute file` / `include` / `load module` loading, and the REPL) use this
/// and propagate the error, so a breach can never let a source *prefix* be
/// parsed, analyzed, or executed as if it were the whole program.
///
/// At each stride the deadline and cancellation are checked **directly** — not
/// through `charge_operation`'s 1024-operation sampling, which (nested inside
/// the 4096-token stride) could postpone those checks by millions of tokens —
/// and the operation ceiling is charged separately.
pub fn lex_wfl_with_positions_checked(
    input: &str,
) -> Result<Vec<TokenWithPosition>, crate::exec::budget::BudgetExceeded> {
    use crate::exec::budget::ExecutionBudget;
    lex_positions_core(input, || {
        let Some(budget) = ExecutionBudget::current() else {
            return Ok(());
        };
        // Direct, every-stride deadline/cancellation checks so a breach is
        // observed within one stride rather than after `charge_operation`'s
        // sampling would next fire.
        budget.check_cancelled()?;
        let exempt = budget.is_deadline_exempt();
        if !exempt {
            budget.check_deadline()?;
        }
        // Charge the operation ceiling separately (skipped while a `main loop`
        // is active, mirroring the interpreter's exemption).
        budget.charge_operation(!exempt)
    })
}
