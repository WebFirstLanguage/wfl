#![no_main]
//! Fuzz target: the pattern engine (parser grammar + compiler + VM).
//!
//! The fuzz bytes are wrapped as the body of a `create pattern` block so the
//! whole pattern pipeline sees untrusted input end-to-end:
//!   1. the parser's pattern grammar (`parse_pattern_tokens`),
//!   2. the pattern compiler (`CompiledPattern::compile`),
//!   3. the pattern VM (`find_all`) against a small fixed haystack.
//!
//! The VM is the ReDoS-relevant surface; the interesting failure modes are a
//! panic in compilation or a pathological match blowup (caught by libFuzzer's
//! timeout). The pattern VM's own step/state ceilings should keep matching
//! bounded even without an interpreter-level budget installed.
use libfuzzer_sys::fuzz_target;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::Statement;
use wfl::pattern::CompiledPattern;

const HAYSTACK: &str = "the quick brown fox 123 !@# aAbBcC   \t\n done";

fuzz_target!(|data: &[u8]| {
    let Ok(body) = std::str::from_utf8(data) else {
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
                let _ = compiled.find_all(HAYSTACK);
            }
        }
    }
});
