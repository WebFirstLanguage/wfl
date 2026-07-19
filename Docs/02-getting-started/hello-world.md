# Hello, World!

The traditional first program in any language. Let's write yours in WFL!

## Your First WFL Program

Create a file called `hello.wfl` in any text editor:

```wfl
display "Hello, World!"
```

That's it. One line. No imports, no setup, no ceremony.

## Run Your Program

Open your terminal and run:

```bash
wfl hello.wfl
```

**Output:**
```
Hello, World!
```

**Congratulations!** 🎉 You just wrote and ran your first WFL program.

## What Just Happened?

Let's break down that one line:

```wfl
display "Hello, World!"
```

- **`display`** - A WFL command that outputs text to the screen
- **`"Hello, World!"`** - The text to display (in quotes)

### No Semicolons

Notice what's **missing**:
- ❌ No semicolon at the end
- ❌ No `console.log()` or `print()`
- ❌ No imports or `#include`
- ❌ No `main()` function
- ❌ No curly braces

WFL is designed to be **simple and clear**. Just write what you want to do in natural language.

## Comparison with Other Languages

### JavaScript
```javascript
console.log("Hello, World!");
```

### Python
```python
print("Hello, World!")
```

### WFL
```wfl
display "Hello, World!"
```

All three do the same thing, but WFL uses the most natural phrasing: `display`.

## Make It Your Own

Try changing the message:

```wfl
display "Welcome to WFL!"
```

Run it again:
```bash
wfl hello.wfl
```

**Output:**
```
Welcome to WFL!
```

## Display Multiple Things

You can display multiple messages:

```wfl
display "Hello, World!"
display "I'm learning WFL!"
display "This is fun!"
```

**Output:**
```
Hello, World!
I'm learning WFL!
This is fun!
```

Each `display` command outputs one line.

## Display with Variables

Let's add a variable:

```wfl
store name as "Alice"
display "Hello, " with name with "!"
```

**Output:**
```
Hello, Alice!
```

Breaking it down:
- **`store name as "Alice"`** - Create a variable called `name` with value "Alice"
- **`display "Hello, " with name with "!"`** - Display text, the variable, and more text
- **`with`** - Joins text and variables together

Try changing the name:
```wfl
store name as "Bob"
display "Hello, " with name with "!"
```

**Output:**
```
Hello, Bob!
```

## Display Several Values at Once

You don't have to write `with` between every piece. A `display` can list
several values separated by spaces — quoted text is shown as-is, and each other
item is evaluated first: a variable, a number, an action call, or an expression
like `age plus 10`:

```wfl
store name as "Alice"
display "Hello, " name "!"
```

**Output:**
```
Hello, Alice!
```

This is just a shorthand: `display "Hello, " name "!"` means exactly the same
thing as `display "Hello, " with name with "!"` — not just the same result,
but the same order of evaluation, so a value that changes as a side effect of
a later item (e.g. popping from a list) behaves identically either way.

Because the values are joined directly (no space is added for you), put any
spaces you want inside the quotes:

```wfl
store age as 25
display "I am " age " years old"   // I am 25 years old
display "I am" age "years old"     // I am25years old  ← note the missing spaces
```

> **Tip:** `with` and the space-separated form do the same job — use whichever
> reads more clearly. Just pick *one* form within a single `display`: mixing
> them in the same statement (like `display a with b c`) can group the values
> differently and change the order they're evaluated, so a run of pure `with`
> or pure spaces stays predictable while a mix may not.

A run of plain words with nothing between them (no quotes, numbers, or
keywords) is a single multi-word variable name, not several values — `display
a b c` looks for one variable literally named `a b c`, the same as it would
outside a `display`. Space-separated values only split apart where the grammar
already has a boundary: a quote, a number, a parenthesis, or one of the
keywords that begins a value on its own (such as `not`, `file exists`, or an
action `call`).

Two kinds of tokens do *not* start a new value, for different reasons:

- Words joined by an operator like `plus` or `minus` stay part of the *same*
  value. `display numbers 0` stays a single value — a direct index into
  `numbers` — and `display total -5` stays a single value — `total` *minus*
  `5` — because both `0` and `-5` attach to the item right before them the
  same way they would anywhere else in WFL.
- A keyword that starts a *different kind of statement* — `count from ...`, a
  loop, `create ...`, `change ... to ...`, and a few others — ends the
  `display` right where it is, exactly as a line break would, and begins its
  own statement immediately after. `display "start" count from 1 to 3:` still
  displays `start` and then opens a count loop, just as it did before
  `display` accepted multiple values.

When you want two values that would otherwise merge like the first case, use
`with` to make the boundary explicit.

## Experiment!

WFL is designed for experimentation. Try these:

### Example 1: Math
```wfl
display 2 plus 3
```

**Output:** `5`

### Example 2: Multiple Variables
```wfl
store first name as "Alice"
store last name as "Smith"
display first name with " " with last name
```

**Output:** `Alice Smith`

### Example 3: Numbers and Text
```wfl
store age as 25
display "I am " with age with " years old"
```

**Output:** `I am 25 years old`

## Common Questions

**Q: Do I need quotes around everything?**
A: Only around text (strings). Numbers don't need quotes:
```wfl
display "Hello"    // Text needs quotes
display 42         // Numbers don't
```

**Q: What if I make a mistake?**
A: WFL will tell you. A missing opening quote is a common slip:

```wfl
// This is intentionally wrong — missing the opening quote:
// display Hello, World!"
// Fix it like this:
display "Hello, World!"
```

You'll get a helpful error message pointing at the problem when the opening quote is missing.

**Q: Can I add comments?**
A: Yes! Use `//` for single-line comments:
```wfl
// This is a comment
display "Hello, World!"  // Comments can go here too
```

**Q: What else can I display?**
A: Anything! Text, numbers, math results, variables—if it has a value, you can display it.

## Try the REPL

Want to experiment without creating files? Use the WFL REPL (Read-Eval-Print Loop):

```bash
wfl
```

This starts an interactive session:
```
WFL REPL v26.1.17
> display "Hello!"
Hello!
> store x as 10
> display x plus 5
15
>
```

Type commands and see results immediately. Perfect for learning!

Exit the REPL with `Ctrl+C` or type `exit`.

**[Learn more about the REPL →](repl-guide.md)**

## What You've Learned

In this first program, you've learned:

✅ **How to create a WFL file** - Save text with `.wfl` extension
✅ **How to run WFL programs** - `wfl filename.wfl`
✅ **The `display` command** - Output text to the screen
✅ **Variables** - `store name as value`
✅ **Joining text** - Use `with` to combine things
✅ **Comments** - Use `//` for notes in your code

## Success!

You've written and run your first WFL program. That wasn't hard, was it?

WFL is designed to be this simple. As you learn more, you'll discover that even complex programs maintain this natural, readable style.

## Next Steps

Ready to build something more interesting? Let's create your first real program:

**[Your First Program →](your-first-program.md)**

Or explore other options:
- **[REPL Guide](repl-guide.md)** - Experiment interactively
- **[Language Basics](../03-language-basics/index.md)** - Learn more WFL syntax
- **[Editor Setup](editor-setup.md)** - Get syntax highlighting and auto-completion

---

**Keep experimenting!** The best way to learn is by trying things. WFL's error messages are friendly and helpful, so don't be afraid to make mistakes.

---

**Previous:** [← Installation](installation.md) | **Next:** [Your First Program →](your-first-program.md)
