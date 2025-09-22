---
type: "always_apply"
---

You may be asked to creat wfl scripts to validate that wfl that will require you to fix bugs or create brand new functionality in the interpretewr itself.

We should ALWAYS run "cargo clippy --all-targets -- -D warnings" and "cargo fmt --all -- --check" fix any issues found. we repeat untill all errors are resolved. then run "cargo test --verbose" to verify we did not break anything. then run clippy and fmt one final time