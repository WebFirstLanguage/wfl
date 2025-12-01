# WFL Standard Library Reference

## Overview

The WFL standard library provides essential built-in functions for common programming tasks. All functions are exposed as natural-language function calls and are implemented as Rust intrinsics for maximum performance.

**Quick Navigation:** Jump directly to module documentation below.

---

## Standard Library Modules

### [Core Module](core-module.md)
**Basic utilities and system functions**

Functions: `print()`, `typeof()`, `isnothing()`, `is_nothing()`

Essential functions for output, type checking, and null value handling.

---

### [Math Module](math-module.md)
**Mathematical operations and numeric functions**

Functions: `abs()`, `round()`, `floor()`, `ceil()`, `clamp()`

Core mathematical operations for numeric computations.

---

### [Random Module](random-module.md)
**Cryptographically secure random number generation**

Functions: `random()`, `random_between()`, `random_int()`, `random_boolean()`, `random_from()`, `random_seed()`

Secure random number generation suitable for security-sensitive applications.

---

### [Text Module](text-module.md)
**String manipulation and text processing**

Functions: `length()`, `touppercase()`, `tolowercase()`, `contains()`, `substring()`, `trim()`, `starts_with()`, `ends_with()`, `string_split()`

Comprehensive text processing with full Unicode support.

---

### [List Module](list-module.md)
**List and collection operations**

Functions: `length()`, `push()`, `pop()`, `contains()`, `indexof()`, `index_of()`

Essential list manipulation for working with collections.

---

### [Crypto Module](crypto-module.md)
**Cryptographic hash functions and message authentication**

Functions: `wflhash256()`, `wflhash512()`, `wflhash256_with_salt()`, `wflmac256()`

Custom WFLHASH cryptographic functions for integrity and authentication.

⚠️ **Not for password hashing** - Use Argon2id for passwords.

---

### [Time Module](time-module.md)
**Date and time operations**

Functions: `today()`, `now()`, `datetime_now()`, `format_date()`, `format_time()`, `format_datetime()`, `parse_date()`, `parse_time()`, `create_time()`, `create_date()`, `add_days()`, `days_between()`, `current_date()`

Comprehensive date and time handling with formatting and parsing.

---

### [Filesystem Module](filesystem-module.md)
**File system operations and path utilities**

Functions: `list_dir()`, `glob()`, `rglob()`, `path_join()`, `path_basename()`, `path_dirname()`, `makedirs()`, `file_mtime()`, `path_exists()`, `is_file()`, `is_dir()`, `count_lines()`

File system navigation and path manipulation.

---

### [Container System](container-system.md)
**Object-oriented programming and container API**

Object-oriented programming features including containers (classes), interfaces, properties, methods, and events.

See also: [WFL-containers.md](../wfldocs/WFL-containers.md) for language syntax.

---

### [Pattern Module](pattern-module.md) (Legacy)
**Pattern matching API - Legacy interface**

⚠️ **Note:** This is the legacy pattern matching API. For current pattern matching features, see [WFL-patterns.md](../wfldocs/WFL-patterns.md).

---

### [Async Patterns](async-patterns.md)
**Asynchronous programming patterns and best practices**

Common async/await patterns and concurrent programming techniques.

See also: [WFL-async.md](../wfldocs/WFL-async.md) for complete async language reference.

---

## Module Organization

### By Category

**System & Core:**
- [Core Module](core-module.md) - Essential utilities

**Data Types:**
- [Math Module](math-module.md) - Numeric operations
- [Text Module](text-module.md) - String processing
- [List Module](list-module.md) - Collections
- [Time Module](time-module.md) - Dates and times

**I/O & System:**
- [Filesystem Module](filesystem-module.md) - File operations
- [Crypto Module](crypto-module.md) - Cryptographic hashing

**Advanced:**
- [Container System](container-system.md) - OOP features
- [Async Patterns](async-patterns.md) - Concurrent programming
- [Pattern Module](pattern-module.md) - Legacy patterns

---

## Implementation Status

Most stdlib functions are fully implemented. For functions with partial or planned implementation:

- **Crypto Module:** ✅ Fully implemented (5 functions)
- **Text Module:** ✅ Fully implemented (8 functions)
- **Math Module:** ⚠️ Partial (5/8 functions) - See [VALIDATION-NOTES.md](../VALIDATION-NOTES.md)
- **Time Module:** ⚠️ Partial (13/18 functions) - See [VALIDATION-NOTES.md](../VALIDATION-NOTES.md)
- **List Module:** ⚠️ Partial (6/11 functions) - See [VALIDATION-NOTES.md](../VALIDATION-NOTES.md)
- **Filesystem Module:** ⚠️ Partial (12/19 functions) - See [VALIDATION-NOTES.md](../VALIDATION-NOTES.md)

For detailed implementation status of each function, see individual module documentation pages.

---

## Function Naming Conventions

WFL supports multiple naming styles for ease of use:

**Snake case (preferred):**
```wfl
store result as to_uppercase of text
store index as index_of of list and item
```

**Camel case (also supported):**
```wfl
store result as toUppercase of text
store nothing_check as isNothing of value
```

**Natural language (where available):**
```wfl
store result as absolute value of number
check if text contains substring
```

---

## Cross-References

- **Language Features:** [WFL Language Specification](../wfldocs/WFL-spec.md)
- **I/O Operations:** [WFL I/O Reference](../wfldocs/WFL-io.md)
- **Async Programming:** [WFL Async Reference](../wfldocs/WFL-async.md)
- **Pattern Matching:** [WFL Patterns Reference](../wfldocs/WFL-patterns.md)
- **Error Handling:** [WFL Errors Reference](../wfldocs/WFL-errors.md)

---

## Getting Started

For a practical introduction to using the standard library:

1. **Start here:** [Getting Started Guide](../guides/wfl-getting-started.md)
2. **Learn by example:** [WFL Cookbook](../guides/wfl-cookbook.md)
3. **See patterns:** [WFL by Example](../guides/wfl-by-example.md)

---

## Version Information

**WFL Version:** 25.11.10
**Last Updated:** 2025-12-01
**Status:** Active documentation - trimmed to navigation index

---

## Document History

**2025-12-01:** Converted to navigation index only (Week 3 Day 4)
- Removed duplicated API content (now in individual module files)
- Kept module summaries and navigation structure
- Added implementation status summary
- Added cross-references to related documentation

Previous version with embedded API documentation available in git history.
