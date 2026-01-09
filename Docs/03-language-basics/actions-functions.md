# Actions (Functions)

Actions are reusable blocks of code that perform specific tasks. In WFL, we call them "actions" because they describe what the code does.

## What are Actions?

Actions let you:
- **Organize code** - Group related operations together
- **Reuse code** - Write once, use many times
- **Simplify programs** - Break complex logic into manageable pieces
- **Improve readability** - Name chunks of code descriptively

Think of actions as recipes: once you define how to make something, you can follow that recipe anytime.

## Defining Actions

### Simple Action (No Parameters)

```wfl
define action called greet:
    display "Hello, World!"
    display "Welcome to WFL!"
end action
```

**Syntax:**
```wfl
define action called <name>:
    <statements>
end action
```

**Call it:**
```wfl
call greet
```

**Output:**
```
Hello, World!
Welcome to WFL!
```

### Action with Parameters

```wfl
define action called greet with parameters name:
    display "Hello, " with name with "!"
end action

call greet with "Alice"
call greet with "Bob"
```

**Output:**
```
Hello, Alice!
Hello, Bob!
```

**Syntax:**
```wfl
define action called <name> with parameters <param1>:
    <statements>
end action
```

### Multiple Parameters

```wfl
define action called introduce with parameters name and age:
    display "My name is " with name with " and I'm " with age with " years old."
end action

call introduce with "Alice" and 28
call introduce with "Bob" and 35
```

**Output:**
```
My name is Alice and I'm 28 years old.
My name is Bob and I'm 35 years old.
```

**Syntax:**
```wfl
define action called <name> with parameters <param1> and <param2> and <param3>:
    <statements>
end action
```

## Returning Values

Actions can return values using `return`:

```wfl
define action called add with parameters x and y:
    store sum as x plus y
    return sum
end action

store result as add with 10 and 20
display "Result: " with result
// Output: "Result: 30"
```

### Return Examples

**Calculate area:**
```wfl
define action called calculate area with parameters width and height:
    store area as width times height
    return area
end action

store room area as calculate area with 10 and 12
display "Room area: " with room area with " sq ft"
// Output: "Room area: 120 sq ft"
```

**Is even:**
```wfl
define action called is even with parameters number:
    store remainder as number modulo 2
    check if remainder is equal to 0:
        return yes
    otherwise:
        return no
    end check
end action

check if is even with 42:
    display "42 is even"
end check
// Output: "42 is even"
```

**Maximum of two numbers:**
```wfl
define action called max with parameters a and b:
    check if a is greater than b:
        return a
    otherwise:
        return b
    end check
end action

store largest as max with 10 and 20
display "Largest: " with largest
// Output: "Largest: 20"
```

## Calling Actions

### Without Parameters

```wfl
define action called say hello:
    display "Hello!"
end action

call say hello
```

### With Parameters

```wfl
define action called greet with parameters name:
    display "Hello, " with name
end action

call greet with "Alice"
```

### With Multiple Parameters

```wfl
define action called calculate total with parameters price and quantity:
    return price times quantity
end action

store total as calculate total with 19.99 and 3
display "Total: $" with total
```

### Using Return Values

```wfl
// Store the result
store result as add with 5 and 3

// Use directly in display
display "Sum: " with add with 10 and 20

// Use in conditionals
check if is even with 42:
    display "Even number"
end check

// Use in calculations
store double sum as add with 5 and 3 times 2
```

## Variable Scope

### Local Variables

Variables created inside actions are local to that action:

```wfl
define action called calculate:
    store local value as 100  // Local to this action
    display "Local: " with local value
end action

call calculate
// display local value  // ERROR: local value not defined outside action
```

### Parameters are Local

```wfl
define action called test with parameters x:
    display "Parameter x: " with x
    change x to x plus 10
    display "Changed x: " with x
end action

store y as 5
call test with y
display "Original y: " with y  // y is still 5 (not modified)
```

**Output:**
```
Parameter x: 5
Changed x: 15
Original y: 5
```

Parameters are **passed by value**, not by reference. Changes inside the action don't affect the original variable.

### Accessing Global Variables

Actions can access variables defined outside them:

```wfl
store global value as 100

define action called use global:
    display "Global value: " with global value
    change global value to 200
end action

call use global
display "Changed global value: " with global value
```

**Output:**
```
Global value: 100
Changed global value: 200
```

## Common Patterns

### Helper Actions

```wfl
define action called print separator:
    display "================================"
end action

display "Section 1"
call print separator
display "Section 2"
call print separator
```

### Validation

```wfl
define action called is valid email with parameters email:
    // Simplified validation
    check if contains of email and "@":
        check if contains of email and ".":
            return yes
        end check
    end check
    return no
end action

check if is valid email with "user@example.com":
    display "Valid email!"
end check
```

### Calculation

```wfl
define action called calculate discount with parameters price and discount percent:
    store discount amount as price times discount percent divided by 100
    store final price as price minus discount amount
    return final price
end action

store sale price as calculate discount with 100.00 and 20
display "Sale price: $" with sale price
// Output: "Sale price: $80.0"
```

### Formatting

```wfl
define action called format currency with parameters amount:
    return "$" with amount
end action

display format currency with 19.99
display format currency with 125.50
```

**Output:**
```
$19.99
$125.5
```

## Real-World Examples

### Temperature Converter

```wfl
define action called celsius to fahrenheit with parameters celsius:
    store fahrenheit as celsius times 9 divided by 5 plus 32
    return fahrenheit
end action

define action called fahrenheit to celsius with parameters fahrenheit:
    store celsius as fahrenheit minus 32 times 5 divided by 9
    return celsius
end action

store temp c as 25
store temp f as celsius to fahrenheit with temp c
display temp c with "°C = " with temp f with "°F"

store temp f2 as 77
store temp c2 as fahrenheit to celsius with temp f2
display temp f2 with "°F = " with temp c2 with "°C"
```

### Grade Calculator

```wfl
define action called calculate letter grade with parameters score:
    check if score is greater than or equal to 90:
        return "A"
    otherwise:
        check if score is greater than or equal to 80:
            return "B"
        otherwise:
            check if score is greater than or equal to 70:
                return "C"
            otherwise:
                check if score is greater than or equal to 60:
                    return "D"
                otherwise:
                    return "F"
                end check
            end check
        end check
    end check
end action

define action called calculate gpa with parameters letter:
    check if letter is "A":
        return 4.0
    otherwise:
        check if letter is "B":
            return 3.0
        otherwise:
            check if letter is "C":
                return 2.0
            otherwise:
                check if letter is "D":
                    return 1.0
                otherwise:
                    return 0.0
                end check
            end check
        end check
    end check
end action

store my score as 85
store my grade as calculate letter grade with my score
store my gpa as calculate gpa with my grade

display "Score: " with my score
display "Grade: " with my grade
display "GPA: " with my gpa
```

**Output:**
```
Score: 85
Grade: B
GPA: 3.0
```

### Fibonacci

```wfl
define action called fibonacci with parameters n:
    check if n is less than or equal to 1:
        return n
    otherwise:
        store fib1 as fibonacci with n minus 1
        store fib2 as fibonacci with n minus 2
        return fib1 plus fib2
    end check
end action

count from 0 to 10:
    store fib as fibonacci with count
    display "Fibonacci(" with count with ") = " with fib
end count
```

### Factorial

```wfl
define action called factorial with parameters n:
    check if n is less than or equal to 1:
        return 1
    otherwise:
        store prev as factorial with n minus 1
        return n times prev
    end check
end action

store result as factorial with 5
display "5! = " with result
// Output: "5! = 120"
```

### String Utilities

```wfl
define action called trim and uppercase with parameters text:
    store trimmed as trim of text
    store upper as touppercase of trimmed
    return upper
end action

define action called is empty with parameters text:
    store trimmed as trim of text
    check if length of trimmed is equal to 0:
        return yes
    otherwise:
        return no
    end check
end action

store input as "  hello world  "
display trim and uppercase with input
// Output: "HELLO WORLD"

check if is empty with "   ":
    display "Empty string!"
end check
// Output: "Empty string!"
```

## Actions in Loops

You can call actions inside loops:

```wfl
define action called square with parameters n:
    return n times n
end action

count from 1 to 5:
    store squared as square with count
    display count with " squared is " with squared
end count
```

**Output:**
```
1 squared is 1
2 squared is 4
3 squared is 9
4 squared is 16
5 squared is 25
```

## Actions with Lists

```wfl
define action called sum list with parameters numbers:
    store total as 0
    for each number in numbers:
        change total to total plus number
    end for
    return total
end action

create list values:
    add 10
    add 20
    add 30
end list

store sum as sum list with values
display "Sum: " with sum
// Output: "Sum: 60"
```

## Recursive Actions

Actions can call themselves (recursion):

```wfl
define action called countdown with parameters n:
    check if n is greater than 0:
        display n
        call countdown with n minus 1
    otherwise:
        display "Blast off!"
    end check
end action

call countdown with 5
```

**Output:**
```
5
4
3
2
1
Blast off!
```

**Warning:** Make sure recursion has a base case (stopping condition) or you'll get a stack overflow!

## Common Mistakes

### Forgetting `end action`

**Wrong:**
```wfl
define action called greet:
    display "Hello!"
// Missing end action!
```

**Right:**
```wfl
define action called greet:
    display "Hello!"
end action
```

### Wrong Parameter Syntax

**Wrong:**
```wfl
define action called greet (name):  // Wrong syntax
```

**Right:**
```wfl
define action called greet with parameters name:
```

### Forgetting `call`

**Wrong:**
```wfl
greet with "Alice"  // Missing 'call'
```

**Right:**
```wfl
call greet with "Alice"
```

### Returning Without Value

**Wrong:**
```wfl
define action called get value:
    return  // What to return?
end action
```

**Right:**
```wfl
define action called get value:
    return 42  // Return a specific value
end action
```

### Using Action Before Defining

**Wrong:**
```wfl
call greet  // ERROR: greet not defined yet

define action called greet:
    display "Hello!"
end action
```

**Right:**
```wfl
define action called greet:
    display "Hello!"
end action

call greet  // Define before calling
```

## Practice Exercises

### Exercise 1: Basic Action

Create an action called `print banner` that displays:
```
********************
*  Welcome to WFL  *
********************
```

Call it multiple times.

### Exercise 2: Simple Calculator

Create actions for:
- `add with a and b`
- `subtract with a and b`
- `multiply with a and b`
- `divide with a and b`

Test each action with different numbers.

### Exercise 3: Even/Odd Checker

Create an action called `is odd with number` that returns `yes` if the number is odd, `no` if even.

Test it with numbers 1-10.

### Exercise 4: Area Calculators

Create actions to calculate area:
- `rectangle area with width and height`
- `circle area with radius` (use π = 3.14159)
- `triangle area with base and height`

Display results for sample shapes.

### Exercise 5: Grade Processor

Create an action that takes a score and returns:
- The letter grade (A, B, C, D, F)
- Whether the student passed (70+)

Use it to process multiple test scores.

### Exercise 6: String Formatter

Create an action called `title case with text` that:
- Converts the first letter to uppercase
- Converts the rest to lowercase
- Returns the result

Hint: You'll need `touppercase`, `tolowercase`, and `substring`.

## Best Practices

✅ **Use descriptive action names:** `calculate total price` not `calc`

✅ **Keep actions focused:** One action, one responsibility

✅ **Use parameters:** Make actions reusable with different values

✅ **Return values:** Let actions produce results

✅ **Add comments:** Explain what complex actions do

✅ **Limit action length:** If an action is too long, break it into smaller actions

❌ **Don't make actions too large:** > 50 lines is usually too much

❌ **Don't use vague names:** `process`, `handle`, `do_stuff`

❌ **Don't modify global state unnecessarily:** Use parameters and returns

❌ **Don't forget to define before calling**

## Action Design Guidelines

### Single Responsibility

Each action should do one thing well:

**Good:**
```wfl
define action called calculate tax with parameters amount:
    return amount times 0.08
end action

define action called calculate total with parameters subtotal:
    store tax as calculate tax with subtotal
    return subtotal plus tax
end action
```

**Bad:**
```wfl
define action called do everything with parameters price and quantity:
    // Too much in one action!
    store subtotal as price times quantity
    store tax as subtotal times 0.08
    store total as subtotal plus tax
    display "Subtotal: " with subtotal
    display "Tax: " with tax
    display "Total: " with total
    // This does calculation AND display - should be separate!
end action
```

### Descriptive Names

Action names should describe what they do:

```wfl
// Good names:
calculate total price
validate email address
format as currency
is eligible for discount
generate invoice
send confirmation email

// Poor names:
calc          // Too cryptic
process       // Too vague
do stuff      // Meaningless
temp          // What does it do?
```

### Parameter Validation

Check parameters before using them:

```wfl
define action called divide with parameters numerator and denominator:
    check if denominator is equal to 0:
        display "Error: Cannot divide by zero"
        return nothing
    end check

    return numerator divided by denominator
end action

store result as divide with 10 and 0
// Output: "Error: Cannot divide by zero"
```

## Complex Example

Here's a complete program using multiple actions:

```wfl
// Shopping cart calculator

define action called calculate discount with parameters price and is member:
    check if is member is yes:
        return price times 0.9  // 10% member discount
    otherwise:
        return price
    end check
end action

define action called calculate tax with parameters amount:
    return amount times 0.08  // 8% tax
end action

define action called format price with parameters amount:
    return "$" with amount
end action

define action called process order with parameters item price and quantity and customer is member:
    // Calculate subtotal
    store subtotal as item price times quantity
    display "Subtotal: " with format price with subtotal

    // Apply discount
    store discounted as calculate discount with subtotal and customer is member
    check if customer is member is yes:
        store saved as subtotal minus discounted
        display "Member discount: -" with format price with saved
    end check

    // Calculate tax
    store tax as calculate tax with discounted
    display "Tax: " with format price with tax

    // Calculate total
    store total as discounted plus tax
    display "Total: " with format price with total

    return total
end action

// Main program
display "=== Order Summary ==="
display ""

store final total as process order with 19.99 and 3 and yes

display ""
display "Amount charged: " with format price with final total
```

**Output:**
```
=== Order Summary ===

Subtotal: $59.97
Member discount: -$5.997
Tax: $4.31784
Total: $58.29084

Amount charged: $58.29084
```

## Recursion

Actions can call themselves for recursive algorithms:

### Countdown

```wfl
define action called countdown with parameters n:
    check if n is greater than 0:
        display n
        call countdown with n minus 1
    otherwise:
        display "Done!"
    end check
end action

call countdown with 5
```

**Output:**
```
5
4
3
2
1
Done!
```

### Sum of Numbers

```wfl
define action called sum to n with parameters n:
    check if n is less than or equal to 0:
        return 0
    otherwise:
        return n plus sum to n with n minus 1
    end check
end action

store result as sum to n with 10
display "Sum of 1-10: " with result
// Output: "Sum of 1-10: 55"
```

### Power Function

```wfl
define action called power with parameters base and exponent:
    check if exponent is equal to 0:
        return 1
    otherwise:
        store prev power as power with base and exponent minus 1
        return base times prev power
    end check
end action

display "2^5 = " with power with 2 and 5
// Output: "2^5 = 32"
```

**Warning:** Recursive actions use stack space. Deep recursion (1000+ levels) can cause stack overflow.

## Actions in Collections

Store actions in variables (if supported) or create action-like patterns with containers.

## What You've Learned

In this section, you learned:

✅ **Defining actions** - `define action called name`
✅ **Parameters** - `with parameters x and y`
✅ **Calling actions** - `call action_name with arguments`
✅ **Returning values** - `return value`
✅ **Variable scope** - Local vs global variables
✅ **Recursion** - Actions calling themselves
✅ **Common patterns** - Validation, calculation, formatting
✅ **Best practices** - Single responsibility, descriptive names

## Next Steps

Now that you understand actions:

**[Lists and Collections →](lists-and-collections.md)**
Learn how to work with multiple values and pass lists to actions.

Or explore related topics:
- [Control Flow →](control-flow.md) - Use actions with conditionals
- [Loops and Iteration →](loops-and-iteration.md) - Call actions in loops
- [Error Handling →](error-handling.md) - Handle errors in actions

---

**Previous:** [← Loops and Iteration](loops-and-iteration.md) | **Next:** [Lists and Collections →](lists-and-collections.md)
