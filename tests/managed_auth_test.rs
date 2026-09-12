//! Managed authentication contracts, including denial and lifecycle boundaries.
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wfl::interpreter::environment::Environment;
use wfl::interpreter::error::RuntimeError;
use wfl::interpreter::value::Value;

fn text(s: &str) -> Value { Value::Text(s.into()) }
fn num(n: f64) -> Value { Value::Number(n) }
fn call(name: &str, args: Vec<Value>) -> Result<Value, RuntimeError> {
    let env = Environment::new_global();
    wfl::stdlib::register_stdlib(&mut env.borrow_mut());
    match env.borrow().get(name) {
        Some(Value::NativeFunction(_, f)) => f(args),
        _ => panic!("managed authentication builtin {name} must be registered"),
    }
}
fn field(value: &Value, name: &str) -> Value {
    let Value::Object(obj) = value else { panic!("expected session record") };
    obj.borrow().get(name).unwrap().clone()
}
fn store(capacity: f64) -> Value {
    call("create_session_store", vec![num(3600.0), num(900.0), num(capacity)]).unwrap()
}
fn session(store: &Value, account: &str) -> Value {
    call("session_create", vec![store.clone(), text(account)]).unwrap()
}
fn lookup(store: &Value, id: &Value) -> Value {
    call("session_lookup", vec![store.clone(), id.clone()]).unwrap()
}

#[test]
fn sessions_are_isolated_and_unforgeable() {
    let store_a = store(3.0);
    let store_b = store(3.0);
    let first = session(&store_a, "alice");
    let second = session(&store_a, "alice");
    let id = field(&first, "id");
    let csrf = field(&first, "csrf_token");
    for token in [&id, &csrf] {
        let Value::Text(token) = token else { panic!("token must be text") };
        assert_eq!(token.len(), 64);
        assert!(token.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)));
    }
    assert_ne!(id, csrf);
    assert_ne!(id, field(&second, "id"));
    assert_eq!(lookup(&store_a, &id), text("alice"));
    assert_eq!(lookup(&store_b, &id), Value::Nothing);
    // Changing a returned record cannot change the store's authenticated account.
    let Value::Object(record) = first else { unreachable!() };
    record.borrow_mut().insert("account".into(), text("admin"));
    assert_eq!(lookup(&store_a, &id), text("alice"));
    assert!(!format!("{store_a:?}").contains(&id.to_string()));
    assert_eq!(lookup(&store_a.deep_clone(), &id), text("alice"));
}

#[test]
fn rotation_and_revocation_invalidate_prior_credentials() {
    let store = store(3.0);
    let old = session(&store, "alice");
    let old_id = field(&old, "id");
    let fresh = call("session_rotate", vec![store.clone(), old_id.clone()]).unwrap();
    let fresh_id = field(&fresh, "id");
    assert_ne!(old_id, fresh_id);
    assert_ne!(field(&old, "csrf_token"), field(&fresh, "csrf_token"));
    assert_eq!(lookup(&store, &old_id), Value::Nothing);
    assert_eq!(lookup(&store, &fresh_id), text("alice"));
    assert_eq!(call("session_rotate", vec![store.clone(), old_id]).unwrap(), Value::Nothing);
    assert_eq!(call("session_revoke", vec![store.clone(), fresh_id.clone()]).unwrap(), Value::Bool(true));
    assert_eq!(lookup(&store, &fresh_id), Value::Nothing);
    assert_eq!(call("session_revoke", vec![store, fresh_id]).unwrap(), Value::Bool(false));
}

#[test]
fn account_revocation_and_capacity_do_not_evict_other_accounts() {
    let store = store(3.0);
    let alice = session(&store, "alice");
    let _alice2 = session(&store, "alice");
    let bob = session(&store, "bob");
    assert!(call("session_create", vec![store.clone(), text("eve")]).is_err());
    assert_eq!(lookup(&store, &field(&bob, "id")), text("bob"));
    assert_eq!(call("session_revoke_account", vec![store.clone(), text("alice")]).unwrap(), num(2.0));
    assert_eq!(lookup(&store, &field(&alice, "id")), Value::Nothing);
    assert_eq!(lookup(&store, &field(&bob, "id")), text("bob"));
    session(&store, "new");
}

fn request(method: &str, cookie: &str, csrf: &str) -> Value {
    let headers = Value::Object(Rc::new(RefCell::new(HashMap::from([
        ("cookie".into(), text(cookie)), ("x-csrf-token".into(), text(csrf)),
    ]))));
    Value::Object(Rc::new(RefCell::new(HashMap::from([
        ("method".into(), text(method)), ("headers".into(), headers),
    ]))))
}

#[test]
fn csrf_guard_requires_bound_session_and_rejects_ambiguous_cookies() {
    let store = store(2.0);
    let first = session(&store, "alice");
    let second = session(&store, "alice");
    let id = field(&first, "id").to_string();
    let csrf = field(&first, "csrf_token").to_string();
    let cookie = format!("__Host-wfl_session={id}");
    let guard = |method: &str, cookie: &str, token: &str| {
        call("session_csrf_guard", vec![store.clone(), request(method, cookie, token)]).unwrap()
    };
    assert_eq!(guard("POST", &cookie, &csrf), Value::Bool(true));
    assert_eq!(guard("GET", &cookie, ""), Value::Bool(true));
    for method in ["POST", "PUT", "PATCH", "DELETE", "CUSTOM", "get"] {
        assert_eq!(guard(method, &cookie, ""), Value::Bool(false));
    }
    assert_eq!(guard("POST", &cookie, &field(&second, "csrf_token").to_string()), Value::Bool(false));
    assert_eq!(guard("POST", &format!("{cookie}; {cookie}"), &csrf), Value::Bool(false));
    assert_eq!(guard("GET", "", ""), Value::Bool(false));
    assert_eq!(guard("POST", &cookie, &"a".repeat(8192)), Value::Bool(false));
    assert_eq!(call("session_csrf_guard", vec![store.clone(), Value::Nothing]).unwrap(), Value::Bool(false));
    call("session_revoke", vec![store.clone(), text(&id)]).unwrap();
    assert_eq!(guard("GET", &cookie, ""), Value::Bool(false));
}

#[test]
fn cookies_have_secure_defaults_and_reject_header_injection() {
    let store = store(1.0);
    let session = session(&store, "alice");
    let cookie = call("session_cookie", vec![field(&session, "id")]).unwrap().to_string();
    assert!(cookie.starts_with("__Host-wfl_session="));
    for flag in ["Path=/", "Secure", "HttpOnly", "SameSite=Strict"] { assert!(cookie.contains(flag)); }
    assert!(!cookie.contains("Domain="));
    assert!(call("session_cookie", vec![text("a\r\nSet-Cookie: evil=1")]).is_err());
}

#[test]
fn account_limiter_counts_attempts_and_fails_closed_when_full() {
    let limiter = call("create_account_rate_limiter", vec![num(2.0), num(60.0), num(2.0)]).unwrap();
    let allow = |key: &str| call("account_rate_limit_allow", vec![limiter.clone(), text(key)]).unwrap();
    assert_eq!(allow("alice"), Value::Bool(true));
    assert_eq!(allow("alice"), Value::Bool(true));
    assert_eq!(allow("alice"), Value::Bool(false));
    assert_eq!(allow("bob"), Value::Bool(true));
    assert_eq!(allow("eve"), Value::Bool(false));
    assert_eq!(allow("alice"), Value::Bool(false));
    assert_eq!(allow("bob"), Value::Bool(true));
    assert_eq!(allow("bob"), Value::Bool(false));
}

#[test]
fn auth_policy_rejects_invalid_and_oversized_input() {
    for bad in [0.0, -1.0, 1.5, f64::NAN, f64::INFINITY, 1e30] {
        assert!(call("create_session_store", vec![num(bad), num(1.0), num(1.0)]).is_err());
        assert!(call("create_account_rate_limiter", vec![num(bad), num(1.0), num(1.0)]).is_err());
    }
    assert!(call("create_session_store", vec![num(10.0), num(11.0), num(1.0)]).is_err());
    assert!(call("create_session_store", vec![]).is_err());
    let store = store(2.0);
    assert!(call("session_create", vec![store.clone(), text("")]).is_err());
    assert!(call("session_create", vec![store.clone(), text(&"a".repeat(1025))]).is_err());
    assert_eq!(lookup(&store, &text(&"a".repeat(8192))), Value::Nothing);
    assert!(call("session_lookup", vec![text("forged store"), text("id")]).is_err());
}

#[test]
fn expired_sessions_and_limiter_windows_reclaim_capacity() {
    let store = call("create_session_store", vec![num(1.0), num(1.0), num(1.0)]).unwrap();
    let first = session(&store, "alice");
    let limiter = call("create_account_rate_limiter", vec![num(1.0), num(1.0), num(1.0)]).unwrap();
    assert_eq!(call("account_rate_limit_allow", vec![limiter.clone(), text("alice")]).unwrap(), Value::Bool(true));
    std::thread::sleep(std::time::Duration::from_millis(1100));
    assert_eq!(lookup(&store, &field(&first, "id")), Value::Nothing);
    assert_eq!(call("session_rotate", vec![store.clone(), field(&first, "id")]).unwrap(), Value::Nothing);
    session(&store, "bob");
    assert_eq!(call("account_rate_limit_allow", vec![limiter, text("bob")]).unwrap(), Value::Bool(true));
}
