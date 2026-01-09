# Typechecker Module

The Typechecker module provides type validation and checking utilities. Use these for runtime type verification and validation.

## Overview

WFL has a static type system that catches most type errors at compile time. The Typechecker module provides runtime utilities for additional type checking.

## Core Type Checking

The primary type checking function is in the Core module:

### typeof

**See:** [Core Module: typeof](core-module.md#typeof)

```wfl
store value_type as typeof of value

check if value_type is "Number":
    display "It's a number"
end check
```

### isnothing

**See:** [Core Module: isnothing](core-module.md#isnothing)

```wfl
check if isnothing of value:
    display "Value is nothing"
end check
```

## Type Validation Patterns

### Validate Number

```wfl
define action called is number with parameters value:
    store value_type as typeof of value
    return value_type is "Number"
end action

check if is number with 42:
    display "Valid number"
end check
```

### Validate Text

```wfl
define action called is text with parameters value:
    store value_type as typeof of value
    return value_type is "Text"
end action

check if is text with "hello":
    display "Valid text"
end check
```

### Validate List

```wfl
define action called is list with parameters value:
    store value_type as typeof of value
    return value_type is "List"
end action

check if is list with [1, 2, 3]:
    display "Valid list"
end check
```

### Validate Boolean

```wfl
define action called is boolean with parameters value:
    store value_type as typeof of value
    return value_type is "Boolean"
end action

check if is boolean with yes:
    display "Valid boolean"
end check
```

## Type Guards

Use type checking to guard operations:

```wfl
define action called safe divide with parameters a and b:
    // Check types
    check if is number with a:
        check if is number with b:
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

store result as safe divide with 10 and 2
display result                  // 5

store bad as safe divide with "ten" and 2
display bad                     // nothing (with error message)
```

## Runtime Type Checking

### Dynamic Type Handling

```wfl
define action called process value with parameters val:
    store val_type as typeof of val

    check if val_type is "Number":
        display "Number: " with val times 2
    check if val_type is "Text":
        display "Text: " with touppercase of val
    check if val_type is "List":
        display "List of length: " with length of val
    check if val_type is "Boolean":
        check if val is yes:
            display "Boolean: True"
        otherwise:
            display "Boolean: False"
        end check
    check if val_type is "Null":
        display "Value is nothing"
    otherwise:
        display "Unknown type: " with val_type
    end check
end action

call process value with 42
call process value with "hello"
call process value with [1, 2, 3]
call process value with yes
call process value with nothing
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
define action called assert number with parameters value:
    check if is number with value:
        return value
    otherwise:
        display "Type assertion failed: expected Number, got " with typeof of value
        return nothing
    end check
end action

store validated as assert number with 42
// Returns: 42

store invalid as assert number with "hello"
// Returns: nothing (with error message)
```

## Complete Example

```wfl
display "=== Typechecker Module Demo ==="
display ""

// Define type validators
define action called validate input with parameters value and expected_type:
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
call validate input with 42 and "Number"
call validate input with "hello" and "Text"
call validate input with yes and "Boolean"
call validate input with [1, 2, 3] and "List"
call validate input with nothing and "Null"
display ""

// Type mismatch
call validate input with "hello" and "Number"
call validate input with 42 and "Text"
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
