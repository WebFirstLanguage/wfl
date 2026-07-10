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
///
/// Security: captured values are percent-decoded but otherwise untrusted —
/// they can contain `..`, `.`, or (after decoding) path separators. Callers
/// must validate captures before using them in filesystem paths, e.g. reject
/// segments containing `..` or verify the resolved path stays inside an
/// allowed base directory.
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
        // Value::Null is the runtime value of WFL's `nothing` literal, so
        // `check if params is nothing` works on a failed match.
        None => Ok(Value::Null),
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

/// Map a file name or path to an HTTP content type based on its extension.
///
/// Returns a best-guess media type for common static-web assets (fonts,
/// images, and text formats). Unknown or extension-less names fall back to
/// `application/octet-stream`, the safe generic binary type. Matching is
/// case-insensitive and only the final `.ext` is considered.
fn content_type_for_extension(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
        // Fonts
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        // Images
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        // Text / web
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "xml" => "application/xml",
        "txt" | "text" => "text/plain; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "md" => "text/markdown; charset=utf-8",
        // Documents / binary
        "pdf" => "application/pdf",
        "wasm" => "application/wasm",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        _ => "application/octet-stream",
    }
}

/// mime_type(name) -> text content type for a file name or path.
/// Usage: mime_type of "Alegreya-Regular.ttf" -> "font/ttf"
pub fn native_mime_type(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("mime_type", &args, 1)?;
    let name = expect_text(&args[0])?;

    // Take the basename, then the substring after the final '.'. An empty or
    // leading-dot-only name (e.g. ".gitignore") has no usable extension.
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    let ext = match base.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => ext,
        _ => "",
    };

    Ok(Value::Text(Arc::from(content_type_for_extension(ext))))
}

/// Extract the `boundary=` parameter from a Content-Type header value.
/// Accepts both `multipart/form-data; boundary=...` and a bare boundary string.
fn extract_multipart_boundary(content_type: &str) -> Option<String> {
    let trimmed = content_type.trim();
    // Bare boundary (no semicolon / no type) is accepted for convenience in tests
    if !trimmed.contains(';') && !trimmed.contains('/') {
        if trimmed.is_empty() {
            return None;
        }
        return Some(trimmed.to_string());
    }
    for part in trimmed.split(';').skip(1) {
        let part = part.trim();
        if let Some((key, value)) = part.split_once('=')
            && key.trim().eq_ignore_ascii_case("boundary")
        {
            let value = value.trim().trim_matches('"');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Parse a single multipart Content-Disposition header for name and optional filename.
///
/// Parameters may contain `=` inside the value (e.g. `filename="report=2024.pdf"`).
/// Values are taken as everything after the first `=` of each parameter; quoted
/// values keep internal `;` by scanning for the closing quote instead of splitting
/// on every semicolon.
fn parse_content_disposition(header: &str) -> (Option<String>, Option<String>) {
    let mut name = None;
    let mut filename = None;

    // Skip the disposition type (form-data) and walk `;`-separated params,
    // respecting double-quoted values so `filename="a;b=c.pdf"` stays intact.
    let mut rest = header;
    // Drop the disposition token before the first `;`
    if let Some((_, after)) = rest.split_once(';') {
        rest = after;
    } else {
        return (None, None);
    }

    while !rest.is_empty() {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }

        let (key, value, next) = match parse_disposition_param(rest) {
            Some(parsed) => parsed,
            None => break,
        };
        rest = next;

        if key.eq_ignore_ascii_case("name") {
            name = Some(value);
        } else if key.eq_ignore_ascii_case("filename") || key.eq_ignore_ascii_case("filename*") {
            // filename* can be charset'lang'value — take the raw value for now
            let value = value
                .rsplit_once('\'')
                .map(|(_, v)| v.to_string())
                .unwrap_or(value);
            filename = Some(value);
        }
    }
    (name, filename)
}

/// Parse one `key=value` (or `key="quoted value"`) parameter from a
/// Content-Disposition parameter list. Returns (key, value, remainder).
fn parse_disposition_param(input: &str) -> Option<(String, String, &str)> {
    let input = input.trim_start();
    let eq = input.find('=')?;
    let key = input[..eq].trim().to_string();
    if key.is_empty() {
        return None;
    }
    let after_eq = input[eq + 1..].trim_start();

    let (value, remainder) = if let Some(rest) = after_eq.strip_prefix('"') {
        // Quoted value: scan to the next unescaped `"`
        let mut end = None;
        let mut i = 0;
        let bytes = rest.as_bytes();
        while i < bytes.len() {
            if bytes[i] == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                end = Some(i);
                break;
            }
            i += 1;
        }
        let end = end?;
        // Unescape simple \" and \\ sequences in the quoted value
        let raw = &rest[..end];
        let mut unescaped = String::with_capacity(raw.len());
        let mut chars = raw.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(next) = chars.next() {
                    unescaped.push(next);
                }
            } else {
                unescaped.push(c);
            }
        }
        let after_quote = &rest[end + 1..];
        let remainder = after_quote.strip_prefix(';').unwrap_or(after_quote);
        (unescaped, remainder)
    } else {
        // Token value: ends at next `;` or end of string
        if let Some((val, after)) = after_eq.split_once(';') {
            (val.trim().to_string(), after)
        } else {
            (after_eq.trim().to_string(), "")
        }
    };

    Some((key, value, remainder))
}

/// Minimal multipart/form-data parser.
///
/// Splits the body on the boundary delimiter and returns each part's headers
/// and raw content bytes. Tolerates both CRLF and LF line endings.
fn parse_multipart_body(body: &[u8], boundary: &str) -> Result<Vec<Value>, RuntimeError> {
    // Boundary markers: --boundary  and  --boundary--
    let delim = format!("--{boundary}");
    let delim_bytes = delim.as_bytes();
    let close_delim = format!("--{boundary}--");
    let close_bytes = close_delim.as_bytes();

    // Find all boundary occurrences
    let mut parts: Vec<Value> = Vec::new();
    let mut search_from = 0;

    // Locate the first boundary
    let Some(mut pos) = find_bytes(body, delim_bytes, search_from) else {
        return Err(RuntimeError::new(
            "parse_multipart: no boundary found in body".to_string(),
            0,
            0,
        ));
    };

    loop {
        // Skip past the boundary line (and optional trailing whitespace / CRLF)
        let mut content_start = pos + delim_bytes.len();
        // Closing boundary ends the parse
        if body.get(pos..pos + close_bytes.len()) == Some(close_bytes) {
            break;
        }
        // Consume trailing spaces/tabs then CRLF or LF after the boundary
        while content_start < body.len()
            && (body[content_start] == b' ' || body[content_start] == b'\t')
        {
            content_start += 1;
        }
        if body.get(content_start..content_start + 2) == Some(b"\r\n") {
            content_start += 2;
        } else if body.get(content_start) == Some(&b'\n') {
            content_start += 1;
        }

        // Find the next boundary
        search_from = content_start;
        let next = match find_bytes(body, delim_bytes, search_from) {
            Some(n) => n,
            None => break,
        };

        // Part data ends just before the next boundary; strip trailing CRLF/LF.
        // Guard against empty parts (consecutive boundaries) where stripping
        // would leave content_end < content_start and panic on the slice.
        let mut content_end = next;
        if content_end >= 2 && &body[content_end - 2..content_end] == b"\r\n" {
            content_end -= 2;
        } else if content_end >= 1 && body[content_end - 1] == b'\n' {
            content_end -= 1;
        }
        if content_end < content_start {
            content_end = content_start;
        }

        let part_bytes = &body[content_start..content_end];
        // Split headers from body at the first blank line
        let (headers_bytes, body_bytes) = split_headers_and_body(part_bytes);

        let headers_text = String::from_utf8_lossy(headers_bytes);
        let mut part_name: Option<String> = None;
        let mut part_filename: Option<String> = None;
        let mut part_content_type: Option<String> = None;

        for line in headers_text.split(['\r', '\n']) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();
                if key.eq_ignore_ascii_case("Content-Disposition") {
                    let (n, f) = parse_content_disposition(value);
                    part_name = n;
                    part_filename = f;
                } else if key.eq_ignore_ascii_case("Content-Type") {
                    part_content_type = Some(value.to_string());
                }
            }
        }

        let mut part_map = HashMap::new();
        part_map.insert(
            "name".to_string(),
            match part_name {
                Some(n) => Value::Text(Arc::from(n.as_str())),
                None => Value::Null,
            },
        );
        part_map.insert(
            "filename".to_string(),
            match part_filename {
                Some(f) => Value::Text(Arc::from(f.as_str())),
                None => Value::Null,
            },
        );
        part_map.insert(
            "content_type".to_string(),
            match part_content_type {
                Some(ct) => Value::Text(Arc::from(ct.as_str())),
                None => Value::Null,
            },
        );
        // Lossy text view + lossless binary view, mirroring request body
        let content_text = String::from_utf8_lossy(body_bytes).into_owned();
        part_map.insert(
            "content".to_string(),
            Value::Text(Arc::from(content_text.as_str())),
        );
        part_map.insert(
            "content_bytes".to_string(),
            Value::Binary(Arc::from(body_bytes)),
        );

        parts.push(Value::Object(Rc::new(RefCell::new(part_map))));
        pos = next;
    }

    Ok(parts)
}

fn find_bytes(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    if needle.is_empty() || start >= haystack.len() {
        return None;
    }
    haystack[start..]
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|p| start + p)
}

fn split_headers_and_body(part: &[u8]) -> (&[u8], &[u8]) {
    // Prefer CRLF blank line, then LF blank line
    if let Some(idx) = find_bytes(part, b"\r\n\r\n", 0) {
        return (&part[..idx], &part[idx + 4..]);
    }
    if let Some(idx) = find_bytes(part, b"\n\n", 0) {
        return (&part[..idx], &part[idx + 2..]);
    }
    // No body separator — treat entire part as headers with empty body
    (part, &[])
}

/// parse_multipart(body, content_type) -> list of part objects
///
/// Usage:
///   store parts as parse_multipart of body_bytes and content_type
///   // or with the Content-Type header value:
///   store parts as parse_multipart of body and header "Content-Type" of req
///
/// Each part is an object with:
///   name, filename (or nothing), content_type (or nothing),
///   content (text, lossy UTF-8), content_bytes (binary)
pub fn native_parse_multipart(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("parse_multipart", &args, 2)?;

    let body_bytes: Vec<u8> = match &args[0] {
        Value::Binary(b) => b.to_vec(),
        Value::Text(t) => t.as_bytes().to_vec(),
        other => {
            return Err(RuntimeError::new(
                format!(
                    "parse_multipart expects text or binary body, got {}",
                    other.type_name()
                ),
                0,
                0,
            ));
        }
    };
    let content_type = expect_text(&args[1])?;

    let boundary = extract_multipart_boundary(&content_type).ok_or_else(|| {
        RuntimeError::new(
            format!("parse_multipart: could not find boundary in Content-Type '{content_type}'"),
            0,
            0,
        )
    })?;

    let parts = parse_multipart_body(&body_bytes, &boundary)?;
    Ok(Value::List(Rc::new(RefCell::new(parts))))
}

pub fn register_web(env: &mut Environment) {
    env.define_native("path_params", native_path_params);
    env.define_native("path_matches", native_path_matches);
    env.define_native("mime_type", native_mime_type);
    env.define_native("parse_multipart", native_parse_multipart);
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
    fn extract_boundary_from_content_type() {
        assert_eq!(
            extract_multipart_boundary("multipart/form-data; boundary=----WebKitFormBoundary"),
            Some("----WebKitFormBoundary".to_string())
        );
        assert_eq!(
            extract_multipart_boundary("multipart/form-data; boundary=\"abc123\""),
            Some("abc123".to_string())
        );
        assert_eq!(
            extract_multipart_boundary("abc123"),
            Some("abc123".to_string())
        );
        assert!(extract_multipart_boundary("application/json").is_none());
    }

    #[test]
    fn parse_multipart_text_and_file_parts() {
        let body = "------bound\r\n\
Content-Disposition: form-data; name=\"title\"\r\n\
\r\n\
Hello\r\n\
------bound\r\n\
Content-Disposition: form-data; name=\"file\"; filename=\"hi.txt\"\r\n\
Content-Type: text/plain\r\n\
\r\n\
file body\r\n\
------bound--\r\n";

        let result = native_parse_multipart(vec![
            Value::Text(Arc::from(body)),
            Value::Text(Arc::from("multipart/form-data; boundary=----bound")),
        ])
        .expect("parse_multipart should succeed");

        match result {
            Value::List(list) => {
                let parts = list.borrow();
                assert_eq!(parts.len(), 2);

                match &parts[0] {
                    Value::Object(obj) => {
                        let m = obj.borrow();
                        assert!(
                            matches!(m.get("name"), Some(Value::Text(t)) if t.as_ref() == "title")
                        );
                        assert!(
                            matches!(m.get("content"), Some(Value::Text(t)) if t.as_ref() == "Hello")
                        );
                        assert!(matches!(m.get("filename"), Some(Value::Null)));
                    }
                    other => panic!("expected object part, got {other:?}"),
                }

                match &parts[1] {
                    Value::Object(obj) => {
                        let m = obj.borrow();
                        assert!(
                            matches!(m.get("name"), Some(Value::Text(t)) if t.as_ref() == "file")
                        );
                        assert!(
                            matches!(m.get("filename"), Some(Value::Text(t)) if t.as_ref() == "hi.txt")
                        );
                        assert!(
                            matches!(m.get("content_type"), Some(Value::Text(t)) if t.as_ref() == "text/plain")
                        );
                        assert!(
                            matches!(m.get("content"), Some(Value::Text(t)) if t.as_ref() == "file body")
                        );
                        assert!(matches!(m.get("content_bytes"), Some(Value::Binary(_))));
                    }
                    other => panic!("expected object part, got {other:?}"),
                }
            }
            other => panic!("expected list, got {other:?}"),
        }
    }

    #[test]
    fn parse_multipart_empty_parts_do_not_panic() {
        // Consecutive boundaries (empty part) used to panic when CRLF stripping
        // left content_end < content_start. Must return without crashing.
        let body = b"--bound\r\n--bound\r\n--bound--\r\n";
        let result = parse_multipart_body(body, "bound");
        assert!(
            result.is_ok(),
            "empty parts between boundaries must not panic: {result:?}"
        );
    }

    #[test]
    fn content_disposition_preserves_equals_in_filename() {
        let (name, filename) =
            parse_content_disposition(r#"form-data; name="file"; filename="report=2024.pdf""#);
        assert_eq!(name.as_deref(), Some("file"));
        assert_eq!(
            filename.as_deref(),
            Some("report=2024.pdf"),
            "filename values may contain '='"
        );
    }

    #[test]
    fn content_disposition_preserves_semicolon_in_quoted_filename() {
        let (name, filename) =
            parse_content_disposition(r#"form-data; name="file"; filename="a;b=c.pdf""#);
        assert_eq!(name.as_deref(), Some("file"));
        assert_eq!(
            filename.as_deref(),
            Some("a;b=c.pdf"),
            "quoted filenames may contain ';' and '='"
        );
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

    fn mime(name: &str) -> String {
        match native_mime_type(vec![Value::Text(Arc::from(name))]).unwrap() {
            Value::Text(t) => t.to_string(),
            other => panic!("expected text, got {other:?}"),
        }
    }

    #[test]
    fn mime_type_known_fonts_and_images() {
        assert_eq!(mime("Alegreya-Regular.ttf"), "font/ttf");
        assert_eq!(mime("font.woff2"), "font/woff2");
        assert_eq!(mime("logo.png"), "image/png");
        assert_eq!(mime("photo.jpeg"), "image/jpeg");
        assert_eq!(mime("icon.svg"), "image/svg+xml");
        assert_eq!(mime("favicon.ico"), "image/x-icon");
    }

    #[test]
    fn mime_type_is_case_insensitive() {
        assert_eq!(mime("STYLE.CSS"), "text/css; charset=utf-8");
        assert_eq!(mime("Photo.JPG"), "image/jpeg");
    }

    #[test]
    fn mime_type_uses_final_extension_and_basename() {
        assert_eq!(mime("archive.tar.gz"), "application/gzip");
        assert_eq!(mime("/var/www/public/assets/fonts/x.ttf"), "font/ttf");
    }

    #[test]
    fn mime_type_unknown_and_extensionless_fall_back() {
        assert_eq!(mime("data.unknownext"), "application/octet-stream");
        assert_eq!(mime("README"), "application/octet-stream");
        assert_eq!(mime(".gitignore"), "application/octet-stream");
    }
}
