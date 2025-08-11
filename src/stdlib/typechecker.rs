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
    register_contains(analyzer);
    register_substring(analyzer);
    register_startswith(analyzer);

    // register_list_length(analyzer); // Commented out - length is now registered once for all types
    register_push(analyzer);
    register_pop(analyzer);
    register_indexof(analyzer);

    register_pattern_matches(analyzer);
    register_pattern_find(analyzer);
    register_pattern_replace(analyzer);
    register_pattern_split(analyzer);
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
    
    // Register length for Text
    let text_param_types = vec![Type::Text];
    analyzer.register_builtin_function("length", text_param_types, return_type.clone());
    
    // Register length for List (any list type)
    let list_param_types = vec![Type::List(Box::new(Type::Unknown))];
    analyzer.register_builtin_function("length", list_param_types, return_type);
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

fn register_contains(analyzer: &mut Analyzer) {
    // Register contains for both text and list cases
    let return_type = Type::Boolean;
    
    // Text contains text case
    let text_param_types = vec![Type::Text, Type::Text];
    analyzer.register_builtin_function("contains", text_param_types, return_type.clone());
    
    // List contains any case  
    let list_param_types = vec![Type::List(Box::new(Type::Unknown)), Type::Unknown];
    analyzer.register_builtin_function("contains", list_param_types, return_type);
}

fn register_substring(analyzer: &mut Analyzer) {
    let return_type = Type::Text;
    let param_types = vec![Type::Text, Type::Number, Type::Number];

    analyzer.register_builtin_function("substring", param_types, return_type);
}

fn register_startswith(analyzer: &mut Analyzer) {
    let return_type = Type::Boolean;
    let param_types = vec![Type::Text, Type::Text];

    analyzer.register_builtin_function("startswith", param_types.clone(), return_type.clone());
    analyzer.register_builtin_function("starts_with", param_types, return_type);
}

// Removed - length is now registered once for all types in register_text_length
// fn register_list_length(analyzer: &mut Analyzer) {
//     let return_type = Type::Number;
//     let param_types = vec![Type::List(Box::new(Type::Unknown))];
//
//     analyzer.register_builtin_function("length", param_types, return_type);
// }

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
