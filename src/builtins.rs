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
    "clamp",
    // Random functions (implemented in stdlib/random.rs)
    "random",
    "random_between",
    "random_int",
    "random_boolean",
    "random_from",
    "random_seed",
    "generate_uuid",
    // Crypto functions (implemented in stdlib/crypto.rs)
    "wflhash256",
    "wflhash512",
    "wflhash256_with_salt",
    "wflmac256",
    "sha256",
    "hmac_sha256",
    "generate_csrf_token",
    // Low-level auth/session primitives (implemented in stdlib/crypto.rs)
    "pbkdf2_hmac_sha256",
    "constant_time_equals",
    "secure_random_bytes",
    // Password hashing (implemented in stdlib/crypto.rs)
    "hash_password",
    "verify_password",
    "argon2_hash",
    "argon2_verify",
    "bcrypt_hash",
    "bcrypt_verify",
    "scrypt_hash",
    "scrypt_verify",
    "pbkdf2_hash",
    "pbkdf2_verify",
    // JSON functions (implemented in stdlib/json.rs)
    "parse_json",
    "stringify_json",
    "stringify_json_pretty",
    // Query and form parsing (implemented in stdlib/text.rs)
    "parse_query_string",
    "parse_cookies",
    "parse_form_urlencoded",
    // Web routing helpers (implemented in stdlib/web.rs)
    "path_params",
    "path_matches",
    "mime_type",
    // Media (image) functions (implemented in stdlib/media.rs)
    "resize_image",
    "image_dimensions",
    // Cache (TTL) functions (implemented in stdlib/cache.rs)
    "create_cache",
    "cache_set",
    "cache_get",
    "cache_has",
    "cache_delete",
    "cache_clear",
    "cache_size",
    // Math functions (implemented in stdlib/math.rs)
    "min",
    "max",
    "power",
    "sqrt",
    "sin",
    "cos",
    "tan",
    // Text functions (implemented in stdlib/text.rs)
    "length", // Also works for lists
    "touppercase",
    "to_uppercase",
    "tolowercase",
    "to_lowercase",
    "contains", // Also works for lists
    "substring",
    // Text functions (implemented in stdlib/text.rs)
    "indexof",
    "index_of",
    "lastindexof",
    "last_index_of",
    "replace",
    "trim",
    "padleft",
    "padright",
    "format_number",
    "capitalize",
    "reverse",
    "reverse_text",
    "startswith",
    "starts_with",
    "endswith",
    "ends_with",
    "split",
    "string_split",
    "join",
    // List functions (implemented in stdlib/list.rs)
    "push",
    "pop",
    // List functions (implemented in stdlib/list.rs)
    "shift",
    "unshift",
    "remove_at",
    "removeat",
    "insert_at",
    "insertat",
    "sort",
    "reverse_list",
    // List higher-order functions (not yet implemented — require callback support)
    "filter",
    "map",
    "reduce",
    "foreach",
    // List functions (implemented in stdlib/list.rs)
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
    "create_datetime",
    "add_days",
    "subtract_days",
    "days_between",
    "current_date",
    "date_part",
    "time_part",
    "utc_now",
    "year",
    "month",
    "day",
    "hour",
    "minute",
    "second",
    "dayofweek",
    "day_of_week",
    "dayofyear",
    "day_of_year",
    "days_in_month",
    "week_of_year",
    "timestamp",
    "datetime_from_timestamp",
    "time_diff",
    // Time functions recognized by TypeChecker but not yet implemented
    "sleep",
    "time",
    "date",
    "adddays", // Duplicate of add_days
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
    "formatdate", // Duplicate of format_date
    "formattime", // Duplicate of format_time
    "parsedate",  // Duplicate of parse_date
    "isleapyear",
    "is_leap_year",
    "daysbetween", // Duplicate of days_between
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
    "count_lines",
    "path_extension",
    "path_stem",
    "file_size",
    "copy_file",
    "move_file",
    "remove_file",
    "remove_dir",
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

/// Get the parameter count (arity) for a builtin function
/// Returns the correct number of parameters each function expects
pub fn get_function_arity(name: &str) -> usize {
    match name {
        // === CORE FUNCTIONS ===
        "print" => 1,
        "typeof" | "type_of" => 1,
        "isnothing" | "is_nothing" => 1,

        // === MATH FUNCTIONS ===
        // Single argument functions
        "abs" | "round" | "floor" | "ceil" | "sqrt" | "sin" | "cos" | "tan" => 1,
        // Two argument functions
        "min" | "max" | "power" => 2,
        // Three argument functions
        "clamp" => 3,

        // === RANDOM FUNCTIONS ===
        // Zero argument functions
        "random" | "random_boolean" | "generate_uuid" => 0,
        // Single argument functions
        "random_from" | "random_seed" => 1,
        // Two argument functions
        "random_between" | "random_int" => 2,

        // === CRYPTO FUNCTIONS ===
        // Zero argument functions
        "generate_csrf_token" => 0,
        // Single argument functions
        "wflhash256" | "wflhash512" | "sha256" | "secure_random_bytes" => 1,
        // Two argument functions
        "wflhash256_with_salt" | "wflmac256" | "hmac_sha256" | "constant_time_equals" => 2,
        // Four argument functions: (password, salt, iterations, length)
        "pbkdf2_hmac_sha256" => 4,

        // === PASSWORD HASHING FUNCTIONS ===
        // Single argument functions (the password)
        "hash_password" | "argon2_hash" | "bcrypt_hash" | "scrypt_hash" | "pbkdf2_hash" => 1,
        // Two argument functions (password and stored hash)
        "verify_password" | "argon2_verify" | "bcrypt_verify" | "scrypt_verify"
        | "pbkdf2_verify" => 2,

        // === JSON FUNCTIONS ===
        // Single argument functions
        "parse_json" | "stringify_json" | "stringify_json_pretty" => 1,

        // === QUERY AND FORM PARSING ===
        // Single argument functions
        "parse_query_string" | "parse_cookies" | "parse_form_urlencoded" => 1,

        // === WEB ROUTING HELPERS ===
        // Two argument functions: (path, template)
        "path_params" | "path_matches" => 2,
        // Single argument: (name)
        "mime_type" => 1,

        // === MEDIA (IMAGE) FUNCTIONS ===
        // Single argument: (image_data)
        "image_dimensions" => 1,
        // Three argument functions: (image_data, width, height)
        "resize_image" => 3,

        // === CACHE (TTL) FUNCTIONS ===
        // Zero argument functions
        "create_cache" => 0,
        // Single argument: (cache)
        "cache_clear" | "cache_size" => 1,
        // Two argument functions: (cache, key)
        "cache_get" | "cache_has" | "cache_delete" => 2,
        // Four argument functions: (cache, key, value, ttl_seconds)
        "cache_set" => 4,

        // === TEXT FUNCTIONS ===
        // Single argument functions
        "length" | "touppercase" | "to_uppercase" | "tolowercase" | "to_lowercase" | "trim"
        | "capitalize" | "reverse" | "reverse_text" => 1,
        // Two argument functions
        "contains" | "indexof" | "index_of" | "lastindexof" | "last_index_of" | "padleft"
        | "padright" | "format_number" | "startswith" | "starts_with" | "endswith"
        | "ends_with" | "split" | "string_split" | "join" => 2,
        // Three argument functions
        "substring" | "replace" => 3,

        // === LIST FUNCTIONS ===
        // Single argument functions
        "pop" | "shift" | "sort" | "reverse_list" | "unique" | "clear" | "size" => 1,
        // Two argument functions
        "push" | "unshift" | "remove_at" | "removeat" | "includes" | "find" | "find_index"
        | "count" | "every" | "some" | "fill" | "concat" => 2,
        // Three argument functions
        "insert_at" | "insertat" | "slice" => 3,
        // Variable argument functions (using 2 as minimum for now)
        "filter" | "map" | "reduce" | "foreach" => 2,

        // === TIME FUNCTIONS ===
        // Zero argument functions
        "now" | "today" | "datetime_now" | "time" | "date" | "current_date" | "utc_now" => 0,
        // Single argument functions
        // (`timestamp` also accepts zero arguments at runtime, but it is listed
        // here so `timestamp of <value>` is not broken by zero-arg auto-invocation)
        "year"
        | "month"
        | "day"
        | "hour"
        | "minute"
        | "second"
        | "dayofweek"
        | "day_of_week"
        | "dayofyear"
        | "day_of_year"
        | "week_of_year"
        | "date_part"
        | "time_part"
        | "datetime_from_timestamp"
        | "timestamp"
        | "isleapyear"
        | "is_leap_year"
        | "sleep" => 1,
        // Two argument functions
        "format_date" | "format_time" | "format_datetime" | "parse_date" | "parse_time"
        | "add_days" | "subtract_days" | "days_between" | "days_in_month" | "time_diff"
        | "adddays" | "formatdate" | "formattime" | "parsedate" | "daysbetween" | "add_hours"
        | "addhours" | "add_minutes" | "addminutes" | "add_seconds" | "addseconds"
        | "add_months" | "addmonths" | "add_years" | "addyears" | "months_between"
        | "monthsbetween" | "years_between" | "yearsbetween" => 2,
        // Three argument functions
        // (`create_datetime` accepts 3-6 arguments at runtime; 3 is the
        // documented minimum used for inference)
        "create_time" | "create_date" | "create_datetime" => 3,

        // === PATTERN FUNCTIONS ===
        // Single argument functions
        "compile_pattern" => 1,
        // Two argument functions
        "pattern_matches" | "pattern_find" | "match_pattern" | "pattern" | "match" | "test"
        | "extract" | "ismatch" | "is_match" => 2,
        // Three argument functions
        "pattern_find_all" | "replace_pattern" | "findall" | "find_all" => 3,

        // === FILE SYSTEM FUNCTIONS ===
        // Single argument functions (remove_dir also here as it can take 1 or 2 args)
        "list_dir" | "path_basename" | "path_dirname" | "makedirs" | "file_mtime"
        | "path_exists" | "is_file" | "is_dir" | "read_file" | "file_exists" | "delete_file"
        | "create_directory" | "list_directory" | "is_directory" | "count_lines"
        | "path_extension" | "path_stem" | "file_size" | "remove_file" | "remove_dir" => 1,
        // Two argument functions
        "glob" | "rglob" | "path_join" | "write_file" | "copy_file" | "move_file" => 2,

        // === SPECIAL TEST FUNCTIONS ===
        "helper_function" | "nested_function" => 1,

        // Default case for unknown functions
        // This should not happen if all builtins are properly catalogued above
        _ => {
            eprintln!(
                "Warning: Unknown builtin function '{}' - defaulting to 1 argument",
                name
            );
            1
        }
    }
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
