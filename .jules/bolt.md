## 2024-03-19 - Optimize percent decoding

**Learning:** Returning an owned `String` from `percent_decode` causes unnecessary intermediate allocations in `parse_key_value_pairs` for strings that do not contain percent-encoded characters (`%` or `+`). Fast-path scanning avoids these allocations.
**Action:** Always check if a transformation is needed before allocating. Use `std::borrow::Cow` to return `Cow::Borrowed(&str)` for unchanged data, and `Cow::Owned(String)` only when actual decoding occurs.
