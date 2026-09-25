use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn analyze(path: &Path) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_wfl"))
        .arg("--analyze")
        .arg(path)
        .output()
        .expect("run WFL analyzer");
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn transitive_literal_include_actions_do_not_warn_but_typos_do() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("leaf.wfl"),
        "define action called greet with parameters name:\n    return name\nend action\n",
    )
    .unwrap();
    fs::write(dir.path().join("middle.wfl"), "include from \"leaf.wfl\"\n").unwrap();
    let main = dir.path().join("main.wfl");
    fs::write(
        &main,
        "include from \"middle.wfl\"\nstore good as call greet with \"Ada\"\nstore bad as call grret with \"Ada\"\n",
    )
    .unwrap();

    let output = analyze(&main);
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

    let output = analyze(&main);
    assert!(
        output.contains("Undefined action 'greet'"),
        "a dynamic include cannot be resolved statically: {output}"
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

    let output = analyze(&main);
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
