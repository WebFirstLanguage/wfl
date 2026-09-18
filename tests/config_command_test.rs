//! Configuration command contract, exercised through the real CLI and filesystem.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use wfl::wfl_config::checker::ConfigChecker;

/// Run the real CLI with a temporary global configuration destination.
fn run(dir: &Path, args: &[&str], input: &str) -> Output {
    run_with_config_path(dir, args, input, &config_path(dir))
}

/// Keep system settings separate from the program's working directory.
fn config_path(dir: &Path) -> PathBuf {
    dir.join("system settings").join("config")
}

/// Feed wizard input and drain output concurrently under a bounded child lifetime.
fn run_with_config_path(dir: &Path, args: &[&str], input: &str, config: &Path) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .args(args)
        .current_dir(dir)
        .env("WFL_GLOBAL_CONFIG_PATH", config)
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

/// Builds answers in prompt order; embedded newlines supply invalid then corrected answers.
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

/// Include stdout and stderr in contract assertions and failure diagnostics.
fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Verifies representative global defaults and the absence of optional TLS paths.
fn assert_defaults(dir: &Path) {
    let text = fs::read_to_string(config_path(dir)).expect("global configuration created");
    assert!(text.contains("# Created by wfl config on "), "{text}");
    assert!(text.contains("timeout_seconds = 60"), "{text}");
    assert!(text.contains("allow_shell_execution = false"), "{text}");
    assert!(
        text.contains("web_server_bind_address = 127.0.0.1"),
        "{text}"
    );
    assert!(text.contains("max_call_depth = 1000"), "{text}");
    assert!(text.contains("outbound_stream_max_seconds = 300"), "{text}");
    assert!(
        !text.lines().any(|line| line.starts_with("web_server_tls_")),
        "optional TLS settings must be omitted: {text}"
    );
}

#[test]
fn config_accepts_all_defaults_in_global_configuration() {
    let dir = TempDir::new().unwrap();
    let output = run(dir.path(), &["config"], &answers(&[]));
    assert!(output.status.success(), "{}", combined(&output));
    assert!(combined(&output).contains("create a WFL configuration file"));
    assert_defaults(dir.path());
    assert!(!dir.path().join(".wflcfg").exists());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn config_preserves_local_configuration() {
    let dir = TempDir::new().unwrap();
    let local = dir.path().join(".wflcfg");
    let original = "# project settings\ntimeout_seconds = 19\n";
    fs::write(&local, original).unwrap();
    let output = run(dir.path(), &["config"], &answers(&[]));
    assert!(output.status.success(), "{}", combined(&output));
    assert_defaults(dir.path());
    assert_eq!(fs::read_to_string(local).unwrap(), original);
}

#[test]
fn config_command_works_when_current_directory_contains_config_file() {
    let dir = TempDir::new().unwrap();
    let existing = dir.path().join("config");
    fs::write(&existing, "display \"script executed\"\n").unwrap();
    let output = run(dir.path(), &["config"], &answers(&[]));
    assert!(output.status.success(), "{}", combined(&output));
    assert_defaults(dir.path());
    assert!(!combined(&output).contains("script executed"));
    assert_eq!(
        fs::read_to_string(existing).unwrap(),
        "display \"script executed\"\n"
    );
}

#[test]
fn config_help_does_not_start_the_wizard() {
    for args in [&["--help"][..], &["config", "--help"], &["config", "-h"]] {
        let dir = TempDir::new().unwrap();
        let output = run(dir.path(), args, "");
        assert!(output.status.success(), "{}", combined(&output));
        assert!(
            combined(&output).contains("wfl config") && combined(&output).contains("global"),
            "{}",
            combined(&output)
        );
        assert!(!combined(&output).contains("[dir]"));
        assert!(!dir.path().join(".wflcfg").exists());
        assert!(!config_path(dir.path()).parent().unwrap().exists());
    }
}

#[test]
fn config_rejects_directory_and_file_arguments() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("file.wfl"), "display \"hello\"").unwrap();
    fs::create_dir(dir.path().join("my app")).unwrap();
    for target in [".", "my app", "missing", "file.wfl"] {
        let output = run(dir.path(), &["config", target], &answers(&[]));
        assert_eq!(output.status.code(), Some(2), "{}", combined(&output));
        assert!(
            combined(&output).contains("does not accept arguments"),
            "{}",
            combined(&output)
        );
        assert!(!dir.path().join(".wflcfg").exists());
        assert!(!config_path(dir.path()).parent().unwrap().exists());
        assert!(!combined(&output).contains("Configuration Wizard"));
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
        assert!(!config_path(dir.path()).parent().unwrap().exists());
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
        let path = config_path(dir.path());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
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
    fs::create_dir_all(config_path(dir.path()).parent().unwrap()).unwrap();
    fs::write(config_path(dir.path()), "# old settings\n").unwrap();
    let output = run(dir.path(), &["config"], &format!("y\n{}", answers(&[])));
    assert!(output.status.success(), "{}", combined(&output));
    assert_defaults(dir.path());
}

#[test]
fn config_prompts_and_writes_outbound_stream_lifetime_on_create_and_overwrite() {
    for value in ["60", "0"] {
        for overwrite in [false, true] {
            let dir = TempDir::new().unwrap();
            let path = config_path(dir.path());
            let mut input = String::new();
            if overwrite {
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(&path, "outbound_stream_max_seconds = 123\n").unwrap();
                input.push_str("y\n");
            }
            input.push_str(&answers(&[("outbound_stream_max_seconds", value)]));

            let output = run(dir.path(), &["config"], &input);
            assert!(output.status.success(), "{}", combined(&output));
            assert!(
                combined(&output).contains("outbound_stream_max_seconds - "),
                "{}",
                combined(&output)
            );
            let text = fs::read_to_string(&path).unwrap();
            assert!(
                text.lines()
                    .any(|line| line == format!("outbound_stream_max_seconds = {value}")),
                "{text}"
            );
            assert!(
                !text.contains("outbound_stream_max_seconds = 123"),
                "{text}"
            );
            assert!(!dir.path().join(".wflcfg").exists());
            let issues = ConfigChecker::new().check_config_file(&path).unwrap();
            assert!(issues.is_empty(), "{issues:?}");
        }
    }
}

#[test]
fn config_reprompts_negative_outbound_stream_lifetime() {
    let dir = TempDir::new().unwrap();
    let input = answers(&[("outbound_stream_max_seconds", "-1\n60")]);
    let output = run(dir.path(), &["config"], &input);
    assert!(output.status.success(), "{}", combined(&output));
    assert!(
        combined(&output).contains("non-negative integer"),
        "{}",
        combined(&output)
    );
    assert!(combined(&output).contains("Please try again."));
    let text = fs::read_to_string(config_path(dir.path())).unwrap();
    assert!(text.contains("outbound_stream_max_seconds = 60"), "{text}");
    assert!(!text.contains("outbound_stream_max_seconds = -1"), "{text}");
    assert!(!dir.path().join(".wflcfg").exists());
}

#[test]
fn config_preserves_read_only_global_configuration() {
    let dir = TempDir::new().unwrap();
    let path = config_path(dir.path());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let original = "# protected global defaults\ntimeout_seconds = 19\n";
    fs::write(&path, original).unwrap();
    let writable = fs::metadata(&path).unwrap().permissions();
    let mut protected = writable.clone();
    protected.set_readonly(true);
    fs::set_permissions(&path, protected).unwrap();

    let output = run(dir.path(), &["config"], &format!("y\n{}", answers(&[])));
    // Restore attributes even when an assertion fails, so TempDir can clean up.
    fs::set_permissions(&path, writable).unwrap();

    assert_eq!(output.status.code(), Some(2), "{}", combined(&output));
    assert!(combined(&output).contains("read-only"));
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn config_eof_does_not_create_a_partial_configuration() {
    let dir = TempDir::new().unwrap();
    let output = run(dir.path(), &["config"], "\n");
    assert_eq!(output.status.code(), Some(2), "{}", combined(&output));
    assert!(!dir.path().join(".wflcfg").exists());
    assert!(!config_path(dir.path()).parent().unwrap().exists());
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
    let text = fs::read_to_string(config_path(dir.path())).unwrap();
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
fn explicit_program_paths_still_run() {
    for name in ["config", "program"] {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(name), "display \"script executed\"\n").unwrap();
        let output = run(dir.path(), &[&format!("./{name}")], "");
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
fn script_arguments_named_config_remain_script_arguments() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("main.wfl"), "display \"script executed\"\n").unwrap();
    let output = run(dir.path(), &["main.wfl", "config"], "");
    assert!(output.status.success(), "{}", combined(&output));
    assert!(
        combined(&output).contains("script executed"),
        "{}",
        combined(&output)
    );
    assert!(!dir.path().join(".wflcfg").exists());
}

#[test]
fn global_configuration_path_can_be_relative() {
    let dir = TempDir::new().unwrap();
    let output = run_with_config_path(dir.path(), &["config"], &answers(&[]), Path::new("wfl.cfg"));
    assert!(output.status.success(), "{}", combined(&output));
    let contents = fs::read_to_string(dir.path().join("wfl.cfg")).unwrap();
    assert!(contents.contains("timeout_seconds = 60"));
    assert!(!dir.path().join(".wflcfg").exists());
}

#[test]
fn global_configuration_write_failure_preserves_existing_files() {
    let dir = TempDir::new().unwrap();
    let blocker = dir.path().join("system settings");
    fs::write(&blocker, "preserve this file").unwrap();
    let output = run(dir.path(), &["config"], &answers(&[]));
    assert_eq!(output.status.code(), Some(2), "{}", combined(&output));
    assert_eq!(fs::read_to_string(blocker).unwrap(), "preserve this file");
    assert!(!dir.path().join(".wflcfg").exists());
}
