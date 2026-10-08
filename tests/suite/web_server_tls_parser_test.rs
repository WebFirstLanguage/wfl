// Parser tests for HTTPS/TLS listen syntax:
//   listen on port 8443 secured with certificate "cert.pem" and key "key.pem" as server
//   listen on port 8443 secured as server                      (paths from .wflcfg)
//   listen on port 8080 redirecting to port 8443 as server     (native 301 redirect)
//   listen on port 8080 as server                              (plain HTTP, unchanged)
//
// `secured`, `certificate`, `key`, and `redirecting` are contextual identifiers,
// NOT reserved keywords — existing programs using them as variable names (e.g.
// `store key as "secret"` in TestPrograms/hash_security_test.wfl) must keep
// working. Because the lexer merges adjacent identifiers into one token, the
// parser must also handle merged forms like Identifier("my_port secured") and
// Identifier("certificate cert_path").

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Expression, Literal, Statement};

fn parse_listen(code: &str) -> Statement {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    let program = parser
        .parse()
        .unwrap_or_else(|e| panic!("Failed to parse {code:?}: {e:?}"));
    program
        .statements
        .into_iter()
        .find(|s| matches!(s, Statement::ListenStatement { .. }))
        .expect("No ListenStatement found")
}

fn parse_should_fail(code: &str) {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    assert!(parser.parse().is_err(), "Expected parse error for {code:?}");
}

fn assert_string(expr: &Expression, expected: &str, what: &str) {
    match expr {
        Expression::Literal(Literal::String(s), ..) => {
            assert_eq!(s.as_ref(), expected, "{what} literal mismatch")
        }
        other => panic!("{what} should be a string literal, got {other:?}"),
    }
}

fn assert_integer(expr: &Expression, expected: i64, what: &str) {
    match expr {
        Expression::Literal(Literal::Integer(n), ..) => {
            assert_eq!(*n, expected, "{what} literal mismatch")
        }
        other => panic!("{what} should be an integer literal, got {other:?}"),
    }
}

fn assert_variable(expr: &Expression, expected: &str, what: &str) {
    match expr {
        Expression::Variable(name, ..) => {
            assert_eq!(name, expected, "{what} variable name mismatch")
        }
        other => panic!("{what} should be a variable, got {other:?}"),
    }
}

#[test]
fn test_plain_listen_unchanged() {
    let stmt = parse_listen("listen on port 8080 as web_server");
    match stmt {
        Statement::ListenStatement {
            server_name,
            tls,
            redirect_to_port,
            ..
        } => {
            assert_eq!(server_name, "web_server");
            assert!(tls.is_none(), "plain listen must not have TLS config");
            assert!(redirect_to_port.is_none());
        }
        _ => unreachable!(),
    }
}

#[test]
fn test_secured_with_certificate_and_key_literals() {
    let stmt = parse_listen(
        r#"listen on port 8443 secured with certificate "cert.pem" and key "key.pem" as secure_server"#,
    );
    match stmt {
        Statement::ListenStatement {
            port,
            server_name,
            tls,
            redirect_to_port,
            ..
        } => {
            assert_integer(&port, 8443, "port");
            assert_eq!(server_name, "secure_server");
            let tls = tls.expect("TLS config missing");
            assert_string(
                tls.cert_path.as_ref().expect("cert_path missing"),
                "cert.pem",
                "cert_path",
            );
            assert_string(
                tls.key_path.as_ref().expect("key_path missing"),
                "key.pem",
                "key_path",
            );
            assert!(redirect_to_port.is_none());
        }
        _ => unreachable!(),
    }
}

#[test]
fn test_bare_secured_uses_config_paths() {
    let stmt = parse_listen("listen on port 8443 secured as secure_server");
    match stmt {
        Statement::ListenStatement {
            server_name, tls, ..
        } => {
            assert_eq!(server_name, "secure_server");
            let tls = tls.expect("TLS config missing");
            assert!(tls.cert_path.is_none(), "bare secured has no cert path");
            assert!(tls.key_path.is_none(), "bare secured has no key path");
        }
        _ => unreachable!(),
    }
}

#[test]
fn test_secured_with_variable_paths_merged_identifiers() {
    // `certificate cert_path` and `key key_path` lex as single merged
    // identifiers; the parser must split the marker word off.
    let stmt = parse_listen(
        r#"
store cert_path as "cert.pem"
store key_path as "key.pem"
listen on port 8443 secured with certificate cert_path and key key_path as secure_server
"#,
    );
    match stmt {
        Statement::ListenStatement { tls, .. } => {
            let tls = tls.expect("TLS config missing");
            assert_variable(
                tls.cert_path.as_ref().expect("cert_path missing"),
                "cert_path",
                "cert_path",
            );
            assert_variable(
                tls.key_path.as_ref().expect("key_path missing"),
                "key_path",
                "key_path",
            );
        }
        _ => unreachable!(),
    }
}

#[test]
fn test_secured_with_variable_port_merged_identifier() {
    // `my_port secured` lexes as one merged identifier; the parser must split
    // the trailing marker off the port variable.
    let stmt = parse_listen(
        r#"
store my_port as 8443
listen on port my_port secured with certificate "cert.pem" and key "key.pem" as secure_server
"#,
    );
    match stmt {
        Statement::ListenStatement { port, tls, .. } => {
            assert_variable(&port, "my_port", "port");
            assert!(tls.is_some(), "TLS config missing");
        }
        _ => unreachable!(),
    }
}

#[test]
fn test_bare_secured_with_variable_port() {
    let stmt = parse_listen(
        r#"
store my_port as 8443
listen on port my_port secured as secure_server
"#,
    );
    match stmt {
        Statement::ListenStatement { port, tls, .. } => {
            assert_variable(&port, "my_port", "port");
            let tls = tls.expect("TLS config missing");
            assert!(tls.cert_path.is_none());
            assert!(tls.key_path.is_none());
        }
        _ => unreachable!(),
    }
}

#[test]
fn test_redirecting_to_port() {
    let stmt = parse_listen("listen on port 8080 redirecting to port 8443 as redirect_server");
    match stmt {
        Statement::ListenStatement {
            port,
            server_name,
            tls,
            redirect_to_port,
            ..
        } => {
            assert_integer(&port, 8080, "port");
            assert_eq!(server_name, "redirect_server");
            assert!(tls.is_none());
            assert_integer(
                &redirect_to_port.expect("redirect target missing"),
                8443,
                "redirect target port",
            );
        }
        _ => unreachable!(),
    }
}

#[test]
fn test_redirecting_with_variable_port_merged_identifier() {
    let stmt = parse_listen(
        r#"
store http_port as 8080
store https_port as 8443
listen on port http_port redirecting to port https_port as redirect_server
"#,
    );
    match stmt {
        Statement::ListenStatement {
            port,
            redirect_to_port,
            ..
        } => {
            assert_variable(&port, "http_port", "port");
            assert_variable(
                &redirect_to_port.expect("redirect target missing"),
                "https_port",
                "redirect target port",
            );
        }
        _ => unreachable!(),
    }
}

#[test]
fn test_key_still_usable_as_variable_name() {
    // Backward compatibility: `key` must remain a valid identifier.
    let code = r#"
store key as "secret_key_456"
display key
"#;
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    parser
        .parse()
        .unwrap_or_else(|e| panic!("'key' as a variable must keep parsing: {e:?}"));
}

#[test]
fn test_secured_and_certificate_still_usable_as_variable_names() {
    let code = r#"
store secured as "yes"
store certificate as "mycert"
display secured
display certificate
"#;
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    parser.parse().unwrap_or_else(|e| {
        panic!("'secured'/'certificate' as variables must keep parsing: {e:?}")
    });
}

#[test]
fn test_error_missing_key_clause() {
    parse_should_fail(r#"listen on port 8443 secured with certificate "cert.pem" as s"#);
}

#[test]
fn test_error_key_without_certificate() {
    parse_should_fail(r#"listen on port 8443 secured with key "key.pem" as s"#);
}

#[test]
fn test_error_redirecting_without_port_keyword() {
    parse_should_fail("listen on port 8080 redirecting to 8443 as s");
}

#[test]
fn test_error_secured_and_redirecting_combined() {
    parse_should_fail(
        r#"listen on port 8080 secured with certificate "c.pem" and key "k.pem" redirecting to port 8443 as s"#,
    );
}
