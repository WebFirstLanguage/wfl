## 2026-01-03 - [Remove inefficient lexer string interning]
**Learning:** The lexer was using a global `Mutex<HashMap<String, String>>` to "intern" strings, but it was returning `String` (cloned) and storing `String` (cloned), effectively doubling allocations and adding lock contention without any benefit.
**Action:** Always verify "interning" implementations actually return a handle or reference (like `Rc`, `Arc`, or integer ID) rather than an owned clone. If it returns an owned clone, it's just a cache that leaks memory.

## 2026-01-03 - [Remove excessive token cloning in parser]
**Learning:** The parser was routinely using `self.cursor.peek().cloned()` to check token types, which forces a deep copy of the `Token` (including heap-allocated Strings for identifiers).
**Action:** Use `self.cursor.peek()` to inspect tokens by reference. The `Cursor` API is designed such that the returned reference lifetime is tied to the underlying token slice, not the `&self` borrow of the cursor, allowing safe inspection without cloning even when `&mut self` is needed later (as long as the reference is dropped before mutation).

## 2026-01-17 - [Optimized Lexer Position Tracking for Strings]
**Learning:** Using `str::contains` twice (once for `\n`, once for `\r`) to check for newlines scans the entire string twice. For strings with newlines at the end, this is inefficient compared to `find`.
**Action:** Use `find` to locate the first occurrence of a delimiter, then use the index to limit the search for other delimiters. This avoids redundant scanning of the prefix and allows jumping directly to the interesting part of the string.

## 2026-01-20 - [Optimize Substring with Zero-Copy Slicing]
**Learning:** `chars().skip(n).take(m).collect::<String>()` is inefficient because it iterates, allocates an intermediate `String`, and re-encodes UTF-8. Using `char_indices()` to find byte boundaries and slicing `&str` allows `Rc::from` to copy bytes directly, avoiding intermediate allocation and UTF-8 overhead.
**Action:** Prefer `char_indices` + slicing over `chars().collect()` when extracting substrings. Also, check for "full string" requests (`start=0` and `length>=len`) to avoid allocation entirely by cloning the `Rc`.

## 2026-01-24 - [Avoid Async Box Allocation for Simple Expressions]
**Learning:** `evaluate_expression` was wrapping every call in `Box::pin` for async recursion, even for simple arithmetic operations like `1 + 2`. This caused significant overhead in tight loops.
**Action:** Implemented `try_evaluate_expression_sync` to recursively evaluate simple expressions (Literals, Variables, Binary/Unary ops) synchronously, bypassing `Box::pin` allocation. This yielded a ~30% performance improvement in arithmetic-heavy loops.
