#### **Summary of Changes**

* **The Issue:** There was significant boilerplate redundancy in `src/stdlib/pattern.rs` involving argument counting (`args.len() != 2`) and manual type coercion matching on `Value::Text` and `Value::Pattern` variants across native functions (`pattern_matches_native`, `pattern_find_native`, `pattern_find_all_native`, `native_pattern_replace`, and `native_pattern_split`), returning multiple independent string copies for error messages.
* **The Rational:** Improved maintainability, consolidated boilerplate code, reduced file size, and adhered better to DRY principles.
* **The Solution:** Implemented a new helper `expect_pattern` using `generate_expect!` in `src/stdlib/helpers.rs` and replaced the manual `match` blocks and length checks with standard library helpers (`check_arg_count`, `expect_text`, and `expect_pattern`), propagating context lines and columns when necessary via `.map_err()`. Also updated unit tests in `src/stdlib/pattern_test.rs` to reflect the new standardized error messages.

#### **Verification Checklist**

* [x] `cargo fmt` executed and passed.
* [x] `cargo clippy` returned no warnings or errors.
* [x] All `cargo test` suites passed (100% success rate).
