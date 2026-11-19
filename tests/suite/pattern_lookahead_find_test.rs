// Lookahead through `find` / `find all` (characterization).
//
// Patterns without a backreference are searched by the lockstep Pike VM
// (`find_at_position_pike`), which evaluates `check ahead for` and
// `check not ahead for` by copying the lookahead body into a sub-program
// (`lookaround_subprogram_matches`). Before these tests, nothing exercised
// that path: the existing lookahead coverage goes through `matches`
// (`execute`, the backtracking `step` path), and deliberately skipping the
// first instruction of the copied body left every test green.
//
// These tests pin the current results for positive and negative lookahead,
// a lookahead body of more than one instruction, and `find all`, both
// through the pattern API (so the Pike path is used for certain) and through
// WFL source.

use wfl::interpreter::value::Value;
use wfl::parser::ast::CharClass;
use wfl::pattern::{MatchResult, PatternCompiler, PatternExpression, PatternVM};

use crate::common::run_wfl_code;

fn digit() -> PatternExpression {
    PatternExpression::CharacterClass(CharClass::Digit)
}

fn letter() -> PatternExpression {
    PatternExpression::CharacterClass(CharClass::Letter)
}

fn ahead(body: PatternExpression) -> PatternExpression {
    PatternExpression::Lookahead(Box::new(body))
}

fn not_ahead(body: PatternExpression) -> PatternExpression {
    PatternExpression::NegativeLookahead(Box::new(body))
}

fn find(pattern: &PatternExpression, text: &str) -> Option<MatchResult> {
    let program = PatternCompiler::new()
        .compile(pattern)
        .expect("pattern compiles");
    assert!(
        !program.contains_backreference(),
        "these tests must use the Pike path, which only runs without backreferences"
    );
    PatternVM::new()
        .try_find(&program, text, &[])
        .expect("find does not hit a budget")
}

fn find_all(pattern: &PatternExpression, text: &str) -> Vec<MatchResult> {
    let program = PatternCompiler::new()
        .compile(pattern)
        .expect("pattern compiles");
    PatternVM::new()
        .try_find_all(&program, text, &[])
        .expect("find all does not hit a budget")
}

fn span(m: &MatchResult) -> (usize, usize, &str) {
    (m.start, m.end, m.matched_text.as_str())
}

#[test]
fn find_with_lookahead_skips_positions_whose_lookahead_fails() {
    // "5" is followed by "9" and "9" by a space; only "7" is followed by a letter.
    let pattern = PatternExpression::Sequence(vec![digit(), ahead(letter())]);
    let found = find(&pattern, "59 7b").expect("a digit followed by a letter exists");
    assert_eq!(
        span(&found),
        (3, 4, "7"),
        "the lookahead is checked but not consumed"
    );

    assert!(
        find(&pattern, "59 7 8").is_none(),
        "no digit is followed by a letter"
    );
}

#[test]
fn find_with_negative_lookahead_skips_positions_whose_lookahead_matches() {
    // "5" is followed by the letter "a", so it is skipped; "7" ends the text.
    let pattern = PatternExpression::Sequence(vec![digit(), not_ahead(letter())]);
    let found = find(&pattern, "5a 7").expect("a digit not followed by a letter exists");
    assert_eq!(span(&found), (3, 4, "7"));

    assert!(
        find(&pattern, "5a6b").is_none(),
        "every digit is followed by a letter"
    );
}

#[test]
fn find_with_a_multi_part_lookahead_checks_every_part() {
    // The lookahead body is two parts: the letter "a", then the letter "b".
    let body = PatternExpression::Sequence(vec![
        PatternExpression::Literal("a".to_string()),
        PatternExpression::Literal("b".to_string()),
    ]);
    let pattern = PatternExpression::Sequence(vec![
        PatternExpression::Literal("x".to_string()),
        ahead(body),
    ]);

    // The first "x" is followed by "a " (fails the second part); the second by "ab".
    let found = find(&pattern, "xa xab").expect("an x followed by ab exists");
    assert_eq!(span(&found), (3, 4, "x"));

    assert!(
        find(&pattern, "xb xa").is_none(),
        "neither x is followed by both parts"
    );
}

#[test]
fn find_all_with_lookahead_returns_only_qualifying_positions() {
    let pattern = PatternExpression::Sequence(vec![digit(), ahead(letter())]);
    let found: Vec<_> = find_all(&pattern, "1a 2 3b")
        .iter()
        .map(|m| (m.start, m.end, m.matched_text.clone()))
        .collect();
    assert_eq!(
        found,
        vec![(0, 1, "1".to_string()), (5, 6, "3".to_string())],
        "digits followed by a letter, in order"
    );
}

async fn wfl_number(code: &str) -> f64 {
    match run_wfl_code(code).await {
        Ok(Value::Number(n)) => n,
        other => panic!("expected a number result, got {other:?}"),
    }
}

#[tokio::test]
async fn wfl_find_uses_check_ahead_for() {
    let code = r#"
        create pattern digit_before_letter:
            digit check ahead for {letter}
        end pattern
        store first_hit as find digit_before_letter in "59 7b"
        store result as first_hit["start"]
    "#;
    assert_eq!(wfl_number(code).await, 3.0);
}

#[tokio::test]
async fn wfl_find_uses_check_not_ahead_for() {
    let code = r#"
        create pattern digit_not_before_letter:
            digit check not ahead for {letter}
        end pattern
        store first_hit as find digit_not_before_letter in "5a 7"
        store result as first_hit["start"]
    "#;
    assert_eq!(wfl_number(code).await, 3.0);
}

#[tokio::test]
async fn wfl_find_all_uses_check_ahead_for() {
    let code = r#"
        create pattern digit_before_letter:
            digit check ahead for {letter}
        end pattern
        store every_hit as find all digit_before_letter in "1a 2 3b"
        store second_hit as every_hit[1]
        store hit_count as length of every_hit
        store result as hit_count times 100 plus second_hit["start"]
    "#;
    // Two matches; the second starts at character 5.
    assert_eq!(wfl_number(code).await, 205.0);
}
