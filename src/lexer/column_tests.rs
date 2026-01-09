use super::*;
use crate::lexer::lex_wfl_with_positions;

#[test]
fn test_column_calculation_logic() {
    // Case 1: Simple newline
    let input = "\nstore";
    let tokens = lex_wfl_with_positions(input);
    assert_eq!(tokens[1].token, Token::KeywordStore);
    assert_eq!(tokens[1].line, 2);
    assert_eq!(tokens[1].column, 1);

    // Case 2: String ending with newline
    // "a\n"
    let input = "\"a\n\"store";
    let tokens = lex_wfl_with_positions(input);
    // Line 1: "a\n" -> Last newline is at the end of the line
    // The next token should be at the start of the next line (conceptually)
    // Actually, "a\n"
    // slice: " a \n "
    // len: 4
    // last_nl_pos: 2 (the \n)
    // col: 4 - 2 = 2.
    // So 'store' starts at col 2.
    // Wait, if I have `store "foo\n" bar`
    // Line 1: store "foo
    // Line 2: " bar
    // `bar` starts after `"` and space.
    // My manual calc says col 2.
    // Let's assert 2.
    assert_eq!(tokens[1].token, Token::KeywordStore);
    assert_eq!(tokens[1].line, 2);
    assert_eq!(tokens[1].column, 2);

    // Case 3: Embedded newline
    // "a\nb"
    let input = "\"a\nb\"store";
    let tokens = lex_wfl_with_positions(input);
    assert_eq!(tokens[1].token, Token::KeywordStore);
    assert_eq!(tokens[1].line, 2);
    assert_eq!(tokens[1].column, 3);
}

#[test]
fn test_column_calculation_with_crlf_in_string() {
    // Verify column calculation after string with CRLF
    let input = "\"foo\r\nbar\" baz";
    let tokens = lex_wfl_with_positions(input);

    // String starts at col 1
    assert_eq!(tokens[0].column, 1);

    // 'baz' should be on line 2
    // "foo\r\nbar" ends on line 2
    // After closing quote and space, baz starts
    let baz_token = tokens
        .iter()
        .find(|t| matches!(&t.token, Token::Identifier(s) if s == "baz"))
        .expect("Should find baz");
    assert_eq!(baz_token.line, 2);
    // After "bar" (3) + closing quote (1) + space (1) = column 6
    assert_eq!(baz_token.column, 6);
}

#[test]
fn test_column_calculation_crlf_at_string_end() {
    // String ending with CRLF
    let input = "\"test\r\n\"x";
    let tokens = lex_wfl_with_positions(input);

    let x_token = tokens
        .iter()
        .find(|t| matches!(&t.token, Token::Identifier(s) if s == "x"))
        .expect("Should find x");
    assert_eq!(x_token.line, 2);
    // Line 2: closing quote at col 1, x starts at col 2
    assert_eq!(x_token.column, 2);
}

#[test]
fn test_column_calculation_multiple_crlf() {
    // Multiple CRLF sequences in string
    let input = "\"a\r\nb\r\nc\" d";
    let tokens = lex_wfl_with_positions(input);

    let d_token = tokens
        .iter()
        .find(|t| matches!(&t.token, Token::Identifier(s) if s == "d"))
        .expect("Should find d");
    assert_eq!(d_token.line, 3); // 2 newlines = line 3
    // After "c" (1) + closing quote (1) + space (1) = column 4
    assert_eq!(d_token.column, 4);
}

#[test]
fn test_column_calculation_crlf_vs_lf() {
    // Compare CRLF and LF to ensure they produce same column results
    let input_crlf = "\"foo\r\nbar\" x";
    let input_lf = "\"foo\nbar\" x";

    let tokens_crlf = lex_wfl_with_positions(input_crlf);
    let tokens_lf = lex_wfl_with_positions(input_lf);

    // Both should have x on line 2 at same column
    let x_crlf = tokens_crlf
        .iter()
        .find(|t| matches!(&t.token, Token::Identifier(s) if s == "x"))
        .expect("Should find x in CRLF");
    let x_lf = tokens_lf
        .iter()
        .find(|t| matches!(&t.token, Token::Identifier(s) if s == "x"))
        .expect("Should find x in LF");

    assert_eq!(x_crlf.line, x_lf.line);
    assert_eq!(x_crlf.column, x_lf.column);
}
