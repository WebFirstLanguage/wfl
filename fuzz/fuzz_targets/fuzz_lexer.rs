#![no_main]
//! Fuzz target: the lexer.
//!
//! `lex_wfl_with_positions_checked` is WFL's untrusted-input tokenization entry
//! point. It must never panic or hang on arbitrary source text — a controlled
//! `Err(BudgetExceeded)` (when a budget is installed) or a token vector are the
//! only acceptable outcomes.
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(source) = std::str::from_utf8(data) {
        let _ = wfl::lexer::lex_wfl_with_positions_checked(source);
    }
});
