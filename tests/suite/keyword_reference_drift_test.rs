//! Guard: the keyword reference pages must list exactly the words the lexer
//! reserves, classified the way the lexer and parser actually treat them.
//!
//! Background (the drift this reproduces): `Docs/reference/reserved-keywords.md`
//! and `Docs/reference/keyword-reference.md` kept saying "181 keywords" while
//! the lexer grew test-framework, module and binary-I/O words. An audit on
//! 2026-10-31 found 19 `#[token]` words missing from the alphabetical table,
//! three listed words (`downward`, `upward`, `pop`) that were never keywords,
//! and section counts that no longer matched their lists. Nothing failed, so
//! the drift was silent.
//!
//! The lexer source is the list of record: every `#[token("word")]` plus the
//! alternatives of the boolean-literal regex. Each word is classified through
//! the real `Token` methods, and "As Var?" is decided by the real parser on
//! `store <word> as 1`, so changing `is_structural_keyword`,
//! `is_contextual_keyword` or `parse_variable_name_list` shows up here too.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use wfl::lexer::lex_wfl_with_positions;
use wfl::lexer::token::Token;
use wfl::parser::Parser;

const COMPLETE_REFERENCE: &str = "Docs/reference/reserved-keywords.md";
const QUICK_REFERENCE: &str = "Docs/reference/keyword-reference.md";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Class {
    Structural,
    Contextual,
    Other,
    Literal,
}

impl Class {
    /// The spelling used in the alphabetical table's "Type" column.
    fn label(self) -> &'static str {
        match self {
            Class::Structural => "Structural",
            Class::Contextual => "Contextual",
            Class::Other => "Other",
            Class::Literal => "Literal",
        }
    }

    /// The complete reference's section heading for this class, without the
    /// trailing `(count)`.
    fn section_title(self) -> &'static str {
        match self {
            Class::Structural => "Structural Keywords",
            Class::Contextual => "Contextual Keywords",
            Class::Other => "Other Reserved Keywords",
            Class::Literal => "Boolean & Null Literals",
        }
    }
}

const CLASSES: [Class; 4] = [
    Class::Structural,
    Class::Contextual,
    Class::Other,
    Class::Literal,
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_repo_file(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Every word the lexer reserves: each `#[token("...")]` that starts with a
/// letter (so punctuation and newlines are skipped), plus the alternatives of
/// the case-insensitive boolean-literal regex `(?i:yes|no|true|false)`.
fn lexer_words() -> BTreeSet<String> {
    let source = read_repo_file("src/lexer/token.rs");
    let mut words = BTreeSet::new();
    for line in source.lines().map(str::trim) {
        if let Some(word) = line
            .strip_prefix("#[token(\"")
            .and_then(|rest| rest.strip_suffix("\")]"))
        {
            if word.starts_with(|c: char| c.is_ascii_alphabetic()) {
                words.insert(word.to_string());
            }
        } else if let Some((alternatives, _)) = line
            .strip_prefix("#[regex(\"(?i:")
            .and_then(|rest| rest.split_once(")\""))
        {
            words.extend(alternatives.split('|').map(str::to_string));
        }
    }
    assert!(
        words.contains("store") && words.contains("yes"),
        "lexer word extraction found neither `store` nor `yes`; has the \
         `#[token]` / `#[regex]` layout in src/lexer/token.rs changed?"
    );
    words
}

/// Classify a word through the real lexer and `Token` methods. The parser
/// checks structural before contextual, so a word in both lists (such as
/// `push`) is structural.
fn classify(word: &str) -> Class {
    let tokens: Vec<Token> = lex_wfl_with_positions(word)
        .into_iter()
        .map(|t| t.token)
        .filter(|t| *t != Token::Eol)
        .collect();
    let [token] = tokens.as_slice() else {
        panic!("`{word}` should lex as exactly one token, got {tokens:?}");
    };
    match token {
        Token::BooleanLiteral(_) | Token::NothingLiteral => Class::Literal,
        Token::Identifier(_) => panic!("`{word}` lexes as an identifier, not a keyword"),
        t if t.is_structural_keyword() => Class::Structural,
        t if t.is_contextual_keyword() => Class::Contextual,
        _ => Class::Other,
    }
}

/// Whether `store <word> as 1` parses, which is what "As Var?" documents.
fn store_accepts(word: &str) -> bool {
    let source = format!("store {word} as 1\n");
    let tokens = lex_wfl_with_positions(&source);
    Parser::new(&tokens).parse().is_ok()
}

/// The truth for every lexer word: its class and whether `store` accepts it.
fn actual_keywords() -> BTreeMap<String, (Class, bool)> {
    lexer_words()
        .into_iter()
        .map(|word| {
            let facts = (classify(&word), store_accepts(&word));
            (word, facts)
        })
        .collect()
}

fn table_cells(line: &str) -> Option<Vec<&str>> {
    let inner = line.trim().strip_prefix('|')?.strip_suffix('|')?;
    Some(inner.split('|').map(str::trim).collect())
}

fn backticked(cell: &str) -> Option<&str> {
    cell.strip_prefix('`')?.strip_suffix('`')
}

/// A Markdown heading's level and title, e.g. `### Name (3)` → `(3, "Name (3)")`.
fn heading(line: &str) -> Option<(usize, &str)> {
    let level = line.chars().take_while(|&c| c == '#').count();
    let title = line.get(level..)?.strip_prefix(' ')?;
    (level > 0).then(|| (level, title.trim()))
}

/// The count a heading states: `Structural Keywords (63)` or
/// `Control Flow (24 keywords)`.
fn stated_count(title: &str) -> Option<usize> {
    let inner = title.strip_suffix(')')?.rsplit_once('(')?.1;
    inner
        .strip_suffix(" keywords")
        .unwrap_or(inner)
        .parse()
        .ok()
}

/// Keywords a line lists: the first cell of a table row, or every item of a
/// line that is only a comma-separated list of backticked words. Prose that
/// mentions a keyword in passing lists nothing.
fn listed_keywords(line: &str) -> Vec<&str> {
    if let Some(cells) = table_cells(line) {
        return backticked(cells[0]).into_iter().collect();
    }
    let items: Option<Vec<&str>> = line.trim().split(", ").map(backticked).collect();
    items.unwrap_or_default()
}

/// Every heading of a page, with the keywords listed beneath it up to the next
/// heading of the same or higher level. Fenced code blocks are skipped, since
/// `#` starts a comment in WFL.
fn sections(page: &str) -> Vec<(String, BTreeSet<String>)> {
    let mut open: Vec<(usize, String, BTreeSet<String>)> = Vec::new();
    let mut done = Vec::new();
    let mut in_fence = false;
    for line in page.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some((level, title)) = heading(line) {
            while open.last().is_some_and(|(l, _, _)| *l >= level) {
                let (_, title, words) = open.pop().unwrap();
                done.push((title, words));
            }
            open.push((level, title.to_string(), BTreeSet::new()));
            continue;
        }
        for word in listed_keywords(line) {
            for (_, _, words) in open.iter_mut() {
                words.insert(word.to_string());
            }
        }
    }
    done.extend(open.into_iter().map(|(_, title, words)| (title, words)));
    done
}

/// Rows of the complete reference's "Complete Alphabetical Reference" table:
/// keyword → (Type, As Var?).
fn alphabetical_table() -> BTreeMap<String, (String, String)> {
    let page = read_repo_file(COMPLETE_REFERENCE);
    let (_, rest) = page
        .split_once("\n## Complete Alphabetical Reference")
        .expect("the complete reference has a `## Complete Alphabetical Reference` section");
    let section = rest.split("\n## ").next().unwrap_or(rest);
    let mut rows = BTreeMap::new();
    for line in section.lines() {
        let Some(cells) = table_cells(line) else {
            continue;
        };
        let Some(word) = backticked(cells[0]) else {
            continue; // header and separator rows
        };
        assert!(
            cells.len() >= 4,
            "alphabetical table row for `{word}` has too few columns: {line}"
        );
        let previous = rows.insert(
            word.to_string(),
            (cells[1].to_string(), cells[3].to_string()),
        );
        assert!(
            previous.is_none(),
            "`{word}` appears twice in the alphabetical table"
        );
    }
    rows
}

#[test]
fn complete_reference_table_lists_exactly_the_lexer_words() {
    let lexer = lexer_words();
    let documented: BTreeSet<String> = alphabetical_table().into_keys().collect();
    let missing: Vec<&String> = lexer.difference(&documented).collect();
    let not_keywords: Vec<&String> = documented.difference(&lexer).collect();
    assert!(
        missing.is_empty() && not_keywords.is_empty(),
        "{COMPLETE_REFERENCE} \"Complete Alphabetical Reference\" is out of sync \
         with src/lexer/token.rs.\n  lexer words missing from the table: \
         {missing:?}\n  table rows that are not lexer words: {not_keywords:?}"
    );
}

#[test]
fn complete_reference_table_matches_lexer_and_parser_behavior() {
    let documented = alphabetical_table();
    let mut wrong = Vec::new();
    for (word, (class, accepted)) in actual_keywords() {
        let Some((doc_type, doc_as_var)) = documented.get(&word) else {
            continue; // reported by complete_reference_table_lists_exactly_the_lexer_words
        };
        let as_var = if accepted { "✅" } else { "❌" };
        if doc_type != class.label() || doc_as_var != as_var {
            wrong.push(format!(
                "`{word}`: documented {doc_type} / {doc_as_var}, actual {} / {as_var}",
                class.label()
            ));
        }
    }
    assert!(
        wrong.is_empty(),
        "{COMPLETE_REFERENCE} alphabetical table disagrees with the lexer \
         (Type) or with `store <word> as 1` (As Var?):\n  {}",
        wrong.join("\n  ")
    );
}

#[test]
fn complete_reference_class_sections_list_exactly_their_keywords() {
    let actual = actual_keywords();
    let page = read_repo_file(COMPLETE_REFERENCE);
    let sections = sections(&page);
    for class in CLASSES {
        let expected: BTreeSet<String> = actual
            .iter()
            .filter(|(_, (c, _))| *c == class)
            .map(|(word, _)| word.clone())
            .collect();
        let heading = format!("{} ({})", class.section_title(), expected.len());
        let Some((_, listed)) = sections.iter().find(|(title, _)| *title == heading) else {
            panic!("{COMPLETE_REFERENCE} has no `## {heading}` section");
        };
        let missing: Vec<&String> = expected.difference(listed).collect();
        let extra: Vec<&String> = listed.difference(&expected).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "{COMPLETE_REFERENCE} `## {heading}` lists the wrong keywords.\n  \
             missing: {missing:?}\n  not {}: {extra:?}",
            class.label()
        );
    }
}

#[test]
fn quick_reference_lists_every_keyword_with_matching_as_var() {
    let actual = actual_keywords();
    let page = read_repo_file(QUICK_REFERENCE);
    // Only rows that end in an "As Var?" mark list a keyword. Other tables,
    // such as "Words That Commonly Bite", end in suggested names instead.
    let mut marks: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for cells in page.lines().filter_map(table_cells) {
        let mark = cells.last().copied().unwrap_or_default();
        if let (Some(word), "✓" | "✗") = (backticked(cells[0]), mark) {
            marks
                .entry(word.to_string())
                .or_default()
                .insert(mark.to_string());
        }
    }
    let mut problems = Vec::new();
    for (word, (_, accepted)) in &actual {
        let expected = if *accepted { "✓" } else { "✗" };
        match marks.get(word) {
            None => problems.push(format!("`{word}` is missing")),
            Some(found) if found.iter().any(|m| m != expected) => problems.push(format!(
                "`{word}` is marked {found:?}, but `store {word} as 1` means {expected}"
            )),
            Some(_) => {}
        }
    }
    for word in marks.keys().filter(|w| !actual.contains_key(*w)) {
        problems.push(format!("`{word}` is listed but is not a lexer word"));
    }
    assert!(
        problems.is_empty(),
        "{QUICK_REFERENCE} is out of sync with the lexer and parser:\n  {}",
        problems.join("\n  ")
    );
}

#[test]
fn reference_heading_counts_match_their_lists() {
    let lexer = lexer_words();
    let mut problems = Vec::new();
    for path in [COMPLETE_REFERENCE, QUICK_REFERENCE] {
        for (title, words) in sections(&read_repo_file(path)) {
            let Some(count) = stated_count(&title) else {
                continue;
            };
            if words.len() != count {
                problems.push(format!(
                    "{path} `{title}` lists {} distinct keywords",
                    words.len()
                ));
            }
            let strangers: Vec<&String> = words.iter().filter(|w| !lexer.contains(*w)).collect();
            if !strangers.is_empty() {
                problems.push(format!("{path} `{title}` lists non-keywords {strangers:?}"));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "keyword reference headings disagree with their lists:\n  {}",
        problems.join("\n  ")
    );
}

#[test]
fn published_keyword_totals_match_the_lexer() {
    let actual = actual_keywords();
    let count = |class: Class| actual.values().filter(|(c, _)| *c == class).count();
    let (total, structural, contextual, other, literal) = (
        actual.len(),
        count(Class::Structural),
        count(Class::Contextual),
        count(Class::Other),
        count(Class::Literal),
    );
    let claims = [
        (
            COMPLETE_REFERENCE,
            format!("**Total:** {total} keywords and literals"),
        ),
        (
            QUICK_REFERENCE,
            format!("**Total:** {total} keywords and literals"),
        ),
        (
            "Docs/reference/language-specification.md",
            format!("**{total}** keywords and literals"),
        ),
        (
            "Docs/reference/language-specification.md",
            format!("- {structural} Structural Keywords"),
        ),
        (
            "Docs/reference/language-specification.md",
            format!("- {contextual} Contextual Keywords"),
        ),
        (
            "Docs/reference/language-specification.md",
            format!("- {other} Other Reserved Keywords"),
        ),
        (
            "Docs/reference/language-specification.md",
            format!("- {literal} Boolean & Null Literals"),
        ),
        (
            "Docs/03-language-basics/variables-and-types.md",
            format!("WFL has **{total}** keywords and literals"),
        ),
        (
            "Docs/06-best-practices/naming-conventions.md",
            format!("WFL has **{total}** keywords and literals"),
        ),
        ("CLAUDE.md", format!("all {total} keywords")),
        (
            "CLAUDE.md",
            format!(
                "**{total} keywords total** ({structural} structural, {contextual} \
                 contextual, {other} other, {literal} literals"
            ),
        ),
    ];
    let missing: Vec<String> = claims
        .iter()
        .filter(|(path, claim)| !read_repo_file(path).contains(claim.as_str()))
        .map(|(path, claim)| format!("{path}: expected {claim:?}"))
        .collect();
    assert!(
        missing.is_empty(),
        "published keyword counts are stale (the lexer has {total}: {structural} \
         structural, {contextual} contextual, {other} other, {literal} literals):\n  {}",
        missing.join("\n  ")
    );
}
