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
