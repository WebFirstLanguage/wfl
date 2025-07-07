use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::time::{SystemTime, UNIX_EPOCH};

fn expect_number(value: &Value) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(n) => Ok(*n),
        _ => Err(RuntimeError::new(
            format!("Expected a number, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

pub fn native_abs(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("abs expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let x = expect_number(&args[0])?;
    Ok(Value::Number(x.abs()))
}

pub fn native_round(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("round expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let x = expect_number(&args[0])?;
    Ok(Value::Number(x.round()))
}

pub fn native_floor(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("floor expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let x = expect_number(&args[0])?;
    Ok(Value::Number(x.floor()))
}

pub fn native_ceil(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("ceil expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let x = expect_number(&args[0])?;
    Ok(Value::Number(x.ceil()))
}

pub fn native_random(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if !args.is_empty() {
        return Err(RuntimeError::new(
            format!("random expects 0 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let nanos = now.subsec_nanos() as f64;
    let random_value = (nanos / 1_000_000_000.0) % 1.0;

    Ok(Value::Number(random_value))
}

pub fn native_clamp(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 3 {
        return Err(RuntimeError::new(
            format!("clamp expects 3 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let value = expect_number(&args[0])?;
    let min = expect_number(&args[1])?;
    let max = expect_number(&args[2])?;

    if min > max {
        return Err(RuntimeError::new(
            format!(
                "clamp min ({}) must be less than or equal to max ({})",
                min, max
            ),
            0,
            0,
        ));
    }

    let clamped = value.max(min).min(max);
    Ok(Value::Number(clamped))
}

pub fn register_math(env: &mut Environment) {
    env.define("abs", Value::NativeFunction("abs", native_abs));
    env.define("round", Value::NativeFunction("round", native_round));
    env.define("floor", Value::NativeFunction("floor", native_floor));
    env.define("ceil", Value::NativeFunction("ceil", native_ceil));
    env.define("random", Value::NativeFunction("random", native_random));
    env.define("clamp", Value::NativeFunction("clamp", native_clamp));
}
