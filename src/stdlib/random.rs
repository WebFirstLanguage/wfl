use super::helpers::{check_arg_count, expect_list, expect_number};
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use rand::SeedableRng;
use rand::prelude::*;
use rand::rngs::StdRng;
use std::cell::RefCell;
use std::sync::Arc;

// Global random number generator state - initialized with entropy
thread_local! {
    static RNG: RefCell<StdRng> = RefCell::new({
        let mut seed = [0u8; 32];
        rand::rng().fill_bytes(&mut seed);
        StdRng::from_seed(seed)
    });
}

/// Generate a cryptographically secure random number between 0 and 1
pub fn native_random(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("random", &args, 0)?;

    RNG.with(|rng| {
        let mut rng = rng.borrow_mut();
        let random_value: f64 = rng.random();
        Ok(Value::Number(random_value))
    })
}

/// Generate a random number between min and max (inclusive)
pub fn native_random_between(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("random_between", &args, 2)?;

    let min = expect_number(&args[0])?;
    let max = expect_number(&args[1])?;

    if min > max {
        return Err(RuntimeError::new(
            format!(
                "random_between: min ({}) cannot be greater than max ({})",
                min, max
            ),
            0,
            0,
        ));
    }

    RNG.with(|rng| {
        let mut rng = rng.borrow_mut();
        let random_value: f64 = rng.random_range(min..=max);
        Ok(Value::Number(random_value))
    })
}

/// Generate a random integer between min and max (inclusive)
pub fn native_random_int(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("random_int", &args, 2)?;

    let min = expect_number(&args[0])? as i64;
    let max = expect_number(&args[1])? as i64;

    if min > max {
        return Err(RuntimeError::new(
            format!(
                "random_int: min ({}) cannot be greater than max ({})",
                min, max
            ),
            0,
            0,
        ));
    }

    RNG.with(|rng| {
        let mut rng = rng.borrow_mut();
        let random_value: i64 = rng.random_range(min..=max);
        Ok(Value::Number(random_value as f64))
    })
}

/// Generate a random boolean value
pub fn native_random_boolean(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("random_boolean", &args, 0)?;

    RNG.with(|rng| {
        let mut rng = rng.borrow_mut();
        let random_value: bool = rng.random();
        Ok(Value::Bool(random_value))
    })
}

/// Select a random element from a list
pub fn native_random_from(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("random_from", &args, 1)?;

    let list_ref = expect_list(&args[0])?;
    let list = list_ref.borrow();

    if list.is_empty() {
        return Err(RuntimeError::new(
            "random_from: cannot select from empty list".to_string(),
            0,
            0,
        ));
    }

    RNG.with(|rng| {
        let mut rng = rng.borrow_mut();
        let index = rng.random_range(0..list.len());
        Ok(list[index].clone())
    })
}

/// Set the random seed for reproducible results
pub fn native_random_seed(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("random_seed", &args, 1)?;

    let seed = expect_number(&args[0])? as u64;

    RNG.with(|rng| {
        // Replace the RNG with a seeded one
        *rng.borrow_mut() = StdRng::seed_from_u64(seed);
        Ok(Value::Nothing)
    })
}

/// Generate a UUID v4 (random UUID)
/// Usage: generate_uuid() -> "550e8400-e29b-41d4-a716-446655440000"
pub fn native_generate_uuid(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("generate_uuid", &args, 0)?;

    use uuid::Uuid;

    let uuid = Uuid::new_v4();
    Ok(Value::Text(Arc::from(uuid.to_string())))
}

/// Register all random functions in the environment
pub fn register_random(env: &mut Environment) {
    let _ = env.define("random", Value::NativeFunction("random", native_random));
    let _ = env.define(
        "random_between",
        Value::NativeFunction("random_between", native_random_between),
    );
    let _ = env.define(
        "random_int",
        Value::NativeFunction("random_int", native_random_int),
    );
    let _ = env.define(
        "random_boolean",
        Value::NativeFunction("random_boolean", native_random_boolean),
    );
    let _ = env.define(
        "random_from",
        Value::NativeFunction("random_from", native_random_from),
    );
    let _ = env.define(
        "random_seed",
        Value::NativeFunction("random_seed", native_random_seed),
    );
    let _ = env.define(
        "generate_uuid",
        Value::NativeFunction("generate_uuid", native_generate_uuid),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_generates_values() {
        let result = native_random(vec![]);
        assert!(result.is_ok());

        if let Ok(Value::Number(n)) = result {
            assert!((0.0..=1.0).contains(&n));
        } else {
            panic!("Expected number from random");
        }
    }

    #[test]
    fn test_generate_uuid_validates_args() {
        let result = native_generate_uuid(vec![Value::Number(1.0)]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .message
                .contains("generate_uuid expects 0 arguments")
        );
    }

    #[test]
    fn test_generate_uuid_format() {
        let result = native_generate_uuid(vec![]);
        assert!(result.is_ok());

        if let Ok(Value::Text(uuid_str)) = result {
            // Very basic UUID format check: 36 chars, 4 hyphens
            assert_eq!(uuid_str.len(), 36);
            assert_eq!(uuid_str.chars().filter(|&c| c == '-').count(), 4);
        } else {
            panic!("Expected text from generate_uuid");
        }
    }

    #[test]
    fn test_random_between_validates_range() {
        let result = native_random_between(vec![Value::Number(5.0), Value::Number(10.0)]);
        assert!(result.is_ok());

        if let Ok(Value::Number(n)) = result {
            assert!((5.0..=10.0).contains(&n));
        } else {
            panic!("Expected number from random_between");
        }
    }

    #[test]
    fn test_random_int_produces_integers() {
        let result = native_random_int(vec![Value::Number(1.0), Value::Number(10.0)]);
        assert!(result.is_ok());

        if let Ok(Value::Number(n)) = result {
            assert!((1.0..=10.0).contains(&n));
            assert_eq!(n.fract(), 0.0, "Should be an integer");
        } else {
            panic!("Expected number from random_int");
        }
    }

    #[test]
    fn test_random_boolean_produces_bool() {
        let result = native_random_boolean(vec![]);
        assert!(result.is_ok());

        match result.unwrap() {
            Value::Bool(_) => {} // Success
            _ => panic!("Expected boolean from random_boolean"),
        }
    }

    #[test]
    fn test_random_seed_reproducibility() {
        // Set seed and generate value
        let _ = native_random_seed(vec![Value::Number(42.0)]);
        let result1 = native_random(vec![]).unwrap();

        // Reset same seed and generate again
        let _ = native_random_seed(vec![Value::Number(42.0)]);
        let result2 = native_random(vec![]).unwrap();

        assert_eq!(result1, result2, "Same seed should produce same values");
    }
}
