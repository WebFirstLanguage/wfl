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

/// Why two same-name signatures cannot coexist as overloads.
#[derive(Debug, Clone, Copy, PartialEq)]
enum SignatureConflict {
    /// Same parameter count and same (normalized) parameter types.
    ExactDuplicate,
    /// Same parameter count and no position where both declare concrete,
    /// different types — an untyped parameter accepts every value, so no
    /// call could ever be routed deterministically between the two.
    Ambiguous,
}

fn signature_conflict(a: &FunctionSignature, b: &FunctionSignature) -> Option<SignatureConflict> {
    if a.parameters.len() != b.parameters.len() {
        return None;
    }
    let exact_same_types = a
        .parameters
        .iter()
        .zip(&b.parameters)
        .all(|(pa, pb)| pa.param_type == pb.param_type);
    if exact_same_types {
        return Some(SignatureConflict::ExactDuplicate);
    }
    // `any`/`Unknown` annotations accept every value, so — like untyped
    // parameters — they cannot separate two overloads at a call site.
    let is_concrete =
        |t: &Option<Type>| matches!(t, Some(inner) if !matches!(inner, Type::Any | Type::Unknown));
    let distinguishable = a.parameters.iter().zip(&b.parameters).any(|(pa, pb)| {
        is_concrete(&pa.param_type) && is_concrete(&pb.param_type) && pa.param_type != pb.param_type
    });
    if distinguishable {
        None
    } else {
        Some(SignatureConflict::Ambiguous)
    }
}

/// Render a signature in WFL surface syntax for diagnostics, e.g.
/// `describe with value as number` or `reset (no parameters)`.
pub(crate) fn format_signature(name: &str, signature: &FunctionSignature) -> String {
    if signature.parameters.is_empty() {
        return format!("{name} (no parameters)");
    }
    let params = signature
        .parameters
        .iter()
        .map(|p| match &p.param_type {
            Some(t) => format!("{} as {}", p.name, format_param_type(t)),
            None => p.name.clone(),
        })
        .collect::<Vec<_>>()
        .join(" and ");
    format!("{name} with {params}")
}

/// Parameter types as they appear in source (`as number`, not `as Number`).
pub(crate) fn format_param_type(t: &Type) -> String {
    match t {
        Type::Text => "text".to_string(),
        Type::Number => "number".to_string(),
        Type::Boolean => "boolean".to_string(),
        Type::Nothing => "nothing".to_string(),
        Type::Pattern => "pattern".to_string(),
        Type::Any => "any".to_string(),
        Type::Custom(name) => name.clone(),
        other => format!("{other:?}"),
    }
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

    /// Mutable resolve: current scope first, then parents. Used when refining a
    /// symbol's type from an inner scope (e.g. `change` of an outer variable
    /// inside a loop body — issue #605). Mutations reach the boxed parent that
    /// `pop_scope` restores, so type updates survive the scope pop.
    pub fn resolve_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        if self.symbols.contains_key(name) {
            self.symbols.get_mut(name)
        } else if let Some(parent) = self.parent.as_mut() {
            parent.resolve_mut(name)
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
    /// The shared-budget breach hit mid-traversal, if any. Its presence latches
    /// the recursive `analyze_statement` checkpoint (record once, then
    /// short-circuit every remaining node instead of pushing a duplicate error
    /// per nested statement) and lets callers recover the *typed* breach — a
    /// budget failure is fatal and must never be mistaken for an ordinary
    /// semantic diagnostic. Reset per `analyze` run.
    budget_error: Option<crate::exec::budget::BudgetExceeded>,
    /// Variables that hold a stored action reference (`store h as f`), mapped
    /// to what is statically known about them so calls through the variable
    /// get real overload resolution. Removed again when the variable is
    /// reassigned to anything that is not a bare action reference; degraded to
    /// [`AliasState::Dynamic`] when control flow makes the binding uncertain.
    /// Reset per `analyze` run.
    action_aliases: HashMap<String, AliasState>,
    /// Overload definitions *visited so far* in the PASS-2 walk, per action
    /// name. PASS 1 registers signatures in lexical order, so an exact count
    /// is a prefix length into the symbol's signature list — the overloads a
    /// stored-action alias captured at its binding point (runtime snapshot
    /// semantics). Path-sensitive: control-flow constructs snapshot and join
    /// it alongside `action_aliases`, so a definition inside a branch that
    /// may not execute degrades the count to [`OverloadCount::Unknown`].
    /// Reset per `analyze` run.
    defined_overloads: HashMap<String, OverloadCount>,
    /// Stack of alias-mutation frames, one per loop/`try` body being
    /// analyzed. Records which alias names were *written at any point*
    /// during the body — endpoint flow states cannot see a binding that was
    /// mutated and then restored, but a `break`/`continue` or an error
    /// transferring to a `when` handler can expose exactly that
    /// intermediate state at runtime. Popping a frame merges it into its
    /// parent (an inner body's mutation is also the outer body's). Reset
    /// per `analyze` run.
    alias_mutation_frames: Vec<std::collections::HashSet<String>>,
    /// What each alias call site resolved to, keyed by (callee, line, column).
    /// The type checker reads this instead of the final alias map so it
    /// observes the alias state that held *at that statement*, not whatever
    /// the map ended up as. Reset per `analyze` run.
    alias_call_sites: HashMap<(String, usize, usize), AliasState>,
}

/// Statically known state of a stored-action alias variable.
#[derive(Debug, Clone, PartialEq)]
pub enum AliasState {
    /// Bound to `action`, seeing only the first `visible_signatures` entries
    /// of its signature list — the overloads lexically defined before the
    /// binding, matching the runtime's snapshot semantics.
    Bound {
        action: String,
        visible_signatures: usize,
    },
    /// Possibly an action (e.g. reassigned differently across branches):
    /// static validation is skipped and the call defers to runtime dispatch.
    Dynamic,
}

/// How many overloads of an action the PASS-2 walk has visited so far.
/// `Exact(n)` is a faithful prefix length into the PASS-1 signature list;
/// `Unknown` means the count depends on control flow (a definition sits in a
/// branch or loop whose execution is undecidable), so aliases bound after
/// that point defer to runtime dispatch instead of trusting a prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverloadCount {
    Exact(usize),
    Unknown,
}

impl OverloadCount {
    fn bump(self) -> Self {
        match self {
            OverloadCount::Exact(n) => OverloadCount::Exact(n + 1),
            OverloadCount::Unknown => OverloadCount::Unknown,
        }
    }
}

/// The control-flow-sensitive state snapshotted around branches: stored-
/// action aliases plus the visible-overload counters. Both describe "what
/// has executed so far", so they must travel through branch joins together —
/// joining one without the other lets unexecuted code leak into the other's
/// view (the round-4 review finding).
#[derive(Clone)]
struct FlowState {
    aliases: HashMap<String, AliasState>,
    overloads: HashMap<String, OverloadCount>,
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

        // Version of the running interpreter, exposed as an immutable Text
        // constant (#602).
        let wfl_version_symbol = Symbol {
            name: "wfl_version".to_string(),
            kind: SymbolKind::Variable { mutable: false },
            symbol_type: Some(Type::Text),
            line: 0,
            column: 0,
        };
        let _ = global_scope.define(wfl_version_symbol);

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
            budget_error: None,
            action_aliases: HashMap::new(),
            defined_overloads: HashMap::new(),
            alias_mutation_frames: Vec::new(),
            alias_call_sites: HashMap::new(),
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
        // Reset the per-run budget breach so a reused analyzer never carries a
        // stale one from a previous program (matches the direct assignment of
        // `has_includes` below).
        self.budget_error = None;

        // Front-end budget checkpoint at the analysis phase boundary. `analyze`
        // backs the type checker and every `load module` / `include` /
        // `execute file`, so consulting the run budget here (deadline/
        // cancellation, exemption-aware) keeps those nested pipelines cooperative
        // rather than only the top-level interpret.
        if let Some(budget) = crate::exec::budget::ExecutionBudget::current()
            && let Err(exceeded) = budget.charge_operation(!budget.is_deadline_exempt())
        {
            // Record the typed breach on the fatal channel BEFORE returning, so a
            // caller that consults `take_budget_error()` (e.g. `TypeChecker::new()`
            // entering with an already-exhausted/cancelled budget) sees an
            // entry-time breach as the fatal `Budget` variant rather than
            // misclassifying its rendered `SemanticError` as ordinary type errors.
            // `analyze_statement` sets this on the recursive path; the phase
            // boundary must too.
            let rendered = SemanticError::new(exceeded.message(), 0, 0);
            self.budget_error = Some(exceeded);
            return Err(vec![rendered]);
        }

        // Detect include statements up front. Includes are resolved at runtime
        // and can expose actions/variables the analyzer never sees, so their
        // presence relaxes undefined-action reporting (see `has_includes`).
        // Assign directly (not just set-to-true) so a reused analyzer instance
        // does not carry a stale flag from a previous program.
        self.has_includes = program_has_includes(program);

        // Stored-action alias state is per-program.
        self.action_aliases.clear();
        self.defined_overloads.clear();
        self.alias_mutation_frames.clear();
        self.alias_call_sites.clear();

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

    /// Take the shared-budget breach recorded during analysis, if any. When
    /// `analyze` returns `Err`, a caller must consult this to tell a fatal
    /// deadline/cancellation/resource breach apart from ordinary semantic
    /// diagnostics — the breach is fatal and its `Vec<SemanticError>` form is
    /// only a rendering of the same event.
    pub fn take_budget_error(&mut self) -> Option<crate::exec::budget::BudgetExceeded> {
        self.budget_error.take()
    }

    /// Report an undefined-name reference. Inside a `try` body this is a
    /// warning rather than a fatal error: the reference raises a catchable
    /// runtime error, which is documented behavior that programs rely on.
    /// Analyze the body of an unbounded loop (`forever` / `main loop`). Shared by
    /// both so the scope handling, flow tracking, and the handler-body error
    /// demotion stay identical. Web-server main-loop bodies reference
    /// handler-provided names the analyzer cannot model, so errors raised *inside*
    /// the body are demoted to warnings (a latched budget breach stays fatal);
    /// errors the caller raises about the loop itself are pushed before this runs
    /// and are not swept up.
    fn analyze_loop_body(&mut self, body: &[Statement]) {
        let outer_scope = std::mem::take(&mut self.current_scope);
        self.current_scope = Scope::with_parent(outer_scope);

        let flow_entry = self.flow_entry();
        self.push_mutation_frame();
        let errors_before = self.errors.len();
        for stmt in body {
            self.analyze_statement(stmt);
        }
        if self.budget_error.is_none() {
            let demoted: Vec<_> = self.errors.drain(errors_before..).collect();
            self.warnings.extend(demoted);
        }
        let flow_body = self.take_flow_branch(&flow_entry);
        let mutated = self.pop_mutation_frame();
        self.join_flow_branches(&[flow_body, flow_entry]);
        self.degrade_mutated_aliases(&mutated);

        let loop_scope = std::mem::take(&mut self.current_scope);
        if let Some(parent) = loop_scope.parent {
            self.current_scope = *parent;
        }
    }

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
        // Recursive front-end checkpoint. The entry poll in `analyze` fires once,
        // but this method recurses through every nested block, loop, `try`,
        // action, and container-method body — so a single deeply nested
        // top-level statement could otherwise run the analyzer to completion
        // without honoring the run budget. Polling here (mirroring the parser's
        // per-`parse_statement` placement) keeps the whole traversal cooperative
        // with the deadline/cancellation/operation limits. Once exhausted, the
        // flag short-circuits every remaining node so the breach is recorded a
        // single time rather than duplicated per statement.
        if self.budget_error.is_some() {
            return;
        }
        if let Some(budget) = crate::exec::budget::ExecutionBudget::current()
            && let Err(exceeded) = budget.charge_operation(!budget.is_deadline_exempt())
        {
            self.errors
                .push(SemanticError::new(exceeded.message(), 0, 0));
            self.budget_error = Some(exceeded);
            return;
        }

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

                // Aliases only track real variable bindings: a container
                // property assignment creates no binding, and a failed define
                // must not disturb an outer variable's alias state.
                if !is_property_assignment {
                    match self.current_scope.define(symbol) {
                        Err(error) => self.errors.push(error),
                        Ok(()) => self.update_action_alias(name, value),
                    }
                }
            }
            Statement::Assignment {
                name,
                value,
                line,
                column,
            } => {
                let mut skip_value_analysis = false;
                let mut is_variable_assignment = false;

                if let Some(symbol) = self.current_scope.resolve(name) {
                    match &symbol.kind {
                        SymbolKind::Variable { mutable } => {
                            is_variable_assignment = *mutable;
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
                    // Aliases only track real variable rebinds; invalid or
                    // property/undefined targets leave alias state untouched.
                    if is_variable_assignment {
                        self.update_action_alias(name, value);
                    }
                }
            }
            Statement::ActionDefinition { name, .. } => {
                // Signature was already registered in Pass 1
                // Now analyze the body in Pass 2
                let counter = self
                    .defined_overloads
                    .entry(name.clone())
                    .or_insert(OverloadCount::Exact(0));
                *counter = counter.bump();
                self.analyze_action_body(statement);
            }
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.analyze_expression(condition);
                let flow_entry = self.flow_entry();

                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope.clone());

                for stmt in then_block {
                    self.analyze_statement(stmt);
                }
                let flow_then = self.take_flow_branch(&flow_entry);

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
                    let flow_else = self.take_flow_branch(&flow_entry);
                    self.join_flow_branches(&[flow_then.clone(), flow_else]);

                    let else_scope = std::mem::take(&mut self.current_scope);

                    for (name, symbol) in &else_scope.symbols {
                        if outer_scope_for_else.resolve(name).is_none() {
                            defined_in_else.push((name.clone(), symbol.clone()));
                        }
                    }

                    if let Some(parent) = else_scope.parent {
                        self.current_scope = *parent;
                    }
                } else {
                    // No else: the construct may be skipped entirely.
                    self.join_flow_branches(&[flow_then.clone(), flow_entry.clone()]);
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
                let flow_entry = self.flow_entry();

                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope);

                self.analyze_statement(then_stmt);
                let flow_then = self.take_flow_branch(&flow_entry);

                let then_scope = std::mem::take(&mut self.current_scope);
                if let Some(parent) = then_scope.parent {
                    self.current_scope = *parent;
                }

                if let Some(else_stmt) = else_stmt {
                    let outer_scope = std::mem::take(&mut self.current_scope);
                    self.current_scope = Scope::with_parent(outer_scope);

                    self.analyze_statement(else_stmt);
                    let flow_else = self.take_flow_branch(&flow_entry);
                    self.join_flow_branches(&[flow_then, flow_else]);

                    let else_scope = std::mem::take(&mut self.current_scope);
                    if let Some(parent) = else_scope.parent {
                        self.current_scope = *parent;
                    }
                } else {
                    self.join_flow_branches(&[flow_then, flow_entry]);
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

                let flow_entry = self.flow_entry();
                self.push_mutation_frame();
                for stmt in body {
                    self.analyze_statement(stmt);
                }
                // The body may run zero times: join it with the skip path.
                let flow_body = self.take_flow_branch(&flow_entry);
                let mutated = self.pop_mutation_frame();
                self.join_flow_branches(&[flow_body, flow_entry]);
                // A break/continue can leave the loop while a mutated alias
                // holds an intermediate binding the body later restores —
                // endpoint states cannot see it, so mutated names defer to
                // runtime dispatch.
                self.degrade_mutated_aliases(&mutated);

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

                let flow_entry = self.flow_entry();
                self.push_mutation_frame();
                for stmt in body {
                    self.analyze_statement(stmt);
                }
                // The body may run zero times: join it with the skip path.
                let flow_body = self.take_flow_branch(&flow_entry);
                let mutated = self.pop_mutation_frame();
                self.join_flow_branches(&[flow_body, flow_entry]);
                // A break/continue can leave the loop while a mutated alias
                // holds an intermediate binding the body later restores —
                // endpoint states cannot see it, so mutated names defer to
                // runtime dispatch.
                self.degrade_mutated_aliases(&mutated);

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

                let flow_entry = self.flow_entry();
                self.push_mutation_frame();
                for stmt in body {
                    self.analyze_statement(stmt);
                }
                // The body may run zero times: join it with the skip path.
                let flow_body = self.take_flow_branch(&flow_entry);
                let mutated = self.pop_mutation_frame();
                self.join_flow_branches(&[flow_body, flow_entry]);
                // A break/continue can leave the loop while a mutated alias
                // holds an intermediate binding the body later restores —
                // endpoint states cannot see it, so mutated names defer to
                // runtime dispatch.
                self.degrade_mutated_aliases(&mutated);

                let loop_scope = std::mem::take(&mut self.current_scope);
                if let Some(parent) = loop_scope.parent {
                    self.current_scope = *parent;
                }
            }
            // The remaining loop forms get the same flow treatment. The
            // pre-checked forms may run zero times, so the body joins with
            // the entry (skip) path; `forever`/`main loop` exit only through
            // an abrupt `break`, which the mutation-frame degradation
            // covers, so including the entry path there is merely
            // conservative, never wrong.
            Statement::RepeatWhileLoop {
                condition, body, ..
            }
            | Statement::RepeatUntilLoop {
                condition, body, ..
            } => {
                self.analyze_expression(condition);

                let outer_scope = std::mem::take(&mut self.current_scope);
                self.current_scope = Scope::with_parent(outer_scope);

                let flow_entry = self.flow_entry();
                self.push_mutation_frame();
                let errors_before = self.errors.len();
                for stmt in body {
                    self.analyze_statement(stmt);
                }
                // These bodies were not analyzed before alias flow tracking
                // reached them; new diagnostics inside would break existing
                // programs (backward compatibility), so they demote to
                // warnings — runtime behavior is authoritative in here. A
                // latched budget breach is different: analysis was *aborted*,
                // not diagnosed, and its rendered error must stay fatal.
                if self.budget_error.is_none() {
                    let demoted: Vec<_> = self.errors.drain(errors_before..).collect();
                    self.warnings.extend(demoted);
                }
                let flow_body = self.take_flow_branch(&flow_entry);
                let mutated = self.pop_mutation_frame();
                self.join_flow_branches(&[flow_body, flow_entry]);
                self.degrade_mutated_aliases(&mutated);

                let loop_scope = std::mem::take(&mut self.current_scope);
                if let Some(parent) = loop_scope.parent {
                    self.current_scope = *parent;
                }
            }
            Statement::ForeverLoop { body, .. } => {
                self.analyze_loop_body(body);
            }
            Statement::MainLoop {
                body,
                concurrent,
                line,
                column,
            } => {
                // `main loop concurrently:` starts up to the concurrency cap of
                // handler futures at once, each running the body from the top. Any
                // statement before the first `wait for request` therefore runs
                // once *per handler slot* before a single request is dequeued —
                // speculative side effects the author almost never intends. Require
                // the body to begin with `wait for request` so nothing runs before
                // a request is in hand. (Serial `main loop` has no such hazard.)
                // Emitted before the body is analyzed so it is NOT swept into the
                // handler-body error demotion below.
                if *concurrent
                    && !matches!(
                        body.first(),
                        Some(Statement::WaitForRequestStatement { .. })
                    )
                {
                    self.errors.push(SemanticError::new(
                        "A `main loop concurrently:` body must begin with `wait for request ...`. \
                         Concurrent handlers start before any request arrives, so statements before \
                         the first `wait for request` run once per handler slot. Move that setup above \
                         the loop, or make `wait for request` the first statement in the loop."
                            .to_string(),
                        *line,
                        *column,
                    ));
                }
                self.analyze_loop_body(body);
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
                let flow_entry = self.flow_entry();
                self.push_mutation_frame();
                self.try_depth += 1;
                for stmt in body {
                    self.analyze_statement(stmt);
                }
                self.try_depth -= 1;
                let flow_try = self.take_flow_branch(&flow_entry);
                let mutated_in_body = self.pop_mutation_frame();
                // Error clauses can be entered from any point in the body, so
                // they start from the entry/body join — with every alias the
                // body mutated *anywhere* degraded to Dynamic, because the
                // error may fire while such an alias holds an intermediate
                // binding the body's endpoint state has already restored.
                self.join_flow_branches(&[flow_try.clone(), flow_entry.clone()]);
                self.degrade_mutated_aliases(&mutated_in_body);
                let flow_handler_entry = self.flow_entry();
                let mut flow_paths: Vec<FlowState> = vec![flow_try];

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

                    self.restore_flow(&flow_handler_entry);
                    for stmt in &when_clause.body {
                        self.analyze_statement(stmt);
                    }
                    flow_paths.push(self.take_flow_branch(&flow_handler_entry));

                    let when_scope = std::mem::take(&mut self.current_scope);
                    if let Some(parent) = when_scope.parent {
                        self.current_scope = *parent;
                    }
                }

                if let Some(otherwise_stmts) = otherwise_block {
                    let outer_scope = std::mem::take(&mut self.current_scope);
                    self.current_scope = Scope::with_parent(outer_scope);

                    self.restore_flow(&flow_handler_entry);
                    for stmt in otherwise_stmts {
                        self.analyze_statement(stmt);
                    }
                    flow_paths.push(self.take_flow_branch(&flow_handler_entry));

                    let otherwise_scope = std::mem::take(&mut self.current_scope);
                    if let Some(parent) = otherwise_scope.parent {
                        self.current_scope = *parent;
                    }
                }

                // After the construct, any of the recorded paths may have run.
                self.join_flow_branches(&flow_paths);

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

            Statement::HttpStreamStatement {
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

                // Binds a streaming-response handle object (status/ok/headers).
                let symbol = Symbol {
                    name: variable_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: None,
                    line: 0,
                    column: 0,
                };
                self.current_scope.define_or_replace(symbol);
            }

            Statement::WaitForNextChunkStatement {
                source,
                variable_name,
                ..
            }
            | Statement::WaitForNextLineStatement {
                source,
                variable_name,
                ..
            } => {
                self.analyze_expression(source);

                // Binds the next chunk/line, or `nothing` at end of stream, so
                // the type is left open. Refreshed on every wait (loop-friendly).
                let symbol = Symbol {
                    name: variable_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: None,
                    line: 0,
                    column: 0,
                };
                self.current_scope.define_or_replace(symbol);
            }

            Statement::StartStreamingResponseStatement {
                request,
                status,
                content_type,
                headers,
                variable_name,
                ..
            } => {
                self.analyze_expression(request);
                if let Some(status) = status {
                    self.analyze_expression(status);
                }
                if let Some(content_type) = content_type {
                    self.analyze_expression(content_type);
                }
                if let Some(headers) = headers {
                    self.analyze_expression(headers);
                }
                // Binds a server response-stream handle object.
                let symbol = Symbol {
                    name: variable_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: None,
                    line: 0,
                    column: 0,
                };
                self.current_scope.define_or_replace(symbol);
            }

            Statement::StreamWriteStatement {
                value,
                target,
                fallback_content,
                line,
                column,
                ..
            } => {
                self.analyze_expression(target);
                match fallback_content {
                    // Unambiguous form: check the stream value normally.
                    None => self.analyze_expression(value),
                    // Ambiguous merged form (`write line <ident> to <target>`):
                    // the live reading — stream write of `<ident>` vs classic
                    // file write of the variable `line <ident>` — depends on the
                    // runtime target type, and the two read different variables.
                    // Still (1) analyze any unambiguous subexpression (the
                    // `<object>` in `<field> of <object>`), and (2) report an
                    // undefined variable only when *neither* candidate name is
                    // defined, so a genuine typo is still caught without breaking
                    // either valid reading.
                    Some(fallback) => {
                        // The `of <object>` argument is the same under both
                        // readings and is unambiguous — analyze it.
                        if let Expression::FunctionCall { arguments, .. } = value {
                            for arg in arguments {
                                self.analyze_expression(&arg.value);
                            }
                        }
                        let stream_name = Self::stream_write_candidate_name(value);
                        let fallback_name = Self::stream_write_candidate_name(fallback);
                        if let (Some(sn), Some(fal)) = (stream_name, fallback_name)
                            && !self.name_is_defined(sn)
                            && !self.name_is_defined(fal)
                        {
                            // Neither reading resolves — report the classic
                            // (file-write) name, matching the runtime fallback.
                            self.report_undefined_name(
                                format!("Variable '{fal}' is not defined"),
                                *line,
                                *column,
                            );
                        }
                    }
                }
            }

            Statement::FlushStreamStatement { target, .. } => {
                self.analyze_expression(target);
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
                            // Unannotated methods get a provisional `Unknown`
                            // (not `Nothing`) so uses of a method result checked
                            // before the type checker refines it degrade
                            // gracefully instead of raising false "found
                            // Nothing" errors (issue #560 residual). The type
                            // checker infers the real return type from the body
                            // and writes it back via `get_container_mut`.
                            return_type: return_type.as_ref().cloned().unwrap_or(Type::Unknown),
                            is_public: true, // Default to public for now
                            line: *method_line,
                            column: *method_column,
                        };

                        // TODO(#638): container methods do not support
                        // overloading — a repeated method name silently
                        // replaces the earlier registration (contrast
                        // `register_action_signature`, which merges or errors).
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

                        // Same isolation as `analyze_action_body`: a method
                        // body has not executed at definition time.
                        let alias_entry = self.action_aliases.clone();
                        let overloads_entry = self.defined_overloads.clone();
                        for stmt in body {
                            self.analyze_statement(stmt);
                        }
                        self.action_aliases = alias_entry;
                        self.defined_overloads = overloads_entry;

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
                            // Same provisional `Unknown` seed as instance
                            // methods above (issue #560 residual).
                            return_type: return_type.as_ref().cloned().unwrap_or(Type::Unknown),
                            is_public: true, // Default to public for now
                            line: *method_line,
                            column: *method_column,
                        };

                        // TODO(#638): same silent last-wins as instance
                        // methods above — no overloading for static methods.
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

                        // Same isolation as `analyze_action_body`: a method
                        // body has not executed at definition time.
                        let alias_entry = self.action_aliases.clone();
                        let overloads_entry = self.defined_overloads.clone();
                        for stmt in body {
                            self.analyze_statement(stmt);
                        }
                        self.action_aliases = alias_entry;
                        self.defined_overloads = overloads_entry;

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

                // Handler bodies run per event, not at registration — same
                // alias isolation as action bodies.
                let alias_entry = self.action_aliases.clone();
                let overloads_entry = self.defined_overloads.clone();
                for stmt in body {
                    self.analyze_statement(stmt);
                }
                self.action_aliases = alias_entry;
                self.defined_overloads = overloads_entry;

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
            let new_signature = FunctionSignature {
                parameters: parameters.clone(),
                return_type: return_type.clone(),
            };

            // Same-scope redefinition of an existing action is the overload
            // path: accumulate the signature instead of erroring, unless the
            // pair is an exact duplicate or cannot be told apart at a call
            // site. Collisions with non-function symbols and parent-scope
            // shadowing keep their existing `Scope::define` errors.
            if let Some(existing) = self.current_scope.symbols.get_mut(name)
                && let SymbolKind::Function { signatures } = &mut existing.kind
            {
                let first_line = existing.line;
                match signatures
                    .iter()
                    .find_map(|sig| signature_conflict(sig, &new_signature))
                {
                    Some(SignatureConflict::ExactDuplicate) => {
                        self.errors.push(SemanticError::new(
                            format!(
                                "Action '{name}' was already defined with the same parameters at line {first_line}. \
                                 Overloads must differ in parameter count or in their declared parameter types \
                                 (e.g. 'value as number' vs 'value as text')."
                            ),
                            *line,
                            *column,
                        ));
                    }
                    Some(SignatureConflict::Ambiguous) => {
                        self.errors.push(SemanticError::new(
                            format!(
                                "These two versions of '{name}' cannot be told apart when called with {} argument(s). \
                                 Give every same-parameter-count overload distinct parameter types \
                                 ('as number', 'as text', ...), or use different parameter counts.",
                                new_signature.parameters.len()
                            ),
                            *line,
                            *column,
                        ));
                    }
                    None => signatures.push(new_signature),
                }
                return;
            }

            let symbol = Symbol {
                name: name.clone(),
                kind: SymbolKind::Function {
                    signatures: vec![new_signature],
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

            // Analyze body statements. The body has not *executed* at the
            // point of definition, so any alias mutations it performs must
            // not leak into (or clobber) the surrounding scope's alias state;
            // inside the body the definition-point state approximates what
            // the closure captured. Nested definitions likewise must not
            // inflate the outer visible-overload counters.
            let alias_entry = self.action_aliases.clone();
            let overloads_entry = self.defined_overloads.clone();
            for stmt in body {
                self.analyze_statement(stmt);
            }
            self.action_aliases = alias_entry;
            self.defined_overloads = overloads_entry;

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
        // Walk parent scopes so type refinements (e.g. widening away from
        // Nothing on reassignment inside a loop) update the real binding and
        // survive `pop_scope` (issue #605). Previously only the innermost
        // scope was mutable, so outer-variable updates from loop/try bodies
        // were silently dropped.
        self.current_scope.resolve_mut(name)
    }

    pub fn define_symbol(&mut self, symbol: Symbol) -> Result<(), SemanticError> {
        self.current_scope.define(symbol)
    }

    /// Bind or overwrite a symbol in the *current* scope only (no parent walk,
    /// no "already defined in outer scope" error). Used for implicit bindings
    /// such as `when error as e` error names that must shadow for the clause
    /// body without clobbering an outer variable of the same name.
    pub fn define_or_replace_symbol(&mut self, symbol: Symbol) {
        self.current_scope.define_or_replace(symbol);
    }

    /// Snapshot `symbol_type` for every symbol in the current scope chain
    /// (innermost first). Used to restore outer bindings after type-checking
    /// a definition body that must not permanently refine them (uncalled
    /// actions / container methods — PR #606 review).
    pub fn snapshot_symbol_types(&self) -> Vec<HashMap<String, Option<Type>>> {
        let mut layers = Vec::new();
        let mut scope = Some(&self.current_scope);
        while let Some(s) = scope {
            let mut map = HashMap::new();
            for (name, sym) in &s.symbols {
                map.insert(name.clone(), sym.symbol_type.clone());
            }
            layers.push(map);
            scope = s.parent.as_deref();
        }
        layers
    }

    /// Restore `symbol_type` values previously captured by
    /// [`snapshot_symbol_types`]. Only updates symbols that still exist; does
    /// not remove symbols defined after the snapshot.
    pub fn restore_symbol_types(&mut self, snapshot: Vec<HashMap<String, Option<Type>>>) {
        let mut scope = Some(&mut self.current_scope);
        let mut i = 0;
        while let Some(s) = scope {
            if i >= snapshot.len() {
                break;
            }
            for (name, ty) in &snapshot[i] {
                if let Some(sym) = s.symbols.get_mut(name) {
                    sym.symbol_type = ty.clone();
                }
            }
            i += 1;
            scope = s.parent.as_mut().map(|p| p.as_mut());
        }
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

    /// Mutable access to a registered container, used by the type checker to
    /// refine an unannotated method's provisional return type after inferring
    /// it from the method body (issue #560 residual).
    pub fn get_container_mut(&mut self, name: &str) -> Option<&mut ContainerInfo> {
        self.containers.get_mut(name)
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

    /// Validates a call against every registered signature of `name`:
    /// filters candidates by argument count, then (when several share the
    /// count) by static argument types. A single surviving candidate gets the
    /// full named-argument and type validation; several survivors — argument
    /// types only known at runtime, `nothing` arguments (compatible with
    /// every parameter type), or container-inheritance overlap — defer
    /// dispatch to the interpreter with no diagnostic.
    ///
    /// `is_of_form` only selects the historical wording of the arity error
    /// ("Function ... arguments" for the `of` form vs "Action ...
    /// argument(s)" for the `call` form).
    fn check_overloaded_call(
        &mut self,
        name: &str,
        signatures: &[FunctionSignature],
        arguments: &[crate::parser::ast::Argument],
        is_of_form: bool,
        line: usize,
        column: usize,
    ) {
        let arity_matches: Vec<&FunctionSignature> = signatures
            .iter()
            .filter(|sig| sig.parameters.len() == arguments.len())
            .collect();

        if arity_matches.is_empty() {
            let mut arities: Vec<usize> =
                signatures.iter().map(|sig| sig.parameters.len()).collect();
            arities.sort_unstable();
            arities.dedup();
            let arities_str = arities
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(" or ");
            let message = if is_of_form {
                format!(
                    "Function '{name}' expects {arities_str} arguments, but {} were provided",
                    arguments.len()
                )
            } else {
                format!(
                    "Action '{name}' expects {arities_str} argument(s), but {} were provided",
                    arguments.len()
                )
            };
            self.errors.push(SemanticError::new(message, line, column));
            return;
        }

        if arity_matches.len() == 1 {
            self.check_call_against_signature(name, arity_matches[0], arguments, line, column);
            return;
        }

        let compatible: Vec<&FunctionSignature> = arity_matches
            .iter()
            .copied()
            .filter(|sig| self.signature_accepts(sig, arguments))
            .collect();

        match compatible.len() {
            1 => self.check_call_against_signature(name, compatible[0], arguments, line, column),
            0 => {
                let provided: Vec<String> = arguments
                    .iter()
                    .map(|arg| {
                        Self::format_type_for_display(&self.infer_expression_type(&arg.value))
                    })
                    .collect();
                let mut message = format!(
                    "No version of '{name}' matches this call.\nYou provided ({}), but '{name}' accepts:",
                    provided.join(", ")
                );
                for sig in &arity_matches {
                    message.push_str(&format!("\n  {}", format_signature(name, sig)));
                }
                self.errors.push(SemanticError::new(message, line, column));
            }
            _ => {
                // Several overloads still accept the call — runtime-only
                // argument types, `nothing` arguments, or container-
                // inheritance overlap. The interpreter dispatches on the
                // actual values.
            }
        }
    }

    /// Whether `signature` could accept this call: every argument maps onto a
    /// distinct parameter (by name or position) and no statically-known
    /// argument type contradicts a concrete parameter annotation. Used to
    /// filter same-arity overload candidates; unlike
    /// `check_call_against_signature` it reports nothing.
    fn signature_accepts(
        &self,
        signature: &FunctionSignature,
        arguments: &[crate::parser::ast::Argument],
    ) -> bool {
        let mut mapped: Vec<Option<&Expression>> = vec![None; signature.parameters.len()];

        for (arg_idx, arg) in arguments.iter().enumerate() {
            let param_idx = if let Some(arg_name) = &arg.name {
                match signature
                    .parameters
                    .iter()
                    .position(|p| p.name == *arg_name)
                {
                    Some(idx) => idx,
                    None => return false,
                }
            } else {
                arg_idx
            };
            if mapped[param_idx].is_some() {
                return false;
            }
            mapped[param_idx] = Some(&arg.value);
        }

        signature
            .parameters
            .iter()
            .zip(mapped)
            .all(|(param, arg_opt)| {
                let (Some(expected_type), Some(arg_val)) = (&param.param_type, arg_opt) else {
                    return true;
                };
                if matches!(expected_type, Type::Unknown | Type::Any) {
                    return true;
                }
                let arg_type = self.infer_expression_type(arg_val);
                arg_type == Type::Unknown || self.is_type_compatible(&arg_type, expected_type)
            })
    }

    /// Full validation of a call against one resolved signature: argument
    /// count, named-argument mapping (unknown parameter, duplicate argument),
    /// and per-argument type compatibility.
    fn check_call_against_signature(
        &mut self,
        name: &str,
        signature: &FunctionSignature,
        arguments: &[crate::parser::ast::Argument],
        line: usize,
        column: usize,
    ) {
        if arguments.len() != signature.parameters.len() {
            self.errors.push(SemanticError::new(
                format!(
                    "Action '{}' expects {} argument(s), but {} were provided",
                    name,
                    signature.parameters.len(),
                    arguments.len()
                ),
                line,
                column,
            ));

            // Skip type validation if arity is mismatched to avoid noisy/cascading errors
            return;
        }

        // Map arguments to parameters for type validation
        let mut matched_args: Vec<Option<&Expression>> = vec![None; signature.parameters.len()];
        let mut has_mapping_error = false;

        for (arg_idx, arg) in arguments.iter().enumerate() {
            let mut param_idx_opt = None;

            if let Some(arg_name) = &arg.name {
                // Named argument
                if let Some(idx) = signature
                    .parameters
                    .iter()
                    .position(|p| p.name == *arg_name)
                {
                    param_idx_opt = Some(idx);
                } else {
                    self.errors.push(SemanticError::new(
                        format!("Unknown parameter '{arg_name}' for action '{name}'"),
                        line,
                        column,
                    ));
                    has_mapping_error = true;
                }
            } else {
                // Positional argument
                if arg_idx < signature.parameters.len() {
                    param_idx_opt = Some(arg_idx);
                }
            }

            if let Some(param_idx) = param_idx_opt {
                if matched_args[param_idx].is_some() {
                    let param_name = &signature.parameters[param_idx].name;
                    self.errors.push(SemanticError::new(
                        format!(
                            "Duplicate argument for parameter '{param_name}' in action '{name}'"
                        ),
                        line,
                        column,
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
        for (param, arg_opt) in signature.parameters.iter().zip(matched_args.iter()) {
            if let Some(arg_val) = arg_opt
                && let Some(expected_type) = &param.param_type
            {
                let arg_type = self.infer_expression_type(arg_val);

                if arg_type != Type::Unknown
                    && expected_type != &Type::Unknown
                    && expected_type != &Type::Any
                    && !self.is_type_compatible(&arg_type, expected_type)
                {
                    let expected_display = Self::format_type_for_display(expected_type);
                    let actual_display = Self::format_type_for_display(&arg_type);
                    self.errors.push(SemanticError::new(
                        format!(
                            "Argument '{}' of action '{}' expects {}, but got {}",
                            param.name, name, expected_display, actual_display
                        ),
                        line,
                        column,
                    ));
                }
            }
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

        // NOTE: the tuple below is deliberately (expected, actual) — the
        // reverse of the parameter order — so every arm reads
        // "(what the target requires, what the value is)". Keep new arms in
        // that orientation (e.g. ancestry checks walk from `actual` up to
        // `expected`).
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

            // Allow implicitly resolving custom types; a descendant container
            // is compatible with an ancestor-typed target.
            (Type::Custom(expected_name), Type::Custom(actual_name)) => {
                expected_name == actual_name
                    || self.container_is_or_extends(actual_name, expected_name)
            }

            // Container instances match a parameter annotated with their own
            // container name (`value as Dog` parses as Custom("Dog")) or any
            // ancestor via the `extends` chain.
            (Type::Custom(expected_name), Type::ContainerInstance(actual_name))
            | (Type::ContainerInstance(expected_name), Type::ContainerInstance(actual_name)) => {
                self.container_is_or_extends(actual_name, expected_name)
            }

            _ => false,
        }
    }

    /// Records or removes the stored-action alias for `name` after a
    /// `store`/`change`: a bare reference to an action (directly or through
    /// another alias) makes `name` callable with that action's overload set;
    /// any other value clears a previous alias.
    fn update_action_alias(&mut self, name: &str, value: &Expression) {
        let target = if let Expression::Variable(source, _, _) = value {
            match self.current_scope.resolve(source) {
                Some(symbol) if matches!(symbol.kind, SymbolKind::Function { .. }) => {
                    // Snapshot semantics: the alias sees the overloads defined
                    // *before this statement*, exactly like the runtime value
                    // it will hold. A pure forward reference (zero defined so
                    // far) has no runtime value yet, so no alias is recorded.
                    match self.defined_overloads.get(source).copied() {
                        Some(OverloadCount::Exact(visible)) if visible > 0 => {
                            Some(AliasState::Bound {
                                action: source.clone(),
                                visible_signatures: visible,
                            })
                        }
                        // Control flow made the visible set undecidable: keep
                        // the alias, but let runtime dispatch judge its calls.
                        Some(OverloadCount::Unknown) => Some(AliasState::Dynamic),
                        _ => None,
                    }
                }
                // Aliasing an alias copies its state — including the original
                // snapshot, not a re-read of the current definition count.
                _ => self.action_aliases.get(source).cloned(),
            }
        } else {
            None
        };
        match target {
            Some(state) => {
                self.action_aliases.insert(name.to_string(), state);
                self.record_alias_mutation(name);
            }
            None => {
                if self.action_aliases.remove(name).is_some() {
                    self.record_alias_mutation(name);
                }
            }
        }
    }

    /// Records that `name`'s alias state was written while a loop/`try`
    /// body frame is active (no-op at unframed depth).
    fn record_alias_mutation(&mut self, name: &str) {
        if let Some(top) = self.alias_mutation_frames.last_mut() {
            top.insert(name.to_string());
        }
    }

    fn push_mutation_frame(&mut self) {
        self.alias_mutation_frames.push(Default::default());
    }

    /// Pops the current mutation frame, merging it into the parent frame —
    /// an inner body's mutation is also a mutation within the outer body.
    fn pop_mutation_frame(&mut self) -> std::collections::HashSet<String> {
        let frame = self.alias_mutation_frames.pop().unwrap_or_default();
        if let Some(parent) = self.alias_mutation_frames.last_mut() {
            parent.extend(frame.iter().cloned());
        }
        frame
    }

    /// Degrades every mutated alias to [`AliasState::Dynamic`]: an abrupt
    /// exit may expose an intermediate binding endpoint states cannot see,
    /// so calls through these names defer to runtime dispatch.
    fn degrade_mutated_aliases(&mut self, mutated: &std::collections::HashSet<String>) {
        for name in mutated {
            self.action_aliases
                .insert(name.clone(), AliasState::Dynamic);
        }
    }

    /// Snapshot of the control-flow-sensitive state at a construct's entry.
    fn flow_entry(&self) -> FlowState {
        FlowState {
            aliases: self.action_aliases.clone(),
            overloads: self.defined_overloads.clone(),
        }
    }

    /// Ends one control-flow branch's analysis: returns the branch's
    /// resulting flow state (aliases + visible-overload counters) and resets
    /// the live state to the `entry` snapshot so the next branch starts from
    /// the construct's entry state.
    ///
    /// Flow state is deliberately *not* threaded through unexecuted or
    /// conditionally-executed code as if it ran — see `join_flow_branches`.
    fn take_flow_branch(&mut self, entry: &FlowState) -> FlowState {
        FlowState {
            aliases: std::mem::replace(&mut self.action_aliases, entry.aliases.clone()),
            overloads: std::mem::replace(&mut self.defined_overloads, entry.overloads.clone()),
        }
    }

    /// Resets the live flow state to a snapshot (used by `try` handlers,
    /// which can be entered from any point in the guarded body).
    fn restore_flow(&mut self, state: &FlowState) {
        self.action_aliases = state.aliases.clone();
        self.defined_overloads = state.overloads.clone();
    }

    /// Joins the flow state of every path that can reach the statement after
    /// a control-flow construct.
    ///
    /// Aliases: a name keeps its state only when every path agrees; any
    /// disagreement (including absence on some path) degrades it to
    /// [`AliasState::Dynamic`], so later calls defer to runtime dispatch
    /// instead of being misjudged from one path's state.
    ///
    /// Overload counters: a count survives only when every path agrees on
    /// the same exact value (absence counts as zero); disagreement — a
    /// definition executed on one path but not another — degrades to
    /// [`OverloadCount::Unknown`], which makes later alias bindings Dynamic.
    fn join_flow_branches(&mut self, paths: &[FlowState]) {
        let mut keys: std::collections::HashSet<&String> = std::collections::HashSet::new();
        for path in paths {
            keys.extend(path.aliases.keys());
        }
        let mut joined = HashMap::new();
        for key in keys {
            let first = paths[0].aliases.get(key);
            let state = if paths.iter().all(|p| p.aliases.get(key) == first) {
                first.cloned()
            } else {
                Some(AliasState::Dynamic)
            };
            if let Some(state) = state {
                joined.insert(key.clone(), state);
            }
        }
        self.action_aliases = joined;

        let mut names: std::collections::HashSet<&String> = std::collections::HashSet::new();
        for path in paths {
            names.extend(path.overloads.keys());
        }
        let mut counters = HashMap::new();
        for name in names {
            let first = paths[0]
                .overloads
                .get(name)
                .copied()
                .unwrap_or(OverloadCount::Exact(0));
            let agreed = paths.iter().all(|p| {
                p.overloads
                    .get(name)
                    .copied()
                    .unwrap_or(OverloadCount::Exact(0))
                    == first
            });
            counters.insert(
                name.clone(),
                if agreed {
                    first
                } else {
                    OverloadCount::Unknown
                },
            );
        }
        self.defined_overloads = counters;
    }

    /// The signatures a call through alias `name` may dispatch to right now:
    /// the snapshot prefix for a bound alias, or `Dynamic` when static
    /// knowledge was lost to control flow. `None` means `name` is not an
    /// alias (or its action no longer resolves to a function).
    fn alias_call_target(
        &self,
        name: &str,
    ) -> Option<(AliasState, Option<Vec<FunctionSignature>>)> {
        match self.action_aliases.get(name)? {
            AliasState::Dynamic => Some((AliasState::Dynamic, None)),
            state @ AliasState::Bound {
                action,
                visible_signatures,
            } => {
                let symbol = self.current_scope.resolve(action)?;
                if let SymbolKind::Function { signatures } = &symbol.kind {
                    // The walk can visit more definitions than PASS 1
                    // registered top-level signatures (e.g. a definition
                    // nested in the branch this alias sits in). The prefix is
                    // then not a faithful description of the runtime value —
                    // defer to runtime dispatch rather than validate against
                    // a truncation.
                    if *visible_signatures > signatures.len() {
                        return Some((AliasState::Dynamic, None));
                    }
                    Some((
                        state.clone(),
                        Some(signatures[..*visible_signatures].to_vec()),
                    ))
                } else {
                    None
                }
            }
        }
    }

    /// What the alias call at (`name`, `line`, `column`) resolved to during
    /// semantic analysis, for the type checker's per-statement view.
    pub(crate) fn alias_call_resolution(
        &self,
        name: &str,
        line: usize,
        column: usize,
    ) -> Option<&AliasState> {
        self.alias_call_sites.get(&(name.to_string(), line, column))
    }

    /// Whether container `child` is `ancestor` or extends it (directly or
    /// transitively). The walk is depth-guarded so a cyclic `extends` chain
    /// cannot hang analysis.
    pub(crate) fn container_is_or_extends(&self, child: &str, ancestor: &str) -> bool {
        let mut current = Some(child.to_string());
        let mut depth = 0;
        while let Some(name) = current {
            if name == ancestor {
                return true;
            }
            depth += 1;
            if depth > 64 {
                return false;
            }
            current = self
                .get_container(&name)
                .and_then(|info| info.extends.clone());
        }
        false
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

    /// The candidate variable name referenced by a `write line|chunk` value: a
    /// bare `Variable`, or the callee of a `<field> of <object>` call.
    fn stream_write_candidate_name(expr: &Expression) -> Option<&str> {
        match expr {
            Expression::Variable(name, ..) => Some(name),
            Expression::FunctionCall { function, .. } => match &**function {
                Expression::Variable(name, ..) => Some(name),
                _ => None,
            },
            _ => None,
        }
    }

    /// Whether a bare name resolves to something known (an action parameter, the
    /// `count` loop variable, a builtin, an in-scope binding, or a container
    /// property) — i.e. it would NOT be reported as an undefined variable. Used
    /// to decide the ambiguous `write line|chunk` case without emitting.
    fn name_is_defined(&self, name: &str) -> bool {
        if self.action_parameters.contains(name)
            || name == "count"
            || Self::is_builtin_function(name)
            || self.current_scope.resolve(name).is_some()
        {
            return true;
        }
        if let Some(container_name) = &self.current_container {
            return self.is_container_property(container_name, name);
        }
        false
    }

    fn analyze_expression(&mut self, expression: &Expression) {
        // Recursive front-end checkpoint for expressions. `analyze_statement`
        // polls per statement, but one statement can hold an arbitrarily large
        // expression tree (a huge list/map literal, a long operator chain), so
        // poll here too. The `budget_error` latch records the breach once and
        // short-circuits the rest of the traversal.
        if self.budget_error.is_some() {
            return;
        }
        if let Some(budget) = crate::exec::budget::ExecutionBudget::current()
            && let Err(exceeded) = budget.charge_operation(!budget.is_deadline_exempt())
        {
            self.errors
                .push(SemanticError::new(exceeded.message(), 0, 0));
            self.budget_error = Some(exceeded);
            return;
        }
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
                        // A bare unresolved name may be a zero-argument action
                        // exposed by an `include from` file and referenced by
                        // its bare name (e.g. `store x as greet`), which lowers
                        // to `Expression::Variable` with no call node and so
                        // never reaches the `of`/`call` forms' include-aware
                        // relaxation. Route it through the same helper so all
                        // three call forms of the #548 -> #580 family behave
                        // consistently under `include from` (issue #592). With
                        // no includes present the helper emits nothing and
                        // returns false, so a genuine typo stays fatal.
                        if !self.warn_undefined_callee_if_includes(name, *line, *column) {
                            self.report_undefined_name(
                                format!("Variable '{name}' is not defined"),
                                *line,
                                *column,
                            );
                        }
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
                    // Clone the resolved signature list (plus the symbol's
                    // definition position, needed by the non-function arm) up
                    // front so the symbol borrow doesn't overlap the mutable
                    // calls below.
                    type ResolvedCallee = (Option<Vec<FunctionSignature>>, usize, usize);
                    let resolved_signatures: Option<ResolvedCallee> =
                        self.current_scope.resolve(name).map(|symbol| {
                            let signatures =
                                if let SymbolKind::Function { signatures } = &symbol.kind {
                                    Some(signatures.clone())
                                } else {
                                    None
                                };
                            (signatures, symbol.line, symbol.column)
                        });
                    if let Some((function_signatures, symbol_line, symbol_column)) =
                        resolved_signatures
                    {
                        match function_signatures {
                            Some(signatures) => {
                                for arg in arguments {
                                    self.analyze_expression(&arg.value);
                                }

                                if Self::is_builtin_function(name) {
                                    // Builtins keep the historical arity-only
                                    // check; their full validation lives in the
                                    // stdlib layer.
                                    if let Some(first_signature) = signatures.first()
                                        && arguments.len() != first_signature.parameters.len()
                                        && !signatures
                                            .iter()
                                            .any(|sig| sig.parameters.len() == arguments.len())
                                    {
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
                                } else {
                                    self.check_overloaded_call(
                                        name,
                                        &signatures,
                                        arguments,
                                        true,
                                        *line,
                                        *column,
                                    );
                                }
                            }
                            None => {
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
                                    && symbol_line == 0
                                    && symbol_column == 0;
                                // An action parameter or handler-property name
                                // (e.g. `body of msg` inside a websocket handler)
                                // is a relaxed `of`-form access even when an outer
                                // scope also defines a non-function symbol of the
                                // same name; the property read must win over it.
                                // A stored action reference (`store h as f`)
                                // is callable with the aliased action's
                                // overload set.
                                let alias_target = self.alias_call_target(name);
                                if let Some((state, signatures)) = alias_target {
                                    self.alias_call_sites
                                        .insert((name.clone(), *line, *column), state);
                                    for arg in arguments {
                                        self.analyze_expression(&arg.value);
                                    }
                                    // A Dynamic alias defers wholly to runtime
                                    // dispatch; only bound snapshots validate.
                                    if let Some(signatures) = signatures {
                                        self.check_overloaded_call(
                                            name,
                                            &signatures,
                                            arguments,
                                            true,
                                            *line,
                                            *column,
                                        );
                                    }
                                } else if is_injected_builtin
                                    || self.action_parameters.contains(name)
                                {
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

                // Validate user-defined action exists and has a matching
                // signature (overload resolution across all registered
                // signatures). Clone the signature list up front so the
                // symbol borrow doesn't overlap the mutable calls below.
                let resolved_signatures: Option<Option<Vec<FunctionSignature>>> =
                    self.current_scope.resolve(name).map(|symbol| {
                        if let SymbolKind::Function { signatures } = &symbol.kind {
                            Some(signatures.clone())
                        } else {
                            None
                        }
                    });
                if let Some(function_signatures) = resolved_signatures {
                    match function_signatures {
                        Some(signatures) => {
                            self.check_overloaded_call(
                                name,
                                &signatures,
                                arguments,
                                false,
                                *line,
                                *column,
                            );
                        }
                        None => {
                            // A stored action reference (`store h as f`) is
                            // callable with the aliased action's snapshot.
                            if let Some((state, signatures)) = self.alias_call_target(name) {
                                self.alias_call_sites
                                    .insert((name.clone(), *line, *column), state);
                                if let Some(signatures) = signatures {
                                    self.check_overloaded_call(
                                        name,
                                        &signatures,
                                        arguments,
                                        false,
                                        *line,
                                        *column,
                                    );
                                }
                            } else {
                                // Symbol exists but is not a function/action
                                self.errors.push(SemanticError::new(
                                    format!("'{name}' is not an action"),
                                    *line,
                                    *column,
                                ));
                            }
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
    fn test_concurrent_main_loop_requires_wait_for_request_first() {
        use crate::lexer::lex_wfl_with_positions;
        use crate::parser::Parser;

        fn analyze_src(src: &str) -> Result<(), Vec<SemanticError>> {
            let program = Parser::new(&lex_wfl_with_positions(src))
                .parse()
                .expect("parse");
            Analyzer::new().analyze(&program)
        }

        // A concurrent loop whose body does NOT begin with `wait for request`
        // would run its opening statements once per handler slot before any
        // request arrives. Static analysis must reject it with an actionable error
        // (and that error must survive the handler-body error demotion).
        let bad = "listen on port 8080 as srv\n\
                   main loop concurrently:\n    \
                   store tick as 1\n    \
                   wait for request comes in on srv as req\n    \
                   respond to req with \"ok\"\nend loop";
        let errs =
            analyze_src(bad).expect_err("setup-before-wait concurrent loop must be rejected");
        assert!(
            errs.iter()
                .any(|e| e.message.contains("must begin with `wait for request")),
            "expected the concurrent-loop ordering error, got: {errs:?}"
        );

        // The identical body under a plain serial `main loop` is fine — a serial
        // loop runs one iteration at a time, so there is no speculative fan-out.
        let serial = "listen on port 8080 as srv\n\
                      main loop:\n    \
                      store tick as 1\n    \
                      wait for request comes in on srv as req\n    \
                      respond to req with \"ok\"\nend loop";
        assert!(
            analyze_src(serial).is_ok(),
            "serial main loop must not require wait-for-request first"
        );

        // A concurrent loop that DOES begin with `wait for request` is accepted.
        let good = "listen on port 8080 as srv\n\
                    main loop concurrently:\n    \
                    wait for request comes in on srv as req\n    \
                    respond to req with \"ok\"\nend loop";
        assert!(
            analyze_src(good).is_ok(),
            "concurrent loop starting with wait-for-request must analyze cleanly"
        );
    }

    // Issue #592: a bare unresolved name in a program that uses `include from`
    // may be a zero-argument action exposed by the included file at runtime, so
    // the `Expression::Variable` arm must relax to a non-fatal warning (like the
    // `of`/`call` forms) instead of aborting with a fatal undefined-name error.
    #[test]
    fn test_bare_undefined_name_relaxed_with_includes_issue_592() {
        let program = Program {
            statements: vec![
                Statement::IncludeStatement {
                    path: Expression::Literal(Literal::String(Arc::from("mod.wfl")), 1, 1),
                    line: 1,
                    column: 1,
                },
                Statement::VariableDeclaration {
                    name: "x".to_string(),
                    value: Expression::Variable("greet".to_string(), 2, 12),
                    is_constant: false,
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut analyzer = Analyzer::new();
        let result = analyzer.analyze(&program);
        assert!(
            result.is_ok(),
            "bare include-exposed name must not be a fatal error (#592): {:?}",
            analyzer.get_errors()
        );
        assert!(
            analyzer
                .get_warnings()
                .iter()
                .any(|w| w.message.contains("Undefined action 'greet'")),
            "should record a non-fatal 'Undefined action' warning (#592): {:?}",
            analyzer.get_warnings()
        );
    }

    // Issue #592 guardrail: the SAME bare reference WITHOUT any `include from`
    // is a genuine typo and must stay fatal — the relaxation must not
    // over-broaden into silencing real undefined-name errors.
    #[test]
    fn test_bare_undefined_name_without_includes_stays_fatal_issue_592() {
        let program = Program {
            statements: vec![Statement::VariableDeclaration {
                name: "x".to_string(),
                value: Expression::Variable("greet".to_string(), 1, 12),
                is_constant: false,
                line: 1,
                column: 1,
            }],
        };

        let mut analyzer = Analyzer::new();
        let result = analyzer.analyze(&program);
        assert!(
            result.is_err(),
            "a bare undefined name without includes must stay fatal (#592 guardrail)"
        );
        assert!(
            analyzer
                .get_errors()
                .iter()
                .any(|e| e.message.contains("Variable 'greet' is not defined")),
            "should report the fatal undefined-variable error (#592 guardrail): {:?}",
            analyzer.get_errors()
        );
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
