# Variables and Types

Variables store data that your program can use and manipulate. In WFL, variables are simple, clear, and type-safe.

## Creating Variables

Use the `store` keyword to create variables:

```wfl
store name as "Alice"
store age as 25
store is active as yes
store account balance as 1250.75
```

**Syntax:**
```wfl
store <variable name> as <value>
```

The variable name can include spaces for readability:

```wfl
store user name as "Bob"           // Variable name: "user name"
store account balance as 1000.00   // Variable name: "account balance"
store is admin as no               // Variable name: "is admin"
```

## Data Types

WFL has several built-in types, automatically inferred from the value you assign.

### Text (Strings)

Text values are enclosed in double quotes:

```wfl
store first name as "Alice"
store last name as "Smith"
store greeting as "Hello, World!"
store empty text as ""
```

**Special characters:**
```wfl
store message as "Hello\nWorld"        // \n = newline
store path as "C:\\Users\\Alice"       // \\ = backslash
store quote as "She said \"Hi!\""      // \" = quote inside text
```

### Number

Numbers don't need quotes. WFL handles both integers and decimals:

```wfl
store age as 25                    // Integer
store pi as 3.14159               // Decimal
store temperature as -5.5          // Negative
store big number as 1000000       // No comma separators
```

**Math with numbers:**
```wfl
store x as 10
store y as 20
store sum as x plus y              // 30
store product as x times y         // 200
```

### Boolean

Boolean values represent true/false:

```wfl
store is active as yes             // true
store is locked as no              // false
store has access as yes
store is verified as no
```

**Alternative forms:**
```wfl
store flag as true                 // Also valid
store another flag as false        // Also valid
```

**Common in conditionals:**
```wfl
check if is active is yes:
    display "User is active"
end check
```

### Nothing (Null)

Represents the absence of a value:

```wfl
store result as nothing
store undefined value as nothing
```

**Check for nothing:**
```wfl
check if isnothing of result:
    display "Result is empty"
end check
```

### List

Collections of values (covered in detail in [Lists and Collections](lists-and-collections.md)):

```wfl
store numbers as [1, 2, 3, 4, 5]
store names as ["Alice", "Bob", "Carol"]
```

### Container (Object)

User-defined types (covered in [Advanced Features](../04-advanced-features/containers-oop.md)):

```wfl
create container Person:
    property name as text
    property age as number
end container
```

## Type Inference

WFL automatically determines the type based on the value:

```wfl
store x as 42                      // WFL knows: Number
store name as "Alice"              // WFL knows: Text
store active as yes                // WFL knows: Boolean
store items as [1, 2, 3]           // WFL knows: List
```

**You don't need to declare types explicitly.** WFL figures it out.

## Checking Types

Use `typeof` to check a variable's type:

```wfl
store age as 25
store name as "Alice"
store active as yes
store value as nothing

display typeof of age              // Output: "Number"
display typeof of name             // Output: "Text"
display typeof of active           // Output: "Boolean"
display typeof of value            // Output: "Nothing"
```

**Practical use:**
```wfl
check if typeof of value is "Number":
    display "It's a number!"
otherwise:
    display "Not a number"
end check
```

## Changing Variables

Use `change` to modify an existing variable:

```wfl
store counter as 0
display counter                    // Output: 0

change counter to 5
display counter                    // Output: 5

change counter to 10
display counter                    // Output: 10
```

**With expressions:**
```wfl
store x as 10
change x to x plus 5               // x is now 15
change x to x times 2              // x is now 30
```

## Type Safety

WFL prevents type mismatches at compile time:

```wfl
store age as 25
store name as "Alice"

// This will cause an error:
// display age plus name
// ERROR: Cannot add Number and Text
```

**WFL catches these errors before your code runs!**

**Correct way:**
```wfl
// To combine number and text:
display "Age: " with age                    // Output: "Age: 25"
display name with " is " with age          // Output: "Alice is 25"
```

## Variable Scope

Variables are accessible from where they're declared onward in the file.

### Global Scope

Variables declared at the top level are accessible everywhere:

```wfl
store global value as 100

action use global:
    display global value           // Can access global value
end action

call use global                    // Output: 100
```

### Local Scope (Actions)

Variables inside actions are local to that action:

```wfl
action calculate:
    store local value as 50        // Local to this action
    display local value
end action

call calculate                     // Output: 50
// display local value             // ERROR: local value not defined
```

### Block Scope (Loops and Conditionals)

Variables inside blocks have their own scope:

```wfl
count from 1 to 5:
    store loop value as the current count
    display loop value
end count
// display loop value              // ERROR: Not accessible outside loop
```

## Constants

Currently, all variables can be changed. True constants (immutable values) are planned for future versions.

**Best practice:** Use uppercase names for values you don't intend to change:

```wfl
store MAX_USERS as 100
store PI as 3.14159
store APP_NAME as "My Application"

// Don't change these (convention, not enforced)
```

## Naming Conventions

### Recommended Style

**Use descriptive names with spaces:**
```wfl
store user age as 25               // Good: Clear what it represents
store customer balance as 1000.00  // Good: Very descriptive
```

**Or snake_case (single words):**
```wfl
store user_age as 25
store customer_balance as 1000.00
```

**Avoid single letters (except in math):**
```wfl
store x as 5                       // OK for math
store total_sum as 0               // Better for real code
```

### Valid Variable Names

Variable names can:
- ✅ Contain letters: `name`, `user data`
- ✅ Contain numbers: `value1`, `item 2`
- ✅ Contain underscores: `user_name`, `total_count`
- ✅ Contain spaces: `user name`, `total count`

Variable names cannot:
- ❌ Start with a number: `1value` (invalid)
- ❌ Contain special characters: `user@email` (invalid)
- ❌ Be a keyword: `store`, `check`, `if` (reserved)

### Reserved Keywords

WFL reserves certain words for the language syntax. You **cannot** use these as variable names:

**Control Flow Keywords:**
- `check`, `if`, `otherwise`, `end`
- `for`, `each`, `in`, `count`, `from`, `to`, `by`
- `repeat`, `while`, `until`, `forever`
- `break`, `skip`, `continue`, `exit`, `loop`

**Declaration Keywords:**
- `store`, `create`, `change`, `define`
- `action`, `called`, `with`, `parameters`, `needs`
- `container`, `property`, `extends`, `implements`
- `list`, `map`, `pattern`

**Operation Keywords:**
- `display`, `return`, `call`
- `add`, `push`, `pop`, `remove`
- `open`, `close`, `read`, `write`, `file`
- `wait`, `execute`, `spawn`, `kill`

**Comparison & Logic Keywords:**
- `is`, `not`, `and`, `or`
- `equal`, `greater`, `less`, `than`
- `as`, `of`, `at`, `on`

**Other Reserved Words:**
- `yes`, `no`, `true`, `false`
- `nothing`, `missing`, `undefined`
- `try`, `catch`, `when`, `error`, `finally`
- `current`, `time`, `date`, `event`

**Examples of Conflicts:**

**Wrong:**
```wfl
store is as 10             // ❌ 'is' is a keyword
store file as "data.txt"   // ❌ 'file' is a keyword
store add as 5             // ❌ 'add' is a keyword
store current as 100       // ❌ 'current' is a keyword
```

**Right:**
```wfl
store is_value as 10       // ✅ Use underscore
store filename as "data.txt"  // ✅ Different name
store addition as 5        // ✅ Different name
store current_value as 100 // ✅ Add suffix
```

**Best Practice:** When in doubt, use underscores or descriptive suffixes:
- `is_active` instead of trying to use just `is`
- `file_handle` instead of `file`
- `add_result` instead of `add`

## Type Conversion

### Number to Text

Use `with` to combine:

```wfl
store age as 25
store message as "Age: " with age
display message                    // Output: "Age: 25"
```

### Text to Number

Not yet built-in, but you can parse manually in future versions.

## Common Patterns

### Swapping Values

```wfl
store a as 10
store b as 20

store temp as a
change a to b
change b to temp

display a                          // Output: 20
display b                          // Output: 10
```

### Accumulation

```wfl
store total as 0

store price1 as 10.50
store price2 as 25.00
store price3 as 5.75

change total to total plus price1
change total to total plus price2
change total to total plus price3

display "Total: $" with total     // Output: "Total: $41.25"
```

### Counters

```wfl
store counter as 0

change counter to counter plus 1
display counter                    // Output: 1

change counter to counter plus 1
display counter                    // Output: 2
```

### Flags

```wfl
store found as no

check if value is 42:
    change found to yes
end check

check if found is yes:
    display "Found the answer!"
end check
```

## Examples

### Personal Information

```wfl
store first name as "Alice"
store last name as "Johnson"
store age as 28
store is employed as yes
store salary as 75000.00

display "Name: " with first name with " " with last name
display "Age: " with age
display "Employed: " with is employed
display "Salary: $" with salary
```

**Output:**
```
Name: Alice Johnson
Age: 28
Employed: yes
Salary: $75000.0
```

### Temperature Converter

```wfl
store celsius as 25
store fahrenheit as celsius times 9 divided by 5 plus 32

display celsius with "°C = " with fahrenheit with "°F"
// Output: 25°C = 77°F
```

### Shopping Cart

```wfl
store item price as 19.99
store quantity as 3
store tax rate as 0.08

store subtotal as item price times quantity
store tax as subtotal times tax rate
store total as subtotal plus tax

display "Subtotal: $" with subtotal
display "Tax: $" with tax
display "Total: $" with total
```

**Output:**
```
Subtotal: $59.97
Tax: $4.7976
Total: $64.7676
```

## Type Checking Example

```wfl
store value1 as 42
store value2 as "hello"
store value3 as yes

display "Type of value1: " with typeof of value1
display "Type of value2: " with typeof of value2
display "Type of value3: " with typeof of value3

check if typeof of value1 is "Number":
    display "value1 is a number, we can do math!"
    display "value1 times 2 = " with value1 times 2
end check

check if typeof of value2 is "Text":
    display "value2 is text, we can display it!"
    display "Message: " with value2
end check
```

**Output:**
```
Type of value1: Number
Type of value2: Text
Type of value3: Boolean
value1 is a number, we can do math!
value1 times 2 = 84
value2 is text, we can display it!
Message: hello
```

## Common Mistakes

### Forgetting Quotes Around Text

**Wrong:**
```wfl
store name as Alice                // ERROR: Alice is not defined
```

**Right:**
```wfl
store name as "Alice"              // Text needs quotes
```

### Trying to Add Different Types

**Wrong:**
```wfl
store age as 25
store name as "Alice"
display age plus name              // ERROR: Cannot add Number and Text
```

**Right:**
```wfl
display "Name: " with name with ", Age: " with age
```

### Using Undefined Variables

**Wrong:**
```wfl
display user name                  // ERROR: user name not defined
```

**Right:**
```wfl
store user name as "Alice"
display user name                  // OK
```

### Changing vs. Storing

**Wrong:**
```wfl
change x to 10                     // ERROR: x doesn't exist yet
```

**Right:**
```wfl
store x as 0                       // Create it first
change x to 10                     // Then change it
```

## Practice Exercises

### Exercise 1: Basic Variables

Create variables for:
- Your name (text)
- Your age (number)
- Whether you like programming (boolean)

Display them all.

### Exercise 2: Temperature Converter

Create a program that:
1. Stores a Celsius temperature
2. Converts it to Fahrenheit (F = C × 9/5 + 32)
3. Displays both temperatures

### Exercise 3: Type Explorer

Create several variables with different types, then:
1. Use `typeof` to check each one
2. Display the type
3. Try changing one variable's type

### Exercise 4: Shopping Calculator

Create variables for:
- Item price: $15.99
- Quantity: 4
- Tax rate: 7% (0.07)

Calculate and display:
- Subtotal
- Tax amount
- Final total

### Exercise 5: Variable Swapping

Create two variables with different values. Swap their values and display them before and after.

## Best Practices

✅ **Use descriptive names:** `customer age` not `ca`

✅ **Use `with` to combine text and numbers:** Not `plus`

✅ **Check types when uncertain:** Use `typeof`

✅ **Initialize before using:** Create variables before referencing them

✅ **Use appropriate types:** Numbers for math, text for display

❌ **Don't use confusing names:** `x`, `tmp`, `data` (unless obvious)

❌ **Don't mix types incorrectly:** Can't add number and text

❌ **Don't use reserved keywords:** `store`, `check`, `if`, etc.

## What You've Learned

In this section, you learned:

✅ **How to create variables** - `store name as value`
✅ **All data types** - Text, Number, Boolean, Nothing, List
✅ **Type inference** - WFL determines types automatically
✅ **Type checking** - `typeof of variable`
✅ **Changing variables** - `change name to new value`
✅ **Type safety** - WFL prevents type mismatches
✅ **Naming conventions** - Clear, descriptive names

## Next Steps

Now that you understand variables and types:

**[Operators and Expressions →](operators-and-expressions.md)**
Learn how to combine values and perform operations.

Or explore related topics:
- [Lists and Collections →](lists-and-collections.md) - Working with multiple values
- [Control Flow →](control-flow.md) - Using variables in decisions
- [Actions (Functions) →](actions-functions.md) - Passing variables as parameters

---

**Previous:** [← Language Basics](index.md) | **Next:** [Operators and Expressions →](operators-and-expressions.md)
