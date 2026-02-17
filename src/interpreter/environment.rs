use super::value::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Debug)]
pub struct Environment {
    pub values: HashMap<String, Value>,
    pub constants: HashSet<String>,
    pub parent: Option<Rc<RefCell<Environment>>>,
    /// When true, provides module isolation: values from parent scopes are deep cloned
    /// to prevent mutations, and assignment to parent variables is prevented.
    pub isolated: bool,
}

impl Environment {
    pub fn new_global() -> Rc<RefCell<Self>> {
        #[cfg(feature = "dhat-ad-hoc")]
        dhat::ad_hoc_event(1);

        Rc::new(RefCell::new(Environment {
            values: HashMap::new(),
            constants: HashSet::new(),
            parent: None,
            isolated: false,
        }))
    }

    pub fn new(parent: &Rc<RefCell<Environment>>) -> Rc<RefCell<Self>> {
        #[cfg(feature = "dhat-ad-hoc")]
        dhat::ad_hoc_event(1);

        Rc::new(RefCell::new(Environment {
            values: HashMap::new(),
            constants: HashSet::new(),
            parent: Some(Rc::clone(parent)),
            isolated: false,
        }))
    }

    #[inline]
    pub fn new_child_env(parent: &Rc<RefCell<Environment>>) -> Rc<RefCell<Self>> {
        #[cfg(feature = "dhat-ad-hoc")]
        dhat::ad_hoc_event(1);

        Rc::new(RefCell::new(Self {
            values: HashMap::new(),
            constants: HashSet::new(),
            parent: Some(Rc::clone(parent)),
            isolated: false,
        }))
    }

    /// Creates an isolated child environment for module execution.
    /// Values from parent scopes are deep cloned to prevent mutations,
    /// and assignment to parent variables is prevented (read-only access).
    #[inline]
    pub fn new_isolated_child_env(parent: &Rc<RefCell<Environment>>) -> Rc<RefCell<Self>> {
        #[cfg(feature = "dhat-ad-hoc")]
        dhat::ad_hoc_event(1);

        Rc::new(RefCell::new(Self {
            values: HashMap::new(),
            constants: HashSet::new(),
            parent: Some(Rc::clone(parent)),
            isolated: true,
        }))
    }

    pub fn define(&mut self, name: &str, value: Value) -> Result<(), String> {
        // Check if the variable already exists in current scope
        if self.values.contains_key(name) {
            return Err(format!(
                "Variable '{name}' has already been defined. Use 'change {name} to <value>' to modify it."
            ));
        }

        // Check if the variable exists in parent scopes
        if let Some(parent) = &self.parent
            && parent.borrow().has(name)
        {
            return Err(format!(
                "Variable '{name}' has already been defined in an outer scope. Use 'change {name} to <value>' to modify it."
            ));
        }

        self.values.insert(name.to_string(), value);
        Ok(())
    }

    /// Defines a variable in the current scope without checking parent scopes for shadowing.
    /// This is an optimization for when existence in parent scopes has already been checked.
    pub fn define_direct(&mut self, name: &str, value: Value) -> Result<(), String> {
        // Check if the variable already exists in current scope
        if self.values.contains_key(name) {
            return Err(format!(
                "Variable '{name}' has already been defined. Use 'change {name} to <value>' to modify it."
            ));
        }

        self.values.insert(name.to_string(), value);
        Ok(())
    }

    pub fn define_constant(&mut self, name: &str, value: Value) -> Result<(), String> {
        // Check if the variable/constant already exists
        if self.values.contains_key(name) {
            return Err(format!(
                "Variable or constant '{name}' has already been defined."
            ));
        }

        // Check if the variable exists in parent scopes
        if let Some(parent) = &self.parent
            && parent.borrow().has(name)
        {
            return Err(format!(
                "Variable or constant '{name}' has already been defined in an outer scope."
            ));
        }

        self.values.insert(name.to_string(), value);
        self.constants.insert(name.to_string());
        Ok(())
    }

    pub fn define_constant_direct(&mut self, name: &str, value: Value) -> Result<(), String> {
        self.values.insert(name.to_string(), value);
        self.constants.insert(name.to_string());
        Ok(())
    }

    /// Clears all variables and constants from the current scope.
    /// Used for environment recycling in loops.
    pub fn clear(&mut self) {
        self.values.clear();
        self.constants.clear();
        // Parent, isolated status, and other flags remain unchanged
    }

    pub fn has(&self, name: &str) -> bool {
        if self.values.contains_key(name) {
            return true;
        }

        let mut current_parent = self.parent.clone();
        while let Some(parent_rc) = current_parent {
            let parent = parent_rc.borrow();
            if parent.values.contains_key(name) {
                return true;
            }
            current_parent = parent.parent.clone();
        }
        false
    }

    pub fn is_constant(&self, name: &str) -> bool {
        if self.constants.contains(name) {
            return true;
        }

        let mut current_parent = self.parent.clone();

        while let Some(parent_rc) = current_parent {
            let parent = parent_rc.borrow();
            if parent.constants.contains(name) {
                return true;
            }
            current_parent = parent.parent.clone();
        }

        false
    }

    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), String> {
        // Check if it's a constant in current scope
        if self.constants.contains(name) {
            return Err(format!("Cannot modify constant '{name}'"));
        }

        // Check current scope
        // Optimization: Use get_mut to update the value in place, avoiding a String allocation for the key.
        if let Some(val_ref) = self.values.get_mut(name) {
            *val_ref = value;
            return Ok(());
        }

        // Iteratively check parent scopes
        let mut current_parent = self.parent.clone();
        let mut is_isolated_context = self.isolated;

        while let Some(parent_rc) = current_parent {
            let mut parent = parent_rc.borrow_mut();

            if parent.constants.contains(name) {
                return Err(format!("Cannot modify constant '{name}'"));
            }

            // Optimization: Use get_mut to update the value in place, avoiding a String allocation for the key.
            if let Some(val_ref) = parent.values.get_mut(name) {
                // If we are in an isolated context (or passed through one), we cannot modify parent variable
                if is_isolated_context {
                    return Err(format!(
                        "Cannot modify parent variable '{name}' from module scope. Modules have read-only access to parent variables."
                    ));
                }

                *val_ref = value;
                return Ok(());
            }

            if parent.isolated {
                is_isolated_context = true;
            }

            // Move to next parent
            let next_parent = parent.parent.clone();
            drop(parent); // Release borrow
            current_parent = next_parent;
        }

        Err(format!("Undefined variable '{name}'"))
    }

    /// Get a value from the local scope only (does not check parent scopes)
    pub fn get_local(&self, name: &str) -> Option<Value> {
        self.values.get(name).cloned()
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        // Check local scope first
        if let Some(value) = self.values.get(name) {
            // Local values are returned as shallow clones.
            // Note: We do NOT deep clone local values even if self.isolated is true.
            // Isolation ensures we don't mutate PARENT variables, but local variables
            // in a module should be fully mutable by the module itself.
            return Some(value.clone());
        }

        // Iteratively check parent scopes
        let mut current_parent = self.parent.clone();
        let mut crossed_isolation_boundary = self.isolated;

        while let Some(parent_rc) = current_parent {
            let parent = parent_rc.borrow();

            if let Some(value) = parent.values.get(name) {
                // If we crossed an isolation boundary, deep clone the value
                return if crossed_isolation_boundary {
                    Some(value.deep_clone())
                } else {
                    Some(value.clone())
                };
            }

            // If this parent is isolated, it means it's isolated from ITS parent.
            // So any lookup further up the chain will cross an isolation boundary.
            if parent.isolated {
                crossed_isolation_boundary = true;
            }

            // Move to next parent
            current_parent = parent.parent.clone();
        }

        None
    }
}
