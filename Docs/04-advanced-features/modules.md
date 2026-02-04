# Modules and Code Organization

WFL's module system allows you to organize code across multiple files, enabling code reuse and better project structure. This guide covers how to load modules, understand scope behavior, and organize larger WFL projects.

## Table of Contents
- [Basic Module Loading](#basic-module-loading)
- [How Modules Work](#how-modules-work)
- [Scope and Variable Access](#scope-and-variable-access)
- [Nested Modules](#nested-modules)
- [Path Resolution](#path-resolution)
- [Error Handling](#error-handling)
- [Best Practices](#best-practices)
- [Limitations](#limitations)

## Basic Module Loading

Load code from another WFL file using the `load module from` statement:

```wfl
load module from "utilities.wfl"
```

This reads, parses, and executes the specified file. The module runs in its own scope but can access variables from the parent file.

### Simple Example

**helper.wfl:**
```wfl
store message as "Hello from helper module"
display message
```

**main.wfl:**
```wfl
display "Loading helper..."
load module from "helper.wfl"
display "Helper loaded"
```

**Output:**
```
Loading helper...
Hello from helper module
Helper loaded
```

## How Modules Work

When you load a module, WFL performs these steps:

1. **Resolves the path** - Finds the file relative to the current file's directory
2. **Reads the file** - Loads the module's source code
3. **Parses and validates** - Checks syntax, semantics, and types
4. **Executes in child scope** - Runs the module's statements
5. **Returns control** - Continues with the parent file

### Execution Pipeline

```
load module from "path.wfl"
         ↓
   Find file (relative to current file)
         ↓
   Read file content
         ↓
   Parse → Analyze → Type Check
         ↓
   Execute in child scope
         ↓
   Return to parent (continue execution)
```

## Scope and Variable Access

Modules use a **child scope with parent access** model:

### What Modules Can Do

✅ **Read parent variables:**
```wfl
# main.wfl
store config_path as "/etc/app/config"
load module from "loader.wfl"

# loader.wfl
display "Loading from: "
display config_path  # Can read parent variable
```

✅ **Execute with side effects:**
```wfl
# main.wfl
load module from "logger.wfl"

# logger.wfl
store log_file as "app.log"
create file at log_file with "Application started"
# File is created (side effect persists)
```

### What Modules Cannot Do

❌ **Define variables visible to parent:**
```wfl
# main.wfl
load module from "setup.wfl"
display utility_function  # Error: not defined

# setup.wfl
store utility_function as "some value"
# This variable is local to the module
```

❌ **Modify parent variables:**
```wfl
# main.wfl
store counter as 0
load module from "incrementer.wfl"
display counter  # Still 0

# incrementer.wfl
change counter to 1
# Creates local copy, doesn't modify parent
```

### Scope Isolation Example

```wfl
# main.wfl
store parent_var as "I'm in parent"

load module from "child.wfl"

# local_var is NOT accessible here
# display local_var would cause an error

display "Back in parent"
display parent_var  # Still accessible
```

```wfl
# child.wfl
store local_var as "I'm local to child"

# Can read parent variables
display parent_var  # Works: "I'm in parent"

# Can define local variables
display local_var  # Works: "I'm local to child"
```

**Output:**
```
I'm in parent
I'm local to child
Back in parent
I'm in parent
```

## Nested Modules

Modules can load other modules, creating a module hierarchy:

**main.wfl:**
```wfl
display "In main"
load module from "a.wfl"
display "Back in main"
```

**a.wfl:**
```wfl
display "In module A"
load module from "b.wfl"
display "Back in A"
```

**b.wfl:**
```wfl
display "In module B"
```

**Output:**
```
In main
In module A
In module B
Back in A
Back in main
```

### Circular Dependency Protection

WFL automatically detects circular dependencies:

**circular_a.wfl:**
```wfl
display "A loading B"
load module from "circular_b.wfl"
```

**circular_b.wfl:**
```wfl
display "B loading A"
load module from "circular_a.wfl"  # Error!
```

**Error message:**
```
Circular dependency detected:
  G:\project\circular_a.wfl →
  G:\project\circular_b.wfl →
  G:\project\circular_a.wfl
```

## Path Resolution

Module paths are resolved **relative to the including file's directory**, not the working directory.

### Directory Structure Example

```
project/
  main.wfl
  utils/
    helpers.wfl
    math.wfl
  config/
    settings.wfl
```

**main.wfl:**
```wfl
load module from "utils/helpers.wfl"
load module from "config/settings.wfl"
```

**utils/helpers.wfl:**
```wfl
# From helpers.wfl, load sibling file
load module from "math.wfl"

# Load from parent directory
load module from "../config/settings.wfl"
```

### Path Types

**Relative paths** (recommended):
```wfl
load module from "helper.wfl"           # Same directory
load module from "utils/math.wfl"       # Subdirectory
load module from "../shared/common.wfl" # Parent directory
```

**Absolute paths** (platform-specific):
```wfl
# Windows
load module from "C:/project/modules/utils.wfl"

# Linux/macOS
load module from "/home/user/project/modules/utils.wfl"
```

### Dynamic Paths

**Note:** While the WFL interpreter supports dynamic path expressions, the LSP (Language Server Protocol) and static analysis tools require string literals for validation. Dynamic paths work at runtime but may show warnings in editors.

Paths can be expressions at runtime:

```wfl
store module_name as "helper"
store module_path as module_name + ".wfl"
load module from module_path  # Works at runtime, LSP may warn
```

```wfl
store modules_dir as "plugins/"
store plugin as "logger"
load module from modules_dir + plugin + ".wfl"  # Works at runtime
```

**Recommendation:** Use string literals when possible for better IDE support:

```wfl
# ✅ Preferred - works everywhere
load module from "plugins/logger.wfl"

# ⚠️  Works at runtime, but LSP cannot validate
load module from plugin_path
```

## Error Handling

### File Not Found

```wfl
load module from "missing.wfl"
```

**Error:**
```
Cannot resolve module path 'missing.wfl':
  No such file or directory
```

### Parse Error in Module

If a module has syntax errors:

```wfl
load module from "broken.wfl"
```

**Error:**
```
Parse error in module 'broken.wfl':
  Unexpected token 'end' at line 5
```

### Runtime Error in Module

If a module fails during execution:

```wfl
load module from "faulty.wfl"
```

**Error:**
```
Error in module chain main.wfl → faulty.wfl:
  Undefined variable 'nonexistent'
  at line 10, column 8 in faulty.wfl
```

### Type Error in Module

If a module has type mismatches:

```wfl
load module from "typed_error.wfl"
```

**Error:**
```
Type error in module 'typed_error.wfl':
  Expected Integer, got Text
```

## Best Practices

### 1. Use Modules for Organization

Group related functionality:

```
project/
  main.wfl
  database/
    connection.wfl
    queries.wfl
  api/
    handlers.wfl
    routes.wfl
  utils/
    logging.wfl
    validation.wfl
```

### 2. Modules for Side Effects

Use modules for initialization and setup:

```wfl
# main.wfl
load module from "init_database.wfl"
load module from "load_config.wfl"
load module from "setup_logging.wfl"

# Now run main application logic
display "Application started"
```

### 3. Keep Modules Self-Contained

Modules should not rely on parent variables for validation:

**❌ Bad - Requires parent variable:**
```wfl
# config_loader.wfl
display config_path  # Assumes parent defines this
```

**✅ Good - Self-contained:**
```wfl
# config_loader.wfl
# Module can work independently or read from parent
store default_path as "/etc/config"
display "Config loading complete"
```

### 4. Document Module Dependencies

Add comments about what modules expect:

```wfl
# database_setup.wfl
# Expects parent to define:
#   - db_host (text)
#   - db_port (integer)
# Provides side effects:
#   - Creates database connection
#   - Initializes schema

display "Connecting to database..."
# ... rest of module
```

### 5. Use Relative Paths

Prefer relative paths for portability:

```wfl
# ✅ Good - works anywhere
load module from "utils/helpers.wfl"

# ❌ Bad - only works on specific machine
load module from "C:/Users/john/project/utils/helpers.wfl"
```

### 6. Avoid Circular Dependencies

Design module hierarchy carefully:

**❌ Bad:**
```
a.wfl → b.wfl → c.wfl → a.wfl  (circular)
```

**✅ Good:**
```
main.wfl → utils.wfl
        → config.wfl → shared.wfl
        → handlers.wfl → shared.wfl
```

### 7. Handle Errors Gracefully

Wrap module loading in try-catch for optional modules:

```wfl
try:
    load module from "optional_plugin.wfl"
    display "Plugin loaded successfully"
when error:
    display "Plugin not available, continuing without it"
end try
```

## Limitations

### Current Limitations (V1)

1. **No Export Mechanism**
   - Modules cannot expose variables/actions to parent
   - All definitions stay in module scope
   - Future: `export` keyword planned

2. **No Namespace Control**
   - Cannot load module with custom name
   - Future: `load module from "x.wfl" as name` planned

3. **No Selective Imports**
   - Must load entire module
   - Future: `load function1, function2 from "x.wfl"` planned

4. **No Module Caching**
   - Each `load module` re-parses and re-executes
   - Multiple loads of same file execute multiple times
   - Future: Optional caching planned

5. **Semantic Analysis Limitation**
   - Modules are analyzed independently
   - Cannot reference parent variables during analysis
   - Runtime execution works, but analysis may show warnings

### Workarounds

**For shared functionality**, use side effects:

```wfl
# Instead of exporting functions, use files

# logger.wfl
create file at "app.log" with ""

# main.wfl
load module from "logger.wfl"
# Now app.log exists and can be used
```

**For configuration**, use parent-to-module flow:

```wfl
# main.wfl
store api_key as "secret123"
store db_host as "localhost"

load module from "app_init.wfl"

# app_init.wfl reads these from parent scope
```

## Common Patterns

### Pattern 1: Initialization Modules

```wfl
# init_all.wfl
load module from "init_database.wfl"
load module from "init_cache.wfl"
load module from "init_logging.wfl"

# main.wfl
load module from "init_all.wfl"
display "All systems initialized"
```

### Pattern 2: Configuration Loading

```wfl
# main.wfl
store environment as "production"

load module from "load_config.wfl"

# load_config.wfl
check if environment is equal to "production":
    # Load production settings
otherwise:
    # Load development settings
end check
```

### Pattern 3: Feature Modules

```wfl
# main.wfl
store enable_analytics as true
store enable_logging as true

check if enable_analytics:
    load module from "features/analytics.wfl"
end check

check if enable_logging:
    load module from "features/logging.wfl"
end check
```

### Pattern 4: Plugin System

```wfl
# main.wfl
create list as plugins

push with plugins and "auth.wfl"
push with plugins and "cache.wfl"
push with plugins and "mailer.wfl"

for each plugin in plugins:
    try:
        load module from "plugins/" + plugin
        display "Loaded: " + plugin
    when error:
        display "Failed to load: " + plugin
    end try
end for
```

## Examples

### Example 1: Multi-File Application

**main.wfl:**
```wfl
display "Starting application..."

# Load configuration
load module from "config/app_config.wfl"

# Load utilities
load module from "utils/logger.wfl"

# Load core functionality
load module from "handlers/request_handler.wfl"

display "Application ready"
```

**config/app_config.wfl:**
```wfl
store app_name as "MyApp"
store version as "1.0.0"
display "Configuration loaded: " + app_name
```

**utils/logger.wfl:**
```wfl
create file at "app.log" with "=== Application Log ===\n"
display "Logger initialized"
```

**handlers/request_handler.wfl:**
```wfl
display "Request handler ready"
```

**Output:**
```
Starting application...
Configuration loaded: MyApp
Logger initialized
Request handler ready
Application ready
```

### Example 2: Conditional Module Loading

```wfl
store debug_mode as true
store database_enabled as true

check if debug_mode:
    load module from "debug_tools.wfl"
end check

check if database_enabled:
    load module from "database/init.wfl"
end check

display "Application configured"
```

### Example 3: Module Chain

**entry.wfl:**
```wfl
display "Step 1: Entry"
load module from "step2.wfl"
display "Step 5: Back to entry"
```

**step2.wfl:**
```wfl
display "Step 2: Processing"
load module from "step3.wfl"
display "Step 4: Back to step 2"
```

**step3.wfl:**
```wfl
display "Step 3: Core logic"
```

**Output:**
```
Step 1: Entry
Step 2: Processing
Step 3: Core logic
Step 4: Back to step 2
Step 5: Back to entry
```

## Future Enhancements

The following features are planned for future versions:

### Module Aliases (V2)
```wfl
load module from "utilities.wfl" as utils
# Access module namespace in future version
```

### Export Control (V2)
```wfl
# In module:
export variable helper_function
export constant VERSION

# Other definitions remain private
```

### Selective Imports (V3)
```wfl
load calculate, validate from "math_utils.wfl"
# Import only specific items
```

### Module Caching (V3)
```wfl
# Modules loaded once and cached
load module from "expensive.wfl"
load module from "expensive.wfl"  # Uses cached version
```

### Package System (V4)
```wfl
load module from "package:http-client"
load module from "package:json-parser"
```

## Summary

- Use `load module from "path.wfl"` to include other WFL files
- Modules execute in child scope with parent read access
- Paths resolve relative to the including file
- Circular dependencies are automatically detected
- Modules are best for initialization and side effects
- Variables defined in modules stay local to that module

Modules enable better code organization and reusability in WFL projects. Start with simple includes and build up to more complex module hierarchies as your project grows.
