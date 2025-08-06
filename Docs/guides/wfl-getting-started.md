# Getting Started with WFL

Welcome to the WebFirst Language (WFL)! This guide will help you take your first steps with WFL, a programming language designed to read like plain English and make coding more intuitive and accessible.

## What is WFL?

WFL is a programming language that uses natural English-like syntax instead of cryptic symbols. Instead of writing code that looks like mathematical formulas, you write instructions that read like sentences:

```wfl
store greeting as "Hello, World!"
display greeting

check if 5 is greater than 3:
    display "Math works!"
end check
```

## Prerequisites

Before you start, you'll need:

- **Rust Programming Environment**: WFL is built with Rust, so you'll need Rust and Cargo installed
  - Install from [rustup.rs](https://rustup.rs/)
  - Verify installation: `cargo --version`
- **Text Editor**: Any text editor works, but VS Code with the WFL extension provides the best experience
- **Terminal/Command Prompt**: For running WFL programs

## Installation

1. **Clone the WFL Repository**:
   ```bash
   git clone https://github.com/wfl-project/wfl.git
   cd wfl
   ```

2. **Build WFL**:
   ```bash
   cargo build --release
   ```

3. **Verify Installation**:
   ```bash
   cargo run -- --version
   ```

## Your First WFL Program

Let's create your first WFL program! 

1. **Create a new file** called `my_first_program.wfl`

2. **Add this code**:
   ```wfl
   // My first WFL program
   store my name as "Alice"
   display "Welcome to WFL, " with my name with "!"
   ```

3. **Run your program**:
   ```bash
   cargo run -- my_first_program.wfl
   ```

4. **You should see**:
   ```
   Welcome to WFL, Alice!
   ```

Congratulations! You've just written and run your first WFL program.

## Understanding Your First Program

Let's break down what happened:

```wfl
// My first WFL program
```
This is a **comment** - text that explains your code but doesn't run. Comments start with `//`.

```wfl
store my name as "Alice"
```
This creates a **variable** called `my name` and stores the text `"Alice"` in it. Notice:
- Variable names can have spaces (like `my name`)
- Text is surrounded by quotes
- We use natural words like `store` and `as`

```wfl
display "Welcome to WFL, " with my name with "!"
```
This **displays** text to the screen. The `with` keyword joins text together, so this combines:
- `"Welcome to WFL, "`
- The value in `my name` (`"Alice"`)
- `"!"`

## Basic WFL Concepts

### Variables and Data Types

WFL automatically figures out what type of data you're storing:

```wfl
store user age as 25              // Number
store user name as "Bob"          // Text
store is online as yes            // Boolean (yes/no or true/false)
store empty value as nothing      // Null/empty value
```

### Displaying Output

```wfl
display "Simple text"
display "Your age is " with user age
print "This also works"           // Alternative to display
```

### Basic Math

```wfl
store first number as 10
store second number as 5

store sum as first number plus second number
store difference as first number minus second number
store product as first number times second number
store quotient as first number divided by second number

display "Sum: " with sum                    // Shows: Sum: 15
display "Difference: " with difference      // Shows: Difference: 5
```

### Making Decisions

```wfl
store temperature as 75

check if temperature is greater than 70:
    display "It's warm outside!"
otherwise:
    display "It's cool outside!"
end check
```

### Repeating Actions (Loops)

```wfl
// Count from 1 to 5
count from 1 to 5:
    display "Count: " with count
end count

// Loop through a list
store fruits as ["apple", "banana", "orange"]
for each fruit in fruits:
    display "I like " with fruit
end for
```

## Working with Lists

Lists store multiple values:

```wfl
// Create a list
store shopping list as ["milk", "bread", "eggs"]

// Add to the list
add "cheese" to shopping list

// Display each item
for each item in shopping list:
    display "Buy: " with item
end for

// Get list length
display "Total items: " with length of shopping list
```

## Working with Functions (Actions)

Functions in WFL are called "actions":

```wfl
define action called greet user:
    parameter user name as Text
    display "Hello, " with user name with "!"
end action

// Use the action
greet user with "Sarah"
```

## Common Development Commands

### Running Programs
```bash
# Run a WFL program
cargo run -- my_program.wfl

# Run with debug information
cargo run -- --debug my_program.wfl

# Run all test programs (to verify WFL is working)
cargo run -- TestPrograms/hello.wfl
```

### Code Quality Tools
```bash
# Check your code style
cargo run -- --lint my_program.wfl

# Auto-format your code
cargo run -- --fix my_program.wfl --in-place

# Analyze your code for issues
cargo run -- --analyze my_program.wfl
```

### Building and Testing WFL Itself
```bash
# Format the WFL source code
cargo fmt --all

# Build WFL
cargo build

# Run WFL's internal tests
cargo test

# Build optimized version
cargo build --release
```

## Example Programs to Try

### 1. Personal Greeting
```wfl
store first name as "John"
store last name as "Doe"
store full name as first name with " " with last name

display "Hello, " with full name with "!"
display "Your name has " with length of full name with " characters."
```

### 2. Simple Calculator
```wfl
store number a as 15
store number b as 4

display number a with " + " with number b with " = " with (number a plus number b)
display number a with " - " with number b with " = " with (number a minus number b)
display number a with " × " with number b with " = " with (number a times number b)
display number a with " ÷ " with number b with " = " with (number a divided by number b)
```

### 3. Grade Checker
```wfl
store score as 85

check if score is greater than or equal to 90:
    display "Grade: A"
check if score is greater than or equal to 80:
    display "Grade: B"
check if score is greater than or equal to 70:
    display "Grade: C"
check if score is greater than or equal to 60:
    display "Grade: D"
otherwise:
    display "Grade: F"
end check
```

## Next Steps

Now that you've mastered the basics:

1. **Explore More Examples**: Look at the programs in the `TestPrograms/` folder
2. **Read the Language Reference**: Check out the detailed documentation in `Docs/language-reference/`
3. **Try the WFL by Example Guide**: For a more comprehensive learning path
4. **Use the Cookbook**: For solutions to common programming tasks

## Getting Help

- **Documentation**: All guides are in the `Docs/` folder
- **Examples**: The `TestPrograms/` folder has many working examples
- **Error Messages**: WFL provides helpful error messages - read them carefully!
- **Community**: Join the WFL community for questions and discussions

## Troubleshooting

### Common Issues

**"cargo: command not found"**
- Install Rust from [rustup.rs](https://rustup.rs/)

**"No such file or directory"**
- Make sure you're in the WFL project directory
- Check that your `.wfl` file exists and the path is correct

**Parse errors in your code**
- Check for missing `end` statements
- Verify quotes are properly closed
- Make sure variable names don't use reserved words

**Program doesn't produce expected output**
- Use `cargo run -- --debug my_program.wfl` to see detailed execution information
- Check the WFL syntax guide for proper formatting

Remember: WFL is designed to be intuitive, so if something feels awkward, there's probably a more natural way to write it. Check the documentation or examples for better approaches!

Welcome to the WFL community! Happy coding! 🚀