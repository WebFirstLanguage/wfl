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

    let second_line_tokens: Vec<_> = tokens.iter().filter(|t| t.line == 2).collect();

    assert!(!second_line_tokens.is_empty());
    assert_eq!(second_line_tokens[0].token, Token::KeywordStore);
    assert_eq!(second_line_tokens[0].column, 1);
}

#[test]
fn test_position_tracking_with_multiline_strings() {
    let input = "store x as \"line1\nline2\"\ndisplay x";
    let tokens = lex_wfl_with_positions(input);

    // Find the string literal token
    let str_token = tokens
        .iter()
        .find(|t| matches!(t.token, Token::StringLiteral(_)))
        .expect("Should find string literal");

    assert_eq!(str_token.line, 1);

    // The next token should be on the correct line (line 2 due to \n in string, or line 3 if the string itself had a newline)
    let display_token = tokens
        .iter()
        .find(|t| matches!(t.token, Token::KeywordDisplay))
        .expect("Should find display keyword");

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
    assert_eq!(tokens[1].token, Token::Eol);
    assert_eq!(tokens[1].line, 1);
    // Note: The lexer emits Eol for \n.

    // x: starts at line 2, col 3 (2 spaces skipped)
    assert_eq!(tokens[2].line, 2);
    // 2 spaces skipped means it starts at col 3
    assert_eq!(tokens[2].column, 3);
}

#[test]
fn test_crlf_impact() {
    // Verify that CRLF is handled as a single newline in terms of line count,
    // and that position tracking works correctly after it.
    let input = "a\r\nb";
    let tokens = lex_wfl_with_positions(input);

    assert_eq!(tokens[0].line, 1); // a
    assert_eq!(tokens[1].token, Token::Eol); // \r\n
    assert_eq!(tokens[2].line, 2); // b
    assert_eq!(tokens[2].column, 1);
}

#[test]
fn test_mixed_line_endings() {
    // Test mixing \n, \r\n, and \r
    let input = "a\nb\r\nc\rd";
    let tokens = lex_wfl_with_positions(input);

    // a (L1)
    assert_eq!(tokens[0].line, 1);

    // \n (L1->L2)
    assert_eq!(tokens[1].token, Token::Eol);
    assert_eq!(tokens[1].line, 1);

    // b (L2)
    assert_eq!(tokens[2].token, Token::Identifier("b".to_string()));
    assert_eq!(tokens[2].line, 2);

    // \r\n (L2->L3)
    assert_eq!(tokens[3].token, Token::Eol);
    assert_eq!(tokens[3].line, 2);

    // c (L3)
    assert_eq!(tokens[4].token, Token::Identifier("c".to_string()));
    assert_eq!(tokens[4].line, 3);

    // \r (L3->L4)
    assert_eq!(tokens[5].token, Token::Eol);
    assert_eq!(tokens[5].line, 3);

    // d (L4)
    assert_eq!(tokens[6].token, Token::Identifier("d".to_string()));
    assert_eq!(tokens[6].line, 4);
    assert_eq!(tokens[6].column, 1);
}
