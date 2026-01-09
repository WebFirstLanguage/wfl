# Language Basics

Master the fundamental building blocks of WFL. This section covers everything you need to write effective WFL programs.

## What You'll Learn

In this section, you'll learn:

1. **[Variables and Types](variables-and-types.md)** - Storing and managing data
2. **[Operators and Expressions](operators-and-expressions.md)** - Combining values and operations
3. **[Control Flow](control-flow.md)** - Making decisions with conditionals
4. **[Loops and Iteration](loops-and-iteration.md)** - Repeating actions
5. **[Actions (Functions)](actions-functions.md)** - Organizing reusable code
6. **[Lists and Collections](lists-and-collections.md)** - Working with multiple values
7. **[Error Handling](error-handling.md)** - Dealing with problems gracefully
8. **[Comments and Documentation](comments-and-documentation.md)** - Writing clear, maintainable code

## Prerequisites

Before diving into Language Basics, you should have:

✅ **Installed WFL** - See [Installation](../02-getting-started/installation.md)
✅ **Written "Hello, World!"** - See [Hello World](../02-getting-started/hello-world.md)
✅ **Completed Your First Program** - See [Your First Program](../02-getting-started/your-first-program.md)

If you've done those, you're ready!

## Learning Approach

### For Beginners

**Read in order:**
1. Start with Variables and Types
2. Progress through each topic sequentially
3. Try the examples as you read
4. Complete the exercises at the end of each section

**Estimated time:** 3-4 hours to complete all topics

### For Experienced Developers

**Skim familiar concepts:**
- Variables and Types - Quick review
- Operators - Focus on natural language alternatives
- Control Flow - Note the `check if` syntax
- Actions - See how WFL handles functions
- Error Handling - Review WFL's approach

**Focus on what's different** from languages you know.

**Estimated time:** 1-2 hours

## Quick Reference

Already know WFL basics? Use these as quick lookups:

| Topic | Quick Example |
|-------|---------------|
| Variables | `store name as "Alice"` |
| Numbers | `store age as 25` |
| Booleans | `store active as yes` |
| Conditionals | `check if x is 5: ... end check` |
| Loops | `count from 1 to 10: ... end count` |
| For Each | `for each item in list: ... end for` |
| Actions | `action greet with name: ... end action` |
| Lists | `create list items: add "x" end list` |
| Error Handling | `try: ... when error: ... end try` |
| Comments | `// This is a comment` |

## What Makes WFL Different?

If you're coming from other languages, here's what makes WFL unique:

### Natural Language Syntax

**JavaScript:**
```javascript
if (age >= 18) {
    console.log("Adult");
}
```

**WFL:**
```wfl
check if age is greater than or equal to 18:
    display "Adult"
end check
```

### No Special Characters

**Python:**
```python
for i in range(1, 11):
    print(i)
```

**WFL:**
```wfl
count from 1 to 10:
    display the current count
end count
```

### Clear Variable Declarations

**JavaScript:**
```javascript
const name = "Alice";
let age = 25;
```

**WFL:**
```wfl
store name as "Alice"
store age as 25
```

Just one keyword: `store`. Simple and clear.

## Core Concepts

Every programming language has these building blocks. Here's how WFL implements them:

### Data Storage (Variables)

```wfl
store name as "Alice"
store age as 25
store is active as yes
```

**Learn more:** [Variables and Types →](variables-and-types.md)

### Decisions (Conditionals)

```wfl
check if temperature is greater than 30:
    display "Hot!"
otherwise:
    display "Comfortable"
end check
```

**Learn more:** [Control Flow →](control-flow.md)

### Repetition (Loops)

```wfl
count from 1 to 5:
    display the current count
end count

for each item in shopping list:
    display item
end for
```

**Learn more:** [Loops and Iteration →](loops-and-iteration.md)

### Reusable Code (Functions)

```wfl
action greet with name:
    display "Hello, " with name with "!"
end action

call greet with "Alice"
```

**Learn more:** [Actions (Functions) →](actions-functions.md)

### Collections (Lists)

```wfl
create list colors:
    add "red"
    add "green"
    add "blue"
end list

for each color in colors:
    display color
end for
```

**Learn more:** [Lists and Collections →](lists-and-collections.md)

### Error Handling

```wfl
try:
    open file at "data.txt" for reading as file
    store content as read content from file
    close file
when error:
    display "Could not read file"
end try
```

**Learn more:** [Error Handling →](error-handling.md)

## Practice Philosophy

**Learn by doing!** Each topic includes:

- ✅ Clear explanations
- ✅ Working code examples
- ✅ Common use cases
- ✅ Practice exercises
- ✅ Common mistakes and how to avoid them

**Don't just read—type the examples yourself.** Experimenting is the best way to learn.

## Using the REPL

As you learn, keep the REPL open for experimentation:

```bash
wfl
```

Try every example in the REPL before adding it to your programs. The REPL is your safe playground!

**[Review REPL Guide →](../02-getting-started/repl-guide.md)**

## Complete Example

Here's a program that uses all the basics you'll learn in this section:

```wfl
// Complete Language Basics Example

// Variables and types
store items needed as 5
store item name as "widgets"
store in stock as yes

// Display with type checking
display "Item: " with item name
display "Type of item name: " with typeof of item name

// Conditional logic
check if in stock is yes:
    display "We have " with items needed with " " with item name with " in stock"
otherwise:
    display "Sorry, " with item name with " is out of stock"
end check

// Loop example
display ""
display "Counting stock:"
count from 1 to items needed:
    display "  Item " with the current count
end count

// List example
display ""
display "Similar items:"
create list similar items:
    add "gadgets"
    add "doodads"
    add "thingamajigs"
end list

for each item in similar items:
    display "  - " with item
end for

// Action (function) example
action calculate total with quantity and price:
    return quantity times price
end action

store unit price as 10.50
store total as calculate total with items needed and unit price
display ""
display "Total cost: $" with total

// Error handling example
try:
    check if items needed is greater than 100:
        display "Warning: Large order!"
    end check
when error:
    display "Error calculating order"
end try
```

By the end of this section, you'll understand every line of this program!

## Ready to Learn?

Start with the foundation:

**[Variables and Types →](variables-and-types.md)**

Or jump to a specific topic:
- [Operators and Expressions →](operators-and-expressions.md)
- [Control Flow →](control-flow.md)
- [Loops and Iteration →](loops-and-iteration.md)
- [Actions (Functions) →](actions-functions.md)
- [Lists and Collections →](lists-and-collections.md)
- [Error Handling →](error-handling.md)
- [Comments and Documentation →](comments-and-documentation.md)

---

**Previous:** [← Resources](../02-getting-started/resources.md) | **Next:** [Variables and Types →](variables-and-types.md)
