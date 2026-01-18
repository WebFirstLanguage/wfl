# ARCHIVED: WFL Reserved Keywords (Technical Reference)

**Note:** This technical reference has been integrated into user documentation:
- Quick Reference: `Docs/reference/keyword-reference.md`
- Comprehensive Reference: `Docs/reference/reserved-keywords.md`

This file is preserved for historical reference and technical accuracy verification.

**Date archived:** 2026-01-16
**Integration approach:** Multi-tiered (quick + comprehensive)
**Original source:** Root directory keywords.md

---

# WFL Reserved Keywords

Complete reference of all reserved keywords in the WFL programming language.

**Total Count:** 178 keywords and literals
**Source:** WFL Compiler v26.1.x (extracted from `src/lexer/token.rs`)

**Breakdown:**
- 52 Structural Keywords (cannot use as variables)
- 29 Contextual Keywords (24 can use as variables, 5 are also structural)
- 95 Other Reserved Keywords
- 7 Boolean & Null Literals
- Total unique: 52 + 24 + 95 + 7 = 178

## Table of Contents
- [Keyword Classification](#keyword-classification)
  - [Structural Keywords (52)](#structural-keywords-52)
  - [Contextual Keywords (24 usable as variables, 29 total)](#contextual-keywords-24-usable-as-variables-29-total)
  - [Other Reserved Keywords (95)](#other-reserved-keywords-95)
  - [Boolean & Null Literals (7)](#boolean--null-literals-7)
- [Keywords by Category](#keywords-by-category)
- [Usage Guidelines](#usage-guidelines)
- [Quick Reference Table](#quick-reference-table)

---

## Keyword Classification

### Structural Keywords (52)

These keywords **MUST** always be reserved and **CANNOT** be used as variable names. They define program structure and control flow.

| Keyword | Description | Example Usage |
|---------|-------------|---------------|
| `action` | Define a function/action | `define action called calculate with x:` |
| `and` | Logical AND operator | `check if x is 5 and y is 10:` |
| `any` | Match any (pattern/quantifier) | `define pattern zero or more any` |
| `as` | Binding/assignment keyword | `store name as "Alice"` |
| `break` | Exit loop early | `break` |
| `by` | Step amount in count loops | `count from 1 to 10 by 2:` |
| `call` | Call an action | `call myFunction with x` |
| `catch` | Error handler | `catch when error:` |
| `check` | Start conditional | `check if x is greater than 5:` |
| `constant` | Define constant property | `constant property max_size as 100` |
| `container` | Define a class/container | `define container called Person:` |
| `continue` | Skip to next iteration | `continue` |
| `define` | Define action/container | `define action called test:` |
| `display` | Output text | `display "Hello World"` |
| `each` | For each loop | `for each item in list:` |
| `end` | Close block | `end` |
| `event` | Define an event | `event click` |
| `extends` | Inheritance | `container Person extends Human:` |
| `for` | For loop | `for each x in items:` |
| `forever` | Infinite loop | `repeat forever:` |
| `from` | Count loop start | `count from 1 to 10:` |
| `if` | Conditional | `check if x is 5:` |
| `implements` | Interface implementation | `container Dog implements Animal:` |
| `in` | For each collection | `for each item in list:` |
| `interface` | Interface definition | `define interface called Runnable:` |
| `load` | Load module | `load module math` |
| `module` | Module reference | `load module fs` |
| `not` | Logical NOT | `check if not x:` |
| `on` | Event handler | `on click:` |
| `or` | Logical OR | `check if x is 5 or y is 10:` |
| `otherwise` | Else clause | `otherwise:` |
| `private` | Private visibility | `private property age` |
| `property` | Container property | `property name as "default"` |
| `public` | Public visibility | `public property name` |
| `push` | Add to list | `push with myList and item` |
| `repeat` | Loop construct | `repeat 10 times:` |
| `requires` | Interface requirement | `requires method run` |
| `return` | Return value | `return result` |
| `skip` | Continue (alias) | `skip` |
| `static` | Static member | `static property counter` |
| `store` | Create variable | `store x as 10` |
| `than` | Comparison keyword | `greater than`, `less than` |
| `then` | Conditional separator | `check if x is 5 then display "yes"` |
| `to` | Count loop end | `count from 1 to 10:` |
| `trigger` | Fire event | `trigger myEvent` |
| `try` | Error handling | `try:` |
| `until` | Loop condition | `repeat until x is 10:` |
| `wait` | Async wait | `wait for result` |
| `when` | Specific error type | `catch when FileError:` |
| `while` | Loop condition | `repeat while x is less than 10:` |
| `with` | Parameter separator | `call action with x and y` |
| `zero` | Pattern quantifier | `define pattern zero or more digits` |

**Count:** 52 structural keywords

**Note:** The keywords `any`, `push`, `skip`, `than`, and `zero` also appear in the contextual keyword list in the source code, but since structural keywords are checked first in the parser, they **CANNOT** be used as variable names.

---

### Contextual Keywords (24 usable as variables, 29 total)

These keywords are **context-dependent**. Most **CAN** be used as variable names outside their keyword context, but 5 of them are also structural keywords and **CANNOT** be used as variables.

| Keyword | Description | Context | Can Use As Variable? |
|---------|-------------|---------|----------------------|
| `any` | Match any (pattern) | Pattern matching | ❌ No (also structural) |
| `at` | Location preposition | File/position operations | ✓ Yes |
| `back` | Return keyword part | `give back` expression | ✓ Yes |
| `called` | Action name | `define action called X` | ✓ Yes |
| `change` | Modify variable | Variable modification | ✓ Yes |
| `contains` | Contains check | Collection/string operations | ✓ Yes (can be function name) |
| `count` | Count loop | `count from X to Y` | ✓ Yes (outside count loops) |
| `create` | Create resource | Resource creation | ✓ Yes (in expressions) |
| `defaults` | Default value | Container properties | ✓ Yes |
| `extension` | File extension | File operations | ✓ Yes |
| `extensions` | File extensions | File operations | ✓ Yes |
| `files` | File collection | File operations | ✓ Yes |
| `give` | Return keyword part | `give back` expression | ✓ Yes |
| `least` | Minimum amount | Quantifiers | ✓ Yes |
| `list` | List type | Type/create context | ✓ Yes (outside type context) |
| `map` | Map type | Type/create context | ✓ Yes (outside type context) |
| `most` | Maximum amount | Quantifiers | ✓ Yes |
| `must` | Requirement | Interface/validation | ✓ Yes |
| `needs` | Action parameters | `action needs X` (alias for `with`) | ✓ Yes |
| `new` | Instantiate container | Container creation | ✓ Yes (context-sensitive) |
| `parent` | Parent reference | Container inheritance | ✓ Yes (context-sensitive) |
| `pattern` | Pattern definition | Pattern matching | ✓ Yes (outside pattern context) |
| `push` | Add to list | List operations | ❌ No (also structural) |
| `read` | Read operation | File I/O | ✓ Yes (context-sensitive) |
| `reversed` | Reverse iteration | Loop direction | ✓ Yes |
| `skip` | Continue (loop control) | Loop operations | ❌ No (also structural) |
| `text` | Text type | Type context | ✓ Yes (outside type context) |
| `than` | Comparison | `greater than`, `less than` | ❌ No (also structural) |
| `zero` | Pattern quantifier | Pattern matching | ❌ No (also structural) |

**Count:** 29 contextual keywords total (24 can be used as variables, 5 cannot due to structural overlap)

**Note:** Keywords marked with ❌ appear in both structural and contextual lists. Since the parser checks structural keywords first, they **CANNOT** be used as variable names despite being contextual.

---

### Other Reserved Keywords (95)

All other reserved keywords that don't fall into structural or contextual-only categories. These keywords cannot be used as variable names.

#### File & I/O Operations
`append`, `appending`, `close`, `content`, `delete`, `directory`, `exists`, `file`, `found`, `open`, `permission`, `denied`, `recursively`, `write`

#### Arithmetic & Comparison
`add`, `above`, `below`, `divide`, `divided`, `equal`, `greater`, `is`, `less`, `minus`, `multiply`, `plus`, `same`, `subtract`, `times`

#### Pattern Matching
`ahead`, `behind`, `between`, `capture`, `captured`, `category`, `character`, `digit`, `exactly`, `find`, `greedy`, `lazy`, `letter`, `matches`, `more`, `of`, `one`, `optional`, `replace`, `script`, `split`, `start`, `unicode`, `whitespace`

#### Web & Network
`accepting`, `comes`, `connections`, `current`, `formatted`, `handler`, `header`, `listen`, `milliseconds`, `port`, `register`, `request`, `respond`, `response`, `server`, `signal`, `status`, `stop`, `timeout`

#### Process & Execution
`arguments`, `command`, `execute`, `into`, `kill`, `output`, `process`, `running`, `shell`, `spawn`, `using`

#### Data & Types
`data`, `database`, `date`, `error`, `remove`, `clear`, `time`, `url`

#### Miscellaneous
`comes`, `downward`, `exit`, `loop`, `parameters`, `upward`

**Count:** 95 other reserved keywords

**Note:** This count (95) represents keywords that are not in the structural (52) or contextual-only (24) categories, and are not boolean/null literals (7). Total: 52 + 24 + 95 + 7 = 178.

---

### Boolean & Null Literals (7)

Special literal values that are also reserved keywords.

| Literal | Type | Value | Notes |
|---------|------|-------|-------|
| `yes` | Boolean | `true` | Natural language true |
| `no` | Boolean | `false` | Natural language false |
| `true` | Boolean | `true` | Standard boolean true |
| `false` | Boolean | `false` | Standard boolean false |
| `nothing` | Null | `null` | Primary null value |
| `missing` | Null | `null` | Null alias |
| `undefined` | Null | `null` | Null alias |

**Count:** 7 literals

---

## Keywords by Category

### Control Flow (23 keywords)
`break`, `check`, `continue`, `each`, `end`, `exit`, `for`, `forever`, `from`, `if`, `in`, `loop`, `otherwise`, `repeat`, `reversed`, `skip`, `then`, `to`, `until`, `while`, `with`, `by`, `count`

### Declaration (18 keywords)
`action`, `as`, `called`, `change`, `constant`, `container`, `create`, `define`, `defaults`, `extends`, `implements`, `interface`, `list`, `map`, `needs`, `new`, `property`, `static`, `store`

### Operations (24 keywords)
`add`, `append`, `call`, `close`, `delete`, `display`, `divide`, `execute`, `give`, `back`, `kill`, `load`, `module`, `multiply`, `open`, `push`, `read`, `remove`, `clear`, `respond`, `return`, `spawn`, `subtract`, `wait`, `write`

### Comparisons (14 keywords)
`above`, `and`, `at`, `below`, `equal`, `greater`, `is`, `less`, `not`, `of`, `or`, `same`, `than`, `contains`

### Pattern Matching (28 keywords)
`ahead`, `any`, `behind`, `between`, `capture`, `captured`, `category`, `character`, `digit`, `exactly`, `find`, `greedy`, `lazy`, `letter`, `matches`, `more`, `one`, `optional`, `pattern`, `replace`, `script`, `split`, `start`, `text`, `unicode`, `whitespace`, `zero`, `of`

### File & I/O (15 keywords)
`append`, `appending`, `close`, `content`, `delete`, `directory`, `exists`, `extension`, `extensions`, `file`, `files`, `found`, `open`, `permission`, `denied`, `recursively`, `read`, `write`

### Web & Network (17 keywords)
`accepting`, `comes`, `connections`, `current`, `formatted`, `handler`, `header`, `listen`, `milliseconds`, `port`, `register`, `request`, `respond`, `response`, `server`, `signal`, `status`, `stop`, `timeout`

### Containers & OOP (17 keywords)
`container`, `constant`, `defaults`, `event`, `extends`, `implements`, `interface`, `must`, `new`, `on`, `parent`, `private`, `property`, `public`, `requires`, `static`, `trigger`

### Error Handling (5 keywords)
`catch`, `error`, `exists`, `try`, `when`

### Process & Execution (11 keywords)
`arguments`, `command`, `execute`, `into`, `kill`, `output`, `process`, `running`, `shell`, `spawn`, `using`

### Data & Types (7 keywords)
`data`, `database`, `date`, `time`, `url`, `list`, `map`, `text`

### Values (7 keywords)
`yes`, `no`, `true`, `false`, `nothing`, `missing`, `undefined`

---

## Usage Guidelines

### Cannot Use as Variable Names (Structural Keywords)

Structural keywords **CANNOT** be used as variable names in any context.

**Wrong:**
```wfl
store is as 10              // ERROR: 'is' is reserved
store file as "data.txt"    // ERROR: 'file' is reserved
store add as 5              // ERROR: 'add' is reserved
store for as 100            // ERROR: 'for' is reserved
store check as yes          // ERROR: 'check' is reserved
```

**Right:**
```wfl
store is_valid as 10
store filename as "data.txt"
store addition as 5
store loop_count as 100
store should_check as yes
```

---

### Safe to Use in Context (Contextual Keywords)

Contextual keywords **CAN** be used as variable names outside their keyword context.

**Examples:**

```wfl
# 'count' can be used as a variable outside count loops
store count as 0
change count to count plus 1
display count

# 'list' can be used as a variable outside type declarations
store list as "shopping_list"

# 'pattern' can be used outside pattern matching
store pattern as "design_pattern"

# 'create' can be used in expressions
store create as "create_mode"

# 'new' can be used outside container instantiation
store new as "new_value"
```

**When they're reserved:**

```wfl
# 'count' is reserved in count loops
count from 1 to 10:          # 'count' is a keyword here
    display count            # 'count' refers to loop variable
end

# 'list' is reserved in type context
create list called items     # 'list' is a type keyword here

# 'pattern' is reserved in pattern matching
define pattern called email  # 'pattern' is a keyword here
```

---

### Avoiding Keyword Conflicts

When you need to use a concept similar to a reserved keyword, use these strategies:

1. **Add underscores:**
   - `is` → `is_valid`, `is_active`, `is_ready`
   - `file` → `file_name`, `file_path`, `file_handle`
   - `data` → `user_data`, `input_data`, `raw_data`
   - `count` → `item_count`, `total_count`

2. **Use different words:**
   - `check` → `validate`, `verify`, `test`
   - `display` → `show`, `print`, `output`
   - `store` → `save`, `keep`, `hold`
   - `change` → `modify`, `update`, `alter`

3. **Add prefixes/suffixes:**
   - `current` → `current_value`, `the_current`
   - `process` → `my_process`, `process_id`
   - `event` → `user_event`, `event_name`

4. **Combine words:**
   - `file` → `filename`, `filepath`
   - `list` → `itemlist`, `datalist`
   - `text` → `message_text`, `display_text`

---

## Quick Reference Table

Complete alphabetical list of all 155 keywords:

| Keyword | Type | Category | Can Use as Variable? |
|---------|------|----------|---------------------|
| `accepting` | Other | Web/Network | ❌ No |
| `action` | Structural | Declaration | ❌ No |
| `add` | Other | Operations | ❌ No |
| `ahead` | Other | Pattern | ❌ No |
| `and` | Structural | Comparison | ❌ No |
| `any` | Contextual | Pattern | ✓ Yes |
| `append` | Other | File I/O | ❌ No |
| `appending` | Other | File I/O | ❌ No |
| `arguments` | Other | Process | ❌ No |
| `as` | Structural | Declaration | ❌ No |
| `at` | Contextual | Comparison | ✓ Yes |
| `above` | Other | Comparison | ❌ No |
| `back` | Contextual | Operations | ✓ Yes |
| `behind` | Other | Pattern | ❌ No |
| `below` | Other | Comparison | ❌ No |
| `between` | Other | Pattern | ❌ No |
| `break` | Structural | Control Flow | ❌ No |
| `by` | Structural | Control Flow | ❌ No |
| `called` | Contextual | Declaration | ✓ Yes |
| `call` | Structural | Operations | ❌ No |
| `capture` | Other | Pattern | ❌ No |
| `captured` | Other | Pattern | ❌ No |
| `catch` | Structural | Error Handling | ❌ No |
| `category` | Other | Pattern | ❌ No |
| `change` | Contextual | Declaration | ✓ Yes |
| `character` | Other | Pattern | ❌ No |
| `check` | Structural | Control Flow | ❌ No |
| `clear` | Other | Operations | ❌ No |
| `close` | Other | File I/O | ❌ No |
| `comes` | Other | Web/Network | ❌ No |
| `command` | Other | Process | ❌ No |
| `connections` | Other | Web/Network | ❌ No |
| `constant` | Structural | Declaration | ❌ No |
| `container` | Structural | OOP | ❌ No |
| `contains` | Contextual | Comparison | ✓ Yes (as function name) |
| `content` | Other | File I/O | ❌ No |
| `continue` | Structural | Control Flow | ❌ No |
| `count` | Contextual | Control Flow | ✓ Yes (outside count loops) |
| `create` | Contextual | Declaration | ✓ Yes (in expressions) |
| `current` | Other | Web/Network | ❌ No |
| `data` | Other | Data & Types | ❌ No |
| `database` | Other | Data & Types | ❌ No |
| `date` | Other | Data & Types | ❌ No |
| `defaults` | Contextual | Declaration | ✓ Yes |
| `define` | Structural | Declaration | ❌ No |
| `delete` | Other | File I/O | ❌ No |
| `denied` | Other | File I/O | ❌ No |
| `digit` | Other | Pattern | ❌ No |
| `directory` | Other | File I/O | ❌ No |
| `display` | Structural | Operations | ❌ No |
| `divide` | Other | Operations | ❌ No |
| `divided` | Other | Operations | ❌ No (multi-word: "divided by") |
| `each` | Structural | Control Flow | ❌ No |
| `end` | Structural | Control Flow | ❌ No |
| `equal` | Other | Comparison | ❌ No |
| `error` | Other | Error Handling | ❌ No |
| `event` | Structural | OOP | ❌ No |
| `exactly` | Other | Pattern | ❌ No |
| `execute` | Other | Process | ❌ No |
| `exists` | Other | Error Handling | ❌ No |
| `exit` | Structural | Control Flow | ❌ No |
| `extension` | Contextual | File I/O | ✓ Yes |
| `extensions` | Contextual | File I/O | ✓ Yes |
| `extends` | Structural | OOP | ❌ No |
| `false` | Literal | Values | ❌ No |
| `file` | Other | File I/O | ❌ No |
| `files` | Contextual | File I/O | ✓ Yes |
| `find` | Other | Pattern | ❌ No |
| `for` | Structural | Control Flow | ❌ No |
| `forever` | Structural | Control Flow | ❌ No |
| `formatted` | Other | Web/Network | ❌ No |
| `found` | Other | File I/O | ❌ No |
| `from` | Structural | Control Flow | ❌ No |
| `give` | Contextual | Operations | ✓ Yes |
| `greater` | Other | Comparison | ❌ No |
| `greedy` | Other | Pattern | ❌ No |
| `handler` | Other | Web/Network | ❌ No |
| `header` | Other | Web/Network | ❌ No |
| `if` | Structural | Control Flow | ❌ No |
| `implements` | Structural | OOP | ❌ No |
| `in` | Structural | Control Flow | ❌ No |
| `interface` | Structural | OOP | ❌ No |
| `into` | Other | Process | ❌ No |
| `is` | Other | Comparison | ❌ No |
| `kill` | Other | Process | ❌ No |
| `lazy` | Other | Pattern | ❌ No |
| `least` | Contextual | Comparison | ✓ Yes |
| `less` | Other | Comparison | ❌ No |
| `letter` | Other | Pattern | ❌ No |
| `list` | Contextual | Data & Types | ✓ Yes (outside type context) |
| `listen` | Other | Web/Network | ❌ No |
| `load` | Structural | Operations | ❌ No |
| `loop` | Structural | Control Flow | ❌ No |
| `map` | Contextual | Data & Types | ✓ Yes (outside type context) |
| `matches` | Other | Pattern | ❌ No |
| `milliseconds` | Other | Web/Network | ❌ No |
| `minus` | Other | Operations | ❌ No |
| `missing` | Literal | Values | ❌ No |
| `module` | Structural | Operations | ❌ No |
| `more` | Other | Pattern | ❌ No |
| `most` | Contextual | Comparison | ✓ Yes |
| `multiply` | Other | Operations | ❌ No |
| `must` | Contextual | OOP | ✓ Yes |
| `needs` | Contextual | Declaration | ✓ Yes |
| `new` | Contextual | Declaration | ✓ Yes (context-sensitive) |
| `no` | Literal | Values | ❌ No |
| `not` | Structural | Comparison | ❌ No |
| `nothing` | Literal | Values | ❌ No |
| `of` | Other | Comparison | ❌ No |
| `on` | Structural | OOP | ❌ No |
| `one` | Other | Pattern | ❌ No |
| `open` | Other | File I/O | ❌ No |
| `optional` | Other | Pattern | ❌ No |
| `or` | Structural | Comparison | ❌ No |
| `otherwise` | Structural | Control Flow | ❌ No |
| `output` | Other | Process | ❌ No |
| `parent` | Contextual | OOP | ✓ Yes (context-sensitive) |
| `pattern` | Contextual | Pattern | ✓ Yes (outside pattern context) |
| `permission` | Other | File I/O | ❌ No |
| `plus` | Other | Operations | ❌ No |
| `port` | Other | Web/Network | ❌ No |
| `private` | Structural | OOP | ❌ No |
| `process` | Other | Process | ❌ No |
| `property` | Structural | OOP | ❌ No |
| `public` | Structural | OOP | ❌ No |
| `push` | Contextual | Operations | ✓ Yes (context-sensitive) |
| `read` | Contextual | File I/O | ✓ Yes (context-sensitive) |
| `recursively` | Other | File I/O | ❌ No |
| `register` | Other | Web/Network | ❌ No |
| `remove` | Other | Operations | ❌ No |
| `repeat` | Structural | Control Flow | ❌ No |
| `replace` | Other | Pattern | ❌ No |
| `request` | Other | Web/Network | ❌ No |
| `requires` | Structural | OOP | ❌ No |
| `respond` | Other | Web/Network | ❌ No |
| `response` | Other | Web/Network | ❌ No |
| `return` | Structural | Operations | ❌ No |
| `reversed` | Contextual | Control Flow | ✓ Yes |
| `running` | Other | Process | ❌ No |
| `same` | Other | Comparison | ❌ No |
| `script` | Other | Pattern | ❌ No |
| `server` | Other | Web/Network | ❌ No |
| `shell` | Other | Process | ❌ No |
| `signal` | Other | Web/Network | ❌ No |
| `skip` | Structural | Control Flow | ❌ No |
| `spawn` | Other | Process | ❌ No |
| `split` | Other | Pattern | ❌ No |
| `start` | Other | Pattern | ❌ No |
| `static` | Structural | OOP | ❌ No |
| `status` | Other | Web/Network | ❌ No |
| `stop` | Other | Web/Network | ❌ No |
| `store` | Structural | Declaration | ❌ No |
| `subtract` | Other | Operations | ❌ No |
| `text` | Contextual | Data & Types | ✓ Yes (outside type context) |
| `than` | Contextual | Comparison | ✓ Yes |
| `then` | Structural | Control Flow | ❌ No |
| `time` | Other | Data & Types | ❌ No |
| `timeout` | Other | Web/Network | ❌ No |
| `times` | Other | Operations | ❌ No |
| `to` | Structural | Control Flow | ❌ No |
| `trigger` | Structural | OOP | ❌ No |
| `true` | Literal | Values | ❌ No |
| `try` | Structural | Error Handling | ❌ No |
| `undefined` | Literal | Values | ❌ No |
| `unicode` | Other | Pattern | ❌ No |
| `until` | Structural | Control Flow | ❌ No |
| `url` | Other | Data & Types | ❌ No |
| `using` | Other | Process | ❌ No |
| `wait` | Structural | Operations | ❌ No |
| `when` | Structural | Error Handling | ❌ No |
| `while` | Structural | Control Flow | ❌ No |
| `whitespace` | Other | Pattern | ❌ No |
| `with` | Structural | Operations | ❌ No |
| `write` | Other | File I/O | ❌ No |
| `yes` | Literal | Values | ❌ No |
| `zero` | Contextual | Pattern | ✓ Yes |

---

## Related Documentation

- [Keyword Reference (User Docs)](Docs/reference/keyword-reference.md) - User-friendly keyword reference
- [Naming Conventions](Docs/06-best-practices/naming-conventions.md) - Best practices for avoiding keyword conflicts
- [Variables and Types](Docs/03-language-basics/variables-and-types.md) - Variable declaration and reserved keywords
- [Token Source Code](src/lexer/token.rs) - Complete token definitions in the WFL compiler

---

## Notes

- **Multi-word keyword:** `divided by` (line 218 in token.rs) is a special two-word keyword
- Some keywords appear in both structural and contextual lists (like `push`, `zero`, `any`, `than`) - these are contextual because the structural check takes precedence
- Total breakdown: 46 structural + 27 contextual + 75 other + 7 literals = **155 keywords**
- Boolean literals (`yes`, `no`, `true`, `false`) are case-insensitive in the lexer
- Null literals have three aliases: `nothing` (primary), `missing`, `undefined`

---

*Generated from WFL Compiler Source Code (src/lexer/token.rs)*
*Last Updated: 2026-01-15*
