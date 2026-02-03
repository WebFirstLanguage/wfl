use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use crate::stdlib::helpers::{
    check_arg_count, expect_date, expect_datetime, expect_number, expect_text, expect_time,
};
use chrono::{Local, NaiveDate, NaiveTime};
use std::rc::Rc;

/// Returns the current date
pub fn native_today(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count(&args, 0, "today")?;

    let today = Local::now().date_naive();
    Ok(Value::Date(Rc::new(today)))
}

/// Returns the current time
pub fn native_now(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count(&args, 0, "now")?;

    let now = Local::now().time();
    Ok(Value::Time(Rc::new(now)))
}

/// Returns the current date and time
pub fn native_datetime_now(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count(&args, 0, "datetime_now")?;

    let now = Local::now().naive_local();
    Ok(Value::DateTime(Rc::new(now)))
}

/// Formats a date according to a format string
pub fn native_format_date(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count(&args, 2, "format_date")?;

    let date = expect_date(&args[0])?;
    let format_string = expect_text(&args[1])?;

    let formatted = date.format(&format_string).to_string();
    Ok(Value::Text(formatted.into()))
}

/// Formats a time according to a format string
pub fn native_format_time(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count(&args, 2, "format_time")?;

    let time = expect_time(&args[0])?;
    let format_string = expect_text(&args[1])?;

    let formatted = time.format(&format_string).to_string();
    Ok(Value::Text(formatted.into()))
}

/// Formats a datetime according to a format string
pub fn native_format_datetime(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count(&args, 2, "format_datetime")?;

    let datetime = expect_datetime(&args[0])?;
    let format_string = expect_text(&args[1])?;

    let formatted = datetime.format(&format_string).to_string();
    Ok(Value::Text(formatted.into()))
}

/// Parses a date from a string
pub fn native_parse_date(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count(&args, 2, "parse_date")?;

    let date_str = expect_text(&args[0])?;
    let format_string = expect_text(&args[1])?;

    match NaiveDate::parse_from_str(&date_str, &format_string) {
        Ok(date) => Ok(Value::Date(Rc::new(date))),
        Err(e) => Err(RuntimeError::new(
            format!("Failed to parse date: {e}"),
            0,
            0,
        )),
    }
}

/// Parses a time from a string
pub fn native_parse_time(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count(&args, 2, "parse_time")?;

    let time_str = expect_text(&args[0])?;
    let format_string = expect_text(&args[1])?;

    match NaiveTime::parse_from_str(&time_str, &format_string) {
        Ok(time) => Ok(Value::Time(Rc::new(time))),
        Err(e) => Err(RuntimeError::new(
            format!("Failed to parse time: {e}"),
            0,
            0,
        )),
    }
}

/// Creates a time from hours, minutes, and seconds
pub fn native_create_time(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() < 2 || args.len() > 3 {
        return Err(RuntimeError::new(
            format!("create_time expects 2 or 3 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let hours = expect_number(&args[0])? as u32;
    let minutes = expect_number(&args[1])? as u32;

    let seconds = if args.len() == 3 {
        expect_number(&args[2])? as u32
    } else {
        0
    };

    if hours >= 24 {
        return Err(RuntimeError::new(
            format!("Hours must be between 0 and 23, got {hours}"),
            0,
            0,
        ));
    }

    if minutes >= 60 {
        return Err(RuntimeError::new(
            format!("Minutes must be between 0 and 59, got {minutes}"),
            0,
            0,
        ));
    }

    if seconds >= 60 {
        return Err(RuntimeError::new(
            format!("Seconds must be between 0 and 59, got {seconds}"),
            0,
            0,
        ));
    }

    match NaiveTime::from_hms_opt(hours, minutes, seconds) {
        Some(time) => Ok(Value::Time(Rc::new(time))),
        None => Err(RuntimeError::new(
            format!(
                "Failed to create time with hours: {hours}, minutes: {minutes}, seconds: {seconds}"
            ),
            0,
            0,
        )),
    }
}

/// Creates a date from year, month, and day
pub fn native_create_date(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count(&args, 3, "create_date")?;

    let year = expect_number(&args[0])? as i32;
    let month = expect_number(&args[1])? as u32;
    let day = expect_number(&args[2])? as u32;

    if !(1..=12).contains(&month) {
        return Err(RuntimeError::new(
            format!("Month must be between 1 and 12, got {month}"),
            0,
            0,
        ));
    }

    if !(1..=31).contains(&day) {
        return Err(RuntimeError::new(
            format!("Day must be between 1 and 31, got {day}"),
            0,
            0,
        ));
    }

    match NaiveDate::from_ymd_opt(year, month, day) {
        Some(date) => Ok(Value::Date(Rc::new(date))),
        None => Err(RuntimeError::new(
            format!("Failed to create date with year: {year}, month: {month}, day: {day}"),
            0,
            0,
        )),
    }
}

/// Adds days to a date
pub fn native_add_days(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count(&args, 2, "add_days")?;

    let date = expect_date(&args[0])?;
    let days = expect_number(&args[1])? as i64;

    let new_date = date
        .checked_add_signed(chrono::Duration::days(days))
        .ok_or_else(|| RuntimeError::new(format!("Failed to add {days} days to date"), 0, 0))?;

    Ok(Value::Date(Rc::new(new_date)))
}

/// Gets the difference in days between two dates
pub fn native_days_between(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count(&args, 2, "days_between")?;

    let date1 = expect_date(&args[0])?;
    let date2 = expect_date(&args[1])?;

    let duration = date2.signed_duration_since(*date1);
    let days = duration.num_days();

    Ok(Value::Number(days as f64))
}

/// Simple test function that returns the current date as a string
pub fn native_current_date(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count(&args, 0, "current_date")?;

    let today = Local::now().date_naive();
    let formatted = today.format("%Y-%m-%d").to_string();
    Ok(Value::Text(formatted.into()))
}

/// Register all time-related functions in the environment
pub fn register_time(env: &mut Environment) {
    let _ = env.define("today", Value::NativeFunction("today", native_today));
    let _ = env.define("now", Value::NativeFunction("now", native_now));
    let _ = env.define(
        "datetime_now",
        Value::NativeFunction("datetime_now", native_datetime_now),
    );
    let _ = env.define(
        "format_date",
        Value::NativeFunction("format_date", native_format_date),
    );
    let _ = env.define(
        "format_time",
        Value::NativeFunction("format_time", native_format_time),
    );
    let _ = env.define(
        "format_datetime",
        Value::NativeFunction("format_datetime", native_format_datetime),
    );
    let _ = env.define(
        "parse_date",
        Value::NativeFunction("parse_date", native_parse_date),
    );
    let _ = env.define(
        "parse_time",
        Value::NativeFunction("parse_time", native_parse_time),
    );
    let _ = env.define(
        "create_time",
        Value::NativeFunction("create_time", native_create_time),
    );
    let _ = env.define(
        "create_date",
        Value::NativeFunction("create_date", native_create_date),
    );
    let _ = env.define(
        "add_days",
        Value::NativeFunction("add_days", native_add_days),
    );
    let _ = env.define(
        "days_between",
        Value::NativeFunction("days_between", native_days_between),
    );
    let _ = env.define(
        "current_date",
        Value::NativeFunction("current_date", native_current_date),
    );
}
