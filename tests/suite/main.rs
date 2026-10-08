//! Single-binary integration suite (issue #693 pilot: crypto + stdlib).
//!
//! Cargo treats each `tests/*.rs` file as its own binary. Files moved under
//! `tests/suite/` become modules of this one target instead.

#[path = "../common/mod.rs"]
mod common;

mod crypto_async_test;
mod crypto_kdf_test;
mod crypto_seal_test;
mod crypto_test;
mod password_hashing_test;
mod random_functions_test;
mod sha256_hmac_test;
mod toml_test;
mod wflhash_hardened_security_test;
mod wflhash_security_test;
