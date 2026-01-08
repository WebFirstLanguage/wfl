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
