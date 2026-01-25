use super::*;

#[test]
fn test_keyword_uniqueness() {
    let keywords = vec![
        Token::KeywordStore,
        Token::KeywordCreate,
        Token::KeywordDisplay,
        Token::KeywordCheck,
        Token::KeywordIf,
        Token::KeywordThen,
        Token::KeywordOtherwise,
        Token::KeywordEnd,
        Token::KeywordFor,
        Token::KeywordEach,
        Token::KeywordIn,
        Token::KeywordReversed,
        Token::KeywordFrom,
        Token::KeywordTo,
        Token::KeywordBy,
        Token::KeywordCount,
        Token::KeywordRepeat,
        Token::KeywordWhile,
        Token::KeywordUntil,
        Token::KeywordForever,
        Token::KeywordAction,
        Token::KeywordCall,
        Token::KeywordCalled,
        Token::KeywordWith,
        Token::KeywordNot,
        Token::KeywordBreak,
        Token::KeywordContinue,
        Token::KeywordReturn,
        Token::KeywordGive,
        Token::KeywordBack,
        Token::KeywordAs,
        Token::KeywordAt,
        Token::KeywordDefine,
        Token::KeywordNeeds,
        Token::KeywordChange,
        Token::KeywordAnd,
        Token::KeywordOr,
        Token::KeywordPattern,
        Token::KeywordRead,
        Token::KeywordWait,
        Token::KeywordSkip,
        Token::KeywordThan,
        Token::KeywordPush,
        Token::KeywordContainer,
        Token::KeywordProperty,
        Token::KeywordExtends,
        Token::KeywordImplements,
        Token::KeywordInterface,
        Token::KeywordRequires,
        Token::KeywordEvent,
        Token::KeywordTrigger,
        Token::KeywordOn,
        Token::KeywordStatic,
        Token::KeywordPublic,
        Token::KeywordPrivate,
        Token::KeywordParent,
        Token::KeywordNew,
        Token::KeywordMust,
        Token::KeywordDefaults,
    ];

    for keyword in &keywords {
        assert!(
            keyword.is_keyword(),
            "Token {keyword:?} should be recognized as a keyword"
        );
    }

    let non_keywords = vec![
        Token::Identifier("test".to_string()),
        Token::StringLiteral("hello".to_string()),
        Token::IntLiteral(42),
        Token::FloatLiteral(2.5),
        Token::BooleanLiteral(true),
        Token::NothingLiteral,
        Token::Colon,
        Token::LeftParen,
        Token::RightParen,
        Token::LeftBracket,
        Token::RightBracket,
        Token::Newline,
        Token::Error,
    ];

    for non_keyword in &non_keywords {
        assert!(
            !non_keyword.is_keyword(),
            "Token {non_keyword:?} should not be recognized as a keyword"
        );
    }
}

#[test]
fn test_container_keywords_lexing() {
    use logos::Logos;

    let test_cases = vec![
        ("container", Token::KeywordContainer),
        ("property", Token::KeywordProperty),
        ("extends", Token::KeywordExtends),
        ("implements", Token::KeywordImplements),
        ("interface", Token::KeywordInterface),
        ("requires", Token::KeywordRequires),
        ("event", Token::KeywordEvent),
        ("trigger", Token::KeywordTrigger),
        ("on", Token::KeywordOn),
        ("static", Token::KeywordStatic),
        ("public", Token::KeywordPublic),
        ("private", Token::KeywordPrivate),
        ("parent", Token::KeywordParent),
        ("new", Token::KeywordNew),
        ("must", Token::KeywordMust),
        ("defaults", Token::KeywordDefaults),
        ("call", Token::KeywordCall),
        ("action", Token::KeywordAction),
        ("called", Token::KeywordCalled),
    ];

    for (input, expected) in test_cases {
        let mut lexer = Token::lexer(input);
        let token = lexer
            .next()
            .unwrap_or_else(|| panic!("Failed to tokenize '{input}'"));
        assert_eq!(
            token,
            Ok(expected.clone()),
            "Input '{input}' should tokenize to {expected:?}"
        );
    }
}

#[test]
fn test_keyword_case_sensitivity() {
    use logos::Logos;

    let test_cases = vec![
        ("CONTAINER", Token::Identifier("CONTAINER".to_string())),
        ("Container", Token::Identifier("Container".to_string())),
        ("PROPERTY", Token::Identifier("PROPERTY".to_string())),
        ("Property", Token::Identifier("Property".to_string())),
    ];

    for (input, expected) in test_cases {
        let mut lexer = Token::lexer(input);
        let token = lexer
            .next()
            .unwrap_or_else(|| panic!("Failed to tokenize '{input}'"));
        assert_eq!(
            token,
            Ok(expected.clone()),
            "Input '{input}' should tokenize to {expected:?}"
        );
    }
}

// Escape sequence tests
#[test]
fn test_parse_string_newline_escape() {
    use logos::Logos;
    let mut lexer = Token::lexer(r#""hello\nworld""#);
    let token = lexer.next().unwrap().unwrap();
    match token {
        Token::StringLiteral(s) => {
            assert_eq!(s, "hello\nworld");
            assert_eq!(s.len(), 11); // includes actual newline
            assert_eq!(s.chars().nth(5), Some('\n'));
        }
        _ => panic!("Expected StringLiteral"),
    }
}

#[test]
fn test_parse_string_tab_escape() {
    use logos::Logos;
    let mut lexer = Token::lexer(r#""name:\tvalue""#);
    let token = lexer.next().unwrap().unwrap();
    match token {
        Token::StringLiteral(s) => {
            assert_eq!(s, "name:\tvalue");
            assert_eq!(s.len(), 11);
            assert!(s.contains('\t'));
        }
        _ => panic!("Expected StringLiteral"),
    }
}

#[test]
fn test_parse_string_carriage_return_escape() {
    use logos::Logos;
    let mut lexer = Token::lexer(r#""line1\rline2""#);
    let token = lexer.next().unwrap().unwrap();
    match token {
        Token::StringLiteral(s) => {
            assert_eq!(s, "line1\rline2");
            assert_eq!(s.chars().nth(5), Some('\r'));
        }
        _ => panic!("Expected StringLiteral"),
    }
}

#[test]
fn test_parse_string_backslash_escape() {
    use logos::Logos;
    let mut lexer = Token::lexer(r#""path\\to\\file""#);
    let token = lexer.next().unwrap().unwrap();
    match token {
        Token::StringLiteral(s) => {
            assert_eq!(s, "path\\to\\file");
            assert_eq!(s.len(), 12);
        }
        _ => panic!("Expected StringLiteral"),
    }
}

#[test]
fn test_parse_string_null_escape() {
    use logos::Logos;
    let mut lexer = Token::lexer(r#""text\0end""#);
    let token = lexer.next().unwrap().unwrap();
    match token {
        Token::StringLiteral(s) => {
            assert_eq!(s, "text\0end");
            assert_eq!(s.len(), 8);
            assert_eq!(s.chars().nth(4), Some('\0'));
        }
        _ => panic!("Expected StringLiteral"),
    }
}

#[test]
fn test_parse_string_double_quote_escape() {
    use logos::Logos;
    let mut lexer = Token::lexer(r#""say \"hello\"""#);
    let token = lexer.next().unwrap().unwrap();
    match token {
        Token::StringLiteral(s) => {
            assert_eq!(s, r#"say "hello""#);
        }
        _ => panic!("Expected StringLiteral"),
    }
}

#[test]
fn test_parse_string_backslash_n_literal() {
    use logos::Logos;
    // \\n should be backslash followed by 'n', not a newline
    let mut lexer = Token::lexer(r#""path\\nfile""#);
    let token = lexer.next().unwrap().unwrap();
    match token {
        Token::StringLiteral(s) => {
            assert_eq!(s, "path\\nfile");
            assert_eq!(s.len(), 10);
            assert_eq!(s.chars().nth(4), Some('\\'));
            assert_eq!(s.chars().nth(5), Some('n'));
        }
        _ => panic!("Expected StringLiteral"),
    }
}

#[test]
fn test_parse_string_multiple_escapes() {
    use logos::Logos;
    let mut lexer = Token::lexer(r#""line1\nline2\ttab\r\nend""#);
    let token = lexer.next().unwrap().unwrap();
    match token {
        Token::StringLiteral(s) => {
            assert_eq!(s, "line1\nline2\ttab\r\nend");
            assert!(s.contains('\n'));
            assert!(s.contains('\t'));
            assert!(s.contains('\r'));
        }
        _ => panic!("Expected StringLiteral"),
    }
}

#[test]
fn test_parse_string_escaped_backslash_before_escape() {
    use logos::Logos;
    // \\\n should be backslash followed by newline (not backslash-backslash-n)
    let mut lexer = Token::lexer(r#""a\\\nb""#);
    let token = lexer.next().unwrap().unwrap();
    match token {
        Token::StringLiteral(s) => {
            assert_eq!(s, "a\\\nb");
            assert_eq!(s.len(), 4); // a, \, newline, b
        }
        _ => panic!("Expected StringLiteral"),
    }
}

#[test]
fn test_parse_string_no_escapes() {
    use logos::Logos;
    let mut lexer = Token::lexer(r#""plain text""#);
    let token = lexer.next().unwrap().unwrap();
    match token {
        Token::StringLiteral(s) => {
            assert_eq!(s, "plain text");
            assert_eq!(s.len(), 10);
        }
        _ => panic!("Expected StringLiteral"),
    }
}

#[test]
fn test_parse_string_invalid_escape() {
    use logos::Logos;
    let mut lexer = Token::lexer(r#""test\x""#);
    let token = lexer.next();
    // Should be ERROR token due to invalid escape
    assert!(token.is_some());
    match token.unwrap() {
        Ok(Token::StringLiteral(_)) => panic!("Should have errored on invalid escape"),
        Err(_) => { /* Expected - invalid escape */ }
        _ => panic!("Expected error token"),
    }
}

// Phase 2: Eol Token Tests

#[test]
fn test_eol_emission() {
    use crate::lexer::lex_wfl_with_positions;
    let input = "store x as 5\nstore y as 10\n";
    let tokens = lex_wfl_with_positions(input);

    // Count Eol tokens (should be 2)
    let eol_count = tokens
        .iter()
        .filter(|t| matches!(t.token, Token::Eol))
        .count();
    assert_eq!(eol_count, 2, "Should have 2 Eol tokens for 2 newlines");
}

#[test]
fn test_multiword_identifier_with_eol() {
    use crate::lexer::lex_wfl_with_positions;
    let input = "store user name as \"Alice\"\n";
    let tokens = lex_wfl_with_positions(input);

    // Should have: store, "user name", as, "Alice", Eol
    assert!(
        tokens
            .iter()
            .any(|t| { matches!(&t.token, Token::Identifier(s) if s == "user name") }),
        "Multi-word identifier should be preserved"
    );

    assert!(
        tokens.iter().any(|t| matches!(t.token, Token::Eol)),
        "Eol token should be emitted after newline"
    );
}

#[test]
fn test_consecutive_eol() {
    use crate::lexer::lex_wfl_with_positions;
    let input = "store x as 5\n\n\nstore y as 10\n";
    let tokens = lex_wfl_with_positions(input);

    // Should emit Eol for each newline
    let eol_count = tokens
        .iter()
        .filter(|t| matches!(t.token, Token::Eol))
        .count();
    assert_eq!(
        eol_count, 4,
        "Should emit Eol for each newline (including blank lines)"
    );
}

#[test]
fn test_line_comment_with_cr_ending() {
    let input = "store x as 5 // comment\rstore y as 10";
    let tokens = lex_wfl(input);

    // Should have: store, x, as, 5, Eol, store, y, as, 10
    // Verify exactly ONE Eol token between the statements
    let eol_count = tokens.iter().filter(|t| matches!(t, Token::Eol)).count();
    assert_eq!(eol_count, 1, "Should have exactly one Eol token");

    // Verify both statements are properly tokenized
    let store_count = tokens
        .iter()
        .filter(|t| matches!(t, Token::KeywordStore))
        .count();
    assert_eq!(store_count, 2, "Should have two store keywords");
}

#[test]
fn test_line_comment_with_lf_ending() {
    let input = "store x as 5 // comment\nstore y as 10";
    let tokens = lex_wfl(input);

    let eol_count = tokens.iter().filter(|t| matches!(t, Token::Eol)).count();
    assert_eq!(eol_count, 1);

    let store_count = tokens
        .iter()
        .filter(|t| matches!(t, Token::KeywordStore))
        .count();
    assert_eq!(store_count, 2);
}

#[test]
fn test_line_comment_with_crlf_ending() {
    let input = "store x as 5 // comment\r\nstore y as 10";
    let tokens = lex_wfl(input);

    let eol_count = tokens.iter().filter(|t| matches!(t, Token::Eol)).count();
    assert_eq!(eol_count, 1);

    let store_count = tokens
        .iter()
        .filter(|t| matches!(t, Token::KeywordStore))
        .count();
    assert_eq!(store_count, 2);
}

#[test]
fn test_hash_comment_with_cr_ending() {
    let input = "store x as 5 # comment\rstore y as 10";
    let tokens = lex_wfl(input);

    let eol_count = tokens.iter().filter(|t| matches!(t, Token::Eol)).count();
    assert_eq!(eol_count, 1);

    let store_count = tokens
        .iter()
        .filter(|t| matches!(t, Token::KeywordStore))
        .count();
    assert_eq!(store_count, 2);
}

#[test]
fn test_mixed_line_endings_with_comments() {
    let input = "store x as 5 // comment\rstore y as 10 # comment\nstore z as 15";
    let tokens = lex_wfl(input);

    // Should have 2 Eol tokens (one after each comment)
    let eol_count = tokens.iter().filter(|t| matches!(t, Token::Eol)).count();
    assert_eq!(eol_count, 2);

    // Should have 3 store statements
    let store_count = tokens
        .iter()
        .filter(|t| matches!(t, Token::KeywordStore))
        .count();
    assert_eq!(store_count, 3);
}

#[test]
fn test_comment_at_eof_no_newline() {
    let input = "store x as 5 // comment at end";
    let tokens = lex_wfl(input);

    // Should have: store, x, as, 5
    // No Eol token at the end
    let eol_count = tokens.iter().filter(|t| matches!(t, Token::Eol)).count();
    assert_eq!(eol_count, 0, "Should not have Eol at EOF without newline");

    let store_count = tokens
        .iter()
        .filter(|t| matches!(t, Token::KeywordStore))
        .count();
    assert_eq!(store_count, 1);
}

#[test]
fn test_empty_comment_with_cr() {
    let input = "store x as 5 //\rstore y as 10";
    let tokens = lex_wfl(input);

    let eol_count = tokens.iter().filter(|t| matches!(t, Token::Eol)).count();
    assert_eq!(eol_count, 1);

    let store_count = tokens
        .iter()
        .filter(|t| matches!(t, Token::KeywordStore))
        .count();
    assert_eq!(store_count, 2);
}

// Issue: Identifiers containing keywords (like content_type) should be tokenized as single identifiers
#[test]
fn test_keyword_with_underscore_becomes_identifier() {
    use crate::lexer::lex_wfl_with_positions;

    // "content_type" should be tokenized as a single identifier, not KeywordContent + error
    let input = "store content_type as \"text/html\"";
    let tokens = lex_wfl_with_positions(input);

    // Should have: store, content_type, as, "text/html"
    let has_content_type_identifier = tokens
        .iter()
        .any(|t| matches!(&t.token, Token::Identifier(s) if s == "content_type"));
    assert!(
        has_content_type_identifier,
        "content_type should be tokenized as a single identifier. Got tokens: {:?}",
        tokens.iter().map(|t| &t.token).collect::<Vec<_>>()
    );

    // Should NOT have KeywordContent
    let has_keyword_content = tokens
        .iter()
        .any(|t| matches!(t.token, Token::KeywordContent));
    assert!(
        !has_keyword_content,
        "Should not have KeywordContent when followed by underscore"
    );
}

#[test]
fn test_multiple_keywords_with_underscores() {
    use crate::lexer::lex_wfl_with_positions;

    // Test various keyword_suffix patterns
    let test_cases = vec![
        ("file_path", "file_path"),
        ("is_active", "is_active"),
        ("on_click", "on_click"),
        ("data_type", "data_type"),
        ("time_value", "time_value"),
        ("status_code", "status_code"),
    ];

    for (input, expected_id) in test_cases {
        let tokens = lex_wfl_with_positions(input);
        let has_identifier = tokens
            .iter()
            .any(|t| matches!(&t.token, Token::Identifier(s) if s == expected_id));
        assert!(
            has_identifier,
            "'{input}' should be tokenized as identifier '{expected_id}'. Got tokens: {:?}",
            tokens.iter().map(|t| &t.token).collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_keyword_without_underscore_stays_keyword() {
    use crate::lexer::lex_wfl_with_positions;

    // "content" alone should still be a keyword
    let input = "content";
    let tokens = lex_wfl_with_positions(input);

    let has_keyword_content = tokens
        .iter()
        .any(|t| matches!(t.token, Token::KeywordContent));
    assert!(
        has_keyword_content,
        "Standalone 'content' should be KeywordContent"
    );
}

#[test]
fn test_keyword_in_context_vs_identifier() {
    use crate::lexer::lex_wfl_with_positions;

    // In this input, "content" should be keyword but "content_type" should be identifier
    let input = "store content_type as content";
    let tokens = lex_wfl_with_positions(input);

    // Should have content_type as identifier
    let has_content_type = tokens
        .iter()
        .any(|t| matches!(&t.token, Token::Identifier(s) if s == "content_type"));
    assert!(has_content_type, "Should have content_type as identifier");

    // Should have content as keyword
    let has_content_keyword = tokens
        .iter()
        .any(|t| matches!(t.token, Token::KeywordContent));
    assert!(
        has_content_keyword,
        "Should have standalone 'content' as keyword"
    );
}

#[test]
fn test_boolean_literal_values() {
    use crate::lexer::lex_wfl_with_positions;

    let test_cases = [
        ("yes", true),
        ("no", false),
        ("true", true),
        ("false", false),
        ("YES", true),
        ("NO", false),
        ("True", true),
        ("False", false),
        ("YeS", true),
    ];

    for (input, expected) in test_cases {
        let tokens = lex_wfl_with_positions(input);
        assert!(!tokens.is_empty(), "Failed to lex input: {}", input);

        match &tokens[0].token {
            Token::BooleanLiteral(val) => {
                assert_eq!(*val, expected, "Failed for input: {}", input);
            }
            other => {
                panic!("Expected BooleanLiteral for input '{}', got {:?}", input, other);
            }
        }
    }
}
