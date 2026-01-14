//! HTTP Header Security Validation
//!
//! Provides validation for HTTP headers to prevent security vulnerabilities
//! including header injection attacks and RFC 7230 compliance.

use std::collections::HashSet;
use std::sync::LazyLock;

/// Maximum allowed length for a single header value
const MAX_HEADER_VALUE_LENGTH: usize = 8192;

/// Maximum allowed length for a header name
const MAX_HEADER_NAME_LENGTH: usize = 256;

/// Headers that should not be set by user code as they could cause security issues
/// or break HTTP protocol handling
static FORBIDDEN_HEADERS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut set = HashSet::new();
    // Headers that control connection/framing - setting these could break HTTP
    set.insert("host");
    set.insert("connection");
    set.insert("content-length");
    set.insert("transfer-encoding");
    set.insert("te");
    set.insert("trailer");
    set.insert("upgrade");
    set.insert("keep-alive");
    // Headers that could be used for request smuggling
    set.insert("proxy-connection");
    // Content-Type is handled separately by WFL
    set.insert("content-type");
    set
});

/// Result of header validation
#[derive(Debug)]
pub enum HeaderValidationError {
    /// Header name contains invalid characters (not RFC 7230 token)
    InvalidHeaderName { name: String, reason: String },
    /// Header value contains CRLF or other forbidden characters
    InvalidHeaderValue { name: String, reason: String },
    /// Header name is forbidden for security reasons
    ForbiddenHeader { name: String },
    /// Header name is too long
    HeaderNameTooLong { name: String, length: usize },
    /// Header value is too long
    HeaderValueTooLong { name: String, length: usize },
}

impl std::fmt::Display for HeaderValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HeaderValidationError::InvalidHeaderName { name, reason } => {
                write!(f, "Invalid header name '{}': {}", name, reason)
            }
            HeaderValidationError::InvalidHeaderValue { name, reason } => {
                write!(f, "Invalid value for header '{}': {}", name, reason)
            }
            HeaderValidationError::ForbiddenHeader { name } => {
                write!(
                    f,
                    "Header '{}' is forbidden - it is managed by the HTTP protocol layer",
                    name
                )
            }
            HeaderValidationError::HeaderNameTooLong { name, length } => {
                write!(
                    f,
                    "Header name '{}' is too long ({} chars, max {})",
                    name, length, MAX_HEADER_NAME_LENGTH
                )
            }
            HeaderValidationError::HeaderValueTooLong { name, length } => {
                write!(
                    f,
                    "Value for header '{}' is too long ({} chars, max {})",
                    name, length, MAX_HEADER_VALUE_LENGTH
                )
            }
        }
    }
}

/// Check if a character is a valid HTTP token character per RFC 7230
/// token = 1*tchar
/// tchar = "!" / "#" / "$" / "%" / "&" / "'" / "*" / "+" / "-" / "." /
///         "^" / "_" / "`" / "|" / "~" / DIGIT / ALPHA
fn is_token_char(c: char) -> bool {
    matches!(c,
        '!' | '#' | '$' | '%' | '&' | '\'' | '*' | '+' | '-' | '.' |
        '^' | '_' | '`' | '|' | '~' |
        '0'..='9' | 'a'..='z' | 'A'..='Z'
    )
}

/// Validate an HTTP header name per RFC 7230
pub fn validate_header_name(name: &str) -> Result<(), HeaderValidationError> {
    // Check length
    if name.len() > MAX_HEADER_NAME_LENGTH {
        return Err(HeaderValidationError::HeaderNameTooLong {
            name: name.to_string(),
            length: name.len(),
        });
    }

    // Check for empty name
    if name.is_empty() {
        return Err(HeaderValidationError::InvalidHeaderName {
            name: name.to_string(),
            reason: "header name cannot be empty".to_string(),
        });
    }

    // Check all characters are valid token characters
    for c in name.chars() {
        if !is_token_char(c) {
            return Err(HeaderValidationError::InvalidHeaderName {
                name: name.to_string(),
                reason: format!("contains invalid character '{}'", c),
            });
        }
    }

    // Check if header is forbidden
    if FORBIDDEN_HEADERS.contains(name.to_lowercase().as_str()) {
        return Err(HeaderValidationError::ForbiddenHeader {
            name: name.to_string(),
        });
    }

    Ok(())
}

/// Validate an HTTP header value
/// - Prevents CRLF injection
/// - Checks for control characters
/// - Enforces length limits
pub fn validate_header_value(name: &str, value: &str) -> Result<(), HeaderValidationError> {
    // Check length
    if value.len() > MAX_HEADER_VALUE_LENGTH {
        return Err(HeaderValidationError::HeaderValueTooLong {
            name: name.to_string(),
            length: value.len(),
        });
    }

    // Check for CRLF injection and control characters
    for c in value.chars() {
        // Reject carriage return and line feed (CRLF injection prevention)
        if c == '\r' || c == '\n' {
            return Err(HeaderValidationError::InvalidHeaderValue {
                name: name.to_string(),
                reason: "contains line break characters (potential header injection)".to_string(),
            });
        }

        // Reject NUL character
        if c == '\0' {
            return Err(HeaderValidationError::InvalidHeaderValue {
                name: name.to_string(),
                reason: "contains null character".to_string(),
            });
        }

        // Reject other ASCII control characters (0x00-0x1F except HT 0x09, and DEL 0x7F)
        // HTTP allows HTAB (0x09) in header values
        if c != '\t' && c.is_ascii_control() {
            return Err(HeaderValidationError::InvalidHeaderValue {
                name: name.to_string(),
                reason: format!("contains control character (0x{:02X})", c as u8),
            });
        }
    }

    Ok(())
}

/// Validate a complete header (name and value)
pub fn validate_header(name: &str, value: &str) -> Result<(), HeaderValidationError> {
    validate_header_name(name)?;
    validate_header_value(name, value)?;
    Ok(())
}

/// Sanitize a header value by removing problematic characters
/// Returns the sanitized value
pub fn sanitize_header_value(value: &str) -> String {
    value
        .chars()
        .filter(|&c| {
            // Keep printable ASCII and HTAB, reject CR, LF, NUL, and other control chars
            c == '\t' || (c >= ' ' && c != '\x7F')
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_header_names() {
        assert!(validate_header_name("X-Custom-Header").is_ok());
        assert!(validate_header_name("X-Request-ID").is_ok());
        assert!(validate_header_name("Cache-Control").is_ok());
        assert!(validate_header_name("x-lowercase").is_ok());
        assert!(validate_header_name("X_Underscore").is_ok());
    }

    #[test]
    fn test_invalid_header_names() {
        // Empty
        assert!(validate_header_name("").is_err());
        // Contains space
        assert!(validate_header_name("X Header").is_err());
        // Contains colon
        assert!(validate_header_name("X:Header").is_err());
        // Contains CRLF
        assert!(validate_header_name("X\r\nHeader").is_err());
    }

    #[test]
    fn test_forbidden_headers() {
        assert!(matches!(
            validate_header_name("Host"),
            Err(HeaderValidationError::ForbiddenHeader { .. })
        ));
        assert!(matches!(
            validate_header_name("host"),
            Err(HeaderValidationError::ForbiddenHeader { .. })
        ));
        assert!(matches!(
            validate_header_name("Content-Length"),
            Err(HeaderValidationError::ForbiddenHeader { .. })
        ));
        assert!(matches!(
            validate_header_name("Transfer-Encoding"),
            Err(HeaderValidationError::ForbiddenHeader { .. })
        ));
    }

    #[test]
    fn test_valid_header_values() {
        assert!(validate_header_value("X-Test", "simple value").is_ok());
        assert!(validate_header_value("X-Test", "value with\ttab").is_ok());
        assert!(validate_header_value("X-Test", "UTF-8: こんにちは").is_ok());
        assert!(validate_header_value("X-Test", "").is_ok()); // Empty is valid
    }

    #[test]
    fn test_crlf_injection_prevention() {
        // CRLF injection attempts
        assert!(validate_header_value("X-Test", "value\r\nEvil-Header: injected").is_err());
        assert!(validate_header_value("X-Test", "value\rEvil").is_err());
        assert!(validate_header_value("X-Test", "value\nEvil").is_err());
    }

    #[test]
    fn test_control_character_rejection() {
        assert!(validate_header_value("X-Test", "value\x00null").is_err());
        assert!(validate_header_value("X-Test", "value\x01control").is_err());
        assert!(validate_header_value("X-Test", "value\x7Fdel").is_err());
    }

    #[test]
    fn test_header_length_limits() {
        let long_name = "X".repeat(MAX_HEADER_NAME_LENGTH + 1);
        assert!(matches!(
            validate_header_name(&long_name),
            Err(HeaderValidationError::HeaderNameTooLong { .. })
        ));

        let long_value = "x".repeat(MAX_HEADER_VALUE_LENGTH + 1);
        assert!(matches!(
            validate_header_value("X-Test", &long_value),
            Err(HeaderValidationError::HeaderValueTooLong { .. })
        ));
    }

    #[test]
    fn test_sanitize_header_value() {
        assert_eq!(sanitize_header_value("normal value"), "normal value");
        assert_eq!(sanitize_header_value("with\ttab"), "with\ttab");
        assert_eq!(sanitize_header_value("no\r\nnewlines"), "nonewlines");
        assert_eq!(sanitize_header_value("no\x00null"), "nonull");
    }
}
