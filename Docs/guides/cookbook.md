# WFL Cookbook

Quick recipes for common tasks. Each recipe follows the format: Problem → Solution → Discussion.

## File Operations

### Recipe: Read a File

**Problem:** Need to read file contents.

**Solution:**
```wfl
try:
    open file at "data.txt" for reading as myfile
    wait for store file_content as read content from myfile
    close file myfile
    display file_content
catch:
    display "File not found"
end try
```

**Discussion:** Always use try-catch for file operations. Use `wait for` for async reading.

---

### Recipe: Write to a File

**Problem:** Save data to a file.

**Solution:**
```wfl
open file at "output.txt" for writing as outfile
wait for write content "My data here" into outfile
close file outfile
```

**Discussion:** `for writing` creates or overwrites. Use `for appending` to add to existing file.

---

### Recipe: Check if File Exists

**Problem:** Avoid errors by checking first.

**Solution:**
```wfl
check if file exists at "config.txt":
    // Safe to read
    open file at "config.txt" for reading as configfile
    // ...
otherwise:
    display "Config file not found"
end check
```

---

### Recipe: List All Files in Directory

**Problem:** Get all filenames.

**Solution:**
```wfl
wait for store files as list files in "."

for each filename in files:
    display filename
end for
```

---

### Recipe: Find Files by Extension

**Problem:** Get all `.wfl` files.

**Solution:**
```wfl
wait for store wfl_files as list files in "." with pattern "*.wfl"

for each wfl_file in wfl_files:
    display wfl_file
end for
```

---

## String Operations

### Recipe: Convert to Uppercase

**Problem:** Normalize text to uppercase.

**Solution:**
```wfl
store text as "hello world"
store upper as touppercase of text
display upper  // HELLO WORLD
```

---

### Recipe: Check if Text Contains Substring

**Problem:** Search for word in text.

**Solution:**
```wfl
store sentence as "The quick brown fox"

check if contains "quick" in sentence:
    display "Found it!"
end check
```

---

### Recipe: Split Text into Words

**Problem:** Parse sentence into words.

**Solution:**
```wfl
store sentence as "The quick brown fox"
store words as split of sentence by " "

for each word in words:
    display word
end for
```

---

### Recipe: Extract First N Characters

**Problem:** Get beginning of string.

**Solution:**
```wfl
store text as "Hello, World!"
store first_five as substring of text from 0 length 5
display first_five  // Hello
```

---

## List Operations

### Recipe: Create and Populate List

**Problem:** Build a list.

**Solution:**
```wfl
create list items:
    add "first"
    add "second"
    add "third"
end list

// Or use literal syntax to assign the whole list at once:
change items to ["first", "second", "third"]
```

---

### Recipe: Add Item to List

**Problem:** Append to existing list.

**Solution:**
```wfl
store items as ["first", "second", "third"]
push with items and "fourth"
```

---

### Recipe: Remove Last Item

**Problem:** Pop from list.

**Solution:**
```wfl
store items as ["first", "second", "third", "fourth"]
store last as pop of items
display "Removed: " with last
```

---

### Recipe: Find Item in List

**Problem:** Get position of item.

**Solution:**
```wfl
store items as ["first", "second", "third"]
store index as indexof of items and "second"

check if index is greater than or equal to 0:
    display "Found at position " with index
otherwise:
    display "Not found"
end check
```

---

### Recipe: Filter List

**Problem:** Extract items matching condition.

**Solution:**
```wfl
store numbers as [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

create list evens:
end list

for each num in numbers:
    store remainder as num % 2
    check if remainder is equal to 0:
        push with evens and num
    end check
end for

display evens
```

---

## Web Server

### Recipe: Start Simple Server

**Problem:** Create HTTP server.

**Solution:**
```wfl
listen on port 8080 as web_server

wait for request comes in on web_server as req
respond to req with "Hello!"
```

---

### Recipe: Serve HTML

**Problem:** Serve HTML page.

**Solution:**
```wfl
listen on port 8081 as web_server

wait for request comes in on web_server as req

store html as "<!DOCTYPE html>
<html><body><h1>WFL Server</h1></body></html>"

respond to req with html and content_type "text/html"
```

---

### Recipe: Handle Multiple Routes

**Problem:** Different responses for different paths.

**Solution:**
```wfl
listen on port 8082 as web_server

main loop:
    wait for request comes in on web_server as req
    store request_path as path of req

    check if request_path is equal to "/":
        respond to req with "Home"
    otherwise:
        check if request_path is equal to "/about":
            respond to req with "About"
        otherwise:
            respond to req with "Not found" and status 404
        end check
    end check
end loop
```

---

## Validation

### Recipe: Validate Email

**Problem:** Check email format.

**Solution:**
```wfl
create pattern email:
    one or more letter or digit
    followed by "@"
    followed by one or more letter or digit
    followed by "."
    followed by 2 to 4 letter
end pattern

check if "user@example.com" matches email:
    display "Valid"
otherwise:
    display "Invalid"
end check
```

---

### Recipe: Validate Number Range

**Problem:** Check if number in valid range.

**Solution:**
```wfl
define action called validate_age with parameters age:
    check if age is less than 0 or age is greater than 120:
        return no
    otherwise:
        return yes
    end check
end action

check if validate_age with 25:
    display "Valid age"
end check
```

---

## Utilities

### Recipe: Generate Random Number

**Problem:** Get random number in range.

**Solution:**
```wfl
store dice_roll as random_int of 1 and 6
display "You rolled: " with dice_roll
```

---

### Recipe: Get Current Date/Time

**Problem:** Timestamp operations.

**Solution:**
```wfl
store current_date as today
store current_time as now
store timestamp as current time in milliseconds

display "Date: " with current_date
display "Time: " with current_time
display "Timestamp: " with timestamp
```

---

### Recipe: Hash Data

**Problem:** Generate checksum.

**Solution:**
```wfl
store payload as "Important data"
store hash as wflhash256 of payload
display "Hash: " with hash
```

---

## Complete Applications

### Recipe: Simple Calculator

```wfl
display "=== Calculator ==="

store a as 10
store b as 5

display a with " + " with b with " = " with a plus b
display a with " - " with b with " = " with a minus b
display a with " × " with b with " = " with a times b
display a with " ÷ " with b with " = " with a divided by b
```

---

### Recipe: Temperature Converter

```wfl
define action called celsius_to_fahrenheit with parameters c:
    return c times 9 divided by 5 plus 32
end action

define action called fahrenheit_to_celsius with parameters f:
    return (f minus 32) times 5 divided by 9
end action

store c as 25
display c with "°C = " with celsius_to_fahrenheit of c with "°F"

store f as 77
display f with "°F = " with fahrenheit_to_celsius of f with "°C"
```

---

### Recipe: Word Counter

```wfl
// Seed a sample document so this recipe runs standalone
open file at "document.txt" for writing as writer
wait for write content "The quick brown fox jumps" into writer
close file writer

open file at "document.txt" for reading as docfile
wait for store file_content as read content from docfile
close file docfile

store words as split of file_content by " "
store word_count as length of words

display "Word count: " with word_count
```

---

### Recipe: File Backup Script

```wfl
wait for store files as list files in "."

for each filename in files:
    check if endswith of filename and ".txt":
        try:
            open file at filename for reading as srcfile
            wait for store file_content as read content from srcfile
            close file srcfile

            store backup_name as filename with ".backup"
            open file at backup_name for writing as backupfile
            wait for write content file_content into backupfile
            close file backupfile

            display "Backed up: " with filename
        catch:
            display "Failed: " with filename
        end try
    end check
end for
```

---

## All recipes are tested and work! Try them in your own programs.

---

**Previous:** [← WFL by Example](wfl-by-example.md) | **Next:** [Migration from JavaScript →](migration-from-javascript.md)
