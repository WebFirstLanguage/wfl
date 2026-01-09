# Keyword Reference

Complete alphabetical list of all WFL reserved keywords.

**Total:** 60+ reserved keywords

## Control Flow Keywords

- `break` - Exit loop early
- `check` - Start conditional
- `continue` - Skip to next iteration
- `downward` - Count loop direction
- `each` - For each loop
- `end` - Close block
- `exit` - Exit loop
- `for` - For each loop
- `forever` - Infinite loop
- `from` - Count loop start
- `if` - Conditional
- `in` - For each collection
- `loop` - Loop reference
- `otherwise` - Else clause
- `repeat` - Loop construct
- `reversed` - Reverse iteration
- `skip` - Continue (alias)
- `to` - Count loop end
- `until` - Loop condition
- `while` - Loop condition

## Declaration Keywords

- `action` - Function definition
- `called` - Action name
- `change` - Modify variable
- `container` - Class definition
- `create` - Create resource
- `define` - Define action
- `extends` - Inheritance
- `implements` - Interface
- `interface` - Interface definition
- `list` - List type
- `map` - Map type
- `needs` - Action parameters (alias)
- `new` - Instantiate container
- `parameters` - Action parameters
- `pattern` - Pattern definition
- `property` - Container field
- `static` - Static member
- `store` - Create variable
- `with` - Parameter separator

## Operation Keywords

- `add` - Add to list/number
- `append` - Append to file
- `call` - Call action
- `close` - Close resource
- `display` - Output
- `divide` - Division (alias)
- `execute` - Run command
- `file` - File resource
- `kill` - Terminate process
- `listen` - Start server
- `multiply` - Multiplication (alias)
- `open` - Open resource
- `pop` - Remove from list
- `push` - Add to list
- `read` - Read operation
- `remove` - Remove item
- `respond` - Send response
- `return` - Return value
- `spawn` - Start process
- `subtract` - Subtraction (alias)
- `wait` - Async wait
- `write` - Write operation

## Comparison Keywords

- `and` - Logical AND
- `as` - Binding/casting
- `at` - Location preposition
- `by` - Step amount
- `equal` - Equality
- `greater` - Greater comparison
- `is` - Comparison/equality
- `less` - Less comparison
- `not` - Logical NOT/inequality
- `of` - Possession
- `on` - Location
- `or` - Logical OR
- `than` - Comparison

## Value Keywords

- `false` - Boolean false (alias)
- `missing` - Null (alias)
- `nothing` - Null
- `no` - Boolean false
- `true` - Boolean true (alias)
- `undefined` - Null (alias)
- `yes` - Boolean true

## Special Keywords

- `catch` - Error handler
- `current` - Current value (time/loop)
- `date` - Date type
- `directory` - Directory resource
- `error` - Error reference
- `event` - Event definition
- `finally` - Cleanup block
- `process` - Process resource
- `request` - Web request
- `time` - Time type
- `trigger` - Fire event
- `try` - Error handling
- `when` - Specific error type

## Usage Notes

### Cannot Use as Variable Names

**Wrong:**
```wfl
store is as 10             // ERROR
store file as "data.txt"   // ERROR
store add as 5             // ERROR
```

**Right:**
```wfl
store is_valid as 10
store filename as "data.txt"
store addition as 5
```

### Contextual Keywords

Some keywords are contextual (meaning depends on context):
- `current` - In time context vs loop context
- `of` - Possession vs function call
- `with` - Parameters vs concatenation

---

**When in doubt, use underscores or different words to avoid conflicts.**

**[See Variables Guide →](../03-language-basics/variables-and-types.md#reserved-keywords)**

---

**Previous:** [← Syntax Reference](syntax-reference.md) | **Next:** [Operator Reference →](operator-reference.md)
