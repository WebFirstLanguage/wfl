# WFL Functions (Actions): Reusing Code with Natural Syntax

**Summary:** In this section, we'll explore **functions** (called **actions** in WFL) – powerful constructs that let you package code into reusable, named blocks. Actions are essential for organizing your code, avoiding repetition, and breaking complex problems into manageable pieces. WFL makes defining and using actions intuitive with natural language syntax that clearly expresses your intent. By the end of this guide, you'll understand how to create actions with parameters, return values from actions, work with asynchronous operations, handle errors gracefully, and organize your code effectively – all using WFL's friendly, English-like approach.

## What Are Functions (Actions)?

An **action** in WFL (called a function in many other programming languages) is a named block of code that performs a specific task. Think of it as a mini-program within your program that you can call whenever you need to perform that task. Actions help you organize your code, avoid repetition, and make your programs more modular and maintainable.

**Why use actions?** Imagine you need to calculate the average of a list of numbers in multiple places in your program. Instead of writing the same calculation code over and over, you can define an action called `calculate average` once and then call it whenever you need that functionality. This makes your code more concise, easier to understand, and simpler to maintain.

## Creating Basic Actions

In WFL, you define an action using the `define action` syntax. Here's a simple example:

```wfl
define action say hello:
    display "Hello, World!"
end action
```

This creates an action called `say hello` that displays the message "Hello, World!" when called. The syntax is straightforward: `define action [name]:` followed by the code to execute, and ending with `end action`.

To call (or execute) this action, you simply use its name:

```wfl
say hello  // Displays: Hello, World!
```

Notice how natural this reads – it's like giving a command in plain English.

## Actions with Parameters

Most actions need to work with different values each time they're called. **Parameters** allow you to pass information into an action. In WFL, parameters can be defined using either the `with` or `needs` keyword:

```wfl
define action say hello with name:
    display "Hello, " with name with "!"
end action
```

Now our action takes a parameter called `name`. When we call the action, we provide a value for this parameter:

```wfl
say hello with "Alice"  // Displays: Hello, Alice!
say hello with "Bob"    // Displays: Hello, Bob!
```

### Parameter Definition Syntaxes

WFL supports two different syntaxes for defining parameters, each with different binding behaviors:

#### 1. "and"-separated Parameters

You can define multiple parameters by separating them with `and`:

```wfl
define action calculate rectangle area with width and height:
    store area as width times height
    display "The area is " with area
end action

calculate rectangle area with 5 and 3  // Displays: The area is 15
```

With this syntax:
- Each parameter is treated as a separate parameter requiring its own argument
- When calling the action, you must provide an argument for each parameter
- If you provide fewer arguments than parameters, it will produce an error

The syntax for calling actions with "and"-separated parameters is very natural: `action name with [value1] and [value2] and [value3]...`. It reads almost like a sentence.

#### 2. Space-separated Parameters

WFL also supports a more natural language-like syntax where parameters are separated by spaces:

```wfl
define action called test_assertion needs label expected actual:
    display "Testing: " with label
    if expected equals actual:
        display "✓ Test passed"
    else:
        display "✗ Test failed: expected " with expected with " but got " with actual
    end if
end action
```

With this syntax:
- All parameters are bound to the same argument when called with a single argument
- This allows for more natural language-like parameter names
- It's particularly useful for testing frameworks and assertion functions

For example, you can call the above action with either a single argument or multiple arguments:

```wfl
// Call with a single argument - all parameters receive the same value
test_assertion with "This is a single argument"

// Call with multiple arguments - each parameter gets its own value
test_assertion with "String length test" and 5 and length of "hello"
```

This flexibility allows you to create more expressive and readable code that matches your specific needs.

### Optional Parameters with Default Values

Sometimes you want parameters to be optional, with sensible defaults if not provided. WFL supports this with the `default` keyword, which works with both parameter syntaxes:

```wfl
define action greet user with name and greeting default "Hello":
    display greeting with ", " with name with "!"
end action

greet user with "Alice" and "Hi"      // Displays: Hi, Alice!
greet user with "Bob"                 // Displays: Hello, Bob!
```

In this example, `greeting` is an optional parameter with a default value of "Hello". If not provided when calling the action, the default value is used.

### Choosing the Right Parameter Syntax

When deciding which parameter syntax to use, consider:

1. **"and"-separated parameters** are best when:
   - Each parameter represents a distinct input that should have its own value
   - You want to enforce that each parameter receives its own argument
   - The parameters represent different concepts or data types

2. **Space-separated parameters** are best when:
   - You want to create a natural language-like interface
   - The parameters form a logical group that might be passed as a single value
   - You're creating testing or assertion functions where all parameters might receive the same value
   - You want flexibility in how the action can be called

## Returning Values from Actions

Actions can also produce (or return) values that can be used elsewhere in your program. In WFL, you use the `provide` or `return` keyword to specify what value an action should return:

```wfl
define action calculate average of numbers:
    store sum as 0
    for each number in numbers:
        add number to sum
    end for
    
    store average as sum divided by length of numbers
    provide average
end action
```

This action calculates the average of a list of numbers and returns the result. To use the returned value, you can store it in a variable:

```wfl
create list test scores:
    add 85
    add 90
    add 78
    add 92
end list

store average score as calculate average of test scores
display "The average score is " with average score  // Displays: The average score is 86.25
```

The syntax for using a returned value is very natural: you can use the action call directly in expressions or assignments, just as you would use any other value.

## Asynchronous Actions

WFL supports asynchronous programming, which is essential for operations that might take time to complete, such as network requests, file operations, or database queries. Asynchronous actions are defined using the `async` keyword:

```wfl
define async action fetch weather for city:
    display "Fetching weather data for " with city with "..."
    wait for 2 seconds  // Simulating a network request
    provide "Sunny, 25°C"
end action
```

When calling an asynchronous action, you typically use the `wait for` syntax to wait for it to complete:

```wfl
display "Starting weather check..."
store weather as wait for fetch weather for "New York"
display "Weather in New York: " with weather
display "Weather check complete."
```

This would display:
```
Starting weather check...
Fetching weather data for New York...
Weather in New York: Sunny, 25°C
Weather check complete.
```

The `wait for` ensures that the program waits for the asynchronous action to complete before continuing. This is important for operations where you need the result before proceeding.

### Parallel Asynchronous Operations

One of the powerful features of WFL's asynchronous programming model is the ability to run multiple operations in parallel using the `wait for ... and ...` syntax:

```wfl
display "Fetching weather data..."
wait for:
    store ny weather as fetch weather for "New York"
    and store la weather as fetch weather for "Los Angeles"
end wait

display "New York: " with ny weather
display "Los Angeles: " with la weather
```

This runs both weather fetches concurrently, potentially saving time compared to running them sequentially. The program continues only after both operations have completed.

## Error Handling in Actions

Actions might encounter errors during execution. WFL provides a robust way to handle these errors using the `try-when-otherwise` pattern:

```wfl
define action read user file with filename:
    try:
        open file filename
        read content from file
        close file
        provide content
    when file not found:
        display "Error: File " with filename with " not found."
        provide nothing
    when permission denied:
        display "Error: No permission to read " with filename
        provide nothing
    otherwise:
        display "An unexpected error occurred while reading " with filename
        provide nothing
    end try
end action
```

This action attempts to read a file and handles different types of errors that might occur. The caller can check the returned value to determine if the operation was successful:

```wfl
store user data as read user file with "user.txt"
if user data is nothing:
    display "Failed to read user data."
else:
    display "User data: " with user data
end if
```

## Organizing Code with Actions

Actions are powerful tools for organizing your code. Here are some common patterns:

### Helper Actions

You can create small, focused actions that perform specific tasks and then use them in larger actions:

```wfl
define action validate email with email:
    check if:
        email contains "@"
        and email contains "." after "@"
        and email does not contain spaces
    end check
    
    provide result of check
end action

define action register user with name and email:
    if validate email with email:
        display "Registering user " with name with " with email " with email
        // Registration code here
    else:
        display "Invalid email address: " with email
    end if
end action
```

This approach makes your code more modular and easier to maintain. The `validate email` action can be reused wherever email validation is needed.

### Action Libraries

As your program grows, you might want to organize related actions into libraries or modules:

```wfl
create module math utilities:
    define action calculate average of numbers:
        // Implementation here
    end action
    
    define action find maximum in numbers:
        // Implementation here
    end action
    
    define action find minimum in numbers:
        // Implementation here
    end action
end module
```

This groups related actions together, making your code more organized and easier to navigate.

## Variable Scope in Actions

Variables defined inside an action are **local** to that action – they exist only while the action is running and can't be accessed from outside. This is important for preventing unintended interactions between different parts of your program.

```wfl
define action calculate square of number:
    store result as number times number  // 'result' is local to this action
    provide result
end action

calculate square of 5
display result  // Error: 'result' is not defined here
```

If you need to access variables from the surrounding context, you can do so directly – actions can "see" variables defined in their outer scope:

```wfl
store base value as 10

define action add to base with amount:
    store new value as base value plus amount  // Can access 'base value'
    provide new value
end action

store result as add to base with 5  // result = 15
```

This behavior allows actions to work with their surrounding context while still maintaining clean boundaries.

## Recursive Actions

Actions in WFL can call themselves, a technique known as **recursion**. This is useful for solving problems that can be broken down into smaller instances of the same problem:

```wfl
define action calculate factorial of n:
    if n is less than or equal to 1:
        provide 1
    else:
        store smaller factorial as calculate factorial of (n minus 1)
        provide n times smaller factorial
    end if
end action
```

This action calculates the factorial of a number by recursively calculating the factorial of smaller numbers. For example, `calculate factorial of 5` would compute 5 × 4 × 3 × 2 × 1 = 120.

## Best Practices for Actions

To write effective and maintainable actions, consider these best practices:

1. **Give actions clear, descriptive names** that indicate what they do. For example, `calculate average` is better than `calc avg`.

2. **Keep actions focused on a single task**. If an action is doing too many things, consider breaking it into smaller, more focused actions.

3. **Use parameters to make actions flexible**. Instead of hardcoding values, pass them as parameters so the action can work with different inputs.

4. **Handle errors gracefully**. Use try-when-otherwise to catch and handle potential errors, providing meaningful feedback to the caller.

5. **Document complex actions** with comments that explain what the action does, what parameters it expects, and what it returns.

6. **Avoid side effects** when possible. Actions that modify global state can make your program harder to understand and debug.

## Common Action Patterns

Here are some common patterns you might use with actions:

### Data Transformation

Actions are great for transforming data from one form to another:

```wfl
define action format name with first and last and middle default nothing:
    if middle is nothing:
        provide first with " " with last
    else:
        provide first with " " with middle with " " with last
    end if
end action

store formatted as format name with "John" and "Doe"  // "John Doe"
store formatted with middle as format name with "Jane" and "Smith" and "Marie"  // "Jane Marie Smith"
```

### Event Handlers

Actions can serve as event handlers, responding to user interactions or system events:

```wfl
define action handle button click with button id:
    if button id is "submit":
        validate and submit form
    else if button id is "cancel":
        clear form and return to main page
    else if button id is "help":
        show help documentation
    end if
end action
```

### Data Processing Pipelines

You can chain actions together to create data processing pipelines:

```wfl
define action process data:
    store raw data as fetch data from server
    store filtered data as filter invalid entries from raw data
    store sorted data as sort filtered data by date
    store formatted data as format data for display with sorted data
    provide formatted data
end action
```

Each step in the pipeline is a separate action, making the code modular and easier to maintain.

## Conclusion

Actions are the building blocks of reusable code in WFL. They allow you to organize your program into logical, focused units that can be combined in powerful ways. WFL's natural language approach to actions makes them particularly accessible, with clear syntax that expresses your intent without cryptic symbols or complex structures.

In this section, we've covered:
- Basic action definition and calling syntax
- Working with parameters and return values
- Asynchronous actions and parallel operations
- Error handling in actions
- Organizing code with actions
- Variable scope and recursion
- Best practices and common patterns

As you practice with actions, you'll find they become an essential part of your programming toolkit. They're the key to writing clean, maintainable, and reusable code that solves complex problems elegantly.

In the next section, we'll explore how to model real-world entities with containers, which build upon the concepts we've learned so far. Happy coding!