use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::rc::Rc;

pub fn native_args(_args: Vec<Value>) -> Result<Value, RuntimeError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let wfl_args: Vec<Value> = args
        .into_iter()
        .map(|arg| Value::Text(Rc::from(arg)))
        .collect();

    Ok(Value::List(Rc::new(RefCell::new(wfl_args))))
}

pub fn register_args(env: &mut Environment) {
    env.define("args", Value::NativeFunction("args", native_args));
}
