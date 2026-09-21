//! Real-CLI lifecycle bound for file-backed transactions (issue #743).
//!
//! The Windows integration runner's 30-second deadline is the public contract
//! this protects: a program that opens, transacts on, and closes a file-backed
//! SQLite pool must exit with its test report visible, not sit in pool
//! acquire/close until the runner reports TIMEOUT.

use std::fs;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};
use tempfile::{NamedTempFile, TempDir};

mod common;

/// Must stay below the integration runner's unchanged 30-second program limit.
const PROCESS_BOUND: Duration = Duration::from_secs(15);

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn file_backed_transaction_program_exits_before_runner_deadline() {
    let directory = TempDir::new().expect("isolated transaction fixture");
    fs::write(
        directory.path().join("tx.test.wfl"),
        r#"
store db_url as "sqlite://tx.db"

describe "file-backed transactions exit":
    test "a failed block rolls back":
        open database at db_url as db
        store made as execute db with "CREATE TABLE rollback_case (slug TEXT)"
        store failed as no
        try:
            in transaction on db:
                store ins as execute db with "INSERT INTO rollback_case (slug) VALUES ('should-vanish')"
                store boom as execute db with "INSERT INTO no_such_table (x) VALUES (1)"
            end transaction
        when error:
            change failed to yes
        end try
        expect failed to be yes
        store rows as query db with "SELECT slug FROM rollback_case"
        expect length of rows to equal 0
        close database db
    end test

    test "a finished block commits":
        open database at db_url as db
        store made as execute db with "CREATE TABLE commit_case (slug TEXT)"
        in transaction on db:
            store a as execute db with "INSERT INTO commit_case (slug) VALUES ('kept-one')"
            store b as execute db with "INSERT INTO commit_case (slug) VALUES ('kept-two')"
        end transaction
        store rows as query db with "SELECT slug FROM commit_case"
        expect length of rows to equal 2
        close database db
    end test
end describe
"#,
    )
    .expect("write transaction program");

    let global_config = NamedTempFile::new().expect("isolated global configuration");
    fs::write(
        global_config.path(),
        "logging_enabled = false\nexecution_logging = false\ndebug_report_enabled = false\n",
    )
    .expect("disable unrelated log artifacts");
    let stdout = NamedTempFile::new().expect("stdout capture");
    let stderr = NamedTempFile::new().expect("stderr capture");

    let mut child = ChildGuard(
        Command::new(common::wfl_exe())
            .args(["--test", "tx.test.wfl"])
            .current_dir(directory.path())
            .env("WFL_GLOBAL_CONFIG_PATH", global_config.path())
            .stdin(Stdio::null())
            .stdout(stdout.reopen().expect("open stdout capture"))
            .stderr(stderr.reopen().expect("open stderr capture"))
            .spawn()
            .expect("start WFL CLI"),
    );

    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.0.try_wait().expect("poll WFL CLI") {
            break status;
        }
        if start.elapsed() >= PROCESS_BOUND {
            child.0.kill().expect("kill timed-out WFL CLI");
            child.0.wait().expect("reap timed-out WFL CLI");
            panic!(
                "issue #743: file-backed transaction --test exceeded {:?}\nstdout: {}\nstderr: {}",
                PROCESS_BOUND,
                fs::read_to_string(stdout.path()).unwrap_or_default(),
                fs::read_to_string(stderr.path()).unwrap_or_default(),
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    };

    let output = Output {
        status,
        stdout: fs::read(stdout.path()).expect("read stdout capture"),
        stderr: fs::read(stderr.path()).expect("read stderr capture"),
    };
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.status.success(),
        "transaction program should pass: {:?}\n{combined}",
        output.status
    );
    assert!(
        combined.contains("Passed: 2"),
        "test report must be visible after shutdown, got:\n{combined}"
    );
}
