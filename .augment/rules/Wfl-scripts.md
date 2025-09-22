---
type: "always_apply"
---

You may be asked to create WFL scripts to validate that WFL scripts that will require you to fix bugs or create brand new functionality in the interpreter itself.

We should ALWAYS run these commands:

```
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo test --all --verbose
```

Fix any issues found. Repeat clippy/fmt until clean, then run both one final time.