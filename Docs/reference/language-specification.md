# WFL Language Specification

Formal specification of the WebFirst Language (WFL). Technical reference for implementers and advanced users.

## Language Overview

**Name:** WebFirst Language (WFL)
**Version:** 26.1.17 (January 2026)
**Status:** Alpha
**Paradigm:** Multi-paradigm (imperative, functional, object-oriented)
**Type System:** Static with inference
**Execution:** Interpreted (direct AST execution)

## Lexical Structure

### Keywords

178 reserved keywords organized by purpose and type.

**Keyword Types:**
- 52 Structural Keywords (core language constructs)
- 29 Contextual Keywords (context-dependent usage)
- 95 Other Reserved Keywords (feature-specific)
- 7 Boolean & Null Literals

**See:** [Quick Reference](keyword-reference.md) | [Complete Reference](reserved-keywords.md)

### Identifiers

**Valid identifier:**
- Starts with letter or underscore
- Contains letters, digits, underscores
- Can include spaces
- Case-sensitive
- Cannot be reserved keyword

**Examples:**
- `user_name` (valid)
- `user name` (valid with spaces)
- `_private` (valid)
- `item2` (valid)
- `2item` (invalid - starts with digit)

### Literals

**Number:** `42`, `3.14`, `-5`, `0`
**Text:** `"hello"`, `"with \"quotes\""`, `"line1\nline2"`
**Boolean:** `yes`, `no`, `true`, `false`
**Nothing:** `nothing`, `missing`, `undefined`
**List:** `[1, 2, 3]`, `["a", "b"]`

### Comments

**Single-line:** `// comment text`

### Operators

See [Operator Reference](operator-reference.md) for complete list.

## Type System

### Basic Types

- **Number** - 64-bit floating point
- **Text** - UTF-8 strings
- **Boolean** - yes/no (true/false)
- **Null** - nothing value
- **List** - Ordered collection
- **Container** - User-defined types
- **Pattern** - Compiled pattern
- **Date/Time/DateTime** - Temporal types

### Type Inference

WFL infers types from values:

```wfl
store x as 42        // Inferred: Number
store s as "hello"   // Inferred: Text
store b as yes       // Inferred: Boolean
```

### Type Checking

Static type analysis prevents:
- Adding incompatible types
- Calling undefined functions
- Invalid operations

## Syntax

### Statements

**Variable Declaration:**
```
store <identifier> as <expression>
```

**Variable Assignment:**
```
change <identifier> to <expression>
```

**Display:**
```
display <expression>
```

**If Statement:**
```
check if <condition>:
    <statements>
[otherwise:
    <statements>]
end check
```

**Count Loop:**
```
count from <start> to <end> [by <step>]:
    <statements>
end count
```

**For Each Loop:**
```
for each <identifier> in <expression>:
    <statements>
end for
```

**Action Definition:**
```
define action called <identifier> [with parameters <param-list>]:
    <statements>
end action
```

**Action Call:**
```
call <identifier> [with <argument-list>]
```

**Try-Catch:**
```
try:
    <statements>
[when <error-type>:
    <statements>]*
[catch:
    <statements>]
[finally:
    <statements>]
end try
```

**Container Definition:**
```
create container <identifier> [extends <identifier>] [implements <identifier-list>]:
    [property <identifier>: <type>]*
    [action <identifier> [with parameters <param-list>]:
        <statements>
    end]*
end
```

### Expressions

**Literals:** numbers, text, booleans, lists
**Variables:** identifiers
**Binary operations:** `<expr> <operator> <expr>`
**Function calls:** `<function> of <argument> [and <argument>]*`
**Action calls:** `<identifier> with <argument> [and <argument>]*`
**Concatenation:** `<expr> with <expr>`

## Scoping Rules

**Global scope:** Top-level declarations accessible everywhere
**Function scope:** Action parameters and locals accessible within action
**Block scope:** Variables in loops/conditionals accessible within block

## Execution Model

**Pipeline:**
```
Source → Lexer → Parser → Analyzer → Type Checker → Interpreter
```

**Execution:** Direct AST interpretation with Tokio async runtime

## Standard Library

181+ built-in functions across 11 modules.

**[Complete reference →](builtin-functions-reference.md)**

---

**Previous:** [← Error Codes](error-codes.md) | **Next:** [Development →](../development/)
