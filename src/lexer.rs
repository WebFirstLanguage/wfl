//! The WFL lexer now lives in the `wfl_core` crate so that the compiler and
//! `wflpkg` share one tokenizer byte-for-byte (see
//! `wflpkg-adr-001-binary-and-crate-structure.md`). This module re-exports it
//! unchanged, so every existing `crate::lexer::…` path keeps resolving.
pub use wfl_core::lexer::*;
