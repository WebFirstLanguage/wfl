# WFL Variables: Storing and Using Data in Plain English

**Summary:** In this section, we'll introduce **variables** in WebFirst Language (WFL) – how to create them, give them values, and use them in a very natural way. Think of variables as labeled containers for information. WFL is designed so that writing code feels like writing simple English sentences. Even if you've never coded before, you'll find WFL variables intuitive and friendly. By the end of this guide, you'll know how to store different types of data (numbers, text, yes/no values, etc.), update those values, group values into collections, and even set default values or validate data – all with easy-to-read syntax.

## What Is a Variable?

A **variable** is like a named **storage box** for a piece of data. You give the box a name and put something inside it. Later, you can use the name to get the data back or change it. In traditional programming, you might use symbols or cryptic syntax to handle variables, but in WFL we do it using plain English.

**Why use variables?** Suppose you want to remember a user's age or a price of an item. Instead of writing the number everywhere, you store it in a variable with a clear name (like "age" or "price"). This way, you can refer to that piece of information later by name, which makes your code easier to read and change.

## Creating and Naming Variables

In WFL, creating a variable reads almost like a simple sentence. You typically use the keyword **`store`** or **`create`** to introduce a new variable. For example:

```wfl
store name as "Alice"
```

This line **creates a variable** called **`name`** and **stores** the text `"Alice"` in it. It's like saying *"store name as Alice"* – very natural! Similarly, you can store numbers or other values. For instance:

```wfl
store count as 42
store temperature as -5
store message as "Hello, world!"
store is active as yes
```

Here we made a **`count`** variable with the number 42, a **`temperature`** with -5, a **`message`** with `"Hello, world!"`, and an **`is active`** flag with the value **yes** (WFL uses **yes/no** as natural equivalents to true/false). Notice that variable names can be multiple words (like **`is active`**) to clearly describe what they hold. WFL encourages descriptive names – it's perfectly fine to have spaces in the name. This makes the code read like English (e.g., *"is active is yes"*).

A couple more examples of different value types:

- **Numbers:** You can write numbers in a friendly way. `store price as 10.99` (a decimal) or even `store very large as 1 million` (WFL understands common phrases for large numbers).
- **Text:** Always put text in quotes. For example, `store greeting as "Hello"`.
- **Yes/No (Booleans):** Use **yes** or **no** (or **true/false**, which WFL also accepts). For example, `store has account as no`.
- **Special values:** WFL has words for "nothingness" – for cases when a variable has no value. You can use **nothing**, **missing**, or **undefined** depending on context. For example, `store unknown value as nothing` might indicate an absence of a value.

### Creating Constants

Sometimes you have a value that never changes (like π = 3.14159). You can mark it as a **constant**. In WFL, that's as easy as saying:

```wfl
create new constant PI as 3.14159
```

This works like a variable, except WFL knows you don't intend to change it later. Naming constants in all caps (like `PI`) is a common convention, but not required.

### A Note on "Let" Syntax

You might wonder if you can use more conversational phrasing. Indeed, one of WFL's goals is to let you write code that sounds like English. For example, you could say something like *"Let the user's name be Alice."* While the official syntax uses `store` or `create`, the idea is the same. The key is that WFL tries to be as readable as possible. So even if we use `store X as Y` in these examples, you can read it in your head as "let X be Y."

## Updating and Changing Variables

Once you have variables, you'll often want to change their values. WFL provides natural-language ways to do this, avoiding cryptic operators. Suppose we have:

```wfl
store count as 0
```

Now we want to update `count`. Here are some ways to change variables:

```wfl
change count to 1
add 5 to count
subtract 2 from count
multiply count by 3
divide count by 2
```

Let's go through those:

- **`change count to 1`:** This sets the value of `count` to 1 (basically an assignment).
- **`add 5 to count`:** Increases `count` by 5. If count was 1, it becomes 6.
- **`subtract 2 from count`:** Decreases `count` by 2.
- **`multiply count by 3`:** Multiplies the value of `count` by 3.
- **`divide count by 2`:** Divides the value of `count` by 2.

These are equivalent to `count = count + 5`, `count = count - 2`, etc., but notice how **readable** they are. You can almost read them as simple instructions in English, which is exactly WFL's goal!

You can use these operations for any numeric variable. They make the intent crystal clear (no need to remember what `+=` or `++` means, as in other languages).

### Working with Text

WFL also makes manipulating text (strings) straightforward:

```wfl
join "Hello, " and name into greeting
```

This would take `"Hello, "` and whatever is in `name` (say "Alice"), and **join** them together into a new variable `greeting`. After running that, `greeting` would contain `"Hello, Alice"`.

Other text operations include:

```wfl
take first 5 letters from message
take last 3 letters from name
convert message to lowercase
convert name to uppercase
```

What do these do? If `message` is `"Hello, World!"`, then:
- `take first 5 letters from message` would result in `"Hello"`.
- `take last 3 letters from name` (if name is "Alice") would give `"ice"`.
- `convert message to lowercase` would change `"Hello, World!"` to `"hello, world!"`.
- `convert name to uppercase` would change "Alice" to "ALICE".

Again, the syntax is self-explanatory: you read it as you would in English. *"Take the first 5 letters from message."* It's clear what's happening without needing any complex functions or slicing indices.

## Collections: Lists, Sets, and Maps

As you get more comfortable, you might want to group multiple values together. WFL offers a few natural ways to handle **collections** of items: **lists, sets,** and **maps**. These terms might sound technical, but think of it this way:
- A **list** is an ordered list of things (allowing duplicates).
- A **set** is a collection of unique things (no duplicates).
- A **map** is like a dictionary or phone book: it maps keys to values (key/value pairs).

### Lists (Ordered Collections)

A list in WFL is like a shopping list or to-do list – an ordered collection where the order matters and you can have the same item more than once. Here's how you create and use a list:

```wfl
create list shopping:
    add "milk"
    add "bread"
    add "eggs"
end list
```

This defines a list called `shopping` with three items: "milk", "bread", "eggs". The syntax is a block: you start with `create list [name]:`, then list each element with `add ...` on its own line, and finish with `end list`.

After creating a list, you can manipulate it with natural commands:
```wfl
add "butter" to shopping      // now shopping has milk, bread, eggs, butter
remove "eggs" from shopping   // now eggs is removed
clear shopping list           // now shopping is empty
```

As shown:
- `add "butter" to shopping` puts "butter" at the end of the list.
- `remove "eggs" from shopping` takes "eggs" out of the list.
- `clear shopping list` empties the list completely.

### Sets (Unique Collections)

A set is for when you need to ensure all items are unique. Creating a set is similar to a list:

```wfl
create set unique colors:
    add "red"
    add "blue"
    add "red"
end set
```

We tried to add "red" twice. WFL will automatically ignore duplicates in a set. So our `unique colors` set effectively contains just "red" and "blue". The order is not as important in a set, and you won't have repeats.

Think of a set like a collection of tags or labels – each label only listed once.

### Maps (Key-Value Pairs)

A map is like a dictionary or a simple database of key-value pairs. Each entry has a unique key and a value associated with that key. For example, imagine settings in an application:

```wfl
create map settings:
    theme is "dark"
    volume is 75
    notifications are on
end map
```

Here we created a map called `settings` with three key-value pairs:
- The key **theme** has value `"dark"`.
- **volume** is 75.
- **notifications** are on (on is synonymous with yes/true).

The syntax uses `key is value` or `key are value` inside the map block, which again reads like English descriptions. After this, you could query or change entries in `settings` in a natural way (e.g., change the volume or check the theme), though the exact syntax for that can depend on context (often similar to removing from a list, etc., but using the key).

Maps are great when you want to look up a value by a name. For instance, `settings` is like a configuration where you can ask "what is the volume?" and get 75.

## Grouping Multiple Values with Records

A **record** in WFL is a way to group several related values together under one name. You can think of it like a simple object or a structured data type. If you have information that naturally goes together (like a person's name, age, and email), you can store it in a record instead of separate variables.

For example:

```wfl
create record person:
    name is "John Smith"
    age is 30
    email is "john@example.com"
    is member is yes
end record
```

This defines a `person` record with four fields: `name`, `age`, `email`, and `is member`. It's like creating a structured variable where each field has its own value.

Using a record feels intuitive:
- To **get** a field from a record, you can use a possessive style. For example, `get person's email` would retrieve `"john@example.com"`. (It reads like *"get person's email"*. In WFL, the apostrophe in code might not actually be required – you might simply write `get person email` – but conceptually it's the same idea of ownership.)
- To **change** a field: `change person's age to 31` would update the age in the `person` record.
- To **remove** a field: `remove person's email` would remove the email information from the record (perhaps setting it to nothing or making it unavailable).

Records are helpful when you want to pass around or manage a bunch of related info as one unit. For instance, you could have a record for a user, a record for a product, etc., each bundling relevant attributes.

### Dates and Times

WFL even handles dates and times in a friendly way. You can create date or time values with simple phrases:

```wfl
create date today
create time now
create date tomorrow as today plus 1 day
create time meeting at "14:30"
```

Explanation:
- `create date today` sets `today` to the current date.
- `create time now` sets `now` to the current time.
- `create date tomorrow as today plus 1 day` is quite literal: tomorrow = today + 1 day.
- `create time meeting at "14:30"` sets a time variable `meeting` to 2:30 PM.

The syntax for adding time (like plus 1 day) or specifying a time in quotes is designed to be straightforward. You could similarly do `plus 5 days` or set dates to specific strings like `"2025-12-31"` if needed.

## Beyond the Basics: Converting and Validating Values

The features above cover the basics of storing and manipulating data. WFL also provides higher-level constructs to make sure your data is in the right format and to convert between types safely. These are more intermediate features – you might not need them when you first start – but it's nice to know they exist as you grow more confident.

### Converting Between Types

Sometimes you have a value in one form and need it in another. For example, you have a text `"123"` and you need it as a number `123` to do math, or vice versa. WFL lets you **convert** types with an easy phrase:

```wfl
convert "123" to number    // yields the number 123
convert 123 to text        // yields the text "123"
convert "yes" to truth value  // yields the boolean yes
```

These are straightforward:
- Converting `"123"` (a text) to number results in the numeric value 123.
- Converting `123` (number) to text gives `"123"` (as text).
- Converting `"yes"` to a truth value yields **yes** (which is like true).

If the conversion is impossible (for example, converting `"abc"` to a number), WFL would normally throw an error or stop. But WFL also provides a **safe conversion** mechanism to handle such cases gracefully:

```wfl
safely convert "abc" to number:
    when invalid:
        use 0 instead
    when missing:
        ask user for number
end convert
```

In this example, we attempt to convert the text `"abc"` to a number. Clearly, `"abc"` isn't a numeric string, so it would fail. The **`safely convert ...`** block catches that:
- `when invalid:` means if the conversion is invalid (e.g., not a number), then do the following indented steps. Here we say `use 0 instead`, meaning if `"abc"` can't convert, just use 0.
- `when missing:` would handle the case where the value was missing/undefined. In our example it doesn't apply, but we included it to show the pattern – maybe if the value was not there, we could prompt the user for a number.

This safe conversion block ensures your program can decide what to do if data isn't in the expected format, without crashing. It reads like a plan: *"Try to convert, if invalid use 0, if missing ask user."*

### Validating Values (Ensuring Data Quality)

When working with user input or any data, it's good to validate that it meets certain criteria. WFL has a friendly way to enforce rules on values using the **`check ... must ...`** syntax.

**Simple validation example:**

```wfl
check age:
    must be number
    must be at least 0
    must be less than 150
end check
```

This sets up requirements for the variable `age`. It reads:
- Age **must be a number** (so if `age` is text or anything not numeric, this check would fail).
- Age **must be at least 0** (no negative ages).
- Age **must be less than 150** (assuming an age should be in a reasonable range).

If any of these conditions are violated, WFL would throw a clear error message explaining which rule failed. This is great for catching mistakes early. The phrasing is so straightforward, it's like writing documentation for your data that the computer also reads.

Another example, more complex:

```wfl
check email:
    must not be empty
    must contain "@"
    must contain "." after "@"
    must not contain spaces
end check
```

This defines what a valid `email` should look like:
- It can't be empty.
- It must have an "@" symbol.
- It must have a "." that comes after the "@"
- It must not have any spaces.

These rules cover basic email format checking, and again, they are written in plain terms. If `email` doesn't meet these, WFL will report which rule was broken (e.g., "email must contain '@'").

You can also define a **named validation rule** if you plan to reuse it:

```wfl
create rule valid password:
    must be at least 8 characters
    must have uppercase letter
    must have lowercase letter
    must have number
    must have special character
end rule
```

This creates a reusable rule (kind of like a template for checking passwords). Later you might apply `valid password` to check a user's password input. The idea is to keep your code DRY (Don't Repeat Yourself) – define the checks once and reuse.

### Using Conditions in Expressions

We've seen how to enforce rules. WFL also lets you simply **check a condition** on the fly. For instance:

```wfl
check if temperature is above 0
check if name is "Alice"
```

These could be used inside other constructs (like in an if-statement, which we cover in the next section on Control Flow). The point here is that WFL uses very natural comparative phrases:
- *"is above 0"*, *"is at least 18"*, *"is at most 100"*, *"is between 1 and 10"* are all valid ways to compare values.
- You can combine conditions with **and** or **or** in a check. For example:

```wfl
check if:
    age is at least 18
    and country is "USA"
    and has consent is yes
end check
```

This checks that **all** the listed conditions are true (the person is at least 18 **and** from USA **and** consent is yes). We could similarly use **or** if only one of multiple conditions needs to be true.

(We will dive deeper into `if` statements and conditional logic in the Control Flow section, so don't worry if you want to learn more about how `check if` works in practice. The takeaway here for the Variables chapter is just that WFL's conditions read naturally and can be used to assert things about your data.)

### Default Values for Variables

When creating variables, you might want to specify a default value up front, especially if it's something that might change later or if it's optional information. WFL allows default values in declarations:

```wfl
create username with default "guest"
create score with default 0
create status with default "pending"
```

This creates `username` defaulting to `"guest"`, `score` defaulting to `0`, and `status` defaulting to `"pending"`. You can later change them, but if not changed, those are the starting values.

For more complex structures, defaults can be even more elaborate:
```wfl
create new user:
    name defaults to "New User"
    joined date defaults to today
    settings default to:
        theme is "light"
        language is "English"
        notifications are on
    end settings
end create
```

This looks like creating a new *record or object* (like a user profile) with multiple fields, each having a default. We see nested defaults: the `settings` field itself is a map with its own defaults (light theme, English language, notifications on). Such nested structure might be part of a container or module, but the syntax shows how natural it is to provide initial values.

Defaults ensure that even if you don't explicitly set something, it has a sensible value from the start.

### Variable Scope and Lifetime (Global vs Local)

As you write larger programs, the concept of **scope** becomes important. Scope determines **where** a variable can be accessed. WFL uses clear words for scope:

- **Global variables:** available everywhere in your program. You declare one by prefixing with `global`. For example:
  ```wfl
  global create server url as "http://example.com"
  ```
  Now `server url` can be used from any part of the code.

- **Local variables:** exist only inside a specific block or routine. For example, if you are inside a process or action and only need a temporary variable:
  ```wfl
  inside process user:
      local create temp id
      local create user data
  end inside
  ```
  Here, `temp id` and `user data` exist only within that `inside` block (perhaps a function or process called "process user"). Once you leave that block, those locals are gone. This is similar to local variables in other languages, but WFL makes it explicit and readable with the `inside ... local create ...` structure.

- **Shared variables:** sometimes you want a variable accessible across a module or file, but not truly global to everything. WFL uses `shared` for that. For example:
  ```wfl
  create module user handling:
      shared create user count as 0
      shared create active users as empty list
  end module
  ```
  This suggests we have a module (like a grouping of related code) named "user handling", and inside it we have `user count` and `active users` defined as shared. Code within the same module can use them, but outside code won't see them (which helps prevent accidental misuse).

For a beginner, you don't need to deeply worry about scope if you're writing small scripts – just create variables as needed. But it's good to know these words (global, local, shared) exist and do exactly what they say. It keeps your code organized as it grows.

### Monitoring Changes to Variables (Advanced)

This is a more advanced feature, but an exciting one: WFL can **watch** for changes in variables and react to them. This is beyond what most beginner programmers need, but it shows the power of WFL's natural syntax.

For example, say you want to take action whenever a variable changes value:

```wfl
track changes to user count:
    when increases:
        check if user count is above limit
    when decreases:
        update display
end track
```

This means: *"Track changes to `user count`. If it increases, then check if it went above some limit (perhaps to alert or prevent something). If it decreases, update the display."* The keywords `when increases` and `when decreases` let you respond to how the value changed.

Another example, watching a settings object for any specific field changes:

```wfl
watch settings:
    when theme changes:
        update display theme
    when volume changes:
        update volume output
    when notifications change:
        update notification preferences
end watch
```

Here, we watch the `settings` (which could be a map or record of user settings). If the `theme` inside `settings` changes, we call some code to update the display theme. If `volume` changes, we adjust the audio output, and so on. It's like saying "keep an eye on these and respond when needed."

These features can make building interactive or reactive programs easier because the code reads like event descriptions. Again, this is an intermediate-to-advanced concept; new users of WFL can safely ignore tracking/watching until they have a need for it. But it's nice to know WFL can do this with such clear syntax.

## Conclusion

In this chapter, we covered a lot of ground in how WFL handles data:

- We learned that variables in WFL are declared with everyday words like **store** and **create**, and you can name them in a descriptive, spaced-out way (making code self-documenting).
- We saw basic data types: numbers, text, yes/no booleans, and special placeholders like nothing/missing.
- We performed operations to update variables (in plain arithmetic language) and to manipulate text.
- We organized data using lists, sets, and maps, and grouped related data into records.
- We touched on converting between types safely, validating data with simple rules, setting default values, and even advanced features like scope control and change tracking.

The key theme is **clarity**. WFL's variable and data handling is designed to be **clear at a glance**. You don't see cryptic symbols or hard-to-guess abbreviations – it's all written out in a way that beginners can understand and experienced developers can appreciate for its readability.

Feel free to play around with these examples. Try creating your own variables and collections. In the next sections, we'll use these variables in control structures (like loops and conditionals) and see how WFL continues the trend of English-like syntax for coding tasks. Happy coding!