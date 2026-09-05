//! Server-side session store for `listen ... with sessions enabled`.
//!
//! Three backends share one [`SessionManager`]: in-memory, a JSON file, and
//! SQLite. Session IDs are 32 CSPRNG bytes encoded as hex. Lookups of unknown
//! or expired IDs return `None`; a full store refuses `create`.

use super::value::Value;
use crate::config::WflConfig;
use serde_json::{Map, json};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

const SESSION_ID_BYTES: usize = 32;
const CSRF_BYTES: usize = 32;

pub use crate::config::{SessionSameSite, SessionStorageKind};

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub timeout_ms: u64,
    pub storage: SessionStorageKind,
    pub db_path: String,
    pub file_path: String,
    pub cookie_name: String,
    pub cookie_secure: bool,
    pub cookie_samesite: SessionSameSite,
    pub cookie_httponly: bool,
    pub csrf_enabled: bool,
    pub max_sessions: usize,
}

impl SessionConfig {
    pub fn from_wfl_config(config: &WflConfig) -> Self {
        Self {
            timeout_ms: config.session_timeout_ms,
            storage: config.session_storage,
            db_path: config.session_db_path.clone(),
            file_path: config.session_file_path.clone(),
            cookie_name: config.session_cookie_name.clone(),
            cookie_secure: config.session_cookie_secure,
            cookie_samesite: config.session_cookie_samesite,
            cookie_httponly: config.session_cookie_httponly,
            csrf_enabled: config.session_csrf_enabled,
            max_sessions: config.session_max_sessions,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionRecord {
    pub id: String,
    pub data: HashMap<String, Value>,
    pub created_at: i64,
    pub last_activity: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone)]
pub struct SessionStats {
    pub active_sessions: u64,
    pub total_created: u64,
    pub expired_count: u64,
    pub storage_type: String,
}

struct StoreState {
    kind: SessionStorageKind,
    sessions: HashMap<String, SessionRecord>,
    kv: HashMap<String, Value>,
    file_path: Option<PathBuf>,
    db: Option<SqlitePool>,
}

pub struct SessionManager {
    config: Mutex<SessionConfig>,
    store: Mutex<StoreState>,
    total_created: Mutex<u64>,
    expired_count: Mutex<u64>,
}

impl std::fmt::Debug for SessionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SessionManager")
    }
}

impl SessionManager {
    pub async fn new(config: SessionConfig) -> Result<Self, String> {
        let store = open_store(&config).await?;
        Ok(Self {
            config: Mutex::new(config),
            store: Mutex::new(store),
            total_created: Mutex::new(0),
            expired_count: Mutex::new(0),
        })
    }

    pub async fn config_snapshot(&self) -> SessionConfig {
        self.config.lock().await.clone()
    }

    pub async fn configure(
        &self,
        timeout_ms: u64,
        storage: SessionStorageKind,
    ) -> Result<(), String> {
        let mut cfg = self.config.lock().await;
        let storage_changed = cfg.storage != storage;
        cfg.timeout_ms = timeout_ms;
        cfg.storage = storage;
        if storage_changed {
            let new_store = open_store(&cfg).await?;
            *self.store.lock().await = new_store;
        }
        Ok(())
    }

    pub async fn enable_csrf(&self) {
        self.config.lock().await.csrf_enabled = true;
    }

    pub async fn enable_secure_cookies(&self) {
        self.config.lock().await.cookie_secure = true;
    }

    pub async fn create(&self) -> Result<SessionRecord, String> {
        let cfg = self.config.lock().await.clone();
        let mut store = self.store.lock().await;
        prune_expired(&mut store, &cfg, &self.expired_count).await;
        if store.sessions.len() >= cfg.max_sessions {
            return Err(format!(
                "Session store is full (maximum {} sessions). Destroy unused sessions or raise session_max_sessions in .wflcfg.",
                cfg.max_sessions
            ));
        }
        let now = now_ms();
        let record = SessionRecord {
            id: random_hex(SESSION_ID_BYTES),
            data: HashMap::new(),
            created_at: now,
            last_activity: now,
            expires_at: now.saturating_add(cfg.timeout_ms as i64),
        };
        store.sessions.insert(record.id.clone(), record.clone());
        persist(&store).await?;
        *self.total_created.lock().await += 1;
        Ok(record)
    }

    pub async fn get(&self, id: &str) -> Result<Option<SessionRecord>, String> {
        let cfg = self.config.lock().await.clone();
        let mut store = self.store.lock().await;
        prune_expired(&mut store, &cfg, &self.expired_count).await;
        match store.sessions.get(id) {
            Some(record) if record.expires_at >= now_ms() => {
                let mut updated = record.clone();
                let now = now_ms();
                updated.last_activity = now;
                updated.expires_at = now.saturating_add(cfg.timeout_ms as i64);
                store.sessions.insert(id.to_string(), updated.clone());
                persist(&store).await?;
                Ok(Some(updated))
            }
            Some(_) => {
                store.sessions.remove(id);
                *self.expired_count.lock().await += 1;
                persist(&store).await?;
                Ok(None)
            }
            None => Ok(None),
        }
    }

    pub async fn set_value(&self, id: &str, key: &str, value: Value) -> Result<(), String> {
        value_to_json(&value)?;
        let cfg = self.config.lock().await.clone();
        let mut store = self.store.lock().await;
        prune_expired(&mut store, &cfg, &self.expired_count).await;
        let record = store
            .sessions
            .get_mut(id)
            .ok_or_else(|| format!("Unknown session '{id}'. It may have been destroyed."))?;
        if record.expires_at < now_ms() {
            store.sessions.remove(id);
            *self.expired_count.lock().await += 1;
            persist(&store).await?;
            return Err(format!(
                "Unknown session '{id}'. It may have been destroyed."
            ));
        }
        record.data.insert(key.to_string(), value);
        let now = now_ms();
        record.last_activity = now;
        record.expires_at = now.saturating_add(cfg.timeout_ms as i64);
        persist(&store).await
    }

    pub async fn destroy(&self, id: &str) -> Result<(), String> {
        let mut store = self.store.lock().await;
        store.sessions.remove(id);
        persist(&store).await
    }

    pub async fn find_expired(&self) -> Result<Vec<SessionRecord>, String> {
        let mut store = self.store.lock().await;
        let now = now_ms();
        let expired: Vec<SessionRecord> = store
            .sessions
            .values()
            .filter(|record| record.expires_at < now)
            .cloned()
            .collect();
        for record in &expired {
            store.sessions.remove(&record.id);
        }
        if !expired.is_empty() {
            *self.expired_count.lock().await += expired.len() as u64;
            persist(&store).await?;
        }
        Ok(expired)
    }

    pub async fn stats(&self) -> SessionStats {
        let cfg = self.config.lock().await.clone();
        let store = self.store.lock().await;
        let now = now_ms();
        let active = store
            .sessions
            .values()
            .filter(|record| record.expires_at >= now)
            .count() as u64;
        SessionStats {
            active_sessions: active,
            total_created: *self.total_created.lock().await,
            expired_count: *self.expired_count.lock().await,
            storage_type: cfg.storage.as_str().to_string(),
        }
    }

    pub async fn put_kv(&self, key: &str, data: Value) -> Result<(), String> {
        value_to_json(&data)?;
        let mut store = self.store.lock().await;
        store.kv.insert(key.to_string(), data);
        persist(&store).await
    }

    pub async fn load_kv(&self, key: &str) -> Result<Option<Value>, String> {
        let store = self.store.lock().await;
        Ok(store.kv.get(key).cloned())
    }

    pub async fn delete_kv(&self, key: &str) -> Result<(), String> {
        let mut store = self.store.lock().await;
        store.kv.remove(key);
        persist(&store).await
    }

    pub fn format_set_cookie(config: &SessionConfig, session_id: &str, clear: bool) -> String {
        let max_age = if clear {
            0
        } else {
            config.timeout_ms.div_ceil(1000)
        };
        let mut parts = vec![
            format!("{}={}", config.cookie_name, session_id),
            "Path=/".to_string(),
            format!("Max-Age={max_age}"),
            format!("SameSite={}", config.cookie_samesite.as_str()),
        ];
        if config.cookie_httponly {
            parts.push("HttpOnly".to_string());
        }
        if config.cookie_secure {
            parts.push("Secure".to_string());
        }
        parts.join("; ")
    }

    pub fn session_object(record: &SessionRecord, server_name: &str) -> Value {
        let mut map = HashMap::new();
        map.insert("id".to_string(), Value::Text(Arc::from(record.id.as_str())));
        map.insert(
            "created_at".to_string(),
            Value::Number(record.created_at as f64),
        );
        map.insert(
            "last_activity".to_string(),
            Value::Number(record.last_activity as f64),
        );
        map.insert("_server".to_string(), Value::Text(Arc::from(server_name)));
        Value::Object(std::rc::Rc::new(std::cell::RefCell::new(map)))
    }

    pub fn stats_object(stats: &SessionStats) -> Value {
        let mut map = HashMap::new();
        map.insert(
            "active_sessions".to_string(),
            Value::Number(stats.active_sessions as f64),
        );
        map.insert(
            "total_created".to_string(),
            Value::Number(stats.total_created as f64),
        );
        map.insert(
            "expired_count".to_string(),
            Value::Number(stats.expired_count as f64),
        );
        map.insert(
            "storage_type".to_string(),
            Value::Text(Arc::from(stats.storage_type.as_str())),
        );
        Value::Object(std::rc::Rc::new(std::cell::RefCell::new(map)))
    }

    pub fn generate_csrf_hex() -> String {
        random_hex(CSRF_BYTES)
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn random_hex(n: usize) -> String {
    use rand::Rng;
    let mut buf = vec![0u8; n];
    rand::rng().fill_bytes(&mut buf);
    hex_encode(&buf)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

async fn open_store(config: &SessionConfig) -> Result<StoreState, String> {
    match config.storage {
        SessionStorageKind::Memory => Ok(StoreState {
            kind: SessionStorageKind::Memory,
            sessions: HashMap::new(),
            kv: HashMap::new(),
            file_path: None,
            db: None,
        }),
        SessionStorageKind::File => {
            let path = PathBuf::from(&config.file_path);
            let (sessions, kv) = load_file_store(&path)?;
            Ok(StoreState {
                kind: SessionStorageKind::File,
                sessions,
                kv,
                file_path: Some(path),
                db: None,
            })
        }
        SessionStorageKind::Database => {
            let options = SqliteConnectOptions::new()
                .filename(&config.db_path)
                .create_if_missing(true);
            let pool = SqlitePoolOptions::new()
                .max_connections(5)
                .connect_with(options)
                .await
                .map_err(|e| format!("Failed to open session database: {e}"))?;
            init_sqlite(&pool).await?;
            let (sessions, kv) = load_sqlite(&pool).await?;
            Ok(StoreState {
                kind: SessionStorageKind::Database,
                sessions,
                kv,
                file_path: None,
                db: Some(pool),
            })
        }
    }
}

async fn prune_expired(store: &mut StoreState, _cfg: &SessionConfig, expired_count: &Mutex<u64>) {
    let now = now_ms();
    let before = store.sessions.len();
    store.sessions.retain(|_, record| record.expires_at >= now);
    let removed = before - store.sessions.len();
    if removed > 0 {
        *expired_count.lock().await += removed as u64;
    }
}

async fn persist(store: &StoreState) -> Result<(), String> {
    match store.kind {
        SessionStorageKind::Memory => Ok(()),
        SessionStorageKind::File => {
            let path = store
                .file_path
                .as_ref()
                .ok_or_else(|| "Session file path is missing".to_string())?;
            save_file_store(path, &store.sessions, &store.kv)
        }
        SessionStorageKind::Database => {
            let pool = store
                .db
                .as_ref()
                .ok_or_else(|| "Session database is missing".to_string())?;
            save_sqlite(pool, &store.sessions, &store.kv).await
        }
    }
}

type LoadedStore = (HashMap<String, SessionRecord>, HashMap<String, Value>);

fn load_file_store(path: &Path) -> Result<LoadedStore, String> {
    if !path.exists() {
        return Ok((HashMap::new(), HashMap::new()));
    }
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read session file {}: {e}", path.display()))?;
    let root: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse session file {}: {e}", path.display()))?;
    decode_store_json(&root)
}

fn save_file_store(
    path: &Path,
    sessions: &HashMap<String, SessionRecord>,
    kv: &HashMap<String, Value>,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create session file directory: {e}"))?;
    }
    let json = encode_store_json(sessions, kv)?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)
        .map_err(|e| format!("Failed to write session file {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path)
        .map_err(|e| format!("Failed to replace session file {}: {e}", path.display()))?;
    Ok(())
}

async fn init_sqlite(pool: &SqlitePool) -> Result<(), String> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS wfl_sessions (
            id TEXT PRIMARY KEY,
            data TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            last_activity INTEGER NOT NULL,
            expires_at INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to create wfl_sessions table: {e}"))?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS wfl_session_kv (
            key TEXT PRIMARY KEY,
            data TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Failed to create wfl_session_kv table: {e}"))?;
    Ok(())
}

async fn load_sqlite(pool: &SqlitePool) -> Result<LoadedStore, String> {
    let session_rows = sqlx::query_as::<_, (String, String, i64, i64, i64)>(
        "SELECT id, data, created_at, last_activity, expires_at FROM wfl_sessions",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Failed to load sessions: {e}"))?;
    let mut sessions = HashMap::new();
    for (id, data, created_at, last_activity, expires_at) in session_rows {
        let json: serde_json::Value =
            serde_json::from_str(&data).map_err(|e| format!("Corrupt session row {id}: {e}"))?;
        sessions.insert(
            id.clone(),
            SessionRecord {
                id,
                data: json_to_map(&json)?,
                created_at,
                last_activity,
                expires_at,
            },
        );
    }
    let kv_rows = sqlx::query_as::<_, (String, String)>("SELECT key, data FROM wfl_session_kv")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Failed to load session storage: {e}"))?;
    let mut kv = HashMap::new();
    for (key, data) in kv_rows {
        let json: serde_json::Value =
            serde_json::from_str(&data).map_err(|e| format!("Corrupt session kv '{key}': {e}"))?;
        kv.insert(key, json_to_value(&json)?);
    }
    Ok((sessions, kv))
}

async fn save_sqlite(
    pool: &SqlitePool,
    sessions: &HashMap<String, SessionRecord>,
    kv: &HashMap<String, Value>,
) -> Result<(), String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Failed to start session transaction: {e}"))?;
    sqlx::query("DELETE FROM wfl_sessions")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to clear sessions: {e}"))?;
    sqlx::query("DELETE FROM wfl_session_kv")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to clear session storage: {e}"))?;
    for record in sessions.values() {
        let data = serde_json::to_string(&map_to_json(&record.data)?)
            .map_err(|e| format!("Failed to encode session: {e}"))?;
        sqlx::query(
            "INSERT INTO wfl_sessions (id, data, created_at, last_activity, expires_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&record.id)
        .bind(data)
        .bind(record.created_at)
        .bind(record.last_activity)
        .bind(record.expires_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Failed to save session: {e}"))?;
    }
    for (key, value) in kv {
        let data = serde_json::to_string(&value_to_json(value)?)
            .map_err(|e| format!("Failed to encode session storage: {e}"))?;
        sqlx::query("INSERT INTO wfl_session_kv (key, data) VALUES (?, ?)")
            .bind(key)
            .bind(data)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Failed to save session storage: {e}"))?;
    }
    tx.commit()
        .await
        .map_err(|e| format!("Failed to commit session store: {e}"))?;
    Ok(())
}

fn encode_store_json(
    sessions: &HashMap<String, SessionRecord>,
    kv: &HashMap<String, Value>,
) -> Result<String, String> {
    let mut session_map = Map::new();
    for (id, record) in sessions {
        session_map.insert(
            id.clone(),
            json!({
                "data": map_to_json(&record.data)?,
                "created_at": record.created_at,
                "last_activity": record.last_activity,
                "expires_at": record.expires_at,
            }),
        );
    }
    let mut kv_map = Map::new();
    for (key, value) in kv {
        kv_map.insert(key.clone(), value_to_json(value)?);
    }
    serde_json::to_string_pretty(&json!({
        "sessions": session_map,
        "kv": kv_map,
    }))
    .map_err(|e| format!("Failed to encode session file: {e}"))
}

fn decode_store_json(root: &serde_json::Value) -> Result<LoadedStore, String> {
    let mut sessions = HashMap::new();
    if let Some(map) = root.get("sessions").and_then(|v| v.as_object()) {
        for (id, record) in map {
            sessions.insert(
                id.clone(),
                SessionRecord {
                    id: id.clone(),
                    data: json_to_map(record.get("data").unwrap_or(&json!({})))?,
                    created_at: record
                        .get("created_at")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                    last_activity: record
                        .get("last_activity")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                    expires_at: record
                        .get("expires_at")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                },
            );
        }
    }
    let mut kv = HashMap::new();
    if let Some(map) = root.get("kv").and_then(|v| v.as_object()) {
        for (key, value) in map {
            kv.insert(key.clone(), json_to_value(value)?);
        }
    }
    Ok((sessions, kv))
}

fn map_to_json(map: &HashMap<String, Value>) -> Result<serde_json::Value, String> {
    let mut out = Map::new();
    for (key, value) in map {
        out.insert(key.clone(), value_to_json(value)?);
    }
    Ok(serde_json::Value::Object(out))
}

fn json_to_map(value: &serde_json::Value) -> Result<HashMap<String, Value>, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| "Session data must be a JSON object".to_string())?;
    let mut map = HashMap::new();
    for (key, val) in obj {
        map.insert(key.clone(), json_to_value(val)?);
    }
    Ok(map)
}

fn value_to_json(value: &Value) -> Result<serde_json::Value, String> {
    match value {
        Value::Number(n) => Ok(json!(n)),
        Value::Text(s) => Ok(json!(s.as_ref())),
        Value::Bool(b) => Ok(json!(b)),
        Value::Nothing | Value::Null => Ok(serde_json::Value::Null),
        Value::List(list) => {
            let items: Result<Vec<_>, _> = list.borrow().iter().map(value_to_json).collect();
            Ok(serde_json::Value::Array(items?))
        }
        Value::Object(obj) => map_to_json(&obj.borrow()),
        other => Err(format!(
            "Session values must be text, numbers, yes/no, lists, maps, or nothing. Cannot store {}.",
            other.type_name()
        )),
    }
}

fn json_to_value(value: &serde_json::Value) -> Result<Value, String> {
    match value {
        serde_json::Value::Null => Ok(Value::Nothing),
        serde_json::Value::Bool(b) => Ok(Value::Bool(*b)),
        serde_json::Value::Number(n) => n
            .as_f64()
            .map(Value::Number)
            .ok_or_else(|| "Session number is out of range".to_string()),
        serde_json::Value::String(s) => Ok(Value::Text(Arc::from(s.as_str()))),
        serde_json::Value::Array(items) => {
            let values: Result<Vec<_>, _> = items.iter().map(json_to_value).collect();
            Ok(Value::List(std::rc::Rc::new(std::cell::RefCell::new(
                values?,
            ))))
        }
        serde_json::Value::Object(obj) => {
            let mut map = HashMap::new();
            for (key, val) in obj {
                map.insert(key.clone(), json_to_value(val)?);
            }
            Ok(Value::Object(std::rc::Rc::new(std::cell::RefCell::new(
                map,
            ))))
        }
    }
}
