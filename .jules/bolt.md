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

## 2026-03-01 - [Avoid format! for simple string concatenation]
**Learning:** The `format!` macro has significant overhead (parsing, dynamic dispatch) for simple string concatenation like `a + b`. In tight loops, this dominates execution time.
**Action:** Replaced `format!("{a}{b}")` with `String::with_capacity` and `push_str` for the common case where both operands are strings. This yielded an ~18% performance improvement in string-heavy workloads.

## 2026-03-01 - [Avoid redundant scope lookups on variable assignment]
**Learning:** Checking for variable existence in the `constants` map before attempting to access the `values` map leads to redundant `HashMap` lookups, significantly reducing execution speed in loops where environment scopes are checked recursively.
**Action:** Always attempt the primary map update via `get_mut` first, then only check the secondary criteria (`constants`) if a match is found. This effectively halves the number of hash map lookups.

## 2026-03-01 - [Optimize list concat with pre-allocation]
**Learning:** Calling `clone()` on a list and then `extend()` with another list causes an unnecessary memory reallocation, making list concatenation inefficient for large lists.
**Action:** Pre-calculate the combined length and use `Vec::with_capacity` followed by `extend()` from both iterators. This avoids reallocation and yields a performance improvement.

## 2026-03-05 - [Optimize Unicode Text Casing Fast Paths]
**Learning:** When trying to avoid string allocations for `touppercase` and `tolowercase` if the string is already in the target case, using a simple check like `!text.chars().any(char::is_lowercase)` is flawed due to complex Unicode casing rules (e.g., modifier marks or Titlecase characters like `ǅ`). These characters might not be lowercase, but they still change when uppercase is applied.
**Action:** Always verify that every character actually remains identical under the casing transformation. Use `.chars().all(|c| { let mut iter = c.to_uppercase(); iter.next() == Some(c) && iter.next().is_none() })` to safely identify if an allocation-free fast path can be taken.

## 2026-03-29 - [Avoid collect::<String>() on Chars iterator]
**Learning:** Using `.collect::<String>()` on a `Chars` iterator (e.g. from `.chars().rev()`) is inefficient because the iterator's `size_hint()` provides a loose lower bound. This forces `String` to guess its required capacity, leading to multiple intermediate reallocations as the string is built up.
**Action:** For string operations where the exact byte capacity is known (like reversing a string, which preserves the number of bytes), pre-allocate a string using `String::with_capacity(text.len())` and `.push()` characters manually. This guarantees exactly one allocation.
## 2026-04-18 - [Optimize Linter Identifier Case Checking]
**Learning:** Checking string characteristics via `char::is_uppercase()` iteration is slow due to UTF-8 decoding and Unicode property lookups. Also, transforming strings by iterating and mapping characters via `chars().enumerate().map().collect::<String>()` causes excessive reallocations because `collect()` cannot know the exact string size ahead of time.
**Action:** When parsing identifiers or keywords, introduce a fast-path for purely ASCII strings via `.is_ascii()` and `.bytes().any()`. When capitalizing or transforming string case, extract the first character manually and use `String::with_capacity` followed by `.push_str(chars.as_str())` for the remainder to guarantee a single memory allocation.
