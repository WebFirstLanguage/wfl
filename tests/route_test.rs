//! End-to-end tests for the `route` construct.
//!
//! `route` is parser-level sugar that lowers to a `check if` chain, so these
//! tests drive the *full* pipeline (lex → parse → analyze → typecheck →
//! interpret) through the compiled binary and assert on observable output.

// Shared harness: `get_wfl_binary_path`, `get_unique_test_file_path`, and
// `run_wfl_program` (which already runs the binary under a 30s timeout, so a
// bug that hangs the interpreter fails fast instead of stalling CI).
mod test_helpers;
use test_helpers::run_wfl_program;

/// Run a program, assert it exited cleanly, and return trimmed stdout lines.
fn run_ok(program: &str, test_name: &str) -> Vec<String> {
    let output = run_wfl_program(program, test_name);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "program should exit 0.\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
    );
    stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

#[test]
fn route_equality_selects_matching_arm() {
    let program = r#"
store path as "/health"
route path:
    when "/":
        display "home"
    when "/health":
        display "ok"
    otherwise:
        display "not found"
end route
"#;
    let lines = run_ok(program, "route_equality");
    assert_eq!(lines, vec!["ok"]);
}

#[test]
fn route_or_list_matches_any_listed_value() {
    let program = r#"
store code as 410
route code:
    when 200:
        display "OK"
    when 404 or 410:
        display "Gone"
    otherwise:
        display "Other"
end route
"#;
    let lines = run_ok(program, "route_or_list");
    assert_eq!(lines, vec!["Gone"]);
}

#[test]
fn route_falls_through_to_otherwise() {
    let program = r#"
store path as "/nope"
route path:
    when "/":
        display "home"
    otherwise:
        display "not found"
end route
"#;
    let lines = run_ok(program, "route_otherwise");
    assert_eq!(lines, vec!["not found"]);
}

#[test]
fn route_starts_with_ends_with_and_contains() {
    // starts with / ends with / contains all run under the full pipeline,
    // not just `--analyze` (the motivating issue #566 for these operators).
    let program = r#"
store results as []
store api as "/api/users"
route api:
    when starts with "/api/":
        display "api-route"
    otherwise:
        display "static"
end route

store css as "/assets/site.css"
route css:
    when ends with ".css":
        display "stylesheet"
    otherwise:
        display "other"
end route

store admin as "/admin/panel"
route admin:
    when contains "admin":
        display "restricted"
    otherwise:
        display "public"
end route
"#;
    let lines = run_ok(program, "route_text_ops");
    assert_eq!(lines, vec!["api-route", "stylesheet", "restricted"]);
}

#[test]
fn route_one_of_tests_list_membership() {
    let program = r#"
store assets as ["/a.js", "/b.js", "/c.css"]
store name as "/b.js"
route name:
    when one of assets:
        display "known"
    otherwise:
        display "unknown"
end route
"#;
    let lines = run_ok(program, "route_one_of");
    assert_eq!(lines, vec!["known"]);
}

#[test]
fn route_arm_precedence_first_match_wins() {
    // Both arms would match "/api/style.css"; the first listed arm must win,
    // exactly like an `otherwise check if` chain.
    let program = r#"
store p as "/api/style.css"
route p:
    when starts with "/api/":
        display "api"
    when ends with ".css":
        display "css"
    otherwise:
        display "other"
end route
"#;
    let lines = run_ok(program, "route_precedence");
    assert_eq!(lines, vec!["api"]);
}

#[test]
fn route_with_no_match_and_no_otherwise_is_noop() {
    let program = r#"
store x as 99
display "before"
route x:
    when 1:
        display "one"
    when 2:
        display "two"
end route
display "after"
"#;
    let lines = run_ok(program, "route_noop");
    assert_eq!(lines, vec!["before", "after"]);
}

#[test]
fn route_multi_statement_arm_body_runs_in_order() {
    let program = r#"
store x as 1
route x:
    when 1:
        display "a"
        display "b"
        display "c"
    otherwise:
        display "z"
end route
"#;
    let lines = run_ok(program, "route_multi_stmt");
    assert_eq!(lines, vec!["a", "b", "c"]);
}

#[test]
fn route_otherwise_before_when_is_rejected() {
    let program = r#"
store x as 1
route x:
    otherwise:
        display "d"
    when 1:
        display "one"
end route
"#;
    let output = run_wfl_program(program, "route_bad_order");
    assert!(
        !output.status.success(),
        "a route with `otherwise` before a `when` arm must fail to parse"
    );
}
