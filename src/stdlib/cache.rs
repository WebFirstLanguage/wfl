//! A simple in-memory TTL (time-to-live) cache primitive.
//!
//! A cache is an opaque handle (a number) created with `create_cache`. Entries
//! are stored with `cache_set`, each given a lifetime in seconds; once that time
//! has elapsed the entry is treated as absent. Expiry is *lazy* — an expired
//! entry is removed the next time it is looked up (or when the cache is counted),
//! so no background thread is required.
//!
//! ## State model
//!
//! WFL native functions are plain `fn` pointers and cannot capture state, so the
//! caches live in thread-local storage. The interpreter runs on a single thread
//! (its `Value`s hold `Rc`, which is `!Send`), so a thread-local registry is both
//! sound and shared across every call within a program run.
//!
//! ## Monotonic time
//!
//! Expiry uses [`Instant`], a monotonic clock, so it is immune to wall-clock
//! adjustments (NTP steps, manual changes) that could otherwise expire entries
//! early or late.

use super::helpers::{check_arg_count, expect_number, expect_text};
use crate::interpreter::environment::Environment;
use crate::interpreter::error::RuntimeError;
use crate::interpreter::value::Value;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// The largest TTL we accept, in seconds (~100 years). Guards against a
/// `Duration` overflow when computing an expiry instant from a wild input.
const MAX_TTL_SECONDS: f64 = 100.0 * 365.0 * 24.0 * 60.0 * 60.0;

/// A single cached value together with when it expires.
struct Entry {
    value: Value,
    /// `None` means the entry never expires (TTL of 0).
    expires_at: Option<Instant>,
}

impl Entry {
    fn is_expired(&self, now: Instant) -> bool {
        matches!(self.expires_at, Some(deadline) if now >= deadline)
    }
}

struct Cache {
    entries: HashMap<String, Entry>,
}

thread_local! {
    /// Registry of live caches keyed by their handle id.
    static CACHES: RefCell<HashMap<u64, Cache>> = RefCell::new(HashMap::new());
    /// Monotonically increasing source of cache handle ids. Starts at 1 so that
    /// a handle is always truthy/non-zero.
    static NEXT_ID: Cell<u64> = const { Cell::new(1) };
}

/// Convert a TTL-in-seconds argument into an optional expiry deadline.
///
/// A TTL of `0` (or negative) means "never expires" and yields `None`. A
/// positive TTL yields `Some(now + ttl)`. Non-finite or absurdly large TTLs are
/// rejected with a clear error.
fn deadline_from_ttl(func_name: &str, ttl: f64) -> Result<Option<Instant>, RuntimeError> {
    if !ttl.is_finite() {
        return Err(RuntimeError::new(
            format!("{func_name}: ttl seconds must be a finite number, got {ttl}"),
            0,
            0,
        ));
    }
    if ttl <= 0.0 {
        return Ok(None);
    }
    if ttl > MAX_TTL_SECONDS {
        return Err(RuntimeError::new(
            format!("{func_name}: ttl seconds {ttl} is too large (max {MAX_TTL_SECONDS})"),
            0,
            0,
        ));
    }
    Ok(Instant::now().checked_add(Duration::from_secs_f64(ttl)))
}

/// Resolve a cache handle argument to its numeric id, erroring if it is not a
/// valid, known handle.
fn cache_id(func_name: &str, value: &Value) -> Result<u64, RuntimeError> {
    let n = expect_number(value)?;
    if !n.is_finite() || n.fract() != 0.0 || n < 1.0 || n > u64::MAX as f64 {
        return Err(RuntimeError::new(
            format!("{func_name}: invalid cache handle {n}"),
            0,
            0,
        ));
    }
    Ok(n as u64)
}

/// Run `f` against the cache identified by `value`, erroring if the handle is
/// unknown (e.g. it was never created, or belongs to a different program run).
fn with_cache<F, R>(func_name: &str, value: &Value, f: F) -> Result<R, RuntimeError>
where
    F: FnOnce(&mut Cache) -> R,
{
    let id = cache_id(func_name, value)?;
    CACHES.with(|caches| {
        let mut caches = caches.borrow_mut();
        match caches.get_mut(&id) {
            Some(cache) => Ok(f(cache)),
            None => Err(RuntimeError::new(
                format!(
                    "{func_name}: unknown cache handle {id} (was it created with create_cache?)"
                ),
                0,
                0,
            )),
        }
    })
}

/// `create_cache` -> a new, empty cache handle.
///
/// Usage: `store pages as create_cache`
pub fn native_create_cache(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("create_cache", &args, 0)?;
    let id = NEXT_ID.with(|next| {
        let id = next.get();
        next.set(id + 1);
        id
    });
    CACHES.with(|caches| {
        caches.borrow_mut().insert(
            id,
            Cache {
                entries: HashMap::new(),
            },
        );
    });
    Ok(Value::Number(id as f64))
}

/// `cache_set of cache and key and value and ttl_seconds` -> nothing.
///
/// Stores `value` under `key`, expiring after `ttl_seconds`. A `ttl_seconds` of
/// `0` (or negative) stores the value with no expiry.
///
/// Usage: `cache_set of pages and "/home" and rendered and 60`
pub fn native_cache_set(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("cache_set", &args, 4)?;
    let key = expect_text(&args[1])?;
    let value = args[2].clone();
    let ttl = expect_number(&args[3])?;
    let expires_at = deadline_from_ttl("cache_set", ttl)?;

    with_cache("cache_set", &args[0], |cache| {
        cache
            .entries
            .insert(key.to_string(), Entry { value, expires_at });
    })?;
    Ok(Value::Nothing)
}

/// `cache_get of cache and key` -> the stored value, or nothing if the key is
/// absent or has expired.
///
/// Usage: `store hit as cache_get of pages and "/home"`
pub fn native_cache_get(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("cache_get", &args, 2)?;
    let key = expect_text(&args[1])?;
    let now = Instant::now();

    with_cache("cache_get", &args[0], |cache| {
        match cache.entries.get(key.as_ref()) {
            Some(entry) if entry.is_expired(now) => {
                cache.entries.remove(key.as_ref());
                Value::Nothing
            }
            Some(entry) => entry.value.clone(),
            None => Value::Nothing,
        }
    })
}

/// `cache_has of cache and key` -> boolean: is the key present and unexpired?
///
/// Usage: `check if cache_has of pages and "/home"`
pub fn native_cache_has(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("cache_has", &args, 2)?;
    let key = expect_text(&args[1])?;
    let now = Instant::now();

    with_cache("cache_has", &args[0], |cache| {
        match cache.entries.get(key.as_ref()) {
            Some(entry) if entry.is_expired(now) => {
                cache.entries.remove(key.as_ref());
                Value::Bool(false)
            }
            Some(_) => Value::Bool(true),
            None => Value::Bool(false),
        }
    })
}

/// `cache_delete of cache and key` -> boolean: was a (live) entry removed?
///
/// Usage: `cache_delete of pages and "/home"`
pub fn native_cache_delete(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("cache_delete", &args, 2)?;
    let key = expect_text(&args[1])?;
    let now = Instant::now();

    with_cache("cache_delete", &args[0], |cache| {
        match cache.entries.remove(key.as_ref()) {
            // Removing an already-expired entry reports false: nothing live was
            // there to delete.
            Some(entry) => Value::Bool(!entry.is_expired(now)),
            None => Value::Bool(false),
        }
    })
}

/// `cache_clear of cache` -> nothing. Removes every entry.
///
/// Usage: `cache_clear of pages`
pub fn native_cache_clear(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("cache_clear", &args, 1)?;
    with_cache("cache_clear", &args[0], |cache| {
        cache.entries.clear();
    })?;
    Ok(Value::Nothing)
}

/// `cache_size of cache` -> number of live (unexpired) entries.
///
/// Expired entries are swept out as a side effect, keeping memory bounded when
/// this is called periodically.
///
/// Usage: `store n as cache_size of pages`
pub fn native_cache_size(args: Vec<Value>) -> Result<Value, RuntimeError> {
    check_arg_count("cache_size", &args, 1)?;
    let now = Instant::now();
    with_cache("cache_size", &args[0], |cache| {
        cache.entries.retain(|_, entry| !entry.is_expired(now));
        Value::Number(cache.entries.len() as f64)
    })
}

pub fn register_cache(env: &mut Environment) {
    env.define_native("create_cache", native_create_cache);
    env.define_native("cache_set", native_cache_set);
    env.define_native("cache_get", native_cache_get);
    env.define_native("cache_has", native_cache_has);
    env.define_native("cache_delete", native_cache_delete);
    env.define_native("cache_clear", native_cache_clear);
    env.define_native("cache_size", native_cache_size);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn new_cache() -> Value {
        native_create_cache(vec![]).unwrap()
    }

    fn text(s: &str) -> Value {
        Value::Text(Arc::from(s))
    }

    fn set(cache: &Value, key: &str, value: Value, ttl: f64) {
        native_cache_set(vec![cache.clone(), text(key), value, Value::Number(ttl)]).unwrap();
    }

    fn get(cache: &Value, key: &str) -> Value {
        native_cache_get(vec![cache.clone(), text(key)]).unwrap()
    }

    #[test]
    fn set_then_get_returns_value() {
        let cache = new_cache();
        set(&cache, "k", text("v"), 0.0);
        assert!(matches!(get(&cache, "k"), Value::Text(t) if t.as_ref() == "v"));
    }

    #[test]
    fn missing_key_returns_nothing() {
        let cache = new_cache();
        assert!(matches!(get(&cache, "absent"), Value::Nothing));
    }

    #[test]
    fn ttl_zero_never_expires() {
        let cache = new_cache();
        set(&cache, "k", text("v"), 0.0);
        // No time passes; a zero TTL means the entry is permanent.
        assert!(matches!(get(&cache, "k"), Value::Text(_)));
        assert!(matches!(
            native_cache_has(vec![cache.clone(), text("k")]).unwrap(),
            Value::Bool(true)
        ));
    }

    #[test]
    fn expired_entry_is_absent_and_swept() {
        let cache = new_cache();
        // Expire (almost) immediately.
        set(&cache, "k", text("v"), 0.000_001);
        std::thread::sleep(Duration::from_millis(5));
        assert!(matches!(get(&cache, "k"), Value::Nothing));
        // A lookup of an expired entry sweeps it, so size is 0.
        assert!(matches!(
            native_cache_size(vec![cache.clone()]).unwrap(),
            Value::Number(n) if n == 0.0
        ));
    }

    #[test]
    fn has_reflects_presence_and_expiry() {
        let cache = new_cache();
        set(&cache, "live", text("v"), 0.0);
        assert!(matches!(
            native_cache_has(vec![cache.clone(), text("live")]).unwrap(),
            Value::Bool(true)
        ));
        assert!(matches!(
            native_cache_has(vec![cache.clone(), text("nope")]).unwrap(),
            Value::Bool(false)
        ));
    }

    #[test]
    fn delete_removes_live_entry_and_reports() {
        let cache = new_cache();
        set(&cache, "k", text("v"), 0.0);
        assert!(matches!(
            native_cache_delete(vec![cache.clone(), text("k")]).unwrap(),
            Value::Bool(true)
        ));
        assert!(matches!(get(&cache, "k"), Value::Nothing));
        // Second delete: nothing live to remove.
        assert!(matches!(
            native_cache_delete(vec![cache.clone(), text("k")]).unwrap(),
            Value::Bool(false)
        ));
    }

    #[test]
    fn clear_empties_the_cache() {
        let cache = new_cache();
        set(&cache, "a", text("1"), 0.0);
        set(&cache, "b", text("2"), 0.0);
        native_cache_clear(vec![cache.clone()]).unwrap();
        assert!(matches!(
            native_cache_size(vec![cache.clone()]).unwrap(),
            Value::Number(n) if n == 0.0
        ));
    }

    #[test]
    fn size_counts_live_entries() {
        let cache = new_cache();
        set(&cache, "a", text("1"), 0.0);
        set(&cache, "b", text("2"), 0.0);
        assert!(matches!(
            native_cache_size(vec![cache.clone()]).unwrap(),
            Value::Number(n) if n == 2.0
        ));
    }

    #[test]
    fn overwriting_a_key_replaces_value_and_ttl() {
        let cache = new_cache();
        set(&cache, "k", text("old"), 0.0);
        set(&cache, "k", text("new"), 0.0);
        assert!(matches!(get(&cache, "k"), Value::Text(t) if t.as_ref() == "new"));
        assert!(matches!(
            native_cache_size(vec![cache.clone()]).unwrap(),
            Value::Number(n) if n == 1.0
        ));
    }

    #[test]
    fn caches_are_independent() {
        let a = new_cache();
        let b = new_cache();
        set(&a, "k", text("in-a"), 0.0);
        assert!(matches!(get(&a, "k"), Value::Text(_)));
        assert!(matches!(get(&b, "k"), Value::Nothing));
    }

    #[test]
    fn unknown_handle_errors() {
        let err = native_cache_get(vec![Value::Number(999_999.0), text("k")]).unwrap_err();
        assert!(
            err.message.contains("unknown cache handle"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn invalid_handle_errors() {
        let err = native_cache_get(vec![Value::Number(0.0), text("k")]).unwrap_err();
        assert!(
            err.message.contains("invalid cache handle"),
            "got: {}",
            err.message
        );
    }
}
