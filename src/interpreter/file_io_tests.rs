//! Real-file lifecycle regressions. The sync probe only pauses and observes the
//! production path. Explicit sync-error injection is a component test of error
//! propagation and retry; writes and flushes still use real temporary files.
use super::*;
use futures_util::poll;
use std::task::Poll;
use tokio::sync::Notify;

async fn bounded<F: Future>(future: F) -> F::Output {
    tokio::time::timeout(Duration::from_secs(15), future)
        .await
        .expect("file lifecycle test exceeded 15 seconds")
}

struct SyncGate {
    operation: &'static str,
    reached: Option<oneshot::Sender<()>>,
    resume: Arc<Notify>,
}

#[derive(Default)]
struct ProbeState {
    events: Vec<String>,
    gates: Vec<SyncGate>,
    failures: Vec<(&'static str, io::ErrorKind)>,
}

thread_local! {
    // These tests use Tokio's current-thread runtime. No global hook can affect
    // another test running on a different Rust test-harness thread.
    static SYNC_PROBE: RefCell<Option<Rc<RefCell<ProbeState>>>> = const { RefCell::new(None) };
}

struct SyncProbe(Rc<RefCell<ProbeState>>);

impl SyncProbe {
    fn new() -> Self {
        let state = Rc::new(RefCell::new(ProbeState::default()));
        SYNC_PROBE.with(|probe| {
            assert!(probe.borrow().is_none());
            *probe.borrow_mut() = Some(Rc::clone(&state));
        });
        Self(state)
    }

    fn gate(&self, operation: &'static str) -> (oneshot::Receiver<()>, Arc<Notify>) {
        let (reached, receiver) = oneshot::channel();
        let resume = Arc::new(Notify::new());
        self.0.borrow_mut().gates.push(SyncGate {
            operation,
            reached: Some(reached),
            resume: Arc::clone(&resume),
        });
        (receiver, resume)
    }

    fn events(&self) -> Vec<String> {
        self.0.borrow().events.clone()
    }

    fn fail_next_sync(&self, operation: &'static str, kind: io::ErrorKind) {
        self.0.borrow_mut().failures.push((operation, kind));
    }
}

impl Drop for SyncProbe {
    fn drop(&mut self) {
        SYNC_PROBE.with(|probe| *probe.borrow_mut() = None);
    }
}

pub(super) async fn before_sync(operation: &str) -> Option<io::Error> {
    let (gate, failure) = SYNC_PROBE.with(|probe| {
        let Some(probe) = probe.borrow().as_ref().cloned() else {
            return (None, None);
        };
        let mut state = probe.borrow_mut();
        state.events.push(operation.to_string());
        let gate = state
            .gates
            .iter()
            .position(|gate| gate.operation == operation)
            .map(|index| state.gates.remove(index));
        let failure = state
            .failures
            .iter()
            .position(|(name, _)| *name == operation)
            .map(|index| state.failures.remove(index).1);
        (gate, failure)
    });
    if let Some(mut gate) = gate {
        let _ = gate.reached.take().unwrap().send(());
        gate.resume.notified().await;
    }
    failure.map(|kind| io::Error::new(kind, "injected file sync failure"))
}

fn client() -> IoClient {
    IoClient::new(Arc::new(WflConfig::default()))
}

async fn open(client: &IoClient, path: &std::path::Path, mode: FileOpenMode) -> Arc<str> {
    client
        .open_file_with_mode(path.to_str().unwrap(), mode)
        .await
        .expect("open real fixture")
}

#[tokio::test]
async fn successful_writes_sync_once_and_clean_close_does_not_sync_again() {
    bounded(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("durability.txt");
        let client = client();
        let probe = SyncProbe::new();
        let handle = open(&client, &path, FileOpenMode::Write).await;
        client.write_file(&handle, "first").await.unwrap();
        client.append_file(&handle, " second").await.unwrap();
        client.close_file(&handle).await.unwrap();
        client.close_file(&handle).await.unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"first second");
        let binary_path = directory.path().join("binary.dat");
        let binary = open(&client, &binary_path, FileOpenMode::WriteBinary).await;
        client.write_binary(&binary, &[0, 127, 255]).await.unwrap();
        client.close_file(&binary).await.unwrap();
        assert_eq!(std::fs::read(binary_path).unwrap(), [0, 127, 255]);
        assert_eq!(probe.events(), ["write", "append", "write_binary"]);
    })
    .await;
}

#[tokio::test]
async fn read_only_close_never_requests_disk_sync() {
    bounded(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("readonly.txt");
        std::fs::write(&path, "unchanged").unwrap();
        let client = client();
        let probe = SyncProbe::new();
        let handle = open(&client, &path, FileOpenMode::Read).await;
        assert_eq!(
            client
                .read_file(&handle, &ExecutionBudget::default())
                .await
                .unwrap(),
            "unchanged"
        );
        client.close_file(&handle).await.unwrap();
        assert!(
            probe.events().is_empty(),
            "read-only close requested sync: {:?}",
            probe.events()
        );
    })
    .await;
}

#[tokio::test]
async fn create_and_truncate_without_writing_are_synced_on_close() {
    bounded(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("truncated.txt");
        std::fs::write(&path, "old contents").unwrap();
        let client = client();
        let probe = SyncProbe::new();
        let handle = open(&client, &path, FileOpenMode::Write).await;
        client.close_file(&handle).await.unwrap();
        let created = directory.path().join("created.txt");
        let handle = open(&client, &created, FileOpenMode::Append).await;
        client.close_file(&handle).await.unwrap();
        assert_eq!(probe.events(), ["close", "close"]);
        assert!(std::fs::read(path).unwrap().is_empty());
        assert!(std::fs::read(created).unwrap().is_empty());
    })
    .await;
}

#[tokio::test]
async fn stalled_append_does_not_block_an_unrelated_file() {
    bounded(async {
        let directory = tempfile::tempdir().unwrap();
        let first_path = directory.path().join("first.txt");
        let second_path = directory.path().join("second.txt");
        let client = client();
        let first = open(&client, &first_path, FileOpenMode::Write).await;
        let second = open(&client, &second_path, FileOpenMode::Write).await;
        let probe = SyncProbe::new();
        let (reached, resume) = probe.gate("append");
        let mut append = Box::pin(client.append_file(&first, "slow"));
        tokio::select! {
            result = &mut append => panic!("append ended before reaching sync gate: {result:?}"),
            result = reached => result.unwrap(),
        }
        let sibling =
            tokio::time::timeout(Duration::from_secs(5), client.write_file(&second, "fast")).await;
        resume.notify_one();
        append.await.unwrap();
        assert!(
            sibling.is_ok(),
            "a stalled file blocked its unrelated sibling"
        );
        sibling.unwrap().unwrap();
        client.close_file(&first).await.unwrap();
        client.close_file(&second).await.unwrap();
        assert_eq!(std::fs::read(first_path).unwrap(), b"slow");
        assert_eq!(std::fs::read(second_path).unwrap(), b"fast");
    })
    .await;
}

#[tokio::test]
async fn stalled_close_does_not_block_an_unrelated_file() {
    bounded(async {
        let directory = tempfile::tempdir().unwrap();
        let first_path = directory.path().join("closing.txt");
        let second_path = directory.path().join("sibling.txt");
        let client = client();
        let first = open(&client, &first_path, FileOpenMode::Write).await;
        let second = open(&client, &second_path, FileOpenMode::Write).await;
        let probe = SyncProbe::new();
        let (reached, resume) = probe.gate("close");
        let mut close = Box::pin(client.close_file(&first));
        tokio::select! {
            result = &mut close => panic!("close ended before gate: {result:?}"),
            result = reached => result.unwrap(),
        }
        let sibling = tokio::time::timeout(
            Duration::from_secs(5),
            client.write_file(&second, "independent"),
        )
        .await;
        resume.notify_one();
        close.await.unwrap();
        assert!(sibling.is_ok(), "stalled close blocked a different file");
        sibling.unwrap().unwrap();
        client.close_file(&second).await.unwrap();
        assert_eq!(std::fs::read(second_path).unwrap(), b"independent");
    })
    .await;
}

#[tokio::test]
async fn close_waits_for_the_in_flight_write_to_finish() {
    bounded(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ordering.txt");
        let client = client();
        let handle = open(&client, &path, FileOpenMode::Write).await;
        let probe = SyncProbe::new();
        let (reached, resume) = probe.gate("write");
        let mut write = Box::pin(client.write_file(&handle, "ordered bytes"));
        tokio::select! {
            result = &mut write => panic!("write ended before gate: {result:?}"),
            result = reached => result.unwrap(),
        }
        let mut close = Box::pin(client.close_file(&handle));
        assert!(matches!(poll!(&mut close), Poll::Pending));
        let completed_while_write_is_pending =
            tokio::time::timeout(Duration::from_millis(20), &mut close).await;
        assert!(
            completed_while_write_is_pending.is_err(),
            "close completed while its writer was still paused"
        );
        let events_while_write_is_pending = probe.events();
        resume.notify_one();
        let (written, closed) = tokio::join!(write, close);
        written.unwrap();
        closed.unwrap();
        assert_eq!(
            events_while_write_is_pending,
            ["write"],
            "close reached disk sync before the write finished"
        );
        assert_eq!(std::fs::read(path).unwrap(), b"ordered bytes");
        assert_eq!(probe.events(), ["write"]);
    })
    .await;
}

#[tokio::test]
async fn cancelling_close_keeps_dirty_file_available_for_cleanup() {
    bounded(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cancel-close.txt");
        let client = client();
        let handle = open(&client, &path, FileOpenMode::Write).await;
        let probe = SyncProbe::new();
        let (reached, _resume) = probe.gate("close");
        let mut close = Box::pin(client.close_file(&handle));
        tokio::select! {
            result = &mut close => panic!("close ended before gate: {result:?}"),
            result = reached => result.unwrap(),
        }
        drop(close);
        client.close_file(&handle).await.unwrap();
        assert_eq!(
            probe.events(),
            ["close", "close"],
            "cancelled close discarded unsynchronized state"
        );
        assert!(std::fs::read(path).unwrap().is_empty());
    })
    .await;
}

#[tokio::test]
async fn cancelling_a_write_retains_dirty_state_until_close() {
    bounded(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cancel-write.txt");
        let client = client();
        let handle = open(&client, &path, FileOpenMode::Write).await;
        let probe = SyncProbe::new();
        let (reached, _resume) = probe.gate("write");
        let mut write = Box::pin(client.write_file(&handle, "flushed before cancellation"));
        tokio::select! {
            result = &mut write => panic!("write ended before gate: {result:?}"),
            result = reached => result.unwrap(),
        }
        drop(write);
        client.close_file(&handle).await.unwrap();
        assert_eq!(probe.events(), ["write", "close"]);
        assert_eq!(std::fs::read(path).unwrap(), b"flushed before cancellation");
    })
    .await;
}

#[tokio::test]
async fn a_timed_out_waiting_write_has_no_later_side_effect() {
    bounded(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("timed-out.txt");
        let client = client();
        let handle = open(&client, &path, FileOpenMode::Write).await;
        let probe = SyncProbe::new();
        let (reached, resume) = probe.gate("append");
        let mut append = Box::pin(client.append_file(&handle, "preserved"));
        tokio::select! {
            result = &mut append => panic!("append ended before gate: {result:?}"),
            result = reached => result.unwrap(),
        }
        assert!(
            tokio::time::timeout(
                Duration::from_millis(20),
                client.write_file(&handle, "must never appear")
            )
            .await
            .is_err()
        );
        resume.notify_one();
        append.await.unwrap();
        client.close_file(&handle).await.unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"preserved");
    })
    .await;
}

#[tokio::test]
async fn failed_write_to_read_only_handle_preserves_contents() {
    bounded(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("protected.txt");
        std::fs::write(&path, "keep").unwrap();
        let client = client();
        let handle = open(&client, &path, FileOpenMode::Read).await;
        assert!(client.write_file(&handle, "replace").await.is_err());
        assert!(client.write_binary(&handle, b"binary").await.is_err());
        assert!(client.append_file(&handle, "append").await.is_err());
        client.close_file(&handle).await.unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"keep");
    })
    .await;
}

#[tokio::test]
async fn sync_failure_during_close_remains_retriable() {
    bounded(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("retry-close.txt");
        let client = client();
        let handle = open(&client, &path, FileOpenMode::Write).await;
        let probe = SyncProbe::new();
        let (reached, _resume) = probe.gate("write");
        let mut write = Box::pin(client.write_file(&handle, "flushed real bytes"));
        tokio::select! {
            result = &mut write => panic!("write ended before gate: {result:?}"),
            result = reached => result.unwrap(),
        }
        drop(write);
        probe.fail_next_sync("close", io::ErrorKind::Other);
        let error = client
            .close_file(&handle)
            .await
            .expect_err("sync failure must propagate");
        assert!(error.contains("injected file sync failure"));
        client.close_file(&handle).await.expect("retry close");
        assert_eq!(
            probe.events(),
            ["write", "close", "close"],
            "failed close discarded the dirty descriptor"
        );
        assert_eq!(std::fs::read(path).unwrap(), b"flushed real bytes");
    })
    .await;
}

#[tokio::test]
async fn failed_write_sync_is_propagated_and_close_retries_sync() {
    bounded(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("failed-write-sync.txt");
        let client = client();
        let handle = open(&client, &path, FileOpenMode::Write).await;
        let probe = SyncProbe::new();
        probe.fail_next_sync("write", io::ErrorKind::StorageFull);
        let error = client
            .write_file(&handle, "written before failed sync")
            .await
            .expect_err("sync failure must propagate");
        assert!(error.contains("injected file sync failure"));
        client.close_file(&handle).await.unwrap();
        assert_eq!(probe.events(), ["write", "close"]);
        assert_eq!(std::fs::read(path).unwrap(), b"written before failed sync");
    })
    .await;
}

#[tokio::test]
async fn sync_permission_error_keeps_the_existing_platform_policy() {
    bounded(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("permission-sync.txt");
        let client = client();
        let handle = open(&client, &path, FileOpenMode::Write).await;
        let probe = SyncProbe::new();
        probe.fail_next_sync("write", io::ErrorKind::PermissionDenied);
        let result = client.write_file(&handle, "flushed contents").await;
        #[cfg(windows)]
        assert!(
            result.is_ok(),
            "Windows sync permission handling changed: {result:?}"
        );
        #[cfg(not(windows))]
        assert!(result.is_err(), "non-Windows sync failure must propagate");
        client.close_file(&handle).await.unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"flushed contents");
    })
    .await;
}

#[tokio::test]
async fn failed_opens_cannot_prevent_dead_handle_values_from_being_pruned() {
    bounded(async {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("readable.txt");
        let missing = directory.path().join("missing.txt");
        std::fs::write(&path, "fixture").unwrap();
        let client = client();
        // Retain one closed alias through every sweep. Independent equal text
        // must remain a path while this canonical alias remains a closed handle.
        let alias = open(&client, &path, FileOpenMode::Read).await;
        client.close_file(&alias).await.unwrap();
        for _ in 0..192 {
            let next_id = *client.next_file_id.lock().await;
            if next_id % 64 == 0 {
                assert!(
                    client
                        .open_file_with_mode(missing.to_str().unwrap(), FileOpenMode::Read)
                        .await
                        .is_err()
                );
                continue;
            }
            let handle = open(&client, &path, FileOpenMode::Read).await;
            client.close_file(&handle).await.unwrap();
        }
        assert!(client.is_file_handle(&alias));
        assert!(!client.is_file_handle(&Arc::from(alias.as_ref())));
        assert!(
            client
                .read_file_or_path(&alias, &ExecutionBudget::default())
                .await
                .is_err()
        );
        let retained = client.file_handle_values.lock().unwrap().len();
        assert!(
            retained <= 65,
            "dead handle metadata grew to {retained} entries despite only one live alias"
        );
        assert!(client.file_handles.lock().unwrap().is_empty());
        assert!(!missing.exists());
    })
    .await;
}

#[test]
fn closed_handles_never_become_paths_or_mutate_files() {
    const CHILD: &str = "WFL_ISSUE_732_CLOSED_HANDLE_CHILD";
    if std::env::var_os(CHILD).is_some() {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(bounded(async {
                let client = client();
                let path = std::env::current_dir().unwrap().join("original.txt");
                let handle = open(&client, &path, FileOpenMode::Write).await;
                client.write_file(&handle, "original").await.unwrap();
                client.close_file(&handle).await.unwrap();
                let budget = ExecutionBudget::default();
                assert!(
                    client.read_file(&handle, &budget).await.is_err(),
                    "closed text read succeeded"
                );
                assert!(
                    client.write_file(&handle, "reopened").await.is_err(),
                    "closed text write succeeded"
                );
                assert!(client.read_binary(&handle, &budget).await.is_err());
                assert!(client.read_binary_n(&handle, 1, &budget).await.is_err());
                assert!(client.write_binary(&handle, b"binary").await.is_err());
                assert!(client.append_file(&handle, "append").await.is_err());
                client.close_file(&handle).await.unwrap();
                assert_eq!(std::fs::read(path).unwrap(), b"original");
                assert!(
                    !std::path::Path::new(handle.as_ref()).exists(),
                    "closed handle created a stray path"
                );
            }));
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    struct ChildGuard(std::process::Child);
    impl Drop for ChildGuard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let stdout = tempfile::tempfile().unwrap();
    let stderr = tempfile::tempfile().unwrap();
    let mut child = ChildGuard(
        std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "interpreter::file_io_tests::closed_handles_never_become_paths_or_mutate_files",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .current_dir(directory.path())
            .stdout(stdout.try_clone().unwrap())
            .stderr(stderr.try_clone().unwrap())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "isolated closed-handle child exceeded 20 seconds"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    fn captured(mut file: std::fs::File) -> String {
        use std::io::{Read, Seek};
        file.rewind().unwrap();
        let mut output = String::new();
        file.read_to_string(&mut output).unwrap();
        output
    }
    assert!(
        status.success(),
        "isolated closed-handle test failed:\n{}\n{}",
        captured(stdout),
        captured(stderr)
    );
}
