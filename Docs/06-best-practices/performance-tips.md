# Performance Tips

Write efficient WFL code with these optimization strategies. Focus on algorithmic efficiency and appropriate use of language features.

## Choose the Right Algorithm

**Most important for performance!**

**Slow (O(n²)):**
```wfl
create list unique_items
end list

for each item in all_items:
    store found as no
    for each unique in unique_items:
        check if item is equal to unique:
            change found to yes
        end check
    end for
    check if found is no:
        push with unique_items and item
    end check
end for
```

**Faster with better algorithm:**
```wfl
// Use hash-based approach (when available)
// Or accept duplicates and filter later
```

## Use Async for I/O

**I/O operations benefit from async:**

```wfl
// Async file operations
open file at "data.txt" for reading as myfile
wait for store content as read content from myfile
close file myfile

// Web server naturally async
wait for request comes in on server as req
```

**Why:** Non-blocking I/O lets WFL handle other work while waiting.

## Short-Circuit Evaluation

**Automatic in WFL!** Conditions stop evaluating when result is determined.

```wfl
// Expensive operation only runs if quick check passes
check if quick_check() and expensive_operation():
    display "Both true"
end check

// Expensive operation never runs if quick check is true
check if quick_check() or expensive_operation():
    display "At least one true"
end check
```

**Put cheap checks first:**

```wfl
// Good: cheap check first
check if user_is_logged_in and has_database_permission:
    // database check only if logged in
end check

// Poor: expensive check first
check if has_database_permission and user_is_logged_in:
    // database query runs even if not logged in!
end check
```

## Avoid Unnecessary Copies

**Reuse variables when possible:**

**Inefficient:**
```wfl
for each item in large_list:
    store item_upper as touppercase of item
    store item_trimmed as trim of item_upper
    store item_processed as process(item_trimmed)
    // Many intermediate strings created
end for
```

**Better:**
```wfl
for each item in large_list:
    store processed as process(trim of touppercase of item)
    // Fewer intermediate values
end for
```

## Cache Expensive Calculations

**Don't recalculate:**

**Slow:**
```wfl
count from 1 to 1000:
    check if count is less than expensive_calculation():
        process(count)
    end check
    // Calculates 1000 times!
end count
```

**Fast:**
```wfl
store limit as expensive_calculation()  // Calculate once
count from 1 to 1000:
    check if count is less than limit:
        process(count)
    end check
end count
```

## List Operations

### Pre-allocate When Possible

**Know the size? Create with capacity (future feature):**

```wfl
// Future syntax:
// create list items with capacity 1000
```

### Use Right Data Structure

**List for ordered data:**
```wfl
create list users  // Fast append, slow search
end list
```

**Map for key-value (when available):**
```wfl
// Future: Fast lookup by key
// create map user_by_id
```

## Pattern Compilation

**Compile patterns once, reuse many times:**

**Inefficient:**
```wfl
for each email in email_list:
    create pattern email_pattern:
        one or more letter or digit
        followed by "@"
        one or more letter or digit
    end pattern
    check if email matches email_pattern:
        display "Valid"
    end check
    // Creates pattern 1000 times!
end for
```

**Efficient:**
```wfl
// Create pattern once
create pattern email_pattern:
    one or more letter or digit
    followed by "@"
    one or more letter or digit
end pattern

// Use many times
for each email in email_list:
    check if email matches email_pattern:
        display "Valid"
    end check
end for
```

## String Building

**For many concatenations, build incrementally:**

```wfl
store result as ""

for each word in words:
    check if length of result is greater than 0:
        change result to result with " "
    end check
    change result to result with word
end for
```

## File I/O

### Buffer Reads/Writes

**Use wait for to batch operations:**

```wfl
open file at "output.txt" for writing as myfile

// Multiple writes batched
wait for write content "Line 1\n" into myfile
wait for append content "Line 2\n" into myfile
wait for append content "Line 3\n" into myfile

close file myfile  // Flushes buffer
```

## Profiling

### Use Timing

```bash
wfl --time your_program.wfl
```

Shows execution time to identify slow parts.

### Measure First, Optimize Second

**Don't guess what's slow. Measure it.**

```wfl
store start as current time in milliseconds

// Code to measure
perform_operation()

store end as current time in milliseconds
store elapsed as end minus start
display "Operation took " with elapsed with "ms"
```

## Common Performance Mistakes

### Mistake 1: Unnecessary Loop Iterations

**Slow:**
```wfl
for each item in list:
    process(item)
    // Even if we only need first match
end for
```

**Fast:**
```wfl
for each item in list:
    check if matches(item):
        process(item)
        break  // Stop when found (if supported)
    end check
end for
```

### Mistake 2: Repeated String Conversion

**Slow:**
```wfl
count from 1 to 1000:
    display "Count: " with count  // Converts count to string 1000 times
end count
```

**Acceptable** - Display is output, optimization not critical here. But for intensive string building, consider batching.

### Mistake 3: Deep Recursion

**Can cause stack overflow:**

```wfl
define action called sum_to with parameters n:
    check if n is less than or equal to 0:
        return 0
    otherwise:
        return n plus sum_to with n minus 1
    end check
end action

store result as sum_to with 10000  // Stack overflow!
```

**Better: Use iteration:**

```wfl
define action called sum_to with parameters n:
    store total as 0
    count from 1 to n:
        change total to total plus count
    end count
    return total
end action
```

## Best Practices

✅ **Choose right algorithm** - Most important!
✅ **Use async for I/O** - Non-blocking operations
✅ **Cache expensive results** - Don't recalculate
✅ **Compile patterns once** - Reuse across iterations
✅ **Short-circuit cleverly** - Cheap checks first
✅ **Measure performance** - Use --time flag
✅ **Optimize bottlenecks** - Profile first
✅ **Use iteration over recursion** - For large N

❌ **Don't optimize prematurely** - Measure first
❌ **Don't sacrifice readability** - Unless necessary
❌ **Don't deep recurse** - Stack limits exist

## What You've Learned

✅ Algorithm choice matters most
✅ Async for I/O efficiency
✅ Short-circuit evaluation
✅ Caching expensive operations
✅ Pattern compilation
✅ List operation efficiency
✅ String building strategies
✅ Profiling with --time
✅ Common mistakes to avoid

**Next:** [Testing Strategies →](testing-strategies.md)

---

**Previous:** [← Security Guidelines](security-guidelines.md) | **Next:** [Testing Strategies →](testing-strategies.md)
