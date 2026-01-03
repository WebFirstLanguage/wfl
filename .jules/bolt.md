## 2026-01-03 - [Remove inefficient lexer string interning]
**Learning:** The lexer was using a global `Mutex<HashMap<String, String>>` to "intern" strings, but it was returning `String` (cloned) and storing `String` (cloned), effectively doubling allocations and adding lock contention without any benefit.
**Action:** Always verify "interning" implementations actually return a handle or reference (like `Rc`, `Arc`, or integer ID) rather than an owned clone. If it returns an owned clone, it's just a cache that leaks memory.
