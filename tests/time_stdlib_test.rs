use wfl::interpreter::value::Value;
use wfl::stdlib::time::native_create_time;

#[test]
fn test_native_create_time_with_three_args() {
    // Verify create_time with 3 args (hour, minute, second) works correctly
    let args = vec![
        Value::Number(10.0),
        Value::Number(30.0),
        Value::Number(45.0),
    ];
    let result = native_create_time(args);
    assert!(
        result.is_ok(),
        "create_time should succeed with valid hour, minute, and second"
    );

    // Verify the returned value is a Time variant
    let time_value = result.unwrap();
    assert!(
        matches!(time_value, Value::Time(_)),
        "create_time should return a Time value"
    );
}

#[test]
fn test_native_create_time_with_two_args() {
    // Verify create_time with 2 args (hour, minute) works correctly (seconds defaults to 0)
    let args = vec![Value::Number(10.0), Value::Number(30.0)];
    let result = native_create_time(args);
    assert!(
        result.is_ok(),
        "create_time should succeed with valid hour and minute (seconds defaults to 0)"
    );

    // Verify the returned value is a Time variant
    let time_value = result.unwrap();
    assert!(
        matches!(time_value, Value::Time(_)),
        "create_time should return a Time value"
    );
}

#[test]
fn test_native_create_time_wrong_arg_count() {
    // Verify create_time with wrong number of args returns error
    let result = native_create_time(vec![]);
    assert!(result.is_err(), "create_time should fail with 0 arguments");

    let result = native_create_time(vec![Value::Number(10.0)]);
    assert!(result.is_err(), "create_time should fail with 1 argument");

    let result = native_create_time(vec![
        Value::Number(10.0),
        Value::Number(30.0),
        Value::Number(45.0),
        Value::Number(0.0),
    ]);
    assert!(result.is_err(), "create_time should fail with 4 arguments");
}

#[test]
fn test_native_create_time_invalid_values() {
    // Verify create_time with invalid hour
    let args = vec![
        Value::Number(25.0), // Invalid hour
        Value::Number(30.0),
        Value::Number(45.0),
    ];
    let result = native_create_time(args);
    assert!(result.is_err(), "create_time should fail with hour > 23");

    // Verify create_time with invalid minute
    let args = vec![
        Value::Number(10.0),
        Value::Number(60.0), // Invalid minute
        Value::Number(45.0),
    ];
    let result = native_create_time(args);
    assert!(result.is_err(), "create_time should fail with minute >= 60");

    // Verify create_time with invalid second
    let args = vec![
        Value::Number(10.0),
        Value::Number(30.0),
        Value::Number(60.0), // Invalid second
    ];
    let result = native_create_time(args);
    assert!(result.is_err(), "create_time should fail with second >= 60");
}
