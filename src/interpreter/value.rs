use super::environment::Environment;
use super::error::RuntimeError;
use crate::parser::ast::Statement;
use crate::stdlib::pattern::CompiledPattern;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::{Rc, Weak};

#[derive(Clone)]
pub enum Value {
    Number(f64),
    Text(Rc<str>),
    Bool(bool),
    List(Rc<RefCell<Vec<Value>>>),
    Object(Rc<RefCell<HashMap<String, Value>>>),
    Function(Rc<FunctionValue>),
    NativeFunction(&'static str, NativeFunction),
    Future(Rc<RefCell<FutureValue>>),
    Date(Rc<chrono::NaiveDate>),
    Time(Rc<chrono::NaiveTime>),
    DateTime(Rc<chrono::NaiveDateTime>),
    Pattern(Rc<CompiledPattern>),
    Null,
}

pub type NativeFunction = fn(Vec<Value>) -> Result<Value, RuntimeError>;

#[derive(Clone)]
pub struct FunctionValue {
    pub name: Option<String>,
    pub params: Vec<String>,
    pub body: Vec<Statement>,
    pub env: Weak<RefCell<Environment>>,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone)]
pub struct FutureValue {
    pub value: Option<Result<Value, RuntimeError>>,
    pub completed: bool,
    pub line: usize,
    pub column: usize,
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_) => "Number",
            Value::Text(_) => "Text",
            Value::Bool(_) => "Boolean",
            Value::List(_) => "List",
            Value::Object(_) => "Object",
            Value::Function(_) => "Function",
            Value::NativeFunction(_, _) => "NativeFunction",
            Value::Future(_) => "Future",
            Value::Date(_) => "Date",
            Value::Time(_) => "Time",
            Value::DateTime(_) => "DateTime",
            Value::Pattern(_) => "Pattern",
            Value::Null => "Null",
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Number(n) => *n != 0.0,
            Value::Text(s) => !s.is_empty(),
            Value::List(list) => !list.borrow().is_empty(),
            Value::Object(obj) => !obj.borrow().is_empty(),
            Value::Function(_) | Value::NativeFunction(_, _) => true,
            Value::Future(future) => future.borrow().completed,
            Value::Date(_) | Value::Time(_) | Value::DateTime(_) => true,
            Value::Pattern(_) => true,
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::Text(s) => write!(f, "\"{}\"", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::List(l) => {
                let values = l.borrow();
                write!(f, "[")?;
                for (i, v) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:?}", v)?;
                }
                write!(f, "]")
            }
            Value::Object(o) => {
                let map = o.borrow();
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {:?}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Function(func) => {
                write!(
                    f,
                    "Function({})",
                    func.name.as_ref().unwrap_or(&"anonymous".to_string())
                )
            }
            Value::NativeFunction(name, _) => write!(f, "NativeFunction({})", name),
            Value::Future(_) => write!(f, "[Future]"),
            Value::Date(d) => write!(f, "Date({})", d),
            Value::Time(t) => write!(f, "Time({})", t),
            Value::DateTime(dt) => write!(f, "DateTime({})", dt),
            Value::Pattern(_) => write!(f, "[Pattern]"),
            Value::Null => write!(f, "null"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::Text(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", if *b { "yes" } else { "no" }),
            Value::List(_) => write!(f, "[List]"),
            Value::Object(o) => {
                let map = o.borrow();
                if map.len() == 1 {
                    if let Some((_, value)) = map.iter().next() {
                        write!(f, "{}", value)
                    } else {
                        write!(f, "[Object]")
                    }
                } else if map.is_empty() {
                    write!(f, "[Object]")
                } else {
                    write!(f, "{{")?;
                    for (i, (k, v)) in map.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}: {}", k, v)?;
                    }
                    write!(f, "}}")
                }
            }
            Value::Function(func) => {
                write!(
                    f,
                    "action {}",
                    func.name.as_ref().unwrap_or(&"anonymous".to_string())
                )
            }
            Value::NativeFunction(name, _) => write!(f, "native {}", name),
            Value::Future(_) => write!(f, "[Future]"),
            Value::Date(d) => write!(f, "{}", d.format("%Y-%m-%d")),
            Value::Time(t) => write!(f, "{}", t.format("%H:%M:%S")),
            Value::DateTime(dt) => write!(f, "{}", dt.format("%Y-%m-%d %H:%M:%S")),
            Value::Pattern(_) => write!(f, "[Pattern]"),
            Value::Null => write!(f, "nothing"),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Date(a), Value::Date(b)) => a == b,
            (Value::Time(a), Value::Time(b)) => a == b,
            (Value::DateTime(a), Value::DateTime(b)) => a == b,
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }
}
