# WFL Language Specification

Formal specification of the WebFirst Language (WFL). This is a technical reference for implementers and advanced users. Everyday learning should start with the [Introduction](../01-introduction/index.md) and [Language Basics](../03-language-basics/index.md).

WFL’s design follows the [19 guiding principles](../wfl-foundation.md) and the **no-unlearning invariant**: beginner forms and expert forms are the same language, connected by growth rather than replacement.

## Language Overview

| Field | Value |
|-------|--------|
| **Name** | WebFirst Language (WFL) |
| **Version** | 26.7.30 |
| **Status** | Active development |
| **Paradigm** | Multi-paradigm (imperative, object-oriented containers, natural-language constructs) |
| **Type system** | Static with inference |
| **Execution** | Interpreted (direct AST execution, Tokio async) |

## Lexical Structure

### Keywords

**181** keywords and literals, organized by purpose and type.

**Keyword types:**
- 54 Structural Keywords (core language constructs)
- 29 Contextual Keywords (context-dependent usage)
- 96 Other Reserved Keywords (feature-specific)
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

Parameters may declare a type with `as` (e.g. `value as number`). An action
name may be defined more than once in the same scope (overloading) when every
pair of same-name definitions differs in parameter count or has at least one
position where both declare concrete, different parameter types. Exact
duplicates, and same-count pairs with no such distinguishing position, are
definition-time errors. Calls resolve by filtering candidates on argument
count, then on argument types (statically when known, otherwise on the runtime
values); among several runtime matches the version with the most
concretely-matched parameters wins, with remaining ties resolved in definition
order. A `nothing` argument is compatible with every parameter type; a
parameter annotated with a container name accepts instances of that container
or any descendant via `extends`. For an action participating in overloading,
declared parameter types are enforced when the action executes — a call whose
argument a declared type rejects is a runtime error, so a call between two
definitions dispatches over the overloads defined so far. This enforcement is
scoped to the statement block whose definitions form the overload set (or
begins when definitions from different blocks merge into one scope): an
action defined exactly once in its block is not runtime-checked; its
annotations inform static analysis only. A variable bound to an action by a
bare reference (`store h as f`) is callable and dispatches with the
signatures the action had at the point of the binding (snapshot semantics),
statically and at runtime; a binding whose state cannot be determined
statically — reassigned in one branch of a conditional, or bound after a
definition that sits inside a branch or loop — defers wholly to runtime
dispatch. Container methods do not support overloading.

**Action Call:**
```
call <identifier> [with <argument-list>]
```

**Try / when / catch / finally:**
```
try:
    <statements>
[when error [as <identifier>]:
    <statements>]*
[when <error-type>:
    <statements>]*
[catch:
    <statements>]
[finally:
    <statements>]
end try
```

`finally` always runs after the try body and any matching handler (success or handled error).

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
