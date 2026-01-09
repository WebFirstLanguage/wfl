# Your First Program

Now that you've written "Hello, World!", let's build a real program step by step. You'll learn variables, conditionals, and loops—the building blocks of programming.

## What We'll Build

A simple greeter program that:
1. Stores your name
2. Checks your age
3. Gives you a personalized greeting
4. Counts to show you a pattern

**Estimated time:** 15 minutes

## Step 1: Variables

Create a new file called `greeter.wfl`:

```wfl
// Step 1: Store information
store name as "Alice"
store age as 25

display "Hello, " with name with "!"
display "You are " with age with " years old."
```

Run it:
```bash
wfl greeter.wfl
```

**Output:**
```
Hello, Alice!
You are 25 years old.
```

### What You Learned

- **`store <name> as <value>`** - Creates a variable
- **Variables can hold text** - Use quotes for text: `"Alice"`
- **Variables can hold numbers** - No quotes for numbers: `25`
- **`with`** - Joins text and variables together

### Try It Yourself

Change the name and age to your own:
```wfl
store name as "YourName"
store age as 30
```

## Step 2: Type Checking

WFL knows what type your data is. Add this to your file:

```wfl
// Step 2: Check types
display "Type of name: " with typeof of name
display "Type of age: " with typeof of age
```

Run it:
```bash
wfl greeter.wfl
```

**Output:**
```
Hello, Alice!
You are 25 years old.
Type of name: Text
Type of age: Number
```

### What You Learned

- **`typeof of <variable>`** - Returns the type of a variable
- **WFL has types** - Text, Number, Boolean, List, etc.
- **Type inference** - WFL figures out types automatically

## Step 3: Conditionals

Let's make decisions based on data. Add this:

```wfl
// Step 3: Make decisions
check if age is greater than or equal to 18:
    display "You are an adult."
otherwise:
    display "You are a minor."
end check
```

Run it again.

**Output (with age 25):**
```
You are an adult.
```

Try changing the age to 15 and run it again:
```wfl
store age as 15
```

**Output:**
```
You are a minor.
```

### What You Learned

- **`check if <condition>:`** - Start a conditional
- **`is greater than or equal to`** - Natural language comparison
- **`otherwise:`** - The "else" clause
- **`end check`** - End the conditional block

### Multiple Conditions

Let's add more logic:

```wfl
// Step 3b: Multiple conditions
check if age is greater than or equal to 65:
    display "You are a senior citizen."
otherwise:
    check if age is greater than or equal to 18:
        display "You are an adult."
    otherwise:
        check if age is greater than or equal to 13:
            display "You are a teenager."
        otherwise:
            display "You are a child."
        end check
    end check
end check
```

### What You Learned

- **Nested `otherwise: check if`** - Chain multiple conditions with nested blocks
- **Order matters** - Conditions are checked top to bottom

## Step 4: Loops

Let's add repetition. Add this to your file:

```wfl
// Step 4: Count from 1 to 5
display ""
display "Let me count to 5 for you:"

count from 1 to 5:
    display "Number: " with the current count
end count
```

Run it:

**Output:**
```
Let me count to 5 for you:
Number: 1
Number: 2
Number: 3
Number: 4
Number: 5
```

### What You Learned

- **`count from <start> to <end>:`** - Create a counting loop
- **`the current count`** - Special variable with the current number
- **`end count`** - End the loop block

### Custom Steps

You can count by different amounts:

```wfl
// Count by 2s
count from 0 to 10 by 2:
    display the current count
end count
```

**Output:**
```
0
2
4
6
8
10
```

### What You Learned

- **`by <step>`** - Change the counting increment

## Step 5: Lists

Let's work with multiple items:

```wfl
// Step 5: Create a list
display ""
display "My favorite colors:"

create list colors:
    add "red"
    add "green"
    add "blue"
end list

for each color in colors:
    display "  - " with color
end for
```

**Output:**
```
My favorite colors:
  - red
  - green
  - blue
```

### What You Learned

- **`create list <name>:`** - Start a list
- **`add <value>`** - Add item to the list
- **`for each <item> in <list>:`** - Loop through list items
- **`end for`** - End the loop

### Alternative List Syntax

You can also create lists with literal syntax:

```wfl
store colors as ["red", "green", "blue"]
```

Both ways work. Use whichever is clearer.

## Complete Program

Here's the complete `greeter.wfl`:

```wfl
// A complete WFL program
// This demonstrates variables, conditionals, loops, and lists

// Variables
store name as "Alice"
store age as 25

// Basic output
display "=== Personal Greeter ==="
display ""
display "Hello, " with name with "!"
display "You are " with age with " years old."
display ""

// Type information
display "Type of name: " with typeof of name
display "Type of age: " with typeof of age
display ""

// Conditional logic
check if age is greater than or equal to 65:
    display "Life status: Senior citizen"
otherwise:
    check if age is greater than or equal to 18:
        display "Life status: Adult"
    otherwise:
        check if age is greater than or equal to 13:
            display "Life status: Teenager"
        otherwise:
            display "Life status: Child"
        end check
    end check
end check
display ""

// Counting loop
display "Let me count to 5 for you:"
count from 1 to 5:
    display "  " with the current count
end count
display ""

// List iteration
display "My favorite colors:"
create list colors:
    add "red"
    add "green"
    add "blue"
end list

for each color in colors:
    display "  - " with color
end for
display ""

display "Thanks for using WFL!"
```

Run it:
```bash
wfl greeter.wfl
```

**Complete output:**
```
=== Personal Greeter ===

Hello, Alice!
You are 25 years old.

Type of name: Text
Type of age: Number

Life status: Adult

Let me count to 5 for you:
  1
  2
  3
  4
  5

My favorite colors:
  - red
  - green
  - blue

Thanks for using WFL!
```

## Exercises

Now it's your turn! Try these modifications:

### Exercise 1: Personalize It
Change the name and age to your own. Run the program.

### Exercise 2: Add More Colors
Add your favorite colors to the list:
```wfl
create list colors:
    add "red"
    add "green"
    add "blue"
    add "purple"    // Add this
    add "orange"    // And this
end list
```

### Exercise 3: Count Higher
Change the counting loop to count from 1 to 10.

### Exercise 4: Count Backwards
Make the loop count backwards from 10 to 1.

**Hint:** You'll need to figure out the right syntax. Try `count from 10 to 1:` or explore the REPL!

### Exercise 5: Add More Conditions
Add a condition for ages 100+:
```wfl
check if age is greater than or equal to 100:
    display "Congratulations on your long life!"
otherwise:
    check if age is greater than or equal to 65:
        // ... rest of the conditions
    end check
end check
```

### Exercise 6: Create Your Own List
Create a list of:
- Your favorite foods
- Places you want to visit
- Hobbies you enjoy

Then use `for each` to display them.

## What You've Learned

In this tutorial, you've learned:

✅ **Variables** - `store name as value`
✅ **Types** - Text, Number, and type inference
✅ **Conditionals** - `check if`, `otherwise`, `end check`
✅ **Comparison operators** - `is greater than`, `is equal to`
✅ **Counting loops** - `count from X to Y by Z`
✅ **Lists** - `create list`, `add`, `for each`
✅ **Type checking** - `typeof of variable`
✅ **Text concatenation** - Using `with`

These are the **fundamental building blocks** of programming. Everything else builds on these concepts.

## Understanding WFL's Natural Style

Notice how the code reads like instructions:

```wfl
// Like giving instructions to a person:
"Check if age is greater than 18, then display 'Adult', otherwise display 'Minor'"

// Becomes:
check if age is greater than 18:
    display "Adult"
otherwise:
    display "Minor"
end check
```

This natural style makes WFL:
- **Easy to learn** - Reads like English
- **Easy to remember** - Natural phrasing
- **Easy to review** - Others can understand your code
- **Self-documenting** - Code explains what it does

## Common Mistakes (and How to Fix Them)

### Forgetting `end`

**Wrong:**
```wfl
check if age is 25:
    display "You're 25"
// Missing: end check
```

**Right:**
```wfl
check if age is 25:
    display "You're 25"
end check
```

### Forgetting Quotes Around Text

**Wrong:**
```wfl
store name as Alice    // Error: Alice is not defined
```

**Right:**
```wfl
store name as "Alice"  // Text needs quotes
```

### Mixing Up `with` and `plus`

**Wrong:**
```wfl
display "Age: " plus age    // This might not work as expected
```

**Right:**
```wfl
display "Age: " with age    // Use 'with' to join text and numbers
```

**Also Right:**
```wfl
store sum as 5 plus 3       // Use 'plus' for math
```

## Next Steps

You've built your first real program! Here's what to explore next:

### Continue Learning

**[Language Basics →](../03-language-basics/index.md)**
- Variables and types (in depth)
- All comparison operators
- More loop types
- Functions (called "actions")
- Error handling

### Experiment Interactively

**[REPL Guide →](repl-guide.md)**
- Try code without creating files
- Experiment with syntax
- Test ideas quickly

### Improve Your Workflow

**[Editor Setup →](editor-setup.md)**
- Syntax highlighting
- Auto-completion
- Real-time error checking
- LSP integration

### Build Something

Try building:
- A simple calculator
- A todo list tracker
- A number guessing game
- A file renamer

The [Cookbook](../guides/cookbook.md) has recipes for common tasks.

## Congratulations!

You've written a complete WFL program from scratch. You understand variables, conditionals, loops, and lists—the foundation of all programming.

**Keep experimenting, keep building, and most importantly: have fun!**

---

**Previous:** [← Hello World](hello-world.md) | **Next:** [REPL Guide →](repl-guide.md)
