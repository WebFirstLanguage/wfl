# Typechecker Module

The Typechecker module provides type validation and checking utilities. Use these for runtime type verification and validation.

## Overview

WFL has a static type system that catches most type errors at compile time. The Typechecker module provides runtime utilities for additional type checking.

## Core Type Checking

The primary type checking function is in the Core module:

### typeof

**See:** [Core Module: typeof](core-module.md#typeof)

```wfl
store value as 42
store value_type as typeof of value

check if value_type is "Number":
    display "It's a number"
end check
```

### isnothing

**See:** [Core Module: isnothing](core-module.md#isnothing)

```wfl
store value as nothing
check if isnothing of value:
    display "Value is nothing"
end check
```

## Type Validation Patterns

### Validate Number

```wfl
define action called is_number with parameters value:
    store value_type as typeof of value
    return value_type is "Number"
end action

check if is_number of 42:
    display "Valid number"
end check
```

### Validate Text

```wfl
define action called is_text with parameters value:
    store value_type as typeof of value
    return value_type is "Text"
end action

check if is_text of "hello":
    display "Valid text"
end check
```

### Validate List

```wfl
define action called is_list with parameters value:
    store value_type as typeof of value
    return value_type is "List"
end action

check if is_list of [1, 2, 3]:
    display "Valid list"
end check
```

### Validate Boolean

```wfl
define action called is_boolean with parameters value:
    store value_type as typeof of value
    return value_type is "Boolean"
end action

check if is_boolean of yes:
    display "Valid boolean"
end check
```

## Type Guards

Use type checking to guard operations:

```wfl
define action called is_number with parameters value:
    return typeof of value is "Number"
end action

define action called safe_divide with parameters a and b:
    // Check types
    check if is_number of a:
        check if is_number of b:
            // Check for zero
            check if b is not equal to 0:
                return a divided by b
            otherwise:
                display "Error: Division by zero"
                return nothing
            end check
        otherwise:
            display "Error: Second argument must be number"
            return nothing
        end check
    otherwise:
        display "Error: First argument must be number"
        return nothing
    end check
end action

store result as safe_divide of 10 and 2
display result                  // 5

store bad as safe_divide of "ten" and 2
display bad                     // nothing (with error message)
```

## Runtime Type Checking

### Dynamic Type Handling

```wfl
define action called process_value with parameters val:
    store val_type as typeof of val

    check if val_type is "Number":
        display "Number: " with val times 2
    otherwise:
        check if val_type is "Text":
            display "Text: " with touppercase of val
        otherwise:
            check if val_type is "List":
                display "List of length: " with length of val
            otherwise:
                check if val_type is "Boolean":
                    check if val is yes:
                        display "Boolean: True"
                    otherwise:
                        display "Boolean: False"
                    end check
                otherwise:
                    check if val_type is "Null":
                        display "Value is nothing"
                    otherwise:
                        display "Unknown type: " with val_type
                    end check
                end check
            end check
        end check
    end check
end action

call process_value with 42
call process_value with "hello"
call process_value with [1, 2, 3]
call process_value with yes
call process_value with nothing
```

**Output:**
```
Number: 84
Text: HELLO
List of length: 3
Boolean: True
Value is nothing
```

## Type Assertions

### Assert Type

```wfl
define action called is_number with parameters value:
    return typeof of value is "Number"
end action

define action called assert_number with parameters value:
    check if is_number of value:
        return value
    otherwise:
        display "Type assertion failed: expected Number, got " with typeof of value
        return nothing
    end check
end action

store validated as assert_number of 42
// Returns: 42

store invalid as assert_number of "hello"
// Returns: nothing (with error message)
```

## Complete Example

```wfl
display "=== Typechecker Module Demo ==="
display ""

// Define type validators
define action called validate_input with parameters value and expected_type:
    store actual_type as typeof of value

    check if actual_type is equal to expected_type:
        display "✓ Type check passed: " with expected_type
        return yes
    otherwise:
        display "✗ Type check failed: expected " with expected_type with ", got " with actual_type
        return no
    end check
end action

// Test validations
call validate_input with 42 and "Number"
call validate_input with "hello" and "Text"
call validate_input with yes and "Boolean"
call validate_input with [1, 2, 3] and "List"
call validate_input with nothing and "Null"
display ""

// Type mismatch
call validate_input with "hello" and "Number"
call validate_input with 42 and "Text"
display ""

display "=== Demo Complete ==="
```

**Output:**
```
=== Typechecker Module Demo ===

✓ Type check passed: Number
✓ Type check passed: Text
✓ Type check passed: Boolean
✓ Type check passed: List
✓ Type check passed: Null

✗ Type check failed: expected Number, got Text
✗ Type check failed: expected Text, got Number

=== Demo Complete ===
```

## Best Practices

✅ **Use static type checking when possible:** Let compiler catch errors

✅ **Use runtime checks for external data:** Validate user input, file content

✅ **Create type validators:** Reusable validation functions

✅ **Provide clear error messages:** Tell users what's wrong

✅ **Return nothing on type errors:** Indicates invalid value

❌ **Don't over-validate:** Trust WFL's type system for internal code

❌ **Don't ignore type errors:** Handle them appropriately

❌ **Don't assume types:** Check when uncertain

## When to Use Runtime Type Checking

### Use Runtime Checks For:

✅ **External data:** User input, API responses, file content
✅ **Dynamic operations:** When type isn't known at compile time
✅ **Defensive programming:** Extra safety for critical operations
✅ **Debugging:** Understanding type issues

### Don't Need Runtime Checks For:

❌ **Literal values:** `store x as 42` (compiler knows it's Number)
❌ **Function returns:** If function signature specifies return type
❌ **Type-safe operations:** WFL's compiler handles these

## What You've Learned

In this module, you learned:

✅ **typeof** - Get runtime type information (from Core module)
✅ **isnothing** - Check for null values (from Core module)
✅ **Type validation patterns** - Create custom validators
✅ **Type guards** - Protect operations with type checks
✅ **Dynamic type handling** - Process different types differently
✅ **When to use runtime checks** - External data, dynamic operations
✅ **Best practices** - Clear errors, defensive programming

---

## Standard Library Complete!

You've completed all standard library modules! You now understand:

✅ **Core** - display, typeof, isnothing
✅ **Math** - abs, round, floor, ceil, clamp
✅ **Text** - Case conversion, length, substring, trim
✅ **List** - length, push, pop, indexof
✅ **Filesystem** - File/directory operations, path utilities
✅ **Time** - Dates, times, formatting, calculations
✅ **Random** - Secure random number generation
✅ **Crypto** - WFLHASH hashing and MACs
✅ **Pattern** - Pattern matching utilities
✅ **Typechecker** - Runtime type validation

**Total:** 181+ functions across 11 modules!

## What's Next?

### Write Better Code

**[Best Practices →](../06-best-practices/index.md)**
Code style, security, performance, testing strategies.

### Practical Examples

**[Guides →](../guides/)**
- WFL by Example
- Cookbook
- Migration guides

### Quick Reference

**[Reference →](../reference/)**
- Language specification
- Syntax reference
- Complete function listing

### Build Something!

You now have all the tools to build real WFL applications. Start coding!

---

**Previous:** [← Pattern Module](pattern-module.md) | **Next:** [Best Practices →](../06-best-practices/index.md)
