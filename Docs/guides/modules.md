# WFL Module System

The WFL module system allows you to organize your code across multiple files, making it easier to build large applications, create reusable libraries, and maintain clean code structure.

## Table of Contents

- [Basic Usage](#basic-usage)
- [Syntax](#syntax)
- [Path Resolution](#path-resolution)
- [How Imports Work](#how-imports-work)
- [Import Behavior](#import-behavior)
- [Error Handling](#error-handling)
- [Best Practices](#best-practices)
- [Examples](#examples)
- [Limitations](#limitations)

## Basic Usage

### Simple Import

To import code from another WFL file, use the `load module from` statement:

```wfl
load module from "helper.wfl"
```

After importing, all variables, actions (functions), and containers (classes) defined in the imported file become available in your current file.

### Example

**helper.wfl:**
```wfl
define action called greet with name:
    display "Hello, " with name with "!"
end action

store GREETING_PREFIX as "Welcome, "
```

**main.wfl:**
```wfl
load module from "helper.wfl"

call greet with "Alice"
display GREETING_PREFIX with "Bob"
```

**Output:**
```
Hello, Alice!
Welcome, Bob
```

## Syntax

WFL supports two syntax forms for importing:

### Full Syntax (Recommended)
```wfl
load module from "path/to/file.wfl"
```

### Simplified Syntax
```wfl
load "path/to/file.wfl"
```

Both forms work identically. The full syntax is more explicit and reads like natural English.

## Path Resolution

WFL resolves import paths in the following order:

### 1. Relative to Importing File (Primary)

Paths are first resolved relative to the directory containing the importing file:

```wfl
// In G:\projects\app\src\main.wfl
load module from "utils.wfl"              // Looks for G:\projects\app\src\utils.wfl
load module from "../lib/helpers.wfl"    // Looks for G:\projects\app\lib\helpers.wfl
```

### 2. Relative to Working Directory (Fallback)

If not found relative to the file, WFL tries the current working directory:

```wfl
load module from "shared/constants.wfl"  // Tries <cwd>/shared/constants.wfl
```

### Path Examples

**Parent Directory:**
```wfl
load module from "../config.wfl"
```

**Subdirectory:**
```wfl
load module from "lib/helper.wfl"
```

**Deep Nesting:**
```wfl
load module from "../../shared/utils/math.wfl"
```

**Cross-Platform Compatibility:**
- Always use forward slashes (`/`) in paths, even on Windows
- WFL handles path conversion automatically

## How Imports Work

### Parse-Time Processing

WFL processes imports during parsing, not at runtime. This means:

1. When the parser encounters `load module from`, it immediately:
   - Resolves the file path
   - Reads and parses the imported file
   - Inlines the imported statements into the current program

2. The interpreter sees a single, flattened program with all imports resolved

This approach provides:
- **Better error reporting**: All errors known before execution
- **Type safety**: Type checker sees all definitions
- **Performance**: No runtime overhead for imports

### Execution Order

Imports are processed in the order they appear:

```wfl
display "Before import"
load module from "config.wfl"
display "After import"

// Variables from config.wfl are now available
check if DEBUG_MODE:
    display "Debug enabled"
end check
```

## Import Behavior

### Global Namespace

All imported definitions go into the global namespace. This means:

```wfl
// helper.wfl
store APP_NAME as "MyApp"

define action called calculate with x:
    give back x times 2
end action

// main.wfl
load module from "helper.wfl"

display APP_NAME              // Works!
store result as call calculate with 5  // Works!
```

### Import Once

Each file is imported only once, even if imported multiple times:

```wfl
load module from "helper.wfl"
load module from "helper.wfl"  // Second import is skipped
```

This prevents:
- Duplicate execution of initialization code
- Variable redefinition errors
- Performance issues

### Nested Imports (Transitive)

If file A imports file B, and file B imports file C, then A has access to everything from both B and C:

```wfl
// constants.wfl
store PI as 3.14159

// math.wfl
load module from "constants.wfl"
store TAU as PI times 2

// main.wfl
load module from "math.wfl"
display PI    // Works! PI is available
display TAU   // Works! TAU is available
```

### Diamond Dependencies

WFL handles diamond dependencies correctly:

```
    Main
    /  \
   A    B
    \  /
     C
```

If both A and B import C, C is only imported once (when first encountered).

## Error Handling

### Circular Dependency Detection

WFL automatically detects and prevents circular imports:

```wfl
// file_a.wfl
load module from "file_b.wfl"

// file_b.wfl
load module from "file_a.wfl"  // ERROR!
```

**Error Message:**
```
error[ERROR]: Circular dependency detected: file_b.wfl -> file_a.wfl -> file_b.wfl

Suggestion: Reorganize your code to break the circular dependency.
Consider creating a shared file for common definitions.
```

### Missing File Errors

If an imported file doesn't exist, WFL shows helpful error messages:

```wfl
load module from "nonexistent.wfl"
```

**Error Message:**
```
error[ERROR]: Cannot find module 'nonexistent.wfl'

Searched in:
  • /path/to/current/directory/nonexistent.wfl (relative to importing file)
  • /working/directory/nonexistent.wfl (relative to working directory)

Suggestion: Check the file path and ensure the file exists.
Make sure to include the .wfl extension.
```

### Syntax Errors in Imports

If an imported file has syntax errors, they're reported with the correct file context:

```wfl
load module from "helper.wfl"  // helper.wfl has errors
```

**Error Message:**
```
error[ERROR]: Error parsing imported file 'helper.wfl': Expected 'end action' after action definition

  helper.wfl:5:1
```

## Best Practices

### 1. Organize by Purpose

Structure your files by functionality:

```
project/
  ├── main.wfl          # Application entry point
  ├── config/
  │   ├── constants.wfl # Application constants
  │   └── settings.wfl  # Configuration
  ├── utils/
  │   ├── math.wfl      # Math utilities
  │   └── text.wfl      # Text utilities
  └── lib/
      └── helpers.wfl   # Helper functions
```

### 2. Place Imports at the Top

Always place imports at the beginning of your files:

```wfl
// Good
load module from "config.wfl"
load module from "utils.wfl"

display "Starting application..."
```

```wfl
// Avoid
display "Starting application..."
load module from "config.wfl"  // Import after code
```

### 3. Use Descriptive File Names

Use clear, descriptive names for your modules:

```wfl
// Good
load module from "user_authentication.wfl"
load module from "database_helpers.wfl"
load module from "api_constants.wfl"

// Avoid
load module from "util.wfl"
load module from "helper.wfl"
load module from "stuff.wfl"
```

### 4. Avoid Circular Dependencies

Design your dependency hierarchy to be acyclic:

```
✓ Good:
  main.wfl → helpers.wfl → constants.wfl

✗ Bad:
  main.wfl → helpers.wfl → constants.wfl → helpers.wfl
```

If you need shared code between modules, extract it to a third file:

```
Before (circular):
  auth.wfl ⇄ user.wfl

After (no circular):
  auth.wfl → shared.wfl ← user.wfl
```

### 5. Keep Files Focused

Each file should have a single, clear purpose:

```wfl
// constants.wfl - Only constants
store MAX_USERS as 1000
store TIMEOUT_SECONDS as 30

// validators.wfl - Only validation functions
define action called validate_email with email:
    // Validation logic
end action

// auth.wfl - Only authentication logic
load module from "constants.wfl"
load module from "validators.wfl"
// Authentication implementation
```

## Examples

### Example 1: Configuration File

**config.wfl:**
```wfl
store APP_NAME as "MyApp"
store VERSION as "1.0.0"
store DEBUG_MODE as yes
store MAX_CONNECTIONS as 100
```

**main.wfl:**
```wfl
load module from "config.wfl"

display APP_NAME with " v" with VERSION

check if DEBUG_MODE:
    display "Debug mode enabled"
    display "Max connections: " with MAX_CONNECTIONS
end check
```

### Example 2: Utility Functions

**string_utils.wfl:**
```wfl
define action called trim with text:
    // Trim whitespace
    give back text
end action

define action called capitalize with text:
    // Capitalize first letter
    give back text
end action
```

**main.wfl:**
```wfl
load module from "string_utils.wfl"

store name as trim with "  Alice  "
store formatted as capitalize with name
display formatted
```

### Example 3: Multi-Layer Architecture

**models/user.wfl:**
```wfl
create container User:
    property name: Text
    property email: Text
    property age: Number
end
```

**services/user_service.wfl:**
```wfl
load module from "models/user.wfl"

define action called create_user with name and email:
    create new User as user:
        name: name
        email: email
        age: 0
    end
    give back user
end action
```

**main.wfl:**
```wfl
load module from "services/user_service.wfl"

store alice as call create_user with "Alice" and "alice@example.com"
display alice
```

### Example 4: Shared Constants

**constants.wfl:**
```wfl
store PI as 3.14159
store E as 2.71828
store SQRT2 as 1.41421
```

**math_utils.wfl:**
```wfl
load module from "constants.wfl"

define action called circle_area with radius:
    give back PI times radius times radius
end action
```

**physics_utils.wfl:**
```wfl
load module from "constants.wfl"

define action called exponential_growth with rate and time:
    // Uses E constant
    display "Computing growth..."
end action
```

**main.wfl:**
```wfl
load module from "math_utils.wfl"
load module from "physics_utils.wfl"

// Both modules imported constants.wfl, but it's only loaded once
store area as call circle_area with 5
display "Circle area: " with area
```

## Limitations

### Current Limitations

1. **No Selective Imports**: You cannot import specific items from a file
   ```wfl
   // Not supported yet:
   // load action greet from "helper.wfl"
   ```

2. **No Namespaces**: All imports go into global namespace
   ```wfl
   // Not supported yet:
   // helper.greet()
   ```

3. **No Module Aliasing**: Cannot rename imports
   ```wfl
   // Not supported yet:
   // load module from "helper.wfl" as h
   ```

4. **No Standard Library Path**: Cannot import from a standard library location
   ```wfl
   // Not supported yet:
   // load module from "@wfl/http"
   ```

### Future Enhancements

These features may be added in future versions:
- Selective imports
- Module namespaces
- Import aliasing
- Standard library modules
- Package management

## Summary

The WFL module system provides:

✅ Simple, natural syntax: `load module from "file.wfl"`
✅ Automatic path resolution
✅ Circular dependency detection
✅ Import caching
✅ Nested imports
✅ Clear error messages
✅ Backward compatibility

Use it to organize your code, create reusable libraries, and build maintainable WFL applications!

## See Also

- [Language Reference](../wfldocs/language-reference.md)
- [Best Practices](best-practices.md)
- [Standard Library](../wfldocs/stdlib-reference.md)
