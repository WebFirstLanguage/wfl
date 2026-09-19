//! Project initialization contracts through the real CLI, configuration, and filesystem.

use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use wfl::wfl_config::ConfigChecker;

const SCAFFOLD: [&str; 3] = [".wflcfg", "AGENTS.md", "CLAUDE.md"];

/// Keep stdin open without sending answers: initialization must never prompt.
fn run(dir: &Path, args: &[&str]) -> Output {
    run_with_global(dir, args, &dir.join("system settings").join("wfl.cfg"))
}

fn run_with_global(dir: &Path, args: &[&str], global: &Path) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .args(args)
        .current_dir(dir)
        .env("WFL_GLOBAL_CONFIG_PATH", global)
        .env("TERM", "dumb")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn WFL");
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
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            let _ = out.join();
            let _ = err.join();
            panic!("WFL did not exit without input within 20 seconds: {args:?}");
        }
        thread::sleep(Duration::from_millis(10));
    };
    Output {
        status,
        stdout: out.join().unwrap(),
        stderr: err.join().unwrap(),
    }
}

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn entries(dir: &Path) -> BTreeSet<String> {
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect()
}

fn assert_scaffold(dir: &Path) {
    assert_eq!(
        entries(dir),
        SCAFFOLD.into_iter().map(String::from).collect()
    );
    for name in SCAFFOLD {
        assert!(dir.join(name).is_file(), "missing {name}");
    }
}

fn assert_report(output: &Output, action: &str, name: &str) {
    assert!(
        combined(output)
            .lines()
            .any(|line| line.to_lowercase().contains(action) && line.contains(name)),
        "expected {action} report for {name}: {}",
        combined(output)
    );
}

#[test]
fn init_creates_only_project_configuration_and_agent_guidance_without_input() {
    let parent = TempDir::new().unwrap();
    let project = parent.path().join("my WFL project café");
    fs::create_dir(&project).unwrap();
    let output = run(&project, &["init"]);
    assert!(output.status.success(), "{}", combined(&output));
    assert_scaffold(&project);
    for name in SCAFFOLD {
        assert_report(&output, "created", name);
    }
    let config = project.join(".wflcfg");
    let issues = ConfigChecker::new().check_config_file(&config).unwrap();
    assert!(
        issues.is_empty(),
        "invalid generated configuration: {issues:?}"
    );
    let output = run(&project, &["--configCheck"]);
    assert!(output.status.success(), "{}", combined(&output));
    assert_scaffold(&project);
    assert_eq!(entries(parent.path()).len(), 1);
}

#[test]
fn generated_agent_adapter_links_to_useful_canonical_guidance() {
    let dir = TempDir::new().unwrap();
    let output = run(dir.path(), &["init"]);
    assert!(output.status.success(), "{}", combined(&output));
    let adapter = fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(adapter.contains("(CLAUDE.md)"), "{adapter}");
    assert!(
        adapter.split_whitespace().count() < 200,
        "adapter should remain thin"
    );
    let guide = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    for required in [
        "https://github.com/WebFirstLanguage/wfl/blob/main/Docs/guides/docker-testing.md",
        "https://context7.com/webfirstlanguage/wfl",
        "wfl-lsp",
        "stdio",
        "Docs/",
        "--lint",
        "--analyze",
        "--test",
        "```wfl",
    ] {
        assert!(
            guide.contains(required),
            "agent guide must include {required}"
        );
    }
}

#[test]
fn init_preserves_customized_files_and_reports_skips_on_rerun() {
    let dir = TempDir::new().unwrap();
    let output = run(dir.path(), &["init"]);
    assert!(output.status.success(), "{}", combined(&output));
    for (index, name) in SCAFFOLD.iter().enumerate() {
        fs::write(dir.path().join(name), [0xff, 0, index as u8]).unwrap();
    }
    let output = run(dir.path(), &["init"]);
    assert!(output.status.success(), "{}", combined(&output));
    assert_scaffold(dir.path());
    for (index, name) in SCAFFOLD.iter().enumerate() {
        assert_eq!(
            fs::read(dir.path().join(name)).unwrap(),
            [0xff, 0, index as u8]
        );
        assert_report(&output, "skipped", name);
    }
}

#[test]
fn init_fills_partial_scaffolds_without_replacing_existing_files() {
    for preserved in SCAFFOLD {
        let dir = TempDir::new().unwrap();
        let original = b"preserve project-specific contents\n";
        fs::write(dir.path().join(preserved), original).unwrap();
        let output = run(dir.path(), &["init"]);
        assert!(output.status.success(), "{}", combined(&output));
        assert_scaffold(dir.path());
        assert_eq!(fs::read(dir.path().join(preserved)).unwrap(), original);
        for name in SCAFFOLD {
            assert_report(
                &output,
                if name == preserved {
                    "skipped"
                } else {
                    "created"
                },
                name,
            );
        }
    }
}

#[test]
fn init_preserves_read_only_existing_files_and_creates_missing_files() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("CLAUDE.md");
    let original = "# My project instructions\n";
    fs::write(&path, original).unwrap();
    let writable = fs::metadata(&path).unwrap().permissions();
    let mut protected = writable.clone();
    protected.set_readonly(true);
    fs::set_permissions(&path, protected).unwrap();
    let output = run(dir.path(), &["init"]);
    fs::set_permissions(&path, writable).unwrap();
    assert!(output.status.success(), "{}", combined(&output));
    assert_scaffold(dir.path());
    assert_eq!(fs::read_to_string(path).unwrap(), original);
    assert_report(&output, "skipped", "CLAUDE.md");
}

#[test]
fn simultaneous_initializations_finish_with_one_complete_scaffold() {
    let expected = TempDir::new().unwrap();
    let output = run(expected.path(), &["init"]);
    assert!(output.status.success(), "{}", combined(&output));
    let dir = TempDir::new().unwrap();
    let (first, second) = thread::scope(|scope| {
        let first = scope.spawn(|| run(dir.path(), &["init"]));
        let second = scope.spawn(|| run(dir.path(), &["init"]));
        (first.join().unwrap(), second.join().unwrap())
    });
    for output in [first, second] {
        assert!(output.status.success(), "{}", combined(&output));
    }
    assert_scaffold(dir.path());
    for name in SCAFFOLD {
        assert_eq!(
            fs::read(dir.path().join(name)).unwrap(),
            fs::read(expected.path().join(name)).unwrap()
        );
    }
}

#[test]
fn init_preflights_all_reserved_names_before_writing_any_files() {
    for conflict in SCAFFOLD {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(conflict)).unwrap();
        fs::write(dir.path().join(conflict).join("keep.txt"), "keep me").unwrap();
        let output = run(dir.path(), &["init"]);
        assert!(!output.status.success(), "{}", combined(&output));
        assert!(
            combined(&output).contains(conflict),
            "{}",
            combined(&output)
        );
        assert_eq!(entries(dir.path()), [conflict.to_string()].into());
        assert_eq!(
            fs::read_to_string(dir.path().join(conflict).join("keep.txt")).unwrap(),
            "keep me"
        );
    }
}

#[cfg(unix)]
#[test]
fn init_rejects_existing_and_dangling_symlinks_without_writing() {
    use std::os::unix::fs::symlink;

    for name in SCAFFOLD {
        for target_exists in [false, true] {
            let dir = TempDir::new().unwrap();
            let outside = TempDir::new().unwrap();
            let target = outside.path().join("user-owned-file");
            if target_exists {
                fs::write(&target, "never replace me").unwrap();
            }
            symlink(&target, dir.path().join(name)).unwrap();
            let output = run(dir.path(), &["init"]);
            assert!(!output.status.success(), "{}", combined(&output));
            assert!(combined(&output).contains(name), "{}", combined(&output));
            assert_eq!(entries(dir.path()), [name.to_string()].into());
            assert_eq!(fs::read_link(dir.path().join(name)).unwrap(), target);
            if target_exists {
                assert_eq!(fs::read_to_string(target).unwrap(), "never replace me");
            } else {
                assert!(!target.exists());
            }
        }
    }
}

#[test]
fn init_does_not_require_or_modify_global_configuration() {
    for global_is_directory in [false, true] {
        let dir = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        let global = outside.path().join("global-config");
        if global_is_directory {
            fs::create_dir(&global).unwrap();
        } else {
            fs::write(&global, [0xff, 0, 0xfe]).unwrap();
        }
        let output = run_with_global(dir.path(), &["init"], &global);
        assert!(output.status.success(), "{}", combined(&output));
        assert_scaffold(dir.path());
        if global_is_directory {
            assert!(entries(&global).is_empty());
        } else {
            assert_eq!(fs::read(&global).unwrap(), [0xff, 0, 0xfe]);
        }
    }
}

#[test]
fn init_help_and_main_help_advertise_the_command_without_writes() {
    for args in [
        &["--help"][..],
        &["-h"],
        &["init", "--help"],
        &["init", "-h"],
    ] {
        let dir = TempDir::new().unwrap();
        let output = run(dir.path(), args);
        assert!(output.status.success(), "{}", combined(&output));
        assert!(
            combined(&output).contains("wfl init"),
            "{}",
            combined(&output)
        );
        assert!(
            !combined(&output).contains("--init"),
            "{}",
            combined(&output)
        );
        assert!(entries(dir.path()).is_empty());
    }
}

#[test]
fn init_rejects_flags_and_extra_arguments_without_writes() {
    for args in [
        &["--init"][..],
        &["init", "."],
        &["init", "my app"],
        &["init", "extra", "another"],
        &["init", "--lint"],
        &["init", "--configFix"],
        &["init", "--unknown"],
        &["init", "--help", "extra"],
    ] {
        let dir = TempDir::new().unwrap();
        let output = run(dir.path(), args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            combined(&output)
        );
        assert!(
            entries(dir.path()).is_empty(),
            "unexpected writes for {args:?}"
        );
    }
}

#[test]
fn bare_init_is_a_command_even_when_a_program_named_init_exists() {
    let dir = TempDir::new().unwrap();
    let source = "display \"script executed\"\n";
    fs::write(dir.path().join("init"), source).unwrap();
    let output = run(dir.path(), &["init"]);
    assert!(output.status.success(), "{}", combined(&output));
    assert!(!combined(&output).contains("script executed"));
    for name in SCAFFOLD {
        assert!(dir.path().join(name).is_file(), "missing {name}");
    }
    assert_eq!(fs::read_to_string(dir.path().join("init")).unwrap(), source);
}

#[test]
fn explicit_init_program_paths_and_script_arguments_keep_working() {
    let settings = TempDir::new().unwrap();
    let global = settings.path().join("config");
    // Runtime defaults can produce execution logs; isolate that unrelated
    // behavior while asserting command dispatch creates no project files.
    fs::write(
        &global,
        "logging_enabled = false\nexecution_logging = false\ndebug_report_enabled = false\n",
    )
    .unwrap();
    for (name, args) in [
        ("init", vec!["./init"]),
        ("init.wfl", vec!["init.wfl"]),
        ("main.wfl", vec!["main.wfl", "init"]),
    ] {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(name), "display \"script executed\"\n").unwrap();
        let output = run_with_global(dir.path(), &args, &global);
        assert!(output.status.success(), "{}", combined(&output));
        assert!(
            combined(&output).contains("script executed"),
            "{}",
            combined(&output)
        );
        assert_eq!(entries(dir.path()), [name.to_string()].into());
    }
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("init"), "display \"script executed\"\n").unwrap();
    let output = run(dir.path(), &["--lint", "init"]);
    assert!(output.status.success(), "{}", combined(&output));
    assert_eq!(entries(dir.path()), ["init".to_string()].into());
}

#[test]
fn generated_project_config_is_discovered_by_normal_program_runs() {
    let dir = TempDir::new().unwrap();
    let output = run(dir.path(), &["init"]);
    assert!(output.status.success(), "{}", combined(&output));
    let mut config = fs::OpenOptions::new()
        .append(true)
        .open(dir.path().join(".wflcfg"))
        .unwrap();
    writeln!(config, "\nmax_source_size = 1").unwrap();
    fs::write(
        dir.path().join("main.wfl"),
        "display \"must not execute\"\n",
    )
    .unwrap();
    let output = run(dir.path(), &["main.wfl"]);
    assert_eq!(output.status.code(), Some(2), "{}", combined(&output));
    assert!(
        combined(&output).to_lowercase().contains("source"),
        "{}",
        combined(&output)
    );
    assert!(!combined(&output).contains("must not execute"));
    assert!(!dir.path().join("system settings").exists());
}

#[test]
fn generated_wfl_examples_lint_analyze_execute_and_run_their_tests() {
    let dir = TempDir::new().unwrap();
    let output = run(dir.path(), &["init"]);
    assert!(output.status.success(), "{}", combined(&output));
    let guide = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    let mut snippets = Vec::new();
    let mut snippet = None;
    for line in guide.lines() {
        if line.trim() == "```wfl" {
            assert!(snippet.is_none(), "nested WFL fences");
            snippet = Some(String::new());
        } else if line.trim() == "```" && snippet.is_some() {
            snippets.push(snippet.take().unwrap());
        } else if let Some(source) = &mut snippet {
            source.push_str(line);
            source.push('\n');
        }
    }
    assert!(snippet.is_none(), "unclosed WFL fence");
    assert!(
        !snippets.is_empty(),
        "guide must have runnable syntax examples"
    );
    let mut tested_example = false;
    for (index, mut source) in snippets.into_iter().enumerate() {
        let is_test = source
            .lines()
            .any(|line| line.trim_start().starts_with("describe "));
        tested_example |= is_test;
        source.push_str("\ndisplay \"generated example completed\"\n");
        let filename = format!("example_{index}.wfl");
        fs::write(dir.path().join(&filename), source).unwrap();
        for operation in ["--lint", "--analyze"] {
            let output = run(dir.path(), &[operation, &filename]);
            assert!(
                output.status.success(),
                "{filename} {operation}: {}",
                combined(&output)
            );
        }
        let args: Vec<&str> = if is_test {
            vec!["--test", &filename]
        } else {
            vec![&filename]
        };
        let output = run(dir.path(), &args);
        assert!(output.status.success(), "{filename}: {}", combined(&output));
        assert!(
            combined(&output).contains("generated example completed"),
            "{}",
            combined(&output)
        );
        if is_test {
            assert!(
                combined(&output).contains("Failed: 0"),
                "{}",
                combined(&output)
            );
            assert!(
                combined(&output).lines().any(|line| line
                    .strip_prefix("Total:")
                    .and_then(|total| total.trim().parse::<usize>().ok())
                    .is_some_and(|total| total > 0)),
                "example tests must execute: {}",
                combined(&output)
            );
        }
    }
    assert!(
        tested_example,
        "guide must demonstrate WFL's test framework"
    );
}
