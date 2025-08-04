# WFL by Example

This comprehensive guide teaches WFL through practical examples, building from simple concepts to advanced features. Each section introduces new concepts while reinforcing what you've already learned.

## Table of Contents

1. [Hello World and Basic Output](#hello-world-and-basic-output)
2. [Variables and Data Types](#variables-and-data-types)
3. [Working with Text](#working-with-text)
4. [Numbers and Math](#numbers-and-math)
5. [Making Decisions](#making-decisions)
6. [Loops and Repetition](#loops-and-repetition)
7. [Lists and Collections](#lists-and-collections)
8. [Functions (Actions)](#functions-actions)
9. [Containers (Objects/Classes)](#containers-objectsclasses)
10. [File Input and Output](#file-input-and-output)
11. [Async Operations and Web Requests](#async-operations-and-web-requests)
12. [Error Handling](#error-handling)
13. [Advanced Features](#advanced-features)

---

## Hello World and Basic Output

Let's start with the classic "Hello, World!" program:

```wfl
// The simplest WFL program
display "Hello, World!"
```

**What's happening:**
- `//` starts a comment (ignored by the computer)
- `display` shows text on the screen
- Text is surrounded by quotes

**Try these variations:**

```wfl
// Multiple lines of output
display "Welcome to WFL!"
display "This language reads like English."
display "Pretty cool, right?"

// Using print instead of display
print "This also works!"
```

---

## Variables and Data Types

Variables store information. WFL automatically determines the type of data:

```wfl
// Text (strings)
store user name as "Alice"
store favorite color as "blue"

// Numbers
store age as 25
store height as 5.8
store temperature as -10

// Booleans (true/false values)
store is student as yes
store has license as no
store is online as true      // Can use true/false or yes/no

// Null/empty values
store middle name as nothing
store score as undefined
```

**Using variables:**

```wfl
store first name as "John"
store last name as "Smith"

display "First name: " with first name
display "Last name: " with last name
display "Full name: " with first name with " " with last name
```

**Variable naming:**

```wfl
// Variable names can have spaces!
store user email address as "john@example.com"
store current balance as 1500.50
store is account active as yes

display "Email: " with user email address
display "Balance: $" with current balance
display "Active: " with is account active
```

---

## Working with Text

Text manipulation is fundamental in programming:

**Basic text operations:**

```wfl
store greeting as "Hello"
store name as "World"

// Combining text
store message as greeting with ", " with name with "!"
display message                    // Shows: Hello, World!

// Text functions
display "Length of greeting: " with length of greeting
display "Uppercase: " with touppercase of greeting
display "Lowercase: " with tolowercase of greeting
```

**Text searching and manipulation:**

```wfl
store sentence as "The quick brown fox jumps over the lazy dog"

// Check if text contains something
check if contains of sentence and "fox":
    display "Found the fox!"
end check

// Get part of text
store first word as substring of sentence and 0 and 3
display "First word: " with first word      // Shows: The

// Text replacement (using standard library)
store new sentence as replace of sentence and "fox" and "cat"
display new sentence                        // The quick brown cat jumps...
```

**Input and text processing:**

```wfl
// Note: This example shows concepts - actual input might vary
store user input as "  Hello WFL!  "

// Clean up user input
store cleaned input as trim of user input
store final input as tolowercase of cleaned input

display "Original: '" with user input with "'"
display "Cleaned: '" with cleaned input with "'"
display "Final: '" with final input with "'"
```

---

## Numbers and Math

WFL makes math operations natural and readable:

**Basic arithmetic:**

```wfl
store a as 10
store b as 3

display a with " + " with b with " = " with (a plus b)           // 13
display a with " - " with b with " = " with (a minus b)          // 7
display a with " × " with b with " = " with (a times b)          // 30
display a with " ÷ " with b with " = " with (a divided by b)     // 3.333...
```

**Math functions:**

```wfl
store number as -7.8

display "Original: " with number
display "Absolute value: " with abs of number         // 7.8
display "Rounded: " with round of number             // -8
display "Floor: " with floor of number               // -8
display "Ceiling: " with ceil of number              // -7
```

**Random numbers and advanced math:**

```wfl
// Random number between 0 and 1
store random value as random
display "Random: " with random value

// Random number in a range (using math)
store min as 1
store max as 10
store random in range as (random times (max minus min)) plus min
store dice roll as round of random in range
display "Dice roll: " with dice roll

// Clamp a value between limits
store user input as 150
store clamped value as clamp of user input and 0 and 100
display "Clamped to 0-100: " with clamped value      // 100
```

**Practical math example - calculating compound interest:**

```wfl
store principal as 1000.0
store interest rate as 0.05
store years as 10

store final amount as principal times (1 plus interest rate) to the power of years
display "Initial amount: $" with principal
display "After " with years with " years at " with (interest rate times 100) with "%:"
display "Final amount: $" with round of final amount
```

---

## Making Decisions

Decision-making with if/then statements:

**Basic conditionals:**

```wfl
store temperature as 72

check if temperature is greater than 80:
    display "It's hot outside!"
check if temperature is greater than 60:
    display "Nice weather!"
otherwise:
    display "It's cold outside!"
end check
```

**Multiple conditions:**

```wfl
store age as 25
store has license as yes

check if age is greater than or equal to 16 and has license is yes:
    display "You can drive!"
check if age is greater than or equal to 16:
    display "You can get a license!"
otherwise:
    display "Too young to drive."
end check
```

**Comparing text:**

```wfl
store user role as "admin"

check if user role is "admin":
    display "Full access granted"
check if user role is "user":
    display "Limited access granted"
check if user role is "guest":
    display "Read-only access"
otherwise:
    display "Access denied"
end check
```

**Complex conditions:**

```wfl
store score as 85
store extra credit as 5
store final score as score plus extra credit

check if final score is greater than or equal to 97:
    display "Grade: A+"
check if final score is greater than or equal to 93:
    display "Grade: A"
check if final score is greater than or equal to 90:
    display "Grade: A-"
check if final score is greater than or equal to 87:
    display "Grade: B+"
check if final score is greater than or equal to 83:
    display "Grade: B"
otherwise:
    display "Grade: B- or lower"
end check
```

---

## Loops and Repetition

Loops let you repeat actions efficiently:

**Counting loops:**

```wfl
// Simple counting
count from 1 to 5:
    display "Count: " with count
end count

// Counting with steps
count from 0 to 20 by 2:
    display "Even number: " with count
end count

// Counting backwards
count from 10 to 1 by -1:
    display "Countdown: " with count
end count
display "Blast off!"
```

**Working with lists:**

```wfl
store fruits as ["apple", "banana", "orange", "grape"]

// Loop through each item
for each fruit in fruits:
    display "I like " with fruit
end for

// Loop with index
for each fruit at index in fruits:
    display "Item " with (index plus 1) with ": " with fruit
end for
```

**Practical example - calculating totals:**

```wfl
store prices as [12.99, 8.50, 23.00, 15.75, 6.25]
store total as 0
store count as 0

for each price in prices:
    store total as total plus price
    store count as count plus 1
    display "Item " with count with ": $" with price
end for

store average as total divided by count
display "Total: $" with total
display "Average: $" with round of average
```

**While loops (conditional repetition):**

```wfl
store number as 1

while number is less than or equal to 1000:
    display number
    store number as number times 2
end while

display "Final number: " with number
```

---

## Lists and Collections

Lists store multiple values in order:

**Creating and using lists:**

```wfl
// Create a list
store shopping list as ["milk", "bread", "eggs", "butter"]

// Access items by index (starting from 0)
display "First item: " with shopping list at 0
display "Second item: " with shopping list at 1

// List information
display "List length: " with length of shopping list
display "Contains milk? " with contains of shopping list and "milk"
```

**Modifying lists:**

```wfl
store numbers as [1, 2, 3]

// Add items
add 4 to numbers
add 5 to numbers
display "After adding: " with numbers        // [1, 2, 3, 4, 5]

// Remove the last item
store removed as pop of numbers
display "Removed: " with removed            // 5
display "After removing: " with numbers     // [1, 2, 3, 4]

// Find item position
store position as indexof of numbers and 3
display "Position of 3: " with position    // 2 (zero-based)
```

**List processing examples:**

```wfl
// Double all numbers in a list
store original as [1, 2, 3, 4, 5]
store doubled as []

for each number in original:
    add (number times 2) to doubled
end for

display "Original: " with original
display "Doubled: " with doubled
```

**Nested lists (lists within lists):**

```wfl
store matrix as [[1, 2, 3], [4, 5, 6], [7, 8, 9]]

// Access nested items
display "First row: " with matrix at 0
display "Middle item: " with (matrix at 1) at 1    // Gets 5

// Process nested lists
for each row at row index in matrix:
    for each value at col index in row:
        display "Row " with row index with ", Col " with col index with ": " with value
    end for
end for
```

---

## Functions (Actions)

Functions (called "actions" in WFL) organize and reuse code:

**Simple actions:**

```wfl
// Define an action
define action called say hello:
    display "Hello from my action!"
end action

// Use the action
say hello
```

**Actions with parameters:**

```wfl
define action called greet person:
    parameter name as Text
    display "Hello, " with name with "!"
end action

// Use with different names
greet person with "Alice"
greet person with "Bob"
greet person with "Charlie"
```

**Actions that return values:**

```wfl
define action called calculate area:
    parameter width as Number
    parameter height as Number
    
    store area as width times height
    return area
end action

// Use the action
store room area as calculate area with 12 and 10
display "Room area: " with room area with " square feet"
```

**More complex example:**

```wfl
define action called format currency:
    parameter amount as Number
    parameter currency as Text
    
    store rounded as round of amount to 2 decimal places
    store formatted as currency with rounded
    return formatted
end action

define action called calculate discount:
    parameter original price as Number
    parameter discount percent as Number
    
    store discount amount as original price times (discount percent divided by 100)
    store final price as original price minus discount amount
    return final price
end action

// Use the actions together
store item price as 29.99
store sale price as calculate discount with item price and 15
store formatted price as format currency with sale price and "$"

display "Original price: " with format currency with item price and "$"
display "Sale price (15% off): " with formatted price
```

**Actions with multiple parameters:**

```wfl
define action called calculate bmi:
    parameter weight as Number
    parameter height as Number
    parameter unit as Text
    
    check if unit is "metric":
        store bmi as weight divided by (height times height)
    otherwise:
        // Imperial: weight in pounds, height in inches
        store bmi as (weight divided by (height times height)) times 703
    end check
    
    return round of bmi
end action

store my bmi as calculate bmi with 150 and 70 and "imperial"
display "Your BMI is: " with my bmi
```

---

## Containers (Objects/Classes)

Containers group related data and actions together:

**Basic container:**

```wfl
// Define a container
create container Person:
    property name as Text
    property age as Number
    
    action introduce:
        display "Hi, I'm " with name with " and I'm " with age with " years old."
    end action
end container

// Create and use a person
create new Person as alice:
    name is "Alice"
    age is 30
end

alice.introduce()
```

**Container with methods that use properties:**

```wfl
create container BankAccount:
    property account number as Text
    property balance as Number
    property account holder as Text
    
    action deposit:
        parameter amount as Number
        
        check if amount is greater than 0:
            store balance as balance plus amount
            display "Deposited $" with amount with ". New balance: $" with balance
        otherwise:
            display "Invalid deposit amount"
        end check
    end action
    
    action withdraw:
        parameter amount as Number
        
        check if amount is greater than balance:
            display "Insufficient funds"
        check if amount is less than or equal to 0:
            display "Invalid withdrawal amount"
        otherwise:
            store balance as balance minus amount
            display "Withdrew $" with amount with ". New balance: $" with balance
        end check
    end action
    
    action get balance:
        return balance
    end action
end container

// Use the bank account
create new BankAccount as my account:
    account number is "12345"
    balance is 1000.0
    account holder is "John Doe"
end

my account.deposit(250.0)
my account.withdraw(100.0)
display "Final balance: $" with my account.get balance()
```

**Container inheritance:**

```wfl
// Base container
create container Animal:
    property name as Text
    property species as Text
    
    action speak:
        display name with " makes a sound"
    end action
end container

// Inherited container
create container Dog extends Animal:
    property breed as Text
    
    action speak:
        display name with " barks!"
    end action
    
    action fetch:
        display name with " fetches the ball!"
    end action
end container

// Create and use
create new Dog as my dog:
    name is "Buddy"
    species is "Canine"
    breed is "Golden Retriever"
end

my dog.speak()
my dog.fetch()
```

---

## File Input and Output

Working with files for data storage and retrieval:

**Reading files:**

```wfl
// Read entire file
store file contents as read file "data.txt"
display "File contents:"
display file contents

// Read file line by line
store lines as read lines from "data.txt"
for each line in lines:
    display "Line: " with line
end for
```

**Writing files:**

```wfl
// Write text to file
store content as "Hello, World!\nThis is a test file.\nWFL is awesome!"
write content to file "output.txt"
display "File written successfully"

// Append to file
append "This line was added later" to file "output.txt"
```

**Practical file processing example:**

```wfl
// Process a CSV-like file
define action called process sales data:
    parameter filename as Text
    
    store total sales as 0
    store line count as 0
    
    store lines as read lines from filename
    
    for each line in lines:
        // Skip header line
        check if line count is greater than 0:
            store parts as split line by ","
            store amount as parse number from (parts at 2)  // Assume price is 3rd column
            store total sales as total sales plus amount
        end check
        
        store line count as line count plus 1
    end for
    
    display "Processed " with (line count minus 1) with " sales records"
    display "Total sales: $" with total sales
    
    return total sales
end action

// Use the action
store sales total as process sales data with "sales.csv"
```

**File operations:**

```wfl
// Check if file exists
check if file exists "config.txt":
    display "Config file found"
otherwise:
    display "Creating default config file"
    write "default settings" to file "config.txt"
end check

// Get file info
store file size as size of file "data.txt"
store last modified as modified date of file "data.txt"
display "File size: " with file size with " bytes"
display "Last modified: " with last modified
```

---

## Async Operations and Web Requests

WFL supports asynchronous operations for web requests and concurrent tasks:

**Simple web request:**

```wfl
// Make a web request
store response as await web.get("https://api.github.com/users/octocat")
display "Response status: " with response.status
display "Response body: " with response.body
```

**Processing JSON responses:**

```wfl
define action called get user info:
    parameter username as Text
    
    store url as "https://api.github.com/users/" with username
    store response as await web.get(url)
    
    check if response.status is 200:
        store user data as parse json from response.body
        display "Name: " with user data.name
        display "Public repos: " with user data.public_repos
        display "Followers: " with user data.followers
    otherwise:
        display "User not found or API error"
    end check
end action

// Use the action
get user info with "octocat"
```

**Multiple concurrent requests:**

```wfl
define action called fetch multiple urls:
    parameter urls as List
    
    store responses as []
    
    // Start all requests concurrently
    for each url in urls:
        store future as async web.get(url)
        add future to responses
    end for
    
    // Wait for all to complete
    for each future in responses:
        store result as await future
        display "Status for " with result.url with ": " with result.status
    end for
end action

store api urls as [
    "https://httpbin.org/status/200",
    "https://httpbin.org/status/404",
    "https://httpbin.org/delay/1"
]

fetch multiple urls with api urls
```

**Web scraping example:**

```wfl
define action called get page title:
    parameter url as Text
    
    store response as await web.get(url)
    
    check if response.status is 200:
        // Simple title extraction (in real implementation)
        store html as response.body
        store title start as indexof of html and "<title>"
        store title end as indexof of html and "</title>"
        
        check if title start is not -1 and title end is not -1:
            store title with tags as substring of html and (title start plus 7) and (title end minus title start minus 7)
            display "Page title: " with title with tags
        otherwise:
            display "No title found"
        end check
    otherwise:
        display "Failed to fetch page"
    end check
end action

get page title with "https://example.com"
```

---

## Error Handling

WFL provides graceful error handling to make programs robust:

**Basic error handling:**

```wfl
try:
    store result as 10 divided by 0
    display "Result: " with result
catch error:
    display "Math error: " with error.message
end try
```

**File operation error handling:**

```wfl
try:
    store content as read file "nonexistent.txt"
    display content
catch file error:
    display "Could not read file: " with file error.message
    display "Creating default file instead"
    write "Default content" to file "nonexistent.txt"
end try
```

**Web request error handling:**

```wfl
define action called safe web request:
    parameter url as Text
    
    try:
        store response as await web.get(url)
        check if response.status is 200:
            return response.body
        otherwise:
            return "HTTP Error: " with response.status
        end check
    catch network error:
        return "Network error: " with network error.message
    catch timeout error:
        return "Request timed out"
    catch error:
        return "Unknown error: " with error.message
    end try
end action

store result as safe web request with "https://httpbin.org/delay/10"
display result
```

**Custom error handling:**

```wfl
define action called validate age:
    parameter age as Number
    
    check if age is less than 0:
        throw error "Age cannot be negative"
    check if age is greater than 150:
        throw error "Age seems unrealistic"
    end check
    
    return "Age is valid"
end action

try:
    store validation result as validate age with -5
    display validation result
catch validation error:
    display "Validation failed: " with validation error.message
end try
```

---

## Advanced Features

### Pattern Matching

```wfl
store user input as "hello@example.com"

// Email pattern matching
define pattern email as "[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}"

check if user input matches email:
    display "Valid email address"
otherwise:
    display "Invalid email format"
end check

// Extract parts using pattern groups
store email parts as extract from user input using email
display "Username: " with email parts.username
display "Domain: " with email parts.domain
```

### Working with Dates and Time

```wfl
// Current date and time
store now as current time
display "Current time: " with now

// Format dates
store formatted as format now as "YYYY-MM-DD HH:mm:ss"
display "Formatted: " with formatted

// Date calculations
store tomorrow as now plus 1 day
store next week as now plus 7 days
store last month as now minus 30 days

display "Tomorrow: " with tomorrow
display "Next week: " with next week
display "Last month: " with last month
```

### Database Operations

```wfl
// Connect to database
store db as connect to database "sqlite://./app.db"

// Create table
execute sql "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)" on db

// Insert data
execute sql "INSERT INTO users (name, email) VALUES (?, ?)" with ["Alice", "alice@example.com"] on db

// Query data
store users as query sql "SELECT * FROM users" on db

for each user in users:
    display "User: " with user.name with " (" with user.email with ")"
end for

// Close database
close database db
```

### Configuration and Environment

```wfl
// Read environment variables
store api key as environment variable "API_KEY"
store debug mode as environment variable "DEBUG" or "false"

// Read configuration file
store config as read json from "config.json"
store database url as config.database.url
store port as config.server.port

display "Database URL: " with database url
display "Server port: " with port
```

### Advanced List Operations

```wfl
store numbers as [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]

// Filter lists
store even numbers as filter numbers where (item mod 2 is 0)
display "Even numbers: " with even numbers

// Map/transform lists
store squared as map numbers to (item times item)
display "Squared: " with squared

// Reduce lists
store sum as reduce numbers from 0 using (accumulator plus item)
display "Sum: " with sum

// Chain operations
store result as numbers
    | filter where (item is greater than 5)
    | map to (item times 2)
    | reduce from 0 using (accumulator plus item)

display "Chained result: " with result
```

---

## Putting It All Together: Complete Example Program

Here's a complete example that demonstrates many WFL concepts:

```wfl
// Task Management System
create container Task:
    property id as Number
    property title as Text
    property description as Text
    property completed as Boolean
    property due date as Text
    
    action mark complete:
        store completed as yes
        display "Task '" with title with "' marked as complete!"
    end action
    
    action is overdue:
        store today as current date
        store due as parse date from due date
        return due is less than today and completed is no
    end action
end container

create container TaskManager:
    property tasks as List
    property next id as Number
    
    action initialize:
        store tasks as []
        store next id as 1
    end action
    
    action add task:
        parameter title as Text
        parameter description as Text
        parameter due date as Text
        
        create new Task as new task:
            id is next id
            title is title
            description is description
            completed is no
            due date is due date
        end
        
        add new task to tasks
        store next id as next id plus 1
        
        display "Added task: " with title
    end action
    
    action list tasks:
        parameter show completed as Boolean
        
        check if length of tasks is 0:
            display "No tasks found"
            return
        end check
        
        display "Tasks:"
        display "------"
        
        for each task in tasks:
            store should show as no
            
            check if show completed is yes:
                store should show as yes
            check if task.completed is no:
                store should show as yes
            end check
            
            check if should show is yes:
                store status as "[ ]"
                check if task.completed is yes:
                    store status as "[✓]"
                end check
                
                store overdue marker as ""
                check if task.is overdue():
                    store overdue marker as " (OVERDUE!)"
                end check
                
                display status with " " with task.title with overdue marker
                display "    " with task.description
                display "    Due: " with task.due date
                display ""
            end check
        end for
    end action
    
    action save to file:
        parameter filename as Text
        
        store output as "# Task List\n\n"
        
        for each task in tasks:
            store status as "Incomplete"
            check if task.completed is yes:
                store status as "Complete"
            end check
            
            store output as output with "## " with task.title with "\n"
            store output as output with "**Status:** " with status with "\n"
            store output as output with "**Due:** " with task.due date with "\n"
            store output as output with "**Description:** " with task.description with "\n\n"
        end for
        
        write output to file filename
        display "Tasks saved to " with filename
    end action
end container

// Main program
define action called main:
    create new TaskManager as manager
    manager.initialize()
    
    // Add some sample tasks
    manager.add task with "Learn WFL" and "Complete the WFL by Example guide" and "2024-12-31"
    manager.add task with "Build web app" and "Create a task management web application" and "2024-12-15"
    manager.add task with "Write tests" and "Add unit tests for the task system" and "2024-12-20"
    
    // Mark one task as complete
    store first task as manager.tasks at 0
    first task.mark complete()
    
    // Show all tasks
    display "All tasks:"
    manager.list tasks with yes
    
    // Show only incomplete tasks
    display "\nIncomplete tasks only:"
    manager.list tasks with no
    
    // Save to file
    manager.save to file with "my_tasks.md"
    
    display "\nTask management demo complete!"
end action

// Run the program
main()
```

This example demonstrates:
- Container definitions with properties and methods
- Error handling and validation
- File I/O operations
- List manipulation
- Method chaining and object-oriented design
- Conditional logic and loops
- Text processing and formatting

## Next Steps

Now that you've seen WFL in action:

1. **Practice**: Try modifying the examples to see how they work
2. **Experiment**: Create your own programs combining different concepts
3. **Explore**: Check out the `TestPrograms/` folder for more examples
4. **Reference**: Use the Language Reference documentation for detailed syntax
5. **Build**: Start building real applications with WFL

Remember: WFL is designed to be readable and intuitive. If something seems complicated, there's probably a simpler way to express it in WFL. Happy coding!