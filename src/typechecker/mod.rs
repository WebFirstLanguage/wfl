use crate::analyzer::{Analyzer, Symbol, SymbolBindingKey, SymbolKind};
use crate::builtins;
use crate::parser::ast::{
    Expression, Literal, Operator, Parameter, PatternExpression, Program, Statement, Type,
    UnaryOperator, WsHandlerEvent,
};
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone)]
pub struct TypeError {
    pub message: String,
    pub expected: Option<Type>,
    pub found: Option<Type>,
    pub line: usize,
    pub column: usize,
}

impl TypeError {
    pub fn new(
        message: String,
        expected: Option<Type>,
        found: Option<Type>,
        line: usize,
        column: usize,
    ) -> Self {
        TypeError {
            message,
            expected,
            found,
            line,
            column,
        }
    }
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut message = format!(
            "Type error at line {}, column {}: {}",
            self.line, self.column, self.message
        );

        if let Some(expected) = &self.expected
            && let Some(found) = &self.found
        {
            message.push_str(&format!(" - Expected {expected} but found {found}"));
        }

        write!(f, "{message}")
    }
}

/// The outcome of [`TypeChecker::check_types`] when it does not succeed.
///
/// A shared-budget breach (deadline, cancellation, operation/depth/byte
/// ceiling) is a **fatal** event: the run must stop, and it must never be
/// mistaken for — or silently downgraded to — an ordinary type diagnostic.
/// Encoding it as a distinct variant forces every caller to distinguish the two
/// at the type level, instead of relying on an optional side channel a caller
/// can forget to consult. This is why `check_types` returns this enum rather
/// than a bare `Vec<TypeError>`.
#[derive(Debug, Clone)]
pub enum TypeCheckError {
    /// The shared run budget was exhausted during analysis or type checking.
    /// Fatal — callers must abort the run (do not execute the program).
    Budget(crate::exec::budget::BudgetExceeded),
    /// Ordinary type diagnostics. Callers may report these and, in the
    /// `include from` path, continue (matching the main-file pipeline).
    Types(Vec<TypeError>),
}

impl TypeCheckError {
    /// Render this failure as type diagnostics: the diagnostics themselves, or a
    /// budget breach rendered as a single diagnostic. Convenience for callers
    /// (and tests) that only need to display the failure.
    pub fn into_diagnostics(self) -> Vec<TypeError> {
        match self {
            TypeCheckError::Types(errors) => errors,
            TypeCheckError::Budget(breach) => {
                vec![TypeError::new(breach.message(), None, None, 0, 0)]
            }
        }
    }

    /// The budget breach, if this failure was one.
    pub fn budget_breach(&self) -> Option<&crate::exec::budget::BudgetExceeded> {
        match self {
            TypeCheckError::Budget(breach) => Some(breach),
            TypeCheckError::Types(_) => None,
        }
    }
}

impl std::error::Error for TypeError {}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Type::Text => write!(f, "Text"),
            Type::Number => write!(f, "Number"),
            Type::Boolean => write!(f, "Boolean"),
            Type::Nothing => write!(f, "Nothing"),
            Type::Pattern => write!(f, "Pattern"),
            Type::Date => write!(f, "Date"),
            Type::Time => write!(f, "Time"),
            Type::DateTime => write!(f, "DateTime"),
            Type::Binary => write!(f, "Binary"),
            Type::Custom(name) => write!(f, "{name}"),
            Type::List(item_type) => write!(f, "List of {item_type}"),
            Type::Map(key_type, value_type) => write!(f, "Map from {key_type} to {value_type}"),
            Type::Function {
                parameters,
                return_type,
            } => {
                write!(f, "Function(")?;
                for (i, param) in parameters.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{param}")?;
                }
                write!(f, ") -> {return_type}")
            }
            Type::Unknown => write!(f, "Unknown"),
            Type::Error => write!(f, "Error"),
            Type::Async(t) => write!(f, "Async<{t}>"),
            Type::Any => write!(f, "Any"),
            Type::Optional(inner) => write!(f, "{inner} or Nothing"),
            Type::Container(name) => write!(f, "Container<{name}>"),
            Type::ContainerInstance(name) => write!(f, "Instance<{name}>"),
            Type::Interface(name) => write!(f, "Interface<{name}>"),
        }
    }
}

#[derive(Clone)]
enum ListMutationEffect {
    Join(Type),
    Replace(Type),
    Escape,
}

/// Deepest `index_depth` the list-alias relation will track.
///
/// The relation's key space is `live bindings × index_depth`. Leaving
/// `index_depth` unbounded left that space infinite, and because
/// [`TypeChecker::add_structural_list_alias`] and
/// [`TypeChecker::list_alias_members_for_path`] both *synthesize* paths at a
/// translated depth, a binding that transitively aliases itself at a different
/// depth drove the depth upward forever with no fixpoint — issue #654, where a
/// three-line program hung the checker with no diagnostic.
///
/// Bounding the depth makes the key space finite, so the relation is guaranteed
/// to stabilize. Paths past the bound are *dropped* rather than clamped: a
/// clamped path would apply a mutation effect at the wrong nesting level, where
/// [`ListMutationEffect::Replace`] could overwrite a type that is not the one
/// the program named. Dropping instead degrades to "not tracked this deep",
/// which is what the checker did before aggregate paths were tracked at all.
///
/// The bound has to be a constant rather than the aggregate's static nesting,
/// because `Type::Any` makes [`TypeChecker::type_at_alias_path`] answer at
/// every depth — a gradually-typed program supplies no structural ceiling.
///
/// Eight is chosen against measurement, not intuition: instrumenting
/// [`TypeChecker::add_list_may_alias_edge`] over the whole `TestPrograms/`
/// corpus — including the comprehensive container, pattern, list, and web-server
/// programs — puts the deepest path any acyclic program reaches at **2**. Eight
/// leaves four times that headroom while keeping the bound tight enough to
/// matter: a saturated cyclic relation costs roughly the fifth power of this
/// constant, so 16 left a pathological twelve-line program taking 14s where 8
/// takes well under a second. Real structure never approaches either value;
/// only cyclic translation does.
const MAX_LIST_ALIAS_INDEX_DEPTH: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ListAliasPath {
    binding: SymbolBindingKey,
    index_depth: usize,
}

impl ListAliasPath {
    /// True when this path is deeper than the relation tracks. Such a path is
    /// never stored and never reported; see [`MAX_LIST_ALIAS_INDEX_DEPTH`].
    fn exceeds_tracked_depth(&self) -> bool {
        self.index_depth > MAX_LIST_ALIAS_INDEX_DEPTH
    }
}

#[derive(Debug, Clone)]
struct RecordedReturn {
    return_type: Type,
    line: usize,
    column: usize,
    has_value: bool,
    list_sources: Vec<(usize, HashSet<ListAliasPath>)>,
}

#[derive(Clone)]
struct DeferredSummarySnapshot {
    list_effects: Option<HashSet<ListAliasPath>>,
    binding_effects: Option<HashMap<SymbolBindingKey, Type>>,
    returns: Option<Vec<RecordedReturn>>,
    dependencies: Option<HashSet<ActionSummaryKey>>,
}

type ActionSummaryKey = (String, usize);
type SharedListReturnProvenance = HashMap<usize, HashSet<ListAliasPath>>;
type SymbolTypeSnapshot = Vec<HashMap<String, Option<Type>>>;
type BindingTypeSnapshot = HashMap<SymbolBindingKey, Option<Type>>;
type ListAliasSnapshot = HashMap<ListAliasPath, HashSet<ListAliasPath>>;
type BlockFlowResult = (bool, Type);

#[derive(Clone, Default)]
struct TryFlowAccumulator {
    binding_types: BindingTypeSnapshot,
    list_aliases: ListAliasSnapshot,
}

pub struct TypeChecker {
    analyzer: Analyzer,
    /// Canonical stdlib signatures are kept separate from program symbols so
    /// constructor choice (`new` versus production's `with_analyzer`) and user
    /// declarations cannot silently remove or overwrite builtin contracts.
    builtin_contracts: Analyzer,
    errors: Vec<TypeError>,
    analyzer_already_run: bool,
    current_container: Option<String>,
    current_method_is_static: Option<bool>,
    /// For each property visible to the active method, records the lexical
    /// binding (if any) that existed outside the method. Comparing the live
    /// binding key with this baseline distinguishes a method parameter/local
    /// from a same-named true outer binding even inside nested try/loop scopes.
    current_method_outer_property_bindings: Option<HashMap<String, Option<SymbolBindingKey>>>,
    /// True when the program contains `include from` statements. Included files
    /// expose their actions dynamically at runtime, so undefined-action errors
    /// are suppressed to match the analyzer (see issue #548).
    has_includes: bool,
    /// A shared-budget breach hit during type checking. Kept **separate** from
    /// `errors` because callers (the CLI, `include`) print `TypeError`s as
    /// non-fatal warnings and continue — which would erase a real
    /// deadline/cancellation/resource breach. This is an internal latch:
    /// `check_types` surfaces it to callers as the fatal
    /// [`TypeCheckError::Budget`] variant so the distinction is enforced by the
    /// type system rather than an optional side channel.
    budget_error: Option<crate::exec::budget::BudgetExceeded>,
    /// Inferred return type of each overload of an action, keyed by
    /// `(action name, index into the analyzer symbol's signatures)`. The
    /// symbol's `symbol_type` can only hold one `Type::Function` (the last
    /// definition checked), so overloaded call sites resolve their return
    /// type through this table instead.
    overload_returns: HashMap<(String, usize), Type>,
    /// True only while checking a runtime-reachable later iteration of a loop
    /// whose environment persists across iterations. A constant declaration
    /// succeeds on the first iteration but would be a runtime redeclaration on
    /// every reachable backedge.
    checking_persistent_loop_backedge: bool,
    /// Flow-sensitive may-alias groups for runtime list allocations. Bindings
    /// are keyed by lexical identity rather than name so sibling/local scopes
    /// can safely reuse names. Reassignment detaches a binding, while
    /// control-flow joins union every alias relation reachable at that point.
    list_alias_groups: HashMap<ListAliasPath, HashSet<ListAliasPath>>,
    /// List paths a user action may mutate or expose from its captured
    /// environment. Definition-time checking records these effects, then
    /// restores the outer state; call sites apply the summary.
    user_action_list_effects: HashMap<ActionSummaryKey, HashSet<ListAliasPath>>,
    /// Scalar bindings that a user action may reassign. The value is the join
    /// of every assigned type on a runtime-reachable path. Call sites join this
    /// with the current flow type, preserving soundness when a closure mutates
    /// an outer Optional value inside a narrowed branch.
    user_action_binding_effects: HashMap<ActionSummaryKey, HashMap<SymbolBindingKey, Type>>,
    user_action_shared_list_returns: HashMap<ActionSummaryKey, SharedListReturnProvenance>,
    user_action_dependencies: HashMap<ActionSummaryKey, HashSet<ActionSummaryKey>>,
    deferred_action_key_stack: Vec<ActionSummaryKey>,
    deferred_list_effect_stack: Vec<HashSet<ListAliasPath>>,
    deferred_binding_effect_stack: Vec<HashMap<SymbolBindingKey, Type>>,
    deferred_return_type_stack: Vec<Vec<RecordedReturn>>,
    /// Streaming joins of every reachable intermediate state in each active
    /// try body. Retaining one accumulator per nesting level avoids keeping a
    /// full symbol/alias snapshot for every statement prefix.
    try_flow_states: Vec<TryFlowAccumulator>,
    try_flow_capture_suspended: usize,
    /// The runtime value produced by the statement currently being checked.
    /// Most statements produce Nothing; expression and control-flow statements
    /// replace this slot. The wrapper saves/restores it across recursion.
    current_statement_completion: Type,
    /// Original Optional types for bindings narrowed by active guards. Opaque
    /// user-code calls restore these instead of silently retaining a stale
    /// present-value refinement.
    optional_refinement_origins: HashMap<SymbolBindingKey, Type>,
    has_websocket_handlers: bool,
    /// Bindings whose current list value is statically known to contain at
    /// least one element. This small cardinality fact lets a for-each over a
    /// non-empty literal retain its guaranteed first-iteration effects.
    definitely_nonempty_lists: HashSet<SymbolBindingKey>,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    fn builtin_contract_analyzer() -> Analyzer {
        let mut analyzer = Analyzer::new();
        crate::stdlib::typechecker::register_stdlib_types(&mut analyzer);
        analyzer
    }

    pub fn new() -> Self {
        TypeChecker {
            analyzer: Analyzer::new(),
            builtin_contracts: Self::builtin_contract_analyzer(),
            errors: Vec::new(),
            analyzer_already_run: false,
            current_container: None,
            current_method_is_static: None,
            current_method_outer_property_bindings: None,
            has_includes: false,
            budget_error: None,
            overload_returns: HashMap::new(),
            checking_persistent_loop_backedge: false,
            list_alias_groups: HashMap::new(),
            user_action_list_effects: HashMap::new(),
            user_action_binding_effects: HashMap::new(),
            user_action_shared_list_returns: HashMap::new(),
            user_action_dependencies: HashMap::new(),
            deferred_action_key_stack: Vec::new(),
            deferred_list_effect_stack: Vec::new(),
            deferred_binding_effect_stack: Vec::new(),
            deferred_return_type_stack: Vec::new(),
            try_flow_states: Vec::new(),
            try_flow_capture_suspended: 0,
            current_statement_completion: Type::Nothing,
            optional_refinement_origins: HashMap::new(),
            has_websocket_handlers: false,
            definitely_nonempty_lists: HashSet::new(),
        }
    }

    /// Create a new TypeChecker with an existing Analyzer
    /// This allows sharing action parameters between the analyzer and type checker
    pub fn with_analyzer(analyzer: Analyzer) -> Self {
        TypeChecker {
            analyzer,
            builtin_contracts: Self::builtin_contract_analyzer(),
            errors: Vec::new(),
            analyzer_already_run: true, // Analyzer has already been run when passed in
            current_container: None,
            current_method_is_static: None,
            current_method_outer_property_bindings: None,
            has_includes: false,
            budget_error: None,
            overload_returns: HashMap::new(),
            checking_persistent_loop_backedge: false,
            list_alias_groups: HashMap::new(),
            user_action_list_effects: HashMap::new(),
            user_action_binding_effects: HashMap::new(),
            user_action_shared_list_returns: HashMap::new(),
            user_action_dependencies: HashMap::new(),
            deferred_action_key_stack: Vec::new(),
            deferred_list_effect_stack: Vec::new(),
            deferred_binding_effect_stack: Vec::new(),
            deferred_return_type_stack: Vec::new(),
            try_flow_states: Vec::new(),
            try_flow_capture_suspended: 0,
            current_statement_completion: Type::Nothing,
            optional_refinement_origins: HashMap::new(),
            has_websocket_handlers: false,
            definitely_nonempty_lists: HashSet::new(),
        }
    }

    /// Get the action parameters from the analyzer
    pub fn get_action_parameters(&self) -> &std::collections::HashSet<String> {
        self.analyzer.get_action_parameters()
    }

    fn join_type_snapshots(
        states: &[Vec<HashMap<String, Option<Type>>>],
    ) -> Vec<HashMap<String, Option<Type>>> {
        let Some(first) = states.first() else {
            return Vec::new();
        };
        let mut joined = first.clone();

        // Analyzer pass 1 normally pre-registers branch-visible symbols, but
        // include names from every state so direct AST users receive the same
        // conservative result.
        for state in states.iter().skip(1) {
            if joined.len() < state.len() {
                joined.resize_with(state.len(), HashMap::new);
            }
            for (layer_index, layer) in state.iter().enumerate() {
                for name in layer.keys() {
                    joined[layer_index].entry(name.clone()).or_insert(None);
                }
            }
        }

        for (layer_index, joined_layer) in joined.iter_mut().enumerate() {
            let names: Vec<String> = joined_layer.keys().cloned().collect();
            for name in names {
                let values: Vec<Option<Type>> = states
                    .iter()
                    .map(|state| {
                        state
                            .get(layer_index)
                            .and_then(|layer| layer.get(&name))
                            .cloned()
                            .unwrap_or(None)
                    })
                    .collect();
                let first_value = values.first().cloned().unwrap_or(None);
                let merged = if values.iter().all(|value| value == &first_value) {
                    first_value
                } else if values.iter().any(Option::is_none) {
                    Some(Type::Unknown)
                } else {
                    values
                        .into_iter()
                        .flatten()
                        .reduce(Self::join_inferred_types)
                };
                joined_layer.insert(name, merged);
            }
        }

        joined
    }

    /// Join values stored in a heterogeneous collection. `Any` is a real
    /// known-dynamic type and therefore dominates; `Unknown` means inference
    /// is incomplete and must never be narrowed merely by visiting a later
    /// concrete value.
    fn join_collection_value_type(current: Option<Type>, next: Type) -> Type {
        let Some(current) = current else {
            return next;
        };
        Self::join_inferred_types(current, next)
    }

    fn optionalize(ty: Type) -> Type {
        match ty {
            Type::Optional(_) | Type::Nothing => ty,
            other => Type::Optional(Box::new(other)),
        }
    }

    /// Join two runtime-reachable types without discarding structure they are
    /// guaranteed to share. A list remains a list when only its element types
    /// differ; likewise for maps and async values. `Unknown` remains the
    /// conservative "insufficient evidence" state, while `Any` represents a
    /// genuine union at the exact position where types diverge.
    fn join_inferred_types(left: Type, right: Type) -> Type {
        if left == right {
            return left;
        }
        match (left, right) {
            (Type::Error, _) | (_, Type::Error) => Type::Error,
            (Type::Optional(left), Type::Optional(right)) => {
                Self::optionalize(Self::join_inferred_types(*left, *right))
            }
            (Type::Optional(inner), Type::Nothing) | (Type::Nothing, Type::Optional(inner)) => {
                Type::Optional(inner)
            }
            (Type::Optional(inner), other) | (other, Type::Optional(inner)) => {
                Self::optionalize(Self::join_inferred_types(*inner, other))
            }
            (Type::Unknown, _) | (_, Type::Unknown) => Type::Unknown,
            (Type::Any, _) | (_, Type::Any) => Type::Any,
            (Type::Nothing, other) | (other, Type::Nothing) => Self::optionalize(other),
            (Type::List(left), Type::List(right)) => {
                Type::List(Box::new(Self::join_inferred_types(*left, *right)))
            }
            (Type::Map(left_key, left_value), Type::Map(right_key, right_value)) => Type::Map(
                Box::new(Self::join_inferred_types(*left_key, *right_key)),
                Box::new(Self::join_inferred_types(*left_value, *right_value)),
            ),
            (Type::Async(left), Type::Async(right)) => {
                Type::Async(Box::new(Self::join_inferred_types(*left, *right)))
            }
            _ => Type::Any,
        }
    }

    fn union_list_alias_bindings_in(
        groups: &mut HashMap<ListAliasPath, HashSet<ListAliasPath>>,
        left: ListAliasPath,
        right: ListAliasPath,
    ) {
        if left.exceeds_tracked_depth() || right.exceeds_tracked_depth() {
            return;
        }
        let left_neighbors = groups
            .get(&left)
            .cloned()
            .unwrap_or_else(|| HashSet::from([left.clone()]));
        let right_neighbors = groups
            .get(&right)
            .cloned()
            .unwrap_or_else(|| HashSet::from([right.clone()]));

        for member in &left_neighbors {
            groups
                .entry(member.clone())
                .or_insert_with(|| HashSet::from([member.clone()]))
                .insert(right.clone());
        }
        for member in &right_neighbors {
            groups
                .entry(member.clone())
                .or_insert_with(|| HashSet::from([member.clone()]))
                .insert(left.clone());
        }
        groups.entry(left).or_default().extend(right_neighbors);
        groups.entry(right).or_default().extend(left_neighbors);
    }

    fn union_list_alias_bindings(&mut self, left: ListAliasPath, right: ListAliasPath) {
        Self::union_list_alias_bindings_in(&mut self.list_alias_groups, left, right);
    }

    /// Sole write path into [`Self::list_alias_groups`] for a new relation.
    /// Rejecting over-deep endpoints here is what keeps the relation's key
    /// space finite, and therefore what makes it stabilize (issue #654).
    fn add_list_may_alias_edge(&mut self, left: ListAliasPath, right: ListAliasPath) {
        if left.exceeds_tracked_depth() || right.exceeds_tracked_depth() {
            return;
        }
        self.list_alias_groups
            .entry(left.clone())
            .or_insert_with(|| HashSet::from([left.clone()]))
            .insert(right.clone());
        self.list_alias_groups
            .entry(right.clone())
            .or_insert_with(|| HashSet::from([right.clone()]))
            .insert(left);
    }

    /// Record an alias reached through an aggregate path and materialize any
    /// already-known descendants at the translated target depth. The alias
    /// graph deliberately is not transitively closed because a branch join can
    /// mean “A aliases B or C” without B ever aliasing C. Materializing only
    /// structural descendants gives deep aggregate paths their real Rc
    /// provenance without inventing that disjunctive B/C edge.
    fn add_structural_list_alias(&mut self, source: ListAliasPath, target: ListAliasPath) {
        let mut edges = vec![(source.clone(), target.clone())];
        for alias in self.list_alias_members_for_path(&source) {
            if alias != source {
                edges.push((alias, target.clone()));
            }
        }

        let descendants = self
            .list_alias_groups
            .keys()
            .filter(|path| path.binding == source.binding && path.index_depth > source.index_depth)
            .cloned()
            .collect::<Vec<_>>();
        for descendant in descendants {
            let translated = ListAliasPath {
                binding: target.binding.clone(),
                index_depth: target.index_depth + descendant.index_depth - source.index_depth,
            };
            // Translating a self-referential relation upward is exactly how the
            // depth used to run away (issue #654). The edge would be rejected
            // anyway; skipping here also avoids expanding its members.
            if translated.exceeds_tracked_depth() {
                continue;
            }
            for alias in self.list_alias_members_for_path(&descendant) {
                if alias != descendant {
                    edges.push((alias, translated.clone()));
                }
            }
        }

        for (left, right) in edges {
            self.add_list_may_alias_edge(left, right);
        }
    }

    fn join_list_alias_snapshots(
        states: &[HashMap<ListAliasPath, HashSet<ListAliasPath>>],
    ) -> HashMap<ListAliasPath, HashSet<ListAliasPath>> {
        let mut joined = HashMap::new();
        for state in states {
            Self::merge_list_alias_snapshot_into(&mut joined, state);
        }
        joined
    }

    fn merge_list_alias_snapshot_into(joined: &mut ListAliasSnapshot, state: &ListAliasSnapshot) {
        for (left, members) in state {
            for right in members {
                joined
                    .entry(left.clone())
                    .or_insert_with(|| HashSet::from([left.clone()]))
                    .insert(right.clone());
                joined
                    .entry(right.clone())
                    .or_insert_with(|| HashSet::from([right.clone()]))
                    .insert(left.clone());
            }
        }
    }

    fn detach_list_alias_binding(&mut self, name: &str) {
        let Some(binding) = self.analyzer.get_symbol_binding_key(name) else {
            return;
        };
        let detached = self
            .list_alias_groups
            .keys()
            .filter(|path| path.binding == binding)
            .cloned()
            .collect::<HashSet<_>>();
        if detached.is_empty() {
            return;
        }
        for path in &detached {
            self.list_alias_groups.remove(path);
        }
        for group in self.list_alias_groups.values_mut() {
            group.retain(|member| !detached.contains(member));
        }
        self.list_alias_groups
            .retain(|_, members| members.len() > 1);
    }

    fn expression_is_definitely_nonempty_list(&self, expression: &Expression) -> bool {
        match expression {
            Expression::Literal(Literal::List(elements), ..) => !elements.is_empty(),
            Expression::Variable(name, ..) => self
                .analyzer
                .get_symbol_binding_key(name)
                .is_some_and(|binding| self.definitely_nonempty_lists.contains(&binding)),
            _ => false,
        }
    }

    fn update_binding_nonempty_fact(&mut self, name: &str, is_nonempty: bool) {
        let Some(binding) = self.analyzer.get_symbol_binding_key(name) else {
            return;
        };
        if is_nonempty {
            self.definitely_nonempty_lists.insert(binding);
        } else {
            self.definitely_nonempty_lists.remove(&binding);
        }
    }

    fn mark_list_target_nonempty(&mut self, target: &Expression) {
        let Some(path) = self.list_target_binding_path(target) else {
            return;
        };
        for member in self.list_alias_members_for_path(&path) {
            if member.index_depth == 0 {
                self.definitely_nonempty_lists.insert(member.binding);
            }
        }
    }

    fn snapshot_deferred_summary(&self) -> DeferredSummarySnapshot {
        DeferredSummarySnapshot {
            list_effects: self.deferred_list_effect_stack.last().cloned(),
            binding_effects: self.deferred_binding_effect_stack.last().cloned(),
            returns: self.deferred_return_type_stack.last().cloned(),
            dependencies: self.deferred_action_key_stack.last().map(|key| {
                self.user_action_dependencies
                    .get(key)
                    .cloned()
                    .unwrap_or_default()
            }),
        }
    }

    fn join_binding_effect(
        effects: &mut HashMap<SymbolBindingKey, Type>,
        binding: SymbolBindingKey,
        effect_type: Type,
    ) {
        effects
            .entry(binding)
            .and_modify(|current| {
                *current = Self::join_inferred_types(current.clone(), effect_type.clone());
            })
            .or_insert(effect_type);
    }

    fn restore_deferred_summary(&mut self, snapshot: DeferredSummarySnapshot) {
        if let (Some(current), Some(saved)) = (
            self.deferred_list_effect_stack.last_mut(),
            snapshot.list_effects,
        ) {
            *current = saved;
        }
        if let (Some(current), Some(saved)) = (
            self.deferred_binding_effect_stack.last_mut(),
            snapshot.binding_effects,
        ) {
            *current = saved;
        }
        if let (Some(current), Some(saved)) =
            (self.deferred_return_type_stack.last_mut(), snapshot.returns)
        {
            *current = saved;
        }
        if let (Some(key), Some(saved)) = (
            self.deferred_action_key_stack.last().cloned(),
            snapshot.dependencies,
        ) {
            self.user_action_dependencies.insert(key, saved);
        }
    }

    fn join_deferred_summaries(
        &mut self,
        entry: &DeferredSummarySnapshot,
        endpoints: &[DeferredSummarySnapshot],
    ) {
        if let Some(current) = self.deferred_list_effect_stack.last_mut() {
            *current = entry.list_effects.clone().unwrap_or_default();
            for endpoint in endpoints {
                if let Some(effects) = &endpoint.list_effects {
                    current.extend(effects.iter().cloned());
                }
            }
        }

        if let Some(current) = self.deferred_binding_effect_stack.last_mut() {
            *current = entry.binding_effects.clone().unwrap_or_default();
            for endpoint in endpoints {
                if let Some(effects) = &endpoint.binding_effects {
                    for (binding, effect_type) in effects {
                        Self::join_binding_effect(current, binding.clone(), effect_type.clone());
                    }
                }
            }
        }

        if let Some(current) = self.deferred_return_type_stack.last_mut() {
            *current = entry.returns.clone().unwrap_or_default();
            let entry_len = current.len();
            for endpoint in endpoints {
                if let Some(returns) = &endpoint.returns {
                    current.extend(returns.iter().skip(entry_len).cloned());
                }
            }
        }
        if let Some(key) = self.deferred_action_key_stack.last().cloned() {
            let mut joined = entry.dependencies.clone().unwrap_or_default();
            for endpoint in endpoints {
                if let Some(dependencies) = &endpoint.dependencies {
                    joined.extend(dependencies.iter().cloned());
                }
            }
            self.user_action_dependencies.insert(key, joined);
        }
    }

    /// Check every statement for diagnostics while retaining state only from
    /// the runtime-reachable prefix.
    fn check_statement_block_impl(&mut self, statements: &[Statement]) -> BlockFlowResult {
        let mut can_continue = true;
        let mut completion_type = Type::Nothing;
        let mut terminal_state = None;

        for (index, statement) in statements.iter().enumerate() {
            let was_reachable = can_continue;
            if !was_reachable {
                self.try_flow_capture_suspended += 1;
            }
            let statement_completion = self.check_statement_types(statement);
            if !was_reachable {
                self.try_flow_capture_suspended -= 1;
            }
            if self.budget_error.is_some() {
                return (false, Type::Error);
            }

            if was_reachable {
                completion_type = statement_completion;
            }
            if was_reachable && Self::statement_definitely_stops_current_block(statement) {
                can_continue = false;
                if index + 1 < statements.len() {
                    terminal_state = Some((
                        self.analyzer.snapshot_current_scope_symbols(),
                        self.analyzer.snapshot_symbol_types(),
                        self.list_alias_groups.clone(),
                        self.snapshot_deferred_summary(),
                        self.user_action_list_effects.clone(),
                        self.user_action_binding_effects.clone(),
                        self.user_action_shared_list_returns.clone(),
                        self.user_action_dependencies.clone(),
                        self.overload_returns.clone(),
                        self.optional_refinement_origins.clone(),
                        self.definitely_nonempty_lists.clone(),
                    ));
                }
            }
        }

        if let Some((
            scope_symbols,
            symbol_types,
            aliases,
            deferred,
            action_effects,
            action_binding_effects,
            shared_returns,
            dependencies,
            overload_returns,
            refinement_origins,
            nonempty_lists,
        )) = terminal_state
        {
            self.analyzer.restore_current_scope_symbols(scope_symbols);
            self.analyzer.restore_symbol_types(symbol_types);
            self.list_alias_groups = aliases;
            self.restore_deferred_summary(deferred);
            self.user_action_list_effects = action_effects;
            self.user_action_binding_effects = action_binding_effects;
            self.user_action_shared_list_returns = shared_returns;
            self.user_action_dependencies = dependencies;
            self.overload_returns = overload_returns;
            self.optional_refinement_origins = refinement_origins;
            self.definitely_nonempty_lists = nonempty_lists;
        }

        (can_continue, completion_type)
    }

    fn check_statement_block(&mut self, statements: &[Statement]) -> bool {
        self.check_statement_block_impl(statements).0
    }

    fn check_statement_block_with_completion(&mut self, statements: &[Statement]) -> (bool, Type) {
        self.check_statement_block_impl(statements)
    }

    fn capture_active_try_flow_state(&mut self) {
        if self.try_flow_states.is_empty() || self.try_flow_capture_suspended > 0 {
            return;
        }
        let types = self
            .analyzer
            .live_binding_types()
            .into_iter()
            .collect::<BindingTypeSnapshot>();
        let live_bindings = types.keys().cloned().collect::<HashSet<_>>();
        let alias_edges = self
            .list_alias_groups
            .values()
            .fold(0usize, |count, members| count.saturating_add(members.len()));
        let work_per_accumulator = types.len().saturating_add(alias_edges).max(1);
        let work_units = work_per_accumulator.saturating_mul(self.try_flow_states.len());
        if !self.charge_try_flow_work(work_units) {
            return;
        }

        for state in &mut self.try_flow_states {
            for (binding, accumulated_type) in &mut state.binding_types {
                let current_type = types.get(binding).cloned().unwrap_or(None);
                *accumulated_type = match (accumulated_type.take(), current_type) {
                    (Some(left), Some(right)) => Some(Self::join_inferred_types(left, right)),
                    _ => Some(Type::Unknown),
                };
            }
            Self::merge_list_alias_snapshot_into(&mut state.list_aliases, &self.list_alias_groups);
            state.list_aliases.retain(|path, members| {
                if !live_bindings.contains(&path.binding) {
                    return false;
                }
                members.retain(|member| live_bindings.contains(&member.binding));
                !members.is_empty()
            });
        }
    }

    fn charge_try_flow_work(&mut self, work_units: usize) -> bool {
        let Some(budget) = crate::exec::budget::ExecutionBudget::current() else {
            return true;
        };
        for _ in 0..work_units {
            if let Err(exceeded) = budget.charge_operation(!budget.is_deadline_exempt()) {
                self.errors
                    .push(TypeError::new(exceeded.message(), None, None, 0, 0));
                self.budget_error = Some(exceeded);
                return false;
            }
        }
        true
    }

    fn apply_try_binding_accumulator(
        &mut self,
        entry: SymbolTypeSnapshot,
        state: &BindingTypeSnapshot,
    ) -> SymbolTypeSnapshot {
        self.analyzer.restore_symbol_types(entry);
        for (binding, merged) in state {
            if let Some(symbol) = self.analyzer.get_symbol_by_binding_key_mut(binding) {
                symbol.symbol_type = merged.clone();
            }
        }
        self.analyzer.snapshot_symbol_types()
    }

    fn retain_live_alias_paths(&self, snapshot: &mut ListAliasSnapshot) {
        snapshot.retain(|path, members| {
            if !self.analyzer.binding_key_is_live(&path.binding) {
                return false;
            }
            members.retain(|member| self.analyzer.binding_key_is_live(&member.binding));
            !members.is_empty()
        });
    }

    fn prune_dead_list_alias_paths(&mut self) {
        let dead = self
            .list_alias_groups
            .keys()
            .filter(|path| !self.analyzer.binding_key_is_live(&path.binding))
            .cloned()
            .collect::<HashSet<_>>();
        if dead.is_empty() {
            return;
        }
        for path in &dead {
            self.list_alias_groups.remove(path);
        }
        for members in self.list_alias_groups.values_mut() {
            members.retain(|member| !dead.contains(member));
        }
        self.list_alias_groups
            .retain(|_, members| members.len() > 1);
    }

    fn record_direct_list_alias(
        &mut self,
        target_name: &str,
        value: &Expression,
        value_type: &Type,
    ) {
        if !Self::type_may_be_list(value_type) {
            return;
        }
        let target_is_list = self
            .analyzer
            .get_symbol(target_name)
            .and_then(|symbol| symbol.symbol_type.as_ref())
            .is_some_and(|ty| {
                matches!(
                    ty,
                    Type::List(_) | Type::Unknown | Type::Any | Type::Error
                ) || matches!(ty, Type::Optional(inner) if matches!(inner.as_ref(), Type::List(_)))
            });
        if !target_is_list {
            return;
        }

        if let Some(source) = self.list_target_binding_path(value)
            && self.alias_path_may_be_list(&source)
        {
            if let Some(target) = self.analyzer.get_symbol_binding_key(target_name) {
                self.union_list_alias_bindings(
                    source,
                    ListAliasPath {
                        binding: target,
                        index_depth: 0,
                    },
                );
            }
        } else if matches!(
            value,
            Expression::MemberAccess { .. }
                | Expression::PropertyAccess { .. }
                | Expression::MethodCall { .. }
        ) {
            // The target now holds a shared list reached through a structural
            // opaque path that the current AST/type model cannot name.
            self.apply_list_mutation_effect(value, ListMutationEffect::Escape);
            if let Some(target) = self.analyzer.get_symbol_mut(target_name)
                && let Some(current_type) = target.symbol_type.clone()
                && let Some(updated_type) =
                    Self::apply_effect_at_list_path(&current_type, 0, &ListMutationEffect::Escape)
            {
                target.symbol_type = Some(updated_type);
            }
        }
    }

    fn type_may_be_list(ty: &Type) -> bool {
        matches!(ty, Type::List(_) | Type::Unknown | Type::Any | Type::Error)
            || matches!(ty, Type::Optional(inner) if Self::type_may_be_list(inner))
    }

    fn type_at_alias_path(ty: &Type, index_depth: usize) -> Option<Type> {
        if index_depth == 0 {
            return Some(ty.clone());
        }
        match ty {
            Type::List(element) => Self::type_at_alias_path(element, index_depth - 1),
            Type::Map(_, value) => Self::type_at_alias_path(value, index_depth - 1),
            Type::Optional(inner) => Self::type_at_alias_path(inner, index_depth),
            Type::Unknown | Type::Any | Type::Error => Some(ty.clone()),
            _ => None,
        }
    }

    fn alias_path_may_be_list(&self, path: &ListAliasPath) -> bool {
        self.analyzer
            .get_symbol_by_binding_key(&path.binding)
            .and_then(|symbol| symbol.symbol_type.as_ref())
            .and_then(|ty| Self::type_at_alias_path(ty, path.index_depth))
            .is_some_and(|ty| Self::type_may_be_list(&ty))
    }

    fn alias_path_may_contain_list(&self, path: &ListAliasPath) -> bool {
        self.analyzer
            .get_symbol_by_binding_key(&path.binding)
            .and_then(|symbol| symbol.symbol_type.as_ref())
            .and_then(|ty| Self::type_at_alias_path(ty, path.index_depth))
            .is_some_and(|ty| Self::type_may_contain_list(&ty))
    }

    fn record_nested_list_alias_expression(
        &mut self,
        target_binding: &SymbolBindingKey,
        target_depth: usize,
        value: &Expression,
    ) {
        let mut captured = Vec::new();
        self.capture_nested_list_alias_sources(value, target_depth, &mut captured);
        for (captured_depth, sources) in captured {
            let target = ListAliasPath {
                binding: target_binding.clone(),
                index_depth: captured_depth,
            };
            for source in sources {
                self.add_structural_list_alias(source, target.clone());
            }
        }
    }

    fn record_nested_list_aliases(&mut self, target_name: &str, value: &Expression) {
        let Some(target_binding) = self.analyzer.get_symbol_binding_key(target_name) else {
            return;
        };
        self.record_nested_list_alias_expression(&target_binding, 0, value);
    }

    fn detach_list_alias_descendants(&mut self, target: &Expression) {
        let Some(target_path) = self.list_target_binding_path(target) else {
            return;
        };
        let roots = self.list_alias_members_for_path(&target_path);
        // Alias groups are may-alias sets after a control-flow join. Clearing
        // or filling through one member only replaces the descendants of the
        // allocation selected at runtime; deleting every member's descendants
        // would lose provenance for every unselected allocation. Without a
        // separate must-alias relation, retain those edges as a sound weak
        // update and strong-update only an unaliased root.
        if roots.len() > 1 {
            return;
        }
        let detached = self
            .list_alias_groups
            .keys()
            .filter(|path| {
                roots
                    .iter()
                    .any(|root| path.binding == root.binding && path.index_depth > root.index_depth)
            })
            .cloned()
            .collect::<HashSet<_>>();
        for path in &detached {
            self.list_alias_groups.remove(path);
        }
        for members in self.list_alias_groups.values_mut() {
            members.retain(|member| !detached.contains(member));
        }
        self.list_alias_groups
            .retain(|_, members| members.len() > 1);
    }

    fn record_list_insertion_aliases(&mut self, target: &Expression, value: &Expression) {
        let Some(target_path) = self.list_target_binding_path(target) else {
            return;
        };
        for root in self.list_alias_members_for_path(&target_path) {
            self.record_nested_list_alias_expression(&root.binding, root.index_depth + 1, value);
        }
    }

    fn capture_nested_list_alias_sources(
        &self,
        value: &Expression,
        target_depth: usize,
        out: &mut Vec<(usize, HashSet<ListAliasPath>)>,
    ) {
        if let Some(source_path) = self.list_target_binding_path(value)
            && self.alias_path_may_contain_list(&source_path)
        {
            self.capture_alias_path_and_descendants(&source_path, target_depth, out);
            return;
        }
        match value {
            Expression::Literal(Literal::List(values), ..) => {
                for item in values {
                    self.capture_nested_list_alias_sources(item, target_depth + 1, out);
                }
            }
            Expression::FunctionCall {
                function,
                arguments,
                line,
                column,
            } => {
                let Expression::Variable(name, ..) = function.as_ref() else {
                    return;
                };
                let action_keys = self.action_summary_keys_for_call(name, *line, *column);
                if !action_keys.is_empty() {
                    out.extend(
                        self.shared_list_return_sources_for_action_keys(&action_keys)
                            .into_iter()
                            .map(|(depth, sources)| (target_depth + depth, sources)),
                    );
                    return;
                }
                let builtin_name = self
                    .builtin_name_for_call(name, *line, *column)
                    .unwrap_or_default();
                let shape_result = matches!(builtin_name.as_str(), "slice" | "unique" | "concat");
                let element_result = matches!(
                    builtin_name.as_str(),
                    "find" | "random_from" | "pop" | "shift" | "remove_at" | "removeat"
                );
                if !shape_result && !element_result {
                    return;
                }
                for argument in arguments
                    .iter()
                    .take(if builtin_name == "concat" { 2 } else { 1 })
                {
                    if let Some(mut source_path) = self.list_target_binding_path(&argument.value) {
                        source_path.index_depth += 1;
                        self.capture_alias_path_and_descendants(
                            &source_path,
                            target_depth + usize::from(shape_result),
                            out,
                        );
                    }
                }
            }
            Expression::ActionCall {
                name, line, column, ..
            } => {
                let action_keys = self.action_summary_keys_for_call(name, *line, *column);
                out.extend(
                    self.shared_list_return_sources_for_action_keys(&action_keys)
                        .into_iter()
                        .map(|(depth, sources)| (target_depth + depth, sources)),
                );
            }
            Expression::Variable(name, line, column) => {
                let action_keys = self.action_summary_keys_for_call(name, *line, *column);
                let auto_calls = action_keys.iter().any(|(action, index)| {
                    self.action_signatures(action)
                        .and_then(|signatures| signatures.get(*index).cloned())
                        .is_some_and(|signature| signature.parameters.is_empty())
                });
                if auto_calls {
                    out.extend(
                        self.shared_list_return_sources_for_action_keys(&action_keys)
                            .into_iter()
                            .map(|(depth, sources)| (target_depth + depth, sources)),
                    );
                }
            }
            _ => {}
        }
    }

    fn capture_alias_path_and_descendants(
        &self,
        source_path: &ListAliasPath,
        target_depth: usize,
        out: &mut Vec<(usize, HashSet<ListAliasPath>)>,
    ) {
        let mut relative_depths = HashSet::new();
        if let Some(source_type) = self
            .analyzer
            .get_symbol_by_binding_key(&source_path.binding)
            .and_then(|symbol| symbol.symbol_type.as_ref())
            .and_then(|ty| Self::type_at_alias_path(ty, source_path.index_depth))
        {
            Self::collect_list_depths(&source_type, 0, &mut relative_depths);
        }
        relative_depths.extend(
            self.list_alias_groups
                .keys()
                .filter(|path| {
                    path.binding == source_path.binding
                        && path.index_depth >= source_path.index_depth
                })
                .map(|path| path.index_depth - source_path.index_depth),
        );

        let mut relative_depths = relative_depths.into_iter().collect::<Vec<_>>();
        relative_depths.sort_unstable();
        for relative_depth in relative_depths {
            let candidate = ListAliasPath {
                binding: source_path.binding.clone(),
                index_depth: source_path.index_depth + relative_depth,
            };
            if self.alias_path_may_be_list(&candidate) {
                out.push((
                    target_depth + relative_depth,
                    self.list_alias_members_for_path(&candidate),
                ));
            }
        }
    }

    fn action_summary_keys_for_call(
        &self,
        name: &str,
        line: usize,
        column: usize,
    ) -> Vec<ActionSummaryKey> {
        if let Some(resolution) = self.analyzer.alias_call_resolution(name, line, column) {
            return match resolution {
                crate::analyzer::AliasState::Bound {
                    action,
                    visible_signatures,
                } => (0..*visible_signatures)
                    .map(|index| (action.clone(), index))
                    .collect(),
                crate::analyzer::AliasState::Builtin { .. }
                | crate::analyzer::AliasState::Dynamic => Vec::new(),
            };
        }
        self.action_signatures(name)
            .map(|signatures| {
                (0..signatures.len())
                    .map(|index| (name.to_string(), index))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn builtin_name_for_call(&self, name: &str, line: usize, column: usize) -> Option<String> {
        match self.analyzer.alias_call_resolution(name, line, column) {
            Some(crate::analyzer::AliasState::Builtin { name }) => Some(name.clone()),
            Some(
                crate::analyzer::AliasState::Bound { .. } | crate::analyzer::AliasState::Dynamic,
            ) => None,
            None if builtins::is_implemented_builtin_function(name) => Some(name.to_string()),
            None => None,
        }
    }

    fn shared_list_return_sources_for_action_keys(
        &self,
        action_keys: &[ActionSummaryKey],
    ) -> Vec<(usize, HashSet<ListAliasPath>)> {
        let mut merged = SharedListReturnProvenance::new();
        for provenance in action_keys
            .iter()
            .filter_map(|key| self.user_action_shared_list_returns.get(key))
        {
            for (depth, sources) in provenance {
                merged
                    .entry(*depth)
                    .or_default()
                    .extend(sources.iter().cloned());
            }
        }
        merged.into_iter().collect()
    }

    fn capture_block_completion_list_sources(
        &self,
        statements: &[Statement],
        target_depth: usize,
        out: &mut Vec<(usize, HashSet<ListAliasPath>)>,
    ) {
        let Some(last) = statements.last() else {
            return;
        };
        self.capture_statement_completion_list_sources(last, target_depth, out);
    }

    fn capture_statement_completion_list_sources(
        &self,
        statement: &Statement,
        target_depth: usize,
        out: &mut Vec<(usize, HashSet<ListAliasPath>)>,
    ) {
        match statement {
            Statement::ExpressionStatement { expression, .. } => {
                self.capture_nested_list_alias_sources(expression, target_depth, out);
            }
            Statement::IfStatement {
                then_block,
                else_block,
                ..
            } => {
                self.capture_block_completion_list_sources(then_block, target_depth, out);
                if let Some(else_block) = else_block {
                    self.capture_block_completion_list_sources(else_block, target_depth, out);
                }
            }
            Statement::SingleLineIf {
                then_stmt,
                else_stmt,
                ..
            } => {
                self.capture_statement_completion_list_sources(then_stmt, target_depth, out);
                if let Some(else_stmt) = else_stmt {
                    self.capture_statement_completion_list_sources(else_stmt, target_depth, out);
                }
            }
            Statement::TryStatement {
                body,
                when_clauses,
                otherwise_block,
                ..
            } => {
                self.capture_block_completion_list_sources(body, target_depth, out);
                for clause in when_clauses {
                    self.capture_block_completion_list_sources(&clause.body, target_depth, out);
                }
                if let Some(otherwise_block) = otherwise_block {
                    self.capture_block_completion_list_sources(otherwise_block, target_depth, out);
                }
            }
            Statement::WaitForStatement { inner, .. } => {
                self.capture_statement_completion_list_sources(inner, target_depth, out);
            }
            // These loops return the final body value at runtime. A
            // repeat-until body executes at least once; the others may not,
            // but retaining a may-alias edge is conservative for every
            // continuation where a body value is produced.
            Statement::WhileLoop { body, .. }
            | Statement::RepeatWhileLoop { body, .. }
            | Statement::RepeatUntilLoop { body, .. }
            | Statement::ForeverLoop { body, .. }
            | Statement::MainLoop { body, .. } => {
                self.capture_block_completion_list_sources(body, target_depth, out);
            }
            _ => {}
        }
    }

    fn restore_captured_list_alias_sources(
        &mut self,
        target_name: &str,
        captured: Vec<(usize, HashSet<ListAliasPath>)>,
    ) {
        let Some(target_binding) = self.analyzer.get_symbol_binding_key(target_name) else {
            return;
        };
        for (target_depth, sources) in captured {
            let target = ListAliasPath {
                binding: target_binding.clone(),
                index_depth: target_depth,
            };
            for source in sources {
                // When the RHS mentions the assigned binding, that path names
                // the pre-assignment allocation. It has no independent live
                // binding after the strong update; any other captured alias
                // still links the retained allocation to the new target path.
                if source.binding != target_binding {
                    self.add_structural_list_alias(source, target.clone());
                }
            }
        }
    }

    fn list_target_binding_path(&self, expression: &Expression) -> Option<ListAliasPath> {
        match expression {
            Expression::Variable(name, ..) => {
                self.analyzer
                    .get_symbol_binding_key(name)
                    .map(|binding| ListAliasPath {
                        binding,
                        index_depth: 0,
                    })
            }
            Expression::IndexAccess { collection, .. } => {
                let mut path = self.list_target_binding_path(collection)?;
                path.index_depth += 1;
                Some(path)
            }
            _ => None,
        }
    }

    fn expression_root_binding_key(&self, expression: &Expression) -> Option<SymbolBindingKey> {
        match expression {
            Expression::Variable(name, ..) => self.analyzer.get_symbol_binding_key(name),
            Expression::IndexAccess { collection, .. } => {
                self.expression_root_binding_key(collection)
            }
            Expression::MemberAccess { object, .. }
            | Expression::PropertyAccess { object, .. }
            | Expression::MethodCall { object, .. } => self.expression_root_binding_key(object),
            _ => None,
        }
    }

    fn apply_effect_at_list_path(
        ty: &Type,
        index_depth: usize,
        effect: &ListMutationEffect,
    ) -> Option<Type> {
        if index_depth == 0 {
            return match ty {
                Type::List(element_type) => {
                    let next_element = match effect {
                        ListMutationEffect::Join(value_type) => Self::join_collection_value_type(
                            Some((**element_type).clone()),
                            value_type.clone(),
                        ),
                        ListMutationEffect::Replace(value_type) => value_type.clone(),
                        ListMutationEffect::Escape => Type::Any,
                    };
                    Some(Type::List(Box::new(next_element)))
                }
                Type::Optional(inner) => {
                    Self::apply_effect_at_list_path(inner, index_depth, effect)
                        .map(|updated| Type::Optional(Box::new(updated)))
                }
                Type::Unknown | Type::Any | Type::Error => Some(ty.clone()),
                _ => None,
            };
        }

        match ty {
            Type::List(element_type) => {
                Self::apply_effect_at_list_path(element_type, index_depth - 1, effect)
                    .map(|updated| Type::List(Box::new(updated)))
            }
            Type::Map(key_type, value_type) => {
                Self::apply_effect_at_list_path(value_type, index_depth - 1, effect)
                    .map(|updated| Type::Map(Box::new((**key_type).clone()), Box::new(updated)))
            }
            Type::Optional(inner) => Self::apply_effect_at_list_path(inner, index_depth, effect)
                .map(|updated| Type::Optional(Box::new(updated))),
            Type::Unknown | Type::Any | Type::Error => Some(ty.clone()),
            _ => None,
        }
    }

    fn list_alias_members_for_path(&self, path: &ListAliasPath) -> HashSet<ListAliasPath> {
        let mut members = self
            .list_alias_groups
            .get(path)
            .cloned()
            .unwrap_or_else(|| HashSet::from([path.clone()]));

        // A relation can connect paths at different depths
        // (`inner@0 <-> nested@1`). A mutation below either endpoint keeps the
        // same relative offset, so `inner@1` corresponds to `nested@2`.
        for (ancestor, aliases) in &self.list_alias_groups {
            if ancestor.binding == path.binding && ancestor.index_depth <= path.index_depth {
                let offset = path.index_depth - ancestor.index_depth;
                for alias in aliases {
                    let translated = ListAliasPath {
                        binding: alias.binding.clone(),
                        index_depth: alias.index_depth + offset,
                    };
                    // A synthesized member is fed straight back into the
                    // relation by every caller, so the bound has to hold on the
                    // read path too or the relation reacquires its unbounded
                    // growth through the back door (issue #654).
                    if !translated.exceeds_tracked_depth() {
                        members.insert(translated);
                    }
                }
            }
        }
        members
    }

    /// Alias members of `path` for the purpose of *applying a mutation effect*,
    /// each paired with whether its depth had to be clamped to
    /// [`MAX_LIST_ALIAS_INDEX_DEPTH`].
    ///
    /// [`Self::list_alias_members_for_path`] discards an over-deep translation.
    /// That is right for the relation, whose key space has to stay finite to
    /// reach a fixpoint (issue #654), and wrong for the effect. Nesting past the
    /// bound is still *finite structure*, so dropping the effect leaves a
    /// genuinely aliased aggregate holding its stale, narrower element type, and
    /// a later read through the original path is then rejected on a type the
    /// program legally widened.
    ///
    /// Such a member is therefore reported clamped, and the caller applies
    /// [`ListMutationEffect::Escape`] there instead of the real effect: widening
    /// the deepest tracked ancestor to `Any` subsumes whatever the mutation would
    /// have done further down. `Escape` is the most permissive of the three
    /// effects, so a clamped path can only cost precision; unlike a clamped
    /// `Replace` it cannot pin an unrelated depth to a narrower type. That is
    /// also why a member seen both exactly and clamped keeps the clamped verdict.
    ///
    /// The work is bounded and nothing here re-enters the relation: the clamp
    /// caps every depth this returns, and the paths the caller records on
    /// [`Self::deferred_list_effect_stack`] are replayed as `Escape` anyway.
    fn list_alias_effect_members_for_path(
        &self,
        path: &ListAliasPath,
    ) -> Vec<(ListAliasPath, bool)> {
        let mut members = self
            .list_alias_groups
            .get(path)
            .cloned()
            .unwrap_or_else(|| HashSet::from([path.clone()]))
            .into_iter()
            .map(|member| (member, false))
            .collect::<HashMap<_, _>>();

        for (ancestor, aliases) in &self.list_alias_groups {
            if ancestor.binding == path.binding && ancestor.index_depth <= path.index_depth {
                let offset = path.index_depth - ancestor.index_depth;
                for alias in aliases {
                    let index_depth = alias.index_depth + offset;
                    let clamped = index_depth > MAX_LIST_ALIAS_INDEX_DEPTH;
                    let member = ListAliasPath {
                        binding: alias.binding.clone(),
                        index_depth: index_depth.min(MAX_LIST_ALIAS_INDEX_DEPTH),
                    };
                    members
                        .entry(member)
                        .and_modify(|seen_clamped| *seen_clamped |= clamped)
                        .or_insert(clamped);
                }
            }
        }

        members.into_iter().collect()
    }

    fn apply_list_mutation_effect_at_path(
        &mut self,
        path: &ListAliasPath,
        effect: &ListMutationEffect,
    ) {
        let members = self.list_alias_effect_members_for_path(path);
        if let Some(active_effects) = self.deferred_list_effect_stack.last_mut() {
            active_effects.extend(members.iter().map(|(member, _)| member.clone()));
        }
        for (member, clamped) in members {
            // A member the relation cannot track at its true depth is widened at
            // the deepest depth it can track, never skipped; see
            // [`Self::list_alias_effect_members_for_path`].
            let effect = if clamped {
                &ListMutationEffect::Escape
            } else {
                effect
            };
            if let Some(symbol) = self.analyzer.get_symbol_by_binding_key_mut(&member.binding)
                && let Some(current_type) = symbol.symbol_type.clone()
                && let Some(updated_type) =
                    Self::apply_effect_at_list_path(&current_type, member.index_depth, effect)
            {
                symbol.symbol_type = Some(updated_type);
            }
        }
    }

    fn apply_list_mutation_effect(&mut self, target: &Expression, effect: ListMutationEffect) {
        if let Some(path) = self.list_target_binding_path(target) {
            self.apply_list_mutation_effect_at_path(&path, &effect);
            return;
        }

        // Property/method-derived list values can still share mutable runtime
        // storage, but the current Type model does not retain a property path.
        // Widen only that expression's root rather than every list in scope.
        if let Some(root) = self.expression_root_binding_key(target) {
            let root = ListAliasPath {
                binding: root,
                index_depth: 0,
            };
            let members = self
                .list_alias_groups
                .get(&root)
                .cloned()
                .unwrap_or_else(|| HashSet::from([root]));
            if let Some(active_effects) = self.deferred_list_effect_stack.last_mut() {
                active_effects.extend(members.iter().cloned());
            }
            for member in members {
                if let Some(symbol) = self.analyzer.get_symbol_by_binding_key_mut(&member.binding) {
                    symbol.symbol_type = Some(Type::Any);
                }
            }
        }
    }

    fn apply_user_action_list_effects(&mut self, action_keys: &[ActionSummaryKey]) {
        self.definitely_nonempty_lists.clear();
        if let Some(caller) = self.deferred_action_key_stack.last().cloned() {
            self.user_action_dependencies
                .entry(caller)
                .or_default()
                .extend(action_keys.iter().cloned());
        }

        let effects = action_keys
            .iter()
            .flat_map(|key| {
                self.user_action_list_effects
                    .get(key)
                    .into_iter()
                    .flat_map(|effects| effects.iter().cloned())
            })
            .collect::<HashSet<_>>();
        for path in effects {
            if self.analyzer.binding_key_is_live(&path.binding) {
                self.apply_list_mutation_effect_at_path(&path, &ListMutationEffect::Escape);
            }
        }

        let mut binding_effects = HashMap::new();
        for key in action_keys {
            if let Some(effects) = self.user_action_binding_effects.get(key) {
                for (binding, effect_type) in effects {
                    Self::join_binding_effect(
                        &mut binding_effects,
                        binding.clone(),
                        effect_type.clone(),
                    );
                }
            }
        }
        for (binding, effect_type) in binding_effects {
            if !self.analyzer.binding_key_is_live(&binding) {
                continue;
            }
            let current_type = self
                .analyzer
                .get_symbol_by_binding_key(&binding)
                .and_then(|symbol| symbol.symbol_type.clone());
            let updated_type = current_type
                .map(|current| Self::join_inferred_types(current, effect_type.clone()))
                .unwrap_or(effect_type);
            if let Some(symbol) = self.analyzer.get_symbol_by_binding_key_mut(&binding) {
                symbol.symbol_type = Some(updated_type);
            }
        }
    }

    fn escape_shared_list_return_type(&self, action_keys: &[ActionSummaryKey], ty: Type) -> Type {
        let mut shared_depths = action_keys
            .iter()
            .filter_map(|key| self.user_action_shared_list_returns.get(key))
            .flat_map(|provenance| provenance.keys().copied())
            .collect::<Vec<_>>();
        shared_depths.sort_unstable();
        shared_depths.dedup();
        shared_depths.reverse();

        let mut escaped = ty;
        for depth in shared_depths {
            if let Some(updated) =
                Self::apply_effect_at_list_path(&escaped, depth, &ListMutationEffect::Escape)
            {
                escaped = updated;
            }
        }
        escaped
    }

    fn propagate_user_action_summaries(&mut self) {
        loop {
            let mut changed = false;
            let dependencies = self.user_action_dependencies.clone();
            for (caller, callees) in dependencies {
                let inherited_effects = callees
                    .iter()
                    .flat_map(|callee| {
                        self.user_action_list_effects
                            .get(callee)
                            .into_iter()
                            .flat_map(|effects| effects.iter().cloned())
                    })
                    .collect::<HashSet<_>>();
                let effects = self
                    .user_action_list_effects
                    .entry(caller.clone())
                    .or_default();
                let previous_len = effects.len();
                effects.extend(inherited_effects);
                changed |= effects.len() != previous_len;

                let inherited_binding_effects = callees
                    .iter()
                    .filter_map(|callee| self.user_action_binding_effects.get(callee))
                    .flat_map(|effects| effects.iter())
                    .map(|(binding, effect_type)| (binding.clone(), effect_type.clone()))
                    .collect::<Vec<_>>();
                let binding_effects = self
                    .user_action_binding_effects
                    .entry(caller.clone())
                    .or_default();
                let previous_binding_effects = binding_effects.clone();
                for (binding, effect_type) in inherited_binding_effects {
                    Self::join_binding_effect(binding_effects, binding, effect_type);
                }
                changed |= *binding_effects != previous_binding_effects;
            }
            if !changed {
                break;
            }
        }
    }

    fn escape_possible_shared_list_return_type(ty: Type) -> Type {
        match ty {
            Type::List(_) => Type::List(Box::new(Type::Any)),
            Type::Map(key, value) => Type::Map(
                Box::new(Self::escape_possible_shared_list_return_type(*key)),
                Box::new(Self::escape_possible_shared_list_return_type(*value)),
            ),
            Type::Optional(inner) => Type::Optional(Box::new(
                Self::escape_possible_shared_list_return_type(*inner),
            )),
            Type::Async(inner) => Type::Async(Box::new(
                Self::escape_possible_shared_list_return_type(*inner),
            )),
            other => other,
        }
    }

    fn type_may_contain_list(ty: &Type) -> bool {
        match ty {
            Type::List(_) | Type::Unknown | Type::Any | Type::Error => true,
            // Runtime maps index values; keys are text and are never mutable
            // list paths. Alias depth therefore follows only the value side.
            Type::Map(_, value) => Self::type_may_contain_list(value),
            Type::Optional(inner) | Type::Async(inner) => Self::type_may_contain_list(inner),
            _ => false,
        }
    }

    fn collect_list_depths(ty: &Type, depth: usize, out: &mut HashSet<usize>) {
        match ty {
            Type::List(element) => {
                out.insert(depth);
                Self::collect_list_depths(element, depth + 1, out);
            }
            Type::Map(_, value) => {
                Self::collect_list_depths(value, depth + 1, out);
            }
            Type::Optional(inner) | Type::Async(inner) => {
                Self::collect_list_depths(inner, depth, out);
            }
            Type::Unknown | Type::Any | Type::Error => {
                out.insert(depth);
            }
            _ => {}
        }
    }

    fn escape_lists_in_type(ty: Type) -> Type {
        match ty {
            Type::List(_) => Type::List(Box::new(Type::Any)),
            Type::Map(key, value) => Type::Map(
                Box::new(Self::escape_lists_in_type(*key)),
                Box::new(Self::escape_lists_in_type(*value)),
            ),
            Type::Optional(inner) => Type::Optional(Box::new(Self::escape_lists_in_type(*inner))),
            Type::Async(inner) => Type::Async(Box::new(Self::escape_lists_in_type(*inner))),
            other => other,
        }
    }

    fn escape_all_visible_mutable_state(&mut self) {
        // A live Optional refinement retains a sound upper bound even when an
        // opaque closure may rebind the value. Preserve that original type
        // instead of erasing it to Any; the closure can select either the
        // present or Nothing branch, but cannot justify values outside the
        // statically checked Optional contract.
        let optional_origins = self.optional_refinement_origins.clone();
        self.invalidate_optional_refinements();
        self.definitely_nonempty_lists.clear();
        let bindings = self
            .analyzer
            .live_binding_types()
            .into_iter()
            .collect::<Vec<_>>();
        for (binding, ty) in bindings {
            let Some(ty) = ty else {
                continue;
            };
            let is_mutable = self
                .analyzer
                .get_symbol_by_binding_key(&binding)
                .is_some_and(|symbol| {
                    matches!(symbol.kind, SymbolKind::Variable { mutable: true })
                });
            if is_mutable {
                let escaped_type = optional_origins.get(&binding).cloned().unwrap_or(Type::Any);
                if let Some(active_effects) = self.deferred_binding_effect_stack.last_mut() {
                    Self::join_binding_effect(
                        active_effects,
                        binding.clone(),
                        escaped_type.clone(),
                    );
                }
                if let Some(symbol) = self.analyzer.get_symbol_by_binding_key_mut(&binding) {
                    // An opaque closure can rebind any mutable captured value,
                    // not merely mutate storage reachable through a list. Keep
                    // a known Optional origin when one bounds the possibilities.
                    symbol.symbol_type = Some(escaped_type);
                }
                continue;
            }
            if !Self::type_may_contain_list(&ty) {
                continue;
            }
            let mut depths = HashSet::new();
            Self::collect_list_depths(&ty, 0, &mut depths);
            if let Some(active_effects) = self.deferred_list_effect_stack.last_mut() {
                active_effects.extend(depths.iter().map(|index_depth| ListAliasPath {
                    binding: binding.clone(),
                    index_depth: *index_depth,
                }));
            }
            if let Some(symbol) = self.analyzer.get_symbol_by_binding_key_mut(&binding) {
                symbol.symbol_type = Some(Self::escape_lists_in_type(ty));
            }
        }
    }

    fn invalidate_optional_refinements(&mut self) {
        let origins = self
            .optional_refinement_origins
            .iter()
            .map(|(binding, origin)| (binding.clone(), origin.clone()))
            .collect::<Vec<_>>();
        for (binding, origin) in origins {
            if let Some(symbol) = self.analyzer.get_symbol_by_binding_key_mut(&binding) {
                symbol.symbol_type = Some(origin);
            }
        }
    }

    fn escape_user_action_list_arguments(
        &mut self,
        arguments: &[crate::parser::ast::Argument],
        argument_types: &[Type],
    ) {
        for (argument, argument_type) in arguments.iter().zip(argument_types) {
            if !Self::type_may_contain_list(argument_type) {
                continue;
            }
            if let Some(root) = self.list_target_binding_path(&argument.value) {
                let mut depths = HashSet::new();
                Self::collect_list_depths(argument_type, 0, &mut depths);
                let mut depths = depths.into_iter().collect::<Vec<_>>();
                depths.sort_unstable_by(|left, right| right.cmp(left));
                for depth in depths {
                    self.apply_list_mutation_effect_at_path(
                        &ListAliasPath {
                            binding: root.binding.clone(),
                            index_depth: root.index_depth + depth,
                        },
                        &ListMutationEffect::Escape,
                    );
                }
            } else {
                self.apply_list_mutation_effect(&argument.value, ListMutationEffect::Escape);
            }
        }
    }

    fn record_deferred_list_rebind(
        &mut self,
        target_name: &str,
        value: &Expression,
        value_type: &Type,
    ) {
        if self.deferred_list_effect_stack.is_empty() || !Self::type_may_be_list(value_type) {
            return;
        }
        let mut affected = HashSet::new();
        if let Some(binding) = self.analyzer.get_symbol_binding_key(target_name) {
            affected.extend(self.list_alias_members_for_path(&ListAliasPath {
                binding,
                index_depth: 0,
            }));
        }
        if let Some(source) = self.list_target_binding_path(value) {
            affected.extend(self.list_alias_members_for_path(&source));
        }
        if let Some(active_effects) = self.deferred_list_effect_stack.last_mut() {
            active_effects.extend(affected);
        }
    }

    fn record_deferred_binding_assignment(&mut self, name: &str) {
        let Some(binding) = self.analyzer.get_symbol_binding_key(name) else {
            return;
        };
        let Some(effect_type) = self
            .analyzer
            .get_symbol_by_binding_key(&binding)
            .and_then(|symbol| symbol.symbol_type.clone())
        else {
            return;
        };
        if let Some(active_effects) = self.deferred_binding_effect_stack.last_mut() {
            Self::join_binding_effect(active_effects, binding, effect_type);
        }
    }

    fn merge_promoted_list_alias_bindings(
        &mut self,
        promoted: Vec<(SymbolBindingKey, SymbolBindingKey)>,
    ) {
        for (old_binding, new_binding) in promoted {
            let old_paths = self
                .list_alias_groups
                .keys()
                .filter(|path| path.binding == old_binding)
                .cloned()
                .collect::<Vec<_>>();
            if old_paths.is_empty() {
                self.union_list_alias_bindings(
                    ListAliasPath {
                        binding: old_binding,
                        index_depth: 0,
                    },
                    ListAliasPath {
                        binding: new_binding,
                        index_depth: 0,
                    },
                );
            } else {
                for old_path in old_paths {
                    self.union_list_alias_bindings(
                        old_path.clone(),
                        ListAliasPath {
                            binding: new_binding.clone(),
                            index_depth: old_path.index_depth,
                        },
                    );
                }
            }
        }
    }

    fn same_type_error(left: &TypeError, right: &TypeError) -> bool {
        left.message == right.message
            && left.expected == right.expected
            && left.found == right.found
            && left.line == right.line
            && left.column == right.column
    }

    fn deduplicate_errors_from(&mut self, start: usize) {
        let mut index = start;
        while index < self.errors.len() {
            if self.errors[..index]
                .iter()
                .any(|earlier| Self::same_type_error(earlier, &self.errors[index]))
            {
                self.errors.remove(index);
            } else {
                index += 1;
            }
        }
    }

    /// Recognize the direct variable forms users write to guard a value that
    /// may be Nothing. The boolean says whether the condition's true branch is
    /// the Nothing branch.
    fn nothing_tested_variable(condition: &Expression) -> Option<(&str, bool)> {
        match condition {
            Expression::BinaryOperation {
                left,
                operator: operator @ (Operator::Equals | Operator::NotEquals),
                right,
                ..
            } => {
                let name = match (left.as_ref(), right.as_ref()) {
                    (Expression::Variable(name, ..), Expression::Literal(Literal::Nothing, ..))
                    | (Expression::Literal(Literal::Nothing, ..), Expression::Variable(name, ..)) => {
                        name.as_str()
                    }
                    _ => return None,
                };
                Some((name, matches!(operator, Operator::Equals)))
            }
            Expression::FunctionCall {
                function,
                arguments,
                ..
            } if arguments.len() == 1
                && matches!(
                    function.as_ref(),
                    Expression::Variable(name, ..)
                        if name == "isnothing" || name == "is_nothing"
                ) =>
            {
                match &arguments[0].value {
                    Expression::Variable(name, ..) => Some((name, true)),
                    _ => None,
                }
            }
            Expression::ActionCall {
                name, arguments, ..
            } if (name == "isnothing" || name == "is_nothing") && arguments.len() == 1 => {
                match &arguments[0].value {
                    Expression::Variable(name, ..) => Some((name, true)),
                    _ => None,
                }
            }
            Expression::UnaryOperation {
                operator: UnaryOperator::Not,
                expression,
                ..
            } => Self::nothing_tested_variable(expression)
                .map(|(name, true_is_nothing)| (name, !true_is_nothing)),
            _ => None,
        }
    }

    fn optional_condition_refinement(
        &self,
        condition: &Expression,
    ) -> Option<(String, Type, Type)> {
        let (name, true_is_nothing) = Self::nothing_tested_variable(condition)?;
        let Type::Optional(inner) = self
            .analyzer
            .get_symbol(name)
            .and_then(|symbol| symbol.symbol_type.clone())?
        else {
            return None;
        };
        let present_type = *inner;
        let (then_type, else_type) = if true_is_nothing {
            (Type::Nothing, present_type)
        } else {
            (present_type, Type::Nothing)
        };
        Some((name.to_string(), then_type, else_type))
    }

    fn refine_symbol_type(&mut self, name: &str, ty: &Type) {
        if let Some(binding) = self.analyzer.get_symbol_binding_key(name)
            && let Some(origin @ Type::Optional(_)) = self
                .analyzer
                .get_symbol_by_binding_key(&binding)
                .and_then(|symbol| symbol.symbol_type.clone())
        {
            self.optional_refinement_origins
                .entry(binding)
                .or_insert(origin);
        }
        if let Some(symbol) = self.analyzer.get_symbol_mut(name) {
            symbol.symbol_type = Some(ty.clone());
        }
    }

    /// Whether execution can reach the current loop's next iteration. A
    /// definitely terminating statement suppresses the backedge; nested loops
    /// do not, because their `break` applies to the nested loop.
    fn loop_body_can_reach_backedge(statements: &[Statement]) -> bool {
        !statements
            .iter()
            .any(Self::statement_definitely_stops_current_loop)
    }

    fn statement_definitely_stops_current_loop(statement: &Statement) -> bool {
        match statement {
            Statement::BreakStatement { .. }
            | Statement::ExitStatement { .. }
            | Statement::ReturnStatement { .. } => true,
            Statement::IfStatement {
                condition: Expression::Literal(Literal::Boolean(true), ..),
                then_block,
                else_block: None,
                ..
            } => !Self::loop_body_can_reach_backedge(then_block),
            Statement::IfStatement {
                condition: Expression::Literal(Literal::Boolean(false), ..),
                else_block: Some(else_block),
                ..
            } => !Self::loop_body_can_reach_backedge(else_block),
            Statement::IfStatement {
                then_block,
                else_block: Some(else_block),
                ..
            } => {
                !Self::loop_body_can_reach_backedge(then_block)
                    && !Self::loop_body_can_reach_backedge(else_block)
            }
            Statement::SingleLineIf {
                condition: Expression::Literal(Literal::Boolean(true), ..),
                then_stmt,
                ..
            } => Self::statement_definitely_stops_current_loop(then_stmt),
            Statement::SingleLineIf {
                condition: Expression::Literal(Literal::Boolean(false), ..),
                else_stmt,
                ..
            } => else_stmt
                .as_ref()
                .is_some_and(|statement| Self::statement_definitely_stops_current_loop(statement)),
            Statement::SingleLineIf {
                then_stmt,
                else_stmt: Some(else_stmt),
                ..
            } => {
                Self::statement_definitely_stops_current_loop(then_stmt)
                    && Self::statement_definitely_stops_current_loop(else_stmt)
            }
            Statement::WaitForStatement { inner, .. } => {
                Self::statement_definitely_stops_current_loop(inner)
            }
            Statement::TryStatement {
                body,
                when_clauses,
                otherwise_block,
                finally_block,
                ..
            } => {
                finally_block
                    .as_ref()
                    .is_some_and(|block| !Self::loop_body_can_reach_backedge(block))
                    || (!Self::loop_body_can_reach_backedge(body)
                        && when_clauses
                            .iter()
                            .all(|clause| !Self::loop_body_can_reach_backedge(&clause.body))
                        && otherwise_block
                            .as_ref()
                            .is_none_or(|block| !Self::loop_body_can_reach_backedge(block)))
            }
            _ => false,
        }
    }

    fn block_can_continue(statements: &[Statement]) -> bool {
        !statements
            .iter()
            .any(Self::statement_definitely_stops_current_block)
    }

    fn statement_definitely_stops_current_block(statement: &Statement) -> bool {
        match statement {
            Statement::BreakStatement { .. }
            | Statement::ContinueStatement { .. }
            | Statement::ExitStatement { .. }
            | Statement::ReturnStatement { .. } => true,
            Statement::IfStatement {
                condition: Expression::Literal(Literal::Boolean(true), ..),
                then_block,
                else_block: None,
                ..
            } => !Self::block_can_continue(then_block),
            Statement::IfStatement {
                condition: Expression::Literal(Literal::Boolean(false), ..),
                else_block: Some(else_block),
                ..
            } => !Self::block_can_continue(else_block),
            Statement::IfStatement {
                then_block,
                else_block: Some(else_block),
                ..
            } => !Self::block_can_continue(then_block) && !Self::block_can_continue(else_block),
            Statement::SingleLineIf {
                condition: Expression::Literal(Literal::Boolean(true), ..),
                then_stmt,
                ..
            } => Self::statement_definitely_stops_current_block(then_stmt),
            Statement::SingleLineIf {
                condition: Expression::Literal(Literal::Boolean(false), ..),
                else_stmt,
                ..
            } => else_stmt
                .as_ref()
                .is_some_and(|statement| Self::statement_definitely_stops_current_block(statement)),
            Statement::SingleLineIf {
                then_stmt,
                else_stmt: Some(else_stmt),
                ..
            } => {
                Self::statement_definitely_stops_current_block(then_stmt)
                    && Self::statement_definitely_stops_current_block(else_stmt)
            }
            Statement::WaitForStatement { inner, .. } => {
                Self::statement_definitely_stops_current_block(inner)
            }
            Statement::TryStatement {
                body,
                when_clauses,
                otherwise_block,
                finally_block,
                ..
            } => {
                finally_block
                    .as_ref()
                    .is_some_and(|block| !Self::block_can_continue(block))
                    || (!Self::block_can_continue(body)
                        && when_clauses
                            .iter()
                            .all(|clause| !Self::block_can_continue(&clause.body))
                        && otherwise_block
                            .as_ref()
                            .is_none_or(|block| !Self::block_can_continue(block)))
            }
            Statement::WhileLoop {
                condition: Expression::Literal(Literal::Boolean(true), ..),
                body,
                ..
            }
            | Statement::RepeatWhileLoop {
                condition: Expression::Literal(Literal::Boolean(true), ..),
                body,
                ..
            }
            | Statement::RepeatUntilLoop {
                condition: Expression::Literal(Literal::Boolean(false), ..),
                body,
                ..
            }
            | Statement::ForeverLoop { body, .. }
            | Statement::MainLoop { body, .. } => !Self::block_may_break_current_loop(body),
            _ => false,
        }
    }

    /// Check a loop whose body executes in one persistent environment (`while`,
    /// `repeat until`, and the child environment of `repeat while`). The first
    /// iteration and the stable later-iteration header are both real runtime
    /// states, so diagnostics from either must survive.
    fn check_persistent_loop_body_fixed_point(
        &mut self,
        body: &[Statement],
        condition_can_repeat: bool,
    ) {
        let previous_backedge_mode = self.checking_persistent_loop_backedge;
        // A literal-false pre-test condition makes the body unreachable. We
        // still validate its statements, but none of its type or alias effects
        // may flow to the post-loop state.
        let unreachable_alias_state =
            (!condition_can_repeat).then(|| self.list_alias_groups.clone());
        let entry = self.analyzer.snapshot_symbol_types();
        let entry_aliases = self.list_alias_groups.clone();
        let summary_entry = self.snapshot_deferred_summary();
        self.checking_persistent_loop_backedge = previous_backedge_mode;
        self.analyzer.restore_symbol_types(entry.clone());
        self.list_alias_groups = entry_aliases.clone();
        self.check_statement_block(body);
        if self.budget_error.is_some() {
            if let Some(alias_state) = unreachable_alias_state {
                self.list_alias_groups = alias_state;
            }
            self.checking_persistent_loop_backedge = previous_backedge_mode;
            return;
        }

        self.prune_dead_list_alias_paths();
        let first_backedge = self.analyzer.snapshot_symbol_types();
        let first_backedge_aliases = self.list_alias_groups.clone();
        let first_error_end = self.errors.len();
        if !condition_can_repeat {
            self.restore_deferred_summary(summary_entry.clone());
        }
        let first_summary = self.snapshot_deferred_summary();
        let mut header = if condition_can_repeat {
            Self::join_type_snapshots(&[entry.clone(), first_backedge])
        } else {
            entry.clone()
        };
        let mut header_aliases = if condition_can_repeat {
            Self::join_list_alias_snapshots(&[
                entry_aliases.clone(),
                first_backedge_aliases.clone(),
            ])
        } else {
            entry_aliases.clone()
        };

        if condition_can_repeat && Self::loop_body_can_reach_backedge(body) {
            loop {
                self.analyzer.restore_symbol_types(header.clone());
                self.list_alias_groups = header_aliases.clone();
                let error_count = self.errors.len();
                self.checking_persistent_loop_backedge = true;
                self.restore_deferred_summary(first_summary.clone());
                self.check_statement_block(body);
                if self.budget_error.is_some() {
                    self.checking_persistent_loop_backedge = previous_backedge_mode;
                    return;
                }
                self.restore_deferred_summary(first_summary.clone());
                self.errors.truncate(error_count);

                self.prune_dead_list_alias_paths();
                let backedge = self.analyzer.snapshot_symbol_types();
                let backedge_aliases = self.list_alias_groups.clone();
                let next = Self::join_type_snapshots(&[entry.clone(), header.clone(), backedge]);
                let next_aliases = Self::join_list_alias_snapshots(&[
                    entry_aliases.clone(),
                    header_aliases.clone(),
                    backedge_aliases,
                ]);
                if next == header && next_aliases == header_aliases {
                    break;
                }
                header = next;
                header_aliases = next_aliases;
            }

            self.analyzer.restore_symbol_types(header.clone());
            self.list_alias_groups = header_aliases.clone();
            self.checking_persistent_loop_backedge = true;
            self.restore_deferred_summary(first_summary);
            self.check_statement_block(body);
            self.prune_dead_list_alias_paths();
            header_aliases =
                Self::join_list_alias_snapshots(&[header_aliases, self.list_alias_groups.clone()]);
            self.deduplicate_errors_from(first_error_end);
        }

        self.analyzer.restore_symbol_types(header);
        self.list_alias_groups = header_aliases;
        if let Some(alias_state) = unreachable_alias_state {
            self.list_alias_groups = alias_state;
        }
        self.checking_persistent_loop_backedge = previous_backedge_mode;
    }

    fn restore_fresh_iteration_state(
        &mut self,
        local_symbols: &HashMap<String, Symbol>,
        parent_types: &[HashMap<String, Option<Type>>],
    ) {
        self.analyzer
            .restore_current_scope_symbols(local_symbols.clone());
        let local_types = local_symbols
            .iter()
            .map(|(name, symbol)| (name.clone(), symbol.symbol_type.clone()))
            .collect();
        let mut state = Vec::with_capacity(parent_types.len() + 1);
        state.push(local_types);
        state.extend_from_slice(parent_types);
        self.analyzer.restore_symbol_types(state);
    }

    /// Check a loop whose runtime creates or clears a child environment before
    /// every iteration (`for each`, `count`, `forever`, and `main loop`).
    /// Iteration-local declarations reset; only mutations resolved into parent
    /// scopes contribute to the next header.
    fn check_fresh_iteration_loop_body(&mut self, body: &[Statement], guaranteed_iteration: bool) {
        let previous_backedge_mode = self.checking_persistent_loop_backedge;
        let local_symbols = self.analyzer.snapshot_current_scope_symbols();
        let entry = self.analyzer.snapshot_symbol_types();
        let entry_aliases = self.list_alias_groups.clone();
        let parent_entry = entry.get(1..).unwrap_or_default().to_vec();
        let summary_entry = self.snapshot_deferred_summary();

        self.checking_persistent_loop_backedge = false;
        self.restore_fresh_iteration_state(&local_symbols, &parent_entry);
        self.list_alias_groups = entry_aliases.clone();
        self.check_statement_block(body);
        if self.budget_error.is_some() {
            self.checking_persistent_loop_backedge = previous_backedge_mode;
            return;
        }

        self.prune_dead_list_alias_paths();
        let first_snapshot = self.analyzer.snapshot_symbol_types();
        let first_backedge_aliases = self.list_alias_groups.clone();
        let first_backedge = first_snapshot.get(1..).unwrap_or_default().to_vec();
        let first_error_end = self.errors.len();
        let first_summary = self.snapshot_deferred_summary();
        let mut parent_header = if guaranteed_iteration {
            first_backedge
        } else {
            Self::join_type_snapshots(&[parent_entry.clone(), first_backedge])
        };
        let mut header_aliases = if guaranteed_iteration {
            first_backedge_aliases
        } else {
            Self::join_list_alias_snapshots(&[entry_aliases.clone(), first_backedge_aliases])
        };

        if Self::loop_body_can_reach_backedge(body) {
            loop {
                self.restore_fresh_iteration_state(&local_symbols, &parent_header);
                self.list_alias_groups = header_aliases.clone();
                let error_count = self.errors.len();
                self.checking_persistent_loop_backedge = false;
                self.restore_deferred_summary(first_summary.clone());
                self.check_statement_block(body);
                if self.budget_error.is_some() {
                    self.checking_persistent_loop_backedge = previous_backedge_mode;
                    return;
                }
                self.restore_deferred_summary(first_summary.clone());
                self.errors.truncate(error_count);

                self.prune_dead_list_alias_paths();
                let snapshot = self.analyzer.snapshot_symbol_types();
                let backedge_aliases = self.list_alias_groups.clone();
                let backedge = snapshot.get(1..).unwrap_or_default().to_vec();
                let next = if guaranteed_iteration {
                    Self::join_type_snapshots(&[parent_header.clone(), backedge])
                } else {
                    Self::join_type_snapshots(&[
                        parent_entry.clone(),
                        parent_header.clone(),
                        backedge,
                    ])
                };
                let next_aliases = if guaranteed_iteration {
                    Self::join_list_alias_snapshots(&[header_aliases.clone(), backedge_aliases])
                } else {
                    Self::join_list_alias_snapshots(&[
                        entry_aliases.clone(),
                        header_aliases.clone(),
                        backedge_aliases,
                    ])
                };
                if next == parent_header && next_aliases == header_aliases {
                    break;
                }
                parent_header = next;
                header_aliases = next_aliases;
            }

            self.restore_fresh_iteration_state(&local_symbols, &parent_header);
            self.list_alias_groups = header_aliases.clone();
            self.checking_persistent_loop_backedge = false;
            self.restore_deferred_summary(first_summary);
            self.check_statement_block(body);
            self.prune_dead_list_alias_paths();
            header_aliases =
                Self::join_list_alias_snapshots(&[header_aliases, self.list_alias_groups.clone()]);
            self.deduplicate_errors_from(first_error_end);
        }

        self.restore_fresh_iteration_state(&local_symbols, &parent_header);
        self.list_alias_groups = header_aliases;
        let _ = summary_entry;
        self.checking_persistent_loop_backedge = previous_backedge_mode;
    }

    /// Type-check a post-test loop. The body runs before the condition on the
    /// first iteration, and later iterations re-enter under the joined
    /// entry/backedge state.
    fn check_repeat_until_fixed_point(
        &mut self,
        condition: &Expression,
        body: &[Statement],
        line: usize,
        column: usize,
    ) {
        let previous_backedge_mode = self.checking_persistent_loop_backedge;
        let entry = self.analyzer.snapshot_symbol_types();
        let entry_aliases = self.list_alias_groups.clone();
        let summary_entry = self.snapshot_deferred_summary();

        self.checking_persistent_loop_backedge = previous_backedge_mode;
        self.analyzer.restore_symbol_types(entry.clone());
        self.list_alias_groups = entry_aliases;
        self.check_statement_block(body);
        let condition_type = self.infer_expression_type(condition);
        if condition_type != Type::Boolean && !self.is_gradual_type(&condition_type) {
            self.type_error(
                "Condition in repeat-until loop must be a boolean expression".to_string(),
                Some(Type::Boolean),
                Some(condition_type),
                line,
                column,
            );
        }
        if self.budget_error.is_some() {
            self.checking_persistent_loop_backedge = previous_backedge_mode;
            return;
        }

        self.prune_dead_list_alias_paths();
        let first_backedge = self.analyzer.snapshot_symbol_types();
        let first_backedge_aliases = self.list_alias_groups.clone();
        let first_error_end = self.errors.len();
        let first_summary = self.snapshot_deferred_summary();
        let mut header = Self::join_type_snapshots(&[entry.clone(), first_backedge]);
        let mut header_aliases = first_backedge_aliases.clone();

        let condition_can_repeat =
            !matches!(condition, Expression::Literal(Literal::Boolean(true), ..));
        if condition_can_repeat && Self::loop_body_can_reach_backedge(body) {
            loop {
                self.analyzer.restore_symbol_types(header.clone());
                self.list_alias_groups = header_aliases.clone();
                let error_count = self.errors.len();
                self.checking_persistent_loop_backedge = true;
                self.restore_deferred_summary(first_summary.clone());
                self.check_statement_block(body);
                let condition_type = self.infer_expression_type(condition);
                if condition_type != Type::Boolean && !self.is_gradual_type(&condition_type) {
                    self.type_error(
                        "Condition in repeat-until loop must be a boolean expression".to_string(),
                        Some(Type::Boolean),
                        Some(condition_type),
                        line,
                        column,
                    );
                }
                if self.budget_error.is_some() {
                    self.checking_persistent_loop_backedge = previous_backedge_mode;
                    return;
                }
                self.restore_deferred_summary(first_summary.clone());
                self.errors.truncate(error_count);

                self.prune_dead_list_alias_paths();
                let backedge = self.analyzer.snapshot_symbol_types();
                let backedge_aliases = self.list_alias_groups.clone();
                let next = Self::join_type_snapshots(&[entry.clone(), header.clone(), backedge]);
                let next_aliases = Self::join_list_alias_snapshots(&[
                    first_backedge_aliases.clone(),
                    header_aliases.clone(),
                    backedge_aliases,
                ]);
                if next == header && next_aliases == header_aliases {
                    break;
                }
                header = next;
                header_aliases = next_aliases;
            }

            self.analyzer.restore_symbol_types(header.clone());
            self.list_alias_groups = header_aliases.clone();
            self.checking_persistent_loop_backedge = true;
            self.restore_deferred_summary(first_summary);
            self.check_statement_block(body);
            let condition_type = self.infer_expression_type(condition);
            if condition_type != Type::Boolean && !self.is_gradual_type(&condition_type) {
                self.type_error(
                    "Condition in repeat-until loop must be a boolean expression".to_string(),
                    Some(Type::Boolean),
                    Some(condition_type),
                    line,
                    column,
                );
            }
            self.prune_dead_list_alias_paths();
            header_aliases =
                Self::join_list_alias_snapshots(&[header_aliases, self.list_alias_groups.clone()]);
            self.deduplicate_errors_from(first_error_end);
        }

        self.analyzer.restore_symbol_types(header);
        self.list_alias_groups = header_aliases;
        let _ = summary_entry;
        self.checking_persistent_loop_backedge = previous_backedge_mode;
    }

    /// Like [`check_loop_body_fixed_point`], but leaves the POST-BODY type
    /// state installed instead of restoring the loop-header state.
    /// `repeat until` evaluates its condition after each body execution and
    /// always runs the body at least once, so the condition — and everything
    /// after the loop — sees the body's final type state, not the header
    /// join (#642).
    ///
    /// When the body can exit early (`break`/`exit`/`return` anywhere in its
    /// subtree), the condition is NOT necessarily reached from the fall-through
    /// state — runtime skips it entirely on a break path — so the caller must
    /// soften the installed state with the returned header join (see
    /// `RepeatUntilLoop`). Returns the stabilized header snapshot for that.
    fn check_loop_body_fixed_point_post_body(
        &mut self,
        body: &[Statement],
    ) -> Vec<HashMap<String, Option<Type>>> {
        let entry = self.analyzer.snapshot_symbol_types();
        let mut header = entry.clone();

        loop {
            self.analyzer.restore_symbol_types(header.clone());
            let error_count = self.errors.len();
            for statement in body {
                self.check_statement_types(statement);
            }
            if self.budget_error.is_some() {
                return header;
            }
            self.errors.truncate(error_count);

            let backedge = self.analyzer.snapshot_symbol_types();
            let next = Self::join_type_snapshots(&[entry.clone(), header.clone(), backedge]);
            if next == header {
                break;
            }
            header = next;
        }

        self.analyzer.restore_symbol_types(header.clone());
        for statement in body {
            self.check_statement_types(statement);
        }
        header
    }

    /// Whether executing `body` can leave its enclosing loop without reaching
    /// the loop's own condition. A `break` counts only at this loop's own
    /// level — a `break` inside a NESTED loop exits that inner loop and the
    /// outer body still falls through to its condition. `exit loop` and
    /// `return` propagate out of every enclosing loop (`ControlFlow::Exit` /
    /// `Return` are re-raised by each loop's dispatch), so they count from
    /// any nesting depth.
    fn body_may_exit_loop_early(body: &[Statement]) -> bool {
        body.iter().any(Self::statement_may_exit_loop_early)
    }

    fn statement_may_exit_loop_early(stmt: &Statement) -> bool {
        match stmt {
            Statement::BreakStatement { .. }
            | Statement::ExitStatement { .. }
            | Statement::ReturnStatement { .. } => true,
            Statement::IfStatement {
                then_block,
                else_block,
                ..
            } => {
                Self::body_may_exit_loop_early(then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|b| Self::body_may_exit_loop_early(b))
            }
            Statement::SingleLineIf {
                then_stmt,
                else_stmt,
                ..
            } => {
                Self::statement_may_exit_loop_early(then_stmt)
                    || else_stmt
                        .as_deref()
                        .is_some_and(Self::statement_may_exit_loop_early)
            }
            // A nested loop consumes `break`; only `exit`/`return` escape it.
            Statement::ForEachLoop { body, .. }
            | Statement::CountLoop { body, .. }
            | Statement::WhileLoop { body, .. }
            | Statement::RepeatWhileLoop { body, .. }
            | Statement::RepeatUntilLoop { body, .. }
            | Statement::ForeverLoop { body, .. }
            | Statement::MainLoop { body, .. } => Self::body_escapes_enclosing_loop(body),
            Statement::TryStatement {
                body,
                when_clauses,
                otherwise_block,
                finally_block,
                ..
            } => {
                Self::body_may_exit_loop_early(body)
                    || when_clauses
                        .iter()
                        .any(|c| Self::body_may_exit_loop_early(&c.body))
                    || otherwise_block
                        .as_ref()
                        .is_some_and(|b| Self::body_may_exit_loop_early(b))
                    || finally_block
                        .as_ref()
                        .is_some_and(|b| Self::body_may_exit_loop_early(b))
            }
            _ => false,
        }
    }

    /// Whether `body` contains a control transfer that escapes EVERY enclosing
    /// loop: `exit loop` or `return`. `break` never qualifies (the nearest
    /// loop absorbs it), so nested loops are descended into freely.
    fn body_escapes_enclosing_loop(body: &[Statement]) -> bool {
        body.iter().any(Self::statement_escapes_enclosing_loop)
    }

    fn statement_escapes_enclosing_loop(stmt: &Statement) -> bool {
        match stmt {
            Statement::ExitStatement { .. } | Statement::ReturnStatement { .. } => true,
            Statement::BreakStatement { .. } => false,
            Statement::IfStatement {
                then_block,
                else_block,
                ..
            } => {
                Self::body_escapes_enclosing_loop(then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|b| Self::body_escapes_enclosing_loop(b))
            }
            Statement::SingleLineIf {
                then_stmt,
                else_stmt,
                ..
            } => {
                Self::statement_escapes_enclosing_loop(then_stmt)
                    || else_stmt
                        .as_deref()
                        .is_some_and(Self::statement_escapes_enclosing_loop)
            }
            Statement::ForEachLoop { body, .. }
            | Statement::CountLoop { body, .. }
            | Statement::WhileLoop { body, .. }
            | Statement::RepeatWhileLoop { body, .. }
            | Statement::RepeatUntilLoop { body, .. }
            | Statement::ForeverLoop { body, .. }
            | Statement::MainLoop { body, .. } => Self::body_escapes_enclosing_loop(body),
            Statement::TryStatement {
                body,
                when_clauses,
                otherwise_block,
                finally_block,
                ..
            } => {
                Self::body_escapes_enclosing_loop(body)
                    || when_clauses
                        .iter()
                        .any(|c| Self::body_escapes_enclosing_loop(&c.body))
                    || otherwise_block
                        .as_ref()
                        .is_some_and(|b| Self::body_escapes_enclosing_loop(b))
                    || finally_block
                        .as_ref()
                        .is_some_and(|b| Self::body_escapes_enclosing_loop(b))
            }
            _ => false,
        }
    }

    /// Get the return type for builtin functions
    fn get_builtin_function_type(&self, name: &str, _arg_count: usize) -> Type {
        match name {
            // Core functions
            "typeof" | "type_of" => Type::Text,
            "isnothing" | "is_nothing" => Type::Boolean,
            "print" | "sleep" | "foreach" => Type::Nothing, // Void functions

            // Math functions
            "abs" | "round" | "floor" | "ceil" | "clamp" | "min" | "max" | "power" | "sqrt"
            | "sin" | "cos" | "tan" => Type::Number,

            // Random functions
            "random" | "random_between" | "random_int" => Type::Number,
            "random_boolean" => Type::Boolean,
            "random_from" => Type::Any, // Returns element from list, so type depends on list
            "random_seed" => Type::Nothing, // Void function

            // Text functions
            "length" | "indexof" | "index_of" | "lastindexof" | "last_index_of" => Type::Number,
            "touppercase" | "to_uppercase" | "tolowercase" | "to_lowercase" | "substring"
            | "replace" | "trim" | "padleft" | "padright" | "format_number" | "capitalize"
            | "reverse" | "reverse_text" => Type::Text,
            "contains" | "startswith" | "starts_with" | "endswith" | "ends_with" => Type::Boolean,
            "split" => Type::List(Box::new(Type::Text)),
            "join" => Type::Text,

            // List functions
            "push" | "sort" | "reverse_list" | "clear" | "unshift" | "insertat" | "insert_at"
            | "fill" => Type::Nothing,
            "slice" | "concat" | "unique" | "filter" | "map" => Type::List(Box::new(Type::Any)),
            "find" => Type::Optional(Box::new(Type::Any)),
            "pop" | "shift" | "removeat" | "remove_at" | "reduce" => Type::Any,
            "count" | "size" | "find_index" => Type::Number,
            "includes" | "every" | "some" => Type::Boolean,

            // Time functions. Date/Time/DateTime are runtime value types, so
            // preserve them as named static types rather than erasing them to
            // Any (or, historically, misclassifying them as Number).
            "today" | "date" | "parsedate" | "parse_date" | "create_date" | "date_part"
            | "adddays" | "add_days" | "subtract_days" | "addmonths" | "add_months"
            | "addyears" | "add_years" => Type::Date,
            "now" | "time" | "parse_time" | "create_time" | "time_part" => Type::Time,
            "datetime_now"
            | "create_datetime"
            | "utc_now"
            | "datetime_from_timestamp"
            | "addhours"
            | "add_hours"
            | "addminutes"
            | "add_minutes"
            | "addseconds"
            | "add_seconds" => Type::DateTime,
            "year" | "month" | "day" | "hour" | "minute" | "second" | "dayofweek"
            | "day_of_week" | "dayofyear" | "day_of_year" | "days_in_month" | "week_of_year"
            | "timestamp" | "time_diff" => Type::Number,
            "formatdate" | "format_date" | "formattime" | "format_time" | "format_datetime"
            | "current_date" => Type::Text,
            "isleapyear" | "is_leap_year" => Type::Boolean,
            "daysbetween" | "days_between" | "monthsbetween" | "months_between"
            | "yearsbetween" | "years_between" => Type::Number,

            // Pattern functions
            "pattern" | "match" | "test" | "replace_pattern" | "extract" => Type::Text,
            "ismatch" | "is_match" | "pattern_matches" => Type::Boolean,
            "findall" | "find_all" => Type::List(Box::new(Type::Text)),
            "pattern_find" => Type::Optional(Box::new(Type::Map(
                Box::new(Type::Text),
                Box::new(Type::Any),
            ))),
            "pattern_find_all" => Type::List(Box::new(Type::Map(
                Box::new(Type::Text),
                Box::new(Type::Any),
            ))),

            // Crypto functions
            "wflhash256"
            | "wflhash512"
            | "wflhash256_with_salt"
            | "wflmac256"
            | "sha256"
            | "hmac_sha256"
            | "generate_uuid"
            | "generate_csrf_token"
            | "pbkdf2_hmac_sha256"
            | "secure_random_bytes" => Type::Text,

            // Timing-safe comparison returns a boolean
            "constant_time_equals" => Type::Boolean,

            // Password hashing: *_hash produce a string, *_verify produce a boolean
            "hash_password" | "argon2_hash" | "bcrypt_hash" | "scrypt_hash" | "pbkdf2_hash" => {
                Type::Text
            }
            "verify_password" | "argon2_verify" | "bcrypt_verify" | "scrypt_verify"
            | "pbkdf2_verify" => Type::Boolean,

            // JSON functions
            "parse_json" => Type::Any, // object/list/scalar depending on the JSON
            "stringify_json" | "stringify_json_pretty" => Type::Text,

            // Query and form parsing (objects with string values)
            "parse_query_string" | "parse_cookies" | "parse_form_urlencoded" => {
                Type::Map(Box::new(Type::Text), Box::new(Type::Text))
            }

            // Web routing helpers
            // A capture map on success, Nothing when the route does not match.
            "path_params" => Type::Optional(Box::new(Type::Map(
                Box::new(Type::Text),
                Box::new(Type::Text),
            ))),
            "path_matches" => Type::Boolean,
            "mime_type" => Type::Text,
            "parse_multipart" => Type::List(Box::new(Type::Map(
                Box::new(Type::Text),
                Box::new(Type::Any),
            ))),

            // Text functions registered under stdlib-specific names
            "string_split" => Type::List(Box::new(Type::Text)),

            // Filesystem functions
            "list_dir" | "glob" | "rglob" | "list_directory" => Type::List(Box::new(Type::Text)),
            "path_join" | "path_basename" | "path_dirname" | "path_extension" | "path_stem"
            | "read_file" => Type::Text,
            "path_exists" | "is_file" | "is_dir" | "file_exists" | "is_directory" => Type::Boolean,
            "file_size" | "file_mtime" | "count_lines" => Type::Number,
            "makedirs" | "copy_file" | "move_file" | "remove_file" | "remove_dir"
            | "write_file" | "delete_file" | "create_directory" => Type::Nothing,

            // Every registered builtin returns a value (void ones return
            // Nothing above). For the few remaining names without an explicit
            // entry (test helpers, not-yet-implemented placeholders), Any
            // keeps variables bound to their results inferable instead of
            // raising spurious "Could not infer type" errors (issue #551).
            _ => Type::Any,
        }
    }

    /// Runtime variable lookup auto-invokes native builtins whose canonical
    /// arity is zero. Mirror that here; nonzero-arity builtins remain callable
    /// function values.
    fn get_bare_builtin_type(&self, name: &str) -> Type {
        let parameter_count = builtins::get_function_arity(name);
        let return_type = self.get_builtin_function_type(name, parameter_count);
        if parameter_count == 0 {
            return_type
        } else {
            let fixed_signatures = self.builtin_signatures(name).map(|signatures| {
                signatures
                    .into_iter()
                    .filter(|signature| signature.parameters.len() == parameter_count)
                    .collect::<Vec<_>>()
            });
            if crate::stdlib::typechecker::variadic_builtin_parameter_type(name).is_none()
                && let Some(signatures) = fixed_signatures
                && signatures.len() == 1
            {
                let signature = &signatures[0];
                return Type::Function {
                    parameters: signature
                        .parameters
                        .iter()
                        .map(|parameter| {
                            parameter
                                .param_type
                                .as_ref()
                                .cloned()
                                .unwrap_or(Type::Unknown)
                        })
                        .collect(),
                    return_type: Box::new(
                        signature
                            .return_type
                            .as_ref()
                            .cloned()
                            .unwrap_or(return_type),
                    ),
                };
            }
            Type::Function {
                parameters: vec![Type::Any; parameter_count],
                return_type: Box::new(return_type),
            }
        }
    }

    /// Infer and validate a builtin call through one path for both
    /// `function of ...` and `call function with ...` syntax.
    fn infer_builtin_call_type(
        &mut self,
        name: &str,
        arguments: &[crate::parser::ast::Argument],
        line: usize,
        column: usize,
    ) -> Type {
        // Always visit arguments, even when the call has the wrong arity. A
        // nested type error must not disappear merely because the callee is a
        // native function.
        let mut declared_property_target = None;
        let mut arg_types = Vec::with_capacity(arguments.len());
        for (index, argument) in arguments.iter().enumerate() {
            if index == 0
                && matches!(
                    name,
                    "push" | "unshift" | "insert_at" | "insertat" | "fill" | "clear"
                )
            {
                let (target_type, property_contract) =
                    self.infer_list_mutation_target(&argument.value);
                arg_types.push(target_type);
                declared_property_target = property_contract;
            } else {
                arg_types.push(self.infer_expression_type(&argument.value));
            }
        }

        if !builtins::is_implemented_builtin_function(name) {
            self.type_error(
                format!("Builtin '{name}' is recognized but not implemented by the runtime"),
                None,
                None,
                line,
                column,
            );
            return Type::Error;
        }

        let (minimum, maximum) = builtins::get_function_arity_range(name);
        let arity_is_valid =
            arguments.len() >= minimum && maximum.is_none_or(|maximum| arguments.len() <= maximum);
        if !arity_is_valid {
            let expected = match maximum {
                None => format!("at least {minimum}"),
                Some(maximum) if minimum == maximum => minimum.to_string(),
                Some(maximum) => format!("{minimum} to {maximum}"),
            };
            self.type_error(
                format!(
                    "Builtin '{name}' expects {expected} arguments, but {} were provided",
                    arguments.len()
                ),
                None,
                None,
                line,
                column,
            );
            return Type::Error;
        }

        if arg_types.contains(&Type::Error) {
            return Type::Error;
        }

        // Variadic builtins have one repeated runtime parameter contract that
        // cannot be represented by the fixed-vector FunctionSignature type.
        let variadic_parameter = crate::stdlib::typechecker::variadic_builtin_parameter_type(name);
        if let Some(parameter_type) = &variadic_parameter
            && let Some((index, arg_type)) = arg_types
                .iter()
                .enumerate()
                .find(|(_, arg_type)| !self.are_builtin_types_compatible(parameter_type, arg_type))
        {
            self.type_error(
                format!(
                    "Argument {} of builtin '{}' expected {}, but found {}",
                    index + 1,
                    name,
                    parameter_type,
                    arg_type
                ),
                Some(parameter_type.clone()),
                Some(arg_type.clone()),
                line,
                column,
            );
            return Type::Error;
        }

        // Every implemented native has at least one registered static
        // signature. Resolve fixed-arity overloads here and return the result
        // from the contract itself, avoiding a second source of truth.
        if let Some(signatures) = self.builtin_signatures(name) {
            let arity_matches: Vec<_> = signatures
                .iter()
                .filter(|signature| signature.parameters.len() == arguments.len())
                .collect();
            if let Some(signature) =
                arity_matches.iter().find(|signature| {
                    signature.parameters.iter().zip(arg_types.iter()).all(
                        |(parameter, arg_type)| {
                            let parameter_type = parameter
                                .param_type
                                .as_ref()
                                .cloned()
                                .unwrap_or(Type::Unknown);
                            self.are_builtin_types_compatible(&parameter_type, arg_type)
                        },
                    )
                })
            {
                let declared_return = signature
                    .return_type
                    .as_ref()
                    .cloned()
                    .unwrap_or(Type::Error);
                if let Some((property_name, Type::List(element_type))) = &declared_property_target {
                    let value_index = match name {
                        "push" | "unshift" | "fill" => Some(1),
                        "insert_at" | "insertat" => Some(2),
                        _ => None,
                    };
                    if let Some(value_index) = value_index
                        && let Some(value_type) = arg_types.get(value_index)
                        && !self.are_declared_property_values_compatible(
                            element_type,
                            value_type,
                            &arguments[value_index].value,
                        )
                    {
                        self.type_error(
                            format!(
                                "Builtin '{name}' cannot put {value_type} into property \
                                 '{property_name}' because its declared element type is \
                                 {element_type}"
                            ),
                            Some((**element_type).clone()),
                            Some(value_type.clone()),
                            line,
                            column,
                        );
                        return Type::Error;
                    }
                }
                self.apply_builtin_type_effects(
                    name,
                    arguments,
                    &arg_types,
                    declared_property_target.is_some(),
                );
                return Self::specialize_builtin_return_type(name, &arg_types, declared_return);
            }

            if !arity_matches.is_empty() {
                let signature = arity_matches[0];
                if let Some((index, (parameter, arg_type))) = signature
                    .parameters
                    .iter()
                    .zip(arg_types.iter())
                    .enumerate()
                    .find(|(_, (parameter, arg_type))| {
                        let parameter_type = parameter
                            .param_type
                            .as_ref()
                            .cloned()
                            .unwrap_or(Type::Unknown);
                        !self.are_builtin_types_compatible(&parameter_type, arg_type)
                    })
                {
                    let parameter_type = parameter
                        .param_type
                        .as_ref()
                        .cloned()
                        .unwrap_or(Type::Unknown);
                    let temporal_hint = match (&parameter_type, arg_type) {
                        (Type::Date, Type::Custom(custom_name))
                            if custom_name.eq_ignore_ascii_case("date")
                                && self.analyzer.get_containers().contains_key(custom_name) =>
                        {
                            Some(format!(
                                "'{custom_name}' is a custom/container annotation in this \
                                 program; use lowercase 'date' for the temporal type"
                            ))
                        }
                        (Type::Time, Type::Custom(custom_name))
                            if custom_name.eq_ignore_ascii_case("time")
                                && self.analyzer.get_containers().contains_key(custom_name) =>
                        {
                            Some(format!(
                                "'{custom_name}' is a custom/container annotation in this \
                                 program; use lowercase 'time' for the temporal type"
                            ))
                        }
                        (Type::DateTime, Type::Custom(custom_name))
                            if custom_name.eq_ignore_ascii_case("datetime")
                                && self.analyzer.get_containers().contains_key(custom_name) =>
                        {
                            Some(format!(
                                "'{custom_name}' is a custom/container annotation in this \
                                 program, and WFL has no unambiguous DateTime spelling while \
                                 that container exists; rename the container or leave the \
                                 parameter gradual"
                            ))
                        }
                        _ => None,
                    };
                    if let Some(hint) = temporal_hint {
                        self.type_error(
                            format!(
                                "Argument {} of builtin '{}' requires a runtime temporal value, \
                                 but {}",
                                index + 1,
                                name,
                                hint
                            ),
                            None,
                            None,
                            line,
                            column,
                        );
                    } else {
                        self.type_error(
                            format!(
                                "Argument {} of builtin '{}' expected {}, but found {}",
                                index + 1,
                                name,
                                parameter_type,
                                arg_type
                            ),
                            Some(parameter_type),
                            Some(arg_type.clone()),
                            line,
                            column,
                        );
                    }
                } else {
                    self.type_error(
                        format!("No signature of builtin '{name}' matches this call"),
                        None,
                        None,
                        line,
                        column,
                    );
                }
                return Type::Error;
            }

            // A variadic call beyond its single registered seed signature was
            // validated by the repeated parameter contract above.
            if variadic_parameter.is_some() {
                return self.get_builtin_function_type(name, arguments.len());
            }
        } else {
            self.type_error(
                format!("Builtin '{name}' has no registered static contract"),
                None,
                None,
                line,
                column,
            );
            return Type::Error;
        }

        // A fixed-arity native with no matching signature indicates a broken
        // checker/runtime contract table rather than a dynamic call.
        self.type_error(
            format!(
                "Builtin '{name}' has no static signature for {} arguments",
                arguments.len()
            ),
            None,
            None,
            line,
            column,
        );
        Type::Error
    }

    fn specialize_builtin_return_type(
        name: &str,
        argument_types: &[Type],
        declared_return: Type,
    ) -> Type {
        let first_element = || match argument_types.first() {
            Some(Type::List(element)) => Some((**element).clone()),
            _ => None,
        };
        match name {
            "random_from" | "pop" | "shift" | "remove_at" | "removeat" => {
                first_element().unwrap_or(declared_return)
            }
            "find" => first_element()
                .map(Self::optionalize)
                .unwrap_or(declared_return),
            "slice" | "unique" => match argument_types.first() {
                Some(list_type @ Type::List(_)) => list_type.clone(),
                _ => declared_return,
            },
            "concat" => match (argument_types.first(), argument_types.get(1)) {
                (Some(Type::List(left)), Some(Type::List(right))) => Type::List(Box::new(
                    Self::join_collection_value_type(Some((**left).clone()), (**right).clone()),
                )),
                _ => declared_return,
            },
            _ => declared_return,
        }
    }

    fn apply_builtin_type_effects(
        &mut self,
        name: &str,
        arguments: &[crate::parser::ast::Argument],
        argument_types: &[Type],
        target_is_declared_property: bool,
    ) {
        if matches!(name, "clear" | "pop" | "shift" | "remove_at" | "removeat") {
            self.definitely_nonempty_lists.clear();
        }
        if name == "clear" {
            if !target_is_declared_property
                && let Some(target) = arguments.first().map(|argument| &argument.value)
            {
                self.detach_list_alias_descendants(target);
            }
            return;
        }
        let value_index = match name {
            "push" | "unshift" | "fill" => Some(1),
            "insert_at" | "insertat" => Some(2),
            _ => None,
        };
        let Some(value_index) = value_index else {
            return;
        };
        let Some(target) = arguments.first().map(|argument| &argument.value) else {
            return;
        };
        if target_is_declared_property {
            // Container properties are not analyzer bindings. Applying the
            // lexical alias effect here would instead widen an unrelated
            // same-named outer variable (for a bare property) or the instance
            // binding itself (for `box.items`).
            return;
        }
        let (Some(target_type), Some(value_type)) =
            (argument_types.first(), argument_types.get(value_index))
        else {
            return;
        };
        if !matches!(
            target_type,
            Type::List(_) | Type::Unknown | Type::Any | Type::Error
        ) {
            return;
        }
        let effect = if name == "fill" {
            self.detach_list_alias_descendants(target);
            ListMutationEffect::Replace(value_type.clone())
        } else {
            ListMutationEffect::Join(value_type.clone())
        };
        self.apply_list_mutation_effect(target, effect);
        self.record_list_insertion_aliases(target, &arguments[value_index].value);
        if name != "fill" {
            self.mark_list_target_nonempty(target);
        }
    }

    /// Infer a list mutation target once while retaining whether its type came
    /// from a declared container property. The ordinary `Type::List<T>` alone
    /// is insufficient: lexical lists may widen under gradual typing, whereas
    /// a property annotation is a persistent contract.
    fn infer_list_mutation_target(
        &mut self,
        target: &Expression,
    ) -> (Type, Option<(String, Type)>) {
        match target {
            Expression::Variable(name, ..) => {
                let target_type = self.infer_expression_type(target);
                let (declared_type, is_property) = self.resolve_bare_mutation_target_type(name);
                (
                    target_type,
                    is_property
                        .then_some(declared_type)
                        .flatten()
                        .map(|declared| (name.clone(), declared)),
                )
            }
            Expression::PropertyAccess {
                object,
                property,
                line,
                column,
            } => {
                let object_type = self.infer_expression_type(object);
                let (target_type, is_declared_property) =
                    self.infer_property_access_type(object_type, property, *line, *column);
                (
                    target_type.clone(),
                    is_declared_property.then_some((property.clone(), target_type)),
                )
            }
            Expression::StaticMemberAccess {
                container, member, ..
            } => {
                let target_type = self.infer_expression_type(target);
                let declared_type = self.container_static_property_type(container, member);
                (
                    target_type,
                    declared_type.map(|declared| (member.clone(), declared)),
                )
            }
            _ => (self.infer_expression_type(target), None),
        }
    }

    /// The registered signatures for `name` when it resolves to a function
    /// symbol, cloned so callers don't hold a borrow of the analyzer.
    fn action_signatures(&self, name: &str) -> Option<Vec<crate::analyzer::FunctionSignature>> {
        let symbol = self.analyzer.get_symbol(name)?;
        if let SymbolKind::Function { signatures } = &symbol.kind {
            Some(signatures.clone())
        } else {
            None
        }
    }

    /// Index of the signature whose parameters match this definition's, within
    /// the analyzer symbol's signature list. Definition-time validation
    /// guarantees signatures are pairwise distinct, so parameter equality
    /// identifies the overload unambiguously.
    fn signature_index_for(&self, name: &str, parameters: &[Parameter]) -> Option<usize> {
        self.action_signatures(name)?
            .iter()
            .position(|sig| sig.parameters == parameters)
    }

    /// Resolves a call against an overloaded action (more than one registered
    /// signature): filters candidates by argument count, then by inferred
    /// argument types, and returns the resolved (or common) return type.
    /// Mirrors the analyzer's `check_overloaded_call`, but produces the type
    /// for the call expression.
    fn infer_overloaded_call_type(
        &mut self,
        name: &str,
        signatures: &[crate::analyzer::FunctionSignature],
        arguments: &[crate::parser::ast::Argument],
        line: usize,
        column: usize,
    ) -> Type {
        let arg_types: Vec<Type> = arguments
            .iter()
            .map(|arg| self.infer_expression_type(&arg.value))
            .collect();

        let arity_matches: Vec<usize> = (0..signatures.len())
            .filter(|&i| signatures[i].parameters.len() == arguments.len())
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
            self.type_error(
                format!(
                    "Action '{name}' expects {arities_str} arguments, but {} were provided",
                    arguments.len()
                ),
                None,
                None,
                line,
                column,
            );
            return Type::Error;
        }

        let compatible: Vec<usize> =
            arity_matches
                .iter()
                .copied()
                .filter(|&i| {
                    signatures[i].parameters.iter().zip(arg_types.iter()).all(
                        |(param, arg_type)| {
                            let param_type =
                                param.param_type.as_ref().cloned().unwrap_or(Type::Unknown);
                            self.are_types_compatible(&param_type, arg_type)
                        },
                    )
                })
                .collect();

        let specificity_of = |index: usize| -> (usize, usize) {
            signatures[index].parameters.iter().zip(&arg_types).fold(
                (0, 0),
                |(concrete, exact_temporal), (param, arg)| {
                    let Some(param_type) = param.param_type.as_ref() else {
                        return (concrete, exact_temporal);
                    };
                    if matches!(param_type, Type::Any | Type::Unknown)
                        || matches!(arg, Type::Any | Type::Unknown | Type::Error)
                        || (matches!(arg, Type::Nothing) && !matches!(param_type, Type::Nothing))
                    {
                        return (concrete, exact_temporal);
                    }
                    let exact_temporal_match = matches!(
                        (param_type, arg),
                        (Type::Date, Type::Date)
                            | (Type::Time, Type::Time)
                            | (Type::DateTime, Type::DateTime)
                    );
                    (
                        concrete + 1,
                        exact_temporal + usize::from(exact_temporal_match),
                    )
                },
            )
        };
        let best_specificity = compatible.iter().map(|&index| specificity_of(index)).max();
        let most_specific: Vec<usize> = compatible
            .iter()
            .copied()
            .filter(|&index| Some(specificity_of(index)) == best_specificity)
            .collect();

        let return_type_of = |checker: &Self, index: usize| -> Type {
            checker
                .overload_returns
                .get(&(name.to_string(), index))
                .cloned()
                .or_else(|| signatures[index].return_type.clone())
                .unwrap_or(Type::Unknown)
        };

        let result = match most_specific.len() {
            0 => {
                let provided: Vec<String> = arg_types.iter().map(|t| t.to_string()).collect();
                let mut message = format!(
                    "No version of '{name}' matches this call.\nYou provided ({}), but '{name}' accepts:",
                    provided.join(", ")
                );
                for &i in &arity_matches {
                    message.push_str(&format!(
                        "\n  {}",
                        crate::analyzer::format_signature(name, &signatures[i])
                    ));
                }
                self.type_error(message, None, None, line, column);
                Type::Error
            }
            1 => return_type_of(self, most_specific[0]),
            _ => {
                // Statically ambiguous — dynamic argument types, `nothing`
                // arguments, or equally-specific container-inheritance overlap:
                // the runtime dispatches on the actual values. Join all
                // reachable results so common structure and a known Nothing
                // path are preserved.
                most_specific
                    .iter()
                    .map(|&i| return_type_of(self, i))
                    .reduce(Self::join_inferred_types)
                    .unwrap_or(Type::Unknown)
            }
        };
        let selected_keys = most_specific
            .iter()
            .map(|index| (name.to_string(), *index))
            .collect::<Vec<_>>();
        if result != Type::Error {
            self.escape_user_action_list_arguments(arguments, &arg_types);
            self.apply_user_action_list_effects(&selected_keys);
        }
        self.escape_shared_list_return_type(&selected_keys, result)
    }

    fn infer_bare_action_statement(
        &mut self,
        name: &str,
        signatures: &[crate::analyzer::FunctionSignature],
        line: usize,
        column: usize,
    ) -> Type {
        self.infer_overloaded_call_type(name, signatures, &[], line, column)
    }

    fn infer_bare_variable_statement(
        &mut self,
        name: &str,
        line: usize,
        column: usize,
    ) -> Option<Type> {
        if let Some(resolution) = self
            .analyzer
            .alias_call_resolution(name, line, column)
            .cloned()
        {
            return match resolution {
                crate::analyzer::AliasState::Bound {
                    action,
                    visible_signatures,
                } => self.action_signatures(&action).map(|signatures| {
                    let visible = visible_signatures.min(signatures.len());
                    self.infer_bare_action_statement(&action, &signatures[..visible], line, column)
                }),
                crate::analyzer::AliasState::Builtin { .. } => None,
                crate::analyzer::AliasState::Dynamic => {
                    self.escape_all_visible_mutable_state();
                    Some(Type::Unknown)
                }
            };
        }

        self.action_signatures(name).and_then(|signatures| {
            (signatures.len() == 1
                || signatures
                    .iter()
                    .any(|signature| signature.parameters.is_empty()))
            .then(|| self.infer_bare_action_statement(name, &signatures, line, column))
        })
    }

    fn infer_zero_arg_variable_expression(
        &mut self,
        name: &str,
        line: usize,
        column: usize,
    ) -> Option<Type> {
        if let Some(resolution) = self
            .analyzer
            .alias_call_resolution(name, line, column)
            .cloned()
        {
            return match resolution {
                crate::analyzer::AliasState::Bound {
                    action,
                    visible_signatures,
                } => self.action_signatures(&action).and_then(|signatures| {
                    let visible = visible_signatures.min(signatures.len());
                    signatures[..visible]
                        .iter()
                        .any(|signature| signature.parameters.is_empty())
                        .then(|| {
                            self.infer_overloaded_call_type(
                                &action,
                                &signatures[..visible],
                                &[],
                                line,
                                column,
                            )
                        })
                }),
                crate::analyzer::AliasState::Builtin { .. } => None,
                crate::analyzer::AliasState::Dynamic => {
                    self.escape_all_visible_mutable_state();
                    Some(Type::Unknown)
                }
            };
        }

        if let Some(signatures) = self.action_signatures(name)
            && signatures
                .iter()
                .any(|signature| signature.parameters.is_empty())
        {
            return Some(self.infer_overloaded_call_type(name, &signatures, &[], line, column));
        }

        let symbol_type = self
            .analyzer
            .get_symbol(name)
            .and_then(|symbol| symbol.symbol_type.clone());
        if let Some(Type::Function {
            parameters,
            return_type,
        }) = symbol_type
            && parameters.is_empty()
        {
            // A stored method/native reference is auto-called by the runtime
            // when read as a variable. Its closure may touch captured state,
            // but unlike a named WFL action it has no keyed effect summary.
            self.escape_all_visible_mutable_state();
            return Some(*return_type);
        }
        None
    }

    pub fn check_types(&mut self, program: &Program) -> Result<(), TypeCheckError> {
        // A checker may be reused by an editor or other long-lived caller.
        // Diagnostics and program symbols belong to one run only.
        self.errors.clear();
        self.current_container = None;
        self.current_method_is_static = None;
        self.current_method_outer_property_bindings = None;
        self.checking_persistent_loop_backedge = false;
        self.list_alias_groups.clear();
        self.user_action_list_effects.clear();
        self.user_action_binding_effects.clear();
        self.user_action_shared_list_returns.clear();
        self.user_action_dependencies.clear();
        self.deferred_action_key_stack.clear();
        self.deferred_list_effect_stack.clear();
        self.deferred_binding_effect_stack.clear();
        self.deferred_return_type_stack.clear();
        self.try_flow_states.clear();
        self.try_flow_capture_suspended = 0;
        self.current_statement_completion = Type::Nothing;
        self.optional_refinement_origins.clear();
        self.has_websocket_handlers = false;
        self.definitely_nonempty_lists.clear();
        // A supplied, already-run analyzer is valid only for this first call.
        // Reuse must create and run a fresh analyzer for the next Program.
        let analyzer_was_pre_run = std::mem::take(&mut self.analyzer_already_run);
        if !analyzer_was_pre_run {
            self.analyzer = Analyzer::new();
        }

        // Reset the per-run budget breach so a reused TypeChecker (e.g. an editor
        // session) neither carries a stale breach nor lets the recursive
        // `check_statement_types` short-circuit fire against a previous run's
        // state.
        self.budget_error = None;

        // Detect includes: their exposed actions are only known at runtime.
        // Assign directly so a reused TypeChecker (e.g. an editor session) does
        // not carry a stale flag from a program that used includes.
        self.has_includes = crate::analyzer::program_has_includes(program);

        // Per-run overload return-type table: clear it like the other per-run
        // state above, so a reused TypeChecker never resolves an overloaded
        // call against a previous program's recorded return types (which a
        // forward-referenced overload would otherwise pick up instead of
        // falling back to Unknown).
        self.overload_returns.clear();

        // Only run the analyzer if it hasn't been run already
        // When created with with_analyzer(), the analyzer has already been run,
        // so we don't need to analyze again. This prevents duplicate symbol registration.
        if !analyzer_was_pre_run && let Err(semantic_errors) = self.analyzer.analyze(program) {
            // Propagate the analyzer's *typed* breach so an analysis-phase
            // deadline/cancellation/resource failure stays fatal and is never
            // mistaken for an ordinary semantic diagnostic.
            if let Some(breach) = self.analyzer.take_budget_error() {
                return Err(TypeCheckError::Budget(breach));
            }
            for error in semantic_errors {
                self.errors.push(TypeError::new(
                    error.message,
                    None,
                    None,
                    error.line,
                    error.column,
                ));
            }
            return Err(TypeCheckError::Types(self.errors.clone()));
        }

        for statement in &program.statements {
            // The budget is polled inside `check_statement_types` (below), which
            // runs for every top-level statement and recurses into nested bodies.
            // Stop iterating the moment a breach is recorded there — including one
            // surfaced deep inside a previous statement's nested body.
            if self.budget_error.is_some() {
                break;
            }
            self.check_statement_types(statement);
        }

        // A budget breach is fatal and takes precedence over any diagnostics
        // accumulated alongside it.
        if let Some(breach) = self.budget_error.take() {
            return Err(TypeCheckError::Budget(breach));
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(TypeCheckError::Types(self.errors.clone()))
        }
    }

    fn check_pattern_expression_types(
        &mut self,
        pattern: &PatternExpression,
        line: usize,
        column: usize,
    ) {
        let mut captures = std::collections::HashSet::new();
        self.check_pattern_expression_types_with_captures(pattern, line, column, &mut captures);
    }

    fn check_pattern_expression_types_with_captures(
        &mut self,
        pattern: &PatternExpression,
        line: usize,
        column: usize,
        captures: &mut std::collections::HashSet<String>,
    ) {
        match pattern {
            PatternExpression::Literal(_)
            | PatternExpression::CharacterClass(_)
            | PatternExpression::Anchor(_) => {
                // Leaf nodes are always valid
            }
            PatternExpression::Backreference(name) => {
                if !captures.contains(name) {
                    self.type_error(
                        format!("Backreference to undefined capture group: '{name}'"),
                        None,
                        None,
                        line,
                        column,
                    );
                }
            }
            PatternExpression::Quantified {
                pattern: inner_pattern,
                ..
            } => {
                self.check_pattern_expression_types_with_captures(
                    inner_pattern,
                    line,
                    column,
                    captures,
                );
            }
            PatternExpression::Sequence(patterns) | PatternExpression::Alternative(patterns) => {
                for inner_pattern in patterns {
                    self.check_pattern_expression_types_with_captures(
                        inner_pattern,
                        line,
                        column,
                        captures,
                    );
                }
            }
            PatternExpression::Capture {
                name,
                pattern: inner_pattern,
            } => {
                // The compiler registers a capture before compiling its inner
                // expression, so a recursive/self backreference follows the
                // same name-resolution order here.
                captures.insert(name.clone());
                self.check_pattern_expression_types_with_captures(
                    inner_pattern,
                    line,
                    column,
                    captures,
                );
            }
            PatternExpression::Lookahead(inner_pattern)
            | PatternExpression::NegativeLookahead(inner_pattern)
            | PatternExpression::Lookbehind(inner_pattern)
            | PatternExpression::NegativeLookbehind(inner_pattern) => {
                self.check_pattern_expression_types_with_captures(
                    inner_pattern,
                    line,
                    column,
                    captures,
                );
            }
            PatternExpression::ListReference(name) => {
                // TypeChecker delegates undefined variable checks to Analyzer.
                // If it can be inferred, we enforce List<Text> semantics.
                let var_type =
                    self.infer_expression_type(&Expression::Variable(name.clone(), line, column));
                match var_type {
                    Type::List(ref item_type) => {
                        if **item_type != Type::Text && !self.is_gradual_type(item_type) {
                            self.type_error(
                                format!("Pattern list reference '{name}' must contain Text, got List of {item_type}"),
                                Some(Type::List(Box::new(Type::Text))),
                                Some(var_type.clone()),
                                line,
                                column,
                            );
                        }
                    }
                    Type::Unknown | Type::Any | Type::Error => {}
                    _ => {
                        self.type_error(
                            format!(
                                "Pattern list reference '{name}' must be a List of Text, got {var_type}"
                            ),
                            Some(Type::List(Box::new(Type::Text))),
                            Some(var_type.clone()),
                            line,
                            column,
                        );
                    }
                }
            }
        }
    }

    fn check_server_expression_type(
        &mut self,
        server_expr: &Expression,
        line: usize,
        column: usize,
    ) {
        let server_type = self.infer_expression_type(server_expr);
        if server_type != Type::Text && !self.is_gradual_type(&server_type) {
            self.type_error(
                "Server must be a text string".to_string(),
                Some(Type::Text),
                Some(server_type),
                line,
                column,
            );
        }
    }

    fn check_statement_types(&mut self, statement: &Statement) -> Type {
        let previous_completion =
            std::mem::replace(&mut self.current_statement_completion, Type::Nothing);
        self.check_statement_types_inner(statement);
        let completion =
            std::mem::replace(&mut self.current_statement_completion, previous_completion);
        if self.budget_error.is_none() {
            self.capture_active_try_flow_state();
        }
        completion
    }

    fn check_statement_types_inner(&mut self, statement: &Statement) {
        // Recursive front-end checkpoint. This method recurses into `if`/loop/
        // `try`/action/container-method bodies, so polling the run budget here
        // (mirroring the parser's per-`parse_statement` placement) keeps deeply
        // nested type-checking cooperative with the deadline/cancellation/
        // operation limits — not just the top-level statement boundary. Once a
        // breach is recorded, short-circuit so it is captured a single time
        // rather than re-charged per nested node. The breach is kept on the
        // dedicated `budget_error` channel (callers must stop) AND pushed as a
        // diagnostic so `Err` still short-circuits normal reporting.
        if self.budget_error.is_some() {
            return;
        }
        if let Some(budget) = crate::exec::budget::ExecutionBudget::current()
            && let Err(exceeded) = budget.charge_operation(!budget.is_deadline_exempt())
        {
            self.errors
                .push(TypeError::new(exceeded.message(), None, None, 0, 0));
            self.budget_error = Some(exceeded);
            return;
        }

        match statement {
            Statement::PushStatement {
                list,
                value,
                line: _line,
                column: _column,
            } => {
                let (list_type, declared_property) = self.infer_list_mutation_target(list);
                let value_type = self.infer_expression_type(value);
                let declared_property_element = match declared_property.as_ref() {
                    Some((name, Type::List(element))) => Some((name.as_str(), (**element).clone())),
                    _ => None,
                };
                match &list_type {
                    Type::List(_) => {
                        let property_violation =
                            declared_property_element.as_ref().filter(|(_, expected)| {
                                !self.are_declared_property_values_compatible(
                                    expected,
                                    &value_type,
                                    value,
                                )
                            });
                        if let Some((name, expected)) = property_violation {
                            self.type_error(
                                format!(
                                    "Cannot push {value_type} into property '{name}' because its \
                                     declared element type is {expected}"
                                ),
                                Some(expected.clone()),
                                Some(value_type),
                                *_line,
                                *_column,
                            );
                        } else if declared_property_element.is_none() {
                            self.apply_list_mutation_effect(
                                list,
                                ListMutationEffect::Join(value_type),
                            );
                            self.record_list_insertion_aliases(list, value);
                        }
                    }
                    Type::Unknown | Type::Any | Type::Error => {
                        // A control-flow join may make the promoted alias
                        // itself gradual while its may-alias group still
                        // contains a precisely typed outer list.
                        self.apply_list_mutation_effect(list, ListMutationEffect::Join(value_type));
                        self.record_list_insertion_aliases(list, value);
                    }
                    _ => {
                        self.errors.push(TypeError::new(
                            format!("Expected list type for push operation, got {list_type:?}"),
                            Some(Type::List(Box::new(Type::Any))),
                            Some(list_type.clone()),
                            *_line,
                            *_column,
                        ));
                    }
                }
                if matches!(
                    list_type,
                    Type::List(_) | Type::Unknown | Type::Any | Type::Error
                ) && declared_property.is_none()
                {
                    self.mark_list_target_nonempty(list);
                }
            }
            Statement::RepeatWhileLoop {
                condition,
                body,
                line: _line,
                column: _column,
            } => {
                // Runtime keeps one child environment alive for every
                // iteration, so bindings from a backedge are visible at the
                // next header but remain local after the loop.
                self.analyzer.push_scope();
                let condition_type = self.infer_expression_type(condition);
                if condition_type != Type::Boolean && !self.is_gradual_type(&condition_type) {
                    self.errors.push(TypeError::new(
                        format!(
                            "Expected boolean condition in repeat-while loop, got {condition_type:?}"
                        ),
                        Some(Type::Boolean),
                        Some(condition_type),
                        *_line,
                        *_column,
                    ));
                }
                let first_condition_error_end = self.errors.len();
                self.check_persistent_loop_body_fixed_point(
                    body,
                    !matches!(condition, Expression::Literal(Literal::Boolean(false), ..)),
                );
                if self.budget_error.is_some() {
                    self.analyzer.pop_scope();
                    return;
                }

                let condition_type = self.infer_expression_type(condition);
                if condition_type != Type::Boolean && !self.is_gradual_type(&condition_type) {
                    self.errors.push(TypeError::new(
                        format!(
                            "Expected boolean condition in repeat-while loop, got {condition_type:?}"
                        ),
                        Some(Type::Boolean),
                        Some(condition_type.clone()),
                        *_line,
                        *_column,
                    ));
                }
                self.deduplicate_errors_from(first_condition_error_end);
                self.analyzer.pop_scope();
                self.current_statement_completion =
                    if matches!(condition, Expression::Literal(Literal::Boolean(false), ..)) {
                        Type::Nothing
                    } else {
                        Type::Any
                    };
            }
            Statement::ExitStatement { line: _, column: _ } => {}
            Statement::WaitForStatement {
                inner,
                line: _line,
                column: _column,
            } => {
                self.current_statement_completion = self.check_statement_types(inner);
            }
            Statement::WaitForDurationStatement {
                duration,
                line: _line,
                column: _column,
                ..
            } => {
                let duration_type = self.infer_expression_type(duration);
                if duration_type != Type::Number && !self.is_gradual_type(&duration_type) {
                    self.type_error(
                        "Expected a number for wait duration".to_string(),
                        Some(Type::Number),
                        Some(duration_type),
                        *_line,
                        *_column,
                    );
                }
                if self.has_websocket_handlers {
                    // Runtime pumps registered WebSocket handlers throughout
                    // this wait. Their deferred bodies are an opaque captured-
                    // environment boundary until handler-specific summaries
                    // become part of the public type model.
                    self.escape_all_visible_mutable_state();
                }
            }
            Statement::TryStatement {
                body,
                when_clauses,
                otherwise_block,
                finally_block,
                line: _line,
                column: _column,
            } => {
                self.definitely_nonempty_lists.clear();
                // Runtime evaluates the try body, handlers, otherwise, and
                // finally block inside one shared child environment.
                self.analyzer.push_scope();
                let try_scope_entry_symbols = self.analyzer.snapshot_current_scope_symbols();
                let entry_types = self.analyzer.snapshot_symbol_types();
                let try_entry_visible_names = entry_types
                    .iter()
                    .flat_map(|layer| layer.keys().cloned())
                    .collect::<HashSet<_>>();
                let entry_aliases = self.list_alias_groups.clone();
                let summary_entry = self.snapshot_deferred_summary();
                self.try_flow_states.push(TryFlowAccumulator {
                    binding_types: self.analyzer.live_binding_types().into_iter().collect(),
                    list_aliases: entry_aliases.clone(),
                });
                let (success_can_continue, success_completion) =
                    self.check_statement_block_with_completion(body);
                let mut body_flow = self.try_flow_states.pop().unwrap_or_default();
                if self.budget_error.is_some() {
                    self.analyzer.pop_scope();
                    return;
                }
                let success_endpoint = self.analyzer.snapshot_symbol_types();
                let success_aliases = self.list_alias_groups.clone();

                // An error can leave the body after any reachable nested
                // statement. The streaming accumulator joins every such state
                // without retaining one full snapshot per prefix.
                let handler_entry = self
                    .apply_try_binding_accumulator(entry_types.clone(), &body_flow.binding_types);
                self.retain_live_alias_paths(&mut body_flow.list_aliases);
                let handler_entry_aliases = body_flow.list_aliases;
                let success_scope_symbols = self.analyzer.snapshot_current_scope_symbols();
                let mut endpoint_scope_symbols = vec![success_scope_symbols.clone()];
                let mut endpoints = vec![success_endpoint];
                let mut endpoint_aliases = vec![success_aliases];
                let mut continuation_endpoints = Vec::new();
                let mut continuation_aliases = Vec::new();
                let mut continuation_completion_types = Vec::new();
                if success_can_continue {
                    continuation_endpoints.push(endpoints[0].clone());
                    continuation_aliases.push(endpoint_aliases[0].clone());
                    continuation_completion_types.push(success_completion);
                }

                // Type check each when clause in its own scope so the bound
                // error name cannot clobber an outer variable of the same
                // name. Required now that get_symbol_mut walks parents
                // (issue #605 / PR review on #606). Use define_or_replace so
                // the binding lives only in the child scope (runtime does the
                // same via Environment::define_or_replace).
                for when_clause in when_clauses {
                    self.analyzer
                        .restore_current_scope_symbols(try_scope_entry_symbols.clone());
                    self.analyzer.restore_symbol_types(handler_entry.clone());
                    self.list_alias_groups = handler_entry_aliases.clone();
                    self.analyzer.push_scope();
                    self.analyzer.define_or_replace_symbol(Symbol {
                        name: when_clause.error_name.clone(),
                        kind: SymbolKind::Variable { mutable: false },
                        symbol_type: Some(Type::Text), // Errors are represented as text
                        line: *_line,
                        column: *_column,
                    });
                    // `error_message` is always available as an alias, matching
                    // the analyzer and runtime.
                    if when_clause.error_name != "error_message" {
                        self.analyzer.define_or_replace_symbol(Symbol {
                            name: "error_message".to_string(),
                            kind: SymbolKind::Variable { mutable: false },
                            symbol_type: Some(Type::Text),
                            line: *_line,
                            column: *_column,
                        });
                    }

                    let (handler_can_continue, handler_completion) =
                        self.check_statement_block_with_completion(&when_clause.body);
                    let mut excluded_aliases = vec![when_clause.error_name.clone()];
                    if when_clause.error_name != "error_message" {
                        excluded_aliases.push("error_message".to_string());
                    }
                    let promoted = self.analyzer.pop_scope_promoting_except(&excluded_aliases);
                    self.merge_promoted_list_alias_bindings(promoted);

                    if self.budget_error.is_some() {
                        self.analyzer.pop_scope();
                        return;
                    }

                    let endpoint = self.analyzer.snapshot_symbol_types();
                    let aliases = self.list_alias_groups.clone();
                    endpoints.push(endpoint.clone());
                    endpoint_aliases.push(aliases.clone());
                    if handler_can_continue {
                        continuation_endpoints.push(endpoint);
                        continuation_aliases.push(aliases);
                        continuation_completion_types.push(handler_completion);
                    }
                    endpoint_scope_symbols.push(self.analyzer.snapshot_current_scope_symbols());
                }

                if let Some(otherwise_stmts) = otherwise_block {
                    self.analyzer
                        .restore_current_scope_symbols(try_scope_entry_symbols.clone());
                    self.analyzer.restore_symbol_types(handler_entry.clone());
                    self.list_alias_groups = handler_entry_aliases.clone();
                    let (otherwise_can_continue, otherwise_completion) =
                        self.check_statement_block_with_completion(otherwise_stmts);
                    if self.budget_error.is_some() {
                        self.analyzer.pop_scope();
                        return;
                    }

                    let endpoint = self.analyzer.snapshot_symbol_types();
                    let aliases = self.list_alias_groups.clone();
                    endpoints.push(endpoint.clone());
                    endpoint_aliases.push(aliases.clone());
                    if otherwise_can_continue {
                        continuation_endpoints.push(endpoint);
                        continuation_aliases.push(aliases);
                        continuation_completion_types.push(otherwise_completion);
                    }
                    endpoint_scope_symbols.push(self.analyzer.snapshot_current_scope_symbols());
                } else if !when_clauses.iter().any(|when_clause| {
                    matches!(
                        &when_clause.error_type,
                        crate::parser::ast::ErrorType::General
                    )
                }) {
                    // A non-matching error reaches finally without running a
                    // handler when there is no catch-all or otherwise block.
                    endpoints.push(handler_entry.clone());
                    endpoint_aliases.push(handler_entry_aliases.clone());
                    endpoint_scope_symbols.push(try_scope_entry_symbols.clone());
                }

                let mut definite_scope_symbols = success_scope_symbols;
                for symbols in &endpoint_scope_symbols {
                    for (name, symbol) in symbols {
                        definite_scope_symbols
                            .entry(name.clone())
                            .or_insert_with(|| symbol.clone());
                    }
                }
                definite_scope_symbols.retain(|name, _| {
                    try_entry_visible_names.contains(name)
                        || endpoint_scope_symbols
                            .iter()
                            .all(|symbols| symbols.contains_key(name))
                });
                self.analyzer
                    .restore_current_scope_symbols(definite_scope_symbols);
                let joined_endpoint = Self::join_type_snapshots(&endpoints);
                self.analyzer.restore_symbol_types(joined_endpoint);
                self.list_alias_groups = Self::join_list_alias_snapshots(&endpoint_aliases);

                if let Some(finally_stmts) = finally_block {
                    let pre_finally_scope_symbols = self.analyzer.snapshot_current_scope_symbols();
                    let primary_summary = self.snapshot_deferred_summary();
                    let primary_return_len = primary_summary.returns.as_ref().map_or(0, Vec::len);
                    let finally_error_start = self.errors.len();
                    let finally_can_continue = self.check_statement_block(finally_stmts);
                    let mut final_summary = self.snapshot_deferred_summary();

                    if !finally_can_continue {
                        // A return/exit/break/continue from finally overrides
                        // the primary control flow. Preserve effects that
                        // happened before finally, but discard primary return
                        // values in favor of the returns produced by finally.
                        if let Some(all_returns) = &final_summary.returns {
                            let mut selected = summary_entry.returns.clone().unwrap_or_default();
                            selected.extend(all_returns.iter().skip(primary_return_len).cloned());
                            if let Some(active) = self.deferred_return_type_stack.last_mut() {
                                *active = selected.clone();
                            }
                            final_summary.returns = Some(selected);
                        }
                    }

                    if finally_can_continue && !continuation_endpoints.is_empty() {
                        // Ordinary finally preserves the primary endpoint's
                        // control flow. Re-apply its state transform to only
                        // the endpoints that can actually reach the statement
                        // after this try; abrupt Return/Exit/error endpoints
                        // must still be considered while validating finally,
                        // but cannot pollute the post-try state.
                        self.analyzer
                            .restore_current_scope_symbols(pre_finally_scope_symbols);
                        self.analyzer
                            .restore_symbol_types(Self::join_type_snapshots(
                                &continuation_endpoints,
                            ));
                        self.list_alias_groups =
                            Self::join_list_alias_snapshots(&continuation_aliases);
                        self.restore_deferred_summary(primary_summary);
                        self.check_statement_block(finally_stmts);
                        self.restore_deferred_summary(final_summary);
                        self.deduplicate_errors_from(finally_error_start);
                    } else if finally_can_continue {
                        self.analyzer.restore_symbol_types(entry_types);
                        self.list_alias_groups = entry_aliases;
                    }
                } else if !continuation_endpoints.is_empty() {
                    self.analyzer
                        .restore_symbol_types(Self::join_type_snapshots(&continuation_endpoints));
                    self.list_alias_groups = Self::join_list_alias_snapshots(&continuation_aliases);
                } else {
                    self.analyzer.restore_symbol_types(entry_types);
                    self.list_alias_groups = entry_aliases;
                }
                self.current_statement_completion = continuation_completion_types
                    .into_iter()
                    .reduce(Self::join_inferred_types)
                    .unwrap_or(Type::Nothing);
                self.analyzer.pop_scope();
            }
            Statement::HttpGetStatement {
                url,
                variable_name,
                line: _line,
                column: _column,
            } => {
                let url_type = self.infer_expression_type(url);
                if url_type != Type::Text && !self.is_gradual_type(&url_type) {
                    self.type_error(
                        "URL must be a text string".to_string(),
                        Some(Type::Text),
                        Some(url_type),
                        *_line,
                        *_column,
                    );
                }

                self.bind_runtime_value(variable_name, Type::Text, true, *_line, *_column);
            }
            Statement::HttpPostStatement {
                url,
                data,
                variable_name,
                line: _line,
                column: _column,
            } => {
                let url_type = self.infer_expression_type(url);
                if url_type != Type::Text && !self.is_gradual_type(&url_type) {
                    self.type_error(
                        "URL must be a text string".to_string(),
                        Some(Type::Text),
                        Some(url_type),
                        *_line,
                        *_column,
                    );
                }

                let data_type = self.infer_expression_type(data);
                if data_type != Type::Text && !self.is_gradual_type(&data_type) {
                    self.type_error(
                        "HTTP POST data must be text".to_string(),
                        Some(Type::Text),
                        Some(data_type),
                        *_line,
                        *_column,
                    );
                }

                self.bind_runtime_value(variable_name, Type::Text, true, *_line, *_column);
            }
            Statement::HttpRequestStatement {
                url,
                method,
                headers,
                body,
                variable_name,
                full_response,
                line: _line,
                column: _column,
            } => {
                let url_type = self.infer_expression_type(url);
                if url_type != Type::Text && !self.is_gradual_type(&url_type) {
                    self.type_error(
                        "URL must be a text string".to_string(),
                        Some(Type::Text),
                        Some(url_type),
                        *_line,
                        *_column,
                    );
                }

                if let Some(method) = method {
                    let method_type = self.infer_expression_type(method);
                    if method_type != Type::Text && !self.is_gradual_type(&method_type) {
                        self.type_error(
                            "HTTP method must be a text string".to_string(),
                            Some(Type::Text),
                            Some(method_type),
                            *_line,
                            *_column,
                        );
                    }
                }
                if let Some(headers) = headers {
                    let headers_type = self.infer_expression_type(headers);
                    if !self.is_valid_header_map_type(&headers_type) {
                        self.type_error(
                            "HTTP headers must be a map of header names to values".to_string(),
                            Some(Type::Map(Box::new(Type::Text), Box::new(Type::Any))),
                            Some(headers_type),
                            *_line,
                            *_column,
                        );
                    }
                }
                if let Some(body) = body {
                    // Text is expected; numbers and booleans are converted at
                    // runtime, so only reject clearly wrong types
                    let body_type = self.infer_expression_type(body);
                    if !matches!(
                        body_type,
                        Type::Text
                            | Type::Number
                            | Type::Boolean
                            | Type::Unknown
                            | Type::Any
                            | Type::Error
                    ) {
                        self.type_error(
                            "HTTP request body must be text, a number, or a boolean (numbers and booleans are converted to text)".to_string(),
                            // No single "expected" type — the accepted set is
                            // Text|Number|Boolean, so a bare `Text` hint would
                            // misrender the expected-vs-actual diagnostic.
                            None,
                            Some(body_type),
                            *_line,
                            *_column,
                        );
                    }
                }

                self.bind_runtime_value(
                    variable_name,
                    if *full_response {
                        Type::Map(Box::new(Type::Text), Box::new(Type::Any))
                    } else {
                        Type::Text
                    },
                    true,
                    *_line,
                    *_column,
                );
            }
            Statement::HttpStreamStatement {
                url,
                method,
                headers,
                body,
                variable_name,
                line: _line,
                column: _column,
            } => {
                let url_type = self.infer_expression_type(url);
                if url_type != Type::Text && !self.is_gradual_type(&url_type) {
                    self.type_error(
                        "URL must be a text string".to_string(),
                        Some(Type::Text),
                        Some(url_type),
                        *_line,
                        *_column,
                    );
                }
                if let Some(method) = method {
                    let method_type = self.infer_expression_type(method);
                    if method_type != Type::Text && !self.is_gradual_type(&method_type) {
                        self.type_error(
                            "HTTP method must be a text string".to_string(),
                            Some(Type::Text),
                            Some(method_type),
                            *_line,
                            *_column,
                        );
                    }
                }
                if let Some(headers) = headers {
                    let headers_type = self.infer_expression_type(headers);
                    if !self.is_valid_header_map_type(&headers_type) {
                        self.type_error(
                            "HTTP headers must be a map of header names to values".to_string(),
                            Some(Type::Map(Box::new(Type::Text), Box::new(Type::Any))),
                            Some(headers_type),
                            *_line,
                            *_column,
                        );
                    }
                }
                if let Some(body) = body {
                    let body_type = self.infer_expression_type(body);
                    if !matches!(
                        body_type,
                        Type::Text
                            | Type::Number
                            | Type::Boolean
                            | Type::Unknown
                            | Type::Any
                            | Type::Error
                    ) {
                        self.type_error(
                            "HTTP request body must be text, a number, or a boolean (numbers and booleans are converted to text)".to_string(),
                            // No single "expected" type — the accepted set is
                            // Text|Number|Boolean, so a bare `Text` hint would
                            // misrender the expected-vs-actual diagnostic.
                            None,
                            Some(body_type),
                            *_line,
                            *_column,
                        );
                    }
                }

                // Binds an outbound streaming-response handle (exposes
                // status/ok/headers via index/member access, and is closeable).
                // A distinct handle type — not a bare `Map` — so `close` accepts
                // it without also accepting an ordinary user map.
                self.bind_runtime_value(
                    variable_name,
                    Type::Custom("HttpStream".to_string()),
                    true,
                    *_line,
                    *_column,
                );
            }
            Statement::WaitForNextChunkStatement {
                source,
                variable_name,
                line,
                column,
            }
            | Statement::WaitForNextLineStatement {
                source,
                variable_name,
                line,
                column,
            } => {
                // The source must be an outbound stream handle. Gradual types
                // (Unknown/Any/Error) pass; a concrete non-stream operand is a
                // static error rather than a runtime "not a stream" surprise.
                let source_type = self.infer_expression_type(source);
                if !self.is_http_stream_source_type(&source_type) {
                    self.type_error(
                        "`wait for next chunk|line` requires an outbound stream handle \
                         (from `open url ... and stream response as ...`)"
                            .to_string(),
                        Some(Type::Custom("HttpStream".to_string())),
                        Some(source_type),
                        *line,
                        *column,
                    );
                }
                let value_type = if matches!(statement, Statement::WaitForNextChunkStatement { .. })
                {
                    Type::Binary
                } else {
                    Type::Text
                };
                self.bind_runtime_value(
                    variable_name,
                    Type::Optional(Box::new(value_type)),
                    true,
                    *line,
                    *column,
                );
            }
            Statement::StartStreamingResponseStatement {
                request,
                status,
                content_type,
                headers,
                variable_name,
                line: _line,
                column: _column,
            } => {
                let request_type = self.infer_expression_type(request);
                if !self.is_pending_request_type(&request_type) {
                    self.type_error(
                        "Streaming response target must be a request object".to_string(),
                        Some(Type::Custom("Request".to_string())),
                        Some(request_type.clone()),
                        *_line,
                        *_column,
                    );
                }
                // Enforce the clause types (like RespondStatement) so obvious
                // mistakes fail at typecheck rather than at runtime.
                if let Some(status) = status {
                    let status_type = self.infer_expression_type(status);
                    if !matches!(
                        status_type,
                        Type::Number | Type::Unknown | Type::Any | Type::Error
                    ) {
                        self.type_error(
                            "Streaming response status must be a number".to_string(),
                            Some(Type::Number),
                            Some(status_type),
                            *_line,
                            *_column,
                        );
                    }
                }
                if let Some(content_type) = content_type {
                    let ct_type = self.infer_expression_type(content_type);
                    if !matches!(
                        ct_type,
                        Type::Text | Type::Unknown | Type::Any | Type::Error
                    ) {
                        self.type_error(
                            "Streaming response content type must be text".to_string(),
                            Some(Type::Text),
                            Some(ct_type),
                            *_line,
                            *_column,
                        );
                    }
                }
                if let Some(headers) = headers {
                    let headers_type = self.infer_expression_type(headers);
                    if !self.is_valid_header_map_type(&headers_type) {
                        self.type_error(
                            "Streaming response headers must be a map of header names to values"
                                .to_string(),
                            Some(Type::Map(Box::new(Type::Text), Box::new(Type::Any))),
                            Some(headers_type),
                            *_line,
                            *_column,
                        );
                    }
                }
                if !variable_name.is_empty() {
                    // A distinct server-response-stream handle type (not a bare
                    // `Map`) so `close out` is accepted without `close` also
                    // type-checking an ordinary user map.
                    //
                    // Always bind in the *current* scope only (shadow, do not
                    // mutate an outer symbol of the same name via get_symbol_mut
                    // parent walk). Analyzer loop scopes are discarded after
                    // body analysis, so we re-create the binding here.
                    self.analyzer.define_or_replace_symbol(Symbol {
                        name: variable_name.clone(),
                        kind: SymbolKind::Variable { mutable: true },
                        symbol_type: Some(Type::Custom("ResponseStream".to_string())),
                        line: *_line,
                        column: *_column,
                    });
                }
            }
            Statement::StreamWriteStatement {
                value,
                target,
                fallback_content,
                line,
                column,
                ..
            } => {
                // Branch-aware: the runtime picks the reading by the TARGET's type
                // (a response-stream handle -> stream write of `value`; anything
                // else with a fallback -> classic file write of `fallback_content`).
                // Type-check the reading the runtime will actually take, so a valid
                // pre-existing file write is never rejected on the stream branch it
                // never runs (and vice versa).
                let target_type = self.infer_expression_type(target);
                let has_fallback = fallback_content.is_some();

                if self.is_response_stream_target_type(&target_type)
                    && !self.is_gradual_type(&target_type)
                {
                    // Concrete response stream: the stream reading is taken.
                    // Report undefined names on this branch (analyzer may have
                    // stayed silent because the classic lead alone was defined).
                    self.check_expression_names_defined(value);
                    let value_type = self.infer_expression_type(value);
                    self.check_streamable_payload(&value_type, *line, *column);
                } else if has_fallback
                    && (matches!(target_type, Type::Text)
                        || matches!(&target_type, Type::Custom(n) if n == "File"))
                {
                    // Concrete text path OR open-file handle (`Custom("File")`):
                    // the classic file-write reading is taken. Validate the
                    // fallback (including definedness), not the stream `value`.
                    if let Some(fallback) = fallback_content {
                        self.check_expression_names_defined(fallback);
                        let _ = self.infer_expression_type(fallback);
                    }
                } else if self.is_gradual_type(&target_type) {
                    // Gradual/unknown target: both readings are viable and the
                    // runtime decides by the target's runtime type. Conservatively
                    // validate EVERY viable branch (not "accept if either is ok"),
                    // so a valid file fallback cannot mask an invalid stream
                    // payload or an undefined stream lead (issue #642).
                    self.check_expression_names_defined(value);
                    let value_type = self.infer_expression_type(value);
                    self.check_streamable_payload(&value_type, *line, *column);
                    if let Some(fallback) = fallback_content {
                        self.check_expression_names_defined(fallback);
                        let _ = self.infer_expression_type(fallback);
                    }
                } else {
                    // Concrete non-stream, non-text target (or a text target with no
                    // fallback): an unambiguous stream write to the wrong type.
                    self.type_error(
                        "`write line|chunk` requires a response-stream handle \
                         (from `start streaming response ... as ...`)"
                            .to_string(),
                        Some(Type::Custom("ResponseStream".to_string())),
                        Some(target_type),
                        *line,
                        *column,
                    );
                }
            }
            Statement::FlushStreamStatement {
                target,
                legacy_binding,
                action_fallback,
                line,
                column,
            } => {
                // Legacy full-name expression (e.g. Variable("flush cache") or
                // IndexAccess over it): when its root is bound, typecheck that
                // expression. Otherwise this is a stream flush.
                let is_expression_fallback = legacy_binding
                    .as_deref()
                    .is_some_and(|name| self.name_is_defined_for_write(name));
                if is_expression_fallback {
                    if let Some(fb) = action_fallback {
                        let _ = self.infer_expression_type(fb);
                    }
                    // Exact `flush (…)` / `flush call …`: the runtime also
                    // evaluates the operand as its own legacy expression
                    // statement, so typecheck it as an ordinary expression —
                    // no response-stream requirement (#642).
                    if legacy_binding.as_deref() == Some("flush") {
                        let _ = self.infer_expression_type(target);
                    }
                } else {
                    let target_type = self.infer_expression_type(target);
                    if !self.is_response_stream_target_type(&target_type) {
                        self.type_error(
                            "`flush` requires a response-stream handle \
                             (from `start streaming response ... as ...`)"
                                .to_string(),
                            Some(Type::Custom("ResponseStream".to_string())),
                            Some(target_type),
                            *line,
                            *column,
                        );
                    }
                }
            }
            Statement::VariableDeclaration {
                name,
                value,
                is_constant,
                line: _line,
                column: _column,
            } => {
                let is_definitely_nonempty = self.expression_is_definitely_nonempty_list(value);
                let inferred_type = self.infer_expression_type(value);

                // Special case for loopcounter variable
                if name == "loopcounter" {
                    // Skip type inference error for loopcounter
                    return;
                }

                // Under gradual typing, an inferred `Unknown` means "statically
                // unknown", not "known incompatible": e.g. `store x as helper of ...`
                // where `helper` returns an expression built from its untyped
                // parameters has an `Unknown` return type. Bind `x` as `Unknown`
                // silently rather than raising a false `Could not infer type`
                // ERROR (issue #588), mirroring #587's treatment of variable
                // references. The type-compatibility and symbol-recording paths
                // below still record the more specific type when one is available.

                let (resolved_type, is_property) = self.resolve_bare_mutation_target_type(name);
                let declared_property_type = is_property.then_some(resolved_type.clone()).flatten();
                let symbol_type_option = (!is_property).then_some(resolved_type).flatten();

                let need_type_error = if let Some(declared_type) = &declared_property_type {
                    !self.are_declared_property_values_compatible(
                        declared_type,
                        &inferred_type,
                        value,
                    )
                } else if let Some(declared_type) = &symbol_type_option {
                    !self.are_types_compatible(declared_type, &inferred_type)
                } else {
                    false
                };

                if need_type_error {
                    self.type_error(
                        if let Some(expected) = &declared_property_type {
                            format!(
                                "Cannot initialize property '{name}' with {inferred_type} because \
                                 its declared type is {expected}"
                            )
                        } else {
                            format!("Cannot initialize variable '{name}' with incompatible type")
                        },
                        declared_property_type
                            .clone()
                            .or(symbol_type_option.clone()),
                        Some(inferred_type.clone()),
                        *_line,
                        *_column,
                    );
                }
                if declared_property_type.is_some() {
                    // Runtime container properties live in the method
                    // environment and shadow outer lexical bindings. Their
                    // declared registry type remains the source of truth.
                    if *is_constant {
                        self.type_error(
                            format!(
                                "Cannot redeclare container property '{name}' as a constant; \
                                 the property binding already exists"
                            ),
                            declared_property_type,
                            Some(inferred_type),
                            *_line,
                            *_column,
                        );
                    }
                    return;
                }

                // Locals declared inside an action body have no symbol left
                // over from analysis (the analyzer discards body scopes), so
                // record them in the type checker's re-created scope; later
                // statements in the body can then see their inferred types
                // (issue #553). Resolving through parent scopes is correct
                // here because WFL forbids shadowing: a `store` reusing an
                // outer variable's name is a fatal semantic error ("Use
                // 'change x to <value>'"), so a resolved outer symbol can
                // only mean the store refers to that same variable.
                let recorded_type = if inferred_type == Type::Error {
                    Type::Unknown
                } else {
                    inferred_type
                };
                let alias_value_type = recorded_type.clone();
                if self.analyzer.get_local_symbol(name).is_some() {
                    if *is_constant && self.checking_persistent_loop_backedge {
                        self.type_error(
                            format!(
                                "Constant '{name}' is redeclared when the persistent loop \
                                 reaches another iteration"
                            ),
                            None,
                            None,
                            *_line,
                            *_column,
                        );
                    }
                    // Fixed-point loop checking revisits the same declaration
                    // under a widened header. The declaration executes before
                    // its later uses on every iteration, so it replaces that
                    // header type with the freshly inferred value just as the
                    // runtime replaces the binding.
                    if let Some(symbol) = self.analyzer.get_symbol_mut(name) {
                        symbol.symbol_type = Some(recorded_type);
                    }
                } else {
                    let _ = self.analyzer.define_symbol(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Variable {
                            mutable: !is_constant,
                        },
                        symbol_type: Some(recorded_type),
                        line: *_line,
                        column: *_column,
                    });
                }
                self.detach_list_alias_binding(name);
                self.record_direct_list_alias(name, value, &alias_value_type);
                self.record_nested_list_aliases(name, value);
                self.update_binding_nonempty_fact(name, is_definitely_nonempty);
            }
            Statement::Assignment {
                name,
                value,
                line,
                column,
            } => {
                let is_definitely_nonempty = self.expression_is_definitely_nonempty_list(value);
                let inferred_type = self.infer_expression_type(value);
                let alias_value_type = inferred_type.clone();
                let mut captured_alias_sources = Vec::new();
                self.capture_nested_list_alias_sources(value, 0, &mut captured_alias_sources);

                // Clone the existing type first so we can re-borrow mutably below
                // when widening away from Nothing (issue #605).
                let (resolved_type, is_property) = self.resolve_bare_mutation_target_type(name);
                let declared_property_type = is_property.then_some(resolved_type.clone()).flatten();
                let symbol_type = (!is_property).then_some(resolved_type).flatten();
                let existing_type = symbol_type.or_else(|| declared_property_type.clone());

                match existing_type {
                    // `store x as nothing` is the idiomatic "uninitialized"
                    // sentinel. Reassignment must widen the stored type to the
                    // new value's type; otherwise later indexing/use stays
                    // pinned to Nothing and raises false
                    // "Cannot index into Nothing" diagnostics (issue #605).
                    Some(Type::Nothing) if declared_property_type.is_none() => {
                        if inferred_type != Type::Nothing
                            && inferred_type != Type::Error
                            && let Some(symbol) = self.analyzer.get_symbol_mut(name)
                        {
                            symbol.symbol_type = Some(inferred_type);
                        }
                    }
                    Some(variable_type) => {
                        let is_compatible = if declared_property_type.is_some() {
                            self.are_declared_property_values_compatible(
                                &variable_type,
                                &inferred_type,
                                value,
                            )
                        } else {
                            self.are_types_compatible(&variable_type, &inferred_type)
                        };
                        if !is_compatible {
                            self.type_error(
                                if declared_property_type.is_some() {
                                    format!(
                                        "Cannot assign {inferred_type} to property '{name}' because \
                                         its declared type is {variable_type}"
                                    )
                                } else {
                                    format!(
                                        "Cannot assign value of incompatible type to variable \
                                         '{name}'"
                                    )
                                },
                                Some(variable_type),
                                Some(inferred_type),
                                *line,
                                *column,
                            );
                        } else if inferred_type != Type::Error
                            && let Some(symbol) = self.analyzer.get_symbol_mut(name)
                        {
                            // `change` replaces the current runtime value.
                            // Flow-sensitive state must therefore record the
                            // assigned value itself, including Nothing, rather
                            // than retaining a stale pre-assignment type.
                            symbol.symbol_type = Some(inferred_type);
                        }
                    }
                    None => {
                        // Symbol exists but has no recorded type yet, or the
                        // name is unresolved (undefined-variable is reported
                        // elsewhere). Record a concrete type when available.
                        if self.analyzer.get_symbol(name).is_some()
                            && inferred_type != Type::Error
                            && inferred_type != Type::Unknown
                            && let Some(symbol) = self.analyzer.get_symbol_mut(name)
                        {
                            symbol.symbol_type = Some(inferred_type);
                        }
                    }
                }
                if declared_property_type.is_some() {
                    // Runtime container properties shadow outer lexical
                    // bindings. Do not accidentally detach aliases or record
                    // deferred effects against a same-named outer variable.
                    return;
                }
                self.record_deferred_binding_assignment(name);
                self.record_deferred_list_rebind(name, value, &alias_value_type);
                self.detach_list_alias_binding(name);
                self.restore_captured_list_alias_sources(name, captured_alias_sources);
                if matches!(
                    value,
                    Expression::MemberAccess { .. }
                        | Expression::PropertyAccess { .. }
                        | Expression::MethodCall { .. }
                ) {
                    self.record_direct_list_alias(name, value, &alias_value_type);
                }
                self.update_binding_nonempty_fact(name, is_definitely_nonempty);
            }
            Statement::ActionDefinition {
                name,
                parameters,
                body,
                return_type,
                line: _line,
                column: _column,
            } => {
                let param_types = parameters
                    .iter()
                    .map(|p| p.param_type.as_ref().cloned().unwrap_or(Type::Unknown))
                    .collect::<Vec<Type>>();

                // WFL has no return-type annotation syntax, so `return_type` is
                // effectively always `None`; seed a provisional return type
                // (so recursive calls in the body resolve to a function type),
                // then refine it below by inferring from the body's `return`
                // expressions (issue #569).
                //
                // Seed with `Unknown` rather than `Nothing`. The body is
                // type-checked before the real return type is inferred (#575's
                // ordering), so a self-recursive call in the body resolves
                // against this provisional type. `Nothing` made such a call — and
                // any indexing/use of its result — raise strict "found Nothing"
                // errors (issue #590). After #588/#589 an `Unknown`-typed value
                // degrades gracefully, so `Unknown` resolves self-references
                // cleanly while still avoiding the spurious "Expected Text but
                // found Nothing" errors that motivated seeding a concrete type
                // in builtin positions (e.g. `respond to req with ...`, #569).
                let return_type_value = return_type.as_ref().cloned().unwrap_or(Type::Unknown);

                // Nested action symbols lived only in the analyzer's discarded
                // body scope. Re-create their ownership in the active scope so
                // calls and local-only exports observe runtime scope rules.
                if self.analyzer.get_local_symbol(name).is_none() {
                    let _ = self.analyzer.define_symbol(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Function {
                            signatures: vec![crate::analyzer::FunctionSignature {
                                parameters: parameters.clone(),
                                return_type: return_type.clone(),
                            }],
                        },
                        symbol_type: Some(Type::Function {
                            parameters: param_types.clone(),
                            return_type: Box::new(return_type_value.clone()),
                        }),
                        line: *_line,
                        column: *_column,
                    });
                }

                if let Some(symbol) = self.analyzer.get_symbol_mut(name) {
                    symbol.symbol_type = Some(Type::Function {
                        parameters: param_types.clone(),
                        return_type: Box::new(return_type_value),
                    });
                }

                // The analyzer's action-body scope is discarded when analysis
                // finishes, so re-create one here: parameters and body-local
                // variables must be resolvable while checking the body,
                // otherwise expressions that depend on a local's type (e.g.
                // `parts[0]` after `store parts as ...`) infer Unknown
                // (issue #553).
                self.analyzer.push_scope();
                for param in parameters {
                    let param_symbol = Symbol {
                        name: param.name.clone(),
                        kind: SymbolKind::Variable { mutable: false },
                        // Untyped parameters get an explicit Unknown so
                        // references resolve without a "cannot determine
                        // type" diagnostic, matching prior behavior.
                        symbol_type: param.param_type.clone().or(Some(Type::Unknown)),
                        line: param.line,
                        column: param.column,
                    };
                    self.analyzer.define_or_replace_symbol(param_symbol);
                }

                // Snapshot before the body so Nothing-widening (and other
                // refinements) of outer variables during the definition check
                // do not permanently stick after the action is defined but
                // never called (PR #606 Codex review).
                let outer_type_snapshot = self.analyzer.snapshot_symbol_types();
                let outer_alias_snapshot = self.list_alias_groups.clone();
                let outer_refinement_snapshot = self.optional_refinement_origins.clone();
                let outer_nonempty_snapshot = self.definitely_nonempty_lists.clone();
                let signature_index = self.signature_index_for(name, parameters).unwrap_or(0);
                let summary_key = (name.clone(), signature_index);
                self.deferred_action_key_stack.push(summary_key.clone());
                self.deferred_list_effect_stack.push(HashSet::new());
                self.deferred_binding_effect_stack.push(HashMap::new());
                self.deferred_return_type_stack.push(Vec::new());

                self.try_flow_capture_suspended += 1;
                let (body_can_continue, implicit_completion) =
                    self.check_statement_block_with_completion(body);
                self.try_flow_capture_suspended -= 1;

                let recorded_returns = self.deferred_return_type_stack.pop().unwrap_or_default();
                let mut implicit_list_sources = Vec::new();
                if body_can_continue && Self::type_may_contain_list(&implicit_completion) {
                    self.capture_block_completion_list_sources(body, 0, &mut implicit_list_sources);
                }
                let inferred_return = if return_type.is_none() {
                    Some(Self::infer_recorded_action_return_type(
                        &recorded_returns,
                        body_can_continue.then_some(&implicit_completion),
                    ))
                } else {
                    None
                };

                if let Some(ret_type) = return_type {
                    self.check_recorded_return_types(&recorded_returns, ret_type);
                    if body_can_continue {
                        self.check_implicit_action_result(
                            &implicit_completion,
                            ret_type,
                            *_line,
                            *_column,
                        );
                    }
                }

                let deferred_effects = self.deferred_list_effect_stack.pop().unwrap_or_default();
                let deferred_binding_effects =
                    self.deferred_binding_effect_stack.pop().unwrap_or_default();
                let popped_summary_key = self.deferred_action_key_stack.pop();
                debug_assert_eq!(popped_summary_key.as_ref(), Some(&summary_key));
                let returned_list_sources = recorded_returns
                    .iter()
                    .flat_map(|record| record.list_sources.iter().cloned())
                    .chain(implicit_list_sources);
                self.analyzer.restore_symbol_types(outer_type_snapshot);
                self.list_alias_groups = outer_alias_snapshot;
                self.optional_refinement_origins = outer_refinement_snapshot;
                self.definitely_nonempty_lists = outer_nonempty_snapshot;
                self.analyzer.pop_scope();
                let mut shared_return_provenance = SharedListReturnProvenance::new();
                for (depth, sources) in returned_list_sources {
                    let live_sources = sources
                        .into_iter()
                        .filter(|source| self.analyzer.binding_key_is_live(&source.binding))
                        .collect::<HashSet<_>>();
                    if !live_sources.is_empty() {
                        shared_return_provenance
                            .entry(depth)
                            .or_default()
                            .extend(live_sources);
                    }
                }
                self.user_action_list_effects
                    .entry(summary_key.clone())
                    .or_default()
                    .extend(deferred_effects);
                let binding_effects = self
                    .user_action_binding_effects
                    .entry(summary_key.clone())
                    .or_default();
                for (binding, effect_type) in deferred_binding_effects {
                    Self::join_binding_effect(binding_effects, binding, effect_type);
                }
                if shared_return_provenance.is_empty() {
                    self.user_action_shared_list_returns.remove(&summary_key);
                } else {
                    self.user_action_shared_list_returns
                        .insert(summary_key.clone(), shared_return_provenance);
                }
                self.propagate_user_action_summaries();

                // Update the action's symbol so call sites see the real result
                // type instead of the provisional `Unknown` seed. Pure `Nothing`
                // results (void actions) are recorded as `Nothing` — the seed is
                // only `Unknown` so self-references resolve gracefully during the
                // body check (#590); external callers still see the same `Nothing`
                // return type void actions had before.
                if let Some(inferred) = inferred_return.clone()
                    && let Some(symbol) = self.analyzer.get_symbol_mut(name)
                {
                    symbol.symbol_type = Some(Type::Function {
                        parameters: param_types,
                        return_type: Box::new(inferred),
                    });
                }

                // Record this overload's return type under its signature index
                // so overloaded call sites can resolve per-overload results
                // (the symbol_type above can only remember one definition).
                let overload_return = return_type
                    .as_ref()
                    .cloned()
                    .or(inferred_return)
                    .unwrap_or(Type::Unknown);
                self.overload_returns.insert(summary_key, overload_return);
                self.current_statement_completion = self
                    .analyzer
                    .get_symbol(name)
                    .and_then(|symbol| symbol.symbol_type.clone())
                    .unwrap_or(Type::Unknown);
            }
            Statement::IfStatement {
                condition,
                then_block,
                else_block,
                line: _line,
                column: _column,
            } => {
                self.definitely_nonempty_lists.clear();
                let condition_type = self.infer_expression_type(condition);
                if condition_type != Type::Boolean && !self.is_gradual_type(&condition_type) {
                    self.type_error(
                        "Condition must be a boolean expression".to_string(),
                        Some(Type::Boolean),
                        Some(condition_type),
                        *_line,
                        *_column,
                    );
                }

                let refinement = self.optional_condition_refinement(condition);
                let literal_condition = match condition {
                    Expression::Literal(Literal::Boolean(value), ..) => Some(*value),
                    _ => None,
                };
                let summary_entry = self.snapshot_deferred_summary();
                let refinement_origins_entry = self.optional_refinement_origins.clone();
                let refinement_origin = refinement.as_ref().and_then(|(name, _, _)| {
                    let binding = self.analyzer.get_symbol_binding_key(name)?;
                    let origin = self
                        .analyzer
                        .get_symbol_by_binding_key(&binding)?
                        .symbol_type
                        .clone()?;
                    Some((binding, origin))
                });
                let entry_aliases = self.list_alias_groups.clone();
                let entry_types = self.analyzer.snapshot_symbol_types();
                if let Some((name, then_type, _)) = &refinement {
                    self.refine_symbol_type(name, then_type);
                }
                if literal_condition == Some(false) {
                    self.try_flow_capture_suspended += 1;
                }
                let (then_can_continue, then_completion) =
                    self.check_statement_block_with_completion(then_block);
                if literal_condition == Some(false) {
                    self.try_flow_capture_suspended -= 1;
                }
                let then_types = self.analyzer.snapshot_symbol_types();
                let then_aliases = self.list_alias_groups.clone();
                let then_summary = self.snapshot_deferred_summary();
                self.analyzer.restore_symbol_types(entry_types.clone());
                self.list_alias_groups = entry_aliases.clone();
                self.restore_deferred_summary(summary_entry.clone());
                self.optional_refinement_origins = refinement_origins_entry.clone();

                if let Some((name, _, else_type)) = &refinement {
                    self.refine_symbol_type(name, else_type);
                }
                let (else_types, else_aliases, else_can_continue, else_completion) =
                    if let Some(else_stmts) = else_block {
                        if literal_condition == Some(true) {
                            self.try_flow_capture_suspended += 1;
                        }
                        let (can_continue, completion) =
                            self.check_statement_block_with_completion(else_stmts);
                        if literal_condition == Some(true) {
                            self.try_flow_capture_suspended -= 1;
                        }
                        (
                            self.analyzer.snapshot_symbol_types(),
                            self.list_alias_groups.clone(),
                            can_continue,
                            completion,
                        )
                    } else {
                        (
                            self.analyzer.snapshot_symbol_types(),
                            self.list_alias_groups.clone(),
                            true,
                            Type::Nothing,
                        )
                    };
                let else_summary = self.snapshot_deferred_summary();
                let reachable_summaries = match literal_condition {
                    Some(true) => vec![then_summary],
                    Some(false) => vec![else_summary],
                    None => vec![then_summary, else_summary],
                };
                self.join_deferred_summaries(&summary_entry, &reachable_summaries);
                let mut continuation_types = Vec::with_capacity(2);
                let mut continuation_aliases = Vec::with_capacity(2);
                let mut continuation_completions = Vec::with_capacity(2);
                if literal_condition != Some(false) && then_can_continue {
                    continuation_types.push(then_types);
                    continuation_aliases.push(then_aliases);
                    continuation_completions.push(then_completion);
                }
                if literal_condition != Some(true) && else_can_continue {
                    continuation_types.push(else_types);
                    continuation_aliases.push(else_aliases);
                    continuation_completions.push(else_completion);
                }
                let joined = if continuation_types.is_empty() {
                    entry_types.clone()
                } else {
                    Self::join_type_snapshots(&continuation_types)
                };
                self.analyzer.restore_symbol_types(joined);
                self.list_alias_groups = if continuation_aliases.is_empty() {
                    entry_aliases
                } else {
                    Self::join_list_alias_snapshots(&continuation_aliases)
                };
                self.current_statement_completion = continuation_completions
                    .into_iter()
                    .reduce(Self::join_inferred_types)
                    .unwrap_or(Type::Nothing);
                self.optional_refinement_origins = refinement_origins_entry;
                if let Some((binding, origin @ Type::Optional(_))) = refinement_origin
                    && let Some(current) = self
                        .analyzer
                        .get_symbol_by_binding_key(&binding)
                        .and_then(|symbol| symbol.symbol_type.as_ref())
                    && refinement
                        .as_ref()
                        .is_some_and(|(_, then_type, else_type)| {
                            current == then_type || current == else_type
                        })
                {
                    self.optional_refinement_origins.insert(binding, origin);
                }
            }
            Statement::SingleLineIf {
                condition,
                then_stmt,
                else_stmt,
                line: _line,
                column: _column,
            } => {
                self.definitely_nonempty_lists.clear();
                let condition_type = self.infer_expression_type(condition);
                if condition_type != Type::Boolean && !self.is_gradual_type(&condition_type) {
                    self.type_error(
                        "Condition must be a boolean expression".to_string(),
                        Some(Type::Boolean),
                        Some(condition_type),
                        *_line,
                        *_column,
                    );
                }

                let refinement = self.optional_condition_refinement(condition);
                let literal_condition = match condition {
                    Expression::Literal(Literal::Boolean(value), ..) => Some(*value),
                    _ => None,
                };
                let summary_entry = self.snapshot_deferred_summary();
                let refinement_origins_entry = self.optional_refinement_origins.clone();
                let refinement_origin = refinement.as_ref().and_then(|(name, _, _)| {
                    let binding = self.analyzer.get_symbol_binding_key(name)?;
                    let origin = self
                        .analyzer
                        .get_symbol_by_binding_key(&binding)?
                        .symbol_type
                        .clone()?;
                    Some((binding, origin))
                });
                let entry_aliases = self.list_alias_groups.clone();
                let entry_types = self.analyzer.snapshot_symbol_types();
                if let Some((name, then_type, _)) = &refinement {
                    self.refine_symbol_type(name, then_type);
                }
                if literal_condition == Some(false) {
                    self.try_flow_capture_suspended += 1;
                }
                let then_completion = self.check_statement_types(then_stmt);
                if literal_condition == Some(false) {
                    self.try_flow_capture_suspended -= 1;
                }
                let then_types = self.analyzer.snapshot_symbol_types();
                let then_can_continue = !Self::statement_definitely_stops_current_block(then_stmt);
                let then_aliases = self.list_alias_groups.clone();
                let then_summary = self.snapshot_deferred_summary();
                self.analyzer.restore_symbol_types(entry_types.clone());
                self.list_alias_groups = entry_aliases.clone();
                self.restore_deferred_summary(summary_entry.clone());
                self.optional_refinement_origins = refinement_origins_entry.clone();

                if let Some((name, _, else_type)) = &refinement {
                    self.refine_symbol_type(name, else_type);
                }
                let (else_types, else_aliases, else_can_continue, else_completion) =
                    if let Some(else_stmt) = else_stmt {
                        if literal_condition == Some(true) {
                            self.try_flow_capture_suspended += 1;
                        }
                        let completion = self.check_statement_types(else_stmt);
                        if literal_condition == Some(true) {
                            self.try_flow_capture_suspended -= 1;
                        }
                        (
                            self.analyzer.snapshot_symbol_types(),
                            self.list_alias_groups.clone(),
                            !Self::statement_definitely_stops_current_block(else_stmt),
                            completion,
                        )
                    } else {
                        (
                            self.analyzer.snapshot_symbol_types(),
                            self.list_alias_groups.clone(),
                            true,
                            Type::Nothing,
                        )
                    };
                let else_summary = self.snapshot_deferred_summary();
                let reachable_summaries = match literal_condition {
                    Some(true) => vec![then_summary],
                    Some(false) => vec![else_summary],
                    None => vec![then_summary, else_summary],
                };
                self.join_deferred_summaries(&summary_entry, &reachable_summaries);
                let mut continuation_types = Vec::with_capacity(2);
                let mut continuation_aliases = Vec::with_capacity(2);
                let mut continuation_completions = Vec::with_capacity(2);
                if literal_condition != Some(false) && then_can_continue {
                    continuation_types.push(then_types);
                    continuation_aliases.push(then_aliases);
                    continuation_completions.push(then_completion);
                }
                if literal_condition != Some(true) && else_can_continue {
                    continuation_types.push(else_types);
                    continuation_aliases.push(else_aliases);
                    continuation_completions.push(else_completion);
                }
                let joined = if continuation_types.is_empty() {
                    entry_types.clone()
                } else {
                    Self::join_type_snapshots(&continuation_types)
                };
                self.analyzer.restore_symbol_types(joined);
                self.list_alias_groups = if continuation_aliases.is_empty() {
                    entry_aliases
                } else {
                    Self::join_list_alias_snapshots(&continuation_aliases)
                };
                self.current_statement_completion = continuation_completions
                    .into_iter()
                    .reduce(Self::join_inferred_types)
                    .unwrap_or(Type::Nothing);
                self.optional_refinement_origins = refinement_origins_entry;
                if let Some((binding, origin @ Type::Optional(_))) = refinement_origin
                    && let Some(current) = self
                        .analyzer
                        .get_symbol_by_binding_key(&binding)
                        .and_then(|symbol| symbol.symbol_type.as_ref())
                    && refinement
                        .as_ref()
                        .is_some_and(|(_, then_type, else_type)| {
                            current == then_type || current == else_type
                        })
                {
                    self.optional_refinement_origins.insert(binding, origin);
                }
            }
            Statement::ForEachLoop {
                item_name,
                collection,
                body,
                line: _line,
                column: _column,
                ..
            } => {
                let guaranteed_iteration = self.expression_is_definitely_nonempty_list(collection);
                let collection_type = self.infer_expression_type(collection);
                let mut item_type_inferred = Type::Unknown;

                match collection_type {
                    Type::List(item_type) => {
                        item_type_inferred = *item_type;
                    }
                    Type::Map(_, value_type) => {
                        item_type_inferred = *value_type;
                    }
                    Type::Unknown | Type::Any | Type::Error => {}
                    _ => {
                        self.type_error(
                            "Collection in for-each loop must be a list or map".to_string(),
                            Some(Type::List(Box::new(Type::Unknown))),
                            Some(collection_type),
                            *_line,
                            *_column,
                        );
                    }
                }

                // Push a new scope for the loop body
                self.analyzer.push_scope();

                // Define the loop variable in the new scope
                let item_may_be_list = Self::type_may_be_list(&item_type_inferred);
                let symbol = Symbol {
                    name: item_name.clone(),
                    kind: SymbolKind::Variable { mutable: false },
                    symbol_type: Some(item_type_inferred),
                    line: *_line,
                    column: *_column,
                };

                // Ignore errors (e.g., if already defined, though in a new scope it shouldn't be)
                let _ = self.analyzer.define_symbol(symbol);
                if item_may_be_list
                    && let Some(mut source_path) = self.list_target_binding_path(collection)
                    && let Some(item_binding) = self.analyzer.get_symbol_binding_key(item_name)
                {
                    source_path.index_depth += 1;
                    self.add_structural_list_alias(
                        source_path,
                        ListAliasPath {
                            binding: item_binding,
                            index_depth: 0,
                        },
                    );
                }

                self.check_fresh_iteration_loop_body(body, guaranteed_iteration);

                // Pop the scope
                self.analyzer.pop_scope();
                self.prune_dead_list_alias_paths();
                // Loop-body mutations can invalidate cardinality facts. The
                // guaranteed-iteration flag above is intentionally consumed
                // only for this loop's control-flow join.
                self.definitely_nonempty_lists.clear();
            }
            Statement::CountLoop {
                start,
                end,
                step,
                body,
                line: _line,
                column: _column,
                variable_name,
                ..
            } => {
                let start_type = self.infer_expression_type(start);
                if start_type != Type::Number && !self.is_gradual_type(&start_type) {
                    self.type_error(
                        "Start value in count loop must be a number".to_string(),
                        Some(Type::Number),
                        Some(start_type),
                        *_line,
                        *_column,
                    );
                }

                let end_type = self.infer_expression_type(end);
                if end_type != Type::Number && !self.is_gradual_type(&end_type) {
                    self.type_error(
                        "End value in count loop must be a number".to_string(),
                        Some(Type::Number),
                        Some(end_type),
                        *_line,
                        *_column,
                    );
                }

                if let Some(step_expr) = step {
                    let step_type = self.infer_expression_type(step_expr);
                    if step_type != Type::Number && !self.is_gradual_type(&step_type) {
                        self.type_error(
                            "Step value in count loop must be a number".to_string(),
                            Some(Type::Number),
                            Some(step_type),
                            *_line,
                            *_column,
                        );
                    }
                }

                // Runtime creates the loop variable in a child environment,
                // shadowing rather than retyping an outer `count` or custom
                // loop-variable binding.
                self.analyzer.push_scope();
                self.analyzer.define_or_replace_symbol(Symbol {
                    name: variable_name.as_deref().unwrap_or("count").to_string(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: Some(Type::Number),
                    line: *_line,
                    column: *_column,
                });

                self.check_fresh_iteration_loop_body(body, false);
                self.analyzer.pop_scope();
                self.definitely_nonempty_lists.clear();
            }
            Statement::WhileLoop {
                condition,
                body,
                line: _line,
                column: _column,
            } => {
                let condition_type = self.infer_expression_type(condition);
                if condition_type != Type::Boolean && !self.is_gradual_type(&condition_type) {
                    self.type_error(
                        "Condition in while loop must be a boolean expression".to_string(),
                        Some(Type::Boolean),
                        Some(condition_type),
                        *_line,
                        *_column,
                    );
                }
                let first_condition_error_end = self.errors.len();
                self.check_persistent_loop_body_fixed_point(
                    body,
                    !matches!(condition, Expression::Literal(Literal::Boolean(false), ..)),
                );
                if self.budget_error.is_some() {
                    return;
                }

                let condition_type = self.infer_expression_type(condition);
                if condition_type != Type::Boolean && !self.is_gradual_type(&condition_type) {
                    self.type_error(
                        "Condition in while loop must be a boolean expression".to_string(),
                        Some(Type::Boolean),
                        Some(condition_type),
                        *_line,
                        *_column,
                    );
                }
                self.deduplicate_errors_from(first_condition_error_end);
                self.current_statement_completion =
                    if matches!(condition, Expression::Literal(Literal::Boolean(false), ..)) {
                        Type::Nothing
                    } else {
                        Type::Any
                    };
            }
            Statement::RepeatUntilLoop {
                condition,
                body,
                line: _line,
                column: _column,
            } => {
                // Runtime order: the body ALWAYS runs before the condition is
                // evaluated, in the same scope. Both the #642 fixed point and
                // the gradual-contract fixed point check in that order so body
                // retypings are visible to the condition.
                if Self::body_may_exit_loop_early(body) {
                    // A body that can `break`/`exit`/`return` skips the
                    // condition on that path at runtime, so the strict
                    // post-body state would falsely reject retype-then-break
                    // bodies (fatal inside `load module`). Walk the body with
                    // the post-body fixed point, then soften to the join of
                    // header and post-body for the condition — and for code
                    // after the loop, which such a path also reaches with
                    // pre-break state (#642).
                    let header = self.check_loop_body_fixed_point_post_body(body);
                    if self.budget_error.is_some() {
                        return;
                    }
                    let post_body = self.analyzer.snapshot_symbol_types();
                    self.analyzer
                        .restore_symbol_types(Self::join_type_snapshots(&[header, post_body]));

                    let condition_type = self.infer_expression_type(condition);
                    if condition_type != Type::Boolean
                        && condition_type != Type::Unknown
                        && condition_type != Type::Error
                    {
                        self.type_error(
                            "Condition in repeat-until loop must be a boolean expression"
                                .to_string(),
                            Some(Type::Boolean),
                            Some(condition_type),
                            *_line,
                            *_column,
                        );
                    }
                } else {
                    // No early exit: the condition and all code after the loop
                    // are always reached from the body's final state, so use
                    // the gradual-contract fixed point directly (post-body
                    // condition check, alias/deferred-summary tracking).
                    self.check_repeat_until_fixed_point(condition, body, *_line, *_column);
                }
                self.current_statement_completion = Type::Any;
            }
            Statement::ForeverLoop { body, .. } => {
                // Push a scope so bindings introduced in the body (e.g.
                // `start streaming response ... as out`) remain visible to later
                // statements in the same body for type checking. Analyzer loop
                // scopes are discarded after analysis.
                self.analyzer.push_scope();
                self.check_fresh_iteration_loop_body(body, true);
                self.analyzer.pop_scope();
                self.definitely_nonempty_lists.clear();
                self.current_statement_completion = Type::Any;
            }
            Statement::MainLoop { body, .. } => {
                self.analyzer.push_scope();
                self.check_fresh_iteration_loop_body(body, true);
                self.analyzer.pop_scope();
                self.definitely_nonempty_lists.clear();
                self.current_statement_completion = Type::Any;
            }
            Statement::DisplayStatement { value, .. } => {
                self.infer_expression_type(value);
            }
            Statement::ReturnStatement {
                value,
                line,
                column,
            } => {
                let (value_type, list_sources) = if let Some(expr) = value {
                    let value_type = self.infer_expression_type(expr);
                    let mut captured = Vec::new();
                    if Self::type_may_contain_list(&value_type) {
                        self.capture_nested_list_alias_sources(expr, 0, &mut captured);
                    }
                    (value_type, captured)
                } else {
                    (Type::Nothing, Vec::new())
                };
                if let Some(active_returns) = self.deferred_return_type_stack.last_mut() {
                    active_returns.push(RecordedReturn {
                        return_type: value_type,
                        line: *line,
                        column: *column,
                        has_value: value.is_some(),
                        list_sources,
                    });
                }
            }
            Statement::ExpressionStatement {
                expression,
                line,
                column,
            } => {
                let completion = match expression {
                    Expression::Variable(name, ..) => self
                        .infer_bare_variable_statement(name, *line, *column)
                        .unwrap_or_else(|| self.infer_expression_type(expression)),
                    _ => self.infer_expression_type(expression),
                };
                self.current_statement_completion = completion;
            }
            Statement::BreakStatement { .. } | Statement::ContinueStatement { .. } => {}
            Statement::OpenFileStatement {
                path,
                variable_name,
                mode: _mode,
                line: _line,
                column: _column,
            } => {
                let path_type = self.infer_expression_type(path);
                if path_type != Type::Text && !self.is_gradual_type(&path_type) {
                    self.type_error(
                        "File path must be a text string".to_string(),
                        Some(Type::Text),
                        Some(path_type),
                        *_line,
                        *_column,
                    );
                }

                // Runtime binds the opened handle in the current environment.
                // Analyzer body scopes are discarded before this pass, so
                // recreate the local symbol here and shadow (rather than
                // parent-walk/retype) any outer binding with the same name.
                self.analyzer.define_or_replace_symbol(Symbol {
                    name: variable_name.clone(),
                    kind: SymbolKind::Variable { mutable: true },
                    symbol_type: Some(Type::Custom("File".to_string())),
                    line: *_line,
                    column: *_column,
                });
            }
            Statement::ReadFileStatement {
                path,
                variable_name,
                line: _line,
                column: _column,
            } => {
                let file_type = self.infer_expression_type(path);
                if file_type != Type::Custom("File".to_string())
                    && file_type != Type::Text
                    && !self.is_gradual_type(&file_type)
                {
                    self.type_error(
                        "Expected a file path or File handle".to_string(),
                        None,
                        Some(file_type),
                        *_line,
                        *_column,
                    );
                }

                self.bind_runtime_value(variable_name, Type::Text, true, *_line, *_column);
            }
            Statement::WriteFileStatement {
                file,
                content,
                mode: _,
                line: _line,
                column: _column,
            } => {
                let file_type = self.infer_expression_type(file);
                if file_type != Type::Custom("File".to_string())
                    && file_type != Type::Text
                    && !self.is_gradual_type(&file_type)
                {
                    self.type_error(
                        "Expected a file path or File handle".to_string(),
                        None,
                        Some(file_type),
                        *_line,
                        *_column,
                    );
                }

                let content_type = self.infer_expression_type(content);
                if content_type != Type::Text && !self.is_gradual_type(&content_type) {
                    self.type_error(
                        "File content must be a text string".to_string(),
                        Some(Type::Text),
                        Some(content_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::CloseFileStatement {
                file,
                line: _line,
                column: _column,
            } => {
                let file_type = self.infer_expression_type(file);
                if !self.is_closeable_type(&file_type) {
                    // No single `expected` type: `close` accepts a File *or* a
                    // (map-shaped) stream handle, so pinning the hint to `File`
                    // would mis-render the expected-vs-found diagnostic.
                    self.type_error(
                        "Expected a file or stream handle".to_string(),
                        None,
                        Some(file_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::OpenDatabaseStatement {
                url,
                variable_name,
                line: _line,
                column: _column,
            } => {
                let url_type = self.infer_expression_type(url);
                if url_type != Type::Text && !self.is_gradual_type(&url_type) {
                    self.type_error(
                        "Database URL must be a text string".to_string(),
                        Some(Type::Text),
                        Some(url_type),
                        *_line,
                        *_column,
                    );
                }

                self.bind_runtime_value(
                    variable_name,
                    Type::Custom("Database".to_string()),
                    true,
                    *_line,
                    *_column,
                );
            }
            Statement::DatabaseQueryStatement {
                db,
                sql,
                parameters,
                variable_name,
                kind,
                line,
                column,
            } => {
                self.check_database_query_operands(db, sql, parameters.as_ref(), *line, *column);

                self.bind_runtime_value(
                    variable_name,
                    Self::database_result_type(*kind),
                    true,
                    *line,
                    *column,
                );
            }
            Statement::CloseDatabaseStatement {
                db,
                line: _line,
                column: _column,
            } => {
                let db_type = self.infer_expression_type(db);
                if db_type != Type::Custom("Database".to_string())
                    && !self.is_gradual_type(&db_type)
                {
                    self.type_error(
                        "Expected a Database connection".to_string(),
                        Some(Type::Custom("Database".to_string())),
                        Some(db_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::TransactionStatement {
                db,
                body,
                line: _line,
                column: _column,
            } => {
                let db_type = self.infer_expression_type(db);
                if db_type != Type::Custom("Database".to_string())
                    && !self.is_gradual_type(&db_type)
                {
                    self.type_error(
                        "Expected a Database connection".to_string(),
                        Some(Type::Custom("Database".to_string())),
                        Some(db_type),
                        *_line,
                        *_column,
                    );
                }
                // Like `try:`, the block shares the enclosing scope, introduces
                // no bindings of its own, and passes its body's value through —
                // `execute_transaction_statement` returns the body's last value
                // directly. Recording the completion keeps an action whose body
                // ends in a transaction from being inferred as returning
                // nothing, which would make every later use of its result a
                // spurious type error.
                let (_, completion) = self.check_statement_block_with_completion(body);
                self.current_statement_completion = completion;
            }
            Statement::CreateDirectoryStatement {
                path,
                line: _line,
                column: _column,
            } => {
                let path_type = self.infer_expression_type(path);
                if path_type != Type::Text && !self.is_gradual_type(&path_type) {
                    self.type_error(
                        "Expected string for directory path".to_string(),
                        Some(Type::Text),
                        Some(path_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::CreateFileStatement {
                path,
                content,
                line: _line,
                column: _column,
            } => {
                let path_type = self.infer_expression_type(path);
                if path_type != Type::Text && !self.is_gradual_type(&path_type) {
                    self.type_error(
                        "Expected string for file path".to_string(),
                        Some(Type::Text),
                        Some(path_type),
                        *_line,
                        *_column,
                    );
                }
                let _content_type = self.infer_expression_type(content); // Content can be any type
            }
            Statement::DeleteFileStatement {
                path,
                line: _line,
                column: _column,
            } => {
                let path_type = self.infer_expression_type(path);
                if path_type != Type::Text && !self.is_gradual_type(&path_type) {
                    self.type_error(
                        "Expected string for file path".to_string(),
                        Some(Type::Text),
                        Some(path_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::DeleteDirectoryStatement {
                path,
                line: _line,
                column: _column,
            } => {
                let path_type = self.infer_expression_type(path);
                if path_type != Type::Text && !self.is_gradual_type(&path_type) {
                    self.type_error(
                        "Expected string for directory path".to_string(),
                        Some(Type::Text),
                        Some(path_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::LoadModuleStatement {
                path,
                line: _line,
                column: _column,
                ..
            } => {
                let path_type = self.infer_expression_type(path);
                if path_type != Type::Text && !self.is_gradual_type(&path_type) {
                    self.type_error(
                        "Expected string for module path".to_string(),
                        Some(Type::Text),
                        Some(path_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::ExecuteCommandStatement {
                command,
                arguments,
                variable_name,
                use_shell: _,
                line: _line,
                column: _column,
            } => {
                let cmd_type = self.infer_expression_type(command);
                if cmd_type != Type::Text && !self.is_gradual_type(&cmd_type) {
                    self.type_error(
                        "Expected string for command".to_string(),
                        Some(Type::Text),
                        Some(cmd_type),
                        *_line,
                        *_column,
                    );
                }
                if let Some(args) = arguments {
                    let args_type = self.infer_expression_type(args);
                    if !self.is_process_arguments_type(&args_type) {
                        self.type_error(
                            "Command arguments must be text or a list".to_string(),
                            None,
                            Some(args_type),
                            *_line,
                            *_column,
                        );
                    }
                }
                if let Some(var_name) = variable_name {
                    self.bind_runtime_value(
                        var_name,
                        Type::Map(Box::new(Type::Text), Box::new(Type::Any)),
                        true,
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::ExecuteFileStatement {
                path,
                request,
                variable_name,
                line: _line,
                column: _column,
            } => {
                let path_type = self.infer_expression_type(path);
                if path_type != Type::Text && !self.is_gradual_type(&path_type) {
                    self.type_error(
                        "Expected string for execute file path".to_string(),
                        Some(Type::Text),
                        Some(path_type),
                        *_line,
                        *_column,
                    );
                }
                if let Some(request_expr) = request {
                    let request_type = self.infer_expression_type(request_expr);
                    if !self.is_execute_file_request_type(&request_type) {
                        self.type_error(
                            "Execute-file request must be a request object".to_string(),
                            Some(Type::Custom("Request".to_string())),
                            Some(request_type),
                            *_line,
                            *_column,
                        );
                    }
                }
                // Captured display output of the executed file is text
                if let Some(var_name) = variable_name {
                    self.bind_runtime_value(var_name, Type::Text, true, *_line, *_column);
                }
            }
            Statement::SpawnProcessStatement {
                command,
                arguments,
                variable_name,
                use_shell: _,
                line: _line,
                column: _column,
            } => {
                let cmd_type = self.infer_expression_type(command);
                if cmd_type != Type::Text && !self.is_gradual_type(&cmd_type) {
                    self.type_error(
                        "Expected string for command".to_string(),
                        Some(Type::Text),
                        Some(cmd_type),
                        *_line,
                        *_column,
                    );
                }
                if let Some(args) = arguments {
                    let args_type = self.infer_expression_type(args);
                    if !self.is_process_arguments_type(&args_type) {
                        self.type_error(
                            "Process arguments must be text or a list".to_string(),
                            None,
                            Some(args_type),
                            *_line,
                            *_column,
                        );
                    }
                }
                self.bind_runtime_value(variable_name, Type::Text, true, *_line, *_column);
            }
            Statement::ReadProcessOutputStatement {
                process_id,
                variable_name,
                line: _line,
                column: _column,
            } => {
                let proc_type = self.infer_expression_type(process_id);
                if proc_type != Type::Text && !self.is_gradual_type(&proc_type) {
                    self.type_error(
                        "Expected string for process ID".to_string(),
                        Some(Type::Text),
                        Some(proc_type),
                        *_line,
                        *_column,
                    );
                }
                self.bind_runtime_value(variable_name, Type::Text, true, *_line, *_column);
            }
            Statement::KillProcessStatement {
                process_id,
                line: _line,
                column: _column,
            } => {
                let proc_type = self.infer_expression_type(process_id);
                if proc_type != Type::Text && !self.is_gradual_type(&proc_type) {
                    self.type_error(
                        "Expected string for process ID".to_string(),
                        Some(Type::Text),
                        Some(proc_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::WaitForProcessStatement {
                process_id,
                variable_name,
                line: _line,
                column: _column,
            } => {
                let proc_type = self.infer_expression_type(process_id);
                if proc_type != Type::Text && !self.is_gradual_type(&proc_type) {
                    self.type_error(
                        "Expected string for process ID".to_string(),
                        Some(Type::Text),
                        Some(proc_type),
                        *_line,
                        *_column,
                    );
                }
                if let Some(var_name) = variable_name {
                    self.bind_runtime_value(var_name, Type::Number, true, *_line, *_column);
                }
            }
            Statement::WriteToStatement {
                content,
                file,
                line: _line,
                column: _column,
            } => {
                let _content_type = self.infer_expression_type(content); // Content can be any type
                let file_type = self.infer_expression_type(file);
                if file_type != Type::Custom("File".to_string())
                    && file_type != Type::Text  // Allow string file handles
                    && !self.is_gradual_type(&file_type)
                {
                    self.type_error(
                        "Expected a file handle or string".to_string(),
                        Some(Type::Custom("File".to_string())),
                        Some(file_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::WriteContentStatement {
                content,
                target,
                line: _line,
                column: _column,
            } => {
                let _content_type = self.infer_expression_type(content); // Content can be any type
                let target_type = self.infer_expression_type(target);
                if target_type != Type::Custom("File".to_string())
                    && target_type != Type::Text  // Allow string file handles
                    && !self.is_gradual_type(&target_type)
                {
                    self.type_error(
                        "Expected a file handle or string".to_string(),
                        Some(Type::Custom("File".to_string())),
                        Some(target_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::WriteBinaryStatement {
                content,
                target,
                line: _line,
                column: _column,
            } => {
                let content_type = self.infer_expression_type(content);
                if content_type != Type::Binary
                    && content_type != Type::List(Box::new(Type::Number))
                    && content_type != Type::List(Box::new(Type::Any))
                    && content_type != Type::List(Box::new(Type::Unknown))
                    && !self.is_gradual_type(&content_type)
                {
                    self.type_error(
                        "Expected Binary or List of Number for write binary content".to_string(),
                        Some(Type::Binary),
                        Some(content_type),
                        *_line,
                        *_column,
                    );
                }
                let target_type = self.infer_expression_type(target);
                if target_type != Type::Custom("File".to_string())
                    && !self.is_gradual_type(&target_type)
                {
                    self.type_error(
                        "Expected an open File handle for binary output".to_string(),
                        Some(Type::Custom("File".to_string())),
                        Some(target_type),
                        *_line,
                        *_column,
                    );
                }
            }
            Statement::CreateListStatement {
                name,
                initial_values,
                line,
                column,
            } => {
                // Infer the element type from initial values
                let mut element_type = None;
                for value in initial_values {
                    let value_type = self.infer_expression_type(value);
                    element_type = Some(Self::join_collection_value_type(element_type, value_type));
                }

                // If empty list, element type remains Unknown
                let list_type = Type::List(Box::new(element_type.unwrap_or(Type::Unknown)));
                self.bind_runtime_value(name, list_type, true, *line, *column);
                if let Some(target_binding) = self.analyzer.get_symbol_binding_key(name) {
                    for value in initial_values {
                        self.record_nested_list_alias_expression(&target_binding, 1, value);
                    }
                }
                self.update_binding_nonempty_fact(name, !initial_values.is_empty());
            }
            Statement::AddToListStatement {
                value,
                list_name,
                line,
                column,
            } => {
                let value_type = self.infer_expression_type(value);

                let (target_type, is_container_property) =
                    self.resolve_bare_mutation_target_type(list_name);
                match &target_type {
                    Some(Type::List(element_type)) => {
                        // A `List(Any)`/`List(Unknown)` is a list of statically
                        // unknown element type (e.g. a `[1, 2]` literal or an
                        // untyped-parameter list), so adding any concrete value
                        // is valid — only flag a concrete element type that is
                        // provably incompatible (gradual typing, issue #567).
                        if is_container_property
                            && !self.are_declared_property_values_compatible(
                                element_type,
                                &value_type,
                                value,
                            )
                        {
                            self.type_error(
                                format!(
                                    "Cannot add {value_type} to property '{list_name}' because its \
                                     declared element type is {element_type}"
                                ),
                                Some((**element_type).clone()),
                                Some(value_type),
                                *line,
                                *column,
                            );
                        } else if !is_container_property {
                            // Bare properties live in the container environment,
                            // not in the lexical analyzer scope. Applying alias
                            // effects to their bare name would mutate a
                            // same-named outer binding.
                            self.apply_list_mutation_effect(
                                &Expression::Variable(list_name.clone(), *line, *column),
                                ListMutationEffect::Join(value_type.clone()),
                            );
                            self.record_list_insertion_aliases(
                                &Expression::Variable(list_name.clone(), *line, *column),
                                value,
                            );
                            self.mark_list_target_nonempty(&Expression::Variable(
                                list_name.clone(),
                                *line,
                                *column,
                            ));
                        }
                    }
                    Some(Type::Number) => {
                        // This is arithmetic add. Accept Unknown/Any operands
                        // (statically unknown, verified at runtime) rather than
                        // emitting a false ERROR — gradual typing, issue #567.
                        if value_type != Type::Number && !self.is_gradual_type(&value_type) {
                            self.type_error(
                                "Cannot add non-numeric value to number".to_string(),
                                Some(Type::Number),
                                Some(value_type),
                                *line,
                                *column,
                            );
                        }
                    }
                    Some(Type::Unknown | Type::Any | Type::Error) => {
                        self.apply_list_mutation_effect(
                            &Expression::Variable(list_name.clone(), *line, *column),
                            ListMutationEffect::Join(value_type.clone()),
                        );
                        self.record_list_insertion_aliases(
                            &Expression::Variable(list_name.clone(), *line, *column),
                            value,
                        );
                        self.mark_list_target_nonempty(&Expression::Variable(
                            list_name.clone(),
                            *line,
                            *column,
                        ));
                    }
                    _ => {
                        // Variable might not be a list
                        self.type_error(
                            format!("Cannot add to non-list variable '{list_name}'"),
                            Some(Type::List(Box::new(Type::Any))),
                            target_type.clone(),
                            *line,
                            *column,
                        );
                    }
                }
            }
            Statement::RemoveFromListStatement {
                value,
                list_name,
                line,
                column,
            } => {
                let _value_type = self.infer_expression_type(value);
                self.definitely_nonempty_lists.clear();

                let (target_type, _is_container_property) =
                    self.resolve_bare_mutation_target_type(list_name);
                if let Some(target_type) = target_type
                    && !matches!(
                        target_type,
                        Type::List(_) | Type::Unknown | Type::Any | Type::Error
                    )
                {
                    self.type_error(
                        format!("Cannot remove from non-list variable '{list_name}'"),
                        Some(Type::List(Box::new(Type::Any))),
                        Some(target_type),
                        *line,
                        *column,
                    );
                }
            }
            Statement::ClearListStatement {
                list_name,
                line,
                column,
            } => {
                self.definitely_nonempty_lists.clear();
                let (target_type, is_container_property) =
                    self.resolve_bare_mutation_target_type(list_name);
                if !is_container_property {
                    self.detach_list_alias_descendants(&Expression::Variable(
                        list_name.clone(),
                        *line,
                        *column,
                    ));
                }
                if let Some(target_type) = target_type
                    && !matches!(
                        target_type,
                        Type::List(_) | Type::Unknown | Type::Any | Type::Error
                    )
                {
                    self.type_error(
                        format!("Cannot clear non-list variable '{list_name}'"),
                        Some(Type::List(Box::new(Type::Any))),
                        Some(target_type),
                        *line,
                        *column,
                    );
                }
            }
            // Container-related statements
            Statement::ContainerDefinition {
                name: _name,
                extends,
                implements,
                properties,
                methods,
                events: _events,
                static_properties,
                static_methods,
                line,
                column,
            } => {
                if self.analyzer.get_local_symbol(_name).is_none() {
                    let _ = self.analyzer.define_symbol(Symbol {
                        name: _name.clone(),
                        kind: SymbolKind::Variable { mutable: false },
                        symbol_type: Some(Type::Container(_name.clone())),
                        line: *line,
                        column: *column,
                    });
                }

                if let Some(parent_name) = extends {
                    if let Some(parent_symbol) = self.analyzer.get_symbol(parent_name) {
                        if parent_symbol.symbol_type != Some(Type::Container(parent_name.clone())) {
                            self.type_error(
                                format!("'{parent_name}' is not a container type"),
                                Some(Type::Container(parent_name.clone())),
                                parent_symbol.symbol_type.clone(),
                                *line,
                                *column,
                            );
                        }
                    } else {
                        self.type_error(
                            format!("Parent container '{parent_name}' not found"),
                            Some(Type::Container(parent_name.clone())),
                            None,
                            *line,
                            *column,
                        );
                    }
                }

                for interface_name in implements {
                    if let Some(interface_symbol) = self.analyzer.get_symbol(interface_name) {
                        if interface_symbol.symbol_type
                            != Some(Type::Interface(interface_name.clone()))
                        {
                            self.type_error(
                                format!("'{interface_name}' is not an interface type"),
                                Some(Type::Interface(interface_name.clone())),
                                interface_symbol.symbol_type.clone(),
                                *line,
                                *column,
                            );
                        }
                    } else {
                        self.type_error(
                            format!("Interface '{interface_name}' not found"),
                            Some(Type::Interface(interface_name.clone())),
                            None,
                            *line,
                            *column,
                        );
                    }
                }

                for property in properties.iter().chain(static_properties.iter()) {
                    if let Some(default_expr) = &property.default_value {
                        let default_type = self.infer_expression_type(default_expr);
                        if let Some(declared_type) = &property.property_type
                            && !self.are_declared_property_values_compatible(
                                declared_type,
                                &default_type,
                                default_expr,
                            )
                        {
                            self.type_error(
                                    format!(
                                        "Default value type {default_type:?} incompatible with declared type {declared_type:?}"
                                    ),
                                    Some(declared_type.clone()),
                                    Some(default_type),
                                    property.line,
                                    property.column,
                                );
                        }
                    }
                }

                // Check method bodies (instance and static) and, for
                // unannotated methods, infer the real return type from the
                // body's `return` statements — the analyzer only registered a
                // provisional `Unknown` (issue #560 residual: leaving the
                // provisional type in place made `instance.method()` results
                // degrade to `Unknown`, and the old `Nothing` default produced
                // false "Cannot index into Nothing" errors). Static methods get
                // the same refinement so `Container.method` member access
                // reports an accurate function type and void statics resolve
                // back to `Nothing`. Inferred types are collected first and
                // written back after the loop so the registry borrow doesn't
                // overlap the body checks.
                let mut inferred_method_returns: Vec<(String, Type)> = Vec::new();
                let mut inferred_static_method_returns: Vec<(String, Type)> = Vec::new();
                for (method, is_static) in methods
                    .iter()
                    .map(|m| (m, false))
                    .chain(static_methods.iter().map(|m| (m, true)))
                {
                    if let Statement::ActionDefinition {
                        name: method_name,
                        parameters,
                        body,
                        return_type,
                        line: _method_line,
                        column: _method_column,
                    } = method
                    {
                        // Set container context for method body analysis
                        let previous_container = self.current_container.clone();
                        let previous_method_is_static = self.current_method_is_static;
                        let previous_outer_property_bindings =
                            self.current_method_outer_property_bindings.take();
                        self.current_container = Some(_name.clone());
                        self.current_method_is_static = Some(is_static);
                        self.current_method_outer_property_bindings =
                            Some(self.snapshot_current_method_outer_property_bindings());

                        // Parameters must be resolvable while checking the body
                        // and inferring return expressions, mirroring the
                        // top-level `ActionDefinition` arm (issue #553).
                        self.analyzer.push_scope();
                        for param in parameters {
                            let param_symbol = Symbol {
                                name: param.name.clone(),
                                kind: SymbolKind::Variable { mutable: false },
                                symbol_type: param.param_type.clone().or(Some(Type::Unknown)),
                                line: param.line,
                                column: param.column,
                            };
                            self.analyzer.define_or_replace_symbol(param_symbol);
                        }

                        // Same as top-level actions: do not permanently refine
                        // outer bindings while checking an uncalled method body
                        // (PR #606 review).
                        let outer_type_snapshot = self.analyzer.snapshot_symbol_types();
                        let outer_alias_snapshot = self.list_alias_groups.clone();
                        let outer_refinement_snapshot = self.optional_refinement_origins.clone();
                        let outer_nonempty_snapshot = self.definitely_nonempty_lists.clone();
                        self.deferred_return_type_stack.push(Vec::new());

                        self.try_flow_capture_suspended += 1;
                        let (body_can_continue, implicit_completion) =
                            self.check_statement_block_with_completion(body);
                        self.try_flow_capture_suspended -= 1;
                        let recorded_returns =
                            self.deferred_return_type_stack.pop().unwrap_or_default();

                        if let Some(ret_type) = return_type {
                            self.check_recorded_return_types(&recorded_returns, ret_type);
                            if body_can_continue {
                                self.check_implicit_action_result(
                                    &implicit_completion,
                                    ret_type,
                                    *_method_line,
                                    *_method_column,
                                );
                            }
                        } else {
                            let inferred = Self::infer_recorded_action_return_type(
                                &recorded_returns,
                                body_can_continue.then_some(&implicit_completion),
                            );
                            if is_static {
                                inferred_static_method_returns
                                    .push((method_name.clone(), inferred));
                            } else {
                                inferred_method_returns.push((method_name.clone(), inferred));
                            }
                        }

                        self.analyzer.restore_symbol_types(outer_type_snapshot);
                        self.list_alias_groups = outer_alias_snapshot;
                        self.optional_refinement_origins = outer_refinement_snapshot;
                        self.definitely_nonempty_lists = outer_nonempty_snapshot;
                        self.analyzer.pop_scope();

                        // Restore previous container context
                        self.current_container = previous_container;
                        self.current_method_is_static = previous_method_is_static;
                        self.current_method_outer_property_bindings =
                            previous_outer_property_bindings;
                    }
                }

                if let Some(container_info) = self.analyzer.get_container_mut(_name) {
                    for (method_name, inferred) in inferred_method_returns {
                        if let Some(method_info) = container_info.methods.get_mut(&method_name) {
                            method_info.return_type = inferred;
                        }
                    }
                    for (method_name, inferred) in inferred_static_method_returns {
                        if let Some(method_info) =
                            container_info.static_methods.get_mut(&method_name)
                        {
                            method_info.return_type = inferred;
                        }
                    }
                }

                // Runtime returns the newly registered container definition.
                self.current_statement_completion = Type::Container(_name.clone());
            }
            Statement::ContainerInstantiation {
                container_type,
                instance_name,
                arguments,
                property_initializers,
                line,
                column,
            } => {
                let mut valid_container = false;
                if let Some(container_symbol) = self.analyzer.get_symbol(container_type) {
                    if container_symbol.symbol_type != Some(Type::Container(container_type.clone()))
                    {
                        self.type_error(
                            format!("'{container_type}' is not a container type"),
                            Some(Type::Container(container_type.clone())),
                            container_symbol.symbol_type.clone(),
                            *line,
                            *column,
                        );
                    } else {
                        valid_container = true;
                    }
                } else {
                    self.type_error(
                        format!("Container type '{container_type}' not found"),
                        Some(Type::Container(container_type.clone())),
                        None,
                        *line,
                        *column,
                    );
                }

                let argument_types: Vec<Type> = arguments
                    .iter()
                    .map(|argument| self.infer_expression_type(&argument.value))
                    .collect();

                if valid_container && !arguments.is_empty() {
                    let initialize_parameters = self
                        .analyzer
                        .get_container(container_type)
                        .and_then(|container| container.methods.get("initialize"))
                        .map(|method| method.parameters.clone());

                    if let Some(parameters) = initialize_parameters {
                        if parameters.len() != argument_types.len() {
                            self.type_error(
                                format!(
                                    "Container '{}' initialize method expects {} arguments, but {} were provided",
                                    container_type,
                                    parameters.len(),
                                    argument_types.len()
                                ),
                                None,
                                None,
                                *line,
                                *column,
                            );
                        }

                        for (index, (parameter, argument_type)) in
                            parameters.iter().zip(&argument_types).enumerate()
                        {
                            let expected = parameter
                                .param_type
                                .as_ref()
                                .cloned()
                                .unwrap_or(Type::Unknown);
                            if !self.are_types_compatible(&expected, argument_type) {
                                self.type_error(
                                    format!(
                                        "Argument {} of container '{}' initialize method expects {}, but found {}",
                                        index + 1,
                                        container_type,
                                        expected,
                                        argument_type
                                    ),
                                    Some(expected),
                                    Some(argument_type.clone()),
                                    *line,
                                    *column,
                                );
                            }
                        }
                    } else {
                        self.type_error(
                            format!(
                                "Container '{container_type}' has no direct initialize method for constructor arguments"
                            ),
                            None,
                            None,
                            *line,
                            *column,
                        );
                    }
                }

                for initializer in property_initializers {
                    let initializer_type = self.infer_expression_type(&initializer.value);
                    if let Some(property_type) =
                        self.container_property_type(container_type, &initializer.name)
                    {
                        if !self.are_declared_property_values_compatible(
                            &property_type,
                            &initializer_type,
                            &initializer.value,
                        ) {
                            self.type_error(
                                format!(
                                    "Property '{}' of container '{}' expects {}, but found {}",
                                    initializer.name,
                                    container_type,
                                    property_type,
                                    initializer_type
                                ),
                                Some(property_type),
                                Some(initializer_type),
                                initializer.line,
                                initializer.column,
                            );
                        }
                    } else if valid_container {
                        self.type_error(
                            format!(
                                "Property '{}' not found in container '{}'",
                                initializer.name, container_type
                            ),
                            None,
                            Some(initializer_type),
                            initializer.line,
                            initializer.column,
                        );
                    }
                }

                if valid_container {
                    self.escape_user_action_list_arguments(arguments, &argument_types);
                    self.escape_all_visible_mutable_state();
                    self.bind_runtime_value(
                        instance_name,
                        Type::ContainerInstance(container_type.clone()),
                        true,
                        *line,
                        *column,
                    );
                    // Runtime returns the newly constructed instance as this
                    // statement's value.
                    self.current_statement_completion =
                        Type::ContainerInstance(container_type.clone());
                }
            }
            Statement::InterfaceDefinition {
                name: _name,
                extends: _extends,
                required_actions: _required_actions,
                line: _line,
                column: _column,
            } => {
                // Interface type registration would be handled by analyzer
                self.current_statement_completion = Type::Interface(_name.clone());
            }
            Statement::EventDefinition {
                parameters,
                line,
                column,
                ..
            } => {
                for parameter in parameters {
                    if let Some(default_value) = &parameter.default_value {
                        let actual = self.infer_expression_type(default_value);
                        if let Some(expected) = &parameter.param_type
                            && !self.are_types_compatible(expected, &actual)
                        {
                            self.type_error(
                                format!(
                                    "Default value for event parameter '{}' expects {}, but found {}",
                                    parameter.name, expected, actual
                                ),
                                Some(expected.clone()),
                                Some(actual),
                                *line,
                                *column,
                            );
                        }
                    }
                }
                // Events have runtime values, but the static model has no
                // dedicated event type.
                self.current_statement_completion = Type::Any;
            }
            Statement::EventTrigger {
                name,
                arguments,
                line,
                column,
            } => {
                let argument_types: Vec<Type> = arguments
                    .iter()
                    .map(|argument| self.infer_expression_type(&argument.value))
                    .collect();
                let event_parameters = self
                    .current_container
                    .as_deref()
                    .and_then(|container_name| self.analyzer.get_container(container_name))
                    .and_then(|container| container.events.get(name))
                    .map(|event| event.parameters.clone())
                    .or_else(|| {
                        self.analyzer
                            .get_event(name)
                            .map(|event| event.parameters.clone())
                    });

                if let Some(parameters) = event_parameters {
                    // Runtime fills missing parameters with Nothing and ignores
                    // extra values after evaluating them. Only overlapping
                    // positions therefore have a static parameter contract.
                    for (index, (parameter, argument_type)) in
                        parameters.iter().zip(&argument_types).enumerate()
                    {
                        let expected = parameter
                            .param_type
                            .as_ref()
                            .cloned()
                            .unwrap_or(Type::Unknown);
                        if !self.are_types_compatible(&expected, argument_type) {
                            self.type_error(
                                format!(
                                    "Argument {} of event '{}' expects {}, but found {}",
                                    index + 1,
                                    name,
                                    expected,
                                    argument_type
                                ),
                                Some(expected),
                                Some(argument_type.clone()),
                                *line,
                                *column,
                            );
                        }
                    }
                } else if !self.has_includes {
                    self.type_error(
                        format!("Event '{name}' not found"),
                        None,
                        None,
                        *line,
                        *column,
                    );
                }
                self.escape_user_action_list_arguments(arguments, &argument_types);
                self.escape_all_visible_mutable_state();
            }
            Statement::EventHandler {
                event_name,
                event_source,
                handler_body,
                line,
                column,
            } => {
                let source_type = self.infer_expression_type(event_source);
                let event_parameters = match &source_type {
                    Type::ContainerInstance(container_name) => {
                        let event = self
                            .analyzer
                            .get_container(container_name)
                            .and_then(|container| container.events.get(event_name))
                            .cloned();
                        if event.is_none() {
                            self.type_error(
                                format!(
                                    "Event '{event_name}' not found in container '{container_name}'"
                                ),
                                None,
                                None,
                                *line,
                                *column,
                            );
                        }
                        event.map(|event| event.parameters)
                    }
                    Type::Unknown | Type::Any => self
                        .analyzer
                        .get_event(event_name)
                        .map(|event| event.parameters.clone()),
                    Type::Error => None,
                    _ => {
                        self.type_error(
                            "Cannot add event handler to non-container value".to_string(),
                            Some(Type::ContainerInstance("Unknown".to_string())),
                            Some(source_type.clone()),
                            *line,
                            *column,
                        );
                        None
                    }
                };

                self.analyzer.push_scope();
                if let Some(parameters) = event_parameters {
                    for parameter in parameters {
                        self.bind_runtime_value(
                            &parameter.name,
                            parameter.param_type.unwrap_or(Type::Unknown),
                            false,
                            parameter.line,
                            parameter.column,
                        );
                    }
                }
                let outer_type_snapshot = self.analyzer.snapshot_symbol_types();
                let outer_alias_snapshot = self.list_alias_groups.clone();
                let outer_refinement_snapshot = self.optional_refinement_origins.clone();
                let outer_nonempty_snapshot = self.definitely_nonempty_lists.clone();
                self.try_flow_capture_suspended += 1;
                for stmt in handler_body {
                    self.check_statement_types(stmt);
                }
                self.try_flow_capture_suspended -= 1;
                self.analyzer.restore_symbol_types(outer_type_snapshot);
                self.list_alias_groups = outer_alias_snapshot;
                self.optional_refinement_origins = outer_refinement_snapshot;
                self.definitely_nonempty_lists = outer_nonempty_snapshot;
                self.analyzer.pop_scope();
            }
            Statement::ParentMethodCall {
                method_name,
                arguments,
                line,
                column,
            } => {
                let argument_types: Vec<Type> = arguments
                    .iter()
                    .map(|argument| self.infer_expression_type(&argument.value))
                    .collect();

                let Some(container_name) = self.current_container.clone() else {
                    self.type_error(
                        "A parent method call is only valid inside a container instance method"
                            .to_string(),
                        None,
                        None,
                        *line,
                        *column,
                    );
                    return;
                };

                if self.current_method_is_static == Some(true) {
                    self.type_error(
                        "A parent method call cannot be used inside a static method".to_string(),
                        None,
                        None,
                        *line,
                        *column,
                    );
                    return;
                }

                let Some(parent_name) = self
                    .analyzer
                    .get_container(&container_name)
                    .and_then(|container| container.extends.clone())
                else {
                    self.type_error(
                        format!("Container '{container_name}' has no parent container"),
                        None,
                        None,
                        *line,
                        *column,
                    );
                    return;
                };

                let method_contract = self
                    .analyzer
                    .get_container(&parent_name)
                    .and_then(|parent| parent.methods.get(method_name))
                    .map(|method| (method.parameters.clone(), method.return_type.clone()));
                let Some((parameters, return_type)) = method_contract else {
                    self.type_error(
                        format!(
                            "Method '{method_name}' not found in direct parent container '{parent_name}'"
                        ),
                        None,
                        None,
                        *line,
                        *column,
                    );
                    return;
                };

                if parameters.len() != argument_types.len() {
                    self.type_error(
                        format!(
                            "Parent method '{}' expects {} arguments, but {} were provided",
                            method_name,
                            parameters.len(),
                            argument_types.len()
                        ),
                        None,
                        None,
                        *line,
                        *column,
                    );
                }

                for (index, (parameter, argument_type)) in
                    parameters.iter().zip(&argument_types).enumerate()
                {
                    let expected = parameter
                        .param_type
                        .as_ref()
                        .cloned()
                        .unwrap_or(Type::Unknown);
                    if !self.are_types_compatible(&expected, argument_type) {
                        self.type_error(
                            format!(
                                "Argument {} of parent method '{}' expects {}, but found {}",
                                index + 1,
                                method_name,
                                expected,
                                argument_type
                            ),
                            Some(expected),
                            Some(argument_type.clone()),
                            *line,
                            *column,
                        );
                    }
                }
                self.escape_user_action_list_arguments(arguments, &argument_types);
                self.escape_all_visible_mutable_state();
                self.current_statement_completion = return_type;
            }
            Statement::PatternDefinition {
                name,
                pattern,
                line,
                column,
            } => {
                self.check_pattern_expression_types(pattern, *line, *column);
                self.analyzer.define_or_replace_symbol(Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Pattern,
                    symbol_type: Some(Type::Pattern),
                    line: *line,
                    column: *column,
                });
                self.current_statement_completion = Type::Pattern;
            }
            Statement::MapCreation {
                name,
                entries,
                line,
                column,
            } => {
                let mut value_type = None;
                for (_key, value) in entries {
                    let inferred = self.infer_expression_type(value);
                    value_type = Some(Self::join_collection_value_type(value_type, inferred));
                }
                self.bind_runtime_value(
                    name,
                    Type::Map(
                        Box::new(Type::Text),
                        Box::new(value_type.unwrap_or(Type::Unknown)),
                    ),
                    true,
                    *line,
                    *column,
                );
                if let Some(target_binding) = self.analyzer.get_symbol_binding_key(name) {
                    for (_, value) in entries {
                        self.record_nested_list_alias_expression(&target_binding, 1, value);
                    }
                }
            }
            Statement::CreateDateStatement {
                name,
                value,
                line,
                column,
            } => {
                let value_type = value
                    .as_ref()
                    .map(|expr| self.infer_expression_type(expr))
                    .unwrap_or(Type::Date);
                self.bind_runtime_value(name, value_type, true, *line, *column);
            }
            Statement::CreateTimeStatement {
                name,
                value,
                line,
                column,
            } => {
                let value_type = value
                    .as_ref()
                    .map(|expr| self.infer_expression_type(expr))
                    .unwrap_or(Type::Time);
                self.bind_runtime_value(name, value_type, true, *line, *column);
            }
            Statement::ListenStatement {
                port,
                server_name,
                tls,
                redirect_to_port,
                line: _line,
                column: _column,
            } => {
                let port_type = self.infer_expression_type(port);
                if port_type != Type::Number && !self.is_gradual_type(&port_type) {
                    self.type_error(
                        "Port must be a number".to_string(),
                        Some(Type::Number),
                        Some(port_type),
                        *_line,
                        *_column,
                    );
                }

                // Certificate and key paths must be text
                if let Some(tls_config) = tls {
                    for (path_expr, what) in [
                        (tls_config.cert_path.as_ref(), "Certificate path"),
                        (tls_config.key_path.as_ref(), "Key path"),
                    ] {
                        if let Some(expr) = path_expr {
                            let path_type = self.infer_expression_type(expr);
                            if path_type != Type::Text && !self.is_gradual_type(&path_type) {
                                self.type_error(
                                    format!("{what} must be text"),
                                    Some(Type::Text),
                                    Some(path_type),
                                    *_line,
                                    *_column,
                                );
                            }
                        }
                    }
                }

                // Redirect target must be a number
                if let Some(target_port) = redirect_to_port {
                    let target_type = self.infer_expression_type(target_port);
                    if target_type != Type::Number && !self.is_gradual_type(&target_type) {
                        self.type_error(
                            "Redirect target port must be a number".to_string(),
                            Some(Type::Number),
                            Some(target_type),
                            *_line,
                            *_column,
                        );
                    }
                }
                self.bind_runtime_value(server_name, Type::Text, false, *_line, *_column);
            }
            Statement::WaitForRequestStatement {
                server,
                request_name,
                timeout,
                line,
                column,
            } => {
                self.check_server_expression_type(server, *line, *column);

                if let Some(timeout_expr) = timeout {
                    let timeout_type = self.infer_expression_type(timeout_expr);
                    if timeout_type != Type::Number && !self.is_gradual_type(&timeout_type) {
                        self.type_error(
                            "Timeout must be a number".to_string(),
                            Some(Type::Number),
                            Some(timeout_type),
                            *line,
                            *column,
                        );
                    }
                }
                self.bind_runtime_value(
                    request_name,
                    Type::Custom("Request".to_string()),
                    false,
                    *line,
                    *column,
                );
                for (name, value_type) in [
                    ("method", Type::Text),
                    ("path", Type::Text),
                    ("query", Type::Text),
                    ("client_ip", Type::Text),
                    ("body", Type::Text),
                    ("body_bytes", Type::Binary),
                    (
                        "headers",
                        Type::Map(Box::new(Type::Text), Box::new(Type::Text)),
                    ),
                ] {
                    self.bind_runtime_value(name, value_type, false, *line, *column);
                }
            }
            Statement::RespondStatement {
                request,
                content,
                status,
                content_type,
                headers,
                line: _line,
                column: _column,
            } => {
                let request_type = self.infer_expression_type(request);
                if !self.is_pending_request_type(&request_type) {
                    self.type_error(
                        "Response target must be a request object".to_string(),
                        Some(Type::Custom("Request".to_string())),
                        Some(request_type.clone()),
                        *_line,
                        *_column,
                    );
                }

                // Runtime serves text/binary losslessly and stringifies scalar
                // number/boolean/nothing values. Composite values are rejected.
                let content_type_result = self.infer_expression_type(content);
                if !Self::is_response_content_type(&content_type_result) {
                    self.type_error(
                        "Response content must be text, binary, a number, a boolean, or nothing"
                            .to_string(),
                        None,
                        Some(content_type_result),
                        *_line,
                        *_column,
                    );
                }

                // Check status if provided (should be number)
                if let Some(status_expr) = status {
                    let status_type = self.infer_expression_type(status_expr);
                    if status_type != Type::Number && !self.is_gradual_type(&status_type) {
                        self.type_error(
                            "HTTP status must be a number".to_string(),
                            Some(Type::Number),
                            Some(status_type),
                            *_line,
                            *_column,
                        );
                    }
                }

                // Check content_type if provided (should be text)
                if let Some(ct_expr) = content_type {
                    let ct_type = self.infer_expression_type(ct_expr);
                    if ct_type != Type::Text && !self.is_gradual_type(&ct_type) {
                        self.type_error(
                            "Content type must be text".to_string(),
                            Some(Type::Text),
                            Some(ct_type),
                            *_line,
                            *_column,
                        );
                    }
                }

                // Check headers if provided (should be a map). Mirrors the
                // outbound HttpRequestStatement headers check.
                if let Some(headers_expr) = headers {
                    let headers_type = self.infer_expression_type(headers_expr);
                    if !self.is_valid_header_map_type(&headers_type) {
                        self.type_error(
                            "Response headers must be a map of header names to values".to_string(),
                            Some(Type::Map(Box::new(Type::Text), Box::new(Type::Any))),
                            Some(headers_type),
                            *_line,
                            *_column,
                        );
                    }
                }
            }
            // Graceful shutdown and signal handling statements
            Statement::RegisterSignalHandlerStatement {
                signal_type,
                handler_name,
                line,
                column,
            } => {
                self.validate_signal_handler_statement(signal_type, handler_name, *line, *column);
            }
            Statement::StopAcceptingConnectionsStatement {
                server,
                line,
                column,
            } => {
                self.check_server_expression_type(server, *line, *column);
            }
            Statement::CloseServerStatement {
                server,
                line,
                column,
            } => {
                self.check_server_expression_type(server, *line, *column);
            }
            // WebSocket statements
            Statement::ListenWebSocketStatement {
                port,
                server_name,
                line,
                column,
            } => {
                let port_type = self.infer_expression_type(port);
                if port_type != Type::Number && !self.is_gradual_type(&port_type) {
                    self.type_error(
                        "WebSocket port must be a number".to_string(),
                        Some(Type::Number),
                        Some(port_type),
                        *line,
                        *column,
                    );
                }
                self.bind_runtime_value(server_name, Type::Text, false, *line, *column);
            }
            Statement::WebSocketHandlerStatement {
                event,
                server,
                binding,
                body,
                line,
                column,
            } => {
                // The server operand and handler body are checked; the handler's
                // bound variable resolves as an object at runtime. Connection
                // lifecycle objects contain only text fields; message objects
                // additionally contain a nested sender object and therefore
                // retain a heterogeneous value type.
                self.check_server_expression_type(server, *line, *column);
                self.has_websocket_handlers = true;
                self.analyzer.push_scope();
                let event_value_type = match event {
                    WsHandlerEvent::Connect | WsHandlerEvent::Disconnect => Type::Text,
                    WsHandlerEvent::Message => Type::Any,
                };
                self.bind_runtime_value(
                    binding,
                    Type::Map(Box::new(Type::Text), Box::new(event_value_type)),
                    false,
                    *line,
                    *column,
                );
                // `bind_runtime_value` above already recreated the binding in
                // this handler scope with its concrete runtime map type,
                // deliberately shadowing any outer same-named variable so the
                // body is not checked against the outer symbol's type (#642).
                // Keeping that map type (rather than downgrading to Unknown)
                // leaves field/index access on the event object permissive while
                // still rejecting misuse such as arithmetic on it.
                let outer_type_snapshot = self.analyzer.snapshot_symbol_types();
                let outer_alias_snapshot = self.list_alias_groups.clone();
                let outer_refinement_snapshot = self.optional_refinement_origins.clone();
                let outer_nonempty_snapshot = self.definitely_nonempty_lists.clone();
                self.try_flow_capture_suspended += 1;
                for stmt in body {
                    self.check_statement_types(stmt);
                }
                self.try_flow_capture_suspended -= 1;
                self.analyzer.restore_symbol_types(outer_type_snapshot);
                self.list_alias_groups = outer_alias_snapshot;
                self.optional_refinement_origins = outer_refinement_snapshot;
                self.definitely_nonempty_lists = outer_nonempty_snapshot;
                self.analyzer.pop_scope();
            }
            Statement::SendWebSocketMessageStatement {
                message,
                target,
                line,
                column,
            } => {
                let message_type = self.infer_expression_type(message);
                self.check_websocket_message_type(message_type, *line, *column);
                let target_type = self.infer_expression_type(target);
                if !self.is_websocket_connection_target_type(&target_type) {
                    self.type_error(
                        "WebSocket connection target must be an object".to_string(),
                        Some(Type::Map(Box::new(Type::Text), Box::new(Type::Any))),
                        Some(target_type),
                        *line,
                        *column,
                    );
                }
            }
            Statement::BroadcastWebSocketMessageStatement {
                message,
                server,
                line,
                column,
            } => {
                let message_type = self.infer_expression_type(message);
                self.check_websocket_message_type(message_type, *line, *column);
                self.check_server_expression_type(server, *line, *column);
            }
            // Test framework statements
            Statement::DescribeBlock {
                description: _description,
                setup,
                teardown,
                tests,
                line: _line,
                column: _column,
            } => {
                // Runtime creates one describe-level child environment shared
                // by setup, every isolated test child, and teardown.
                self.analyzer.push_scope();

                // Type check setup block if present
                if let Some(setup_stmts) = setup {
                    for stmt in setup_stmts {
                        self.check_statement_types(stmt);
                    }
                }

                // Type check all test blocks
                for test in tests {
                    self.check_statement_types(test);
                }

                // Type check teardown block if present
                if let Some(teardown_stmts) = teardown {
                    for stmt in teardown_stmts {
                        self.check_statement_types(stmt);
                    }
                }

                self.analyzer.pop_scope();
            }
            Statement::TestBlock {
                description: _description,
                body,
                line: _line,
                column: _column,
            } => {
                // Each test runs in an isolated child of the describe
                // environment. Its declarations and type refinements cannot
                // leak to sibling tests or teardown.
                self.analyzer.push_scope();
                let describe_type_snapshot = self.analyzer.snapshot_symbol_types();
                let describe_alias_snapshot = self.list_alias_groups.clone();
                for stmt in body {
                    self.check_statement_types(stmt);
                }
                self.analyzer.restore_symbol_types(describe_type_snapshot);
                self.list_alias_groups = describe_alias_snapshot;
                self.analyzer.pop_scope();
            }
            Statement::ExpectStatement {
                subject,
                assertion,
                line: _line,
                column: _column,
            } => {
                // Type check the subject expression
                let subject_type = self.infer_expression_type(subject);

                // Perform compile-time type checking for assertions where possible
                use crate::parser::ast::Assertion;
                match assertion {
                    Assertion::Equal(expr) | Assertion::Be(expr) => {
                        // Type check the expected value
                        self.infer_expression_type(expr);
                    }
                    Assertion::GreaterThan(expr) | Assertion::LessThan(expr) => {
                        // Check that subject is a number
                        if subject_type != Type::Number && !self.is_gradual_type(&subject_type) {
                            self.type_error(
                                "Comparison assertions require numeric types".to_string(),
                                Some(Type::Number),
                                Some(subject_type.clone()),
                                *_line,
                                *_column,
                            );
                        }
                        // Type check the comparison value
                        let expr_type = self.infer_expression_type(expr);
                        if expr_type != Type::Number && !self.is_gradual_type(&expr_type) {
                            self.type_error(
                                "Comparison value must be numeric".to_string(),
                                Some(Type::Number),
                                Some(expr_type),
                                *_line,
                                *_column,
                            );
                        }
                    }
                    Assertion::BeYes | Assertion::BeNo => {
                        // Truthiness checks work on any type, no validation needed
                    }
                    Assertion::Exist => {
                        // Existence checks work on any type
                    }
                    Assertion::Contain(expr) => {
                        // Check that subject is a list or text
                        if !matches!(
                            subject_type,
                            Type::List(_) | Type::Text | Type::Unknown | Type::Any | Type::Error
                        ) {
                            self.type_error(
                                "contain assertion requires List or Text type".to_string(),
                                None,
                                Some(subject_type.clone()),
                                *_line,
                                *_column,
                            );
                        }
                        // Type check the item expression
                        self.infer_expression_type(expr);
                    }
                    Assertion::BeEmpty => {
                        // Check that subject is a list or text
                        if !matches!(
                            subject_type,
                            Type::List(_) | Type::Text | Type::Unknown | Type::Any | Type::Error
                        ) {
                            self.type_error(
                                "be empty assertion requires List or Text type".to_string(),
                                None,
                                Some(subject_type.clone()),
                                *_line,
                                *_column,
                            );
                        }
                    }
                    Assertion::HaveLength(expr) => {
                        // Check that subject is a list or text
                        if !matches!(
                            subject_type,
                            Type::List(_) | Type::Text | Type::Unknown | Type::Any | Type::Error
                        ) {
                            self.type_error(
                                "have length assertion requires List or Text type".to_string(),
                                None,
                                Some(subject_type.clone()),
                                *_line,
                                *_column,
                            );
                        }
                        // Type check the length value (should be number)
                        let length_type = self.infer_expression_type(expr);
                        if length_type != Type::Number && !self.is_gradual_type(&length_type) {
                            self.type_error(
                                "Length value must be numeric".to_string(),
                                Some(Type::Number),
                                Some(length_type),
                                *_line,
                                *_column,
                            );
                        }
                    }
                    Assertion::BeOfType(_type_name) => {
                        // Type name is validated at runtime
                    }
                }
            }

            Statement::IncludeStatement {
                path, line, column, ..
            } => {
                // Type check the path expression - must be a string
                let path_type = self.infer_expression_type(path);
                if path_type != Type::Text && !self.is_gradual_type(&path_type) {
                    self.type_error(
                        "Expected string for include path".to_string(),
                        Some(Type::Text),
                        Some(path_type),
                        *line,
                        *column,
                    );
                }
                // Note: Include statements execute in parent scope, making their symbols available
                // at runtime. However, the type checker doesn't currently parse included files,
                // which can result in false "not found" errors for symbols defined in included files.
                // Future improvement: Parse and analyze included files during type checking
                // to register their symbols in the current scope for more accurate diagnostics.
                // An included file may propagate an arbitrary `return` value.
                self.current_statement_completion = Type::Any;
            }

            Statement::ExportStatement {
                export_type,
                name,
                line,
                column,
                ..
            } => {
                // Runtime exports are explicitly local-only: a definition
                // inherited from a parent environment cannot be re-exported.
                let local_symbol = self.analyzer.get_local_symbol(name);
                match export_type {
                    crate::parser::ast::ExportType::Container => {
                        if !matches!(
                            local_symbol.and_then(|symbol| symbol.symbol_type.as_ref()),
                            Some(Type::Container(container_name)) if container_name == name
                        ) {
                            self.type_error(
                                format!("Container '{}' not found for export", name),
                                None,
                                None,
                                *line,
                                *column,
                            );
                        }
                    }
                    crate::parser::ast::ExportType::Action => {
                        if let Some(symbol) = local_symbol {
                            match &symbol.kind {
                                crate::analyzer::SymbolKind::Function { .. } => {}
                                _ => {
                                    self.type_error(
                                        format!(
                                            "'{}' is not an action and cannot be exported as one",
                                            name
                                        ),
                                        None,
                                        None,
                                        *line,
                                        *column,
                                    );
                                }
                            }
                        } else {
                            self.type_error(
                                format!("Action '{}' not found for export", name),
                                None,
                                None,
                                *line,
                                *column,
                            );
                        }
                    }
                    crate::parser::ast::ExportType::Constant => {
                        if let Some(symbol) = local_symbol {
                            match &symbol.kind {
                                crate::analyzer::SymbolKind::Variable { mutable } => {
                                    if *mutable {
                                        self.type_error(
                                            format!(
                                                "'{}' is mutable and cannot be exported as constant",
                                                name
                                            ),
                                            None,
                                            None,
                                            *line,
                                            *column,
                                        );
                                    }
                                }
                                _ => {
                                    self.type_error(
                                        format!("'{}' is not a variable and cannot be exported as constant", name),
                                        None,
                                        None,
                                        *line,
                                        *column,
                                    );
                                }
                            }
                        } else {
                            self.type_error(
                                format!("Constant '{}' not found for export", name),
                                None,
                                None,
                                *line,
                                *column,
                            );
                        }
                    }
                }
            }
        }
    }

    /// Names that are callable even when they do not resolve to a scope symbol:
    /// builtin stdlib functions, action parameters, and the internal test stubs.
    /// Shared by the `FunctionCall` (`of` form) and `ActionCall` (`call ... with`)
    /// inference paths so the "is this callee statically known?" decision cannot
    /// drift apart between them again (the class of divergence behind issue #580).
    fn is_callable_without_symbol(&self, name: &str) -> bool {
        Analyzer::is_builtin_function(name)
            || self.analyzer.get_action_parameters().contains(name)
            || name == "helper_function"
            || name == "nested_function"
    }

    /// Whether this call site resolves to the standard-library native rather
    /// than a stored callable or a user action using a future-reserved name.
    fn should_use_builtin_contract(&self, name: &str, line: usize, column: usize) -> bool {
        if !Analyzer::is_builtin_function(name)
            || self
                .analyzer
                .alias_call_resolution(name, line, column)
                .is_some()
        {
            return false;
        }
        if builtins::is_implemented_builtin_function(name) {
            return true;
        }
        self.analyzer.get_symbol(name).is_none() && !self.has_includes
    }

    fn infer_expression_type(&mut self, expression: &Expression) -> Type {
        // Recursive front-end checkpoint for expressions (mirrors the analyzer's
        // `analyze_expression`): `check_statement_types` polls per statement, but
        // one statement can hold an arbitrarily large expression tree, so poll
        // here too. The `budget_error` latch records the breach once and
        // short-circuits; the returned `Any` is irrelevant because `check_types`
        // turns the latched breach into the fatal `TypeCheckError::Budget`.
        if self.budget_error.is_some() {
            return Type::Any;
        }
        if let Some(budget) = crate::exec::budget::ExecutionBudget::current()
            && let Err(exceeded) = budget.charge_operation(!budget.is_deadline_exempt())
        {
            self.errors
                .push(TypeError::new(exceeded.message(), None, None, 0, 0));
            self.budget_error = Some(exceeded);
            return Type::Any;
        }
        match expression {
            Expression::Literal(literal, _, _) => match literal {
                Literal::String(_) => Type::Text,
                Literal::Integer(_) => Type::Number,
                Literal::Float(_) => Type::Number,
                Literal::Boolean(_) => Type::Boolean,
                Literal::Nothing => Type::Nothing,
                Literal::Pattern(_) => Type::Pattern,
                Literal::List(elements) => {
                    let mut element_type = None;
                    for element in elements {
                        let inferred = self.infer_expression_type(element);
                        element_type =
                            Some(Self::join_collection_value_type(element_type, inferred));
                    }
                    // Preserve useful precision for homogeneous literals while
                    // widening genuinely heterogeneous values to Any.
                    Type::List(Box::new(element_type.unwrap_or(Type::Unknown)))
                }
            },
            Expression::Variable(name, line, column) => {
                let (resolved_type, is_property) = self.resolve_bare_mutation_target_type(name);
                if is_property && let Some(property_type) = resolved_type {
                    return property_type;
                }
                if let Some(result) = self.infer_zero_arg_variable_expression(name, *line, *column)
                {
                    return result;
                }
                if let Some(symbol) = self.analyzer.get_symbol(name) {
                    // Builtin stdlib functions get injected into an included
                    // file's scope as parent-scope variable bindings (defined
                    // at position 0:0) whose runtime value is a native
                    // function, so their recorded type is Unknown. Resolve
                    // them to their real builtin signature so variables bound
                    // to their results stay inferable (issue #551). A user who
                    // shadows a builtin with a concrete value keeps that
                    // value's type and the normal path below.
                    let is_injected_builtin = symbol.line == 0
                        && symbol.column == 0
                        && Analyzer::is_builtin_function(name);
                    if is_injected_builtin
                        && matches!(symbol.symbol_type, None | Some(Type::Unknown))
                    {
                        if builtins::is_implemented_builtin_function(name) {
                            return self.get_bare_builtin_type(name);
                        }
                        if self.has_includes {
                            return Type::Unknown;
                        }
                        self.type_error(
                            format!(
                                "Builtin '{name}' is recognized but not implemented by the runtime"
                            ),
                            None,
                            None,
                            *line,
                            *column,
                        );
                        return Type::Error;
                    }
                    if let Some(var_type) = &symbol.symbol_type {
                        var_type.clone()
                    } else {
                        // A binding with no recorded type — most commonly an
                        // untyped action parameter — is statically unknown, not
                        // provably wrong. Treat it as Unknown (gradual typing)
                        // instead of emitting a false ERROR at every reference
                        // (issue #567). Genuine type mismatches are still caught
                        // where a concrete type is required at runtime.
                        Type::Unknown
                    }
                } else {
                    // Check if this is an action parameter, builtin function, or special function name before reporting it as undefined
                    if self.analyzer.get_action_parameters().contains(name)
                        || Analyzer::is_builtin_function(name)
                        || name == "helper_function"
                        || name == "nested_function"
                    {
                        // It's an action parameter or a special function name, so don't report an error
                        if name == "loopcounter" || name == "count" {
                            // Special case for loopcounter and count - they're Numbers
                            return Type::Number;
                        }

                        // For builtin functions, return their proper type
                        if Analyzer::is_builtin_function(name) {
                            if builtins::is_implemented_builtin_function(name) {
                                return self.get_bare_builtin_type(name);
                            }
                            if self.has_includes {
                                return Type::Unknown;
                            }
                            self.type_error(
                                format!(
                                    "Builtin '{name}' is recognized but not implemented by the runtime"
                                ),
                                None,
                                None,
                                *line,
                                *column,
                            );
                            return Type::Error;
                        }

                        Type::Unknown
                    } else {
                        // The analyzer already reports undefined variables, so we don't need to duplicate the error
                        // Return Unknown type to continue type checking without cascading errors
                        Type::Unknown
                    }
                }
            }
            Expression::BinaryOperation {
                left,
                operator,
                right,
                line,
                column,
            } => {
                let left_type = self.infer_expression_type(left);
                let right_type = self.infer_expression_type(right);

                if left_type == Type::Error || right_type == Type::Error {
                    return Type::Error;
                }

                if left_type == Type::Unknown || right_type == Type::Unknown {
                    // Comparisons and logical operations always produce a
                    // Boolean, no matter what the operand types turn out to
                    // be at runtime — only arithmetic results depend on the
                    // operand types (issue #553).
                    return match operator {
                        Operator::Equals
                        | Operator::NotEquals
                        | Operator::GreaterThan
                        | Operator::LessThan
                        | Operator::GreaterThanOrEqual
                        | Operator::LessThanOrEqual
                        | Operator::And
                        | Operator::Or
                        | Operator::Contains => Type::Boolean,
                        _ => Type::Unknown,
                    };
                }

                // Dynamically-typed (Any) operands are checked at runtime, so the
                // static result type can't be known. Degrade gracefully instead
                // of raising a false ERROR — an `Any` (e.g. a list-index result)
                // represents "statically unknown", not "known incompatible"
                // (gradual typing — issue #567). This mirrors the `Unknown`
                // handling above: comparisons still yield Boolean, arithmetic
                // yields Any (or Text when the other Plus operand is Text).
                if left_type == Type::Any || right_type == Type::Any {
                    return match operator {
                        Operator::Equals
                        | Operator::NotEquals
                        | Operator::GreaterThan
                        | Operator::LessThan
                        | Operator::GreaterThanOrEqual
                        | Operator::LessThanOrEqual
                        | Operator::And
                        | Operator::Or
                        | Operator::Contains => Type::Boolean,
                        Operator::Plus if left_type == Type::Text || right_type == Type::Text => {
                            Type::Text
                        }
                        _ => Type::Any,
                    };
                }

                match operator {
                    Operator::Plus => {
                        // Plus operation allows:
                        // - Number + Number = Number
                        // - Text + Text = Text
                        // - Text + Number = Text (automatic conversion)
                        // - Number + Text = Text (automatic conversion)
                        if left_type == Type::Number && right_type == Type::Number {
                            Type::Number
                        } else if left_type == Type::Text || right_type == Type::Text {
                            // If either operand is Text, the result is Text (automatic conversion)
                            Type::Text
                        } else {
                            self.type_error(
                                format!(
                                    "Cannot perform {operator:?} operation on {left_type} and {right_type}"
                                ),
                                Some(Type::Text),
                                Some(if left_type != Type::Text {
                                    left_type
                                } else {
                                    right_type
                                }),
                                *line,
                                *column,
                            );
                            Type::Error
                        }
                    }
                    Operator::Minus | Operator::Multiply | Operator::Divide | Operator::Modulo => {
                        // These operations require both operands to be numbers
                        if left_type == Type::Number && right_type == Type::Number {
                            Type::Number
                        } else {
                            self.type_error(
                                format!(
                                    "Cannot perform {operator:?} operation on {left_type} and {right_type}"
                                ),
                                Some(Type::Number),
                                Some(if left_type != Type::Number {
                                    left_type
                                } else {
                                    right_type
                                }),
                                *line,
                                *column,
                            );
                            Type::Error
                        }
                    }
                    Operator::Equals | Operator::NotEquals => {
                        // Runtime equality is total: unlike types simply compare
                        // unequal. Rejecting unlike concrete types here would be
                        // stricter than the language's actual semantics.
                        Type::Boolean
                    }
                    Operator::GreaterThan
                    | Operator::LessThan
                    | Operator::GreaterThanOrEqual
                    | Operator::LessThanOrEqual => {
                        if (left_type == Type::Number && right_type == Type::Number)
                            || (left_type == Type::Text && right_type == Type::Text)
                            || self.are_same_temporal_type(&left_type, &right_type)
                        {
                            Type::Boolean
                        } else {
                            self.type_error(
                                format!(
                                    "Cannot compare {left_type} and {right_type} with {operator:?}"
                                ),
                                Some(
                                    if left_type == Type::Number
                                        || left_type == Type::Text
                                        || Self::temporal_kind(&left_type).is_some()
                                    {
                                        left_type.clone()
                                    } else {
                                        Type::Number
                                    },
                                ),
                                Some(right_type),
                                *line,
                                *column,
                            );
                            Type::Error
                        }
                    }
                    Operator::And | Operator::Or => {
                        if left_type == Type::Boolean && right_type == Type::Boolean {
                            Type::Boolean
                        } else {
                            self.type_error(
                                format!(
                                    "Cannot perform logical {operator:?} on {left_type} and {right_type}"
                                ),
                                Some(Type::Boolean),
                                Some(if left_type != Type::Boolean {
                                    left_type
                                } else {
                                    right_type
                                }),
                                *line,
                                *column,
                            );
                            Type::Error
                        }
                    }
                    Operator::Contains => match &left_type {
                        // Runtime list membership uses total equality and
                        // therefore accepts a needle of any type.
                        Type::List(_) => Type::Boolean,
                        Type::Map(key_type, _) => {
                            if !self.are_types_compatible(key_type, &right_type) {
                                self.type_error(
                                    format!(
                                        "Cannot check if {left_type} contains {right_type}, map keys are {key_type}"
                                    ),
                                    Some(*key_type.clone()),
                                    Some(right_type),
                                    *line,
                                    *column,
                                );
                                Type::Error
                            } else {
                                Type::Boolean
                            }
                        }
                        Type::Text => {
                            if right_type != Type::Text && !self.is_gradual_type(&right_type) {
                                self.type_error(
                                    format!("Cannot check if {left_type} contains {right_type}"),
                                    Some(Type::Text),
                                    Some(right_type),
                                    *line,
                                    *column,
                                );
                                Type::Error
                            } else {
                                Type::Boolean
                            }
                        }
                        _ => {
                            self.type_error(
                                format!("Cannot check if {left_type} contains {right_type}"),
                                Some(Type::List(Box::new(Type::Unknown))),
                                Some(left_type),
                                *line,
                                *column,
                            );
                            Type::Error
                        }
                    },
                }
            }
            Expression::UnaryOperation {
                operator,
                expression,
                line,
                column,
            } => {
                let expr_type = self.infer_expression_type(expression);

                if expr_type == Type::Error {
                    return Type::Error;
                }

                match operator {
                    UnaryOperator::Not => {
                        if expr_type == Type::Boolean || self.is_gradual_type(&expr_type) {
                            Type::Boolean
                        } else {
                            self.type_error(
                                format!("Cannot apply 'not' to {expr_type}"),
                                Some(Type::Boolean),
                                Some(expr_type),
                                *line,
                                *column,
                            );
                            Type::Error
                        }
                    }
                    UnaryOperator::Minus => {
                        if expr_type == Type::Number {
                            Type::Number
                        } else if self.is_gradual_type(&expr_type) {
                            expr_type
                        } else {
                            self.type_error(
                                format!("Cannot negate {expr_type}"),
                                Some(Type::Number),
                                Some(expr_type),
                                *line,
                                *column,
                            );
                            Type::Error
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
                if let Expression::Variable(callee, _, _) = &**function
                    && self.should_use_builtin_contract(callee, *line, *column)
                {
                    return self.infer_builtin_call_type(callee, arguments, *line, *column);
                }

                // The idiomatic `of` call form (`greet of "bob"`) parses as a
                // FunctionCall whose callee is a bare Variable. When that callee
                // is not resolvable statically but the program uses `include
                // from`, the action may be exposed by an included file at
                // runtime, so its result type is unknowable — treat it as Any to
                // avoid cascading "could not infer type" errors, mirroring the
                // ActionCall path (issues #580 / #548). Arguments are still
                // inferred so type errors inside them are not missed.
                if let Expression::Variable(callee, _, _) = &**function {
                    let is_known = self.analyzer.get_symbol(callee).is_some()
                        || self.is_callable_without_symbol(callee);
                    if !is_known && self.has_includes {
                        let argument_types: Vec<_> = arguments
                            .iter()
                            .map(|arg| self.infer_expression_type(&arg.value))
                            .collect();
                        self.escape_user_action_list_arguments(arguments, &argument_types);
                        self.escape_all_visible_mutable_state();
                        return Type::Any;
                    }

                    // Stored action references (`store h as f`) resolve
                    // through the analyzer's per-call-site record, which
                    // carries the alias state that held *at this statement*
                    // (snapshot prefix, or Dynamic when control flow made the
                    // binding uncertain).
                    if let Some(resolution) = self
                        .analyzer
                        .alias_call_resolution(callee, *line, *column)
                        .cloned()
                    {
                        match resolution {
                            crate::analyzer::AliasState::Dynamic => {
                                let argument_types: Vec<_> = arguments
                                    .iter()
                                    .map(|arg| self.infer_expression_type(&arg.value))
                                    .collect();
                                self.escape_user_action_list_arguments(arguments, &argument_types);
                                self.escape_all_visible_mutable_state();
                                return Type::Unknown;
                            }
                            crate::analyzer::AliasState::Builtin { name } => {
                                return self
                                    .infer_builtin_call_type(&name, arguments, *line, *column);
                            }
                            crate::analyzer::AliasState::Bound {
                                action,
                                visible_signatures,
                            } => {
                                if let Some(signatures) = self.action_signatures(&action) {
                                    let visible = visible_signatures.min(signatures.len());
                                    return self.infer_overloaded_call_type(
                                        &action,
                                        &signatures[..visible],
                                        arguments,
                                        *line,
                                        *column,
                                    );
                                }
                            }
                        }
                    }

                    // Direct user actions called in the `of` form resolve
                    // through the signature list. This also gives a
                    // forward-referenced single action its provisional
                    // Unknown return without inventing a missing-type error.
                    if !builtins::is_implemented_builtin_function(callee)
                        && let Some(signatures) = self.action_signatures(callee)
                    {
                        return self.infer_overloaded_call_type(
                            callee,
                            &signatures,
                            arguments,
                            *line,
                            *column,
                        );
                    }
                }

                let function_type = self.infer_expression_type(function);
                let user_callee = match function.as_ref() {
                    Expression::Variable(name, ..) => Some(name.as_str()),
                    _ => None,
                };
                let argument_types: Vec<Type> = arguments
                    .iter()
                    .map(|argument| self.infer_expression_type(&argument.value))
                    .collect();

                match function_type {
                    Type::Function {
                        parameters,
                        return_type,
                    } => {
                        if arguments.len() != parameters.len() {
                            self.type_error(
                                format!(
                                    "Function expects {} arguments, but {} were provided",
                                    parameters.len(),
                                    arguments.len()
                                ),
                                None,
                                None,
                                *line,
                                *column,
                            );
                            return Type::Error;
                        }

                        let mut has_type_error = false;
                        for (i, (arg_type, param_type)) in
                            argument_types.iter().zip(parameters.iter()).enumerate()
                        {
                            if !self.are_types_compatible(param_type, arg_type) {
                                self.type_error(
                                    format!(
                                        "Argument {} has incorrect type: expected {}, found {}",
                                        i + 1,
                                        param_type,
                                        arg_type
                                    ),
                                    Some(param_type.clone()),
                                    Some(arg_type.clone()),
                                    *line,
                                    *column,
                                );
                                has_type_error = true;
                            }
                        }

                        if has_type_error {
                            Type::Error
                        } else {
                            self.escape_user_action_list_arguments(arguments, &argument_types);
                            let _ = user_callee;
                            // Reaching this generic Function path means there
                            // is no named WFL action summary (for example, a
                            // stored container method reference). Treat it as
                            // an opaque closure boundary.
                            self.escape_all_visible_mutable_state();
                            *return_type
                        }
                    }
                    Type::Unknown => {
                        self.escape_user_action_list_arguments(arguments, &argument_types);
                        self.escape_all_visible_mutable_state();
                        Type::Unknown
                    }
                    Type::Any => {
                        self.escape_user_action_list_arguments(arguments, &argument_types);
                        self.escape_all_visible_mutable_state();
                        Type::Any
                    }
                    Type::Error => Type::Error,
                    _ => {
                        self.type_error(
                            format!("Cannot call {function_type}, not a function"),
                            Some(Type::Function {
                                parameters: vec![],
                                return_type: Box::new(Type::Unknown),
                            }),
                            Some(function_type),
                            *line,
                            *column,
                        );
                        Type::Error
                    }
                }
            }
            Expression::MemberAccess {
                object,
                property,
                line: _line,
                column: _column,
            } => {
                let object_type = self.infer_expression_type(object);

                if object_type == Type::Error {
                    return Type::Error;
                }

                match object_type {
                    Type::Custom(_) | Type::Unknown => Type::Unknown,
                    Type::Any => Type::Any,
                    Type::Error => Type::Error,
                    _ => {
                        self.type_error(
                            format!("Cannot access property '{property}' on {object_type}"),
                            Some(Type::Custom("Object".to_string())),
                            Some(object_type),
                            *_line,
                            *_column,
                        );
                        Type::Error
                    }
                }
            }
            Expression::IndexAccess {
                collection,
                index,
                line,
                column,
            } => {
                let collection_type = self.infer_expression_type(collection);
                let index_type = self.infer_expression_type(index);

                if collection_type == Type::Error || index_type == Type::Error {
                    return Type::Error;
                }

                match collection_type {
                    Type::List(item_type) => {
                        if index_type != Type::Number
                            && index_type != Type::Unknown
                            && index_type != Type::Any
                        {
                            self.type_error(
                                format!("List index must be a number, got {index_type}"),
                                Some(Type::Number),
                                Some(index_type),
                                *line,
                                *column,
                            );
                            Type::Error
                        } else {
                            *item_type
                        }
                    }
                    Type::Map(key_type, value_type) => {
                        if !self.are_types_compatible(&key_type, &index_type) {
                            self.type_error(
                                format!("Map key must be {key_type}, got {index_type}"),
                                Some(*key_type.clone()),
                                Some(index_type),
                                *line,
                                *column,
                            );
                            Type::Error
                        } else {
                            *value_type
                        }
                    }
                    Type::Text => {
                        if index_type != Type::Number
                            && index_type != Type::Unknown
                            && index_type != Type::Any
                        {
                            self.type_error(
                                format!("Text index must be a number, got {index_type}"),
                                Some(Type::Number),
                                Some(index_type),
                                *line,
                                *column,
                            );
                            Type::Error
                        } else {
                            Type::Text
                        }
                    }
                    Type::Unknown => Type::Unknown,
                    // A dynamically-typed collection (e.g. a parse_json
                    // result) is indexable; the element type is only known
                    // at runtime (issue #553).
                    Type::Any => Type::Any,
                    // Stream handles expose fields (`status`/`ok`/`headers`) by
                    // index; the key must be text (runtime object indexing rejects
                    // a numeric key), and the field type is only known at runtime.
                    Type::Custom(ref name) if name == "HttpStream" || name == "ResponseStream" => {
                        if index_type != Type::Text
                            && index_type != Type::Unknown
                            && index_type != Type::Any
                            && index_type != Type::Error
                        {
                            self.type_error(
                                format!("Stream handle field name must be text, got {index_type}"),
                                Some(Type::Text),
                                Some(index_type),
                                *line,
                                *column,
                            );
                            Type::Error
                        } else {
                            match &**index {
                                Expression::Literal(Literal::String(field), ..) => {
                                    Self::stream_field_type(name, field).unwrap_or(Type::Any)
                                }
                                _ => Type::Any,
                            }
                        }
                    }
                    _ => {
                        self.type_error(
                            format!("Cannot index into {collection_type}"),
                            Some(Type::List(Box::new(Type::Unknown))),
                            Some(collection_type),
                            *line,
                            *column,
                        );
                        Type::Error
                    }
                }
            }
            Expression::Concatenation {
                left,
                right,
                line: _line,
                column: _column,
            } => {
                let left_type = self.infer_expression_type(left);
                let right_type = self.infer_expression_type(right);

                if left_type == Type::Error || right_type == Type::Error {
                    return Type::Error;
                }

                // Allow concatenation of any types - they will be converted to text at runtime
                // This matches the interpreter's behavior which converts values to strings
                Type::Text
            }
            Expression::PatternMatch {
                text,
                pattern,
                line,
                column,
            } => {
                let text_type = self.infer_expression_type(text);
                let pattern_type = self.infer_expression_type(pattern);

                if text_type != Type::Text && !self.is_gradual_type(&text_type) {
                    self.type_error(
                        format!("Expected Text for pattern matching, got {text_type}"),
                        Some(Type::Text),
                        Some(text_type),
                        *line,
                        *column,
                    );
                }

                if pattern_type != Type::Pattern && !self.is_gradual_type(&pattern_type) {
                    self.type_error(
                        format!("Expected Pattern for pattern matching, got {pattern_type}"),
                        Some(Type::Pattern),
                        Some(pattern_type),
                        *line,
                        *column,
                    );
                }

                Type::Boolean
            }
            Expression::PatternFind {
                text,
                pattern,
                line,
                column,
            } => {
                let text_type = self.infer_expression_type(text);
                let pattern_type = self.infer_expression_type(pattern);

                if text_type != Type::Text && !self.is_gradual_type(&text_type) {
                    self.type_error(
                        format!("Expected Text for pattern finding, got {text_type}"),
                        Some(Type::Text),
                        Some(text_type),
                        *line,
                        *column,
                    );
                }

                if pattern_type != Type::Pattern && !self.is_gradual_type(&pattern_type) {
                    self.type_error(
                        format!("Expected Pattern for pattern finding, got {pattern_type}"),
                        Some(Type::Pattern),
                        Some(pattern_type),
                        *line,
                        *column,
                    );
                }

                Type::Optional(Box::new(Type::Map(
                    Box::new(Type::Text),
                    Box::new(Type::Any),
                )))
            }
            Expression::PatternReplace {
                text,
                pattern,
                replacement,
                ..
            } => {
                let text_type = self.infer_expression_type(text);
                let pattern_type = self.infer_expression_type(pattern);
                let replacement_type = self.infer_expression_type(replacement);

                if text_type != Type::Text && !self.is_gradual_type(&text_type) {
                    self.type_error(
                        format!("Expected Text for pattern replacement, got {text_type}"),
                        Some(Type::Text),
                        Some(text_type),
                        0,
                        0,
                    );
                }

                if pattern_type != Type::Pattern && !self.is_gradual_type(&pattern_type) {
                    self.type_error(
                        format!("Expected Pattern for pattern replacement, got {pattern_type}"),
                        Some(Type::Pattern),
                        Some(pattern_type),
                        0,
                        0,
                    );
                }

                if replacement_type != Type::Text && !self.is_gradual_type(&replacement_type) {
                    self.type_error(
                        format!("Expected Text for replacement, got {replacement_type}"),
                        Some(Type::Text),
                        Some(replacement_type),
                        0,
                        0,
                    );
                }

                Type::Text
            }
            Expression::PatternSplit { text, pattern, .. } => {
                let text_type = self.infer_expression_type(text);
                let pattern_type = self.infer_expression_type(pattern);

                if text_type != Type::Text && !self.is_gradual_type(&text_type) {
                    self.type_error(
                        format!("Expected Text for pattern splitting, got {text_type}"),
                        Some(Type::Text),
                        Some(text_type),
                        0,
                        0,
                    );
                }

                if pattern_type != Type::Pattern && !self.is_gradual_type(&pattern_type) {
                    self.type_error(
                        format!("Expected Pattern for pattern splitting, got {pattern_type}"),
                        Some(Type::Pattern),
                        Some(pattern_type),
                        0,
                        0,
                    );
                }

                Type::List(Box::new(Type::Text))
            }
            Expression::StringSplit {
                text,
                delimiter,
                line,
                column,
            } => {
                let text_type = self.infer_expression_type(text);
                let delimiter_type = self.infer_expression_type(delimiter);

                // Accept statically-unknown operands (Unknown from untyped params,
                // Any from list-index/map results) without a false ERROR — they are
                // verified at runtime (gradual typing, issue #567).
                if text_type != Type::Text && !self.is_gradual_type(&text_type) {
                    self.type_error(
                        format!("Expected Text for string splitting, got {text_type}"),
                        Some(Type::Text),
                        Some(text_type),
                        *line,
                        *column,
                    );
                }

                if delimiter_type != Type::Text && !self.is_gradual_type(&delimiter_type) {
                    self.type_error(
                        format!("Expected Text for delimiter, got {delimiter_type}"),
                        Some(Type::Text),
                        Some(delimiter_type),
                        *line,
                        *column,
                    );
                }

                Type::List(Box::new(Type::Text))
            }
            Expression::AwaitExpression {
                expression,
                line,
                column,
            } => {
                let expr_type = self.infer_expression_type(expression);

                match expr_type {
                    Type::Async(inner_type) => *inner_type,
                    Type::Unknown => Type::Unknown,
                    Type::Any => Type::Any,
                    Type::Error => Type::Error,
                    _ => {
                        self.type_error(
                            format!("Cannot await non-async value of type {expr_type}"),
                            Some(Type::Async(Box::new(Type::Unknown))),
                            Some(expr_type),
                            *line,
                            *column,
                        );
                        Type::Error
                    }
                }
            }
            Expression::ActionCall {
                name,
                arguments,
                line: _line,
                column: _column,
            } => {
                // Builtins share argument traversal, arity checks, registered
                // parameter contracts, and return inference with the `of` form.
                if self.should_use_builtin_contract(name, *_line, *_column) {
                    return self.infer_builtin_call_type(name, arguments, *_line, *_column);
                }

                // Stored action references called with `call ... with` get the
                // same per-call-site alias resolution as the `of` form.
                if let Some(resolution) = self
                    .analyzer
                    .alias_call_resolution(name, *_line, *_column)
                    .cloned()
                {
                    match resolution {
                        crate::analyzer::AliasState::Dynamic => {
                            let argument_types: Vec<_> = arguments
                                .iter()
                                .map(|arg| self.infer_expression_type(&arg.value))
                                .collect();
                            self.escape_user_action_list_arguments(arguments, &argument_types);
                            self.escape_all_visible_mutable_state();
                            return Type::Unknown;
                        }
                        crate::analyzer::AliasState::Builtin { name } => {
                            return self
                                .infer_builtin_call_type(&name, arguments, *_line, *_column);
                        }
                        crate::analyzer::AliasState::Bound {
                            action,
                            visible_signatures,
                        } => {
                            if let Some(signatures) = self.action_signatures(&action) {
                                let visible = visible_signatures.min(signatures.len());
                                return self.infer_overloaded_call_type(
                                    &action,
                                    &signatures[..visible],
                                    arguments,
                                    *_line,
                                    *_column,
                                );
                            }
                        }
                    }
                }

                // Direct actions resolve through their registered signatures,
                // including a forward-referenced single definition whose
                // result is still provisional.
                if let Some(signatures) = self.action_signatures(name) {
                    return self.infer_overloaded_call_type(
                        name,
                        &signatures,
                        arguments,
                        *_line,
                        *_column,
                    );
                }

                let symbol_opt = self.analyzer.get_symbol(name);

                if symbol_opt.is_none() {
                    // Check if this is an action parameter, builtin function, or special function name before reporting it as undefined
                    if self.is_callable_without_symbol(name) {
                        // It's an action parameter or a special function name, so don't report an error
                        // For builtin functions, return their proper type
                        if Analyzer::is_builtin_function(name) {
                            return self.infer_builtin_call_type(name, arguments, *_line, *_column);
                        }
                        let argument_types: Vec<_> = arguments
                            .iter()
                            .map(|argument| self.infer_expression_type(&argument.value))
                            .collect();
                        self.escape_user_action_list_arguments(arguments, &argument_types);
                        self.escape_all_visible_mutable_state();
                        return Type::Unknown;
                    } else if self.has_includes {
                        // Action may be provided by an included file at runtime;
                        // its result type is unknowable statically, so treat it as
                        // Any to avoid cascading "could not infer type" errors.
                        // Still infer each argument expression first so type errors
                        // inside the arguments are not missed in include-using
                        // programs.
                        let argument_types: Vec<_> = arguments
                            .iter()
                            .map(|arg| self.infer_expression_type(&arg.value))
                            .collect();
                        self.escape_user_action_list_arguments(arguments, &argument_types);
                        self.escape_all_visible_mutable_state();
                        return Type::Any;
                    } else {
                        for argument in arguments {
                            self.infer_expression_type(&argument.value);
                        }
                        self.type_error(
                            format!("Undefined action '{name}'"),
                            None,
                            None,
                            *_line,
                            *_column,
                        );
                        return Type::Error;
                    }
                }

                let symbol = symbol_opt.unwrap();

                if symbol.symbol_type.is_none() {
                    let argument_types: Vec<_> = arguments
                        .iter()
                        .map(|argument| self.infer_expression_type(&argument.value))
                        .collect();
                    self.escape_user_action_list_arguments(arguments, &argument_types);
                    self.type_error(
                        format!("Cannot determine type of action '{name}'"),
                        None,
                        None,
                        *_line,
                        *_column,
                    );
                    return Type::Unknown;
                }

                let symbol_type = symbol.symbol_type.clone().unwrap();
                let arg_types: Vec<Type> = arguments
                    .iter()
                    .map(|argument| self.infer_expression_type(&argument.value))
                    .collect();

                match symbol_type {
                    Type::Function {
                        parameters,
                        return_type,
                    } => {
                        if arguments.len() != parameters.len() {
                            self.type_error(
                                format!(
                                    "Action '{}' expects {} arguments, but {} were provided",
                                    name,
                                    parameters.len(),
                                    arguments.len()
                                ),
                                None,
                                None,
                                *_line,
                                *_column,
                            );
                            return Type::Error;
                        }

                        for (i, (param_type, arg_type)) in
                            parameters.iter().zip(arg_types.iter()).enumerate()
                        {
                            if !self.are_types_compatible(param_type, arg_type) {
                                self.type_error(
                                    format!(
                                        "Argument {} of action '{}' expects {}, but got {}",
                                        i + 1,
                                        name,
                                        param_type,
                                        arg_type
                                    ),
                                    Some(param_type.clone()),
                                    Some(arg_type.clone()),
                                    *_line,
                                    *_column,
                                );
                                return Type::Error;
                            }
                        }

                        self.escape_user_action_list_arguments(arguments, &arg_types);
                        let keys = vec![(name.clone(), 0)];
                        self.apply_user_action_list_effects(&keys);
                        self.escape_shared_list_return_type(&keys, *return_type)
                    }
                    _ => {
                        self.type_error(
                            format!("'{name}' is not an action"),
                            Some(Type::Function {
                                parameters: vec![],
                                return_type: Box::new(Type::Unknown),
                            }),
                            Some(symbol_type),
                            *_line,
                            *_column,
                        );
                        Type::Error
                    }
                }
            }
            Expression::StaticMemberAccess {
                container,
                member,
                line,
                column,
            } => {
                if self.analyzer.get_container(container).is_some() {
                    if let Some(property_type) =
                        self.container_static_property_type(container, member)
                    {
                        return property_type;
                    }
                    if let Some(method_info) = self.container_static_method(container, member) {
                        return Type::Function {
                            parameters: method_info
                                .parameters
                                .iter()
                                .map(|p| p.param_type.as_ref().cloned().unwrap_or(Type::Unknown))
                                .collect(),
                            return_type: Box::new(method_info.return_type),
                        };
                    }
                    self.errors.push(TypeError::new(
                        format!("Static member '{member}' not found in container '{container}'"),
                        None,
                        None,
                        *line,
                        *column,
                    ));
                    Type::Error
                } else {
                    // Container not found
                    self.errors.push(TypeError::new(
                        format!("Container '{container}' not found"),
                        None,
                        None,
                        *line,
                        *column,
                    ));
                    Type::Error
                }
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                line,
                column,
            } => {
                // First, determine the type of the object
                let object_type = self.infer_expression_type(object);
                // Runtime evaluates every argument before dispatch, including
                // extra arguments and calls that later fail lookup/arity.
                let argument_types: Vec<Type> = arguments
                    .iter()
                    .map(|argument| self.infer_expression_type(&argument.value))
                    .collect();

                // Check if the object is a container instance.
                let result = match object_type {
                    Type::Container(container_name) => {
                        if let Some(method_info) =
                            self.container_static_method(&container_name, method)
                        {
                            if arguments.len() != method_info.parameters.len() {
                                self.errors.push(TypeError::new(
                                    format!(
                                        "Static method '{}' expects {} arguments but {} were provided",
                                        method,
                                        method_info.parameters.len(),
                                        arguments.len()
                                    ),
                                    None,
                                    None,
                                    *line,
                                    *column,
                                ));
                            }
                            for (index, (argument_type, parameter)) in argument_types
                                .iter()
                                .zip(&method_info.parameters)
                                .enumerate()
                            {
                                let expected = parameter
                                    .param_type
                                    .as_ref()
                                    .cloned()
                                    .unwrap_or(Type::Unknown);
                                if !self.are_types_compatible(&expected, argument_type) {
                                    self.errors.push(TypeError::new(
                                        format!(
                                            "Argument {} of static method '{}' has type {} but expected {}",
                                            index + 1,
                                            method,
                                            argument_type,
                                            expected
                                        ),
                                        Some(expected),
                                        Some(argument_type.clone()),
                                        *line,
                                        *column,
                                    ));
                                }
                            }
                            method_info.return_type
                        } else {
                            self.errors.push(TypeError::new(
                                format!(
                                    "Static method '{method}' not found in container '{container_name}'"
                                ),
                                None,
                                None,
                                *line,
                                *column,
                            ));
                            Type::Error
                        }
                    }
                    Type::ContainerInstance(container_name) => {
                        // Look up the container in the analyzer's registry
                        if let Some(container_info) = self.analyzer.get_container(&container_name) {
                            // Look up the method in the container
                            if let Some(method_info) = container_info.methods.get(method) {
                                // Check argument count
                                let param_count = method_info.parameters.len();
                                let return_type = method_info.return_type.clone();
                                let method_params = method_info.parameters.clone();

                                if arguments.len() != param_count {
                                    self.errors.push(TypeError::new(
                                        format!(
                                            "Method '{}' expects {} arguments but {} were provided",
                                            method,
                                            param_count,
                                            arguments.len()
                                        ),
                                        None,
                                        None,
                                        *line,
                                        *column,
                                    ));
                                }

                                // Check argument types
                                for (i, (arg_type, param)) in
                                    argument_types.iter().zip(&method_params).enumerate()
                                {
                                    let expected_type =
                                        param.param_type.as_ref().cloned().unwrap_or(Type::Unknown);

                                    if !self.are_types_compatible(&expected_type, arg_type) {
                                        self.errors.push(TypeError::new(
                                            format!(
                                                "Argument {} of method '{}' has type {} but expected {}",
                                                i + 1,
                                                method,
                                                arg_type,
                                                expected_type
                                            ),
                                            Some(expected_type),
                                            Some(arg_type.clone()),
                                            *line,
                                            *column,
                                        ));
                                    }
                                }

                                // Return the method's return type
                                return_type
                            } else {
                                // Check parent containers if the method is not found
                                let mut current_container = container_info.extends.as_ref();
                                let mut found_method = None;
                                let mut visited = HashSet::new();

                                while let Some(parent_name) = current_container {
                                    if !visited.insert(parent_name.as_str()) {
                                        break;
                                    }
                                    if let Some(parent_info) =
                                        self.analyzer.get_container(parent_name)
                                    {
                                        if let Some(method_info) = parent_info.methods.get(method) {
                                            found_method = Some((
                                                method_info.parameters.clone(),
                                                method_info.return_type.clone(),
                                            ));
                                            break;
                                        }
                                        current_container = parent_info.extends.as_ref();
                                    } else {
                                        break;
                                    }
                                }

                                if let Some((method_params, return_type)) = found_method {
                                    // Found in parent - do the same checks
                                    if arguments.len() != method_params.len() {
                                        self.errors.push(TypeError::new(
                                            format!(
                                                "Method '{}' expects {} arguments but {} were provided",
                                                method,
                                                method_params.len(),
                                                arguments.len()
                                            ),
                                            None,
                                            None,
                                            *line,
                                            *column,
                                        ));
                                    }

                                    for (i, (arg_type, param)) in
                                        argument_types.iter().zip(&method_params).enumerate()
                                    {
                                        let expected_type = param
                                            .param_type
                                            .as_ref()
                                            .cloned()
                                            .unwrap_or(Type::Unknown);

                                        if !self.are_types_compatible(&expected_type, arg_type) {
                                            self.errors.push(TypeError::new(
                                                format!(
                                                    "Argument {} of method '{}' has type {} but expected {}",
                                                    i + 1,
                                                    method,
                                                    arg_type,
                                                    expected_type
                                                ),
                                                Some(expected_type),
                                                Some(arg_type.clone()),
                                                *line,
                                                *column,
                                            ));
                                        }
                                    }

                                    return_type
                                } else {
                                    self.errors.push(TypeError::new(
                                        format!(
                                            "Method '{method}' not found in container '{container_name}'"
                                        ),
                                        None,
                                        None,
                                        *line,
                                        *column,
                                    ));
                                    Type::Error
                                }
                            }
                        } else {
                            self.errors.push(TypeError::new(
                                format!("Container '{container_name}' not found"),
                                None,
                                None,
                                *line,
                                *column,
                            ));
                            Type::Error
                        }
                    }
                    Type::Unknown => Type::Unknown,
                    Type::Any => Type::Any,
                    Type::Error => Type::Error,
                    _ => {
                        self.type_error(
                            format!(
                                "Cannot call method '{method}' on non-container type {object_type}"
                            ),
                            Some(Type::ContainerInstance(String::from("Unknown"))),
                            Some(object_type),
                            *line,
                            *column,
                        );
                        Type::Error
                    }
                };
                if result != Type::Error {
                    self.escape_user_action_list_arguments(arguments, &argument_types);
                    // Methods can close over the caller's runtime environment
                    // and container properties can retain shared list values.
                    // Until method/property effect summaries carry those paths,
                    // this is the explicit conservative user-code boundary.
                    self.escape_all_visible_mutable_state();
                }
                Self::escape_possible_shared_list_return_type(result)
            }
            Expression::PropertyAccess {
                object,
                property,
                line,
                column,
            } => {
                let object_type = self.infer_expression_type(object);
                self.infer_property_access_type(object_type, property, *line, *column)
                    .0
            }
            Expression::FileExists { path, line, column }
            | Expression::DirectoryExists { path, line, column }
            | Expression::ListFiles { path, line, column } => {
                let path_type = self.infer_expression_type(path);
                if path_type != Type::Text && !self.is_gradual_type(&path_type) {
                    self.type_error(
                        "Filesystem path must be text".to_string(),
                        Some(Type::Text),
                        Some(path_type),
                        *line,
                        *column,
                    );
                }
                match expression {
                    Expression::FileExists { .. } | Expression::DirectoryExists { .. } => {
                        Type::Boolean
                    }
                    Expression::ListFiles { .. } => Type::List(Box::new(Type::Text)),
                    _ => unreachable!(),
                }
            }
            Expression::ReadContent {
                file_handle,
                line,
                column,
            }
            | Expression::ReadBinaryContent {
                file_handle,
                line,
                column,
            }
            | Expression::FileSizeOf {
                file_handle,
                line,
                column,
            } => {
                let handle_type = self.infer_expression_type(file_handle);
                if handle_type != Type::Text
                    && handle_type != Type::Custom("File".to_string())
                    && !self.is_gradual_type(&handle_type)
                {
                    self.type_error(
                        "File handle or path must be text".to_string(),
                        Some(Type::Text),
                        Some(handle_type),
                        *line,
                        *column,
                    );
                }
                match expression {
                    Expression::ReadContent { .. } => Type::Text,
                    Expression::ReadBinaryContent { .. } => Type::Binary,
                    Expression::FileSizeOf { .. } => Type::Number,
                    _ => unreachable!(),
                }
            }
            Expression::ReadBinaryN {
                file_handle,
                count,
                line,
                column,
            } => {
                let handle_type = self.infer_expression_type(file_handle);
                if handle_type != Type::Text
                    && handle_type != Type::Custom("File".to_string())
                    && !self.is_gradual_type(&handle_type)
                {
                    self.type_error(
                        "File handle or path must be text".to_string(),
                        Some(Type::Text),
                        Some(handle_type),
                        *line,
                        *column,
                    );
                }
                let count_type = self.infer_expression_type(count);
                if count_type != Type::Number && !self.is_gradual_type(&count_type) {
                    self.type_error(
                        "Binary byte count must be a number".to_string(),
                        Some(Type::Number),
                        Some(count_type),
                        *line,
                        *column,
                    );
                }
                Type::Binary
            }
            Expression::ListFilesRecursive {
                path,
                extensions,
                line,
                column,
            } => {
                self.check_file_listing_operands(
                    path,
                    extensions.as_deref().unwrap_or_default(),
                    *line,
                    *column,
                );
                Type::List(Box::new(Type::Text))
            }
            Expression::ListFilesFiltered {
                path,
                extensions,
                line,
                column,
            } => {
                self.check_file_listing_operands(path, extensions, *line, *column);
                Type::List(Box::new(Type::Text))
            }
            Expression::HeaderAccess {
                request,
                line,
                column,
                ..
            } => {
                // Request objects are currently gradual/map-shaped, but the
                // operand still needs traversal so nested diagnostics survive.
                let request_type = self.infer_expression_type(request);
                let headers_fallback = self
                    .analyzer
                    .get_symbol("headers")
                    .and_then(|symbol| symbol.symbol_type.clone());
                let has_headers_fallback = headers_fallback
                    .as_ref()
                    .is_some_and(|ty| matches!(ty, Type::Map(_, _)) || self.is_gradual_type(ty));
                if !self.is_execute_file_request_type(&request_type) && !has_headers_fallback {
                    self.type_error(
                        "Header access requires a request object or request headers in scope"
                            .to_string(),
                        Some(Type::Custom("Request".to_string())),
                        Some(request_type.clone()),
                        *line,
                        *column,
                    );
                }
                let header_value_type = match &request_type {
                    Type::Custom(name) if name == "Request" => Type::Text,
                    Type::Map(_, _) | Type::Unknown | Type::Any | Type::Error => Type::Any,
                    _ => match headers_fallback {
                        Some(Type::Map(_, value_type)) => *value_type,
                        Some(Type::Unknown | Type::Any | Type::Error) => Type::Any,
                        _ => Type::Error,
                    },
                };
                Type::Optional(Box::new(header_value_type))
            }
            Expression::CurrentTimeMilliseconds { .. } => Type::Number,
            Expression::CurrentTimeFormatted { .. } => Type::Text,
            Expression::ProcessRunning {
                process_id,
                line,
                column,
            } => {
                let process_type = self.infer_expression_type(process_id);
                if process_type != Type::Text && !self.is_gradual_type(&process_type) {
                    self.type_error(
                        "Process ID must be text".to_string(),
                        Some(Type::Text),
                        Some(process_type),
                        *line,
                        *column,
                    );
                }
                Type::Boolean
            }
            Expression::DatabaseQuery {
                db,
                sql,
                parameters,
                kind,
                line,
                column,
            } => {
                self.check_database_query_operands(db, sql, parameters.as_deref(), *line, *column);
                Self::database_result_type(*kind)
            }
        }
    }

    /// Builtin contracts are independent of program symbols and constructor
    /// choice; the CLI supplies an already-run analyzer that does not contain
    /// these registrations.
    fn builtin_signatures(&self, name: &str) -> Option<Vec<crate::analyzer::FunctionSignature>> {
        let symbol = self.builtin_contracts.get_symbol(name)?;
        if let SymbolKind::Function { signatures } = &symbol.kind {
            Some(signatures.clone())
        } else {
            None
        }
    }

    fn check_file_listing_operands(
        &mut self,
        path: &Expression,
        extensions: &[Expression],
        line: usize,
        column: usize,
    ) {
        let path_type = self.infer_expression_type(path);
        if path_type != Type::Text && !self.is_gradual_type(&path_type) {
            self.type_error(
                "Directory path must be text".to_string(),
                Some(Type::Text),
                Some(path_type),
                line,
                column,
            );
        }

        for extension in extensions {
            let extension_type = self.infer_expression_type(extension);
            let valid = match &extension_type {
                Type::Text => true,
                Type::List(item_type) => {
                    **item_type == Type::Text || self.is_gradual_type(item_type)
                }
                other => self.is_gradual_type(other),
            };
            if !valid {
                self.type_error(
                    "File extension filter must be text or a list of text".to_string(),
                    None,
                    Some(extension_type),
                    line,
                    column,
                );
            }
        }
    }

    /// Validate the operand types of a database query/execute form. Shared by
    /// `DatabaseQueryStatement` and the `Expression::DatabaseQuery` arm so the
    /// two paths cannot drift apart.
    fn check_database_query_operands(
        &mut self,
        db: &Expression,
        sql: &Expression,
        parameters: Option<&Expression>,
        line: usize,
        column: usize,
    ) {
        let db_type = self.infer_expression_type(db);
        if db_type != Type::Custom("Database".to_string()) && !self.is_gradual_type(&db_type) {
            self.type_error(
                "Expected a Database connection".to_string(),
                Some(Type::Custom("Database".to_string())),
                Some(db_type),
                line,
                column,
            );
        }

        let sql_type = self.infer_expression_type(sql);
        if sql_type != Type::Text && !self.is_gradual_type(&sql_type) {
            self.type_error(
                "SQL statement must be a text string".to_string(),
                Some(Type::Text),
                Some(sql_type),
                line,
                column,
            );
        }

        if let Some(params) = parameters {
            let params_type = self.infer_expression_type(params);
            let valid = match &params_type {
                Type::List(item_type) => self.is_sql_parameter_type(item_type),
                other => self.is_gradual_type(other),
            };
            if !valid {
                self.type_error(
                    "Query parameters must be a list of SQL scalar values".to_string(),
                    Some(Type::List(Box::new(Type::Any))),
                    Some(params_type),
                    line,
                    column,
                );
            }
        }
    }

    fn is_sql_parameter_type(&self, ty: &Type) -> bool {
        if let Type::Optional(inner) = ty {
            return self.is_sql_parameter_type(inner);
        }
        matches!(
            ty,
            Type::Text
                | Type::Number
                | Type::Boolean
                | Type::Binary
                | Type::Date
                | Type::Time
                | Type::DateTime
                | Type::Nothing
                | Type::Unknown
                | Type::Any
                | Type::Error
        ) || self.is_unambiguous_temporal_type(ty)
    }

    /// Result type of a database query/execute. Rows are objects keyed by
    /// column name; execute results are {affected_rows, last_insert_id}.
    /// Typing them as text-keyed maps lets downstream indexing typecheck
    /// cleanly.
    fn database_result_type(kind: crate::parser::ast::DatabaseQueryKind) -> Type {
        let row_type = Type::Map(Box::new(Type::Text), Box::new(Type::Any));
        match kind {
            crate::parser::ast::DatabaseQueryKind::Query => Type::List(Box::new(row_type)),
            crate::parser::ast::DatabaseQueryKind::Execute => row_type,
        }
    }

    fn validate_signal_handler_statement(
        &mut self,
        signal_type: &str,
        handler_name: &str,
        line: usize,
        column: usize,
    ) {
        // Validate signal type
        // List of valid signals based on common POSIX signals that are handleable
        // SIGKILL and SIGSTOP cannot be handled, so they are excluded
        let valid_signals = [
            "SIGINT", "SIGTERM", "SIGHUP", "SIGQUIT", "SIGABRT", "SIGALRM", "SIGCHLD", "SIGCONT",
            "SIGPIPE", "SIGUSR1", "SIGUSR2", "INT", "TERM", "HUP", "QUIT", "ABRT", "ALRM", "CHLD",
            "CONT", "PIPE", "USR1", "USR2",
        ];

        if !valid_signals.contains(&signal_type) {
            self.type_error(
                format!(
                    "Invalid signal type: '{signal_type}'. Supported signals include: {}",
                    valid_signals.join(", ")
                ),
                None,
                None,
                line,
                column,
            );
        }

        // Validate handler name
        if let Some(symbol) = self.analyzer.get_symbol(handler_name) {
            if let Some(symbol_type) = &symbol.symbol_type {
                match symbol_type {
                    Type::Function {
                        parameters,
                        return_type: _,
                    } => {
                        // Handler should accept 0 or 1 arguments
                        if parameters.len() > 1 {
                            self.type_error(
                                format!(
                                    "Signal handler '{handler_name}' must accept 0 or 1 arguments, but accepts {}",
                                    parameters.len()
                                ),
                                None,
                                None,
                                line,
                                column,
                            );
                        } else if parameters.len() == 1 {
                            // If it accepts an argument, it must be a Number (the signal number)
                            // Also allow Unknown for backward compatibility with untyped parameters
                            let param_type = &parameters[0];
                            if *param_type != Type::Number && !self.is_gradual_type(param_type) {
                                self.type_error(
                                    format!(
                                        "Signal handler parameter must be a Number (signal code), but got {}",
                                        param_type
                                    ),
                                    Some(Type::Number),
                                    Some(param_type.clone()),
                                    line,
                                    column,
                                );
                            }
                        }
                    }
                    _ => {
                        self.type_error(
                            format!("'{handler_name}' is not a function"),
                            Some(Type::Function {
                                parameters: vec![],
                                return_type: Box::new(Type::Nothing),
                            }),
                            Some(symbol_type.clone()),
                            line,
                            column,
                        );
                    }
                }
            } else {
                self.type_error(
                    format!("Cannot determine type of signal handler '{handler_name}'"),
                    None,
                    None,
                    line,
                    column,
                );
            }
        } else {
            self.type_error(
                format!("Undefined signal handler '{handler_name}'"),
                None,
                None,
                line,
                column,
            );
        }
    }

    fn join_return_types(left: Type, right: Type) -> Type {
        match (left, right) {
            (Type::Error, _) | (_, Type::Error) => Type::Error,
            (Type::Nothing, Type::Nothing) => Type::Nothing,
            (Type::Nothing, other) | (other, Type::Nothing) => Self::optionalize(other),
            (left, right) => Self::join_inferred_types(left, right),
        }
    }

    #[allow(dead_code)]
    fn action_block_must_terminate(statements: &[Statement]) -> bool {
        for statement in statements {
            let must_terminate = match statement {
                Statement::ReturnStatement { .. } | Statement::ExitStatement { .. } => true,
                Statement::IfStatement {
                    condition: Expression::Literal(Literal::Boolean(true), ..),
                    then_block,
                    ..
                } => Self::action_block_must_terminate(then_block),
                Statement::IfStatement {
                    condition: Expression::Literal(Literal::Boolean(false), ..),
                    else_block,
                    ..
                } => else_block
                    .as_ref()
                    .is_some_and(|block| Self::action_block_must_terminate(block)),
                Statement::IfStatement {
                    then_block,
                    else_block: Some(else_block),
                    ..
                } => {
                    Self::action_block_must_terminate(then_block)
                        && Self::action_block_must_terminate(else_block)
                }
                Statement::SingleLineIf {
                    condition: Expression::Literal(Literal::Boolean(true), ..),
                    then_stmt,
                    ..
                } => Self::action_block_must_terminate(std::slice::from_ref(then_stmt.as_ref())),
                Statement::SingleLineIf {
                    condition: Expression::Literal(Literal::Boolean(false), ..),
                    else_stmt,
                    ..
                } => else_stmt.as_ref().is_some_and(|statement| {
                    Self::action_block_must_terminate(std::slice::from_ref(statement.as_ref()))
                }),
                Statement::SingleLineIf {
                    then_stmt,
                    else_stmt: Some(else_stmt),
                    ..
                } => {
                    Self::action_block_must_terminate(std::slice::from_ref(then_stmt.as_ref()))
                        && Self::action_block_must_terminate(std::slice::from_ref(
                            else_stmt.as_ref(),
                        ))
                }
                Statement::TryStatement {
                    body,
                    when_clauses,
                    otherwise_block,
                    finally_block,
                    ..
                } => {
                    if finally_block
                        .as_ref()
                        .is_some_and(|block| Self::action_block_must_terminate(block))
                    {
                        true
                    } else {
                        Self::action_block_must_terminate(body)
                            && when_clauses
                                .iter()
                                .all(|clause| Self::action_block_must_terminate(&clause.body))
                            && otherwise_block
                                .as_ref()
                                .is_none_or(|block| Self::action_block_must_terminate(block))
                    }
                }
                Statement::WaitForStatement { inner, .. } => {
                    Self::action_block_must_terminate(std::slice::from_ref(inner.as_ref()))
                }
                Statement::ForeverLoop { body, .. } | Statement::MainLoop { body, .. } => {
                    !Self::block_may_break_current_loop(body)
                }
                Statement::WhileLoop {
                    condition: Expression::Literal(Literal::Boolean(true), ..),
                    body,
                    ..
                }
                | Statement::RepeatWhileLoop {
                    condition: Expression::Literal(Literal::Boolean(true), ..),
                    body,
                    ..
                } => !Self::block_may_break_current_loop(body),
                Statement::RepeatUntilLoop {
                    condition, body, ..
                } => {
                    Self::action_block_must_terminate(body)
                        || (matches!(condition, Expression::Literal(Literal::Boolean(false), ..))
                            && !Self::block_may_break_current_loop(body))
                }
                _ => false,
            };
            if must_terminate {
                return true;
            }
        }
        false
    }

    fn block_may_break_current_loop(statements: &[Statement]) -> bool {
        statements
            .iter()
            .any(Self::statement_may_break_current_loop)
    }

    fn statement_may_break_current_loop(statement: &Statement) -> bool {
        match statement {
            Statement::BreakStatement { .. } => true,
            Statement::IfStatement {
                condition: Expression::Literal(Literal::Boolean(true), ..),
                then_block,
                ..
            } => Self::block_may_break_current_loop(then_block),
            Statement::IfStatement {
                condition: Expression::Literal(Literal::Boolean(false), ..),
                else_block,
                ..
            } => else_block
                .as_ref()
                .is_some_and(|block| Self::block_may_break_current_loop(block)),
            Statement::IfStatement {
                then_block,
                else_block,
                ..
            } => {
                Self::block_may_break_current_loop(then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|block| Self::block_may_break_current_loop(block))
            }
            Statement::SingleLineIf {
                condition: Expression::Literal(Literal::Boolean(true), ..),
                then_stmt,
                ..
            } => Self::statement_may_break_current_loop(then_stmt),
            Statement::SingleLineIf {
                condition: Expression::Literal(Literal::Boolean(false), ..),
                else_stmt,
                ..
            } => else_stmt
                .as_ref()
                .is_some_and(|statement| Self::statement_may_break_current_loop(statement)),
            Statement::SingleLineIf {
                then_stmt,
                else_stmt,
                ..
            } => {
                Self::statement_may_break_current_loop(then_stmt)
                    || else_stmt
                        .as_ref()
                        .is_some_and(|statement| Self::statement_may_break_current_loop(statement))
            }
            Statement::TryStatement {
                body,
                when_clauses,
                otherwise_block,
                finally_block,
                ..
            } => {
                Self::block_may_break_current_loop(body)
                    || when_clauses
                        .iter()
                        .any(|clause| Self::block_may_break_current_loop(&clause.body))
                    || otherwise_block
                        .as_ref()
                        .is_some_and(|block| Self::block_may_break_current_loop(block))
                    || finally_block
                        .as_ref()
                        .is_some_and(|block| Self::block_may_break_current_loop(block))
            }
            Statement::WaitForStatement { inner, .. } => {
                Self::statement_may_break_current_loop(inner)
            }
            // A break inside a nested loop belongs to that nested loop.
            Statement::ForEachLoop { .. }
            | Statement::CountLoop { .. }
            | Statement::WhileLoop { .. }
            | Statement::RepeatWhileLoop { .. }
            | Statement::RepeatUntilLoop { .. }
            | Statement::ForeverLoop { .. }
            | Statement::MainLoop { .. } => false,
            _ => false,
        }
    }

    fn infer_recorded_action_return_type(
        returns: &[RecordedReturn],
        implicit_completion: Option<&Type>,
    ) -> Type {
        returns
            .iter()
            .map(|record| record.return_type.clone())
            .chain(implicit_completion.cloned())
            .reduce(Self::join_return_types)
            .unwrap_or(Type::Nothing)
    }

    fn check_recorded_return_types(&mut self, returns: &[RecordedReturn], expected_type: &Type) {
        for record in returns {
            if !record.has_value && *expected_type != Type::Nothing {
                self.type_error(
                    "Function must return a value".to_string(),
                    Some(expected_type.clone()),
                    Some(Type::Nothing),
                    record.line,
                    record.column,
                );
            } else if record.has_value
                && !self.are_types_compatible(expected_type, &record.return_type)
            {
                self.type_error(
                    "Return statement has incorrect type".to_string(),
                    Some(expected_type.clone()),
                    Some(record.return_type.clone()),
                    record.line,
                    record.column,
                );
            }
        }
    }

    fn check_implicit_action_result(
        &mut self,
        actual_type: &Type,
        expected_type: &Type,
        line: usize,
        column: usize,
    ) {
        if (*actual_type == Type::Nothing && *expected_type != Type::Nothing)
            || !self.are_types_compatible(expected_type, actual_type)
        {
            self.type_error(
                "Action's implicit result has incorrect type".to_string(),
                Some(expected_type.clone()),
                Some(actual_type.clone()),
                line,
                column,
            );
        }
    }

    /// Infer an action's return type from its `return` statements (issue #569).
    ///
    /// WFL has no return-type annotation, so the type checker must derive it
    /// from the body. Collect the type of every reachable `return <expr>` and
    /// merge them: identical types collapse to that type, common collection
    /// structure is retained with joined inner types, and otherwise differing
    /// concrete types widen to `Any`. `Unknown` remains unknown so we never turn
    /// an un-inferrable body into a false positive at the call site. A body with
    /// no value-returning `return` yields `Nothing`, preserving void-action
    /// behavior. If execution can fall through after at least one value return,
    /// retain that fact as `Optional<T>` rather than claiming every call
    /// produces `T`.
    #[allow(dead_code)]
    fn infer_action_return_type(&mut self, body: &[Statement]) -> Type {
        let mut return_types = Vec::new();
        let must_return = self.collect_return_types(body, &mut return_types);

        let mut result: Option<Type> = None;
        for t in return_types {
            result = Some(match result {
                None => t,
                Some(existing) => Self::join_return_types(existing, t),
            });
        }
        let inferred = result.unwrap_or(Type::Nothing);
        if must_return || inferred == Type::Nothing {
            inferred
        } else {
            match inferred {
                Type::Optional(_) => inferred,
                other => Type::Optional(Box::new(other)),
            }
        }
    }

    /// Gather the inferred type of each `return <expr>` reachable in `body`,
    /// descending into conditionals and loops (mirrors `check_return_statements`
    /// traversal). Diagnostics produced while inferring are discarded: the body
    /// pass has already reported them, so this is purely for type collection.
    #[allow(dead_code)]
    fn collect_return_types(&mut self, statements: &[Statement], out: &mut Vec<Type>) -> bool {
        for statement in statements {
            let must_return = match statement {
                Statement::ReturnStatement { value, .. } => {
                    if let Some(expr) = value {
                        let errors_before = self.errors.len();
                        let t = self.infer_expression_type(expr);
                        self.errors.truncate(errors_before);
                        out.push(t);
                    } else {
                        out.push(Type::Nothing);
                    }
                    // A bare `return` unconditionally exits the action, so any
                    // sibling statements after it in this block are unreachable.
                    // Stop here: collecting their returns would let dead code
                    // widen a precise type (e.g. `Text`) to `Any` and mask a
                    // genuine mismatch at the call site.
                    true
                }
                Statement::ExitStatement { .. } => true,
                Statement::IfStatement {
                    then_block,
                    else_block,
                    ..
                } => {
                    let then_returns = self.collect_return_types(then_block, out);
                    let else_returns = else_block
                        .as_ref()
                        .is_some_and(|else_stmts| self.collect_return_types(else_stmts, out));
                    then_returns && else_returns
                }
                Statement::SingleLineIf {
                    then_stmt,
                    else_stmt,
                    ..
                } => {
                    let then_returns =
                        self.collect_return_types(std::slice::from_ref(then_stmt.as_ref()), out);
                    let else_returns = else_stmt.as_ref().is_some_and(|else_stmt| {
                        self.collect_return_types(std::slice::from_ref(else_stmt.as_ref()), out)
                    });
                    then_returns && else_returns
                }
                Statement::ForEachLoop { body, .. }
                | Statement::CountLoop { body, .. }
                | Statement::WhileLoop { body, .. }
                | Statement::RepeatWhileLoop { body, .. }
                | Statement::RepeatUntilLoop { body, .. }
                | Statement::ForeverLoop { body, .. }
                | Statement::MainLoop { body, .. } => {
                    let _ = self.collect_return_types(body, out);
                    false
                }
                // Actions commonly return from inside error handling — a `try:`
                // body, its `when error` clauses, `otherwise`, or `finally`.
                // Skipping these blocks inferred such actions as `Nothing` and
                // produced false "Cannot index into Nothing" diagnostics at the
                // call site (issue #560 residual).
                Statement::TryStatement {
                    body,
                    when_clauses,
                    otherwise_block,
                    finally_block,
                    ..
                } => {
                    let primary_start = out.len();
                    let body_must_return = self.collect_return_types(body, out);
                    let mut handlers_must_return = true;
                    for clause in when_clauses {
                        handlers_must_return &= self.collect_return_types(&clause.body, out);
                    }
                    let otherwise_must_return =
                        otherwise_block.as_ref().is_none_or(|otherwise_stmts| {
                            self.collect_return_types(otherwise_stmts, out)
                        });
                    // An unhandled error propagates out of the action rather
                    // than producing Nothing. Only normally-completing try
                    // paths contribute a fallthrough value.
                    let primary_must_return =
                        body_must_return && handlers_must_return && otherwise_must_return;
                    if let Some(finally_stmts) = finally_block {
                        let mut finally_returns = Vec::new();
                        let finally_must_return =
                            self.collect_return_types(finally_stmts, &mut finally_returns);
                        if finally_must_return {
                            // A definitely-returning finally overrides every
                            // success/error-path return from the primary try.
                            out.truncate(primary_start);
                            out.extend(finally_returns);
                            true
                        } else {
                            out.extend(finally_returns);
                            primary_must_return
                        }
                    } else {
                        primary_must_return
                    }
                }
                Statement::WaitForStatement { inner, .. } => {
                    self.collect_return_types(std::slice::from_ref(inner), out)
                }
                _ => false,
            };
            if must_return {
                return true;
            }
        }
        false
    }

    // `line`/`column` are the action's fallback location, threaded through the
    // recursive descent; each error site prefers the offending statement's own
    // position, so the parameters are only forwarded to recursive calls.
    #[allow(clippy::only_used_in_recursion)]
    #[allow(dead_code)]
    fn check_return_statements(
        &mut self,
        statements: &[Statement],
        expected_type: &Type,
        line: usize,
        column: usize,
    ) {
        for statement in statements {
            match statement {
                Statement::ReturnStatement {
                    value,
                    line,
                    column,
                } => {
                    if let Some(expr) = value {
                        // The body pass (check_statement_types) has already
                        // inferred every return expression and reported any
                        // diagnostics inside it; re-inferring here is only to
                        // learn the type for the compatibility check, so drop
                        // the duplicate expression diagnostics it produces.
                        let errors_before = self.errors.len();
                        let return_type = self.infer_expression_type(expr);
                        self.errors.truncate(errors_before);
                        if !self.are_types_compatible(expected_type, &return_type) {
                            self.type_error(
                                "Return statement has incorrect type".to_string(),
                                Some(expected_type.clone()),
                                Some(return_type),
                                *line,
                                *column,
                            );
                        }
                    } else if *expected_type != Type::Nothing {
                        self.type_error(
                            "Function must return a value".to_string(),
                            Some(expected_type.clone()),
                            Some(Type::Nothing),
                            *line,
                            *column,
                        );
                    }
                }
                Statement::IfStatement {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.check_return_statements(then_block, expected_type, line, column);
                    if let Some(else_stmts) = else_block {
                        self.check_return_statements(else_stmts, expected_type, line, column);
                    }
                }
                Statement::SingleLineIf {
                    then_stmt,
                    else_stmt,
                    ..
                } => {
                    self.check_return_statements(
                        &[*(*then_stmt).clone()],
                        expected_type,
                        line,
                        column,
                    );
                    if let Some(else_stmt) = else_stmt {
                        self.check_return_statements(
                            &[*(*else_stmt).clone()],
                            expected_type,
                            line,
                            column,
                        );
                    }
                }
                Statement::ForEachLoop { body, .. }
                | Statement::CountLoop { body, .. }
                | Statement::WhileLoop { body, .. }
                | Statement::RepeatWhileLoop { body, .. }
                | Statement::RepeatUntilLoop { body, .. }
                | Statement::ForeverLoop { body, .. }
                | Statement::MainLoop { body, .. } => {
                    self.check_return_statements(body, expected_type, line, column);
                }
                // Keep in sync with `collect_return_types`: returns inside
                // `try:` blocks and `wait for` wrappers count too.
                Statement::TryStatement {
                    body,
                    when_clauses,
                    otherwise_block,
                    finally_block,
                    ..
                } => {
                    self.check_return_statements(body, expected_type, line, column);
                    for clause in when_clauses {
                        self.check_return_statements(&clause.body, expected_type, line, column);
                    }
                    if let Some(otherwise_stmts) = otherwise_block {
                        self.check_return_statements(otherwise_stmts, expected_type, line, column);
                    }
                    if let Some(finally_stmts) = finally_block {
                        self.check_return_statements(finally_stmts, expected_type, line, column);
                    }
                }
                Statement::WaitForStatement { inner, .. } => {
                    self.check_return_statements(
                        std::slice::from_ref(inner),
                        expected_type,
                        line,
                        column,
                    );
                }
                _ => {}
            }
        }
    }

    fn type_error(
        &mut self,
        message: String,
        expected: Option<Type>,
        found: Option<Type>,
        line: usize,
        column: usize,
    ) {
        self.errors
            .push(TypeError::new(message, expected, found, line, column));
    }

    /// Recreate a value that the interpreter binds while executing a statement.
    ///
    /// The analyzer checks action/loop/handler bodies in temporary scopes and
    /// discards those scopes before the type-checker pass. Updating an existing
    /// symbol with `get_symbol_mut` therefore loses statement-produced locals in
    /// exactly the places where their types matter most. Bind into the current
    /// checker scope instead, matching the interpreter's local environment.
    fn bind_runtime_value(
        &mut self,
        name: &str,
        value_type: Type,
        mutable: bool,
        line: usize,
        column: usize,
    ) {
        if name.is_empty() {
            return;
        }
        self.analyzer.define_or_replace_symbol(Symbol {
            name: name.to_string(),
            kind: SymbolKind::Variable { mutable },
            symbol_type: Some(value_type),
            line,
            column,
        });
    }

    /// Whether an inferred type is acceptable as an HTTP header map. Header names
    /// must be text, and header values are what the interpreter accepts and
    /// stringifies — text, number, or boolean (see the `respond`/HTTP header
    /// handling); a concretely-typed non-text key or a value type the runtime
    /// rejects (e.g. `Map<Text, Binary>`) is flagged. `Unknown`/`Any`/`Error` —
    /// whether as the whole type or as the key/value type of a map the checker
    /// could not fully pin down (map literals often infer unknown key/value
    /// types) — are always accepted so a header set the checker cannot resolve is
    /// never falsely flagged.
    fn is_valid_header_map_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Unknown | Type::Any | Type::Error => true,
            Type::Map(key, value) => {
                let key_ok = matches!(**key, Type::Text | Type::Unknown | Type::Any | Type::Error);
                let value_ok = matches!(
                    **value,
                    Type::Text
                        | Type::Number
                        | Type::Boolean
                        | Type::Unknown
                        | Type::Any
                        | Type::Error
                );
                key_ok && value_ok
            }
            _ => false,
        }
    }

    /// Server response statements require the opaque pending Request produced
    /// by `wait for request`; an ordinary map has no response sender.
    fn is_pending_request_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::Custom(name) if name == "Request"
        ) || matches!(ty, Type::Unknown | Type::Any | Type::Error)
    }

    /// `execute file ... with <request>` accepts either a live Request or a
    /// structurally complete object. Static Map types do not retain field
    /// shape, so map-shaped values must defer to the runtime field validator.
    fn is_execute_file_request_type(&self, ty: &Type) -> bool {
        self.is_pending_request_type(ty) || matches!(ty, Type::Map(_, _))
    }

    fn is_process_arguments_type(&self, ty: &Type) -> bool {
        matches!(ty, Type::Text | Type::List(_)) || self.is_gradual_type(ty)
    }

    /// WebSocket send targets are runtime objects whose text `id` field names
    /// the connection. Handler bindings are `Map<Text, Text>` for lifecycle
    /// events and `Map<Text, Any>` for message events; gradual key/value types
    /// stay deferred to the runtime shape check.
    fn is_websocket_connection_target_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Map(key_type, value_type) => {
                (matches!(key_type.as_ref(), Type::Text) || self.is_gradual_type(key_type))
                    && (matches!(value_type.as_ref(), Type::Text)
                        || self.is_gradual_type(value_type))
            }
            _ => self.is_gradual_type(ty),
        }
    }

    fn check_websocket_message_type(&mut self, ty: Type, line: usize, column: usize) {
        if !matches!(&ty, Type::Text | Type::Number | Type::Boolean) && !self.is_gradual_type(&ty) {
            self.type_error(
                "WebSocket message must be text, a number, or a boolean".to_string(),
                None,
                Some(ty),
                line,
                column,
            );
        }
    }

    fn temporal_kind(ty: &Type) -> Option<&'static str> {
        match ty {
            Type::Date => Some("date"),
            Type::Time => Some("time"),
            Type::DateTime => Some("datetime"),
            Type::Custom(name) if name.eq_ignore_ascii_case("date") => Some("date"),
            Type::Custom(name) if name.eq_ignore_ascii_case("time") => Some("time"),
            Type::Custom(name) if name.eq_ignore_ascii_case("datetime") => Some("datetime"),
            _ => None,
        }
    }

    fn custom_temporal_is_unambiguous(&self, ty: &Type) -> bool {
        let Type::Custom(name) = ty else {
            return true;
        };
        if Self::temporal_kind(ty).is_none() {
            return false;
        }
        !self
            .analyzer
            .get_containers()
            .keys()
            .any(|container_name| container_name == name)
    }

    fn is_unambiguous_temporal_type(&self, ty: &Type) -> bool {
        Self::temporal_kind(ty).is_some() && self.custom_temporal_is_unambiguous(ty)
    }

    fn are_same_temporal_type(&self, left: &Type, right: &Type) -> bool {
        Self::temporal_kind(left) == Self::temporal_kind(right)
            && Self::temporal_kind(left).is_some()
            && self.custom_temporal_is_unambiguous(left)
            && self.custom_temporal_is_unambiguous(right)
    }

    /// Whether a type can name a closeable resource: a file handle
    /// (`Custom("File")`), a stream handle (`Custom("HttpStream")` outbound or
    /// `Custom("ResponseStream")` server-side), or a statically-unresolved value.
    /// These are the only things the runtime can close, so an ordinary map,
    /// another custom type (`Database`/`Request`), or a scalar (`close 5`) is
    /// rejected — keeping real mistakes as static errors rather than runtime-only.
    fn is_closeable_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Custom(name) => {
                name == "File" || name == "HttpStream" || name == "ResponseStream"
            }
            Type::Text | Type::Unknown | Type::Any | Type::Error => true,
            _ => false,
        }
    }

    /// The operand of `wait for next chunk|line from <source>` must be an outbound
    /// stream handle (`stream response as ...` binds `HttpStream`). Unknown/Any/
    /// Error pass for gradual typing; a concrete non-stream type is rejected.
    fn is_http_stream_source_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Custom(name) => name == "HttpStream",
            Type::Unknown | Type::Any | Type::Error => true,
            _ => false,
        }
    }

    /// Concrete fields stored in runtime stream-handle objects. Unknown fields
    /// remain gradual because a historical custom annotation can also carry
    /// these names, but documented literal fields retain their real type.
    fn stream_field_type(stream_name: &str, field: &str) -> Option<Type> {
        match (stream_name, field) {
            ("HttpStream", "status") | ("ResponseStream", "status") => Some(Type::Number),
            ("HttpStream", "ok") => Some(Type::Boolean),
            ("HttpStream", "headers") => {
                Some(Type::Map(Box::new(Type::Text), Box::new(Type::Text)))
            }
            ("HttpStream", "_stream") | ("ResponseStream", "_server_stream") => Some(Type::Text),
            _ => None,
        }
    }

    /// The `<target>` of `write line|chunk` / `flush` must be a server response
    /// stream handle (`start streaming response as ...` binds `ResponseStream`).
    /// Unknown/Any/Error pass for gradual typing.
    fn is_response_stream_target_type(&self, ty: &Type) -> bool {
        match ty {
            Type::Custom(name) => name == "ResponseStream",
            Type::Unknown | Type::Any | Type::Error => true,
            _ => false,
        }
    }

    /// A type that inference has not pinned down (gradual typing): it may turn out
    /// to be anything at runtime, so a static check must stay lenient rather than
    /// reject it.
    fn is_gradual_type(&self, ty: &Type) -> bool {
        matches!(ty, Type::Unknown | Type::Any | Type::Error)
    }

    fn is_response_content_type(ty: &Type) -> bool {
        match ty {
            Type::Optional(inner) => Self::is_response_content_type(inner),
            Type::Text
            | Type::Binary
            | Type::Number
            | Type::Boolean
            | Type::Nothing
            | Type::Unknown
            | Type::Any
            | Type::Error => true,
            _ => false,
        }
    }

    /// The value types `write line|chunk` can send to a response stream — the
    /// runtime stringifies numbers/booleans and sends text/binary as-is, and
    /// rejects everything else (Map/List/Nothing/...). Gradual types pass.
    fn is_streamable_payload(&self, ty: &Type) -> bool {
        matches!(ty, Type::Text | Type::Number | Type::Boolean | Type::Binary)
            || self.is_gradual_type(ty)
    }

    /// Emit a type error if `ty` is a concrete value the runtime would reject as a
    /// response-stream payload (a Map/List/Nothing/... reaches `write` only to fail
    /// at runtime otherwise).
    fn check_streamable_payload(&mut self, ty: &Type, line: usize, column: usize) {
        if !self.is_streamable_payload(ty) {
            self.type_error(
                "`write line|chunk` can only send text, binary, a number, or a boolean \
                 to a response stream"
                    .to_string(),
                Some(Type::Text),
                Some(ty.clone()),
                line,
                column,
            );
        }
    }

    /// Whether a bare name is known to the typechecker/analyzer scopes (or is a
    /// builtin / action parameter / loop counter). Used when validating the
    /// concrete `write line|chunk` branch the runtime will select — the analyzer
    /// may have stayed silent on a one-sided undefined lead because the other
    /// reading was defined (issue #642).
    fn name_is_defined_for_write(&self, name: &str) -> bool {
        if self.analyzer.name_is_defined_for_write(name) {
            return true;
        }

        self.current_container_property_type(name).is_some()
    }

    /// Resolve a direct or inherited property against the typechecker's live
    /// container context. The analyzer has restored its own container context
    /// by the time method bodies are typechecked, so both definedness and type
    /// inference must use this view.
    fn current_container_property_type(&self, name: &str) -> Option<Type> {
        // Analyzer has already completed its container walk and restored its
        // own `current_container` by the time TypeChecker revisits method
        // bodies. Use TypeChecker's live container context here so direct and
        // inherited properties remain defined on the selected write branch.
        let mut container_name = self.current_container.as_deref();
        let mut visited = HashSet::new();
        while let Some(container_key) = container_name {
            if !visited.insert(container_key) {
                break;
            }
            let Some(container) = self.analyzer.get_container(container_key) else {
                break;
            };
            let property = match self.current_method_is_static {
                Some(true) => container.static_properties.get(name),
                Some(false) => container.properties.get(name),
                None => container
                    .properties
                    .get(name)
                    .or_else(|| container.static_properties.get(name)),
            };
            if let Some(property) = property {
                return Some(property.property_type.clone());
            }
            container_name = container.extends.as_deref();
        }

        None
    }

    /// Capture the true outer lexical binding for every property visible to
    /// the active method. Method parameters and locals receive different
    /// binding keys, and keep those keys when referenced through nested
    /// try/loop scopes; a same-named global retains the captured key.
    fn snapshot_current_method_outer_property_bindings(
        &self,
    ) -> HashMap<String, Option<SymbolBindingKey>> {
        let mut result = HashMap::new();
        let mut container_name = self.current_container.as_deref();
        let mut visited = HashSet::new();
        while let Some(container_key) = container_name {
            if !visited.insert(container_key) {
                break;
            }
            let Some(container) = self.analyzer.get_container(container_key) else {
                break;
            };
            let properties = if self.current_method_is_static == Some(true) {
                &container.static_properties
            } else {
                &container.properties
            };
            for name in properties.keys() {
                result
                    .entry(name.clone())
                    .or_insert_with(|| self.analyzer.get_symbol_binding_key(name));
            }
            container_name = container.extends.as_deref();
        }
        result
    }

    /// True when the nearest lexical binding is owned by the active method
    /// (a parameter, body local, or nested implicit binding), rather than the
    /// same-named lexical binding that existed outside the method.
    fn method_lexical_binding_shadows_property(&self, name: &str) -> bool {
        let Some(outer_bindings) = &self.current_method_outer_property_bindings else {
            return self.analyzer.get_local_symbol(name).is_some();
        };
        let Some(outer_binding) = outer_bindings.get(name) else {
            return self.analyzer.get_local_symbol(name).is_some();
        };
        self.analyzer.get_symbol_binding_key(name).as_ref() != outer_binding.as_ref()
    }

    /// Resolve a legacy bare mutation target with the same precedence as the
    /// runtime environment: a method-local binding shadows a current container
    /// property, which in turn shadows an outer lexical binding.
    fn resolve_bare_mutation_target_type(&self, name: &str) -> (Option<Type>, bool) {
        if self.method_lexical_binding_shadows_property(name)
            && let Some(symbol) = self.analyzer.get_symbol(name)
        {
            return (symbol.symbol_type.clone(), false);
        }
        if let Some(property_type) = self.current_container_property_type(name) {
            return (Some(property_type), true);
        }
        (
            self.analyzer
                .get_symbol(name)
                .and_then(|symbol| symbol.symbol_type.clone()),
            false,
        )
    }

    fn container_static_property_type(
        &self,
        container_name: &str,
        property_name: &str,
    ) -> Option<Type> {
        let mut current = Some(container_name);
        let mut visited = HashSet::new();
        while let Some(name) = current {
            if !visited.insert(name) {
                return None;
            }
            let container = self.analyzer.get_container(name)?;
            if let Some(property) = container.static_properties.get(property_name) {
                return Some(property.property_type.clone());
            }
            current = container.extends.as_deref();
        }
        None
    }

    fn container_static_method(
        &self,
        container_name: &str,
        method_name: &str,
    ) -> Option<crate::analyzer::MethodInfo> {
        let mut current = Some(container_name);
        let mut visited = HashSet::new();
        while let Some(name) = current {
            if !visited.insert(name) {
                return None;
            }
            let container = self.analyzer.get_container(name)?;
            if let Some(method) = container.static_methods.get(method_name) {
                return Some(method.clone());
            }
            current = container.extends.as_deref();
        }
        None
    }

    fn container_property_type(&self, container_name: &str, property_name: &str) -> Option<Type> {
        let mut current = Some(container_name);
        let mut visited = HashSet::new();
        while let Some(name) = current {
            if !visited.insert(name) {
                return None;
            }
            let container = self.analyzer.get_container(name)?;
            if let Some(property) = container.properties.get(property_name) {
                return Some(property.property_type.clone());
            }
            current = container.extends.as_deref();
        }
        None
    }

    /// Resolve a dot-property from an already-inferred receiver. The boolean
    /// identifies registry-backed instance/static properties (as opposed to a
    /// static method, map field, or gradual value), allowing mutation sites to
    /// preserve declared property contracts without evaluating the receiver a
    /// second time.
    fn infer_property_access_type(
        &mut self,
        object_type: Type,
        property: &str,
        line: usize,
        column: usize,
    ) -> (Type, bool) {
        match object_type {
            Type::Container(container_name) => {
                if let Some(property_type) =
                    self.container_static_property_type(&container_name, property)
                {
                    (property_type, true)
                } else if let Some(method_info) =
                    self.container_static_method(&container_name, property)
                {
                    (
                        Type::Function {
                            parameters: method_info
                                .parameters
                                .iter()
                                .map(|parameter| {
                                    parameter.param_type.clone().unwrap_or(Type::Unknown)
                                })
                                .collect(),
                            return_type: Box::new(method_info.return_type),
                        },
                        false,
                    )
                } else {
                    self.type_error(
                        format!(
                            "Static property '{property}' not found in container '{container_name}'"
                        ),
                        None,
                        None,
                        line,
                        column,
                    );
                    (Type::Error, false)
                }
            }
            Type::ContainerInstance(container_name) => {
                if self.analyzer.get_container(&container_name).is_none() {
                    self.type_error(
                        format!("Container '{container_name}' not found"),
                        None,
                        None,
                        line,
                        column,
                    );
                    return (Type::Error, false);
                }
                if let Some(property_type) = self.container_property_type(&container_name, property)
                {
                    (property_type, true)
                } else {
                    self.type_error(
                        format!("Property '{property}' not found in container '{container_name}'"),
                        None,
                        None,
                        line,
                        column,
                    );
                    (Type::Error, false)
                }
            }
            // Objects/maps support property access at runtime
            // (e.g. `response.status` on an HTTP response object).
            Type::Map(_, value_type) => (*value_type, false),
            Type::Unknown => (Type::Unknown, false),
            Type::Any => (Type::Any, false),
            Type::Error => (Type::Error, false),
            // Stream handles expose documented dot fields.
            Type::Custom(ref name) if name == "HttpStream" || name == "ResponseStream" => (
                Self::stream_field_type(name, property).unwrap_or(Type::Unknown),
                false,
            ),
            other => {
                self.type_error(
                    format!("Cannot access property '{property}' on non-container type {other}"),
                    Some(Type::ContainerInstance("Unknown".to_string())),
                    Some(other),
                    line,
                    column,
                );
                (Type::Error, false)
            }
        }
    }

    /// Walk an expression and report every undefined bare name. Used for the
    /// selected (or every viable gradual) `write line|chunk` branch so a missing
    /// classic `line <ident>` lead is not accepted just because the stream lead
    /// alone exists (and vice versa).
    fn check_expression_names_defined(&mut self, expression: &Expression) {
        match expression {
            Expression::Literal(Literal::List(items), ..) => {
                for item in items {
                    self.check_expression_names_defined(item);
                }
            }
            Expression::Literal(_, _, _)
            | Expression::StaticMemberAccess { .. }
            | Expression::CurrentTimeMilliseconds { .. }
            | Expression::CurrentTimeFormatted { .. } => {}
            Expression::Variable(name, l, c) => {
                if !self.name_is_defined_for_write(name) {
                    self.type_error(
                        format!("Variable '{name}' is not defined"),
                        None,
                        None,
                        *l,
                        *c,
                    );
                }
            }
            Expression::BinaryOperation { left, right, .. }
            | Expression::Concatenation { left, right, .. }
            | Expression::PatternMatch {
                text: left,
                pattern: right,
                ..
            }
            | Expression::PatternFind {
                text: left,
                pattern: right,
                ..
            }
            | Expression::PatternSplit {
                text: left,
                pattern: right,
                ..
            }
            | Expression::StringSplit {
                text: left,
                delimiter: right,
                ..
            } => {
                self.check_expression_names_defined(left);
                self.check_expression_names_defined(right);
            }
            Expression::UnaryOperation {
                expression: inner, ..
            }
            | Expression::AwaitExpression {
                expression: inner, ..
            }
            | Expression::FileExists { path: inner, .. }
            | Expression::DirectoryExists { path: inner, .. }
            | Expression::ListFiles { path: inner, .. }
            | Expression::ReadContent {
                file_handle: inner, ..
            }
            | Expression::ReadBinaryContent {
                file_handle: inner, ..
            }
            | Expression::FileSizeOf {
                file_handle: inner, ..
            }
            | Expression::ProcessRunning {
                process_id: inner, ..
            } => {
                self.check_expression_names_defined(inner);
            }
            Expression::IndexAccess {
                collection, index, ..
            } => {
                self.check_expression_names_defined(collection);
                self.check_expression_names_defined(index);
            }
            Expression::PropertyAccess { object, .. } | Expression::MemberAccess { object, .. } => {
                self.check_expression_names_defined(object);
            }
            Expression::MethodCall {
                object, arguments, ..
            } => {
                self.check_expression_names_defined(object);
                for arg in arguments {
                    self.check_expression_names_defined(&arg.value);
                }
            }
            Expression::FunctionCall {
                function,
                arguments,
                ..
            } => {
                self.check_expression_names_defined(function);
                for arg in arguments {
                    self.check_expression_names_defined(&arg.value);
                }
            }
            Expression::ActionCall { arguments, .. } => {
                for arg in arguments {
                    self.check_expression_names_defined(&arg.value);
                }
            }
            Expression::PatternReplace {
                text,
                pattern,
                replacement,
                ..
            } => {
                self.check_expression_names_defined(text);
                self.check_expression_names_defined(pattern);
                self.check_expression_names_defined(replacement);
            }
            Expression::HeaderAccess { request, .. } => {
                self.check_expression_names_defined(request);
            }
            Expression::ReadBinaryN {
                file_handle, count, ..
            } => {
                self.check_expression_names_defined(file_handle);
                self.check_expression_names_defined(count);
            }
            Expression::ListFilesRecursive {
                path, extensions, ..
            } => {
                self.check_expression_names_defined(path);
                if let Some(extensions) = extensions {
                    for extension in extensions {
                        self.check_expression_names_defined(extension);
                    }
                }
            }
            Expression::ListFilesFiltered {
                path, extensions, ..
            } => {
                self.check_expression_names_defined(path);
                for extension in extensions {
                    self.check_expression_names_defined(extension);
                }
            }
            Expression::DatabaseQuery {
                db,
                sql,
                parameters,
                ..
            } => {
                self.check_expression_names_defined(db);
                self.check_expression_names_defined(sql);
                if let Some(parameters) = parameters {
                    self.check_expression_names_defined(parameters);
                }
            }
        }
    }

    /// Builtin `Custom` contracts describe runtime-branded values (Date,
    /// Database, Request, and so on), not user containers that happen to use
    /// the same name. User action annotations retain their historical
    /// container-name semantics through `are_types_compatible`.
    fn are_builtin_types_compatible(&self, target_type: &Type, source_type: &Type) -> bool {
        if source_type == &Type::Nothing
            && !matches!(
                target_type,
                Type::Any | Type::Unknown | Type::Nothing | Type::Optional(_)
            )
        {
            return false;
        }
        if matches!(
            (target_type, source_type),
            (Type::Custom(_), Type::ContainerInstance(_))
        ) {
            return false;
        }
        if Self::temporal_kind(target_type) == Self::temporal_kind(source_type)
            && matches!(target_type, Type::Date | Type::Time | Type::DateTime)
            && self.custom_temporal_is_unambiguous(source_type)
        {
            return true;
        }
        self.are_types_compatible(target_type, source_type)
    }

    /// Container property annotations are persistent runtime invariants, not
    /// one-shot flow hints. Unlike an ordinary mutable local, a property is
    /// read later from the container registry using its declared type, so
    /// accepting an `Any`/`Unknown` or incompatible replacement would leave
    /// those later reads unsafely precise.
    fn are_declared_property_types_compatible(
        &self,
        target_type: &Type,
        source_type: &Type,
    ) -> bool {
        if source_type == &Type::Error {
            return true;
        }
        match (target_type, source_type) {
            (a, b) if a == b => true,
            (Type::Any | Type::Unknown, _) => true,
            (_, Type::Any | Type::Unknown) => false,
            (Type::Optional(target), Type::Optional(source)) => {
                self.are_declared_property_types_compatible(target, source)
            }
            (Type::Optional(_), Type::Nothing) => true,
            (Type::Optional(target), source) => {
                self.are_declared_property_types_compatible(target, source)
            }
            (_, Type::Optional(_)) | (_, Type::Nothing) => false,
            (Type::List(target), Type::List(source)) => {
                self.are_declared_property_types_compatible(target, source)
            }
            (Type::Map(target_key, target_value), Type::Map(source_key, source_value)) => {
                self.are_declared_property_types_compatible(target_key, source_key)
                    && self.are_declared_property_types_compatible(target_value, source_value)
            }
            (Type::Async(target), Type::Async(source)) => {
                self.are_declared_property_types_compatible(target, source)
            }
            _ => self.are_types_compatible(target_type, source_type),
        }
    }

    /// Expression-aware form of the persistent property contract. A fresh
    /// empty list literal is safe for any declared list element type: it has no
    /// elements that could violate the contract, while a shared
    /// `List<Unknown>` binding remains unsafe because another alias may later
    /// insert an incompatible value.
    fn are_declared_property_values_compatible(
        &self,
        target_type: &Type,
        source_type: &Type,
        source: &Expression,
    ) -> bool {
        self.are_declared_property_types_compatible(target_type, source_type)
            || Self::is_fresh_empty_list_shape_compatible(target_type, source)
    }

    fn is_fresh_empty_list_shape_compatible(target_type: &Type, source: &Expression) -> bool {
        match (target_type, source) {
            (Type::Optional(inner), source) => {
                Self::is_fresh_empty_list_shape_compatible(inner, source)
            }
            (Type::List(element_type), Expression::Literal(Literal::List(elements), ..)) => {
                elements.is_empty()
                    || elements.iter().all(|element| {
                        Self::is_fresh_empty_list_shape_compatible(element_type, element)
                    })
            }
            _ => false,
        }
    }

    fn are_types_compatible(&self, target_type: &Type, source_type: &Type) -> bool {
        #[allow(clippy::only_used_in_recursion)]
        let _self = self; // Suppress the warning for self parameter
        match (target_type, source_type) {
            (a, b) if a == b => true,

            (Type::Unknown, _) => true,
            (_, Type::Unknown) => true, // Unknown can be assigned to any type

            (Type::Any, _) => true, // Any can accept any type
            (_, Type::Any) => true, // Any can be assigned to any type

            // Optional return inference is deliberately stricter than the
            // general gradual `Any` type: a value that may fall through as
            // Nothing cannot satisfy a consumer requiring a definite value.
            (Type::Optional(target), Type::Optional(source)) => {
                self.are_types_compatible(target, source)
            }
            (Type::Optional(_), Type::Nothing) => true,
            (Type::Optional(target), source) => self.are_types_compatible(target, source),
            (_, Type::Optional(_)) => false,

            (_, Type::Nothing) => true,

            (_, Type::Error) => true,

            (inner, Type::Async(async_type)) => self.are_types_compatible(inner, async_type),

            // Lowercase temporal annotations use dedicated runtime-value
            // types. Historical named annotations remain Custom(...) and
            // accept those values, but not conversely: a runtime-branded
            // temporal contract must never accept a same-named container.
            (Type::Custom(name), Type::Date) if name.eq_ignore_ascii_case("date") => true,
            (Type::Custom(name), Type::Time) if name.eq_ignore_ascii_case("time") => true,
            (Type::Custom(name), Type::DateTime) if name.eq_ignore_ascii_case("datetime") => true,

            (Type::List(a), Type::List(b)) => self.are_types_compatible(a, b),
            (Type::Map(a_key, a_val), Type::Map(b_key, b_val)) => {
                self.are_types_compatible(a_key, b_key) && self.are_types_compatible(a_val, b_val)
            }

            (
                Type::Function {
                    parameters: a_params,
                    return_type: a_ret,
                },
                Type::Function {
                    parameters: b_params,
                    return_type: b_ret,
                },
            ) => {
                if a_params.len() != b_params.len() {
                    return false;
                }

                for (a, b) in a_params.iter().zip(b_params.iter()) {
                    if !self.are_types_compatible(a, b) {
                        return false;
                    }
                }

                self.are_types_compatible(a_ret, b_ret)
            }

            // Container instances satisfy targets typed as their own
            // container (`value as Dog` parses as Custom("Dog")) or as any
            // ancestor via the `extends` chain; same for descendant custom
            // types. Mirrors `Analyzer::is_type_compatible`.
            (Type::Custom(target), Type::ContainerInstance(source))
            | (Type::ContainerInstance(target), Type::ContainerInstance(source))
            | (Type::Custom(target), Type::Custom(source)) => {
                self.analyzer.container_is_or_extends(source, target)
            }

            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_wfl_with_positions;
    use crate::parser::Parser;
    use crate::parser::ast::{Argument, Expression, Literal, Parameter, Program, Statement, Type};
    use std::sync::Arc;

    fn typecheck_symbol_type(source: &str, name: &str) -> Type {
        let tokens = lex_wfl_with_positions(source);
        let mut parser = Parser::new(&tokens);
        let program = parser.parse().expect("program should parse");
        let mut checker = TypeChecker::new();
        checker
            .check_types(&program)
            .unwrap_or_else(|error| panic!("program should type-check: {error:?}"));
        checker
            .analyzer
            .get_symbol(name)
            .and_then(|symbol| symbol.symbol_type.clone())
            .unwrap_or_else(|| panic!("symbol {name:?} should have a type"))
    }

    fn list_of(element: Type) -> Type {
        Type::List(Box::new(element))
    }

    #[test]
    fn clear_through_may_alias_preserves_unselected_descendant_type_effects() {
        let leaf_type = typecheck_symbol_type(
            r#"
store leaf_b as [1]
store leaf_c as [1]
store b as [leaf_b]
store c as [leaf_c]
store selected as b
store choose_c as yes
check if choose_c:
    change selected to c
end check
clear selected
push with b[0] and "text"
"#,
            "leaf_b",
        );
        assert_eq!(leaf_type, list_of(Type::Any));
    }

    #[test]
    fn known_action_map_argument_escapes_nested_list_alias_type() {
        let leaf_type = typecheck_symbol_type(
            r#"
define action called append_text with parameters wrapper:
    push with wrapper["items"] and "text"
end action
store leaf as [1]
create map wrapper:
    "items" is leaf
end map
call append_text with wrapper
"#,
            "leaf",
        );
        assert_eq!(leaf_type, list_of(Type::Any));
    }

    #[test]
    fn known_action_nested_list_argument_escapes_every_alias_depth_type() {
        let leaf_type = typecheck_symbol_type(
            r#"
define action called append_text with parameters wrapper:
    push with wrapper[0] and "text"
end action
store leaf as [1]
store wrapper as [leaf]
call append_text with wrapper
"#,
            "leaf",
        );
        assert_eq!(leaf_type, list_of(Type::Any));
    }

    #[test]
    fn returned_map_carries_captured_nested_list_type_effect() {
        let leaf_type = typecheck_symbol_type(
            r#"
store leaf as [1]
define action called expose:
    create map result:
        "items" is leaf
    end map
    return result
end action
store exposed as call expose
push with exposed["items"] and "text"
"#,
            "leaf",
        );
        assert_eq!(leaf_type, list_of(Type::Any));
    }

    #[test]
    fn projection_reassignment_rebases_descendant_alias_type_effects() {
        let leaf_type = typecheck_symbol_type(
            r#"
store leaf as [1]
store outer as [0 and [leaf]]
change outer to pop of outer
push with outer[0] and "text"
"#,
            "leaf",
        );
        assert_eq!(leaf_type, list_of(Type::Any));
    }

    #[test]
    fn gradual_add_records_inserted_list_alias_type_effect() {
        let leaf_type = typecheck_symbol_type(
            r#"
store leaf as [1]
store target as parse_json of "[]"
add leaf to target
push with target[0] and "text"
"#,
            "leaf",
        );
        assert_eq!(leaf_type, list_of(Type::Any));
    }

    #[test]
    fn implicit_shared_return_through_try_carries_captured_type_effect() {
        let leaf_type = typecheck_symbol_type(
            r#"
store leaf as [1]
define action called expose:
    try:
        leaf
    when error:
        leaf
    end try
end action
store exposed as call expose
push with exposed and "text"
"#,
            "leaf",
        );
        assert_eq!(leaf_type, list_of(Type::Any));
    }

    #[test]
    fn optional_joins_preserve_the_known_nothing_path() {
        let optional_text = Type::Optional(Box::new(Type::Text));
        for (other, expected) in [
            (Type::Text, optional_text.clone()),
            (
                Type::Optional(Box::new(Type::Number)),
                Type::Optional(Box::new(Type::Any)),
            ),
            (Type::Unknown, Type::Optional(Box::new(Type::Unknown))),
            (Type::Any, Type::Optional(Box::new(Type::Any))),
            (Type::Nothing, optional_text.clone()),
        ] {
            assert_eq!(
                TypeChecker::join_inferred_types(optional_text.clone(), other),
                expected
            );
        }

        assert_eq!(
            TypeChecker::join_inferred_types(
                Type::List(Box::new(optional_text.clone())),
                Type::List(Box::new(Type::Text)),
            ),
            Type::List(Box::new(optional_text)),
        );
        assert_eq!(
            TypeChecker::join_inferred_types(Type::Nothing, Type::Text),
            Type::Optional(Box::new(Type::Text)),
        );
    }

    #[test]
    fn test_header_map_type_requires_text_keys() {
        // HTTP header names must be text. The header-map validity check (shared by
        // outbound HTTP, streaming-response, and `respond` header clauses) must
        // reject a map with a concretely-typed non-text key, while accepting
        // text-keyed maps and any map whose key the checker could not pin down.
        let tc = TypeChecker::new();

        // Accepted: text keys with a value type the runtime stringifies
        // (text/number/bool), or an unresolved/loose key or value type.
        for ok in [
            Type::Map(Box::new(Type::Text), Box::new(Type::Text)),
            Type::Map(Box::new(Type::Text), Box::new(Type::Number)),
            Type::Map(Box::new(Type::Text), Box::new(Type::Boolean)),
            Type::Map(Box::new(Type::Text), Box::new(Type::Any)),
            Type::Map(Box::new(Type::Text), Box::new(Type::Unknown)),
            Type::Map(Box::new(Type::Unknown), Box::new(Type::Unknown)),
            Type::Map(Box::new(Type::Any), Box::new(Type::Text)),
            Type::Unknown,
            Type::Any,
        ] {
            assert!(
                tc.is_valid_header_map_type(&ok),
                "expected {ok:?} to be a valid header map type"
            );
        }

        // Rejected: a concrete non-text key, a value type the runtime rejects
        // (e.g. Binary or a nested list), or a non-map entirely.
        for bad in [
            Type::Map(Box::new(Type::Number), Box::new(Type::Text)),
            Type::Map(Box::new(Type::Boolean), Box::new(Type::Any)),
            Type::Map(Box::new(Type::Text), Box::new(Type::Binary)),
            Type::Map(
                Box::new(Type::Text),
                Box::new(Type::List(Box::new(Type::Text))),
            ),
            Type::Number,
            Type::Text,
        ] {
            assert!(
                !tc.is_valid_header_map_type(&bad),
                "expected {bad:?} to be rejected as a header map type"
            );
        }
    }

    #[test]
    fn test_variable_declaration_type_inference() {
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

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(result.is_ok(), "Expected no type errors");
    }

    #[test]
    fn test_type_mismatch_in_assignment() {
        let program = Program {
            statements: vec![
                Statement::VariableDeclaration {
                    name: "x".to_string(),
                    value: Expression::Literal(Literal::Integer(10), 1, 1),
                    is_constant: false,
                    line: 1,
                    column: 1,
                },
                Statement::Assignment {
                    name: "x".to_string(),
                    value: Expression::Literal(Literal::String(Arc::from("hello")), 2, 1),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(result.is_err(), "Expected type error for mismatched types");

        let errors = result.err().unwrap().into_diagnostics();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("incompatible type"))
        );
    }

    #[test]
    fn test_respond_headers_must_be_a_map() {
        // A non-map value in the `headers` clause must be a type error, the same
        // way non-map request headers are rejected on `open url`.
        let program = Program {
            statements: vec![
                Statement::VariableDeclaration {
                    name: "req".to_string(),
                    value: Expression::Literal(Literal::String(Arc::from("stub")), 1, 1),
                    is_constant: false,
                    line: 1,
                    column: 1,
                },
                Statement::RespondStatement {
                    request: Expression::Variable("req".to_string(), 2, 1),
                    content: Expression::Literal(Literal::String(Arc::from("ok")), 2, 1),
                    status: None,
                    content_type: None,
                    headers: Some(Expression::Literal(Literal::Integer(42), 2, 1)),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(
            result.is_err(),
            "Expected a type error for non-map response headers"
        );
        let errors = result.err().unwrap().into_diagnostics();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Response headers must be a map")),
            "Expected the response-headers type error, got: {errors:?}"
        );
    }

    #[test]
    fn test_binary_operation_type_checking() {
        let program = Program {
            statements: vec![Statement::VariableDeclaration {
                name: "x".to_string(),
                is_constant: false,
                value: Expression::BinaryOperation {
                    left: Box::new(Expression::Literal(
                        Literal::String(Arc::from("hello")),
                        1,
                        5,
                    )),
                    operator: crate::parser::ast::Operator::Plus,
                    right: Box::new(Expression::Literal(
                        Literal::String(Arc::from("world")),
                        1,
                        10,
                    )),
                    line: 1,
                    column: 5,
                },
                line: 1,
                column: 1,
            }],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(
            result.is_ok(),
            "Expected no type errors for string concatenation with +"
        );

        let program = Program {
            statements: vec![Statement::VariableDeclaration {
                name: "x".to_string(),
                is_constant: false,
                value: Expression::BinaryOperation {
                    left: Box::new(Expression::Literal(Literal::Integer(10), 1, 5)),
                    operator: crate::parser::ast::Operator::Minus,
                    right: Box::new(Expression::Literal(
                        Literal::String(Arc::from("hello")),
                        1,
                        10,
                    )),
                    line: 1,
                    column: 5,
                },
                line: 1,
                column: 1,
            }],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(
            result.is_err(),
            "Expected type error for incompatible operation"
        );

        let errors = result.err().unwrap().into_diagnostics();
        assert!(errors.iter().any(|e| e.message.contains("Cannot perform")));
    }

    #[test]
    fn test_function_call_type_checking() {
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
                            value: Expression::Literal(Literal::Integer(123), 3, 7),
                        }],
                        line: 3,
                        column: 1,
                    },
                    line: 3,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(
            result.is_err(),
            "Expected type error for wrong argument type"
        );

        // The analyzer's of-form call validation now catches this first with
        // "Argument 'name' of action 'greet' expects Text, but got Number";
        // the typechecker's own path words it as "incorrect type".
        let errors = result.err().unwrap().into_diagnostics();
        assert!(errors.iter().any(|e| e.message.contains("incorrect type")
            || e.message.contains("expects Text, but got Number")));
    }

    #[test]
    fn test_conditional_type_checking() {
        let program = Program {
            statements: vec![Statement::IfStatement {
                condition: Expression::Literal(Literal::Integer(1), 1, 10),
                then_block: vec![],
                else_block: None,
                line: 1,
                column: 1,
            }],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(
            result.is_err(),
            "Expected type error for non-boolean condition"
        );

        let errors = result.err().unwrap().into_diagnostics();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Condition must be a boolean"))
        );
    }

    #[test]
    fn test_async_type_compatibility() {
        assert!(
            TypeChecker::new()
                .are_types_compatible(&Type::Number, &Type::Async(Box::new(Type::Number)))
        );

        assert!(
            !TypeChecker::new()
                .are_types_compatible(&Type::Text, &Type::Async(Box::new(Type::Number)))
        );
    }

    #[test]
    fn test_foreach_type_inference() {
        let program = Program {
            statements: vec![
                Statement::CreateListStatement {
                    name: "numbers".to_string(),
                    initial_values: vec![Expression::Literal(Literal::Integer(1), 1, 1)],
                    line: 1,
                    column: 1,
                },
                Statement::ForEachLoop {
                    item_name: "item".to_string(),
                    collection: Expression::Variable("numbers".to_string(), 2, 10),
                    reversed: false,
                    body: vec![Statement::ExpressionStatement {
                        expression: Expression::BinaryOperation {
                            left: Box::new(Expression::Variable("item".to_string(), 3, 5)),
                            operator: crate::parser::ast::Operator::Minus,
                            right: Box::new(Expression::Literal(
                                Literal::String(Arc::from("text")),
                                3,
                                12,
                            )),
                            line: 3,
                            column: 10,
                        },
                        line: 3,
                        column: 1,
                    }],
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(
            result.is_err(),
            "Expected type error for incompatible operation on loop variable"
        );

        let errors = result.err().unwrap().into_diagnostics();
        assert!(
            errors.iter().any(|e| e.message.contains("Cannot perform")),
            "Expected error about invalid operation, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_signal_handler_type_checking() {
        // Test case 1: Valid registration
        let valid_program = Program {
            statements: vec![
                Statement::ActionDefinition {
                    name: "handler".to_string(),
                    parameters: vec![],
                    body: vec![],
                    return_type: None,
                    line: 1,
                    column: 1,
                },
                Statement::RegisterSignalHandlerStatement {
                    signal_type: "SIGINT".to_string(),
                    handler_name: "handler".to_string(),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&valid_program);
        assert!(
            result.is_ok(),
            "Expected valid signal registration to pass type checking"
        );

        // Test case 2: Invalid signal name (using unhandleable signal)
        let invalid_signal = Program {
            statements: vec![
                Statement::ActionDefinition {
                    name: "handler".to_string(),
                    parameters: vec![],
                    body: vec![],
                    return_type: None,
                    line: 1,
                    column: 1,
                },
                Statement::RegisterSignalHandlerStatement {
                    signal_type: "SIGKILL".to_string(),
                    handler_name: "handler".to_string(),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&invalid_signal);
        assert!(
            result.is_err(),
            "Expected type error for unhandleable signal name (SIGKILL)"
        );
        let errors = result.err().unwrap().into_diagnostics();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Invalid signal type"))
        );

        // Test case 3: Undefined handler
        let undefined_handler = Program {
            statements: vec![Statement::RegisterSignalHandlerStatement {
                signal_type: "SIGINT".to_string(),
                handler_name: "unknown_handler".to_string(),
                line: 1,
                column: 1,
            }],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&undefined_handler);
        assert!(result.is_err(), "Expected type error for undefined handler");
        let errors = result.err().unwrap().into_diagnostics();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Undefined signal handler"))
        );

        // Test case 4: Handler is not a function
        let not_a_function = Program {
            statements: vec![
                Statement::VariableDeclaration {
                    name: "handler".to_string(),
                    value: Expression::Literal(Literal::Integer(1), 1, 1),
                    is_constant: false,
                    line: 1,
                    column: 1,
                },
                Statement::RegisterSignalHandlerStatement {
                    signal_type: "SIGINT".to_string(),
                    handler_name: "handler".to_string(),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&not_a_function);
        assert!(
            result.is_err(),
            "Expected type error when handler is not a function"
        );
        let errors = result.err().unwrap().into_diagnostics();
        assert!(errors.iter().any(|e| e.message.contains("not a function")));

        // Test case 5: Handler has too many parameters
        let too_many_params = Program {
            statements: vec![
                Statement::ActionDefinition {
                    name: "handler".to_string(),
                    parameters: vec![
                        Parameter {
                            name: "a".to_string(),
                            param_type: Some(Type::Text),
                            default_value: None,
                            line: 1,
                            column: 1,
                        },
                        Parameter {
                            name: "b".to_string(),
                            param_type: Some(Type::Text),
                            default_value: None,
                            line: 1,
                            column: 1,
                        },
                    ],
                    body: vec![],
                    return_type: None,
                    line: 1,
                    column: 1,
                },
                Statement::RegisterSignalHandlerStatement {
                    signal_type: "SIGINT".to_string(),
                    handler_name: "handler".to_string(),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&too_many_params);
        assert!(
            result.is_err(),
            "Expected type error when handler has too many parameters"
        );
        let errors = result.err().unwrap().into_diagnostics();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("must accept 0 or 1 arguments"))
        );

        // Test case 6: Handler has wrong parameter type
        let wrong_param_type = Program {
            statements: vec![
                Statement::ActionDefinition {
                    name: "handler".to_string(),
                    parameters: vec![Parameter {
                        name: "a".to_string(),
                        param_type: Some(Type::Text),
                        default_value: None,
                        line: 1,
                        column: 1,
                    }],
                    body: vec![],
                    return_type: None,
                    line: 1,
                    column: 1,
                },
                Statement::RegisterSignalHandlerStatement {
                    signal_type: "SIGINT".to_string(),
                    handler_name: "handler".to_string(),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&wrong_param_type);
        assert!(
            result.is_err(),
            "Expected type error when handler has wrong parameter type"
        );
        let errors = result.err().unwrap().into_diagnostics();
        assert!(errors.iter().any(|e| {
            e.message
                .contains("Signal handler parameter must be a Number")
        }));
    }

    #[test]
    fn test_web_server_statements_type_checking() {
        // Test valid web server statements
        let valid_program = Program {
            statements: vec![
                Statement::VariableDeclaration {
                    name: "my_server".to_string(),
                    value: Expression::Literal(Literal::String(Arc::from("server_1")), 1, 1),
                    is_constant: false,
                    line: 1,
                    column: 1,
                },
                Statement::WaitForRequestStatement {
                    server: Expression::Variable("my_server".to_string(), 2, 5),
                    request_name: "req".to_string(),
                    timeout: Some(Expression::Literal(Literal::Integer(5000), 2, 20)),
                    line: 2,
                    column: 1,
                },
                Statement::StopAcceptingConnectionsStatement {
                    server: Expression::Variable("my_server".to_string(), 3, 5),
                    line: 3,
                    column: 1,
                },
                Statement::CloseServerStatement {
                    server: Expression::Variable("my_server".to_string(), 4, 5),
                    line: 4,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&valid_program);
        assert!(
            result.is_ok(),
            "Expected valid web server statements to pass type checking"
        );

        // Test invalid server type (number instead of string)
        let invalid_server_program = Program {
            statements: vec![
                Statement::VariableDeclaration {
                    name: "invalid_server".to_string(),
                    value: Expression::Literal(Literal::Integer(123), 1, 1),
                    is_constant: false,
                    line: 1,
                    column: 1,
                },
                Statement::WaitForRequestStatement {
                    server: Expression::Variable("invalid_server".to_string(), 2, 5),
                    request_name: "req".to_string(),
                    timeout: None,
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&invalid_server_program);
        assert!(
            result.is_err(),
            "Expected type error for invalid server expression"
        );
        let errors = result.err().unwrap().into_diagnostics();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Server must be a text string"))
        );

        // Test invalid timeout type (string instead of number)
        let invalid_timeout_program = Program {
            statements: vec![
                Statement::VariableDeclaration {
                    name: "my_server".to_string(),
                    value: Expression::Literal(Literal::String(Arc::from("server_1")), 1, 1),
                    is_constant: false,
                    line: 1,
                    column: 1,
                },
                Statement::WaitForRequestStatement {
                    server: Expression::Variable("my_server".to_string(), 2, 5),
                    request_name: "req".to_string(),
                    timeout: Some(Expression::Literal(Literal::String(Arc::from("5s")), 2, 20)),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&invalid_timeout_program);
        assert!(
            result.is_err(),
            "Expected type error for invalid timeout expression"
        );
        let errors = result.err().unwrap().into_diagnostics();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Timeout must be a number"))
        );
    }

    #[test]
    fn test_pattern_definition_type_checking() {
        let valid_program = Program {
            statements: vec![Statement::PatternDefinition {
                name: "my_pattern".to_string(),
                pattern: PatternExpression::Literal("test".to_string()),
                line: 1,
                column: 1,
            }],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&valid_program);
        assert!(
            result.is_ok(),
            "Expected valid pattern to pass type checking"
        );

        // Test invalid list reference in pattern
        let invalid_list_ref_program = Program {
            statements: vec![
                Statement::VariableDeclaration {
                    name: "not_a_list".to_string(),
                    value: Expression::Literal(Literal::Integer(123), 1, 1),
                    is_constant: false,
                    line: 1,
                    column: 1,
                },
                Statement::PatternDefinition {
                    name: "my_pattern".to_string(),
                    pattern: PatternExpression::ListReference("not_a_list".to_string()),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&invalid_list_ref_program);
        assert!(
            result.is_err(),
            "Expected type error for invalid list reference in pattern"
        );
        let errors = result.err().unwrap().into_diagnostics();
        assert!(errors.iter().any(|e| e.message.contains("must be a List")));

        // Test valid List<Text> reference
        let valid_list_text_program = Program {
            statements: vec![
                Statement::CreateListStatement {
                    name: "text_list".to_string(),
                    initial_values: vec![Expression::Literal(
                        Literal::String(Arc::from("abc")),
                        1,
                        1,
                    )],
                    line: 1,
                    column: 1,
                },
                Statement::PatternDefinition {
                    name: "my_pattern".to_string(),
                    pattern: PatternExpression::ListReference("text_list".to_string()),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&valid_list_text_program);
        assert!(
            result.is_ok(),
            "Expected valid List<Text> reference to pass type checking"
        );

        // Test invalid List<Number> reference
        let invalid_list_number_program = Program {
            statements: vec![
                Statement::CreateListStatement {
                    name: "number_list".to_string(),
                    initial_values: vec![Expression::Literal(Literal::Integer(123), 1, 1)],
                    line: 1,
                    column: 1,
                },
                Statement::PatternDefinition {
                    name: "my_pattern".to_string(),
                    pattern: PatternExpression::ListReference("number_list".to_string()),
                    line: 2,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&invalid_list_number_program);
        assert!(
            result.is_err(),
            "Expected type error for List<Number> reference in pattern"
        );
        let errors = result.err().unwrap().into_diagnostics();
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("must contain Text"))
        );
    }

    /// Issue #569: an action's return type must be inferred from its body so a
    /// call result feeding a Text-required position (e.g. `open file at ...`)
    /// does not spuriously report "Expected Text but found Nothing".
    #[test]
    fn test_action_return_type_inferred_for_text_position() {
        let program = Program {
            statements: vec![
                Statement::ActionDefinition {
                    name: "h".to_string(),
                    parameters: vec![Parameter {
                        name: "name".to_string(),
                        param_type: None,
                        default_value: None,
                        line: 1,
                        column: 1,
                    }],
                    body: vec![Statement::ReturnStatement {
                        value: Some(Expression::Literal(
                            Literal::String(Arc::from("hello")),
                            2,
                            5,
                        )),
                        line: 2,
                        column: 5,
                    }],
                    return_type: None,
                    line: 1,
                    column: 1,
                },
                Statement::VariableDeclaration {
                    name: "c".to_string(),
                    value: Expression::ActionCall {
                        name: "h".to_string(),
                        arguments: vec![Argument {
                            name: None,
                            value: Expression::Literal(Literal::String(Arc::from("world")), 4, 1),
                        }],
                        line: 4,
                        column: 1,
                    },
                    is_constant: false,
                    line: 4,
                    column: 1,
                },
                Statement::OpenFileStatement {
                    path: Expression::Variable("c".to_string(), 5, 1),
                    variable_name: "f".to_string(),
                    mode: crate::parser::ast::FileOpenMode::Read,
                    line: 5,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(
            result.is_ok(),
            "Action returning Text should satisfy a Text-required position, got: {:?}",
            result.err()
        );
    }

    /// Issue #569 (converse): inference must not mask a genuine mismatch — an
    /// action that returns a Number used where Text is required must still error.
    #[test]
    fn test_action_return_type_inference_still_flags_real_mismatch() {
        let program = Program {
            statements: vec![
                Statement::ActionDefinition {
                    name: "g".to_string(),
                    parameters: vec![],
                    body: vec![Statement::ReturnStatement {
                        value: Some(Expression::Literal(Literal::Integer(42), 2, 5)),
                        line: 2,
                        column: 5,
                    }],
                    return_type: None,
                    line: 1,
                    column: 1,
                },
                Statement::VariableDeclaration {
                    name: "n".to_string(),
                    value: Expression::ActionCall {
                        name: "g".to_string(),
                        arguments: vec![],
                        line: 4,
                        column: 1,
                    },
                    is_constant: false,
                    line: 4,
                    column: 1,
                },
                Statement::OpenFileStatement {
                    path: Expression::Variable("n".to_string(), 5, 1),
                    variable_name: "f".to_string(),
                    mode: crate::parser::ast::FileOpenMode::Read,
                    line: 5,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(
            result.is_err(),
            "Action returning Number used as a file path must still be flagged"
        );
        let errors = result.err().unwrap().into_diagnostics();
        assert!(
            errors.iter().any(|e| e.found == Some(Type::Number)),
            "Mismatch should report the inferred Number type, got: {errors:?}"
        );
    }

    /// Issue #569 (review follow-up): a `return` inside a `repeat while` body
    /// must contribute to return-type inference, otherwise such actions still
    /// infer `Nothing`.
    #[test]
    fn test_action_return_type_inferred_from_repeat_while_body() {
        let program = Program {
            statements: vec![
                Statement::ActionDefinition {
                    name: "loop_ret".to_string(),
                    parameters: vec![],
                    body: vec![Statement::RepeatWhileLoop {
                        condition: Expression::Literal(Literal::Boolean(true), 2, 1),
                        body: vec![Statement::ReturnStatement {
                            value: Some(Expression::Literal(Literal::String(Arc::from("x")), 3, 5)),
                            line: 3,
                            column: 5,
                        }],
                        line: 2,
                        column: 1,
                    }],
                    return_type: None,
                    line: 1,
                    column: 1,
                },
                Statement::VariableDeclaration {
                    name: "r".to_string(),
                    value: Expression::ActionCall {
                        name: "loop_ret".to_string(),
                        arguments: vec![],
                        line: 6,
                        column: 1,
                    },
                    is_constant: false,
                    line: 6,
                    column: 1,
                },
                Statement::OpenFileStatement {
                    path: Expression::Variable("r".to_string(), 7, 1),
                    variable_name: "f".to_string(),
                    mode: crate::parser::ast::FileOpenMode::Read,
                    line: 7,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(
            result.is_ok(),
            "Text returned from a repeat-while body should satisfy a Text position, got: {:?}",
            result.err()
        );
    }

    /// Issue #569 (review follow-up): statements after an unconditional `return`
    /// are unreachable and must not contribute to inference. Here the reachable
    /// return is `Number`; a dead sibling returning `Text` must not widen the
    /// inferred type to `Any` and hide the genuine mismatch at the call site.
    #[test]
    fn test_unreachable_return_does_not_widen_inferred_type() {
        let program = Program {
            statements: vec![
                Statement::ActionDefinition {
                    name: "dead".to_string(),
                    parameters: vec![],
                    body: vec![
                        Statement::ReturnStatement {
                            value: Some(Expression::Literal(Literal::Integer(42), 2, 5)),
                            line: 2,
                            column: 5,
                        },
                        // Unreachable: must be ignored by inference.
                        Statement::ReturnStatement {
                            value: Some(Expression::Literal(
                                Literal::String(Arc::from("text")),
                                3,
                                5,
                            )),
                            line: 3,
                            column: 5,
                        },
                    ],
                    return_type: None,
                    line: 1,
                    column: 1,
                },
                Statement::VariableDeclaration {
                    name: "n".to_string(),
                    value: Expression::ActionCall {
                        name: "dead".to_string(),
                        arguments: vec![],
                        line: 6,
                        column: 1,
                    },
                    is_constant: false,
                    line: 6,
                    column: 1,
                },
                Statement::OpenFileStatement {
                    path: Expression::Variable("n".to_string(), 7, 1),
                    variable_name: "f".to_string(),
                    mode: crate::parser::ast::FileOpenMode::Read,
                    line: 7,
                    column: 1,
                },
            ],
        };

        let mut type_checker = TypeChecker::new();
        let result = type_checker.check_types(&program);
        assert!(
            result.is_err(),
            "Reachable Number return used as a file path must still be flagged"
        );
        let errors = result.err().unwrap().into_diagnostics();
        assert!(
            errors.iter().any(|e| e.found == Some(Type::Number)),
            "Inferred type should be precisely Number (not widened to Any), got: {errors:?}"
        );
    }

    /// Issue #560 residual: unannotated static container methods must be
    /// refined in the container registry just like instance methods — a
    /// value-returning one gets its inferred type (so `Container.method`
    /// member access reports an accurate function type) and a void one goes
    /// back to `Nothing` instead of keeping the provisional `Unknown` seed.
    /// Static method *calls* are still a future feature at runtime, so the
    /// registry is the observable surface to pin down here.
    #[test]
    fn test_static_method_return_types_refined_in_registry() {
        let code = r#"
create container MathUtils:
    static action get_pair:
        return [1 and 2]
    end
    static action log_it:
        display "hi"
    end
end
"#;
        let tokens = crate::lexer::lex_wfl_with_positions(code);
        let mut parser = crate::parser::Parser::new(&tokens);
        let program = parser.parse().expect("Should parse");

        let mut type_checker = TypeChecker::new();
        let _ = type_checker.check_types(&program);

        let container = type_checker
            .analyzer
            .get_container("MathUtils")
            .expect("container should be registered");
        let get_pair = container
            .static_methods
            .get("get_pair")
            .expect("static method get_pair should be registered");
        assert!(
            matches!(get_pair.return_type, Type::List(_)),
            "value-returning static method should have an inferred List return type, got {:?}",
            get_pair.return_type
        );
        let log_it = container
            .static_methods
            .get("log_it")
            .expect("static method log_it should be registered");
        assert_eq!(
            log_it.return_type,
            Type::Nothing,
            "void static method should be refined to Nothing, not left as the Unknown seed"
        );
    }

    #[test]
    fn opaque_method_calls_escape_captured_mutable_scalars() {
        let code = r#"
store captured_number as 1
create container Mutator:
    action reset:
        change captured_number to nothing
    end
end
create new Mutator as mutator:
end
mutator.reset()
"#;
        let tokens = crate::lexer::lex_wfl_with_positions(code);
        let program = crate::parser::Parser::new(&tokens)
            .parse()
            .expect("program should parse");
        let mut checker = TypeChecker::new();
        checker
            .check_types(&program)
            .expect("the opaque call is gradual rather than a static rejection");

        assert_eq!(
            checker
                .analyzer
                .get_symbol("captured_number")
                .and_then(|symbol| symbol.symbol_type.clone()),
            Some(Type::Any),
            "a method can rebind a captured mutable scalar, so its old Number type is stale"
        );
    }

    /// Binding key for `name` after checking a one-statement program. Alias
    /// tests drive the relation directly so a non-converging relation shows up
    /// as a failed assertion rather than a hung test process.
    fn binding_key_after_checking(
        checker: &mut TypeChecker,
        source: &str,
        name: &str,
    ) -> SymbolBindingKey {
        let tokens = lex_wfl_with_positions(source);
        let mut parser = Parser::new(&tokens);
        let program = parser.parse().expect("program should parse");
        checker
            .check_types(&program)
            .unwrap_or_else(|error| panic!("setup program should type-check: {error:?}"));
        checker
            .analyzer
            .get_symbol_binding_key(name)
            .unwrap_or_else(|| panic!("binding {name:?} should exist"))
    }

    /// Deepest `index_depth` reachable anywhere in the alias relation, counting
    /// both map keys and group members.
    fn deepest_alias_depth(checker: &TypeChecker) -> usize {
        checker
            .list_alias_groups
            .iter()
            .flat_map(|(path, members)| std::iter::once(path).chain(members))
            .map(|path| path.index_depth)
            .max()
            .unwrap_or(0)
    }

    fn alias_path(binding: &SymbolBindingKey, index_depth: usize) -> ListAliasPath {
        ListAliasPath {
            binding: binding.clone(),
            index_depth,
        }
    }

    /// Issue #654. `add_structural_list_alias` materializes descendants at a
    /// *translated* depth. When a binding aliases itself at a different depth —
    /// what `push with scope and scope` records — each application produces a
    /// strictly deeper path, that path becomes a new map key, and the new key
    /// is a "descendant" for the next application. The relation must instead
    /// stabilize, or the checker spins forever with no diagnostic.
    #[test]
    fn self_referential_structural_alias_relation_reaches_a_fixpoint() {
        let mut checker = TypeChecker::new();
        let scope = binding_key_after_checking(&mut checker, "store scope as [1]\n", "scope");

        let root = alias_path(&scope, 0);
        let nested = alias_path(&scope, 1);

        // Re-apply the exact translation the statement walker performs for a
        // self-push. A relation with a fixpoint stops deepening; the buggy one
        // gains a level on every single application.
        let mut deepest = deepest_alias_depth(&checker);
        let mut applications = 0;
        let mut reached_fixpoint = false;
        for _ in 0..64 {
            checker.add_structural_list_alias(root.clone(), nested.clone());
            applications += 1;
            let next = deepest_alias_depth(&checker);
            if next == deepest {
                reached_fixpoint = true;
                break;
            }
            deepest = next;
        }

        assert!(
            reached_fixpoint,
            "the self-referential alias relation never stabilized: depth reached {deepest} after \
             {applications} applications and was still growing (issue #654)"
        );

        // Stability has to hold, not just be observed once.
        for _ in 0..8 {
            checker.add_structural_list_alias(root.clone(), nested.clone());
        }
        assert_eq!(
            deepest_alias_depth(&checker),
            deepest,
            "alias depth resumed growing after the relation had stabilized"
        );
    }

    /// The companion read path. `list_alias_members_for_path` translates every
    /// ancestor relation upward by the query's offset, so it can report members
    /// deeper than anything stored. Those synthesized members are fed straight
    /// back into the relation, so they must be bounded too.
    #[test]
    fn alias_members_never_report_a_path_deeper_than_the_relation_admits() {
        let mut checker = TypeChecker::new();
        let scope = binding_key_after_checking(&mut checker, "store scope as [1]\n", "scope");

        let root = alias_path(&scope, 0);
        checker.add_structural_list_alias(root.clone(), alias_path(&scope, 1));

        // Saturate the relation first, so `bound` is the relation's true ceiling.
        let mut bound = deepest_alias_depth(&checker);
        for _ in 0..64 {
            checker.add_structural_list_alias(root.clone(), alias_path(&scope, 1));
            let next = deepest_alias_depth(&checker);
            if next == bound {
                break;
            }
            bound = next;
        }

        // Querying *at* the ceiling translates every ancestor by that offset.
        let deepest_member = checker
            .list_alias_members_for_path(&alias_path(&scope, bound))
            .into_iter()
            .map(|member| member.index_depth)
            .max()
            .unwrap_or(0);

        assert!(
            deepest_member <= MAX_LIST_ALIAS_INDEX_DEPTH,
            "alias members reported depth {deepest_member}, past the tracked bound of \
             {MAX_LIST_ALIAS_INDEX_DEPTH} (issue #654)"
        );
        assert!(
            deepest_member <= bound,
            "alias members reported depth {deepest_member} for a query at the relation's ceiling \
             {bound}; synthesized members must not escape the bound (issue #654)"
        );
    }

    /// The bound has to hold on the *real* pipeline, not only when the relation
    /// is driven by hand. Checking a program whose aggregates are cyclic — the
    /// `binds`-inside-`scope` shape from the issue, plus a direct self-push —
    /// must leave every path in the relation within the documented depth.
    ///
    /// This is the assertion that ties the constant to observed behaviour: if a
    /// future change reintroduces an unbounded translation, the relation stops
    /// respecting the bound here even if the hand-driven fixpoint tests are
    /// edited around.
    #[test]
    fn checking_a_cyclic_program_keeps_every_alias_path_within_the_bound() {
        let source = r#"
store scope as [1]
store binds as [1]
create map nsval:
    "binds" is binds
end map
push with scope and nsval
push with binds and scope
push with scope and scope
push with scope and binds
"#;
        let tokens = lex_wfl_with_positions(source);
        let mut parser = Parser::new(&tokens);
        let program = parser.parse().expect("program should parse");
        let mut checker = TypeChecker::new();
        // Acceptance is not the claim under test — a cyclic aggregate may or may
        // not draw a diagnostic. Termination and the depth bound are the claim.
        let _ = checker.check_types(&program);

        let deepest = deepest_alias_depth(&checker);
        assert!(
            deepest <= MAX_LIST_ALIAS_INDEX_DEPTH,
            "checking a cyclic program left an alias path at depth {deepest}, past the tracked \
             bound of {MAX_LIST_ALIAS_INDEX_DEPTH} (issue #654)"
        );
    }
}
