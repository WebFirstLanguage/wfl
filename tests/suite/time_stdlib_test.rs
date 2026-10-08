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

use chrono::{Local, NaiveDate};
use std::rc::Rc;
use wfl::stdlib::time::{native_add_days, native_create_date, native_days_between, native_today};

#[test]
fn test_native_today() {
    let expected = Local::now().date_naive();
    let result = native_today(vec![]);
    assert!(result.is_ok());
    if let Value::Date(d) = result.unwrap() {
        assert_eq!(*d, expected);
    } else {
        panic!("Expected Date variant");
    }
}

#[test]
fn test_native_create_date_valid() {
    let args = vec![
        Value::Number(2023.0),
        Value::Number(10.0),
        Value::Number(25.0),
    ];
    let result = native_create_date(args);
    assert!(result.is_ok());
    if let Value::Date(d) = result.unwrap() {
        assert_eq!(*d, NaiveDate::from_ymd_opt(2023, 10, 25).unwrap());
    } else {
        panic!("Expected Date variant");
    }
}

#[test]
fn test_native_create_date_invalid_month() {
    let args = vec![
        Value::Number(2023.0),
        Value::Number(13.0), // Invalid month
        Value::Number(25.0),
    ];
    let result = native_create_date(args);
    assert!(result.is_err());
}

#[test]
fn test_native_create_date_invalid_day() {
    let args = vec![
        Value::Number(2023.0),
        Value::Number(10.0),
        Value::Number(32.0), // Invalid day
    ];
    let result = native_create_date(args);
    assert!(result.is_err());

    let args_zero = vec![
        Value::Number(2023.0),
        Value::Number(10.0),
        Value::Number(0.0), // Invalid day (too low)
    ];
    let result_zero = native_create_date(args_zero);
    assert!(result_zero.is_err());
}

#[test]
fn test_native_add_days() {
    let start_date = Rc::new(NaiveDate::from_ymd_opt(2023, 10, 25).unwrap());
    let args = vec![Value::Date(start_date), Value::Number(5.0)];
    let result = native_add_days(args);
    assert!(result.is_ok());
    if let Value::Date(d) = result.unwrap() {
        assert_eq!(*d, NaiveDate::from_ymd_opt(2023, 10, 30).unwrap());
    } else {
        panic!("Expected Date variant");
    }
}

#[test]
fn test_native_add_days_negative() {
    let start_date = Rc::new(NaiveDate::from_ymd_opt(2023, 10, 25).unwrap());
    let args = vec![Value::Date(start_date), Value::Number(-5.0)];
    let result = native_add_days(args);
    assert!(result.is_ok());
    if let Value::Date(d) = result.unwrap() {
        assert_eq!(*d, NaiveDate::from_ymd_opt(2023, 10, 20).unwrap());
    } else {
        panic!("Expected Date variant");
    }
}

#[test]
fn test_native_days_between() {
    let date1 = Rc::new(NaiveDate::from_ymd_opt(2023, 10, 20).unwrap());
    let date2 = Rc::new(NaiveDate::from_ymd_opt(2023, 10, 25).unwrap());

    // date1 is older, date2 is newer
    let args = vec![Value::Date(date1), Value::Date(date2)];
    let result = native_days_between(args);
    assert!(result.is_ok());
    if let Value::Number(n) = result.unwrap() {
        assert_eq!(n, 5.0);
    } else {
        panic!("Expected Number variant");
    }
}
