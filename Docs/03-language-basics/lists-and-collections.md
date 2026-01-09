# Lists and Collections

Lists store multiple values in a single variable. They're perfect for working with collections of data.

## Creating Lists

### Block Syntax

Create a list with the `create list` block:

```wfl
create list fruits:
    add "apple"
    add "banana"
    add "orange"
end list
```

**Syntax:**
```wfl
create list <name>:
    add <value>
    add <value>
    ...
end list
```

### Literal Syntax

Create lists with literal notation using `and` as separator:

```wfl
store numbers as [1 and 2 and 3 and 4 and 5]
store colors as ["red" and "green" and "blue"]
```

**Syntax:**
```wfl
store <name> as [<value> and <value> and ...]
```

### Empty Lists

Create an empty list:

```wfl
create list items
end list

// Or (if literal syntax supports it):
store items as []
```

## Accessing List Elements

### By Index

Access list items by position (zero-indexed):

```wfl
store numbers as [10 and 20 and 30 and 40]

store first as numbers[0]          // 10
store second as numbers[1]         // 20
store third as numbers[2]          // 30
```

**Note:** Lists start at index 0, not 1!

- Index 0 = first item
- Index 1 = second item
- Index 2 = third item
- etc.

### Direct Index Access

WFL supports natural index syntax:

```wfl
store items as ["first" and "second" and "third"]

display items 0                    // "first"
display items 1                    // "second"
display items 2                    // "third"
```

## List Operations

### Length

Get the number of items in a list:

```wfl
store numbers as [1 and 2 and 3 and 4 and 5]
store size as length of numbers

display "List has " with size with " items"
// Output: "List has 5 items"
```

### Push (Add to End)

Add an item to the end of a list:

```wfl
create list tasks:
    add "Write code"
    add "Test code"
end list

push to tasks with "Deploy code"

// tasks now contains: ["Write code", "Test code", "Deploy code"]
```

**Syntax:**
```wfl
push to <list> with <value>
```

### Pop (Remove from End)

Remove and return the last item:

```wfl
create list stack:
    add "first"
    add "second"
    add "third"
end list

store last item as pop from stack

display "Popped: " with last item
// Output: "Popped: third"

// stack now contains: ["first", "second"]
```

**Syntax:**
```wfl
<variable> as pop from <list>
```

### Contains

Check if a list contains a specific value:

```wfl
store colors as ["red" and "green" and "blue"]

check if contains of colors and "red":
    display "Red is in the list"
end check

check if contains of colors and "yellow":
    display "Yellow is in the list"
otherwise:
    display "Yellow is NOT in the list"
end check
```

**Syntax:**
```wfl
contains of <list> and <value>
```

Returns `yes` or `no`.

### Index Of

Find the position of an item in a list:

```wfl
store animals as ["cat" and "dog" and "bird" and "fish"]

store dog position as indexof of animals and "dog"
display "Dog is at position: " with dog position
// Output: "Dog is at position: 1"

store lion position as indexof of animals and "lion"
display "Lion position: " with lion position
// Output: "Lion position: -1" (not found)
```

**Returns:** Index number, or -1 if not found

## Iterating Over Lists

### For Each Loop

The most common way to process lists:

```wfl
create list names:
    add "Alice"
    add "Bob"
    add "Carol"
end list

for each name in names:
    display "Hello, " with name with "!"
end for
```

**Output:**
```
Hello, Alice!
Hello, Bob!
Hello, Carol!
```

### By Index

Use a count loop with indexing:

```wfl
store items as ["first" and "second" and "third"]
store size as length of items

count from 0 to size minus 1:
    store item as items[count]
    display "Item " with count with ": " with item
end count
```

**Output:**
```
Item 0: first
Item 1: second
Item 2: third
```

## List Patterns

### Building a List

```wfl
create list squares
end list

count from 1 to 10:
    store squared as count times count
    push to squares with squared
end count

display "Squares: " with squares
// Output: "Squares: [1, 4, 9, 16, 25, 36, 49, 64, 81, 100]"
```

### Filtering

```wfl
store all numbers as [1 and 2 and 3 and 4 and 5 and 6 and 7 and 8 and 9 and 10]

create list even numbers
end list

for each number in all numbers:
    check if number modulo 2 is equal to 0:
        push to even numbers with number
    end check
end for

display "Even numbers: " with even numbers
// Output: "Even numbers: [2, 4, 6, 8, 10]"
```

### Mapping (Transformation)

```wfl
store celsius temps as [0 and 10 and 20 and 30 and 40]

create list fahrenheit temps
end list

for each celsius in celsius temps:
    store fahrenheit as celsius times 9 divided by 5 plus 32
    push to fahrenheit temps with fahrenheit
end for

display "Celsius: " with celsius temps
display "Fahrenheit: " with fahrenheit temps
```

### Searching

```wfl
create list items:
    add "apple"
    add "banana"
    add "cherry"
    add "date"
end list

store search for as "cherry"
store found at as -1

count from 0 to length of items minus 1:
    check if items[count] is equal to search for:
        change found at to count
    end check
end count

check if found at is greater than or equal to 0:
    display "Found " with search for with " at index " with found at
otherwise:
    display search for with " not found"
end check
```

### Accumulation

```wfl
store prices as [10.50 and 25.00 and 5.75 and 30.25]

store total as 0

for each price in prices:
    change total to total plus price
end for

display "Total: $" with total
// Output: "Total: $71.5"
```

## List with Different Types

Lists can contain different types (though usually you'd keep them consistent):

```wfl
store mixed as [42 and "hello" and yes and 3.14]

for each item in mixed:
    display "Item: " with item with ", Type: " with typeof of item
end for
```

**Output:**
```
Item: 42, Type: Number
Item: hello, Type: Text
Item: yes, Type: Boolean
Item: 3.14, Type: Number
```

**Best practice:** Keep lists homogeneous (all same type) for clarity.

## Nested Lists

Lists can contain other lists:

```wfl
store matrix as [[1 and 2] and [3 and 4] and [5 and 6]]

for each row in matrix:
    display "Row: " with row
end for
```

**Output:**
```
Row: [1, 2]
Row: [3, 4]
Row: [5, 6]
```

**Accessing nested elements:**
```wfl
store first row as matrix[0]       // [1, 2]
store first element as first row[0] // 1

// Or directly (if supported):
// store first element as matrix[0][0]
```

## Real-World Examples

### Shopping List

```wfl
create list shopping list:
    add "milk"
    add "eggs"
    add "bread"
    add "butter"
end list

display "=== Shopping List ==="
store item number as 1

for each item in shopping list:
    display item number with ". " with item
    change item number to item number plus 1
end for

store total items as length of shopping list
display ""
display "Total items: " with total items
```

**Output:**
```
=== Shopping List ===
1. milk
2. eggs
3. bread
4. butter

Total items: 4
```

### Grade Book

```wfl
create list student names:
    add "Alice"
    add "Bob"
    add "Carol"
end list

create list student scores:
    add 92
    add 87
    add 95
end list

display "=== Grade Report ==="

count from 0 to length of student names minus 1:
    store name as student names[count]
    store score as student scores[count]
    display name with ": " with score with "%"
end count
```

**Output:**
```
=== Grade Report ===
Alice: 92%
Bob: 87%
Carol: 95%
```

### Top Scores

```wfl
store scores as [85 and 92 and 78 and 95 and 88]

store max score as 0

for each score in scores:
    check if score is greater than max score:
        change max score to score
    end check
end for

display "Highest score: " with max score
// Output: "Highest score: 95"
```

### Word Counter

```wfl
store sentence as "the quick brown fox jumps over the lazy dog"
store words as split of sentence by " "

store word count as length of words
display "Word count: " with word count
// Output: "Word count: 9"

display "Words:"
for each word in words:
    display "  - " with word
end for
```

### Unique Items

```wfl
store all items as ["apple" and "banana" and "apple" and "cherry" and "banana"]

create list unique items
end list

for each item in all items:
    check if not contains of unique items and item:
        push to unique items with item
    end check
end for

display "Unique items: " with unique items
// Output: "Unique items: [apple, banana, cherry]"
```

### Statistics Calculator

```wfl
store data as [10 and 15 and 20 and 25 and 30]

// Calculate sum
store sum as 0
for each value in data:
    change sum to sum plus value
end for

// Calculate average
store count as length of data
store average as sum divided by count

// Find min and max
store min as data[0]
store max as data[0]

for each value in data:
    check if value is less than min:
        change min to value
    end check
    check if value is greater than max:
        change max to value
    end check
end for

display "Sum: " with sum
display "Average: " with average
display "Min: " with min
display "Max: " with max
```

**Output:**
```
Sum: 100
Average: 20
Min: 10
Max: 30
```

## Common Mistakes

### Wrong Index

**Remember:** Lists are zero-indexed!

**Wrong:**
```wfl
store items as ["first" and "second" and "third"]
store first as items[1]  // This gets "second", not "first"!
```

**Right:**
```wfl
store first as items[0]  // Index 0 is the first item
```

### Index Out of Bounds

**Dangerous:**
```wfl
store items as ["a" and "b" and "c"]
store item as items[10]  // ERROR: Index out of bounds!
```

**Safe:**
```wfl
store size as length of items
check if 10 is less than size:
    store item as items[10]
otherwise:
    display "Index too large"
end check
```

### Forgetting `end list`

**Wrong:**
```wfl
create list items:
    add "first"
    add "second"
// Missing end list!
```

**Right:**
```wfl
create list items:
    add "first"
    add "second"
end list
```

### Popping from Empty List

**Dangerous:**
```wfl
create list empty
end list

store item as pop from empty  // ERROR: Cannot pop from empty list
```

**Safe:**
```wfl
check if length of empty is greater than 0:
    store item as pop from empty
otherwise:
    display "List is empty"
end check
```

## Practice Exercises

### Exercise 1: List Builder

Create a list of your 5 favorite movies. Display each one with its position (1-5).

### Exercise 2: Number Doubler

Given a list [1, 2, 3, 4, 5], create a new list where each number is doubled.

### Exercise 3: Filter Negatives

Given a list of numbers (including negatives), create a new list containing only the positive numbers.

### Exercise 4: Find Average

Create a list of test scores. Calculate and display:
- The sum
- The average
- The highest score
- The lowest score

### Exercise 5: Reverse a List

Given a list, create a new list with items in reverse order.
Hint: Use a count loop going backwards.

### Exercise 6: Shopping Cart

Create a shopping cart system with:
- List of item names
- List of item prices (same order)
- Display each item with its price
- Calculate and display the total

## Best Practices

✅ **Use descriptive list names:** `customer names` not `list1`

✅ **Keep lists homogeneous:** All items same type (usually)

✅ **Check bounds:** Verify index before accessing

✅ **Use for each when possible:** Clearer than index loops

✅ **Initialize before adding:** Create the list first

✅ **Check for empty:** Test `length` before popping

❌ **Don't forget zero-indexing:** First item is 0, not 1

❌ **Don't modify while iterating:** Can cause unexpected behavior

❌ **Don't use magic numbers:** Use `length of list` instead of hardcoded counts

## What You've Learned

In this section, you learned:

✅ **Creating lists** - Block syntax and literal syntax
✅ **Accessing elements** - By index (zero-based)
✅ **List operations** - length, push, pop, contains, indexof
✅ **Iterating** - for each loops and index-based iteration
✅ **Common patterns** - Building, filtering, mapping, searching
✅ **Nested lists** - Lists containing lists
✅ **Best practices** - Safe list manipulation

## Next Steps

Now that you understand lists:

**[Error Handling →](error-handling.md)**
Learn how to handle errors gracefully when working with lists and other operations.

Or explore related topics:
- [Loops and Iteration →](loops-and-iteration.md) - Review loop syntax
- [Actions (Functions) →](actions-functions.md) - Pass lists to actions
- [Standard Library: List Module →](../05-standard-library/list-module.md) - All list functions

---

**Previous:** [← Actions (Functions)](actions-functions.md) | **Next:** [Error Handling →](error-handling.md)
