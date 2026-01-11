# Control Flow

Control flow lets your programs make decisions based on conditions. WFL uses natural language to express conditional logic clearly.

## Basic Conditionals

### Check If Statement

The `check if` statement executes code only when a condition is true:

```wfl
check if age is greater than or equal to 18:
    display "You are an adult"
end check
```

**Syntax:**
```wfl
check if <condition>:
    <statements>
end check
```

**Example:**
```wfl
store temperature as 75

check if temperature is greater than 70:
    display "It's warm outside!"
end check
```

### Otherwise (Else)

Use `otherwise` to execute code when the condition is false:

```wfl
check if age is greater than or equal to 18:
    display "You are an adult"
otherwise:
    display "You are a minor"
end check
```

**Syntax:**
```wfl
check if <condition>:
    <statements>
otherwise:
    <statements>
end check
```

**Example:**
```wfl
store is raining as yes

check if is raining is yes:
    display "Take an umbrella!"
otherwise:
    display "Enjoy the sunshine!"
end check
```

### Chained Conditionals (Else If)

You can chain multiple conditions by nesting `check if` blocks inside `otherwise` clauses:

```wfl
check if score is greater than or equal to 90:
    display "Grade: A"
otherwise:
    check if score is greater than or equal to 80:
        display "Grade: B"
    otherwise:
        check if score is greater than or equal to 70:
            display "Grade: C"
        otherwise:
            check if score is greater than or equal to 60:
                display "Grade: D"
            otherwise:
                display "Grade: F"
            end check
        end check
    end check
end check
```

**Syntax:**
```wfl
check if <condition1>:
    <statements>
otherwise:
    check if <condition2>:
        <statements>
    otherwise:
        check if <condition3>:
            <statements>
        otherwise:
            <statements>
        end check
    end check
end check
```

## Conditions

### Comparison Conditions

```wfl
// Equality
check if name is "Alice":
    display "Hello, Alice!"
end check

// Inequality
check if name is not "Bob":
    display "You're not Bob"
end check

// Greater than
check if age is greater than 21:
    display "Can drink alcohol"
end check

// Greater than or equal
check if score is greater than or equal to 70:
    display "Passed!"
end check

// Less than
check if temperature is less than 32:
    display "Freezing!"
end check

// Less than or equal
check if age is less than or equal to 12:
    display "Child ticket"
end check
```

### Logical Conditions

**AND - Both conditions must be true:**
```wfl
check if age is greater than or equal to 18 and has license is yes:
    display "Can drive"
end check
```

**OR - At least one condition must be true:**
```wfl
check if is weekend is yes or is holiday is yes:
    display "No work today!"
end check
```

**NOT - Negates a condition:**
```wfl
check if not is logged in:
    display "Please log in"
end check
```

**Combined:**
```wfl
check if age is greater than or equal to 18 and is citizen is yes or has permit is yes:
    display "Can vote"
end check
```

### Boolean Variables

```wfl
store is active as yes
store is verified as no

check if is active is yes:
    display "Account is active"
end check

check if is verified is yes:
    display "Account is verified"
otherwise:
    display "Please verify your account"
end check
```

## Nested Conditionals

You can nest conditionals inside other conditionals:

```wfl
check if has account is yes:
    check if is logged in is yes:
        display "Welcome back!"
    otherwise:
        display "Please log in"
    end check
otherwise:
    display "Please create an account"
end check
```

**Another example:**
```wfl
check if age is greater than or equal to 18:
    check if has license is yes:
        check if has insurance is yes:
            display "Ready to drive!"
        otherwise:
            display "Need insurance"
        end check
    otherwise:
        display "Need driver's license"
    end check
otherwise:
    display "Too young to drive"
end check
```

**Best practice:** Avoid deep nesting (more than 2-3 levels). Break into separate checks or actions instead.

## Common Patterns

### Range Checking

Check if a value is within a range:

```wfl
store age as 15

check if age is greater than or equal to 13 and age is less than 20:
    display "Teenager"
otherwise:
    check if age is less than 13:
        display "Child"
    otherwise:
        display "Adult"
    end check
end check
```

### Multiple Equality Checks

```wfl
store day as "Monday"

check if day is "Saturday" or day is "Sunday":
    display "Weekend!"
otherwise:
    display "Weekday"
end check
```

### Validation

```wfl
store username as "alice123"
store password as "secret"

check if username is not "":
    check if password is not "":
        display "Processing login..."
    otherwise:
        display "Password required"
    end check
otherwise:
    display "Username required"
end check
```

### Eligibility Check

```wfl
store age as 25
store income as 50000
store has job as yes

check if age is greater than or equal to 18 and has job is yes and income is greater than 30000:
    display "Loan approved"
otherwise:
    display "Loan denied"
end check
```

### Status Determination

```wfl
store temperature as 25

check if temperature is above 30:
    display "Hot"
otherwise:
    check if temperature is above 20:
        display "Warm"
    otherwise:
        check if temperature is above 10:
            display "Cool"
        otherwise:
            display "Cold"
        end check
    end check
end check
```

## Examples

### Age Category

```wfl
store age as 35

check if age is greater than or equal to 65:
    display "Senior citizen - discount available"
otherwise:
    check if age is greater than or equal to 18:
        display "Adult - regular price"
    otherwise:
        check if age is greater than or equal to 13:
            display "Teenager - youth price"
        otherwise:
            display "Child - free entry"
        end check
    end check
end check
```

### Login System

```wfl
store username as "alice"
store password as "password123"
store correct username as "alice"
store correct password as "secret"

check if username is equal to correct username:
    check if password is equal to correct password:
        display "Login successful!"
    otherwise:
        display "Incorrect password"
    end check
otherwise:
    display "Username not found"
end check
```

### Grade Calculator

```wfl
store score as 85

check if score is greater than or equal to 90:
    display "Excellent! Grade: A"
otherwise:
    check if score is greater than or equal to 80:
        display "Great! Grade: B"
    otherwise:
        check if score is greater than or equal to 70:
            display "Good! Grade: C"
        otherwise:
            check if score is greater than or equal to 60:
                display "Pass. Grade: D"
            otherwise:
                display "Failed. Grade: F"
            end check
        end check
    end check
end check
```

### Shipping Calculator

```wfl
store total as 75.00
store is member as yes
store shipping as 0

check if total is greater than or equal to 100:
    store shipping as 0
    display "Free shipping!"
otherwise:
    check if is member is yes:
        store shipping as 5.00
        display "Member shipping: $" with shipping
    otherwise:
        store shipping as 10.00
        display "Standard shipping: $" with shipping
    end check
end check

store final total as total plus shipping
display "Total with shipping: $" with final total
```

### BMI Calculator

```wfl
store weight as 70  // kg
store height as 1.75  // meters
store bmi as weight divided by height divided by height

check if bmi is less than 18.5:
    display "Underweight (BMI: " with bmi with ")"
otherwise:
    check if bmi is less than 25:
        display "Normal weight (BMI: " with bmi with ")"
    otherwise:
        check if bmi is less than 30:
            display "Overweight (BMI: " with bmi with ")"
        otherwise:
            display "Obese (BMI: " with bmi with ")"
        end check
    end check
end check
```

### Season Detector

```wfl
store month as 7  // July

check if month is greater than or equal to 3 and month is less than or equal to 5:
    display "Spring"
otherwise:
    check if month is greater than or equal to 6 and month is less than or equal to 8:
        display "Summer"
    otherwise:
        check if month is greater than or equal to 9 and month is less than or equal to 11:
            display "Fall"
        otherwise:
            display "Winter"
        end check
    end check
end check
```

### Access Control

```wfl
store role as "admin"
store is active as yes
store is verified as yes

check if is active is no:
    display "Account is disabled"
otherwise:
    check if is verified is no:
        display "Please verify your account"
    otherwise:
        check if role is "admin":
            display "Full access granted"
        otherwise:
            check if role is "moderator":
                display "Moderator access granted"
            otherwise:
                check if role is "user":
                    display "User access granted"
                otherwise:
                    display "Unknown role"
                end check
            end check
        end check
    end check
end check
```

## Short-Circuit Evaluation

WFL evaluates conditions from left to right and stops as soon as the result is determined:

**AND (stops on first false):**
```wfl
check if expensive operation() and another check():
    // another check() only runs if expensive operation() is true
    display "Both true"
end check
```

**OR (stops on first true):**
```wfl
check if quick check() or expensive operation():
    // expensive operation() only runs if quick check() is false
    display "At least one true"
end check
```

This is automatic and helps performance!

## Common Mistakes

### Forgetting `end check`

**Wrong:**
```wfl
check if age is 18:
    display "You're 18"
// Missing end check!
```

**Right:**
```wfl
check if age is 18:
    display "You're 18"
end check
```

### Using Assignment Instead of Comparison

**Wrong:**
```wfl
check if age as 18:  // Wrong! This is assignment
    display "Age is 18"
end check
```

**Right:**
```wfl
check if age is 18:  // Correct! This is comparison
    display "Age is 18"
end check
```

### Confusing AND/OR Logic

**Wrong assumption:**
```wfl
// This checks if name is "Alice" OR if name is "Bob"
check if name is "Alice" or "Bob":  // This won't work!
```

**Right:**
```wfl
check if name is "Alice" or name is "Bob":
    display "Hello, Alice or Bob!"
end check
```

### Unreachable Conditions

**Wrong:**
```wfl
check if score is greater than 70:
    display "Pass"
otherwise:
    check if score is greater than 60:  // This is good
        display "Barely pass"
    otherwise:
        check if score is greater than 80:  // UNREACHABLE! 80 > 70
            display "Great"
        end check
    end check
end check
```

**Right (order from highest to lowest):**
```wfl
check if score is greater than or equal to 80:
    display "Great"
otherwise:
    check if score is greater than or equal to 70:
        display "Pass"
    otherwise:
        check if score is greater than or equal to 60:
            display "Barely pass"
        end check
    end check
end check
```

### Too Much Nesting

**Problematic:**
```wfl
check if a is yes:
    check if b is yes:
        check if c is yes:
            check if d is yes:
                check if e is yes:
                    display "Too deep!"
                end check
            end check
        end check
    end check
end check
```

**Better:**
```wfl
check if a is yes and b is yes and c is yes and d is yes and e is yes:
    display "All conditions met!"
end check
```

## Main Loop

The `main loop` provides an infinite loop for long-running processes like servers. Unlike other loops, it continues indefinitely until explicitly broken.

### Basic Main Loop

```wfl
store iteration as 0

main loop:
    add 1 to iteration
    display "Iteration: " with iteration

    check if iteration is greater than or equal to 10:
        break
    end check
end loop
```

**Key Features:**
- Runs indefinitely until `break` is executed
- Useful for servers, daemons, and continuous processes
- Can be combined with error handling

### Main Loop with Error Handling

You can wrap main loops in `try/catch` blocks for robust error handling:

```wfl
try:
    main loop:
        display "Processing..."

        // Your code here

        break  // Exit when done
    end loop
catch:
    display "Error occurred: " with error
end try
```

### Web Server Example

Main loops are commonly used for web servers:

```wfl
listen on port 8080 as server
display "Server running on http://localhost:8080"

try:
    main loop:
        wait for request comes in on server as req

        check if path is equal to "/":
            respond to req with "Hello, World!" and content_type "text/plain"
        otherwise:
            respond to req with "Not found" and status 404
        end check

        // Optional: limit to specific number of requests
        check if request_count is greater than 100:
            break
        end check
    end loop
catch:
    display "Server error: " with error
end try

display "Server stopped"
```

**Best Practices:**
- Always include a `break` condition to prevent truly infinite loops
- Use `try/catch` for production servers to handle unexpected errors gracefully
- Consider request counters or time limits for testing

## Practice Exercises

### Exercise 1: Temperature Advisor

Write a program that:
- Stores a temperature value
- Displays advice based on temperature:
  - Below 32°F: "Freezing - bundle up!"
  - 32-60°F: "Cold - wear a jacket"
  - 60-75°F: "Pleasant - enjoy!"
  - 75-90°F: "Warm - stay hydrated"
  - Above 90°F: "Hot - seek shade!"

### Exercise 2: Ticket Pricing

Create a ticket pricing system:
- Age < 3: Free
- Age 3-12: $5
- Age 13-64: $15
- Age 65+: $10 (senior discount)

Display the ticket price for a given age.

### Exercise 3: Login Validator

Create a login validation system that checks:
1. Username is not empty
2. Password is not empty
3. Password is at least 8 characters (you'll need to estimate length)
4. Display appropriate messages for each validation failure

### Exercise 4: Leap Year Checker

Check if a year is a leap year:
- Divisible by 4 AND (not divisible by 100 OR divisible by 400)

Hint: Use modulo operator.

### Exercise 5: Grade with Comments

Extend the grade calculator to include comments:
- A (90+): "Excellent work!"
- B (80-89): "Good job!"
- C (70-79): "Satisfactory"
- D (60-69): "Needs improvement"
- F (below 60): "Please see instructor"

## Best Practices

✅ **Use descriptive conditions:** `age is greater than 18` is clearer than symbols

✅ **Order conditions logically:** Most specific first, most general last

✅ **Avoid deep nesting:** Use logical operators or extract to actions

✅ **Always include `end check`:** Complete every conditional block

✅ **Consider all cases:** Use `otherwise` for the default case

✅ **Test edge cases:** Test with boundary values (e.g., exactly 18, not just 17 or 19)

❌ **Don't repeat conditions:** Use nested `otherwise: check if` blocks for multiple conditions

❌ **Don't make unreachable conditions:** Order matters!

❌ **Don't nest too deeply:** More than 3 levels is hard to read

## What You've Learned

In this section, you learned:

✅ **Basic conditionals** - `check if`, `otherwise`, `end check`
✅ **Multiple conditions** - Nested `otherwise: check if` blocks
✅ **Logical operators** - `and`, `or`, `not`
✅ **Nested conditionals** - Conditionals inside conditionals
✅ **Common patterns** - Range checking, validation, status determination
✅ **Short-circuit evaluation** - Automatic optimization
✅ **Best practices** - Clear, maintainable conditional code

## Next Steps

Now that you understand control flow:

**[Loops and Iteration →](loops-and-iteration.md)**
Learn how to repeat actions with loops.

Or explore related topics:
- [Operators and Expressions →](operators-and-expressions.md) - Review comparison operators
- [Actions (Functions) →](actions-functions.md) - Use conditionals in functions
- [Error Handling →](error-handling.md) - Handle errors with conditions

---

**Previous:** [← Operators and Expressions](operators-and-expressions.md) | **Next:** [Loops and Iteration →](loops-and-iteration.md)
