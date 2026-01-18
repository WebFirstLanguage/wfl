# WFL Testing Guide

WFL includes a built-in testing framework with natural language syntax that makes it easy to write and run tests for your WFL programs.

## Table of Contents

- [Getting Started](#getting-started)
- [Writing Your First Test](#writing-your-first-test)
- [Test Structure](#test-structure)
- [Assertions](#assertions)
- [Setup and Teardown](#setup-and-teardown)
- [Running Tests](#running-tests)
- [Best Practices](#best-practices)

## Getting Started

The WFL testing framework allows you to write tests using familiar natural language syntax. Tests are organized in `describe` blocks and individual `test` blocks, similar to popular testing frameworks like Jest, RSpec, or PHPUnit.

## Writing Your First Test

Create a file with a `.wfl` extension (commonly `.test.wfl` for test files):

```wfl
describe "Basic arithmetic":

    test "addition works correctly":
        store result as 5 plus 3
        expect result to equal 8
    end test

    test "subtraction works correctly":
        store result as 10 minus 4
        expect result to equal 6
    end test

end describe
```

Run your tests with:

```bash
wfl --test my_tests.wfl
```

## Test Structure

### Describe Blocks

Describe blocks organize related tests together. They provide context and can be nested:

```wfl
describe "Calculator":

    describe "Addition":
        test "handles positive numbers":
            store result as 2 plus 3
            expect result to equal 5
        end test
    end describe

    describe "Subtraction":
        test "handles negative results":
            store result as 3 minus 8
            expect result to be less than 0
        end test
    end describe

end describe
```

### Test Blocks

Each `test` block represents a single test case:

```wfl
test "description of what is being tested":
    // Test code goes here
    store actual as some calculation
    expect actual to equal expected_value
end test
```

**Important**: Each test runs in an isolated environment. Variables created in one test do not affect other tests.

## Assertions

WFL provides natural language assertions that read like English:

### Equality

```wfl
expect value to equal 42
expect message to be "hello"  // 'be' is a synonym for 'equal'
```

### Comparisons

```wfl
expect score to be greater than 50
expect age to be less than 100
```

### Truthiness

```wfl
expect condition to be yes  // Truthy check
expect flag to be no        // Falsy check
```

### Existence

```wfl
expect value to exist
```

### Collections

```wfl
expect numbers to contain 5
expect list to be empty
expect items to have length 3
```

### Text

```wfl
expect message to contain "error"
expect text to be empty
expect word to have length 5
```

### Type Checking

```wfl
expect value to be of type "Number"
expect message to be of type "Text"
expect items to be of type "List"
```

## Setup and Teardown

Use `setup` and `teardown` blocks to run code before and after tests in a describe block:

```wfl
describe "Database operations":

    setup:
        display "Connecting to test database..."
        store test_data as [1, 2, 3, 4, 5]
    end setup

    test "can read data":
        expect test_data to have length 5
    end test

    test "can process data":
        store sum as 0
        for each item in test_data:
            change sum to sum plus item
        end for
        expect sum to equal 15
    end test

    teardown:
        display "Cleaning up test database..."
    end teardown

end describe
```

**Note**: Setup and teardown run once per describe block, not before/after each individual test.

## Running Tests

### Basic Test Execution

```bash
wfl --test my_tests.wfl
```

### Test Output

The test runner provides clear, formatted output:

```
============================================================
Test Results
============================================================
Total:  10
Passed: 9 ✓
Failed: 1 ✗

────────────────────────────────────────────────────────────
Failures:
────────────────────────────────────────────────────────────

1. addition handles large numbers
   Context: Calculator > Addition
   Expected value to equal 1000, but got 999
   at line 45

============================================================
```

### Exit Codes

- **0**: All tests passed
- **1**: One or more tests failed

This allows integration with CI/CD pipelines.

## Best Practices

### 1. Organize Tests by Feature

```wfl
describe "User authentication":
    test "validates email format": ... end test
    test "requires password": ... end test
    test "creates session on success": ... end test
end describe
```

### 2. Use Descriptive Test Names

Good:
```wfl
test "rejects invalid email addresses":
```

Bad:
```wfl
test "test 1":
```

### 3. Test One Thing Per Test

Each test should verify one specific behavior:

```wfl
// Good - tests one thing
test "addition returns correct sum":
    store result as 2 plus 3
    expect result to equal 5
end test

// Avoid - tests multiple things
test "math operations":
    expect 2 plus 3 to equal 5
    expect 10 minus 4 to equal 6
    expect 3 times 4 to equal 12
end test
```

### 4. Avoid Reserved Keywords as Variable Names

Some words are reserved for the testing framework:
- `empty` - Use `empty_string`, `empty_list`, etc.
- `type` - Use `data_type`, `value_type`, etc.
- `length` - Use `list_length`, `text_length`, etc.

### 5. Keep Tests Independent

Don't rely on test execution order. Each test should work in isolation:

```wfl
// Good - each test is independent
describe "Counter":
    test "increments from zero":
        store counter as 0
        change counter to counter plus 1
        expect counter to equal 1
    end test

    test "decrements from initial value":
        store counter as 5
        change counter to counter minus 1
        expect counter to equal 4
    end test
end describe
```

### 6. Use Setup for Common Initialization

```wfl
describe "List operations":
    setup:
        store test_list as [1, 2, 3, 4, 5]
    end setup

    test "list has correct length":
        expect test_list to have length 5
    end test

    test "list contains all values":
        expect test_list to contain 3
    end test
end describe
```

## Complete Example

Here's a comprehensive example showing all features:

```wfl
describe "Shopping Cart":

    setup:
        display "Setting up test cart..."
        store cart_items as []
    end setup

    test "starts empty":
        expect cart_items to be empty
        expect cart_items to have length 0
    end test

    test "can add items":
        add "apple" to cart_items
        add "banana" to cart_items
        expect cart_items to have length 2
        expect cart_items to contain "apple"
    end test

    test "total price calculation":
        store price as 10 plus 20 plus 30
        expect price to equal 60
        expect price to be greater than 50
    end test

    teardown:
        display "Cleaning up test cart..."
    end teardown

end describe
```

## Troubleshooting

### Tests Not Running

- Ensure you're using the `--test` flag: `wfl --test myfile.wfl`
- Check that your file has test blocks inside describe blocks
- Verify all `describe` and `test` blocks are properly closed with `end describe` and `end test`

### Assertion Failures

- Check the error message for the expected vs. actual values
- Verify variable names are spelled correctly
- Ensure you're using the correct assertion type for your data

### Parse Errors

- Make sure you're not using reserved keywords as variable names
- Check that all blocks are properly closed
- Verify the syntax of your assertions matches the examples above

## Next Steps

- Explore the [WFL Language Basics](../03-language-basics/README.md) for more on WFL syntax
- Check out example test files in the `TestPrograms/` directory
- Learn about [Best Practices](../06-best-practices/README.md) for writing maintainable code

---

For questions or issues, please refer to the [WFL documentation](../README.md) or file an issue on GitHub.
