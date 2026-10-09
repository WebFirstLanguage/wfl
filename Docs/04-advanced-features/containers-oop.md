# Containers (Object-Oriented Programming)

WFL supports object-oriented programming through **containers**—a natural way to organize code and data.

## What are Containers?

Containers are WFL's version of classes. They combine:
- **Properties** - Data fields
- **Actions** - Methods/functions
- **Inheritance** - Code reuse
- **Interfaces** - Contracts

Think of containers as templates for creating objects.

## Basic Container

### Defining a Container

```wfl
create container Person:
    property name: Text
    property age: Number

    action greet:
        display "Hello, I am " with name
    end
end
```

**Syntax:**
```wfl
create container <Name>:
    property <name>: <Type>
    ...
    action <name>:
        <statements>
    end
end
```

### Creating an Instance

```wfl
create new Person as alice:
    name is "Alice"
    age is 28
end
```

**Syntax:**
```wfl
create new <ContainerType> as <variable>:
    <property> is <value>
    ...
end
```

### Calling Actions

```wfl
alice.greet()
```

**Output:** `Hello, I am Alice`

## Properties

Properties store data:

```wfl
create container Book:
    property title: Text
    property author: Text
    property pages: Number
    property is_available: Boolean
end

create new Book as my_book:
    title is "WFL Guide"
    author is "WFL Team"
    pages is 250
    is_available is yes
end
```

### Accessing Properties

```wfl
display my_book.title         // "WFL Guide"
display my_book.pages         // 250
```

### Modifying Properties

Properties are changed from *inside* an action on the container—not by assigning
to `object.property` directly. Give the container an action that updates the
property:

```wfl
create container Book:
    property title: Text
    property is_available: Boolean

    action check_out:
        change is_available to no
        display "Book is now unavailable"
    end
end

create new Book as my_book:
    title is "WFL Guide"
    is_available is yes
end

my_book.check_out()
```

## Actions (Methods)

Actions are functions that belong to containers:

```wfl
create container Calculator:
    property value: Number

    action increase needs amount: Number:
        change value to value + amount
    end

    action get_value: Number
        return value
    end
end

create new Calculator as calc:
    value is 0
end

calc.increase(10)
calc.increase(5)
store result as calc.get_value()
display "Result: " with result  // 15
```

### Actions with Parameters

```wfl
action set_name needs new_name: Text:
    store name as new_name
    display "Name changed to: " with name
end
```

### Actions with Returns

```wfl
action get_full_name: Text
    return first_name with " " with last_name
end
```

### Calling Other Actions with `this`

Inside an action, `this` means **the object the action was called on**. Use it
to call another action of the same container, the same way code outside the
container calls an action through the object's name:

```wfl
create container Greeter:
    property name: Text
    property greetings: Number

    action greet: Text
        this.count_greeting()
        return this.message()
    end

    action message: Text
        return "Hello, " with name with "! (greeting " with greetings with ")"
    end

    action count_greeting:
        change greetings to greetings + 1
    end
end

create new Greeter as greeter:
    name is "Ada"
    greetings is 0
end

display greeter.greet()                            // Hello, Ada! (greeting 1)
display greeter.greet()                            // Hello, Ada! (greeting 2)
display "Total greetings: " with greeter.greetings // Total greetings: 2
```

Outside the container you write `greeter.message()`. Inside it you write
`this.message()`. Both run the same action on the same object.

What to know about `this`:

- **Changes are shared.** Property changes an action makes before calling
  `this.other()` are visible to `other`, and the changes `other` makes are
  visible when it returns. This also holds when the object reaches another
  action under a different name, for example `call helper with this`.
- **Each call keeps its own local variables.** A called action cannot see or
  change the caller's local variables, so recursion through `this` works.
- **Order does not matter.** An action can call a sibling declared later in
  the container. In the example, `greet` calls `message`, which comes after it.
- **Inheritance works.** `this.action()` also finds actions inherited from a
  parent container. When a parent's action calls `this.action()` on an object
  of a child container, the child's override runs (see
  [Overriding Actions](#overriding-actions)).
- **Reading properties.** `this.name` reads the property's current value,
  including changes the action has just made.
- **Other objects.** An action can call actions on other objects it is given,
  for example `other.bump()`. Each call runs on its own object, and inside it
  `this` means that object.
- **`this` is fixed.** `change this to ...` is an error.
- **Instance actions only.** A `static action` belongs to the container, not
  to one object, so using `this` in it is an error.
- **Existing variables keep their meaning.** `this` is not a reserved word.
  If a program already has its own variable named `this` where an action runs
  (a top-level variable, or one the action creates with `store this as ...` or
  `for each this in ...`), that variable keeps its meaning, and `this` there
  does not mean the object. Rename such a variable to use `this` for the
  object.
- **Concurrent handlers.** Under `main loop concurrently:`, each running
  action works on its own copy of its object's properties and writes the copy
  back when it finishes. Two handlers running actions on the same object at
  the same time can therefore overwrite each other's property changes.

A bare call such as `call count_greeting` is not looked up among the
container's actions. WFL stops before running the program and names the fix:

```
error[ANALYZE-SEMANTIC]: 'count_greeting' is an action of container 'Greeter'. Inside the container's actions, call it on the current object: this.count_greeting(...)
```

In a file that uses `include from`, WFL cannot see which actions the included
files define, so it only warns about the bare call, and the call fails with
`Undefined action 'count_greeting'` when it runs.

## Inheritance

Containers can extend other containers:

```wfl
create container Person:
    property name: Text
    property age: Number

    action greet:
        display "Hello, I am " with name
    end
end

create container Employee extends Person:
    property job_title: Text
    property salary: Number

    action greet:
        display "Hello, I am " with name with ", " with job_title
    end

    action get_salary: Number
        return salary
    end
end

create new Employee as bob:
    name is "Bob"
    age is 35
    job_title is "Developer"
    salary is 75000
end

bob.greet()
// Output: "Hello, I am Bob, Developer"
```

### Overriding Actions

Child containers can override parent actions:

```wfl
create container Animal:
    property name: Text

    action make_sound:
        display "Some generic sound"
    end
end

create container Dog extends Animal:
    action make_sound:
        display "Woof! I'm " with name
    end
end

create new Dog as buddy:
    name is "Buddy"
end

buddy.make_sound()
// Output: "Woof! I'm Buddy"
```

## Interfaces

Interfaces define contracts that containers must fulfill. An interface body
lists the actions every implementing container is **required** to provide:

```wfl
create interface Drawable:
    requires action draw
    requires action get_area: Number
end

create container Rectangle implements Drawable:
    property width: Number
    property height: Number

    action draw:
        display "Drawing rectangle: " with width with " x " with height
    end

    action get_area: Number
        return width times height
    end
end

create new Rectangle as rect:
    width is 10
    height is 5
end

rect.draw()
store area as rect.get_area()
display "Area: " with area
```

**Syntax:**
```wfl
create interface <Name>:
    requires action <name>
    requires action <name>: <ReturnType>
    requires action <name> needs <param>: <Type>, <param>: <Type>
end
```

### Contracts Are Enforced

A container that claims `implements X` but does not provide every required
action is rejected. The static checker reports the breach, and the program
stops with an error when the container definition runs:

```wfl
create interface Drawable:
    requires action draw
end

create container Circle implements Drawable:
    property radius: Number
end

// Error: Container 'Circle' does not satisfy interface 'Drawable':
//        missing required action 'draw'
```

A required action with parameters must be implemented with the same number of
parameters. A requirement may also be satisfied by an action inherited from a
parent container (`extends`).

Two details of the contract:

- **Interface contracts are instance contracts.** A `static action` with the
  right name does not satisfy `requires action` — the requirement must be met
  by a regular (instance) action.
- **Required return types are checked statically.** If an interface declares
  `requires action get_area: Number` and the implementing action returns
  `Text`, the static checker reports the mismatch before the program runs.

### Interface Inheritance

Interfaces can extend other interfaces; the requirements accumulate:

```wfl
create interface Drawable:
    requires action draw
end

create interface Shape extends Drawable:
    requires action get_area: Number
end

// A container implementing Shape must provide BOTH draw and get_area.
```

### Marker Interfaces

An interface without a body is an empty contract — useful as a marker or tag
that any container can implement:

```wfl
create interface Serializable
```

## Complete Example: Task Manager

```wfl
create container Task:
    property description: Text
    property completed: Boolean
    property priority: Number

    action mark_complete:
        store completed as yes
        display "✓ Completed: " with description
    end

    action set_priority needs level: Number:
        store priority as level
    end

    action to_string: Text
        store mark as "☐"
        check if completed is yes:
            change mark to "✓"
        end check
        return mark with " " with description with " (P" with priority with ")"
    end
end

create container TaskList:
    property tasks: List

    action add_task needs task: Task:
        push with tasks and task
    end

    action show_all:
        display "=== Task List ==="
        for each task in tasks:
            store task_str as task.to_string()
            display task_str
        end for
    end

    action complete_first:
        check if length of tasks is greater than 0:
            store first_task as tasks[0]
            first_task.mark_complete()
        end check
    end
end

// Usage
create new Task as task1:
    description is "Learn WFL"
    completed is no
    priority is 1
end

create new Task as task2:
    description is "Build web server"
    completed is no
    priority is 2
end

create new TaskList as my_tasks:
    tasks is []
end

my_tasks.add_task(task1)
my_tasks.add_task(task2)
my_tasks.show_all()

my_tasks.complete_first()

display ""
my_tasks.show_all()
```

## Best Practices

✅ **Use descriptive container names:** `Person`, `Employee`, `Task`

✅ **PascalCase for containers:** `TaskManager`, `UserAccount`

✅ **snake_case for properties:** `first_name`, `email_address`

✅ **Descriptive action names:** `calculate_total`, `validate_input`

✅ **Type annotations:** Always specify property types

❌ **Don't create god objects:** Keep containers focused

❌ **Don't skip type annotations:** They help catch errors

❌ **Don't overuse inheritance:** Prefer composition when appropriate

## What You've Learned

In this section, you learned:

✅ **Defining containers** - `create container`
✅ **Properties** - Data fields with types
✅ **Actions** - Methods belonging to containers
✅ **Creating instances** - `create new`
✅ **Calling actions** - `object.action()`
✅ **Inheritance** - `extends` keyword
✅ **Interfaces** - `implements` keyword, contracts enforced via `requires action`
✅ **Complete examples** - Task manager with OOP

## Next Steps

Explore related topics:

**[Actions (Functions) →](../03-language-basics/actions-functions.md)**
Review action syntax for use in containers.

**[Subprocess Execution →](subprocess-execution.md)**
Run external commands in your OOP applications.

**[Best Practices: Project Organization →](../06-best-practices/project-organization.md)**
Structure large applications with containers.

---

**Previous:** [← Async Programming](async-programming.md) | **Next:** [Subprocess Execution →](subprocess-execution.md)
