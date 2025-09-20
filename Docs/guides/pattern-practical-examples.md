# Practical Pattern Matching Examples

This guide provides real-world examples of WFL pattern matching using only **implemented features**. All examples have been tested and work correctly.

## Table of Contents
- [Data Validation](#data-validation)
- [Text Processing](#text-processing)
- [Log Parsing](#log-parsing)
- [Configuration Files](#configuration-files)
- [Web Development](#web-development)
- [Performance Tips](#performance-tips)

## Data Validation

### Email Validation

```wfl
// Simple but effective email validation
create pattern email_validator:
    one or more letter or digit or "." or "_" or "-"
    "@"
    one or more letter or digit or "." or "-"
    "."
    2 to 4 letter
end pattern

// Usage
store user_email as "user@example.com"
check if user_email matches email_validator:
    display "Valid email format"
otherwise:
    display "Invalid email format"
end check
```

### Phone Number Validation

```wfl
// US phone number with flexible formatting
create pattern us_phone_flexible:
    optional "("
    exactly 3 digit
    optional ")"
    optional " " or "-"
    exactly 3 digit
    optional " " or "-"
    exactly 4 digit
end pattern

// Test multiple formats
store phone1 as "(555) 123-4567"
store phone2 as "555-123-4567"
store phone3 as "5551234567"

check if phone1 matches us_phone_flexible:
    display "Format 1 valid"
end check

check if phone2 matches us_phone_flexible:
    display "Format 2 valid"
end check

check if phone3 matches us_phone_flexible:
    display "Format 3 valid"
end check
```

### ID Number Validation

```wfl
// Employee ID: 3 letters followed by 4 digits
create pattern employee_id:
    exactly 3 letter
    exactly 4 digit
end pattern

// Social Security Number format
create pattern ssn_format:
    exactly 3 digit
    "-"
    exactly 2 digit
    "-"
    exactly 4 digit
end pattern

store emp_id as "ABC1234"
store ssn as "123-45-6789"

check if emp_id matches employee_id:
    display "Valid employee ID"
end check

check if ssn matches ssn_format:
    display "Valid SSN format"
end check
```

## Text Processing

### Data Extraction with Captures

```wfl
// Extract name components
create pattern full_name:
    capture {one or more letter} as first_name
    " "
    capture {one or more letter} as last_name
end pattern

store name as "John Smith"
store result as find full_name in name
check if result is not nothing:
    display "First: " with result.captures.first_name
    display "Last: " with result.captures.last_name
end check
```

### URL Component Extraction

```wfl
// Extract domain from URL
create pattern simple_url:
    "http" then optional "s" then "://"
    capture {one or more letter or digit or "." or "-"} as domain
    optional "/"
    zero or more any character
end pattern

store url as "https://example.com/path"
store url_result as find simple_url in url
check if url_result is not nothing:
    display "Domain: " with url_result.captures.domain
end check
```

### File Extension Detection

```wfl
// Match common file extensions
create pattern image_files:
    ".jpg" or ".jpeg" or ".png" or ".gif" or ".bmp"
end pattern

create pattern document_files:
    ".pdf" or ".doc" or ".docx" or ".txt" or ".md"
end pattern

store filename as "document.pdf"
check if filename matches document_files:
    display "Document file detected"
end check
```

## Log Parsing

### Simple Log Entry Parser

```wfl
// Parse basic log format: YYYY-MM-DD HH:MM:SS LEVEL Message
create pattern log_entry:
    capture {exactly 4 digit} as year
    "-"
    capture {exactly 2 digit} as month
    "-"
    capture {exactly 2 digit} as day
    " "
    capture {exactly 2 digit} as hour
    ":"
    capture {exactly 2 digit} as minute
    ":"
    capture {exactly 2 digit} as second
    " "
    capture {one or more letter} as level
    " "
    capture {one or more any character} as message
end pattern

store log_line as "2025-09-20 14:30:15 INFO Application started"
store log_result as find log_entry in log_line
check if log_result is not nothing:
    display "Date: " with log_result.captures.year with "-" with log_result.captures.month with "-" with log_result.captures.day
    display "Time: " with log_result.captures.hour with ":" with log_result.captures.minute with ":" with log_result.captures.second
    display "Level: " with log_result.captures.level
    display "Message: " with log_result.captures.message
end check
```

### Error Code Extraction

```wfl
// Extract error codes from logs
create pattern error_code:
    "ERROR"
    optional " "
    capture {exactly 3 digit} as code
    ":"
    capture {one or more any character} as description
end pattern

store error_log as "ERROR 404: File not found"
store error_result as find error_code in error_log
check if error_result is not nothing:
    display "Error Code: " with error_result.captures.code
    display "Description: " with error_result.captures.description
end check
```

## Configuration Files

### Key-Value Pair Parsing

```wfl
// Parse simple config: key=value
create pattern config_line:
    capture {one or more letter or digit or "_"} as key
    "="
    capture {one or more any character} as value
end pattern

store config as "database_host=localhost"
store config_result as find config_line in config
check if config_result is not nothing:
    display "Key: " with config_result.captures.key
    display "Value: " with config_result.captures.value
end check
```

### Version Number Parsing

```wfl
// Parse semantic version: major.minor.patch
create pattern version_number:
    capture {one or more digit} as major
    "."
    capture {one or more digit} as minor
    "."
    capture {one or more digit} as patch
end pattern

store version as "1.2.3"
store version_result as find version_number in version
check if version_result is not nothing:
    display "Major: " with version_result.captures.major
    display "Minor: " with version_result.captures.minor
    display "Patch: " with version_result.captures.patch
end check
```

## Web Development

### HTML Tag Matching

```wfl
// Match simple HTML tags (without attributes)
create pattern html_tag:
    "<"
    capture {one or more letter} as tag_name
    ">"
end pattern

store html as "<div>content</div>"
store tag_result as find html_tag in html
check if tag_result is not nothing:
    display "Found tag: " with tag_result.captures.tag_name
end check
```

### CSS Class Extraction

```wfl
// Extract CSS class names
create pattern css_class:
    "class=\""
    capture {one or more letter or digit or "-" or "_" or " "} as class_names
    "\""
end pattern

store html_element as "class=\"btn btn-primary active\""
store class_result as find css_class in html_element
check if class_result is not nothing:
    display "Classes: " with class_result.captures.class_names
end check
```

## Performance Tips

### Efficient Pattern Design

```wfl
// ✅ Good: Specific patterns are faster
create pattern specific_date:
    exactly 4 digit then "-" then exactly 2 digit then "-" then exactly 2 digit
end pattern

// ❌ Avoid: Overly general patterns
// create pattern too_general:
//     one or more any character
// end pattern

// ✅ Good: Use alternatives efficiently
create pattern file_types:
    ".txt" or ".md" or ".wfl"
end pattern

// ✅ Good: Anchor patterns when possible
create pattern starts_with_http:
    at start of text then "http"
end pattern
```

### Testing Patterns

```wfl
// Always test edge cases
create pattern number_range:
    1 to 3 digit
end pattern

// Test with different inputs
store test1 as "5"      // Should match (1 digit)
store test2 as "42"     // Should match (2 digits)  
store test3 as "123"    // Should match (3 digits)
store test4 as "1234"   // Should match (starts with 3 digits)
store test5 as ""       // Should not match (0 digits)

check if test1 matches number_range:
    display "✓ Single digit works"
end check

check if test2 matches number_range:
    display "✓ Two digits work"
end check

check if test3 matches number_range:
    display "✓ Three digits work"
end check

check if test4 matches number_range:
    display "✓ Four digits work (matches first 3)"
end check

check if test5 matches number_range:
    display "This shouldn't print"
otherwise:
    display "✓ Empty string correctly rejected"
end check
```

## Best Practices Summary

1. **Start Simple**: Begin with basic patterns and add complexity gradually
2. **Test Thoroughly**: Use multiple test cases including edge cases
3. **Use Specific Patterns**: Avoid overly general patterns that match too much
4. **Leverage Captures**: Extract data you need with named capture groups
5. **Handle Failures**: Always check if pattern matching results are not nothing
6. **Document Intent**: Use clear pattern names that explain what they match

## See Also

- [Pattern Matching Reference](../language-reference/wfl-patterns.md)
- [Working Pattern Test Program](../../TestPrograms/patterns_working_comprehensive.wfl)
- [Implementation Analysis](../dev-notes/pattern-implementation-analysis.md)
