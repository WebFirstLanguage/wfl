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
///
/// # Examples
///
/// ```ignore
/// pub fn native_add(args: Vec<Value>) -> Result<Value, RuntimeError> {
///     check_arg_count("add", &args, 2)?;  // Requires exactly 2 arguments
///     // ... function implementation
/// }
/// ```
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
///
/// # Examples
///
/// ```ignore
/// pub fn native_print(args: Vec<Value>) -> Result<Value, RuntimeError> {
///     check_min_arg_count("print", &args, 1)?;  // Requires at least 1 argument
///     // ... can process args.len() arguments
/// }
/// ```
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
///
/// # Examples
///
/// ```ignore
/// pub fn native_substring(args: Vec<Value>) -> Result<Value, RuntimeError> {
///     check_arg_range("substring", &args, 2, 3)?;  // Requires 2 or 3 arguments
///     // ... handle optional third argument
/// }
/// ```
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

/// Extracts a number value from a WFL Value, returning it as a primitive f64.
///
/// This is the most common type extractor for numeric operations. Returns a copy
/// of the f64 value rather than a reference since f64 implements Copy.
///
/// # Arguments
///
/// * `value` - The WFL Value to extract from
///
/// # Returns
///
/// Returns the f64 number if the value is a Number variant.
///
/// # Errors
///
/// Returns `RuntimeError` if the value is not a Number, with an error message
/// indicating the expected type and the actual type received.
///
/// # Examples
///
/// ```ignore
/// pub fn native_abs(args: Vec<Value>) -> Result<Value, RuntimeError> {
///     check_arg_count("abs", &args, 1)?;
///     let num = expect_number(&args[0])?;
///     Ok(Value::Number(num.abs()))
/// }
/// ```
pub fn expect_number(value: &Value) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(n) => Ok(*n),
        _ => Err(RuntimeError::new(
            format!("Expected a number, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

/// Extracts a text value from a WFL Value, returning it as a reference-counted string.
///
/// Returns an `Rc<str>` to enable efficient memory sharing without copying the string
/// data. This is the standard way to extract text values in the WFL runtime.
///
/// # Arguments
///
/// * `value` - The WFL Value to extract from
///
/// # Returns
///
/// Returns an `Rc<str>` clone (incrementing the reference count) if the value is a Text variant.
/// The underlying string data is not copied, only the reference count is incremented.
///
/// # Errors
///
/// Returns `RuntimeError` if the value is not a Text, with an error message
/// indicating the expected type and the actual type received.
///
/// # Examples
///
/// ```ignore
/// pub fn native_uppercase(args: Vec<Value>) -> Result<Value, RuntimeError> {
///     check_arg_count("uppercase", &args, 1)?;
///     let text = expect_text(&args[0])?;
///     Ok(Value::Text(Rc::from(text.to_uppercase())))
/// }
/// ```
pub fn expect_text(value: &Value) -> Result<Arc<str>, RuntimeError> {
    match value {
        Value::Text(s) => Ok(Arc::clone(s)),
        _ => Err(RuntimeError::new(
            format!("Expected text, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

/// Extracts a list value from a WFL Value, returning it as a reference-counted mutable vector.
///
/// Returns an `Rc<RefCell<Vec<Value>>>` to enable efficient memory sharing with interior
/// mutability. The RefCell allows mutation of the list contents even through shared references,
/// which is essential for list operations like push, pop, and element modification.
///
/// # Arguments
///
/// * `value` - The WFL Value to extract from
///
/// # Returns
///
/// Returns an `Rc<RefCell<Vec<Value>>>` clone (incrementing the reference count) if the value
/// is a List variant. Multiple references to the same list share the underlying data.
///
/// # Errors
///
/// Returns `RuntimeError` if the value is not a List, with an error message
/// indicating the expected type and the actual type received.
///
/// # Examples
///
/// ```ignore
/// pub fn native_push(args: Vec<Value>) -> Result<Value, RuntimeError> {
///     check_arg_count("push", &args, 2)?;
///     let list = expect_list(&args[0])?;
///     list.borrow_mut().push(args[1].clone());
///     Ok(Value::Nothing)
/// }
/// ```
pub fn expect_list(value: &Value) -> Result<Rc<RefCell<Vec<Value>>>, RuntimeError> {
    match value {
        Value::List(list) => Ok(Rc::clone(list)),
        _ => Err(RuntimeError::new(
            format!("Expected a list, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

/// Extracts a boolean value from a WFL Value, returning it as a primitive bool.
///
/// Returns a copy of the bool value rather than a reference since bool implements Copy.
/// This is commonly used in conditional logic and boolean operations.
///
/// # Arguments
///
/// * `value` - The WFL Value to extract from
///
/// # Returns
///
/// Returns the bool if the value is a Bool variant.
///
/// # Errors
///
/// Returns `RuntimeError` if the value is not a Bool, with an error message
/// indicating the expected type and the actual type received.
///
/// # Examples
///
/// ```ignore
/// pub fn native_not(args: Vec<Value>) -> Result<Value, RuntimeError> {
///     check_arg_count("not", &args, 1)?;
///     let b = expect_bool(&args[0])?;
///     Ok(Value::Bool(!b))
/// }
/// ```
pub fn expect_bool(value: &Value) -> Result<bool, RuntimeError> {
    match value {
        Value::Bool(b) => Ok(*b),
        _ => Err(RuntimeError::new(
            format!("Expected a boolean, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

/// Extracts a Date value from a WFL Value, returning it as a reference-counted NaiveDate.
///
/// Returns an `Rc<chrono::NaiveDate>` to enable efficient memory sharing of date values.
/// NaiveDate represents a date without timezone information (year, month, day only).
///
/// # Arguments
///
/// * `value` - The WFL Value to extract from
///
/// # Returns
///
/// Returns an `Rc<NaiveDate>` clone (incrementing the reference count) if the value
/// is a Date variant. The underlying date data is shared, not copied.
///
/// # Errors
///
/// Returns `RuntimeError` if the value is not a Date, with an error message
/// indicating the expected type and the actual type received.
///
/// # Examples
///
/// ```ignore
/// pub fn native_date_year(args: Vec<Value>) -> Result<Value, RuntimeError> {
///     check_arg_count("date_year", &args, 1)?;
///     let date = expect_date(&args[0])?;
///     Ok(Value::Number(date.year() as f64))
/// }
/// ```
pub fn expect_date(value: &Value) -> Result<Rc<chrono::NaiveDate>, RuntimeError> {
    match value {
        Value::Date(d) => Ok(Rc::clone(d)),
        _ => Err(RuntimeError::new(
            format!("Expected a Date, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

/// Extracts a Time value from a WFL Value, returning it as a reference-counted NaiveTime.
///
/// Returns an `Rc<chrono::NaiveTime>` to enable efficient memory sharing of time values.
/// NaiveTime represents a time of day without timezone information (hour, minute, second, nanosecond).
///
/// # Arguments
///
/// * `value` - The WFL Value to extract from
///
/// # Returns
///
/// Returns an `Rc<NaiveTime>` clone (incrementing the reference count) if the value
/// is a Time variant. The underlying time data is shared, not copied.
///
/// # Errors
///
/// Returns `RuntimeError` if the value is not a Time, with an error message
/// indicating the expected type and the actual type received.
///
/// # Examples
///
/// ```ignore
/// pub fn native_time_hour(args: Vec<Value>) -> Result<Value, RuntimeError> {
///     check_arg_count("time_hour", &args, 1)?;
///     let time = expect_time(&args[0])?;
///     Ok(Value::Number(time.hour() as f64))
/// }
/// ```
pub fn expect_time(value: &Value) -> Result<Rc<chrono::NaiveTime>, RuntimeError> {
    match value {
        Value::Time(t) => Ok(Rc::clone(t)),
        _ => Err(RuntimeError::new(
            format!("Expected a Time, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

/// Extracts a DateTime value from a WFL Value, returning it as a reference-counted NaiveDateTime.
///
/// Returns an `Rc<chrono::NaiveDateTime>` to enable efficient memory sharing of datetime values.
/// NaiveDateTime represents a date and time without timezone information, combining both
/// date (year, month, day) and time (hour, minute, second, nanosecond) components.
///
/// # Arguments
///
/// * `value` - The WFL Value to extract from
///
/// # Returns
///
/// Returns an `Rc<NaiveDateTime>` clone (incrementing the reference count) if the value
/// is a DateTime variant. The underlying datetime data is shared, not copied.
///
/// # Errors
///
/// Returns `RuntimeError` if the value is not a DateTime, with an error message
/// indicating the expected type and the actual type received.
///
/// # Examples
///
/// ```ignore
/// pub fn native_datetime_add_days(args: Vec<Value>) -> Result<Value, RuntimeError> {
///     check_arg_count("datetime_add_days", &args, 2)?;
///     let dt = expect_datetime(&args[0])?;
///     let days = expect_number(&args[1])? as i64;
///     let new_dt = dt.checked_add_signed(Duration::days(days))
///         .ok_or_else(|| RuntimeError::new("Date overflow".to_string(), 0, 0))?;
///     Ok(Value::DateTime(Rc::new(new_dt)))
/// }
/// ```
pub fn expect_datetime(value: &Value) -> Result<Rc<chrono::NaiveDateTime>, RuntimeError> {
    match value {
        Value::DateTime(dt) => Ok(Rc::clone(dt)),
        _ => Err(RuntimeError::new(
            format!("Expected a DateTime, got {}", value.type_name()),
            0,
            0,
        )),
    }
}
