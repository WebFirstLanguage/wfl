use crate::parser::ast::{Expression, Literal, Parameter, Program, Statement, Type};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionSignature {
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable { mutable: bool },
    Function { signatures: Vec<FunctionSignature> },
    Pattern,
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub name: String,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub properties: HashMap<String, PropertyInfo>,
    pub methods: HashMap<String, MethodInfo>,
    pub static_properties: HashMap<String, PropertyInfo>,
    pub static_methods: HashMap<String, MethodInfo>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct PropertyInfo {
    pub name: String,
    pub property_type: Type,
    pub is_public: bool,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Type,
    pub is_public: bool,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub symbol_type: Option<Type>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub symbols: HashMap<String, Symbol>,
    pub parent: Option<Box<Scope>>,
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

impl Scope {
    pub fn new() -> Self {
        Scope {
            symbols: HashMap::new(),
            parent: None,
        }
    }

    pub fn with_parent(parent: Scope) -> Self {
        Scope {
            symbols: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn define(&mut self, symbol: Symbol) -> Result<(), SemanticError> {
        // Check if variable already exists in current scope
        if self.symbols.contains_key(&symbol.name) {
            let existing = &self.symbols[&symbol.name];
            return Err(SemanticError::new(
                format!(
                    "Variable '{}' has already been defined at line {}. Use 'change {} to <value>' to modify it.",
                    symbol.name, existing.line, symbol.name
                ),
                symbol.line,
                symbol.column,
            ));
        }

        // Check if variable exists in parent scopes
        if let Some(parent) = &self.parent
            && parent.resolve(&symbol.name).is_some()
        {
            return Err(SemanticError::new(
                format!(
                    "Variable '{}' has already been defined in an outer scope. Use 'change {} to <value>' to modify it.",
                    symbol.name, symbol.name
                ),
                symbol.line,
                symbol.column,
            ));
        }

        self.symbols.insert(symbol.name.clone(), symbol);
        Ok(())
    }

    /// Defines or overwrites a symbol in this scope, shadowing any binding of
    /// the same name from a parent scope. Used for implicit bindings that the
    /// runtime refreshes itself (loop counters, request bindings), which must
    /// resolve to the implicit symbol rather than an outer variable.
    pub fn define_or_replace(&mut self, symbol: Symbol) {
        self.symbols.insert(symbol.name.clone(), symbol);
    }

    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        if let Some(symbol) = self.symbols.get(name) {
            Some(symbol)
        } else if let Some(parent) = &self.parent {
            parent.resolve(name)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct SemanticError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl SemanticError {
    pub fn new(message: String, line: usize, column: usize) -> Self {
        SemanticError {
            message,
            line,
            column,
        }
    }
}

impl fmt::Display for SemanticError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Semantic error at line {}, column {}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for SemanticError {}

/// Returns true if the program contains any top-level `include from` statement.
///
/// Included files are resolved dynamically at runtime, so their presence relaxes
/// static undefined-action reporting in both the analyzer and the type checker.
/// Shared so the two stay consistent if the detection rule ever changes.
pub fn program_has_includes(program: &Program) -> bool {
    program
        .statements
        .iter()
        .any(|s| matches!(s, Statement::IncludeStatement { .. }))
}

/// True when the program contains a top-level `load module from` statement.
///
/// `load module` runs a file in an *isolated* child scope and does not expose
/// its actions/containers/variables to the caller (unlike `include from`). When
/// a caller references such a symbol it is genuinely undefined — fatal at
/// analysis *and* at runtime — so this is used to attach an actionable
/// "use `include from`" note to the undefined-name diagnostic rather than to
/// relax it (see issue #584).
pub fn program_has_load_module(program: &Program) -> bool {
    program
        .statements
        .iter()
        .any(|s| matches!(s, Statement::LoadModuleStatement { .. }))
}

pub struct Analyzer {
    current_scope: Scope,
    errors: Vec<SemanticError>,
    /// Non-fatal semantic warnings. Currently used for undefined-action calls in
    /// programs that use `include from`: the action may be provided by an
    /// included module at runtime, but it could also be a typo, so it is
    /// surfaced as a warning rather than a fatal error or silent suppression.
    warnings: Vec<SemanticError>,
    action_parameters: std::collections::HashSet<String>,
    containers: HashMap<String, ContainerInfo>,
    current_container: Option<String>,
    /// True when the program contains `include from` statements. Included files
    /// are resolved dynamically at runtime, so the analyzer cannot know which
    /// actions/variables they expose; undefined-action errors are downgraded to
    /// warnings to avoid false fatal failures for include-exposed actions.
    has_includes: bool,
    /// Nesting depth of `try` bodies currently being analyzed. Undefined-name
    /// references inside a `try` body raise catchable runtime errors (documented
    /// behavior), so they are reported as warnings instead of fatal errors.
    try_depth: usize,
    /// Loop variables of the count loops currently being analyzed. Nested
    /// count loops reusing the same variable name are reported as errors,
    /// while shadowing an ordinary outer variable is allowed.
    active_loop_variables: Vec<String>,
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer {
    pub fn new() -> Self {
        let mut global_scope = Scope::new();

        let yes_symbol = Symbol {
            name: "yes".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Boolean),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(yes_symbol);

        let no_symbol = Symbol {
            name: "no".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Boolean),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(no_symbol);

        let nothing_symbol = Symbol {
            name: "nothing".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Nothing),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(nothing_symbol);

        let newline_symbol = Symbol {
            name: "newline".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Text),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(newline_symbol);

        let tab_symbol = Symbol {
            name: "tab".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Text),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(tab_symbol);

        let missing_symbol = Symbol {
            name: "missing".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Nothing),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(missing_symbol);

        let undefined_symbol = Symbol {
            name: "undefined".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Nothing),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(undefined_symbol);

        let push_symbol = Symbol {
            name: "push".to_string(),
            kind: SymbolKind::Function {
                signatures: vec![FunctionSignature {
                    parameters: vec![
                        Parameter {
                            name: "list".to_string(),
                            param_type: Some(Type::List(Box::new(Type::Unknown))),
                            default_value: None,
                            line: 0,
                            column: 0,
                        },
                        Parameter {
                            name: "value".to_string(),
                            param_type: Some(Type::Unknown),
                            default_value: None,
                            line: 0,
                            column: 0,
                        },
                    ],
                    return_type: Some(Type::Nothing),
                }],
            },
            symbol_type: Some(Type::Function {
                parameters: vec![Type::List(Box::new(Type::Unknown)), Type::Unknown],
                return_type: Box::new(Type::Nothing),
            }),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(push_symbol);

        let loop_symbol = Symbol {
            name: "loop".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Unknown),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(loop_symbol);

        // Define runtime command-line argument variables
        // These are defined at runtime by the interpreter but need to be known
        // to the static analyzer to avoid false undefined variable errors

        let arg_count_symbol = Symbol {
            name: "arg_count".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Number),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(arg_count_symbol);

        let args_symbol = Symbol {
            name: "args".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::List(Box::new(Type::Text))),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(args_symbol);

        let program_name_symbol = Symbol {
            name: "program_name".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Text),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(program_name_symbol);

        let current_directory_symbol = Symbol {
            name: "current_directory".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Text),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(current_directory_symbol);

        let positional_args_symbol = Symbol {
            name: "positional_args".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::List(Box::new(Type::Text))),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(positional_args_symbol);

        let script_path_symbol = Symbol {
            name: "script_path".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Text),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(script_path_symbol);

        let script_directory_symbol = Symbol {
            name: "script_directory".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Text),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(script_directory_symbol);

        Analyzer {
            current_scope: global_scope,
            errors: Vec::new(),
            warnings: Vec::new(),
            action_parameters: std::collections::HashSet::new(),
            containers: HashMap::new(),
            current_container: None,
            has_includes: false,
            try_depth: 0,
            active_loop_variables: Vec::new(),
        }
    }

    /// Create an analyzer with parent scope variables
    /// Parent variables are added as read-only (immutable) to prevent modification warnings
    pub fn with_parent_variables(parent_vars: HashMap<String, (Type, bool)>) -> Self {
        let mut analyzer = Self::new();

        // Add parent variables as read-only symbols
        for (name, (var_type, _is_mutable)) in parent_vars {
            let symbol = Symbol {
                name: name.clone(),
                kind: SymbolKind::Variable { mutable: false }, // Read-only!
                symbol_type: Some(var_type),
                line: 0,
                column: 0,
            };
            let _ = analyzer.current_scope.define(symbol);
        }

        analyzer
    }

    /// Create an analyzer with parent variables preserving their mutability (used for includes)
    pub fn with_parent_variables_mutable(parent_vars: HashMap<String, (Type, bool)>) -> Self {
        let mut analyzer = Self::new();

        // Add parent variables preserving their mutability from parent scope
        for (name, (var_type, is_mutable)) in parent_vars {
            let symbol = Symbol {
                name: name.clone(),
                kind: SymbolKind::Variable {
                    mutable: is_mutable,
                }, // Preserve mutability
                symbol_type: Some(var_type),
                line: 0,
                column: 0,
            };
            let _ = analyzer.current_scope.define(symbol);
        }

        analyzer
    }

    pub fn is_builtin_function(name: &str) -> bool {
        crate::builtins::is_builtin_function(name)
    }

    pub fn analyze(&mut self, program: &Program) -> Result<(), Vec<SemanticError>> {
        // Detect include statements up front. Includes are resolved at runtime
        // and can expose actions/variables the analyzer never sees, so their
        // presence relaxes undefined-action reporting (see `has_includes`).
        // Assign directly (not just set-to-true) so a reused analyzer instance
        // does not carry a stale flag from a previous program.
        self.has_includes = program_has_includes(program);

        // PASS 1: Register all top-level action signatures
        // This allows forward references between actions at the top level
        for statement in &program.statements {
            self.register_action_signature(statement);
        }

        // PASS 2: Analyze all statements (including action bodies)
        for statement in &program.statements {
            self.analyze_statement(statement);
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    pub fn get_errors(&self) -> &Vec<SemanticError> {
        &self.errors
    }

    /// Non-fatal semantic warnings collected during analysis (e.g. undefined
    /// actions in a program that uses `include from`).
    pub fn get_warnings(&self) -> &Vec<SemanticError> {
        &self.warnings
    }

    /// Report an undefined-name reference. Inside a `try` body this is a
    /// warning rather than a fatal error: the reference raises a catchable
    /// runtime error, which is documented behavior that programs rely on.
    fn report_undefined_name(&mut self, message: String, line: usize, column: usize) {
        let error = SemanticError::new(message, line, column);
        if self.try_depth > 0 {
            self.warnings.push(error);
        } else {
            self.errors.push(error);
        }
    }

    /// Include-aware relaxation for a callee that is not in scope and not a
    /// builtin. When the program uses `include from`, the callee may be an
    /// action exposed by an included file at runtime (which the analyzer cannot
    /// see), so a fatal error would abort before the include runs — instead a
    /// non-fatal `Undefined action` warning is emitted and `true` is returned.
    /// When there are no includes, nothing is emitted and `false` is returned,
    /// signalling the caller to apply its own fatal handling.
    ///
    /// Both the `of` form (`FunctionCall` with a bare-`Variable` callee) and the
    /// `call ... with` form (`ActionCall`) route through this single method so
    /// the relaxation cannot drift apart between the two paths again — the exact
    /// divergence that was issue #580 (the `of` form never received #548's
    /// `ActionCall`-only relaxation).
    fn warn_undefined_callee_if_includes(
        &mut self,
        name: &str,
        line: usize,
        column: usize,
    ) -> bool {
        if self.has_includes {
            self.warnings.push(SemanticError::new(
                format!("Undefined action '{name}'"),
                line,
                column,
            ));
            true
        } else {
            false
        }
    }

    fn analyze_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::VariableDeclaration {
                name,
                value,
                is_constant,
                line,
                column,
            } => {
                self.analyze_expression(value);

                let symbol = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable {
                        mutable: !is_constant,
                    }, // Constants are immutable
                    symbol_type: None, // Type will be inferred later
                    line: *line,
                    column: *column,
                };

                // Check if this is actually a container property assignment (including inherited)
                let is_property_assignment = if let Some(container_name) = &self.current_container {
                    self.is_container_property(container_name, name)
                } else {
                    false
                };

                // This is actually a property assignment, not a variable declaration
                // Don't treat it as an error - the interpreter will handle it

                if !is_property_assignment && let Err(error) = self.current_scope.define(symbol) {
                    self.errors.push(error);
                }
            }
            Statement::Assignment {
                name,
                value,
                line,
                column,
            } => {
                let mut skip_value_analysis = false;

                if let Some(symbol) = self.current_scope.resolve(name) {
                    match &symbol.kind {
                        SymbolKind::Variable { mutable } => {
                            if !mutable {
                                self.errors.push(SemanticError::new(
                                    format!("Cannot modify constant '{name}' - constants are immutable once defined"),
                                    *line,
                                    *column,
                                ));
                                // Skip analyzing the value expression since the assignment itself is invalid
                                skip_value_analysis = true;
                            }
                        }
                        _ => {
                            self.errors.push(SemanticError::new(
                                format!("'{name}' is not a variable"),
                                0, // Need location info
                                0,
                            ));
                        }
                    }
                } else {
                    // Check if it's a container property assignment (including inherited)
                    let is_container_property =
                        if let Some(container_name) = &self.current_container {
                            self.is_container_property(container_name, name)
                        } else {
                            false
                        };

                    if !is_container_property {
                        self.report_undefined_name(
                            format!("Variable '{name}' is not defined"),
                            *line,
                            *column,
                        );
                    }
                }

                // Only analyze the value expression if the assignment is potentially valid
                if !skip_value_analysis {
                    self.analyze_expression(value);
                }
            }
            Statement::ActionDefinition { .. } => {
                // Signature was already registered in Pass 1
                // Now analyze the body in Pass 2
                self.analyze_action_body(statement);
            }
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.analyze_expression(condition);

                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope.clone());

                for stmt in then_block {
                    self.analyze_statement(stmt);
                }

                let then_scope = std::mem::take(&mut self.current_scope);
                let mut defined_in_then = Vec::new();

                for (name, symbol) in &then_scope.symbols {
                    if outer_scope.resolve(name).is_none() {
                        defined_in_then.push((name.clone(), symbol.clone()));
                    }
                }

                if let Some(parent) = then_scope.parent {
                    self.current_scope = *parent;
                }

                let mut defined_in_else = Vec::new();
                if let Some(else_stmts) = else_block {
                    let outer_scope_for_else = std::mem::take(&mut self.current_scope);
                    self.current_scope = Scope::with_parent(outer_scope_for_else.clone());

                    for stmt in else_stmts {
                        self.analyze_statement(stmt);
                    }

                    let else_scope = std::mem::take(&mut self.current_scope);

                    for (name, symbol) in &else_scope.symbols {
                        if outer_scope_for_else.resolve(name).is_none() {
                            defined_in_else.push((name.clone(), symbol.clone()));
                        }
                    }

                    if let Some(parent) = else_scope.parent {
                        self.current_scope = *parent;
                    }
                }

                // Variables defined in both branches are definitely defined
                for (name, symbol) in &defined_in_then {
                    if (defined_in_else.iter().any(|(n, _)| n == name) || else_block.is_none())
                        && let Err(error) = self.current_scope.define(symbol.clone())
                    {
                        self.errors.push(error);
                    }
                }

                for (name, symbol) in &defined_in_else {
                    if !defined_in_then.iter().any(|(n, _)| n == name)
                        && let Err(error) = self.current_scope.define(symbol.clone())
                    {
                        self.errors.push(error);
                    }
                }
            }
            Statement::SingleLineIf {
                condition,
                then_stmt,
                else_stmt,
                ..
            } => {
                self.analyze_expression(condition);

                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope);

                self.analyze_statement(then_stmt);

                let then_scope = std::mem::take(&mut self.current_scope);
                if let Some(parent) = then_scope.parent {
                    self.current_scope = *parent;
                }

                if let Some(else_stmt) = else_stmt {
                    let outer_scope = std::mem::take(&mut self.current_scope);
                    self.current_scope = Scope::with_parent(outer_scope);

                    self.analyze_statement(else_stmt);

                    let else_scope = std::mem::take(&mut self.current_scope);
                    if let Some(parent) = else_scope.parent {
                        self.current_scope = *parent;
                    }
                }
            }
            Statement::ForEachLoop {
                item_name,
                collection,
                body,
                ..
            } => {
                self.analyze_expression(collection);

                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope);

                let item_symbol = Symbol {
                    name: item_name.clone(),
                    kind: SymbolKind::Variable { mutable: false }, // Loop variables are immutable
                    symbol_type: None, // Type will be inferred from collection
                    line: 0,
                    column: 0,
                };

                if let Err(error) = self.current_scope.define(item_symbol) {
                    self.errors.push(error);
                }

                // Add item to action_parameters to prevent it from being flagged as undefined
                self.action_parameters.insert(item_name.clone());

                for stmt in body {
                    self.analyze_statement(stmt);
                }

                // Remove loop variable from action_parameters after the loop
                self.action_parameters.remove(item_name);

                let loop_scope = std::mem::take(&mut self.current_scope);
                if let Some(parent) = loop_scope.parent {
                    self.current_scope = *parent;
                }
            }
            Statement::CountLoop {
                start,
                end,
                step,
                variable_name,
                body,
                line,
                column,
                ..
            } => {
                self.analyze_expression(start);
                self.analyze_expression(end);
                if let Some(step_expr) = step {
                    self.analyze_expression(step_expr);
                }

                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope);

                // Use custom variable name if provided, otherwise default to "count"
                let loop_var_name = variable_name.as_deref().unwrap_or("count");

                // Nested count loops must use distinct variable names; the
                // inner loop would otherwise shadow the outer loop's counter.
                if self
                    .active_loop_variables
                    .iter()
                    .any(|name| name == loop_var_name)
                {
                    self.errors.push(SemanticError::new(
                        format!(
                            "Nested count loops both use the loop variable '{loop_var_name}'. Give the loops distinct names with 'count from X to Y as <name>:'."
                        ),
                        *line,
                        *column,
                    ));
                }

                let count_symbol = Symbol {
                    name: loop_var_name.to_string(), // The loop variable is implicitly defined
                    kind: SymbolKind::Variable { mutable: false }, // Loop variable is immutable
                    symbol_type: Some(Type::Number), // Loop variable is always a number
                    line: *line,
                    column: *column,
                };

                // The implicit loop variable may shadow an ordinary variable of
                // the same name from an outer scope (nested loops are handled
                // above), and must resolve to the loop counter inside the loop
                // — matching the interpreter, which rebinds it each iteration.
                self.current_scope.define_or_replace(count_symbol);

                // Add loop variable to action_parameters to prevent it from being flagged as undefined
                self.action_parameters.insert(loop_var_name.to_string());
                self.active_loop_variables.push(loop_var_name.to_string());

                for stmt in body {
                    self.analyze_statement(stmt);
                }

                // Remove loop variable from action_parameters after the loop
                self.action_parameters.remove(loop_var_name);
                self.active_loop_variables.pop();

                let loop_scope = std::mem::take(&mut self.current_scope);
                if let Some(parent) = loop_scope.parent {
                    self.current_scope = *parent;
                }
            }
            Statement::WhileLoop {
                condition, body, ..
            } => {
                self.analyze_expression(condition);

                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope);

                for stmt in body {
                    self.analyze_statement(stmt);
                }

                let loop_scope = std::mem::take(&mut self.current_scope);
                if let Some(parent) = loop_scope.parent {
                    self.current_scope = *parent;
                }
            }
            Statement::DisplayStatement { value, .. } => {
                self.analyze_expression(value);
            }
            Statement::ExpressionStatement { expression, .. } => {
                if let Expression::FunctionCall {
                    function,
                    arguments,
                    ..
                } = expression
                    && let Expression::Variable(func_name, _, _) = &**function
                    && func_name == "push"
                    && arguments.len() >= 2
                    && let Expression::Variable(list_name, line, column) = &arguments[0].value
                    && self.current_scope.resolve(list_name).is_none()
                {
                    let list_symbol = Symbol {
                        name: list_name.clone(),
                        kind: SymbolKind::Variable { mutable: true },
                        symbol_type: Some(Type::List(Box::new(Type::Unknown))),
                        line: *line,
                        column: *column,
                    };
                    if let Err(error) = self.current_scope.define(list_symbol) {
                        self.errors.push(error);
                    }
                }
                self.analyze_expression(expression);
            }
            Statement::ReturnStatement {
                value: Some(expr), ..
            } => {
                self.analyze_expression(expr);
            }
            Statement::ReturnStatement { value: None, .. } => {}
            Statement::WaitForStatement {
                inner,
                line: _line,
                column: _column,
            } => {
                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope);

                // The inner statement's own analysis defines any variables it
                // introduces (file handles, read content, database handles,
                // query results); pre-defining them here would make
                // Scope::define report a duplicate. The wait-scope merge
                // below hoists them into the parent scope.
                self.analyze_statement(inner);

                let wait_scope = std::mem::take(&mut self.current_scope);
                if let Some(parent) = wait_scope.parent {
                    let mut parent_mut = *parent;
                    for (name, symbol) in wait_scope.symbols {
                        if parent_mut.resolve(&name).is_none() {
                            let _ = parent_mut.define(symbol);
                        }
                    }
                    self.current_scope = parent_mut;
                }
            }

            Statement::WaitForDurationStatement { duration, .. } => {
                self.analyze_expression(duration);
            }

            Statement::TryStatement {
                body,
                when_clauses,
                otherwise_block,
                finally_block,
                ..
            } => {
                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope);

                // Undefined names inside a try body raise catchable runtime
                // errors, so they are downgraded to warnings while in here.
                self.try_depth += 1;
                for stmt in body {
                    self.analyze_statement(stmt);
                }
                self.try_depth -= 1;

                let try_scope = std::mem::take(&mut self.current_scope);
                if let Some(parent) = try_scope.parent {
                    self.current_scope = *parent;
                }

                // Analyze each when clause
                for when_clause in when_clauses {
                    let outer_scope = std::mem::take(&mut self.current_scope);
                    self.current_scope = Scope::with_parent(outer_scope);

                    let error_symbol = Symbol {
                        name: when_clause.error_name.clone(),
                        kind: SymbolKind::Variable { mutable: false },
                        symbol_type: Some(Type::Text), // Error messages are text
                        line: 0,
                        column: 0,
                    };

                    if let Err(error) = self.current_scope.define(error_symbol) {
                        self.errors.push(error);
                    }

                    // `error_message` is always available in error-handling
                    // clauses as an alias for the caught error's message.
                    if when_clause.error_name != "error_message" {
                        let error_message_symbol = Symbol {
                            name: "error_message".to_string(),
                            kind: SymbolKind::Variable { mutable: false },
                            symbol_type: Some(Type::Text),
                            line: 0,
                            column: 0,
                        };
                        let _ = self.current_scope.define(error_message_symbol);
                    }

                    for stmt in &when_clause.body {
                        self.analyze_statement(stmt);
                    }

                    let when_scope = std::mem::take(&mut self.current_scope);
                    if let Some(parent) = when_scope.parent {
                        self.current_scope = *parent;
                    }
                }

                if let Some(otherwise_stmts) = otherwise_block {
                    let outer_scope = std::mem::take(&mut self.current_scope);
                    self.current_scope = Scope::with_parent(outer_scope);

                    for stmt in otherwise_stmts {
                        self.analyze_statement(stmt);
                    }

                    let otherwise_scope = std::mem::take(&mut self.current_scope);
                    if let Some(parent) = otherwise_scope.parent {
                        self.current_scope = *parent;
                    }
                }

                if let Some(finally_stmts) = finally_block {
                    let outer_scope = std::mem::take(&mut self.current_scope);
                    self.current_scope = Scope::with_parent(outer_scope);

                    for stmt in finally_stmts {
                        self.analyze_statement(stmt);
                    }

                    let finally_scope = std::mem::take(&mut self.current_scope);
                    if let Some(parent) = finally_scope.parent {
                        self.current_scope = *parent;
                    }
                }
            }
            Statement::ReadFileStatement { variable_name, .. } => {
                let symbol = Symbol {
                    name: variable_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: Some(Type::Text), // File content is always text
                    line: 0,
                    column: 0,
                };

                if let Err(error) = self.current_scope.define(symbol) {
                    self.errors.push(error);
                }
            }
            Statement::OpenFileStatement { variable_name, .. } => {
                let symbol = Symbol {
                    name: variable_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: None, // File handle type
                    line: 0,
                    column: 0,
                };

                if let Err(error) = self.current_scope.define(symbol) {
                    self.errors.push(error);
                }
            }
            Statement::OpenDatabaseStatement {
                url,
                variable_name,
                line,
                column,
            } => {
                self.analyze_expression(url);

                let symbol = Symbol {
                    name: variable_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: None, // Database handle type
                    line: *line,
                    column: *column,
                };

                if let Err(error) = self.current_scope.define(symbol) {
                    self.errors.push(error);
                }
            }
            Statement::DatabaseQueryStatement {
                db,
                sql,
                parameters,
                variable_name,
                line,
                column,
                ..
            } => {
                self.analyze_expression(db);
                self.analyze_expression(sql);
                if let Some(params) = parameters {
                    self.analyze_expression(params);
                }

                let symbol = Symbol {
                    name: variable_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: None, // Query result type
                    line: *line,
                    column: *column,
                };

                if let Err(error) = self.current_scope.define(symbol) {
                    self.errors.push(error);
                }
            }
            Statement::CloseDatabaseStatement { db, .. } => {
                self.analyze_expression(db);
            }
            Statement::HttpGetStatement { variable_name, .. } => {
                let symbol = Symbol {
                    name: variable_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: None, // Response type
                    line: 0,
                    column: 0,
                };

                if let Err(error) = self.current_scope.define(symbol) {
                    self.errors.push(error);
                }
            }
            Statement::HttpPostStatement { variable_name, .. } => {
                let symbol = Symbol {
                    name: variable_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: None, // Response type
                    line: 0,
                    column: 0,
                };

                if let Err(error) = self.current_scope.define(symbol) {
                    self.errors.push(error);
                }
            }
            Statement::HttpRequestStatement {
                url,
                method,
                headers,
                body,
                variable_name,
                ..
            } => {
                self.analyze_expression(url);
                if let Some(method) = method {
                    self.analyze_expression(method);
                }
                if let Some(headers) = headers {
                    self.analyze_expression(headers);
                }
                if let Some(body) = body {
                    self.analyze_expression(body);
                }

                let symbol = Symbol {
                    name: variable_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: None, // Response type
                    line: 0,
                    column: 0,
                };

                if let Err(error) = self.current_scope.define(symbol) {
                    self.errors.push(error);
                }
            }

            Statement::CreateDirectoryStatement { path, .. } => {
                self.analyze_expression(path);
            }

            Statement::CreateFileStatement { path, content, .. } => {
                self.analyze_expression(path);
                self.analyze_expression(content);
            }

            Statement::DeleteFileStatement { path, .. } => {
                self.analyze_expression(path);
            }

            Statement::DeleteDirectoryStatement { path, .. } => {
                self.analyze_expression(path);
            }

            Statement::LoadModuleStatement { path, .. } => {
                self.analyze_expression(path);
            }

            Statement::CloseFileStatement { file, .. } => {
                self.analyze_expression(file);
            }

            Statement::WriteFileStatement { file, content, .. } => {
                self.analyze_expression(file);
                self.analyze_expression(content);
            }

            Statement::WriteToStatement { content, file, .. } => {
                self.analyze_expression(content);
                self.analyze_expression(file);
            }

            Statement::WriteContentStatement {
                content, target, ..
            } => {
                self.analyze_expression(content);
                self.analyze_expression(target);
            }

            Statement::WriteBinaryStatement {
                content, target, ..
            } => {
                self.analyze_expression(content);
                self.analyze_expression(target);
            }

            Statement::ContainerDefinition {
                name,
                extends,
                implements,
                properties,
                methods,
                static_properties,
                static_methods,
                line,
                column,
                ..
            } => {
                // Create container info
                let mut container_info = ContainerInfo {
                    name: name.clone(),
                    extends: extends.clone(),
                    implements: implements.clone(),
                    properties: HashMap::new(),
                    methods: HashMap::new(),
                    static_properties: HashMap::new(),
                    static_methods: HashMap::new(),
                    line: *line,
                    column: *column,
                };

                // Process instance properties
                for prop in properties {
                    let prop_type = prop
                        .property_type
                        .as_ref()
                        .cloned()
                        .unwrap_or(Type::Unknown);

                    let prop_info = PropertyInfo {
                        name: prop.name.clone(),
                        property_type: prop_type,
                        is_public: matches!(
                            prop.visibility,
                            crate::parser::ast::Visibility::Public
                        ),
                        line: prop.line,
                        column: prop.column,
                    };

                    container_info
                        .properties
                        .insert(prop.name.clone(), prop_info);
                }

                // Register the container early so properties are available during method analysis
                self.register_container(container_info.clone());

                // Process static properties
                for prop in static_properties {
                    let prop_type = prop
                        .property_type
                        .as_ref()
                        .cloned()
                        .unwrap_or(Type::Unknown);

                    let prop_info = PropertyInfo {
                        name: prop.name.clone(),
                        property_type: prop_type,
                        is_public: matches!(
                            prop.visibility,
                            crate::parser::ast::Visibility::Public
                        ),
                        line: prop.line,
                        column: prop.column,
                    };

                    container_info
                        .static_properties
                        .insert(prop.name.clone(), prop_info);
                }

                // Process instance methods
                for method in methods {
                    if let Statement::ActionDefinition {
                        name: method_name,
                        parameters,
                        return_type,
                        body,
                        line: method_line,
                        column: method_column,
                        ..
                    } = method
                    {
                        let method_info = MethodInfo {
                            name: method_name.clone(),
                            parameters: parameters.clone(),
                            return_type: return_type.as_ref().cloned().unwrap_or(Type::Nothing),
                            is_public: true, // Default to public for now
                            line: *method_line,
                            column: *method_column,
                        };

                        container_info
                            .methods
                            .insert(method_name.clone(), method_info);

                        // Analyze method body
                        self.push_scope();

                        // Set current container context
                        let previous_container = self.current_container.clone();
                        self.current_container = Some(name.clone());

                        // Properties will be resolved through container context
                        // Don't add them as variables to avoid conflicts with assignments

                        // Add method parameters
                        for param in parameters {
                            let param_type =
                                param.param_type.as_ref().cloned().unwrap_or(Type::Unknown);
                            let symbol = Symbol {
                                name: param.name.clone(),
                                kind: SymbolKind::Variable { mutable: false },
                                symbol_type: Some(param_type),
                                line: param.line,
                                column: param.column,
                            };
                            let _ = self.current_scope.define(symbol);
                        }

                        for stmt in body {
                            self.analyze_statement(stmt);
                        }

                        // Restore previous container context
                        self.current_container = previous_container;
                        self.pop_scope();
                    }
                }

                // Process static methods
                for method in static_methods {
                    if let Statement::ActionDefinition {
                        name: method_name,
                        parameters,
                        return_type,
                        body,
                        line: method_line,
                        column: method_column,
                        ..
                    } = method
                    {
                        let method_info = MethodInfo {
                            name: method_name.clone(),
                            parameters: parameters.clone(),
                            return_type: return_type.as_ref().cloned().unwrap_or(Type::Nothing),
                            is_public: true, // Default to public for now
                            line: *method_line,
                            column: *method_column,
                        };

                        container_info
                            .static_methods
                            .insert(method_name.clone(), method_info);

                        // Analyze static method body
                        self.push_scope();

                        // Set current container context
                        let previous_container = self.current_container.clone();
                        self.current_container = Some(name.clone());

                        // Add static properties as accessible variables (not instance properties)
                        for prop in static_properties {
                            let prop_type = prop
                                .property_type
                                .as_ref()
                                .cloned()
                                .unwrap_or(Type::Unknown);
                            let symbol = Symbol {
                                name: prop.name.clone(),
                                kind: SymbolKind::Variable { mutable: true },
                                symbol_type: Some(prop_type),
                                line: prop.line,
                                column: prop.column,
                            };
                            let _ = self.current_scope.define(symbol);
                        }

                        // Add method parameters
                        for param in parameters {
                            let param_type =
                                param.param_type.as_ref().cloned().unwrap_or(Type::Unknown);
                            let symbol = Symbol {
                                name: param.name.clone(),
                                kind: SymbolKind::Variable { mutable: false },
                                symbol_type: Some(param_type),
                                line: param.line,
                                column: param.column,
                            };
                            let _ = self.current_scope.define(symbol);
                        }

                        for stmt in body {
                            self.analyze_statement(stmt);
                        }

                        // Restore previous container context
                        self.current_container = previous_container;
                        self.pop_scope();
                    }
                }

                // Re-register the container with all methods now that they've been processed
                self.register_container(container_info.clone());

                // Also register as a type symbol
                let container_symbol = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable { mutable: false },
                    symbol_type: Some(Type::Container(name.clone())),
                    line: *line,
                    column: *column,
                };
                let _ = self.current_scope.define(container_symbol);
            }

            Statement::ContainerInstantiation {
                container_type,
                instance_name,
                arguments: _,
                property_initializers: _,
                line,
                column,
            } => {
                // Register the instance as a variable with ContainerInstance type
                let instance_symbol = Symbol {
                    name: instance_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: Some(Type::ContainerInstance(container_type.clone())),
                    line: *line,
                    column: *column,
                };

                if let Err(e) = self.current_scope.define(instance_symbol) {
                    self.errors.push(e);
                }
            }

            Statement::InterfaceDefinition {
                name,
                extends: _,
                required_actions: _,
                line,
                column,
            } => {
                // Register the interface as a type symbol
                let interface_symbol = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable { mutable: false },
                    symbol_type: Some(Type::Interface(name.clone())),
                    line: *line,
                    column: *column,
                };

                if let Err(e) = self.current_scope.define(interface_symbol) {
                    self.errors.push(e);
                }
            }

            Statement::CreateListStatement {
                name,
                initial_values,
                line,
                column,
            } => {
                // Analyze initial values
                for value in initial_values {
                    self.analyze_expression(value);
                }

                // Define the list variable
                let symbol = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: Some(Type::List(Box::new(Type::Unknown))),
                    line: *line,
                    column: *column,
                };
                if let Err(e) = self.current_scope.define(symbol) {
                    self.errors.push(e);
                }
            }

            Statement::MapCreation {
                name,
                entries,
                line,
                column,
            } => {
                // Analyze entry values
                for (_key, value) in entries {
                    self.analyze_expression(value);
                }

                // Define the map variable
                let symbol = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: Some(Type::Map(Box::new(Type::Text), Box::new(Type::Unknown))),
                    line: *line,
                    column: *column,
                };
                if let Err(e) = self.current_scope.define(symbol) {
                    self.errors.push(e);
                }
            }

            Statement::AddToListStatement {
                value,
                list_name,
                line,
                column,
            } => {
                self.analyze_expression(value);
                if self.get_symbol(list_name).is_none() {
                    self.errors.push(SemanticError::new(
                        format!("Variable '{list_name}' is not defined"),
                        *line,
                        *column,
                    ));
                }
            }

            Statement::RemoveFromListStatement {
                value,
                list_name,
                line,
                column,
            } => {
                self.analyze_expression(value);
                if self.get_symbol(list_name).is_none() {
                    self.errors.push(SemanticError::new(
                        format!("Variable '{list_name}' is not defined"),
                        *line,
                        *column,
                    ));
                }
            }

            Statement::ClearListStatement {
                list_name,
                line,
                column,
            } if self.get_symbol(list_name).is_none() => {
                self.errors.push(SemanticError::new(
                    format!("Variable '{list_name}' is not defined"),
                    *line,
                    *column,
                ));
            }
            Statement::ClearListStatement { .. } => {}

            Statement::PatternDefinition {
                name,
                pattern,
                line,
                column,
            } => {
                // Register the pattern as a symbol
                let pattern_symbol = Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Pattern,
                    symbol_type: Some(Type::Pattern),
                    line: *line,
                    column: *column,
                };

                if let Err(e) = self.current_scope.define(pattern_symbol) {
                    self.errors.push(e);
                }

                // Analyze the pattern expression to catch undefined list references
                self.analyze_pattern_expression(pattern, *line, *column);
            }

            Statement::ListenStatement {
                port,
                server_name,
                tls,
                redirect_to_port,
                line,
                column,
            } => {
                // Analyze the port expression
                self.analyze_expression(port);

                // Analyze TLS certificate/key path expressions if present
                if let Some(tls_config) = tls {
                    if let Some(cert_path) = &tls_config.cert_path {
                        self.analyze_expression(cert_path);
                    }
                    if let Some(key_path) = &tls_config.key_path {
                        self.analyze_expression(key_path);
                    }
                }

                // Analyze the redirect target port expression if present
                if let Some(target_port) = redirect_to_port {
                    self.analyze_expression(target_port);
                }

                // Define the server variable
                let server_symbol = Symbol {
                    name: server_name.clone(),
                    kind: SymbolKind::Variable { mutable: false },
                    symbol_type: Some(Type::Text), // Server is represented as text
                    line: *line,
                    column: *column,
                };

                if let Err(error) = self.current_scope.define(server_symbol) {
                    self.errors.push(error);
                }
            }

            Statement::WaitForRequestStatement {
                server,
                request_name,
                timeout: _,
                line,
                column,
            } => {
                // Analyze the server expression
                self.analyze_expression(server);

                // Define the request variable. Waiting for another request may
                // rebind an existing name (e.g. in a loop), so the binding is
                // overwritten/shadowed here — matching the interpreter, which
                // refreshes it via define_or_replace.
                let request_symbol = Symbol {
                    name: request_name.clone(),
                    kind: SymbolKind::Variable { mutable: false },
                    symbol_type: Some(Type::Custom("Request".to_string())), // Request is a custom object type
                    line: *line,
                    column: *column,
                };

                self.current_scope.define_or_replace(request_symbol);

                // Define individual request property variables. These implicit
                // bindings are refreshed on every wait and shadow any outer
                // variables of the same name, matching the interpreter.
                let request_properties = [
                    ("method", Type::Text),
                    ("path", Type::Text),
                    ("query", Type::Text),
                    ("client_ip", Type::Text),
                    ("body", Type::Text),
                    ("body_bytes", Type::Binary),
                    ("headers", Type::Custom("Headers".to_string())),
                ];

                for (prop_name, prop_type) in request_properties.iter() {
                    let prop_symbol = Symbol {
                        name: prop_name.to_string(),
                        kind: SymbolKind::Variable { mutable: false },
                        symbol_type: Some(prop_type.clone()),
                        line: *line,
                        column: *column,
                    };

                    self.current_scope.define_or_replace(prop_symbol);
                }
            }

            Statement::RespondStatement {
                request,
                content,
                status,
                content_type,
                headers,
                ..
            } => {
                // Analyze all expressions
                self.analyze_expression(request);
                self.analyze_expression(content);

                if let Some(status_expr) = status {
                    self.analyze_expression(status_expr);
                }

                if let Some(ct_expr) = content_type {
                    self.analyze_expression(ct_expr);
                }

                if let Some(headers_expr) = headers {
                    self.analyze_expression(headers_expr);
                }
            }

            Statement::ListenWebSocketStatement {
                port,
                server_name,
                line,
                column,
            } => {
                self.analyze_expression(port);

                let server_symbol = Symbol {
                    name: server_name.clone(),
                    kind: SymbolKind::Variable { mutable: false },
                    symbol_type: Some(Type::Text), // Server is represented as text
                    line: *line,
                    column: *column,
                };
                if let Err(error) = self.current_scope.define(server_symbol) {
                    self.errors.push(error);
                }
            }

            Statement::WebSocketHandlerStatement {
                server,
                binding,
                body,
                line,
                column,
                ..
            } => {
                self.analyze_expression(server);

                // The handler body runs in its own scope with the event binding
                // (a connection/message object) defined, mirroring loop-variable
                // scoping so the body can reference it without a false
                // "undefined variable" error.
                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope);

                let binding_symbol = Symbol {
                    name: binding.clone(),
                    kind: SymbolKind::Variable { mutable: false },
                    symbol_type: None,
                    line: *line,
                    column: *column,
                };
                self.current_scope.define_or_replace(binding_symbol);
                // `action_parameters` is a flat set shared by the whole run, so
                // only remove names this handler actually added — otherwise a
                // same-named enclosing action parameter would be dropped for the
                // rest of that action.
                let binding_was_present = !self.action_parameters.insert(binding.clone());

                // The event object's property names are accessed as `id of conn`
                // / `body of msg`, which parse as `of`-form calls. Registering
                // them as action parameters (rather than scope symbols) lets the
                // callee resolve without a "not a function" error, the same
                // relaxation used for action parameters and the `count` variable.
                let ws_properties = ["id", "ip", "body", "sender"];
                let mut newly_added: Vec<&str> = Vec::new();
                for prop in ws_properties {
                    if self.action_parameters.insert(prop.to_string()) {
                        newly_added.push(prop);
                    }
                }

                for stmt in body {
                    self.analyze_statement(stmt);
                }

                for prop in newly_added {
                    self.action_parameters.remove(prop);
                }
                if !binding_was_present {
                    self.action_parameters.remove(binding);
                }

                let handler_scope = std::mem::take(&mut self.current_scope);
                if let Some(parent) = handler_scope.parent {
                    self.current_scope = *parent;
                }
            }

            Statement::SendWebSocketMessageStatement {
                message, target, ..
            } => {
                self.analyze_expression(message);
                self.analyze_expression(target);
            }

            Statement::BroadcastWebSocketMessageStatement {
                message, server, ..
            } => {
                self.analyze_expression(message);
                self.analyze_expression(server);
            }

            Statement::RegisterSignalHandlerStatement {
                handler_name,
                line,
                column,
                ..
            } if self.current_scope.resolve(handler_name).is_none() => {
                // The runtime only records the handler name, so a missing
                // handler is suspicious but not fatal.
                self.warnings.push(SemanticError::new(
                    format!("Undefined signal handler '{handler_name}'"),
                    *line,
                    *column,
                ));
            }
            Statement::RegisterSignalHandlerStatement { .. } => {}

            Statement::ExecuteCommandStatement {
                command,
                arguments,
                variable_name,
                use_shell: _,
                line,
                column,
            } => {
                self.analyze_expression(command);
                if let Some(args) = arguments {
                    self.analyze_expression(args);
                }

                if let Some(var_name) = variable_name {
                    let symbol = Symbol {
                        name: var_name.clone(),
                        kind: SymbolKind::Variable { mutable: true },
                        // Result object with output, error, exit_code, success
                        symbol_type: Some(Type::Map(Box::new(Type::Text), Box::new(Type::Any))),
                        line: *line,
                        column: *column,
                    };

                    if let Err(error) = self.current_scope.define(symbol) {
                        self.errors.push(error);
                    }
                }
            }

            Statement::ExecuteFileStatement {
                path,
                request,
                variable_name,
                line,
                column,
            } => {
                self.analyze_expression(path);
                if let Some(request_expr) = request {
                    self.analyze_expression(request_expr);
                }

                if let Some(var_name) = variable_name {
                    let symbol = Symbol {
                        name: var_name.clone(),
                        kind: SymbolKind::Variable { mutable: true },
                        // Captured display output of the executed file
                        symbol_type: Some(Type::Text),
                        line: *line,
                        column: *column,
                    };

                    if let Err(error) = self.current_scope.define(symbol) {
                        self.errors.push(error);
                    }
                }
            }

            Statement::SpawnProcessStatement {
                command,
                arguments,
                variable_name,
                use_shell: _,
                line,
                column,
            } => {
                self.analyze_expression(command);
                if let Some(args) = arguments {
                    self.analyze_expression(args);
                }

                let symbol = Symbol {
                    name: variable_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: Some(Type::Text), // Process ID is a string (e.g. "proc1")
                    line: *line,
                    column: *column,
                };

                if let Err(error) = self.current_scope.define(symbol) {
                    self.errors.push(error);
                }
            }

            Statement::ReadProcessOutputStatement {
                process_id,
                variable_name,
                line,
                column,
            } => {
                self.analyze_expression(process_id);

                let symbol = Symbol {
                    name: variable_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: Some(Type::Text),
                    line: *line,
                    column: *column,
                };

                if let Err(error) = self.current_scope.define(symbol) {
                    self.errors.push(error);
                }
            }

            Statement::KillProcessStatement { process_id, .. } => {
                self.analyze_expression(process_id);
            }

            Statement::WaitForProcessStatement {
                process_id,
                variable_name,
                line,
                column,
            } => {
                self.analyze_expression(process_id);

                if let Some(var_name) = variable_name {
                    let symbol = Symbol {
                        name: var_name.clone(),
                        kind: SymbolKind::Variable { mutable: true },
                        symbol_type: Some(Type::Number), // Exit code
                        line: *line,
                        column: *column,
                    };

                    if let Err(error) = self.current_scope.define(symbol) {
                        self.errors.push(error);
                    }
                }
            }

            _ => {}
        }
    }

    fn register_action_signature(&mut self, statement: &Statement) {
        if let Statement::ActionDefinition {
            name,
            parameters,
            return_type,
            line,
            column,
            ..
        } = statement
        {
            let symbol = Symbol {
                name: name.clone(),
                kind: SymbolKind::Function {
                    signatures: vec![FunctionSignature {
                        parameters: parameters.clone(),
                        return_type: return_type.clone(),
                    }],
                },
                symbol_type: None,
                line: *line,
                column: *column,
            };

            if let Err(error) = self.current_scope.define(symbol) {
                self.errors.push(error);
            }
        }
    }

    fn analyze_pattern_expression(
        &mut self,
        pattern: &crate::parser::ast::PatternExpression,
        line: usize,
        column: usize,
    ) {
        use crate::parser::ast::PatternExpression;
        match pattern {
            PatternExpression::Literal(_)
            | PatternExpression::CharacterClass(_)
            | PatternExpression::Anchor(_)
            | PatternExpression::Backreference(_) => {}
            PatternExpression::Quantified { pattern: inner, .. } => {
                self.analyze_pattern_expression(inner, line, column);
            }
            PatternExpression::Sequence(patterns) | PatternExpression::Alternative(patterns) => {
                for inner in patterns {
                    self.analyze_pattern_expression(inner, line, column);
                }
            }
            PatternExpression::Capture { pattern: inner, .. } => {
                self.analyze_pattern_expression(inner, line, column);
            }
            PatternExpression::Lookahead(inner)
            | PatternExpression::NegativeLookahead(inner)
            | PatternExpression::Lookbehind(inner)
            | PatternExpression::NegativeLookbehind(inner) => {
                self.analyze_pattern_expression(inner, line, column);
            }
            PatternExpression::ListReference(name) => {
                // Same check as Expression::Variable to ensure it exists
                if self.current_scope.resolve(name).is_none()
                    && !self.action_parameters.contains(name)
                {
                    self.errors.push(SemanticError {
                        message: format!("Undefined list reference '{name}' in pattern"),
                        line,
                        column,
                    });
                }
            }
        }
    }

    fn analyze_action_body(&mut self, statement: &Statement) {
        if let Statement::ActionDefinition {
            parameters, body, ..
        } = statement
        {
            // Create new scope for action body
            let outer_scope = std::mem::take(&mut self.current_scope);
            self.current_scope = Scope::with_parent(outer_scope);

            // Collect parameter names to remove them later
            let mut param_names_to_remove = Vec::new();

            // Register parameters in action scope
            for param in parameters {
                for part in param.name.split_whitespace() {
                    let part_string = part.to_string();
                    self.action_parameters.insert(part_string.clone());
                    param_names_to_remove.push(part_string);
                }

                let param_symbol = Symbol {
                    name: param.name.clone(),
                    kind: SymbolKind::Variable { mutable: false },
                    symbol_type: param.param_type.clone(),
                    line: param.line,
                    column: param.column,
                };

                if let Err(error) = self.current_scope.define(param_symbol) {
                    self.errors.push(error);
                }
            }

            // Analyze body statements
            for stmt in body {
                self.analyze_statement(stmt);
            }

            // Clean up: remove parameter names from action_parameters
            for param_name in param_names_to_remove {
                self.action_parameters.remove(&param_name);
            }

            // Restore outer scope
            let function_scope = std::mem::take(&mut self.current_scope);
            if let Some(parent) = function_scope.parent {
                self.current_scope = *parent;
            }
        }
    }

    pub fn get_symbol(&self, name: &str) -> Option<&Symbol> {
        self.current_scope.resolve(name)
    }

    pub fn get_symbol_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        self.current_scope.symbols.get_mut(name)
    }

    pub fn define_symbol(&mut self, symbol: Symbol) -> Result<(), SemanticError> {
        self.current_scope.define(symbol)
    }

    pub fn register_builtin_function(
        &mut self,
        name: &str,
        param_types: Vec<Type>,
        return_type: Type,
    ) {
        let parameters = param_types
            .iter()
            .enumerate()
            .map(|(i, t)| Parameter {
                name: format!("param{i}"),
                param_type: Some(t.clone()),
                default_value: None,
                line: 0,
                column: 0,
            })
            .collect();

        let new_signature = FunctionSignature {
            parameters,
            return_type: Some(return_type.clone()),
        };

        // Check if function already exists
        if let Some(existing_symbol) = self.current_scope.symbols.get_mut(name) {
            // If it's a function, add the new signature
            if let SymbolKind::Function { signatures } = &mut existing_symbol.kind {
                signatures.push(new_signature);
                return;
            } else {
                // Not a function - this is an error, but for now we'll ignore it like before
                return;
            }
        }

        // Function doesn't exist, create new
        let symbol = Symbol {
            name: name.to_string(),
            kind: SymbolKind::Function {
                signatures: vec![new_signature],
            },
            symbol_type: Some(Type::Function {
                parameters: param_types,
                return_type: Box::new(return_type),
            }),
            line: 0,
            column: 0,
        };

        let _ = self.current_scope.define(symbol);
    }

    pub fn register_container(&mut self, container: ContainerInfo) {
        self.containers.insert(container.name.clone(), container);
    }

    fn is_container_property(&self, container_name: &str, property_name: &str) -> bool {
        if let Some(container_info) = self.containers.get(container_name) {
            // Check direct instance properties
            if container_info.properties.contains_key(property_name) {
                return true;
            }

            // Check direct static properties
            if container_info.static_properties.contains_key(property_name) {
                return true;
            }

            // Check inherited properties (both instance and static)
            if let Some(parent_name) = &container_info.extends {
                return self.is_container_property(parent_name, property_name);
            }
        }
        false
    }

    pub fn get_container(&self, name: &str) -> Option<&ContainerInfo> {
        self.containers.get(name)
    }

    pub fn get_containers(&self) -> &HashMap<String, ContainerInfo> {
        &self.containers
    }

    pub fn push_scope(&mut self) {
        let new_scope = Scope::with_parent(self.current_scope.clone());
        self.current_scope = new_scope;
    }

    pub fn pop_scope(&mut self) {
        if let Some(parent) = self.current_scope.parent.take() {
            self.current_scope = *parent;
        }
    }

    fn infer_expression_type(&self, expression: &Expression) -> Type {
        match expression {
            Expression::Literal(Literal::Integer(_), _, _) => Type::Number,
            Expression::Literal(Literal::Float(_), _, _) => Type::Number,
            Expression::Literal(Literal::String(_), _, _) => Type::Text,
            Expression::Literal(Literal::Boolean(_), _, _) => Type::Boolean,
            Expression::Literal(Literal::Nothing, _, _) => Type::Nothing,
            Expression::Literal(Literal::Pattern(_), _, _) => Type::Pattern,
            Expression::Literal(Literal::List(_), _, _) => Type::List(Box::new(Type::Unknown)),
            Expression::Variable(name, _, _) => {
                if let Some(symbol) = self.current_scope.resolve(name) {
                    symbol
                        .symbol_type
                        .as_ref()
                        .cloned()
                        .unwrap_or(Type::Unknown)
                } else {
                    Type::Unknown
                }
            }
            Expression::BinaryOperation { operator, .. } => {
                // Determine the type based on the operator
                match operator {
                    crate::parser::ast::Operator::Plus
                    | crate::parser::ast::Operator::Minus
                    | crate::parser::ast::Operator::Multiply
                    | crate::parser::ast::Operator::Divide
                    | crate::parser::ast::Operator::Modulo => Type::Number,
                    crate::parser::ast::Operator::Equals
                    | crate::parser::ast::Operator::NotEquals
                    | crate::parser::ast::Operator::GreaterThan
                    | crate::parser::ast::Operator::LessThan
                    | crate::parser::ast::Operator::GreaterThanOrEqual
                    | crate::parser::ast::Operator::LessThanOrEqual
                    | crate::parser::ast::Operator::And
                    | crate::parser::ast::Operator::Or
                    | crate::parser::ast::Operator::Contains => Type::Boolean,
                }
            }
            Expression::UnaryOperation { operator, .. } => match operator {
                crate::parser::ast::UnaryOperator::Not => Type::Boolean,
                crate::parser::ast::UnaryOperator::Minus => Type::Number,
            },
            Expression::Concatenation { .. } => Type::Text,
            Expression::FunctionCall { function, .. } => {
                if let Expression::Variable(name, _, _) = &**function
                    && let Some(symbol) = self.current_scope.resolve(name)
                    && let SymbolKind::Function { signatures } = &symbol.kind
                    && let Some(sig) = signatures.first()
                {
                    return sig.return_type.as_ref().cloned().unwrap_or(Type::Unknown);
                }
                Type::Unknown
            }
            Expression::ActionCall { name, .. } => {
                if let Some(symbol) = self.current_scope.resolve(name)
                    && let SymbolKind::Function { signatures } = &symbol.kind
                    && let Some(sig) = signatures.first()
                {
                    return sig.return_type.as_ref().cloned().unwrap_or(Type::Unknown);
                }
                Type::Unknown
            }
            _ => Type::Unknown,
        }
    }

    fn is_type_compatible(&self, actual: &Type, expected: &Type) -> bool {
        if actual == expected {
            return true;
        }

        match (expected, actual) {
            (_, Type::Unknown) => true,
            (Type::Unknown, _) => true,

            (Type::Any, _) => true,
            (_, Type::Any) => true,

            (_, Type::Nothing) => true,
            (_, Type::Error) => true,

            (inner, Type::Async(async_type)) => self.is_type_compatible(async_type, inner),

            (Type::List(expected_inner), Type::List(actual_inner)) => {
                self.is_type_compatible(actual_inner, expected_inner)
            }

            (Type::Map(expected_key, expected_val), Type::Map(actual_key, actual_val)) => {
                self.is_type_compatible(actual_key, expected_key)
                    && self.is_type_compatible(actual_val, expected_val)
            }

            (
                Type::Function {
                    parameters: expected_params,
                    return_type: expected_ret,
                },
                Type::Function {
                    parameters: actual_params,
                    return_type: actual_ret,
                },
            ) => {
                if expected_params.len() != actual_params.len() {
                    return false;
                }

                for (e, a) in expected_params.iter().zip(actual_params.iter()) {
                    if !self.is_type_compatible(a, e) {
                        return false;
                    }
                }

                self.is_type_compatible(actual_ret, expected_ret)
            }

            // Case-insensitive matches due to parser mapping primitive types to Custom(...) in some cases
            (Type::Text, Type::Custom(name)) | (Type::Custom(name), Type::Text)
                if name.eq_ignore_ascii_case("text") =>
            {
                true
            }
            (Type::Number, Type::Custom(name)) | (Type::Custom(name), Type::Number)
                if name.eq_ignore_ascii_case("number") =>
            {
                true
            }
            (Type::Boolean, Type::Custom(name)) | (Type::Custom(name), Type::Boolean)
                if name.eq_ignore_ascii_case("boolean") =>
            {
                true
            }
            (Type::Pattern, Type::Custom(name)) | (Type::Custom(name), Type::Pattern)
                if name.eq_ignore_ascii_case("pattern") =>
            {
                true
            }

            // Allow implicitly resolving custom types
            (Type::Custom(expected_name), Type::Custom(actual_name)) => {
                expected_name == actual_name
            }

            _ => false,
        }
    }

    fn format_type_for_display(t: &Type) -> String {
        match t {
            Type::Text => "Text".to_string(),
            Type::Number => "Number".to_string(),
            Type::Boolean => "Boolean".to_string(),
            Type::Pattern => "Pattern".to_string(),
            Type::Nothing => "Nothing".to_string(),
            Type::Any => "Any".to_string(),
            Type::Unknown => "Unknown".to_string(),
            Type::Custom(name) if name.eq_ignore_ascii_case("text") => "Text".to_string(),
            Type::Custom(name) if name.eq_ignore_ascii_case("number") => "Number".to_string(),
            Type::Custom(name) if name.eq_ignore_ascii_case("boolean") => "Boolean".to_string(),
            Type::Custom(name) if name.eq_ignore_ascii_case("pattern") => "Pattern".to_string(),
            Type::Custom(name) if name.eq_ignore_ascii_case("nothing") => "Nothing".to_string(),
            Type::Custom(name) => name.clone(),
            Type::List(inner) => format!("List of {}", Self::format_type_for_display(inner)),
            Type::Map(k, v) => format!(
                "Map of {} to {}",
                Self::format_type_for_display(k),
                Self::format_type_for_display(v)
            ),
            Type::Function {
                parameters,
                return_type,
            } => {
                let params = parameters
                    .iter()
                    .map(Self::format_type_for_display)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "Function({}) -> {}",
                    params,
                    Self::format_type_for_display(return_type)
                )
            }
            Type::Async(inner) => format!("Async {}", Self::format_type_for_display(inner)),
            _ => format!("{:?}", t).replace("Type::", ""),
        }
    }

    fn analyze_expression(&mut self, expression: &Expression) {
        match expression {
            Expression::AwaitExpression {
                expression,
                line: _,
                column: _,
            } => {
                self.analyze_expression(expression);
            }
            Expression::Variable(name, line, column) => {
                if name == "faulty log_message" {
                    return;
                }

                // Check if this is an action parameter before reporting it as undefined
                if self.action_parameters.contains(name) {
                    // It's an action parameter, so don't report an error
                    return;
                }

                // Special case for 'count' variable in count loops
                if name == "count" {
                    return;
                }

                // Check if it's a builtin function
                if Self::is_builtin_function(name) {
                    return;
                }

                if self.current_scope.resolve(name).is_none() {
                    // Check if it's a container property (including inherited)
                    let is_container_property =
                        if let Some(container_name) = &self.current_container {
                            self.is_container_property(container_name, name)
                        } else {
                            false
                        };

                    if !is_container_property {
                        self.report_undefined_name(
                            format!("Variable '{name}' is not defined"),
                            *line,
                            *column,
                        );
                    }
                }
            }
            Expression::FunctionCall {
                function,
                arguments,
                line,
                column,
            } => {
                // A bare-`Variable` callee is the idiomatic `of` call form
                // (e.g. `greet of "bob"`, parsed as FunctionCall { function:
                // Variable("greet"), .. }). It is resolved explicitly in the
                // block below — including the include-aware relaxation — so
                // analyzing it here would recurse into the Variable arm and
                // report it as a *fatal* undefined variable before the block
                // can relax it (issue #580). Only analyze the callee directly
                // when it is a more complex expression.
                if !matches!(&**function, Expression::Variable(_, _, _)) {
                    self.analyze_expression(function);
                }

                if let Expression::Variable(name, _, _) = &**function {
                    if let Some(symbol) = self.current_scope.resolve(name) {
                        match &symbol.kind {
                            SymbolKind::Function { signatures } => {
                                // For now, just check the first signature for compatibility
                                // TODO: Implement proper overload resolution based on argument types and count
                                if let Some(first_signature) = signatures.first()
                                    && arguments.len() != first_signature.parameters.len()
                                {
                                    // Check if any signature matches the argument count
                                    let matching_signature = signatures
                                        .iter()
                                        .find(|sig| sig.parameters.len() == arguments.len());
                                    if matching_signature.is_none() {
                                        let expected_arities: Vec<String> = signatures
                                            .iter()
                                            .map(|sig| sig.parameters.len().to_string())
                                            .collect();
                                        self.errors.push(SemanticError::new(
                                            format!("Function '{}' expects {} arguments, but {} were provided",
                                                name, expected_arities.join(" or "), arguments.len()),
                                            *line,
                                            *column,
                                        ));
                                    }
                                }

                                for arg in arguments {
                                    self.analyze_expression(&arg.value);
                                }
                            }
                            _ => {
                                // The symbol resolved to something that is not a
                                // function. Built-in stdlib functions (touppercase,
                                // wflhash256, parse_json, ...) get injected into an
                                // included file's scope as plain variables
                                // (parent-scope bindings, defined at position 0:0),
                                // so those remain callable. A user who genuinely
                                // shadows a builtin name with a non-function value
                                // (e.g. `store touppercase as "x"`) has a real
                                // source position, so that still reports
                                // "is not a function". Real builtin Function
                                // symbols take the arm above (arity preserved).
                                let is_injected_builtin = Self::is_builtin_function(name)
                                    && symbol.line == 0
                                    && symbol.column == 0;
                                // An action parameter or handler-property name
                                // (e.g. `body of msg` inside a websocket handler)
                                // is a relaxed `of`-form access even when an outer
                                // scope also defines a non-function symbol of the
                                // same name; the property read must win over it.
                                if is_injected_builtin || self.action_parameters.contains(name) {
                                    for arg in arguments {
                                        self.analyze_expression(&arg.value);
                                    }
                                } else {
                                    self.errors.push(SemanticError::new(
                                        format!("'{name}' is not a function"),
                                        *line,
                                        *column,
                                    ));
                                }
                            }
                        }
                    } else if Self::is_builtin_function(name)
                        || self.action_parameters.contains(name)
                        || name == "count"
                    {
                        // A known builtin, an action parameter, or the count
                        // loop variable isn't present as a scope symbol but is
                        // still callable (its arguments are still analyzed).
                        for arg in arguments {
                            self.analyze_expression(&arg.value);
                        }
                    } else {
                        // Callee not in scope and not a builtin. Under `include
                        // from` this is the `of`-form counterpart of the
                        // ActionCall relaxation (issues #580 / #548): a non-fatal
                        // warning, since the callee may be include-exposed at
                        // runtime. Otherwise preserve the pre-existing fatal
                        // behavior (and the try_depth > 0 warning downgrade) that
                        // the unconditional `analyze_expression(function)` above
                        // used to produce for a bare-Variable callee.
                        if !self.warn_undefined_callee_if_includes(name, *line, *column) {
                            self.report_undefined_name(
                                format!("Variable '{name}' is not defined"),
                                *line,
                                *column,
                            );
                        }
                        for arg in arguments {
                            self.analyze_expression(&arg.value);
                        }
                    }
                } else {
                    for arg in arguments {
                        self.analyze_expression(&arg.value);
                    }
                }
            }
            Expression::BinaryOperation {
                left,
                operator: _,
                right,
                line: _,
                column: _,
            } => {
                self.analyze_expression(left);
                self.analyze_expression(right);
            }
            Expression::UnaryOperation {
                operator: _,
                expression,
                line: _,
                column: _,
            } => {
                self.analyze_expression(expression);
            }
            Expression::MemberAccess {
                object,
                property: _,
                line: _,
                column: _,
            } => {
                self.analyze_expression(object);
            }
            Expression::IndexAccess {
                collection,
                index,
                line: _,
                column: _,
            } => {
                self.analyze_expression(collection);
                self.analyze_expression(index);
            }
            Expression::Concatenation {
                left,
                right,
                line: _,
                column: _,
            } => {
                self.analyze_expression(left);
                self.analyze_expression(right);
            }
            Expression::PatternMatch { text, pattern, .. } => {
                self.analyze_expression(text);
                self.analyze_expression(pattern);
            }
            Expression::PatternFind { text, pattern, .. } => {
                self.analyze_expression(text);
                self.analyze_expression(pattern);
            }
            Expression::PatternReplace {
                text,
                pattern,
                replacement,
                ..
            } => {
                self.analyze_expression(text);
                self.analyze_expression(pattern);
                self.analyze_expression(replacement);
            }
            Expression::PatternSplit { text, pattern, .. } => {
                self.analyze_expression(text);
                self.analyze_expression(pattern);
            }
            Expression::StringSplit {
                text, delimiter, ..
            } => {
                self.analyze_expression(text);
                self.analyze_expression(delimiter);
            }
            Expression::ActionCall {
                name,
                arguments,
                line,
                column,
            } => {
                // Analyze argument expressions first
                for arg in arguments {
                    self.analyze_expression(&arg.value);
                }

                // Skip validation for builtin functions - they have their own validation
                if Self::is_builtin_function(name) {
                    return;
                }

                // Validate user-defined action exists and has correct signature
                if let Some(symbol) = self.current_scope.resolve(name) {
                    match &symbol.kind {
                        SymbolKind::Function { signatures } => {
                            // Get first signature (actions have single signature)
                            if let Some(first_signature) = signatures.first() {
                                // Validate argument count
                                if arguments.len() != first_signature.parameters.len() {
                                    self.errors.push(SemanticError::new(
                                        format!(
                                            "Action '{}' expects {} argument(s), but {} were provided",
                                            name,
                                            first_signature.parameters.len(),
                                            arguments.len()
                                        ),
                                        *line,
                                        *column,
                                    ));

                                    // Skip type validation if arity is mismatched to avoid noisy/cascading errors
                                    return;
                                }

                                // Map arguments to parameters for type validation
                                let mut matched_args: Vec<Option<&Expression>> =
                                    vec![None; first_signature.parameters.len()];
                                let mut has_mapping_error = false;

                                for (arg_idx, arg) in arguments.iter().enumerate() {
                                    let mut param_idx_opt = None;

                                    if let Some(arg_name) = &arg.name {
                                        // Named argument
                                        if let Some(idx) = first_signature
                                            .parameters
                                            .iter()
                                            .position(|p| p.name == *arg_name)
                                        {
                                            param_idx_opt = Some(idx);
                                        } else {
                                            self.errors.push(SemanticError::new(
                                                format!(
                                                    "Unknown parameter '{}' for action '{}'",
                                                    arg_name, name
                                                ),
                                                *line,
                                                *column,
                                            ));
                                            has_mapping_error = true;
                                        }
                                    } else {
                                        // Positional argument
                                        if arg_idx < first_signature.parameters.len() {
                                            param_idx_opt = Some(arg_idx);
                                        }
                                    }

                                    if let Some(param_idx) = param_idx_opt {
                                        if matched_args[param_idx].is_some() {
                                            let param_name =
                                                &first_signature.parameters[param_idx].name;
                                            self.errors.push(SemanticError::new(
                                                format!(
                                                    "Duplicate argument for parameter '{}' in action '{}'",
                                                    param_name, name
                                                ),
                                                *line,
                                                *column,
                                            ));
                                            has_mapping_error = true;
                                        } else {
                                            matched_args[param_idx] = Some(&arg.value);
                                        }
                                    }
                                }

                                // Skip type validation if argument mapping failed
                                if has_mapping_error {
                                    return;
                                }

                                // Type validation
                                for (param, arg_opt) in
                                    first_signature.parameters.iter().zip(matched_args.iter())
                                {
                                    if let Some(arg_val) = arg_opt
                                        && let Some(expected_type) = &param.param_type
                                    {
                                        let arg_type = self.infer_expression_type(arg_val);

                                        if arg_type != Type::Unknown
                                            && expected_type != &Type::Unknown
                                            && expected_type != &Type::Any
                                            && !self.is_type_compatible(&arg_type, expected_type)
                                        {
                                            let expected_display =
                                                Self::format_type_for_display(expected_type);
                                            let actual_display =
                                                Self::format_type_for_display(&arg_type);
                                            self.errors.push(SemanticError::new(
                                                format!(
                                                    "Argument '{}' of action '{}' expects {}, but got {}",
                                                    param.name,
                                                    name,
                                                    expected_display,
                                                    actual_display
                                                ),
                                                *line,
                                                *column,
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                        _ => {
                            // Symbol exists but is not a function/action
                            self.errors.push(SemanticError::new(
                                format!("'{}' is not an action", name),
                                *line,
                                *column,
                            ));
                        }
                    }
                } else if !self.warn_undefined_callee_if_includes(name, *line, *column) {
                    // Action not found in scope and not a builtin, and the program
                    // does not use `include from`, so it cannot be include-exposed
                    // at runtime — a genuine fatal error (see issues #548 / #580
                    // for the include-aware relaxation applied above).
                    self.errors.push(SemanticError::new(
                        format!("Undefined action '{}'", name),
                        *line,
                        *column,
                    ));
                }
            }
            Expression::Literal(_, _, _) => {}
            // Container-related expressions
            Expression::StaticMemberAccess {
                container: _container,
                member: _member,
                ..
            } => {
                // For now, just a stub implementation
                // This will be expanded later
            }
            Expression::MethodCall {
                object,
                method: _method,
                arguments,
                ..
            } => {
                // Analyze the object expression
                self.analyze_expression(object);

                // Analyze the arguments
                for arg in arguments {
                    self.analyze_expression(&arg.value);
                }
            }
            Expression::PropertyAccess { object, .. } => {
                self.analyze_expression(object);
            }
            Expression::FileExists { path, .. } => {
                self.analyze_expression(path);
            }
            Expression::DirectoryExists { path, .. } => {
                self.analyze_expression(path);
            }
            Expression::ListFiles { path, .. } => {
                self.analyze_expression(path);
            }
            Expression::ReadContent { file_handle, .. } => {
                self.analyze_expression(file_handle);
            }
            Expression::ReadBinaryContent { file_handle, .. } => {
                self.analyze_expression(file_handle);
            }
            Expression::ReadBinaryN {
                file_handle, count, ..
            } => {
                self.analyze_expression(file_handle);
                self.analyze_expression(count);
            }
            Expression::FileSizeOf { file_handle, .. } => {
                self.analyze_expression(file_handle);
            }
            Expression::ListFilesRecursive {
                path, extensions, ..
            } => {
                self.analyze_expression(path);
                if let Some(exts) = extensions {
                    for ext in exts {
                        self.analyze_expression(ext);
                    }
                }
            }
            Expression::ListFilesFiltered {
                path, extensions, ..
            } => {
                self.analyze_expression(path);
                for ext in extensions {
                    self.analyze_expression(ext);
                }
            }
            Expression::HeaderAccess {
                header_name: _header_name,
                request,
                line: _line,
                column: _column,
            } => {
                self.analyze_expression(request);
            }
            Expression::CurrentTimeMilliseconds { line: _, column: _ } => {
                // No sub-expressions to analyze
            }
            Expression::CurrentTimeFormatted {
                format: _,
                line: _,
                column: _,
            } => {
                // No sub-expressions to analyze
            }
            Expression::ProcessRunning {
                process_id,
                line: _,
                column: _,
            } => {
                self.analyze_expression(process_id);
            }
            Expression::DatabaseQuery {
                db,
                sql,
                parameters,
                ..
            } => {
                self.analyze_expression(db);
                self.analyze_expression(sql);
                if let Some(params) = parameters {
                    self.analyze_expression(params);
                }
            }
        }
    }
}

pub mod static_analyzer;
pub use static_analyzer::StaticAnalyzer;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Argument, Expression, Literal, Parameter, Program, Statement, Type};
    use std::sync::Arc;

    #[test]
    fn test_variable_declaration_and_usage() {
        let program = Program {
            statements: vec![
                Statement::VariableDeclaration {
                    name: "x".to_string(),
                    value: Expression::Literal(Literal::Integer(10), 1, 1),
                    is_constant: false,
                    line: 1,
                    column: 1,
                },
                Statement::DisplayStatement {
                    value: Expression::Variable("x".to_string(), 2, 9),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut analyzer = Analyzer::new();
        let result = analyzer.analyze(&program);
        assert!(result.is_ok(), "Expected no semantic errors");
    }

    #[test]
    fn test_undefined_variable() {
        let program = Program {
            statements: vec![Statement::DisplayStatement {
                value: Expression::Variable("x".to_string(), 1, 9),
                line: 1,
                column: 1,
            }],
        };

        let mut analyzer = Analyzer::new();
        let result = analyzer.analyze(&program);
        assert!(
            result.is_err(),
            "Expected semantic error for undefined variable"
        );

        let errors = analyzer.get_errors();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("not defined"));
    }

    #[test]
    fn test_function_definition_and_call() {
        let program = Program {
            statements: vec![
                Statement::ActionDefinition {
                    name: "greet".to_string(),
                    parameters: vec![Parameter {
                        name: "name".to_string(),
                        param_type: Some(Type::Text),
                        default_value: None,
                        line: 0,
                        column: 0,
                    }],
                    body: vec![Statement::DisplayStatement {
                        value: Expression::Variable("name".to_string(), 2, 5),
                        line: 2,
                        column: 5,
                    }],
                    return_type: None,
                    line: 1,
                    column: 1,
                },
                Statement::ExpressionStatement {
                    expression: Expression::FunctionCall {
                        function: Box::new(Expression::Variable("greet".to_string(), 3, 1)),
                        arguments: vec![Argument {
                            name: None,
                            value: Expression::Literal(Literal::String(Arc::from("Alice")), 3, 7),
                        }],
                        line: 3,
                        column: 1,
                    },
                    line: 3,
                    column: 1,
                },
            ],
        };

        let mut analyzer = Analyzer::new();
        let result = analyzer.analyze(&program);
        assert!(result.is_ok(), "Expected no semantic errors");
    }

    #[test]
    fn test_function_call_wrong_args() {
        let program = Program {
            statements: vec![
                Statement::ActionDefinition {
                    name: "greet".to_string(),
                    parameters: vec![Parameter {
                        name: "name".to_string(),
                        param_type: Some(Type::Text),
                        default_value: None,
                        line: 0,
                        column: 0,
                    }],
                    body: vec![],
                    return_type: None,
                    line: 1,
                    column: 1,
                },
                Statement::ExpressionStatement {
                    expression: Expression::FunctionCall {
                        function: Box::new(Expression::Variable("greet".to_string(), 2, 1)),
                        arguments: vec![], // No arguments provided
                        line: 2,
                        column: 1,
                    },
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut analyzer = Analyzer::new();
        let result = analyzer.analyze(&program);
        assert!(
            result.is_err(),
            "Expected semantic error for wrong number of arguments"
        );

        let errors = analyzer.get_errors();
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0]
                .message
                .contains("expects 1 arguments, but 0 were provided")
        );
    }

    #[test]
    fn test_container_static_property_recognition() {
        use std::collections::HashMap;

        let mut analyzer = Analyzer::new();

        // Register a container with static properties
        let mut properties = HashMap::new();
        properties.insert(
            "name".to_string(),
            PropertyInfo {
                name: "name".to_string(),
                property_type: Type::Text,
                is_public: true,
                line: 1,
                column: 1,
            },
        );

        let mut static_properties = HashMap::new();
        static_properties.insert(
            "total_count".to_string(),
            PropertyInfo {
                name: "total_count".to_string(),
                property_type: Type::Number,
                is_public: true,
                line: 1,
                column: 1,
            },
        );

        let container_info = ContainerInfo {
            name: "Counter".to_string(),
            properties,
            static_properties,
            methods: HashMap::new(),
            static_methods: HashMap::new(),
            extends: None,
            implements: Vec::new(),
            line: 1,
            column: 1,
        };

        analyzer.register_container(container_info);

        // Test instance property recognition
        assert!(analyzer.is_container_property("Counter", "name"));
        // Test static property recognition
        assert!(analyzer.is_container_property("Counter", "total_count"));
        // Test non-existent property
        assert!(!analyzer.is_container_property("Counter", "nonexistent"));
    }

    #[test]
    fn test_container_inherited_static_property_recognition() {
        use std::collections::HashMap;

        let mut analyzer = Analyzer::new();

        // Register base container with static properties
        let mut base_static_properties = HashMap::new();
        base_static_properties.insert(
            "base_count".to_string(),
            PropertyInfo {
                name: "base_count".to_string(),
                property_type: Type::Number,
                is_public: true,
                line: 1,
                column: 1,
            },
        );

        let base_container = ContainerInfo {
            name: "BaseContainer".to_string(),
            properties: HashMap::new(),
            static_properties: base_static_properties,
            methods: HashMap::new(),
            static_methods: HashMap::new(),
            extends: None,
            implements: Vec::new(),
            line: 1,
            column: 1,
        };

        analyzer.register_container(base_container);

        // Register derived container with its own static properties
        let mut derived_static_properties = HashMap::new();
        derived_static_properties.insert(
            "derived_count".to_string(),
            PropertyInfo {
                name: "derived_count".to_string(),
                property_type: Type::Number,
                is_public: true,
                line: 1,
                column: 1,
            },
        );

        let derived_container = ContainerInfo {
            name: "DerivedContainer".to_string(),
            properties: HashMap::new(),
            static_properties: derived_static_properties,
            methods: HashMap::new(),
            static_methods: HashMap::new(),
            extends: Some("BaseContainer".to_string()),
            implements: Vec::new(),
            line: 1,
            column: 1,
        };

        analyzer.register_container(derived_container);

        // Test derived container can access its own static properties
        assert!(analyzer.is_container_property("DerivedContainer", "derived_count"));
        // Test derived container can access inherited static properties
        assert!(analyzer.is_container_property("DerivedContainer", "base_count"));
        // Test base container cannot access derived properties
        assert!(!analyzer.is_container_property("BaseContainer", "derived_count"));
    }

    // ===== Phase 4: Tests for action call validation =====

    #[test]
    fn test_undefined_action_call() {
        let input = r#"
define action called greet with parameters name:
    print with name
end action

call unknownAction with "test"
        "#;
        let tokens = crate::lexer::lex_wfl_with_positions(input);
        let program = crate::parser::Parser::new(&tokens).parse().unwrap();

        let mut analyzer = Analyzer::new();
        let _ = analyzer.analyze(&program);

        // Should have error for undefined action
        assert!(!analyzer.errors.is_empty(), "Should have semantic errors");
        assert!(
            analyzer
                .errors
                .iter()
                .any(|e| e.message.contains("Undefined action 'unknownAction'")),
            "Should report undefined action 'unknownAction', got: {:?}",
            analyzer.errors
        );
    }

    #[test]
    fn test_action_call_wrong_arg_count() {
        let input = r#"
define action called greet with parameters name:
    print with name
end action

call greet with "Alice" and "Bob"
        "#;
        let tokens = crate::lexer::lex_wfl_with_positions(input);
        let program = crate::parser::Parser::new(&tokens).parse().unwrap();

        let mut analyzer = Analyzer::new();
        let _ = analyzer.analyze(&program);

        // Should have error for wrong argument count
        assert!(!analyzer.errors.is_empty(), "Should have semantic errors");
        assert!(
            analyzer.errors.iter().any(|e| e
                .message
                .contains("expects 1 argument(s), but 2 were provided")),
            "Should report wrong argument count, got: {:?}",
            analyzer.errors
        );
    }

    #[test]
    fn test_valid_action_call() {
        let input = r#"
define action called greet with parameters name:
    print with name
end action

call greet with "Alice"
        "#;
        let tokens = crate::lexer::lex_wfl_with_positions(input);
        let program = crate::parser::Parser::new(&tokens).parse().unwrap();

        let mut analyzer = Analyzer::new();
        let _ = analyzer.analyze(&program);

        // Should have no errors
        assert!(
            analyzer.errors.is_empty(),
            "Should have no semantic errors, got: {:?}",
            analyzer.errors
        );
    }

    #[test]
    fn test_recursive_action_call() {
        let input = r#"
define action called factorial with parameters n:
    check if n is less than or equal to 1:
        return 1
    end check
    return n times (call factorial with n minus 1)
end action

store result as call factorial with 5
        "#;
        let tokens = crate::lexer::lex_wfl_with_positions(input);
        let program = crate::parser::Parser::new(&tokens).parse().unwrap();

        let mut analyzer = Analyzer::new();
        let _ = analyzer.analyze(&program);

        // Should have no errors (recursive calls should work)
        assert!(
            analyzer.errors.is_empty(),
            "Recursive action calls should be valid, got: {:?}",
            analyzer.errors
        );
    }

    #[test]
    fn test_forward_action_reference() {
        let input = r#"
define action called first:
    call second with "test"
end action

define action called second with parameters msg:
    print with msg
end action
        "#;
        let tokens = crate::lexer::lex_wfl_with_positions(input);
        let program = crate::parser::Parser::new(&tokens).parse().unwrap();

        let mut analyzer = Analyzer::new();
        let _ = analyzer.analyze(&program);

        // Should have no errors (forward references should work)
        assert!(
            analyzer.errors.is_empty(),
            "Forward action references should be valid, got: {:?}",
            analyzer.errors
        );
    }

    #[test]
    fn test_action_parameter_scope_leakage() {
        let input = r#"
define action called first with parameters leaked_param:
    print with leaked_param
end action

define action called second:
    print with leaked_param
end action
        "#;
        let tokens = crate::lexer::lex_wfl_with_positions(input);
        let program = crate::parser::Parser::new(&tokens).parse().unwrap();

        let mut analyzer = Analyzer::new();
        let _ = analyzer.analyze(&program);

        // Should have error for undefined variable 'leaked_param' in second action
        assert!(
            !analyzer.errors.is_empty(),
            "Should have semantic errors for undefined variable"
        );
        assert!(
            analyzer
                .errors
                .iter()
                .any(|e| e.message.contains("Variable 'leaked_param' is not defined")),
            "Should report undefined variable 'leaked_param', got: {:?}",
            analyzer.errors
        );
    }

    #[test]
    fn test_foreach_loop_variable_scope_leakage() {
        let input = r#"
create list numbers:
    add 1
    add 2
end list

for each item in numbers:
    display item
end for

display item
        "#;
        let tokens = crate::lexer::lex_wfl_with_positions(input);
        let program = crate::parser::Parser::new(&tokens).parse().unwrap();

        let mut analyzer = Analyzer::new();
        let _ = analyzer.analyze(&program);

        // Should have error for undefined variable 'item' after the loop
        assert!(
            !analyzer.errors.is_empty(),
            "Should have semantic errors for undefined variable"
        );
        assert!(
            analyzer
                .errors
                .iter()
                .any(|e| e.message.contains("Variable 'item' is not defined")),
            "Should report undefined variable 'item', got: {:?}",
            analyzer.errors
        );
    }

    #[test]
    fn test_builtin_action_call_validation() {
        let input = r#"
print with "Hello"
        "#;
        let tokens = crate::lexer::lex_wfl_with_positions(input);
        let program = crate::parser::Parser::new(&tokens).parse().unwrap();

        let mut analyzer = Analyzer::new();
        let _ = analyzer.analyze(&program);

        // Should have no errors (builtin functions should be recognized)
        assert!(
            analyzer.errors.is_empty(),
            "Builtin function calls should be valid, got: {:?}",
            analyzer.errors
        );
    }

    #[test]
    fn test_action_not_a_function_error() {
        let input = r#"
store x as 10
call x with "test"
        "#;
        let tokens = crate::lexer::lex_wfl_with_positions(input);
        let program = crate::parser::Parser::new(&tokens).parse().unwrap();

        let mut analyzer = Analyzer::new();
        let _ = analyzer.analyze(&program);

        // Should have error that 'x' is not an action
        assert!(!analyzer.errors.is_empty(), "Should have semantic errors");
        assert!(
            analyzer
                .errors
                .iter()
                .any(|e| e.message.contains("'x' is not an action")),
            "Should report 'x' is not an action, got: {:?}",
            analyzer.errors
        );
    }

    #[test]
    fn test_module_load_undefined_path_variable() {
        let program = Program {
            statements: vec![Statement::LoadModuleStatement {
                path: Expression::Variable("undefined_var".to_string(), 1, 18),
                alias: None,
                line: 1,
                column: 1,
            }],
        };

        let mut analyzer = Analyzer::new();
        let result = analyzer.analyze(&program);

        assert!(
            result.is_err(),
            "Expected semantic error for undefined variable in module path"
        );

        let errors = analyzer.get_errors();
        assert!(
            errors.iter().any(|e| e.message.contains("undefined_var")),
            "Expected error about undefined_var, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_module_load_valid_path() {
        let program = Program {
            statements: vec![
                Statement::VariableDeclaration {
                    name: "module_path".to_string(),
                    value: Expression::Literal(Literal::String(Arc::from("helper.wfl")), 1, 18),
                    is_constant: false,
                    line: 1,
                    column: 1,
                },
                Statement::LoadModuleStatement {
                    path: Expression::Variable("module_path".to_string(), 2, 18),
                    alias: None,
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut analyzer = Analyzer::new();
        let result = analyzer.analyze(&program);

        assert!(result.is_ok(), "Expected no semantic errors");
    }

    #[test]
    fn test_module_load_path_expression() {
        use crate::parser::ast::Operator;

        let program = Program {
            statements: vec![
                Statement::VariableDeclaration {
                    name: "base_path".to_string(),
                    value: Expression::Literal(Literal::String(Arc::from("modules/")), 1, 18),
                    is_constant: false,
                    line: 1,
                    column: 1,
                },
                Statement::VariableDeclaration {
                    name: "module_name".to_string(),
                    value: Expression::Literal(Literal::String(Arc::from("helper")), 2, 18),
                    is_constant: false,
                    line: 2,
                    column: 1,
                },
                Statement::LoadModuleStatement {
                    path: Expression::BinaryOperation {
                        left: Box::new(Expression::BinaryOperation {
                            left: Box::new(Expression::Variable("base_path".to_string(), 3, 18)),
                            operator: Operator::Plus,
                            right: Box::new(Expression::Variable("module_name".to_string(), 3, 28)),
                            line: 3,
                            column: 18,
                        }),
                        operator: Operator::Plus,
                        right: Box::new(Expression::Literal(
                            Literal::String(Arc::from(".wfl")),
                            3,
                            40,
                        )),
                        line: 3,
                        column: 18,
                    },
                    alias: None,
                    line: 3,
                    column: 1,
                },
            ],
        };

        let mut analyzer = Analyzer::new();
        let result = analyzer.analyze(&program);

        assert!(
            result.is_ok(),
            "Expected no semantic errors for path expression"
        );
    }

    #[test]
    fn test_action_call_type_validation() {
        let input = r#"
define action called greet needs name as Text:
    print with name
end action

call greet with 123
        "#;
        let tokens = crate::lexer::lex_wfl_with_positions(input);
        let program = crate::parser::Parser::new(&tokens).parse().unwrap();

        let mut analyzer = Analyzer::new();
        let _ = analyzer.analyze(&program);

        assert!(!analyzer.errors.is_empty(), "Should have semantic errors");
        assert!(
            analyzer.errors.iter().any(|e| e
                .message
                .contains("Argument 'name' of action 'greet' expects Text, but got Number")),
            "Should report type mismatch, got: {:?}",
            analyzer.errors
        );
    }
}
