#### **Summary of Changes**

* **The Issue:** Manual extraction of `Value::Pattern` into `Rc<CompiledPattern>` was duplicated across multiple pattern matching native functions (`pattern_matches_native`, `pattern_find_native`, `pattern_find_all_native`, `native_pattern_replace`, and `native_pattern_split`) in `src/stdlib/pattern.rs`. Argument counting was also manually performed in many of these functions.
* **The Rational:** Reduced binary size, improved maintainability, reduced duplicated boilerplate code, and made error messages more consistent.
* **The Solution:** Added a new `expect_pattern` macro helper to `src/stdlib/helpers.rs` using `generate_expect!`. Refactored `src/stdlib/pattern.rs` native functions to utilize `check_arg_count`, `expect_text`, and the new `expect_pattern` helper. Updated tests to reflect the standardized error messages produced by these helpers.

#### **Verification Checklist**

* [x] `cargo fmt` executed and passed.
* [x] `cargo clippy` returned no warnings or errors.
* [x] All `cargo test` suites passed (100% success rate).
