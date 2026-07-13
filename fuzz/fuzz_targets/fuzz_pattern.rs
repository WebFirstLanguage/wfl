#![no_main]
//! Fuzz target: the pattern engine (parser grammar + compiler + VM).
//!
//! The input is split into a **pattern/haystack pair** at the first NUL byte, so
//! the fuzzer controls both sides independently — the pattern *and* the text it
//! runs against. This is what surfaces ReDoS-style blowups, which depend on the
//! interaction between a pathological pattern and a crafted input, not on either
//! alone. (With no NUL, the whole input is the pattern and a default haystack is
//! used.)
//!
//! The pattern bytes are wrapped as the body of a `create pattern` block so the
//! whole pattern pipeline sees untrusted input end-to-end:
//!   1. the parser's pattern grammar (`parse_pattern_tokens`),
//!   2. the pattern compiler (`CompiledPattern::compile`),
//!   3. the pattern VM (`find_all`) against the fuzzed haystack.
//!
//! The interesting failure modes are a panic in compilation or a pathological
//! match blowup (caught by libFuzzer's timeout). The pattern VM's own step/state
//! ceilings should keep matching bounded even without an interpreter-level
//! budget installed — a hang here is a finding.
use libfuzzer_sys::fuzz_target;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::Statement;
use wfl::pattern::CompiledPattern;

const DEFAULT_HAYSTACK: &str = "the quick brown fox 123 !@# aAbBcC   \t\n done";

fuzz_target!(|data: &[u8]| {
    // Split "pattern\0haystack"; fall back to a default haystack if no NUL.
    let (pattern_bytes, haystack): (&[u8], String) = match data.iter().position(|&b| b == 0) {
        Some(i) => (
            &data[..i],
            String::from_utf8_lossy(&data[i + 1..]).into_owned(),
        ),
        None => (data, DEFAULT_HAYSTACK.to_string()),
    };
    let Ok(body) = std::str::from_utf8(pattern_bytes) else {
        return;
    };
    let source = format!("create pattern fuzzpat:\n{body}\nend pattern\n");
    let tokens = lex_wfl_with_positions(&source);
    let mut parser = Parser::new(&tokens);
    let Ok(program) = parser.parse() else {
        return;
    };
    for statement in &program.statements {
        if let Statement::PatternDefinition { pattern, .. } = statement {
            if let Ok(compiled) = CompiledPattern::compile(pattern) {
                let _ = compiled.find_all(&haystack);
            }
        }
    }
});
