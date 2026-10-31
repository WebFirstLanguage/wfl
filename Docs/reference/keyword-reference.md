# Keyword Reference (Quick)

Quick lookup for all WFL reserved keywords.

**→ For complete technical details:** [Reserved Keywords (Complete) →](reserved-keywords.md)

**Total:** 198 keywords and literals (63 structural, 32 contextual, 96 other, 7 literals)

---

## How to Use This Reference

- **✗** = Cannot use as variable name (reserved in all contexts)
- **✓** = Can use as variable name: `store <word> as value` works outside the word's keyword context (see comprehensive reference for details)

**Tip:** When in doubt, add an underscore or use a different word. Example: `file` → `filename`, `is` → `is_valid`

A keyword can appear in more than one table below; it has the same mark everywhere.
`tests/suite/keyword_reference_drift_test.rs` checks that every lexer keyword is
listed here with the right mark, and that each heading's count matches its table.

**New to reserved words?** Read [What Is a Reserved Word?](reserved-keywords.md#what-is-a-reserved-word-start-here)
first. It explains what "reserved" means and why WFL reserves words at all.

---

## Words That Commonly Bite

These everyday words are reserved. None of them can be a variable, loop
variable, action parameter, or action name, and they also break multi-word
names such as `start time`. Full explanations:
[Words That Commonly Bite](reserved-keywords.md#words-that-commonly-bite).

| Reserved word | What WFL uses it for | Use instead |
|---|---|---|
| `start` | Streaming web response; `start of text` pattern anchor | `start_time`, `start_index` |
| `output` | `read output from process` | `command_output`, `process_output` |
| `route` | `route ... end route` dispatch | `current_route`, `request_path` |
| `captured` | `same as captured "name"` in patterns | `captured_text`, `matched_word` |
| `by` | `count ... by 2`, `divided by`, `split ... by` | `step_size`, `divisor`, `separator` |
| `find` | `find pattern in text` | `search_term`, `find_result` |

---

## Control Flow Keywords (23)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `break` | Exit loop early | ✗ |
| `by` | Count-loop step; also `divided by`, `split ... by` | ✗ |
| `check` | Start conditional | ✗ |
| `continue` | Skip to next iteration | ✗ |
| `count` | Count loop keyword | ✓ |
| `each` | For each loop | ✗ |
| `end` | Close block | ✗ |
| `exit` | Exit loops (`exit loop`) or the program (`exit program [with code number]`) | ✗ |
| `for` | For loop | ✗ |
| `forever` | Infinite loop | ✗ |
| `from` | Count loop start | ✗ |
| `if` | Conditional | ✗ |
| `in` | For each collection | ✗ |
| `loop` | Loop reference | ✗ |
| `otherwise` | Else, else-if (`otherwise check if`), or default clause | ✗ |
| `repeat` | Loop construct | ✗ |
| `reversed` | Reverse a for-each loop (`for each item in reversed items`) | ✓ |
| `route` | Dispatch on a value (match/switch) | ✗ |
| `skip` | Continue (alias) | ✗ |
| `then` | Conditional separator | ✗ |
| `to` | Count loop end | ✗ |
| `until` | Loop condition | ✗ |
| `while` | Loop condition | ✗ |

**Counting down:** write `count from 5 down to 1:`. The `down` in `down to` is
not a keyword. A count loop recognizes it only between the start value and
`to`, so `down` is still a normal variable name everywhere else. `downward`
and `upward` are not count directions. If the start value is a variable, put
it in parentheses: `count from (top) down to 1:`. See
[Counting Down](../03-language-basics/loops-and-iteration.md#counting-down).

---

## Declaration Keywords (19)

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
| `new` | Instantiate container (`create new K [with value and ...] as k: ... end`); `store new` always starts `store new constant` | ✗ |
| `parameters` | Action parameter list (`define action called f with parameters x:`) | ✗ |
| `property` | Container field | ✗ |
| `store` | Create variable | ✗ |

---

## Operations Keywords (23)

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
| `divided` | Division (first word of `divided by`) | ✗ |
| `divided by` | Division (`10 divided by 2`) | ✗ |
| `execute` | Run command | ✗ |
| `give` | Return keyword part | ✓ |
| `kill` | Terminate process | ✗ |
| `minus` | Subtraction | ✗ |
| `modulo` | Modulo/remainder (word form of `%`) | ✗ |
| `multiply` | Multiplication | ✗ |
| `plus` | Addition | ✗ |
| `push` | Add to list | ✗ |
| `remove` | Remove item | ✗ |
| `return` | Return value | ✗ |
| `subtract` | Subtraction | ✗ |
| `times` | Multiplication | ✗ |
| `wait` | Async wait | ✗ |

---

## Module Keywords (4)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `export` | Export a definition from a module file (`export container Person`) | ✗ |
| `include` | Run another file in this scope (`include from "lib.wfl"`) | ✗ |
| `load` | Load module | ✗ |
| `module` | Module reference | ✗ |

---

## Comparison Keywords (19)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `above` | Greater than (alias) | ✗ |
| `and` | Logical AND | ✗ |
| `at` | Location preposition | ✓ |
| `below` | Less than (alias) | ✗ |
| `contains` | Contains check | ✓ |
| `ends` | Text suffix check (`name ends with ".css"`) | ✓ |
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
| `starts` | Text prefix check (`path starts with "/api"`) | ✓ |
| `than` | Comparison | ✗ |
| `with` | Action/constructor arguments or concatenation | ✗ |

---

## Pattern Matching Keywords (27)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `ahead` | Lookahead assertion | ✗ |
| `any` | Match any character | ✗ |
| `behind` | Lookbehind assertion | ✗ |
| `between` | Range specification | ✗ |
| `capture` | Capture group | ✗ |
| `captured` | Backreference: `same as captured "name"` | ✗ |
| `category` | Unicode category | ✗ |
| `character` | Character class | ✗ |
| `digit` | Digit class | ✗ |
| `exactly` | Exact quantifier | ✗ |
| `find` | Search text: `find pattern in text` | ✗ |
| `greedy` | Reserved word; not accepted in pattern syntax. Default quantifiers are already greedy. | ✗ |
| `lazy` | Reserved word; not accepted in pattern syntax. There is no lazy quantifier spelling. | ✗ |
| `letter` | Letter class | ✗ |
| `matches` | Pattern match check | ✗ |
| `more` | Quantifier part | ✗ |
| `one` | Quantifier (one or more) | ✗ |
| `optional` | Optional quantifier | ✗ |
| `pattern` | Pattern definition | ✓ |
| `replace` | Pattern or text replacement | ✗ |
| `script` | Unicode script | ✗ |
| `split` | Split by pattern | ✗ |
| `start` | `start of text` anchor; also `start streaming response` | ✗ |
| `text` | Text type | ✓ |
| `unicode` | Unicode support | ✗ |
| `whitespace` | Whitespace class | ✗ |
| `zero` | Zero quantifier | ✗ |

---

## File & I/O Keywords (20)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `append` | Append to file | ✗ |
| `appending` | Append mode | ✗ |
| `binary` | Binary mode (`read binary from h`, `write binary data into h`) | ✓ |
| `bytes` | Byte count (`read 8 bytes from h`) | ✓ |
| `close` | Close file | ✗ |
| `content` | File content | ✗ |
| `delete` | Delete file | ✗ |
| `denied` | Permission denied | ✗ |
| `directory` | Directory resource | ✗ |
| `exists` | File exists check | ✗ |
| `extension` | File extension | ✓ |
| `extensions` | File extensions | ✓ |
| `file` | File resource | ✗ |
| `files` | File collection | ✓ |
| `found` | File found check | ✗ |
| `open` | Open file | ✗ |
| `permission` | File permission | ✗ |
| `read` | Read operation | ✓ |
| `recursively` | Recursive operation | ✗ |
| `write` | Write operation | ✗ |

---

## Web & Network Keywords (21)

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
| `that` | Optional filler (`wait for request that comes in on server as req`) | ✓ |
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
| `new` | Create instance | ✗ |
| `on` | Event handler | ✗ |
| `parent` | Parent reference | ✓ |
| `private` | Private visibility | ✗ |
| `property` | Container property | ✗ |
| `public` | Public visibility | ✗ |
| `requires` | Interface requirement | ✗ |
| `static` | Static member | ✗ |
| `trigger` | Fire event | ✗ |

`this` is not a keyword (it is not counted above): inside an instance action it
names the object the action was called on, as in `this.other_action()`, unless
the program has its own variable named `this` there. See
[Calling Other Actions with `this`](../04-advanced-features/containers-oop.md#calling-other-actions-with-this).

---

## Error Handling Keywords (5)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `catch` | Error handler | ✗ |
| `error` | Error reference | ✗ |
| `finally` | Always-run cleanup clause | ✗ |
| `try` | Error handling block | ✗ |
| `when` | Specific error type; also a `route` arm head | ✗ |

---

## Process & Execution Keywords (11)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `arguments` | Command arguments | ✗ |
| `command` | System command | ✗ |
| `execute` | Run command | ✗ |
| `into` | Output redirection | ✗ |
| `kill` | Terminate process | ✗ |
| `output` | `read output from process` | ✗ |
| `process` | Process resource | ✗ |
| `running` | Running state | ✗ |
| `shell` | Shell execution | ✗ |
| `spawn` | Start process | ✗ |
| `using` | Using clause | ✗ |

---

## Data & Types Keywords (8)

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

## Testing Keywords (10)

| Keyword | Description | As Var? |
|---------|-------------|---------|
| `be` | Assertion verb (`expect x to be greater than 5`) | ✗ |
| `contain` | Assertion: contains a value (`to contain "apple"`) | ✓ |
| `describe` | Group related tests (`describe "Math":`) | ✗ |
| `empty` | Assertion: empty value (`to be empty`) | ✗ |
| `exist` | Assertion: value exists (`to exist`) | ✓ |
| `expect` | Start an assertion (`expect result to equal 5`) | ✗ |
| `have` | Assertion: length (`to have length 3`) | ✓ |
| `setup` | Setup block inside `describe` | ✗ |
| `teardown` | Cleanup block inside `describe` | ✗ |
| `test` | Single test case (`test "adds numbers":`) | ✗ |

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

// In a list declaration, 'list' is still the type keyword
create list items:
    add "apple"
end list
```

**Example: `query` and `connect` (database statement shapes)**
```wfl
// ✅ 'query' works as a plain variable
store query as "SELECT 1"
store copy as query

// ❌ 'query <handle> with ...' after 'store ... as' is a database query
store rows as query db with "SELECT * FROM users"

// ❌ 'connect to database at ... as ...' is a statement
connect to database at "sqlite://app.db" as db
```

---

## Common Scenarios

### "Is X a keyword?" → Find it in the tables above
- ✗ means never use as variable
- ✓ means context-dependent (see comprehensive reference)

### "Getting 'Expected identifier' or 'Cannot use reserved keyword' error?" → Check if you used a reserved keyword
- The error names the word as a token: `KeywordStart` means `start`, `KeywordRoute` means `route`
- Add underscore: `is` → `is_valid`, `start` → `start_time`
- Use different word: `file` → `filename`
- See [Reading the Error Message](reserved-keywords.md#reading-the-error-message) for every message form

### "Can I use this contextual keyword?" → See the Comprehensive Reference
- 31 of the 32 contextual keywords CAN be used as variables in certain contexts
- `new` cannot be declared with `store`, because `store new` starts `store new constant`
- `any`, `push`, `skip`, `than` and `zero` are listed as contextual in the source code too, but they are structural and always reserved

### "What about `pop`, `upward`, `downward`?" → Not keywords

`pop` is a standard-library list function, and `upward` / `downward` have no
special meaning. All three are ordinary names. Earlier versions of these pages
listed them as keywords by mistake. To count down, write `down to`, as in
`count from 5 down to 1:`; see the **Counting down** note under
[Control Flow Keywords](#control-flow-keywords-23).

### "What about `secured`, `certificate`, `key`, `redirecting`, `content_type`, `transaction`, `schema`, `changes`, `without following redirects`, `all`?" → Not keywords

For HTTPS, `certificate ... and key ... for "example.com"` associates a pair
with a DNS name using the existing `for` keyword. Join more named pairs with
`and certificate`. See [multiple-domain HTTPS](../04-advanced-features/web-servers.md#multiple-domains-on-one-https-port).
These words are recognized purely by position — inside `listen` / `respond` statements, in `in transaction on db:` / `in transaction on db for schema changes:` / `end transaction`, as the `and without following redirects` clause in `open url`, or as `all` directly after `find` (`find all <pattern> in <text>`) — and are **never reserved**. Use them as variable names freely. `raise_error` is a standard-library function, not a keyword. See [Marker Words That Are Not Keywords](reserved-keywords.md#marker-words-that-are-not-keywords-at-all).

---

## Related Documentation

- **[Reserved Keywords (Complete) →](reserved-keywords.md)** - Complete technical reference with classifications
- **[Variables and Types →](../03-language-basics/variables-and-types.md)** - Variable naming rules
- **[Naming Conventions →](../06-best-practices/naming-conventions.md)** - Best practices for naming
- **[Syntax Reference →](syntax-reference.md)** - Quick syntax lookup
- **[Error Codes →](error-codes.md)** - Understanding keyword errors

---

**Previous:** [← Language Specification](language-specification.md) | **Next:** [Reserved Keywords (Complete) →](reserved-keywords.md)
