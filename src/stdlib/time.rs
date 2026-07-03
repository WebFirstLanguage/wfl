use super::helpers::{
    check_arg_count, check_arg_range, expect_date, expect_datetime, expect_number, expect_text,
    expect_time,
};
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use chrono::{Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use std::rc::Rc;

/// Extracts the date component from a Date or DateTime value.
fn expect_date_like(func_name: &str, value: &Value) -> Result<NaiveDate, RuntimeError> {
    match value {
        Value::Date(date) => Ok(**date),
        Value::DateTime(dt) => Ok(dt.date()),
        other => Err(RuntimeError::new(
            format!(
                "{func_name} expects a Date or DateTime, got {}",
                other.type_name()
            ),
            0,
            0,
        )),
    }
}

/// Extracts the time component from a Time or DateTime value.
fn expect_time_like(func_name: &str, value: &Value) -> Result<NaiveTime, RuntimeError> {
    match value {
        Value::Time(time) => Ok(**time),
        Value::DateTime(dt) => Ok(dt.time()),
        other => Err(RuntimeError::new(
            format!(
                "{func_name} expects a Time or DateTime, got {}",
                other.type_name()
            ),
            0,
            0,
        )),
    }
}

/// Returns the current date
pub fn native_today(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("today", &args, 0)?;

    let today = Local::now().date_naive();
    Ok(Value::Date(Rc::new(today)))
}

/// Returns the current time
pub fn native_now(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("now", &args, 0)?;

    let now = Local::now().time();
    Ok(Value::Time(Rc::new(now)))
}

/// Returns the current date and time
pub fn native_datetime_now(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("datetime_now", &args, 0)?;

    let now = Local::now().naive_local();
    Ok(Value::DateTime(Rc::new(now)))
}

/// Formats a date according to a format string
pub fn native_format_date(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("format_date", &args, 2)?;

    let date = expect_date(&args[0])?;
    let format_string = expect_text(&args[1])?;

    let formatted = date.format(&format_string).to_string();
    Ok(Value::Text(formatted.into()))
}

/// Formats a time according to a format string
pub fn native_format_time(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("format_time", &args, 2)?;

    let time = expect_time(&args[0])?;
    let format_string = expect_text(&args[1])?;

    let formatted = time.format(&format_string).to_string();
    Ok(Value::Text(formatted.into()))
}

/// Formats a datetime according to a format string
pub fn native_format_datetime(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("format_datetime", &args, 2)?;

    let datetime = expect_datetime(&args[0])?;
    let format_string = expect_text(&args[1])?;

    let formatted = datetime.format(&format_string).to_string();
    Ok(Value::Text(formatted.into()))
}

/// Parses a date from a string
pub fn native_parse_date(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("parse_date", &args, 2)?;

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
    check_arg_count("parse_time", &args, 2)?;

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
    check_arg_range("create_time", &args, 2, 3)?;

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
    check_arg_count("create_date", &args, 3)?;

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
    check_arg_count("add_days", &args, 2)?;

    let date = expect_date(&args[0])?;
    let days = expect_number(&args[1])? as i64;

    let new_date = date
        .checked_add_signed(chrono::Duration::days(days))
        .ok_or_else(|| RuntimeError::new(format!("Failed to add {days} days to date"), 0, 0))?;

    Ok(Value::Date(Rc::new(new_date)))
}

/// Gets the difference in days between two dates
pub fn native_days_between(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("days_between", &args, 2)?;

    let date1 = expect_date(&args[0])?;
    let date2 = expect_date(&args[1])?;

    let duration = date2.signed_duration_since(*date1);
    let days = duration.num_days();

    Ok(Value::Number(days as f64))
}

/// Simple test function that returns the current date as a string
pub fn native_current_date(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("current_date", &args, 0)?;

    let today = Local::now().date_naive();
    let formatted = today.format("%Y-%m-%d").to_string();
    Ok(Value::Text(formatted.into()))
}

/// Subtracts days from a date
pub fn native_subtract_days(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("subtract_days", &args, 2)?;

    let date = expect_date(&args[0])?;
    let days = expect_number(&args[1])? as i64;

    let new_date = date
        .checked_sub_signed(chrono::Duration::days(days))
        .ok_or_else(|| {
            RuntimeError::new(format!("Failed to subtract {days} days from date"), 0, 0)
        })?;

    Ok(Value::Date(Rc::new(new_date)))
}

/// Creates a datetime from year, month, day, hours, minutes, and seconds
pub fn native_create_datetime(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_range("create_datetime", &args, 3, 6)?;

    let year = expect_number(&args[0])? as i32;
    let month = expect_number(&args[1])? as u32;
    let day = expect_number(&args[2])? as u32;
    let hours = if args.len() > 3 {
        expect_number(&args[3])? as u32
    } else {
        0
    };
    let minutes = if args.len() > 4 {
        expect_number(&args[4])? as u32
    } else {
        0
    };
    let seconds = if args.len() > 5 {
        expect_number(&args[5])? as u32
    } else {
        0
    };

    let date = NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
        RuntimeError::new(
            format!("Failed to create date with year: {year}, month: {month}, day: {day}"),
            0,
            0,
        )
    })?;
    let time = NaiveTime::from_hms_opt(hours, minutes, seconds).ok_or_else(|| {
        RuntimeError::new(
            format!(
                "Failed to create time with hours: {hours}, minutes: {minutes}, seconds: {seconds}"
            ),
            0,
            0,
        )
    })?;

    Ok(Value::DateTime(Rc::new(NaiveDateTime::new(date, time))))
}

/// Extracts the date part of a datetime
pub fn native_date_part(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("date_part", &args, 1)?;

    let datetime = expect_datetime(&args[0])?;
    Ok(Value::Date(Rc::new(datetime.date())))
}

/// Extracts the time part of a datetime
pub fn native_time_part(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("time_part", &args, 1)?;

    let datetime = expect_datetime(&args[0])?;
    Ok(Value::Time(Rc::new(datetime.time())))
}

/// Returns the current date and time in UTC
pub fn native_utc_now(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("utc_now", &args, 0)?;

    Ok(Value::DateTime(Rc::new(Utc::now().naive_utc())))
}

/// Returns the year of a date or datetime
pub fn native_year(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("year", &args, 1)?;
    let date = expect_date_like("year", &args[0])?;
    Ok(Value::Number(date.year() as f64))
}

/// Returns the month of a date or datetime
pub fn native_month(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("month", &args, 1)?;
    let date = expect_date_like("month", &args[0])?;
    Ok(Value::Number(date.month() as f64))
}

/// Returns the day of the month of a date or datetime
pub fn native_day(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("day", &args, 1)?;
    let date = expect_date_like("day", &args[0])?;
    Ok(Value::Number(date.day() as f64))
}

/// Returns the day of the week of a date (0 = Sunday .. 6 = Saturday)
pub fn native_dayofweek(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("dayofweek", &args, 1)?;
    let date = expect_date_like("dayofweek", &args[0])?;
    Ok(Value::Number(date.weekday().num_days_from_sunday() as f64))
}

/// Returns the day of the year of a date (1..366)
pub fn native_dayofyear(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("dayofyear", &args, 1)?;
    let date = expect_date_like("dayofyear", &args[0])?;
    Ok(Value::Number(date.ordinal() as f64))
}

/// Returns the hour of a time or datetime
pub fn native_hour(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("hour", &args, 1)?;
    let time = expect_time_like("hour", &args[0])?;
    Ok(Value::Number(time.hour() as f64))
}

/// Returns the minute of a time or datetime
pub fn native_minute(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("minute", &args, 1)?;
    let time = expect_time_like("minute", &args[0])?;
    Ok(Value::Number(time.minute() as f64))
}

/// Returns the second of a time or datetime
pub fn native_second(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("second", &args, 1)?;
    let time = expect_time_like("second", &args[0])?;
    Ok(Value::Number(time.second() as f64))
}

/// Returns whether the given year is a leap year
pub fn native_is_leap_year(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("is_leap_year", &args, 1)?;

    let year = match &args[0] {
        Value::Number(n) => *n as i32,
        other => expect_date_like("is_leap_year", other)?.year(),
    };

    Ok(Value::Bool(NaiveDate::from_ymd_opt(year, 2, 29).is_some()))
}

/// Returns the number of days in the given month of the given year
pub fn native_days_in_month(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("days_in_month", &args, 2)?;

    let year = expect_number(&args[0])? as i32;
    let month = expect_number(&args[1])? as u32;

    if !(1..=12).contains(&month) {
        return Err(RuntimeError::new(
            format!("Month must be between 1 and 12, got {month}"),
            0,
            0,
        ));
    }

    let first = NaiveDate::from_ymd_opt(year, month, 1).ok_or_else(|| {
        RuntimeError::new(
            format!("Failed to create date with year: {year}, month: {month}"),
            0,
            0,
        )
    })?;
    let next_month_first = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .ok_or_else(|| RuntimeError::new(format!("Failed to compute days in month {month}"), 0, 0))?;

    let days = next_month_first.signed_duration_since(first).num_days();
    Ok(Value::Number(days as f64))
}

/// Returns the ISO week number of a date (1..53)
pub fn native_week_of_year(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("week_of_year", &args, 1)?;
    let date = expect_date_like("week_of_year", &args[0])?;
    Ok(Value::Number(date.iso_week().week() as f64))
}

/// Returns a Unix timestamp (seconds) for a date, time, or datetime.
///
/// WFL date/time values are naive (no timezone), so an explicit argument is
/// treated as UTC wall-clock time — this makes `timestamp` and
/// `datetime_from_timestamp` exact inverses of each other. With no argument,
/// the current true Unix time is returned.
pub fn native_timestamp(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_range("timestamp", &args, 0, 1)?;

    let datetime = if args.is_empty() {
        Utc::now().naive_utc()
    } else {
        match &args[0] {
            Value::DateTime(dt) => **dt,
            Value::Date(date) => date
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| RuntimeError::new("Failed to convert date".to_string(), 0, 0))?,
            // A bare time is interpreted as that time on the current UTC date
            Value::Time(time) => NaiveDateTime::new(Utc::now().date_naive(), **time),
            other => {
                return Err(RuntimeError::new(
                    format!(
                        "timestamp expects a Date, Time, or DateTime, got {}",
                        other.type_name()
                    ),
                    0,
                    0,
                ));
            }
        }
    };

    Ok(Value::Number(datetime.and_utc().timestamp() as f64))
}

/// Creates a datetime from a Unix timestamp (seconds)
pub fn native_datetime_from_timestamp(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("datetime_from_timestamp", &args, 1)?;

    let seconds = expect_number(&args[0])? as i64;
    let datetime = chrono::DateTime::from_timestamp(seconds, 0)
        .ok_or_else(|| RuntimeError::new(format!("Invalid Unix timestamp: {seconds}"), 0, 0))?;

    Ok(Value::DateTime(Rc::new(datetime.naive_utc())))
}

/// Returns the difference between two times or datetimes in milliseconds
pub fn native_time_diff(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("time_diff", &args, 2)?;

    let millis = match (&args[0], &args[1]) {
        (Value::DateTime(a), Value::DateTime(b)) => b.signed_duration_since(**a).num_milliseconds(),
        (a, b) => {
            let time_a = expect_time_like("time_diff", a)?;
            let time_b = expect_time_like("time_diff", b)?;
            time_b.signed_duration_since(time_a).num_milliseconds()
        }
    };

    Ok(Value::Number(millis as f64))
}

/// Register all time-related functions in the environment
pub fn register_time(env: &mut Environment) {
    env.define_native("today", native_today);
    env.define_native("now", native_now);
    env.define_native("datetime_now", native_datetime_now);
    env.define_native("format_date", native_format_date);
    env.define_native("format_time", native_format_time);
    env.define_native("format_datetime", native_format_datetime);
    env.define_native("parse_date", native_parse_date);
    env.define_native("parse_time", native_parse_time);
    env.define_native("create_time", native_create_time);
    env.define_native("create_date", native_create_date);
    env.define_native("create_datetime", native_create_datetime);
    env.define_native("add_days", native_add_days);
    env.define_native("subtract_days", native_subtract_days);
    env.define_native("days_between", native_days_between);
    env.define_native("current_date", native_current_date);
    env.define_native("date_part", native_date_part);
    env.define_native("time_part", native_time_part);
    env.define_native("utc_now", native_utc_now);
    env.define_native("year", native_year);
    env.define_native("month", native_month);
    env.define_native("day", native_day);
    env.define_native("dayofweek", native_dayofweek);
    env.define_native("day_of_week", native_dayofweek);
    env.define_native("dayofyear", native_dayofyear);
    env.define_native("day_of_year", native_dayofyear);
    env.define_native("hour", native_hour);
    env.define_native("minute", native_minute);
    env.define_native("second", native_second);
    env.define_native("is_leap_year", native_is_leap_year);
    env.define_native("isleapyear", native_is_leap_year);
    env.define_native("days_in_month", native_days_in_month);
    env.define_native("week_of_year", native_week_of_year);
    env.define_native("timestamp", native_timestamp);
    env.define_native("datetime_from_timestamp", native_datetime_from_timestamp);
    env.define_native("time_diff", native_time_diff);
}
