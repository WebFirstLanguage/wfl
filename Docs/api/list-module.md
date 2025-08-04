# WFL List Module API Reference

## Overview

The List module provides functions for working with collections of values. Lists in WFL are dynamic arrays that can hold any mix of data types and grow or shrink as needed.

## List Basics

In WFL, lists are created using square brackets and can contain any mix of data types:

```wfl
// Creating lists
store numbers as [1, 2, 3, 4, 5]
store mixed as ["hello", 42, yes, nothing]
store empty_list as []

// Accessing list elements (0-based indexing)
store first_number as numbers[0]  // 1
store second_item as mixed[1]     // 42
```

## Functions

### `length(list)`

Returns the number of elements in a list.

**Parameters:**
- `list` (List): The list to count

**Returns:** Number (element count)

**Examples:**

```wfl
// Basic length calculation
store fruits as ["apple", "banana", "cherry"]
store fruit_count as length of fruits
display "Number of fruits: " with fruit_count  // 3

// Empty list
store empty as []
store empty_count as length of empty
display "Empty list length: " with empty_count  // 0

// Mixed type list
store mixed as [1, "hello", yes, nothing, [1, 2]]
store mixed_count as length of mixed
display "Mixed list length: " with mixed_count  // 5
```

**Natural Language Variants:**
```wfl
// All equivalent ways to get list length
store len1 as length of list
store len2 as size of list
store len3 as count of list
store len4 as how many items in list
store len5 as number of elements in list
```

**Practical Use Cases:**

```wfl
// Loop boundaries
action process_all_items with items:
    store item_count as length of items
    count i from 0 to item_count - 1:
        store current_item as items[i]
        display "Processing: " with current_item
    end
end

// Validation
action validate_shopping_cart with cart:
    store item_count as length of cart
    check if item_count is 0:
        display "Your cart is empty"
        return no
    end
    check if item_count > 50:
        display "Too many items in cart (max 50)"
        return no
    end
    return yes
end

// Progress tracking
action show_progress with completed_tasks and total_tasks:
    store completed_count as length of completed_tasks
    store total_count as length of total_tasks
    store percentage as (completed_count / total_count) * 100
    display "Progress: " with completed_count with "/" with total_count
    display "(" with percentage with "% complete)"
end
```

---

### `push(list, item)`

Adds an item to the end of a list. This function modifies the original list.

**Parameters:**
- `list` (List): The list to add to
- `item` (Any): The item to add

**Returns:** Nothing (modifies list in place)

**Examples:**

```wfl
// Basic push operations
store colors as ["red", "green"]
display "Before push: " with colors  // ["red", "green"]

push of colors and "blue"
display "After push: " with colors   // ["red", "green", "blue"]

// Adding different types
store mixed as [1, 2]
push of mixed and "hello"
push of mixed and yes
push of mixed and nothing
display "Mixed list: " with mixed    // [1, 2, "hello", yes, nothing]

// Adding lists to lists
store nested as [1, 2]
push of nested and [3, 4]
display "Nested: " with nested       // [1, 2, [3, 4]]
```

**Natural Language Variants:**
```wfl
// All equivalent ways to push
push of list and item
add item to list
append item to list
insert item at end of list
```

**Practical Use Cases:**

```wfl
// Building a shopping cart
action add_to_cart with cart and product:
    push of cart and product
    store cart_size as length of cart
    display "Added " with product with " to cart"
    display "Cart now has " with cart_size with " items"
end

// Collecting user input
action collect_names:
    store names as []
    store continue_input as yes
    
    while continue_input:
        display "Enter a name (or 'done' to finish):"
        store user_input as get_input
        
        check if user_input is "done":
            store continue_input as no
        otherwise:
            push of names and user_input
        end
    end
    
    return names
end

// Building search results
action search_products with products and query:
    store results as []
    
    count product in products:
        check if contains of product and query:
            push of results and product
        end
    end
    
    return results
end
```

---

### `pop(list)`

Removes and returns the last item from a list. This function modifies the original list.

**Parameters:**
- `list` (List): The list to remove from

**Returns:** The removed item (error if list is empty)

**Examples:**

```wfl
// Basic pop operations
store stack as [1, 2, 3, 4, 5]
display "Before pop: " with stack     // [1, 2, 3, 4, 5]

store last_item as pop of stack
display "Popped item: " with last_item  // 5
display "After pop: " with stack       // [1, 2, 3, 4]

// Multiple pops
store top as pop of stack              // 4
store next as pop of stack             // 3
display "Remaining: " with stack       // [1, 2]

// Working with different types
store mixed as ["a", "b", "c"]
store last_letter as pop of mixed      // "c"
display "Last letter: " with last_letter
```

**Error Handling:**
```wfl
// Attempting to pop from empty list causes error
store empty as []
// store item as pop of empty  // This would cause a runtime error

// Safe popping with length check
action safe_pop with list:
    store list_length as length of list
    check if list_length > 0:
        return pop of list
    otherwise:
        display "Cannot pop from empty list"
        return nothing
    end
end
```

**Natural Language Variants:**
```wfl
// All equivalent ways to pop
store item as pop of list
store item as remove last from list
store item as take from end of list
```

**Practical Use Cases:**

```wfl
// Stack implementation (Last In, First Out)
action create_undo_system:
    store undo_stack as []
    
    // Add action to undo stack
    action save_action with action_description:
        push of undo_stack and action_description
        display "Saved action: " with action_description
    end
    
    // Undo last action
    action undo_last:
        store stack_size as length of undo_stack
        check if stack_size > 0:
            store last_action as pop of undo_stack
            display "Undoing: " with last_action
            return last_action
        otherwise:
            display "Nothing to undo"
            return nothing
        end
    end
end

// Processing queue in reverse
action process_reverse_order with tasks:
    while length of tasks > 0:
        store current_task as pop of tasks
        display "Processing: " with current_task
        // Process the task here
    end
end
```

---

### `contains(list, item)`

Checks if a list contains a specific item.

**Parameters:**
- `list` (List): The list to search in
- `item` (Any): The item to search for

**Returns:** Boolean (yes if found, no if not found)

**Examples:**

```wfl
// Basic contains check
store fruits as ["apple", "banana", "cherry"]
store has_apple as contains of fruits and "apple"
display "Has apple: " with has_apple        // yes

store has_orange as contains of fruits and "orange"
display "Has orange: " with has_orange      // no

// Different data types
store numbers as [1, 2, 3, 4, 5]
store has_three as contains of numbers and 3
display "Has 3: " with has_three            // yes

store mixed as ["hello", 42, yes]
store has_bool as contains of mixed and yes
display "Has boolean: " with has_bool       // yes
```

**Natural Language Variants:**
```wfl
// All equivalent ways to check contains
check if contains of list and item
check if list contains item
check if item is in list
check if list has item
```

**Practical Use Cases:**

```wfl
// User permission checking
action user_has_permission with user_permissions and required_permission:
    return contains of user_permissions and required_permission
end

// Duplicate prevention
action add_unique_item with list and item:
    check if contains of list and item:
        display "Item already exists: " with item
    otherwise:
        push of list and item
        display "Added new item: " with item
    end
end

// Menu validation
action is_valid_choice with menu_options and user_choice:
    return contains of menu_options and user_choice
end

// Shopping cart validation
action validate_purchase with available_products and cart:
    count item in cart:
        check if not contains of available_products and item:
            display "Product not available: " with item
            return no
        end
    end
    return yes
end
```

---

### `indexof(list, item)` / `index_of(list, item)`

Finds the index (position) of an item in a list. Returns -1 if the item is not found.

**Parameters:**
- `list` (List): The list to search in
- `item` (Any): The item to find

**Returns:** Number (0-based index, or -1 if not found)

**Examples:**

```wfl
// Basic index finding
store colors as ["red", "green", "blue", "yellow"]
store green_index as indexof of colors and "green"
display "Green is at index: " with green_index  // 1

store purple_index as indexof of colors and "purple"
display "Purple index: " with purple_index      // -1 (not found)

// With numbers
store numbers as [10, 20, 30, 40]
store thirty_index as indexof of numbers and 30
display "30 is at index: " with thirty_index    // 2

// First occurrence only
store duplicates as ["a", "b", "a", "c"]
store first_a as indexof of duplicates and "a"
display "First 'a' at index: " with first_a     // 0
```

**Natural Language Variants:**
```wfl
// All equivalent ways to find index
store pos1 as indexof of list and item
store pos2 as index_of of list and item
store pos3 as position of item in list
store pos4 as find index of item in list
```

**Practical Use Cases:**

```wfl
// Safe list access
action safe_get_item with list and item:
    store item_index as indexof of list and item
    check if item_index >= 0:
        return list[item_index]
    otherwise:
        display "Item not found: " with item
        return nothing
    end
end

// List modification by value
action remove_by_value with list and item:
    store item_index as indexof of list and item
    check if item_index >= 0:
        // Would need additional functions to remove by index
        display "Found item at index: " with item_index
    otherwise:
        display "Item not found, cannot remove: " with item
    end
end

// Sorting and positioning
action insert_sorted with sorted_list and new_item:
    store list_length as length of sorted_list
    
    // Find insertion point (simplified)
    count i from 0 to list_length - 1:
        store current_item as sorted_list[i]
        check if new_item < current_item:
            display "Should insert at index: " with i
            // Would need insert function to complete
            return
        end
    end
    
    // If we get here, append to end
    push of sorted_list and new_item
end

// Replace item
action replace_item with list and old_item and new_item:
    store item_index as indexof of list and old_item
    check if item_index >= 0:
        // Direct assignment (WFL supports this)
        store list[item_index] as new_item
        display "Replaced " with old_item with " with " with new_item
    otherwise:
        display "Item not found for replacement: " with old_item
    end
end
```

## Advanced Examples

### List Processing Patterns

```wfl
// Filter list (create new list with matching items)
action filter_numbers with numbers and min_value:
    store filtered as []
    
    count num in numbers:
        check if num >= min_value:
            push of filtered and num
        end
    end
    
    return filtered
end

// Find maximum value
action find_max with numbers:
    store list_length as length of numbers
    check if list_length is 0:
        return nothing
    end
    
    store max_value as numbers[0]
    count i from 1 to list_length - 1:
        store current as numbers[i]
        check if current > max_value:
            store max_value as current
        end
    end
    
    return max_value
end

// List statistics
action calculate_stats with numbers:
    store count as length of numbers
    check if count is 0:
        return ["empty", 0, 0, 0]
    end
    
    store total as 0
    store max_val as numbers[0]
    store min_val as numbers[0]
    
    count num in numbers:
        store total as total + num
        check if num > max_val:
            store max_val as num
        end
        check if num < min_val:
            store min_val as num
        end
    end
    
    store average as total / count
    return [count, total, average, min_val, max_val]
end
```

### Data Management

```wfl
// Simple database-like operations
action create_user_database:
    store users as []
    
    action add_user with name and email:
        store user as [name, email]
        push of users and user
    end
    
    action find_user with email:
        count user in users:
            store user_email as user[1]
            check if user_email is email:
                return user
            end
        end
        return nothing
    end
    
    action get_all_users:
        return users
    end
end

// Shopping cart with quantities
action create_shopping_cart:
    store cart_items as []
    store cart_quantities as []
    
    action add_item with item and quantity:
        store item_index as indexof of cart_items and item
        check if item_index >= 0:
            // Item exists, update quantity
            store current_qty as cart_quantities[item_index]
            store cart_quantities[item_index] as current_qty + quantity
        otherwise:
            // New item
            push of cart_items and item
            push of cart_quantities and quantity
        end
    end
    
    action get_cart_summary:
        store summary as []
        store item_count as length of cart_items
        
        count i from 0 to item_count - 1:
            store item as cart_items[i]
            store qty as cart_quantities[i]
            store item_summary as [item, qty]
            push of summary and item_summary
        end
        
        return summary
    end
end
```

## Integration with Other Modules

### With Text Module

```wfl
// Text processing with lists
action split_into_words with sentence:
    store words as []
    store current_word as ""
    store sentence_length as length of sentence
    
    count i from 0 to sentence_length - 1:
        store char as substring of sentence and i and 1
        check if char is " ":
            check if length of current_word > 0:
                push of words and current_word
                store current_word as ""
            end
        otherwise:
            store current_word as current_word with char
        end
    end
    
    // Add last word
    check if length of current_word > 0:
        push of words and current_word
    end
    
    return words
end
```

### With Math Module

```wfl
// Statistical operations
action calculate_average with numbers:
    store count as length of numbers
    check if count is 0:
        return 0
    end
    
    store total as 0
    count num in numbers:
        store total as total + num
    end
    
    return total / count
end

// Random sampling
action get_random_item with list:
    store list_length as length of list
    check if list_length is 0:
        return nothing
    end
    
    store random_index as floor of (random * list_length)
    return list[random_index]
end
```

## Error Handling and Best Practices

### Safe List Operations

```wfl
// Safe list access with bounds checking
action safe_access with list and index:
    store list_length as length of list
    check if index < 0 or index >= list_length:
        display "Index out of bounds: " with index
        return nothing
    end
    return list[index]
end

// Safe pop operation
action safe_pop with list:
    check if length of list > 0:
        return pop of list
    otherwise:
        display "Cannot pop from empty list"
        return nothing
    end
end

// Validate list before operations
action validate_list with list:
    check if isnothing of list:
        display "List is nothing"
        return no
    end
    
    check if typeof of list is not "List":
        display "Expected list, got " with typeof of list
        return no
    end
    
    return yes
end
```

### Performance Considerations

1. **List growth**: Lists automatically resize, but frequent growth can be slow
2. **Search operations**: `contains` and `indexof` are O(n) operations
3. **Memory usage**: Lists store references, not copies of objects
4. **Modification during iteration**: Be careful when modifying lists while iterating

```wfl
// Efficient list building
action build_large_list with size:
    store result as []
    
    // Better to know approximate size if possible
    count i from 0 to size - 1:
        push of result and i
    end
    
    return result
end
```

## See Also

- [Core Module](core-module.md) - Basic utilities and type checking
- [Math Module](math-module.md) - Numeric operations for list calculations
- [Text Module](text-module.md) - String operations that work with lists
- [WFL Language Reference](../language-reference/wfl-spec.md) - Complete language specification
- [WFL Control Flow](../language-reference/wfl-control-flow.md) - Loops and iteration with lists