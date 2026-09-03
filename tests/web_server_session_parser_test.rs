// TDD parser tests for issue #555 session statements.
//
// Session markers (`sessions`, `session`, `configure`, `enable`, …) are
// contextual identifiers, not reserved keywords. Existing programs that use
// those words as variable names must keep parsing.

use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;
use wfl::parser::ast::{Expression, Literal, Statement};

fn parse_program(code: &str) -> Vec<Statement> {
    let tokens = lex_wfl_with_positions(code);
    let mut parser = Parser::new(&tokens);
    parser
        .parse()
        .unwrap_or_else(|e| panic!("Failed to parse {code:?}: {e:?}"))
        .statements
}

fn parse_one(code: &str) -> Statement {
    let mut stmts = parse_program(code);
    assert_eq!(stmts.len(), 1, "expected one statement for {code:?}");
    stmts.pop().unwrap()
}

fn parse_store_value(code: &str) -> Expression {
    match parse_one(code) {
        Statement::VariableDeclaration { value, .. } => value,
        other => panic!("expected store statement, got {other:?}"),
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

#[test]
fn plain_listen_does_not_enable_sessions() {
    let stmt = parse_one("listen on port 8080 as web_server");
    match stmt {
        Statement::ListenStatement {
            sessions_enabled, ..
        } => {
            assert!(
                !sessions_enabled,
                "plain listen must leave sessions disabled"
            );
        }
        other => panic!("expected ListenStatement, got {other:?}"),
    }
}

#[test]
fn listen_with_sessions_enabled_sets_flag() {
    let stmt = parse_one("listen on port 8080 as web_server with sessions enabled");
    match stmt {
        Statement::ListenStatement {
            server_name,
            sessions_enabled,
            tls,
            redirect_to_port,
            ..
        } => {
            assert_eq!(server_name, "web_server");
            assert!(sessions_enabled);
            assert!(tls.is_none());
            assert!(redirect_to_port.is_none());
        }
        other => panic!("expected ListenStatement, got {other:?}"),
    }
}

#[test]
fn configure_sessions_captures_timeout_and_storage() {
    let stmt = parse_one(
        r#"configure sessions on web_server with timeout 1800000 and storage "database""#,
    );
    match stmt {
        Statement::ConfigureSessionsStatement {
            server,
            timeout,
            storage,
            ..
        } => {
            assert_variable(&server, "web_server", "server");
            assert_integer(&timeout, 1_800_000, "timeout");
            assert_string(&storage, "database", "storage");
        }
        other => panic!("expected ConfigureSessionsStatement, got {other:?}"),
    }
}

#[test]
fn enable_csrf_protection_statement() {
    let stmt = parse_one("enable csrf protection on web_server");
    match stmt {
        Statement::EnableCsrfProtectionStatement { server, .. } => {
            assert_variable(&server, "web_server", "server");
        }
        other => panic!("expected EnableCsrfProtectionStatement, got {other:?}"),
    }
}

#[test]
fn enable_secure_cookies_statement() {
    let stmt = parse_one("enable secure cookies on web_server");
    match stmt {
        Statement::EnableSecureCookiesStatement { server, .. } => {
            assert_variable(&server, "web_server", "server");
        }
        other => panic!("expected EnableSecureCookiesStatement, got {other:?}"),
    }
}

#[test]
fn create_session_expression() {
    let expr = parse_store_value("store sess as create session for req");
    match expr {
        Expression::CreateSession { request, .. } => {
            assert_variable(&request, "req", "request");
        }
        other => panic!("expected CreateSession, got {other:?}"),
    }
}

#[test]
fn get_session_expression() {
    let expr = parse_store_value("store sess as get session from req");
    match expr {
        Expression::GetSession { request, .. } => {
            assert_variable(&request, "req", "request");
        }
        other => panic!("expected GetSession, got {other:?}"),
    }
}

#[test]
fn get_session_value_expression() {
    let expr = parse_store_value(r#"store user_id as get session value "user_id" from sess"#);
    match expr {
        Expression::GetSessionValue { key, session, .. } => {
            assert_string(&key, "user_id", "key");
            assert_variable(&session, "sess", "session");
        }
        other => panic!("expected GetSessionValue, got {other:?}"),
    }
}

#[test]
fn generate_csrf_token_for_session_expression() {
    let expr = parse_store_value("store csrf as generate csrf token for sess");
    match expr {
        Expression::GenerateCsrfTokenForSession { session, .. } => {
            assert_variable(&session, "sess", "session");
        }
        other => panic!("expected GenerateCsrfTokenForSession, got {other:?}"),
    }
}

#[test]
fn set_session_value_statement() {
    let stmt = parse_one(r#"set session value "user_id" to "guest" in sess"#);
    match stmt {
        Statement::SetSessionValueStatement {
            key,
            value,
            session,
            ..
        } => {
            assert_string(&key, "user_id", "key");
            assert_string(&value, "guest", "value");
            assert_variable(&session, "sess", "session");
        }
        other => panic!("expected SetSessionValueStatement, got {other:?}"),
    }
}

#[test]
fn destroy_session_statement() {
    let stmt = parse_one("destroy session sess");
    match stmt {
        Statement::DestroySessionStatement { session, .. } => {
            assert_variable(&session, "sess", "session");
        }
        other => panic!("expected DestroySessionStatement, got {other:?}"),
    }
}

#[test]
fn find_expired_sessions_does_not_steal_find_in() {
    let expr = parse_store_value("store expired as find expired sessions on web_server");
    match expr {
        Expression::FindExpiredSessions { server, .. } => {
            assert_variable(&server, "web_server", "server");
        }
        other => panic!("expected FindExpiredSessions, got {other:?}"),
    }

    let pattern_find = parse_store_value(r#"store hits as find "a" in "abc""#);
    assert!(
        matches!(pattern_find, Expression::PatternFind { .. }),
        "find … in … must stay a pattern find, got {pattern_find:?}"
    );
}

#[test]
fn get_session_statistics_expression() {
    let expr = parse_store_value("store stats as get session statistics from web_server");
    match expr {
        Expression::GetSessionStatistics { server, .. } => {
            assert_variable(&server, "web_server", "server");
        }
        other => panic!("expected GetSessionStatistics, got {other:?}"),
    }
}

#[test]
fn store_session_data_is_not_store_as() {
    let stmt =
        parse_one(r#"store session_data to storage with key "test_session_123" and data payload"#);
    match stmt {
        Statement::StoreSessionDataStatement { key, data, .. } => {
            assert_string(&key, "test_session_123", "key");
            assert_variable(&data, "payload", "data");
        }
        other => panic!("expected StoreSessionDataStatement, got {other:?}"),
    }

    let ordinary = parse_one(r#"store name as "alice""#);
    assert!(
        matches!(ordinary, Statement::VariableDeclaration { .. }),
        "store x as y must stay a variable declaration, got {ordinary:?}"
    );
}

#[test]
fn load_session_data_does_not_steal_load_module() {
    let expr = parse_store_value(
        r#"store retrieved as load session data from storage with key "test_session_123""#,
    );
    match expr {
        Expression::LoadSessionData { key, .. } => {
            assert_string(&key, "test_session_123", "key");
        }
        other => panic!("expected LoadSessionData, got {other:?}"),
    }

    let tokens = lex_wfl_with_positions(r#"load module from "mod.wfl""#);
    let mut parser = Parser::new(&tokens);
    let program = parser.parse().expect("load module from must still parse");
    assert!(matches!(
        program.statements[0],
        Statement::LoadModuleStatement { .. }
    ));
}

#[test]
fn delete_session_data_does_not_steal_delete_file() {
    let stmt = parse_one(r#"delete session data from storage with key "test_session_123""#);
    match stmt {
        Statement::DeleteSessionDataStatement { key, .. } => {
            assert_string(&key, "test_session_123", "key");
        }
        other => panic!("expected DeleteSessionDataStatement, got {other:?}"),
    }

    let file_delete = parse_one(r#"delete file at "gone.txt""#);
    assert!(
        matches!(file_delete, Statement::DeleteFileStatement { .. }),
        "delete file at must stay a file delete, got {file_delete:?}"
    );
}

#[test]
fn respond_and_set_session_clause() {
    let stmt = parse_one(r#"respond to req with "ok" and set session sess"#);
    match stmt {
        Statement::RespondStatement {
            set_session,
            clear_session,
            ..
        } => {
            let set_session = set_session.expect("set session clause missing");
            assert_variable(&set_session, "sess", "set_session");
            assert!(!clear_session);
        }
        other => panic!("expected RespondStatement, got {other:?}"),
    }
}

#[test]
fn respond_and_clear_session_clause() {
    let stmt = parse_one(r#"respond to req with "bye" and clear session"#);
    match stmt {
        Statement::RespondStatement {
            set_session,
            clear_session,
            ..
        } => {
            assert!(set_session.is_none());
            assert!(clear_session);
        }
        other => panic!("expected RespondStatement, got {other:?}"),
    }
}

#[test]
fn respond_set_session_keeps_headers_and_status() {
    let stmt = parse_one(
        r#"respond to req with "ok" and status 200 and headers hdrs and set session sess"#,
    );
    match stmt {
        Statement::RespondStatement {
            status,
            headers,
            set_session,
            ..
        } => {
            assert!(status.is_some());
            assert!(headers.is_some());
            assert!(set_session.is_some());
        }
        other => panic!("expected RespondStatement, got {other:?}"),
    }
}

#[test]
fn session_is_still_a_legal_variable_name() {
    let stmt = parse_one(r#"store session as "active""#);
    match stmt {
        Statement::VariableDeclaration { name, .. } => {
            assert_eq!(name, "session");
        }
        other => panic!("expected VariableDeclaration, got {other:?}"),
    }
}
