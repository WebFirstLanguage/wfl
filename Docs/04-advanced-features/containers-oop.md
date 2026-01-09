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

```wfl
store my_book.is_available as no
display "Book is now unavailable"
```

## Actions (Methods)

Actions are functions that belong to containers:

```wfl
create container Calculator:
    property value: Number

    action add needs amount: Number:
        store value as value + amount
    end

    action get_value: Number
        return value
    end
end

create new Calculator as calc:
    value is 0
end

calc.add(10)
calc.add(5)
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

## Inheritance

Containers can extend other containers:

```wfl
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

Interfaces define contracts that containers must fulfill:

```wfl
create interface Drawable

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
        store status as "☐"
        check if completed is yes:
            change status to "✓"
        end check
        return status with " " with description with " (P" with priority with ")"
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
✅ **Interfaces** - `implements` keyword
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
