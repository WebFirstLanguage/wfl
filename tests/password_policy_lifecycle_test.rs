//! Configured hashing admission, cancellation and recovery use the real Tokio
//! blocking pool. Kept in a separate integration executable to isolate the
//! process-wide limits from other password tests.

use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;
use wfl::interpreter::Interpreter;
use wfl::interpreter::value::Value;
use wfl::stdlib::crypto_async::route;

#[test]
fn configured_hashing_bounds_pending_work_and_recovers_after_cancellation() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let interpreter = Interpreter::new();
        let factory = interpreter
            .global_env()
            .borrow()
            .get("password_hash_policy")
            .unwrap();
        let Value::NativeFunction(_, factory) = factory else {
            panic!("policy factory missing")
        };
        let policy = factory(vec![
            Value::Number(19456.0),
            Value::Number(2.0),
            Value::Number(1.0),
        ])
        .unwrap();
        let args = [Value::Text(Arc::from("cancelled-secret")), policy];

        // Unpolled futures reserve admission and release it immediately on drop.
        let reservations: Vec<_> = (0..16)
            .map(|_| route("hash_password_with_policy", &args).unwrap())
            .collect();
        let busy = route("hash_password_with_policy", &args)
            .unwrap()
            .await
            .unwrap_err();
        assert!(busy.message.contains("busy"));
        assert!(!busy.message.contains("cancelled-secret"));
        drop(reservations);

        // Hold the only worker to make cancellation timing observable without
        // sleep-based races. Two jobs enter the pool's queue, then lose callers.
        let (release, wait) = std::sync::mpsc::channel();
        let (ready, started) = tokio::sync::oneshot::channel();
        let blocker = tokio::task::spawn_blocking(move || {
            ready.send(()).unwrap();
            wait.recv().unwrap();
        });
        started.await.unwrap();
        for _ in 0..2 {
            let mut job = route("hash_password_with_policy", &args).unwrap();
            std::future::poll_fn(|context| {
                assert!(matches!(job.as_mut().poll(context), Poll::Pending));
                Poll::Ready(())
            })
            .await;
            drop(job);
        }
        let mut pending: Vec<_> = (0..14)
            .map(|_| route("hash_password_with_policy", &args).unwrap())
            .collect();
        let error = route("hash_password_with_policy", &args)
            .unwrap()
            .await
            .unwrap_err();
        assert!(
            error.message.contains("busy"),
            "cancelled workers must retain their admission slots"
        );
        let followup = pending.pop().unwrap();
        drop(pending);
        release.send(()).unwrap();
        blocker.await.unwrap();
        let recovered = tokio::time::timeout(Duration::from_secs(30), followup)
            .await
            .expect("cancelled jobs must finish and release worker permits")
            .expect("subsequent hash must succeed");
        assert!(matches!(recovered, Value::Text(_)));
    });
    // Waiting for shutdown also proves no worker was left blocked by cancellation.
    runtime.shutdown_timeout(Duration::from_secs(30));
}
