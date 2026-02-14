//! Centralized registry of WFL builtin functions
//! This module ensures the Analyzer and TypeChecker remain synchronized

use std::collections::HashSet;
use std::sync::OnceLock;

macro_rules! define_builtins {
    (
        $(
            $arity:expr => [ $( $name:literal ),* $(,)? ]
        ),* $(,)?
    ) => {
        /// Complete list of all builtin function names in WFL
        /// This list includes:
        /// 1. Functions actually implemented in stdlib modules
        /// 2. Functions recognized by TypeChecker (for future compatibility)
        /// 3. Special test functions used in test programs
        const BUILTIN_FUNCTIONS: &[&str] = &[
            $( $( $name ),* ),*
        ];

        /// Get the parameter count (arity) for a builtin function
        /// Returns the correct number of parameters each function expects
        pub fn get_function_arity(name: &str) -> usize {
            match name {
                $(
                    $( $name )|* => $arity,
                )*
                _ => {
                    eprintln!(
                        "Warning: Unknown builtin function '{}' - defaulting to 1 argument",
                        name
                    );
                    1
                }
            }
        }
    }
}

define_builtins! {
    // === CORE FUNCTIONS ===
    // Core functions (implemented in stdlib/core.rs)
    1 => ["print", "typeof", "type_of", "isnothing", "is_nothing"],

    // === MATH FUNCTIONS ===
    // Single argument functions (implemented in stdlib/math.rs)
    1 => ["abs", "round", "floor", "ceil", "sqrt", "sin", "cos", "tan"],
    // Two argument functions
    2 => ["min", "max", "power"],
    // Three argument functions
    3 => ["clamp"],

    // === RANDOM FUNCTIONS ===
    // Zero argument functions (implemented in stdlib/random.rs)
    0 => ["random", "random_boolean", "generate_uuid"],
    // Single argument functions
    1 => ["random_from", "random_seed"],
    // Two argument functions
    2 => ["random_between", "random_int"],

    // === CRYPTO FUNCTIONS ===
    // Zero argument functions (implemented in stdlib/crypto.rs)
    0 => ["generate_csrf_token"],
    // Single argument functions
    1 => ["wflhash256", "wflhash512"],
    // Two argument functions
    2 => ["wflhash256_with_salt", "wflmac256"],

    // === JSON FUNCTIONS ===
    // Single argument functions (implemented in stdlib/json.rs)
    1 => ["parse_json", "stringify_json", "stringify_json_pretty"],

    // === QUERY AND FORM PARSING ===
    // Single argument functions (implemented in stdlib/text.rs)
    1 => ["parse_query_string", "parse_cookies", "parse_form_urlencoded"],

    // === TEXT FUNCTIONS ===
    // Single argument functions (implemented in stdlib/text.rs)
    1 => [
        "length", // Also works for lists
        "touppercase", "to_uppercase",
        "tolowercase", "to_lowercase",
        "trim", "capitalize",
        "reverse", "reverse_text"
    ],
    // Two argument functions
    2 => [
        "contains", // Also works for lists
        "indexof", "index_of",
        "lastindexof", "last_index_of",
        "padleft", "padright",
        "startswith", "starts_with",
        "endswith", "ends_with",
        "split", "string_split", "join"
    ],
    // Three argument functions
    3 => ["substring", "replace"],

    // === LIST FUNCTIONS ===
    // Single argument functions (implemented in stdlib/list.rs)
    1 => ["pop", "shift", "sort", "reverse_list", "unique", "clear", "size"],
    // Two argument functions
    2 => [
        "push", "unshift",
        "remove_at", "removeat",
        "includes", "find", "find_index",
        "count", "every", "some", "fill", "concat"
    ],
    // Three argument functions
    3 => ["insert_at", "insertat", "slice"],
    // Variable argument functions (using 2 as minimum for now)
    // List higher-order functions (not yet implemented — require callback support)
    2 => ["filter", "map", "reduce", "foreach"],

    // === TIME FUNCTIONS ===
    // Zero argument functions (implemented in stdlib/time.rs)
    0 => ["now", "today", "datetime_now", "time", "date", "current_date"],
    // Single argument functions
    1 => [
        "year", "month", "day", "hour", "minute", "second",
        "dayofweek", "day_of_week",
        "isleapyear", "is_leap_year",
        "sleep"
    ],
    // Two argument functions
    2 => [
        "format_date", "format_time", "format_datetime",
        "parse_date", "parse_time",
        "add_days", "days_between",
        "adddays", // Duplicate of add_days
        "formatdate", // Duplicate of format_date
        "formattime", // Duplicate of format_time
        "parsedate",  // Duplicate of parse_date
        "daysbetween", // Duplicate of days_between
        "add_hours", "addhours",
        "add_minutes", "addminutes",
        "add_seconds", "addseconds",
        "add_months", "addmonths",
        "add_years", "addyears",
        "months_between", "monthsbetween",
        "years_between", "yearsbetween"
    ],
    // Three argument functions
    3 => ["create_time", "create_date"],

    // === PATTERN FUNCTIONS ===
    // Single argument functions (implemented in stdlib/pattern.rs)
    1 => ["compile_pattern"],
    // Two argument functions
    2 => [
        "pattern_matches", "pattern_find",
        "match_pattern", "pattern", "match", "test",
        "extract", "ismatch", "is_match"
    ],
    // Three argument functions
    3 => ["pattern_find_all", "replace_pattern", "findall", "find_all"],

    // === FILE SYSTEM FUNCTIONS ===
    // Single argument functions (implemented in stdlib/filesystem.rs)
    1 => [
        "list_dir", "path_basename", "path_dirname", "makedirs",
        "file_mtime", "path_exists", "is_file", "is_dir",
        "read_file", "file_exists", "delete_file",
        "create_directory", "list_directory", "is_directory",
        "count_lines", "path_extension", "path_stem", "file_size",
        "remove_file", "remove_dir" // Note: remove_dir can take 1 or 2 args, but listed as 1 here
    ],
    // Two argument functions
    2 => ["glob", "rglob", "path_join", "write_file", "copy_file", "move_file"],

    // === SPECIAL TEST FUNCTIONS ===
    // Special test functions (used in test programs)
    1 => ["helper_function", "nested_function"]
}

/// Cached HashSet for O(1) lookup performance
static BUILTIN_SET: OnceLock<HashSet<&'static str>> = OnceLock::new();

/// Initialize the builtin function set
fn get_builtin_set() -> &'static HashSet<&'static str> {
    BUILTIN_SET.get_or_init(|| BUILTIN_FUNCTIONS.iter().copied().collect())
}

/// Check if a function name is a builtin
pub fn is_builtin_function(name: &str) -> bool {
    get_builtin_set().contains(name)
}

/// Get an iterator over all builtin function names
pub fn builtin_functions() -> impl Iterator<Item = &'static str> {
    BUILTIN_FUNCTIONS.iter().copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_builtin_function() {
        // Core functions
        assert!(is_builtin_function("print"));
        assert!(is_builtin_function("typeof"));
        assert!(is_builtin_function("type_of"));

        // Math functions
        assert!(is_builtin_function("abs"));
        assert!(is_builtin_function("min"));
        assert!(is_builtin_function("max"));
        assert!(is_builtin_function("sqrt"));

        // Text functions
        assert!(is_builtin_function("length"));
        assert!(is_builtin_function("substring"));
        assert!(is_builtin_function("index_of"));
        assert!(is_builtin_function("starts_with"));

        // List functions
        assert!(is_builtin_function("push"));
        assert!(is_builtin_function("pop"));
        assert!(is_builtin_function("unique"));
        assert!(is_builtin_function("clear"));

        // Time functions
        assert!(is_builtin_function("now"));
        assert!(is_builtin_function("today"));
        assert!(is_builtin_function("year"));

        // Pattern functions
        assert!(is_builtin_function("pattern"));
        assert!(is_builtin_function("match"));
        assert!(is_builtin_function("test"));

        // Non-builtins
        assert!(!is_builtin_function("not_a_function"));
        assert!(!is_builtin_function("random_name"));
    }

    #[test]
    fn test_no_duplicates() {
        let set = get_builtin_set();
        assert_eq!(
            set.len(),
            BUILTIN_FUNCTIONS.len(),
            "Duplicate builtin function names detected"
        );
    }

    #[test]
    fn test_function_arity_mappings() {
        // Test critical functions that were causing the bug
        assert_eq!(
            get_function_arity("random"),
            0,
            "random should take 0 arguments"
        );
        assert_eq!(get_function_arity("min"), 2, "min should take 2 arguments");
        assert_eq!(get_function_arity("max"), 2, "max should take 2 arguments");
        assert_eq!(
            get_function_arity("power"),
            2,
            "power should take 2 arguments"
        );
        assert_eq!(
            get_function_arity("contains"),
            2,
            "contains should take 2 arguments"
        );
        assert_eq!(
            get_function_arity("push"),
            2,
            "push should take 2 arguments"
        );

        // Test correctly defined functions still work
        assert_eq!(
            get_function_arity("substring"),
            3,
            "substring should take 3 arguments"
        );
        assert_eq!(
            get_function_arity("clamp"),
            3,
            "clamp should take 3 arguments"
        );
        assert_eq!(get_function_arity("abs"), 1, "abs should take 1 argument");

        // Test both versions of function names
        assert_eq!(
            get_function_arity("indexof"),
            2,
            "indexof should take 2 arguments"
        );
        assert_eq!(
            get_function_arity("index_of"),
            2,
            "index_of should take 2 arguments"
        );
    }

    #[test]
    fn test_all_builtins_have_arity_definition() {
        // Ensure all builtin functions have arity definitions
        // This prevents regression where new functions are added but arity is not defined
        for function_name in BUILTIN_FUNCTIONS {
            let arity = get_function_arity(function_name);
            // Just verify it doesn't panic and returns a reasonable value
            assert!(
                arity <= 10,
                "Function '{}' has unreasonable arity: {}",
                function_name,
                arity
            );
        }
    }
}
