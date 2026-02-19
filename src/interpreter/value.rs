use super::environment::Environment;
use super::error::RuntimeError;
use crate::parser::ast::Statement;
use crate::pattern::CompiledPattern;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::{Rc, Weak};
use std::sync::Arc;

#[derive(Clone)]
pub enum Value {
    Number(f64),
    Text(Arc<str>),
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
    Binary(Vec<u8>),
    Null,
    Nothing, // Used for void returns

    // Container-related values
    ContainerDefinition(Rc<ContainerDefinitionValue>),
    ContainerInstance(Rc<RefCell<ContainerInstanceValue>>),
    ContainerMethod(Rc<ContainerMethodValue>),
    ContainerEvent(Rc<ContainerEventValue>),
    InterfaceDefinition(Rc<InterfaceDefinitionValue>),
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

// Container-related structs
#[derive(Clone)]
pub struct ContainerDefinitionValue {
    pub name: String,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub properties: HashMap<String, PropertyDefinition>,
    pub methods: HashMap<String, ContainerMethodValue>,
    pub events: HashMap<String, ContainerEventValue>,
    pub static_properties: HashMap<String, Value>,
    pub static_methods: HashMap<String, ContainerMethodValue>,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone)]
pub struct PropertyDefinition {
    pub name: String,
    pub property_type: Option<String>,
    pub default_value: Option<Value>,
    pub validation_rules: Vec<ValidationRule>,
    pub is_static: bool,
    pub is_public: bool,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone)]
pub struct ValidationRule {
    pub rule_type: ValidationRuleType,
    pub parameters: Vec<Value>,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, PartialEq)]
pub enum ValidationRuleType {
    NotEmpty,
    MinLength,
    MaxLength,
    ExactLength,
    MinValue,
    MaxValue,
    Pattern,
    Custom,
}

#[derive(Clone)]
pub struct ContainerInstanceValue {
    pub container_type: String,
    pub properties: HashMap<String, Value>,
    pub parent: Option<Rc<RefCell<ContainerInstanceValue>>>,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone)]
pub struct ContainerMethodValue {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Statement>,
    pub is_static: bool,
    pub is_public: bool,
    pub env: Weak<RefCell<Environment>>,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone)]
pub struct ContainerEventValue {
    pub name: String,
    pub params: Vec<String>,
    pub handlers: Vec<EventHandler>,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone)]
pub struct EventHandler {
    pub body: Vec<Statement>,
    pub env: Weak<RefCell<Environment>>,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone)]
pub struct InterfaceDefinitionValue {
    pub name: String,
    pub extends: Vec<String>,
    pub required_actions: HashMap<String, ActionSignature>,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone)]
pub struct ActionSignature {
    pub name: String,
    pub params: Vec<String>,
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
            Value::Binary(_) => "Binary",
            Value::Null => "Null",
            Value::Nothing => "Nothing",
            Value::ContainerDefinition(_def) => "Container",
            Value::ContainerInstance(_) => "ContainerInstance",
            Value::ContainerMethod(_) => "ContainerMethod",
            Value::ContainerEvent(_) => "ContainerEvent",
            Value::InterfaceDefinition(_) => "Interface",
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
            Value::Binary(b) => !b.is_empty(),
            Value::Nothing => false,
            Value::ContainerDefinition(_) => true,
            Value::ContainerInstance(_) => true,
            Value::ContainerMethod(_) => true,
            Value::ContainerEvent(_) => true,
            Value::InterfaceDefinition(_) => true,
        }
    }

    /// Deep clone a value, creating independent copies of reference-counted containers.
    /// This is used for module isolation to prevent mutations from affecting parent scopes.
    pub fn deep_clone(&self) -> Self {
        match self {
            // For List, create a new Rc<RefCell<>> with recursively cloned elements
            Value::List(list) => {
                let cloned_vec = list
                    .borrow()
                    .iter()
                    .map(|v| v.deep_clone())
                    .collect::<Vec<_>>();
                Value::List(Rc::new(RefCell::new(cloned_vec)))
            }
            // For Object, create a new Rc<RefCell<>> with recursively cloned values
            Value::Object(obj) => {
                let cloned_map = obj
                    .borrow()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.deep_clone()))
                    .collect::<HashMap<_, _>>();
                Value::Object(Rc::new(RefCell::new(cloned_map)))
            }
            // For ContainerInstance, create a new Rc<RefCell<>> with deep cloned properties
            Value::ContainerInstance(instance) => {
                let inst = instance.borrow();
                let cloned_properties = inst
                    .properties
                    .iter()
                    .map(|(k, v)| (k.clone(), v.deep_clone()))
                    .collect::<HashMap<_, _>>();
                let cloned_parent = inst.parent.as_ref().map(|p| {
                    // Clone the parent reference, not deep clone (to avoid infinite recursion)
                    Rc::clone(p)
                });
                Value::ContainerInstance(Rc::new(RefCell::new(ContainerInstanceValue {
                    container_type: inst.container_type.clone(),
                    properties: cloned_properties,
                    parent: cloned_parent,
                    line: inst.line,
                    column: inst.column,
                })))
            }
            // For all other types, use regular clone (they're either primitives or immutable Rc types)
            _ => self.clone(),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{n}"),
            Value::Text(s) => write!(f, "\"{s}\""),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Nothing => write!(f, "nothing"),
            Value::List(l) => {
                let values = l.borrow();
                write!(f, "[")?;
                for (i, v) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v:?}")?;
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
                    write!(f, "{k}: {v:?}")?;
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
            Value::NativeFunction(name, _) => write!(f, "NativeFunction({name})"),
            Value::Future(_) => write!(f, "[Future]"),
            Value::Date(d) => write!(f, "Date({d})"),
            Value::Time(t) => write!(f, "Time({t})"),
            Value::DateTime(dt) => write!(f, "DateTime({dt})"),
            Value::Pattern(_) => write!(f, "[Pattern]"),
            Value::Binary(b) => write!(f, "[Binary: {} bytes]", b.len()),
            Value::Null => write!(f, "null"),
            Value::ContainerDefinition(def) => write!(f, "<container {}>", def.name),
            Value::ContainerInstance(instance) => {
                let instance = instance.borrow();
                write!(f, "<instance of {}>", instance.container_type)
            }
            Value::ContainerMethod(method) => write!(f, "<container method {}>", method.name),
            Value::ContainerEvent(event) => write!(f, "<container event {}>", event.name),
            Value::InterfaceDefinition(interface) => write!(f, "<interface {}>", interface.name),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{n}"),
            Value::Text(s) => write!(f, "{s}"),
            Value::Bool(b) => write!(f, "{}", if *b { "yes" } else { "no" }),
            Value::Nothing => write!(f, "nothing"),
            Value::List(list) => {
                let items = list.borrow();
                write!(f, "[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            Value::Object(o) => {
                let map = o.borrow();
                if map.len() == 1 {
                    if let Some((_, value)) = map.iter().next() {
                        write!(f, "{value}")
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
                        write!(f, "{k}: {v}")?;
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
            Value::NativeFunction(name, _) => write!(f, "native {name}"),
            Value::Future(_) => write!(f, "[Future]"),
            Value::Date(d) => write!(f, "{}", d.format("%Y-%m-%d")),
            Value::Time(t) => write!(f, "{}", t.format("%H:%M:%S")),
            Value::DateTime(dt) => write!(f, "{}", dt.format("%Y-%m-%d %H:%M:%S")),
            Value::Pattern(_) => write!(f, "[Pattern]"),
            Value::Binary(b) => write!(f, "[Binary: {} bytes]", b.len()),
            Value::Null => write!(f, "nothing"),
            Value::ContainerDefinition(def) => write!(f, "container {}", def.name),
            Value::ContainerInstance(instance) => {
                let instance = instance.borrow();
                write!(f, "{} instance", instance.container_type)
            }
            Value::ContainerMethod(method) => write!(f, "method {}", method.name),
            Value::ContainerEvent(event) => write!(f, "event {}", event.name),
            Value::InterfaceDefinition(interface) => write!(f, "interface {}", interface.name),
        }
    }
}

impl Value {
    // Internal helper for equality checking with cycle detection
    fn eq_with_visited(&self, other: &Self, visited: &mut HashSet<(*const (), *const ())>) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => numbers_equal(*a, *b),
            (Value::Text(a), Value::Text(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Date(a), Value::Date(b)) => a == b,
            (Value::Time(a), Value::Time(b)) => a == b,
            (Value::DateTime(a), Value::DateTime(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::Nothing, Value::Nothing) => true,

            (Value::List(a), Value::List(b)) => {
                if Rc::ptr_eq(a, b) {
                    return true;
                }

                let ptr_a = Rc::as_ptr(a) as *const ();
                let ptr_b = Rc::as_ptr(b) as *const ();
                let pair = (ptr_a, ptr_b);

                if visited.contains(&pair) {
                    return true; // Cycle detected, assume equal for now
                }
                visited.insert(pair);

                // Use try_borrow to avoid panics if already borrowed mutably
                match (a.try_borrow(), b.try_borrow()) {
                    (Ok(a_ref), Ok(b_ref)) => {
                        if a_ref.len() != b_ref.len() {
                            return false;
                        }
                        a_ref
                            .iter()
                            .zip(b_ref.iter())
                            .all(|(x, y)| x.eq_with_visited(y, visited))
                    }
                    _ => false, // Cannot compare if mutably borrowed elsewhere
                }
            }

            (Value::Object(a), Value::Object(b)) => {
                if Rc::ptr_eq(a, b) {
                    return true;
                }

                let ptr_a = Rc::as_ptr(a) as *const ();
                let ptr_b = Rc::as_ptr(b) as *const ();
                let pair = (ptr_a, ptr_b);

                if visited.contains(&pair) {
                    return true;
                }
                visited.insert(pair);

                match (a.try_borrow(), b.try_borrow()) {
                    (Ok(a_ref), Ok(b_ref)) => {
                        if a_ref.len() != b_ref.len() {
                            return false;
                        }
                        a_ref.iter().all(|(k, v)| {
                            b_ref
                                .get(k)
                                .is_some_and(|bv| v.eq_with_visited(bv, visited))
                        })
                    }
                    _ => false,
                }
            }

            (Value::Function(a), Value::Function(b)) => Rc::ptr_eq(a, b),
            (Value::NativeFunction(name_a, func_a), Value::NativeFunction(name_b, func_b)) => {
                // Use cast to usize for MSRV 1.75 compatibility (fn_addr_eq is 1.85+)
                // This compares the function code addresses directly
                name_a == name_b && (*func_a as usize) == (*func_b as usize)
            }
            (Value::Future(a), Value::Future(b)) => Rc::ptr_eq(a, b),
            (Value::Pattern(a), Value::Pattern(b)) => Rc::ptr_eq(a, b),
            (Value::Binary(a), Value::Binary(b)) => a == b,

            (Value::ContainerDefinition(a), Value::ContainerDefinition(b)) => a.name == b.name,
            (Value::ContainerInstance(a), Value::ContainerInstance(b)) => {
                if Rc::ptr_eq(a, b) {
                    return true;
                }

                let ptr_a = Rc::as_ptr(a) as *const ();
                let ptr_b = Rc::as_ptr(b) as *const ();
                let pair = (ptr_a, ptr_b);

                if visited.contains(&pair) {
                    return true;
                }
                visited.insert(pair);

                match (a.try_borrow(), b.try_borrow()) {
                    (Ok(a_ref), Ok(b_ref)) => {
                        if a_ref.container_type != b_ref.container_type {
                            return false;
                        }

                        // Compare parent hierarchy
                        let parents_match = match (&a_ref.parent, &b_ref.parent) {
                            (Some(p1), Some(p2)) => {
                                let v1 = Value::ContainerInstance(Rc::clone(p1));
                                let v2 = Value::ContainerInstance(Rc::clone(p2));
                                v1.eq_with_visited(&v2, visited)
                            }
                            (None, None) => true,
                            _ => false,
                        };

                        if !parents_match {
                            return false;
                        }

                        if a_ref.properties.len() != b_ref.properties.len() {
                            return false;
                        }
                        a_ref.properties.iter().all(|(k, v)| {
                            b_ref
                                .properties
                                .get(k)
                                .is_some_and(|bv| v.eq_with_visited(bv, visited))
                        })
                    }
                    _ => false,
                }
            }
            (Value::ContainerMethod(a), Value::ContainerMethod(b)) => a.name == b.name,
            (Value::ContainerEvent(a), Value::ContainerEvent(b)) => a.name == b.name,
            (Value::InterfaceDefinition(a), Value::InterfaceDefinition(b)) => a.name == b.name,
            _ => false,
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        // Optimization: Mismatched types are never equal.
        // This avoids allocating the cycle-detection HashSet for mismatched types (e.g. List == Number).
        if std::mem::discriminant(self) != std::mem::discriminant(other) {
            return false;
        }

        // Optimization: Check for reference identity before full cycle-safe comparison
        // This avoids allocating the HashSet for identical objects/lists
        match (self, other) {
            (Value::List(a), Value::List(b)) if Rc::ptr_eq(a, b) => return true,
            (Value::Object(a), Value::Object(b)) if Rc::ptr_eq(a, b) => return true,
            (Value::ContainerInstance(a), Value::ContainerInstance(b)) if Rc::ptr_eq(a, b) => {
                return true;
            }
            (Value::Function(a), Value::Function(b)) if Rc::ptr_eq(a, b) => return true,
            (Value::Future(a), Value::Future(b)) if Rc::ptr_eq(a, b) => return true,
            (Value::Pattern(a), Value::Pattern(b)) if Rc::ptr_eq(a, b) => return true,
            _ => {}
        }

        let mut visited = HashSet::new();
        self.eq_with_visited(other, &mut visited)
    }
}

/// Helper function for consistent numeric equality comparison using EPSILON
fn numbers_equal(a: f64, b: f64) -> bool {
    (a - b).abs() < f64::EPSILON
}
