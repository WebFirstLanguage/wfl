# WFL Core Module API Reference

## Overview

The Core module provides essential utility functions that form the foundation of WFL programming. These functions handle basic operations like output, type checking, and null value testing.

## Functions

### `print(value, ...)`

Outputs values to the console with automatic formatting and spacing.

**Parameters:**
- `value` (Any): One or more values of any type to output
- Accepts multiple arguments separated by spaces or commas

**Returns:** Nothing

**Examples:**

```wfl
// Basic printing
print "Hello, World!"
print 42
print yes

// Multiple values (automatically space-separated)
print "The answer is" 42
print "User:" "Alice" "Age:" 25

// Variables
store name as "Bob"
store age as 30
print "Name:" name "Age:" age
```

**Output:**
```
Hello, World!
42
yes
The answer is 42
User: Alice Age: 25
Name: Bob Age: 30
```

**Natural Language Variants:**
```wfl
// All equivalent ways to print
print "Hello"
display "Hello"
show "Hello"
output "Hello"
```

---

### `typeof(value)`

Returns a text string describing the type of the given value.

**Parameters:**
- `value` (Any): The value to check the type of

**Returns:** Text (one of: "Number", "Text", "Boolean", "List", "Date", "Time", "DateTime", "Null")

**Examples:**

```wfl
// Basic type checking
store number_type as typeof of 42
display "Type of 42: " with number_type  // "Number"

store text_type as typeof of "Hello"
display "Type of Hello: " with text_type  // "Text"

store bool_type as typeof of yes
display "Type of yes: " with bool_type  // "Boolean"

store list_type as typeof of [1, 2, 3]
display "Type of list: " with list_type  // "List"

store null_type as typeof of nothing
display "Type of nothing: " with null_type  // "Null"
```

**Natural Language Variants:**
```wfl
// All equivalent ways to get type
store type1 as typeof of value
store type2 as type of value
store type3 as what type is value
```

**Practical Use Cases:**

```wfl
// Type validation in functions
action validate_input with value:
    store value_type as typeof of value
    check if value_type is "Number":
        display "Valid number input"
    otherwise:
        display "Error: Expected number, got " with value_type
    end
end

// Dynamic behavior based on type
action process_value with input:
    store input_type as typeof of input
    check if input_type is "List":
        display "Processing list with " with length of input with " items"
    check if input_type is "Text":
        display "Processing text with " with length of input with " characters"
    check if input_type is "Number":
        display "Processing number: " with input
    end
end
```

---

### `isnothing(value)`

Checks if a value is "nothing" (WFL's equivalent of null/none/undefined).

**Parameters:**
- `value` (Any): The value to test

**Returns:** Boolean (yes if the value is nothing, no otherwise)

**Examples:**

```wfl
// Basic null checking
store empty_value as nothing
store some_value as 42

check if isnothing of empty_value:
    display "empty_value is nothing"  // This will execute
end

check if isnothing of some_value:
    display "some_value is nothing"   // This will NOT execute
end

// Checking uninitialized variables
store result as get_user_input  // Might return nothing
check if isnothing of result:
    display "No input provided"
otherwise:
    display "User entered: " with result
end
```

**Natural Language Variants:**
```wfl
// All equivalent ways to check for nothing
check if isnothing of value
check if value is nothing
check if value is null
check if value is empty
```

**Practical Use Cases:**

```wfl
// Safe value processing
action safe_divide with numerator and denominator:
    check if isnothing of denominator:
        display "Error: denominator cannot be nothing"
        return nothing
    end
    
    check if denominator is 0:
        display "Error: division by zero"
        return nothing
    end
    
    return numerator / denominator
end

// Optional parameter handling
action greet_user with name and title:
    check if isnothing of title:
        display "Hello, " with name
    otherwise:
        display "Hello, " with title with " " with name
    end
end

// Default value assignment
store user_preference as get_setting of "theme"
check if isnothing of user_preference:
    store user_preference as "default"
end
```

## Error Handling

All core module functions are designed to be robust:

- **`print`**: Handles any data type gracefully, converting to string representation
- **`typeof`**: Never fails, always returns a valid type string
- **`isnothing`**: Never fails, safely handles any input including nothing itself

## Integration with WFL Features

### With Conditional Statements

```wfl
store user_input as get_input
check if isnothing of user_input:
    print "No input provided"
otherwise if typeof of user_input is "Number":
    print "You entered a number: " with user_input
otherwise if typeof of user_input is "Text":
    print "You entered text: " with user_input
otherwise:
    print "You entered a " with typeof of user_input
end
```

### With Loops

```wfl
store items as [1, "hello", nothing, yes]
count item in items:
    check if isnothing of item:
        print "Found nothing value"
    otherwise:
        print "Item type: " with typeof of item with ", value: " with item
    end
end
```

### With Actions (Functions)

```wfl
action debug_value with value and label:
    print label with ":"
    print "  Type: " with typeof of value
    print "  Is nothing: " with isnothing of value
    print "  Value: " with value
end

// Usage
debug_value of 42 and "My Number"
debug_value of nothing and "Empty Value"
```

## Best Practices

1. **Use `print` for debugging and user output**: It's the most versatile output function
2. **Check types before operations**: Use `typeof` to ensure safe operations
3. **Always handle nothing values**: Use `isnothing` to prevent runtime errors
4. **Combine functions for robust code**: Use all three together for comprehensive validation

```wfl
action safe_operation with value:
    check if isnothing of value:
        print "Error: Value cannot be nothing"
        return nothing
    end
    
    store value_type as typeof of value
    check if value_type is "Number":
        return value * 2
    otherwise:
        print "Error: Expected number, got " with value_type
        return nothing
    end
end
```

## See Also

- [Math Module](math-module.md) - Numeric operations
- [Text Module](text-module.md) - String manipulation
- [List Module](list-module.md) - Collection operations
- [WFL Language Reference](../language-reference/wfl-spec.md) - Complete language specification