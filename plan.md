1. **Optimize `native_replace` in `src/stdlib/text.rs`**:
   - Update `native_replace` to use `!text.contains(old.as_ref())` check before calling `replace`.
   - If the string does not contain `old`, just return `Value::Text(Arc::clone(&text))`.
   - Otherwise, perform the replacement and allocate a new `Arc<str>`.
   - This avoids unnecessary allocation.
2. **Update `.jules/bolt.md` with the learning**:
   - Add an entry about optimizing string replacements to use `contains` before allocating.
3. **Pre-commit step**:
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
4. **Submit PR**:
   - Use source control commands to branch out, commit, push, and submit a PR for Bolt performance improvement.
