# WFL Text Module API Reference

## Overview

The Text module provides functions for string manipulation and text processing. These functions handle Unicode text properly and follow WFL's natural language conventions.

## Functions

### `length(text)`

Returns the number of characters in a text string.

**Parameters:**
- `text` (Text): The text to measure

**Returns:** Number (character count)

**Examples:**

```wfl
// Basic length calculation
store message as "Hello, World!"
store char_count as length of message
display "Message length: " with char_count  // 13

// Empty string
store empty as ""
store empty_length as length of empty
display "Empty length: " with empty_length  // 0

// Unicode characters
store emoji_text as "Hello 👋 World 🌍"
store emoji_length as length of emoji_text
display "Emoji text length: " with emoji_length  // 15
```

**Natural Language Variants:**
```wfl
// All equivalent ways to get length
store len1 as length of text
store len2 as size of text
store len3 as character count of text
store len4 as how long is text
```

**Practical Use Cases:**

```wfl
// Input validation
action validate_password with password:
    store pwd_length as length of password
    check if pwd_length < 8:
        display "Password must be at least 8 characters"
        return no
    end
    return yes
end

// Text truncation check
action needs_truncation with text and max_length:
    return length of text > max_length
end

// Progress display
action show_typing_progress with text and target_length:
    store current_length as length of text
    store percentage as (current_length / target_length) * 100
    display "Progress: " with percentage with "%"
end
```

---

### `touppercase(text)` / `to_uppercase(text)`

Converts text to uppercase letters.

**Parameters:**
- `text` (Text): The text to convert

**Returns:** Text (all uppercase)

**Examples:**

```wfl
// Basic uppercase conversion
store greeting as "hello world"
store loud_greeting as touppercase of greeting
display loud_greeting  // "HELLO WORLD"

// Mixed case
store mixed as "HeLLo WoRLd"
store normalized as touppercase of mixed
display normalized  // "HELLO WORLD"

// With special characters
store sentence as "hello, world! 123"
store upper_sentence as touppercase of sentence
display upper_sentence  // "HELLO, WORLD! 123"
```

**Natural Language Variants:**
```wfl
// All equivalent ways to convert to uppercase
store result as touppercase of text
store result as to_uppercase of text
store result as uppercase of text
store result as make uppercase text
store result as convert text to uppercase
```

**Practical Use Cases:**

```wfl
// Case-insensitive comparison
action texts_match with text1 and text2:
    store upper1 as touppercase of text1
    store upper2 as touppercase of text2
    return upper1 is upper2
end

// Command processing
action process_command with user_input:
    store command as touppercase of user_input
    check if command is "HELP":
        show_help
    check if command is "QUIT":
        exit_program
    otherwise:
        display "Unknown command: " with user_input
    end
end

// Acronym generation
action create_acronym with words:
    store acronym as ""
    count word in words:
        store first_char as substring of word and 0 and 1
        store upper_char as touppercase of first_char
        store acronym as acronym with upper_char
    end
    return acronym
end
```

---

### `tolowercase(text)` / `to_lowercase(text)`

Converts text to lowercase letters.

**Parameters:**
- `text` (Text): The text to convert

**Returns:** Text (all lowercase)

**Examples:**

```wfl
// Basic lowercase conversion
store shout as "HELLO WORLD"
store whisper as tolowercase of shout
display whisper  // "hello world"

// Mixed case normalization
store mixed as "HeLLo WoRLd"
store normalized as tolowercase of mixed
display normalized  // "hello world"

// Preserving numbers and symbols
store complex as "Hello, World! 123"
store lower_complex as tolowercase of complex
display lower_complex  // "hello, world! 123"
```

**Natural Language Variants:**
```wfl
// All equivalent ways to convert to lowercase
store result as tolowercase of text
store result as to_lowercase of text
store result as lowercase of text
store result as make lowercase text
store result as convert text to lowercase
```

**Practical Use Cases:**

```wfl
// Email normalization
action normalize_email with email:
    return tolowercase of email
end

// Search functionality
action search_items with items and query:
    store lower_query as tolowercase of query
    store matches as []
    
    count item in items:
        store lower_item as tolowercase of item
        check if contains of lower_item and lower_query:
            push of matches and item
        end
    end
    
    return matches
end

// URL slug generation
action create_slug with title:
    store lower_title as tolowercase of title
    // Would need additional functions for full slug creation
    return lower_title
end
```

---

### `contains(text, search)`

Checks if a text string contains another text string as a substring.

**Parameters:**
- `text` (Text): The text to search in
- `search` (Text): The substring to search for

**Returns:** Boolean (yes if found, no if not found)

**Examples:**

```wfl
// Basic substring search
store message as "Hello, World!"
store has_world as contains of message and "World"
display "Contains 'World': " with has_world  // yes

store has_universe as contains of message and "Universe"
display "Contains 'Universe': " with has_universe  // no

// Case-sensitive search
store text as "Hello"
store has_hello as contains of text and "hello"
display "Contains 'hello': " with has_hello  // no (case-sensitive)

// Empty string search
store has_empty as contains of "anything" and ""
display "Contains empty string: " with has_empty  // yes
```

**Natural Language Variants:**
```wfl
// All equivalent ways to check contains
check if contains of text and search
check if text contains search
check if search is in text
check if text has search
```

**Practical Use Cases:**

```wfl
// Input filtering
action is_profanity with text:
    store bad_words as ["bad", "evil", "nasty"]
    store lower_text as tolowercase of text
    
    count word in bad_words:
        check if contains of lower_text and word:
            return yes
        end
    end
    
    return no
end

// File type checking
action is_image_file with filename:
    store lower_name as tolowercase of filename
    store image_extensions as [".jpg", ".png", ".gif", ".bmp"]
    
    count extension in image_extensions:
        check if contains of lower_name and extension:
            return yes
        end
    end
    
    return no
end

// Search highlighting
action highlight_search with text and search_term:
    check if contains of text and search_term:
        display "Found '" with search_term with "' in: " with text
    otherwise:
        display "'" with search_term with "' not found in text"
    end
end
```

---

### `substring(text, start, length)`

Extracts a portion of text starting at a specific position.

**Parameters:**
- `text` (Text): The source text
- `start` (Number): Starting position (0-based index)
- `length` (Number): Number of characters to extract

**Returns:** Text (extracted substring)

**Examples:**

```wfl
// Basic substring extraction
store message as "Hello, World!"
store first_five as substring of message and 0 and 5
display first_five  // "Hello"

store world_part as substring of message and 7 and 5
display world_part  // "World"

// Beyond string bounds (safe)
store beyond as substring of message and 10 and 20
display beyond  // "rld!" (stops at end of string)

store way_beyond as substring of message and 100 and 5
display way_beyond  // "" (empty string if start is beyond end)
```

**Natural Language Variants:**
```wfl
// All equivalent ways to get substring
store result as substring of text and start and length
store result as substr of text and start and length
store result as slice of text from start for length
store result as extract length characters from text starting at start
```

**Practical Use Cases:**

```wfl
// Extract file extension
action get_file_extension with filename:
    store name_length as length of filename
    store dot_position as name_length - 4  // Assuming 3-char extension
    return substring of filename and dot_position and 4
end

// Extract initials
action get_initials with full_name:
    store first_initial as substring of full_name and 0 and 1
    // Would need additional string functions to find space and get second initial
    return touppercase of first_initial
end

// Preview text generation
action create_preview with full_text and max_length:
    check if length of full_text <= max_length:
        return full_text
    end
    
    store preview as substring of full_text and 0 and (max_length - 3)
    return preview with "..."
end

// Parse coordinates (example: "x:10,y:20")
action parse_x_coordinate with coord_string:
    // Assumes format "x:NUMBER,y:NUMBER"
    store x_start as 2  // Position after "x:"
    store comma_pos as 5  // Simplified - would need indexOf function
    store x_length as comma_pos - x_start
    return substring of coord_string and x_start and x_length
end
```

---

### `trim(text)`

Removes leading and trailing whitespace from text.

**Parameters:**
- `text` (Text): The text to trim

**Returns:** Text (with whitespace removed from start and end)

**Examples:**

```wfl
// Remove spaces from both ends
store messy as "  Hello, World!  "
store clean as trim of messy
display clean  // "Hello, World!" (no spaces at ends)

// Remove tabs and newlines
store whitespace_text as "\t\n  Centered Text  \n\t"
store trimmed as trim of whitespace_text
display trimmed  // "Centered Text"

// No effect on already-trimmed text
store already_clean as "No extra spaces"
store still_clean as trim of already_clean
display still_clean  // "No extra spaces"

// Only internal spaces remain
store internal as "  Multiple   spaces   inside  "
store result as trim of internal
display result  // "Multiple   spaces   inside" (internal spaces preserved)
```

**Natural Language Variants:**
```wfl
// All equivalent ways to trim text
store result as trim of text
store result as trimmed text
store result as remove whitespace from text
store result as strip text
```

**Practical Use Cases:**

```wfl
// Clean user input
action clean_user_input with input:
    store cleaned as trim of input
    check if length of cleaned is 0:
        return "Error: Empty input after trimming"
    end
    return cleaned
end

// Email validation preparation
action prepare_email with email:
    store trimmed_email as trim of email
    store lower_email as tolowercase of trimmed_email
    return lower_email
end

// Form data processing
action process_form_data with form_fields:
    store cleaned_fields as []
    count field in form_fields:
        store cleaned_field as trim of field
        push of cleaned_fields and cleaned_field
    end
    return cleaned_fields
end

// Password comparison (don't trim passwords in real apps!)
action check_password with entered and stored:
    // Example only - real password comparison should not trim
    store clean_entered as trim of entered
    return clean_entered is stored
end
```

---

### `starts_with(text, prefix)`

Checks if text begins with a specific prefix.

**Parameters:**
- `text` (Text): The text to check
- `prefix` (Text): The prefix to look for

**Returns:** Boolean (yes if text starts with prefix, no otherwise)

**Examples:**

```wfl
// Basic prefix check
store filename as "document.txt"
store is_doc as starts_with of filename and "doc"
display is_doc  // yes

store is_report as starts_with of filename and "report"
display is_report  // no

// Case-sensitive check
store greeting as "Hello, World!"
store starts_hello as starts_with of greeting and "Hello"
display starts_hello  // yes

store starts_hello_lower as starts_with of greeting and "hello"
display starts_hello_lower  // no (case-sensitive)

// Empty prefix always matches
store any_text as "anything"
store starts_empty as starts_with of any_text and ""
display starts_empty  // yes (empty prefix always matches)
```

**Natural Language Variants:**
```wfl
// All equivalent ways to check prefix
check if starts_with of text and prefix
check if text starts with prefix
check if text begins with prefix
check if prefix is at start of text
```

**Practical Use Cases:**

```wfl
// Protocol detection
action is_secure_url with url:
    return starts_with of url and "https://"
end

// Command parsing
action is_admin_command with command:
    store lower_command as tolowercase of command
    return starts_with of lower_command and "admin:"
end

// File extension grouping
action is_text_file with filename:
    store lower_name as tolowercase of filename
    store extensions as ["txt", "md", "log"]

    count ext in extensions:
        // Check if filename starts with pattern (simplified)
        check if starts_with of lower_name and ext:
            return yes
        end
    end

    return no
end

// Path validation
action is_absolute_path with path:
    // Unix-style absolute path
    store is_unix_absolute as starts_with of path and "/"

    // Windows-style absolute path (simplified)
    check if length of path >= 2:
        store second_char as substring of path and 1 and 1
        check if second_char is ":":
            return yes  // Like "C:"
        end
    end

    return is_unix_absolute
end

// Version string parsing
action is_beta_version with version:
    return starts_with of version and "beta-"
end
```

---

### `ends_with(text, suffix)`

Checks if text ends with a specific suffix.

**Parameters:**
- `text` (Text): The text to check
- `suffix` (Text): The suffix to look for

**Returns:** Boolean (yes if text ends with suffix, no otherwise)

**Examples:**

```wfl
// Basic suffix check
store filename as "document.txt"
store is_txt as ends_with of filename and ".txt"
display is_txt  // yes

store is_doc as ends_with of filename and ".doc"
display is_doc  // no

// Case-sensitive check
store sentence as "Hello, World!"
store ends_exclaim as ends_with of sentence and "!"
display ends_exclaim  // yes

store ends_period as ends_with of sentence and "."
display ends_period  // no

// Empty suffix always matches
store any_text as "anything"
store ends_empty as ends_with of any_text and ""
display ends_empty  // yes (empty suffix always matches)
```

**Natural Language Variants:**
```wfl
// All equivalent ways to check suffix
check if ends_with of text and suffix
check if text ends with suffix
check if text finishes with suffix
check if suffix is at end of text
```

**Practical Use Cases:**

```wfl
// File type detection
action is_image_file with filename:
    store lower_name as tolowercase of filename
    store image_exts as [".jpg", ".jpeg", ".png", ".gif", ".bmp"]

    count extension in image_exts:
        check if ends_with of lower_name and extension:
            return yes
        end
    end

    return no
end

// Sentence detection
action is_question with text:
    return ends_with of text and "?"
end

// URL path checking
action is_api_endpoint with path:
    return ends_with of path and "/api"
end

// Backup file detection
action is_backup_file with filename:
    store backup_suffixes as [".bak", ".backup", "~", ".old"]

    count suffix in backup_suffixes:
        check if ends_with of filename and suffix:
            return yes
        end
    end

    return no
end

// Plural detection (simplified)
action appears_plural with word:
    store lower_word as tolowercase of word
    check if ends_with of lower_word and "s":
        return yes
    end
    check if ends_with of lower_word and "es":
        return yes
    end
    return no
end
```

---

### `string_split(text, delimiter)`

Splits text into a list of parts using a delimiter.

**Parameters:**
- `text` (Text): The text to split
- `delimiter` (Text): The string to split on (cannot be empty)

**Returns:** List (list of text parts)

**Examples:**

```wfl
// Split by comma
store csv as "apple,banana,orange"
store fruits as string_split of csv and ","
display fruits  // ["apple", "banana", "orange"]
display length of fruits  // 3

// Split by space
store sentence as "Hello world from WFL"
store words as string_split of sentence and " "
display words  // ["Hello", "world", "from", "WFL"]

// Split with multi-character delimiter
store data as "one::two::three"
store parts as string_split of data and "::"
display parts  // ["one", "two", "three"]

// Split results in empty strings
store text as "a,,b"
store parts as string_split of text and ","
display parts  // ["a", "", "b"] (empty string in middle)

// No delimiter found
store no_match as "no commas here"
store result as string_split of no_match and ","
display result  // ["no commas here"] (returns list with one element)
```

**Natural Language Variants:**
```wfl
// All equivalent ways to split text
store result as string_split of text and delimiter
store result as split text by delimiter
store result as divide text using delimiter
store result as break text at delimiter
```

**Practical Use Cases:**

```wfl
// Parse CSV data
action parse_csv_line with line:
    store fields as string_split of line and ","
    return fields
end

// Parse command with arguments
action parse_command with input:
    store parts as string_split of input and " "
    store command as index of parts and 0
    // Rest of parts are arguments
    return parts
end

// Extract email username
action get_email_username with email:
    store parts as string_split of email and "@"
    check if length of parts is 2:
        return index of parts and 0
    otherwise:
        return "Invalid email"
    end
end

// Parse URL path segments
action parse_url_path with path:
    // Remove leading slash if present
    store clean_path as path
    check if starts_with of path and "/":
        store clean_path as substring of path and 1 and (length of path - 1)
    end

    store segments as string_split of clean_path and "/"
    return segments
end

// Process multi-line text
action split_into_lines with text:
    store lines as string_split of text and "\n"
    return lines
end

// Parse key-value pairs
action parse_config_line with line:
    store parts as string_split of line and "="
    check if length of parts is 2:
        store key as trim of index of parts and 0
        store value as trim of index of parts and 1
        display "Config: " with key with " = " with value
    otherwise:
        display "Invalid config line"
    end
end

// Word frequency counter
action count_word_frequency with text and target_word:
    store words as string_split of text and " "
    store count as 0

    count word in words:
        store lower_word as tolowercase of word
        store lower_target as tolowercase of target_word
        check if lower_word is lower_target:
            store count as count + 1
        end
    end

    return count
end
```

**Error Handling:**

```wfl
// Empty delimiter causes error
try:
    store result as string_split of "text" and ""
when error:
    display "Error: Empty delimiter not allowed"
end try

// Safe splitting with error handling
action safe_split with text and delimiter:
    check if length of delimiter is 0:
        display "Warning: Empty delimiter, returning original text"
        return [text]
    end

    try:
        return string_split of text and delimiter
    when error:
        display "Error during split: " with error message
        return [text]
    end try
end
```

**Integration with List Module:**

```wfl
// Process split results
action process_csv with csv_line:
    store fields as string_split of csv_line and ","

    // Trim each field
    store cleaned_fields as []
    count field in fields:
        store trimmed as trim of field
        push of cleaned_fields and trimmed
    end

    return cleaned_fields
end

// Join split parts back together
action replace_delimiter with text and old_delim and new_delim:
    store parts as string_split of text and old_delim
    // Would need join function to recombine with new delimiter
    // This is conceptual without a join function
    return parts
end
```

---

## Advanced Examples

### Text Processing Pipeline

```wfl
// Multi-step text processing
action clean_and_format with user_input:
    // Remove extra whitespace
    store step1 as trim of user_input
    
    // Convert to lowercase for processing
    store step2 as tolowercase of step1
    
    // Check for forbidden content
    check if contains of step2 and "spam":
        return "Content blocked"
    end
    
    // Create title case (first letter uppercase)
    store first_char as substring of step2 and 0 and 1
    store rest_chars as substring of step2 and 1 and (length of step2 - 1)
    store title_case as touppercase of first_char with rest_chars
    
    return title_case
end
```

### Search and Replace (Conceptual)

```wfl
// Simple character replacement using available functions
action replace_character with text and old_char and new_char:
    store result as ""
    store text_length as length of text
    
    count i from 0 to text_length - 1:
        store current_char as substring of text and i and 1
        check if current_char is old_char:
            store result as result with new_char
        otherwise:
            store result as result with current_char
        end
    end
    
    return result
end
```

### Text Statistics

```wfl
// Count words (simplified - assumes single spaces)
action count_words with text:
    store word_count as 1
    store text_length as length of text
    
    check if text_length is 0:
        return 0
    end
    
    count i from 0 to text_length - 1:
        store char as substring of text and i and 1
        check if char is " ":
            store word_count as word_count + 1
        end
    end
    
    return word_count
end

// Check if text is all uppercase
action is_all_uppercase with text:
    store upper_version as touppercase of text
    return text is upper_version
end

// Find common substring (simplified)
action has_common_start with text1 and text2:
    store min_length as length of text1
    check if length of text2 < min_length:
        store min_length as length of text2
    end
    
    count i from 0 to min_length - 1:
        store char1 as substring of text1 and i and 1
        store char2 as substring of text2 and i and 1
        check if char1 is not char2:
            return no
        end
    end
    
    return yes
end
```

## Integration with Other Modules

### With List Module

```wfl
// Split text into characters (conceptual)
action text_to_characters with text:
    store chars as []
    store text_length as length of text
    
    count i from 0 to text_length - 1:
        store char as substring of text and i and 1
        push of chars and char
    end
    
    return chars
end
```

### With Math Module

```wfl
// Text-based calculations
action calculate_reading_time with text and words_per_minute:
    store char_count as length of text
    store estimated_words as char_count / 5  // Average word length
    store minutes as estimated_words / words_per_minute
    return ceil of minutes  // Round up to next minute
end
```

## Error Handling

Text functions handle edge cases gracefully:

- **Empty strings**: All functions work correctly with empty input
- **Unicode**: Full Unicode support for international characters
- **Out of bounds**: `substring` returns empty string or truncates safely
- **Invalid indices**: Negative indices are treated as 0

```wfl
// Safe text operations
action safe_text_operation with text:
    check if isnothing of text:
        display "Text is nothing"
        return ""
    end
    
    check if length of text is 0:
        display "Text is empty"
        return ""
    end
    
    // Proceed with text operations
    store processed as touppercase of text
    return processed
end
```

## Performance Notes

- String operations create new strings (immutable)
- Unicode-aware operations may be slower than ASCII-only
- `contains` uses efficient substring search algorithms
- `substring` is optimized for UTF-8 text

## Best Practices

1. **Case-insensitive comparisons**: Always normalize case before comparing
2. **Input validation**: Check for empty or null strings
3. **Unicode awareness**: Remember that character count may not equal byte count
4. **Efficient searching**: Use `contains` before more complex operations

```wfl
// Example of best practices
action validate_and_process with user_text:
    // Validate input
    check if isnothing of user_text:
        return "Error: No text provided"
    end
    
    check if length of user_text is 0:
        return "Error: Empty text"
    end
    
    // Normalize for processing
    store normalized as tolowercase of user_text
    
    // Check for required content
    check if contains of normalized and "required":
        store result as touppercase of user_text
        return "Processed: " with result
    otherwise:
        return "Error: Required content not found"
    end
end
```

## See Also

- [Core Module](core-module.md) - Basic utilities and type checking
- [Math Module](math-module.md) - Numeric operations  
- [List Module](list-module.md) - Collection operations
- [Pattern Module](pattern-module.md) - Regular expression functions
- [WFL Language Reference](../language-reference/wfl-spec.md) - Complete language specification