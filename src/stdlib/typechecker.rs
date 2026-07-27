use crate::analyzer::Analyzer;
use crate::parser::ast::Type;

/// Register the static contracts for every native function installed by
/// `stdlib::register_stdlib`.
///
/// `Type::Any` is used only for values that are known to be dynamically typed
/// at runtime (for example parsed JSON or an element from a heterogeneous
/// list). `Type::Unknown` is intentionally absent from these contracts because
/// it represents incomplete inference, not a runtime union.
pub fn register_stdlib_types(analyzer: &mut Analyzer) {
    register_core(analyzer);
    register_math(analyzer);
    register_random(analyzer);
    register_text(analyzer);
    register_list(analyzer);
    register_pattern(analyzer);
    register_json(analyzer);
    register_web(analyzer);
    register_crypto(analyzer);
    register_filesystem(analyzer);
    register_time(analyzer);
}

/// The repeated parameter contract for the two variadic native functions.
pub(crate) fn variadic_builtin_parameter_type(name: &str) -> Option<Type> {
    match name {
        "print" => Some(Type::Any),
        "path_join" => Some(Type::Text),
        _ => None,
    }
}

fn register(analyzer: &mut Analyzer, names: &[&str], parameters: Vec<Type>, result: Type) {
    for name in names {
        analyzer.register_builtin_function(name, parameters.clone(), result.clone());
    }
}

fn register_same_result_overloads(
    analyzer: &mut Analyzer,
    names: &[&str],
    parameter_sets: impl IntoIterator<Item = Vec<Type>>,
    result: Type,
) {
    for parameters in parameter_sets {
        register(analyzer, names, parameters, result.clone());
    }
}

fn list(element: Type) -> Type {
    Type::List(Box::new(element))
}

fn map(key: Type, value: Type) -> Type {
    Type::Map(Box::new(key), Box::new(value))
}

fn repeated(value: Type, count: usize) -> Vec<Type> {
    vec![value; count]
}

fn register_core(analyzer: &mut Analyzer) {
    // `print` is variadic; its repeated Any contract is enforced separately.
    register(analyzer, &["print"], vec![], Type::Nothing);
    register(
        analyzer,
        &["typeof", "type_of"],
        vec![Type::Any],
        Type::Text,
    );
    register(
        analyzer,
        &["isnothing", "is_nothing"],
        vec![Type::Any],
        Type::Boolean,
    );
}

fn register_math(analyzer: &mut Analyzer) {
    register(
        analyzer,
        &["abs", "round", "floor", "ceil", "sqrt", "sin", "cos", "tan"],
        vec![Type::Number],
        Type::Number,
    );
    register(
        analyzer,
        &["min", "max", "power"],
        vec![Type::Number, Type::Number],
        Type::Number,
    );
    register(
        analyzer,
        &["clamp"],
        vec![Type::Number, Type::Number, Type::Number],
        Type::Number,
    );
}

fn register_random(analyzer: &mut Analyzer) {
    register(analyzer, &["random"], vec![], Type::Number);
    register(analyzer, &["random_boolean"], vec![], Type::Boolean);
    register(analyzer, &["generate_uuid"], vec![], Type::Text);
    register(
        analyzer,
        &["random_between", "random_int"],
        vec![Type::Number, Type::Number],
        Type::Number,
    );
    register(analyzer, &["random_from"], vec![list(Type::Any)], Type::Any);
    register(
        analyzer,
        &["random_seed"],
        vec![Type::Number],
        Type::Nothing,
    );
}

fn register_text(analyzer: &mut Analyzer) {
    register(
        analyzer,
        &[
            "touppercase",
            "to_uppercase",
            "tolowercase",
            "to_lowercase",
            "trim",
            "capitalize",
            "reverse",
            "reverse_text",
        ],
        vec![Type::Text],
        Type::Text,
    );
    register(
        analyzer,
        &["substring"],
        vec![Type::Text, Type::Number, Type::Number],
        Type::Text,
    );
    register(
        analyzer,
        &["string_split", "split"],
        vec![Type::Text, Type::Text],
        list(Type::Text),
    );
    register(
        analyzer,
        &["starts_with", "startswith", "ends_with", "endswith"],
        vec![Type::Text, Type::Text],
        Type::Boolean,
    );
    register(
        analyzer,
        &["replace"],
        vec![Type::Text, Type::Text, Type::Text],
        Type::Text,
    );
    register(
        analyzer,
        &["last_index_of", "lastindexof"],
        vec![Type::Text, Type::Text],
        Type::Number,
    );
    register(
        analyzer,
        &["padleft", "padright"],
        vec![Type::Text, Type::Number],
        Type::Text,
    );
    register(
        analyzer,
        &["format_number"],
        vec![Type::Number, Type::Number],
        Type::Text,
    );
    register(
        analyzer,
        &[
            "parse_query_string",
            "parse_cookies",
            "parse_form_urlencoded",
        ],
        vec![Type::Text],
        map(Type::Text, Type::Text),
    );
}

fn register_list(analyzer: &mut Analyzer) {
    let dynamic_list = list(Type::Any);

    register_same_result_overloads(
        analyzer,
        &["length", "size"],
        [
            vec![dynamic_list.clone()],
            vec![Type::Text],
            vec![Type::Binary],
        ],
        Type::Number,
    );
    register_same_result_overloads(
        analyzer,
        &["contains", "includes"],
        [
            vec![dynamic_list.clone(), Type::Any],
            vec![Type::Text, Type::Text],
        ],
        Type::Boolean,
    );
    register_same_result_overloads(
        analyzer,
        &["indexof", "index_of", "find_index"],
        [
            vec![dynamic_list.clone(), Type::Any],
            vec![Type::Text, Type::Text],
        ],
        Type::Number,
    );

    register(
        analyzer,
        &["push", "unshift"],
        vec![dynamic_list.clone(), Type::Any],
        Type::Nothing,
    );
    register(
        analyzer,
        &["insert_at", "insertat"],
        vec![dynamic_list.clone(), Type::Number, Type::Any],
        Type::Nothing,
    );
    register(
        analyzer,
        &["clear", "sort", "reverse_list"],
        vec![dynamic_list.clone()],
        Type::Nothing,
    );
    register(
        analyzer,
        &["fill"],
        vec![dynamic_list.clone(), Type::Any],
        Type::Nothing,
    );

    register(
        analyzer,
        &["pop", "shift"],
        vec![dynamic_list.clone()],
        Type::Any,
    );
    register(
        analyzer,
        &["remove_at", "removeat"],
        vec![dynamic_list.clone(), Type::Number],
        Type::Any,
    );

    register(
        analyzer,
        &["slice"],
        vec![dynamic_list.clone(), Type::Number, Type::Number],
        dynamic_list.clone(),
    );
    register(
        analyzer,
        &["concat"],
        vec![dynamic_list.clone(), dynamic_list.clone()],
        dynamic_list.clone(),
    );
    register(
        analyzer,
        &["unique"],
        vec![dynamic_list.clone()],
        dynamic_list.clone(),
    );

    register(
        analyzer,
        &["join"],
        vec![dynamic_list.clone(), Type::Text],
        Type::Text,
    );
    register(
        analyzer,
        &["count"],
        vec![dynamic_list.clone(), Type::Any],
        Type::Number,
    );
    register(
        analyzer,
        &["find"],
        vec![dynamic_list.clone(), Type::Any],
        Type::Optional(Box::new(Type::Any)),
    );
    register(
        analyzer,
        &["every", "some"],
        vec![dynamic_list, Type::Any],
        Type::Boolean,
    );
}

fn register_pattern(analyzer: &mut Analyzer) {
    let parameters = vec![Type::Text, Type::Pattern];
    register(
        analyzer,
        &["pattern_matches"],
        parameters.clone(),
        Type::Boolean,
    );
    register(
        analyzer,
        &["pattern_find"],
        parameters.clone(),
        Type::Optional(Box::new(map(Type::Text, Type::Any))),
    );
    register(
        analyzer,
        &["pattern_find_all"],
        parameters,
        list(map(Type::Text, Type::Any)),
    );
}

fn register_json(analyzer: &mut Analyzer) {
    register(analyzer, &["parse_json"], vec![Type::Text], Type::Any);

    let json_values = [
        Type::Nothing,
        Type::Boolean,
        Type::Number,
        Type::Text,
        list(Type::Any),
        map(Type::Text, Type::Any),
    ];
    for value_type in json_values {
        register(
            analyzer,
            &["stringify_json", "stringify_json_pretty"],
            vec![value_type],
            Type::Text,
        );
    }
}

fn register_web(analyzer: &mut Analyzer) {
    register(
        analyzer,
        &["path_params"],
        vec![Type::Text, Type::Text],
        Type::Optional(Box::new(map(Type::Text, Type::Text))),
    );
    register(
        analyzer,
        &["path_matches"],
        vec![Type::Text, Type::Text],
        Type::Boolean,
    );
    register(analyzer, &["mime_type"], vec![Type::Text], Type::Text);

    let parts = list(map(Type::Text, Type::Any));
    register_same_result_overloads(
        analyzer,
        &["parse_multipart"],
        [vec![Type::Text, Type::Text], vec![Type::Binary, Type::Text]],
        parts,
    );
}

fn register_crypto(analyzer: &mut Analyzer) {
    register(
        analyzer,
        &["wflhash256", "wflhash512", "sha256"],
        vec![Type::Text],
        Type::Text,
    );
    register(
        analyzer,
        &["wflhash256_with_salt", "wflmac256", "hmac_sha256"],
        vec![Type::Text, Type::Text],
        Type::Text,
    );
    register(analyzer, &["generate_csrf_token"], vec![], Type::Text);
    register(
        analyzer,
        &["pbkdf2_hmac_sha256"],
        vec![Type::Text, Type::Text, Type::Number, Type::Number],
        Type::Text,
    );
    register(
        analyzer,
        &["constant_time_equals"],
        vec![Type::Text, Type::Text],
        Type::Boolean,
    );
    register(
        analyzer,
        &["secure_random_bytes"],
        vec![Type::Number],
        Type::Text,
    );
    register(
        analyzer,
        &[
            "hash_password",
            "argon2_hash",
            "bcrypt_hash",
            "scrypt_hash",
            "pbkdf2_hash",
        ],
        vec![Type::Text],
        Type::Text,
    );
    register(
        analyzer,
        &[
            "verify_password",
            "argon2_verify",
            "bcrypt_verify",
            "scrypt_verify",
            "pbkdf2_verify",
        ],
        vec![Type::Text, Type::Text],
        Type::Boolean,
    );
}

fn register_filesystem(analyzer: &mut Analyzer) {
    register(analyzer, &["list_dir"], vec![Type::Text], list(Type::Text));
    register(
        analyzer,
        &["glob", "rglob"],
        vec![Type::Text, Type::Text],
        list(Type::Text),
    );

    // `path_join` is variadic; this one-argument signature supplies its result
    // type while the repeated Text contract is enforced separately.
    register(analyzer, &["path_join"], vec![Type::Text], Type::Text);
    register(
        analyzer,
        &[
            "path_basename",
            "path_dirname",
            "path_extension",
            "path_stem",
        ],
        vec![Type::Text],
        Type::Text,
    );
    register(
        analyzer,
        &["makedirs", "remove_file", "delete_file"],
        vec![Type::Text],
        Type::Nothing,
    );
    register(
        analyzer,
        &["file_mtime", "count_lines", "file_size"],
        vec![Type::Text],
        Type::Number,
    );
    register(
        analyzer,
        &["path_exists", "is_file", "is_dir"],
        vec![Type::Text],
        Type::Boolean,
    );
    register(
        analyzer,
        &["copy_file", "move_file"],
        vec![Type::Text, Type::Text],
        Type::Nothing,
    );
    register_same_result_overloads(
        analyzer,
        &["remove_dir"],
        [vec![Type::Text], vec![Type::Text, Type::Boolean]],
        Type::Nothing,
    );
}

fn register_time(analyzer: &mut Analyzer) {
    let date = Type::Date;
    let time = Type::Time;
    let datetime = Type::DateTime;

    register(analyzer, &["today"], vec![], date.clone());
    register(analyzer, &["now"], vec![], time.clone());
    register(
        analyzer,
        &["datetime_now", "utc_now"],
        vec![],
        datetime.clone(),
    );
    register(analyzer, &["current_date"], vec![], Type::Text);

    register(
        analyzer,
        &["format_date"],
        vec![date.clone(), Type::Text],
        Type::Text,
    );
    register(
        analyzer,
        &["format_time"],
        vec![time.clone(), Type::Text],
        Type::Text,
    );
    register(
        analyzer,
        &["format_datetime"],
        vec![datetime.clone(), Type::Text],
        Type::Text,
    );
    register(
        analyzer,
        &["parse_date"],
        vec![Type::Text, Type::Text],
        date.clone(),
    );
    register(
        analyzer,
        &["parse_time"],
        vec![Type::Text, Type::Text],
        time.clone(),
    );

    register_same_result_overloads(
        analyzer,
        &["create_time"],
        [repeated(Type::Number, 2), repeated(Type::Number, 3)],
        time.clone(),
    );
    register(
        analyzer,
        &["create_date"],
        repeated(Type::Number, 3),
        date.clone(),
    );
    register_same_result_overloads(
        analyzer,
        &["create_datetime"],
        (3..=6).map(|count| repeated(Type::Number, count)),
        datetime.clone(),
    );

    register(
        analyzer,
        &["add_days", "subtract_days"],
        vec![date.clone(), Type::Number],
        date.clone(),
    );
    register(
        analyzer,
        &["days_between"],
        vec![date.clone(), date.clone()],
        Type::Number,
    );
    register(
        analyzer,
        &["date_part"],
        vec![datetime.clone()],
        date.clone(),
    );
    register(
        analyzer,
        &["time_part"],
        vec![datetime.clone()],
        time.clone(),
    );

    register_same_result_overloads(
        analyzer,
        &[
            "year",
            "month",
            "day",
            "dayofweek",
            "day_of_week",
            "dayofyear",
            "day_of_year",
            "week_of_year",
        ],
        [vec![date.clone()], vec![datetime.clone()]],
        Type::Number,
    );
    register_same_result_overloads(
        analyzer,
        &["hour", "minute", "second"],
        [vec![time.clone()], vec![datetime.clone()]],
        Type::Number,
    );
    register_same_result_overloads(
        analyzer,
        &["is_leap_year", "isleapyear"],
        [
            vec![Type::Number],
            vec![date.clone()],
            vec![datetime.clone()],
        ],
        Type::Boolean,
    );
    register(
        analyzer,
        &["days_in_month"],
        vec![Type::Number, Type::Number],
        Type::Number,
    );

    register_same_result_overloads(
        analyzer,
        &["timestamp"],
        [
            vec![],
            vec![date.clone()],
            vec![time.clone()],
            vec![datetime.clone()],
        ],
        Type::Number,
    );
    register(
        analyzer,
        &["datetime_from_timestamp"],
        vec![Type::Number],
        datetime.clone(),
    );
    register_same_result_overloads(
        analyzer,
        &["time_diff"],
        [
            vec![time.clone(), time.clone()],
            vec![time.clone(), datetime.clone()],
            vec![datetime.clone(), time],
            vec![datetime.clone(), datetime],
        ],
        Type::Number,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::SymbolKind;
    use crate::builtins;
    use crate::interpreter::environment::Environment;
    use crate::interpreter::value::Value;
    use std::collections::BTreeSet;

    fn contains_unknown(value_type: &Type) -> bool {
        match value_type {
            Type::Unknown => true,
            Type::List(inner) | Type::Async(inner) => contains_unknown(inner),
            Type::Map(key, value) => contains_unknown(key) || contains_unknown(value),
            Type::Function {
                parameters,
                return_type,
            } => parameters.iter().any(contains_unknown) || contains_unknown(return_type),
            _ => false,
        }
    }

    #[test]
    fn runtime_inventory_matches_installed_native_functions() {
        let environment = Environment::new_global();
        crate::stdlib::register_stdlib(&mut environment.borrow_mut());

        let actual: BTreeSet<String> = environment
            .borrow()
            .values
            .iter()
            .filter_map(|(name, value)| {
                matches!(value, Value::NativeFunction(_, _)).then_some(name.clone())
            })
            .collect();
        let catalogued: BTreeSet<String> = builtins::implemented_builtin_functions()
            .map(str::to_string)
            .collect();

        assert_eq!(
            actual, catalogued,
            "the native runtime and implemented-builtin catalog must stay synchronized"
        );
    }

    #[test]
    fn every_runtime_builtin_has_a_precise_static_contract() {
        let mut analyzer = Analyzer::new();
        register_stdlib_types(&mut analyzer);

        for name in builtins::implemented_builtin_functions() {
            let symbol = analyzer
                .get_symbol(name)
                .unwrap_or_else(|| panic!("runtime builtin '{name}' has no static contract"));
            let SymbolKind::Function { signatures } = &symbol.kind else {
                panic!("runtime builtin '{name}' was not registered as a function");
            };
            assert!(
                !signatures.is_empty(),
                "runtime builtin '{name}' has no callable signature"
            );
            for signature in signatures {
                assert!(
                    signature
                        .parameters
                        .iter()
                        .filter_map(|parameter| parameter.param_type.as_ref())
                        .all(|parameter| !contains_unknown(parameter)),
                    "runtime builtin '{name}' uses Unknown as a parameter wildcard"
                );
                assert!(
                    signature
                        .return_type
                        .as_ref()
                        .is_some_and(|result| !contains_unknown(result)),
                    "runtime builtin '{name}' has an absent or Unknown result"
                );
            }
        }
    }

    #[test]
    fn future_reserved_names_do_not_receive_fake_runtime_contracts() {
        let mut analyzer = Analyzer::new();
        register_stdlib_types(&mut analyzer);

        for name in builtins::builtin_functions()
            .filter(|name| !builtins::is_implemented_builtin_function(name))
        {
            assert!(
                analyzer.get_symbol(name).is_none(),
                "reserved but unimplemented name '{name}' must not have a fake contract"
            );
        }
    }

    #[test]
    fn remove_dir_registers_both_runtime_arities() {
        let mut analyzer = Analyzer::new();
        register_stdlib_types(&mut analyzer);
        let symbol = analyzer.get_symbol("remove_dir").unwrap();
        let SymbolKind::Function { signatures } = &symbol.kind else {
            panic!("remove_dir should be a function");
        };
        let arities: BTreeSet<_> = signatures
            .iter()
            .map(|signature| signature.parameters.len())
            .collect();
        assert_eq!(arities, BTreeSet::from([1, 2]));
    }
}
