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
