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
    register_regex_find(analyzer);
    register_regex_match_all(analyzer);
    register_regex_replace(analyzer);

    register_list_length(analyzer);
    register_push(analyzer);
    register_pop(analyzer);
    register_list_contains(analyzer);
    register_indexof(analyzer);
    register_sort_by(analyzer);

    register_pattern_matches(analyzer);
    register_pattern_find(analyzer);
    register_pattern_replace(analyzer);
    register_pattern_split(analyzer);

    register_cli_functions(analyzer);
    register_fs_functions(analyzer);
    register_path_functions(analyzer);
    register_time_iso(analyzer);
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

fn register_regex_find(analyzer: &mut Analyzer) {
    let return_type = Type::Unknown; // Can return Text or Nothing
    let param_types = vec![Type::Text, Type::Text];

    analyzer.register_builtin_function("regex_find", param_types, return_type);
}

fn register_regex_match_all(analyzer: &mut Analyzer) {
    let return_type = Type::List(Box::new(Type::Text));
    let param_types = vec![Type::Text, Type::Text];

    analyzer.register_builtin_function("regex_match_all", param_types, return_type);
}

fn register_regex_replace(analyzer: &mut Analyzer) {
    let return_type = Type::Text;
    let param_types = vec![Type::Text, Type::Text, Type::Text];

    analyzer.register_builtin_function("regex_replace", param_types, return_type);
}

fn register_sort_by(analyzer: &mut Analyzer) {
    let return_type = Type::List(Box::new(Type::Unknown));
    let param_types = vec![Type::List(Box::new(Type::Unknown)), Type::Text]; // Optional third param for custom order

    analyzer.register_builtin_function("sort_by", param_types, return_type);
}

fn register_cli_functions(analyzer: &mut Analyzer) {
    analyzer.register_builtin_function("get_args", vec![], Type::List(Box::new(Type::Text)));
    
    analyzer.register_builtin_function("parse_flags", vec![Type::Text], Type::Map(Box::new(Type::Text), Box::new(Type::Unknown)));
    
    analyzer.register_builtin_function("usage", vec![Type::Text], Type::Text);
}

fn register_fs_functions(analyzer: &mut Analyzer) {
    analyzer.register_builtin_function("glob", vec![Type::Text, Type::Text], Type::List(Box::new(Type::Text)));
    
    analyzer.register_builtin_function("rglob", vec![Type::Text, Type::Text], Type::List(Box::new(Type::Text)));
    
    analyzer.register_builtin_function("read_text", vec![Type::Text], Type::Text);
    
    analyzer.register_builtin_function("write_stream_open", vec![Type::Text], Type::Number);
    
    analyzer.register_builtin_function("write_stream_write", vec![Type::Number, Type::Text], Type::Nothing);
    
    analyzer.register_builtin_function("write_stream_close", vec![Type::Number], Type::Nothing);
}

fn register_path_functions(analyzer: &mut Analyzer) {
    analyzer.register_builtin_function("path_join", vec![Type::Text, Type::Text], Type::Text);
    
    analyzer.register_builtin_function("path_basename", vec![Type::Text], Type::Text);
    
    analyzer.register_builtin_function("path_dirname", vec![Type::Text], Type::Text);
    
    analyzer.register_builtin_function("path_relpath", vec![Type::Text, Type::Text], Type::Text);
    
    analyzer.register_builtin_function("path_normalize", vec![Type::Text], Type::Text);
}

fn register_time_iso(analyzer: &mut Analyzer) {
    analyzer.register_builtin_function("now_iso", vec![], Type::Text);
}
