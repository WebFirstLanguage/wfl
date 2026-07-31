# WFL Container Property Mutation Issue

## Summary

Container properties **do not persist** when modified inside container actions. The `store` statement creates a local variable instead of mutating the container's property.

## Reproduction

```wfl
create container Counter:
    property value: Number

    action increment:
        display "Before: " with value    // Shows: 0
        store value as value plus 1
        display "After: " with value     // Shows: 1
    end
end

create new Counter as my_counter:
    value is 0
end

my_counter.increment()
display my_counter.value  // Expected: 1, Actual: 0 ❌
```

## Test Results

Test file: `TestPrograms/test_container_property_mutation.wfl`

```
Calling increment() first time...
  Before: value = 0
  After: value = 1        ← Changed inside action
External view of value: 0  ← NOT persisted! ❌

Calling increment() second time...
  Before: value = 0
  After: value = 1
External view of value: 0  ← Still 0!

Expected after 3 calls: value = 3
Actual: value = 0
```

## Root Cause Analysis

The `store` statement inside a container action appears to:
1. **Read** the property value correctly from the container
2. **Modify** it locally within the action scope
3. **Fail** to write back to the container's property

This suggests the issue is in how container action scopes handle property assignment.

## Impact on WFL MVC Framework

This bug blocks several critical framework features:

### ❌ Blocked Features
- **Stateful middleware**: Cannot track request_count, error_count
- **Session management**: Cannot modify session data
- **Router state**: Cannot track routes_count
- **Rate limiting**: Cannot increment request counters
- **Any stateful operations** in containers

### ✅ Working Features
- Container creation and initialization
- Property reading in actions
- Actions that don't modify properties
- Passing containers between actions
- Container methods that return values

## Workarounds

### Option 1: External State (Current Framework Approach)
```wfl
// Keep state outside container
store global_request_count as 0

create container Logger:
    action log_request:
        // Read global state
        display "Request #" with global_request_count

        // Modify global state (works)
        store global_request_count as global_request_count plus 1
    end
end
```

### Option 2: Return New State
```wfl
create container Counter:
    property value: Number

    action increment_and_return:
        return value plus 1
    end
end

// Caller must reassign
store my_counter.value as my_counter.increment_and_return()
```

**Problem**: Cannot reassign container properties from outside either!

### Option 3: Use Lists/Objects Instead of Containers
```wfl
// Use WFL objects for mutable state
store counter as create list
push with counter and 0  // counter[0] is the value

// Mutation works in actions with lists
```

## Expected Behavior

The `store` statement inside a container action should mutate the container's property:

```wfl
create container Counter:
    property value: Number

    action increment:
        store value as value plus 1  // Should mutate container property
    end
end

my_counter.increment()
display my_counter.value  // Should show: 1
```

## Suggested Fix Location

Likely in the interpreter where container action scopes are created:
- `src/interpreter/mod.rs` - Container instance handling
- `src/interpreter/environment.rs` - Scope and variable resolution
- Look for where container actions create child scopes

The fix should ensure that `store` statements for properties write back to the container instance's property map.

## Priority

**HIGH** - This blocks stateful operations in containers, which are essential for:
- Web frameworks (middleware, sessions, routing)
- Game state management
- Any OOP pattern requiring mutable state
- Counters, accumulators, caches

## Test Case for Fix Validation

When fixed, this test should pass:

```wfl
create container Counter:
    property value: Number
    action increment:
        store value as value plus 1
    end
end

create new Counter as c:
    value is 0
end

c.increment()
c.increment()
c.increment()

// Should print: 3
display c.value
```

## Related Issues

This issue was discovered while building the WFL MVC framework (`wfl framework/` directory) as part of Sprint 2-4 implementation. It affects:
- `core/router.wfl` - routes_count doesn't increment
- `middleware/logging.wfl` - request_count doesn't increment
- `middleware/error_handler.wfl` - error_count doesn't increment
- Any future stateful framework components

---

**Reported by**: WFL MVC Framework Development
**Date**: 2026-01-10
**Test file**: `TestPrograms/test_container_property_mutation.wfl`
