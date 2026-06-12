use super::helpers::{check_arg_count, expect_text};
use super::text::percent_decode;
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Match a request path against a route template like "/users/:id".
///
/// Template segments:
/// - `:name` captures a single path segment (percent-decoded)
/// - `*name` as the final segment captures the rest of the path (at least one segment)
/// - anything else must match the path segment literally
///
/// The query string portion of `path` is ignored. Empty segments from leading,
/// trailing, or doubled slashes are skipped on both sides.
///
/// Returns `Some(captures)` on a match (empty map for parameterless templates),
/// `None` otherwise.
fn match_path_template(path: &str, template: &str) -> Option<HashMap<String, Value>> {
    let path = path.split(['?', '#']).next().unwrap_or("");

    let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let template_segments: Vec<&str> = template.split('/').filter(|s| !s.is_empty()).collect();

    let mut captures = HashMap::new();

    for (i, tmpl_seg) in template_segments.iter().enumerate() {
        if let Some(name) = tmpl_seg.strip_prefix('*') {
            if i != template_segments.len() - 1 {
                // Wildcard is only meaningful as the final segment
                return None;
            }
            if i >= path_segments.len() {
                return None;
            }
            let rest = path_segments[i..]
                .iter()
                .map(|s| percent_decode(s).into_owned())
                .collect::<Vec<_>>()
                .join("/");
            captures.insert(name.to_string(), Value::Text(Arc::from(rest.as_str())));
            return Some(captures);
        }

        let path_seg = path_segments.get(i)?;

        if let Some(name) = tmpl_seg.strip_prefix(':') {
            let decoded = percent_decode(path_seg);
            captures.insert(name.to_string(), Value::Text(Arc::from(decoded.as_ref())));
        } else if tmpl_seg != path_seg {
            return None;
        }
    }

    if path_segments.len() != template_segments.len() {
        return None;
    }

    Some(captures)
}

/// path_params(path, template) -> Object of captures, or nothing when the path
/// does not match the template.
/// Usage: path_params of "/users/42" and "/users/:id" -> {"id": "42"}
pub fn native_path_params(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("path_params", &args, 2)?;
    let path = expect_text(&args[0])?;
    let template = expect_text(&args[1])?;

    match match_path_template(&path, &template) {
        Some(captures) => Ok(Value::Object(Rc::new(RefCell::new(captures)))),
        None => Ok(Value::Nothing),
    }
}

/// path_matches(path, template) -> boolean
/// Usage: path_matches of "/users/42" and "/users/:id" -> yes
pub fn native_path_matches(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("path_matches", &args, 2)?;
    let path = expect_text(&args[0])?;
    let template = expect_text(&args[1])?;

    Ok(Value::Bool(match_path_template(&path, &template).is_some()))
}

pub fn register_web(env: &mut Environment) {
    env.define_native("path_params", native_path_params);
    env.define_native("path_matches", native_path_matches);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_exact_path() {
        let captures = match_path_template("/about", "/about").unwrap();
        assert!(captures.is_empty());
    }

    #[test]
    fn captures_single_segment() {
        let captures = match_path_template("/users/42", "/users/:id").unwrap();
        assert!(matches!(captures.get("id"), Some(Value::Text(t)) if t.as_ref() == "42"));
    }

    #[test]
    fn rejects_segment_count_mismatch() {
        assert!(match_path_template("/users", "/users/:id").is_none());
        assert!(match_path_template("/users/1/2", "/users/:id").is_none());
    }

    #[test]
    fn ignores_query_string_and_fragment() {
        let captures = match_path_template("/users/42?page=2#top", "/users/:id").unwrap();
        assert!(matches!(captures.get("id"), Some(Value::Text(t)) if t.as_ref() == "42"));
    }

    #[test]
    fn percent_decodes_captured_segments() {
        let captures = match_path_template("/users/John%20Doe", "/users/:name").unwrap();
        assert!(matches!(captures.get("name"), Some(Value::Text(t)) if t.as_ref() == "John Doe"));
    }

    #[test]
    fn wildcard_captures_remaining_segments() {
        let captures = match_path_template("/static/css/main.css", "/static/*filepath").unwrap();
        assert!(
            matches!(captures.get("filepath"), Some(Value::Text(t)) if t.as_ref() == "css/main.css")
        );
    }

    #[test]
    fn wildcard_requires_at_least_one_segment() {
        assert!(match_path_template("/static", "/static/*filepath").is_none());
    }

    #[test]
    fn wildcard_must_be_final_segment() {
        assert!(match_path_template("/a/b/c", "/a/*rest/c").is_none());
    }
}
