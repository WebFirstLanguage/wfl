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
