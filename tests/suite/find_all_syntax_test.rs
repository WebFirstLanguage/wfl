//! `find all <pattern> in <text>` — the documented all-matches spelling.
//!
//! `Docs/05-standard-library/pattern-module.md` and the builtin reference
//! promise `find all <pattern> in <text>`, but the lexer merges `all word` into
//! one identifier and the `find` parser had no `all` handling, so the
//! documented program failed with "Variable 'all word' is not defined" (exit
//! 3). These tests drive the real `wfl` binary so the documented form is
//! checked end to end: it must return every non-overlapping match, exactly
//! like `pattern_find_all of <text> and <pattern>`.
//!
//! The same tokens used to mean a single-match `find` over a binding literally
//! named `all <name>` (or `all`). Programs that rely on such a binding are
//! valid today, so the new form must not steal them (testing.md §11.6): the
//! compatibility tests below pin the legacy reading whenever that full-name
//! binding exists.

use std::fs;
use std::process::Command;

use wfl::fixer::CodeFixer;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::Statement;

use crate::common::{run_src, wfl_exe};

const WORD_PATTERN: &str = "create pattern word:\n    one or more letter\nend pattern\n";

#[test]
fn documented_find_all_spelling_returns_every_match() {
    // The exact reproduction from the defect report.
    let src = format!(
        "{WORD_PATTERN}\
         store text_value as \"The quick fox\"\n\
         store word_matches as find all word in text_value\n\
         display \"count: \" with length of word_matches\n"
    );
    let (out, code) = run_src(&src);
    assert!(
        !out.contains("is not defined"),
        "`find all word in ...` must not be read as a variable named `all word`: {out}"
    );
    assert!(
        out.contains("count: 3"),
        "expected three word matches: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn find_all_matches_carry_text_and_positions_in_order() {
    let src = format!(
        "{WORD_PATTERN}\
         store word_matches as find all word in \"The quick fox\"\n\
         for each item in word_matches:\n\
         \x20\x20\x20\x20display item[\"matched_text\"] with \"@\" with item[\"start\"] with \"-\" with item[\"end\"]\n\
         end for\n"
    );
    let (out, code) = run_src(&src);
    let lines: Vec<&str> = out.lines().filter(|l| l.contains('@')).collect();
    assert_eq!(
        lines,
        vec!["The@0-3", "quick@4-9", "fox@10-13"],
        "matches must be whole words, in order, with start/end offsets: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn find_all_agrees_with_pattern_find_all() {
    let src = format!(
        "{WORD_PATTERN}\
         store sample as \"one two three four\"\n\
         store via_phrase as find all word in sample\n\
         store via_function as pattern_find_all of sample and word\n\
         display \"phrase: \" with length of via_phrase\n\
         display \"function: \" with length of via_function\n\
         check if via_phrase[3][\"matched_text\"] is equal to via_function[3][\"matched_text\"]:\n\
         \x20\x20\x20\x20display \"same last match\"\n\
         end check\n"
    );
    let (out, code) = run_src(&src);
    assert!(out.contains("phrase: 4"), "phrase form must find 4: {out}");
    assert!(
        out.contains("function: 4"),
        "function form must find 4: {out}"
    );
    assert!(
        out.contains("same last match"),
        "both forms must return the same matches: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn find_all_accepts_a_multi_word_pattern_name() {
    // `all phone number` lexes as one identifier; the pattern is `phone number`.
    let src = "create pattern phone number:\n\
               \x20\x20\x20\x20digit digit digit followed by \"-\" followed by digit digit digit digit\n\
               end pattern\n\
               store numbers as find all phone number in \"call 555-1234 or 555-9876\"\n\
               display \"phones: \" with length of numbers\n\
               display \"second: \" with numbers[1][\"matched_text\"]\n";
    let (out, code) = run_src(src);
    assert!(
        out.contains("phones: 2"),
        "expected two phone numbers: {out}"
    );
    assert!(
        out.contains("second: 555-9876"),
        "expected the second number: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn find_all_with_no_matches_is_an_empty_list_not_nothing() {
    let src = "create pattern digits:\n    one or more digit\nend pattern\n\
               store hits as find all digits in \"no numbers here\"\n\
               display \"found: \" with length of hits\n";
    let (out, code) = run_src(src);
    assert!(
        out.contains("found: 0"),
        "no matches must be an empty list (length 0): {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn find_all_works_directly_as_a_for_each_collection() {
    let src = format!(
        "{WORD_PATTERN}\
         for each item in find all word in \"red green blue\":\n\
         \x20\x20\x20\x20display \"colour: \" with item[\"matched_text\"]\n\
         end for\n"
    );
    let (out, code) = run_src(&src);
    assert!(
        out.contains("colour: red")
            && out.contains("colour: green")
            && out.contains("colour: blue"),
        "each match must be visited: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn find_all_with_a_bare_all_and_a_parenthesized_pattern() {
    // `all` stays a separate token when the pattern operand does not start
    // with a plain word, so the bare-`all` path must handle it too.
    let src = format!(
        "{WORD_PATTERN}\
         store hits as find all (word) in \"alpha beta\"\n\
         display \"found: \" with length of hits\n"
    );
    let (out, code) = run_src(&src);
    assert!(out.contains("found: 2"), "expected two matches: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn find_all_rejects_a_non_pattern_operand() {
    let src = "store needle as 42\n\
               store hits as find all needle in \"a 42 b\"\n\
               display \"UNREACHABLE\"\n";
    let (out, code) = run_src(src);
    assert_ne!(code, Some(0), "a non-pattern operand must fail: {out}");
    assert!(
        out.contains("Pattern"),
        "the diagnostic must say a pattern was expected: {out}"
    );
    assert!(!out.contains("UNREACHABLE"), "must not run on: {out}");
}

#[test]
fn find_all_without_in_is_a_parse_error_naming_in() {
    let src = format!("{WORD_PATTERN}store hits as find all word\n");
    let (out, code) = run_src(&src);
    assert_ne!(code, Some(0), "missing `in` must fail: {out}");
    assert!(
        out.contains("'in'"),
        "the parse error must name the missing `in`: {out}"
    );
}

#[test]
fn find_all_is_bounded_by_the_pattern_step_budget() {
    // A low step ceiling must surface as a catchable resource error, never as
    // a silently short (or empty) match list.
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join(".wflcfg"), "max_pattern_steps = 3\n").expect("cfg");
    let script = dir.path().join("main.wfl");
    fs::write(
        &script,
        "create pattern digits:\n    one or more digit\nend pattern\n\
         try:\n\
         \x20\x20\x20\x20store hits as find all digits in \"123 456 789\"\n\
         \x20\x20\x20\x20display \"FOUND \" with length of hits\n\
         catch:\n\
         \x20\x20\x20\x20display \"CAUGHT\"\n\
         end try\n",
    )
    .expect("program");
    let output = Command::new(wfl_exe())
        .arg(&script)
        .current_dir(dir.path())
        .output()
        .expect("run wfl");
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        out.contains("CAUGHT"),
        "budget breach must be catchable: {out}"
    );
    assert!(
        !out.contains("FOUND"),
        "a budget breach must not yield a partial list: {out}"
    );
}

#[test]
fn parser_lowers_merged_all_to_a_find_all_over_the_split_off_name() {
    let src = "store word_matches as find all word in text_value\n";
    let tokens = lex_wfl_with_positions(src);
    let program = Parser::new(&tokens).parse().expect("parse");
    let Some(Statement::VariableDeclaration { value, .. }) = program.statements.first() else {
        panic!("expected a store statement, got {:?}", program.statements);
    };
    let shape = format!("{value:?}");
    assert!(
        shape.starts_with("PatternFindAll"),
        "`find all word in ...` must parse as a find-all, got {shape}"
    );
    assert!(
        shape.contains("pattern: Variable(\"word\""),
        "the pattern operand must be the split-off name `word`, got {shape}"
    );
    assert!(
        shape.contains("text: Variable(\"text_value\""),
        "the text operand must follow `in`, got {shape}"
    );
    assert!(
        shape.contains("legacy_binding: Some(\"all word\")"),
        "the merged name must be kept for the backward-compatible reading, got {shape}"
    );
}

#[test]
fn fixer_keeps_a_find_all_operand_in_step_with_its_declaration() {
    // The lexer merges `all Word Finder` into one token, so the fixer's
    // whole-token snake_case rename cannot update that use. It must leave the
    // declaration's spelling alone too, as it does for `exit ... with code`.
    let src = format!(
        "{WORD_PATTERN}\
         store Word Finder as word\n\
         store hits as find all Word Finder in \"alpha beta\"\n\
         display \"count: \" with length of hits\n"
    );
    let program = Parser::new(&lex_wfl_with_positions(&src))
        .parse()
        .expect("parse");
    let (fixed, _) = CodeFixer::new()
        .fix_checked(&program, &src)
        .expect("formatting valid source must succeed");
    assert!(
        fixed.contains("store Word Finder as word"),
        "a name used as a merged `find all` operand must keep its spelling:\n{fixed}"
    );
    let (out, code) = run_src(&fixed);
    assert!(
        out.contains("count: 2"),
        "the fixed program must still find both words:\n{fixed}\n{out}"
    );
    assert_eq!(code, Some(0), "fixed program should exit 0: {out}");
}

// --- backward compatibility (testing.md §11.6) -----------------------------
//
// These programs are valid before `find all` exists and must keep their
// single-match meaning afterwards.

#[test]
fn pattern_named_all_digits_keeps_its_single_match_meaning() {
    // A competing `digits` pattern makes both readings viable; the existing
    // full-name binding `all digits` must win.
    let src = "create pattern digits:\n    one or more letter\nend pattern\n\
               create pattern all digits:\n    one or more digit\nend pattern\n\
               store hit as find all digits in \"ab 12 cd 345\"\n\
               display \"legacy: \" with hit[\"matched_text\"]\n";
    let (out, code) = run_src(src);
    assert!(
        out.contains("legacy: 12"),
        "`find all digits` must still find the first match of `all digits`: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn pattern_named_all_digits_alone_keeps_its_single_match_meaning() {
    // No `digits` binding at all: the legacy reading must not be reported as
    // an undefined `digits` by the analyzer or type checker.
    let src = "create pattern all digits:\n    one or more digit\nend pattern\n\
               store hit as find all digits in \"ab 12 cd 345\"\n\
               display \"legacy: \" with hit[\"matched_text\"]\n";
    let (out, code) = run_src(src);
    assert!(
        !out.contains("is not defined"),
        "the legacy binding must be recognised: {out}"
    );
    assert!(out.contains("legacy: 12"), "expected the first run: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn pattern_named_all_keeps_its_single_match_meaning() {
    let src = "create pattern all:\n    one or more digit\nend pattern\n\
               store hit as find all in \"ab 12 cd 345\"\n\
               display \"legacy: \" with hit[\"matched_text\"]\n";
    let (out, code) = run_src(src);
    assert!(out.contains("legacy: 12"), "expected the first run: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn list_named_all_patterns_keeps_its_indexed_single_match_meaning() {
    // Postfix on the merged name: `all patterns[1]` indexed the list binding
    // `all patterns`; with that binding present it still must.
    let src = format!(
        "{WORD_PATTERN}\
         create pattern digits:\n    one or more digit\nend pattern\n\
         store all patterns as [word and digits]\n\
         store hit as find all patterns[1] in \"ab 12 cd 345\"\n\
         display \"legacy: \" with hit[\"matched_text\"]\n"
    );
    let (out, code) = run_src(&src);
    assert!(
        out.contains("legacy: 12"),
        "expected the first digit run: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}
