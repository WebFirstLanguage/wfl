# WFL Container System Tutorial

## Overview

The WFL Container System provides object-oriented programming capabilities with natural language syntax. Containers are similar to classes in other languages but use intuitive English-like keywords and structures. This system supports inheritance, interfaces, events, static members, and encapsulation.

## Basic Container Concepts

### What is a Container?

A container is a blueprint for creating objects that have:
- **Properties**: Data that the container holds
- **Actions**: Functions that the container can perform  
- **Events**: Notifications that the container can send
- **Static members**: Shared data across all instances

Think of containers as templates for creating related objects with similar behavior.

## Creating Your First Container

### Basic Container Definition

```wfl
// Define a simple Person container
create container Person:
    property name: Text
    property age: Number
    
    action greet:
        display "Hello, I am " with name with " and I am " with age with " years old."
    end
end
```

### Creating Container Instances

```wfl
// Create a new instance of Person
create new Person as alice:
    name is "Alice"
    age is 28
end

// Use the container's action
alice.greet()  // Output: "Hello, I am Alice and I am 28 years old."
```

### Alternative Creation Syntax

```wfl
// More explicit creation syntax
create new Person as bob:
    set name to "Bob"
    set age to 35
end create

bob.greet()  // Output: "Hello, I am Bob and I am 35 years old."
```

## Working with Properties

### Property Types and Access

```wfl
create container BankAccount:
    property account_number: Text
    property balance: Number
    property is_active: Boolean
    
    action get_balance:
        return balance
    end
    
    action deposit with amount:
        check if amount > 0:
            store balance as balance + amount
            display "Deposited $" with amount with ". New balance: $" with balance
        otherwise:
            display "Invalid deposit amount"
        end
    end
    
    action withdraw with amount:
        check if amount > 0 and amount <= balance:
            store balance as balance - amount
            display "Withdrew $" with amount with ". New balance: $" with balance
        otherwise:
            display "Invalid withdrawal amount or insufficient funds"
        end
    end
end

// Usage
create new BankAccount as my_account:
    account_number is "123456789"
    balance is 1000.0
    is_active is yes
end

my_account.deposit with 250
my_account.withdraw with 100
store current_balance as my_account.get_balance
display "Current balance: $" with current_balance
```

### Property Validation

```wfl
create container Employee:
    property employee_id: Number
    property name: Text
    property salary: Number
    property department: Text
    
    action set_salary with new_salary:
        check if new_salary >= 0:
            store salary as new_salary
            display "Salary updated to $" with salary
        otherwise:
            display "Error: Salary cannot be negative"
        end
    end
    
    action give_raise with percentage:
        check if percentage > 0 and percentage <= 100:
            store raise_amount as salary * (percentage / 100)
            store salary as salary + raise_amount
            display name with " received a " with percentage with "% raise"
            display "New salary: $" with salary
        otherwise:
            display "Invalid raise percentage"
        end
    end
    
    action get_info:
        display "Employee: " with name
        display "ID: " with employee_id
        display "Department: " with department
        display "Salary: $" with salary
    end
end

// Usage
create new Employee as john:
    employee_id is 1001
    name is "John Smith"
    salary is 50000
    department is "Engineering"
end

john.get_info
john.give_raise with 10
john.set_salary with 60000
```

## Advanced Container Features

### Inheritance

Inheritance allows containers to extend other containers, inheriting their properties and actions.

```wfl
// Base container
create container Animal:
    property name: Text
    property species: Text
    property age: Number
    
    action make_sound:
        display name with " makes a generic animal sound"
    end
    
    action eat:
        display name with " is eating"
    end
    
    action describe:
        display "This is " with name with ", a " with age with "-year-old " with species
    end
end

// Derived container with inheritance
create container Dog extends Animal:
    property breed: Text
    property is_good_boy: Boolean
    
    // Override parent action
    action make_sound:
        display name with " barks: Woof! Woof!"
    end
    
    // New action specific to Dog
    action fetch:
        check if is_good_boy:
            display name with " happily fetches the ball!"
        otherwise:
            display name with " ignores the ball"
        end
    end
    
    // Override parent action with additional behavior
    action describe:
        parent describe  // Call parent version first
        display "Breed: " with breed
        display "Good boy status: " with is_good_boy
    end
end

// Usage
create new Dog as buddy:
    name is "Buddy"
    species is "Canine"
    age is 3
    breed is "Golden Retriever"
    is_good_boy is yes
end

buddy.describe     // Uses both parent and child versions
buddy.make_sound   // Uses overridden version
buddy.eat          // Uses inherited version
buddy.fetch        // Uses Dog-specific version
```

### Interfaces

Interfaces define contracts that containers must implement.

```wfl
// Define interfaces
create interface Drawable:
    requires action draw
    requires action resize with width and height
end interface

create interface Colorable:
    requires action set_color with new_color
    requires action get_color
end interface

// Base container
create container Shape:
    property x: Number
    property y: Number
    
    action move with new_x and new_y:
        store x as new_x
        store y as new_y
        display "Shape moved to (" with x with ", " with y with ")"
    end
end

// Container implementing multiple interfaces
create container Rectangle extends Shape implements Drawable and Colorable:
    property width: Number
    property height: Number  
    property color: Text
    
    // Implement Drawable interface
    action draw:
        display "Drawing a " with color with " rectangle at (" with x with ", " with y with ")"
        display "Dimensions: " with width with " x " with height
    end
    
    action resize with new_width and new_height:
        store width as new_width
        store height as new_height
        display "Rectangle resized to " with width with " x " with height
    end
    
    // Implement Colorable interface
    action set_color with new_color:
        store color as new_color
        display "Rectangle color changed to " with color
    end
    
    action get_color:
        return color
    end
    
    // Additional methods
    action get_area:
        return width * height
    end
end

// Usage
create new Rectangle as my_rect:
    x is 10
    y is 20
    width is 100
    height is 50
    color is "blue"
end

my_rect.draw
my_rect.resize with 120 and 60
my_rect.set_color with "red"
my_rect.move with 15 and 25
my_rect.draw

store area as my_rect.get_area
display "Rectangle area: " with area
```

## Events and Event Handling

Events allow containers to notify other parts of the program when something happens.

```wfl
create container Button:
    property label: Text
    property is_enabled: Boolean
    property click_count: Number
    
    // Define events
    event clicked
    event double_clicked
    event enabled_changed
    
    action initialize with button_label:
        store label as button_label
        store is_enabled as yes
        store click_count as 0
    end
    
    action click:
        check if is_enabled:
            store click_count as click_count + 1
            
            check if click_count >= 2:
                trigger double_clicked
                store click_count as 0  // Reset for next double-click
            otherwise:
                trigger clicked
            end
        otherwise:
            display "Button '" with label with "' is disabled"
        end
    end
    
    action enable:
        check if not is_enabled:
            store is_enabled as yes
            trigger enabled_changed
            display "Button '" with label with "' enabled"
        end
    end
    
    action disable:
        check if is_enabled:
            store is_enabled as no
            trigger enabled_changed
            display "Button '" with label with "' disabled"
        end
    end
end

// Create button instance
create new Button as submit_btn:
    label is "Submit"
    is_enabled is yes
    click_count is 0
end

// Set up event handlers
on submit_btn clicked:
    display "Submit button was clicked!"
    display "Processing form..."
end on

on submit_btn double_clicked:
    display "Submit button was double-clicked!"
    display "Quick submit activated!"
end on

on submit_btn enabled_changed:
    check if submit_btn.is_enabled:
        display "Submit button is now available"
    otherwise:
        display "Submit button is now unavailable"
    end
end on

// Test event handling
submit_btn.click        // Triggers 'clicked' event
submit_btn.click        // Triggers 'double_clicked' event
submit_btn.disable      // Triggers 'enabled_changed' event
submit_btn.click        // No event (button disabled)
submit_btn.enable       // Triggers 'enabled_changed' event
```

## Static Members and Class-Level Data

Static members belong to the container class itself, not individual instances.

```wfl
create container Car:
    // Static properties (shared across all instances)
    static property total_cars_created: Number is 0
    static property manufacturer: Text is "WFL Motors"
    
    // Instance properties
    property model: Text
    property year: Number
    property mileage: Number
    
    // Constructor-like method
    action initialize with car_model and car_year:
        store model as car_model
        store year as car_year
        store mileage as 0
        
        // Update static property
        store Car.total_cars_created as Car.total_cars_created + 1
        
        display "New car created: " with year with " " with model
        display "Total cars created: " with Car.total_cars_created
    end
    
    // Static method
    static action get_company_info:
        display "Manufacturer: " with Car.manufacturer
        display "Total cars produced: " with Car.total_cars_created
    end
    
    // Instance methods
    action drive with miles:
        store mileage as mileage + miles
        display "Drove " with miles with " miles. Total mileage: " with mileage
    end
    
    action get_info:
        display "Car: " with year with " " with model
        display "Mileage: " with mileage with " miles"
        display "Made by: " with Car.manufacturer
    end
end

// Create instances
create new Car as car1:
    initialize with "Sedan" and 2023
end

create new Car as car2:
    initialize with "SUV" and 2024
end

// Use static method
Car.get_company_info

// Use instance methods
car1.drive with 150
car2.drive with 200

car1.get_info
car2.get_info

display "Company produced " with Car.total_cars_created with " cars total"
```

## Real-World Examples

### Library Management System

```wfl
// Book container
create container Book:
    property title: Text
    property author: Text
    property isbn: Text
    property is_available: Boolean
    property due_date: Date
    
    action checkout with borrower_name and return_date:
        check if is_available:
            store is_available as no
            store due_date as return_date
            display "Book '" with title with "' checked out to " with borrower_name
            display "Due date: " with due_date
            return yes
        otherwise:
            display "Book '" with title with "' is not available"
            return no
        end
    end
    
    action return_book:
        check if not is_available:
            store is_available as yes
            store due_date as nothing
            display "Book '" with title with "' has been returned"
            return yes
        otherwise:
            display "Book '" with title with "' was not checked out"
            return no
        end
    end
    
    action get_info:
        display "Title: " with title
        display "Author: " with author
        display "ISBN: " with isbn
        display "Available: " with is_available
        check if not is_available:
            display "Due: " with due_date
        end
    end
end

// Library Member container
create container LibraryMember:
    property member_id: Number
    property name: Text
    property books_checked_out: List
    
    action initialize with id and member_name:
        store member_id as id
        store name as member_name
        store books_checked_out as []
    end
    
    action borrow_book with book:
        store future_date as add_days of today and 14  // 2 week loan
        store success as book.checkout with name and future_date
        
        check if success:
            push of books_checked_out and book
            display name with " successfully borrowed '" with book.title with "'"
        end
        
        return success
    end
    
    action return_book with book:
        store success as book.return_book
        
        check if success:
            // Remove book from checked out list (simplified)
            store book_index as indexof of books_checked_out and book
            check if book_index >= 0:
                // Would need list removal function for complete implementation
                display name with " returned '" with book.title with "'"
            end
        end
        
        return success
    end
    
    action list_borrowed_books:
        display name with "'s borrowed books:"
        check if length of books_checked_out is 0:
            display "  No books currently borrowed"
        otherwise:
            count book in books_checked_out:
                display "  - " with book.title with " (due: " with book.due_date with ")"
            end
        end
    end
end

// Usage
create new Book as book1:
    title is "The WFL Programming Guide"
    author is "Jane Smith"
    isbn is "978-0123456789"
    is_available is yes
end

create new Book as book2:
    title is "Advanced Container Patterns"
    author is "John Doe"
    isbn is "978-9876543210"
    is_available is yes
end

create new LibraryMember as alice:
    initialize with 1001 and "Alice Johnson"
end

create new LibraryMember as bob:
    initialize with 1002 and "Bob Wilson"
end

// Library operations
alice.borrow_book with book1
bob.borrow_book with book1     // Should fail - book not available
bob.borrow_book with book2     // Should succeed

alice.list_borrowed_books
bob.list_borrowed_books

// Return books
alice.return_book with book1
bob.borrow_book with book1     // Now should succeed

book1.get_info
book2.get_info
```

### Game Character System

```wfl
// Base character interface
create interface Combatant:
    requires action attack with target
    requires action take_damage with amount
    requires action is_alive
end interface

// Base character container
create container Character implements Combatant:
    property name: Text
    property health: Number
    property max_health: Number
    property level: Number
    
    // Events
    event health_changed
    event level_up
    event died
    
    action initialize with character_name and starting_health:
        store name as character_name
        store health as starting_health
        store max_health as starting_health
        store level as 1
    end
    
    action take_damage with amount:
        store old_health as health
        store health as health - amount
        
        check if health < 0:
            store health as 0
        end
        
        trigger health_changed
        display name with " takes " with amount with " damage! Health: " with health with "/" with max_health
        
        check if health is 0 and old_health > 0:
            trigger died
            display name with " has been defeated!"
        end
    end
    
    action heal with amount:
        store old_health as health
        store health as health + amount
        
        check if health > max_health:
            store health as max_health
        end
        
        check if health is not old_health:
            trigger health_changed
            display name with " heals for " with amount with " health! Health: " with health with "/" with max_health
        end
    end
    
    action is_alive:
        return health > 0
    end
    
    action gain_experience with exp:
        display name with " gains " with exp with " experience"
        // Simplified leveling system
        check if exp >= (level * 100):
            store level as level + 1
            store health as health + 10
            store max_health as max_health + 10
            trigger level_up
            display name with " levels up to level " with level with "!"
        end
    end
end

// Warrior specialization
create container Warrior extends Character:
    property armor: Number
    property strength: Number
    
    action initialize with warrior_name:
        parent initialize with warrior_name and 120
        store armor as 5
        store strength as 15
    end
    
    action attack with target:
        check if is_alive:
            store damage as strength + random * 10
            store rounded_damage as round of damage
            display name with " attacks " with target.name with " for " with rounded_damage with " damage!"
            target.take_damage with rounded_damage
        otherwise:
            display name with " cannot attack while defeated"
        end
    end
    
    action take_damage with amount:
        store reduced_damage as amount - armor
        check if reduced_damage < 0:
            store reduced_damage as 0
        end
        
        check if reduced_damage < amount:
            display name with "'s armor reduces damage by " with (amount - reduced_damage)
        end
        
        parent take_damage with reduced_damage
    end
    
    action defensive_stance:
        store armor as armor + 3
        display name with " takes a defensive stance! Armor increased to " with armor
    end
end

// Mage specialization  
create container Mage extends Character:
    property mana: Number
    property max_mana: Number
    property spell_power: Number
    
    action initialize with mage_name:
        parent initialize with mage_name and 80
        store mana as 100
        store max_mana as 100
        store spell_power as 20
    end
    
    action attack with target:
        check if is_alive and mana >= 10:
            store mana as mana - 10
            store damage as spell_power + random * 15
            store rounded_damage as round of damage
            display name with " casts a spell at " with target.name with " for " with rounded_damage with " damage!"
            display name with " has " with mana with "/" with max_mana with " mana remaining"
            target.take_damage with rounded_damage
        check if not is_alive:
            display name with " cannot cast spells while defeated"
        otherwise:
            display name with " doesn't have enough mana to cast a spell"
        end
    end
    
    action restore_mana:
        store mana as max_mana
        display name with " restores all mana!"
    end
end

// Create characters
create new Warrior as conan:
    initialize with "Conan the Barbarian"
end

create new Mage as gandalf:
    initialize with "Gandalf the Grey"
end

// Set up event handlers
on conan health_changed:
    check if conan.health <= 30:
        display "Warning: " with conan.name with " is badly wounded!"
    end
end on

on gandalf level_up:
    store gandalf.max_mana as gandalf.max_mana + 20
    store gandalf.mana as gandalf.max_mana
    display gandalf.name with " gains more mana! Now has " with gandalf.max_mana with " max mana"
end on

// Combat simulation
display "=== BATTLE BEGINS ==="
conan.attack with gandalf
gandalf.attack with conan
conan.defensive_stance
gandalf.attack with conan
conan.attack with gandalf

// Experience and healing
conan.gain_experience with 150
gandalf.heal with 20
gandalf.restore_mana

display "\n=== FINAL STATUS ==="
display conan.name with " - Health: " with conan.health with "/" with conan.max_health with ", Level: " with conan.level
display gandalf.name with " - Health: " with gandalf.health with "/" with gandalf.max_health with ", Level: " with gandalf.level
```

## Best Practices

### 1. Design Clear Hierarchies

```wfl
// Good: Clear inheritance hierarchy
create container Vehicle:
    property make: Text
    property model: Text
    property year: Number
end

create container LandVehicle extends Vehicle:
    property wheels: Number
end

create container Car extends LandVehicle:
    property doors: Number
end

// Bad: Confusing hierarchy
create container Animal:
    property name: Text
end

create container Car extends Animal:  // Makes no sense!
    property doors: Number
end
```

### 2. Use Interfaces for Contracts

```wfl
// Good: Define clear contracts
create interface Printable:
    requires action print
end interface

create interface Serializable:
    requires action to_json
    requires action from_json with data
end interface

create container Document implements Printable and Serializable:
    // Must implement both interfaces
end
```

### 3. Handle Events Appropriately

```wfl
// Good: Meaningful events
create container FileManager:
    event file_created
    event file_deleted
    event error_occurred
    
    action create_file with filename:
        // File creation logic
        trigger file_created
    end
end

// Bad: Too many trivial events
create container Counter:
    event incremented
    event decremented
    event value_checked  // Probably unnecessary
end
```

### 4. Use Static Members Wisely

```wfl
// Good: Shared configuration and counters
create container DatabaseConnection:
    static property max_connections: Number is 10
    static property active_connections: Number is 0
    
    action connect:
        check if DatabaseConnection.active_connections < DatabaseConnection.max_connections:
            // Connect logic
            store DatabaseConnection.active_connections as DatabaseConnection.active_connections + 1
        otherwise:
            display "Too many active connections"
        end
    end
end

// Bad: Instance data as static
create container Person:
    static property name: Text  // Should be instance property!
end
```

## Error Handling in Containers

```wfl
create container SafeContainer:
    property data: Text
    
    action safe_operation with input:
        try:
            // Potentially risky operation
            store data as process_input of input
            display "Operation successful"
            return yes
            
        when invalid_input:
            display "Error: Invalid input provided"
            return no
            
        otherwise:
            display "Unexpected error occurred"
            return no
        end try
    end
    
    action validate_state:
        check if isnothing of data:
            display "Warning: Container has no data"
            return no
        end
        
        check if length of data is 0:
            display "Warning: Container has empty data"
            return no
        end
        
        return yes
    end
end
```

## Performance Considerations

1. **Container Creation**: Creating many instances can be memory-intensive
2. **Event Handling**: Too many events can slow down execution
3. **Inheritance Depth**: Deep inheritance chains can impact performance
4. **Static Members**: Shared across all instances, use memory efficiently

## See Also

- [Core Module](core-module.md) - Basic utilities for container operations
- [List Module](list-module.md) - Working with container collections
- [WFL Language Reference](../language-reference/wfl-spec.md) - Complete language specification
- [WFL Variables](../language-reference/wfl-variables.md) - Variable and property management
- [WFL Control Flow](../language-reference/wfl-control-flow.md) - Conditional logic in containers

<function_calls>
<invoke name="TodoWrite">
<parameter name="todos">[{"id": "1", "content": "Update main standard library reference with missing functions and better examples", "status": "completed", "priority": "high"}, {"id": "2", "content": "Create core module guide with basic utilities documentation", "status": "completed", "priority": "high"}, {"id": "3", "content": "Create math module guide with comprehensive numeric function examples", "status": "completed", "priority": "high"}, {"id": "4", "content": "Create text module guide with string manipulation examples", "status": "completed", "priority": "high"}, {"id": "5", "content": "Create list module guide with collection operation examples", "status": "completed", "priority": "high"}, {"id": "6", "content": "Create time module guide with date/time functions and formatting", "status": "completed", "priority": "high"}, {"id": "7", "content": "Create filesystem module guide with file I/O operations", "status": "completed", "priority": "high"}, {"id": "8", "content": "Create async/await patterns guide", "status": "completed", "priority": "medium"}, {"id": "9", "content": "Create container system tutorial", "status": "completed", "priority": "medium"}]