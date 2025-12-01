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
