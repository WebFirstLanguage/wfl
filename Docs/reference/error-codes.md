# Error Codes and Messages

Understanding WFL error messages and how to fix them.

## Error Categories

WFL errors fall into 4 categories:

1. **Parse Errors** - Syntax problems
2. **Semantic Errors** - Invalid program structure
3. **Type Errors** - Type mismatches
4. **Runtime Errors** - Execution failures

## Parse Errors

### "Expected identifier, found Keyword..."

**Cause:** Using reserved keyword as variable name.

**Example:**
```wfl
store is as 10  // ERROR: 'is' is reserved
```

**Fix:**
```wfl
store is_valid as 10
```

**[Reserved keywords →](../03-language-basics/variables-and-types.md#reserved-keywords)**

---

### "Expected 'end' after if block"

**Cause:** Missing `end check`.

**Example:**
```wfl
check if condition:
    code
// Missing end check!
```

**Fix:**
```wfl
check if condition:
    code
end check
```

---

### "Unexpected token in expression"

**Cause:** Invalid syntax.

**Example:**
```wfl
display age plus name  // If name is reserved keyword
```

**Fix:** Check for reserved keywords, verify syntax.

---

## Semantic Errors

### "Variable 'x' is not defined"

**Cause:** Using variable before declaring.

**Example:**
```wfl
display user_name  // Not defined yet
```

**Fix:**
```wfl
store user_name as "Alice"
display user_name
```

---

### "Action 'foo' is not defined"

**Cause:** Calling undefined action.

**Example:**
```wfl
call greet with "World"  // greet not defined
```

**Fix:**
```wfl
define action called greet with parameters name:
    display "Hello, " with name
end action

call greet with "World"
```

---

## Type Errors

### "Type mismatch: cannot add Number and Text"

**Cause:** Incompatible types in operation.

**Example:**
```wfl
store age as 25
store name as "Alice"
display age plus name  // ERROR
```

**Fix:**
```wfl
display "Name: " with name with ", Age: " with age
```

---

### "Expected Number, got Text"

**Cause:** Function expects number, got text.

**Example:**
```wfl
store result as abs of "hello"  // ERROR
```

**Fix:**
```wfl
store result as abs of -5  // Numbers only
```

---

### "Could not infer type for variable"

**Cause:** Type checker can't determine type (usually in actions with parameters).

**Example:**
```wfl
define action called process with parameters x:
    store result as x plus 1  // Type of x unknown
    return result
end action
```

**Fix:** This is typically a warning, not an error. Code still works.

---

## Runtime Errors

### "Division by zero"

**Cause:** Dividing by zero.

**Example:**
```wfl
store result as 10 divided by 0
```

**Fix:**
```wfl
check if divisor is not equal to 0:
    store result as 10 divided by divisor
otherwise:
    display "Cannot divide by zero"
end check
```

---

### "Index out of bounds"

**Cause:** Accessing list index that doesn't exist.

**Example:**
```wfl
store items as [1, 2, 3]
store item as items[10]  // Only 3 items!
```

**Fix:**
```wfl
check if 10 is less than length of items:
    store item as items[10]
otherwise:
    display "Index too large"
end check
```

---

### "Cannot pop from empty list"

**Cause:** Popping when list has no items.

**Example:**
```wfl
create list empty
end list
store item as pop from empty  // ERROR
```

**Fix:**
```wfl
check if length of list is greater than 0:
    store item as pop from list
otherwise:
    display "List is empty"
end check
```

---

### "File not found"

**Cause:** Opening non-existent file.

**Example:**
```wfl
open file at "missing.txt" for reading as myfile
```

**Fix:**
```wfl
try:
    open file at "missing.txt" for reading as myfile
    // ...
catch:
    display "File not found"
end try
```

---

### "Permission denied"

**Cause:** Insufficient permissions.

**Fix:** Check file permissions, run with appropriate privileges.

---

## Error Message Format

WFL error messages include:

```
[ERROR_TYPE]: Description
   ┌─ file.wfl:line:column
   │
line │ code
   │   ^ Error occurred here
```

**Components:**
- **Error type** - Parse, semantic, type, runtime
- **Description** - What went wrong
- **Location** - Exact line and column
- **Context** - Code snippet showing error
- **Pointer** - Visual indicator

---

## Getting Help

**For any error:**

1. **Read the error message** - WFL provides helpful descriptions
2. **Check line and column** - Exact location shown
3. **Look for typos** - Common cause of errors
4. **Verify syntax** - Use [Syntax Reference](syntax-reference.md)
5. **Check examples** - [WFL by Example](../guides/wfl-by-example.md)

**Still stuck?**
- [Troubleshooting Guide](../guides/troubleshooting.md)
- [GitHub Issues](https://github.com/WebFirstLanguage/wfl/issues)
- [GitHub Discussions](https://github.com/WebFirstLanguage/wfl/discussions)

---

**Previous:** [← Built-in Functions](builtin-functions-reference.md) | **Next:** [Development →](../development/)
