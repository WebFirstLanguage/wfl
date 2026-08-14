# Containers (Object-Oriented Programming)

WFL supports object-oriented programming through **containers**—a natural way to organize code and data.

## What are Containers?

Containers are WFL's version of classes. They combine:
- **Properties** - Data fields
- **Actions** - Methods/functions
- **Events** - Announcements other code can react to
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

## Events

An **event** is an announcement a container makes. The container says *what
happened*; other code decides *what to do about it*. That keeps a container from
having to know about everything that cares — a `Button` announces that it was
clicked without knowing who is listening.

Three pieces make up the whole feature:

| Piece | Where it goes | What it does |
|---|---|---|
| `event <name>` | In a container body | Declares an event the container can announce |
| `trigger <name>` | In an action | Announces it — every registered handler runs |
| `on <name> of <instance>:` … `end on` | Anywhere | Registers a handler to run when it is announced |

### Declaring and Triggering an Event

Declare an event alongside the container's properties and actions, then
`trigger` it from an action:

```wfl
create container Button:
    property label: Text

    event on_click

    action click:
        display "Clicking " with label
        trigger on_click
    end
end
```

### Handling an Event

An `on` block registers a handler. It names the event first, then the instance
to listen to, and ends with `end on`:

```wfl
create new Button as save_button:
    label is "Save"
end

on on_click of save_button:
    display "Saving your work..."
end on

save_button.click()
```

```
Clicking Save
Saving your work...
```

The event name comes before the instance because WFL reads adjacent bare words
as a single name — `on save_button on_click` would be one name, with no way to
tell the button from the event. The word `of` keeps the two apart.

### Handlers Belong to One Instance

Registering a handler attaches it to *that instance*, not to the container as a
whole. Two buttons made from the same container listen independently:

```wfl
create new Button as save_button:
    label is "Save"
end

create new Button as cancel_button:
    label is "Cancel"
end

on on_click of save_button:
    display "Saving your work..."
end on

cancel_button.click()   // No handler registered — nothing extra happens
save_button.click()     // Runs the handler above
```

```
Clicking Cancel
Clicking Save
Saving your work...
```

### Several Handlers

An event can have any number of handlers. They all run, in the order they were
registered:

```wfl
on on_click of save_button:
    display "1. Validating..."
end on

on on_click of save_button:
    display "2. Writing to disk..."
end on

save_button.click()
```

```
Clicking Save
1. Validating...
2. Writing to disk...
```

A handler registered while an event is being dispatched runs on the *next*
trigger, not the one in progress.

### Passing Values with an Event

An event can declare values it carries, using `needs` — the same parameter
syntax actions use. `trigger` supplies them with `with`, and handlers refer to
them by the names the event declared:

```wfl
create container Slider:
    property value: Number

    event on_change needs old_value: Number, new_value: Number

    action set_to needs new_setting: Number:
        store previous as value
        store value as new_setting
        trigger on_change with previous and new_setting
    end
end

create new Slider as volume:
    value is 3
end

on on_change of volume:
    display "Volume moved from " with old_value with " to " with new_value
end on

volume.set_to(7)
```

```
Volume moved from 3 to 7
```

If a `trigger` supplies fewer values than the event declares, the remaining
parameters are `nothing`.

### Handlers Remember Where They Were Written

A handler body can use the variables that were in scope where it was
registered, and changes it makes to them are visible afterwards:

```wfl
create container Bell:
    event ring

    action strike:
        trigger ring
    end
end

create new Bell as dinner_bell:
end

store times_rung as []

on ring of dinner_bell:
    push with times_rung and "ring"
end on

dinner_bell.strike()
dinner_bell.strike()

display "Rang " with length of times_rung with " times"
```

```
Rang 2 times
```

### Inherited Events

A container that `extends` another inherits its events. A handler registered on
the child instance runs whether the trigger came from an inherited action or
one of the child's own:

```wfl
create container Machine:
    event started

    action power_on:
        trigger started
    end
end

create container Printer extends Machine:
    action warm_up:
        trigger started
    end
end

create new Printer as office_printer:
end

on started of office_printer:
    display "Printer is starting"
end on

office_printer.power_on()
office_printer.warm_up()
```

```
Printer is starting
Printer is starting
```

### Events Outside Containers

`event` also works on its own, without a container. Declare it, register a
handler with the short `on <name>:` form, and trigger it:

```wfl
event data_ready needs payload: Text

on data_ready:
    display "Received: " with payload
end on

trigger data_ready with "42 records"
```

```
Received: 42 records
```

### Event Errors

Events fail loudly rather than silently doing nothing:

- Registering a handler for an event the container does not declare is an error
  (`Event 'on_hover' not found in container 'Button'`), reported by the type
  checker before the program runs and again at runtime.
- `on <name> of <something that is not a container>:` is an error.
- A handler that triggers its own event forever stops with a
  `Maximum call depth exceeded` error rather than crashing.

### Events vs. Calling an Action

Use an action when the container should do the work itself. Use an event when
the container should announce something and let other code decide what happens
— especially when the number of interested parties can grow, or when the
container should not depend on them.

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
✅ **Events** - `event`, `trigger ... with ...`, and `on <event> of <instance>:`
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
