# Testing Strategies

Testing ensures your WFL code works correctly. This guide covers testing approaches for WFL applications.

## Test-Driven Development (TDD)

**WFL follows TDD: Write tests FIRST, then implementation.**

### TDD Workflow

1. **Write failing test** - Define expected behavior
2. **Run test** - Verify it fails
3. **Write minimum code** - Make test pass
4. **Run test** - Verify it passes
5. **Refactor** - Improve code
6. **Repeat**

### Example

**1. Write a failing test (test_calculator.wfl):**
```wfl
// The implementation is just a stub for now — it returns the wrong value
define action called add_numbers with parameters x and y:
    return 0
end action

define action called test_addition:
    store result as add_numbers of 2 and 3
    check if result is equal to 5:
        display "✓ Addition test passed"
    otherwise:
        display "✗ Addition test failed"
    end check
end action

call test_addition
```

**2. Run (fails — the stub returns 0, not 5)**

**3. Implement `add_numbers` correctly:**
```wfl
define action called add_numbers with parameters x and y:
    return x plus y
end action
```

**4. Run (passes)**

## Test Organization

### TestPrograms Directory

WFL uses `TestPrograms/` for end-to-end tests:

```
TestPrograms/
├── basic_syntax_comprehensive.wfl
├── file_io_comprehensive.wfl
├── error_handling_comprehensive.wfl
└── ...
```

**Every feature has a comprehensive test program.**

### Test File Naming

- `*_comprehensive.wfl` - Complete feature tests
- `*_test.wfl` - Specific feature tests
- `simple_*.wfl` - Minimal examples

## Testing Patterns

### Pattern 1: Assertion Testing

```wfl
define action called assert_equals with parameters actual and expected and message:
    check if actual is equal to expected:
        display "✓ " with message
        return yes
    otherwise:
        display "✗ " with message with " (expected: " with expected with ", got: " with actual with ")"
        return no
    end check
end action

// Define the actions we want to test
define action called add_numbers with parameters x and y:
    return x plus y
end action

define action called multiply_numbers with parameters x and y:
    return x times y
end action

// Use it (compute the value first, then assert on it)
store sum_result as add_numbers of 2 and 3
call assert_equals with sum_result and 5 and "Addition works"
store product_result as multiply_numbers of 4 and 5
call assert_equals with product_result and 20 and "Multiplication works"
```

### Pattern 2: Error Testing

**Test that errors occur when expected:**

```wfl
display "Testing error handling..."

store error_caught as no

try:
    store result as 10 divided by 0
    display "✗ Should have thrown error"
when error:
    change error_caught to yes
end try

check if error_caught is yes:
    display "✓ Division by zero error caught correctly"
otherwise:
    display "✗ Error was not caught"
end check
```

### Pattern 3: Edge Case Testing

**Test boundaries:**

```wfl
define action called assert_equals with parameters actual and expected and message:
    check if actual is equal to expected:
        display "✓ " with message
        return yes
    otherwise:
        display "✗ " with message
        return no
    end check
end action

define action called validate_age with parameters age:
    check if age is greater than or equal to 0 and age is less than or equal to 120:
        return yes
    otherwise:
        return no
    end check
end action

// Test edge cases for age validation
store age_neg as validate_age of -1
call assert_equals with age_neg and no and "Negative age rejected"
store age_zero as validate_age of 0
call assert_equals with age_zero and yes and "Zero age accepted"
store age_max as validate_age of 120
call assert_equals with age_max and yes and "Maximum age accepted"
store age_over as validate_age of 121
call assert_equals with age_over and no and "Over maximum rejected"
```

### Pattern 4: Integration Testing

**Test multiple components together:**

```wfl
// The components under test (defined here so the example runs standalone)
define action called create_user with parameters email and password:
    return email
end action

define action called validate_user with parameters user:
    check if user is not nothing:
        return yes
    otherwise:
        return no
    end check
end action

define action called save_user_to_file with parameters user and filename:
    open file at filename for writing as f
    wait for write content user into f
    close file f
    return yes
end action

define action called load_user_from_file with parameters filename:
    open file at filename for reading as f
    wait for store user_data as read content from f
    close file f
    return user_data
end action

display "=== Integration Test: User Registration ==="

// Create user
store user as create_user of "alice@example.com" and "password123"
check if user is not nothing:
    display "✓ User created"
end check

// Validate user
store is_valid as validate_user of user
check if is_valid is yes:
    display "✓ User validated"
end check

// Save user
store saved as save_user_to_file of user and "users.txt"
check if saved is yes:
    display "✓ User saved"
end check

// Load user
store loaded as load_user_from_file of "users.txt"
check if loaded is not nothing:
    display "✓ User loaded"
end check

display "=== Integration test complete ==="
```

## Testing File Operations

```wfl
display "Testing file operations..."

// Test write
try:
    open file at "test_file.txt" for writing as testfile
    wait for write content "test data" into testfile
    close file testfile
    display "✓ File write works"
when error:
    display "✗ File write failed"
end try

// Test read
try:
    open file at "test_file.txt" for reading as testfile
    wait for store file_content as read content from testfile
    close file testfile

    check if file_content is equal to "test data":
        display "✓ File read works"
    otherwise:
        display "✗ File content mismatch"
    end check
when error:
    display "✗ File read failed"
end try

// Cleanup
remove_file at "test_file.txt"
```

## Testing Actions

```wfl
define action called assert_equals with parameters actual and expected and message:
    check if actual is equal to expected:
        display "✓ " with message
        return yes
    otherwise:
        display "✗ " with message
        return no
    end check
end action

define action called add_numbers with parameters x and y:
    return x plus y
end action

// Test suite
display "Testing add_numbers action..."

store t1 as add_numbers of 2 and 3
call assert_equals with t1 and 5 and "2 + 3 = 5"
store t2 as add_numbers of 0 and 0
call assert_equals with t2 and 0 and "0 + 0 = 0"
store t3 as add_numbers of -5 and 5
call assert_equals with t3 and 0 and "-5 + 5 = 0"
store t4 as add_numbers of 100 and 200
call assert_equals with t4 and 300 and "100 + 200 = 300"

display "Add action tests complete"
```

## Best Practices

✅ **Write tests first** - TDD prevents bugs
✅ **Test edge cases** - Boundaries, empty, null
✅ **Test error cases** - Ensure errors are caught
✅ **Test integration** - Components working together
✅ **Name tests clearly** - What they test
✅ **Use assertions** - Verify expected behavior
✅ **Clean up after tests** - Remove test files
✅ **Run all tests** - Before commits
✅ **Keep tests in TestPrograms/** - Standard location

❌ **Don't skip error testing** - Errors must be caught
❌ **Don't test only happy path** - Test failures too
❌ **Don't leave test files** - Clean up
❌ **Don't skip edge cases** - They find bugs

## What You've Learned

✅ TDD workflow (test first)
✅ Assertion testing pattern
✅ Error testing
✅ Edge case testing
✅ Integration testing
✅ File operation testing
✅ Action testing
✅ Best practices for quality

**Next:** [Project Organization →](project-organization.md)

---

**Previous:** [← Performance Tips](performance-tips.md) | **Next:** [Project Organization →](project-organization.md)
