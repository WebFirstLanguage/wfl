//! Integration tests for the end-to-end package workflow:
//! create → add → remove → update.

use std::fs;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helper: create a project and return (TempDir, project_dir PathBuf)
// ---------------------------------------------------------------------------
fn create_test_project(name: &str) -> (TempDir, std::path::PathBuf) {
    let temp = TempDir::new().unwrap();
    let project_dir = wflpkg::commands::create::create_project(Some(name), temp.path()).unwrap();
    (temp, project_dir)
}

// ===========================================================================
// create_project tests
// ===========================================================================

#[test]
fn test_create_project_produces_expected_files() {
    let (_temp, project_dir) = create_test_project("my-test-app");
    assert!(project_dir.join("project.wfl").exists());
    assert!(project_dir.join("src/main.wfl").exists());
    assert!(project_dir.join(".wflcfg").exists());
    assert!(project_dir.join(".gitignore").exists());
}

#[test]
fn test_create_project_manifest_has_correct_fields() {
    let (_temp, project_dir) = create_test_project("my-test-app");
    let manifest = wflpkg::ProjectManifest::load(&project_dir.join("project.wfl")).unwrap();
    assert_eq!(manifest.name, "my-test-app");
    assert!(!manifest.version_string.is_empty());
    assert_eq!(manifest.description, "A new WFL project");
}

#[test]
fn test_create_project_src_main_contains_hello() {
    let (_temp, project_dir) = create_test_project("my-test-app");
    let main_content = fs::read_to_string(project_dir.join("src/main.wfl")).unwrap();
    assert!(
        main_content.contains("display") || main_content.contains("Hello"),
        "main.wfl should contain a display/hello statement: {main_content}"
    );
}

#[test]
fn test_create_project_invalid_name_fails() {
    let temp = TempDir::new().unwrap();
    let result = wflpkg::commands::create::create_project(Some("BAD-NAME"), temp.path());
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("not valid"),
        "expected InvalidPackageName: {msg}"
    );
}

// ===========================================================================
// add_dependency tests
// ===========================================================================

#[test]
fn test_add_dependency_updates_manifest() {
    let (_temp, project_dir) = create_test_project("add-test");
    let args: Vec<String> = vec!["http-client".to_string()];
    wflpkg::commands::add::add_dependency(&args, &project_dir).unwrap();

    let manifest = wflpkg::ProjectManifest::load(&project_dir.join("project.wfl")).unwrap();
    assert!(
        manifest.find_dependency("http-client").is_some(),
        "http-client should be in dependencies"
    );
}

#[test]
fn test_add_dependency_with_version_constraint() {
    let (_temp, project_dir) = create_test_project("add-ver-test");
    let args: Vec<String> = "json-parser 26.1 or newer"
        .split_whitespace()
        .map(String::from)
        .collect();
    wflpkg::commands::add::add_dependency(&args, &project_dir).unwrap();

    let manifest = wflpkg::ProjectManifest::load(&project_dir.join("project.wfl")).unwrap();
    let dep = manifest.find_dependency("json-parser").unwrap();
    assert_eq!(
        dep.constraint,
        wflpkg::VersionConstraint::OrNewer(wflpkg::Version::new(26, 1, None))
    );
}

#[test]
fn test_add_dependency_dev_flag() {
    let (_temp, project_dir) = create_test_project("add-dev-test");
    let args: Vec<String> = "test-runner 26.1 or newer for development"
        .split_whitespace()
        .map(String::from)
        .collect();
    wflpkg::commands::add::add_dependency(&args, &project_dir).unwrap();

    let manifest = wflpkg::ProjectManifest::load(&project_dir.join("project.wfl")).unwrap();
    let dep = manifest.find_dependency("test-runner").unwrap();
    assert!(dep.dev_only, "test-runner should be a dev dependency");
}

#[test]
fn test_add_dependency_no_manifest_fails() {
    let temp = TempDir::new().unwrap();
    let args: Vec<String> = vec!["http-client".to_string()];
    let result = wflpkg::commands::add::add_dependency(&args, temp.path());
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("could not find a project.wfl"),
        "expected ManifestNotFound: {msg}"
    );
}

// ===========================================================================
// remove_dependency tests
// ===========================================================================

#[test]
fn test_remove_dependency_updates_manifest() {
    let (_temp, project_dir) = create_test_project("remove-test");
    // Add first
    let args: Vec<String> = vec!["http-client".to_string()];
    wflpkg::commands::add::add_dependency(&args, &project_dir).unwrap();
    // Remove
    wflpkg::commands::remove::remove_dependency("http-client", &project_dir).unwrap();

    let manifest = wflpkg::ProjectManifest::load(&project_dir.join("project.wfl")).unwrap();
    assert!(
        manifest.find_dependency("http-client").is_none(),
        "http-client should be removed"
    );
}

#[test]
fn test_remove_dependency_cleans_packages_dir() {
    let (_temp, project_dir) = create_test_project("remove-clean-test");
    // Add dependency so it is in the manifest
    let args: Vec<String> = vec!["http-client".to_string()];
    wflpkg::commands::add::add_dependency(&args, &project_dir).unwrap();
    // Simulate an installed package directory
    let pkg_dir = project_dir.join("packages").join("http-client");
    fs::create_dir_all(&pkg_dir).unwrap();
    fs::write(pkg_dir.join("lib.wfl"), "// stub").unwrap();

    wflpkg::commands::remove::remove_dependency("http-client", &project_dir).unwrap();
    assert!(!pkg_dir.exists(), "packages/http-client should be cleaned");
}

#[test]
fn test_remove_dependency_rejects_invalid_name() {
    let (_temp, project_dir) = create_test_project("remove-name-test");
    let result = wflpkg::commands::remove::remove_dependency("../outside", &project_dir);
    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("not valid"),
        "path-like dependency names must be rejected before path construction"
    );
}

#[cfg(unix)]
#[test]
fn test_remove_dependency_rejects_symlinked_packages_root() {
    use std::os::unix::fs::symlink;

    let (temp, project_dir) = create_test_project("remove-root-link-test");
    wflpkg::commands::add::add_dependency(&["http-client".to_string()], &project_dir).unwrap();

    let outside = temp.path().join("outside-packages");
    let outside_package = outside.join("http-client");
    fs::create_dir_all(&outside_package).unwrap();
    let sentinel = outside_package.join("sentinel.txt");
    fs::write(&sentinel, "must survive").unwrap();
    symlink(&outside, project_dir.join("packages")).unwrap();

    let result = wflpkg::commands::remove::remove_dependency("http-client", &project_dir);
    assert!(
        result.is_err(),
        "a symlinked packages root must be rejected"
    );
    assert!(
        result.unwrap_err().to_string().contains("symbolic link"),
        "the error should explain why recursive removal was refused"
    );
    assert!(sentinel.exists(), "outside package content must survive");

    let manifest = wflpkg::ProjectManifest::load(&project_dir.join("project.wfl")).unwrap();
    assert!(
        manifest.find_dependency("http-client").is_some(),
        "a refused removal must not change the manifest"
    );
}

#[cfg(unix)]
#[test]
fn test_remove_dependency_rejects_symlinked_package_target() {
    use std::os::unix::fs::symlink;

    let (temp, project_dir) = create_test_project("remove-target-link-test");
    wflpkg::commands::add::add_dependency(&["http-client".to_string()], &project_dir).unwrap();

    let outside = temp.path().join("outside-package");
    fs::create_dir_all(&outside).unwrap();
    let sentinel = outside.join("sentinel.txt");
    fs::write(&sentinel, "must survive").unwrap();
    fs::create_dir_all(project_dir.join("packages")).unwrap();
    symlink(&outside, project_dir.join("packages/http-client")).unwrap();

    let result = wflpkg::commands::remove::remove_dependency("http-client", &project_dir);
    assert!(
        result.is_err(),
        "a symlinked package target must be rejected"
    );
    assert!(
        result.unwrap_err().to_string().contains("symbolic link"),
        "the error should explain why recursive removal was refused"
    );
    assert!(sentinel.exists(), "outside package content must survive");
}

#[cfg(unix)]
#[test]
fn test_remove_dependency_rejects_symlinked_project_manifest() {
    use std::os::unix::fs::symlink;

    let (temp, project_dir) = create_test_project("remove-manifest-link-test");
    wflpkg::commands::add::add_dependency(&["http-client".to_string()], &project_dir).unwrap();

    let project_manifest = project_dir.join("project.wfl");
    let outside_manifest = temp.path().join("outside-project.wfl");
    let original = fs::read_to_string(&project_manifest).unwrap();
    fs::write(&outside_manifest, &original).unwrap();
    fs::remove_file(&project_manifest).unwrap();
    symlink(&outside_manifest, &project_manifest).unwrap();

    let result = wflpkg::commands::remove::remove_dependency("http-client", &project_dir);
    assert!(
        result.is_err(),
        "a symlinked project manifest must be rejected"
    );
    assert!(
        result.unwrap_err().to_string().contains("symbolic link"),
        "the error should identify the unsafe manifest"
    );
    assert_eq!(
        fs::read_to_string(&outside_manifest).unwrap(),
        original,
        "a refused removal must not rewrite an outside manifest"
    );
}

#[test]
fn test_remove_dependency_not_found() {
    let (_temp, project_dir) = create_test_project("remove-nf-test");
    let result = wflpkg::commands::remove::remove_dependency("nonexistent", &project_dir);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("not listed"),
        "expected 'not listed', got: {msg}"
    );
}

#[test]
fn test_remove_dependency_no_manifest_fails() {
    let temp = TempDir::new().unwrap();
    let result = wflpkg::commands::remove::remove_dependency("http-client", temp.path());
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("could not find a project.wfl"),
        "expected ManifestNotFound: {msg}"
    );
}

// ===========================================================================
// update_dependencies tests
// ===========================================================================

#[test]
fn test_update_all_not_implemented() {
    let (_temp, project_dir) = create_test_project("update-all-test");
    // Add a dependency so there's something to update
    let args: Vec<String> = vec!["http-client".to_string()];
    wflpkg::commands::add::add_dependency(&args, &project_dir).unwrap();

    let result = wflpkg::commands::update::update_dependencies(None, &project_dir);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("not yet implemented"),
        "expected 'not yet implemented', got: {msg}"
    );
}

#[test]
fn test_update_specific_not_implemented() {
    let (_temp, project_dir) = create_test_project("update-spec-test");
    let args: Vec<String> = vec!["http-client".to_string()];
    wflpkg::commands::add::add_dependency(&args, &project_dir).unwrap();

    let result = wflpkg::commands::update::update_dependencies(Some("http-client"), &project_dir);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("not yet implemented"),
        "expected 'not yet implemented', got: {msg}"
    );
}

#[test]
fn test_update_unknown_package() {
    let (_temp, project_dir) = create_test_project("update-unk-test");
    let result = wflpkg::commands::update::update_dependencies(Some("nonexistent"), &project_dir);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("not listed"),
        "expected 'not listed', got: {msg}"
    );
}

#[test]
fn test_update_no_manifest_fails() {
    let temp = TempDir::new().unwrap();
    let result = wflpkg::commands::update::update_dependencies(None, temp.path());
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("could not find a project.wfl"),
        "expected ManifestNotFound: {msg}"
    );
}

// ===========================================================================
// Full roundtrip
// ===========================================================================

#[test]
fn test_full_roundtrip_create_add_remove() {
    let (_temp, project_dir) = create_test_project("roundtrip-test");

    // Add two dependencies
    let args1: Vec<String> = vec!["http-client".to_string()];
    wflpkg::commands::add::add_dependency(&args1, &project_dir).unwrap();

    let args2: Vec<String> = "json-parser any version"
        .split_whitespace()
        .map(String::from)
        .collect();
    wflpkg::commands::add::add_dependency(&args2, &project_dir).unwrap();

    // Verify both exist
    let manifest = wflpkg::ProjectManifest::load(&project_dir.join("project.wfl")).unwrap();
    assert_eq!(manifest.dependencies.len(), 2);

    // Remove one
    wflpkg::commands::remove::remove_dependency("http-client", &project_dir).unwrap();

    // Verify final state
    let manifest = wflpkg::ProjectManifest::load(&project_dir.join("project.wfl")).unwrap();
    assert_eq!(manifest.dependencies.len(), 1);
    assert!(manifest.find_dependency("json-parser").is_some());
    assert!(manifest.find_dependency("http-client").is_none());
}
