# WFL Command-Line Arguments

This document describes how command-line arguments are handled in WFL scripts.

## Overview

WFL scripts can accept command-line arguments passed after the script filename. These arguments are automatically parsed and made available to the script through built-in variables.

## Usage

```bash
wfl script.wfl [arguments...]
```

Example:
```bash
wfl myapp.wfl --name "John Doe" --verbose file1.txt file2.txt
```

## Available Variables

When a WFL script runs, the following variables are automatically defined:

### `arg_count`
- Type: `number`
- Description: Total number of arguments passed to the script
- Example: `display "Total arguments: " with arg_count`

### `args`
- Type: `list`
- Description: List containing all arguments in the order they were provided
- Example:
  ```wfl
  for each arg in args
      display "Argument: " with arg
  end for
  ```

### `positional_args`
- Type: `list`
- Description: List containing only positional arguments (non-flag arguments)
- Example:
  ```wfl
  for each file in positional_args
      display "Processing file: " with file
  end for
  ```

### Flag Variables
- Type: `text` or `boolean`
- Description: Flags are automatically converted to variables with the prefix `flag_`
- Long flags (--flag) and short flags (-f) are both supported
- If a flag has a value, the variable contains that value as text
- If a flag has no value, the variable contains `true`
- If a flag is not provided, the variable is undefined

Examples:
- `--verbose` creates `flag_verbose` with value `true`
- `--name John` creates `flag_name` with value `"John"`
- `-o output.txt` creates `flag_o` with value `"output.txt"`

## Argument Parsing Rules

1. **Flags**: Arguments starting with `-` or `--` are treated as flags
2. **Flag Values**: If a flag is followed by a non-flag argument, that argument becomes the flag's value
3. **Boolean Flags**: Flags without values are set to `true`
4. **Positional Arguments**: All non-flag arguments that aren't consumed as flag values
5. **Order Matters**: Flag values are consumed greedily. Place positional arguments after all flags or use `--` to separate them

## Examples

### Basic Example
```wfl
// Display all arguments
display "Arguments: " with arg_count
for each arg in args
    display "  " with arg
end for
```

### Checking for Flags
```wfl
// Check if verbose mode is enabled
if flag_verbose then
    display "Verbose mode enabled"
end if

// Check if output file is specified
if flag_output then
    display "Output will be written to: " with flag_output
end if
```

### Processing Files
```wfl
// Process all positional arguments as files
if length of positional_args is 0 then
    display "No files specified"
else
    for each file in positional_args
        display "Processing: " with file
        // Process the file here
    end for
end if
```

### Complete Example
```wfl
// args_example.wfl - A complete argument handling example

// Set defaults
store output_file as "output.txt"
store verbose as false

// Override defaults with command-line arguments
if flag_output then
    store output_file as flag_output
end if

if flag_verbose then
    store verbose as true
end if

// Process files
if length of positional_args is 0 then
    display "Usage: wfl args_example.wfl [options] file1 file2 ..."
    display "Options:"
    display "  --output FILE    Specify output file"
    display "  --verbose        Enable verbose mode"
else
    for each input_file in positional_args
        if verbose then
            display "Processing " with input_file with " -> " with output_file
        end if
        // Process the file here
    end for
end if
```

## Best Practices

1. **Check for undefined flags**: Always use `if flag_name then` before accessing flag values
2. **Provide defaults**: Set default values before checking for flags
3. **Show usage**: Display help when no arguments are provided
4. **Validate arguments**: Check that required arguments are present
5. **Use descriptive flag names**: Prefer `--output-file` over `-o` for clarity

## Notes

- The semantic analyzer will show warnings for undefined argument variables. This is expected since these variables are defined at runtime.
- Flag names are case-sensitive (`--verbose` is different from `--Verbose`)
- Multiple values for the same flag are not supported (last value wins)
- The special `--` argument to separate flags from positional arguments is not currently supported