# WFL Object-Oriented Programming Design

## Overview

This document outlines the design for implementing Python-style object-oriented programming features in WFL while maintaining its natural language philosophy. The goal is to provide the same capabilities as Python's OOP system (methods, inheritance, operator overloading) but expressed through WFL's English-like syntax.

## Core Principles

1. **Natural Language First**: All OOP features must be expressible in clear, English-like syntax
2. **No Special Characters**: Avoid dots, underscores, and other symbols in favor of prepositions and keywords
3. **Explicit Over Implicit**: Actions and relationships should be clearly stated
4. **Backward Compatibility**: All existing WFL programs must continue to work

## 1. Unified Actions: Natural Syntax for All Types

### Current State
WFL currently uses function-style calls for operations on data:
```wfl
store len as length(my_text)
store upper as to_uppercase(my_text)
```

### Proposed Enhancement
Introduce prepositional syntax that reads like natural English:

```wfl
// Text operations
get length of my_text as len
perform to_uppercase on my_text as upper_text
check if my_text contains "hello"

// List operations
add item to my_list
remove last from my_list
get first of my_list as head
reverse order of my_list

// Number operations
round down my_number as floor_value
get absolute of negative_num as positive
```

### Implementation Strategy

1. **Parser Updates**:
   - Recognize patterns: `(verb) (object) (preposition) (target) [with/and (arguments)]`
   - Support variations: `get X of Y`, `perform X on Y`, `add X to Y`, etc.
   - Create unified AST nodes for these operations

2. **Type Dispatch**:
   - Interpreter maps prepositional actions to appropriate stdlib functions
   - Type checker validates action availability for each type
   - Error messages guide users to valid actions for their data type

3. **Syntax Variations**:
   ```wfl
   // All equivalent:
   get length of my_text
   find length in my_text
   measure length for my_text
   ```

## 2. Enhanced Container Inheritance

### Current State
WFL supports basic container inheritance with `extends` keyword.

### Proposed Enhancement
Full inheritance system with method overriding and parent access:

```wfl
create container Animal:
    property name as text
    property age as number
    
    action make_sound:
        display "The animal makes a sound"
    end
    
    action describe:
        display name with " is " with age with " years old"
    end
end

create container Dog extends Animal:
    property breed as text
    
    // Override parent action
    action make_sound:
        display name with " barks!"
    end
    
    // Extend parent action
    action describe:
        perform parent describe
        display "Breed: " with breed
    end
end

create container GuideDog extends Dog:
    property handler as text
    
    action guide:
        display name with " guides " with handler
    end
end

// Usage
create new GuideDog as buddy
set buddy's name to "Buddy"
set buddy's age to 5
set buddy's breed to "Labrador"
set buddy's handler to "Alice"

perform make_sound on buddy    // "Buddy barks!"
perform describe on buddy      // Full inheritance chain
perform guide on buddy         // Specific to GuideDog
```

### Implementation Strategy

1. **Method Resolution Order (MRO)**:
   - Follow inheritance chain from child to parent
   - Stop at first matching action found
   - Cache lookups for performance

2. **Parent Access**:
   - `parent` keyword provides access to overridden actions
   - Can be chained: `perform parent parent action_name`
   - Validates parent action exists at parse time

3. **Protected vs Public**:
   - Actions starting with underscore are container-private
   - Public actions accessible through inheritance
   - Properties follow same visibility rules

## 3. Operator Overloading with Special Actions

### Design
Containers can define special actions that customize behavior for WFL operators:

```wfl
create container Vector:
    property x as number
    property y as number
    
    // Addition operator
    action on add (other):
        create new Vector as result:
            x = this.x plus other.x
            y = this.y plus other.y
        end
        give back result
    end
    
    // Subtraction operator
    action on subtract (other):
        create new Vector as result:
            x = this.x minus other.x
            y = this.y minus other.y
        end
        give back result
    end
    
    // Comparison operators
    action on compare (other):
        // Return -1, 0, or 1
        if this.x is less than other.x:
            give back -1
        else if this.x is greater than other.x:
            give back 1
        else if this.y is less than other.y:
            give back -1
        else if this.y is greater than other.y:
            give back 1
        else:
            give back 0
        end
    end
    
    // String representation
    action on display:
        give back "(" with x with ", " with y with ")"
    end
    
    // Containment check
    action on check contains (value):
        give back x is equal to value or y is equal to value
    end
end

// Usage
store v1 as new Vector with x 3 and y 4
store v2 as new Vector with x 1 and y 2

store v3 as v1 plus v2              // Calls 'on add'
display v3                          // Calls 'on display', shows "(4, 6)"

check if v1 is greater than v2      // Calls 'on compare'
check if v1 contains 3              // Calls 'on check contains'
```

### Special Action Reference

| Special Action | WFL Operator | Description |
|----------------|--------------|-------------|
| `on add (other)` | `plus` | Addition |
| `on subtract (other)` | `minus` | Subtraction |
| `on multiply (other)` | `times` | Multiplication |
| `on divide (other)` | `divided by` | Division |
| `on compare (other)` | `is equal to`, `is greater than`, etc. | Comparison |
| `on display` | `display` statement | String representation |
| `on check contains (item)` | `contains` | Membership test |
| `on get length` | `get length of` | Length/size |
| `on get item (index)` | `get item at X from` | Indexing |
| `on set item (index, value)` | `set item at X in Y to` | Index assignment |

### Implementation Strategy

1. **Operator Dispatch**:
   - When operator is used, check for special action on left operand
   - Fall back to built-in behavior if not found
   - Type checker ensures special actions have correct signatures

2. **Return Type Inference**:
   - Special actions must declare return types
   - Type system propagates these through expressions
   - Validation at compile time

## 4. First-Class Actions and Action Handles

### Design
Actions can be stored in variables and passed as arguments:

```wfl
create container Calculator:
    action add (a, b):
        give back a plus b
    end
    
    action multiply (a, b):
        give back a times b
    end
    
    action apply_operation (operation, x, y):
        // 'operation' is an action handle
        store result as perform operation with x and y
        give back result
    end
end

create new Calculator as calc

// Get action handles
store add_func as get action add from calc
store mult_func as get action multiply from calc

// Use action handles
store sum as perform add_func with 5 and 3        // 8
store product as perform mult_func with 4 and 6    // 24

// Pass action as argument
store result as perform apply_operation on calc with add_func and 10 and 20  // 30
```

### Advanced Features

```wfl
// Action handles from instances
create container Printer:
    property prefix as text
    
    action print_message (msg):
        display prefix with ": " with msg
    end
end

create new Printer as error_printer
set error_printer's prefix to "ERROR"

create new Printer as info_printer
set info_printer's prefix to "INFO"

// Get bound action handles
store print_error as get action print_message from error_printer
store print_info as get action print_message from info_printer

// Use them
perform print_error with "Something went wrong"    // "ERROR: Something went wrong"
perform print_info with "Process complete"          // "INFO: Process complete"

// Store in a list
store printers as list of print_error, print_info
for each printer in printers:
    perform printer with "Test message"
end
```

### Implementation Strategy

1. **Action Handle Type**:
   - New value type: `ActionHandle(container_instance, action_name, signature)`
   - Tracks both the action and its bound instance (if any)
   - Type system validates compatibility

2. **Syntax Support**:
   - `get action X from Y` creates handle
   - `perform handle_var [with args]` executes
   - Action handles are first-class values

## 5. Static Actions and Properties

### Design
Support for container-level (static) actions and properties:

```wfl
create container Math:
    static property PI as 3.14159
    static property E as 2.71828
    
    static action calculate_circle_area (radius):
        give back Math.PI times radius times radius
    end
end

// Access static members
display Math's PI
store area as perform calculate_circle_area on Math with 5
```

## 6. Interfaces and Contracts

### Design
Define behavioral contracts that containers must fulfill:

```wfl
define interface Drawable:
    requires action draw
    requires action get_bounds giving back rectangle
    requires property visible as boolean
end

define interface Clickable:
    requires action on_click (x, y)
    requires action is_point_inside (x, y) giving back boolean
end

create container Button implements Drawable, Clickable:
    property x as number
    property y as number
    property width as number
    property height as number
    property visible as boolean
    property text as text
    
    action draw:
        if visible:
            // Drawing implementation
            display "Drawing button: " with text
        end
    end
    
    action get_bounds:
        create new rectangle as bounds:
            x = this.x
            y = this.y
            width = this.width
            height = this.height
        end
        give back bounds
    end
    
    action on_click (click_x, click_y):
        display "Button " with text with " clicked!"
    end
    
    action is_point_inside (px, py) giving back boolean:
        check if px is between x and x plus width
            and py is between y and y plus height
    end
end
```

## 7. Property Getters and Setters

### Design
Allow computed properties with custom logic:

```wfl
create container Temperature:
    property celsius as number
    
    // Computed property
    property fahrenheit:
        get:
            give back celsius times 9 divided by 5 plus 32
        end
        set (value):
            set celsius to (value minus 32) times 5 divided by 9
        end
    end
    
    property kelvin:
        get:
            give back celsius plus 273.15
        end
        // Read-only - no setter
    end
end

create new Temperature as temp
set temp's celsius to 0
display temp's fahrenheit    // 32
set temp's fahrenheit to 212
display temp's celsius       // 100
display temp's kelvin        // 373.15
```

## Implementation Phases

### Phase 1: Unified Actions (2 weeks)
- Parser updates for prepositional syntax
- Type dispatch system
- Update stdlib to support new syntax

### Phase 2: Enhanced Inheritance (2 weeks)
- Method resolution order
- Parent access mechanism
- Visibility modifiers

### Phase 3: Operator Overloading (3 weeks)
- Special action recognition
- Operator dispatch system
- Type system integration

### Phase 4: First-Class Actions (2 weeks)
- Action handle type
- Syntax support
- Type checking for action handles

### Phase 5: Advanced Features (3 weeks)
- Static members
- Interfaces
- Property getters/setters

## Testing Strategy

1. **Unit Tests**: Each feature gets comprehensive unit tests
2. **Integration Tests**: Test feature interactions
3. **Backward Compatibility**: All existing TestPrograms must pass
4. **Performance Tests**: Ensure no significant slowdown
5. **Error Message Tests**: Verify helpful error messages

## Migration Guide

Existing WFL code will continue to work. New features are additive:

```wfl
// Old style (still works)
store len as length(my_text)

// New style (recommended)
get length of my_text as len

// Both can coexist during migration
```

## Conclusion

This design brings the full power of object-oriented programming to WFL while maintaining its core philosophy of natural, readable syntax. The implementation will be done in phases to ensure stability and backward compatibility at each step.