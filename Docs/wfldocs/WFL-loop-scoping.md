# Loop Variable Scoping in WFL

## Overview

As of version 25.8.24, WFL implements iteration-scoped loop variables. This means that loop variables are created fresh for each iteration rather than being reused across iterations.

## Key Behavior

### Loop Variable Scoping

In loops (`count from`, `for each`), the loop variable is automatically scoped to each iteration:

```wfl
count from 1 to 5 as i
    display i
end
// Variable 'i' is not accessible here
```

### Each Iteration Gets a Fresh Variable

The loop variable is created anew for each iteration:

```wfl
count from 1 to 3 as x
    store x as x * 2  // This would fail - can't redefine loop variable
    display x
end
```

## Variable Redefinition Rules

WFL enforces strict variable redefinition rules:

1. **Initial Definition**: Use `store` to define a variable for the first time
2. **Subsequent Changes**: Use `change` to modify an existing variable
3. **Loop Variables**: Are automatically managed and cannot be redefined within the loop body

### Examples

```wfl
// Correct usage
store name as "Alice"
change name to "Bob"  // Must use 'change' for reassignment

// Incorrect - will produce an error
store age as 25
store age as 26  // Error: Variable 'age' is already defined

// Loop variables
count from 1 to 10 as i
    // Variable 'i' is read-only within the loop
    display i
    // store i as 5  // This would fail
end
```

## Implementation Details

The interpreter creates a new scope for each loop iteration and automatically defines the loop variable in that scope. This ensures:

1. **Isolation**: Each iteration's variables don't interfere with others
2. **Safety**: Prevents accidental variable shadowing
3. **Clarity**: Makes the code's intent more explicit

## Backward Compatibility

This change maintains backward compatibility with existing WFL programs. Programs that previously worked will continue to work, as the scoping is more restrictive but doesn't break valid code patterns.

## Best Practices

1. Don't try to modify loop variables within the loop body
2. Use descriptive names for loop variables
3. If you need a mutable counter inside a loop, create a separate variable:

```wfl
store total as 0
count from 1 to 10 as i
    change total to total + i
end
display "Sum: " with total
```

## Related Documentation

- [WFL Variables](wfl-variables.md)
- [Control Flow](wfl-control-flow.md)
- [WFL Specification](wfl-spec.md)