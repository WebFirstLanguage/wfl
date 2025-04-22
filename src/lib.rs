#![deny(clippy::await_holding_refcell_ref)]

pub mod analyzer;
pub mod config;
pub mod debug;
pub mod debug_report;
pub mod diagnostics;
pub mod fixer;
pub mod interpreter;
pub mod lexer;
pub mod linter;
pub mod logging;
pub mod parser;
pub mod repl;
pub mod stdlib;
pub mod typechecker;

pub use interpreter::Interpreter;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
