use super::value::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

#[derive(Debug)]
pub struct Environment {
    pub values: HashMap<String, Value>,
    pub constants: HashSet<String>,
    pub parent: Option<Weak<RefCell<Environment>>>,
    /// When true, provides module isolation: values from parent scopes are deep cloned
    /// to prevent mutations, and assignment to parent variables is prevented.
    pub isolated: bool,
    /// Canonical paths of the files `include from` has already run in this
    /// scope. An include of a file whose definitions are already visible
    /// here (recorded on this scope or an ancestor) is a no-op, which is
    /// what makes diamond includes work: two files that both include the
    /// same shared file reach it once, not twice.
    pub included_files: HashSet<PathBuf>,
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
            included_files: HashSet::new(),
        }))
    }

    pub fn new(parent: &Rc<RefCell<Environment>>) -> Rc<RefCell<Self>> {
        #[cfg(feature = "dhat-ad-hoc")]
        dhat::ad_hoc_event(1);

        Rc::new(RefCell::new(Environment {
            values: HashMap::new(),
            constants: HashSet::new(),
            parent: Some(Rc::downgrade(parent)),
            isolated: false,
            included_files: HashSet::new(),
        }))
    }

    #[inline]
    pub fn new_child_env(parent: &Rc<RefCell<Environment>>) -> Rc<RefCell<Self>> {
        #[cfg(feature = "dhat-ad-hoc")]
        dhat::ad_hoc_event(1);

        Rc::new(RefCell::new(Self {
            values: HashMap::new(),
            constants: HashSet::new(),
            parent: Some(Rc::downgrade(parent)),
            isolated: false,
            included_files: HashSet::new(),
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
            parent: Some(Rc::downgrade(parent)),
            isolated: true,
            included_files: HashSet::new(),
        }))
    }

    /// True when `path` was already included into this scope or into any
    /// scope this one can see, so its definitions are already reachable.
    pub fn has_included(&self, path: &Path) -> bool {
        if self.included_files.contains(path) {
            return true;
        }
        let mut parent = self.parent.as_ref().and_then(|weak| weak.upgrade());
        while let Some(env) = parent {
            let env_ref = env.borrow();
            if env_ref.included_files.contains(path) {
                return true;
            }
            parent = env_ref.parent.as_ref().and_then(|weak| weak.upgrade());
        }
        false
    }

    /// Records that `path` has been included into this scope.
    pub fn mark_included(&mut self, path: PathBuf) {
        self.included_files.insert(path);
    }

    pub fn define(&mut self, name: &str, value: Value) -> Result<(), String> {
        // Check if the variable already exists in current scope
        if self.values.contains_key(name) {
            return Err(format!(
                "Variable '{name}' has already been defined. Use 'change {name} to <value>' to modify it."
            ));
        }

        // Check if the variable exists in parent scopes
        if let Some(parent_weak) = &self.parent
            && let Some(parent) = parent_weak.upgrade()
            && parent.borrow().has(name)
        {
            return Err(format!(
                "Variable '{name}' has already been defined in an outer scope. Use 'change {name} to <value>' to modify it."
            ));
        }

        self.values.insert(name.to_string(), value);
        Ok(())
    }

    /// Defines an action, merging a same-scope redefinition into an overload
    /// set (`Value::Overloaded`) instead of erroring. Overloads must differ in
    /// parameter count or in at least one position where both declare
    /// concrete, different parameter types — mirroring the analyzer's
    /// definition-time rules. Collisions with non-function bindings and
    /// parent-scope shadowing keep the same errors as [`Self::define`].
    /// Returns the value now bound to `name`.
    pub fn define_or_merge_action(
        &mut self,
        name: &str,
        func: Rc<super::value::FunctionValue>,
    ) -> Result<Value, String> {
        self.define_or_merge_action_impl(name, func, false)
    }

    /// Defines a method-body action in the current call scope. This has the
    /// same overload behavior as [`Self::define_or_merge_action`], but permits
    /// the declaration to shadow a container property held in an outer method
    /// environment, matching the analyzer's lexical binding model.
    pub fn define_or_merge_action_direct(
        &mut self,
        name: &str,
        func: Rc<super::value::FunctionValue>,
    ) -> Result<Value, String> {
        self.define_or_merge_action_impl(name, func, true)
    }

    fn define_or_merge_action_impl(
        &mut self,
        name: &str,
        func: Rc<super::value::FunctionValue>,
        define_directly: bool,
    ) -> Result<Value, String> {
        use super::value::OverloadedFunction;

        let merged = match self.values.get(name) {
            Some(Value::Function(existing)) => {
                Self::check_overload_distinct(name, std::slice::from_ref(existing), &func)?;
                Some(vec![Rc::clone(existing), Rc::clone(&func)])
            }
            Some(Value::Overloaded(existing)) => {
                Self::check_overload_distinct(name, &existing.overloads, &func)?;
                let mut overloads = existing.overloads.clone();
                overloads.push(Rc::clone(&func));
                Some(overloads)
            }
            Some(_) => {
                return Err(format!(
                    "Variable '{name}' has already been defined. Use 'change {name} to <value>' to modify it."
                ));
            }
            None => None,
        };

        if let Some(overloads) = merged {
            // Every member of an overload set enforces its declared parameter
            // types at call time — including through previously captured
            // references to an individual member (snapshot aliases), which
            // behave as an overload set of one. Covers include-driven
            // overloads the interpreter's program pre-scan cannot see.
            for member in &overloads {
                member.enforce_param_types.set(true);
            }
            let value = Value::Overloaded(Rc::new(OverloadedFunction {
                name: name.to_string(),
                overloads,
            }));
            self.values.insert(name.to_string(), value.clone());
            return Ok(value);
        }

        let value = Value::Function(func);
        if define_directly {
            self.define_direct(name, value.clone())?;
        } else {
            self.define(name, value.clone())?;
        }
        Ok(value)
    }

    /// Rejects a new overload that an existing one could never be told apart
    /// from at a call site: same parameter count and no position where both
    /// declare concrete, different types.
    fn check_overload_distinct(
        name: &str,
        existing: &[Rc<super::value::FunctionValue>],
        new_func: &Rc<super::value::FunctionValue>,
    ) -> Result<(), String> {
        for prior in existing {
            if prior.param_types.len() != new_func.param_types.len() {
                continue;
            }
            // Exact duplicates get the analyzer's clearer wording — the
            // interpreter is the source of truth for include-driven and
            // dynamically-constructed definitions the analyzer never saw.
            if prior.param_types == new_func.param_types {
                return Err(format!(
                    "Action '{name}' was already defined with the same parameters (previous definition at line {}). \
                     Overloads must differ in parameter count or in their declared parameter types.",
                    prior.line
                ));
            }
            // Mirrors the analyzer's rule: `any`/`Unknown` annotations accept
            // every value, so they cannot separate two overloads.
            let is_concrete = |t: &Option<crate::parser::ast::Type>| {
                matches!(
                    t,
                    Some(inner) if !matches!(
                        inner,
                        crate::parser::ast::Type::Any | crate::parser::ast::Type::Unknown
                    )
                )
            };
            let distinguishable = prior
                .param_types
                .iter()
                .zip(&new_func.param_types)
                .any(|(a, b)| is_concrete(a) && is_concrete(b) && a != b);
            if !distinguishable {
                return Err(format!(
                    "Action '{name}' was already defined with {} parameter(s). \
                     Overloads must differ in parameter count or in their declared parameter types \
                     (e.g. 'value as number' vs 'value as text').",
                    new_func.param_types.len()
                ));
            }
        }
        Ok(())
    }

    pub fn define_native(
        &mut self,
        name: &'static str,
        func: crate::interpreter::value::NativeFunction,
    ) {
        let _ = self.define(
            name,
            crate::interpreter::value::Value::NativeFunction(name, func),
        );
    }

    /// Defines or overwrites a binding in the current scope, shadowing any
    /// parent-scope binding. Used for implicit bindings the runtime refreshes
    /// itself (e.g. request variables from `wait for request`), which must not
    /// fail when re-bound in the same scope.
    pub fn define_or_replace(&mut self, name: &str, value: Value) {
        // A refreshed implicit binding is never a constant; clear any stale
        // constant marker so the binding's state stays consistent.
        self.constants.remove(name);
        self.values.insert(name.to_string(), value);
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

    /// Returns the nearest parent scope that owns `name` directly.
    ///
    /// Method bodies use this to distinguish a container property's synthetic
    /// runtime binding from an ordinary outer lexical binding. The former may
    /// be shadowed by an explicit method-local binder; the latter retains WFL's
    /// historical no-shadowing rule.
    pub fn parent_scope_defining(&self, name: &str) -> Option<Rc<RefCell<Environment>>> {
        let mut candidate = self.parent.as_ref().and_then(Weak::upgrade);
        while let Some(scope) = candidate {
            let (defines_name, next) = {
                let borrowed = scope.borrow();
                (
                    borrowed.values.contains_key(name),
                    borrowed.parent.as_ref().and_then(Weak::upgrade),
                )
            };
            if defines_name {
                return Some(scope);
            }
            candidate = next;
        }
        None
    }

    /// Handles variable declarations and re-assignments in a single scope chain traversal.
    ///
    /// This method optimizes variable declaration (`store x as y`) by consolidating what was
    /// previously two separate operations (`has` followed by `define_direct` or `assign`).
    pub fn declare_variable(
        &mut self,
        name: &str,
        value: Value,
        is_constant: bool,
    ) -> Result<(), String> {
        // Check current scope
        if let Some(val_ref) = self.values.get_mut(name) {
            if is_constant {
                return Err(format!(
                    "Variable or constant '{name}' has already been defined."
                ));
            }
            if self.constants.contains(name) {
                return Err(format!("Cannot modify constant '{name}'"));
            }
            // Use assignment instead of definition
            *val_ref = value;
            return Ok(());
        }

        // Check parent scopes using the helper
        if let Some(result) = self.assign_in_parent_scope(name, value.clone(), is_constant) {
            return result;
        }

        // Variable doesn't exist, use normal definition
        self.values.insert(name.to_string(), value);
        if is_constant {
            self.constants.insert(name.to_string());
        }
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
        if let Some(parent_weak) = &self.parent
            && let Some(parent) = parent_weak.upgrade()
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

        let mut current_parent = self.parent.as_ref().and_then(|p| p.upgrade());
        while let Some(parent_rc) = current_parent {
            let parent = parent_rc.borrow();
            if parent.values.contains_key(name) {
                return true;
            }
            current_parent = parent.parent.as_ref().and_then(|p| p.upgrade());
        }
        false
    }

    pub fn is_constant(&self, name: &str) -> bool {
        if self.constants.contains(name) {
            return true;
        }

        let mut current_parent = self.parent.as_ref().and_then(|p| p.upgrade());

        while let Some(parent_rc) = current_parent {
            let parent = parent_rc.borrow();
            if parent.constants.contains(name) {
                return true;
            }
            current_parent = parent.parent.as_ref().and_then(|p| p.upgrade());
        }

        false
    }

    /// Common helper to find a mutable reference to a variable in parent scopes.
    /// Handles traversal, isolated contexts, and checking constants.
    /// Used by both `assign` and `declare_variable`.
    fn assign_in_parent_scope(
        &self,
        name: &str,
        value: Value,
        enforce_constant_shadowing: bool,
    ) -> Option<Result<(), String>> {
        let mut current_parent = self.parent.as_ref().and_then(|p| p.upgrade());
        let mut is_isolated_context = self.isolated;

        while let Some(parent_rc) = current_parent {
            let mut parent = parent_rc.borrow_mut();

            let is_parent_constant = parent.constants.contains(name);

            if let Some(val_ref) = parent.values.get_mut(name) {
                if enforce_constant_shadowing {
                    return Some(Err(format!(
                        "Variable or constant '{name}' has already been defined in an outer scope."
                    )));
                } else if is_parent_constant {
                    return Some(Err(format!("Cannot modify constant '{name}'")));
                }

                if is_isolated_context {
                    return Some(Err(format!(
                        "Cannot modify parent variable '{name}' from module scope. Modules have read-only access to parent variables."
                    )));
                }

                *val_ref = value;
                return Some(Ok(()));
            }

            if parent.isolated {
                is_isolated_context = true;
            }

            let next_parent = parent.parent.as_ref().and_then(|p| p.upgrade());
            drop(parent);
            current_parent = next_parent;
        }

        None
    }

    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), String> {
        // Check current scope
        if let Some(val_ref) = self.values.get_mut(name) {
            if self.constants.contains(name) {
                return Err(format!("Cannot modify constant '{name}'"));
            }
            *val_ref = value;
            return Ok(());
        }

        // Check parent scopes using the helper
        if let Some(result) = self.assign_in_parent_scope(name, value, false) {
            return result;
        }

        Err(format!("Undefined variable '{name}'"))
    }

    /// Get a value from the local scope only (does not check parent scopes)
    pub fn get_local(&self, name: &str) -> Option<Value> {
        self.values.get(name).cloned()
    }

    /// Remove and return a binding from this scope only. Used for temporary
    /// clause-local aliases that must reveal any outer/local binding again
    /// after a handler finishes.
    pub fn take_local_binding(&mut self, name: &str) -> Option<(Value, bool)> {
        let value = self.values.remove(name)?;
        let was_constant = self.constants.remove(name);
        Some((value, was_constant))
    }

    /// Replace the current local binding with a previously saved one, or remove
    /// it when no binding existed before the temporary override.
    pub fn restore_local_binding(&mut self, name: &str, saved: Option<(Value, bool)>) {
        self.values.remove(name);
        self.constants.remove(name);
        if let Some((value, was_constant)) = saved {
            self.values.insert(name.to_string(), value);
            if was_constant {
                self.constants.insert(name.to_string());
            }
        }
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
        let mut current_parent = self.parent.as_ref().and_then(|p| p.upgrade());
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
            current_parent = parent.parent.as_ref().and_then(|p| p.upgrade());
        }

        None
    }
}
