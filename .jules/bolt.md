## 2026-01-03 - [Remove inefficient lexer string interning]
**Learning:** The lexer was using a global `Mutex<HashMap<String, String>>` to "intern" strings, but it was returning `String` (cloned) and storing `String` (cloned), effectively doubling allocations and adding lock contention without any benefit.
**Action:** Always verify "interning" implementations actually return a handle or reference (like `Rc`, `Arc`, or integer ID) rather than an owned clone. If it returns an owned clone, it's just a cache that leaks memory.

## 2026-01-03 - [Remove excessive token cloning in parser]
**Learning:** The parser was routinely using `self.cursor.peek().cloned()` to check token types, which forces a deep copy of the `Token` (including heap-allocated Strings for identifiers).
**Action:** Use `self.cursor.peek()` to inspect tokens by reference. The `Cursor` API is designed such that the returned reference lifetime is tied to the underlying token slice, not the `&self` borrow of the cursor, allowing safe inspection without cloning even when `&mut self` is needed later (as long as the reference is dropped before mutation).

## 2026-01-17 - [Optimized Lexer Position Tracking for Strings]
**Learning:** Using `str::contains` twice (once for `\n`, once for `\r`) to check for newlines scans the entire string twice. For strings with newlines at the end, this is inefficient compared to `find`.
**Action:** Use `find` to locate the first occurrence of a delimiter, then use the index to limit the search for other delimiters. This avoids redundant scanning of the prefix and allows jumping directly to the interesting part of the string.

## 2025-05-18 - [Token Vector Pre-allocation Heuristic]
**Learning:** WFL code token density varies significantly (strings vs code). `input.len() / 10` provides a good balance for `Vec` pre-allocation, improving lexing of dense code (no strings) by ~23% while keeping string-heavy code performance stable. Denser heuristics (e.g. `/5`) caused regressions, likely due to memory pressure from over-allocation.
**Action:** When pre-allocating vectors based on input size, conservative heuristics (under-estimating) are often safer than aggressive ones (over-estimating), especially when element size is large.
