# WFL Containers (Classes): Structuring Data and Behavior Together

**Summary:** In this section, we'll explore **containers** in WebFirst Language (WFL) – a powerful way to organize related data and actions into cohesive units. Containers are similar to what other languages call "classes" but with WFL's natural language approach. They allow you to model real-world entities by bundling properties (data) and behaviors (actions) together. By the end of this guide, you'll understand how to create containers, define their properties and actions, create instances, and use inheritance to build relationships between different types of containers – all using WFL's friendly, English-like syntax.

## What Are Containers?

A **container** is a blueprint or template that defines a type of object in your program. Think of it as a custom data type that you design to represent something specific in your application – like a User, Product, Vehicle, or any other concept that has both data (properties) and behavior (actions).

**Why use containers?** As your programs grow more complex, organizing related data and functionality together becomes essential. Containers help you:

- **Group related data and actions** in one place
- **Create multiple instances** with the same structure but different values
- **Encapsulate complexity** by hiding implementation details
- **Model real-world entities** in a natural way

## Creating a Basic Container

In WFL, you create a container using the `create container` syntax. Here's a simple example:

```wfl
create container Person:
    // Properties (data)
    property name as text
    property age as number
    property email as text
    
    // Actions (behavior)
    define action greet:
        display "Hello, my name is " with name
    end action
    
    define action has birthday:
        add 1 to age
        display name with " is now " with age with " years old"
    end action
end container
```

This defines a `Person` container with:
- Three properties: `name`, `age`, and `email`
- Two actions: `greet` and `has birthday`

The container itself is just a blueprint – it doesn't represent any specific person yet. To create actual people based on this blueprint, you need to create instances.

## Creating and Using Container Instances

An **instance** is a specific object created from a container. If the container is a blueprint for a house, an instance is an actual house built from that blueprint.

Here's how to create instances of our `Person` container:

```wfl
create new Person as alice:
    set name to "Alice Smith"
    set age to 28
    set email to "alice@example.com"
end create

create new Person as bob:
    set name to "Bob Johnson"
    set age to 35
    set email to "bob@example.com"
end create
```

Now we have two `Person` instances: `alice` and `bob`. Each has its own set of property values.

To access properties or call actions on an instance, use the dot notation or possessive form:

```wfl
// Accessing properties
display alice's name        // Displays: Alice Smith
display bob's age           // Displays: 35

// Calling actions
alice greet                 // Displays: Hello, my name is Alice Smith
bob has birthday            // Displays: Bob Johnson is now 36 years old
```

Notice how natural this syntax is – `alice greet` reads like giving a command to Alice, and `bob has birthday` describes an event happening to Bob.

## Container Initialization and Constructors

When creating a new instance, you often need to set up its initial state. WFL provides a special action called `initialize` that runs automatically when an instance is created:

```wfl
create container Product:
    property name as text
    property price as number
    property in stock as yes
    
    define action initialize with product name and product price:
        set name to product name
        set price to product price
        display "Created new product: " with name
    end action
    
    define action mark as sold out:
        set in stock to no
    end action
end container
```

Now you can create a product with initial values in one step:

```wfl
create new Product with "Smartphone" and 499.99 as phone
```

This creates a new `Product` instance called `phone` with name "Smartphone" and price 499.99, and displays "Created new product: Smartphone".

## Properties with Validation and Default Values

You can make your containers more robust by adding validation rules and default values to properties:

```wfl
create container BankAccount:
    property account number as text:
        must not be empty
        must be exactly 10 characters
    end property
    
    property balance as number:
        must be at least 0
        defaults to 0
    end property
    
    property owner name as text
    
    define action deposit amount:
        check amount:
            must be greater than 0
        end check
        
        add amount to balance
        display "Deposited " with amount with ". New balance: " with balance
    end action
    
    define action withdraw amount:
        check amount:
            must be greater than 0
            must be less than or equal to balance
        end check
        
        subtract amount from balance
        display "Withdrew " with amount with ". New balance: " with balance
    end action
end container
```

This `BankAccount` container includes:
- Validation for `account number` (must be exactly 10 characters)
- Validation for `balance` (must be non-negative) with a default value of 0
- Actions that include their own validation logic

When you try to create an instance with invalid data or call an action with invalid parameters, WFL will provide clear error messages.

## Private and Public Members

In larger applications, you might want to control which properties and actions are accessible from outside the container. WFL allows you to mark members as `private` or `public`:

```wfl
create container User:
    // Public properties (accessible from anywhere)
    public property username as text
    public property display name as text
    
    // Private properties (only accessible within the container)
    private property password hash as text
    private property login attempts as number defaults to 0
    
    public define action greet:
        display "Hello, " with display name
    end action
    
    private define action hash password with raw password:
        // Implementation details hidden
        store password hash as secure hash of raw password
    end action
    
    public define action set password with new password:
        check new password:
            must be at least 8 characters
            must have uppercase letter
            must have number
        end check
        
        hash password with new password
        display "Password updated successfully"
    end action
end container
```

In this example:
- `username` and `display name` are public properties that can be accessed from anywhere
- `password hash` and `login attempts` are private properties that can only be accessed within the container
- `greet` and `set password` are public actions that can be called from anywhere
- `hash password` is a private action that can only be called by other actions within the container

This encapsulation helps protect sensitive data and implementation details.

## Container Inheritance

**Inheritance** allows you to create a new container based on an existing one. The new container (called a **child** or **subcontainer**) inherits all the properties and actions of the original container (called the **parent** or **supercontainer**), and can add its own or override existing ones.

Here's an example:

```wfl
create container Vehicle:
    property make as text
    property model as text
    property year as number
    
    define action describe:
        display year with " " with make with " " with model
    end action
end container

create container Car extends Vehicle:
    property number of doors as number defaults to 4
    property fuel type as text defaults to "gasoline"
    
    // Override the parent's describe action
    define action describe:
        // Call the parent's version first
        parent describe
        display "This car has " with number of doors with " doors and runs on " with fuel type
    end action
    
    define action honk:
        display "Beep beep!"
    end action
end container
```

Now you can create a `Car` instance that has all the properties and actions from both containers:

```wfl
create new Car as my car:
    set make to "Toyota"
    set model to "Corolla"
    set year to 2025
    set fuel type to "hybrid"
end create

my car describe
// Displays:
// 2025 Toyota Corolla
// This car has 4 doors and runs on hybrid

my car honk  // Displays: Beep beep!
```

Inheritance helps you build hierarchies of related containers, promoting code reuse and logical organization.

## Container Composition

While inheritance creates "is-a" relationships (a Car is a Vehicle), **composition** creates "has-a" relationships. This means one container can include instances of other containers as properties:

```wfl
create container Engine:
    property horsepower as number
    property cylinders as number
    
    define action start:
        display "Engine started with " with horsepower with " HP"
    end action
end container

create container Car:
    property make as text
    property model as text
    property engine as Engine  // Composition: a Car has an Engine
    
    define action start:
        display "Starting " with make with " " with model
        engine start  // Delegate to the engine's start action
    end action
end container
```

To use this:

```wfl
create new Engine as v6:
    set horsepower to 280
    set cylinders to 6
end create

create new Car as sports car:
    set make to "Nissan"
    set model to "370Z"
    set engine to v6
end create

sports car start
// Displays:
// Starting Nissan 370Z
// Engine started with 280 HP
```

Composition is powerful because it allows you to build complex objects from simpler ones, creating a modular design.

## Static Properties and Actions

Sometimes you want properties or actions that belong to the container itself, not to individual instances. These are called **static** members:

```wfl
create container MathUtils:
    static property PI as 3.14159
    
    static define action calculate circle area with radius:
        provide PI times radius times radius
    end action
end container
```

You can use static members directly through the container, without creating an instance:

```wfl
display MathUtils PI  // Displays: 3.14159

store area as MathUtils calculate circle area with 5
display "The area is " with area  // Displays: The area is 78.53975
```

Static members are useful for utility functions, constants, or tracking information across all instances of a container.

## Container Events and Callbacks

WFL containers can define and respond to events, allowing for reactive programming:

```wfl
create container Button:
    property label as text
    property is enabled as yes
    
    // Define events that this container can trigger
    event clicked
    event hover start
    event hover end
    
    define action click:
        if is enabled:
            trigger clicked  // Fire the clicked event
            display "Button '" with label with "' was clicked"
        end if
    end action
    
    define action on hover:
        trigger hover start
        display "Hovering over button: " with label
    end action
    
    define action end hover:
        trigger hover end
        display "No longer hovering over button: " with label
    end action
end container
```

Other parts of your code can listen for and respond to these events:

```wfl
create new Button as submit button:
    set label to "Submit"
end create

// Set up event handlers
on submit button clicked:
    submit form
end on

on submit button hover start:
    show tooltip with "Click to submit the form"
end on
```

This event-driven approach is particularly useful for user interfaces and asynchronous operations.

## Container Interfaces and Polymorphism

An **interface** defines a contract that containers can implement. It specifies a set of actions that a container must provide, without dictating how they should be implemented:

```wfl
create interface Drawable:
    requires action draw
    requires action resize with width and height
end interface

create container Circle implements Drawable:
    property radius as number
    property color as text
    
    define action draw:
        display "Drawing a " with color with " circle with radius " with radius
    end action
    
    define action resize with width and height:
        set radius to minimum of width and height divided by 2
        display "Circle resized to radius " with radius
    end action
end container

create container Rectangle implements Drawable:
    property width as number
    property height as number
    property color as text
    
    define action draw:
        display "Drawing a " with color with " rectangle " with width with "x" with height
    end action
    
    define action resize with new width and new height:
        set width to new width
        set height to new height
        display "Rectangle resized to " with width with "x" with height
    end action
end container
```

The power of interfaces comes from **polymorphism** – the ability to treat different types of objects uniformly as long as they implement the same interface:

```wfl
create list shapes:
    add new Circle with radius 5 and color "red"
    add new Rectangle with width 10 and height 20 and color "blue"
end list

for each shape in shapes:
    shape draw  // Works for both Circle and Rectangle
    shape resize with 30 and 30
    shape draw  // Shows the updated shapes
end for
```

This code works because both `Circle` and `Rectangle` implement the `Drawable` interface, guaranteeing they both have `draw` and `resize` actions.

## Best Practices for Containers

To write effective and maintainable containers, consider these best practices:

1. **Single Responsibility Principle**: Each container should have a single, well-defined purpose. If a container is doing too many things, consider splitting it into multiple containers.

2. **Meaningful Names**: Choose clear, descriptive names for containers, properties, and actions that reflect their purpose.

3. **Proper Encapsulation**: Use private members to hide implementation details and expose only what's necessary through public members.

4. **Validation**: Add validation rules to properties and action parameters to ensure data integrity.

5. **Documentation**: Include comments that explain the purpose of the container and any non-obvious behavior.

6. **Favor Composition Over Inheritance**: While inheritance is powerful, it can create tight coupling. When possible, use composition to build complex objects from simpler ones.

7. **Keep Inheritance Hierarchies Shallow**: Deep inheritance hierarchies can become difficult to understand and maintain. Try to limit inheritance to 2-3 levels.

## Common Container Patterns

Here are some common patterns you might use with containers:

### Model-View Pattern

Separate data (model) from its presentation (view):

```wfl
create container UserModel:
    property id as number
    property username as text
    property email as text
    property last login as date
    
    define action update last login:
        set last login to today
    end action
end container

create container UserView:
    property model as UserModel
    
    define action display user card:
        display "User: " with model username
        display "Email: " with model email
        display "Last login: " with model last login
    end action
    
    define action display compact:
        display model username with " (" with model email with ")"
    end action
end container
```

### Factory Pattern

Create a specialized container that knows how to create instances of other containers:

```wfl
create container VehicleFactory:
    static define action create sedan with make and model:
        create new Car as result:
            set make to make
            set model to model
            set number of doors to 4
            set vehicle type to "sedan"
        end create
        
        provide result
    end action
    
    static define action create truck with make and model and bed length:
        create new Truck as result:
            set make to make
            set model to model
            set bed length to bed length
        end create
        
        provide result
    end action
end container

// Usage
store my car as VehicleFactory create sedan with "Honda" and "Accord"
```

### Observer Pattern

Implement a publish-subscribe mechanism where objects can register interest in events:

```wfl
create container Subject:
    property observers as empty list
    
    define action add observer with observer:
        add observer to observers
    end action
    
    define action remove observer with observer:
        remove observer from observers
    end action
    
    define action notify with message:
        for each observer in observers:
            observer update with message
        end for
    end action
end container

create container ConcreteSubject extends Subject:
    property state as text
    
    define action change state to new state:
        set state to new state
        notify with "State changed to: " with state
    end action
end container

create container Observer:
    property name as text
    
    define action update with message:
        display name with " received: " with message
    end action
end container
```

## Conclusion

Containers in WFL provide a powerful way to organize your code by grouping related data and behavior together. They allow you to:

- Model real-world entities in a natural, intuitive way
- Create reusable blueprints for objects in your program
- Build relationships between different types of objects
- Encapsulate complexity and protect data integrity
- Structure your code in a modular, maintainable fashion

In this section, we've covered:
- Basic container creation and instantiation
- Properties with validation and default values
- Actions and initialization
- Access control with public and private members
- Inheritance and composition
- Static members and events
- Interfaces and polymorphism
- Best practices and common patterns

As you practice with containers, you'll find they become an essential tool for managing complexity in larger programs. They help you think about your code in terms of objects and their interactions, leading to more organized, maintainable, and intuitive code.

Happy coding!