# WFL Main Loop

## Overview

The `main loop` is a special looping construct in WFL designed for long-running applications such as servers, game engines, event processors, and other programs that need to run indefinitely without triggering timeout errors.

## Key Features

- **Timeout Override**: Main loops automatically disable the execution timeout, allowing programs to run indefinitely
- **Graceful Exit**: Supports standard control flow statements (`break`, `continue`, `exit`, `return`)
- **Server-Ready**: Perfect for implementing server-like applications, event loops, and interactive programs
- **Backward Compatible**: Does not affect existing WFL programs or other loop types

## Syntax

```wfl
main loop:
    // Loop body
    // Runs indefinitely until explicitly terminated
end loop
```

## Difference from Forever Loop

While both `main loop` and `forever loop` create infinite loops, they have different behaviors:

| Feature | Main Loop | Forever Loop |
|---------|-----------|--------------|
| Timeout | Disabled (overrides timeout) | Subject to timeout |
| Use Case | Long-running services | Regular infinite loops |
| Purpose | Servers, event processors | Game loops with timeout safety |

## Examples

### Basic Main Loop

```wfl
store counter as 0

main loop:
    store counter as counter plus 1
    display "Iteration: " with counter
    
    // Exit condition
    check if counter is 100:
        break
    end check
end loop
```

### Server Application

```wfl
store server_running as true
store requests_processed as 0

display "Starting server..."

main loop:
    // Process request
    store requests_processed as requests_processed plus 1
    display "Processing request #" with requests_processed
    
    // Handle shutdown signal
    check if requests_processed is 1000:
        display "Shutting down server..."
        break
    end check
end loop

display "Server stopped after " with requests_processed with " requests"
```

### Event Processor

```wfl
store events_queue as 0
store processing as true

main loop:
    // Simulate event arrival
    store events_queue as events_queue plus 1
    
    // Process event
    display "Processing event: " with events_queue
    
    // Check for stop condition
    check if events_queue is 50:
        store processing as false
    end check
    
    check if processing is false:
        display "Stopping event processor..."
        break
    end check
end loop
```

## Control Flow

The main loop supports all standard control flow statements:

### Break Statement
Exits the main loop immediately:

```wfl
main loop:
    check if should_exit:
        break  // Exit the loop
    end check
end loop
```

### Continue Statement
Skips to the next iteration:

```wfl
main loop:
    check if skip_this:
        continue  // Skip to next iteration
    end check
    // Rest of loop body
end loop
```

### Return Statement
Returns from the containing action:

```wfl
define action called server:
    main loop:
        check if error_occurred:
            give "Error occurred"  // Return from action
        end check
    end loop
end action
```

## Best Practices

1. **Always Include Exit Conditions**: While main loops bypass timeouts, always include a way to exit the loop gracefully
2. **Monitor Resource Usage**: Since main loops can run indefinitely, ensure proper resource management
3. **Use for Appropriate Tasks**: Reserve main loops for truly long-running tasks that need to bypass timeouts
4. **Consider User Interruption**: In production, provide mechanisms for users to stop the program (Ctrl+C handling)

## Implementation Notes

- Main loops set an internal flag that disables timeout checking
- The timeout is re-enabled when the main loop exits
- Nested main loops are possible but not recommended
- The timeout override only affects the current execution context

## Use Cases

Main loops are ideal for:
- Web servers and API servers
- Game engines with continuous update loops
- Event-driven applications
- Message queue processors
- Real-time data processors
- Interactive REPL-like applications
- Monitoring and logging services
- Background job processors

## Configuration Interaction

When a main loop is active:
- The `timeout_seconds` setting in `.wflcfg` is ignored for that execution context
- Other timeout-subject code outside the main loop still respects the timeout
- The timeout is automatically restored when the main loop exits