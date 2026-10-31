# Reserved Keywords (Complete Technical Reference)

Complete technical documentation for all WFL reserved keywords.

**→ For quick lookup:** [Keyword Reference (Quick) →](keyword-reference.md)

**Source:** WFL Compiler v26.9.x (extracted from `src/lexer/token.rs`)
**Total:** 198 keywords and literals
**Last Updated:** 2026-10-31

**Kept in sync by a test:** `tests/suite/keyword_reference_drift_test.rs` reads
`src/lexer/token.rs` and fails when this page or the
[quick reference](keyword-reference.md) misses a lexer word, lists a word that
is not one, gives a word the wrong type or "As Var?" value, or states a count
that does not match its list. If you add a keyword to the lexer, add it here,
to the quick reference, and to the counts.

---

## Table of Contents

- [What Is a Reserved Word? (Start Here)](#what-is-a-reserved-word-start-here)
  - [Why WFL Reserves Words](#why-wfl-reserves-words)
  - [What It Costs, and How WFL Keeps the Cost Small](#what-it-costs-and-how-wfl-keeps-the-cost-small)
  - [Reading the Error Message](#reading-the-error-message)
- [Words That Commonly Bite: `start`, `output`, `route`, `captured`, `by`, `find`](#words-that-commonly-bite)
1. [Understanding Keyword Classifications](#understanding-keyword-classifications)
   - [What Makes a Keyword "Structural"?](#what-makes-a-keyword-structural)
   - [What Makes a Keyword "Contextual"?](#what-makes-a-keyword-contextual)
   - [What Makes a Keyword "Other"?](#what-makes-a-keyword-other)
   - [Why Some Keywords Appear in Both Lists](#why-some-keywords-appear-in-both-lists)
2. [Structural Keywords (63)](#structural-keywords-63)
3. [Contextual Keywords (32)](#contextual-keywords-32)
4. [Other Reserved Keywords (96)](#other-reserved-keywords-96)
5. [Boolean & Null Literals (7)](#boolean--null-literals-7)
6. [Keywords by Feature Category](#keywords-by-feature-category)
7. [Usage Guidelines & Best Practices](#usage-guidelines--best-practices)
8. [Common Pitfalls](#common-pitfalls)
9. [Complete Alphabetical Reference](#complete-alphabetical-reference)

---

## What Is a Reserved Word? (Start Here)

A **reserved word** is a word that WFL has claimed for its own grammar. WFL
also calls these words **keywords**. Examples are `store`, `check`, `route`,
and `find`.

You **cannot** use a reserved word as a name that you choose yourself. "A name
you choose" means any of these:

| Where you choose a name | Example of the mistake |
|---|---|
| A variable | `store find as "cat"` |
| A loop variable | `for each start in items:` |
| An action parameter | `define action called greet with parameters output:` |
| An action name | `define action called route:` |
| One word of a multi-word name | `store start time as 9` |

When this happens, WFL stops **before your program runs** and reports a parse
error. No line of your program executes, so nothing is left half-done.

The fix is always the same: **add a describing word, joined with an
underscore.** `start_time`, `command_output`, and `search_term` are ordinary
names, because WFL reads a name joined with underscores as one whole word.
(A name with *spaces*, such as `start time`, is read word by word, so a
reserved word inside it still breaks it.)

```wfl
// ❌ Each of these lines would stop the program before it runs:
// store find as "cat"
// store start time as 9

// ✅ The same ideas, with underscore names:
store search_term as "cat"
store start_time as 9
display search_term
display start_time
```

### Why WFL Reserves Words

Most programming languages mark structure with **symbols**: brackets `{ }`,
parentheses `( )`, semicolons `;`, and the equals sign `=`. WFL deliberately
uses very few symbols so that code reads like English (see Principles 1 and 2
in [WFL Foundation](../wfl-foundation.md)). Instead, WFL uses **words** to do
the job that punctuation does in other languages.

That only works if each grammar word means exactly one thing. Look at this
line:

```text
route path:
```

WFL reads `route` and knows that a dispatch block starts here. Now imagine that
`route` could also be a variable you created. WFL would have to guess which
meaning you wanted. A wrong guess would make your program do something
different from what you wrote, without telling you. Reserving the word removes
the guess.

Reserving words gives you four concrete benefits:

1. **No guessing.** Every line has exactly one meaning. WFL never has to decide
   between "this is a keyword" and "this is your variable".
2. **Early errors.** A naming conflict is reported when WFL reads your file,
   before any line runs. It never appears later as a surprise in the middle of
   a running program (Principles 4 and 17: clear error reporting and error
   transparency).
3. **Programs keep their meaning.** A reserved word can never be a variable.
   So when a later WFL version adds a new phrase that uses that word, programs
   that already work keep meaning the same thing. This protects WFL's
   backward-compatibility promise.
4. **Code you can trust when reading.** In every WFL program, `find` means
   "search text with a pattern". It never secretly means somebody's variable.

### What It Costs, and How WFL Keeps the Cost Small

Reserving a word has a real cost: it takes a natural English name away from
you. Words such as `start`, `output`, and `find` are names that people
genuinely want to use. WFL tries to keep this cost small in two ways:

- **Contextual keywords** are special in only one position. For example,
  `count` is the loop variable inside `count from 1 to 10:`, but
  `store count as 0` works everywhere else. See
  [Contextual Keywords](#contextual-keywords-32).
- **Marker words** are not reserved at all. HTTPS settings (`secured`,
  `certificate`, `key`) and database transactions (`transaction`) were added
  as marker words, so they take no names away from you. See
  [Marker Words That Are Not Keywords at All](#marker-words-that-are-not-keywords-at-all).

The words in the next section are **fully** reserved: no name you choose may be
one of them (with one narrow exception for `output`, explained below).

### Reading the Error Message

The error message points at the exact word, but it names the word by WFL's
**internal token name**, not by the word itself. To find the word, remove
`Keyword` from the front of the token name and read the rest in lowercase:
`KeywordStart` means `start`, and `KeywordRoute` means `route`.

Depending on the word and where you used it, you will see one of these
messages:

| You wrote | WFL says |
|---|---|
| `store start as 1` | `Expected identifier for variable name, found KeywordStart` |
| `store route as 1` | `Cannot use reserved keyword 'KeywordRoute' as a variable name` |
| `for each start in items:` | `Expected identifier after 'each', found KeywordStart` |
| `define action called find:` | `Expected identifier after 'called', found KeywordFind` |
| `define action called greet with parameters output:` | `Expected ':' after action definition: expected Colon, found KeywordOutput` |

The last message does not mention a name at all. If WFL reports an unexpected
`Keyword...` token where you meant to write a name, check this page first.

---

<a id="words-that-commonly-bite"></a>

## Words That Commonly Bite: `start`, `output`, `route`, `captured`, `by`, `find`

These six words are everyday English words that programmers often want to use
as names. All six are reserved. WFL rejects each of them as a variable name, a
loop variable, an action parameter, and an action name.

| Word | Kind | What WFL uses it for | Use instead |
|---|---|---|---|
| `start` | Other reserved | Starting a streaming web response; the `start of text` pattern anchor | `start_time`, `start_index`, `starting_value` |
| `output` | Other reserved | Reading a running process's output | `command_output`, `process_output`, `result_text` |
| `route` | Structural | Dispatching on a value (`route ... end route`) | `current_route`, `request_path`, `route_name` |
| `captured` | Other reserved | Referring back to a named capture inside a pattern | `captured_text`, `matched_word`, `capture_value` |
| `by` | Structural | Count-loop steps, division, splitting, multiply/divide | `step_size`, `divisor`, `separator` |
| `find` | Other reserved | Searching text with a pattern | `search_term`, `found_item`, `find_result` |

"Structural" and "Other reserved" are explained in
[Understanding Keyword Classifications](#understanding-keyword-classifications).
For you, the result is the same: neither kind can be a name you choose.

### `start`

**Why it is reserved:** `start` opens two different constructs:

- In a web server, `start streaming response to req with status 200 and content type "application/x-ndjson" as out`
  begins a response that is sent in pieces. See
  [Streaming a response](../04-advanced-features/web-servers.md#streaming-a-response).
- Inside a pattern, `start of text` means "the match must begin at the very
  start of the text".

**Use instead:** `start_time`, `start_index`, `starting_value`.

### `output`

**Why it is reserved:** `output` is part of the statement that reads what a
running process has printed: `wait for read output from process child as text`.
See [Subprocess Execution](../04-advanced-features/subprocess-execution.md).

**The one narrow exception:** that same statement may name its result
`output` (`wait for read output from process child as output`), and later lines
may read the value. This is accepted because existing programs use it. You
still cannot create or change a variable named `output` with `store` or
`change`, or use it as a loop variable, parameter, or action name. Choosing a
different name such as `command_output` avoids the exception entirely.

**Use instead:** `command_output`, `process_output`, `result_text`.

### `route`

**Why it is reserved:** `route` starts a dispatch block that chooses one branch
by value:

```text
route path:
    when "/health":
        ...
    otherwise:
        ...
end route
```

`route` is a **structural** keyword, the strictest kind. See
[Routing](../04-advanced-features/routing.md). Its partner word `when` is also
reserved.

**Use instead:** `current_route`, `request_path`, `route_name`.

### `captured`

**Why it is reserved:** inside a pattern, `same as captured "word"` means "match
the same text that the capture named `word` matched earlier". This lets a
pattern find a repeated word such as `"hello hello"`.

**Use instead:** `captured_text`, `matched_word`, `capture_value`.

### `by`

**Why it is reserved:** `by` connects a value to the amount or separator that
acts on it, in several different statements:

- `count from 1 to 10 by 2:` (count-loop step)
- `10 divided by 2` (division)
- `split "a,b,c" by ","` (splitting text)
- `multiply total by 3` and `divide total by 2` (changing a number in place)

`by` is a **structural** keyword, the strictest kind.

**Use instead:** `step_size`, `divisor`, `separator`.

### `find`

**Why it is reserved:** `find email_pattern in contact_info` searches text with
a pattern. It gives back the first match, or `nothing` when there is no match.
See [Pattern Matching](../04-advanced-features/pattern-matching.md) and the
[Pattern Module](../05-standard-library/pattern-module.md).

**Use instead:** `search_term`, `found_item`, `find_result`.

### All Six, Side by Side

This program uses every one of the six words in its real job, and uses the
suggested replacement names for its own variables. The validated copy is
`TestPrograms/docs_examples/keyword_reference/common_reserved_words.wfl`.

```wfl
// `start` and `captured` doing their real jobs, inside patterns
create pattern begins_with_hello:
    start of text then "hello"
end pattern

create pattern doubled_word:
    capture {one or more letter} as word then " " then same as captured "word"
end pattern

// `by` doing its real jobs
store step_size as 2
store separator as ","
store parts as split "red,green,blue" by separator
store half as 10 divided by 2

// `find` doing its real job
store search_text as "say hello hello"
store find_result as find doubled_word in search_text

// Our own names: never the bare reserved word
store start_time as "09:00"
store command_output as "nothing run yet"
store current_route as "/health"
store captured_text as "hello"

check if "hello world" matches begins_with_hello:
    display "starts with hello"
end check

check if isnothing of find_result:
    display "no doubled word"
otherwise:
    display "found a doubled word"
end check

count from 1 to 5 by step_size:
    display count
end count

display parts
display half
display captured_text
display start_time
display command_output
display current_route
```

**Output:**
```
starts with hello
found a doubled word
1
3
5
[red, green, blue]
5
hello
09:00
nothing run yet
/health
```

---

## Understanding Keyword Classifications

WFL has 198 reserved words: 63 structural keywords, 32 contextual keywords, 96
other reserved keywords, and 7 boolean and null literals. Each word belongs to
exactly one of these four types. The type tells you whether you can use the
word as a variable name.

You can see the type in the error message when you try `store <word> as 1`:

| Type | Count | `store <word> as 1` |
|------|-------|---------------------|
| Structural | 63 | Error: `Cannot use reserved keyword '…' as a variable name` |
| Contextual | 32 | Works (except `new`, see below) |
| Other | 96 | Error: `Expected identifier for variable name, found …` |
| Literal | 7 | Error: `Expected identifier for variable name, found …` |

The one structural word with a different message is `as`: `store as as 1`
fails with `Expected a variable name before 'as'`.

### What Makes a Keyword "Structural"?

**Structural keywords** are the core building blocks of WFL programs. They define program structure, control flow, and fundamental operations. Think of them as the "grammar words" of WFL—just like you wouldn't use "if" or "the" as names in English writing, these words have special meaning in WFL.

**Key characteristics:**
- **Always reserved** - Cannot be used as variable names in any context
- **Parser priority** - Checked first by the parser
- **Total count:** 63 keywords

**Examples:**
```wfl
// These are structural - NEVER use as variables
check if x is 5:      // 'check' and 'if' are structural
    display "Yes"     // 'display' is structural
end check            // 'end', 'check' are structural

store name as "Alice"  // 'store', 'as' are structural
```

**Why they're always reserved:** The parser relies on these keywords to understand the structure of your program. If you could use `if` as a variable name, the parser wouldn't know whether `if` means "conditional" or "your variable."

### What Makes a Keyword "Contextual"?

**Contextual keywords** have special meaning only in specific statement positions. Everywhere else they are ordinary names.

**Key characteristics:**
- **Context-dependent meaning** - Special in certain situations
- **31 are usable as variables** - `store count as 0` works, and `display count` reads the value back
- **1 cannot be declared with `store`** - `new`, because `store new` always begins `store new constant NAME as value`
- **Total count:** 32 keywords

**The 31 you CAN use as variables:**
```wfl
// Example 1: 'count' as a variable (outside count loops)
store count as 0              // ✅ 'count' is just a variable here
change count to count plus 1
display count

// But inside count loops, 'count' is the loop counter:
count from 1 to 10:          // ✅ 'count' is a keyword here
    display count            // ✅ 'count' refers to loop variable
end count

// Example 2: 'list' as a variable (outside list declarations)
store list as "my_list_name"  // ✅ 'list' is just a variable here
display list

// 'list' is still the type keyword in a list declaration:
create list items:            // ✅ 'list' is a type keyword here
    add "apple"
end list
```

**The one you CANNOT declare with `store`:**
- `new` - `store new as "fresh"` fails with `Expected 'constant' after 'new'`, because `store new` is the start of `store new constant MAX as 10`

### What Makes a Keyword "Other"?

**Other reserved keywords** belong to one feature, such as file I/O, pattern
matching, or web servers. They are neither structural nor contextual, and the
parser does not accept them as variable names: `store file as "a.txt"` fails
with `Expected identifier for variable name, found KeywordFile`. `is` is in
this group, not in the structural group, but it is just as reserved.

### Marker Words That Are Not Keywords at All

A few words have special meaning in exactly one statement position but are **not reserved in any way** — they are ordinary identifiers that the parser recognizes purely by position. They do not count toward the 198 keywords, and you can freely use them as variable names everywhere:

- `secured` - HTTPS marker in `listen on port 8443 secured ... as server`
- `certificate` - certificate path marker in `secured with certificate "cert.pem"`
- `key` - private key path marker in `secured with ... and key "key.pem"`
- `redirecting` - redirect marker in `listen on port 8080 redirecting to port 8443 as server`
- `content_type` - response content type marker in `respond to req with ... and content_type "text/html"`
- `transaction` - transaction block marker in `in transaction on db:` and `end transaction`
- `schema`, `changes` - optional SQLite transaction mode in `in transaction on db for schema changes:`
- `without following redirects` - per-request redirect clause in `open url at address and without following redirects and read response as reply`; the individual words and the complete phrase remain ordinary variable names outside that clause position
- `all` - every-match marker in `find all <pattern> in <text>`; see below for names that start with `all`
- `down` - count-down marker in `count from 5 down to 1:`; recognized only between a count loop's start value and `to` (see [`down` and Counting Down](#down-and-counting-down) below)

TLS pairs can also use the existing `for` keyword to select a DNS name:
`certificate "cert.pem" and key "key.pem" for "example.com"`. Additional named
pairs begin with `and certificate`; this adds no reserved words. See
[multiple-domain HTTPS](../04-advanced-features/web-servers.md#multiple-domains-on-one-https-port).

`initialize` is an ordinary container action name, not a reserved keyword.
`create new K with value and another as k: ... end` passes positional values
to it after the property block runs. This syntax adds no keywords.

`raise_error` is an ordinary standard-library function called with `call
raise_error with message` or `raise_error of message`; it is not a keyword.

`pop`, `upward` and `downward` are not keywords either. `pop` is a
standard-library list function, and `upward` / `downward` have no special
meaning. Earlier versions of this page listed all three by mistake. A count
loop counts down with `down to`; see
[`down` and Counting Down](#down-and-counting-down) below.

```wfl
// All perfectly valid — these words are not reserved:
store key as "secret_key_456"
store certificate as "diploma"
store secured as yes
store transaction as "TX-1094"
```

`transaction` is recognized in exactly two positions: directly after a leading
`in`, and directly after the `end` that closes the block. A statement beginning
with `in` was always a parse error before, so nothing that used to be valid
changed meaning.

`all` is recognized only directly after `find`. Before `find all` existed,
`find all digits in text` searched with a variable literally named
`all digits` (WFL names may contain spaces), and `find all in text` with one
named `all`. Those programs keep that meaning: when a variable with the full
name (`all digits`, or `all`) exists, the single-match `find` is used. Otherwise
`find all digits in text` returns every match of `digits`.

`this` is not a keyword either. Inside a container's instance action it is a
predefined, read-only name for the object the action was called on, used to
call sibling actions (`this.other_action()`) and read properties
(`this.name`); see [Calling Other Actions with
`this`](../04-advanced-features/containers-oop.md#calling-other-actions-with-this).
It only fills in where the name would otherwise be undefined. A program's own
variable named `this` keeps its meaning: one that already exists where the
container is defined (such as a top-level `this` above it), or one the action
creates with `store this as ...` or `for each this in ...`. So does a property
named `this`. Outside instance actions `this` is an ordinary identifier.

#### `down` and Counting Down

A count loop counts down with `down to`: `count from 5 down to 1:`. `down` is
recognized in exactly one position, between the loop's start value and `to`.
Everywhere else it is an ordinary name.

`downward` and `upward` are not count directions. Writing
`count from 5 downward to 1:` is a parse error that points at the word and
suggests the real form:
`Expected 'to' or 'down to', found Identifier("downward")`. All three words
work as variable names:

```wfl
// `down` is not a keyword: a count loop recognizes it only right before `to`
count from 3 down to 1:
    display count
end count

// So `down` is still an ordinary name everywhere else,
// and so are `downward` and `upward`, which WFL never used
store down as "the floor below"
store downward as "toward the floor"
store upward as "toward the sky"
display down
display downward
display upward
```

When the start value is a variable, put it in parentheses:
`count from (top) down to 1:`. Without them, WFL joins neighboring plain words
into one name, reads `top down` as a single variable, and reports
`Variable 'top down' is not defined`. See
[Counting Down](../03-language-basics/loops-and-iteration.md#counting-down).

### Why Some Keywords Appear in Both Lists

Five keywords — `any`, `push`, `skip`, `than` and `zero` — appear in both the
structural list (`Token::is_structural_keyword`) and the contextual list
(`Token::is_contextual_keyword`) in `src/lexer/token.rs`.

1. **Parser checks structural keywords first** - If a keyword is structural, it's always reserved
2. **Net result:** A keyword in BOTH lists behaves exactly like a structural keyword
3. **Counting:** This page counts each of the five once, as structural. The 32 contextual keywords are the words that are contextual and not structural.

**Example:**
```wfl
// 'push' appears in both lists, but structural wins
push with myList and item    // ✅ 'push' is a keyword here
store push as "save"         // ❌ ERROR: 'push' is reserved (structural)
```

---

## Structural Keywords (63)

These keywords **MUST** always be reserved and **CANNOT** be used as variable names. They define program structure and control flow.

| Keyword | Description | Example Usage |
|---------|-------------|---------------|
| `action` | Define a function/action | `define action called calculate with x:` |
| `and` | Logical AND operator | `check if x is 5 and y is 10:` |
| `any` | Match any (pattern/quantifier) | `define pattern zero or more any` |
| `as` | Binding/assignment keyword | `store name as "Alice"` |
| `be` | Test assertion verb | `expect x to be greater than 5` |
| `break` | Exit loop early | `break` |
| `by` | Step distance in count loops — always positive; `down to` sets the direction | `count from 1 to 10 by 2:` |
| `call` | Call an action | `call myFunction with x` |
| `catch` | Error handler | `catch when error:` |
| `check` | Start conditional | `check if x is greater than 5:` |
| `constant` | Define constant property | `constant property max_size as 100` |
| `container` | Define a class/container | `create container Person:` |
| `continue` | Skip to next iteration | `continue` |
| `define` | Define action/container | `define action called test:` |
| `describe` | Group related tests | `describe "Math":` |
| `display` | Output text | `display "Hello World"` |
| `each` | For each loop | `for each item in list:` |
| `empty` | Test assertion: empty value | `expect items to be empty` |
| `end` | Close block | `end` |
| `event` | Define an event | `event click` |
| `expect` | Test assertion | `expect result to equal 5` |
| `export` | Export a definition from a module file | `export container Person` |
| `extends` | Inheritance | `create container Employee extends Person:` |
| `finally` | Always-run cleanup clause on try | `finally:` |
| `for` | For loop | `for each x in items:` |
| `forever` | Infinite loop | `repeat forever:` |
| `from` | Count loop start | `count from 1 to 10:` |
| `if` | Conditional | `check if x is 5:` |
| `implements` | Interface implementation | `create container Dog implements Animal:` |
| `in` | For each collection | `for each item in list:` |
| `include` | Run another file in this scope | `include from "lib.wfl"` |
| `interface` | Interface definition | `create interface Runnable:` |
| `load` | Load module | `load module math` |
| `module` | Module reference | `load module fs` |
| `not` | Logical NOT | `check if not x:` |
| `on` | Event handler | `on click:` |
| `or` | Logical OR | `check if x is 5 or y is 10:` |
| `otherwise` | Else / else-if clause | `otherwise:` / `otherwise check if` |
| `private` | Private visibility | `private property age` |
| `property` | Container property | `property name: Text` |
| `public` | Public visibility | `public property name` |
| `push` | Add to list | `push with myList and item` |
| `repeat` | Loop construct | `repeat 10 times:` |
| `requires` | Interface requirement | `requires action run` |
| `return` | Return value | `return result` |
| `route` | Dispatch on a value (match/switch) | `route path:` |
| `setup` | Setup block inside `describe` | `setup:` |
| `skip` | Continue (alias) | `skip` |
| `static` | Static member | `static property counter` |
| `store` | Create variable | `store x as 10` |
| `teardown` | Cleanup block inside `describe` | `teardown:` |
| `test` | Single test case | `test "adds numbers":` |
| `than` | Comparison keyword | `greater than`, `less than` |
| `then` | Conditional separator | `check if x is 5 then display "yes"` |
| `to` | Count loop end | `count from 1 to 10:` |
| `trigger` | Fire event | `trigger myEvent` |
| `try` | Error handling | `try:` |
| `until` | Loop condition | `repeat until x is 10:` |
| `wait` | Async wait | `wait for result` |
| `when` | Error type / `route` arm head | `catch when FileError:` · `when "/health":` |
| `while` | Loop condition | `repeat while x is less than 10:` |
| `with` | Parameter separator | `call action with x and y` |
| `zero` | Pattern quantifier | `define pattern zero or more digits` |

**Total:** 63 structural keywords

**Important Note:** The keywords `any`, `push`, `skip`, `than`, and `zero` also appear in the contextual keyword list in the source code, but since structural keywords are checked first in the parser, they **CANNOT** be used as variable names. They are counted here, not under contextual keywords.

---

## Contextual Keywords (32)

These keywords are **context-dependent**. All but one **CAN** be used as variable names outside their keyword context. Each "works" example below was checked with the release binary: the value can be stored and read back with `display`.

### The 31 You CAN Use as Variables

| Keyword | Description | Context | Can Use As Variable? | Example |
|---------|-------------|---------|----------------------|---------|
| `at` | Location preposition | File/position operations | ✅ Yes | `store at as 5` works outside file ops |
| `back` | Return keyword part | `give back` expression | ✅ Yes | `store back as "rear"` works |
| `binary` | Binary I/O mode | `read binary from h`, `write binary data into h` | ✅ Yes | `store binary as yes` works |
| `bytes` | Byte count | `read 8 bytes from h` | ✅ Yes | `store bytes as 8` works |
| `called` | Action name | `define action called X` | ✅ Yes | `store called as "named"` works |
| `change` | Modify variable | Variable modification | ✅ Yes | `store change as "coins"` works |
| `contain` | Test assertion | `expect items to contain "apple"` | ✅ Yes | `store contain as 3` works |
| `contains` | Contains check | Collection/string operations | ✅ Yes | `store contains as yes` works |
| `count` | Count loop | `count from X to Y` | ✅ Yes | `store count as 0` works outside count loops |
| `create` | Create resource | Resource creation | ✅ Yes | `store create as "make"` works |
| `defaults` | Default value | Container properties | ✅ Yes | `store defaults as "standard"` works |
| `ends` | Text suffix check | `name ends with ".css"` | ✅ Yes | `store ends as "tail"` works |
| `exist` | Test assertion | `expect value to exist` | ✅ Yes | `store exist as yes` works |
| `extension` | File extension | File operations | ✅ Yes | `store extension as ".txt"` works |
| `extensions` | File extensions | File operations | ✅ Yes | `store extensions as [".txt", ".md"]` works |
| `files` | File collection | File operations | ✅ Yes | `store files as ["a.txt", "b.txt"]` works |
| `give` | Return keyword part | `give back` expression | ✅ Yes | `store give as "donate"` works |
| `have` | Test assertion | `expect items to have length 3` | ✅ Yes | `store have as 2` works |
| `least` | Minimum amount | Quantifiers | ✅ Yes | `store least as 1` works |
| `list` | List type | Type/create context | ✅ Yes | `store list as "shopping"` works outside type context |
| `map` | Map type | Type/create context | ✅ Yes | `store map as "directions"` works outside type context |
| `most` | Maximum amount | Quantifiers | ✅ Yes | `store most as 100` works |
| `must` | Requirement | Interface/validation | ✅ Yes | `store must as "required"` works |
| `needs` | Action parameters | `action needs X` (alias for `with`) | ✅ Yes | `store needs as "requirements"` works |
| `parent` | Parent reference | Container inheritance | ✅ Yes | `store parent as "mom"` works (context-sensitive) |
| `pattern` | Pattern definition | Pattern matching | ✅ Yes | `store pattern as "design"` works outside pattern context |
| `read` | Read operation | File I/O | ✅ Yes | `store read as "reading"` works (context-sensitive) |
| `reversed` | Reverse iteration | `for each item in reversed items` | ✅ Yes | `store reversed as yes` works |
| `starts` | Text prefix check | `path starts with "/api"` | ✅ Yes | `store starts as "head"` works |
| `text` | Text type | Type context | ✅ Yes | `store text as "message"` works outside type context |
| `that` | Optional filler word | `wait for request that comes in on server as req` | ✅ Yes | `store that as 3` works |

**Count:** 31 contextual keywords that CAN be used as variables (in appropriate contexts)

### The One You CANNOT Declare with `store`

| Keyword | Why | Example Error |
|---------|-----|---------------|
| `new` | `store new` always begins `store new constant NAME as value` | `store new as "fresh"` → ERROR: `Expected 'constant' after 'new'` |

**Total Contextual Keywords:** 32 (31 usable with `store`, plus `new`)

---

## Other Reserved Keywords (96)

All other reserved keywords that are neither structural nor contextual. These keywords cannot be used as variable names.

### File & I/O Operations (14)
`append`, `appending`, `close`, `content`, `delete`, `directory`, `exists`, `file`, `found`, `open`, `permission`, `denied`, `recursively`, `write`

### Arithmetic & Comparison (17)
`add`, `above`, `below`, `divide`, `divided`, `divided by`, `equal`, `greater`, `is`, `less`, `minus`, `modulo`, `multiply`, `plus`, `same`, `subtract`, `times`

### Pattern Matching (24)
`ahead`, `behind`, `between`, `capture`, `captured`, `category`, `character`, `digit`, `exactly`, `find`, `greedy`, `lazy`, `letter`, `matches`, `more`, `of`, `one`, `optional`, `replace`, `script`, `split`, `start`, `unicode`, `whitespace`

**Note:** Some pattern keywords like `any`, `zero`, `pattern`, `text` are counted in other categories (structural or contextual). `greedy` and `lazy` are reserved (they cannot be variable names) but are not accepted in pattern syntax; `greedy match` / `lazy match` is a parse error. Default quantifiers are greedy.

### Web & Network (19)
`accepting`, `comes`, `connections`, `current`, `formatted`, `handler`, `header`, `listen`, `milliseconds`, `port`, `register`, `request`, `respond`, `response`, `server`, `signal`, `status`, `stop`, `timeout`

### Process & Execution (11)
`arguments`, `command`, `execute`, `into`, `kill`, `output`, `process`, `running`, `shell`, `spawn`, `using`

### Data & Types (5)
`data`, `database`, `date`, `time`, `url`

**Note:** `list`, `map`, `text` are contextual keywords counted separately.

### Miscellaneous (6)
`clear`, `error`, `exit`, `loop`, `parameters`, `remove`

**Count:** 96 other reserved keywords

**Note:** This count (96) represents keywords that are not in the structural (63) or contextual (32) categories, and are not boolean/null literals (7). Total: 63 + 32 + 96 + 7 = 198.

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

Keywords organized by functional area for easier reference. A keyword can belong to more than one area.

### Control Flow (23 keywords)
`break`, `by`, `check`, `continue`, `count`, `each`, `end`, `exit`, `for`, `forever`, `from`, `if`, `in`, `loop`, `otherwise`, `repeat`, `reversed`, `route`, `skip`, `then`, `to`, `until`, `while`

### Declaration (19 keywords)
`action`, `as`, `called`, `change`, `constant`, `container`, `create`, `defaults`, `define`, `extends`, `implements`, `interface`, `list`, `map`, `needs`, `new`, `parameters`, `property`, `store`

### Operations (23 keywords)
`add`, `append`, `back`, `call`, `clear`, `close`, `display`, `divide`, `divided`, `divided by`, `execute`, `give`, `kill`, `minus`, `modulo`, `multiply`, `plus`, `push`, `remove`, `return`, `subtract`, `times`, `wait`

### Modules (4 keywords)
`export`, `include`, `load`, `module`

### Comparisons (19 keywords)
`above`, `and`, `at`, `below`, `contains`, `ends`, `equal`, `greater`, `is`, `least`, `less`, `most`, `not`, `of`, `or`, `same`, `starts`, `than`, `with`

### Pattern Matching (27 keywords)
`ahead`, `any`, `behind`, `between`, `capture`, `captured`, `category`, `character`, `digit`, `exactly`, `find`, `greedy`, `lazy`, `letter`, `matches`, `more`, `one`, `optional`, `pattern`, `replace`, `script`, `split`, `start`, `text`, `unicode`, `whitespace`, `zero`

### File & I/O (20 keywords)
`append`, `appending`, `binary`, `bytes`, `close`, `content`, `delete`, `denied`, `directory`, `exists`, `extension`, `extensions`, `file`, `files`, `found`, `open`, `permission`, `read`, `recursively`, `write`

### Web & Network (21 keywords)
`accepting`, `comes`, `connections`, `current`, `formatted`, `handler`, `header`, `listen`, `milliseconds`, `on`, `port`, `register`, `request`, `respond`, `response`, `server`, `signal`, `status`, `stop`, `that`, `timeout`

### Containers & OOP (17 keywords)
`constant`, `container`, `defaults`, `event`, `extends`, `implements`, `interface`, `must`, `new`, `on`, `parent`, `private`, `property`, `public`, `requires`, `static`, `trigger`

### Error Handling (5 keywords)
`catch`, `error`, `finally`, `try`, `when`

### Process & Execution (11 keywords)
`arguments`, `command`, `execute`, `into`, `kill`, `output`, `process`, `running`, `shell`, `spawn`, `using`

### Data & Types (8 keywords)
`data`, `database`, `date`, `list`, `map`, `text`, `time`, `url`

### Testing (10 keywords)
`be`, `contain`, `describe`, `empty`, `exist`, `expect`, `have`, `setup`, `teardown`, `test`

### Values (7 keywords)
`yes`, `no`, `true`, `false`, `nothing`, `missing`, `undefined`

---

## Usage Guidelines & Best Practices

### Rule 1: Structural and Other Keywords Are Always Reserved

Never use the 63 structural keywords or the 96 other reserved keywords as variable names, function names, or any other identifier.

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

The 31 contextual keywords marked ✅ CAN be used as variables, but only outside their keyword context.

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

// In a list declaration, 'list' is still the type keyword
create list items:
    add "apple"
end list
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

### Rule 5: Reserved Statement Shapes (Database)

The database statements reserve a few phrase shapes built from contextual
words rather than new keywords:

- `connect to database at ... as ...` — `connect` stays usable as an ordinary
  identifier everywhere else; only this exact phrase is a statement.
- `store <name> as query <handle> with <sql> ...` — a value expression
  beginning with `query <identifier>` directly followed by `with` is parsed as
  a database query. A plain variable named `query` still works
  (`store copy as query`), but avoid multi-word variable names whose first
  word is `query`.
- `store <name> as execute <handle> with <sql> ...` — same shape with the
  `execute` keyword.

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
store test as "unit"        // ❌ 'test' is reserved (testing framework)
```

**Solution:** Add context to your variable names.
```wfl
store target_file as "data.txt"
store is_valid as yes
store check_type as "verify"
store test_kind as "unit"
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

**Problem:** Not every contextual keyword works with `store`, and some words look contextual in the source code but are structural.

**The exceptions:**
```wfl
store new as "fresh"        // ❌ ERROR: 'store new' starts 'store new constant'
store any as "something"    // ❌ ERROR: 'any' is structural (also in the contextual list)
store push as "add"         // ❌ ERROR: 'push' is structural (also in the contextual list)
store skip as "jump"        // ❌ ERROR: 'skip' is structural (also in the contextual list)
store than as "then"        // ❌ ERROR: 'than' is structural (also in the contextual list)
store zero as 0             // ❌ ERROR: 'zero' is structural (also in the contextual list)
```

**Solution:** Check the "As Var?" column in the table below.

### Pitfall 5: A Reserved Word Inside a Multi-word Name

**Problem:** WFL lets a name contain spaces, such as `account balance`. WFL
reads such a name word by word. If any one of those words is reserved, the
name breaks, even though the whole phrase is not a keyword.

**Example:**
```wfl
// ❌ Each of these lines stops the program before it runs:
// store start time as "09:00"     ('start' is reserved)
// store program output as "done"  ('output' is reserved)
// store last route as "/home"     ('route' is reserved)

// ✅ Join the words with underscores instead:
store start_time as "09:00"
store program_output as "done"
store last_route as "/home"
display start_time
display program_output
display last_route
```

**Solution:** Use underscores (`start_time`) for any name that contains a
reserved word. See [Words That Commonly Bite](#words-that-commonly-bite).

---

## Complete Alphabetical Reference

Complete reference table of all 198 keywords and literals. "As Var?" shows
whether `store <word> as 1` is accepted.

| Keyword | Type | Category | As Var? | Example |
|---------|------|----------|---------|---------|
| `above` | Other | Comparison | ❌ | `above threshold` |
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
| `back` | Contextual | Operations | ✅ | `give back result` |
| `be` | Structural | Testing | ❌ | `expect x to be greater than 5` |
| `behind` | Other | Pattern | ❌ | `behind pattern` |
| `below` | Other | Comparison | ❌ | `below limit` |
| `between` | Other | Pattern | ❌ | `between values` |
| `binary` | Contextual | File I/O | ✅ | `read binary from h` / `write binary data into h` |
| `break` | Structural | Control Flow | ❌ | `break loop` |
| `by` | Structural | Control Flow | ❌ | `count from 1 to 10 by 2:` · `10 divided by 2` · `split text by ","` |
| `bytes` | Contextual | File I/O | ✅ | `read 8 bytes from h` |
| `call` | Structural | Operations | ❌ | `call function` |
| `called` | Contextual | Declaration | ✅ | `action called name` |
| `capture` | Other | Pattern | ❌ | `capture group` |
| `captured` | Other | Pattern | ❌ | `same as captured "word"` (pattern backreference) |
| `catch` | Structural | Error Handling | ❌ | `catch error` |
| `category` | Other | Pattern | ❌ | `unicode category` |
| `change` | Contextual | Declaration | ✅ | `change value` |
| `character` | Other | Pattern | ❌ | `character class` |
| `check` | Structural | Control Flow | ❌ | `check if condition` |
| `clear` | Other | Operations | ❌ | `clear data` |
| `close` | Other | File I/O / Process | ❌ | `close file` / `close process` |
| `comes` | Other | Web/Network | ❌ | `request comes in` |
| `command` | Other | Process | ❌ | `execute command` |
| `connections` | Other | Web/Network | ❌ | `network connections` |
| `constant` | Structural | Declaration | ❌ | `constant property` |
| `contain` | Contextual | Testing | ✅ | `expect items to contain "apple"` |
| `container` | Structural | OOP | ❌ | `create container` |
| `contains` | Contextual | Comparison | ✅ | `list contains item` |
| `content` | Other | File I/O | ❌ | `file content` |
| `continue` | Structural | Control Flow | ❌ | `continue loop` |
| `count` | Contextual | Control Flow | ✅ | `count from 1 to 10` |
| `create` | Contextual | Declaration | ✅ | `create resource` |
| `current` | Other | Web/Network | ❌ | `current time` |
| `data` | Other | Data & Types | ❌ | `process data` |
| `database` | Other | Data & Types | ❌ | `open database at "sqlite://app.db" as db` |
| `date` | Other | Data & Types | ❌ | `current date` |
| `defaults` | Contextual | Declaration | ✅ | `property defaults` |
| `define` | Structural | Declaration | ❌ | `define action` |
| `delete` | Other | File I/O | ❌ | `delete file` |
| `denied` | Other | File I/O | ❌ | `permission denied` |
| `describe` | Structural | Testing | ❌ | `describe "Math":` |
| `digit` | Other | Pattern | ❌ | `digit class` |
| `directory` | Other | File I/O | ❌ | `create directory` |
| `display` | Structural | Operations | ❌ | `display message` |
| `divide` | Other | Operations | ❌ | `divide by 2` |
| `divided` | Other | Operations | ❌ | `10 divided by 2` (first word of `divided by`) |
| `divided by` | Other | Operations | ❌ | `10 divided by 2` |
| `each` | Structural | Control Flow | ❌ | `for each item` |
| `empty` | Structural | Testing | ❌ | `expect items to be empty` |
| `end` | Structural | Control Flow | ❌ | `end block` |
| `ends` | Contextual | Comparison | ✅ | `name ends with ".css"` |
| `equal` | Other | Comparison | ❌ | `is equal to` |
| `error` | Other | Error Handling | ❌ | `handle error` |
| `event` | Structural | OOP | ❌ | `define event` |
| `exactly` | Other | Pattern | ❌ | `exactly 5 times` |
| `execute` | Other | Process | ❌ | `execute command` |
| `exist` | Contextual | Testing | ✅ | `expect value to exist` |
| `exists` | Other | File I/O | ❌ | `file exists` |
| `exit` | Other | Control Flow | ❌ | `exit loop` / `exit program with code 1` |
| `expect` | Structural | Testing | ❌ | `expect result to equal 5` |
| `export` | Structural | Modules | ❌ | `export container Person` |
| `extends` | Structural | OOP | ❌ | `container extends` |
| `extension` | Contextual | File I/O | ✅ | `file extension` |
| `extensions` | Contextual | File I/O | ✅ | `file extensions` |
| `false` | Literal | Values | ❌ | Boolean false |
| `file` | Other | File I/O | ❌ | `open file` |
| `files` | Contextual | File I/O | ✅ | `list files` |
| `finally` | Structural | Error Handling | ❌ | `finally:` |
| `find` | Other | Pattern | ❌ | `find email_pattern in contact_info` |
| `for` | Structural | Control Flow | ❌ | `for each` |
| `forever` | Structural | Control Flow | ❌ | `repeat forever` |
| `formatted` | Other | Web/Network | ❌ | `formatted response` |
| `found` | Other | File I/O | ❌ | `file found` |
| `from` | Structural | Control Flow | ❌ | `count from 1` |
| `give` | Contextual | Operations | ✅ | `give back` |
| `greater` | Other | Comparison | ❌ | `greater than` |
| `greedy` | Other | Pattern | ❌ | Reserved; not accepted in pattern syntax. Default quantifiers (`one or more`, `zero or more`, `at least N`) are greedy. |
| `handler` | Other | Web/Network | ❌ | `request handler` |
| `have` | Contextual | Testing | ✅ | `expect items to have length 3` |
| `header` | Other | Web/Network | ❌ | `HTTP header` |
| `if` | Structural | Control Flow | ❌ | `check if` |
| `implements` | Structural | OOP | ❌ | `implements interface` |
| `in` | Structural | Control Flow | ❌ | `for each in` |
| `include` | Structural | Modules | ❌ | `include from "lib.wfl"` |
| `interface` | Structural | OOP | ❌ | `create interface` |
| `into` | Other | Process | ❌ | `output into` |
| `is` | Other | Comparison | ❌ | `x is 5` |
| `kill` | Other | Process | ❌ | `kill process` |
| `lazy` | Other | Pattern | ❌ | Reserved; not accepted in pattern syntax. There is no lazy quantifier spelling. |
| `least` | Contextual | Comparison | ✅ | `at least 5` |
| `less` | Other | Comparison | ❌ | `less than` |
| `letter` | Other | Pattern | ❌ | `letter class` |
| `list` | Contextual | Data & Types | ✅ | `create list` |
| `listen` | Other | Web/Network | ❌ | `listen on port` |
| `load` | Structural | Modules | ❌ | `load module` |
| `loop` | Other | Control Flow | ❌ | `loop reference` |
| `map` | Contextual | Data & Types | ✅ | `create map` |
| `matches` | Other | Pattern | ❌ | `pattern matches` |
| `milliseconds` | Other | Web/Network | ❌ | `timeout milliseconds` |
| `minus` | Other | Operations | ❌ | `minus 5` |
| `missing` | Literal | Values | ❌ | Null alias |
| `module` | Structural | Modules | ❌ | `load module` |
| `modulo` | Other | Operations | ❌ | `10 modulo 3` |
| `more` | Other | Pattern | ❌ | `one or more` |
| `most` | Contextual | Comparison | ✅ | `at most 10` |
| `multiply` | Other | Operations | ❌ | `multiply by 2` |
| `must` | Contextual | OOP | ✅ | `must implement` |
| `needs` | Contextual | Declaration | ✅ | `action needs` |
| `new` | Contextual | Declaration | ❌ | `store new constant MAX as 10` (so `store new as ...` is an error) |
| `no` | Literal | Values | ❌ | Boolean false |
| `not` | Structural | Comparison | ❌ | `not equal` |
| `nothing` | Literal | Values | ❌ | Null value |
| `of` | Other | Comparison | ❌ | `typeof of` |
| `on` | Structural | OOP | ❌ | `on event` |
| `one` | Other | Pattern | ❌ | `one or more` |
| `open` | Other | File I/O | ❌ | `open file` |
| `optional` | Other | Pattern | ❌ | `optional match` |
| `or` | Structural | Comparison | ❌ | `x or y` |
| `otherwise` | Structural | Control Flow | ❌ | `otherwise:` / `otherwise check if` |
| `output` | Other | Process | ❌ | `wait for read output from process child as text` |
| `parameters` | Other | Declaration | ❌ | `define action called f with parameters x:` |
| `parent` | Contextual | OOP | ✅ | `parent reference` |
| `pattern` | Contextual | Pattern | ✅ | `define pattern` |
| `permission` | Other | File I/O | ❌ | `file permission` |
| `plus` | Other | Operations | ❌ | `plus 5` |
| `port` | Other | Web/Network | ❌ | `listen on port` |
| `private` | Structural | OOP | ❌ | `private property` |
| `process` | Other | Process | ❌ | `spawn process` |
| `property` | Structural | OOP | ❌ | `container property` |
| `public` | Structural | OOP | ❌ | `public property` |
| `push` | Structural | Operations | ❌ | `push with list and value` |
| `read` | Contextual | File I/O | ✅ | `read file` |
| `recursively` | Other | File I/O | ❌ | `delete recursively` |
| `register` | Other | Web/Network | ❌ | `register handler` |
| `remove` | Other | Operations | ❌ | `remove item` |
| `repeat` | Structural | Control Flow | ❌ | `repeat 10 times` |
| `replace` | Other | Pattern | ❌ | `replace <pattern or text> with <text> in <text>` |
| `request` | Other | Web/Network | ❌ | `HTTP request` |
| `requires` | Structural | OOP | ❌ | `requires action` |
| `respond` | Other | Web/Network | ❌ | `respond to request` |
| `response` | Other | Web/Network | ❌ | `HTTP response` |
| `return` | Structural | Operations | ❌ | `return value` |
| `reversed` | Contextual | Control Flow | ✅ | `for each item in reversed items` |
| `route` | Structural | Control Flow | ❌ | `route path:` |
| `running` | Other | Process | ❌ | `process running` |
| `same` | Other | Comparison | ❌ | `is same as` |
| `script` | Other | Pattern | ❌ | `unicode script` |
| `server` | Other | Web/Network | ❌ | `web server` |
| `setup` | Structural | Testing | ❌ | `setup:` inside `describe` |
| `shell` | Other | Process | ❌ | `shell command` |
| `signal` | Other | Web/Network | ❌ | `server signal` |
| `skip` | Structural | Control Flow | ❌ | `skip iteration` |
| `spawn` | Other | Process | ❌ | `spawn process` |
| `split` | Other | Pattern | ❌ | `split by pattern` |
| `start` | Other | Pattern / Web | ❌ | `start of text` (pattern anchor) · `start streaming response to req ...` |
| `starts` | Contextual | Comparison | ✅ | `path starts with "/api"` |
| `static` | Structural | OOP | ❌ | `static property` |
| `status` | Other | Web/Network | ❌ | `response status` |
| `stop` | Other | Web/Network | ❌ | `stop server` |
| `store` | Structural | Declaration | ❌ | `store variable` |
| `subtract` | Other | Operations | ❌ | `subtract 5` |
| `teardown` | Structural | Testing | ❌ | `teardown:` inside `describe` |
| `test` | Structural | Testing | ❌ | `test "adds numbers":` |
| `text` | Contextual | Data & Types | ✅ | `text type` |
| `than` | Structural | Comparison | ❌ | `greater than` |
| `that` | Contextual | Web/Network | ✅ | `wait for request that comes in on server as req` |
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

**Total: 198 keywords and literals**

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
