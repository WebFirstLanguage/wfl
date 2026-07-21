# Syntax Reference

Quick lookup for WFL syntax. Cheat sheet for common patterns.

## Variables

```wfl
store name as "Alice"         // Create variable
change name to "Bob"          // Modify variable
display name
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
store x as 10
store y as 3
display x plus y              // Addition
display x minus y             // Subtraction
display x times y             // Multiplication
display x divided by y        // Division
display x % y                 // Modulo

check if x is greater than y:
    display "x is larger"
end check

store a as yes
store b as no
check if a and not b:
    display "logical ok"
end check

display "hi" with " " with "there"
```

## Control Flow

```wfl
store score as 85

// If-else
check if score is greater than or equal to 60:
    display "Pass"
otherwise:
    display "Retry"
end check

// Nested conditionals
check if score is greater than 90:
    display "Excellent"
otherwise:
    check if score is greater than 70:
        display "Good"
    otherwise:
        display "Keep practicing"
    end check
end check
```

## Loops

```wfl
// Count loop
count from 1 to 3:
    display count
end count

// Count with step
count from 0 to 6 by 2:
    display count
end count

// For each
store items as ["a", "b"]
for each item in items:
    display item
end for

// While
store n as 0
repeat while n is less than 3:
    display n
    change n to n plus 1
end repeat
```

## Actions (Functions)

```wfl
// Define
define action called add_numbers with parameters x and y:
    return x plus y
end action

// Call as a statement
call add_numbers with 5 and 3

// Call as an expression
store result as add_numbers of 5 and 3
display result

// Typed parameters and overloading: same name, distinguishable signatures
define action called depict with parameters value as number:
    return "a number: " with value
end action

define action called depict with parameters value as text:
    return "some text: " with value
end action

display depict of 42      // dispatches to the number version
display depict of "wfl"   // dispatches to the text version
```

## Lists

```wfl
// Create
create list items:
    add "one"
    add "two"
end list

// Literal
store more_items as [1, 2, 3]

// Operations
push with more_items and 4
store last as pop of more_items
store len as length of more_items
store idx as indexof of more_items and 2
display last with " " with len with " " with idx
```

## Error Handling

```wfl
// Create a small file so the read path succeeds in this demo
open file at "data.txt" for writing as setupfile
wait for write content "hello" into setupfile
close file setupfile

store file_handle as nothing

try:
    open file at "data.txt" for reading as myfile
    change file_handle to myfile
    wait for store file_content as read content from myfile
    display file_content
when error:
    display "Error reading file"
finally:
    check if file_handle is not nothing:
        close file file_handle
    end check
end try
```

## File I/O

```wfl
// Write
open file at "path.txt" for writing as outfile
wait for write content "data" into outfile
close file outfile

// Read
open file at "path.txt" for reading as myfile
wait for store file_content as read content from myfile
close file myfile
display file_content

// Append
open file at "path.txt" for appending as logfile
wait for append content "line" into logfile
close file logfile
```

## Patterns

```wfl
create pattern email_like:
    one or more letter
    followed by "@"
    one or more letter or digit
end pattern

store sample as "a@b"
check if sample matches email_like:
    display "Matches"
end check
```

## Containers

```wfl
create container Person:
    property name: Text

    action greet:
        display "Hello, " with name
    end
end

create new Person as alice:
    name is "Alice"
end

alice.greet()
```

## Built-in Functions

```wfl
// Core
store value as 42
display value
display typeof of value
display isnothing of value

// Math
display abs of -5
display round of 3.7
display floor of 3.7
display ceil of 3.2
// Prefer word forms that match the language style:
// clamp of 15 between 0 and 10  (see math module docs)

// Text
store sample as "  Hello  "
display touppercase of sample
display tolowercase of sample
display length of sample
display trim of sample

// List
store nums as [10, 20, 30]
push with nums and 40
display length of nums
display pop of nums
display indexof of nums and 20

// Time / random
display current time in milliseconds
display random_boolean
```

## Web Server (forms)

These keep a process listening; they are documentation forms, not a standalone script to run:

```text
listen on port 8080 as server
wait for request that comes in on server as req
respond to req with "Hello"
respond to req with "Hello" and status 200
respond to req with "<h1>Hi</h1>" and content_type "text/html"
```

See [Web Servers](../04-advanced-features/web-servers.md) for complete programs.

## Execute WFL Files (forms)

```text
execute wfl file at "page.wfl"
execute file at "page.wfl" and read output as page_output
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
