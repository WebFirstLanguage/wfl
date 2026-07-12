//! Cross-cutting execution primitives shared by the whole runtime.
//!
//! The only inhabitant today is [`budget::ExecutionBudget`], a single object
//! that travels through parsing, evaluation, pattern matching, web handling,
//! and module loading. It consolidates a scatter of previously-isolated caps
//! (the interpreter timeout, the pattern-VM step limit, the web-server body and
//! queue bounds, and so on) behind one coherent, thread-safe mechanism.

pub mod budget;

pub use budget::{BudgetExceeded, BudgetLimits, ExecutionBudget};
