//! Pins how WFL counts down, and that it has no `downward` or `upward`
//! keyword.
//!
//! A count loop counts down with `count from 5 down to 1:`. `down` is not a
//! keyword: the lexer produces an ordinary identifier, and `parse_count_loop`
//! recognizes it only between the start value and `to`. The keyword reference
//! pages once listed `downward` and `upward` as reserved count-direction
//! keywords, but `src/lexer/token.rs` never defined either word.
//! `Docs/reference/keyword-reference.md` and
//! `Docs/reference/reserved-keywords.md` describe `down to` instead. Changing
//! any of this is a user-visible change: update both pages, the "Counting
//! Down" section of the loops guide, and this test together.
//!
//! `keyword_reference_drift_test.rs` already fails if either page lists a word
//! the lexer does not define, so this file only covers `down to` itself.

use std::path::PathBuf;

use wfl::lexer::lex_wfl_with_positions;
use wfl::lexer::token::Token;
use wfl::parser::Parser;
use wfl::parser::ast::{Expression, Literal, ParseError, Statement};

/// Both keyword pages; `CLAUDE.md` says they are updated together.
const KEYWORD_PAGES: [&str; 2] = [
    "Docs/reference/keyword-reference.md",
    "Docs/reference/reserved-keywords.md",
];

/// Count-direction words the keyword pages used to list as keywords.
const NOT_KEYWORDS: [&str; 2] = ["downward", "upward"];

fn parse(code: &str) -> Result<Vec<Statement>, Vec<ParseError>> {
    let tokens = lex_wfl_with_positions(code);
    Parser::new(&tokens)
        .parse()
        .map(|program| program.statements)
}

/// Reads a repository file with LF line endings, so a Windows checkout with
/// `core.autocrlf=true` compares the same as a Linux one.
fn read_lf(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {relative}: {e}"))
        .replace("\r\n", "\n")
}

#[test]
fn down_downward_and_upward_lex_as_plain_identifiers() {
    for word in ["down", "downward", "upward"] {
        let tokens = lex_wfl_with_positions(word);
        assert!(
            matches!(&tokens[0].token, Token::Identifier(name) if name == word),
            "`{word}` should lex as an identifier, got {:?}",
            tokens[0].token
        );
    }
}

#[test]
fn down_downward_and_upward_are_ordinary_variable_names() {
    for word in ["down", "downward", "upward"] {
        let code = format!("store {word} as 1\ndisplay {word}");
        let statements =
            parse(&code).unwrap_or_else(|e| panic!("{word} should be a valid name: {e:?}"));
        match &statements[0] {
            Statement::VariableDeclaration { name, .. } => {
                assert_eq!(name, word, "{word} should be stored under its own name")
            }
            other => panic!("expected a variable declaration for {word}, got {other:?}"),
        }
    }
}

#[test]
fn down_to_parses_as_a_downward_count_loop() {
    let statements = parse("count from 5 down to 1:\n    display count\nend count")
        .unwrap_or_else(|e| panic!("`count from 5 down to 1` must parse: {e:?}"));
    match &statements[0] {
        Statement::CountLoop {
            start,
            end,
            downward,
            ..
        } => {
            assert!(*downward, "`down to` must make the loop count down");
            assert!(
                matches!(start, Expression::Literal(Literal::Integer(5), ..)),
                "start should be 5, got {start:?}"
            );
            assert!(
                matches!(end, Expression::Literal(Literal::Integer(1), ..)),
                "end should be 1, got {end:?}"
            );
        }
        other => panic!("expected a count loop, got {other:?}"),
    }
}

#[test]
fn downward_and_upward_are_not_count_directions() {
    for word in NOT_KEYWORDS {
        let code = format!("count from 5 {word} to 1:\n    display count\nend count");
        let errors = match parse(&code) {
            Ok(statements) => panic!("`{word}` must not parse as a direction: {statements:?}"),
            Err(errors) => errors,
        };
        assert!(
            errors[0].message.contains("Expected 'to' or 'down to'"),
            "the error for `{word}` should suggest `down to`: {}",
            errors[0].message
        );
    }
}

#[test]
fn keyword_pages_point_readers_at_down_to() {
    for page in KEYWORD_PAGES {
        let doc = read_lf(page);
        assert!(
            doc.contains("count from 5 down to 1"),
            "{page} should show the real count-down form"
        );
        assert!(
            doc.contains("loops-and-iteration.md#counting-down"),
            "{page} should link to the loops guide's Counting Down section"
        );
    }
}

#[test]
fn docs_count_down_blocks_match_validated_examples() {
    for (page, example) in [
        (
            "Docs/03-language-basics/loops-and-iteration.md",
            "TestPrograms/docs_examples/basic_syntax/count_down_01.wfl",
        ),
        (
            "Docs/reference/reserved-keywords.md",
            "TestPrograms/docs_examples/keyword_reference/down_is_not_reserved.wfl",
        ),
    ] {
        let doc = read_lf(page);
        let code = read_lf(example);
        assert!(
            doc.contains(&format!("```wfl\n{code}```")),
            "{page} should contain {example} verbatim as a wfl code block"
        );
    }
}
