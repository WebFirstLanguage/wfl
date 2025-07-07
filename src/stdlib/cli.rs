use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

fn expect_text(value: &Value) -> Result<Rc<str>, RuntimeError> {
    match value {
        Value::Text(s) => Ok(Rc::clone(s)),
        _ => Err(RuntimeError::new(
            format!("Expected text, got {}", value.type_name()),
            0,
            0,
        )),
    }
}

pub fn native_get_args(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if !args.is_empty() {
        return Err(RuntimeError::new(
            format!("get_args expects 0 arguments, got {}", args.len()),
            0,
            0,
        ));
    }

    let args: Vec<String> = std::env::args().collect();
    let wfl_args: Vec<Value> = args
        .into_iter()
        .map(|arg| Value::Text(Rc::from(arg)))
        .collect();

    Ok(Value::List(Rc::new(RefCell::new(wfl_args))))
}

pub fn native_parse_flags(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("parse_flags expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let spec = expect_text(&args[0])?;
    let cmd_args: Vec<String> = std::env::args().skip(1).collect();

    let mut flags = HashMap::new();
    let mut i = 0;

    while i < cmd_args.len() {
        let arg = &cmd_args[i];

        if let Some(flag_name) = arg.strip_prefix("--") {
            if flag_name == "help" {
                flags.insert("help".to_string(), Value::Bool(true));
                i += 1;
                continue;
            }

            if spec.contains(&format!("{flag_name}: boolean")) {
                flags.insert(flag_name.to_string(), Value::Bool(true));
                i += 1;
            } else if spec.contains(&format!("{flag_name}: string"))
                || spec.contains(&format!("{flag_name}: choice"))
                || spec.contains(&format!("{flag_name}: number"))
            {
                if i + 1 < cmd_args.len() {
                    let value = &cmd_args[i + 1];
                    if spec.contains(&format!("{flag_name}: number")) {
                        match value.parse::<f64>() {
                            Ok(num) => {
                                flags.insert(flag_name.to_string(), Value::Number(num));
                            }
                            Err(_) => {
                                return Err(RuntimeError::new(
                                    format!("Invalid number for flag --{flag_name}: {value}"),
                                    0,
                                    0,
                                ));
                            }
                        }
                    } else {
                        flags.insert(flag_name.to_string(), Value::Text(Rc::from(value.as_str())));
                    }
                    i += 2;
                } else {
                    return Err(RuntimeError::new(
                        format!("Flag --{flag_name} requires a value"),
                        0,
                        0,
                    ));
                }
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }

    for line in spec.lines() {
        let line = line.trim();
        if line.contains("default") {
            if let Some(colon_pos) = line.find(':') {
                let flag_name = line[2..colon_pos].trim();
                if !flags.contains_key(flag_name) {
                    if let Some(default_pos) = line.find("default") {
                        let default_part = &line[default_pos + 7..].trim();
                        if line.contains("boolean") {
                            flags.insert(flag_name.to_string(), Value::Bool(false));
                        } else if line.contains("number") {
                            if let Ok(num) = default_part.parse::<f64>() {
                                flags.insert(flag_name.to_string(), Value::Number(num));
                            }
                        } else {
                            flags.insert(
                                flag_name.to_string(),
                                Value::Text(Rc::from(*default_part)),
                            );
                        }
                    }
                }
            }
        }
    }

    let mut result_map = HashMap::new();
    for (key, value) in flags {
        result_map.insert(key, value);
    }

    Ok(Value::Object(Rc::new(RefCell::new(result_map))))
}

pub fn native_usage(args: Vec<Value>) -> Result<Value, RuntimeError> {
    if args.len() != 1 {
        return Err(RuntimeError::new(
            format!("usage expects 1 argument, got {}", args.len()),
            0,
            0,
        ));
    }

    let spec = expect_text(&args[0])?;
    let mut usage_lines = Vec::new();

    usage_lines.push("Usage:".to_string());

    for line in spec.lines() {
        let line = line.trim();
        if line.starts_with("--") {
            if let Some(colon_pos) = line.find(':') {
                let flag_name = &line[2..colon_pos].trim();
                let flag_type = &line[colon_pos + 1..].trim();

                if flag_type.contains("boolean") {
                    usage_lines.push(format!("  --{flag_name:<20} (boolean flag)"));
                } else if flag_type.contains("string") {
                    usage_lines.push(format!("  --{flag_name:<20} <string>"));
                } else if flag_type.contains("number") {
                    usage_lines.push(format!("  --{flag_name:<20} <number>"));
                } else if flag_type.contains("choice") {
                    usage_lines.push(format!("  --{flag_name:<20} <choice>"));
                }
            }
        }
    }

    let usage_text = usage_lines.join("\n");
    Ok(Value::Text(Rc::from(usage_text)))
}

pub fn register_cli(env: &mut Environment) {
    env.define(
        "get_args",
        Value::NativeFunction("get_args", native_get_args),
    );
    env.define(
        "parse_flags",
        Value::NativeFunction("parse_flags", native_parse_flags),
    );
    env.define("usage", Value::NativeFunction("usage", native_usage));
}
