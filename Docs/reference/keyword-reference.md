# Keyword Reference (Quick)

Quick lookup for all WFL reserved keywords.

**→ For complete technical details:** [Reserved Keywords (Complete) →](reserved-keywords.md)

**Total:** 178 keywords and literals

---

## How to Use This Reference

- **✗** = Cannot use as variable name (reserved in all contexts)
- **✓** = Can use as variable name in certain contexts (see comprehensive reference for details)

**Tip:** When in doubt, add an underscore or use a different word. Example: `file` → `filename`, `is` → `is_valid`

---

## Control Flow Keywords (23)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `break` | Exit loop early | ✗ |
| `by` | Step amount in count loops | ✗ |
| `check` | Start conditional | ✗ |
| `continue` | Skip to next iteration | ✗ |
| `count` | Count loop keyword | ✓ |
| `downward` | Count loop direction | ✗ |
| `each` | For each loop | ✗ |
| `end` | Close block | ✗ |
| `exit` | Exit program/loop | ✗ |
| `for` | For loop | ✗ |
| `forever` | Infinite loop | ✗ |
| `from` | Count loop start | ✗ |
| `if` | Conditional | ✗ |
| `in` | For each collection | ✗ |
| `loop` | Loop reference | ✗ |
| `otherwise` | Else clause | ✗ |
| `repeat` | Loop construct | ✗ |
| `reversed` | Reverse iteration | ✓ |
| `skip` | Continue (alias) | ✗ |
| `then` | Conditional separator | ✗ |
| `to` | Count loop end | ✗ |
| `until` | Loop condition | ✗ |
| `while` | Loop condition | ✗ |

---

## Declaration Keywords (18)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `action` | Define function | ✗ |
| `as` | Binding/assignment | ✗ |
| `called` | Action name | ✓ |
| `change` | Modify variable | ✓ |
| `constant` | Define constant property | ✗ |
| `container` | Define class | ✗ |
| `create` | Create resource | ✓ |
| `defaults` | Default value | ✓ |
| `define` | Define action/container | ✗ |
| `extends` | Inheritance | ✗ |
| `implements` | Interface | ✗ |
| `interface` | Interface definition | ✗ |
| `list` | List type | ✓ |
| `map` | Map type | ✓ |
| `needs` | Action parameters (alias) | ✓ |
| `new` | Instantiate container | ✓ |
| `property` | Container field | ✗ |
| `store` | Create variable | ✗ |

---

## Operations Keywords (24)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `add` | Add to number/list | ✗ |
| `append` | Append to file | ✗ |
| `back` | Return keyword part (`give back`) | ✓ |
| `call` | Call action | ✗ |
| `clear` | Clear data | ✗ |
| `close` | Close resource | ✗ |
| `display` | Output text | ✗ |
| `divide` | Division | ✗ |
| `divided` | Division (multi-word: "divided by") | ✗ |
| `execute` | Run command | ✗ |
| `give` | Return keyword part | ✓ |
| `kill` | Terminate process | ✗ |
| `load` | Load module | ✗ |
| `minus` | Subtraction | ✗ |
| `module` | Module reference | ✗ |
| `multiply` | Multiplication | ✗ |
| `plus` | Addition | ✗ |
| `pop` | Remove from list | ✗ |
| `push` | Add to list | ✗ |
| `remove` | Remove item | ✗ |
| `return` | Return value | ✗ |
| `subtract` | Subtraction | ✗ |
| `times` | Multiplication | ✗ |
| `wait` | Async wait | ✗ |

---

## Comparison Keywords (14)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `above` | Greater than (alias) | ✗ |
| `and` | Logical AND | ✗ |
| `at` | Location preposition | ✓ |
| `below` | Less than (alias) | ✗ |
| `contains` | Contains check | ✓ |
| `equal` | Equality | ✗ |
| `greater` | Greater comparison | ✗ |
| `is` | Comparison/equality | ✗ |
| `least` | Minimum amount | ✓ |
| `less` | Less comparison | ✗ |
| `most` | Maximum amount | ✓ |
| `not` | Logical NOT | ✗ |
| `of` | Possession/function call | ✗ |
| `or` | Logical OR | ✗ |
| `same` | Equality (alias) | ✗ |
| `than` | Comparison | ✗ |
| `with` | Parameter separator/concatenation | ✗ |

---

## Pattern Matching Keywords (28)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `ahead` | Lookahead assertion | ✗ |
| `any` | Match any character | ✗ |
| `behind` | Lookbehind assertion | ✗ |
| `between` | Range specification | ✗ |
| `capture` | Capture group | ✗ |
| `captured` | Captured text reference | ✗ |
| `category` | Unicode category | ✗ |
| `character` | Character class | ✗ |
| `digit` | Digit class | ✗ |
| `exactly` | Exact quantifier | ✗ |
| `find` | Find pattern | ✗ |
| `greedy` | Greedy matching | ✗ |
| `lazy` | Lazy matching | ✗ |
| `letter` | Letter class | ✗ |
| `matches` | Pattern match check | ✗ |
| `more` | Quantifier part | ✗ |
| `one` | Quantifier (one or more) | ✗ |
| `optional` | Optional quantifier | ✗ |
| `pattern` | Pattern definition | ✓ |
| `replace` | Pattern replacement | ✗ |
| `script` | Unicode script | ✗ |
| `split` | Split by pattern | ✗ |
| `start` | Start anchor | ✗ |
| `text` | Text type | ✓ |
| `unicode` | Unicode support | ✗ |
| `whitespace` | Whitespace class | ✗ |
| `zero` | Zero quantifier | ✗ |

---

## File & I/O Keywords (16)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `append` | Append to file | ✗ |
| `appending` | Append mode | ✗ |
| `close` | Close file | ✗ |
| `content` | File content | ✗ |
| `delete` | Delete file | ✗ |
| `directory` | Directory resource | ✗ |
| `exists` | File exists check | ✗ |
| `extension` | File extension | ✓ |
| `extensions` | File extensions | ✓ |
| `file` | File resource | ✗ |
| `files` | File collection | ✓ |
| `found` | File found check | ✗ |
| `open` | Open file | ✗ |
| `permission` | File permission | ✗ |
| `denied` | Permission denied | ✗ |
| `read` | Read operation | ✓ |
| `recursively` | Recursive operation | ✗ |
| `write` | Write operation | ✗ |

---

## Web & Network Keywords (17)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `accepting` | Accepting connections | ✗ |
| `comes` | Request comes in | ✗ |
| `connections` | Network connections | ✗ |
| `current` | Current value/context | ✗ |
| `formatted` | Formatted response | ✗ |
| `handler` | Request handler | ✗ |
| `header` | HTTP header | ✗ |
| `listen` | Start server | ✗ |
| `milliseconds` | Time unit | ✗ |
| `on` | Event handler | ✗ |
| `port` | Network port | ✗ |
| `register` | Register handler | ✗ |
| `request` | Web request | ✗ |
| `respond` | Send response | ✗ |
| `response` | Web response | ✗ |
| `server` | Web server | ✗ |
| `signal` | Server signal | ✗ |
| `status` | Response status | ✗ |
| `stop` | Stop server | ✗ |
| `timeout` | Timeout duration | ✗ |

---

## Containers & OOP Keywords (17)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `constant` | Constant property | ✗ |
| `container` | Class definition | ✗ |
| `defaults` | Default value | ✓ |
| `event` | Event definition | ✗ |
| `extends` | Inheritance | ✗ |
| `implements` | Interface implementation | ✗ |
| `interface` | Interface definition | ✗ |
| `must` | Interface requirement | ✓ |
| `new` | Create instance | ✓ |
| `on` | Event handler | ✗ |
| `parent` | Parent reference | ✓ |
| `private` | Private visibility | ✗ |
| `property` | Container property | ✗ |
| `public` | Public visibility | ✗ |
| `requires` | Interface requirement | ✗ |
| `static` | Static member | ✗ |
| `trigger` | Fire event | ✗ |

---

## Error Handling Keywords (5)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `catch` | Error handler | ✗ |
| `error` | Error reference | ✗ |
| `try` | Error handling block | ✗ |
| `when` | Specific error type | ✗ |

---

## Process & Execution Keywords (11)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `arguments` | Command arguments | ✗ |
| `command` | System command | ✗ |
| `execute` | Run command | ✗ |
| `into` | Output redirection | ✗ |
| `kill` | Terminate process | ✗ |
| `output` | Process output | ✗ |
| `process` | Process resource | ✗ |
| `running` | Running state | ✗ |
| `shell` | Shell execution | ✗ |
| `spawn` | Start process | ✗ |
| `using` | Using clause | ✗ |

---

## Data & Types Keywords (7)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `data` | Data reference | ✗ |
| `database` | Database resource | ✗ |
| `date` | Date type | ✗ |
| `list` | List type | ✓ |
| `map` | Map type | ✓ |
| `text` | Text type | ✓ |
| `time` | Time type | ✗ |
| `url` | URL type | ✗ |

---

## Boolean & Null Literals (7)

| Keyword | Type | Value | As Var? |
|---------|------|-------|---------|
| `yes` | Boolean | `true` | ✗ |
| `no` | Boolean | `false` | ✗ |
| `true` | Boolean | `true` | ✗ |
| `false` | Boolean | `false` | ✗ |
| `nothing` | Null | `null` | ✗ |
| `missing` | Null | `null` | ✗ |
| `undefined` | Null | `null` | ✗ |

---

## Quick Usage Guide

### Cannot Use as Variables (Most Keywords)

**Wrong:**
```wfl
store is as 10             // ❌ ERROR: 'is' is reserved
store file as "data.txt"   // ❌ ERROR: 'file' is reserved
store add as 5             // ❌ ERROR: 'add' is reserved
```

**Right:**
```wfl
store is_valid as 10       // ✅ Use underscore
store filename as "data.txt"  // ✅ Different name
store addition as 5        // ✅ Different name
```

### Can Use in Certain Contexts (Contextual Keywords)

**Example: `count` as a variable**
```wfl
// ✅ Outside count loops, 'count' can be a variable
store count as 0
change count to count plus 1
display count

// ❌ Inside count loops, 'count' is reserved
count from 1 to 10:
    display count  // 'count' is the loop variable here
end count
```

**Example: `list` as a variable**
```wfl
// ✅ Outside type declarations, 'list' can be a variable
store list as "shopping_list"

// ❌ In type context, 'list' is reserved
create list called items
```

---

## Common Scenarios

### "Is X a keyword?" → Find it in the tables above
- ✗ means never use as variable
- ✓ means context-dependent (see comprehensive reference)

### "Getting 'Expected identifier' error?" → Check if you used a reserved keyword
- Add underscore: `is` → `is_valid`
- Use different word: `file` → `filename`

### "Can I use this contextual keyword?" → See the Comprehensive Reference
- 24 contextual keywords CAN be used as variables in certain contexts
- 5 appear contextual but are actually always reserved

---

## Related Documentation

- **[Reserved Keywords (Complete) →](reserved-keywords.md)** - Complete technical reference with classifications
- **[Variables and Types →](../03-language-basics/variables-and-types.md)** - Variable naming rules
- **[Naming Conventions →](../06-best-practices/naming-conventions.md)** - Best practices for naming
- **[Syntax Reference →](syntax-reference.md)** - Quick syntax lookup
- **[Error Codes →](error-codes.md)** - Understanding keyword errors

---

**Previous:** [← Language Specification](language-specification.md) | **Next:** [Reserved Keywords (Complete) →](reserved-keywords.md)
