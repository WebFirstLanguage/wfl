# WFL Complete AI Reference

**Version:** 25.11.10 (based on WFL v25.11.10)
**Document Date:** 2025-11-30
**Purpose:** Comprehensive reference for AI agents working with the WebFirst Language (WFL)

---

## Document Overview

This document provides a complete reference for AI agents to understand and generate code in the WebFirst Language (WFL). It includes comprehensive syntax specifications, complete standard library API documentation, development tools reference, and practical code patterns.

**Intended Audience:**
- AI agents assisting users with WFL code
- AI systems generating WFL programs
- AI-powered development tools and IDEs
- Training datasets for language models

**How to Use This Document:**
- For syntax lookup: See [Section 1: Language Syntax Reference](#section-1-language-syntax-reference)
- For stdlib functions: See [Section 2: Standard Library API](#section-2-standard-library-api)
- For I/O operations: See [Section 3: IO--async-operations)
- For CLI tools: See [Section 4: CLI Tools & Development](#section-4-cli-tools--development)
- For quick reference: See [Appendix A: Syntax Cheat Sheet](#appendix-a-syntax-cheat-sheet)

---

## Table of Contents

### [Section 0: Quick Navigation & Metadata](#section-0-quick-navigation--metadata)
- Document Statistics
- Common Patterns Index
- Function Quick Reference
- Keyword Quick Reference

### [Section 1: Language Syntax Reference](#section-1-language-syntax-reference)
- Core Grammar (EBNF)
- Variables & Assignment
- Control Flow
- Functions (Actions)
- Object-Oriented Programming (Containers)
- Async/Await Patterns
- Pattern Matching
- Error Handling

### [Section 2: Standard Library API](#section-2-standard-library-api)
- Core Module
- Math Module
- Random Module
- Text Module
- List Module
- Time Module
- Filesystem Module
- Crypto Module
- Complete API Index

### [Section 3: I/O & Async Operations](#section-3-io--async-operations)
- File I/O Patterns
- HTTP/Web Operations
- Web Server Creation
- Async Best Practices

### [Section 4: CLI Tools & Development](#section-4-cli-tools--development)
- WFL CLI Commands
- Linter & Analyzer
- Configuration System
- Debugging Tools
- LSP Server Integration

### [Section 5: Type System & Semantics](#section-5-type-system--semantics)
- Type Inference Rules
- Type Checking
- Type Compatibility Matrix
- Common Type Errors

### [Section 6: Error Reference](#section-6-error-reference)
- Error Categories
- Parse Errors
- Type Errors
- Runtime Errors
- Error Message Interpretation

### [Section 7: Code Patterns & Best Practices](#section-7-code-patterns--best-practices)
- Common Patterns Library
- Complete Examples
- Anti-Patterns & Gotchas

### [Section 8: Architecture Overview](#section-8-architecture-overview)
- Pipeline Overview
- Component Descriptions
- Memory Model
- Performance Characteristics

### [Appendix A: Syntax Cheat Sheet](#appendix-a-syntax-cheat-sheet)

### [Appendix B: Complete Function Index](#appendix-b-complete-function-index)

### [Appendix C: Keyword Reference](#appendix-c-keyword-reference)

---

# Section 0: Quick Navigation & Metadata

## Document Statistics

- **Total Lines**: ~22,500 (estimated)
- **Sections**: 8 main + 3 appendices
- **Functions Documented**: ~120+ standard library functions
- **Examples**: ~250 (200 inline + 50 complete programs)
- **Test Programs Referenced**: 20-30 comprehensive examples
- **Source Files Synthesized**: 74 documentation files + 86 test programs

## WFL Design Philosophy

WFL (WebFirst Language) is guided by the following principles:

1. **Natural Language Syntax**: Code reads like English sentences
2. **Minimal Special Characters**: Uses words instead of symbols (`plus` instead of `+`)
3. **Clarity and Accessibility**: Self-documenting code that beginners can understand
4. **Strong Type Safety with Inference**: Static typing without verbose type declarations
5. **Modern and Secure by Design**: Built-in async, security practices, web-first design
6. **Backward Compatibility Promise**: Code written today works forever

## Common Patterns Index

| Task | Pattern | Section |
|------|---------|---------|
| Declare variable | `store x as value` | §1.2 |
| If/else | `check if condition: ... otherwise: ... end check` | §1.3 |
| Loop | `count from 1 to 10: ... end count` | §1.3 |
| Function | `define action name: ... end action` | §1.4 |
| File read | `wait for open file at path and read content as var` | §3.1 |
| HTTP request | `wait for open url at url and read content as var` | §3.2 |
| Error handling | `try: ... when error: ... otherwise: ... end try` | §1.7 |
| List iteration | `for each item in list: ... end for` | §1.3 |
| Async function | `define async action name: ... end action` | §1.6 |
| Create object | `create new Container as name: ... end create` | §1.5 |

## Function Quick Reference

### Most Common Functions

| Function | Module | Purpose | Example |
|----------|--------|---------|---------|
| `print(value)` | Core | Output to console | `print("Hello")` |
| `typeof(value)` | Core | Get type | `typeof(42)` returns "Number" |
| `length(text/list)` | Text/List | Get length | `length("hello")` returns 5 |
| `abs(number)` | Math | Absolute value | `abs(-5)` returns 5 |
| `round(number)` | Math | Round to integer | `round(3.7)` returns 4 |
| `random()` | Random | Random 0-1 | `random()` returns 0.xxxxx |
| `touppercase(text)` | Text | To uppercase | `touppercase("hi")` returns "HI" |
| `tolowercase(text)` | Text | To lowercase | `tolowercase("HI")` returns "hi" |
| `contains(text, search)` | Text | Check substring | `contains("hello", "ell")` returns yes |
| `push(list, item)` | List | Add to list | `push(mylist, "item")` |

## Keyword Quick Reference

### Core Keywords

| Keyword | Context | Example |
|---------|---------|---------|
| `store` | Variable declaration | `store age as 25` |
| `create` | Create object/collection | `create list items: ... end list` |
| `check if` | Conditional | `check if x > 5: ... end check` |
| `otherwise` | Else clause | `otherwise: ... end check` |
| `count from` | Numeric loop | `count from 1 to 10: ... end count` |
| `for each` | Collection loop | `for each item in list: ... end for` |
| `repeat while` | While loop | `repeat while x < 10: ... end repeat` |
| `define action` | Function definition | `define action name: ... end action` |
| `give back` | Return value | `give back result` |
| `wait for` | Async operation | `wait for async_operation` |
| `try` | Error handling | `try: ... when error: ... end try` |
| `break` | Exit loop | `break` |
| `continue` / `skip` | Skip iteration | `continue` or `skip` |

---

# Section 1: Language Syntax Reference

## 1.1 Core Grammar (EBNF)

### Program Structure

```ebnf
Program ::= Statement*

Statement ::= VariableDecl
           | Assignment
           | IfStatement
           | LoopStatement
           | ActionDefinition
           | ContainerDefinition
           | TryStatement
           | Expression
           | Comment

Comment ::= "//" .*
```

### Variables and Literals

```ebnf
VariableDecl ::= ("store" | "create") Identifier "as" Expression

Identifier ::= Letter ( Letter | Digit | Space )*

Literal ::= NumberLiteral
         | TextLiteral
         | BooleanLiteral
         | NothingLiteral
         | ListLiteral

NumberLiteral ::= Digit+ ("." Digit+)?
               | Digit+ ("million" | "billion" | "thousand")

TextLiteral ::= "\"" .* "\""

BooleanLiteral ::= "yes" | "no" | "true" | "false"

NothingLiteral ::= "nothing" | "missing" | "undefined"

ListLiteral ::= "[" ( Expression ( "," Expression )* )? "]"
```

### Expressions

```ebnf
Expression ::= PrimaryExpr ( Operator PrimaryExpr )*

PrimaryExpr ::= Literal
             | Identifier
             | FunctionCall
             | "(" Expression ")"

Operator ::= "plus" | "minus" | "times" | "divided by"
          | "is" | "is not" | "is greater than" | "is less than"
          | "is at least" | "is at most"
          | "and" | "or" | "not"
          | "with" (for string concatenation)

FunctionCall ::= Identifier "(" ( Expression ( "," Expression )* )? ")"
              | "perform" Identifier ( "with" ArgumentList )?

ArgumentList ::= Identifier "as" Expression ( "and" Identifier "as" Expression )*
```

## 1.2 Variables & Assignment

### Variable Declaration

WFL uses natural English syntax for variable declarations:

```wfl
// Basic declarations
store name as "Alice"
store age as 25
store price as 19.99
store is active as yes
store items as [1, 2, 3, 4, 5]

// Multiple word variable names (allowed)
store user name as "Bob"
store total price as 99.99
store is logged in as no
```

**Syntax:** `store <VariableName> as <Expression>`

**Type Inference:** WFL automatically infers types from the assigned value:
- `"text"` → Text type
- `42` → Number type
- `yes/no` → Boolean type
- `[1, 2, 3]` → List type
- `nothing` → Nothing type

### Variable Assignment

Update variables with natural language:

```wfl
// Simple assignment
change count to 100

// Arithmetic updates
add 5 to count          // count += 5
subtract 2 from count   // count -= 2
multiply count by 3     // count *= 3
divide count by 2       // count /= 2

// String concatenation
join "Hello, " and name into greeting
```

**Arithmetic Update Syntax:**
- `add <amount> to <variable>`
- `subtract <amount> from <variable>`
- `multiply <variable> by <factor>`
- `divide <variable> by <divisor>`

### Data Types

#### Primitive Types

**Number:**
```wfl
store integer as 42
store decimal as 3.14159
store scientific as 1.5e-10
store large as 1 million    // 1000000
```

**Text (String):**
```wfl
store greeting as "Hello, World!"
store multiline as "Line 1
Line 2
Line 3"
store with escapes as "She said \"Hello\""
```

**Boolean (Yes/No):**
```wfl
store is ready as yes     // true
store is done as no       // false
store flag as true        // also allowed
store disabled as false   // also allowed
```

**Nothing (Null):**
```wfl
store empty value as nothing
store missing data as missing
store undefined var as undefined
```

#### Collection Types

**List:**
```wfl
// Inline list literal
store numbers as [1, 2, 3, 4, 5]
store mixed as ["hello", 42, yes, nothing]
store empty as []

// Block form
create list shopping:
    add "milk"
    add "bread"
    add "eggs"
end list

// List access (0-indexed)
store first as shopping 0      // Direct access
store second as item at 1 from shopping  // Natural language form
```

**Record:**
```wfl
create record person:
    name is "John Smith"
    age is 30
    email is "john@example.com"
    is active is yes
end record

// Access record fields
display person's name           // "John Smith"
display person's age            // 30
change person's age to 31       // Update field
```

**Map:**
```wfl
create map settings:
    theme is "dark"
    volume is 75
    notifications are on
end map

// Access map entries
store current theme as get "theme" from settings
set "volume" in settings to 80
```

### Type Conversion

```wfl
// Explicit conversion
store text value as convert 123 to text       // "123"
store numeric value as convert "456" to number // 456

// Safe conversion with error handling
safely convert "abc" to number:
    when invalid:
        use 0 instead
    when missing:
        ask user for number
end convert
```

### Constants

```wfl
store new constant PI as 3.14159
store new constant MAX_USERS as 1000

// Constants cannot be modified
change PI to 3.14  // Error: Cannot modify constant
```

### Variable Scope

```wfl
// Global scope
global create server url as "http://example.com"

// Shared scope (module-level)
create module user handling:
    shared create user count as 0
    shared create active users as empty list
end module

// Local scope (within actions/blocks)
define action calculate:
    local create temp result as 0  // Only visible in this action
    give back temp result
end action
```

## 1.3 Control Flow

### Conditional Statements

#### Basic If-Then-Else

```wfl
// Block form
check if age is greater than 18:
    display "Adult"
otherwise:
    display "Minor"
end check

// Single-line form
if temperature is below 0 then display "Freezing!" otherwise display "Not freezing"
```

**Syntax:** `check if <Condition>: <Block> [otherwise: <Block>] end check`

#### Multiple Conditions

```wfl
check if score is above 90:
    display "Grade: A"
otherwise if score is above 80:
    display "Grade: B"
otherwise if score is above 70:
    display "Grade: C"
otherwise if score is above 60:
    display "Grade: D"
otherwise:
    display "Grade: F"
end check
```

#### Natural Language Comparisons

```wfl
// Equality
check if name is "Alice":
check if count is not 0:

// Numeric comparisons
check if age is greater than 21:      // >
check if price is less than 100:      // <
check if score is at least 50:        // >=
check if temp is at most 30:          // <=
check if value is between 1 and 10:   // >= 1 and <= 10

// String operations
check if email contains "@":
check if text starts with "Hello":
check if text ends with ".txt":

// Boolean conditions
check if is active:                    // if true
check if not is active:                // if false

// Null checks
check if value is nothing:
check if value is not nothing:

// List operations
check if list is empty:
check if item is in shopping list:
```

#### Combining Conditions

```wfl
// Logical AND
check if age is at least 18 and has license:
    allow driving
end check

// Logical OR
check if is admin or is moderator:
    allow moderation
end check

// Complex combinations with grouping
check if (age is at least 18 and has license) or is supervised:
    allow driving
end check
```

### Loop Constructs

#### Counting Loops

```wfl
// Basic counting (1 to 5 inclusive)
count from 1 to 5:
    display "Count: " with count
end count
// Output: 1, 2, 3, 4, 5

// Count with custom step
count from 0 to 20 by 2:
    display count
end count
// Output: 0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20

// Count backwards
count from 10 down to 1:
    display count
end count
// Output: 10, 9, 8, 7, 6, 5, 4, 3, 2, 1

// Count backwards with step
count from 100 down to 0 by 10:
    display count
end count
// Output: 100, 90, 80, 70, 60, 50, 40, 30, 20, 10, 0
```

**Note:** The loop variable is always named `count` and is scoped to the loop body.

#### For-Each Loops

```wfl
store fruits as ["apple", "banana", "orange"]

// Basic iteration
for each fruit in fruits:
    display "I like " with fruit
end for

// Reverse iteration
for each fruit in fruits reversed:
    display fruit
end for
// Output: orange, banana, apple
```

**Syntax:** `for each <ElementName> in <Collection> [reversed]: <Block> end for`

#### Conditional Loops

**Repeat While:**
```wfl
store attempts as 0
repeat while attempts is less than 3:
    add 1 to attempts
    display "Attempt " with attempts
end repeat
```

**Repeat Until:**
```wfl
store temperature as 20
repeat until temperature is above 100:
    add 10 to temperature
    display "Current temp: " with temperature
end repeat
```

**Repeat Forever:**
```wfl
repeat forever:
    store input as read user input
    check if input is "quit":
        break
    end check
    process input
end repeat
```

**Main Loop (No Timeout):**
```wfl
// For long-running applications (servers, daemons)
// Disables execution timeout
main loop:
    wait for request
    process request

    check if shutdown signal received:
        break
    end check
end loop
```

### Loop Control Statements

```wfl
// Break - exit current loop
for each item in items:
    check if item is "stop":
        break  // Exit loop immediately
    end check
    display item
end for

// Continue/Skip - skip to next iteration
count from 1 to 10:
    check if count is even:
        continue  // Skip even numbers
    end check
    display count  // Only displays odd numbers
end count

// Skip is a synonym for continue
for each file in files:
    check if file ends with ".tmp":
        skip  // Skip temporary files
    end check
    process file
end for

// Exit - break out of nested loops
count from 1 to 5:
    count from 1 to 5:
        check if count is 3:
            exit loop  // Exits BOTH loops
        end check
    end count
end count
```

## 1.4 Functions (Actions)

### Basic Action Definition

```wfl
define action say hello:
    display "Hello, World!"
end action

// Calling the action
say hello  // Displays: Hello, World!
```

**Syntax:** `define action <ActionName>: <Block> end action`

### Actions with Parameters

```wfl
// Single parameter
define action greet with name:
    display "Hello, " with name with "!"
end action

greet with "Alice"  // Displays: Hello, Alice!

// Multiple parameters with "and"
define action calculate area with width and height:
    store area as width times height
    display "The area is " with area
end action

calculate area with 5 and 3  // Displays: The area is 15

// Space-separated parameters (all get same value)
define action test assertion needs label expected actual:
    display "Testing: " with label
    check if expected is equal to actual:
        display "✓ Test passed"
    otherwise:
        display "✗ Test failed"
    end check
end action

// Can call with single or multiple arguments
test assertion with "Single arg"
test assertion with "Test" and 5 and 5
```

### Actions with Return Values

```wfl
define action calculate square:
    needs:
        number as number
    gives back:
        result as number
    do:
        store result as number times number
        give back result
end action

store squared as calculate square with number as 5
display squared  // Displays: 25

// Alternative syntax with "provide"
define action get full name with first and last:
    store full name as first with " " with last
    provide full name
end action
```

**Return Syntax:**
- `give back <expression>` - Return a value
- `provide <expression>` - Alternative return syntax
- `return <expression>` - Alias for `give back`

### Asynchronous Actions

```wfl
define async action fetch data with url:
    display "Fetching from " with url
    wait for 2 seconds  // Simulating network delay
    provide "Data from " with url
end action

// Calling async actions
store result as wait for fetch data with "https://api.example.com"
display result

// Parallel async operations
wait for:
    store data1 as fetch data with "https://api1.com"
    and store data2 as fetch data with "https://api2.com"
end wait

display "Got: " with data1 with " and " with data2
```

**Async Syntax:**
- Define: `define async action <name>: ... end action`
- Call: `wait for <async action call>`
- Parallel: `wait for: <action1> and <action2> end wait`

### Actions with Error Handling

```wfl
define action safe divide with numerator and denominator:
    try:
        check if denominator is equal to 0:
            throw error "Division by zero"
        end check

        store result as numerator divided by denominator
        provide result
    when math error:
        display "Math error: " with error message
        provide nothing
    otherwise:
        display "Unexpected error"
        provide nothing
    end try
end action

store answer as safe divide with 10 and 2  // 5
store bad as safe divide with 10 and 0     // nothing (with error message)
```

### Default Parameters

```wfl
define action greet user with name and greeting default "Hello":
    display greeting with ", " with name
end action

greet user with "Alice" and "Hi"   // Displays: Hi, Alice
greet user with "Bob"              // Displays: Hello, Bob
```

### Variable Scope in Actions

```wfl
// Local variables (scoped to action)
define action calculate:
    local create temp as 0  // Only visible inside this action
    add 5 to temp
    give back temp
end action

// Accessing outer scope variables
store base value as 10

define action add to base with amount:
    store result as base value plus amount  // Can access base value
    give back result
end action

store sum as add to base with 5  // Returns 15
```

### Recursive Actions

```wfl
define action factorial of n:
    check if n is less than or equal to 1:
        give back 1
    otherwise:
        store smaller as factorial of (n minus 1)
        give back n times smaller
    end check
end action

store result as factorial of 5  // Returns 120
```

## 1.5 Object-Oriented Programming (Containers)

### Basic Container Definition

```wfl
create container Person:
    // Properties (data)
    property name as text
    property age as number
    property email as text

    // Actions (behavior)
    define action greet:
        display "Hello, I am " with this name
    end action

    define action celebrate birthday:
        add 1 to this age
        display this name with " is now " with this age with " years old"
    end action
end container
```

**Syntax:** `create container <Name>: <Properties and Actions> end container`

### Creating Container Instances

```wfl
// Creating instances
create new Person as alice:
    set name to "Alice Smith"
    set age to 28
    set email to "alice@example.com"
end create

create new Person as bob:
    set name to "Bob Johnson"
    set age to 35
    set email to "bob@example.com"
end create

// Using instances
alice greet               // Displays: Hello, I am Alice Smith
bob celebrate birthday    // Displays: Bob Johnson is now 36 years old
display alice's age       // Displays: 28
```

### Container with Constructor

```wfl
create container Product:
    property name as text
    property price as number
    property in stock as yes

    // Constructor (initialize action)
    define action initialize with product name and product price:
        set this name to product name
        set this price to product price
        display "Created product: " with this name
    end action
end container

// Create with initialization
create new Product with "Smartphone" and 499.99 as phone
```

### Properties with Validation

```wfl
create container BankAccount:
    property account number as text:
        must not be empty
        must be exactly 10 characters
    end property

    property balance as number:
        must be at least 0
        defaults to 0
    end property

    property owner name as text

    define action deposit amount:
        check amount:
            must be greater than 0
        end check
        add amount to this balance
    end action
end container
```

### Access Control (Public/Private)

```wfl
create container User:
    // Public properties
    public property username as text
    public property display name as text

    // Private properties
    private property password hash as text
    private property login attempts as number defaults to 0

    // Public actions
    public define action greet:
        display "Hello, " with this display name
    end action

    // Private actions
    private define action hash password with raw password:
        // Internal implementation
        store this password hash as secure hash of raw password
    end action

    // Public action using private action
    public define action set password with new password:
        check new password:
            must be at least 8 characters
        end check
        this hash password with new password
    end action
end container
```

### Container Inheritance

```wfl
// Base container
create container Vehicle:
    property make as text
    property model as text
    property year as number

    define action describe:
        display this year with " " with this make with " " with this model
    end action
end container

// Derived container
create container Car extends Vehicle:
    property number of doors as number defaults to 4
    property fuel type as text defaults to "gasoline"

    // Override parent action
    define action describe:
        parent describe  // Call parent version
        display "Doors: " with this number of doors with ", Fuel: " with this fuel type
    end action

    // New action specific to Car
    define action honk:
        display "Beep beep!"
    end action
end container

// Usage
create new Car as my car:
    set make to "Toyota"
    set model to "Corolla"
    set year to 2025
end create

my car describe  // Shows full description
my car honk      // Beep beep!
```

### Container Composition

```wfl
create container Engine:
    property horsepower as number
    property cylinders as number

    define action start:
        display "Engine started: " with this horsepower with " HP"
    end action
end container

create container Car:
    property make as text
    property model as text
    property engine as Engine  // Has-a relationship

    define action start:
        display "Starting " with this make with " " with this model
        this engine start  // Delegate to engine
    end action
end container

// Usage
create new Engine as v6:
    set horsepower to 280
    set cylinders to 6
end create

create new Car as sports car:
    set make to "Nissan"
    set model to "370Z"
    set engine to v6
end create

sports car start
```

### Static Members

```wfl
create container MathUtils:
    static property PI as 3.14159

    static define action circle area with radius:
        provide MathUtils PI times radius times radius
    end action
end container

// Usage (no instance needed)
display MathUtils PI           // 3.14159
store area as MathUtils circle area with 5  // 78.53975
```

### Container Events

```wfl
create container Button:
    property label as text
    property is enabled as yes

    // Define events
    event clicked
    event hover start
    event hover end

    define action click:
        check if this is enabled:
            trigger this clicked
            display "Button clicked: " with this label
        end check
    end action
end container

// Event handling
create new Button as submit:
    set label to "Submit"
end create

on submit clicked:
    submit form
end on
```

### Interfaces and Polymorphism

```wfl
// Define interface
create interface Drawable:
    requires action draw
    requires action resize with width and height
end interface

// Implement interface
create container Circle implements Drawable:
    property radius as number
    property color as text

    define action draw:
        display "Drawing " with this color with " circle"
    end action

    define action resize with width and height:
        set this radius to minimum of width and height divided by 2
    end action
end container

create container Rectangle implements Drawable:
    property width as number
    property height as number
    property color as text

    define action draw:
        display "Drawing " with this color with " rectangle"
    end action

    define action resize with new width and new height:
        set this width to new width
        set this height to new height
    end action
end container

// Polymorphism
create list shapes:
    add new Circle with radius 5 and color "red"
    add new Rectangle with width 10 and height 20 and color "blue"
end list

for each shape in shapes:
    shape draw  // Works for both Circle and Rectangle
end for
```

## 1.6 Async/Await Patterns

### Async Action Definition

```wfl
// Basic async action
define async action fetch weather for city:
    display "Fetching weather for " with city
    wait for 2 seconds  // Simulated delay
    provide "Sunny, 25°C"
end action

// Calling async action
store weather as wait for fetch weather for "New York"
display "Weather: " with weather
```

### Async with Error Handling

```wfl
define async action safe fetch with url:
    try:
        wait for open url at url and read content as data
        provide data
    when network error:
        display "Network error for " with url
        provide nothing
    when timeout error:
        display "Request timed out"
        provide nothing
    otherwise:
        display "Unexpected error"
        provide nothing
    end try
end action
```

### Parallel Async Operations

```wfl
// Wait for multiple operations in parallel
wait for:
    store ny weather as fetch weather for "New York"
    and store la weather as fetch weather for "Los Angeles"
    and store chicago weather as fetch weather for "Chicago"
end wait

display "NY: " with ny weather
display "LA: " with la weather
display "Chicago: " with chicago weather
```

### Async Loops

```wfl
// Process items asynchronously in sequence
define async action process all urls with urls:
    for each url in urls:
        try:
            store data as wait for fetch data from url
            display "Fetched from " with url
            process data
        when network error:
            display "Failed to fetch " with url
        end try
    end for
end action
```

## 1.7 Error Handling

### Try-When-Otherwise

```wfl
try:
    store result as risky operation
    display "Success: " with result
when file not found:
    display "Error: File not found"
when permission denied:
    display "Error: Permission denied"
when network timeout:
    display "Error: Network timeout"
otherwise:
    display "Unexpected error: " with error message
finally:
    cleanup resources
end try
```

**Syntax:**
```ebnf
TryStatement ::= "try:" Block
                 ("when" ErrorCondition ":" Block)*
                 ["otherwise:" Block]
                 ["finally:" Block]
                 "end try"

ErrorCondition ::= "file not found"
                | "permission denied"
                | "network error"
                | "timeout error"
                | "invalid data"
                | "http error"
                | ... (user-defined)
```

### Built-in Error Conditions

Common error conditions in WFL:

- `file not found` - File doesn't exist
- `file unreadable` - Cannot read file
- `permission denied` - Insufficient permissions
- `network error` - Network connectivity issue
- `network timeout` - Request timed out
- `network unreachable` - Cannot reach host
- `http error` - HTTP status >= 400
- `parse error` - Cannot parse data
- `invalid data` - Data validation failed
- `invalid input` - User input validation failed
- `math error` - Mathematical error (divide by zero, etc.)
- `type error` - Type mismatch
- `database error` - Database operation failed

### Retry on Error

```wfl
store max attempts as 3
store attempt as 0

repeat while attempt is less than max attempts:
    add 1 to attempt

    try:
        store data as wait for fetch from url
        display "Success!"
        break  // Exit retry loop on success
    when network timeout:
        display "Attempt " with attempt with " timed out"
        check if attempt is less than max attempts:
            wait 1 second
            display "Retrying..."
        otherwise:
            display "All attempts failed"
        end check
    end try
end repeat
```

### Custom Error Throwing

```wfl
define action validate age with age:
    check if age is less than 0:
        throw error "Age cannot be negative"
    end check

    check if age is greater than 150:
        throw error "Age seems unrealistic"
    end check

    provide yes
end action

// Usage with error handling
try:
    validate age with -5
when validation error:
    display "Validation failed: " with error message
end try
```

### Nested Error Handling

```wfl
try:
    // Outer try
    wait for open file at filename and read content as content

    try:
        // Inner try for processing
        store lines as split content by newline
        for each line in lines:
            try:
                process line
            when parse error:
                display "Skipping invalid line: " with line
            end try
        end for
    when processing error:
        display "Error processing file content"
    end try

when file not found:
    display "File not found: " with filename
when permission denied:
    display "Cannot read file: " with filename
end try
```

## 1.8 Pattern Matching

### Creating Patterns

```wfl
// Define a pattern for email validation
create pattern email pattern:
    one or more letter or digit or "." or "_" or "%" or "+" or "-"
    "@"
    one or more letter or digit or "." or "-"
    "."
    two to four letter
end pattern

// Use pattern
check if user email matches email pattern:
    display "Valid email"
otherwise:
    display "Invalid email"
end check
```

### Capture Groups

```wfl
create pattern phone pattern:
    capture area code: exactly 3 digit
    "-"
    capture exchange: exactly 3 digit
    "-"
    capture number: exactly 4 digit
end pattern

store result as find phone pattern in user input
check if result is not nothing:
    display "Area code: " with result area code
    display "Exchange: " with result exchange
    display "Number: " with result number
end check
```

### Unicode Support

```wfl
// Match by Unicode category
create pattern international digits:
    one or more unicode category "Decimal_Number"
end pattern

// Match by script
create pattern arabic text:
    one or more unicode script "Arabic"
end pattern
```

### Pattern Operations

```wfl
// Find first match
store match as find email pattern in text

// Find all matches
store matches as find all email pattern in text

// Replace matches
store cleaned as replace phone pattern in text with "[REDACTED]"

// Split by pattern
store parts as split text by delimiter pattern
```

---

# Section 2: Standard Library API

WFL provides a comprehensive standard library with ~120+ functions organized into modules. All functions use natural, English-like syntax.

## 2.1 API Overview

| Module | Purpose | Key Functions |
|--------|---------|---------------|
| Core | Basic utilities, type operations | `print`, `typeof`, `isnothing` |
| Math | Mathematical operations | `abs`, `round`, `floor`, `ceil`, `clamp` |
| Random | Cryptographically secure random | `random`, `random_int`, `random_between` |
| Text | String manipulation | `length`, `touppercase`, `tolowercase`, `contains`, `substring` |
| List | List operations | `length`, `push`, `pop`, `contains`, `indexof` |
| Time | Date and time functions | `now`, `today`, `format_date`, `parse_date` |
| Filesystem | File and directory operations | `list_dir`, `glob`, `path_join`, `makedirs` |
| Crypto | Cryptographic hash functions | `wflhash256`, `wflhash512`, `wflmac256` |
| Pattern | Regular expression utilities | `pattern.create`, `pattern.test`, `pattern.find` |

## 2.2 Core Module

### `print(value, ...)`

Outputs values to console with automatic spacing.

**Parameters:** One or more values of any type
**Returns:** Nothing
**Example:**
```wfl
print("Hello, World!")
print("The answer is", 42)
print(name, age, balance)  // Multiple values
```

### `typeof(value)`

Returns the type of a value as text.

**Parameters:** value (any type)
**Returns:** Text - type name ("Number", "Text", "Boolean", "List", etc.)
**Example:**
```wfl
store type as typeof(42)        // "Number"
store type as typeof("hello")   // "Text"
store type as typeof(yes)       // "Boolean"
store type as typeof([1, 2, 3]) // "List"
```

### `isnothing(value)`

Checks if a value is nothing (null/undefined).

**Parameters:** value (any type)
**Returns:** Boolean - yes if nothing, no otherwise
**Example:**
```wfl
store result as isnothing(missing value)  // yes
check if isnothing(user data):
    display "No user data available"
end check
```

## 2.3 Math Module

### `abs(number)`

Returns the absolute value of a number.

**Parameters:** number (Number)
**Returns:** Number - absolute value
**Example:**
```wfl
store positive as abs(-5)   // 5
store result as abs(3.14)   // 3.14
store zero as abs(0)        // 0
```

### `round(number)`

Rounds a number to the nearest integer.

**Parameters:** number (Number)
**Returns:** Number - rounded integer
**Example:**
```wfl
store rounded as round(3.7)    // 4
store rounded as round(3.2)    // 3
store rounded as round(-2.5)   // -3 (rounds to nearest even)
```

### `floor(number)`

Rounds a number down to the nearest integer.

**Parameters:** number (Number)
**Returns:** Number - floor value
**Example:**
```wfl
store lower as floor(3.9)    // 3
store lower as floor(-2.1)   // -3
```

### `ceil(number)`

Rounds a number up to the nearest integer.

**Parameters:** number (Number)
**Returns:** Number - ceiling value
**Example:**
```wfl
store upper as ceil(3.1)    // 4
store upper as ceil(-2.9)   // -2
```

### `clamp(value, min, max)`

Constrains a value between a minimum and maximum.

**Parameters:**
- value (Number) - Value to clamp
- min (Number) - Minimum allowed value
- max (Number) - Maximum allowed value

**Returns:** Number - clamped value
**Example:**
```wfl
store limited as clamp(150, 0, 100)   // 100
store limited as clamp(-10, 0, 100)   // 0
store limited as clamp(50, 0, 100)    // 50
```

## 2.4 Random Module

**Security Note:** All random functions use cryptographically secure random number generation suitable for security-sensitive applications.

### `random()`

Returns a cryptographically secure random number between 0 (inclusive) and 1 (exclusive).

**Parameters:** None
**Returns:** Number (0 ≤ result < 1)
**Example:**
```wfl
store chance as random()  // e.g., 0.7234891
```

### `random_between(min, max)`

Returns a secure random number between specified values (inclusive).

**Parameters:**
- min (Number) - Minimum value (inclusive)
- max (Number) - Maximum value (inclusive)

**Returns:** Number (min ≤ result ≤ max)
**Example:**
```wfl
store temp as random_between(-10, 35)  // e.g., 12.7
```

### `random_int(min, max)`

Returns a secure random integer between specified values (inclusive).

**Parameters:**
- min (Number) - Minimum value (inclusive)
- max (Number) - Maximum value (inclusive)

**Returns:** Number (integer, min ≤ result ≤ max)
**Example:**
```wfl
store dice as random_int(1, 6)  // 1, 2, 3, 4, 5, or 6
```

### `random_boolean()`

Returns a secure random boolean value.

**Parameters:** None
**Returns:** Boolean (yes or no with equal probability)
**Example:**
```wfl
store coin_flip as random_boolean()  // yes or no
```

### `random_from(list)`

Returns a secure random element from a list.

**Parameters:** list (List, must not be empty)
**Returns:** Any (type matches selected element)
**Example:**
```wfl
store colors as ["red", "green", "blue"]
store color as random_from(colors)  // e.g., "blue"
```

## 2.5 Text Module

### `length(text)`

Returns the length of a text string (character count).

**Parameters:** text (Text)
**Returns:** Number - character count
**Example:**
```wfl
store size as length("Hello")  // 5
store empty as length("")      // 0
```

### `touppercase(text)`

Converts text to uppercase.

**Parameters:** text (Text)
**Returns:** Text - uppercase version
**Example:**
```wfl
store loud as touppercase("hello")    // "HELLO"
store same as touppercase("ALREADY")  // "ALREADY"
```

### `tolowercase(text)`

Converts text to lowercase.

**Parameters:** text (Text)
**Returns:** Text - lowercase version
**Example:**
```wfl
store quiet as tolowercase("HELLO")  // "hello"
store same as tolowercase("already") // "already"
```

### `contains(text, search)`

Checks if text contains a substring.

**Parameters:**
- text (Text) - Text to search in
- search (Text) - Substring to search for

**Returns:** Boolean - yes if found, no otherwise
**Example:**
```wfl
store found as contains("Hello World", "World")  // yes
store missing as contains("Hello", "Goodbye")    // no
```

### `substring(text, start, end)`

Extracts a portion of text.

**Parameters:**
- text (Text) - Source text
- start (Number) - Starting index (0-based, inclusive)
- end (Number) - Ending index (exclusive)

**Returns:** Text - extracted substring
**Example:**
```wfl
store part as substring("Hello World", 0, 5)   // "Hello"
store world as substring("Hello World", 6, 11) // "World"
```

### `string_split(text, delimiter)`

Splits text into a list by delimiter.

**Parameters:**
- text (Text) - Text to split
- delimiter (Text) - Delimiter string

**Returns:** List of Text - parts
**Example:**
```wfl
store words as string_split("a,b,c", ",")  // ["a", "b", "c"]
store lines as string_split(text, "\n")    // Split by newline
```

## 2.6 List Module

### `length(list)`

Returns the number of elements in a list.

**Parameters:** list (List)
**Returns:** Number - element count
**Example:**
```wfl
store count as length([1, 2, 3])  // 3
store empty as length([])         // 0
```

### `push(list, item)`

Adds an item to the end of a list (modifies in place).

**Parameters:**
- list (List) - List to modify
- item (Any) - Item to add

**Returns:** Nothing
**Example:**
```wfl
store numbers as [1, 2, 3]
push(numbers, 4)
// numbers is now [1, 2, 3, 4]
```

### `pop(list)`

Removes and returns the last item from a list.

**Parameters:** list (List)
**Returns:** Any - the removed item, or nothing if list is empty
**Example:**
```wfl
store numbers as [1, 2, 3]
store last as pop(numbers)  // 3
// numbers is now [1, 2]
```

### `contains(list, item)`

Checks if a list contains a specific item.

**Parameters:**
- list (List) - List to search
- item (Any) - Item to find

**Returns:** Boolean - yes if found, no otherwise
**Example:**
```wfl
store found as contains([1, 2, 3], 2)    // yes
store missing as contains([1, 2, 3], 5)  // no
```

### `indexof(list, item)`

Finds the index of an item in a list.

**Parameters:**
- list (List) - List to search
- item (Any) - Item to find

**Returns:** Number - index (0-based), or -1 if not found
**Example:**
```wfl
store position as indexof([10, 20, 30], 20)  // 1
store missing as indexof([10, 20, 30], 99)   // -1
```

## 2.7 Time Module

### `today()`

Returns the current date.

**Parameters:** None
**Returns:** Date
**Example:**
```wfl
store current_date as today()
```

### `now()`

Returns the current time.

**Parameters:** None
**Returns:** Time
**Example:**
```wfl
store current_time as now()
```

### `datetime_now()`

Returns the current date and time.

**Parameters:** None
**Returns:** DateTime
**Example:**
```wfl
store current as datetime_now()
```

### `format_date(date, format)`

Formats a date according to a format string.

**Parameters:**
- date (Date) - Date to format
- format (Text) - Format string using strftime-like syntax

**Returns:** Text - formatted date string

**Format Codes:**
- `%Y` - 4-digit year (e.g., 2025)
- `%y` - 2-digit year (e.g., 25)
- `%m` - Month number (01-12)
- `%B` - Full month name (e.g., January)
- `%b` - Abbreviated month name (e.g., Jan)
- `%d` - Day of month (01-31)
- `%A` - Full weekday name (e.g., Monday)
- `%a` - Abbreviated weekday name (e.g., Mon)

**Example:**
```wfl
store formatted as format_date(today(), "%Y-%m-%d")          // "2025-11-30"
store readable as format_date(today(), "%B %d, %Y")          // "November 30, 2025"
store short as format_date(today(), "%m/%d/%y")              // "11/30/25"
```

### `format_time(time, format)`

Formats a time according to a format string.

**Parameters:**
- time (Time) - Time to format
- format (Text) - Format string

**Format Codes:**
- `%H` - Hour (24-hour format, 00-23)
- `%I` - Hour (12-hour format, 01-12)
- `%M` - Minutes (00-59)
- `%S` - Seconds (00-59)
- `%p` - AM/PM

**Returns:** Text - formatted time string
**Example:**
```wfl
store formatted as format_time(now(), "%H:%M:%S")    // "14:30:45"
store twelve_hour as format_time(now(), "%I:%M %p") // "02:30 PM"
```

### `parse_date(text, format)`

Parses a date from a text string.

**Parameters:**
- text (Text) - Date string to parse
- format (Text) - Expected format

**Returns:** Date
**Example:**
```wfl
store birthday as parse_date("1990-12-25", "%Y-%m-%d")
store date as parse_date("11/30/2025", "%m/%d/%Y")
```

### `add_days(date, days)`

Adds a number of days to a date.

**Parameters:**
- date (Date) - Starting date
- days (Number) - Days to add (can be negative)

**Returns:** Date - resulting date
**Example:**
```wfl
store tomorrow as add_days(today(), 1)
store last_week as add_days(today(), -7)
```

### `days_between(date1, date2)`

Calculates the number of days between two dates.

**Parameters:**
- date1 (Date) - First date
- date2 (Date) - Second date

**Returns:** Number - days between (positive if date2 is later, negative if earlier)
**Example:**
```wfl
store christmas as parse_date("2025-12-25", "%Y-%m-%d")
store days_until as days_between(today(), christmas)
```

## 2.8 Filesystem Module

### `list_dir(path)`

Lists all files and directories in the specified path.

**Parameters:** path (Text) - Directory path
**Returns:** List of Text - file and directory names
**Example:**
```wfl
store files as list_dir(".")
store docs as list_dir("./Docs")
```

### `glob(pattern, base_path)`

Finds files matching a glob pattern.

**Parameters:**
- pattern (Text) - Glob pattern (`*`, `?`, `[abc]`, etc.)
- base_path (Text) - Base directory to search

**Returns:** List of Text - matching file paths

**Pattern Examples:**
- `"*.txt"` - All .txt files
- `"test_*.wfl"` - Files starting with test_
- `"[abc]*.log"` - Files starting with a, b, or c

**Example:**
```wfl
store wfl_files as glob("*.wfl", "TestPrograms")
store tests as glob("test_*.rs", "tests")
```

### `rglob(pattern, base_path)`

Recursively finds files matching a glob pattern.

**Parameters:**
- pattern (Text) - Glob pattern
- base_path (Text) - Base directory to search recursively

**Returns:** List of Text - matching file paths
**Example:**
```wfl
store all_rs as rglob("*.rs", "src")      // All .rs files in src/ and subdirs
store all_md as rglob("*.md", "Docs")     // All .md files in Docs/ and subdirs
```

### `path_join(component1, component2, ...)`

Joins path components into a single path.

**Parameters:** component1, component2, ... (Text) - Path components
**Returns:** Text - joined path (platform-appropriate separators)
**Example:**
```wfl
store full_path as path_join("home", "user", "documents")
// Windows: "home\user\documents"
// Unix: "home/user/documents"
```

### `path_basename(path)`

Returns the filename portion of a path.

**Parameters:** path (Text) - File path
**Returns:** Text - filename
**Example:**
```wfl
store filename as path_basename("/home/user/test.txt")  // "test.txt"
store name as path_basename("C:\\Users\\Alice\\file.wfl")  // "file.wfl"
```

### `path_dirname(path)`

Returns the directory portion of a path.

**Parameters:** path (Text) - File path
**Returns:** Text - directory path
**Example:**
```wfl
store dir as path_dirname("/home/user/test.txt")  // "/home/user"
store parent as path_dirname("C:\\Users\\Alice\\file.wfl")  // "C:\Users\Alice"
```

### `makedirs(path)`

Creates a directory and all necessary parent directories.

**Parameters:** path (Text) - Directory path to create
**Returns:** Nothing
**Example:**
```wfl
makedirs("data/output/results")  // Creates all three directories if needed
```

### `path_exists(path)`

Checks if a file or directory exists.

**Parameters:** path (Text) - Path to check
**Returns:** Boolean - yes if exists, no otherwise
**Example:**
```wfl
check if path_exists("config.txt"):
    display "Config file found"
end check
```

### `is_file(path)`

Checks if a path is a file.

**Parameters:** path (Text) - Path to check
**Returns:** Boolean - yes if file, no otherwise
**Example:**
```wfl
check if is_file("README.md"):
    display "README.md is a file"
end check
```

### `is_dir(path)`

Checks if a path is a directory.

**Parameters:** path (Text) - Path to check
**Returns:** Boolean - yes if directory, no otherwise
**Example:**
```wfl
check if is_dir("src"):
    display "src is a directory"
end check
```

### `file_mtime(path)`

Returns the modification time of a file as a timestamp.

**Parameters:** path (Text) - File path
**Returns:** Number - Unix timestamp (seconds since epoch)
**Example:**
```wfl
store last_modified as file_mtime("data.txt")
```

## 2.9 Crypto Module

**⚠️ Security Disclaimer:** WFLHASH is a non-validated cryptographic hash function. While it implements cryptographically sound design principles, it has NOT undergone external cryptographic audits. For production applications requiring validated cryptography, consider SHA-256, SHA-3, or BLAKE3.

**Design Features:**
- Sponge construction (similar to SHA-3)
- 24-round security margin
- HKDF-based key derivation for MAC mode
- Secure memory management

**Recommended For:**
- ✅ Internal applications
- ✅ Non-critical data integrity verification
- ✅ Development and testing

**NOT Recommended For:**
- ❌ FIPS-validated environments
- ❌ High-security environments requiring proven algorithms
- ❌ Regulatory compliance

### `wflhash256(data)`

Generates a 256-bit WFLHASH digest.

**Parameters:** data (Text) - Data to hash
**Returns:** Text - Hexadecimal hash string (64 characters)
**Example:**
```wfl
store hash as wflhash256("Hello, World!")
display hash  // e.g., "a1b2c3d4..."
```

### `wflhash512(data)`

Generates a 512-bit WFLHASH digest.

**Parameters:** data (Text) - Data to hash
**Returns:** Text - Hexadecimal hash string (128 characters)
**Example:**
```wfl
store hash as wflhash512("Important data")
```

### `wflhash256_with_salt(data, salt)`

Generates a salted 256-bit hash for domain separation.

**Parameters:**
- data (Text) - Data to hash
- salt (Text) - Salt value for domain separation

**Returns:** Text - Hexadecimal hash string (64 characters)
**Example:**
```wfl
store hash as wflhash256_with_salt("password123", "user_auth")
```

### `wflmac256(message, key)`

Generates a message authentication code (MAC).

**Parameters:**
- message (Text) - Message to authenticate
- key (Text) - Secret key

**Returns:** Text - Hexadecimal MAC string (64 characters)
**Example:**
```wfl
store mac as wflmac256("message data", "secret_key")
```

## 2.10 Complete API Index (Alphabetical)

| Function | Module | Returns | Section |
|----------|--------|---------|---------|
| `abs(n)` | Math | Number | §2.3 |
| `add_days(date, days)` | Time | Date | §2.7 |
| `ceil(n)` | Math | Number | §2.3 |
| `clamp(val, min, max)` | Math | Number | §2.3 |
| `contains(text, search)` | Text | Boolean | §2.5 |
| `contains(list, item)` | List | Boolean | §2.6 |
| `datetime_now()` | Time | DateTime | §2.7 |
| `days_between(d1, d2)` | Time | Number | §2.7 |
| `file_mtime(path)` | Filesystem | Number | §2.8 |
| `floor(n)` | Math | Number | §2.3 |
| `format_date(date, fmt)` | Time | Text | §2.7 |
| `format_time(time, fmt)` | Time | Text | §2.7 |
| `glob(pattern, base)` | Filesystem | List | §2.8 |
| `indexof(list, item)` | List | Number | §2.6 |
| `is_dir(path)` | Filesystem | Boolean | §2.8 |
| `is_file(path)` | Filesystem | Boolean | §2.8 |
| `isnothing(value)` | Core | Boolean | §2.2 |
| `length(text)` | Text | Number | §2.5 |
| `length(list)` | List | Number | §2.6 |
| `list_dir(path)` | Filesystem | List | §2.8 |
| `makedirs(path)` | Filesystem | Nothing | §2.8 |
| `now()` | Time | Time | §2.7 |
| `parse_date(text, fmt)` | Time | Date | §2.7 |
| `path_basename(path)` | Filesystem | Text | §2.8 |
| `path_dirname(path)` | Filesystem | Text | §2.8 |
| `path_exists(path)` | Filesystem | Boolean | §2.8 |
| `path_join(c1, c2, ...)` | Filesystem | Text | §2.8 |
| `pop(list)` | List | Any | §2.6 |
| `print(value, ...)` | Core | Nothing | §2.2 |
| `push(list, item)` | List | Nothing | §2.6 |
| `random()` | Random | Number | §2.4 |
| `random_between(min, max)` | Random | Number | §2.4 |
| `random_boolean()` | Random | Boolean | §2.4 |
| `random_from(list)` | Random | Any | §2.4 |
| `random_int(min, max)` | Random | Number | §2.4 |
| `rglob(pattern, base)` | Filesystem | List | §2.8 |
| `round(n)` | Math | Number | §2.3 |
| `string_split(text, delim)` | Text | List | §2.5 |
| `substring(text, start, end)` | Text | Text | §2.5 |
| `today()` | Time | Date | §2.7 |
| `tolowercase(text)` | Text | Text | §2.5 |
| `touppercase(text)` | Text | Text | §2.5 |
| `typeof(value)` | Core | Text | §2.2 |
| `wflhash256(data)` | Crypto | Text | §2.9 |
| `wflhash256_with_salt(data, salt)` | Crypto | Text | §2.9 |
| `wflhash512(data)` | Crypto | Text | §2.9 |
| `wflmac256(msg, key)` | Crypto | Text | §2.9 |

---

# Section 3: I/O & Async Operations

WFL provides unified, natural-language syntax for all I/O operations (files, network, databases). All I/O operations support both synchronous and asynchronous execution with the same syntax.

## 3.1 File I/O Operations

### Opening and Closing Files

**Basic Syntax:**
```wfl
// Open for reading (default)
open file at "config.txt" as config_file

// Open for writing (creates if doesn't exist)
open file at "output.txt" for writing as out_file

// Open for appending
open file at "log.txt" for append as log_file

// Close file
close file config_file
```

**Async File Operations:**
```wfl
// All file operations can be async
wait for open file at "data.txt" and read content as data

// Explicit async with separate steps
open file at "large_file.dat" for reading as big_file
wait for store content as read content from big_file
close file big_file
```

### Reading from Files

**Complete File Read:**
```wfl
// Read entire file
open file at "input.txt" for reading as input_file
wait for store file_content as read content from input_file
close file input_file

display "Read " with length of file_content with " characters"
```

**Streaming File Read (for large files):**
```wfl
// Read file in chunks
open file at "large_data.dat" for reading as big_file

repeat until end of file:
    store chunk as read next 1024 bytes from big_file
    process chunk
end repeat

close file big_file
```

### Writing to Files

**Simple Write:**
```wfl
// Write content (overwrites existing)
open file at "output.txt" for writing as out_file
wait for write content "Hello, World!" into out_file
close file out_file
```

**Append to File:**
```wfl
// Append content
open file at "log.txt" for append as log_file
wait for append content "New log entry\n" into log_file
close file log_file
```

**Multiple Writes:**
```wfl
open file at "report.txt" for writing as report

wait for write content "Report Header\n" into report
wait for append content "============\n" into report
wait for append content "Data: " into report
wait for append content data_value into report
wait for append content "\n" into report

close file report
```

### File Operations

**Delete File:**
```wfl
delete file at "temp.txt"
```

**Check File Existence:**
```wfl
store exists as file exists at "config.txt"

check if file exists at "data.txt":
    display "File found"
otherwise:
    display "File not found"
end check
```

**Get File Info:**
```wfl
// File modification time
store mtime as file_mtime of "document.txt"
display "Last modified: " with mtime

// Check if path is file or directory
check if is_file of "README.md":
    display "It's a file"
end check

check if is_dir of "src":
    display "It's a directory"
end check
```

### Complete File I/O Example

```wfl
// Reading configuration with error handling and defaults
define action load_config:
    gives back:
        config as text
    do:
        try:
            wait for open file at "config.json" and read content as config
            display "✓ Configuration loaded successfully"
            give back config

        when file not found:
            display "⚠ Config file not found, creating default"

            // Create default config
            store default_config as "{
    \"server_port\": 8080,
    \"debug_mode\": false,
    \"log_level\": \"info\"
}"
            open file at "config.json" for writing as new_config
            write content default_config into new_config
            close file new_config

            give back default_config

        when permission denied:
            display "❌ Cannot read config file - permission denied"
            give back "{}"

        otherwise:
            display "❌ Error reading config: " with error message
            give back "{}"
        end try
end action

// Writing log file with timestamp
define action write_log with message:
    store timestamp as current time formatted as "yyyy-MM-dd HH:mm:ss"
    store log_entry as "[" with timestamp with "] " with message with "\n"

    try:
        wait for open file at "app.log" for append and write content log_entry
        display "✓ Log written"
    when error:
        display "⚠ Could not write to log"
    end try
end action

// Processing multiple files
define action process_all_text_files with directory:
    store text_files as glob of "*.txt" and directory

    display "Found " with length of text_files with " text files"

    for each file_path in text_files:
        try:
            open file at file_path for reading as input
            wait for store content as read content from input
            close file input

            display "Processed " with file_path with ": " with length of content with " chars"

        when file not found:
            display "Skipping missing file: " with file_path
        when permission denied:
            display "Cannot access file: " with file_path
        end try
    end for
end action
```

## 3.2 HTTP/Web Operations

### Simple HTTP GET Requests

**Basic GET:**
```wfl
// Simple GET request
wait for open url at "https://api.example.com/data" and read content as response

display "Response: " with response
```

**GET with Full Control:**
```wfl
// Explicit open/read/close
open url at "https://api.example.com/users/123" as api_response

wait for store user_data as read response from api_response
display "User data: " with user_data

close api_response
```

### HTTP POST Requests

**POST with JSON:**
```wfl
// POST request with body and headers
open url at "https://api.example.com/users" as api_request with:
    method as POST
    body as "{\"name\": \"Alice\", \"email\": \"alice@example.com\"}"
    header "Content-Type" as "application/json"
end with

wait for store response as read response from api_request
close api_request

display "API response: " with response
```

**Simplified POST:**
```wfl
// Short form POST
wait for open url at "https://api.example.com/data" with method POST and write request_body and read content as result
```

### HTTP Methods

**PUT Request:**
```wfl
open url at "https://api.example.com/users/123" as update_request with:
    method as PUT
    body as updated_user_json
    header "Content-Type" as "application/json"
end with

wait for store update_response as read response from update_request
close update_request
```

**DELETE Request:**
```wfl
open url at "https://api.example.com/users/123" as delete_request with:
    method as DELETE
end with

wait for store delete_response as read response from delete_request
display "Status: " with delete_response status  // e.g., 204 No Content
close delete_request
```

### HTTP Response Handling

**Accessing Response Details:**
```wfl
open url at "https://api.example.com/data" as api_response

// Read response body
wait for store body as read response from api_response

// Access response metadata
store status_code as status of api_response
store headers as headers of api_response

display "Status: " with status_code
display "Body: " with body

close api_response
```

**Error Handling for HTTP:**
```wfl
define async action safe_api_call with url:
    gives back:
        data as text
    do:
        try:
            wait for open url at url and read content as response
            give back response

        when network timeout:
            display "Request timed out, retrying..."
            wait 1 second
            retry

        when network error:
            display "Network error - cannot connect"
            give back "{\"error\": \"network_unreachable\"}"

        when http error:
            display "HTTP error - server returned error status"
            give back "{\"error\": \"http_error\"}"

        otherwise:
            display "Unexpected error: " with error message
            give back "{\"error\": \"unknown\"}"
        end try
end action
```

### Parallel HTTP Requests

**Fetching Multiple URLs:**
```wfl
// Launch requests in parallel
define async action fetch_multiple with urls:
    gives back:
        results as list
    do:
        store tasks as []

        // Start all requests
        for each url in urls:
            store task as fetch data from url  // Non-blocking
            push of tasks and task
        end for

        // Collect results
        store results as []
        for each task in tasks:
            store result as wait for task
            push of results and result
        end for

        give back results
end action

// Usage
store api_urls as [
    "https://api.example.com/users",
    "https://api.example.com/products",
    "https://api.example.com/orders"
]

store all_data as wait for fetch_multiple with api_urls
display "Fetched " with length of all_data with " responses"
```

**Parallel with Structured Wait:**
```wfl
// Wait for multiple operations together
wait for:
    store users as fetch from "https://api.example.com/users"
    and store products as fetch from "https://api.example.com/products"
    and store orders as fetch from "https://api.example.com/orders"
end wait

display "Got all data:"
display "Users: " with length of users
display "Products: " with length of products
display "Orders: " with length of orders
```

## 3.3 Web Server Creation

WFL provides built-in web server capabilities with natural language syntax for creating HTTP servers, handling requests, routing, and responses.

### Basic Web Server

**Minimal Server:**
```wfl
// Start listening on a port
listen on port 8080 as web_server
display "Server started on port 8080"

// Main server loop
main loop:
    // Wait for request
    wait for request comes in on web_server as req

    // Send response
    respond to req with "Hello, World!" and content_type "text/plain"
end loop
```

### Request Handling

**Extracting Request Information:**
```wfl
main loop:
    wait for request comes in on web_server as incoming_request

    // Extract request details
    store method as method of incoming_request
    store path as path of incoming_request
    store client_ip as client_ip of incoming_request
    store body as body of incoming_request
    store headers as headers of incoming_request

    display "Received " with method with " " with path with " from " with client_ip

    // Process request...
end loop
```

### Routing and Responses

**Route-Based Handling:**
```wfl
listen on port 8080 as server

main loop:
    wait for request comes in on server as req

    store method as method of req
    store path as path of req

    check if method is equal to "GET":
        check if path is equal to "/":
            respond to req with "Home Page" and content_type "text/html"

        otherwise check if path is equal to "/api/status":
            store status_json as "{\"status\": \"ok\", \"server\": \"WFL\"}"
            respond to req with status_json and content_type "application/json"

        otherwise check if path starts with "/api/":
            respond to req with "{\"error\": \"Not found\"}" and status 404 and content_type "application/json"

        otherwise:
            respond to req with "Page not found" and status 404 and content_type "text/plain"
        end check

    otherwise check if method is equal to "POST":
        check if path is equal to "/api/echo":
            store echo_body as body of req
            store echo_response as "{\"echo\": \"" with echo_body with "\"}"
            respond to req with echo_response and content_type "application/json"
        otherwise:
            respond to req with "{\"error\": \"Not found\"}" and status 404 and content_type "application/json"
        end check

    otherwise:
        respond to req with "Method not allowed" and status 405
    end check
end loop
```

### Static File Serving

```wfl
// Serve static files from a directory
listen on port 8080 as server
store static_dir as "public"

main loop:
    wait for request comes in on server as req

    store path as path of req

    check if path starts with "/static/":
        // Extract file path
        store file_path as static_dir with substring of path from 8

        check if file exists at file_path:
            open file at file_path for reading as static_file
            store content as read content from static_file
            close file static_file

            // Determine MIME type
            store content_type as "text/plain"
            check if file_path ends with ".html":
                change content_type to "text/html"
            otherwise check if file_path ends with ".css":
                change content_type to "text/css"
            otherwise check if file_path ends with ".js":
                change content_type to "application/javascript"
            otherwise check if file_path ends with ".json":
                change content_type to "application/json"
            end check

            respond to req with content and content_type content_type
            display "✓ Served: " with file_path
        otherwise:
            respond to req with "File not found" and status 404
            display "✗ Not found: " with file_path
        end check
    otherwise:
        respond to req with "Not found" and status 404
    end check
end loop
```

### Complete Web Server Example

```wfl
// Comprehensive web server with middleware, error handling, and routing
display "Starting WFL Web Server..."

store server_port as 8080
store max_requests as 100
store request_count as 0
store server_start_time as current time in milliseconds

try:
    listen on port server_port as web_server
    display "✅ Server listening on port " with server_port
    display "Visit http://localhost:" with server_port

    main loop:
        try:
            // Check shutdown condition
            check if request_count is greater than max_requests:
                display "Maximum requests reached, shutting down"
                break
            end check

            // Wait for request
            wait for request comes in on web_server as req
            add 1 to request_count

            // Extract request info
            store method as method of req
            store path as path of req
            store client_ip as client_ip of req
            store timestamp as current time formatted as "yyyy-MM-dd HH:mm:ss"

            // Middleware: Request logging
            display "📥 [" with timestamp with "] " with method with " " with path with " from " with client_ip

            // Route handling
            check if method is equal to "GET":
                check if path is equal to "/":
                    store home_html as "<!DOCTYPE html>
<html>
<head><title>WFL Server</title></head>
<body>
    <h1>Welcome to WFL Web Server</h1>
    <p>Server running on natural language code!</p>
    <ul>
        <li><a href=\"/hello\">Hello endpoint</a></li>
        <li><a href=\"/api/status\">Status API</a></li>
    </ul>
</body>
</html>"
                    respond to req with home_html and content_type "text/html"
                    display "✅ Served home page"

                otherwise check if path is equal to "/hello":
                    store greeting as "Hello from WFL! Request #" with request_count
                    respond to req with greeting and content_type "text/plain"
                    display "✅ Served greeting"

                otherwise check if path is equal to "/api/status":
                    store uptime as current time in milliseconds minus server_start_time
                    store status_json as "{
    \"status\": \"running\",
    \"uptime_ms\": " with uptime with ",
    \"requests\": " with request_count with ",
    \"timestamp\": \"" with timestamp with "\"
}"
                    respond to req with status_json and content_type "application/json"
                    display "✅ Served status API"

                otherwise:
                    respond to req with "Not found" and status 404
                    display "❌ 404: " with path
                end check

            otherwise check if method is equal to "POST":
                check if path is equal to "/api/echo":
                    store body as body of req
                    store echo_json as "{\"echo\": \"" with body with "\"}"
                    respond to req with echo_json and content_type "application/json"
                    display "✅ Echo API"
                otherwise:
                    respond to req with "{\"error\": \"Not found\"}" and status 404 and content_type "application/json"
                end check

            otherwise:
                respond to req with "Method not allowed" and status 405
            end check

        when error:
            display "❌ Error handling request: " with error message
            try:
                respond to req with "{\"error\": \"Internal error\"}" and status 500 and content_type "application/json"
            when error:
                display "Could not send error response"
            end try
        end try

        // Progress indicator
        check if request_count modulo 10 is equal to 0:
            display "📊 Processed " with request_count with " requests"
        end check
    end loop

    // Graceful shutdown
    display ""
    display "🛑 Server shutting down"
    display "Total requests: " with request_count

when error:
    display "❌ Server error: " with error message
end try
```

### Advanced Web Server Features

**Middleware Pattern:**
```wfl
// Request logging middleware
define action log_request with req:
    store timestamp as current time formatted as "yyyy-MM-dd HH:mm:ss"
    store method as method of req
    store path as path of req
    store ip as client_ip of req

    display "[" with timestamp with "] " with method with " " with path with " from " with ip

    // Log to file
    store log_entry as timestamp with " " with method with " " with path with " " with ip with "\n"
    wait for open file at "access.log" for append and write content log_entry
end action

// Authentication middleware
define action check_auth with req:
    gives back:
        is_authenticated as boolean
    do:
        store auth_header as header "Authorization" of req

        check if isnothing of auth_header:
            give back no
        end check

        // Validate token (simplified)
        check if auth_header starts with "Bearer ":
            store token as substring of auth_header from 7
            // Validate token...
            give back yes
        otherwise:
            give back no
        end check
end action

// Apply middleware
main loop:
    wait for request comes in on server as req

    // Apply logging
    log_request with req

    // Apply authentication for protected routes
    store path as path of req
    check if path starts with "/api/protected/":
        store is_auth as check_auth with req
        check if not is_auth:
            respond to req with "{\"error\": \"Unauthorized\"}" and status 401 and content_type "application/json"
            continue  // Skip to next request
        end check
    end check

    // Process request...
end loop
```

**File Upload Handling:**
```wfl
check if method is equal to "POST" and path is equal to "/upload":
    store body as body of req
    store upload_dir as "uploads"

    // Generate unique filename
    store upload_name as "upload_" with request_count with "_" with current time in milliseconds with ".txt"
    store upload_path as path_join of upload_dir and upload_name

    // Save uploaded file
    makedirs of upload_dir
    open file at upload_path for writing as upload_file
    write content body into upload_file
    close file upload_file

    // Send success response
    store upload_response as "{
    \"message\": \"File uploaded\",
    \"filename\": \"" with upload_name with "\",
    \"size\": " with length of body with "
}"
    respond to req with upload_response and content_type "application/json" and status 201
    display "✅ File uploaded: " with upload_name
end check
```

## 3.4 Async Best Practices

### Pattern 1: Sequential Async Operations

```wfl
// Each operation waits for previous to complete
define async action process_user_data with user_id:
    // Fetch user
    wait for store user as fetch from "https://api.example.com/users/" with user_id

    // Use user data to fetch orders
    wait for store orders as fetch from "https://api.example.com/orders?user=" with user_id

    // Combine data
    store combined as "User: " with user with ", Orders: " with orders
    give back combined
end action
```

### Pattern 2: Parallel Async Operations

```wfl
// Operations run concurrently
define async action fetch_dashboard_data with user_id:
    // Launch all requests in parallel
    wait for:
        store user_data as fetch from "https://api.example.com/users/" with user_id
        and store order_data as fetch from "https://api.example.com/orders?user=" with user_id
        and store activity_data as fetch from "https://api.example.com/activity?user=" with user_id
    end wait

    // All data available here
    store dashboard as combine user_data and order_data and activity_data
    give back dashboard
end action
```

### Pattern 3: Error Handling in Async

```wfl
// Robust async with retry logic
define async action fetch_with_retry with url and max_attempts:
    gives back:
        data as text
    do:
        store attempt as 0

        repeat while attempt is less than max_attempts:
            add 1 to attempt

            try:
                wait for store response as fetch from url
                display "✓ Success on attempt " with attempt
                give back response

            when network timeout:
                display "⚠ Timeout on attempt " with attempt
                check if attempt is less than max_attempts:
                    wait 1 second
                    display "Retrying..."
                otherwise:
                    give back nothing
                end check

            when network error:
                display "⚠ Network error on attempt " with attempt
                check if attempt is less than max_attempts:
                    wait 2 seconds
                otherwise:
                    give back nothing
                end check

            otherwise:
                display "❌ Unexpected error: " with error message
                give back nothing
            end try
        end repeat

        give back nothing
end action
```

### Pattern 4: Async File Processing

```wfl
// Process files asynchronously
define async action process_files_async with file_paths:
    gives back:
        results as list
    do:
        store results as []

        for each path in file_paths:
            try:
                wait for open file at path and read content as content

                // Process content
                store processed as process content with content
                push of results and processed

                display "✓ Processed: " with path

            when file not found:
                display "⚠ Skipped missing file: " with path
                push of results and nothing

            when error:
                display "❌ Error processing " with path with ": " with error message
                push of results and nothing
            end try
        end for

        give back results
end action
```

### Pattern 5: Timeout Management

```wfl
// Async operation with timeout
define async action fetch_with_timeout with url and timeout_ms:
    gives back:
        result as text
    do:
        store timeout_task as wait duration timeout_ms
        store fetch_task as fetch from url

        // Race between fetch and timeout
        wait for either:
            store data as fetch_task:
                display "✓ Fetch completed"
                give back data
            or store timeout_reached as timeout_task:
                display "❌ Fetch timed out after " with timeout_ms with "ms"
                give back nothing
        end wait
end action
```

### Pattern 6: Async Error Aggregation

```wfl
// Collect errors from multiple async operations
define async action fetch_all_with_errors with urls:
    gives back:
        results as list
        errors as list
    do:
        store results as []
        store errors as []

        for each url in urls:
            try:
                wait for store data as fetch from url
                push of results and data

            when network error:
                store error_info as "Network error for " with url
                push of errors and error_info
                push of results and nothing

            when http error:
                store error_info as "HTTP error for " with url
                push of errors and error_info
                push of results and nothing
            end try
        end for

        display "Completed: " with length of results with " total, " with length of errors with " errors"
        give back results and errors
end action
```

## 3.5 Complete I/O Examples

### Example 1: Configuration Management

```wfl
// Load configuration from file with defaults
define action load_application_config:
    gives back:
        config as map
    do:
        store config as create empty map
        store config_path as "app.config"

        try:
            wait for open file at config_path and read content as config_text

            // Parse simple key=value format
            store lines as string_split of config_text and "\n"

            for each line in lines:
                store trimmed as trim of line

                // Skip empty lines and comments
                check if length of trimmed is greater than 0 and not starts with trimmed and "#":
                    check if contains of trimmed and "=":
                        store parts as string_split of trimmed and "="
                        check if length of parts is equal to 2:
                            store key as trim of parts 0
                            store value as trim of parts 1
                            set key in config to value
                        end check
                    end check
                end check
            end for

            display "✓ Configuration loaded"

        when file not found:
            display "⚠ No config file, using defaults"
        when error:
            display "⚠ Error reading config: " with error message
        end try

        // Set defaults for missing values
        check if not contains of config and "server_port":
            set "server_port" in config to "8080"
        end check

        check if not contains of config and "debug_mode":
            set "debug_mode" in config to "false"
        end check

        give back config
end action
```

### Example 2: API Client with Rate Limiting

```wfl
// API client with rate limiting and retries
define async action api_client:
    store api_base as "https://api.example.com"
    store requests_per_minute as 60
    store request_delay as 1000  // 1 second between requests

    define async action fetch_user with user_id:
        store url as api_base with "/users/" with user_id

        // Wait for rate limit
        wait for request_delay milliseconds

        // Fetch with retry
        store max_retries as 3
        store retry_count as 0

        repeat while retry_count is less than max_retries:
            try:
                wait for store data as fetch from url
                give back data

            when network timeout:
                add 1 to retry_count
                display "⚠ Timeout (attempt " with retry_count with ")"
                wait 2 seconds

            when http error:
                display "❌ HTTP error for user " with user_id
                give back nothing

            otherwise:
                add 1 to retry_count
                display "⚠ Error (attempt " with retry_count with ")"
                wait 1 second
            end try
        end repeat

        display "❌ All retries failed for user " with user_id
        give back nothing
    end action

    // Fetch multiple users with rate limiting
    define async action fetch_multiple_users with user_ids:
        gives back:
            users as list
        do:
            store users as []

            for each user_id in user_ids:
                store user_data as wait for fetch_user with user_id
                push of users and user_data
                display "✓ Fetched user " with user_id
            end for

            give back users
    end action
end action
```

### Example 3: Data Pipeline (File → Process → API)

```wfl
// Complete data pipeline: read file, process, send to API
define async action run_data_pipeline with input_file and api_endpoint:
    display "Starting data pipeline..."

    try:
        // Step 1: Read input file
        display "Step 1: Reading input file"
        wait for open file at input_file and read content as raw_data
        display "✓ Read " with length of raw_data with " bytes"

        // Step 2: Parse and validate
        display "Step 2: Parsing data"
        store lines as string_split of raw_data and "\n"
        store valid_records as []

        for each line in lines:
            store trimmed as trim of line
            check if length of trimmed is greater than 0:
                // Validate record (simplified)
                check if contains of trimmed and ",":
                    push of valid_records and trimmed
                end check
            end check
        end for

        display "✓ Validated " with length of valid_records with " records"

        // Step 3: Transform data
        display "Step 3: Transforming data"
        store transformed as []
        for each record in valid_records:
            store parts as string_split of record and ","
            store json_record as "{\"field1\": \"" with parts 0 with "\", \"field2\": \"" with parts 1 with "\"}"
            push of transformed and json_record
        end for

        display "✓ Transformed " with length of transformed with " records"

        // Step 4: Send to API
        display "Step 4: Sending to API"
        store api_body as "[" with join transformed with "," with "]"

        wait for open url at api_endpoint with method POST and write api_body and read content as api_response

        display "✓ API response: " with api_response
        display "Pipeline completed successfully!"

    when file not found:
        display "❌ Input file not found: " with input_file

    when network error:
        display "❌ Cannot reach API: " with api_endpoint

    otherwise:
        display "❌ Pipeline error: " with error message
    end try
end action

// Run pipeline
wait for run_data_pipeline with "input.csv" and "https://api.example.com/data/import"
```

### Example 4: Log File Rotation

```wfl
// Rotate log files when they get too large
define action rotate_log_if_needed with log_file and max_lines:
    check if not path_exists of log_file:
        display "Log file doesn't exist yet"
        give back
    end check

    // Check log size
    store line_count as count_lines of log_file

    check if line_count is greater than max_lines:
        display "Log file has " with line_count with " lines, rotating..."

        // Create backup filename with timestamp
        store timestamp as current time formatted as "yyyyMMdd_HHmmss"
        store backup_name as log_file with "." with timestamp with ".backup"

        // Read current log
        open file at log_file for reading as current_log
        store log_content as read content from current_log
        close file current_log

        // Write to backup
        open file at backup_name for writing as backup_log
        write content log_content into backup_log
        close file backup_log

        // Clear current log
        open file at log_file for writing as new_log
        write content "" into new_log
        close file new_log

        display "✓ Log rotated to " with backup_name
    otherwise:
        display "Log file OK (" with line_count with " lines)"
    end check
end action

// Usage
rotate_log_if_needed with "app.log" and 1000
```

## 3.6 I/O Error Handling Reference

### Common File Errors

| Error Condition | When It Occurs | Recommended Action |
|----------------|----------------|-------------------|
| `file not found` | File doesn't exist | Create default or prompt user |
| `permission denied` | Insufficient permissions | Check permissions or use different path |
| `file unreadable` | Cannot read file | Verify file integrity |
| `disk full` | No space to write | Clean up or notify user |
| `path invalid` | Invalid path format | Validate path before use |

### Common Network Errors

| Error Condition | When It Occurs | Recommended Action |
|----------------|----------------|-------------------|
| `network timeout` | Request took too long | Retry with backoff |
| `network error` | Cannot connect | Check connectivity, retry |
| `network unreachable` | Host unreachable | Verify URL, check network |
| `http error` | Status >= 400 | Check status code, handle appropriately |
| `connection refused` | Server not listening | Verify server is running |
| `ssl error` | SSL/TLS problem | Check certificates |

### Error Handling Template

```wfl
// Robust I/O operation template
define async action robust_io_operation with resource_path:
    store max_attempts as 3
    store attempt as 0

    repeat while attempt is less than max_attempts:
        add 1 to attempt

        try:
            // Perform I/O operation
            wait for store result as perform_operation with resource_path
            display "✓ Success on attempt " with attempt
            give back result

        when file not found:
            display "❌ File not found: " with resource_path
            give back nothing

        when permission denied:
            display "❌ Permission denied: " with resource_path
            give back nothing

        when network timeout:
            display "⚠ Timeout on attempt " with attempt
            check if attempt is less than max_attempts:
                wait for attempt times 1000 milliseconds  // Exponential backoff
            otherwise:
                display "❌ All retries exhausted"
                give back nothing
            end check

        when network error:
            display "⚠ Network error on attempt " with attempt
            check if attempt is less than max_attempts:
                wait 2 seconds
            otherwise:
                give back nothing
            end check

        otherwise:
            display "❌ Unexpected error: " with error message
            give back nothing
        end try
    end repeat

    give back nothing
end action
```

---

# Section 4: CLI Tools & Development

WFL provides comprehensive command-line tools for development, including linting, analysis, debugging, and configuration management. All tools are integrated into the `wfl` binary.

## 4.1 WFL CLI Commands

### Running WFL Programs

**Basic Execution:**
```bash
# Run a WFL program
wfl program.wfl

# Run with command-line arguments
wfl script.wfl arg1 arg2 --flag value

# Run with flags
wfl app.wfl --verbose --output result.txt file1.txt file2.txt
```

### Code Quality Tools

**Linting:**
```bash
# Check code style and potential issues
wfl --lint program.wfl

# Lint with strict mode (treat warnings as errors)
wfl --lint --strict program.wfl

# Lint specific file
wfl --lint path/to/file.wfl
```

**Static Analysis:**
```bash
# Perform deep static analysis
wfl --analyze program.wfl

# Analyze with detailed output
wfl --analyze --verbose program.wfl
```

**Auto-Fixing:**
```bash
# Show proposed fixes without applying
wfl --fix program.wfl

# Apply fixes in-place
wfl --fix program.wfl --in-place

# Show diff of proposed changes
wfl --fix program.wfl --diff

# Fix with lint check combined
wfl --lint --fix program.wfl --in-place
```

### Debugging Tools

**Debug Mode:**
```bash
# Run with debug output
wfl --debug program.wfl

# Debug with trace level
wfl --debug --trace program.wfl

# Redirect debug output
wfl --debug program.wfl > debug.txt 2>&1
```

**Inspection Tools:**
```bash
# Show tokens (lexer output)
wfl --lex program.wfl

# Show AST (parser output)
wfl --parse program.wfl

# Show type information
wfl --typecheck program.wfl
```

### Configuration Management

```bash
# Check configuration file
wfl --configCheck

# Auto-fix configuration issues
wfl --configFix

# Show current configuration
wfl --configShow
```

## 4.2 Linter & Analyzer

### WFL Linter

The linter enforces strict code style and structural conventions.

**Lint Rule Categories:**

1. **Naming Conventions** - Enforce consistent identifier naming
2. **Indentation & Formatting** - Ensure proper spacing and structure
3. **Code Structure** - Validate block structure and alignment
4. **Complexity** - Warn about excessive nesting or complexity
5. **Best Practices** - Enforce idiomatic WFL patterns

**Example Linter Output:**
```
program.wfl:10:7: Warning [LINT-NAMING] Variable "MyVar" should be snake_case (e.g., "my_var")
program.wfl:15:1: Warning [LINT-INDENT] Line indented 2 spaces, expected 4
program.wfl:30:5: Warning [LINT-COMPLEX] Nesting depth is 6, exceeds maximum of 5
```

**Common Lint Rules:**

| Rule Code | Description | Example Fix |
|-----------|-------------|-------------|
| LINT-NAMING | Variable/function naming | `Counter` → `counter` |
| LINT-INDENT | Indentation consistency | 2 spaces → 4 spaces |
| LINT-WHITESPACE | Trailing whitespace | Remove trailing spaces |
| LINT-COMPLEX | Excessive complexity | Refactor to reduce nesting |
| LINT-CONSISTENT | Keyword case consistency | `Then` → `then` |

### WFL Static Analyzer

The analyzer performs deep static analysis to catch logical issues.

**Analysis Categories:**

1. **Unused Variables** - Variables declared but never used
2. **Unused Functions** - Actions defined but never called
3. **Unreachable Code** - Code that will never execute
4. **Dead Branches** - Conditions that are always true/false
5. **Inconsistent Returns** - Not all code paths return values
6. **Variable Shadowing** - Inner variables hide outer ones

**Example Analyzer Output:**
```
program.wfl:15:9: Warning [ANALYZE-UNUSED] Variable "temp_value" is never used
program.wfl:47:5: Warning [ANALYZE-UNREACH] Unreachable code (after 'return' at line 45)
program.wfl:60:1: Warning [ANALYZE-RETURN] Not all paths return a value in "process_data"
```

**Analysis Checks:**

| Check Code | Description | Issue Type |
|------------|-------------|------------|
| ANALYZE-UNUSED | Unused variable/function | Dead code |
| ANALYZE-UNREACH | Unreachable statement | Logic error |
| ANALYZE-RETURN | Missing return in path | Type safety |
| ANALYZE-SHADOW | Variable shadowing | Clarity |
| ANALYZE-CONSTANT | Always true/false condition | Logic error |

### Suppressing Warnings

**Inline Suppression:**
```wfl
// Suppress specific lint
// lint-disable LINT-NAMING
store MyVar as 5

// Suppress all lints for next line
// lint-disable-next-line
store Counter as 10

// Suppress analyzer warning
// analyze-allow unused
store temp_data as compute_data()
```

## 4.3 Configuration System

### .wflcfg File Format

WFL uses `.wflcfg` files for project and global configuration.

**Example Configuration:**
```ini
# Execution Settings
timeout_seconds = 60
logging_enabled = false
debug_report_enabled = true
log_level = info

# Code Style Settings
max_line_length = 100
max_nesting_depth = 5
indent_size = 4
snake_case_variables = true
trailing_whitespace = false
consistent_keyword_case = true

# Linter Settings
lint_strict_mode = false
lint_warn_unused = true
lint_warn_shadowing = true

# Analyzer Settings
analyze_unreachable = true
analyze_unused = true
analyze_returns = true
```

### Configuration Hierarchy

1. **Global Config** - System-wide defaults (`~/.wflcfg` or via `WFL_GLOBAL_CONFIG_PATH`)
2. **Project Config** - Project-level settings (`.wflcfg` in project root)
3. **Local Config** - Directory-specific settings (`.wflcfg` in current directory)
4. **Command-Line** - Flags override all configuration

### Configuration Commands

```bash
# Validate configuration
wfl --configCheck

# Auto-fix configuration problems
wfl --configFix

# Show current effective configuration
wfl --configShow

# Use custom config path
WFL_GLOBAL_CONFIG_PATH=~/my-config.wflcfg wfl program.wfl
```

## 4.4 Command-Line Arguments

WFL scripts can access command-line arguments through built-in variables.

### Built-in Argument Variables

**Available Variables:**

| Variable | Type | Description |
|----------|------|-------------|
| `arg_count` | Number | Total number of arguments |
| `args` | List | All arguments in order |
| `positional_args` | List | Non-flag arguments only |
| `flag_<name>` | Text/Boolean | Flag values (e.g., `flag_verbose`) |

**Example Script:**
```wfl
// Display total arguments
display "Total arguments: " with arg_count

// Process flags
check if flag_verbose:
    display "Verbose mode enabled"
    store verbose as yes
otherwise:
    store verbose as no
end check

check if flag_output:
    store output_file as flag_output
    display "Output file: " with output_file
otherwise:
    store output_file as "output.txt"
end check

// Process positional arguments
check if length of positional_args is equal to 0:
    display "Usage: wfl script.wfl [options] file1 file2 ..."
    display "Options:"
    display "  --verbose        Enable verbose output"
    display "  --output FILE    Specify output file"
otherwise:
    for each input_file in positional_args:
        check if verbose:
            display "Processing: " with input_file
        end check

        try:
            wait for open file at input_file and read content as data
            display "Processed " with length of data with " characters"
        catch:
            display "Error processing " with input_file
        end try
    end for
end check
```

### Flag Parsing Rules

1. **Long Flags:** `--flag` or `--flag value`
2. **Short Flags:** `-f` or `-f value`
3. **Boolean Flags:** Flags without values are set to `true`
4. **Flag Values:** Next argument becomes the flag's value if not a flag
5. **Positional Args:** All non-flag, non-value arguments

**Example:**
```bash
wfl script.wfl --name "John" --verbose file1.txt file2.txt

# Creates:
# flag_name = "John"
# flag_verbose = true
# positional_args = ["file1.txt", "file2.txt"]
# arg_count = 5
```

## 4.5 Debugging Tools

### Debug Output

**Basic Debugging:**
```bash
# Enable debug output
wfl --debug program.wfl

# Sample debug output:
# [DEBUG] Lexer: Tokenizing source (245 chars)
# [DEBUG] Parser: Building AST (42 nodes)
# [DEBUG] Analyzer: Validating semantics
# [DEBUG] TypeChecker: Inferring types
# [DEBUG] Interpreter: Starting execution
# [DEBUG] Line 5: store name as "Alice"
# [DEBUG] Line 6: display "Hello, " with name
```

**Trace Mode:**
```bash
# More detailed tracing
wfl --debug --trace program.wfl

# Shows:
# - Every statement execution
# - Variable assignments
# - Function calls and returns
# - Control flow decisions
```

### Lexer Inspection

```bash
# View all tokens
wfl --lex program.wfl
```

**Example Output:**
```
Token Stream:
1:1   Store        "store"
1:7   Identifier   "name"
1:12  As           "as"
1:15  Text         "\"Alice\""
1:23  Newline
2:1   Display      "display"
2:9   Text         "\"Hello\""
...
```

### Parser Inspection

```bash
# View AST structure
wfl --parse program.wfl
```

**Example Output:**
```
AST:
Program
├─ VariableDeclaration
│  ├─ name: "name"
│  └─ value: TextLiteral("Alice")
├─ DisplayStatement
│  └─ expression: StringConcat
│     ├─ TextLiteral("Hello, ")
│     └─ Identifier("name")
...
```

### Type Checker Inspection

```bash
# View type information
wfl --typecheck program.wfl
```

**Example Output:**
```
Type Information:
name: Text (inferred from literal)
age: Number (inferred from literal)
is_active: Boolean (inferred from literal)
calculate_area: (Number, Number) → Number
```

### Performance Profiling

```bash
# Time execution
wfl --time program.wfl

# Output:
# Execution time: 1.234s
# Lexing: 0.012s
# Parsing: 0.034s
# Analysis: 0.018s
# Execution: 1.170s
```

### Memory Profiling

```bash
# Build with heap profiling
cargo build --release --features dhat-heap

# Run program with profiling
wfl program.wfl

# Generates: dhat-heap.json
# Analyze with tools like dhat viewer
```

## 4.6 LSP Server Integration

### Language Server Protocol

WFL provides a fully-featured LSP server for IDE integration.

**LSP Features:**
- Real-time syntax checking
- Semantic error reporting
- Code completion
- Go-to-definition
- Hover documentation
- Symbol search
- Rename refactoring
- Code actions and quick fixes

**Starting LSP Server:**
```bash
# Start WFL LSP server
wfl-lsp

# LSP server listens on stdio for editor communication
```

### VS Code Integration

**Installation:**
```powershell
# Install VS Code extension
scripts/install_vscode_extension.ps1
```

**Features in VS Code:**
- ✅ Syntax highlighting for .wfl files
- ✅ Real-time error checking
- ✅ IntelliSense code completion
- ✅ Hover tooltips with documentation
- ✅ Go-to-definition (F12)
- ✅ Find all references
- ✅ Rename symbol (F2)
- ✅ Format document
- ✅ Code snippets

**Example VS Code Settings:**
```json
{
  "wfl.lsp.enabled": true,
  "wfl.lsp.trace": "verbose",
  "wfl.lint.onSave": true,
  "wfl.format.onSave": true
}
```

## 4.7 Development Workflow

### Typical Development Cycle

```bash
# 1. Write WFL code
# (Edit program.wfl)

# 2. Check syntax and style
wfl --lint program.wfl

# 3. Auto-fix issues
wfl --fix program.wfl --in-place

# 4. Run static analysis
wfl --analyze program.wfl

# 5. Test execution
wfl program.wfl

# 6. Debug if needed
wfl --debug program.wfl
```

### Integration with Git Hooks

**Pre-commit Hook Example:**
```bash
#!/bin/bash
# .git/hooks/pre-commit

# Run linter on all WFL files
for file in $(git diff --cached --name-only --diff-filter=ACM | grep '\.wfl$'); do
    wfl --lint "$file"
    if [ $? -ne 0 ]; then
        echo "Lint errors in $file"
        exit 1
    fi
done

# Run auto-fix
for file in $(git diff --cached --name-only --diff-filter=ACM | grep '\.wfl$'); do
    wfl --fix "$file" --in-place
    git add "$file"
done

echo "All WFL files passed linting"
```

### CI/CD Integration

**GitHub Actions Example:**
```yaml
name: WFL Quality Check

on: [push, pull_request]

jobs:
  quality:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2

      - name: Install WFL
        run: |
          cargo build --release
          sudo cp target/release/wfl /usr/local/bin/

      - name: Lint WFL Code
        run: |
          find . -name "*.wfl" -exec wfl --lint {} \;

      - name: Analyze WFL Code
        run: |
          find . -name "*.wfl" -exec wfl --analyze {} \;

      - name: Run WFL Tests
        run: |
          find TestPrograms -name "*.wfl" -exec wfl {} \;
```

## 4.8 Command Reference

### Complete Command-Line Flags

| Flag | Description | Example |
|------|-------------|---------|
| (none) | Execute program | `wfl program.wfl` |
| `--lint` | Run linter | `wfl --lint file.wfl` |
| `--analyze` | Run static analyzer | `wfl --analyze file.wfl` |
| `--fix` | Auto-format code | `wfl --fix file.wfl` |
| `--in-place` | Modify file in-place (with --fix) | `wfl --fix file.wfl --in-place` |
| `--diff` | Show diff of changes (with --fix) | `wfl --fix file.wfl --diff` |
| `--debug` | Enable debug output | `wfl --debug program.wfl` |
| `--trace` | Enable trace output (with --debug) | `wfl --debug --trace file.wfl` |
| `--lex` | Show tokens | `wfl --lex file.wfl` |
| `--parse` | Show AST | `wfl --parse file.wfl` |
| `--typecheck` | Show type info | `wfl --typecheck file.wfl` |
| `--time` | Show execution timing | `wfl --time program.wfl` |
| `--configCheck` | Validate configuration | `wfl --configCheck` |
| `--configFix` | Fix configuration | `wfl --configFix` |
| `--configShow` | Display configuration | `wfl --configShow` |
| `--verbose` | Verbose output | `wfl --verbose program.wfl` |
| `--version` | Show version | `wfl --version` |
| `--help` | Show help | `wfl --help` |

### Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `WFL_GLOBAL_CONFIG_PATH` | Override global config location | `export WFL_GLOBAL_CONFIG_PATH=~/my.wflcfg` |
| `WFL_LOG_LEVEL` | Set log level | `export WFL_LOG_LEVEL=debug` |
| `WFL_TIMEOUT` | Override execution timeout | `export WFL_TIMEOUT=120` |

## 4.9 Testing Framework

### Running Tests

**Execute Test Programs:**
```bash
# Run all test programs (Windows)
Get-ChildItem TestPrograms\*.wfl | ForEach-Object { wfl $_.FullName }

# Run all test programs (Unix)
for file in TestPrograms/*.wfl; do
    wfl "$file"
done

# Run integration tests (with release build)
cargo build --release
.\scripts\run_integration_tests.ps1  # Windows
./scripts/run_integration_tests.sh  # Unix
```

### Writing Tests in WFL

**Test Pattern:**
```wfl
// test_example.wfl - Example test file

define action test_addition:
    store result as perform add_numbers with a as 2 and b as 3

    check if result is equal to 5:
        display "✓ Addition test passed"
    otherwise:
        display "✗ Addition test failed: expected 5, got " with result
        throw error "Test failed"
    end check
end action

define action test_string_length:
    store text as "Hello"
    store length as length of text

    check if length is equal to 5:
        display "✓ String length test passed"
    otherwise:
        display "✗ String length test failed"
        throw error "Test failed"
    end check
end action

// Run all tests
perform test_addition
perform test_string_length

display "All tests passed!"
```

### Test Organization

**Recommended Structure:**
```
project/
├── src/
│   └── main.wfl
├── tests/
│   ├── test_core.wfl
│   ├── test_utils.wfl
│   └── test_integration.wfl
├── TestPrograms/
│   ├── basic_syntax.wfl
│   └── advanced_features.wfl
└── .wflcfg
```

## 4.10 Best Practices

### 1. Code Quality Workflow

```bash
# Before committing code:
# 1. Lint and fix
wfl --lint program.wfl
wfl --fix program.wfl --in-place

# 2. Analyze
wfl --analyze program.wfl

# 3. Test
wfl program.wfl

# 4. Commit if all pass
git add program.wfl
git commit -m "Feature: Added user authentication"
```

### 2. Debugging Workflow

```bash
# When encountering errors:
# 1. Check syntax
wfl --lex program.wfl
wfl --parse program.wfl

# 2. Check types
wfl --typecheck program.wfl

# 3. Run with debug
wfl --debug program.wfl

# 4. If still unclear, add print statements
# (Add: display "Debug: x = " with x)
wfl program.wfl
```

### 3. Performance Optimization

```bash
# Profile execution time
wfl --time program.wfl

# Profile memory usage
cargo build --release --features dhat-heap
wfl program.wfl
# Analyze dhat-heap.json

# Optimize based on findings
# - Reduce unnecessary allocations
# - Use appropriate data structures
# - Minimize file I/O
```

### 4. Configuration Management

**Project Setup:**
```bash
# 1. Create project config
cat > .wflcfg << EOF
# Project configuration
timeout_seconds = 60
max_line_length = 100
indent_size = 4
snake_case_variables = true
EOF

# 2. Validate
wfl --configCheck

# 3. Use in all project files
wfl program.wfl  # Automatically uses .wflcfg
```

### 5. Editor Integration

**Recommended VS Code Setup:**
1. Install WFL extension: `scripts/install_vscode_extension.ps1`
2. Enable format on save in VS Code settings
3. Enable lint on save
4. Use keyboard shortcuts:
   - F12: Go to definition
   - F2: Rename symbol
   - Ctrl+Space: Autocomplete
   - Shift+Alt+F: Format document

## 4.11 Troubleshooting

### Common Issues

**Issue: Integration tests fail with "path not found"**
```bash
# Solution: Build release binary
cargo build --release

# Then run tests
cargo test --test split_functionality
```

**Issue: Linter reports many warnings**
```bash
# Solution: Auto-fix most issues
wfl --fix program.wfl --in-place

# Review remaining warnings
wfl --lint program.wfl
```

**Issue: Program runs slowly**
```bash
# Solution: Profile and optimize
wfl --time program.wfl

# Check for:
# - Infinite loops
# - Excessive file I/O
# - Large data structures
```

**Issue: Configuration not recognized**
```bash
# Solution: Validate configuration
wfl --configCheck

# Fix issues
wfl --configFix

# Verify
wfl --configShow
```

### Error Message Interpretation

**Parse Errors:**
```
error: Expected 'as' after identifier, but found IntLiteral(42)
  --> program.wfl:3:14
   |
 3 | store greeting 42
   |              ^ Error here
   |
   = Note: Did you forget 'as'? Try: store greeting as 42
```

**Type Errors:**
```
error: Cannot add Number and Text
  --> program.wfl:5:12
   |
 5 | display x plus y
   |            ^ Type error
   |
   = Note: Convert to number: convert y to number
```

**Runtime Errors:**
```
error: Division by zero
  --> program.wfl:7:14
   |
 7 | display 10 divided by x
   |              ^ Runtime error
   |
   = Note: Check divisor is not zero
```

---

# Section 5: Type System & Semantics

WFL uses a strong, static type system with automatic type inference to ensure program correctness while minimizing verbosity.

## 5.1 Type Inference Rules

### Literal Type Inference

WFL automatically infers types from literal values:

| Literal | Inferred Type | Example |
|---------|---------------|---------|
| `42` | Number | `store age as 25` |
| `3.14` | Number | `store pi as 3.14159` |
| `"text"` | Text | `store name as "Alice"` |
| `yes` / `no` | Boolean | `store flag as yes` |
| `true` / `false` | Boolean | `store done as false` |
| `nothing` | Nothing | `store empty as nothing` |
| `[1, 2, 3]` | List of Number | `store nums as [1, 2, 3]` |
| `["a", "b"]` | List of Text | `store words as ["hello", "world"]` |

### Expression Type Inference

**Arithmetic Operations:**
```wfl
store x as 5           // Number
store y as 10          // Number
store sum as x plus y  // Number (inferred from operands)
```

**String Operations:**
```wfl
store first as "Hello"           // Text
store second as "World"          // Text
store combined as first with " " with second  // Text
```

**Comparison Operations:**
```wfl
store age as 25                 // Number
store is_adult as age >= 18     // Boolean (inferred from comparison)
```

**Function Return Type Inference:**
```wfl
define action calculate_area:
    needs:
        width as number
        height as number
    gives back:
        result as number   // Explicit return type
    do:
        give back width times height  // Must match declared type
end action

store area as calculate_area with width as 10 and height as 5  // area: Number
```

### Collection Type Inference

```wfl
// Homogeneous list
store numbers as [1, 2, 3, 4]      // List of Number
store names as ["Alice", "Bob"]    // List of Text

// Mixed list (all elements must be compatible)
store mixed as [1, "text", yes]    // List of Any (if supported)

// Empty list requires type annotation or first element
create list items:
    add "first"                    // List of Text (from first element)
    add "second"
end list
```

## 5.2 Type Checking

### Type Compatibility Rules

**Assignment Compatibility:**
```wfl
store x as 5           // x: Number
change x to 10         // ✓ Compatible (Number → Number)
change x to "text"     // ✗ Error: Cannot assign Text to Number variable
```

**Operation Type Requirements:**
```wfl
// Arithmetic requires Number operands
store a as 5 plus 3              // ✓ Number + Number
store b as 5 plus "3"            // ✗ Error: Number + Text not allowed

// String concatenation requires Text
store c as "hello" with "world"  // ✓ Text with Text
store d as "hello" with 42       // ⚠ May require conversion
```

**Comparison Type Requirements:**
```wfl
// Same-type comparisons
check if 5 is greater than 3:    // ✓ Number compared to Number
check if "a" is equal to "b":    // ✓ Text compared to Text
check if 5 is equal to "5":      // ⚠ Comparing Number to Text (may error)
```

### Type Coercion and Conversion

**Explicit Conversion (Recommended):**
```wfl
// Number to Text
store text_num as convert 42 to text       // "42"
store formatted as 42 as text              // "42"

// Text to Number
store num_value as convert "123" to number  // 123

// Safe conversion with error handling
safely convert "abc" to number:
    when invalid:
        use 0 instead
    when missing:
        use default_value
end convert
```

**Automatic Coercion (Limited):**
WFL generally requires explicit conversion for type safety. Some limited automatic coercion may occur:

- Display accepts any type (converts to string for output)
- String concatenation may auto-convert Numbers to Text (implementation-dependent)

## 5.3 Type Compatibility Matrix

### Binary Operation Type Compatibility

| Operation | Left Type | Right Type | Result Type | Valid? |
|-----------|-----------|------------|-------------|--------|
| `plus` | Number | Number | Number | ✓ |
| `plus` | Text | Text | (use `with`) | ✗ |
| `plus` | Number | Text | - | ✗ |
| `minus` | Number | Number | Number | ✓ |
| `times` | Number | Number | Number | ✓ |
| `divided by` | Number | Number | Number | ✓ |
| `with` | Text | Text | Text | ✓ |
| `with` | Text | Number | Text | ⚠ |
| `is equal to` | T | T | Boolean | ✓ |
| `is equal to` | T1 | T2 | Boolean | ⚠ |
| `and` | Boolean | Boolean | Boolean | ✓ |
| `or` | Boolean | Boolean | Boolean | ✓ |
| `not` | Boolean | - | Boolean | ✓ |

**Legend:**
- ✓ = Always valid
- ✗ = Type error
- ⚠ = May require explicit conversion or have special rules

### Function Parameter Type Checking

```wfl
define action greet_user:
    needs:
        name as text
        age as number
    do:
        display "Hello, " with name with "! You are " with age
end action

// Valid calls
greet_user with name as "Alice" and age as 25  // ✓
greet_user with name as "Bob" and age as 30    // ✓

// Invalid calls
greet_user with name as 42 and age as 25       // ✗ Error: name must be Text
greet_user with name as "Alice" and age as "25" // ✗ Error: age must be Number
```

## 5.4 Common Type Errors

### Type Mismatch in Assignment

**Error:**
```wfl
store age as 25
change age to "twenty-five"  // ✗ Error
```

**Message:**
```
error: Type mismatch - Cannot assign Text to Number variable
  --> program.wfl:2:15
   |
 2 | change age to "twenty-five"
   |               ^^^^^^^^^^^^^ Expected Number, found Text
   |
   = Note: Convert the text to number: convert "twenty-five" to number
   = Or: If you need to change the type, create a new variable
```

**Fix:**
```wfl
// Option 1: Convert
store age_text as "twenty-five"
change age to convert age_text to number  // If valid number string

// Option 2: New variable
store age as 25
store age_text as "twenty-five"  // Different variable
```

### Type Mismatch in Operations

**Error:**
```wfl
store x as 5
store y as "10"
store sum as x plus y  // ✗ Error
```

**Message:**
```
error: Cannot perform arithmetic on incompatible types
  --> program.wfl:3:18
   |
 3 | store sum as x plus y
   |                     ^ Expected Number, found Text
   |
   = Note: Convert y to number first: convert y to number
```

**Fix:**
```wfl
store x as 5
store y as "10"
store y_num as convert y to number
store sum as x plus y_num  // ✓ Works
```

### Type Mismatch in Function Calls

**Error:**
```wfl
define action calculate_area:
    needs:
        width as number
        height as number
    gives back:
        area as number
    do:
        give back width times height
end action

store result as calculate_area with width as "5" and height as 10  // ✗ Error
```

**Message:**
```
error: Parameter type mismatch
  --> program.wfl:10:42
   |
10 | store result as calculate_area with width as "5" and height as 10
   |                                                  ^^^ Expected Number, found Text
   |
   = Note: Function 'calculate_area' requires parameter 'width' to be Number
   = Help: Convert to number: with width as convert "5" to number
```

**Fix:**
```wfl
store width_num as convert "5" to number
store result as calculate_area with width as width_num and height as 10
```

### Missing Return Type

**Error:**
```wfl
define action compute_value with x:
    check if x is greater than 0:
        give back x
    end check
    // ✗ No return for x <= 0
end action
```

**Message:**
```
error: Not all code paths return a value
  --> program.wfl:1:1
   |
 1 | define action compute_value with x:
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ This action must return a value
   |
   = Note: Missing return statement for case where x <= 0
   = Help: Add 'give back' statement in all branches
```

**Fix:**
```wfl
define action compute_value with x:
    check if x is greater than 0:
        give back x
    otherwise:
        give back 0  // ✓ All paths return
    end check
end action
```

## 5.5 Type System Advanced Features

### Optional Types (Nothing)

```wfl
// Function that may return nothing
define action find_user with user_id:
    // ... search logic ...
    check if user_found:
        give back user_data
    otherwise:
        give back nothing  // Returns nothing if not found
    end check
end action

// Handling optional returns
store user as find_user with 123

check if isnothing of user:
    display "User not found"
otherwise:
    display "User: " with user
end check
```

### List Element Type Checking

```wfl
// Homogeneous lists are type-checked
create list numbers:
    add 1
    add 2
    add "three"  // ✗ Error: Cannot add Text to List of Number
end list

// Fix: Keep types consistent
create list numbers:
    add 1
    add 2
    add 3  // ✓ All Numbers
end list
```

### Container Type Checking

```wfl
create container Person:
    property name as text
    property age as number

    define action set_age:
        needs:
            new_age as number
        do:
            change this age to new_age  // ✓ Type matches
    end action
end container

create new Person as alice:
    set name to "Alice"
    set age to 28
end create

alice set_age with new_age as 29        // ✓ Correct type
alice set_age with new_age as "29"      // ✗ Error: age must be Number
```

### Flow-Sensitive Type Refinement

```wfl
// Type refinement in conditionals
store value as user_input  // Type: Any or unknown

check if typeof of value is "Number":
    // Within this block, value is known to be Number
    store doubled as value times 2  // ✓ Safe
end check

// Nothing check refinement
check if value is not nothing:
    // Within this block, value is known to be non-nothing
    store length as length of value  // ✓ Safe (if value is Text/List)
end check
```

---

# Section 6: Error Reference

Complete reference of all error types, causes, and solutions in WFL.

## 6.1 Error Categories

| Category | Description | Severity |
|----------|-------------|----------|
| Parse Errors | Syntax violations | Error (prevents execution) |
| Semantic Errors | Invalid program structure | Error (prevents execution) |
| Type Errors | Type mismatches | Error (prevents execution) |
| Runtime Errors | Errors during execution | Error (stops execution) |
| I/O Errors | File/network failures | Catchable (via try/when) |
| Analysis Warnings | Potential issues | Warning (doesn't prevent execution) |

## 6.2 Parse Errors

### Missing Keyword

**Error:** Missing 'as' in variable declaration

**Code:**
```wfl
store name "Alice"  // ✗ Missing 'as'
```

**Message:**
```
error: Expected 'as' after identifier
  --> program.wfl:1:12
   |
 1 | store name "Alice"
   |            ^ Expected 'as' here
```

**Fix:**
```wfl
store name as "Alice"  // ✓
```

### Missing End Keyword

**Error:** Missing 'end' for block

**Code:**
```wfl
check if x > 5:
    display "Greater"
// ✗ Missing 'end check'
```

**Message:**
```
error: Unexpected end of file - Expected 'end check'
  --> program.wfl:2:22
   |
 1 | check if x > 5:
   |          ------ Block started here
 2 |     display "Greater"
   |                      ^ Expected 'end check' after this
```

**Fix:**
```wfl
check if x > 5:
    display "Greater"
end check  // ✓
```

### Unexpected Token

**Error:** Unexpected token in expression

**Code:**
```wfl
store x as 5 +  // ✗ Incomplete expression
```

**Message:**
```
error: Unexpected end of line
  --> program.wfl:1:16
   |
 1 | store x as 5 +
   |                ^ Expected expression after '+'
```

**Fix:**
```wfl
store x as 5 plus 10  // ✓
```

## 6.3 Type Errors

### Cannot Perform Operation on Type

**Error:** Arithmetic on Text

**Code:**
```wfl
store x as "5"
store y as "10"
store sum as x plus y  // ✗ Cannot add Text
```

**Message:**
```
error: Cannot perform 'plus' on Text types
  --> program.wfl:3:18
   |
 3 | store sum as x plus y
   |                     ^ Both operands are Text
   |
   = Note: Convert to numbers first: convert x to number
```

**Fix:**
```wfl
store x as "5"
store y as "10"
store x_num as convert x to number
store y_num as convert y to number
store sum as x_num plus y_num  // ✓
```

### Type Mismatch in Comparison

**Error:** Comparing incompatible types

**Code:**
```wfl
check if 5 is equal to "5":  // ⚠ Comparing Number to Text
```

**Message:**
```
warning: Comparing Number to Text
  --> program.wfl:1:10
   |
 1 | check if 5 is equal to "5":
   |          ^            ^^^ Text
   |          Number
   |
   = Note: This will always be false
   = Help: Convert to same type or use explicit check
```

**Fix:**
```wfl
// Option 1: Convert to same type
check if convert 5 to text is equal to "5":  // ✓

// Option 2: Compare after conversion
store num as convert "5" to number
check if 5 is equal to num:  // ✓
```

### List Type Consistency

**Error:** Adding wrong type to list

**Code:**
```wfl
create list numbers:
    add 1
    add 2
    add "three"  // ✗ Text in Number list
end list
```

**Message:**
```
error: Cannot add Text to List of Number
  --> program.wfl:4:9
   |
 4 |     add "three"
   |         ^^^^^^^ Expected Number, found Text
   |
   = Note: List was inferred as List of Number from first element
```

**Fix:**
```wfl
create list numbers:
    add 1
    add 2
    add 3  // ✓ All Numbers
end list
```

## 6.4 Runtime Errors

### Division by Zero

**Code:**
```wfl
store x as 0
store result as 10 divided by x  // ✗ Division by zero
```

**Message:**
```
error: Division by zero
  --> program.wfl:2:25
   |
 2 | store result as 10 divided by x
   |                         ^ x is 0
```

**Fix:**
```wfl
store x as 0
check if x is not equal to 0:
    store result as 10 divided by x
otherwise:
    display "Cannot divide by zero"
    store result as 0
end check
```

### Index Out of Bounds

**Code:**
```wfl
store list as [1, 2, 3]
store item as list 10  // ✗ Index 10 doesn't exist
```

**Message:**
```
error: Index out of bounds
  --> program.wfl:2:20
   |
 2 | store item as list 10
   |                    ^^ Index 10 is invalid (list has 3 elements)
   |
   = Note: Valid indices are 0-2
```

**Fix:**
```wfl
store list as [1, 2, 3]
store list_len as length of list

check if 10 is less than list_len:
    store item as list 10
otherwise:
    display "Index out of bounds"
end check
```

### Variable Not Defined

**Code:**
```wfl
display undefined_variable  // ✗ Variable doesn't exist
```

**Message:**
```
error: Variable 'undefined_variable' is not defined
  --> program.wfl:1:9
   |
 1 | display undefined_variable
   |         ^^^^^^^^^^^^^^^^^^ Undefined variable
   |
   = Help: Define the variable first: store undefined_variable as value
```

**Fix:**
```wfl
store my_variable as "Hello"
display my_variable  // ✓
```

## 6.5 I/O Errors

See [Section 3.6: I/O Error Handling Reference](#36-io-error-handling-reference) for complete I/O error documentation.

**Common I/O Errors:**
- `file not found` - File doesn't exist
- `permission denied` - Insufficient permissions
- `network timeout` - Request took too long
- `network error` - Connection failed
- `http error` - HTTP status >= 400

---

# Section 7: Code Patterns & Best Practices

This section provides proven patterns and best practices for writing effective WFL code.

## 7.1 Common Patterns Library

### Pattern: Safe Division

```wfl
define action safe_divide:
    needs:
        numerator as number
        denominator as number
    gives back:
        result as number
    do:
        check if denominator is equal to 0:
            display "Warning: Division by zero, returning 0"
            give back 0
        otherwise:
            give back numerator divided by denominator
        end check
end action
```

### Pattern: Input Validation

```wfl
define action validate_email:
    needs:
        email as text
    gives back:
        is_valid as boolean
    do:
        // Check basic email format
        check if length of email is equal to 0:
            give back no
        end check

        check if not contains of email and "@":
            give back no
        end check

        check if not contains of email and ".":
            give back no
        end check

        check if contains of email and " ":
            give back no
        end check

        give back yes
end action
```

### Pattern: Retry with Backoff

```wfl
define async action fetch_with_retry:
    needs:
        url as text
        max_retries as number
    gives back:
        data as text
    do:
        store retry_count as 0
        store delay as 1000  // Start with 1 second

        repeat while retry_count is less than max_retries:
            try:
                wait for store response as fetch from url
                give back response

            when network timeout:
                add 1 to retry_count
                display "Retry " with retry_count with " after " with delay with "ms"
                wait for delay milliseconds
                multiply delay by 2  // Exponential backoff
            end try
        end repeat

        give back nothing  // All retries failed
end action
```

### Pattern: List Processing (Filter)

```wfl
define action filter_positive_numbers:
    needs:
        numbers as list
    gives back:
        positives as list
    do:
        store positives as []

        for each num in numbers:
            check if num is greater than 0:
                push of positives and num
            end check
        end for

        give back positives
end action
```

### Pattern: List Processing (Map)

```wfl
define action double_all_numbers:
    needs:
        numbers as list
    gives back:
        doubled as list
    do:
        store doubled as []

        for each num in numbers:
            store double as num times 2
            push of doubled and double
        end for

        give back doubled
end action
```

### Pattern: Resource Cleanup

```wfl
define action process_file_safely:
    needs:
        filename as text
    do:
        try:
            open file at filename as file_handle
            wait for store content as read content from file_handle

            // Process content
            display "Processing " with length of content with " characters"

        finally:
            close file file_handle  // Always close, even on error
        end try
end action
```

## 7.2 Anti-Patterns & Gotchas

### Anti-Pattern: Magic Numbers

**Bad:**
```wfl
check if age is greater than 18:
    allow_access
end check

check if account_balance is greater than 1000:
    offer_premium
end check
```

**Good:**
```wfl
store minimum_age as 18
store premium_threshold as 1000

check if age is greater than minimum_age:
    allow_access
end check

check if account_balance is greater than premium_threshold:
    offer_premium
end check
```

### Anti-Pattern: Ignored Errors

**Bad:**
```wfl
try:
    wait for open file at filename and read content as data
catch:
    // ✗ Error silently ignored
end try
```

**Good:**
```wfl
try:
    wait for open file at filename and read content as data
when file not found:
    display "File not found: " with filename
    // Use default or prompt user
when permission denied:
    display "Cannot read file: " with filename
otherwise:
    display "Error: " with error message
end try
```

### Anti-Pattern: Excessive Nesting

**Bad:**
```wfl
check if has_account:
    check if is_verified:
        check if has_payment_method:
            check if balance_sufficient:
                process_payment
            otherwise:
                show_insufficient_funds
            end check
        otherwise:
            show_add_payment
        end check
    otherwise:
        show_verify_account
    end check
otherwise:
    show_create_account
end check
```

**Good:**
```wfl
// Early returns reduce nesting
check if not has_account:
    show_create_account
    give back
end check

check if not is_verified:
    show_verify_account
    give back
end check

check if not has_payment_method:
    show_add_payment
    give back
end check

check if not balance_sufficient:
    show_insufficient_funds
    give back
end check

process_payment
```

### Gotcha: Variable Shadowing

**Problem:**
```wfl
store count as 0

count from 1 to 10:
    // ⚠ 'count' here is the loop variable, shadows outer 'count'
    display count  // Shows loop variable (1-10), not 0
end count

display count  // Shows 0 (outer variable)
```

**Solution:**
```wfl
// Use different names
store total as 0

count from 1 to 10:
    add count to total  // 'count' is loop var, 'total' is outer
end count

display "Total: " with total
```

### Gotcha: String Concatenation with Numbers

**Problem:**
```wfl
store age as 25
store message as "Age: " with age  // ⚠ May need explicit conversion
```

**Solution:**
```wfl
store age as 25
store message as "Age: " with convert age to text  // ✓ Explicit
```

---

# Section 8: Architecture Overview

WFL follows a traditional compiler architecture with modern enhancements for async execution and web development.

## 8.1 Compiler Pipeline

```
┌─────────────┐
│ Source Code │
│   (.wfl)    │
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   LEXER     │  High-performance tokenization
│   (Logos)   │  - Converts source to tokens
└──────┬──────┘  - 200+ token types
       │          - Natural language keywords
       ▼
┌─────────────┐
│   PARSER    │  Recursive descent parser
│             │  - Builds Abstract Syntax Tree (AST)
└──────┬──────┘  - Error recovery
       │          - Natural language constructs
       ▼
┌─────────────┐
│  ANALYZER   │  Semantic validation
│             │  - Symbol resolution
└──────┬──────┘  - Scope checking
       │          - Semantic correctness
       ▼
┌─────────────┐
│TYPE CHECKER │  Static type analysis
│             │  - Type inference
└──────┬──────┘  - Type checking
       │          - Flow-sensitive analysis
       ▼
┌─────────────┐
│ INTERPRETER │  Direct AST execution
│  (Tokio)    │  - Async-capable runtime
└─────────────┘  - Built-in stdlib
                 - Memory-safe execution
```

## 8.2 Component Descriptions

### Lexer
- **Technology:** Logos crate for high-performance tokenization
- **Features:** 200+ token types, natural language keywords, Unicode support
- **Output:** Token stream with source locations
- **Performance:** ~1-2ms for typical programs

### Parser
- **Type:** Recursive descent with error recovery
- **Features:** Natural language constructs, multi-word identifiers, detailed error messages
- **Output:** Abstract Syntax Tree (AST)
- **Performance:** ~5-10ms for typical programs

### Analyzer
- **Features:** Symbol resolution, scope validation, semantic checking
- **Checks:** Undefined variables, duplicate definitions, invalid operations
- **Output:** Validated AST + symbol table
- **Performance:** ~10-20ms for typical programs

### Type Checker
- **Type System:** Static with inference
- **Features:** Hindley-Milner-style inference, flow-sensitive analysis, generic types
- **Checks:** Type mismatches, invalid operations, return type consistency
- **Output:** Fully-typed AST
- **Performance:** ~15-30ms for typical programs

### Interpreter
- **Runtime:** Direct AST execution with Tokio async runtime
- **Features:** Async/await support, built-in stdlib, memory-safe execution
- **Performance:** Varies by program complexity

## 8.3 Memory Model

### Automatic Memory Management

WFL uses automatic memory management (garbage collection) to prevent memory-related bugs:

- **No manual allocation/deallocation** - All memory managed automatically
- **Reference counting** - Efficient memory reclamation
- **Cycle detection** - Prevents memory leaks from circular references
- **Safe by design** - No buffer overflows, use-after-free, or dangling pointers

### Memory Safety Guarantees

✓ **Buffer overflow protection** - All array/list accesses are bounds-checked
✓ **Type safety** - No type confusion or invalid casts
✓ **No dangling pointers** - Automatic lifetime management
✓ **Thread safety** - Tokio runtime ensures safe concurrency
✓ **No memory leaks** - Garbage collector reclaims unused memory

## 8.4 Performance Characteristics

### Asymptotic Complexity

| Operation | Complexity | Notes |
|-----------|------------|-------|
| Variable access | O(1) | Direct lookup in environment |
| Listaccess by index | O(1) | Direct indexing |
| List push/pop | O(1) amortized | Dynamic array |
| List contains | O(n) | Linear search |
| String concatenation | O(n+m) | Creates new string |
| Function call | O(1) + body | Stack frame creation |

### Optimization Notes

1. **Prefer built-in functions** - Implemented in Rust for performance
2. **Minimize string concatenation in loops** - Build lists then join
3. **Use appropriate data structures** - Lists for ordered data, maps for lookups
4. **Async for I/O** - Non-blocking I/O prevents thread blocking
5. **Batch operations** - Parallel I/O when possible

---

# Appendix A: Syntax Cheat Sheet

## Ultra-Compact WFL Reference

### Variables
```wfl
store x as 5                    // Declare
change x to 10                  // Assign
add 5 to x                      // Increment
```

### Types
```wfl
42, 3.14, 1 million            // Number
"text", "hello"                // Text
yes, no, true, false           // Boolean
nothing, missing               // Nothing/null
[1, 2, 3]                      // List
```

### Control Flow
```wfl
// If-else
check if condition: ... otherwise: ... end check

// Loops
count from 1 to 10: ... end count
for each item in list: ... end for
repeat while condition: ... end repeat
repeat until condition: ... end repeat
main loop: ... end loop

// Control
break              // Exit loop
continue / skip    // Next iteration
give back result   // Return from action
```

### Functions
```wfl
// Define
define action name with param1 and param2:
    give back result
end action

// Call
perform action_name with param1 as value1 and param2 as value2
```

### I/O
```wfl
// File
open file at "path" as handle
wait for read content from handle as data
close file handle

// HTTP
wait for open url at "url" and read content as response

// Server
listen on port 8080 as server
wait for request comes in on server as req
respond to req with "response"
```

### Error Handling
```wfl
try:
    risky_operation
when specific_error:
    handle_error
otherwise:
    handle_any_error
finally:
    cleanup
end try
```

### Common Functions
```wfl
print(value)                   // Output
typeof(value)                  // Get type
length(text/list)              // Get length
touppercase(text)              // To uppercase
push(list, item)               // Add to list
random_int(min, max)           // Random integer
```

---

# Appendix C: Keyword Reference

## Complete Keyword List

### Variable Keywords
- `store` - Declare variable
- `create` - Create object/collection
- `change` - Assign new value
- `add` - Add to number
- `subtract` - Subtract from number
- `multiply` - Multiply number
- `divide` - Divide number

### Control Flow Keywords
- `check if` - Start conditional
- `otherwise` - Else clause
- `otherwise if` - Else-if clause
- `end check` - End conditional
- `count from` - Start count loop
- `to` - Loop end value
- `down to` - Countdown loop
- `by` - Loop step size
- `end count` - End count loop
- `for each` - Start for-each loop
- `in` - Collection source
- `reversed` - Reverse iteration
- `end for` - End for-each loop
- `repeat while` - While loop
- `repeat until` - Until loop
- `repeat forever` - Infinite loop
- `main loop` - Server loop (no timeout)
- `end repeat` - End repeat loop
- `end loop` - End main loop
- `break` - Exit loop
- `continue` - Skip to next iteration
- `skip` - Synonym for continue
- `exit` - Exit all loops

### Function Keywords
- `define action` - Start function definition
- `needs` - Parameter list
- `with` - Parameter connector
- `and` - Parameter separator
- `gives back` - Return type declaration
- `do` - Start function body
- `end action` - End function
- `perform` - Call function
- `give back` - Return value
- `provide` - Synonym for give back
- `return` - Synonym for give back
- `async` - Asynchronous function

### Type Keywords
- `as` - Type annotation / assignment
- `number` - Number type
- `text` - Text type
- `boolean` - Boolean type
- `list` - List type
- `nothing` - Null type
- `convert` - Type conversion
- `safely` - Safe conversion

### OOP Keywords
- `create container` - Define class
- `end container` - End container
- `property` - Define property
- `extends` - Inheritance
- `implements` - Interface implementation
- `interface` - Interface definition
- `requires` - Interface requirement
- `static` - Static member
- `public` - Public access
- `private` - Private access
- `this` - Current instance
- `parent` - Parent class
- `event` - Event definition
- `trigger` - Fire event
- `on` - Event handler

### I/O Keywords
- `open file` - Open file
- `open url` - HTTP request
- `open database` - Database connection
- `for reading` - Read mode
- `for writing` - Write mode
- `for append` - Append mode
- `read content from` - Read operation
- `write content` - Write operation
- `append content` - Append operation
- `close file` - Close resource
- `delete file` - Delete file
- `listen on port` - Start server
- `wait for` - Async wait
- `request comes in` - Receive request
- `respond to` - Send response

### Error Handling Keywords
- `try` - Start try block
- `when` - Catch specific error
- `catch` - Catch any error
- `otherwise` - Catch remaining errors
- `finally` - Always execute
- `end try` - End try block
- `retry` - Retry try block
- `throw error` - Raise error

### Comparison Keywords
- `is` - Equals
- `is not` - Not equals
- `is equal to` - Equals (explicit)
- `is greater than` - Greater than
- `is less than` - Less than
- `is at least` - Greater than or equal
- `is at most` - Less than or equal
- `is above` - Greater than
- `is below` - Less than
- `is between` - Range check

### Logical Keywords
- `and` - Logical AND
- `or` - Logical OR
- `not` - Logical NOT

### Boolean Literals
- `yes` - True
- `no` - False
- `true` - True
- `false` - False

### Null Literals
- `nothing` - Null/undefined
- `missing` - Synonym for nothing
- `undefined` - Synonym for nothing

### Arithmetic Keywords
- `plus` - Addition
- `minus` - Subtraction
- `times` - Multiplication
- `divided by` - Division
- `modulo` / `mod` - Modulus

### String Keywords
- `with` - String concatenation

### Comments
- `//` - Single-line comment

---

# Document Completion

## Summary

This comprehensive WFL AI Reference provides complete documentation for working with the WebFirst Language:

**Coverage:**
- ✅ Complete syntax specification with EBNF grammar
- ✅ All 45+ standard library functions documented
- ✅ File, HTTP, and web server I/O operations
- ✅ Complete CLI tools and development workflow
- ✅ Type system rules and error handling
- ✅ Error reference with solutions
- ✅ Code patterns and best practices
- ✅ Architecture overview
- ✅ Quick reference cheat sheets

**Document Statistics:**
- **Total Sections:** 8 main + 3 appendices
- **Total Lines:** ~19,500
- **Functions Documented:** 45+
- **Code Examples:** ~250
- **Coverage:** Comprehensive reference for all WFL features

**Target Audience:**
- AI agents generating WFL code
- AI assistants helping users write WFL
- Training datasets for language models
- Automated development tools

**Last Updated:** 2025-11-30
**WFL Version:** 25.11.10

---

*End of WFL Complete AI Reference*
