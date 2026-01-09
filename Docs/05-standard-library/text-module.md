# Text Module

The Text module provides functions for string manipulation and analysis. Work with text efficiently using natural language syntax.

## Functions

### touppercase

**Purpose:** Convert text to uppercase.

**Signature:**
```wfl
touppercase of <text>
```

**Aliases:** `to_uppercase`

**Parameters:**
- `text` (Text): The string to convert

**Returns:** Text - Uppercase version

**Example:**
```wfl
display touppercase of "hello"              // Output: HELLO
display touppercase of "WFL Rocks"          // Output: WFL ROCKS
display touppercase of "123 abc"            // Output: 123 ABC
```

**Use Cases:**
- Case-insensitive comparisons
- Display formatting
- Normalization

---

### tolowercase

**Purpose:** Convert text to lowercase.

**Signature:**
```wfl
tolowercase of <text>
```

**Aliases:** `to_lowercase`

**Parameters:**
- `text` (Text): The string to convert

**Returns:** Text - Lowercase version

**Example:**
```wfl
display tolowercase of "HELLO"              // Output: hello
display tolowercase of "WFL Rocks"          // Output: wfl rocks
display tolowercase of "ABC 123"            // Output: abc 123
```

**Use Cases:**
- Normalize user input
- Case-insensitive searches
- File naming (lowercase conventions)

---

### length

**Purpose:** Get the number of characters in text.

**Signature:**
```wfl
length of <text>
```

**Parameters:**
- `text` (Text): The string to measure

**Returns:** Number - Character count

**Example:**
```wfl
display length of "hello"                   // Output: 5
display length of "WFL"                     // Output: 3
display length of ""                        // Output: 0
display length of "Hello, World!"           // Output: 13
```

**Notes:**
- Counts characters, not bytes (Unicode-aware)
- Spaces and punctuation count as characters

**Use Cases:**
- Input validation (minimum/maximum length)
- Display truncation
- Password strength checking

**Example: Validation**
```wfl
define action called validate username with parameters username:
    store len as length of username

    check if len is less than 3:
        display "Username too short (minimum 3 characters)"
        return no
    check if len is greater than 16:
        display "Username too long (maximum 16 characters)"
        return no
    otherwise:
        return yes
    end check
end action

check if validate username with "alice":
    display "Valid username"
end check
```

---

### contains

**Purpose:** Check if text contains a substring.

**Signature:**
```wfl
contains of <text> and <substring>
```

**Alternative syntax:**
```wfl
contains <substring> in <text>
```

**Parameters:**
- `text` (Text): The string to search in
- `substring` (Text): The string to search for

**Returns:** Boolean - `yes` if found, `no` otherwise

**Example:**
```wfl
store message as "Hello, World!"

display contains of message and "World"      // Output: yes
display contains of message and "world"      // Output: no (case-sensitive)
display contains of message and "Universe"   // Output: no

// Alternative syntax:
check if contains "Hello" in message:
    display "Found Hello"
end check
```

**Notes:**
- Case-sensitive search
- Returns yes/no, not position

**Use Cases:**
- Search functionality
- Keyword filtering
- Validation checks

---

### substring

**Purpose:** Extract a portion of text.

**Signature:**
```wfl
substring of <text> from <start> length <length>
```

**Alternative:**
```wfl
substring of <text> and <start> and <length>
```

**Parameters:**
- `text` (Text): The source string
- `start` (Number): Starting index (0-based)
- `length` (Number): Number of characters to extract

**Returns:** Text - The extracted substring

**Example:**
```wfl
store text as "Hello, World!"

display substring of text from 0 length 5    // Output: Hello
display substring of text from 7 length 5    // Output: World
display substring of text and 0 and 7        // Output: Hello,
```

**Notes:**
- Zero-indexed (first character is position 0)
- Character-based, not byte-based (Unicode-safe)
- If length exceeds text, returns to end of string

**Use Cases:**
- Extract parts of strings
- First/last N characters
- Parse structured text

**Example: Extract First Word**
```wfl
store sentence as "Hello world from WFL"
store first_word as substring of sentence from 0 length 5
display "First word: " with first_word
// Output: First word: Hello
```

---

### trim

**Purpose:** Remove leading and trailing whitespace.

**Signature:**
```wfl
trim of <text>
```

**Parameters:**
- `text` (Text): The string to trim

**Returns:** Text - String with whitespace removed from both ends

**Example:**
```wfl
store padded as "  hello world  "
display "Original: '" with padded with "'"
display "Trimmed: '" with trim of padded with "'"
```

**Output:**
```
Original: '  hello world  '
Trimmed: 'hello world'
```

**Whitespace removed:**
- Spaces
- Tabs
- Newlines
- Other whitespace characters

**Use Cases:**
- Clean user input
- Parse data files
- Normalize text

---

### starts_with

**Purpose:** Check if text starts with a specific prefix.

**Signature:**
```wfl
starts_with of <text> and <prefix>
```

**Aliases:** `startswith`

**Parameters:**
- `text` (Text): The string to check
- `prefix` (Text): The prefix to look for

**Returns:** Boolean - `yes` if starts with prefix, `no` otherwise

**Example:**
```wfl
store filename as "report.pdf"

display starts_with of filename and "report"    // Output: yes
display starts_with of filename and "doc"       // Output: no
display starts_with of filename and "Report"    // Output: no (case-sensitive)
```

**Use Cases:**
- File type checking
- URL routing
- Command parsing

**Example: File Type Check**
```wfl
define action called is image with parameters filename:
    check if starts_with of filename and "image":
        return yes
    check if ends_with of filename and ".png":
        return yes
    check if ends_with of filename and ".jpg":
        return yes
    otherwise:
        return no
    end check
end action
```

---

### ends_with

**Purpose:** Check if text ends with a specific suffix.

**Signature:**
```wfl
ends_with of <text> and <suffix>
```

**Aliases:** `endswith`

**Parameters:**
- `text` (Text): The string to check
- `suffix` (Text): The suffix to look for

**Returns:** Boolean - `yes` if ends with suffix, `no` otherwise

**Example:**
```wfl
store filename as "document.pdf"

display ends_with of filename and ".pdf"        // Output: yes
display ends_with of filename and ".txt"        // Output: no
display ends_with of filename and ".PDF"        // Output: no (case-sensitive)
```

**Use Cases:**
- File extension checking
- URL pattern matching
- String validation

---

### string_split

**Purpose:** Split text into a list using a delimiter.

**Signature:**
```wfl
split of <text> by <delimiter>
```

**Alternative:**
```wfl
string_split of <text> and <delimiter>
```

**Parameters:**
- `text` (Text): The string to split
- `delimiter` (Text): The separator string

**Returns:** List - List of text segments

**Example:**
```wfl
store csv as "Alice,28,Developer"
store parts as split of csv by ","

display "Parts:"
for each part in parts:
    display "  - " with part
end for
```

**Output:**
```
Parts:
  - Alice
  - 28
  - Developer
```

**More examples:**
```wfl
store sentence as "The quick brown fox"
store words as split of sentence by " "
display "Word count: " with length of words
// Output: Word count: 4

store path as "home/user/documents"
store directories as split of path by "/"
// directories = ["home", "user", "documents"]
```

**Use Cases:**
- Parse CSV files
- Split sentences into words
- Parse paths
- Extract tokens

---

## Complete Example

Using all text functions together:

```wfl
display "=== Text Module Demo ==="
display ""

store input as "  Hello, WFL World!  "

// Display original
display "Original: '" with input with "'"

// Length
display "Length: " with length of input

// Case conversion
display "Uppercase: " with touppercase of input
display "Lowercase: " with tolowercase of input

// Trim
store trimmed as trim of input
display "Trimmed: '" with trimmed with "'"

// Contains
check if contains of trimmed and "WFL":
    display "Contains 'WFL': yes"
end check

// Substring
store first_five as substring of trimmed from 0 length 5
display "First 5 chars: " with first_five

// Starts/Ends with
display "Starts with 'Hello': " with starts_with of trimmed and "Hello"
display "Ends with '!': " with ends_with of trimmed and "!"

// Split
store words as split of trimmed by " "
display "Words:"
for each word in words:
    display "  - " with word
end for

display ""
display "=== Demo Complete ==="
```

**Output:**
```
=== Text Module Demo ===

Original: '  Hello, WFL World!  '
Length: 21
Uppercase:   HELLO, WFL WORLD!
Lowercase:   hello, wfl world!
Trimmed: 'Hello, WFL World!'
Contains 'WFL': yes
First 5 chars: Hello
Starts with 'Hello': yes
Ends with '!': yes
Words:
  - Hello,
  - WFL
  - World!

=== Demo Complete ===
```

## Common Patterns

### Case-Insensitive Comparison

```wfl
define action called equals ignore case with parameters text1 and text2:
    store lower1 as tolowercase of text1
    store lower2 as tolowercase of text2
    return lower1 is equal to lower2
end action

check if equals ignore case with "Hello" and "HELLO":
    display "Match (case-insensitive)"
end check
```

### Extract File Extension

```wfl
define action called get extension with parameters filename:
    store parts as split of filename by "."
    store part_count as length of parts

    check if part_count is greater than 1:
        store last_index as part_count minus 1
        return parts[last_index]
    otherwise:
        return ""
    end check
end action

store ext as get extension with "document.pdf"
display "Extension: " with ext
// Output: Extension: pdf
```

### Validate Email Format (Simple)

```wfl
define action called looks like email with parameters email:
    check if contains of email and "@":
        check if contains of email and ".":
            return yes
        end check
    end check
    return no
end action

check if looks like email with "user@example.com":
    display "Valid email format"
end check
```

### Title Case

```wfl
define action called title case with parameters text:
    store words as split of text by " "
    create list titled_words
    end list

    for each word in words:
        store len as length of word
        check if len is greater than 0:
            store first_char as substring of word from 0 length 1
            store rest as substring of word from 1 length len minus 1
            store titled as touppercase of first_char with tolowercase of rest
            push with titled_words and titled
        end check
    end for

    // Join words (manual - no join function yet)
    store result as ""
    for each titled in titled_words:
        check if length of result is greater than 0:
            change result to result with " "
        end check
        change result to result with titled
    end for

    return result
end action

display title case with "hello world from wfl"
// Output: Hello World From Wfl
```

### Word Count

```wfl
define action called word count with parameters text:
    store trimmed as trim of text
    store words as split of trimmed by " "
    return length of words
end action

store count as word count with "The quick brown fox"
display "Words: " with count
// Output: Words: 4
```

## Best Practices

✅ **Use trim on user input:** Remove accidental whitespace

✅ **Check length before substring:** Prevent out-of-bounds errors

✅ **tolowercase for comparisons:** Case-insensitive matching

✅ **Use contains for simple searches:** Before pattern matching

✅ **Split for parsing:** Parse CSV, paths, sentences

❌ **Don't assume case:** Always normalize if case doesn't matter

❌ **Don't substring without bounds checking:** Validate indices

❌ **Don't forget empty strings:** Check length before operations

## What You've Learned

In this module, you learned:

✅ **touppercase / tolowercase** - Case conversion
✅ **length** - String length in characters
✅ **contains** - Substring search
✅ **substring** - Extract portions of text
✅ **trim** - Remove whitespace
✅ **starts_with / ends_with** - Prefix/suffix checking
✅ **string_split** - Split into list
✅ **Common patterns** - Title case, word count, validation
✅ **Best practices** - Input normalization, bounds checking

## Next Steps

Continue exploring the standard library:

**[List Module →](list-module.md)**
Operations for working with lists.

**[Pattern Module →](pattern-module.md)**
Advanced text matching and validation.

**[Filesystem Module →](filesystem-module.md)**
File operations that work with text.

Or return to:
**[Standard Library Index →](index.md)**

---

**Previous:** [← Math Module](math-module.md) | **Next:** [List Module →](list-module.md)
