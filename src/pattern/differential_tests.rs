//! Randomized Pike vs inert-backtracker (and regex crate) differential.
//!
//! CI runs a seeded subset. Set `WFL_PATTERN_DIFF_CASES` to raise the count
//! (the local 100k+ gate uses this).

use crate::exec::budget::ExecutionBudget;
use crate::parser::ast::{Anchor, CharClass, PatternExpression, Quantifier};
use crate::pattern::CompiledPattern;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;

const CI_CASES: usize = 4_096;
const CI_SEED: u64 = 0x9E37_79B9_7F4A_7C15;
const ALPHA: &[char] = &['a', 'b', '1', '-', ' '];
const LITERALS: &[&str] = &["", "a", "b", "1", "-", " ", "ab", "ing", "12", "a1", "b1"];

#[derive(Default)]
struct DiffReport {
    total: usize,
    compile_fail: usize,
    budget_fail: usize,
    pike_vs_backref: usize,
    pike_vs_regex: usize,
    backref_vs_regex: usize,
    regex_untranslatable: usize,
    first_mismatches: Vec<String>,
}

impl DiffReport {
    fn push_mismatch(&mut self, line: String) {
        if self.first_mismatches.len() < 8 {
            self.first_mismatches.push(line);
        }
    }
}

fn with_inert_backref(inner: PatternExpression) -> PatternExpression {
    PatternExpression::Sequence(vec![
        PatternExpression::Capture {
            name: "e".to_string(),
            pattern: Box::new(PatternExpression::Quantified {
                pattern: Box::new(PatternExpression::Literal("x".to_string())),
                quantifier: Quantifier::Optional,
            }),
        },
        inner,
        PatternExpression::Backreference("e".to_string()),
    ])
}

fn extent(pattern: &PatternExpression, text: &str) -> Result<Option<String>, String> {
    let compiled = CompiledPattern::compile(pattern).map_err(|e| format!("compile: {e:?}"))?;
    compiled
        .find_with_budget(text, &ExecutionBudget::current_or_default())
        .map(|found| found.map(|m| m.matched_text))
        .map_err(|e| format!("budget: {e:?}"))
}

fn gen_quantifier(rng: &mut StdRng) -> Quantifier {
    match rng.random_range(0..7) {
        0 => Quantifier::Optional,
        1 => Quantifier::ZeroOrMore,
        2 => Quantifier::OneOrMore,
        3 => Quantifier::Exactly(rng.random_range(1..=3)),
        4 => Quantifier::AtLeast(rng.random_range(1..=2)),
        5 => {
            let lo = rng.random_range(0..=2);
            Quantifier::Between(lo, lo + rng.random_range(0..=2))
        }
        _ => Quantifier::AtMost(rng.random_range(1..=3)),
    }
}

fn gen_class(rng: &mut StdRng) -> CharClass {
    match rng.random_range(0..4) {
        0 => CharClass::Digit,
        1 => CharClass::Letter,
        2 => CharClass::Whitespace,
        _ => CharClass::Any,
    }
}

fn gen_leaf(rng: &mut StdRng) -> PatternExpression {
    if rng.random_bool(0.55) {
        PatternExpression::Literal(LITERALS[rng.random_range(0..LITERALS.len())].to_string())
    } else {
        PatternExpression::CharacterClass(gen_class(rng))
    }
}

fn gen_pattern(rng: &mut StdRng, depth: u32) -> PatternExpression {
    if depth == 0 {
        return gen_leaf(rng);
    }
    match rng.random_range(0..11) {
        0..=2 => gen_leaf(rng),
        3..=4 => PatternExpression::Quantified {
            pattern: Box::new(gen_pattern(rng, depth - 1)),
            quantifier: gen_quantifier(rng),
        },
        5 => {
            let n = rng.random_range(2..=3);
            PatternExpression::Sequence((0..n).map(|_| gen_pattern(rng, depth - 1)).collect())
        }
        6 => {
            let n = rng.random_range(2..=3);
            PatternExpression::Alternative((0..n).map(|_| gen_pattern(rng, depth - 1)).collect())
        }
        7 => PatternExpression::Capture {
            name: format!("c{}", rng.random_range(0..4)),
            pattern: Box::new(gen_pattern(rng, depth - 1)),
        },
        8 => PatternExpression::Anchor(if rng.random_bool(0.5) {
            Anchor::StartOfText
        } else {
            Anchor::EndOfText
        }),
        // Nested star of (nullable | class) — the empty-iteration class.
        9 => PatternExpression::Quantified {
            pattern: Box::new(PatternExpression::Alternative(vec![
                PatternExpression::Quantified {
                    pattern: Box::new(gen_pattern(rng, depth.saturating_sub(2))),
                    quantifier: Quantifier::ZeroOrMore,
                },
                PatternExpression::CharacterClass(gen_class(rng)),
            ])),
            quantifier: Quantifier::ZeroOrMore,
        },
        // Bounded quantifier arm inside `zero or more (… or …)`. `at most`
        // / `between` emit Splits that no Jump targets; empty re-entry
        // still has to take the Split exit.
        _ => PatternExpression::Quantified {
            pattern: Box::new(PatternExpression::Alternative(vec![
                PatternExpression::Quantified {
                    pattern: Box::new(PatternExpression::CharacterClass(CharClass::Letter)),
                    quantifier: if rng.random_bool(0.5) {
                        Quantifier::AtMost(rng.random_range(2..=3))
                    } else {
                        let lo = rng.random_range(0..=1);
                        Quantifier::Between(lo, lo + rng.random_range(1..=2))
                    },
                },
                PatternExpression::Literal("-".to_string()),
            ])),
            quantifier: Quantifier::ZeroOrMore,
        },
    }
}

fn gen_haystack(rng: &mut StdRng) -> String {
    if rng.random_bool(0.12) {
        return [
            "sing",
            "ab1",
            "ab",
            "12",
            "the king is running",
            "sing ring bring",
            "a-b",
            "a-1bb--",
        ][rng.random_range(0..8)]
        .to_string();
    }
    let len = rng.random_range(0..=8);
    (0..len)
        .map(|_| ALPHA[rng.random_range(0..ALPHA.len())])
        .collect()
}

fn to_regex(expr: &PatternExpression) -> Option<String> {
    match expr {
        PatternExpression::Literal(s) => Some(regex::escape(s)),
        PatternExpression::CharacterClass(CharClass::Digit) => Some("[0-9]".into()),
        PatternExpression::CharacterClass(CharClass::Letter) => Some("[A-Za-z]".into()),
        PatternExpression::CharacterClass(CharClass::Whitespace) => Some("[ \\t\\n\\r]".into()),
        PatternExpression::CharacterClass(CharClass::Any) => Some("(?s:.)".into()),
        PatternExpression::CharacterClass(_) => None,
        PatternExpression::Quantified {
            pattern,
            quantifier,
        } => {
            let inner = to_regex(pattern)?;
            let wrapped = format!("(?:{inner})");
            Some(match quantifier {
                Quantifier::Optional => format!("{wrapped}?"),
                Quantifier::ZeroOrMore => format!("{wrapped}*"),
                Quantifier::OneOrMore => format!("{wrapped}+"),
                Quantifier::Exactly(n) => format!("{wrapped}{{{n}}}"),
                Quantifier::AtLeast(n) => format!("{wrapped}{{{n},}}"),
                Quantifier::Between(a, b) => format!("{wrapped}{{{a},{b}}}"),
                Quantifier::AtMost(n) => format!("{wrapped}{{0,{n}}}"),
            })
        }
        PatternExpression::Sequence(parts) => {
            let mut out = String::new();
            for part in parts {
                out.push_str(&to_regex(part)?);
            }
            Some(out)
        }
        PatternExpression::Alternative(parts) => {
            let mut alts = Vec::new();
            for part in parts {
                alts.push(to_regex(part)?);
            }
            Some(format!("(?:{})", alts.join("|")))
        }
        PatternExpression::Capture { pattern, .. } => to_regex(pattern),
        PatternExpression::Anchor(Anchor::StartOfText) => Some(r"\A".into()),
        PatternExpression::Anchor(Anchor::EndOfText) => Some(r"\z".into()),
        PatternExpression::Backreference(_)
        | PatternExpression::Lookahead(_)
        | PatternExpression::NegativeLookahead(_)
        | PatternExpression::Lookbehind(_)
        | PatternExpression::NegativeLookbehind(_)
        | PatternExpression::ListReference(_) => None,
    }
}

fn regex_extent(expr: &PatternExpression, text: &str) -> Option<Option<String>> {
    let src = to_regex(expr)?;
    let compiled = regex::Regex::new(&src).ok()?;
    Some(compiled.find(text).map(|m| m.as_str().to_string()))
}

fn bounded_quantifier_in_star_or(quantifier: Quantifier) -> PatternExpression {
    PatternExpression::Quantified {
        pattern: Box::new(PatternExpression::Alternative(vec![
            PatternExpression::Quantified {
                pattern: Box::new(PatternExpression::CharacterClass(CharClass::Letter)),
                quantifier,
            },
            PatternExpression::Literal("-".to_string()),
        ])),
        quantifier: Quantifier::ZeroOrMore,
    }
}

fn review_shapes() -> Vec<(PatternExpression, &'static str)> {
    vec![
        (bounded_quantifier_in_star_or(Quantifier::AtMost(2)), "a-b"),
        (
            bounded_quantifier_in_star_or(Quantifier::Between(1, 2)),
            "a-b",
        ),
        (
            bounded_quantifier_in_star_or(Quantifier::AtMost(2)),
            "a-1bb--",
        ),
    ]
}

fn run_differential(seed: u64, cases: usize) -> DiffReport {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut report = DiffReport {
        total: cases,
        ..DiffReport::default()
    };
    let mut remaining = cases;
    for (pattern, text) in review_shapes() {
        if remaining == 0 {
            break;
        }
        remaining -= 1;
        compare_case(&mut report, pattern, text);
    }
    for _ in 0..remaining {
        let depth = rng.random_range(0..=3);
        let pattern = gen_pattern(&mut rng, depth);
        let text = gen_haystack(&mut rng);
        compare_case(&mut report, pattern, &text);
    }
    report
}

fn compare_case(report: &mut DiffReport, pattern: PatternExpression, text: &str) {
    let pike = match extent(&pattern, text) {
        Ok(v) => v,
        Err(e) if e.starts_with("compile") => {
            report.compile_fail += 1;
            return;
        }
        Err(_) => {
            report.budget_fail += 1;
            return;
        }
    };
    let back = match extent(&with_inert_backref(pattern.clone()), text) {
        Ok(v) => v,
        Err(_) => {
            report.budget_fail += 1;
            return;
        }
    };
    if pike != back {
        report.pike_vs_backref += 1;
        report.push_mismatch(format!(
            "pike≠backref pattern={pattern:?} text={text:?} pike={pike:?} back={back:?}"
        ));
    }
    match regex_extent(&pattern, text) {
        None => report.regex_untranslatable += 1,
        Some(re) => {
            if pike != re {
                report.pike_vs_regex += 1;
                report.push_mismatch(format!(
                    "pike≠regex pattern={pattern:?} text={text:?} pike={pike:?} re={re:?}"
                ));
            }
            if back != re {
                report.backref_vs_regex += 1;
            }
        }
    }
}

fn case_count() -> usize {
    std::env::var("WFL_PATTERN_DIFF_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(CI_CASES)
}

fn print_report(seed: u64, report: &DiffReport) {
    println!(
        "diff seed={seed:#x} n={} pike≠back={} pike≠re={} back≠re={} compile={} budget={} untrans={}",
        report.total,
        report.pike_vs_backref,
        report.pike_vs_regex,
        report.backref_vs_regex,
        report.compile_fail,
        report.budget_fail,
        report.regex_untranslatable
    );
    if report.pike_vs_backref > 0 || report.total > CI_CASES {
        for line in &report.first_mismatches {
            println!("  {line}");
        }
    }
}

#[test]
fn pike_equals_inert_backref_on_seeded_random_patterns() {
    let n = case_count();
    let seeds: &[u64] = if n > CI_CASES {
        &[CI_SEED, 12_345, 777]
    } else {
        &[CI_SEED]
    };
    for &seed in seeds {
        let report = run_differential(seed, n);
        print_report(seed, &report);
        assert_eq!(
            report.pike_vs_backref,
            0,
            "Pike must agree with the inert-backref path on {n} cases (seed {seed:#x}): {samples:?}",
            samples = report.first_mismatches
        );
    }
}

#[test]
fn pike_equals_inert_backref_on_second_seeded_subset() {
    let report = run_differential(12_345, case_count().min(2_048));
    print_report(12_345, &report);
    assert_eq!(
        report.pike_vs_backref,
        0,
        "second seed mismatches: {samples:?}",
        samples = report.first_mismatches
    );
}
