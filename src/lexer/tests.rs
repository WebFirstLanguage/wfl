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
