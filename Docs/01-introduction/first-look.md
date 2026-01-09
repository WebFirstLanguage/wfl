# First Look at WFL

Before diving into tutorials, let's explore what WFL code looks like in practice. You'll see side-by-side comparisons with traditional languages and learn what makes WFL different.

## The Simplest Program

**WFL:**
```wfl
display "Hello, World!"
```

**JavaScript:**
```javascript
console.log("Hello, World!");
```

**Python:**
```python
print("Hello, World!")
```

WFL is as simple as the simplest languages, but with more natural phrasing.

## Variables and Types

### WFL
```wfl
// Declaring variables
store user name as "Alice"
store user age as 28
store is active as yes
store account balance as 1250.75

// Changing variables
change user age to 29

// Type inference
display typeof of user name        // Output: "Text"
display typeof of user age          // Output: "Number"
display typeof of is active         // Output: "Boolean"
```

### JavaScript Equivalent
```javascript
// Declaring variables
let userName = "Alice";
let userAge = 28;
let isActive = true;
let accountBalance = 1250.75;

// Changing variables
userAge = 29;

// Type checking
console.log(typeof userName);      // Output: "string"
console.log(typeof userAge);        // Output: "number"
console.log(typeof isActive);       // Output: "boolean"
```

**Notice:** WFL uses natural phrases like `store... as` and `change... to` instead of cryptic keywords like `let` and assignment operators.

## Conditionals

### WFL
```wfl
store temperature as 25

check if temperature is greater than 30:
    display "It's hot outside!"
otherwise check if temperature is greater than 20:
    display "Nice weather!"
otherwise:
    display "It's a bit cool."
end check
```

### JavaScript Equivalent
```javascript
let temperature = 25;

if (temperature > 30) {
    console.log("It's hot outside!");
} else if (temperature > 20) {
    console.log("Nice weather!");
} else {
    console.log("It's a bit cool.");
}
```

**Notice:** WFL reads like natural English. `check if` instead of `if`, `is greater than` instead of `>`, `otherwise` instead of `else`.

## Loops

### Count Loops (For Loop)

**WFL:**
```wfl
count from 1 to 10:
    display "Number: " with the current count
end count

// With step
count from 0 to 100 by 10:
    display the current count
end count
```

**JavaScript:**
```javascript
for (let i = 1; i <= 10; i++) {
    console.log("Number: " + i);
}

// With step
for (let i = 0; i <= 100; i += 10) {
    console.log(i);
}
```

### For Each Loops (Array Iteration)

**WFL:**
```wfl
create list fruits:
    add "apple"
    add "banana"
    add "orange"
end list

for each fruit in fruits:
    display "I like " with fruit
end for
```

**JavaScript:**
```javascript
const fruits = ["apple", "banana", "orange"];

for (const fruit of fruits) {
    console.log("I like " + fruit);
}
```

**Notice:** WFL's loop syntax is more intuitive. `count from 1 to 10` is clearer than `for (let i = 1; i <= 10; i++)`.

## Functions (Actions)

### WFL
```wfl
// Define an action
action greet with name:
    display "Hello, " with name with "!"
end action

// Call the action
call greet with "Alice"
// Output: Hello, Alice!

// Action with return value
action calculate area with width and height:
    store result as width times height
    return result
end action

store room area as calculate area with 10 and 20
display "Area: " with room area
// Output: Area: 200
```

### JavaScript Equivalent
```javascript
// Define a function
function greet(name) {
    console.log("Hello, " + name + "!");
}

// Call the function
greet("Alice");

// Function with return value
function calculateArea(width, height) {
    const result = width * height;
    return result;
}

const roomArea = calculateArea(10, 20);
console.log("Area: " + roomArea);
```

**Notice:** WFL calls functions "actions" and uses natural phrasing: `action greet with name` instead of `function greet(name)`.

## Lists (Arrays)

### WFL
```wfl
// Create a list
create list numbers:
    add 1
    add 2
    add 3
    add 4
    add 5
end list

// Or use literal syntax
store numbers as [1, 2, 3, 4, 5]

// List operations
push to numbers with 6
store last as pop from numbers
store size as length of numbers
check if contains of numbers and 3

// Iterate
for each number in numbers:
    display number
end for
```

### JavaScript Equivalent
```javascript
// Create an array
const numbers = [1, 2, 3, 4, 5];

// Array operations
numbers.push(6);
const last = numbers.pop();
const size = numbers.length;
const hasThree = numbers.includes(3);

// Iterate
for (const number of numbers) {
    console.log(number);
}
```

## Error Handling

### WFL
```wfl
try:
    store result as risky operation()
    display "Success: " with result
when error:
    display "An error occurred: " with error message
otherwise:
    display "Operation completed"
end try
```

### JavaScript Equivalent
```javascript
try {
    const result = riskyOperation();
    console.log("Success: " + result);
} catch (error) {
    console.log("An error occurred: " + error.message);
} finally {
    console.log("Operation completed");
}
```

## File Operations

### WFL
```wfl
// Read a file
open file at "data.txt" for reading as file
store content as read content from file
close file
display "File content: " with content

// Write a file
open file at "output.txt" for writing as file
write content "Hello, WFL!" into file
close file

// List files in directory
list files in "." as file list
for each file in file list:
    display "Found: " with file
end for
```

### Node.js Equivalent
```javascript
const fs = require('fs');

// Read a file
const content = fs.readFileSync('data.txt', 'utf8');
console.log("File content: " + content);

// Write a file
fs.writeFileSync('output.txt', 'Hello, WFL!');

// List files in directory
const fileList = fs.readdirSync('.');
for (const file of fileList) {
    console.log("Found: " + file);
}
```

## Web Server

### WFL
```wfl
// Start a web server
listen on port 8080 as web server

display "Server running at http://127.0.0.1:8080"

// Handle requests
wait for request comes in on web server as req

check if path is equal to "/":
    respond to req with "Welcome to WFL!"
check if path is equal to "/about":
    respond to req with "WFL - Programming in Plain English"
otherwise:
    respond to req with "Page not found" and status 404
end check
```

### Express.js (Node.js) Equivalent
```javascript
const express = require('express');
const app = express();

app.get('/', (req, res) => {
    res.send('Welcome to WFL!');
});

app.get('/about', (req, res) => {
    res.send('WFL - Programming in Plain English');
});

app.use((req, res) => {
    res.status(404).send('Page not found');
});

app.listen(8080, () => {
    console.log('Server running at http://127.0.0.1:8080');
});
```

**Notice:** WFL has web server capabilities built-in. No need to import external frameworks like Express.

## Pattern Matching (Regex Alternative)

### WFL
```wfl
// Define a pattern for email validation
create pattern email address:
    one or more letter or digit or symbol from "._-"
    followed by "@"
    followed by one or more letter or digit
    followed by "."
    followed by between 2 and 4 letter
end pattern

// Test the pattern
check if "user@example.com" matches email address:
    display "Valid email!"
otherwise:
    display "Invalid email!"
end check
```

### JavaScript Regex Equivalent
```javascript
// Define a regex pattern
const emailPattern = /^[a-zA-Z0-9._-]+@[a-zA-Z0-9]+\.[a-zA-Z]{2,4}$/;

// Test the pattern
if (emailPattern.test("user@example.com")) {
    console.log("Valid email!");
} else {
    console.log("Invalid email!");
}
```

**Notice:** WFL's pattern syntax is readable. You can understand what it's checking without being a regex expert.

## Object-Oriented Programming (Containers)

### WFL
```wfl
// Define a container (class)
create container Person:
    property name as text
    property age as number

    action introduce:
        display "Hello, I'm " with name with " and I'm " with age with " years old."
    end action

    action have birthday:
        change age to age plus 1
        display "Happy birthday! Now " with age with " years old."
    end action
end container

// Create an instance
create new Person as alice with property name as "Alice" and property age as 28

// Call methods
call alice introduce
call alice have birthday

// Output:
// Hello, I'm Alice and I'm 28 years old.
// Happy birthday! Now 29 years old.
```

### JavaScript Class Equivalent
```javascript
class Person {
    constructor(name, age) {
        this.name = name;
        this.age = age;
    }

    introduce() {
        console.log(`Hello, I'm ${this.name} and I'm ${this.age} years old.`);
    }

    haveBirthday() {
        this.age++;
        console.log(`Happy birthday! Now ${this.age} years old.`);
    }
}

const alice = new Person("Alice", 28);
alice.introduce();
alice.haveBirthday();
```

## Async Operations

### WFL
```wfl
// Async file operation
wait for file operation completes as result
display "File saved: " with result

// Async web request (conceptual)
wait for response from "https://api.example.com/data" as data
display "Received: " with data

// Multiple async operations
wait for all operations complete
display "All tasks finished!"
```

### JavaScript Async/Await Equivalent
```javascript
// Async file operation
const result = await fileOperation();
console.log("File saved: " + result);

// Async web request
const response = await fetch("https://api.example.com/data");
const data = await response.json();
console.log("Received: " + data);

// Multiple async operations
await Promise.all(operations);
console.log("All tasks finished!");
```

**Notice:** WFL uses `wait for` instead of `await`, making async code more natural.

## Putting It All Together

Here's a complete WFL program that demonstrates multiple features:

```wfl
// A simple task manager

// Define Task container
create container Task:
    property description as text
    property completed as boolean

    action mark complete:
        change completed to yes
        display "✓ Completed: " with description
    end action
end container

// Main program
display "=== Task Manager ==="
display ""

// Create task list
create list tasks
end list

// Add some tasks
create new Task as task1 with property description as "Learn WFL" and property completed as no
create new Task as task2 with property description as "Build web server" and property completed as no
create new Task as task3 with property description as "Write documentation" and property completed as no

push to tasks with task1
push to tasks with task2
push to tasks with task3

// Display all tasks
display "All Tasks:"
for each task in tasks:
    check if task completed is yes:
        display "  ✓ " with task description
    otherwise:
        display "  ☐ " with task description
    end check
end for

display ""

// Complete first task
call task1 mark complete

display ""
display "Updated Tasks:"
for each task in tasks:
    check if task completed is yes:
        display "  ✓ " with task description
    otherwise:
        display "  ☐ " with task description
    end check
end for
```

**Output:**
```
=== Task Manager ===

All Tasks:
  ☐ Learn WFL
  ☐ Build web server
  ☐ Write documentation

✓ Completed: Learn WFL

Updated Tasks:
  ✓ Learn WFL
  ☐ Build web server
  ☐ Write documentation
```

## What You've Seen

In this first look at WFL, you've seen:

✅ **Natural syntax** - Code that reads like English
✅ **Type safety** - Automatic type inference and checking
✅ **Modern features** - Async, web servers, pattern matching
✅ **Built-in capabilities** - File I/O, web servers without frameworks
✅ **Object-oriented** - Containers (classes) with natural syntax
✅ **Readable code** - Self-documenting and easy to understand

## Compare for Yourself

| Feature | WFL | JavaScript | Python |
|---------|-----|------------|--------|
| Hello World | `display "Hello"` | `console.log("Hello")` | `print("Hello")` |
| Variable | `store x as 5` | `let x = 5;` | `x = 5` |
| Conditional | `check if x is 5:` | `if (x === 5) {` | `if x == 5:` |
| Loop | `count from 1 to 10:` | `for (let i=1; i<=10; i++) {` | `for i in range(1, 11):` |
| Function | `action greet with name:` | `function greet(name) {` | `def greet(name):` |
| Web Server | Built-in | Requires Express | Requires Flask |
| Pattern Matching | Natural syntax | Regex | Regex |

## Try It Yourself

The best way to understand WFL is to write it yourself. Head to the next section to install WFL and start coding:

**[Getting Started →](../02-getting-started/index.md)**

Or continue learning about why WFL matters:

**[Why WFL? →](why-wfl.md)**

---

**Previous:** [← Natural Language Philosophy](natural-language-philosophy.md) | **Next:** [Why WFL? →](why-wfl.md)
