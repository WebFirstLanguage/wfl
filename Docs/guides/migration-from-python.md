# Migration from Python

Guide for Python developers learning WFL. Side-by-side comparisons and concept mapping.

## Quick Comparison

| Concept | Python | WFL |
|---------|--------|-----|
| Variable | `x = 5` | `store x as 5` |
| Function | `def add(a, b):` | `define action called add with parameters a and b:` |
| If Statement | `if x > 5:` | `check if x is greater than 5:` |
| For Loop | `for i in range(1, 11):` | `count from 1 to 10:` |
| List | `[1, 2, 3]` | `[1, 2, 3]` (same!) |
| Class | `class Person:` | `create container Person:` |

## Variables

**Python:**
```python
name = "Alice"
age = 28
is_active = True
```

**WFL:**
```wfl
store name as "Alice"
store age as 28
store is_active as yes
```

**Key differences:**
- `store ... as` instead of `=`
- `yes`/`no` instead of `True`/`False`
- Use `change` to modify: `change age to 29`

## Functions

**Python:**
```python
def greet(name):
    print(f"Hello, {name}")

def add(a, b):
    return a + b

greet("World")
result = add(2, 3)
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
store result as add with 2 and 3
```

**Note:** Must explicitly `call` actions in WFL.

## Conditionals

**Python:**
```python
if age >= 18:
    print("Adult")
elif age >= 13:
    print("Teen")
else:
    print("Child")
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

**Note:** Nested blocks instead of `elif`.

## Loops

**Python:**
```python
# For loop with range
for i in range(1, 11):
    print(i)

# For each
for item in items:
    print(item)

# While
while condition:
    # code
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

## Lists

**Python:**
```python
arr = [1, 2, 3]
arr.append(4)
last = arr.pop()
length = len(arr)
index = arr.index(3)
```

**WFL:**
```wfl
store arr as [1, 2, 3]
push with arr and 4
store last as pop from arr
store length as length of arr
store index as indexof of arr and 3
```

## Classes/Containers

**Python:**
```python
class Person:
    def __init__(self, name, age):
        self.name = name
        self.age = age

    def greet(self):
        print(f"Hi, I'm {self.name}")

alice = Person("Alice", 28)
alice.greet()
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

## File Operations

**Python:**
```python
# Read
with open('data.txt') as f:
    content = f.read()

# Write
with open('output.txt', 'w') as f:
    f.write('data')
```

**WFL:**
```wfl
// Read
try:
    open file at "data.txt" for reading as myfile
    wait for store content as read content from myfile
    close file myfile
catch:
    display "Error"
end try

// Write
open file at "output.txt" for writing as outfile
wait for write content "data" into outfile
close file outfile
```

## Error Handling

**Python:**
```python
try:
    risky_operation()
except FileNotFoundError:
    print("File not found")
except Exception as e:
    print(f"Error: {e}")
finally:
    cleanup()
```

**WFL:**
```wfl
try:
    risky_operation()
when file not found:
    display "File not found"
catch:
    display "Error occurred"
finally:
    cleanup()
end try
```

## What WFL Doesn't Have

❌ **List comprehensions** - Use loops
❌ **Dict comprehensions** - Use loops
❌ **Lambda functions** - Use actions
❌ **Decorators** - Not available
❌ **Generators** - Not available
❌ **Multiple inheritance** - Single inheritance only
❌ **`with` statement** - Use try-finally

## Common Patterns

### List Comprehension

**Python:** `[x * 2 for x in numbers]`

**WFL:**
```wfl
create list doubled
end list

for each num in numbers:
    push with doubled and num times 2
end for
```

### Dictionary

**Python:** `user = {"name": "Alice", "age": 28}`

**WFL:** Use containers

```wfl
create container User:
    property name: Text
    property age: Number
end

create new User as user:
    name is "Alice"
    age is 28
end
```

### String Formatting

**Python:** `f"Name: {name}, Age: {age}"`

**WFL:**
```wfl
display "Name: " with name with ", Age: " with age
```

---

**Previous:** [← Migration from JavaScript](migration-from-javascript.md) | **Next:** [FAQ →](faq.md)
