//! Exercise the shipped CLI across rewriting and execution, so preservation is
//! demonstrated at the process/file boundary as well as by token comparisons.
use std::fs;
use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Run with isolated configuration and a deadline while draining both pipes.
fn run(directory: &std::path::Path, args: &[&str]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wfl"));
    child.current_dir(directory).args(args);
    child.env(
        "WFL_GLOBAL_CONFIG_PATH",
        directory.join("missing-global.cfg"),
    );
    let mut child = child
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("execute WFL CLI");
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let out = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let err = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("WFL CLI timed out: {args:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    Output {
        status,
        stdout: out.join().unwrap(),
        stderr: err.join().unwrap(),
    }
}

#[test]
/// Compare real program output before/after fixing and demand an empty second diff.
fn formatting_preserves_observable_program_results() {
    let cases = [
        "// Keep expression grouping\nstore total as (1 + 2) times 3\nstore quotient as 24 divided by (2 times 3)\ndisplay total\ndisplay quotient\n",
        "store userName as \"Ada\"  \ndisplay userName\n",
        "store Counter as 10\nstore counter as 20\ndisplay Counter\ndisplay counter\n",
        "store message as \"Grüße 世界 🦀\\nquote: \\\" slash: \\\\\"\r\ndisplay message\r\n",
        "define action called getName:\n give back \"Ada\"\nend action\ncall getName\n",
        "store userName as \"local\"\ncreate map profile:\n userName is \"remote\"\nend map\ndisplay stringify_json of profile\ndisplay userName\n",
        "store flag as YES\ncheck if flag:\ndisplay \"yes\" // preserve this note\notherwise:\ndisplay \"no\"\nend check\n",
    ];
    for source in cases {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("program.wfl");
        fs::write(&path, source).unwrap();
        let before = run(directory.path(), &["program.wfl"]);
        assert!(
            before.status.success(),
            "invalid fixture: {source}\n{}",
            String::from_utf8_lossy(&before.stderr)
        );
        let fixed = run(
            directory.path(),
            &["--lint", "--fix", "program.wfl", "--in-place"],
        );
        assert!(
            fixed.status.success(),
            "fix failed: {source}\n{}",
            String::from_utf8_lossy(&fixed.stderr)
        );
        let after = run(directory.path(), &["program.wfl"]);
        assert!(
            after.status.success(),
            "fixed program failed: {}\n{}",
            fs::read_to_string(&path).unwrap(),
            String::from_utf8_lossy(&after.stderr)
        );
        assert_eq!(
            after.stdout, before.stdout,
            "fix changed execution: {source}"
        );
        let diff = run(
            directory.path(),
            &["--lint", "--fix", "program.wfl", "--diff"],
        );
        assert!(diff.status.success());
        assert!(
            diff.stdout.is_empty(),
            "second fix must be empty: {}",
            String::from_utf8_lossy(&diff.stdout)
        );
    }
}
