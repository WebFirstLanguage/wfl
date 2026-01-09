# List Module

The List module provides functions for working with lists (arrays). Manipulate collections of values efficiently.

## Functions

### length

**Purpose:** Get the number of items in a list.

**Signature:**
```wfl
length of <list>
```

**Parameters:**
- `list` (List): The list to measure

**Returns:** Number - Item count

**Example:**
```wfl
store numbers as [1, 2, 3, 4, 5]
display length of numbers                // Output: 5

store empty as []
display length of empty                  // Output: 0

create list fruits:
    add "apple"
    add "banana"
end list
display length of fruits                 // Output: 2
```

**Notes:**
- Also works on text (returns character count)
- Returns 0 for empty lists

**Use Cases:**
- Check if list is empty
- Loop bounds
- Validation
- Statistics

---

### push

**Purpose:** Add an item to the end of a list.

**Signature:**
```wfl
push with <list> and <value>
```

**Parameters:**
- `list` (List): The list to modify
- `value` (Any): The item to add

**Returns:** None (modifies list in place)

**Example:**
```wfl
create list items:
    add "first"
    add "second"
end list

push with items and "third"
display items                            // Output: [first, second, third]

push with items and 42
display items                            // Output: [first, second, third, 42]
```

**Notes:**
- Modifies the original list
- Can add any type of value
- Lists can be mixed-type (though not recommended)

**Use Cases:**
- Building lists dynamically
- Adding results from loops
- Accumulating data

**Example: Building a List**
```wfl
create list even_numbers
end list

count from 1 to 10:
    check if count % 2 is equal to 0:
        push with even_numbers and count
    end check
end count

display "Even numbers: " with even_numbers
// Output: Even numbers: [2, 4, 6, 8, 10]
```

---

### pop

**Purpose:** Remove and return the last item from a list.

**Signature:**
```wfl
pop from <list>
```

**Alternative:**
```wfl
<variable> as pop from <list>
```

**Parameters:**
- `list` (List): The list to modify

**Returns:** Any - The removed item

**Error:** Throws error if list is empty

**Example:**
```wfl
store stack as [1, 2, 3, 4, 5]

store last as pop from stack
display "Popped: " with last             // Output: Popped: 5
display "Remaining: " with stack         // Output: Remaining: [1, 2, 3, 4]

store another as pop from stack
display "Popped: " with another          // Output: Popped: 4
display "Remaining: " with stack         // Output: Remaining: [1, 2, 3]
```

**Use Cases:**
- Stack operations (LIFO)
- Undo functionality
- Processing from end to beginning

**Example: Safe Pop**
```wfl
define action called safe pop with parameters list:
    check if length of list is greater than 0:
        return pop from list
    otherwise:
        display "Warning: Cannot pop from empty list"
        return nothing
    end check
end action

store items as [1, 2]
store item1 as safe pop with items       // Returns: 2
store item2 as safe pop with items       // Returns: 1
store item3 as safe pop with items       // Returns: nothing (with warning)
```

---

### contains

**Purpose:** Check if a list contains a specific value.

**Signature:**
```wfl
contains of <list> and <value>
```

**Alternative syntax:**
```wfl
contains <value> in <list>
```

**Parameters:**
- `list` (List): The list to search
- `value` (Any): The value to find

**Returns:** Boolean - `yes` if found, `no` otherwise

**Example:**
```wfl
store numbers as [1, 2, 3, 4, 5]

check if contains of numbers and 3:
    display "Found 3"
end check

check if contains of numbers and 10:
    display "Found 10"
otherwise:
    display "10 not found"
end check
```

**Note:** Currently has limitations with function dispatch - may work only with specific syntax. Use iteration as workaround if needed.

**Use Cases:**
- Membership testing
- Validation
- Search functionality

---

### indexof

**Purpose:** Find the position of an item in a list.

**Signature:**
```wfl
indexof of <list> and <value>
```

**Aliases:** `index_of`

**Parameters:**
- `list` (List): The list to search
- `value` (Any): The item to find

**Returns:** Number - Index (0-based) or -1 if not found

**Example:**
```wfl
store animals as ["cat", "dog", "bird", "fish"]

store dog_pos as indexof of animals and "dog"
display "Dog is at index: " with dog_pos
// Output: Dog is at index: 1

store lion_pos as indexof of animals and "lion"
display "Lion is at index: " with lion_pos
// Output: Lion is at index: -1 (not found)
```

**Notes:**
- Returns first occurrence only
- Zero-indexed (first item is 0)
- Returns -1 if not found

**Use Cases:**
- Find item position
- Check existence (>= 0 means found)
- Remove item by index

**Example: Find and Remove**
```wfl
define action called remove item with parameters list and value:
    store index as indexof of list and value

    check if index is greater than or equal to 0:
        display "Found " with value with " at index " with index
        // Note: No built-in remove_at yet
        // Would need manual implementation
        return yes
    otherwise:
        display value with " not found in list"
        return no
    end check
end action
```

---

## Complete Example

Using all list functions together:

```wfl
display "=== List Module Demo ==="
display ""

// Create a list
create list tasks:
    add "Write code"
    add "Test code"
    add "Deploy code"
end list

display "Initial tasks: " with tasks
display "Count: " with length of tasks
display ""

// Add item
push with tasks and "Document code"
display "After push: " with tasks
display "New count: " with length of tasks
display ""

// Check membership
check if contains of tasks and "Test code":
    display "Testing is in the list"
end check
display ""

// Find position
store test_index as indexof of tasks and "Test code"
display "Test code is at index: " with test_index
display ""

// Remove item
store removed as pop from tasks
display "Removed: " with removed
display "After pop: " with tasks
display "Final count: " with length of tasks
display ""

display "=== Demo Complete ==="
```

**Output:**
```
=== List Module Demo ===

Initial tasks: [Write code, Test code, Deploy code]
Count: 3

After push: [Write code, Test code, Deploy code, Document code]
New count: 4

Testing is in the list

Test code is at index: 1

Removed: Document code
After pop: [Write code, Test code, Deploy code]
Final count: 3

=== Demo Complete ===
```

## Common Patterns

### Filter List

```wfl
store numbers as [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

create list even_numbers
end list

for each num in numbers:
    check if num % 2 is equal to 0:
        push with even_numbers and num
    end check
end for

display "Even numbers: " with even_numbers
// Output: Even numbers: [2, 4, 6, 8, 10]
```

### Find Maximum

```wfl
store numbers as [5, 12, 8, 23, 4, 16]

store max as numbers[0]

for each num in numbers:
    check if num is greater than max:
        change max to num
    end check
end for

display "Maximum: " with max
// Output: Maximum: 23
```

### Remove Duplicates

```wfl
store items as ["apple", "banana", "apple", "cherry", "banana"]

create list unique
end list

for each item in items:
    store index as indexof of unique and item
    check if index is equal to -1:
        push with unique and item
    end check
end for

display "Unique items: " with unique
// Output: Unique items: [apple, banana, cherry]
```

### Reverse List

```wfl
store original as [1, 2, 3, 4, 5]

create list reversed
end list

store count as length of original

count from 1 to count:
    store index as count minus count
    store item as original[index]
    push with reversed and item
end count

display "Reversed: " with reversed
// Output: Reversed: [5, 4, 3, 2, 1]
```

## Best Practices

✅ **Check length before pop:** Prevent errors on empty lists

✅ **Use contains for membership:** Simpler than indexof when you just need yes/no

✅ **Use indexof for position:** When you need the actual index

✅ **Initialize empty lists:** Create before pushing

✅ **Use for each for iteration:** Clearer than index loops

❌ **Don't pop from empty lists:** Check length first

❌ **Don't assume item exists:** Use indexof to check first

❌ **Don't modify while iterating:** Can cause unexpected behavior

## What You've Learned

In this module, you learned:

✅ **length** - Get list size
✅ **push** - Add items to end
✅ **pop** - Remove from end
✅ **contains** - Check membership
✅ **indexof** - Find item position
✅ **Common patterns** - Filter, find max, remove duplicates, reverse
✅ **Best practices** - Safe list operations

## Next Steps

Continue exploring the standard library:

**[Filesystem Module →](filesystem-module.md)**
File and directory operations.

**[Text Module →](text-module.md)**
Review string functions for list of words.

Or return to:
**[Standard Library Index →](index.md)**

---

**Previous:** [← Text Module](text-module.md) | **Next:** [Filesystem Module →](filesystem-module.md)
