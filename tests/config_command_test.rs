//! Configuration command contract, exercised through the real CLI and filesystem.

use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use wfl::wfl_config::checker::ConfigChecker;

fn run(dir: &Path, args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .args(args)
        .current_dir(dir)
        .env("WFL_GLOBAL_CONFIG_PATH", dir.join("absent-global.cfg"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn WFL");
    // Drain both pipes while the wizard prints its prompts, avoiding pipe-capacity
    // deadlocks. Closing stdin after the answers also makes EOF tests bounded.
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    let out = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let err = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).unwrap();
        bytes
    });
    let _ = child.stdin.take().unwrap().write_all(input.as_bytes());
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("WFL did not exit within 20 seconds: {args:?}");
        }
        thread::sleep(Duration::from_millis(10));
    };
    Output {
        status,
        stdout: out.join().unwrap(),
        stderr: err.join().unwrap(),
    }
}

fn answers(overrides: &[(&str, &str)]) -> String {
    let checker = ConfigChecker::new();
    let mut input = String::new();
    for (_, mut settings) in checker.get_settings_by_category() {
        settings.sort_by(|a, b| a.name.cmp(&b.name));
        for setting in settings {
            if let Some((_, answer)) = overrides.iter().find(|(name, _)| *name == setting.name) {
                input.push_str(answer);
            }
            input.push('\n');
        }
    }
    input
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn assert_defaults(dir: &Path) {
    let text = fs::read_to_string(dir.join(".wflcfg")).expect("configuration created");
    assert!(text.contains("# Created by wfl config on "), "{text}");
    assert!(text.contains("timeout_seconds = 60"), "{text}");
    assert!(text.contains("allow_shell_execution = false"), "{text}");
    assert!(
        text.contains("web_server_bind_address = 127.0.0.1"),
        "{text}"
    );
    assert!(text.contains("max_call_depth = 1000"), "{text}");
    assert!(
        !text.lines().any(|line| line.starts_with("web_server_tls_")),
        "optional TLS settings must be omitted: {text}"
    );
}

#[test]
fn config_accepts_all_defaults_in_current_directory() {
    let dir = TempDir::new().unwrap();
    let output = run(dir.path(), &["config"], &answers(&[]));
    assert!(output.status.success(), "{}", combined(&output));
    assert_defaults(dir.path());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn config_accepts_existing_target_directory_with_spaces() {
    let dir = TempDir::new().unwrap();
    let target = dir.path().join("my app");
    fs::create_dir(&target).unwrap();
    let output = run(dir.path(), &["config", "my app"], &answers(&[]));
    assert!(output.status.success(), "{}", combined(&output));
    assert_defaults(&target);
    assert!(!dir.path().join(".wflcfg").exists());
}

#[test]
fn removed_init_flag_explains_the_configuration_command_without_writes() {
    let dir = TempDir::new().unwrap();
    let output = run(dir.path(), &["--init"], &answers(&[]));
    assert_eq!(output.status.code(), Some(2), "{}", combined(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("removed") && stderr.contains("wfl config"),
        "{stderr}"
    );
    assert!(!dir.path().join(".wflcfg").exists());
    assert!(
        !combined(&output).contains("Configuration Wizard"),
        "{}",
        combined(&output)
    );
}

#[test]
fn bare_init_explains_the_configuration_command() {
    let dir = TempDir::new().unwrap();
    let output = run(dir.path(), &["init"], "");
    assert_eq!(output.status.code(), Some(2), "{}", combined(&output));
    assert!(
        combined(&output).contains("wfl config"),
        "{}",
        combined(&output)
    );
    assert!(!dir.path().join(".wflcfg").exists());
}

#[test]
fn config_help_does_not_start_the_wizard() {
    for args in [&["--help"][..], &["config", "--help"], &["config", "-h"]] {
        let dir = TempDir::new().unwrap();
        let output = run(dir.path(), args, "");
        assert!(output.status.success(), "{}", combined(&output));
        assert!(
            combined(&output).contains("wfl config [dir]"),
            "{}",
            combined(&output)
        );
        assert!(!dir.path().join(".wflcfg").exists());
    }
}

#[test]
fn config_rejects_missing_directory_and_file_target() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("file.wfl"), "display \"hello\"").unwrap();
    for target in ["missing", "file.wfl"] {
        let output = run(dir.path(), &["config", target], "");
        assert_eq!(output.status.code(), Some(2), "{}", combined(&output));
        assert!(
            combined(&output).contains("valid directory"),
            "{}",
            combined(&output)
        );
        assert!(!dir.path().join(".wflcfg").exists());
    }
    assert!(!dir.path().join("missing").exists());
}

#[test]
fn config_rejects_extra_arguments_and_operation_flags_without_writes() {
    for args in [
        &["config", ".", "extra"][..],
        &["config", "--lint"],
        &["config", ".", "--configFix"],
        &["config", "--unknown"],
        &["--init", ".", "--lint"],
        &["--lint", "--init"],
    ] {
        let dir = TempDir::new().unwrap();
        let output = run(dir.path(), args, "");
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            combined(&output)
        );
        assert!(!dir.path().join(".wflcfg").exists());
        assert!(
            !combined(&output).contains("Configuration Wizard"),
            "{args:?}: {}",
            combined(&output)
        );
    }
}

#[test]
fn config_preserves_existing_file_when_overwrite_is_declined_or_input_ends() {
    for input in ["n\n", "\n", "", "y\n"] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(".wflcfg");
        let original = "# keep my settings\ntimeout_seconds = 17\n";
        fs::write(&path, original).unwrap();
        let output = run(dir.path(), &["config"], input);
        assert_eq!(
            output.status.code(),
            Some(if input == "y\n" { 2 } else { 0 }),
            "{}",
            combined(&output)
        );
        assert_eq!(fs::read_to_string(path).unwrap(), original);
    }
}

#[test]
fn config_overwrites_only_after_confirmation_and_complete_answers() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".wflcfg"), "# old settings\n").unwrap();
    let output = run(dir.path(), &["config"], &format!("y\n{}", answers(&[])));
    assert!(output.status.success(), "{}", combined(&output));
    assert_defaults(dir.path());
}

#[test]
fn config_eof_does_not_create_a_partial_configuration() {
    let dir = TempDir::new().unwrap();
    let output = run(dir.path(), &["config"], "\n");
    assert_eq!(output.status.code(), Some(2), "{}", combined(&output));
    assert!(!dir.path().join(".wflcfg").exists());
}

#[test]
fn config_reprompts_invalid_input_and_preserves_explicit_tls_paths() {
    let dir = TempDir::new().unwrap();
    let input = answers(&[
        ("debug_report_enabled", "invalid\nno"),
        ("web_server_tls_cert_file", "certs/my cert.pem"),
        ("web_server_tls_key_file", "certs/my key.pem"),
    ]);
    let output = run(dir.path(), &["config"], &input);
    assert!(output.status.success(), "{}", combined(&output));
    assert!(
        combined(&output).contains("Invalid boolean value"),
        "{}",
        combined(&output)
    );
    let text = fs::read_to_string(dir.path().join(".wflcfg")).unwrap();
    assert!(text.contains("debug_report_enabled = false"), "{text}");
    assert!(
        text.contains("web_server_tls_cert_file = certs/my cert.pem"),
        "{text}"
    );
    assert!(
        text.contains("web_server_tls_key_file = certs/my key.pem"),
        "{text}"
    );
}

#[test]
fn existing_scripts_named_config_or_init_still_run() {
    for name in ["config", "init"] {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(name), "display \"script executed\"\n").unwrap();
        let output = run(dir.path(), &[name], "");
        assert!(output.status.success(), "{}", combined(&output));
        assert!(
            combined(&output).contains("script executed"),
            "{}",
            combined(&output)
        );
        assert!(!dir.path().join(".wflcfg").exists());
    }
}

#[test]
fn script_arguments_named_config_or_init_remain_script_arguments() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("main.wfl"), "display \"script executed\"\n").unwrap();
    let output = run(dir.path(), &["main.wfl", "config", "init", "--init"], "");
    assert!(output.status.success(), "{}", combined(&output));
    assert!(
        combined(&output).contains("script executed"),
        "{}",
        combined(&output)
    );
    assert!(!dir.path().join(".wflcfg").exists());
}
