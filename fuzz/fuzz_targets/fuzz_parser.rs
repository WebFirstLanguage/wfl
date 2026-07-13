#![no_main]
//! Fuzz target: the recursive-descent parser.
//!
//! Lexes arbitrary source, then drives `Parser::parse`. The parser has error
//! recovery, so a `Vec<ParseError>` is an expected outcome; the invariant under
//! test is that no input panics, overflows the stack, or hangs the parser.
use libfuzzer_sys::fuzz_target;
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

fuzz_target!(|data: &[u8]| {
    if let Ok(source) = std::str::from_utf8(data) {
        let tokens = lex_wfl_with_positions(source);
        let mut parser = Parser::new(&tokens);
        let _ = parser.parse();
    }
});
