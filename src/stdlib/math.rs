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

pub fn native_min(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("min", &args, 2)?;

    let a = expect_number(&args[0])?;
    let b = expect_number(&args[1])?;
    Ok(Value::Number(a.min(b)))
}

pub fn native_max(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("max", &args, 2)?;

    let a = expect_number(&args[0])?;
    let b = expect_number(&args[1])?;
    Ok(Value::Number(a.max(b)))
}

pub fn native_power(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("power", &args, 2)?;

    let base = expect_number(&args[0])?;
    let exponent = expect_number(&args[1])?;
    Ok(Value::Number(base.powf(exponent)))
}

pub fn native_sqrt(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("sqrt", &args, 1)?;

    let x = expect_number(&args[0])?;
    if x < 0.0 {
        return Err(RuntimeError::new(
            format!("sqrt of negative number: {x}"),
            0,
            0,
        ));
    }
    Ok(Value::Number(x.sqrt()))
}

pub fn native_sin(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("sin", &args, 1)?;

    let x = expect_number(&args[0])?;
    Ok(Value::Number(x.sin()))
}

pub fn native_cos(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("cos", &args, 1)?;

    let x = expect_number(&args[0])?;
    Ok(Value::Number(x.cos()))
}

pub fn native_tan(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("tan", &args, 1)?;

    let x = expect_number(&args[0])?;
    Ok(Value::Number(x.tan()))
}

pub fn register_math(env: &mut Environment) {
    let _ = env.define("abs", Value::NativeFunction("abs", native_abs));
    let _ = env.define("round", Value::NativeFunction("round", native_round));
    let _ = env.define("floor", Value::NativeFunction("floor", native_floor));
    let _ = env.define("ceil", Value::NativeFunction("ceil", native_ceil));
    let _ = env.define("clamp", Value::NativeFunction("clamp", native_clamp));
    let _ = env.define("min", Value::NativeFunction("min", native_min));
    let _ = env.define("max", Value::NativeFunction("max", native_max));
    let _ = env.define("power", Value::NativeFunction("power", native_power));
    let _ = env.define("sqrt", Value::NativeFunction("sqrt", native_sqrt));
    let _ = env.define("sin", Value::NativeFunction("sin", native_sin));
    let _ = env.define("cos", Value::NativeFunction("cos", native_cos));
    let _ = env.define("tan", Value::NativeFunction("tan", native_tan));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min() {
        assert_eq!(
            native_min(vec![Value::Number(3.0), Value::Number(5.0)]).unwrap(),
            Value::Number(3.0)
        );
        assert_eq!(
            native_min(vec![Value::Number(-1.0), Value::Number(1.0)]).unwrap(),
            Value::Number(-1.0)
        );
        assert_eq!(
            native_min(vec![Value::Number(2.0), Value::Number(2.0)]).unwrap(),
            Value::Number(2.0)
        );
    }

    #[test]
    fn test_max() {
        assert_eq!(
            native_max(vec![Value::Number(3.0), Value::Number(5.0)]).unwrap(),
            Value::Number(5.0)
        );
        assert_eq!(
            native_max(vec![Value::Number(-1.0), Value::Number(1.0)]).unwrap(),
            Value::Number(1.0)
        );
    }

    #[test]
    fn test_power() {
        assert_eq!(
            native_power(vec![Value::Number(2.0), Value::Number(3.0)]).unwrap(),
            Value::Number(8.0)
        );
        assert_eq!(
            native_power(vec![Value::Number(0.0), Value::Number(0.0)]).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(
            native_power(vec![Value::Number(5.0), Value::Number(0.0)]).unwrap(),
            Value::Number(1.0)
        );
    }

    #[test]
    fn test_sqrt() {
        assert_eq!(
            native_sqrt(vec![Value::Number(9.0)]).unwrap(),
            Value::Number(3.0)
        );
        assert_eq!(
            native_sqrt(vec![Value::Number(0.0)]).unwrap(),
            Value::Number(0.0)
        );
        assert!(native_sqrt(vec![Value::Number(-1.0)]).is_err());
    }

    #[test]
    fn test_trig() {
        assert_eq!(
            native_sin(vec![Value::Number(0.0)]).unwrap(),
            Value::Number(0.0)
        );
        assert_eq!(
            native_cos(vec![Value::Number(0.0)]).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(
            native_tan(vec![Value::Number(0.0)]).unwrap(),
            Value::Number(0.0)
        );
        // sin(pi/2) ≈ 1.0
        let sin_result = native_sin(vec![Value::Number(std::f64::consts::FRAC_PI_2)]).unwrap();
        if let Value::Number(v) = sin_result {
            assert!((v - 1.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_math_arg_count() {
        assert!(native_min(vec![Value::Number(1.0)]).is_err());
        assert!(native_max(vec![]).is_err());
        assert!(native_power(vec![Value::Number(1.0)]).is_err());
        assert!(native_sqrt(vec![]).is_err());
        assert!(native_sin(vec![]).is_err());
        assert!(native_cos(vec![]).is_err());
        assert!(native_tan(vec![]).is_err());
    }
}
