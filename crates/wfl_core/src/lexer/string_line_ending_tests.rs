use crate::lexer::lex_wfl_with_positions;
use crate::lexer::token::Token;

/// Test suite for string literals containing different line ending types
/// This ensures CRLF, CR, and LF are all handled correctly in string literals

// Category 1: Basic CRLF in Strings

#[test]
fn test_string_with_crlf() {
    // String containing CRLF should correctly track positions
    let input = "store x as \"line1\r\nline2\"\ndisplay x";
    let tokens = lex_wfl_with_positions(input);

    // Find the display token (after the string)
    let display = tokens
        .iter()
        .find(|t| matches!(t.token, Token::KeywordDisplay))
        .expect("Should find display");

    // String spans lines 1-2, display should be on line 3, column 1
    assert_eq!(display.line, 3, "Display should be on line 3");
    assert_eq!(display.column, 1, "Display should be at column 1");
}

#[test]
fn test_string_with_cr() {
    // String containing standalone CR
    let input = "store x as \"line1\rline2\"\ndisplay x";
    let tokens = lex_wfl_with_positions(input);

    let display = tokens
        .iter()
        .find(|t| matches!(t.token, Token::KeywordDisplay))
        .expect("Should find display");

    // String spans lines 1-2, display should be on line 3
    assert_eq!(display.line, 3, "Display should be on line 3");
    assert_eq!(display.column, 1, "Display should be at column 1");
}

#[test]
fn test_string_with_lf() {
    // Baseline test: String containing LF (should already work)
    let input = "store x as \"line1\nline2\"\ndisplay x";
    let tokens = lex_wfl_with_positions(input);

    let display = tokens
        .iter()
        .find(|t| matches!(t.token, Token::KeywordDisplay))
        .expect("Should find display");

    assert_eq!(display.line, 3, "Display should be on line 3");
    assert_eq!(display.column, 1, "Display should be at column 1");
}

// Category 2: Multiple Line Endings in Strings

#[test]
fn test_string_with_multiple_crlf() {
    // String with multiple CRLF sequences
    let input = "store x as \"line1\r\nline2\r\nline3\"\ndisplay x";
    let tokens = lex_wfl_with_positions(input);

    let display = tokens
        .iter()
        .find(|t| matches!(t.token, Token::KeywordDisplay))
        .expect("Should find display");

    // String spans lines 1-3, display should be on line 4
    assert_eq!(display.line, 4, "Display should be on line 4");
    assert_eq!(display.column, 1, "Display should be at column 1");
}

#[test]
fn test_string_with_mixed_endings_in_string() {
    // String containing mix of \n, \r\n, and \r
    let input = "store x as \"line1\nline2\r\nline3\rline4\"\ndisplay x";
    let tokens = lex_wfl_with_positions(input);

    let display = tokens
        .iter()
        .find(|t| matches!(t.token, Token::KeywordDisplay))
        .expect("Should find display");

    // String spans lines 1-4, display should be on line 5
    assert_eq!(display.line, 5, "Display should be on line 5");
    assert_eq!(display.column, 1, "Display should be at column 1");
}

// Category 3: Column Tracking

#[test]
fn test_column_after_string_with_crlf() {
    // Verify exact column calculation after CRLF in string
    let input = "\"a\r\nb\"store";
    let tokens = lex_wfl_with_positions(input);

    let store_token = tokens
        .iter()
        .find(|t| matches!(t.token, Token::KeywordStore))
        .expect("Should find store");

    // String "a\r\nb" has 5 bytes total, ends with 'b' on line 2
    // 'store' should be on line 2, column 3 (after closing quote and 'b')
    assert_eq!(store_token.line, 2, "Store should be on line 2");
    assert_eq!(store_token.column, 3, "Store should be at column 3");
}

#[test]
fn test_column_after_string_ending_with_crlf() {
    // String ending with CRLF
    let input = "\"a\r\n\"store";
    let tokens = lex_wfl_with_positions(input);

    let store_token = tokens
        .iter()
        .find(|t| matches!(t.token, Token::KeywordStore))
        .expect("Should find store");

    // String "a\r\n" ends with CRLF, store starts after closing quote on line 2
    assert_eq!(store_token.line, 2, "Store should be on line 2");
    assert_eq!(store_token.column, 2, "Store should be at column 2");
}

#[test]
fn test_column_after_string_starting_with_crlf() {
    // String starting with CRLF
    let input = "\"\r\nb\"store";
    let tokens = lex_wfl_with_positions(input);

    let store_token = tokens
        .iter()
        .find(|t| matches!(t.token, Token::KeywordStore))
        .expect("Should find store");

    // String "\r\nb" starts with CRLF, ends with 'b' on line 2
    assert_eq!(store_token.line, 2, "Store should be on line 2");
    assert_eq!(store_token.column, 3, "Store should be at column 3");
}

// Category 4: Edge Cases

#[test]
fn test_string_with_crlf_at_various_positions() {
    // Test CRLF at start, middle, and end
    let input = "store x as \"\r\nmiddle\r\n\"\ndisplay x";
    let tokens = lex_wfl_with_positions(input);

    let display = tokens
        .iter()
        .find(|t| matches!(t.token, Token::KeywordDisplay))
        .expect("Should find display");

    // String has CRLF at start and after "middle", spans lines 1-3
    assert_eq!(display.line, 4, "Display should be on line 4");
    assert_eq!(display.column, 1, "Display should be at column 1");
}

#[test]
fn test_adjacent_strings_with_crlf() {
    // Multiple strings with CRLF on consecutive lines
    let input = "store x as \"a\r\nb\"\nstore y as \"c\r\nd\"\ndisplay x";
    let tokens = lex_wfl_with_positions(input);

    // Find the second store
    let stores: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t.token, Token::KeywordStore))
        .collect();
    assert_eq!(stores.len(), 2, "Should find 2 store keywords");

    // First store at line 1, second store at line 3
    assert_eq!(stores[0].line, 1);
    assert_eq!(stores[1].line, 3);

    let display = tokens
        .iter()
        .find(|t| matches!(t.token, Token::KeywordDisplay))
        .expect("Should find display");

    // Display should be on line 5
    assert_eq!(display.line, 5);
}

#[test]
fn test_string_with_crlf_followed_by_comment() {
    // String with CRLF followed by a comment
    let input = "store x as \"a\r\nb\" // comment\ndisplay x";
    let tokens = lex_wfl_with_positions(input);

    let display = tokens
        .iter()
        .find(|t| matches!(t.token, Token::KeywordDisplay))
        .expect("Should find display");

    // String spans lines 1-2, comment is on line 2, display on line 3
    assert_eq!(display.line, 3, "Display should be on line 3");
}

// Category 5: Real-world Scenarios

#[test]
fn test_multiline_wfl_code_with_crlf_strings() {
    // Complete WFL program with CRLF in strings
    let input = "store message as \"Hello\r\nWorld\"\r\nstore count as 5\r\ndisplay message\r\ndisplay count";
    let tokens = lex_wfl_with_positions(input);

    // Find all display keywords
    let displays: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t.token, Token::KeywordDisplay))
        .collect();

    assert_eq!(displays.len(), 2, "Should find 2 display keywords");

    // First display should be on line 3 (after string spans 1-2, second store on line 2)
    // Wait, let's trace:
    // Line 1: store message as "Hello\r\n
    // Line 2: World"\r\n  <- string ends, then CRLF token
    // Line 3: store count as 5\r\n
    // Line 4: display message\r\n
    // Line 5: display count

    assert_eq!(displays[0].line, 4, "First display should be on line 4");
    assert_eq!(displays[1].line, 5, "Second display should be on line 5");
}

#[test]
fn test_string_with_escape_sequences_and_crlf() {
    // String with both escape sequences and actual CRLF
    // Note: WFL strings might not support \n escape, but test literal newlines
    let input = "store x as \"text\r\nmore\"\ndisplay x";
    let tokens = lex_wfl_with_positions(input);

    let string_token = tokens
        .iter()
        .find(|t| matches!(t.token, Token::StringLiteral(_)))
        .expect("Should find string literal");

    // String should start on line 1
    assert_eq!(string_token.line, 1);

    let display = tokens
        .iter()
        .find(|t| matches!(t.token, Token::KeywordDisplay))
        .expect("Should find display");

    // String spans lines 1-2, display on line 3
    assert_eq!(display.line, 3);
}
