# Performance Tips

Write efficient WFL code with these optimization strategies. Focus on algorithmic efficiency and appropriate use of language features.

## Choose the Right Algorithm

**Most important for performance!**

**Slow (O(n²)):**
```wfl
store all_items as [1, 2, 2, 3, 3, 3]
create list unique_items:
end list

for each item in all_items:
    // Scanning unique_items for every item makes this O(n²)
    check if unique_items contains item:
        // already present, skip
    otherwise:
        push with unique_items and item
    end check
end for

display unique_items
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
open file at "perf_demo.txt" for writing as myfile
wait for write content "sample data" into myfile
close file myfile

open file at "perf_demo.txt" for reading as reader
wait for store file_content as read content from reader
close file reader
display file_content

// Web servers are naturally async too:
//   listen on port 8080 as web_server
//   wait for request comes in on web_server as req
```

**Why:** Non-blocking I/O lets WFL handle other work while waiting.

## Short-Circuit Evaluation

**Automatic in WFL!** Conditions stop evaluating when result is determined.

```wfl
store quick_check as yes
store expensive_check as no

// The second operand is only evaluated if the first doesn't decide the result
check if quick_check and expensive_check:
    display "Both true"
otherwise:
    display "Short-circuited on the first check"
end check

// With or, evaluation stops as soon as one operand is true
check if quick_check or expensive_check:
    display "At least one true"
end check
```

**Put cheap checks first:**

```wfl
store user_is_logged_in as yes
store has_database_permission as no

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
store large_list as ["  alice  ", "  bob  "]
define action called process_item with parameters value:
    return "[" with value with "]"
end action

for each item in large_list:
    store item_upper as touppercase of item
    store item_trimmed as trim of item_upper
    store item_processed as process_item of item_trimmed
    display item_processed
    // Many intermediate strings created
end for
```

**Better:**
```wfl
store large_list as ["  alice  ", "  bob  "]
define action called process_item with parameters value:
    return "[" with value with "]"
end action

for each item in large_list:
    store processed as process_item of (trim of (touppercase of item))
    display processed
    // Fewer intermediate values
end for
```

## Cache Expensive Calculations

**Don't recalculate:**

**Slow:**
```wfl
define action called expensive_calculation with parameters seed:
    return 500
end action
define action called do_work with parameters value:
    // process the value
end action

count from 1 to 1000:
    check if count is less than (expensive_calculation of count):
        do_work of count
    end check
    // Calls expensive_calculation 1000 times!
end count
```

**Fast:**
```wfl
define action called expensive_calculation with parameters seed:
    return 500
end action
define action called do_work with parameters value:
    // process the value
end action

store limit as expensive_calculation of 0  // Calculate once
count from 1 to 1000:
    check if count is less than limit:
        do_work of count
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
create list users:  // Fast append, slow search
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
store email_list as ["a@b.com", "c@d.com"]

for each email in email_list:
    create pattern email_pattern:
        one or more letter or digit
        followed by "@"
        one or more letter or digit
    end pattern
    check if email matches email_pattern:
        display "Valid"
    end check
    // Creates pattern on every iteration!
end for
```

**Efficient:**
```wfl
store email_list as ["a@b.com", "c@d.com"]

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
store words as ["hello", "world", "again"]
store result as ""

for each word in words:
    check if length of result is greater than 0:
        change result to result with " "
    end check
    change result to result with word
end for

display result
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
define action called perform_operation:
    display "working..."
end action

store start_time as current time in milliseconds

// Code to measure
perform_operation

store end_time as current time in milliseconds
store elapsed as end_time minus start_time
display "Operation took " with elapsed with "ms"
```

## Common Performance Mistakes

### Mistake 1: Unnecessary Loop Iterations

**Slow:**
```wfl
store items as ["a", "b", "c"]
define action called handle_item with parameters value:
    display "handling " with value
end action

for each item in items:
    handle_item of item
    // Even if we only need first match
end for
```

**Fast:**
```wfl
store items as ["a", "match", "c"]
define action called handle_item with parameters value:
    display "handling " with value
end action

for each item in items:
    check if item is equal to "match":
        handle_item of item
        break  // Stop as soon as we find it
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
        return n plus (sum_to of (n minus 1))
    end check
end action

// A small depth is fine:
display sum_to of 100

// store result as sum_to of 10000  // Deep recursion can overflow the stack!
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
