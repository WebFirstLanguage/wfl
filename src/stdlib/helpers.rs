use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// Validates that a native function receives exactly the expected number of arguments.
///
/// This is the most common argument validation helper, used when a function requires
/// a specific number of arguments (not a range or minimum). The error message
/// automatically handles singular/plural grammar for better user experience.
///
/// # Arguments
///
/// * `func_name` - The name of the function being validated (used in error messages)
/// * `args` - The slice of argument values to check
/// * `expected` - The exact number of arguments required
///
/// # Returns
///
/// Returns `Ok(())` if the argument count matches, allowing the function to proceed.
///
/// # Errors
///
/// Returns `RuntimeError` if the argument count doesn't match the expected value.
/// The error message format is: "{func_name} expects {expected} argument(s), got {actual}"
/// with proper singular/plural handling.
pub fn check_arg_count(
    func_name: &str,
    args: &[Value],
    expected: usize,
) -> Result<(), RuntimeError> {
    if args.len() != expected {
        return Err(RuntimeError::new(
            format!(
                "{} expects {} argument{}, got {}",
                func_name,
                expected,
                if expected == 1 { "" } else { "s" },
                args.len()
            ),
            0,
            0,
        ));
    }
    Ok(())
}

/// Validates that a native function receives at least the minimum number of arguments.
///
/// Use this helper for variadic functions that accept a minimum number of required
/// arguments plus optional additional arguments. This is common for functions like
/// print, format, or concatenation operations that can handle variable inputs.
///
/// # Arguments
///
/// * `func_name` - The name of the function being validated (used in error messages)
/// * `args` - The slice of argument values to check
/// * `min_count` - The minimum number of arguments required
///
/// # Returns
///
/// Returns `Ok(())` if the argument count is at least `min_count`.
///
/// # Errors
///
/// Returns `RuntimeError` if fewer than `min_count` arguments are provided.
/// The error message format is: "{func_name} expects at least {min_count} argument(s), got {actual}"
/// with proper singular/plural handling.
pub fn check_min_arg_count(
    func_name: &str,
    args: &[Value],
    min_count: usize,
) -> Result<(), RuntimeError> {
    if args.len() < min_count {
        return Err(RuntimeError::new(
            format!(
                "{} expects at least {} argument{}, got {}",
                func_name,
                min_count,
                if min_count == 1 { "" } else { "s" },
                args.len()
            ),
            0,
            0,
        ));
    }
    Ok(())
}

/// Validates that a native function receives an argument count within a specified range.
///
/// Use this helper for functions that accept a flexible number of arguments within
/// bounds, such as functions with multiple optional parameters. The range is inclusive
/// on both ends: [min, max].
///
/// # Arguments
///
/// * `func_name` - The name of the function being validated (used in error messages)
/// * `args` - The slice of argument values to check
/// * `min` - The minimum number of arguments allowed (inclusive)
/// * `max` - The maximum number of arguments allowed (inclusive)
///
/// # Returns
///
/// Returns `Ok(())` if the argument count is within [min, max] (inclusive).
///
/// # Errors
///
/// Returns `RuntimeError` if the argument count is outside the specified range.
/// The error message format is: "{func_name} expects between {min} and {max} arguments, got {actual}".
pub fn check_arg_range(
    func_name: &str,
    args: &[Value],
    min: usize,
    max: usize,
) -> Result<(), RuntimeError> {
    if args.len() < min || args.len() > max {
        return Err(RuntimeError::new(
            format!(
                "{} expects between {} and {} arguments, got {}",
                func_name,
                min,
                max,
                args.len()
            ),
            0,
            0,
        ));
    }
    Ok(())
}

macro_rules! impl_expect_fn {
    (
        $(#[$doc:meta])*
        fn $name:ident($variant:ident) -> $ret_type:ty,
        expected: $type_str:expr
    ) => {
        $(#[$doc])*
        pub fn $name(value: &Value) -> Result<$ret_type, RuntimeError> {
            match value {
                Value::$variant(v) => Ok(v.clone()),
                _ => Err(RuntimeError::new(
                    format!("Expected {}, got {}", $type_str, value.type_name()),
                    0,
                    0,
                )),
            }
        }
    };
    (
        $(#[$doc:meta])*
        fn $name:ident($variant:ident) -> $ret_type:ty,
        expected: $type_str:expr,
        copy
    ) => {
        $(#[$doc])*
        pub fn $name(value: &Value) -> Result<$ret_type, RuntimeError> {
            match value {
                Value::$variant(v) => Ok(*v),
                _ => Err(RuntimeError::new(
                    format!("Expected {}, got {}", $type_str, value.type_name()),
                    0,
                    0,
                )),
            }
        }
    };
}

impl_expect_fn! {
    /// Extracts a number value from a WFL Value, returning it as a primitive f64.
    ///
    /// This is the most common type extractor for numeric operations. Returns a copy
    /// of the f64 value rather than a reference since f64 implements Copy.
    fn expect_number(Number) -> f64,
    expected: "a number",
    copy
}

impl_expect_fn! {
    /// Extracts a text value from a WFL Value, returning it as a reference-counted string.
    ///
    /// Returns an `Rc<str>` to enable efficient memory sharing without copying the string
    /// data. This is the standard way to extract text values in the WFL runtime.
    fn expect_text(Text) -> Arc<str>,
    expected: "text"
}

impl_expect_fn! {
    /// Extracts a list value from a WFL Value, returning it as a reference-counted mutable vector.
    ///
    /// Returns an `Rc<RefCell<Vec<Value>>>` to enable efficient memory sharing with interior
    /// mutability. The RefCell allows mutation of the list contents even through shared references,
    /// which is essential for list operations like push, pop, and element modification.
    fn expect_list(List) -> Rc<RefCell<Vec<Value>>>,
    expected: "a list"
}

impl_expect_fn! {
    /// Extracts a boolean value from a WFL Value, returning it as a primitive bool.
    ///
    /// Returns a copy of the bool value rather than a reference since bool implements Copy.
    /// This is commonly used in conditional logic and boolean operations.
    fn expect_bool(Bool) -> bool,
    expected: "a boolean",
    copy
}

impl_expect_fn! {
    /// Extracts a Date value from a WFL Value, returning it as a reference-counted NaiveDate.
    ///
    /// Returns an `Rc<chrono::NaiveDate>` to enable efficient memory sharing of date values.
    /// NaiveDate represents a date without timezone information (year, month, day only).
    fn expect_date(Date) -> Rc<chrono::NaiveDate>,
    expected: "a Date"
}

impl_expect_fn! {
    /// Extracts a Time value from a WFL Value, returning it as a reference-counted NaiveTime.
    ///
    /// Returns an `Rc<chrono::NaiveTime>` to enable efficient memory sharing of time values.
    /// NaiveTime represents a time of day without timezone information (hour, minute, second, nanosecond).
    fn expect_time(Time) -> Rc<chrono::NaiveTime>,
    expected: "a Time"
}

impl_expect_fn! {
    /// Extracts a DateTime value from a WFL Value, returning it as a reference-counted NaiveDateTime.
    ///
    /// Returns an `Rc<chrono::NaiveDateTime>` to enable efficient memory sharing of datetime values.
    /// NaiveDateTime represents a date and time without timezone information, combining both
    /// date (year, month, day) and time (hour, minute, second, nanosecond) components.
    fn expect_datetime(DateTime) -> Rc<chrono::NaiveDateTime>,
    expected: "a DateTime"
}
