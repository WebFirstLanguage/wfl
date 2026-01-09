# Core Module

The Core module provides essential functions for output, type introspection, and null checking. These are the most fundamental functions in WFL.

## Functions

### display

**Purpose:** Output text or values to the console with a newline.

**Signature:**
```wfl
display <value>
```

**Parameters:**
- `value` (Any): The value to display

**Returns:** None (outputs to console)

**Example:**
```wfl
display "Hello, World!"
// Output: Hello, World!

store name as "Alice"
display "Name: " with name
// Output: Name: Alice

store age as 25
display "Age: " with age
// Output: Age: 25

display 42
// Output: 42
```

**Use Cases:**
- Debugging - Print variable values
- User output - Show results
- Logging - Display status messages
- Testing - Verify program behavior

**Notes:**
- Automatically adds newline after output
- Can display any type (Text, Number, Boolean, List, etc.)
- Converts values to text representation automatically

---

### typeof

**Purpose:** Get the type name of a value as a string.

**Signature:**
```wfl
typeof of <value>
```

**Aliases:** `type_of`

**Parameters:**
- `value` (Any): The value to check

**Returns:** Text - The type name

**Type Names:**
- `"Text"` - String values
- `"Number"` - Numeric values (int or float)
- `"Boolean"` - True/false values
- `"Null"` - Nothing/null values
- `"List"` - List/array values
- `"Container"` - Container instances
- `"Pattern"` - Pattern values
- `"Date"`, `"Time"`, `"DateTime"` - Temporal values

**Example:**
```wfl
store name as "Alice"
store age as 25
store active as yes
store items as [1, 2, 3]
store value as nothing

display typeof of name           // Output: Text
display typeof of age             // Output: Number
display typeof of active          // Output: Boolean
display typeof of items           // Output: List
display typeof of value           // Output: Null
```

**Use Cases:**
- **Type checking:** Verify variable types
- **Debugging:** Understand what type you're working with
- **Conditional logic:** Different behavior for different types
- **Validation:** Ensure correct types before operations

**Example: Type-Based Behavior**
```wfl
define action called process value with parameters val:
    store val_type as typeof of val

    check if val_type is "Number":
        display "Processing number: " with val times 2
    check if val_type is "Text":
        display "Processing text: " with touppercase of val
    check if val_type is "List":
        display "Processing list of length: " with length of val
    otherwise:
        display "Unknown type: " with val_type
    end check
end action

call process value with 42
call process value with "hello"
call process value with [1, 2, 3]
```

**Output:**
```
Processing number: 84
Processing text: HELLO
Processing list of length: 3
```

---

### isnothing

**Purpose:** Check if a value is nothing (null/undefined).

**Signature:**
```wfl
isnothing of <value>
```

**Aliases:** `is_nothing`

**Parameters:**
- `value` (Any): The value to check

**Returns:** Boolean - `yes` if nothing, `no` otherwise

**Example:**
```wfl
store value1 as nothing
store value2 as 42
store value3 as "hello"

display isnothing of value1       // Output: yes
display isnothing of value2       // Output: no
display isnothing of value3       // Output: no
```

**Use Cases:**
- **Null checking:** Verify values exist before using
- **Optional returns:** Check if function returned nothing
- **Error prevention:** Avoid operations on null values
- **Validation:** Ensure required values are present

**Example: Safe Value Access**
```wfl
define action called safe display with parameters value:
    check if isnothing of value:
        display "Value is nothing - cannot display"
    otherwise:
        display "Value: " with value
    end check
end action

call safe display with nothing
call safe display with 42
```

**Output:**
```
Value is nothing - cannot display
Value: 42
```

**Example: Optional Return Handling**
```wfl
define action called find user with parameters id:
    // Simulate database lookup
    check if id is equal to 1:
        return "Alice"
    otherwise:
        return nothing  // User not found
    end check
end action

store user as find user with 1
check if isnothing of user:
    display "User not found"
otherwise:
    display "Found user: " with user
end check
```

---

## Complete Example

Using all core module functions together:

```wfl
display "=== Core Module Demo ==="
display ""

// Display different types
display "Text output"
display 42
display yes
display [1, 2, 3]
display ""

// Type checking
store name as "Alice"
store age as 25
store active as yes
store result as nothing

display "Type of name: " with typeof of name
display "Type of age: " with typeof of age
display "Type of active: " with typeof of active
display "Type of result: " with typeof of result
display ""

// Null checking
display "Is name nothing? " with isnothing of name
display "Is result nothing? " with isnothing of result
display ""

// Conditional based on type
define action called describe value with parameters val:
    store val_type as typeof of val

    check if isnothing of val:
        display "Value is nothing"
    otherwise:
        check if val_type is "Number":
            display "Number: " with val
        check if val_type is "Text":
            display "Text: " with val
        check if val_type is "Boolean":
            display "Boolean: " with val
        otherwise:
            display "Other type: " with val_type
        end check
    end check
end action

call describe value with 42
call describe value with "hello"
call describe value with yes
call describe value with nothing

display ""
display "=== Demo Complete ==="
```

**Output:**
```
=== Core Module Demo ===

Text output
42
yes
[1, 2, 3]

Type of name: Text
Type of age: Number
Type of active: Boolean
Type of result: Null

Is name nothing? no
Is result nothing? yes

Number: 42
Text: hello
Boolean: yes
Value is nothing

=== Demo Complete ===
```

## Best Practices

✅ **Use display for output:** Don't try to print to console other ways

✅ **Check types when uncertain:** Use `typeof` for debugging

✅ **Check for nothing before using:** Prevent null errors

✅ **Use typeof for conditional logic:** Handle different types differently

✅ **Display for debugging:** Temporary displays help understand code

❌ **Don't assume types:** Check with `typeof` if unsure

❌ **Don't use nothing values:** Always check with `isnothing` first

## What You've Learned

In this module, you learned:

✅ **display** - Output values to console
✅ **typeof** - Get type information
✅ **isnothing** - Check for null values
✅ **Type names** - Text, Number, Boolean, Null, List, etc.
✅ **Use cases** - Debugging, validation, conditional logic
✅ **Best practices** - Safe value handling

## Next Steps

Explore more standard library modules:

**[Math Module →](math-module.md)**
Mathematical operations for calculations.

**[Text Module →](text-module.md)**
String manipulation functions.

**[List Module →](list-module.md)**
Working with collections.

Or return to:
**[Standard Library Index →](index.md)**

---

**Previous:** [← Overview](overview.md) | **Next:** [Math Module →](math-module.md)
