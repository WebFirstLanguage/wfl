//! Centralized registry of WFL builtin functions
//! This module ensures the Analyzer and TypeChecker remain synchronized

use std::collections::HashSet;
use std::sync::OnceLock;

/// Complete list of all builtin function names in WFL
/// This list includes:
/// 1. Functions actually implemented in stdlib modules
/// 2. Functions recognized by TypeChecker (for future compatibility)
/// 3. Special test functions used in test programs
const BUILTIN_FUNCTIONS: &[&str] = &[
    // Core functions (implemented in stdlib/core.rs)
    "print",
    "typeof",
    "type_of",
    "isnothing",
    "is_nothing",
    
    // Math functions (implemented in stdlib/math.rs)
    "abs",
    "round",
    "floor",
    "ceil",
    "random",
    "clamp",
    // Math functions recognized by TypeChecker but not yet implemented
    "min",
    "max",
    "power",
    "sqrt",
    "sin",
    "cos",
    "tan",
    
    // Text functions (implemented in stdlib/text.rs)
    "length",  // Also works for lists
    "touppercase",
    "to_uppercase",
    "tolowercase",
    "to_lowercase",
    "contains",  // Also works for lists
    "substring",
    // Text functions recognized by TypeChecker but not yet implemented
    "indexof",
    "index_of",
    "lastindexof",
    "last_index_of",
    "replace",
    "trim",
    "padleft",
    "padright",
    "capitalize",
    "reverse",
    "startswith",
    "starts_with",
    "endswith",
    "ends_with",
    "split",
    "join",
    
    // List functions (implemented in stdlib/list.rs)
    "push",
    "pop",
    // List functions recognized by TypeChecker but not yet implemented
    "shift",
    "unshift",
    "remove_at",
    "removeat",
    "insert_at",
    "insertat",
    "sort",
    "reverse_list",
    "filter",
    "map",
    "reduce",
    "foreach",
    "find",
    "find_index",
    "includes",
    "slice",
    "every",
    "some",
    "fill",
    "concat",
    "unique",
    "clear",
    "count",
    "size",
    
    // Time functions (implemented in stdlib/time.rs)
    "now",
    "today",
    "datetime_now",
    "format_date",
    "format_time",
    "format_datetime",
    "parse_date",
    "parse_time",
    "create_time",
    "create_date",
    "add_days",
    "days_between",
    "current_date",
    // Time functions recognized by TypeChecker but not yet implemented
    "sleep",
    "time",
    "date",
    "year",
    "month",
    "day",
    "hour",
    "minute",
    "second",
    "dayofweek",
    "day_of_week",
    "adddays",  // Duplicate of add_days
    "addmonths",
    "add_months",
    "addyears",
    "add_years",
    "addhours",
    "add_hours",
    "addminutes",
    "add_minutes",
    "addseconds",
    "add_seconds",
    "formatdate",  // Duplicate of format_date
    "formattime",  // Duplicate of format_time
    "parsedate",  // Duplicate of parse_date
    "isleapyear",
    "is_leap_year",
    "daysbetween",  // Duplicate of days_between
    "monthsbetween",
    "months_between",
    "yearsbetween",
    "years_between",
    
    // Pattern functions (implemented in stdlib/pattern.rs)
    "pattern_matches",
    "pattern_find",
    "pattern_find_all",
    // Pattern functions recognized by TypeChecker but not yet implemented
    "compile_pattern",
    "match_pattern",
    "replace_pattern",
    "pattern",
    "match",
    "test",
    "extract",
    "ismatch",
    "is_match",
    "findall",
    "find_all",
    
    // File system functions (implemented in stdlib/filesystem.rs)
    "list_dir",
    "glob",
    "rglob",
    "path_join",
    "path_basename",
    "path_dirname",
    "makedirs",
    "file_mtime",
    "path_exists",
    "is_file",
    "is_dir",
    // File system functions recognized by TypeChecker but not yet implemented
    "read_file",
    "write_file",
    "file_exists",
    "delete_file",
    "create_directory",
    "list_directory",
    "is_directory",
    
    // Special test functions (used in test programs)
    "helper_function",
    "nested_function",
];

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
}
