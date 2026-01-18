# Syntax Reference

Quick lookup for WFL syntax. Cheat sheet for common patterns.

## Variables

```wfl
store name as value           // Create variable
change name to new_value      // Modify variable
```

## Data Types

```wfl
store text as "Hello"                    // Text
store number as 42                       // Number
store decimal as 3.14                    // Number (float)
store boolean as yes                     // Boolean (yes/no)
store nothing_val as nothing             // Null
store list as [1, 2, 3]                 // List
```

## Operators

```wfl
// Arithmetic
x plus y                      // Addition
x minus y                     // Subtraction
x times y                     // Multiplication
x divided by y                // Division
x % y                         // Modulo

// Comparison
x is equal to y               // Equality
x is not equal to y           // Inequality
x is greater than y
x is greater than or equal to y
x is less than y
x is less than or equal to y

// Logical
a and b                       // AND
a or b                        // OR
not a                         // NOT

// String
text1 with text2              // Concatenation
```

## Control Flow

```wfl
// If-else
check if condition:
    code
otherwise:
    code
end check

// Nested conditionals
check if condition1:
    code
otherwise:
    check if condition2:
        code
    otherwise:
        code
    end check
end check
```

## Loops

```wfl
// Count loop
count from 1 to 10:
    display count
end count

// Count with step
count from 0 to 100 by 10:
    display count
end count

// For each
for each item in list:
    display item
end for

// While
repeat while condition:
    code
end repeat

// Until
repeat until condition:
    code
end repeat
```

## Actions (Functions)

```wfl
// Define
define action called name with parameters x and y:
    return x plus y
end action

// Call
call name with 5 and 3
store result as name with 5 and 3
```

## Lists

```wfl
// Create
create list items:
    add "one"
    add "two"
end list

// Literal
store items as [1, 2, 3]

// Operations
push with items and value     // Add
store last as pop from items  // Remove
store len as length of items  // Length
store idx as indexof of items and value
```

## Error Handling

```wfl
try:
    risky_operation()
catch:
    display "Error"
finally:
    cleanup()
end try

// Specific errors
try:
    code
when file not found:
    handle_missing_file()
when permission denied:
    handle_permission()
catch:
    handle_other()
end try
```

## File I/O

```wfl
// Read
open file at "path" for reading as myfile
wait for store content as read content from myfile
close file myfile

// Write
open file at "path" for writing as outfile
wait for write content "data" into outfile
close file outfile

// Append
open file at "path" for appending as logfile
wait for append content "line" into logfile
close file logfile
```

## Patterns

```wfl
create pattern name:
    one or more letter
    followed by "@"
    one or more letter or digit
end pattern

check if text matches name:
    display "Matches"
end check
```

## Containers

```wfl
create container Name:
    property field: Type

    action method:
        code
    end
end

create new Name as instance:
    field is value
end

instance.method()
```

## Built-in Functions

```wfl
// Core
display value
typeof of value
isnothing of value

// Math
abs of -5
round of 3.7
floor of 3.7
ceil of 3.2
clamp of 15 between 0 and 10

// Text
touppercase of text
tolowercase of text
length of text
substring of text from start length len
trim of text

// List
length of list
push with list and item
pop from list
indexof of list and item

// File
file exists at path
file size at path
path extension of path

// Time
today
now
current time in milliseconds

// Random
random
random_int between min and max
random_boolean
```

## Web Server

```wfl
listen on port 8080 as server

wait for request comes in on server as req

respond to req with content
respond to req with content and status 200
respond to req with content and content_type "text/html"
```

## Comments

```wfl
// Single-line comment

store x as 5  // Inline comment
```

---

**Print this page for quick reference while coding!**

---

**Previous:** [← FAQ](faq.md) | **Next:** [Keyword Reference (Quick) →](keyword-reference.md) | [Reserved Keywords (Complete) →](reserved-keywords.md)
