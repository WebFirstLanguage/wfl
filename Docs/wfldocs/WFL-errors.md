# WFL Error Handling and Diagnostics

## Overview

WFL's error handling system is designed to provide clear, actionable, and user-friendly error messages that help developers understand and fix issues quickly. The system is inspired by languages like Elm and Rust, which are known for their helpful and informative error reporting.

## Core Principles

### 1. **Clear and Human-Friendly Messages**
- Error messages are written in plain English, avoiding technical jargon
- Messages explain what went wrong in terms beginners can understand
- The focus is on the code being at fault, not the coder

### 2. **Actionable Guidance**
- Every error includes suggestions for how to fix it
- Messages provide context about why something is wrong
- Hints guide developers toward the correct solution

### 3. **Visual Context**
- Errors show the exact location in the code where the problem occurred
- Source code snippets are displayed with highlighting
- Line numbers and column positions are clearly indicated

### 4. **Scalable Complexity**
- Default messages are beginner-friendly
- Advanced users can access more detailed information
- Error codes allow looking up comprehensive documentation

## Error Categories

### Parse Errors (Syntax)
Errors that occur when the code doesn't follow WFL's syntax rules.

**Example: Missing 'as' keyword**
```
error: Expected 'as' after identifier(s), but found IntLiteral(42)
  --> example.wfl:3:14
   |
 3 | store greeting 42
   |              ^ Error occurred here
   |
   = Note: Did you forget to use 'as' before assigning a value? 
          For example: `store greeting as 42`
```

### Type Errors
Errors that occur when operations use incompatible types.

**Example: Type mismatch in operations**
```
error: Cannot add number and text - Expected Number but found Text
  --> example.wfl:3:12
   |
 3 | display x plus y
   |            ^ Type error occurred here
   |
   = Note: Try converting the text to a number using 'convert to number'
```

### Semantic Errors
Errors related to the meaning of the code, such as undefined variables.

**Example: Undefined variable**
```
error: Variable 'countt' is not defined
  --> example.wfl:5:9
   |
 5 | display countt
   |         ^^^^^^ Semantic error occurred here
   |
   = Note: Did you misspell the variable name? Did you mean 'count'?
```

### Runtime Errors
Errors that occur during program execution.

**Example: Division by zero**
```
error: Division by zero
  --> example.wfl:7:14
   |
 7 | display 10 divided by x
   |              ^ Runtime error occurred here
   |
   = Note: Check your divisor to ensure it's never zero
```

## Error Message Format

Each error message consists of:

1. **Error Type and Description**: A brief statement of what went wrong
2. **Location Information**: File name, line number, and column
3. **Code Context**: The relevant source code with visual markers
4. **Helpful Note**: Suggestions for fixing the error

## Implementation Details

### Error Reporting System

The error reporting system uses the `codespan-reporting` library to format and display errors with source code snippets. The system maintains:

- **Structured Error Data**: Each error contains severity, message, location, and context
- **Source Mapping**: Errors track their position in the original source code
- **Diagnostic Formatting**: Errors are rendered with syntax highlighting and visual indicators

### Error Types in Code

```rust
// Example error structures
pub enum WflError {
    ParseError { 
        location: Span, 
        message: String,
        suggestion: Option<String>
    },
    TypeError { 
        location: Span, 
        expected: Type, 
        found: Type,
        context: String 
    },
    SemanticError { 
        location: Span, 
        message: String,
        similar_names: Vec<String>
    },
    RuntimeError { 
        location: Option<Span>, 
        message: String,
        help: Option<String>
    }
}
```

### Logging System

WFL includes a structured logging system that complements error handling:

**Log Levels**:
- **Detail** (Debug): Low-level information for debugging
- **Note** (Info): General runtime events
- **Caution** (Warning): Potential issues that aren't errors
- **Error**: Operation failures
- **Critical**: Severe failures requiring immediate attention

**Example Log Entry**:
```
Note: User successfully logged in.
Caution: The configuration file was not found, using defaults.
Error: Could not connect to the database — retrying in 5 seconds.
```

## Visual Debugging Tools

### Step-Through Debugger
- Set breakpoints with visual markers or `breakpoint here` comments
- Step through code line by line
- View variable values with descriptions
- Natural language explanations of code execution

### REPL Integration
- Errors don't crash the REPL
- Clear error messages with suggestions
- Ability to continue after errors

### Editor Integration (LSP)
- Real-time error highlighting
- Hover for error details
- Quick fixes and suggestions
- Error codes for detailed documentation lookup

## Multilingual Support

WFL supports error messages in multiple languages:
- Error templates are stored with placeholders
- Language-specific message files provide translations
- Technical terms remain consistent across languages
- User's locale determines the display language

## Error Recovery

WFL includes smart error recovery to:
- Continue parsing after errors to find multiple issues
- Provide context-aware suggestions
- Avoid cascading errors from a single mistake
- Maintain program state in REPL after errors

## Best Practices for Error Messages

### Writing Error Messages
1. Use simple, clear language
2. Be specific about what went wrong
3. Always provide a suggestion or next step
4. Avoid technical jargon unless necessary
5. Use a friendly, helpful tone

### Examples of Good Error Messages
- ✅ "The value needs to be a number — try converting it first"
- ✅ "I don't recognize 'pubic'. Did you mean 'public'?"
- ✅ "It looks like something's missing in your loop — did you forget 'end'?"

### Examples to Avoid
- ❌ "Invalid syntax"
- ❌ "Type error: incompatible types"
- ❌ "Illegal operation"

## Future Enhancements

- **AI-Assisted Error Resolution**: Integration with AI to suggest fixes
- **Error Pattern Learning**: Track common errors to improve messages
- **Interactive Error Resolution**: Step-by-step guided fixes
- **Community Error Database**: Shared solutions for common problems

## Error Codes Reference

Each error has a unique code for detailed documentation:
- **WFL-001** to **WFL-099**: Parse errors
- **WFL-100** to **WFL-199**: Type errors
- **WFL-200** to **WFL-299**: Semantic errors
- **WFL-300** to **WFL-399**: Runtime errors
- **WFL-400** to **WFL-499**: I/O errors
- **WFL-500** to **WFL-599**: Async/concurrency errors

Advanced users can look up error codes for comprehensive explanations and edge cases.