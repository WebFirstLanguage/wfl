/// Binary I/O integration tests for WFL
///
/// Tests the full pipeline: parse → analyze → typecheck → interpret
/// for binary file operations.
use std::path::PathBuf;

fn wfl_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("release");
    if cfg!(windows) {
        path.push("wfl.exe");
    } else {
        path.push("wfl");
    }
    path
}

/// Run a WFL script in the given directory.
/// The source is written as `test.wfl` in the dir and executed from there.
fn run_wfl_in(dir: &std::path::Path, source: &str) -> (String, String, bool) {
    let script_path = dir.join("test.wfl");
    std::fs::write(&script_path, source).expect("failed to write script");

    let output = std::process::Command::new(wfl_binary())
        .arg("test.wfl")
        .current_dir(dir)
        .output()
        .expect("failed to run wfl");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status.success())
}

// ============================================================
// Binary read/write roundtrip
// ============================================================

#[test]
fn test_binary_read_write_roundtrip() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    // Write 256 bytes (0x00..0xFF)
    let bytes: Vec<u8> = (0..=255).collect();
    std::fs::write(dir.path().join("test.bin"), &bytes).expect("write bin");

    let source = r#"
open file at "test.bin" for reading binary as rh
store contents as read binary from rh
store sz as file size of rh
display sz
close rh
"#;

    let (stdout, stderr, success) = run_wfl_in(dir.path(), source);
    assert!(success, "wfl failed: {stderr}");
    assert_eq!(stdout.trim(), "256");
}

#[test]
fn test_binary_write_and_read_back() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let original: Vec<u8> = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, 0xFF];
    std::fs::write(dir.path().join("input.bin"), &original).expect("write original");

    let source = r#"
open file at "input.bin" for reading binary as rh
store contents as read binary from rh
close rh

open file at "output.bin" for writing binary as wh
write binary contents into wh
close wh
"#;

    let (stdout, stderr, success) = run_wfl_in(dir.path(), source);
    assert!(success, "wfl failed: {stderr}\nstdout: {stdout}");

    // Verify actual file contents
    let copied = std::fs::read(dir.path().join("output.bin")).expect("read copy");
    assert_eq!(original, copied, "binary roundtrip mismatch");
}

#[test]
fn test_read_n_bytes() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let bytes: Vec<u8> = (0..100).collect();
    std::fs::write(dir.path().join("chunk.bin"), &bytes).expect("write bin");

    let source = r#"
open file at "chunk.bin" for reading binary as rh
store chunk as read 10 bytes from rh
store sz as file size of rh
display sz
close rh
"#;

    let (stdout, stderr, success) = run_wfl_in(dir.path(), source);
    assert!(success, "wfl failed: {stderr}");
    assert_eq!(stdout.trim(), "100");
}

#[test]
fn test_file_size_of() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let bytes = vec![0u8; 4096];
    std::fs::write(dir.path().join("sized.bin"), &bytes).expect("write");

    let source = r#"
open file at "sized.bin" for reading binary as fh
store sz as file size of fh
display sz
close fh
"#;

    let (stdout, stderr, success) = run_wfl_in(dir.path(), source);
    assert!(success, "wfl failed: {stderr}");
    assert_eq!(stdout.trim(), "4096");
}

#[test]
fn test_text_io_still_works() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    std::fs::write(dir.path().join("text.txt"), "hello world").expect("write");

    let source = r#"
open file at "text.txt" for reading as fh
store contents as read content from fh
display contents
close fh
"#;

    let (stdout, stderr, success) = run_wfl_in(dir.path(), source);
    assert!(success, "wfl failed: {stderr}");
    assert_eq!(stdout.trim(), "hello world");
}

#[test]
fn test_parse_binary_syntax() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let source = r#"
open file at "test.bin" for reading binary as rh
open file at "out.bin" for writing binary as wh
close rh
close wh
"#;
    let script = dir.path().join("parse_test.wfl");
    std::fs::write(&script, source).expect("write script");

    let output = std::process::Command::new(wfl_binary())
        .arg("--parse")
        .arg(script.to_str().unwrap())
        .output()
        .expect("failed to run wfl --parse");

    assert!(
        output.status.success(),
        "parse failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
