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

## Advanced Examples

### Text Processing Pipeline

```wfl
// Multi-step text processing
action clean_and_format with user_input:
    // Remove extra whitespace (conceptual - would need trim function)
    store step1 as user_input
    
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