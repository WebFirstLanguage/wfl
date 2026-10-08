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
