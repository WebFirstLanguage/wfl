// TDD unit tests for the session store backends (memory, file, SQLite).
//
// These talk to the real store — no mocks of the persistence boundary.

use std::sync::Arc;
use std::time::Duration;
use wfl::interpreter::sessions::{
    SessionConfig, SessionManager, SessionSameSite, SessionStorageKind,
};
use wfl::interpreter::value::Value;

fn memory_config() -> SessionConfig {
    SessionConfig {
        timeout_ms: 60_000,
        storage: SessionStorageKind::Memory,
        db_path: "wfl_sessions.db".into(),
        file_path: "wfl_sessions.json".into(),
        cookie_name: "wfl_sid".into(),
        cookie_secure: false,
        cookie_samesite: SessionSameSite::Lax,
        cookie_httponly: true,
        csrf_enabled: false,
        max_sessions: 10,
    }
}

fn text(s: &str) -> Value {
    Value::Text(s.into())
}

#[tokio::test]
async fn memory_create_get_set_destroy() {
    let manager = SessionManager::new(memory_config()).await.unwrap();
    let created = manager.create().await.unwrap();
    assert!(!created.id.is_empty());
    assert_eq!(created.id.len(), 64, "32 CSPRNG bytes as hex");

    manager
        .set_value(&created.id, "user_id", text("alice"))
        .await
        .unwrap();
    let loaded = manager.get(&created.id).await.unwrap().expect("session");
    assert_eq!(loaded.id, created.id);
    match loaded.data.get("user_id") {
        Some(Value::Text(s)) => assert_eq!(s.as_ref(), "alice"),
        other => panic!("expected alice, got {other:?}"),
    }

    manager.destroy(&created.id).await.unwrap();
    assert!(manager.get(&created.id).await.unwrap().is_none());
}

#[tokio::test]
async fn unknown_and_forged_ids_return_none() {
    let manager = SessionManager::new(memory_config()).await.unwrap();
    assert!(manager.get("not-a-real-session").await.unwrap().is_none());
    assert!(
        manager
            .get("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn expired_session_get_returns_none() {
    let mut cfg = memory_config();
    cfg.timeout_ms = 1;
    let manager = SessionManager::new(cfg).await.unwrap();
    let created = manager.create().await.unwrap();
    tokio::time::sleep(Duration::from_millis(5)).await;
    assert!(manager.get(&created.id).await.unwrap().is_none());
}

#[tokio::test]
async fn max_sessions_denies_create() {
    let mut cfg = memory_config();
    cfg.max_sessions = 1;
    let manager = SessionManager::new(cfg).await.unwrap();
    manager.create().await.unwrap();
    let err = manager.create().await.expect_err("store should be full");
    assert!(
        err.to_lowercase().contains("maximum") || err.to_lowercase().contains("full"),
        "expected a max-sessions error, got {err}"
    );
}

#[tokio::test]
async fn set_after_destroy_errors() {
    let manager = SessionManager::new(memory_config()).await.unwrap();
    let created = manager.create().await.unwrap();
    manager.destroy(&created.id).await.unwrap();
    let err = manager
        .set_value(&created.id, "x", text("y"))
        .await
        .expect_err("set after destroy");
    assert!(
        err.to_lowercase().contains("destroy")
            || err.to_lowercase().contains("not found")
            || err.to_lowercase().contains("unknown"),
        "expected a missing-session error, got {err}"
    );
}

#[tokio::test]
async fn kv_storage_put_load_delete() {
    let manager = SessionManager::new(memory_config()).await.unwrap();
    manager
        .put_kv("test_session_123", text("payload"))
        .await
        .unwrap();
    match manager.load_kv("test_session_123").await.unwrap() {
        Some(Value::Text(s)) => assert_eq!(s.as_ref(), "payload"),
        other => panic!("expected payload, got {other:?}"),
    }
    manager.delete_kv("test_session_123").await.unwrap();
    assert!(manager.load_kv("test_session_123").await.unwrap().is_none());
}

#[tokio::test]
async fn find_expired_and_stats() {
    let mut cfg = memory_config();
    cfg.timeout_ms = 1;
    let manager = SessionManager::new(cfg).await.unwrap();
    let created = manager.create().await.unwrap();
    tokio::time::sleep(Duration::from_millis(5)).await;
    let expired = manager.find_expired().await.unwrap();
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].id, created.id);

    let stats = manager.stats().await;
    assert_eq!(stats.storage_type, "memory");
    assert_eq!(stats.total_created, 1);
}

#[tokio::test]
async fn file_backend_persists_across_managers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wfl_sessions.json");
    let mut cfg = memory_config();
    cfg.storage = SessionStorageKind::File;
    cfg.file_path = path.to_string_lossy().into_owned();

    let first = SessionManager::new(cfg.clone()).await.unwrap();
    let created = first.create().await.unwrap();
    first
        .set_value(&created.id, "user_id", text("bob"))
        .await
        .unwrap();
    drop(first);

    let second = SessionManager::new(cfg).await.unwrap();
    let loaded = second.get(&created.id).await.unwrap().expect("reloaded");
    match loaded.data.get("user_id") {
        Some(Value::Text(s)) => assert_eq!(s.as_ref(), "bob"),
        other => panic!("expected bob, got {other:?}"),
    }
}

#[tokio::test]
async fn sqlite_backend_persists_across_managers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("wfl_sessions.db");
    let mut cfg = memory_config();
    cfg.storage = SessionStorageKind::Database;
    cfg.db_path = path.to_string_lossy().into_owned();

    let first = SessionManager::new(cfg.clone()).await.unwrap();
    let created = first.create().await.unwrap();
    first
        .set_value(&created.id, "user_id", text("carol"))
        .await
        .unwrap();
    drop(first);

    let second = SessionManager::new(cfg).await.unwrap();
    let loaded = second.get(&created.id).await.unwrap().expect("reloaded");
    match loaded.data.get("user_id") {
        Some(Value::Text(s)) => assert_eq!(s.as_ref(), "carol"),
        other => panic!("expected carol, got {other:?}"),
    }
}

#[tokio::test]
async fn concurrent_updates_do_not_corrupt_store() {
    let manager = Arc::new(SessionManager::new(memory_config()).await.unwrap());
    let a = manager.create().await.unwrap();
    let b = manager.create().await.unwrap();
    let id_a = a.id.clone();
    let id_b = b.id.clone();
    let left = {
        let manager = Arc::clone(&manager);
        async move {
            for i in 0..50 {
                manager
                    .set_value(&id_a, "n", Value::Number(i as f64))
                    .await
                    .unwrap();
            }
        }
    };
    let right = {
        let manager = Arc::clone(&manager);
        async move {
            for i in 0..50 {
                manager
                    .set_value(&id_b, "n", Value::Number(i as f64))
                    .await
                    .unwrap();
            }
        }
    };
    tokio::join!(left, right);
    assert!(manager.get(&a.id).await.unwrap().is_some());
    assert!(manager.get(&b.id).await.unwrap().is_some());
}

#[test]
fn cookie_header_includes_httponly_and_clear_expires() {
    let cfg = memory_config();
    let set = SessionManager::format_set_cookie(&cfg, "abc123", false);
    assert!(set.contains("wfl_sid=abc123"));
    assert!(set.contains("HttpOnly"));
    assert!(set.contains("SameSite=Lax"));
    assert!(set.contains("Max-Age=60"));

    let clear = SessionManager::format_set_cookie(&cfg, "", true);
    assert!(clear.contains("Max-Age=0"));
}
