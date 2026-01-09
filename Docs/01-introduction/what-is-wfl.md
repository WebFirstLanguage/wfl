# What is WFL?

**WFL (WebFirst Language)** is a programming language designed to make coding intuitive and accessible by using natural English-like syntax instead of cryptic symbols and abstract conventions.

## The Core Idea

Traditional programming languages look like this:

```javascript
// JavaScript
const userName = "Alice";
const userAge = 28;
if (userAge > 18) {
    console.log("You can vote!");
}
```

WFL looks like this:

```wfl
store user name as "Alice"
store user age as 28
check if user age is greater than 18:
    display "You can vote!"
end check
```

**Notice the difference?** WFL code reads like plain English. You can understand what it does even if you've never programmed before.

## Mission Statement

> WFL's mission is to **revolutionize web development** by making it intuitive, accessible, and aligned with human communication. By leveraging natural-language patterns and minimizing special characters, WFL bridges the gap between how people think and how code is written.

WFL empowers developers of all experience levels—beginners and experts alike—to create clear, readable, and maintainable web applications.

## What Makes WFL Different?

### 1. Natural Language Syntax

WFL uses words you already know:

```wfl
// Variables
store name as "Developer"
change name to "Expert Developer"

// Conditionals
check if temperature is greater than 30:
    display "It's hot!"
otherwise:
    display "It's comfortable"
end check

// Loops
count from 1 to 10:
    display "Number: " with the current count
end count

for each item in shopping list:
    display "Need to buy: " with item
end for
```

No semicolons. No curly braces. No confusing symbols. Just natural English phrases.

### 2. Type Safety with Inference

WFL knows what type your data is:

```wfl
store age as 25              // WFL knows this is a number
store name as "Alice"         // WFL knows this is text
store is active as yes        // WFL knows this is a boolean

display typeof of age         // Output: "Number"
display typeof of name        // Output: "Text"
display typeof of is active   // Output: "Boolean"
```

The compiler catches type errors before your code runs, preventing common bugs.

### 3. Modern Features, Natural Syntax

WFL includes powerful features with readable syntax:

**Web Servers:**
```wfl
listen on port 8080 as server

wait for request comes in on server as req
respond to req with "Hello from WFL!"
```

**Async Operations:**
```wfl
wait for file operation completes
display "File saved successfully!"
```

**Pattern Matching:**
```wfl
create pattern email:
    one or more letter or digit
    followed by "@"
    followed by one or more letter or digit
end pattern
```

## Current Status

WFL is currently in **alpha development** (version 26.1.17 as of January 2026). This means:

- ✅ Core language features are implemented and working
- ✅ Standard library with 180+ functions
- ✅ Web server support, file I/O, pattern matching
- ✅ IDE integration via Language Server Protocol
- ⚠️ **Not recommended for production use** (yet)
- 🔄 Active development and improvements

### Version Scheme

WFL uses calendar-based versioning: **YY.MM.BUILD**
- Example: `26.1.17` = Year 2026, January, Build 17
- Easy to understand when releases happen
- Predictable monthly release cycles

## Who Created WFL?

WFL is developed by **Logbie LLC** with assistance from AI development partners:
- **Devin.ai** - Primary development
- **ChatGPT** - Code review and optimization
- **Claude** - Documentation and architecture

The project is open source under the Apache 2.0 license.

## Target Audience

### Beginners
If you're new to programming, WFL is designed for you. The natural language syntax means you can start writing code immediately without memorizing complex syntax rules.

**First program:**
```wfl
display "Hello, World!"
```

That's it! You're programming.

### Experienced Developers
If you're already a programmer, WFL offers:
- **Readability** - Code reviews are easier when code reads like English
- **Maintainability** - Come back to your code months later and understand it instantly
- **Team collaboration** - Junior developers can read senior developers' code
- **Productivity** - Less time deciphering syntax, more time solving problems

### Educators
Teaching programming? WFL eliminates the "syntax barrier" that frustrates beginners:
- Students focus on **concepts**, not semicolons
- Code is **self-documenting**
- Natural language reduces cognitive load
- Easier to explain logic when code reads like English

## What Can You Build with WFL?

WFL is designed for web development and automation:

✅ **Web Servers and APIs**
- HTTP servers with routing
- RESTful APIs
- Static file serving
- Middleware and authentication

✅ **Automation Scripts**
- File processing
- Data manipulation
- System administration tasks
- Batch operations

✅ **Command-Line Tools**
- Utilities and helpers
- Build scripts
- Development tools

✅ **Data Processing**
- Text manipulation
- List operations
- File I/O
- Pattern matching

❌ **Not (currently) suitable for:**
- Mobile apps
- Desktop GUI applications
- Real-time games
- Systems programming

## Philosophy in Action

WFL is built on 19 guiding principles (detailed in [Natural Language Philosophy](natural-language-philosophy.md)). Here's a glimpse:

**Principle 1: Natural Language Syntax**
```wfl
// Instead of: x += 10
add 10 to score

// Instead of: if (x > 5 && y < 10)
check if score is greater than 5 and attempts is less than 10:
```

**Principle 2: Minimize Special Characters**
```wfl
// No semicolons, no curly braces, no unnecessary symbols
store name as "Alice"
display "Hello, " with name
```

**Principle 3: Readability and Clarity**
```wfl
// Code that reads like documentation
for each customer in customer list:
    check if customer balance is greater than 0:
        send reminder to customer
    end check
end for
```

## The Backward Compatibility Promise

> **We guarantee that WFL code you write today will continue to work with all future versions of the language.**

We will not actively kill features unless a security bug forces our hand. If we must deprecate something, we will give you **at least 1 year notice**.

Your investment in learning WFL is protected.

## Next Steps

Now that you understand what WFL is, explore:

- **[Key Features](key-features.md)** - Detailed look at WFL's capabilities
- **[Natural Language Philosophy](natural-language-philosophy.md)** - The 19 principles behind WFL
- **[First Look](first-look.md)** - See more code examples
- **[Getting Started](../02-getting-started/index.md)** - Install WFL and write your first program

---

**Previous:** [← Introduction](index.md) | **Next:** [Key Features →](key-features.md)
