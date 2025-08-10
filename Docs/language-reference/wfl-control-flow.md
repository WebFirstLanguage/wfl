# WFL Control Flow Reference

Control flow statements in WFL allow you to direct the execution path of your program using natural, English-like syntax. This guide covers conditional statements, loops, and flow control keywords that make your programs dynamic and responsive.

## Overview

WFL provides intuitive control flow constructs that read like natural English:
- **Conditionals**: Make decisions with `check if`, `otherwise if`, and `otherwise`
- **Loops**: Repeat actions with `count`, `for each`, `repeat while/until`, and more
- **Flow Control**: Direct execution with `break`, `continue`, `skip`, and `return`

All control flow structures use clear start and end markers, making code blocks easy to identify and understand.

## Conditional Statements

### Basic If-Then-Else

WFL uses `check if` blocks for conditional execution:

```wfl
check if user is logged in:
    display "Welcome back!"
end check
```

### With Else Clause

Add an `otherwise` clause to handle the false case:

```wfl
check if age is at least 18:
    display "You can vote"
otherwise:
    display "Too young to vote"
end check
```

### Multiple Conditions

Chain conditions with `otherwise if`:

```wfl
check if score is above 90:
    display "Grade: A"
otherwise if score is above 80:
    display "Grade: B"
otherwise if score is above 70:
    display "Grade: C"
otherwise:
    display "Grade: F"
end check
```

### Single-Line Conditionals

For simple cases, use the inline form:

```wfl
if temperature is below 0 then display "Freezing!" otherwise display "Not freezing"

// Can also omit the otherwise part
if file exists then display "File found"
```

### Natural Language Conditions

WFL supports readable comparison operators:

```wfl
// Equality
check if name is "Alice":           // equals
check if count is not 0:            // not equals

// Numeric comparisons
check if age is greater than 21:    // >
check if price is less than 100:    // <
check if score is at least 50:      // >=
check if temp is at most 30:        // <=

// Boolean conditions
check if is active:                 // if true
check if not is active:             // if false

// String operations
check if email contains "@":       // substring check
check if text starts with "Hello":  // prefix check
check if text ends with ".txt":     // suffix check

// Null checks
check if value is nothing:          // null check
check if value is not nothing:      // not null

// List operations
check if list is empty:             // empty check
check if item is in shopping list:  // membership test
```

### Combining Conditions

Use `and` and `or` to combine multiple conditions:

```wfl
check if user is logged in and role is "admin":
    display "Admin panel"
end check

check if temperature is below 0 or temperature is above 40:
    display "Extreme weather warning!"
end check

// Complex combinations
check if (age is at least 18 and has license) or is supervised:
    display "Can drive"
end check
```

## Loop Constructs

### Counting Loops

Iterate over numeric ranges with `count`:

```wfl
// Basic counting
count from 1 to 5:
    display "Iteration: " with count
end count
// Output: 1, 2, 3, 4, 5

// Count with custom step
count from 0 to 20 by 2:
    display "Even: " with count
end count
// Output: 0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20

// Count backwards
count from 10 down to 1:
    display "Countdown: " with count
end count
// Output: 10, 9, 8, 7, 6, 5, 4, 3, 2, 1

// Count backwards with step
count from 100 down to 0 by 10:
    display count
end count
// Output: 100, 90, 80, 70, 60, 50, 40, 30, 20, 10, 0
```

The loop variable `count` is automatically available within the loop body.

### For-Each Loops

Iterate over collections:

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

// With index (if supported)
for each fruit at index in fruits:
    display index with ": " with fruit
end for
// Output: 0: apple, 1: banana, 2: orange
```

### Conditional Loops

#### Repeat While

Continue looping while a condition is true:

```wfl
store attempts as 0
repeat while attempts is less than 3:
    store attempts as attempts plus 1
    display "Attempt " with attempts
    check if login successful:
        break
    end check
end repeat
```

#### Repeat Until

Continue looping until a condition becomes true:

```wfl
store temperature as 20
repeat until temperature is above 100:
    store temperature as temperature plus 10
    display "Heating... Current temp: " with temperature
end repeat
display "Target temperature reached!"
```

### Infinite Loops

#### Forever Loop

A standard infinite loop that respects timeout settings:

```wfl
store counter as 0
repeat forever:
    store counter as counter plus 1
    display "Processing item " with counter
    
    check if counter is 100:
        break  // Exit the loop
    end check
end repeat
```

#### Main Loop

Special infinite loop for long-running applications that **disables timeout**:

```wfl
// Server application example
main loop:
    wait for request
    process request
    
    check if shutdown signal received:
        display "Shutting down gracefully..."
        break
    end check
end loop
```

**Key Differences:**
- `repeat forever`: Subject to execution timeout (will error if runs too long)
- `main loop`: Bypasses timeout, designed for servers and continuous services

## Loop Control Statements

### Break Statement

Exits the current loop immediately:

```wfl
for each item in items:
    check if item is "stop":
        break  // Exit the loop
    end check
    display item
end for
display "Loop ended"
```

### Continue/Skip Statements

Skips to the next iteration (both keywords work the same):

```wfl
count from 1 to 10:
    check if count is even:
        continue  // Skip even numbers
    end check
    display count
end count
// Output: 1, 3, 5, 7, 9

// 'skip' is a synonym for 'continue'
for each file in files:
    check if file ends with ".tmp":
        skip  // Skip temporary files
    end check
    process file
end for
```

### Exit Statement

Breaks out of nested loops:

```wfl
count from 1 to 5:
    display "Outer: " with count
    count from 1 to 5:
        display "  Inner: " with count
        check if count is 3:
            exit loop  // Exits BOTH loops
        end check
    end count
end count
display "Both loops exited"
```

**Note:** `exit` or `exit loop` breaks out of all enclosing loops, while `break` only exits the innermost loop.

## Control Flow in Actions

### Return/Give Back

Return values from actions (functions):

```wfl
define action calculate sum:
    needs:
        numbers as list
    gives back:
        total as number
    do:
        store sum as 0
        for each num in numbers:
            check if num is negative:
                give back 0  // Early return
            end check
            store sum as sum plus num
        end for
        give back sum
end action

// Using the action
store result as perform calculate sum with numbers as [1, 2, 3]
display "Sum: " with result
```

### Control Flow Interaction

Control flow statements behave consistently in nested contexts:

```wfl
define action process items:
    needs:
        items as list
    gives back:
        result as text
    do:
        for each item in items:
            check if item is "abort":
                give back "Aborted"  // Returns from the entire action
            end check
            
            check if item is "skip":
                continue  // Skips to next iteration
            end check
            
            check if item is "done":
                break  // Exits the loop, continues with action
            end check
            
            display "Processing: " with item
        end for
        
        give back "Completed"
end action
```

## Common Patterns

### Search Pattern

Find an item in a collection:

```wfl
store found as no
store target as "apple"

for each item in shopping list:
    check if item is target:
        store found as yes
        break  // Stop searching once found
    end check
end for

check if found:
    display "Found " with target
otherwise:
    display target with " not in list"
end check
```

### Filter Pattern

Process only certain items:

```wfl
for each number in numbers:
    // Skip negative numbers
    check if number is less than 0:
        continue
    end check
    
    // Process positive numbers
    display "Processing: " with number
end for
```

### Accumulator Pattern

Build up a result:

```wfl
store total as 0
store count as 0

for each score in scores:
    store total as total plus score
    store count as count plus 1
end for

check if count is greater than 0:
    store average as total divided by count
    display "Average: " with average
end check
```

### Nested Loop with Early Exit

Search in a 2D structure:

```wfl
store found as no

for each row in grid:
    for each cell in row:
        check if cell is target:
            display "Found at row " with row_index with ", column " with cell_index
            store found as yes
            exit loop  // Exit both loops
        end check
    end for
end for

check if not found:
    display "Target not found in grid"
end check
```

### Event Processing Loop

Continuous event processor:

```wfl
main loop:
    store event as get next event
    
    check if event is nothing:
        wait 100 milliseconds
        continue  // Check again
    end check
    
    check if event.type is "shutdown":
        display "Shutdown requested"
        break
    end check
    
    // Process the event
    perform handle event with event as event
    
    // Check for errors
    check if error occurred:
        log error
        continue  // Skip to next event
    end check
    
    update statistics
end loop

display "Event processor stopped"
```

## Best Practices

### 1. Choose the Right Loop Type

- Use `count` for numeric ranges
- Use `for each` for collections
- Use `repeat while/until` for condition-based loops
- Use `main loop` for servers and long-running services

### 2. Clear Exit Conditions

Always provide a way to exit infinite loops:

```wfl
// Good: Clear exit condition
main loop:
    check if should stop:
        break
    end check
    // ... rest of loop
end loop

// Bad: No exit condition
repeat forever:
    // ... this could run forever
end repeat
```

### 3. Minimize Nesting

Use early returns and continues to reduce nesting:

```wfl
// Good: Early exit reduces nesting
for each item in items:
    check if item is invalid:
        continue  // Skip invalid items
    end check
    
    process item  // Main logic not nested
end for

// Less readable: Deeply nested
for each item in items:
    check if item is valid:
        process item  // Main logic is nested
    end check
end for
```

### 4. Descriptive Conditions

Use WFL's natural language to make conditions self-documenting:

```wfl
// Good: Self-documenting
check if user is logged in and subscription is active:
    allow access
end check

// Less clear: Using just variables
check if logged and active:
    allow access
end check
```

## Technical Notes

### Control Flow Implementation

Internally, WFL uses a control flow signaling system where:
- Each statement can return a control flow signal
- Signals propagate up through nested structures
- Loops and functions handle signals appropriately

This ensures consistent behavior across all contexts:
- `break` affects only the innermost loop
- `exit/exit loop` can break out of nested loops
- `return/give back` exits the entire function
- `continue/skip` jumps to the next iteration

### Performance Considerations

- Loop control statements have minimal overhead
- `main loop` disables timeout checking for better performance
- Early exits (`break`, `return`) can improve performance by avoiding unnecessary iterations
- Condition evaluation short-circuits (in `and`/`or` expressions)

## See Also

- [WFL Language Specification](wfl-spec.md) - Complete language reference
- [Main Loop Documentation](wfl-main-loop.md) - Details on long-running loops
- [Actions Documentation](wfl-actions.md) - Functions and return values
- [WFL by Example](../guides/wfl-by-example.md) - Practical examples