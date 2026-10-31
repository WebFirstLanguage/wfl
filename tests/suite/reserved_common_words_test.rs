//! Pins the documented behavior of everyday words that WFL reserves:
//! `start`, `output`, `route`, `captured`, `by`, and `find`.
//!
//! `Docs/reference/reserved-keywords.md` ("What Is a Reserved Word?" and
//! "Words That Commonly Bite") promises that none of these words can be a
//! name the programmer chooses, that the error names the word as a
//! `Keyword...` token at the word's position, that the suggested underscore
//! names work, and that `read output from process ... as output` is the one
//! narrow exception. `CLAUDE.md` points agents at those sections. Changing any
//! of this behavior is a user-visible change: update the docs and the
//! `CLAUDE.md` pointer in the same change as this test.

use std::path::PathBuf;

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{ParseError, Statement};

/// Each word with the token name that parse errors report for it.
const COMMON_RESERVED: [(&str, &str); 6] = [
    ("start", "KeywordStart"),
    ("output", "KeywordOutput"),
    ("route", "KeywordRoute"),
    ("captured", "KeywordCaptured"),
    ("by", "KeywordBy"),
    ("find", "KeywordFind"),
];

/// Every replacement name the reserved-word docs suggest, including the
/// multi-word pitfall fixes (`program_output`, `last_route`).
const SUGGESTED_NAMES: [&str; 20] = [
    "start_time",
    "start_index",
    "starting_value",
    "command_output",
    "process_output",
    "result_text",
    "program_output",
    "current_route",
    "request_path",
    "route_name",
    "last_route",
    "captured_text",
    "matched_word",
    "capture_value",
    "step_size",
    "divisor",
    "separator",
    "search_term",
    "found_item",
    "find_result",
];

fn parse(code: &str) -> Result<Vec<Statement>, Vec<ParseError>> {
    let tokens = lex_wfl_with_positions(code);
    Parser::new(&tokens)
        .parse()
        .map(|program| program.statements)
}

/// Asserts `code` fails to parse and that the first error names `token` at
/// the given 1-based line and column (where the reserved word starts).
fn assert_rejected_at(code: &str, token: &str, line: usize, column: usize) {
    let errors = match parse(code) {
        Ok(_) => panic!("expected a parse error for {code:?}"),
        Err(errors) => errors,
    };
    let first = &errors[0];
    assert!(
        first.message.contains(token),
        "error for {code:?} should name {token}: {}",
        first.message
    );
    assert_eq!(
        (first.line, first.column),
        (line, column),
        "error for {code:?} should point at the reserved word: {}",
        first.message
    );
}

#[test]
fn common_words_are_rejected_in_every_naming_position() {
    for (word, token) in COMMON_RESERVED {
        // `store <word> as 1`
        assert_rejected_at(&format!("store {word} as 1"), token, 1, 7);
        // `for each <word> in items:`
        assert_rejected_at(
            &format!("store items as [1, 2]\nfor each {word} in items:\n    display 1\nend for"),
            token,
            2,
            10,
        );
        // Action parameter.
        assert_rejected_at(
            &format!(
                "define action called greet with parameters {word}:\n    display 1\nend action"
            ),
            token,
            1,
            44,
        );
        // Action name.
        assert_rejected_at(
            &format!("define action called {word}:\n    display 1\nend action"),
            token,
            1,
            22,
        );
    }
}

#[test]
fn common_words_break_multi_word_names() {
    for (word, token) in COMMON_RESERVED {
        // Leading word of a spaced name, as in `store start time as 9`.
        assert_rejected_at(&format!("store {word} value as 9"), token, 1, 7);
    }
    // A reserved word later in a spaced name breaks it too.
    for code in [
        "store program output as \"done\"",
        "store last route as \"/home\"",
    ] {
        assert!(parse(code).is_err(), "expected a parse error for {code:?}");
    }
}

#[test]
fn suggested_underscore_names_are_ordinary_variables() {
    for name in SUGGESTED_NAMES {
        let code = format!("store {name} as 1\ndisplay {name}");
        let statements =
            parse(&code).unwrap_or_else(|e| panic!("{name} should be a valid name: {e:?}"));
        match &statements[0] {
            Statement::VariableDeclaration { name: declared, .. } => {
                assert_eq!(declared, name, "{name} should be stored under its own name")
            }
            other => panic!("expected a variable declaration for {name}, got {other:?}"),
        }
    }
}

#[test]
fn read_output_may_bind_a_result_named_output_but_store_and_change_may_not() {
    // The documented narrow exception: existing programs name the result of
    // `read output from process` `output`, and later lines can read it.
    let exception = "wait for spawn command \"sh\" with arguments [\"-c\", \"echo hi\"] as child\n\
                     wait for read output from process child as output\n\
                     display output";
    parse(exception).unwrap_or_else(|e| panic!("read output ... as output must parse: {e:?}"));

    assert_rejected_at("store output as 1", "KeywordOutput", 1, 7);
    assert!(
        parse("change output to 2").is_err(),
        "`change output to ...` must stay a parse error"
    );
}

#[test]
fn docs_side_by_side_block_matches_validated_example() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let doc = std::fs::read_to_string(root.join("Docs/reference/reserved-keywords.md"))
        .expect("read reserved-keywords.md");
    let example = std::fs::read_to_string(
        root.join("TestPrograms/docs_examples/keyword_reference/common_reserved_words.wfl"),
    )
    .expect("read common_reserved_words.wfl");

    let section = doc
        .split("### All Six, Side by Side")
        .nth(1)
        .expect("reserved-keywords.md has an 'All Six, Side by Side' section");
    let block = section
        .split("```wfl\n")
        .nth(1)
        .and_then(|rest| rest.split("```").next())
        .expect("the section has a wfl code block");

    // Normalize line endings so a Windows checkout compares equal.
    assert_eq!(
        block.replace("\r\n", "\n"),
        example.replace("\r\n", "\n"),
        "the docs block and the validated example must stay identical"
    );
}
