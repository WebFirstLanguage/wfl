use crate::analyzer::Analyzer;
use crate::parser::ast::Type;

pub fn register_stdlib_types(analyzer: &mut Analyzer) {
    register_print(analyzer);
    register_typeof(analyzer);
    register_isnothing(analyzer);

    register_abs(analyzer);
    register_round(analyzer);
    register_floor(analyzer);
    register_ceil(analyzer);
    register_random(analyzer);
    register_clamp(analyzer);

    register_text_length(analyzer);
    register_touppercase(analyzer);
    register_tolowercase(analyzer);
    register_text_contains(analyzer);
    register_substring(analyzer);

    register_list_length(analyzer);
    register_push(analyzer);
    register_pop(analyzer);
    register_list_contains(analyzer);
    register_indexof(analyzer);

    register_pattern_matches(analyzer);
    register_pattern_find(analyzer);
    register_pattern_replace(analyzer);
    register_pattern_split(analyzer);

    register_parse_json(analyzer);
    register_stringify_json(analyzer);
    register_stringify_json_pretty(analyzer);

    register_wflhash256(analyzer);
    register_wflhash512(analyzer);
    register_wflhash256_with_salt(analyzer);
    register_wflmac256(analyzer);
    register_count_lines(analyzer);
    register_path_extension(analyzer);
    register_path_stem(analyzer);
    register_file_size(analyzer);
    register_copy_file(analyzer);
    register_move_file(analyzer);
    register_remove_file(analyzer);
    register_remove_dir(analyzer);
}

fn register_print(analyzer: &mut Analyzer) {
    let return_type = Type::Nothing;
    let param_types = vec![]; // Variadic, accepts any number of arguments

    analyzer.register_builtin_function("print", param_types, return_type);
}

fn register_typeof(analyzer: &mut Analyzer) {
    let return_type = Type::Text;
    let param_types = vec![Type::Unknown]; // Accepts any type

    analyzer.register_builtin_function("typeof", param_types.clone(), return_type.clone());

    analyzer.register_builtin_function("type_of", param_types, return_type);
}

fn register_isnothing(analyzer: &mut Analyzer) {
    let return_type = Type::Boolean;
    let param_types = vec![Type::Unknown]; // Accepts any type

    analyzer.register_builtin_function("isnothing", param_types.clone(), return_type.clone());

    analyzer.register_builtin_function("is_nothing", param_types, return_type);
}

fn register_abs(analyzer: &mut Analyzer) {
    let return_type = Type::Number;
    let param_types = vec![Type::Number];

    analyzer.register_builtin_function("abs", param_types, return_type);
}

fn register_round(analyzer: &mut Analyzer) {
    let return_type = Type::Number;
    let param_types = vec![Type::Number];

    analyzer.register_builtin_function("round", param_types, return_type);
}

fn register_floor(analyzer: &mut Analyzer) {
    let return_type = Type::Number;
    let param_types = vec![Type::Number];

    analyzer.register_builtin_function("floor", param_types, return_type);
}

fn register_ceil(analyzer: &mut Analyzer) {
    let return_type = Type::Number;
    let param_types = vec![Type::Number];

    analyzer.register_builtin_function("ceil", param_types, return_type);
}

fn register_random(analyzer: &mut Analyzer) {
    let return_type = Type::Number;
    let param_types = vec![]; // No parameters

    analyzer.register_builtin_function("random", param_types, return_type);
}

fn register_clamp(analyzer: &mut Analyzer) {
    let return_type = Type::Number;
    let param_types = vec![Type::Number, Type::Number, Type::Number];

    analyzer.register_builtin_function("clamp", param_types, return_type);
}

fn register_text_length(analyzer: &mut Analyzer) {
    let return_type = Type::Number;
    let param_types = vec![Type::Text];

    analyzer.register_builtin_function("length", param_types, return_type);
}

fn register_touppercase(analyzer: &mut Analyzer) {
    let return_type = Type::Text;
    let param_types = vec![Type::Text];

    analyzer.register_builtin_function("touppercase", param_types.clone(), return_type.clone());

    analyzer.register_builtin_function("to_uppercase", param_types, return_type);
}

fn register_tolowercase(analyzer: &mut Analyzer) {
    let return_type = Type::Text;
    let param_types = vec![Type::Text];

    analyzer.register_builtin_function("tolowercase", param_types.clone(), return_type.clone());

    analyzer.register_builtin_function("to_lowercase", param_types, return_type);
}

fn register_text_contains(analyzer: &mut Analyzer) {
    let return_type = Type::Boolean;
    let param_types = vec![Type::Text, Type::Text];

    analyzer.register_builtin_function("contains", param_types, return_type);
}

fn register_substring(analyzer: &mut Analyzer) {
    let return_type = Type::Text;
    let param_types = vec![Type::Text, Type::Number, Type::Number];

    analyzer.register_builtin_function("substring", param_types, return_type);
}

fn register_list_length(analyzer: &mut Analyzer) {
    let return_type = Type::Number;
    let param_types = vec![Type::List(Box::new(Type::Unknown))];

    analyzer.register_builtin_function("length", param_types, return_type);
}

fn register_push(analyzer: &mut Analyzer) {
    let return_type = Type::Nothing;
    let param_types = vec![Type::List(Box::new(Type::Unknown)), Type::Unknown];

    analyzer.register_builtin_function("push", param_types, return_type);
}

fn register_pop(analyzer: &mut Analyzer) {
    let return_type = Type::Unknown;
    let param_types = vec![Type::List(Box::new(Type::Unknown))];

    analyzer.register_builtin_function("pop", param_types, return_type);
}

fn register_list_contains(analyzer: &mut Analyzer) {
    let return_type = Type::Boolean;
    let param_types = vec![Type::List(Box::new(Type::Unknown)), Type::Unknown];

    analyzer.register_builtin_function("contains", param_types, return_type);
}

fn register_indexof(analyzer: &mut Analyzer) {
    let return_type = Type::Number;
    let param_types = vec![Type::List(Box::new(Type::Unknown)), Type::Unknown];

    analyzer.register_builtin_function("indexof", param_types.clone(), return_type.clone());

    analyzer.register_builtin_function("index_of", param_types, return_type);
}

fn register_pattern_matches(analyzer: &mut Analyzer) {
    let return_type = Type::Boolean;
    let param_types = vec![Type::Text, Type::Text];

    analyzer.register_builtin_function("matches_pattern", param_types, return_type);
}

fn register_pattern_find(analyzer: &mut Analyzer) {
    let return_type = Type::Map(Box::new(Type::Text), Box::new(Type::Text));
    let param_types = vec![Type::Text, Type::Text];

    analyzer.register_builtin_function("find_pattern", param_types, return_type);
}

fn register_pattern_replace(analyzer: &mut Analyzer) {
    let return_type = Type::Text;
    let param_types = vec![Type::Text, Type::Text, Type::Text];

    analyzer.register_builtin_function("replace_pattern", param_types, return_type);
}

fn register_pattern_split(analyzer: &mut Analyzer) {
    let return_type = Type::List(Box::new(Type::Text));
    let param_types = vec![Type::Text, Type::Text];

    analyzer.register_builtin_function("split_by_pattern", param_types, return_type);
}

fn register_wflhash256(analyzer: &mut Analyzer) {
    let return_type = Type::Text;
    let param_types = vec![Type::Text];

    analyzer.register_builtin_function("wflhash256", param_types, return_type);
}

fn register_wflhash512(analyzer: &mut Analyzer) {
    let return_type = Type::Text;
    let param_types = vec![Type::Text];

    analyzer.register_builtin_function("wflhash512", param_types, return_type);
}

fn register_wflhash256_with_salt(analyzer: &mut Analyzer) {
    let return_type = Type::Text;
    let param_types = vec![Type::Text, Type::Text];

    analyzer.register_builtin_function("wflhash256_with_salt", param_types, return_type);
}

fn register_wflmac256(analyzer: &mut Analyzer) {
    let return_type = Type::Text;
    let param_types = vec![Type::Text, Type::Text];

    analyzer.register_builtin_function("wflmac256", param_types, return_type);
}

fn register_count_lines(analyzer: &mut Analyzer) {
    let return_type = Type::Number;
    let param_types = vec![Type::Text]; // Takes a file path as text

    analyzer.register_builtin_function("count_lines", param_types, return_type);
}

fn register_path_extension(analyzer: &mut Analyzer) {
    let return_type = Type::Text;
    let param_types = vec![Type::Text];

    analyzer.register_builtin_function("path_extension", param_types, return_type);
}

fn register_path_stem(analyzer: &mut Analyzer) {
    let return_type = Type::Text;
    let param_types = vec![Type::Text];

    analyzer.register_builtin_function("path_stem", param_types, return_type);
}

fn register_file_size(analyzer: &mut Analyzer) {
    let return_type = Type::Number;
    let param_types = vec![Type::Text];

    analyzer.register_builtin_function("file_size", param_types, return_type);
}

fn register_copy_file(analyzer: &mut Analyzer) {
    let return_type = Type::Nothing;
    let param_types = vec![Type::Text, Type::Text];

    analyzer.register_builtin_function("copy_file", param_types, return_type);
}

fn register_move_file(analyzer: &mut Analyzer) {
    let return_type = Type::Nothing;
    let param_types = vec![Type::Text, Type::Text];

    analyzer.register_builtin_function("move_file", param_types, return_type);
}

fn register_remove_file(analyzer: &mut Analyzer) {
    let return_type = Type::Nothing;
    let param_types = vec![Type::Text];

    analyzer.register_builtin_function("remove_file", param_types, return_type);
}

fn register_remove_dir(analyzer: &mut Analyzer) {
    let return_type = Type::Nothing;

    // Register 1-arg version (non-recursive)
    let param_types = vec![Type::Text];
    analyzer.register_builtin_function("remove_dir", param_types.clone(), return_type.clone());

    // Register 2-arg version (with recursive flag)
    let param_types_with_recursive = vec![Type::Text, Type::Boolean];
    analyzer.register_builtin_function("remove_dir", param_types_with_recursive, return_type);
}

fn register_parse_json(analyzer: &mut Analyzer) {
    let param_types = vec![Type::Text]; // JSON string
    let return_type = Type::Unknown; // Can return object, list, text, number, boolean, or nothing

    analyzer.register_builtin_function("parse_json", param_types, return_type);
}

fn register_stringify_json(analyzer: &mut Analyzer) {
    let param_types = vec![Type::Unknown]; // Accepts any value
    let return_type = Type::Text; // Returns JSON string

    analyzer.register_builtin_function("stringify_json", param_types, return_type);
}

fn register_stringify_json_pretty(analyzer: &mut Analyzer) {
    let param_types = vec![Type::Unknown]; // Accepts any value
    let return_type = Type::Text; // Returns pretty-printed JSON string

    analyzer.register_builtin_function("stringify_json_pretty", param_types, return_type);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::Analyzer;

    #[test]
    fn test_remove_dir_overload_registration() {
        let mut analyzer = Analyzer::new();

        // Register the overloaded remove_dir function
        register_remove_dir(&mut analyzer);

        // This should succeed - 1-arg version
        let one_arg_result = analyzer.get_symbol("remove_dir");
        assert!(one_arg_result.is_some(), "remove_dir should be registered");

        // Check that we can find the function with appropriate signatures
        let symbol = one_arg_result.unwrap();
        if let crate::analyzer::SymbolKind::Function { signatures } = &symbol.kind {
            // After the fix, we should have both signatures
            println!("Function has {} signatures", signatures.len());

            // Test that we have both 1-arg and 2-arg versions
            assert_eq!(
                signatures.len(),
                2,
                "remove_dir should have both 1-arg and 2-arg versions"
            );

            let has_one_param = signatures.iter().any(|sig| sig.parameters.len() == 1);
            let has_two_param = signatures.iter().any(|sig| sig.parameters.len() == 2);

            assert!(has_one_param, "Should have 1-arg signature");
            assert!(has_two_param, "Should have 2-arg signature");
        }
    }

    #[test]
    fn test_function_overloading_issue() {
        let mut analyzer = Analyzer::new();

        // This test demonstrates the core issue: duplicate function registration fails
        let return_type = Type::Nothing;

        // Register first version - should succeed
        let param_types_1 = vec![Type::Text];
        analyzer.register_builtin_function("test_overload", param_types_1, return_type.clone());

        // Register second version - currently fails silently
        let param_types_2 = vec![Type::Text, Type::Boolean];
        analyzer.register_builtin_function("test_overload", param_types_2, return_type);

        // Lookup the function
        let symbol = analyzer.get_symbol("test_overload");
        assert!(symbol.is_some(), "Function should be registered");

        // This test will now pass after we fix the overloading mechanism
        if let Some(sym) = symbol
            && let crate::analyzer::SymbolKind::Function { signatures } = &sym.kind
        {
            // After the fix, should have multiple signatures
            println!("test_overload function has {} signatures", signatures.len());
            // This assertion now tests that overloading works
            assert_eq!(
                signatures.len(),
                2,
                "Should have both 1-arg and 2-arg signatures"
            );
        }
    }

    #[test]
    fn test_remove_dir_should_support_both_arities() {
        let mut analyzer = Analyzer::new();

        // Register the overloaded remove_dir function
        register_remove_dir(&mut analyzer);

        // Get the registered function
        let symbol = analyzer.get_symbol("remove_dir").unwrap();
        if let crate::analyzer::SymbolKind::Function { signatures } = &symbol.kind {
            // After fixing the overloading, this should pass
            assert_eq!(
                signatures.len(),
                2,
                "Both 1-arg and 2-arg versions should be supported"
            );

            // Check we have the right arities
            let arities: Vec<usize> = signatures.iter().map(|sig| sig.parameters.len()).collect();
            assert!(arities.contains(&1), "Should have 1-arg version");
            assert!(arities.contains(&2), "Should have 2-arg version");
        }
    }
}
