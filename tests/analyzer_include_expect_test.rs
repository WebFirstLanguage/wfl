use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const LEAF: &str =
    "define action called greet with parameters name:\n    return name\nend action\n";

/// Run `wfl` with `args` and return its exit code and combined output.
fn wfl(args: &[&str], path: &Path) -> (Option<i32>, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .args(args)
        .arg(path)
        .env("NO_COLOR", "1")
        .output()
        .expect("run WFL");
    (
        output.status.code(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}

fn analyze(path: &Path) -> (Option<i32>, String) {
    wfl(&["--analyze"], path)
}

#[test]
fn transitive_literal_include_actions_do_not_warn_but_typos_do() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("leaf.wfl"), LEAF).unwrap();
    fs::write(dir.path().join("middle.wfl"), "include from \"leaf.wfl\"\n").unwrap();
    let main = dir.path().join("main.wfl");
    fs::write(
        &main,
        "include from \"middle.wfl\"\nstore good as call greet with \"Ada\"\nstore bad as call grret with \"Ada\"\n",
    )
    .unwrap();

    let (status, output) = analyze(&main);
    assert_eq!(status, Some(1), "warnings must exit 1: {output}");
    assert!(
        !output.contains("Undefined action 'greet'"),
        "included action was reported undefined: {output}"
    );
    assert!(
        output.contains("Undefined action 'grret'"),
        "genuine typo was hidden: {output}"
    );
}

#[test]
fn dynamic_include_keeps_unresolved_action_warning() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("module.wfl"),
        "define action called greet:\n    return \"hi\"\nend action\n",
    )
    .unwrap();
    let main = dir.path().join("main.wfl");
    fs::write(
        &main,
        "store module_path as \"module.wfl\"\ninclude from module_path\nstore good as call greet\n",
    )
    .unwrap();

    let (status, output) = analyze(&main);
    assert_eq!(status, Some(1), "warnings must exit 1: {output}");
    assert!(
        output.contains("Undefined action 'greet'"),
        "a dynamic include cannot be resolved statically: {output}"
    );
}

#[test]
fn call_before_literal_include_keeps_undefined_action_warning() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("leaf.wfl"), LEAF).unwrap();
    let main = dir.path().join("main.wfl");
    fs::write(
        &main,
        "store early as call greet with \"Ada\"\ninclude from \"leaf.wfl\"\nstore late as call greet with \"Bob\"\ndisplay early with late\n",
    )
    .unwrap();

    let (status, output) = analyze(&main);
    assert_eq!(status, Some(1), "the early call must still warn: {output}");
    assert_eq!(
        output.matches("Undefined action 'greet'").count(),
        1,
        "only the call that runs before the include should warn: {output}"
    );
}

#[test]
fn action_body_call_warns_only_when_the_action_can_run_before_the_include() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(dir.path().join("leaf.wfl"), LEAF).unwrap();
    let action = "define action called welcome:\n    return call greet with \"Ada\"\nend action\n";

    // The include runs before any statement that could invoke `welcome`.
    let resolved = dir.path().join("resolved.wfl");
    fs::write(
        &resolved,
        format!(
            "{action}include from \"leaf.wfl\"\nstore message as call welcome\ndisplay message\n"
        ),
    )
    .unwrap();
    let (status, output) = analyze(&resolved);
    assert_eq!(
        status,
        Some(0),
        "include precedes every invocation: {output}"
    );
    assert!(
        output.contains("No static analysis warnings found."),
        "{output}"
    );

    // `welcome` is invoked before the include runs, so `greet` is undefined then.
    let early = dir.path().join("early.wfl");
    fs::write(
        &early,
        format!(
            "{action}store message as call welcome\ninclude from \"leaf.wfl\"\ndisplay message\n"
        ),
    )
    .unwrap();
    let (status, output) = analyze(&early);
    assert_eq!(
        status,
        Some(1),
        "the body can run before the include: {output}"
    );
    assert!(
        output.contains("Undefined action 'greet'"),
        "action body invoked before the include lost its warning: {output}"
    );
}

/// Smallest `max_operations` ceiling under which `wfl <path>` exits 0.
fn minimum_operations(dir: &Path, path: &Path) -> u64 {
    let (mut low, mut high) = (1u64, 200_000u64);
    while low < high {
        let middle = (low + high) / 2;
        fs::write(dir.join(".wflcfg"), format!("max_operations = {middle}\n")).unwrap();
        if wfl(&[], path).0 == Some(0) {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    low
}

fn budget_fixture(dir: &Path) {
    let mut library = String::from(LEAF);
    for index in 0..300 {
        library.push_str(&format!("store filler_{index} as {index}\n"));
    }
    fs::write(dir.join("library.wfl"), library).unwrap();
    // Same program, but the dynamic include is never scanned by the analyzer.
    fs::write(
        dir.join("dynamic.wfl"),
        "store library_path as \"library.wfl\"\ninclude from library_path\nstore good as call greet with \"Ada\"\ndisplay good\n",
    )
    .unwrap();
    fs::write(
        dir.join("literal.wfl"),
        "include from \"library.wfl\"\nstore good as call greet with \"Ada\"\ndisplay good\n",
    )
    .unwrap();
}

#[test]
fn include_scan_does_not_spend_the_execution_operation_budget() {
    let dir = TempDir::new().expect("tempdir");
    budget_fixture(dir.path());

    // The literal program does strictly less runtime work than the dynamic one,
    // so any ceiling that runs the dynamic program must run it too.
    let ceiling = minimum_operations(dir.path(), &dir.path().join("dynamic.wfl"));
    fs::write(
        dir.path().join(".wflcfg"),
        format!("max_operations = {ceiling}\n"),
    )
    .unwrap();
    let (status, output) = wfl(&[], &dir.path().join("literal.wfl"));
    assert_eq!(
        status,
        Some(0),
        "the analyzer's include scan consumed the run's operation budget (ceiling {ceiling}): {output}"
    );
    assert!(output.contains("Ada"), "{output}");
}

#[test]
fn exhausted_include_scan_keeps_warnings_instead_of_failing_analysis() {
    let dir = TempDir::new().expect("tempdir");
    budget_fixture(dir.path());

    // Sweep ceilings from "too small for the entry file" to "enough for the
    // entry file but far too small to scan the 300-statement library". Once
    // the entry file's own analysis completes (its warning is printed), the
    // optional scan must neither report a budget failure nor drop the warning.
    let mut completed = 0;
    for ceiling in 1..=40 {
        fs::write(
            dir.path().join(".wflcfg"),
            format!("max_operations = {ceiling}\n"),
        )
        .unwrap();
        let (status, output) = analyze(&dir.path().join("literal.wfl"));
        if !output.contains("Undefined action 'greet'") {
            continue;
        }
        completed += 1;
        assert!(
            !output.contains("operation budget"),
            "ceiling {ceiling}: an optional scan must not report a budget failure: {output}"
        );
        assert_eq!(status, Some(1), "ceiling {ceiling}: {output}");
    }
    assert!(
        completed > 0,
        "no ceiling let the entry file's analysis finish"
    );
}

#[test]
fn expect_subject_and_expected_value_count_as_variable_uses() {
    let dir = TempDir::new().expect("tempdir");
    let main = dir.path().join("main.wfl");
    fs::write(
        &main,
        "store measured as 2\nstore expected_value as 2\nstore dead as 0\ndescribe \"uses\":\n    test \"assert\":\n        expect measured to equal expected_value\n    end test\nend describe\n",
    )
    .unwrap();

    let (status, output) = analyze(&main);
    assert_eq!(status, Some(1), "warnings must exit 1: {output}");
    assert!(
        !output.contains("Unused variable 'measured'"),
        "assertion subject was reported unused: {output}"
    );
    assert!(
        !output.contains("Unused variable 'expected_value'"),
        "assertion expected value was reported unused: {output}"
    );
    assert!(
        output.contains("Unused variable 'dead'"),
        "genuinely unused variable was hidden: {output}"
    );
}

#[test]
fn expect_property_and_method_receivers_count_as_variable_uses() {
    let dir = TempDir::new().expect("tempdir");
    let main = dir.path().join("main.wfl");
    fs::write(
        &main,
        "store items as [1, 2]\nstore other as [3]\nstore wanted as 1\ndescribe \"uses\":\n    test \"members\":\n        expect items.length to equal 2\n        expect other.size(wanted) to equal 1\n    end test\nend describe\n",
    )
    .unwrap();

    let (status, output) = analyze(&main);
    assert_eq!(status, Some(0), "no variable is unused: {output}");
    for name in ["items", "other", "wanted"] {
        assert!(
            !output.contains(&format!("Unused variable '{name}'")),
            "`{name}` is read by an assertion: {output}"
        );
    }
}
