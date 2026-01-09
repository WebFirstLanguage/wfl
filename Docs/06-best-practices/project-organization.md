# Project Organization

Organize WFL projects for maintainability and scalability. Good structure makes code easier to navigate and understand.

## Recommended Directory Structure

```
my-wfl-project/
├── src/
│   ├── main.wfl              # Entry point
│   ├── config.wfl            # Configuration
│   ├── utils.wfl             # Utility actions
│   └── models/               # Container definitions
│       ├── user.wfl
│       └── product.wfl
├── tests/
│   ├── test_main.wfl
│   ├── test_utils.wfl
│   └── test_models.wfl
├── data/
│   ├── config.txt
│   └── defaults.txt
├── logs/
│   └── app.log
├── .wflcfg                   # Style configuration
└── README.md                 # Project documentation
```

## Separation of Concerns

### One Responsibility Per File

**Good:**
```
user_manager.wfl       # User CRUD operations
email_sender.wfl       # Email functionality
validator.wfl          # Input validation
database.wfl           # Database operations
```

**Poor:**
```
app.wfl               # Everything in one file (thousands of lines)
```

### One Responsibility Per Action

**Good:**
```wfl
define action called validate_email
define action called send_email
define action called log_email_sent
```

**Poor:**
```wfl
define action called do_email_stuff:  // Does too much
    // Validates, sends, logs, formats, etc.
end action
```

## Configuration Management

### Use .wflcfg for Style

```ini
# .wflcfg
max_line_length = 100
max_nesting_depth = 5
indent_size = 4
```

### Use Config Files for Application Settings

**config.txt:**
```
PORT=8080
MAX_USERS=100
TIMEOUT=30
```

**Load in code:**
```wfl
open file at "config.txt" for reading as configfile
wait for store config_data as read content from configfile
close file configfile
// Parse config_data
```

## Modular Actions

**Extract reusable code:**

```wfl
// utils.wfl
define action called log_message with parameters message:
    open file at "logs/app.log" for appending as logfile
    store timestamp as current time in milliseconds
    wait for append content timestamp with ": " with message with "\n" into logfile
    close file logfile
end action

define action called format_currency with parameters amount:
    return "$" with round of amount
end action

// Use in main.wfl:
call log_message with "Application started"
store price_display as format_currency with 19.99
```

## Container Organization

**Group related data and behavior:**

```wfl
create container User:
    property id: Number
    property name: Text
    property email: Text

    action validate:
        // Validation logic here
    end

    action save:
        // Save to file/database
    end

    action to_string: Text
        return name with " <" with email with ">"
    end
end
```

## Testing Organization

**Mirror source structure:**

```
src/
  user_manager.wfl
  email_sender.wfl
tests/
  test_user_manager.wfl
  test_email_sender.wfl
```

## Documentation

### Project README

Every project should have:

```markdown
# Project Name

## What it does
Brief description

## Installation
How to run

## Usage
Example commands

## Configuration
.wflcfg and config files

## Testing
How to run tests
```

### Code Documentation

**File headers:**
```wfl
// user_validator.wfl
// Validates user registration data
// Created: 2026-01-09
```

**Action documentation:**
```wfl
// validate_email
// Checks if email format is valid
// Returns: yes if valid, no otherwise
define action called validate_email with parameters email:
    // Implementation
end action
```

## Best Practices

✅ **Separate concerns** - One file per responsibility
✅ **Organize by feature** - Not by type
✅ **Use configuration files** - Don't hardcode
✅ **Extract utilities** - Reusable actions in utils.wfl
✅ **Mirror test structure** - Tests match source
✅ **Document projects** - README for every project
✅ **Group with containers** - Related data and behavior

❌ **Don't put everything in one file**
❌ **Don't hardcode configuration**
❌ **Don't mix concerns** - Separate validation from processing
❌ **Don't skip documentation**

## What You've Learned

✅ Directory structure recommendations
✅ Separation of concerns
✅ Configuration management
✅ Modular action organization
✅ Container-based organization
✅ Testing organization
✅ Documentation requirements

**Next:** [Collaboration Guide →](collaboration-guide.md)

---

**Previous:** [← Testing Strategies](testing-strategies.md) | **Next:** [Collaboration Guide →](collaboration-guide.md)
