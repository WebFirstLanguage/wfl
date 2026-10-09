//! Issue #704: exact stdout bytes through the real CLI and nested captures.
mod common;

use std::fs;
use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::time::Duration;
use tempfile::TempDir;

fn run(dir: &TempDir, source: &str) -> Output {
    let path = dir.path().join("main.wfl");
    fs::write(&path, source).unwrap();
    Command::new(common::wfl_exe())
        .args(["--execution-timeout", "5"])
        .arg(path)
        .current_dir(dir.path())
        .output()
        .unwrap()
}

fn assert_output(output: Output, expected: &[u8]) {
    assert!(output.status.success(), "{:?}", output);
    assert!(output.stderr.is_empty(), "{:?}", output);
    assert_eq!(output.stdout, expected);
}

#[test]
fn writes_exact_text_bytes_including_empty_unicode_and_control_characters() {
    let dir = TempDir::new().unwrap();
    assert_output(
        run(
            &dir,
            "call write_stdout with \"\"\ncall write_stdout with \"abc\"\ncall write_stdout with \"é🙂\\0\\t\\r\\n\"\n",
        ),
        "abcé🙂\0\t\r\n".as_bytes(),
    );
}

#[test]
fn partial_writes_compose_with_display_and_print_in_order() {
    let dir = TempDir::new().unwrap();
    assert_output(
        run(
            &dir,
            "call write_stdout with \"a\"\ncall write_stdout with \"b\"\ndisplay \"c\"\ncall print with \"d\" and \"e\"\ncall write_stdout with \"f\"\n",
        ),
        b"abc\nd e\nf",
    );
}

#[test]
fn nested_capture_keeps_partial_lines_and_the_final_unterminated_fragment() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("inner.wfl"),
        "call write_stdout with \"inner\"\n",
    )
    .unwrap();
    fs::write(dir.path().join("outer.wfl"), "call write_stdout with \"prefix:\"\nexecute file at \"inner.wfl\" and read output as inner_output\ncall write_stdout with inner_output\ndisplay \"!\"\ncall write_stdout with \"tail\"\n").unwrap();
    assert_output(
        run(
            &dir,
            "execute file at \"outer.wfl\" and read output as child_output\ncall write_stdout with child_output\n",
        ),
        b"prefix:inner!\ntail",
    );
}

#[test]
fn failed_child_capture_is_removed_before_parent_output() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("child.wfl"),
        "call write_stdout with \"discarded\"\ncall raise_error with \"child failed\"\n",
    )
    .unwrap();
    assert_output(
        run(
            &dir,
            "try:\n    execute file at \"child.wfl\" and read output as child_output\n    call write_stdout with child_output\nwhen error:\n    call write_stdout with \"recovered\"\nend try\ndisplay \"!\"\n",
        ),
        b"recovered!\n",
    );
}

#[test]
fn invalid_arguments_fail_without_stdout_output() {
    let dir = TempDir::new().unwrap();
    for source in [
        "call write_stdout\n",
        "call write_stdout with \"a\" and \"b\"\n",
        "call write_stdout with 42\n",
        "call write_stdout with nothing\n",
    ] {
        let output = run(&dir, source);
        assert!(!output.status.success(), "{source}: {output:?}");
        assert!(output.stdout.is_empty(), "{source}: {output:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("write_stdout"),
            "{source}: {output:?}"
        );
    }
}

#[test]
fn native_validation_rejects_bad_arguments_even_without_static_checks() {
    use wfl::interpreter::{Interpreter, value::Value};
    let interpreter = Interpreter::new();
    let native = interpreter
        .global_env()
        .borrow()
        .get("write_stdout")
        .unwrap();
    let Value::NativeFunction(_, write) = native else {
        panic!("expected a native writer")
    };
    for args in [
        vec![],
        vec![Value::Number(42.0)],
        vec![Value::Nothing],
        vec![Value::Text("a".into()), Value::Text("b".into())],
    ] {
        let error = write(args).expect_err("invalid arguments must fail at the runtime boundary");
        assert!(error.message.contains("write_stdout"));
    }
}

#[test]
fn static_contract_accepts_text_and_rejects_numbers() {
    use wfl::{
        analyzer::Analyzer, lexer::lex_wfl_with_positions, parser::Parser, typechecker::TypeChecker,
    };
    let tokens = lex_wfl_with_positions("call write_stdout with \"abc\"\n");
    let program = Parser::new(&tokens).parse().unwrap();
    let mut analyzer = Analyzer::new();
    analyzer.analyze(&program).unwrap();
    TypeChecker::with_analyzer(analyzer)
        .check_types(&program)
        .unwrap();
    let tokens = lex_wfl_with_positions("call write_stdout with 42\n");
    let program = Parser::new(&tokens).parse().unwrap();
    let diagnostics = TypeChecker::new()
        .check_types(&program)
        .unwrap_err()
        .into_diagnostics();
    assert!(
        diagnostics
            .iter()
            .any(|error| error.message.contains("expected Text")),
        "{diagnostics:?}"
    );
}

#[test]
fn an_existing_constant_named_write_stdout_still_concatenates() {
    let dir = TempDir::new().unwrap();
    assert_output(
        run(
            &dir,
            "store new constant write_stdout as \"prefix\"\ndisplay write_stdout with \"value\"\n",
        ),
        b"prefixvalue\n",
    );
}

#[test]
fn unterminated_output_is_flushed_before_the_program_finishes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("main.wfl");
    fs::write(
        &path,
        "call write_stdout with \"ready\"\nwait for 10 seconds\n",
    )
    .unwrap();
    let mut child = Command::new(common::wfl_exe())
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut bytes = [0; 5];
        let result = stdout.read_exact(&mut bytes).map(|()| bytes);
        let _ = sender.send(result);
    });
    let observed = receiver.recv_timeout(Duration::from_secs(5));
    let still_running = child.try_wait().unwrap().is_none();
    let _ = child.kill();
    child.wait().unwrap();
    reader.join().unwrap();
    assert!(
        still_running,
        "the write must be observable while the program is waiting"
    );
    assert_eq!(observed.unwrap().unwrap(), *b"ready");
}

#[test]
fn a_closed_stdout_pipe_returns_a_runtime_error() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("main.wfl");
    // Exceeds a pipe buffer, so the writer cannot finish before we close it.
    fs::write(
        &path,
        format!("call write_stdout with \"{}\"\n", "x".repeat(1_000_000)),
    )
    .unwrap();
    let mut child = Command::new(common::wfl_exe())
        .args(["--execution-timeout", "5"])
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("could not write to stdout"), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
}
