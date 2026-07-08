//! `wfl_core` — the dependency-light foundation shared by the WFL compiler
//! (`wfl`) and the package manager (`wflpkg`).
//!
//! The crate exists so that both the compiler and every first-party package
//! tool tokenize manifests with the *same* `logos` lexer, byte-for-byte. This
//! is the structural substance of Decision 1, condition 5 ("a single shared,
//! continuously-fuzzed parser used byte-identically by the compiler and every
//! first-party tool"): with the lexer living below both crates there is one
//! implementation, not two that must agree. See
//! `wflpkg-adr-001-binary-and-crate-structure.md`.

pub mod lexer;
pub mod version;
