# REPL Guide

The WFL REPL (Read-Eval-Print Loop) lets you experiment with code interactively. It's perfect for learning, testing ideas, and trying out syntax.

## What is a REPL?

A REPL is an interactive programming environment:

1. **Read** - You type a command
2. **Eval** - WFL executes it
3. **Print** - You see the result immediately
4. **Loop** - Repeat

Think of it as a conversation with WFL: you ask questions, WFL answers.

## Starting the REPL

Simply run `wfl` without a filename:

```bash
wfl
```

**You'll see:**
```
WFL REPL v26.1.17
Type 'exit' or press Ctrl+C to quit
>
```

The `>` prompt means WFL is ready for your input.

## Basic Usage

### Simple Expressions

Try typing expressions and pressing Enter:

```wfl
> display "Hello, REPL!"
Hello, REPL!

> 2 plus 3
5

> 10 times 5
50
```

WFL evaluates each line immediately and shows the result.

### Variables

Create and use variables:

```wfl
> store name as "Alice"

> display "Hello, " with name
Hello, Alice

> store age as 25

> age plus 5
30
```

**Variables persist** during your REPL session. You can use them in later commands.

### Type Checking

Check types interactively:

```wfl
> store x as 42
> typeof of x
Number

> store text as "Hello"
> typeof of text
Text

> store active as yes
> typeof of active
Boolean
```

## Multi-Line Code

For longer code blocks, WFL will wait for you to complete them:

```wfl
> check if 5 is greater than 3:
...     display "Math works!"
... end check
Math works!
```

Notice the `...` prompt? WFL knows you're not done yet. Type `end check` to complete the block.

### Loops in the REPL

```wfl
> count from 1 to 5:
...     display "Number: " with the current count
... end count
Number: 1
Number: 2
Number: 3
Number: 4
Number: 5
```

### Lists in the REPL

```wfl
> create list fruits:
...     add "apple"
...     add "banana"
...     add "orange"
... end list

> for each fruit in fruits:
...     display fruit
... end for
apple
banana
orange
```

## Quick Experiments

The REPL is perfect for trying things:

### Testing Comparisons

```wfl
> 5 is greater than 3
yes

> 10 is equal to 10
yes

> "hello" is "world"
no
```

### Testing Math

```wfl
> 100 divided by 4
25

> 7 modulo 3
1

> abs of -5
5

> round of 3.7
4
```

### Testing Text Operations

```wfl
> store message as "hello world"

> touppercase of message
HELLO WORLD

> length of message
11

> contains of message and "world"
yes

> substring of message from 0 length 5
hello
```

### Testing List Operations

```wfl
> store numbers as [1, 2, 3, 4, 5]

> length of numbers
5

> push to numbers with 6

> numbers
[1, 2, 3, 4, 5, 6]

> contains of numbers and 3
yes
```

## Learning with the REPL

### Explore Type Inference

```wfl
> store mystery as 42
> typeof of mystery
Number

> change mystery to "text"
> typeof of mystery
Text
```

### Test Conditions

```wfl
> store temperature as 25

> check if temperature is greater than 30:
...     display "Hot!"
... otherwise:
...     display "Nice!"
... end check
Nice!

> change temperature to 35
> check if temperature is greater than 30:
...     display "Hot!"
... otherwise:
...     display "Nice!"
... end check
Hot!
```

### Experiment with Patterns

```wfl
> create pattern email:
...     one or more letter or digit
...     followed by "@"
...     followed by one or more letter or digit
... end pattern

> "user@example.com" matches email
yes

> "invalid-email" matches email
no
```

## REPL Commands

### Display Variables

Just type the variable name:

```wfl
> store x as 100
> x
100

> store name as "Alice"
> name
Alice
```

### Clear Screen (if supported)

Some terminals support:
```wfl
> clear
```

### Exit the REPL

Multiple ways to exit:

1. **Type `exit`:**
   ```wfl
   > exit
   ```

2. **Type `quit`:**
   ```wfl
   > quit
   ```

3. **Press Ctrl+C**

4. **Press Ctrl+D** (Linux/macOS)

## Tips and Tricks

### 1. Test Before You Write Files

Try code in the REPL before adding it to your program:

```wfl
// Test this first:
> check if 10 is greater than 5:
...     display "Yes!"
... end check
Yes!

// Then add to your .wfl file once it works
```

### 2. Quick Calculations

Use the REPL as a calculator:

```wfl
> 1234 plus 5678
6912

> 100 times 1.08
108.0

> 500 divided by 4
125
```

### 3. Learn by Exploring

Don't know how something works? Try it:

```wfl
> // What does typeof return?
> typeof of 42
Number

> typeof of "hello"
Text

> typeof of [1, 2, 3]
List
```

### 4. Test Error Messages

See how WFL reports errors:

```wfl
> 5 plus "hello"
ERROR: Cannot add Number and Text

Type mismatch: expected Number, got Text
```

Error messages in the REPL help you understand what went wrong.

### 5. Prototype Functions

Test action (function) ideas:

```wfl
> action greet with name:
...     display "Hello, " with name with "!"
... end action

> call greet with "Alice"
Hello, Alice!

> call greet with "Bob"
Hello, Bob!
```

## Common REPL Workflows

### Workflow 1: Learning Syntax

**Goal:** Learn how `count` loops work

```wfl
> // Try basic count
> count from 1 to 3:
...     display the current count
... end count
1
2
3

> // Try with step
> count from 0 to 10 by 2:
...     display the current count
... end count
0
2
4
6
8
10

> // Try backwards
> count from 5 to 1:
...     display the current count
... end count
5
4
3
2
1
```

**Result:** Learned how `count` works through experimentation.

### Workflow 2: Debugging Logic

**Goal:** Figure out why a condition isn't working

```wfl
> store age as 17
> check if age is greater than or equal to 18:
...     display "Adult"
... otherwise:
...     display "Minor"
... end check
Minor

> // Ah, 17 is less than 18. That's why!
> change age to 18
> check if age is greater than or equal to 18:
...     display "Adult"
... otherwise:
...     display "Minor"
... end check
Adult
```

**Result:** Found the bug (off-by-one error).

### Workflow 3: Testing Libraries

**Goal:** Learn how `substring` works

```wfl
> store text as "Hello, World!"

> substring of text from 0 length 5
Hello

> substring of text from 7 length 5
World

> // What if I go past the end?
> substring of text from 0 length 100
Hello, World!

> // WFL handles it gracefully
```

**Result:** Learned `substring` behavior through experimentation.

## What the REPL is Good For

✅ **Learning WFL** - Try syntax without creating files

✅ **Quick calculations** - Use as a calculator

✅ **Testing functions** - Try standard library functions

✅ **Experimenting** - "What if I do this?"

✅ **Debugging** - Test problematic expressions

✅ **Prototyping** - Try ideas before committing to code

## What the REPL is Not For

❌ **Long programs** - Use `.wfl` files instead

❌ **Permanent code** - REPL sessions aren't saved

❌ **Complex projects** - Better suited for experimentation

❌ **File operations** - Testing file I/O is awkward in REPL

## REPL Limitations

The REPL has some limitations:

1. **No file saving** - Your session disappears when you exit
2. **No history** (yet) - Can't scroll through previous commands
3. **Limited editing** - Can't edit multi-line blocks easily
4. **No undo** - Can't undo variable assignments

**Solution:** Use the REPL for experiments, then move working code to `.wfl` files.

## From REPL to File

Once you've figured something out in the REPL, save it to a file:

**REPL session:**
```wfl
> action calculate area with width and height:
...     return width times height
... end action

> calculate area with 10 and 20
200

> // Works! Let me save this to a file.
```

**Create `area.wfl`:**
```wfl
action calculate area with width and height:
    return width times height
end action

store room area as calculate area with 10 and 20
display "Area: " with room area
```

**Run it:**
```bash
wfl area.wfl
```

## Practice Exercises

Try these in the REPL:

### Exercise 1: Variables and Math

```wfl
> store x as 10
> store y as 20
> store sum as x plus y
> display "Sum: " with sum
```

What do you get?

### Exercise 2: Type Exploration

```wfl
> store value as 42
> typeof of value
> change value to "text"
> typeof of value
> change value to yes
> typeof of value
```

What types did you see?

### Exercise 3: List Building

```wfl
> create list items
> end list
> push to items with "first"
> push to items with "second"
> push to items with "third"
> items
```

What does the list contain?

### Exercise 4: Conditionals

Create a variable `score` and write conditions for:
- Score >= 90: "A"
- Score >= 80: "B"
- Score >= 70: "C"
- Below 70: "F"

Test with different scores!

### Exercise 5: String Manipulation

```wfl
> store message as "  hello world  "
> trim of message
> touppercase of trim of message
```

What's the result?

## Advanced REPL Usage

### Chaining Operations

```wfl
> store numbers as [5, 10, 15, 20]
> length of numbers
4

> store text as "hello"
> touppercase of text
HELLO

> round of 3.7
4
```

### Testing Pattern Matching

```wfl
> create pattern phone:
...     digit digit digit
...     followed by "-"
...     digit digit digit
...     followed by "-"
...     digit digit digit digit
... end pattern

> "555-123-4567" matches phone
yes

> "invalid" matches phone
no
```

## Conclusion

The REPL is your **experimentation playground**:

- **Learn** by trying things
- **Test** ideas before writing files
- **Debug** expressions interactively
- **Explore** WFL's features

Don't be afraid to experiment! The REPL is safe—you can't break anything. The worst that happens is an error message that teaches you something.

## Next Steps

Now that you know the REPL, explore:

**[Editor Setup →](editor-setup.md)**
- Get syntax highlighting
- Real-time error checking
- Auto-completion

**[Language Basics →](../03-language-basics/index.md)**
- Variables and types (in depth)
- All operators and expressions
- Functions and error handling

**[Standard Library →](../05-standard-library/index.md)**
- All built-in functions to try in the REPL

---

**Pro Tip:** Keep the REPL open while you're learning. Whenever you wonder "What does X do?", try it in the REPL!

---

**Previous:** [← Your First Program](your-first-program.md) | **Next:** [Editor Setup →](editor-setup.md)
