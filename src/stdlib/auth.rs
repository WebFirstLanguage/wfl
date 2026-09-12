//! Bounded, process-local authentication state. Handles share state across
//! request environments and drop it with the last handle; no global registry.
//! Operations never await, making each mutation atomic on WFL's interpreter.
use super::helpers::{check_arg_count, expect_text};
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use rand::TryRng;
use sha2::{Digest, Sha256};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::rc::Rc;
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

const MAX_CAPACITY: u64 = 100_000;
const MAX_SECONDS: u64 = 2_592_000; // 30 days
const MAX_ACCOUNT_BYTES: usize = 1024;
const COOKIE_NAME: &str = "__Host-wfl_session";
type TokenHash = [u8; 32];

fn error(message: &str) -> RuntimeError {
    RuntimeError::new(format!("managed authentication: {message}"), 0, 0)
}

fn bounded_count(value: &Value, name: &str, max: u64) -> Result<u64, RuntimeError> {
    match value {
        Value::Number(n) if n.is_finite() && n.fract() == 0.0 && *n >= 1.0 && *n <= max as f64 => {
            Ok(*n as u64)
        }
        _ => Err(error(&format!(
            "{name} must be a whole number from 1 to {max}"
        ))),
    }
}

fn account(value: &Value) -> Result<&str, RuntimeError> {
    let Value::Text(account) = value else {
        return Err(error("account key must be text"));
    };
    if account.is_empty() || account.len() > MAX_ACCOUNT_BYTES {
        return Err(error("account key must contain 1 to 1024 UTF-8 bytes"));
    }
    Ok(account)
}

fn valid_token(token: &str) -> bool {
    token.len() == 64
        && token
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn token_hash(token: &str) -> TokenHash {
    Sha256::digest(token.as_bytes()).into()
}

fn new_token() -> Result<Zeroizing<String>, RuntimeError> {
    let mut bytes = Zeroizing::new([0u8; 32]);
    rand::rngs::SysRng
        .try_fill_bytes(bytes.as_mut())
        .map_err(|_| error("operating system random source unavailable"))?;
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut token = Zeroizing::new(String::with_capacity(64));
    for byte in bytes.iter() {
        token.push(HEX[(byte >> 4) as usize] as char);
        token.push(HEX[(byte & 15) as usize] as char);
    }
    Ok(token)
}

struct Session {
    account: String,
    csrf_hash: TokenHash,
    absolute_end: Instant,
    idle_end: Instant,
}

impl Session {
    fn deadline(&self) -> Instant {
        self.absolute_end.min(self.idle_end)
    }
}

/// Opaque interpreter handle backing `create_session_store`.
pub struct SessionStore {
    absolute: Duration,
    idle: Duration,
    capacity: usize,
    sessions: HashMap<TokenHash, Session>,
    // Exactly one index entry per live session, including after refresh/rotate.
    expiry: BTreeSet<(Instant, TokenHash)>,
}

impl SessionStore {
    fn prune(&mut self, now: Instant) {
        while let Some(&(deadline, key)) = self.expiry.first() {
            if deadline > now {
                break;
            }
            self.expiry.pop_first();
            self.sessions.remove(&key);
        }
    }

    fn remove(&mut self, key: &TokenHash) -> Option<Session> {
        let session = self.sessions.remove(key)?;
        self.expiry.remove(&(session.deadline(), *key));
        Some(session)
    }

    fn touch(&mut self, key: &TokenHash, now: Instant) -> Option<String> {
        let session = self.sessions.get_mut(key)?;
        self.expiry.remove(&(session.deadline(), *key));
        session.idle_end = now + self.idle;
        self.expiry.insert((session.deadline(), *key));
        Some(session.account.clone())
    }

    fn issue(
        &mut self,
        account: &str,
        now: Instant,
        prior: Option<TokenHash>,
    ) -> Result<Value, RuntimeError> {
        self.issue_using(account, now, prior, new_token)
    }

    fn issue_using(
        &mut self,
        account: &str,
        now: Instant,
        prior: Option<TokenHash>,
        mut token: impl FnMut() -> Result<Zeroizing<String>, RuntimeError>,
    ) -> Result<Value, RuntimeError> {
        self.prune(now);
        let absolute_end = if let Some(key) = prior {
            let Some(old) = self.sessions.get(&key) else {
                return Ok(Value::Nothing);
            };
            old.absolute_end
        } else {
            if self.sessions.len() >= self.capacity {
                return Err(error("session store capacity reached"));
            }
            now + self.absolute
        };
        // Generate before removing the old session: entropy failure leaves it
        // usable. Check collisions before mutating so failure is atomic too.
        let id = token()?;
        let csrf = token()?;
        let key = token_hash(&id);
        if self.sessions.contains_key(&key) {
            return Err(error("session token collision; session was not changed"));
        }
        let session = Session {
            account: account.to_owned(),
            csrf_hash: token_hash(&csrf),
            absolute_end,
            idle_end: now + self.idle,
        };
        if let Some(prior) = prior {
            self.remove(&prior);
        }
        self.expiry.insert((session.deadline(), key));
        self.sessions.insert(key, session);
        Ok(Value::Object(Rc::new(RefCell::new(HashMap::from([
            ("id".into(), Value::Text(id.as_str().into())),
            ("csrf_token".into(), Value::Text(csrf.as_str().into())),
            ("account".into(), Value::Text(account.into())),
        ])))))
    }
}

fn expect_store(value: &Value) -> Result<&Rc<RefCell<SessionStore>>, RuntimeError> {
    match value {
        Value::SessionStore(store) => Ok(store),
        _ => Err(error("expected a session store from create_session_store")),
    }
}

pub fn native_create_session_store(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("create_session_store", &args, 3)?;
    let absolute = bounded_count(&args[0], "absolute seconds", MAX_SECONDS)?;
    let idle = bounded_count(&args[1], "idle seconds", absolute)?;
    let capacity = bounded_count(&args[2], "capacity", MAX_CAPACITY)? as usize;
    Ok(Value::SessionStore(Rc::new(RefCell::new(SessionStore {
        absolute: Duration::from_secs(absolute),
        idle: Duration::from_secs(idle),
        capacity,
        sessions: HashMap::new(),
        expiry: BTreeSet::new(),
    }))))
}

pub fn native_session_create(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("session_create", &args, 2)?;
    expect_store(&args[0])?
        .borrow_mut()
        .issue(account(&args[1])?, Instant::now(), None)
}

pub fn native_session_lookup(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("session_lookup", &args, 2)?;
    let mut store = expect_store(&args[0])?.borrow_mut();
    let id = expect_text(&args[1])?;
    let now = Instant::now();
    store.prune(now);
    if !valid_token(&id) {
        return Ok(Value::Nothing);
    }
    Ok(store
        .touch(&token_hash(&id), now)
        .map_or(Value::Nothing, |s| Value::Text(s.into())))
}

pub fn native_session_rotate(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("session_rotate", &args, 2)?;
    let mut store = expect_store(&args[0])?.borrow_mut();
    let id = expect_text(&args[1])?;
    let now = Instant::now();
    store.prune(now);
    if !valid_token(&id) {
        return Ok(Value::Nothing);
    }
    let key = token_hash(&id);
    let Some(session) = store.sessions.get(&key) else {
        return Ok(Value::Nothing);
    };
    let account = session.account.clone();
    store.issue(&account, now, Some(key))
}

pub fn native_session_revoke(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("session_revoke", &args, 2)?;
    let mut store = expect_store(&args[0])?.borrow_mut();
    let id = expect_text(&args[1])?;
    store.prune(Instant::now());
    Ok(Value::Bool(
        valid_token(&id) && store.remove(&token_hash(&id)).is_some(),
    ))
}

pub fn native_session_revoke_account(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("session_revoke_account", &args, 2)?;
    let mut store = expect_store(&args[0])?.borrow_mut();
    let account = account(&args[1])?;
    store.prune(Instant::now());
    let keys: Vec<_> = store
        .sessions
        .iter()
        .filter_map(|(key, s)| (s.account == account).then_some(*key))
        .collect();
    for key in &keys {
        store.remove(key);
    }
    Ok(Value::Number(keys.len() as f64))
}

// A strict cookie reader rather than parse_cookies (which intentionally has
// general-purpose last-value semantics). Ambiguity must never select a token.
fn session_id_from_cookie(cookie: &str) -> Option<&str> {
    if cookie.len() > 8192 || cookie.bytes().any(|b| b.is_ascii_control() || b == b',') {
        return None;
    }
    let mut id = None;
    for part in cookie.split(';') {
        let (name, value) = part.trim().split_once('=')?;
        if name == COOKIE_NAME {
            if id.is_some() || !valid_token(value) {
                return None;
            }
            id = Some(value);
        }
    }
    id
}

fn header<'a>(headers: &'a HashMap<String, Value>, name: &str) -> Option<&'a str> {
    let mut values = headers
        .iter()
        .filter(|(key, _)| key.eq_ignore_ascii_case(name));
    let (_, Value::Text(value)) = values.next()? else {
        return None;
    };
    if values.next().is_some() {
        return None;
    }
    Some(value)
}

pub fn native_session_csrf_guard(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("session_csrf_guard", &args, 2)?;
    let mut store = expect_store(&args[0])?.borrow_mut();
    let now = Instant::now();
    store.prune(now);
    let denied = Value::Bool(false);
    let Value::Object(request) = &args[1] else {
        return Ok(denied);
    };
    let request = request.borrow();
    if request
        .get("ambiguous_auth_headers")
        .is_some_and(|v| !matches!(v, Value::Bool(false)))
    {
        return Ok(denied);
    }
    let Some(Value::Text(method)) = request.get("method") else {
        return Ok(denied);
    };
    let Some(Value::Object(headers)) = request.get("headers") else {
        return Ok(denied);
    };
    let headers = headers.borrow();
    let Some(id) = header(&headers, "cookie").and_then(session_id_from_cookie) else {
        return Ok(denied);
    };
    let key = token_hash(id);
    let Some(session) = store.sessions.get(&key) else {
        return Ok(denied);
    };
    if !matches!(method.as_ref(), "GET" | "HEAD" | "OPTIONS") {
        let Some(csrf) = header(&headers, "x-csrf-token").filter(|token| valid_token(token)) else {
            return Ok(denied);
        };
        if !bool::from(session.csrf_hash.ct_eq(&token_hash(csrf))) {
            return Ok(denied);
        }
    }
    store.touch(&key, now);
    Ok(Value::Bool(true))
}

pub fn native_session_cookie(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("session_cookie", &args, 1)?;
    let id = expect_text(&args[0])?;
    if !valid_token(&id) {
        return Err(error(
            "session_cookie requires a 64-character lowercase hexadecimal session ID",
        ));
    }
    Ok(Value::Text(
        format!("{COOKIE_NAME}={id}; Path=/; Secure; HttpOnly; SameSite=Strict").into(),
    ))
}

struct RateWindow {
    count: u64,
}

/// A bounded fixed-window account limiter. Full stores deny new keys rather
/// than evicting blocked accounts; denied attempts do not extend the window.
pub struct AccountRateLimiter {
    attempts: u64,
    window: Duration,
    capacity: usize,
    accounts: HashMap<String, RateWindow>,
    expiry: BTreeSet<(Instant, String)>,
}

impl AccountRateLimiter {
    fn allow(&mut self, account: &str, now: Instant) -> bool {
        while let Some((deadline, key)) = self.expiry.first() {
            if *deadline > now {
                break;
            }
            self.accounts.remove(key);
            self.expiry.pop_first();
        }
        if let Some(window) = self.accounts.get_mut(account) {
            if window.count >= self.attempts {
                return false;
            }
            window.count += 1;
            return true;
        }
        if self.accounts.len() >= self.capacity {
            return false;
        }
        let end = now + self.window;
        self.accounts
            .insert(account.to_owned(), RateWindow { count: 1 });
        self.expiry.insert((end, account.to_owned()));
        true
    }
}

pub fn native_create_account_rate_limiter(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("create_account_rate_limiter", &args, 3)?;
    let attempts = bounded_count(&args[0], "attempt limit", MAX_CAPACITY)?;
    let seconds = bounded_count(&args[1], "window seconds", MAX_SECONDS)?;
    let capacity = bounded_count(&args[2], "capacity", MAX_CAPACITY)? as usize;
    Ok(Value::AccountRateLimiter(Rc::new(RefCell::new(
        AccountRateLimiter {
            attempts,
            window: Duration::from_secs(seconds),
            capacity,
            accounts: HashMap::new(),
            expiry: BTreeSet::new(),
        },
    ))))
}

pub fn native_account_rate_limit_allow(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("account_rate_limit_allow", &args, 2)?;
    let Value::AccountRateLimiter(limiter) = &args[0] else {
        return Err(error("expected a limiter from create_account_rate_limiter"));
    };
    Ok(Value::Bool(
        limiter
            .borrow_mut()
            .allow(account(&args[1])?, Instant::now()),
    ))
}

pub fn register_auth(env: &mut Environment) {
    env.define_native("create_session_store", native_create_session_store);
    env.define_native("session_create", native_session_create);
    env.define_native("session_lookup", native_session_lookup);
    env.define_native("session_rotate", native_session_rotate);
    env.define_native("session_revoke", native_session_revoke);
    env.define_native("session_revoke_account", native_session_revoke_account);
    env.define_native("session_csrf_guard", native_session_csrf_guard);
    env.define_native("session_cookie", native_session_cookie);
    env.define_native(
        "create_account_rate_limiter",
        native_create_account_rate_limiter,
    );
    env.define_native("account_rate_limit_allow", native_account_rate_limit_allow);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> SessionStore {
        SessionStore {
            absolute: Duration::from_secs(10),
            idle: Duration::from_secs(4),
            capacity: 2,
            sessions: HashMap::new(),
            expiry: BTreeSet::new(),
        }
    }

    fn id(session: &Value) -> String {
        let Value::Object(session) = session else {
            panic!("expected session")
        };
        session.borrow()["id"].to_string()
    }

    #[test]
    fn idle_refresh_and_rotation_never_extend_absolute_expiry() {
        let now = Instant::now();
        let mut store = store();
        let first = store.issue("alice", now, None).unwrap();
        let first_key = token_hash(&id(&first));
        for seconds in [3, 6] {
            store.prune(now + Duration::from_secs(seconds));
            assert_eq!(
                store.touch(&first_key, now + Duration::from_secs(seconds)),
                Some("alice".into())
            );
            assert_eq!(store.expiry.len(), 1);
        }
        let next = store
            .issue("alice", now + Duration::from_secs(8), Some(first_key))
            .unwrap();
        let next_key = token_hash(&id(&next));
        assert!(!store.sessions.contains_key(&first_key));
        store.prune(now + Duration::from_secs(9));
        assert_eq!(
            store.touch(&next_key, now + Duration::from_secs(9)),
            Some("alice".into())
        );
        store.prune(now + Duration::from_secs(10));
        assert!(store.sessions.is_empty());
        assert!(store.expiry.is_empty());
        assert_eq!(
            store
                .issue("alice", now + Duration::from_secs(10), Some(next_key))
                .unwrap(),
            Value::Nothing
        );
    }

    #[test]
    fn idle_deadline_is_inclusive_and_reclaims_capacity() {
        let now = Instant::now();
        let mut store = store();
        store.issue("alice", now, None).unwrap();
        store.issue("bob", now, None).unwrap();
        store.prune(now + Duration::from_secs(4));
        assert!(store.sessions.is_empty());
        assert!(store.expiry.is_empty());
        store
            .issue("new", now + Duration::from_secs(4), None)
            .unwrap();
    }

    #[test]
    fn entropy_failure_and_collision_preserve_existing_session() {
        let now = Instant::now();
        let mut store = store();
        let old = store.issue("alice", now, None).unwrap();
        let old_id = id(&old);
        let key = token_hash(&old_id);
        let mut calls = 0;
        assert!(
            store
                .issue_using("alice", now, Some(key), || {
                    calls += 1;
                    if calls == 1 {
                        Ok(Zeroizing::new("a".repeat(64)))
                    } else {
                        Err(error("test entropy failure"))
                    }
                })
                .is_err()
        );
        assert_eq!(store.sessions[&key].account, "alice");
        assert_eq!(store.expiry.len(), 1);
        assert!(
            store
                .issue_using("alice", now, Some(key), || Ok(Zeroizing::new(
                    old_id.clone()
                )))
                .is_err()
        );
        assert_eq!(store.sessions.len(), 1);
        assert_eq!(store.expiry.len(), 1);
    }

    #[test]
    fn limiter_denial_does_not_extend_window_or_expand_indexes() {
        let now = Instant::now();
        let mut limiter = AccountRateLimiter {
            attempts: 1,
            window: Duration::from_secs(10),
            capacity: 1,
            accounts: HashMap::new(),
            expiry: BTreeSet::new(),
        };
        assert!(limiter.allow("alice", now));
        for seconds in 1..10 {
            assert!(!limiter.allow("alice", now + Duration::from_secs(seconds)));
            assert!(!limiter.allow("other", now + Duration::from_secs(seconds)));
            assert_eq!(limiter.accounts.len(), 1);
            assert_eq!(limiter.expiry.len(), 1);
        }
        assert!(limiter.allow("other", now + Duration::from_secs(10)));
        assert_eq!(limiter.accounts.len(), 1);
        assert_eq!(limiter.expiry.len(), 1);
    }
}
