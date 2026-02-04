use super::helpers::{check_arg_count, expect_number};
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;

pub fn native_abs(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("abs", &args, 1)?;

    let x = expect_number(&args[0])?;
    Ok(Value::Number(x.abs()))
}

pub fn native_round(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("round", &args, 1)?;

    let x = expect_number(&args[0])?;
    Ok(Value::Number(x.round()))
}

pub fn native_floor(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("floor", &args, 1)?;

    let x = expect_number(&args[0])?;
    Ok(Value::Number(x.floor()))
}

pub fn native_ceil(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("ceil", &args, 1)?;

    let x = expect_number(&args[0])?;
    Ok(Value::Number(x.ceil()))
}

pub fn native_clamp(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("clamp", &args, 3)?;

    let value = expect_number(&args[0])?;
    let min = expect_number(&args[1])?;
    let max = expect_number(&args[2])?;

    if min > max {
        return Err(RuntimeError::new(
            format!("clamp min ({min}) must be less than or equal to max ({max})"),
            0,
            0,
        ));
    }

    let clamped = value.max(min).min(max);
    Ok(Value::Number(clamped))
}

pub fn register_math(env: &mut Environment) {
    let _ = env.define("abs", Value::NativeFunction("abs", native_abs));
    let _ = env.define("round", Value::NativeFunction("round", native_round));
    let _ = env.define("floor", Value::NativeFunction("floor", native_floor));
    let _ = env.define("ceil", Value::NativeFunction("ceil", native_ceil));
    let _ = env.define("clamp", Value::NativeFunction("clamp", native_clamp));
}
