# Reserved Keywords (Complete Technical Reference)

Complete technical documentation for all WFL reserved keywords.

**→ For quick lookup:** [Keyword Reference (Quick) →](keyword-reference.md)

**Source:** WFL Compiler v26.1.x (extracted from `src/lexer/token.rs`)
**Total:** 178 keywords and literals
**Last Updated:** 2026-01-16

---

## Table of Contents

1. [Understanding Keyword Classifications](#understanding-keyword-classifications)
   - [What Makes a Keyword "Structural"?](#what-makes-a-keyword-structural)
   - [What Makes a Keyword "Contextual"?](#what-makes-a-keyword-contextual)
   - [Why Some Keywords Appear in Multiple Lists](#why-some-keywords-appear-in-multiple-lists)
2. [Structural Keywords (52)](#structural-keywords-52)
3. [Contextual Keywords (29)](#contextual-keywords-29)
4. [Other Reserved Keywords (95)](#other-reserved-keywords-95)
5. [Boolean & Null Literals (7)](#boolean--null-literals-7)
6. [Keywords by Feature Category](#keywords-by-feature-category)
7. [Usage Guidelines & Best Practices](#usage-guidelines--best-practices)
8. [Common Pitfalls](#common-pitfalls)
9. [Complete Alphabetical Reference](#complete-alphabetical-reference)

---

## Understanding Keyword Classifications

WFL's 178 reserved keywords are organized into four distinct types. Understanding these classifications helps you know which keywords you can never use as variable names, and which ones might be available in certain contexts.

### What Makes a Keyword "Structural"?

**Structural keywords** are the core building blocks of WFL programs. They define program structure, control flow, and fundamental operations. Think of them as the "grammar words" of WFL—just like you wouldn't use "if" or "the" as names in English writing, these words have special meaning in WFL.

**Key characteristics:**
- **Always reserved** - Cannot be used as variable names in any context
- **Parser priority** - Checked first by the parser
- **Total count:** 52 keywords

**Examples:**
```wfl
// These are structural - NEVER use as variables
check if x is 5:      // 'check', 'if', 'is' are structural
    display "Yes"     // 'display' is structural
end check            // 'end', 'check' are structural

store name as "Alice"  // 'store', 'as' are structural
```

**Why they're always reserved:** The parser relies on these keywords to understand the structure of your program. If you could use `if` as a variable name, the parser wouldn't know whether `if` means "conditional" or "your variable."

### What Makes a Keyword "Contextual"?

**Contextual keywords** have special meaning only in specific contexts. Outside those contexts, some (but not all) can be used as variable names.

**Key characteristics:**
- **Context-dependent meaning** - Special in certain situations
- **24 are usable as variables** - Outside their keyword context
- **5 overlap with structural** - Cannot be used as variables despite being contextual
- **Total count:** 29 keywords

**The 24 you CAN use as variables:**
```wfl
// Example 1: 'count' as a variable (outside count loops)
store count as 0              // ✅ 'count' is just a variable here
change count to count plus 1
display count

// But inside count loops, 'count' is reserved:
count from 1 to 10:          // ✅ 'count' is a keyword here
    display count            // ✅ 'count' refers to loop variable
end count

// Example 2: 'list' as a variable (outside type declarations)
store list as "my_list_name"  // ✅ 'list' is just a variable here
display list

// But in type context, 'list' is reserved:
create list called items     // ✅ 'list' is a type keyword here
```

**The 5 you CANNOT use as variables** (despite being contextual):
- `any` - Also appears as structural keyword
- `push` - Also appears as structural keyword
- `skip` - Also appears as structural keyword
- `than` - Also appears as structural keyword
- `zero` - Also appears as structural keyword

**Why the overlap?** The parser checks structural keywords first. Even though these keywords are listed as contextual in the source code, the structural check takes precedence, making them always reserved.

### Why Some Keywords Appear in Multiple Lists

You might notice keywords like `push`, `zero`, and `than` appear in both the structural and contextual keyword lists in the compiler source code. This isn't a bug—it's by design:

1. **Parser checks structural keywords first** - If a keyword is structural, it's always reserved
2. **Contextual list provides additional context** - Helps the parser handle specific situations
3. **Net result:** If a keyword is in BOTH lists, treat it as structural (always reserved)

**Example:**
```wfl
// 'push' appears in both lists, but structural wins
push with myList and item    // ✅ 'push' is a keyword here
store push as "save"         // ❌ ERROR: 'push' is reserved (structural)
```

---

## Structural Keywords (52)

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

**Total:** 52 structural keywords

**Important Note:** The keywords `any`, `push`, `skip`, `than`, and `zero` also appear in the contextual keyword list in the source code, but since structural keywords are checked first in the parser, they **CANNOT** be used as variable names.

---

## Contextual Keywords (29)

These keywords are **context-dependent**. Most **CAN** be used as variable names outside their keyword context, but 5 of them overlap with structural keywords and **CANNOT** be used as variables.

### The 24 You CAN Use as Variables

| Keyword | Description | Context | Can Use As Variable? | Example |
|---------|-------------|---------|----------------------|---------|
| `at` | Location preposition | File/position operations | ✅ Yes | `store at as 5` works outside file ops |
| `back` | Return keyword part | `give back` expression | ✅ Yes | `store back as "rear"` works |
| `called` | Action name | `define action called X` | ✅ Yes | `store called as "named"` works |
| `change` | Modify variable | Variable modification | ✅ Yes | `store change as "coins"` works in expressions |
| `contains` | Contains check | Collection/string operations | ✅ Yes | Can be function name |
| `count` | Count loop | `count from X to Y` | ✅ Yes | `store count as 0` works outside count loops |
| `create` | Create resource | Resource creation | ✅ Yes | `store create as "make"` works in expressions |
| `defaults` | Default value | Container properties | ✅ Yes | `store defaults as "standard"` works |
| `extension` | File extension | File operations | ✅ Yes | `store extension as ".txt"` works |
| `extensions` | File extensions | File operations | ✅ Yes | `store extensions as list` works |
| `files` | File collection | File operations | ✅ Yes | `store files as list` works |
| `give` | Return keyword part | `give back` expression | ✅ Yes | `store give as "donate"` works |
| `least` | Minimum amount | Quantifiers | ✅ Yes | `store least as 1` works |
| `list` | List type | Type/create context | ✅ Yes | `store list as "shopping"` works outside type context |
| `map` | Map type | Type/create context | ✅ Yes | `store map as "directions"` works outside type context |
| `most` | Maximum amount | Quantifiers | ✅ Yes | `store most as 100` works |
| `must` | Requirement | Interface/validation | ✅ Yes | `store must as "required"` works |
| `needs` | Action parameters | `action needs X` (alias for `with`) | ✅ Yes | `store needs as "requirements"` works |
| `new` | Instantiate container | Container creation | ✅ Yes | `store new as "fresh"` works (context-sensitive) |
| `parent` | Parent reference | Container inheritance | ✅ Yes | `store parent as "mom"` works (context-sensitive) |
| `pattern` | Pattern definition | Pattern matching | ✅ Yes | `store pattern as "design"` works outside pattern context |
| `read` | Read operation | File I/O | ✅ Yes | `store read as "reading"` works (context-sensitive) |
| `reversed` | Reverse iteration | Loop direction | ✅ Yes | `store reversed as yes` works |
| `text` | Text type | Type context | ✅ Yes | `store text as "message"` works outside type context |

**Count:** 24 contextual keywords that CAN be used as variables (in appropriate contexts)

### The 5 You CANNOT Use as Variables

These keywords appear in the contextual list but also appear as structural keywords. Since the parser checks structural keywords first, they are **always reserved**:

| Keyword | Why Reserved | Example Error |
|---------|-------------|---------------|
| `any` | Also structural (pattern matching) | `store any as "something"` → ERROR |
| `push` | Also structural (list operations) | `store push as "add"` → ERROR |
| `skip` | Also structural (loop control) | `store skip as "jump"` → ERROR |
| `than` | Also structural (comparisons) | `store than as "then"` → ERROR |
| `zero` | Also structural (pattern quantifier) | `store zero as 0` → ERROR |

**Count:** 5 contextual keywords that CANNOT be used as variables

**Total Contextual Keywords:** 29 (24 usable + 5 structural overlap)

---

## Other Reserved Keywords (95)

All other reserved keywords that don't fall into structural or contextual-only categories. These keywords cannot be used as variable names.

### File & I/O Operations (14)
`append`, `appending`, `close`, `content`, `delete`, `directory`, `exists`, `file`, `found`, `open`, `permission`, `denied`, `recursively`, `write`

### Arithmetic & Comparison (15)
`add`, `above`, `below`, `divide`, `divided`, `equal`, `greater`, `is`, `less`, `minus`, `multiply`, `plus`, `same`, `subtract`, `times`

### Pattern Matching (26)
`ahead`, `behind`, `between`, `capture`, `captured`, `category`, `character`, `digit`, `exactly`, `find`, `greedy`, `lazy`, `letter`, `matches`, `more`, `of`, `one`, `optional`, `replace`, `script`, `split`, `start`, `unicode`, `whitespace`

**Note:** Some pattern keywords like `any`, `zero`, `pattern`, `text` are counted in other categories (structural or contextual).

### Web & Network (17)
`accepting`, `comes`, `connections`, `current`, `formatted`, `handler`, `header`, `listen`, `milliseconds`, `port`, `register`, `request`, `respond`, `response`, `server`, `signal`, `status`, `stop`, `timeout`

### Process & Execution (11)
`arguments`, `command`, `execute`, `into`, `kill`, `output`, `process`, `running`, `shell`, `spawn`, `using`

### Data & Types (6)
`data`, `database`, `date`, `time`, `url`

**Note:** `list`, `map`, `text` are contextual keywords counted separately.

### Miscellaneous (6)
`comes`, `downward`, `exit`, `loop`, `parameters`, `upward`

**Count:** 95 other reserved keywords

**Note:** This count (95) represents keywords that are not in the structural (52) or contextual-only (24) categories, and are not boolean/null literals (7). Total: 52 + 24 + 95 + 7 = 178.

---

## Boolean & Null Literals (7)

Special literal values that are also reserved keywords.

| Literal | Type | Value | Usage | Can Use As Variable? |
|---------|------|-------|-------|----------------------|
| `yes` | Boolean | `true` | Natural language true | ❌ No |
| `no` | Boolean | `false` | Natural language false | ❌ No |
| `true` | Boolean | `true` | Standard boolean true | ❌ No |
| `false` | Boolean | `false` | Standard boolean false | ❌ No |
| `nothing` | Null | `null` | Primary null value | ❌ No |
| `missing` | Null | `null` | Null alias | ❌ No |
| `undefined` | Null | `null` | Null alias | ❌ No |

**Examples:**
```wfl
// Boolean literals
store is_active as yes        // Using 'yes' (true)
store is_ready as no          // Using 'no' (false)
store has_data as true        // Using 'true'
store is_valid as false       // Using 'false'

// Null literals
store result as nothing       // Using 'nothing' (null)
store value as missing        // Using 'missing' (null)
store data as undefined       // Using 'undefined' (null)

// All are reserved - cannot use as variable names
store yes as 10               // ❌ ERROR: 'yes' is reserved
store nothing as 0            // ❌ ERROR: 'nothing' is reserved
```

**Count:** 7 literals (4 boolean + 3 null)

---

## Keywords by Feature Category

Keywords organized by functional area for easier reference.

### Control Flow (23 keywords)
`break`, `check`, `continue`, `count`, `downward`, `each`, `end`, `exit`, `for`, `forever`, `from`, `if`, `in`, `loop`, `otherwise`, `repeat`, `reversed`, `skip`, `then`, `to`, `until`, `while`, `by`

### Declaration (18 keywords)
`action`, `as`, `called`, `change`, `constant`, `container`, `create`, `defaults`, `define`, `extends`, `implements`, `interface`, `list`, `map`, `needs`, `new`, `property`, `static`, `store`

### Operations (24 keywords)
`add`, `append`, `back`, `call`, `clear`, `close`, `display`, `divide`, `divided`, `execute`, `give`, `kill`, `load`, `minus`, `module`, `multiply`, `plus`, `pop`, `push`, `remove`, `return`, `subtract`, `times`, `wait`, `write`

### Comparisons (16 keywords)
`above`, `and`, `at`, `below`, `contains`, `equal`, `greater`, `is`, `least`, `less`, `most`, `not`, `of`, `or`, `same`, `than`, `with`

### Pattern Matching (28 keywords)
`ahead`, `any`, `behind`, `between`, `capture`, `captured`, `category`, `character`, `digit`, `exactly`, `find`, `greedy`, `lazy`, `letter`, `matches`, `more`, `one`, `optional`, `pattern`, `replace`, `script`, `split`, `start`, `text`, `unicode`, `whitespace`, `zero`

### File & I/O (18 keywords)
`append`, `appending`, `close`, `content`, `delete`, `directory`, `exists`, `extension`, `extensions`, `file`, `files`, `found`, `open`, `permission`, `denied`, `read`, `recursively`, `write`

### Web & Network (20 keywords)
`accepting`, `comes`, `connections`, `current`, `formatted`, `handler`, `header`, `listen`, `milliseconds`, `on`, `port`, `register`, `request`, `respond`, `response`, `server`, `signal`, `status`, `stop`, `timeout`

### Containers & OOP (17 keywords)
`constant`, `container`, `defaults`, `event`, `extends`, `implements`, `interface`, `must`, `new`, `on`, `parent`, `private`, `property`, `public`, `requires`, `static`, `trigger`

### Error Handling (4 keywords)
`catch`, `error`, `try`, `when`

### Process & Execution (11 keywords)
`arguments`, `command`, `execute`, `into`, `kill`, `output`, `process`, `running`, `shell`, `spawn`, `using`

### Data & Types (8 keywords)
`data`, `database`, `date`, `list`, `map`, `text`, `time`, `url`

### Values (7 keywords)
`yes`, `no`, `true`, `false`, `nothing`, `missing`, `undefined`

---

## Usage Guidelines & Best Practices

### Rule 1: Structural Keywords Are Always Reserved

Never use the 52 structural keywords as variable names, function names, or any other identifier.

**Wrong:**
```wfl
store is as 10              // ❌ ERROR: 'is' is reserved
store file as "data.txt"    // ❌ ERROR: 'file' is reserved
store add as 5              // ❌ ERROR: 'add' is reserved
store for as 100            // ❌ ERROR: 'for' is reserved
store check as yes          // ❌ ERROR: 'check' is reserved
```

**Right:**
```wfl
store is_valid as 10        // ✅ Added suffix
store filename as "data.txt"  // ✅ Different name
store addition as 5         // ✅ Different name
store loop_count as 100     // ✅ Descriptive name
store should_check as yes   // ✅ Descriptive name
```

### Rule 2: Check Context for Contextual Keywords

The 24 contextual keywords CAN be used as variables, but only outside their keyword context.

**Example: `count` contextual usage**
```wfl
// ✅ Outside count loops, 'count' is fine as a variable
store count as 0
change count to count plus 1
display count

// ❌ Inside count loops, 'count' is the loop variable
count from 1 to 10:
    display count      // This is the loop count, not your variable
end count
```

**Example: `list` contextual usage**
```wfl
// ✅ Outside type context, 'list' is fine as a variable
store list as "shopping_list"
display list

// ❌ In type context, 'list' is reserved
create list called items   // 'list' is a type keyword here
```

### Rule 3: Use Underscore Convention

When you want to use a concept similar to a keyword, add an underscore or suffix.

**Common patterns:**
| Keyword | Alternatives |
|---------|-------------|
| `is` | `is_valid`, `is_active`, `is_ready` |
| `file` | `filename`, `file_path`, `file_handle` |
| `data` | `user_data`, `input_data`, `raw_data` |
| `current` | `current_value`, `current_item` |
| `count` | `item_count`, `total_count` (when in count loop context) |
| `add` | `addition`, `add_result`, `add_value` |

### Rule 4: Prefer Clarity Over Brevity

Don't sacrifice clarity to avoid keywords. Use descriptive names.

**Wrong approach:**
```wfl
store x as "data.txt"       // What is 'x'?
store temp as 100          // Temp what?
```

**Right approach:**
```wfl
store filename as "data.txt"  // Clear purpose
store max_users as 100       // Descriptive
```

---

## Common Pitfalls

### Pitfall 1: Using 'count' Inside Count Loops

**Problem:** Trying to use a variable named `count` inside a count loop.

**Example:**
```wfl
store count as 0   // Your variable

count from 1 to 10:
    change count to count plus 1  // ❌ ERROR: 'count' is loop variable here
end count
```

**Solution:** Use a different name or track separately.
```wfl
store total_count as 0

count from 1 to 10:
    change total_count to total_count plus 1
    display count  // Loop variable (1-10)
    display total_count  // Your variable
end count
```

### Pitfall 2: Reserved Keywords in Natural Phrasing

**Problem:** WFL's natural syntax makes it easy to accidentally use keywords.

**Example:**
```wfl
// Trying to name things naturally
store file as "data.txt"    // ❌ 'file' is reserved
store is as yes             // ❌ 'is' is reserved
store check as "verify"     // ❌ 'check' is reserved
```

**Solution:** Add context to your variable names.
```wfl
store target_file as "data.txt"
store is_valid as yes
store check_type as "verify"
```

### Pitfall 3: Multi-word Keywords

**Problem:** WFL has one multi-word keyword: `divided by`

**Example:**
```wfl
// This might look confusing:
store result as 10 divided by 2  // 'divided by' is a single keyword

// Not the same as:
store result as 10 divide 2      // ❌ ERROR: 'divide' alone isn't valid here
```

**Solution:** Understand multi-word keywords are treated as single units.

### Pitfall 4: Assuming Contextual = Always Usable

**Problem:** Not all contextual keywords are usable as variables.

**The 5 exceptions:**
```wfl
store any as "something"    // ❌ ERROR: 'any' is also structural
store push as "add"         // ❌ ERROR: 'push' is also structural
store skip as "jump"        // ❌ ERROR: 'skip' is also structural
store than as "then"        // ❌ ERROR: 'than' is also structural
store zero as 0             // ❌ ERROR: 'zero' is also structural
```

**Solution:** Check both the contextual and structural keyword lists.

---

## Complete Alphabetical Reference

Complete reference table of all 178 keywords.

| Keyword | Type | Category | As Var? | Example |
|---------|------|----------|---------|---------|
| `accepting` | Other | Web/Network | ❌ | `accepting connections` |
| `action` | Structural | Declaration | ❌ | `define action called name:` |
| `add` | Other | Operations | ❌ | `add 5 to total` |
| `ahead` | Other | Pattern | ❌ | `ahead of pattern` |
| `and` | Structural | Comparison | ❌ | `x is 5 and y is 10` |
| `any` | Structural | Pattern | ❌ | `zero or more any` |
| `append` | Other | File I/O | ❌ | `append to file` |
| `appending` | Other | File I/O | ❌ | `appending mode` |
| `arguments` | Other | Process | ❌ | `with arguments` |
| `as` | Structural | Declaration | ❌ | `store x as 10` |
| `at` | Contextual | Comparison | ✅ | `at position 5` |
| `above` | Other | Comparison | ❌ | `above threshold` |
| `back` | Contextual | Operations | ✅ | `give back result` |
| `behind` | Other | Pattern | ❌ | `behind pattern` |
| `below` | Other | Comparison | ❌ | `below limit` |
| `between` | Other | Pattern | ❌ | `between values` |
| `break` | Structural | Control Flow | ❌ | `break loop` |
| `by` | Structural | Control Flow | ❌ | `count by 2` |
| `called` | Contextual | Declaration | ✅ | `action called name` |
| `call` | Structural | Operations | ❌ | `call function` |
| `capture` | Other | Pattern | ❌ | `capture group` |
| `captured` | Other | Pattern | ❌ | `captured text` |
| `catch` | Structural | Error Handling | ❌ | `catch error` |
| `category` | Other | Pattern | ❌ | `unicode category` |
| `change` | Contextual | Declaration | ✅ | `change value` |
| `character` | Other | Pattern | ❌ | `character class` |
| `check` | Structural | Control Flow | ❌ | `check if condition` |
| `clear` | Other | Operations | ❌ | `clear data` |
| `close` | Other | File I/O | ❌ | `close file` |
| `comes` | Other | Web/Network | ❌ | `request comes in` |
| `command` | Other | Process | ❌ | `execute command` |
| `connections` | Other | Web/Network | ❌ | `network connections` |
| `constant` | Structural | Declaration | ❌ | `constant property` |
| `container` | Structural | OOP | ❌ | `define container` |
| `contains` | Contextual | Comparison | ✅ | `list contains item` |
| `content` | Other | File I/O | ❌ | `file content` |
| `continue` | Structural | Control Flow | ❌ | `continue loop` |
| `count` | Contextual | Control Flow | ✅ | `count from 1 to 10` |
| `create` | Contextual | Declaration | ✅ | `create resource` |
| `current` | Other | Web/Network | ❌ | `current time` |
| `data` | Other | Data & Types | ❌ | `process data` |
| `database` | Other | Data & Types | ❌ | `connect database` |
| `date` | Other | Data & Types | ❌ | `current date` |
| `defaults` | Contextual | Declaration | ✅ | `property defaults` |
| `define` | Structural | Declaration | ❌ | `define action` |
| `delete` | Other | File I/O | ❌ | `delete file` |
| `denied` | Other | File I/O | ❌ | `permission denied` |
| `digit` | Other | Pattern | ❌ | `digit class` |
| `directory` | Other | File I/O | ❌ | `create directory` |
| `display` | Structural | Operations | ❌ | `display message` |
| `divide` | Other | Operations | ❌ | `divide by 2` |
| `divided` | Other | Operations | ❌ | `divided by` (multi-word) |
| `downward` | Other | Control Flow | ❌ | `count downward` |
| `each` | Structural | Control Flow | ❌ | `for each item` |
| `end` | Structural | Control Flow | ❌ | `end block` |
| `equal` | Other | Comparison | ❌ | `is equal to` |
| `error` | Other | Error Handling | ❌ | `handle error` |
| `event` | Structural | OOP | ❌ | `define event` |
| `exactly` | Other | Pattern | ❌ | `exactly 5 times` |
| `execute` | Other | Process | ❌ | `execute command` |
| `exists` | Other | File I/O | ❌ | `file exists` |
| `exit` | Other | Control Flow | ❌ | `exit program` |
| `extension` | Contextual | File I/O | ✅ | `file extension` |
| `extensions` | Contextual | File I/O | ✅ | `file extensions` |
| `extends` | Structural | OOP | ❌ | `container extends` |
| `false` | Literal | Values | ❌ | Boolean false |
| `file` | Other | File I/O | ❌ | `open file` |
| `files` | Contextual | File I/O | ✅ | `list files` |
| `find` | Other | Pattern | ❌ | `find pattern` |
| `for` | Structural | Control Flow | ❌ | `for each` |
| `forever` | Structural | Control Flow | ❌ | `repeat forever` |
| `formatted` | Other | Web/Network | ❌ | `formatted response` |
| `found` | Other | File I/O | ❌ | `file found` |
| `from` | Structural | Control Flow | ❌ | `count from 1` |
| `give` | Contextual | Operations | ✅ | `give back` |
| `greater` | Other | Comparison | ❌ | `greater than` |
| `greedy` | Other | Pattern | ❌ | `greedy match` |
| `handler` | Other | Web/Network | ❌ | `request handler` |
| `header` | Other | Web/Network | ❌ | `HTTP header` |
| `if` | Structural | Control Flow | ❌ | `check if` |
| `implements` | Structural | OOP | ❌ | `implements interface` |
| `in` | Structural | Control Flow | ❌ | `for each in` |
| `interface` | Structural | OOP | ❌ | `define interface` |
| `into` | Other | Process | ❌ | `output into` |
| `is` | Other | Comparison | ❌ | `x is 5` |
| `kill` | Other | Process | ❌ | `kill process` |
| `lazy` | Other | Pattern | ❌ | `lazy match` |
| `least` | Contextual | Comparison | ✅ | `at least 5` |
| `less` | Other | Comparison | ❌ | `less than` |
| `letter` | Other | Pattern | ❌ | `letter class` |
| `list` | Contextual | Data & Types | ✅ | `create list` |
| `listen` | Other | Web/Network | ❌ | `listen on port` |
| `load` | Structural | Operations | ❌ | `load module` |
| `loop` | Other | Control Flow | ❌ | `loop reference` |
| `map` | Contextual | Data & Types | ✅ | `create map` |
| `matches` | Other | Pattern | ❌ | `pattern matches` |
| `milliseconds` | Other | Web/Network | ❌ | `timeout milliseconds` |
| `minus` | Other | Operations | ❌ | `minus 5` |
| `missing` | Literal | Values | ❌ | Null alias |
| `module` | Structural | Operations | ❌ | `load module` |
| `more` | Other | Pattern | ❌ | `one or more` |
| `most` | Contextual | Comparison | ✅ | `at most 10` |
| `multiply` | Other | Operations | ❌ | `multiply by 2` |
| `must` | Contextual | OOP | ✅ | `must implement` |
| `needs` | Contextual | Declaration | ✅ | `action needs` |
| `new` | Contextual | Declaration | ✅ | `new instance` |
| `no` | Literal | Values | ❌ | Boolean false |
| `not` | Structural | Comparison | ❌ | `not equal` |
| `nothing` | Literal | Values | ❌ | Null value |
| `of` | Other | Comparison | ❌ | `typeof of` |
| `on` | Structural | OOP | ❌ | `on event` |
| `one` | Other | Pattern | ❌ | `one or more` |
| `open` | Other | File I/O | ❌ | `open file` |
| `optional` | Other | Pattern | ❌ | `optional match` |
| `or` | Structural | Comparison | ❌ | `x or y` |
| `otherwise` | Structural | Control Flow | ❌ | `otherwise:` |
| `output` | Other | Process | ❌ | `process output` |
| `parent` | Contextual | OOP | ✅ | `parent reference` |
| `pattern` | Contextual | Pattern | ✅ | `define pattern` |
| `permission` | Other | File I/O | ❌ | `file permission` |
| `plus` | Other | Operations | ❌ | `plus 5` |
| `pop` | Other | Operations | ❌ | `pop from list` |
| `port` | Other | Web/Network | ❌ | `listen on port` |
| `private` | Structural | OOP | ❌ | `private property` |
| `process` | Other | Process | ❌ | `spawn process` |
| `property` | Structural | OOP | ❌ | `container property` |
| `public` | Structural | OOP | ❌ | `public property` |
| `push` | Structural | Operations | ❌ | `push to list` |
| `read` | Contextual | File I/O | ✅ | `read file` |
| `recursively` | Other | File I/O | ❌ | `delete recursively` |
| `register` | Other | Web/Network | ❌ | `register handler` |
| `remove` | Other | Operations | ❌ | `remove item` |
| `repeat` | Structural | Control Flow | ❌ | `repeat 10 times` |
| `replace` | Other | Pattern | ❌ | `replace pattern` |
| `request` | Other | Web/Network | ❌ | `HTTP request` |
| `requires` | Structural | OOP | ❌ | `requires method` |
| `respond` | Other | Web/Network | ❌ | `respond to request` |
| `response` | Other | Web/Network | ❌ | `HTTP response` |
| `return` | Structural | Operations | ❌ | `return value` |
| `reversed` | Contextual | Control Flow | ✅ | `count reversed` |
| `running` | Other | Process | ❌ | `process running` |
| `same` | Other | Comparison | ❌ | `is same as` |
| `script` | Other | Pattern | ❌ | `unicode script` |
| `server` | Other | Web/Network | ❌ | `web server` |
| `shell` | Other | Process | ❌ | `shell command` |
| `signal` | Other | Web/Network | ❌ | `server signal` |
| `skip` | Structural | Control Flow | ❌ | `skip iteration` |
| `spawn` | Other | Process | ❌ | `spawn process` |
| `split` | Other | Pattern | ❌ | `split by pattern` |
| `start` | Other | Pattern | ❌ | `start anchor` |
| `static` | Structural | OOP | ❌ | `static property` |
| `status` | Other | Web/Network | ❌ | `response status` |
| `stop` | Other | Web/Network | ❌ | `stop server` |
| `store` | Structural | Declaration | ❌ | `store variable` |
| `subtract` | Other | Operations | ❌ | `subtract 5` |
| `text` | Contextual | Data & Types | ✅ | `text type` |
| `than` | Structural | Comparison | ❌ | `greater than` |
| `then` | Structural | Control Flow | ❌ | `if condition then` |
| `time` | Other | Data & Types | ❌ | `current time` |
| `timeout` | Other | Web/Network | ❌ | `request timeout` |
| `times` | Other | Operations | ❌ | `repeat 5 times` |
| `to` | Structural | Control Flow | ❌ | `count to 10` |
| `trigger` | Structural | OOP | ❌ | `trigger event` |
| `true` | Literal | Values | ❌ | Boolean true |
| `try` | Structural | Error Handling | ❌ | `try block` |
| `undefined` | Literal | Values | ❌ | Null alias |
| `unicode` | Other | Pattern | ❌ | `unicode support` |
| `until` | Structural | Control Flow | ❌ | `repeat until` |
| `upward` | Other | Control Flow | ❌ | `count upward` |
| `url` | Other | Data & Types | ❌ | `URL type` |
| `using` | Other | Process | ❌ | `using shell` |
| `wait` | Structural | Operations | ❌ | `wait for result` |
| `when` | Structural | Error Handling | ❌ | `catch when` |
| `while` | Structural | Control Flow | ❌ | `repeat while` |
| `whitespace` | Other | Pattern | ❌ | `whitespace class` |
| `with` | Structural | Operations | ❌ | `call with args` |
| `write` | Other | File I/O | ❌ | `write to file` |
| `yes` | Literal | Values | ❌ | Boolean true |
| `zero` | Structural | Pattern | ❌ | `zero or more` |

**Total: 178 keywords and literals**

---

## Related Documentation

- **[Keyword Reference (Quick) →](keyword-reference.md)** - Fast scannable lookup
- **[Variables and Types →](../03-language-basics/variables-and-types.md)** - Variable naming and types
- **[Naming Conventions →](../06-best-practices/naming-conventions.md)** - Best practices for naming
- **[Syntax Reference →](syntax-reference.md)** - Complete syntax guide
- **[Language Specification →](language-specification.md)** - Formal language specification
- **[Error Codes →](error-codes.md)** - Understanding keyword errors

---

**Previous:** [← Keyword Reference (Quick)](keyword-reference.md) | **Next:** [Operator Reference →](operator-reference.md)
