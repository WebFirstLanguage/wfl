# Loops and Iteration

Loops let you repeat actions multiple times. WFL provides natural language syntax for different types of iteration.

## Count Loops

The `count` loop repeats a specific number of times:

```wfl
count from 1 to 5:
    display "Number: " with count
end count
```

**Output:**
```
Number: 1
Number: 2
Number: 3
Number: 4
Number: 5
```

**Syntax:**
```wfl
count from <start> to <end>:
    <statements>
end count
```

The variable `count` is automatically created and contains the current number.

### Count with Step

Count by different increments using `by`:

```wfl
count from 0 to 10 by 2:
    display "Even: " with count
end count
```

**Output:**
```
Even: 0
Even: 2
Even: 4
Even: 6
Even: 8
Even: 10
```

**Syntax:**
```wfl
count from <start> to <end> by <step>:
    <statements>
end count
```

### Count Examples

**Count to 100 by tens:**
```wfl
count from 0 to 100 by 10:
    display count
end count
```

**Count by fives:**
```wfl
count from 5 to 50 by 5:
    display count with " is divisible by 5"
end count
```

**Multiplication table:**
```wfl
store number as 7
count from 1 to 10:
    store result as number times count
    display number with " × " with count with " = " with result
end count
```

**Output:**
```
7 × 1 = 7
7 × 2 = 14
7 × 3 = 21
...
7 × 10 = 70
```

## For Each Loops

Iterate over items in a list:

```wfl
create list fruits:
    add "apple"
    add "banana"
    add "orange"
end list

for each fruit in fruits:
    display "I like " with fruit
end for
```

**Output:**
```
I like apple
I like banana
I like orange
```

**Syntax:**
```wfl
for each <item> in <list>:
    <statements>
end for
```

The variable `<item>` is automatically created and contains the current list element.

### For Each Examples

**Processing items:**
```wfl
create list prices:
    add 10.50
    add 25.00
    add 5.75
end list

store total as 0

for each price in prices:
    change total to total plus price
    display "Added " with price with ", total now: " with total
end for

display "Final total: $" with total
```

**String processing:**
```wfl
create list names:
    add "alice"
    add "bob"
    add "carol"
end list

for each name in names:
    store uppercase name as touppercase of name
    display "Hello, " with uppercase name with "!"
end for
```

**Output:**
```
Hello, ALICE!
Hello, BOB!
Hello, CAROL!
```

### Reversed Iteration

Iterate backwards through a list (if supported):

```wfl
create list numbers:
    add 1
    add 2
    add 3
end list

for each number in numbers reversed:
    display number
end for
```

**Output:**
```
3
2
1
```

## Repeat Loops

### Repeat While

Repeat while a condition is true:

```wfl
store counter as 1

repeat while counter is less than or equal to 5:
    display "Counter: " with counter
    change counter to counter plus 1
end repeat
```

**Output:**
```
Counter: 1
Counter: 2
Counter: 3
Counter: 4
Counter: 5
```

**Syntax:**
```wfl
repeat while <condition>:
    <statements>
end repeat
```

**Warning:** Make sure the condition eventually becomes false, or you'll have an infinite loop!

### Repeat Until

Repeat until a condition becomes true:

```wfl
store counter as 1

repeat until counter is greater than 5:
    display "Counter: " with counter
    change counter to counter plus 1
end repeat
```

**Output:** (Same as above)

**Syntax:**
```wfl
repeat until <condition>:
    <statements>
end repeat
```

**Difference:** `while` continues while true, `until` continues while false.

### Repeat Forever

Create an infinite loop (use with caution!):

```wfl
repeat forever:
    display "This runs forever!"
    // Need break or exit mechanism (if supported)
end repeat
```

**Common use:** Server request loops, game loops

## Loop Control

### Break (Exit Loop)

Exit a loop early (if supported):

```wfl
count from 1 to 100:
    display count
    check if count is equal to 5:
        break  // Exit the loop
    end check
end count

display "Loop exited at 5"
```

### Continue (Skip)

Skip to the next iteration (if supported):

```wfl
count from 1 to 10:
    check if count modulo 2 is equal to 0:
        skip  // Skip even numbers
    end check
    display count  // Only odd numbers
end count
```

**Output:**
```
1
3
5
7
9
```

## Nested Loops

Loops inside loops for multi-dimensional iteration:

```wfl
count from 1 to 3:
    display "Outer: " with count
    count from 1 to 2:
        display "  Inner: " with count
    end count
end count
```

**Output:**
```
Outer: 1
  Inner: 1
  Inner: 2
Outer: 2
  Inner: 1
  Inner: 2
Outer: 3
  Inner: 1
  Inner: 2
```

### Multiplication Table

```wfl
count from 1 to 5:
    store row as count
    count from 1 to 5:
        store column as count
        store product as row times column
        display row with " × " with column with " = " with product
    end count
    display ""  // Blank line between rows
end count
```

### Grid Pattern

```wfl
count from 1 to 3:
    store row as count
    count from 1 to 5:
        display "*"
    end count
    display ""  // Newline after each row
end count
```

**Output:**
```
*****
*****
*****
```

## Common Patterns

### Accumulation

Sum up values:

```wfl
store total as 0

count from 1 to 10:
    change total to total plus count
end count

display "Sum of 1-10: " with total
// Output: "Sum of 1-10: 55"
```

### Finding Maximum

```wfl
create list numbers:
    add 5
    add 12
    add 8
    add 23
    add 4
end list

store max as 0

for each number in numbers:
    check if number is greater than max:
        change max to number
    end check
end for

display "Maximum: " with max
// Output: "Maximum: 23"
```

### Counting Matches

```wfl
create list scores:
    add 95
    add 82
    add 91
    add 78
    add 88
end list

store a count as 0

for each score in scores:
    check if score is greater than or equal to 90:
        change a count to a count plus 1
    end check
end for

display "Number of A grades: " with a count
// Output: "Number of A grades: 2"
```

### Building a List

```wfl
create list even numbers
end list

count from 1 to 10:
    check if count modulo 2 is equal to 0:
        push with even numbers and count
    end check
end count

display "Even numbers: " with even numbers
```

### Filtering

```wfl
create list all items:
    add "apple"
    add "apricot"
    add "banana"
    add "avocado"
end list

create list a items
end list

for each item in all items:
    store first letter as substring of item from 0 length 1
    check if first letter is "a":
        push with a items and item
    end check
end for

display "Items starting with 'a':"
for each item in a items:
    display "  - " with item
end for
```

### Searching

```wfl
create list names:
    add "Alice"
    add "Bob"
    add "Carol"
    add "David"
end list

store search name as "Carol"
store found as no

for each name in names:
    check if name is search name:
        change found to yes
        display "Found " with search name with "!"
    end check
end for

check if found is no:
    display search name with " not found"
end check
```

## Real-World Examples

### File Counter

```wfl
// Count files in a directory
list files in "." as file list

store file count as 0

for each file in file list:
    change file count to file count plus 1
end for

display "Total files: " with file count
```

### Price Calculator with Tax

```wfl
create list prices:
    add 19.99
    add 34.50
    add 12.25
end list

store subtotal as 0

for each price in prices:
    change subtotal to subtotal plus price
end for

store tax rate as 0.08
store tax as subtotal times tax rate
store total as subtotal plus tax

display "Subtotal: $" with subtotal
display "Tax: $" with tax
display "Total: $" with total
```

### Countdown Timer

```wfl
store seconds as 10

repeat while seconds is greater than 0:
    display "Time remaining: " with seconds with " seconds"
    change seconds to seconds minus 1
end repeat

display "Time's up!"
```

### Score Analyzer

```wfl
create list test scores:
    add 85
    add 92
    add 78
    add 95
    add 88
    add 76
end list

store total as 0
store count as 0

for each score in test scores:
    change total to total plus score
    change count to count plus 1
end for

store average as total divided by count

display "Total scores: " with count
display "Sum: " with total
display "Average: " with average
```

### Pattern Generator

```wfl
count from 1 to 5:
    store row as count
    count from 1 to row:
        display "*"
    end count
    display ""  // Newline
end count
```

**Output:**
```
*
**
***
****
*****
```

## Loop Variables

### In Count Loops

The `count` variable is automatically created:

```wfl
count from 5 to 10:
    display "Current count: " with count
end count
```

**You can use it in calculations:**
```wfl
count from 1 to 5:
    store squared as count times count
    display count with " squared is " with squared
end count
```

### In For Each Loops

The iteration variable takes the name you specify:

```wfl
for each fruit in fruits:         // Variable name is "fruit"
    display fruit
end for

for each number in numbers:        // Variable name is "number"
    display number times 2
end for
```

## Common Mistakes

### Forgetting `end count` or `end for`

**Wrong:**
```wfl
count from 1 to 5:
    display count
// Missing end count!
```

**Right:**
```wfl
count from 1 to 5:
    display count
end count
```

### Infinite Loops

**Dangerous:**
```wfl
store x as 1
repeat while x is less than 10:
    display x
    // FORGOT to increment x - infinite loop!
end repeat
```

**Safe:**
```wfl
store x as 1
repeat while x is less than 10:
    display x
    change x to x plus 1  // Always increment!
end repeat
```

### Modifying Loop Variable

**Confusing:**
```wfl
count from 1 to 10:
    change count to count plus 5  // Don't do this! Confusing behavior
    display count
end count
```

**Better:** Use a separate variable if you need to modify values.

### Off-by-One Errors

**Wrong assumption:**
```wfl
count from 1 to 10:
    // This runs 10 times (1, 2, 3, ..., 10)
    // Not 9 times!
end count
```

**Remember:** `count from 1 to 10` includes both 1 and 10.

## Practice Exercises

### Exercise 1: Times Table

Create a program that displays the 7 times table (7×1 through 7×10).

### Exercise 2: Sum Calculator

Calculate the sum of all numbers from 1 to 100 using a count loop.

### Exercise 3: Even/Odd Filter

Given a list of numbers [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]:
- Create two new lists: one for even numbers, one for odd
- Use a for each loop to categorize each number

### Exercise 4: Countdown

Create a countdown from 10 to 1, displaying each number. At 0, display "Blast off!"

### Exercise 5: Shopping List Total

Create a shopping list with items and prices. Use a loop to:
- Display each item with its price
- Calculate and display the total cost

### Exercise 6: Pattern Challenge

Create this pattern using nested loops:
```
1
22
333
4444
55555
```

## Best Practices

✅ **Use count loops for fixed iterations:** When you know how many times

✅ **Use for each for collections:** When processing lists

✅ **Use descriptive variable names:** `for each customer in customers` not `for each x in list`

✅ **Avoid modifying loop variables:** Keep `count` as-is

✅ **Always increment in while loops:** Prevent infinite loops

✅ **Consider performance:** Avoid expensive operations inside loops when possible

❌ **Don't nest too deeply:** More than 2-3 levels is hard to read

❌ **Don't forget `end count` / `end for` / `end repeat`**

❌ **Don't create infinite loops accidentally**

## What You've Learned

In this section, you learned:

✅ **Count loops** - `count from X to Y` and `count from X to Y by Z`
✅ **For each loops** - Iterate over lists
✅ **Repeat while loops** - Continue while condition is true
✅ **Repeat until loops** - Continue until condition is true
✅ **Loop variables** - `count` in count loops, custom names in for each
✅ **Nested loops** - Loops inside loops
✅ **Common patterns** - Accumulation, filtering, searching
✅ **Loop control** - break and continue (if supported)

## Next Steps

Now that you understand loops:

**[Actions (Functions) →](actions-functions.md)**
Learn how to organize reusable code.

Or explore related topics:
- [Lists and Collections →](lists-and-collections.md) - Learn more about lists to iterate
- [Control Flow →](control-flow.md) - Combine loops with conditionals
- [Variables and Types →](variables-and-types.md) - Review variable basics

---

**Previous:** [← Control Flow](control-flow.md) | **Next:** [Actions (Functions) →](actions-functions.md)
