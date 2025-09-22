use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use rand::SeedableRng;
use rand::prelude::*;
use rand::rngs::StdRng;
use std::cell::RefCell;

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
    if !args.is_empty() {
        return Err(RuntimeError::new(
            format!("random expects 0 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    RNG.with(|rng| {
        let mut rng = rng.borrow_mut();
        let random_value: f64 = rng.random();
        Ok(Value::Number(random_value))
    })
}

/// Generate a random number between min and max (inclusive)
pub fn native_random_between(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("random_between expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let min = match &args[0] {
        Value::Number(n) => *n,
        _ => {
            return Err(RuntimeError::new(
                format!(
                    "random_between expects numbers, got {}",
                    args[0].type_name()
                ),
                0,
                0,
            ));
        }
    };

    let max = match &args[1] {
        Value::Number(n) => *n,
        _ => {
            return Err(RuntimeError::new(
                format!(
                    "random_between expects numbers, got {}",
                    args[1].type_name()
                ),
                0,
                0,
            ));
        }
    };

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
    if args.len() != 2 {
        return Err(RuntimeError::new(
            format!("random_int expects 2 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let min = match &args[0] {
        Value::Number(n) => *n as i64,
        _ => {
            return Err(RuntimeError::new(
                format!("random_int expects numbers, got {}", args[0].type_name()),
                0,
                0,
            ));
        }
    };

    let max = match &args[1] {
        Value::Number(n) => *n as i64,
        _ => {
            return Err(RuntimeError::new(
                format!("random_int expects numbers, got {}", args[1].type_name()),
                0,
                0,
            ));
        }
    };

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
    if !args.is_empty() {
        return Err(RuntimeError::new(
            format!("random_boolean expects 0 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    RNG.with(|rng| {
        let mut rng = rng.borrow_mut();
        let random_value: bool = rng.random();
        Ok(Value::Bool(random_value))
    })
}

/// Select a random element from a list
pub fn native_random_from(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("random_from expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    match &args[0] {
        Value::List(list_ref) => {
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
        _ => Err(RuntimeError::new(
            format!("random_from expects a list, got {}", args[0].type_name()),
            0,
            0,
        )),
    }
}

/// Set the random seed for reproducible results
pub fn native_random_seed(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("random_seed expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let seed = match &args[0] {
        Value::Number(n) => *n as u64,
        _ => {
            return Err(RuntimeError::new(
                format!("random_seed expects a number, got {}", args[0].type_name()),
                0,
                0,
            ));
        }
    };

    RNG.with(|rng| {
        // Replace the RNG with a seeded one
        *rng.borrow_mut() = StdRng::seed_from_u64(seed);
        Ok(Value::Nothing)
    })
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
