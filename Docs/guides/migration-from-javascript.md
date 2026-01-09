# Migration from JavaScript

Guide for JavaScript developers learning WFL. Side-by-side comparisons and concept mapping.

## Quick Comparison

| Concept | JavaScript | WFL |
|---------|------------|-----|
| Variable | `const x = 5` | `store x as 5` |
| Function | `function add(a, b)` | `define action called add with parameters a and b:` |
| If Statement | `if (x > 5) { }` | `check if x is greater than 5:` |
| For Loop | `for (let i = 0; i < 10; i++)` | `count from 0 to 9:` |
| Array | `[1, 2, 3]` | `[1, 2, 3]` (same!) |
| Class | `class Person { }` | `create container Person:` |

## Variables

**JavaScript:**
```javascript
const name = "Alice";
let age = 28;
var status = true;
```

**WFL:**
```wfl
store name as "Alice"
store age as 28
store status as yes
```

**Key differences:**
- One keyword (`store`) instead of three (`const`, `let`, `var`)
- Use `change` to modify: `change age to 29`
- `yes`/`no` instead of `true`/`false`

## Functions

**JavaScript:**
```javascript
function greet(name) {
    console.log("Hello, " + name);
}

const add = (a, b) => a + b;

greet("World");
const sum = add(2, 3);
```

**WFL:**
```wfl
define action called greet with parameters name:
    display "Hello, " with name
end action

define action called add with parameters a and b:
    return a plus b
end action

call greet with "World"
store sum as add with 2 and 3
```

## Conditionals

**JavaScript:**
```javascript
if (age >= 18) {
    console.log("Adult");
} else if (age >= 13) {
    console.log("Teen");
} else {
    console.log("Child");
}
```

**WFL:**
```wfl
check if age is greater than or equal to 18:
    display "Adult"
otherwise:
    check if age is greater than or equal to 13:
        display "Teen"
    otherwise:
        display "Child"
    end check
end check
```

**Note:** WFL uses nested blocks, not `else if`.

## Loops

**JavaScript:**
```javascript
// For loop
for (let i = 1; i <= 10; i++) {
    console.log(i);
}

// For...of
for (const item of items) {
    console.log(item);
}

// While
while (condition) {
    // code
}
```

**WFL:**
```wfl
// Count loop
count from 1 to 10:
    display count
end count

// For each
for each item in items:
    display item
end for

// While
repeat while condition:
    // code
end repeat
```

## Arrays/Lists

**JavaScript:**
```javascript
const arr = [1, 2, 3];
arr.push(4);
const last = arr.pop();
const len = arr.length;
const idx = arr.indexOf(3);
```

**WFL:**
```wfl
store arr as [1, 2, 3]
push with arr and 4
store last as pop from arr
store len as length of arr
store idx as indexof of arr and 3
```

## Objects/Containers

**JavaScript:**
```javascript
class Person {
    constructor(name, age) {
        this.name = name;
        this.age = age;
    }

    greet() {
        console.log(`Hi, I'm ${this.name}`);
    }
}

const alice = new Person("Alice", 28);
alice.greet();
```

**WFL:**
```wfl
create container Person:
    property name: Text
    property age: Number

    action greet:
        display "Hi, I'm " with name
    end
end

create new Person as alice:
    name is "Alice"
    age is 28
end

alice.greet()
```

## Async/Await

**JavaScript:**
```javascript
async function readFile() {
    const content = await fs.readFile('data.txt');
    return content;
}

await readFile();
```

**WFL:**
```wfl
define action called read_file:
    open file at "data.txt" for reading as myfile
    wait for store content as read content from myfile
    close file myfile
    return content
end action

// Call automatically handles async
call read_file
```

**Key difference:** `wait for` instead of `await`.

## Error Handling

**JavaScript:**
```javascript
try {
    riskyOperation();
} catch (error) {
    console.log("Error:", error.message);
} finally {
    cleanup();
}
```

**WFL:**
```wfl
try:
    risky_operation()
catch:
    display "Error occurred"
finally:
    cleanup()
end try
```

## What WFL Doesn't Have

❌ **Promises** - Use `wait for` instead
❌ **Classes with constructors** - Use containers with properties
❌ **Arrow functions** - Use regular actions
❌ **Template literals** - Use `with` for concatenation
❌ **Destructuring** - Access properties individually
❌ **Spread operator** - Not available
❌ **`map`/`filter`/`reduce`** - Use loops

## Common Patterns

### Map

**JavaScript:** `arr.map(x => x * 2)`

**WFL:**
```wfl
create list doubled
end list

for each num in arr:
    push with doubled and num times 2
end for
```

### Filter

**JavaScript:** `arr.filter(x => x > 5)`

**WFL:**
```wfl
create list filtered
end list

for each num in arr:
    check if num is greater than 5:
        push with filtered and num
    end check
end for
```

### Find

**JavaScript:** `arr.find(x => x > 5)`

**WFL:**
```wfl
store found as nothing

for each num in arr:
    check if num is greater than 5:
        change found to num
        break  // If supported
    end check
end for
```

---

**Previous:** [← Cookbook](cookbook.md) | **Next:** [Migration from Python →](migration-from-python.md)
