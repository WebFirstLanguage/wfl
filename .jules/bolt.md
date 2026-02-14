## 2026-01-03 - [Remove inefficient lexer string interning]
**Learning:** The lexer was using a global `Mutex<HashMap<String, String>>` to "intern" strings, but it was returning `String` (cloned) and storing `String` (cloned), effectively doubling allocations and adding lock contention without any benefit.
**Action:** Always verify "interning" implementations actually return a handle or reference (like `Rc`, `Arc`, or integer ID) rather than an owned clone. If it returns an owned clone, it's just a cache that leaks memory.

## 2026-01-03 - [Remove excessive token cloning in parser]
**Learning:** The parser was routinely using `self.cursor.peek().cloned()` to check token types, which forces a deep copy of the `Token` (including heap-allocated Strings for identifiers).
**Action:** Use `self.cursor.peek()` to inspect tokens by reference. The `Cursor` API is designed such that the returned reference lifetime is tied to the underlying token slice, not the `&self` borrow of the cursor, allowing safe inspection without cloning even when `&mut self` is needed later (as long as the reference is dropped before mutation).

## 2026-01-17 - [Optimized Lexer Position Tracking for Strings]
**Learning:** Using `str::contains` twice (once for `\n`, once for `\r`) to check for newlines scans the entire string twice. For strings with newlines at the end, this is inefficient compared to `find`.
**Action:** Use `find` to locate the first occurrence of a delimiter, then use the index to limit the search for other delimiters. This avoids redundant scanning of the prefix and allows jumping directly to the interesting part of the string.

## 2026-01-18 - [Token Vector Pre-allocation Heuristic]
**Learning:** WFL code token density varies significantly (strings vs code). `input.len() / 10` provides a good balance for `Vec` pre-allocation, improving lexing of dense code (no strings) by ~23% while keeping string-heavy code performance stable. Denser heuristics (e.g. `/5`) caused regressions, likely due to memory pressure from over-allocation.
**Action:** When pre-allocating vectors based on input size, conservative heuristics (under-estimating) are often safer than aggressive ones (over-estimating), especially when element size is large.

## 2026-01-20 - [Optimize Substring with Zero-Copy Slicing]
**Learning:** `chars().skip(n).take(m).collect::<String>()` is inefficient because it iterates, allocates an intermediate `String`, and re-encodes UTF-8. Using `char_indices()` to find byte boundaries and slicing `&str` allows `Rc::from` to copy bytes directly, avoiding intermediate allocation and UTF-8 overhead.
**Action:** Prefer `char_indices` + slicing over `chars().collect()` when extracting substrings. Also, check for "full string" requests (`start=0` and `length>=len`) to avoid allocation entirely by cloning the `Rc`.

## 2026-01-24 - [Avoid Async Box Allocation for Simple Expressions]
**Learning:** `evaluate_expression` was wrapping every call in `Box::pin` for async recursion, even for simple arithmetic operations like `1 + 2`. This caused significant overhead in tight loops.
**Action:** Implemented `try_evaluate_expression_sync` to recursively evaluate simple expressions (Literals, Variables, Binary/Unary ops) synchronously, bypassing `Box::pin` allocation. This yielded a ~30% performance improvement in arithmetic-heavy loops.

## 2026-02-05 - [Batch Interpreter Timeout Checks]
**Learning:** Checking `Instant::elapsed()` on every instruction creates significant overhead (15-20%) in tight loops due to syscalls/hardware clock reads.
**Action:** Implemented a batched check using a simple instruction counter (`op_count & 1023 == 0`), only checking the system clock every 1024 operations. This maintains safety (timeouts are still enforced, just with slightly coarser granularity) while significantly reducing per-instruction overhead.

## 2026-02-12 - [Optimize Variable Declaration in Deep Scopes]
**Learning:** `Statement::VariableDeclaration` was performing a redundant environment chain traversal. It first called `get(name)` to check for existence (traversing up), and then called `define(name, value)` which *also* traversed up to check for shadowing. In deep scopes (e.g. recursion), this doubled the cost of variable declaration (2 * depth).
**Action:** Implemented `Environment::define_direct` which skips the parent scope check (since `get` already confirmed absence), reducing complexity from 2N to 1N for new variable declarations. This improved performance by ~11% in deep recursion benchmarks.

## 2026-02-18 - [Unified and Optimized Value Equality]
**Learning:** Three different equality implementations existed (`Value::eq`, `Interpreter::is_equal`, `values_equal`), leading to inconsistent behavior (e.g., `[1] == [1]` was false in WFL code but true in Rust `PartialEq`). Additionally, `Value::eq` unconditionally allocated a `HashSet` for cycle detection, penalizing simple primitive comparisons.
**Action:** Optimized `Value::eq` with a fast path for primitives (avoiding allocation) and updated all call sites to use it. This unified equality logic, fixed correctness bugs for containers, and improved performance for primitives.

## 2026-02-28 - [Use Rc<str> for String Literals]
**Learning:** `Literal::String` stored an owned `String`, causing a deep copy every time the literal was evaluated (e.g., in a loop). Since string literals are immutable and constant after parsing, they should be shared.
**Action:** Changed `Literal::String(String)` to `Literal::String(Rc<str>)`. This avoids heap allocation during runtime evaluation, reducing it to a reference count increment. Resulted in ~8% speedup in tight loops involving string literals.
## 2026-02-14 - Environment Assignment Optimization
**Learning:** Rust's `HashMap::insert` requires an owned key `K` even if the key already exists in the map, causing unnecessary allocation when updating values keyed by `String`. Using `get_mut` avoids this by only requiring a borrowed `&Q` (where `K: Borrow<Q>`).
**Action:** Always prefer `get_mut` over `contains_key` + `insert` when updating values in HashMaps, especially with `String` keys, to avoid double hashing and allocation.
