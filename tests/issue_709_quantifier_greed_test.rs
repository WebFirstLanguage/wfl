//! #709 — unbounded quantifiers must match the longest run.
//!
//! `find`, `find all`, `split`, `replace`, and `capture` all consume the match
//! extent the pattern VM reports. The VM previously returned on the first
//! `Match` in a BFS sweep, so `one or more` / `zero or more` / `at least N`
//! came out shortest. These drive the real `wfl` binary so a silently-wrong
//! answer with exit 0 cannot hide.

mod common;
use common::run_src;

#[test]
fn find_returns_the_full_digit_run() {
    let (out, code) = run_src(
        "create pattern digits:\n    one or more digit\nend pattern\n\
         store hit as find digits in \"a12345b\"\n\
         display \"find: [\" with hit[\"matched_text\"] with \"]\"\n",
    );
    assert!(
        out.contains("find: [12345]"),
        "find must return the whole run, not the first digit: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn find_all_one_or_more_letter_returns_words() {
    let (out, code) = run_src(
        "create pattern word:\n    one or more letter\nend pattern\n\
         store text as \"The quick brown fox\"\n\
         store word_matches as pattern_find_all of text and word\n\
         display \"words found: \" with length of word_matches\n",
    );
    assert!(
        out.contains("words found: 4"),
        "find all of one or more letter must extract words, not letters: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn split_on_one_or_more_whitespace_does_not_emit_empty_parts() {
    let (out, code) = run_src(
        "create pattern spaces:\n    one or more whitespace\nend pattern\n\
         store parts as split \"a  b   c\" on pattern spaces\n\
         display \"split parts: \" with length of parts\n\
         display \"first: [\" with parts[0] with \"]\"\n\
         display \"second: [\" with parts[1] with \"]\"\n\
         display \"third: [\" with parts[2] with \"]\"\n",
    );
    assert!(
        out.contains("split parts: 3"),
        "split on one or more whitespace must yield 3 parts, not empties between spaces: {out}"
    );
    assert!(
        out.contains("first: [a]") && out.contains("second: [b]") && out.contains("third: [c]"),
        "split parts must be the words: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn replace_rewrites_each_digit_run_once() {
    let (out, code) = run_src(
        "create pattern digits:\n    one or more digit\nend pattern\n\
         store s as \"a1b22c333\"\n\
         display \"replace: [\" with (replace digits with \"#\" in s) with \"]\"\n",
    );
    assert!(
        out.contains("replace: [a#b#c#]"),
        "replace must rewrite each run once, not each character: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn capture_keeps_the_full_digit_run() {
    let (out, code) = run_src(
        "create pattern id:\n    capture {one or more digit} as number\nend pattern\n\
         store m as find id in \"abc12345\"\n\
         store caps as m.captures\n\
         display \"captured: [\" with caps.number with \"]\"\n",
    );
    assert!(
        out.contains("captured: [12345]"),
        "capture of one or more digit must keep the whole run: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn nested_one_or_more_with_a_shorter_inner_alt_is_greedy() {
    let (out, code) = run_src(
        "create pattern p:\n    one or more (one or more letter or letter letter)\nend pattern\n\
         store hit as find p in \"bb\"\n\
         display \"p1: [\" with hit.matched_text with \"]\"\n",
    );
    assert!(
        out.contains("p1: [bb]"),
        "inner letter-letter must not steal the greedy extent: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn at_least_n_with_an_inner_alternation_is_greedy() {
    let (out, code) = run_src(
        "create pattern p:\n    at least 2 (one or more letter or \"ab\")\nend pattern\n\
         store hit as find p in \"bab\"\n\
         display \"p2: [\" with hit.matched_text with \"]\"\n",
    );
    assert!(
        out.contains("p2: [bab]"),
        "at least 2 must keep the full greedy run: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn nested_alternation_under_one_or_more_is_greedy() {
    let (out, code) = run_src(
        "create pattern p:\n    one or more ((\"1\" or \"a1\") or \"11\")\nend pattern\n\
         store hit as find p in \"11\"\n\
         display \"p3: [\" with hit.matched_text with \"]\"\n",
    );
    assert!(
        out.contains("p3: [11]"),
        "left-first inner alt must still allow the greedy outer run: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn backreference_find_keeps_the_capture_setting_thread() {
    let (out, code) = run_src(
        "create pattern p:\n    (\"b\" or capture {letter} as c) then optional \"-\" then same as captured \"c\"\nend pattern\n\
         store hit as find p in \"bb\"\n\
         check if isnothing of hit:\n    display \"p4: NONE\"\n\
         otherwise:\n    display \"p4: [\" with hit.matched_text with \"]\"\n\
         end check\n\
         check if \"bb\" matches p:\n    display \"p4 matches: yes\"\n\
         otherwise:\n    display \"p4 matches: no\"\n\
         end check\n",
    );
    assert!(
        out.contains("p4: [bb]"),
        "find must not drop the thread that set the capture: {out}"
    );
    assert!(
        out.contains("p4 matches: yes"),
        "matches and find must agree: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn zero_or_more_empty_literal_finds_empty() {
    let (out, code) = run_src(
        "create pattern p:\n    zero or more \"\"\nend pattern\n\
         store hit as find p in \"abc\"\n\
         display \"empty: [\" with hit[\"matched_text\"] with \"]\"\n",
    );
    assert!(
        !out.contains("step limit") && !out.contains("resource"),
        "nullable star must not blow the pattern meter: {out}"
    );
    assert!(
        out.contains("empty: []"),
        "zero or more empty literal matches empty: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn zero_or_more_optional_literal_is_greedy() {
    let (out, code) = run_src(
        "create pattern p:\n    zero or more optional \"a\"\nend pattern\n\
         store hit as find p in \"aaa\"\n\
         display \"opt: [\" with hit[\"matched_text\"] with \"]\"\n\
         store parts as split \"xaaay\" on pattern p\n\
         display \"split: \" with length of parts\n\
         display \"replaced: [\" with (replace p with \"#\" in \"aaa\") with \"]\"\n",
    );
    assert!(
        !out.contains("step limit") && !out.contains("resource"),
        "nullable star must not blow the pattern meter: {out}"
    );
    assert!(
        out.contains("opt: [aaa]"),
        "zero or more optional a must stay greedy: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn zero_or_more_optional_a_on_b_is_empty() {
    let (out, code) = run_src(
        "create pattern p:\n    zero or more (optional \"a\")\nend pattern\n\
         store hit as find p in \"b\"\n\
         display \"n1b: [\" with hit[\"matched_text\"] with \"]\"\n",
    );
    assert!(
        !out.contains("limit exceeded"),
        "nullable star must not blow the meter: {out}"
    );
    assert!(out.contains("n1b: []"), "empty match on b: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn zero_or_more_optional_a_on_aab_is_aa() {
    let (out, code) = run_src(
        "create pattern p:\n    zero or more (optional \"a\")\nend pattern\n\
         store hit as find p in \"aab\"\n\
         display \"n1aab: [\" with hit[\"matched_text\"] with \"]\"\n",
    );
    assert!(
        !out.contains("limit exceeded"),
        "nullable star must not blow the meter: {out}"
    );
    assert!(out.contains("n1aab: [aa]"), "greedy aa: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn one_or_more_of_zero_or_more_digit_terminates() {
    let (out, code) = run_src(
        "create pattern p:\n    one or more (zero or more digit)\nend pattern\n\
         store hit as find p in \"x12\"\n\
         display \"n2: [\" with hit[\"matched_text\"] with \"]\"\n",
    );
    assert!(
        !out.contains("limit exceeded"),
        "nullable plus must not blow the meter: {out}"
    );
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn zero_or_more_letter_or_optional_digit_gives_ab1() {
    let (out, code) = run_src(
        "create pattern p:\n    zero or more (letter or optional digit)\nend pattern\n\
         store hit as find p in \"ab1-\"\n\
         display \"n3: [\" with hit[\"matched_text\"] with \"]\"\n",
    );
    assert!(
        !out.contains("limit exceeded"),
        "nullable alt-star must not blow the meter: {out}"
    );
    assert!(out.contains("n3: [ab1]"), "greedy ab1: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn x_then_star_of_star_letter() {
    let (out, code) = run_src(
        "create pattern p:\n    \"x\" then zero or more (zero or more letter)\nend pattern\n\
         store hit as find p in \"xab\"\n\
         display \"n4: [\" with hit[\"matched_text\"] with \"]\"\n",
    );
    assert!(
        !out.contains("limit exceeded"),
        "nested nullable star must not blow the meter: {out}"
    );
    assert!(out.contains("n4: [xab]"), "greedy xab: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

fn expensive_alt_src(inner: &str, haystack: &str, tag: &str) -> String {
    format!(
        "create pattern p:\n    ({inner} then \"!\") or letter\nend pattern\n\
         store hit as find p in \"{haystack}\"\n\
         display \"{tag}: [\" with hit[\"matched_text\"] with \"]\"\n"
    )
}

#[test]
fn expensive_letter_or_letter_then_bang_returns_a() {
    let hay = "a".repeat(64);
    let (out, code) = run_src(&expensive_alt_src(
        "one or more (letter or letter)",
        &hay,
        "e1",
    ));
    assert!(
        !out.contains("limit exceeded"),
        "failing higher-priority arm must stay linear: {out}"
    );
    assert!(out.contains("e1: [a]"), "left-first letter: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn expensive_nested_one_or_more_then_bang_returns_a() {
    let hay = "a".repeat(64);
    let (out, code) = run_src(&expensive_alt_src(
        "one or more (one or more letter)",
        &hay,
        "e1b",
    ));
    assert!(
        !out.contains("limit exceeded"),
        "nested one-or-more then bang must stay linear: {out}"
    );
    assert!(out.contains("e1b: [a]"), "left-first letter: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn expensive_two_letter_runs_then_bang_returns_a() {
    let hay = "a".repeat(4_000);
    let (out, code) = run_src(&expensive_alt_src(
        "one or more letter then one or more letter",
        &hay,
        "e2",
    ));
    assert!(
        !out.contains("limit exceeded"),
        "quadratic then-bang arm must stay linear: {out}"
    );
    assert!(out.contains("e2: [a]"), "left-first letter: {out}");
    assert_eq!(code, Some(0), "program should exit 0: {out}");
}

#[test]
fn empty_or_arm_in_star_agrees_with_inert_backref() {
    let (plain, code1) = run_src(
        "create pattern p:\n    zero or more (optional \"-\" or digit)\nend pattern\n\
         store hit as find p in \"12\"\n\
         display \"plain: [\" with hit[\"matched_text\"] with \"]\"\n",
    );
    let (backed, code2) = run_src(
        "create pattern p:\n    capture {optional \"x\"} as e then zero or more (optional \"-\" or digit) then same as captured \"e\"\nend pattern\n\
         store hit as find p in \"12\"\n\
         display \"back: [\" with hit[\"matched_text\"] with \"]\"\n",
    );
    assert!(
        plain.contains("plain: []"),
        "pike path must be left-first empty: {plain}"
    );
    assert!(
        backed.contains("back: []"),
        "backref path must be left-first empty: {backed}"
    );
    assert_eq!(code1, Some(0), "{plain}");
    assert_eq!(code2, Some(0), "{backed}");
}

#[test]
fn empty_whitespace_or_letter_star_agrees_with_inert_backref() {
    let (plain, code1) = run_src(
        "create pattern p:\n    zero or more ((zero or more whitespace) or letter)\nend pattern\n\
         store hit as find p in \"ab\"\n\
         display \"plain: [\" with hit[\"matched_text\"] with \"]\"\n",
    );
    let (backed, code2) = run_src(
        "create pattern p:\n    capture {optional \"x\"} as e then zero or more ((zero or more whitespace) or letter) then same as captured \"e\"\nend pattern\n\
         store hit as find p in \"ab\"\n\
         display \"back: [\" with hit[\"matched_text\"] with \"]\"\n",
    );
    assert!(
        plain.contains("plain: []"),
        "pike path must be left-first empty: {plain}"
    );
    assert!(
        backed.contains("back: []"),
        "backref path must be left-first empty: {backed}"
    );
    assert_eq!(code1, Some(0), "{plain}");
    assert_eq!(code2, Some(0), "{backed}");
}

fn assert_plain_and_backref(plain_src: &str, back_src: &str, plain_tag: &str, back_tag: &str) {
    let (plain, code1) = run_src(plain_src);
    let (backed, code2) = run_src(back_src);
    assert!(plain.contains(plain_tag), "pike path: {plain}");
    assert!(backed.contains(back_tag), "backref path: {backed}");
    assert_eq!(code1, Some(0), "{plain}");
    assert_eq!(code2, Some(0), "{backed}");
}

#[test]
fn letter_plus_then_ing_agrees_with_inert_backref() {
    assert_plain_and_backref(
        "create pattern p:\n    one or more letter then \"ing\"\nend pattern\n\
         store hit as find p in \"sing\"\n\
         display \"plain: [\" with hit[\"matched_text\"] with \"]\"\n",
        "create pattern p:\n    capture {optional \"x\"} as e then one or more letter then \"ing\" then same as captured \"e\"\nend pattern\n\
         store hit as find p in \"sing\"\n\
         display \"back: [\" with hit[\"matched_text\"] with \"]\"\n",
        "plain: [sing]",
        "back: [sing]",
    );
}

#[test]
fn letter_plus_then_ing_finds_king() {
    let (out, code) = run_src(
        "create pattern p:\n    one or more letter then \"ing\"\nend pattern\n\
         store hit as find p in \"the king is running\"\n\
         display \"hit: [\" with hit[\"matched_text\"] with \"]\"\n",
    );
    assert!(out.contains("hit: [king]"), "must find king: {out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn find_all_letter_plus_ing_finds_three_words() {
    let (out, code) = run_src(
        "create pattern p:\n    one or more letter then \"ing\"\nend pattern\n\
         store hits as pattern_find_all of \"sing ring bring\" and p\n\
         display \"count: \" with length of hits\n",
    );
    assert!(out.contains("count: 3"), "three -ing words: {out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn optional_letter_then_ab_agrees_with_inert_backref() {
    assert_plain_and_backref(
        "create pattern p:\n    optional letter then \"ab\"\nend pattern\n\
         store hit as find p in \"ab\"\n\
         display \"plain: [\" with hit[\"matched_text\"] with \"]\"\n",
        "create pattern p:\n    capture {optional \"x\"} as e then optional letter then \"ab\" then same as captured \"e\"\nend pattern\n\
         store hit as find p in \"ab\"\n\
         display \"back: [\" with hit[\"matched_text\"] with \"]\"\n",
        "plain: [ab]",
        "back: [ab]",
    );
}

#[test]
fn letter_star_or_digit_star_on_ab1_agrees_with_inert_backref() {
    assert_plain_and_backref(
        "create pattern p:\n    zero or more ((zero or more letter) or digit)\nend pattern\n\
         store hit as find p in \"ab1\"\n\
         display \"plain: [\" with hit[\"matched_text\"] with \"]\"\n",
        "create pattern p:\n    capture {optional \"x\"} as e then zero or more ((zero or more letter) or digit) then same as captured \"e\"\nend pattern\n\
         store hit as find p in \"ab1\"\n\
         display \"back: [\" with hit[\"matched_text\"] with \"]\"\n",
        "plain: [ab]",
        "back: [ab]",
    );
}

#[test]
fn at_most_two_letter_or_dash_star_on_a_dash_b_agrees_with_inert_backref() {
    assert_plain_and_backref(
        "create pattern p:\n    zero or more ((at most 2 letter) or \"-\")\nend pattern\n\
         store hit as find p in \"a-b\"\n\
         display \"plain: [\" with hit[\"matched_text\"] with \"]\"\n",
        "create pattern p:\n    capture {optional \"x\"} as e then zero or more ((at most 2 letter) or \"-\") then same as captured \"e\"\nend pattern\n\
         store hit as find p in \"a-b\"\n\
         display \"back: [\" with hit[\"matched_text\"] with \"]\"\n",
        "plain: [a]",
        "back: [a]",
    );
}

#[test]
fn between_one_and_two_letter_or_dash_star_on_a_dash_b_agrees_with_inert_backref() {
    assert_plain_and_backref(
        "create pattern p:\n    zero or more ((1 to 2 letter) or \"-\")\nend pattern\n\
         store hit as find p in \"a-b\"\n\
         display \"plain: [\" with hit[\"matched_text\"] with \"]\"\n",
        "create pattern p:\n    capture {optional \"x\"} as e then zero or more ((1 to 2 letter) or \"-\") then same as captured \"e\"\nend pattern\n\
         store hit as find p in \"a-b\"\n\
         display \"back: [\" with hit[\"matched_text\"] with \"]\"\n",
        "plain: [a]",
        "back: [a]",
    );
}
