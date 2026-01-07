use super::*;
use crate::lexer::lex_wfl_with_positions;

#[test]
fn test_position_tracking_basic() {
    let input = "store x as 5";
    let tokens = lex_wfl_with_positions(input);

    // store (line 1, col 1)
    assert_eq!(tokens[0].token, Token::KeywordStore);
    assert_eq!(tokens[0].line, 1);
    assert_eq!(tokens[0].column, 1);

    // x (line 1, col 7)
    match &tokens[1].token {
        Token::Identifier(s) => assert_eq!(s, "x"),
        _ => panic!("Expected identifier"),
    }
    assert_eq!(tokens[1].line, 1);
    assert_eq!(tokens[1].column, 7);

    // as (line 1, col 9)
    assert_eq!(tokens[2].token, Token::KeywordAs);
    assert_eq!(tokens[2].line, 1);
    assert_eq!(tokens[2].column, 9);

    // 5 (line 1, col 12)
    assert_eq!(tokens[3].token, Token::IntLiteral(5));
    assert_eq!(tokens[3].line, 1);
    assert_eq!(tokens[3].column, 12);
}

#[test]
fn test_position_tracking_multiline() {
    let input = "store x as 5\nstore y as 10";
    let tokens = lex_wfl_with_positions(input);

    let second_line_tokens: Vec<_> = tokens.iter()
        .filter(|t| t.line == 2)
        .collect();

    assert!(!second_line_tokens.is_empty());
    assert_eq!(second_line_tokens[0].token, Token::KeywordStore);
    assert_eq!(second_line_tokens[0].column, 1);
}

#[test]
fn test_position_tracking_with_multiline_strings() {
    let input = "store x as \"line1\nline2\"\ndisplay x";
    let tokens = lex_wfl_with_positions(input);

    // Find the string literal token
    let str_token = tokens.iter()
        .find(|t| matches!(t.token, Token::StringLiteral(_)))
        .expect("Should find string literal");

    assert_eq!(str_token.line, 1);

    // The next token should be on the correct line (line 2 due to \n in string, or line 3 if the string itself had a newline)
    // The input has "store x as "line1\nline2"\n"
    // "store" (L1), "x" (L1), "as" (L1), "line1\nline2" (L1..L2), \n (L2->L3 or EOL)

    // Let's trace how the lexer should handle this:
    // "store x as " (line 1)
    // "line1\nline2" starts at line 1.
    // It contains one newline.
    // The Eol token following it comes from the \n after the string.

    let display_token = tokens.iter()
        .find(|t| matches!(t.token, Token::KeywordDisplay))
        .expect("Should find display keyword");

    // "line1\nline2" consumes 1 newline. So we are at line 2.
    // Then there is a \n in the input. That moves us to line 3.
    // Wait, let's verify exact behavior.
    // The string token itself spans line 1 to 2.
    // The `\n` after the string is a Token::Newline (or Eol).

    // Let's verify the line of 'display'.
    // Line 1: store x as "line1\nline2"
    // Line 2: (part of string)
    // Line 3: display x
    // Actually, "line1\nline2" is 12 chars + quotes = 14 chars.
    // Input:
    // L1: store x as "line1
    // L2: line2"
    // L3: display x

    // If the input string literally contains a newline character:
    assert_eq!(display_token.line, 3, "Display token should be on line 3");
}

#[test]
fn test_position_tracking_empty_input() {
    let input = "";
    let tokens = lex_wfl_with_positions(input);
    assert!(tokens.is_empty());
}

#[test]
fn test_position_tracking_consecutive_newlines() {
    let input = "a\n\n\nb";
    let tokens = lex_wfl_with_positions(input);

    // a at 1:1
    assert_eq!(tokens[0].line, 1);

    // \n -> Eol at 1:2
    assert_eq!(tokens[1].token, Token::Eol);
    assert_eq!(tokens[1].line, 1);

    // \n -> Eol at 2:1
    assert_eq!(tokens[2].token, Token::Eol);
    assert_eq!(tokens[2].line, 2);

    // \n -> Eol at 3:1
    assert_eq!(tokens[3].token, Token::Eol);
    assert_eq!(tokens[3].line, 3);

    // b at 4:1
    let b_token = tokens.last().unwrap();
    match &b_token.token {
        Token::Identifier(s) => assert_eq!(s, "b"),
        _ => panic!("Expected identifier b"),
    }
    assert_eq!(b_token.line, 4);
    assert_eq!(b_token.column, 1);
}

#[test]
fn test_position_tracking_mixed_content() {
    // Test a complex mix of tokens, spaces, and newlines
    let input = "   store   \n  x  ";
    let tokens = lex_wfl_with_positions(input);

    // store: starts at col 4 (3 spaces skipped)
    assert_eq!(tokens[0].token, Token::KeywordStore);
    assert_eq!(tokens[0].line, 1);
    assert_eq!(tokens[0].column, 4);

    // \n: Eol at col 12 (store=5, +3 spaces = 12?)
    // "   " (3) + "store" (5) + "   " (3) = 11. \n is at 12.
    assert_eq!(tokens[1].token, Token::Eol);
    assert_eq!(tokens[1].line, 1);
    // Note: The lexer emits Eol for \n.

    // x: starts at line 2, col 3 (2 spaces skipped)
    assert_eq!(tokens[2].line, 2);
    // 2 spaces skipped means it starts at col 3
    assert_eq!(tokens[2].column, 3);
}

#[test]
fn test_crlf_normalization_impact() {
    // The lexer normalizes CRLF to LF before processing.
    // So "a\r\nb" becomes "a\nb".
    let input = "a\r\nb";
    let tokens = lex_wfl_with_positions(input);

    assert_eq!(tokens[0].line, 1); // a
    assert_eq!(tokens[1].token, Token::Eol); // \n
    assert_eq!(tokens[2].line, 2); // b
    assert_eq!(tokens[2].column, 1);
}
