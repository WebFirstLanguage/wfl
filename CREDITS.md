# Credits

WFL would not exist without the amazing open source community. If we missed any credits, please open an issue and let us know so we can give proper credit to the projects that make WFL possible.

This project uses the following third-party Rust packages:

## logos (v0.15.0)
- **Description**: A parser library that creates tokenizers through procedural macros and minimal code.
- **License**: MIT
- **Repository**: https://github.com/maciejhirsz/logos
- **Usage in WFL**: Used for lexical analysis (tokenization) of WFL source code.

## rand (v0.9.1)
- **Description**: A Rust library for random number generation.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/rust-random/rand
- **Usage in WFL**: Provides random number generation capabilities for the WFL standard library.

## regex (v1.10.3)
- **Description**: An implementation of regular expressions for Rust.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/rust-lang/regex
- **Usage in WFL**: Powers pattern matching and text processing features in WFL.

## log (v0.4.20)
- **Description**: A lightweight logging facade for Rust.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/rust-lang/log
- **Usage in WFL**: Provides logging infrastructure throughout the WFL implementation.

## rustyline (v12.0.0)
- **Description**: Readline implementation in Rust.
- **License**: MIT
- **Repository**: https://github.com/kkawakam/rustyline
- **Usage in WFL**: Powers the interactive REPL experience with command history and editing.

## tokio (v1.35.1)
- **Description**: An event-driven, non-blocking I/O platform for writing asynchronous applications.
- **License**: MIT
- **Repository**: https://github.com/tokio-rs/tokio
- **Usage in WFL**: Enables asynchronous operations and concurrency in WFL programs.

## reqwest (v0.11.24)
- **Description**: A high-level HTTP client library.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/seanmonstar/reqwest
- **Usage in WFL**: Provides HTTP capabilities for WFL's network operations.

## sqlx (v0.8.1)
- **Description**: A async, pure Rust SQL crate featuring compile-time checked queries.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/launchbadge/sqlx
- **Usage in WFL**: Enables database connectivity for WFL programs.

## serde_json (v1.0.114)
- **Description**: A JSON serialization/deserialization library.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/serde-rs/json
- **Usage in WFL**: Handles JSON data processing in WFL programs.

## codespan-reporting (v0.11.1)
- **Description**: Beautiful diagnostic reporting for text-based programming languages.
- **License**: Apache-2.0
- **Repository**: https://github.com/brendanzab/codespan
- **Usage in WFL**: Provides error reporting with source code context for WFL programs.

## simplelog (v0.12.1)
- **Description**: A simple logging implementation for the log crate.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/drakulix/simplelog.rs
- **Usage in WFL**: Configures logging output for the WFL interpreter.

## chrono (v0.4.31)
- **Description**: Date and time library for Rust.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/chronotope/chrono
- **Usage in WFL**: Provides date and time functionality in WFL programs.

## once_cell (v1.18.0)
- **Description**: Single assignment cells and lazy values for Rust.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/matklad/once_cell
- **Usage in WFL**: Enables lazy initialization patterns in the WFL interpreter.

## time (v0.3)
- **Description**: A time library for Rust.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/time-rs/time
- **Usage in WFL**: Provides additional time handling functionality.

## warp (v0.3.7)
- **Description**: A fast, composable web server framework.
- **License**: MIT
- **Repository**: https://github.com/seanmonstar/warp
- **Usage in WFL**: Powers WFL's web server capabilities and HTTP request/response handling.

## uuid (v1.6.1)
- **Description**: A library for generating and parsing UUIDs.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/uuid-rs/uuid
- **Usage in WFL**: Provides unique identifier generation for WFL applications.

## bytes (v1.5.0)
- **Description**: A utility library for working with bytes.
- **License**: MIT
- **Repository**: https://github.com/tokio-rs/bytes
- **Usage in WFL**: Efficient byte buffer management for network and I/O operations.

## glob (v0.3.2)
- **Description**: Support for matching file paths against Unix shell style patterns.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/rust-lang/glob
- **Usage in WFL**: File pattern matching for path operations in WFL programs.

## zeroize (v1.7)
- **Description**: Securely clear secrets from memory.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/RustCrypto/utils/tree/master/zeroize
- **Usage in WFL**: Ensures sensitive cryptographic data is properly cleared from memory.

## subtle (v2.5)
- **Description**: Pure-Rust traits and utilities for constant-time cryptographic implementations.
- **License**: BSD-3-Clause
- **Repository**: https://github.com/dalek-cryptography/subtle
- **Usage in WFL**: Provides constant-time operations for cryptographic security in WFLHASH.

## sha2 (v0.10)
- **Description**: SHA-2 hash functions.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/RustCrypto/hashes
- **Usage in WFL**: Implements SHA-256 and SHA-512 hash functions for cryptographic operations.

## hkdf (v0.12)
- **Description**: HMAC-based Extract-and-Expand Key Derivation Function (HKDF).
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/RustCrypto/KDFs
- **Usage in WFL**: Key derivation for WFLHASH custom cryptographic hash function.

## num-bigint-dig (v0.8.6)
- **Description**: Big integer implementation for Rust.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/dignifiedquire/num-bigint
- **Usage in WFL**: Dependency for cryptographic operations requiring large integer arithmetic.

## dhat (v0.3.0)
- **Description**: A heap profiling and ad hoc profiling tool.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/nnethercote/dhat-rs
- **Usage in WFL**: Optional dependency for heap profiling during development and performance analysis.

## Development Dependencies

The following packages are used during development and testing:

## tempfile (v3.19.1)
- **Description**: A library for creating temporary files and directories.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/Stebalien/tempfile
- **Usage in WFL**: Creates isolated temporary files for testing file I/O operations.

## libc (v0.2.152)
- **Description**: Raw FFI bindings to platform libraries like libc.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/rust-lang/libc
- **Usage in WFL**: Provides low-level system interface bindings for testing platform-specific behavior.

## criterion (v0.4)
- **Description**: Statistics-driven benchmarking library.
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/bheisler/criterion.rs
- **Usage in WFL**: Powers the benchmarking suite for parser and lexer performance measurements.

## Rust Standard Library
- **License**: MIT/Apache-2.0
- **Repository**: https://github.com/rust-lang/rust

## Research Sources

During development, the following resources were researched and considered for WFL implementation:

### Parsing Libraries
- **Pest**: A parser generator using Parsing Expression Grammars (PEG)
- **rust-peg**: A simple PEG parser generator using Rust macros
- **Nom**: A parser combinator library focused on safe and fast parsing
- **LALRPOP**: A LR(1) parser generator (like YACC/ANTLR)
- **Chumsky**: A modern parser combinator library with error recovery features

### I/O and Async Execution
- **Mio**: A low-level non-blocking I/O library for event-driven programming
- **Smol**: A small and fast async runtime
- **Async-std**: An async runtime with an API modeled after the standard library

### HTTP Client Libraries
- **Ureq**: A simple, synchronous HTTP client focused on ease of use
- **AttoHTTPc**: A lightweight and simple HTTP client
- **Minreq**: A minimal-dependency HTTP client

### Database Access Libraries
- **Rusqlite**: The most popular SQLite driver for Rust
- **Postgres (rust-postgres)**: A native PostgreSQL client for Rust
- **Tokio-Postgres**: An async version of the Postgres client

### Pattern Matching and Regular Expression Alternatives
- **SNOBOL/Icon**: Pattern matching languages offering alternatives to traditional regexes
- **Raku (Perl 6) Rules**: Advanced pattern matching system with better readability
- **Red Programming Language's Parse**: Domain-specific language for parsing without regex syntax
- **Martin Fowler's Composed Regex**: Concept for more maintainable regex alternatives
- **Cucumber Expressions**: More intuitive syntax for step definitions in behavior-driven development

### Asynchronous Programming Models
- **Tokio Documentation**: Research on event-driven, non-blocking I/O platforms
- **Futures and Rust async/await**: Research on ergonomic asynchronous code patterns
- **Deno Security Model**: Influence for WFL's permission-based security system
- **RustSec Advisory Database**: Best practices for securing file operations
