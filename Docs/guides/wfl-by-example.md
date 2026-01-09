# WFL by Example

Learn WFL through 25 standalone examples. Each example is complete, tested, and ready to run.

## Beginner Examples

### 1. Hello World

```wfl
display "Hello, World!"
```

**Run:** `wfl hello.wfl`

---

### 2. Variables and Display

```wfl
store name as "Alice"
store age as 28

display "Name: " with name
display "Age: " with age
```

---

### 3. Simple Math

```wfl
store x as 10
store y as 5

display "Sum: " with x plus y
display "Product: " with x times y
display "Difference: " with x minus y
```

---

### 4. Conditionals

```wfl
store age as 20

check if age is greater than or equal to 18:
    display "Adult"
otherwise:
    display "Minor"
end check
```

---

### 5. Count Loop

```wfl
count from 1 to 10:
    display count
end count
```

---

### 6. For Each Loop

```wfl
create list fruits:
    add "apple"
    add "banana"
    add "orange"
end list

for each fruit in fruits:
    display fruit
end for
```

---

### 7. Simple Action

```wfl
define action called greet with parameters name:
    display "Hello, " with name with "!"
end action

call greet with "World"
call greet with "WFL"
```

---

### 8. Action with Return

```wfl
define action called double with parameters n:
    return n times 2
end action

store result as double with 21
display "Double of 21 is " with result
```

---

## Intermediate Examples

### 9. File Reading

```wfl
try:
    open file at "data.txt" for reading as myfile
    wait for store content as read content from myfile
    close file myfile
    display content
catch:
    display "Error: Could not read file"
end try
```

---

### 10. File Writing

```wfl
open file at "output.txt" for writing as outfile
wait for write content "Line 1\n" into outfile
wait for append content "Line 2\n" into outfile
close file outfile

display "File written"
```

---

### 11. List Processing

```wfl
store numbers as [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

create list evens
end list

for each num in numbers:
    check if num % 2 is equal to 0:
        push with evens and num
    end check
end for

display "Even numbers: " with evens
```

---

### 12. String Manipulation

```wfl
store text as "  Hello, WFL!  "

display "Original: '" with text with "'"
display "Trimmed: '" with trim of text with "'"
display "Uppercase: " with touppercase of trim of text
display "Length: " with length of text
```

---

### 13. Pattern Matching

```wfl
create pattern email:
    one or more letter or digit
    followed by "@"
    followed by one or more letter or digit
    followed by "."
    followed by 2 to 4 letter
end pattern

check if "user@example.com" matches email:
    display "Valid email"
otherwise:
    display "Invalid email"
end check
```

---

### 14. Error Handling

```wfl
try:
    store result as 10 divided by 0
catch:
    display "Error caught: Division by zero"
finally:
    display "Cleanup complete"
end try
```

---

### 15. Temperature Converter

```wfl
define action called c_to_f with parameters celsius:
    return celsius times 9 divided by 5 plus 32
end action

store temp_c as 25
store temp_f as c_to_f with temp_c
display temp_c with "°C = " with temp_f with "°F"
```

---

## Advanced Examples

### 16. Simple Web Server

```wfl
listen on port 8080 as server
display "Server at http://127.0.0.1:8080"

wait for request comes in on server as req

check if path is equal to "/":
    respond to req with "Hello from WFL!"
otherwise:
    respond to req with "Not found" and status 404
end check
```

---

### 17. File Listing

```wfl
wait for store files as list files in "."

display "Files in current directory:"
for each filename in files:
    check if file exists at filename:
        store size as file size at filename
        display "  " with filename with " (" with size with " bytes)"
    end check
end for
```

---

### 18. Container (Class)

```wfl
create container Person:
    property name: Text
    property age: Number

    action introduce:
        display "I'm " with name with ", age " with age
    end
end

create new Person as alice:
    name is "Alice"
    age is 28
end

alice.introduce()
```

---

### 19. Random Numbers

```wfl
display "Rolling dice..."

count from 1 to 10:
    store roll as random_int between 1 and 6
    display "Roll " with count with ": " with roll
end count
```

---

### 20. Hash Generation

```wfl
store data as "Sensitive information"
store hash as wflhash256 of data

display "Data: " with data
display "Hash: " with substring of hash from 0 length 32 with "..."
```

---

### 21. CSV Processing

```wfl
store csv as "Alice,28,Developer\nBob,35,Designer"
store lines as split of csv by "\n"

display "Users:"
for each line in lines:
    store parts as split of line by ","
    display "  Name: " with parts[0] with ", Age: " with parts[1] with ", Role: " with parts[2]
end for
```

---

### 22. Configuration Loader

```wfl
define action called load_config with parameters filename:
    try:
        open file at filename for reading as configfile
        wait for store config as read content from configfile
        close file configfile
        return config
    catch:
        display "Config not found, using defaults"
        return "default config"
    end try
end action

store app_config as load_config with "app.config"
display "Config: " with app_config
```

---

### 23. Subprocess Execution

```wfl
try:
    wait for execute command "git status" as output
    display "Git status executed"
catch:
    display "Git command failed - is this a git repository?"
end try
```

---

### 24. Time Operations

```wfl
store today_date as today
store next_week as add_days of today_date and 7

display "Today: " with today_date
display "Next week: " with next_week

store days_diff as days_between of today_date and next_week
display "Days between: " with days_diff
```

---

### 25. Complete Application: Task Manager

```wfl
display "=== Simple Task Manager ==="

create list tasks
end list

// Add tasks
push with tasks and "Learn WFL"
push with tasks and "Build web server"
push with tasks and "Write documentation"

// Display tasks
display "Tasks:"
store task_num as 1
for each task in tasks:
    display "  " with task_num with ". " with task
    add 1 to task_num
end for

// Mark first task complete
store completed as pop from tasks
display ""
display "Completed: " with completed

// Show remaining
display ""
display "Remaining tasks:"
store remaining_num as 1
for each task in tasks:
    display "  " with remaining_num with ". " with task
    add 1 to remaining_num
end for
```

---

## Running the Examples

All examples are tested and work. To run:

1. Copy example to a `.wfl` file
2. Run with `wfl filename.wfl`
3. Experiment by modifying the code

## Learning Path

**Beginners:** Work through examples 1-15 in order
**Experienced:** Jump to examples 16-25 for advanced features
**Reference:** Use as copy-paste templates

## More Examples

- **TestPrograms/** - 90+ comprehensive examples in repository
- **Documentation** - Examples throughout all sections
- **Cookbook** - Recipe-based examples

---

**Previous:** [← Best Practices](../06-best-practices/index.md) | **Next:** [Cookbook →](cookbook.md)
