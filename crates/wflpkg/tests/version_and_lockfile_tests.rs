//! Edge-case tests for version parsing, version constraint matching,
//! lockfile parsing, manifest loading, and error display formatting.

use tempfile::TempDir;
use wflpkg::error::PackageError;
use wflpkg::manifest::version::{Version, VersionConstraint};

// ===========================================================================
// Version parsing edge cases
// ===========================================================================

#[test]
fn test_version_parse_year_only() {
    let v = Version::parse("27").unwrap();
    assert_eq!(v.year, 27);
    assert_eq!(v.month, 1);
    assert_eq!(v.build, None);
}

#[test]
fn test_version_parse_year_month() {
    let v = Version::parse("26.12").unwrap();
    assert_eq!(v.year, 26);
    assert_eq!(v.month, 12);
    assert_eq!(v.build, None);
}

#[test]
fn test_version_parse_full() {
    let v = Version::parse("26.1.3").unwrap();
    assert_eq!(v.year, 26);
    assert_eq!(v.month, 1);
    assert_eq!(v.build, Some(3));
}

#[test]
fn test_version_parse_rejects_month_zero() {
    let result = Version::parse("26.0");
    assert!(result.is_err(), "month 0 should be invalid");
}

#[test]
fn test_version_parse_rejects_month_13() {
    let result = Version::parse("26.13");
    assert!(result.is_err(), "month 13 should be invalid");
}

#[test]
fn test_version_parse_rejects_month_zero_with_build() {
    let result = Version::parse("26.0.1");
    assert!(result.is_err(), "month 0 with build should be invalid");
}

#[test]
fn test_version_parse_rejects_month_13_with_build() {
    let result = Version::parse("26.13.1");
    assert!(result.is_err(), "month 13 with build should be invalid");
}

#[test]
fn test_version_parse_month_boundaries() {
    assert!(Version::parse("26.1").is_ok(), "month 1 should be valid");
    assert!(Version::parse("26.12").is_ok(), "month 12 should be valid");
}

#[test]
fn test_version_parse_rejects_empty_string() {
    let result = Version::parse("");
    assert!(result.is_err(), "empty string should be invalid");
}

#[test]
fn test_version_parse_rejects_non_numeric() {
    assert!(Version::parse("abc").is_err());
    assert!(Version::parse("26.abc").is_err());
    assert!(Version::parse("26.1.abc").is_err());
}

#[test]
fn test_version_parse_rejects_too_many_parts() {
    assert!(Version::parse("26.1.3.4").is_err());
}

#[test]
fn test_version_parse_trims_whitespace() {
    let v = Version::parse("  26.1.3  ").unwrap();
    assert_eq!(v.year, 26);
    assert_eq!(v.month, 1);
    assert_eq!(v.build, Some(3));
}

#[test]
fn test_version_display_with_build() {
    assert_eq!(Version::new(26, 1, Some(3)).to_string(), "26.1.3");
}

#[test]
fn test_version_display_without_build() {
    assert_eq!(Version::new(26, 1, None).to_string(), "26.1");
}

#[test]
fn test_version_ordering() {
    let v_25_12_1 = Version::new(25, 12, Some(1));
    let v_26_1_0 = Version::new(26, 1, Some(0));
    let v_26_1_3 = Version::new(26, 1, Some(3));
    let v_26_2_0 = Version::new(26, 2, Some(0));

    assert!(v_25_12_1 < v_26_1_0);
    assert!(v_26_1_0 < v_26_1_3);
    assert!(v_26_1_3 < v_26_2_0);
}

#[test]
fn test_version_equality() {
    let a = Version::new(26, 1, Some(3));
    let b = Version::new(26, 1, Some(3));
    assert_eq!(a, b);
}

#[test]
fn test_version_matches_prefix() {
    let v = Version::new(26, 1, Some(5));
    let prefix = Version::new(26, 1, None);
    assert!(v.matches_prefix(&prefix));
    let other_prefix = Version::new(26, 2, None);
    assert!(!v.matches_prefix(&other_prefix));
}

// ===========================================================================
// Version constraint parsing edge cases
// ===========================================================================

#[test]
fn test_constraint_any_version() {
    let c = VersionConstraint::parse("any version").unwrap();
    assert_eq!(c, VersionConstraint::AnyVersion);
    assert!(c.matches(&Version::new(1, 1, Some(0))));
    assert!(c.matches(&Version::new(99, 12, Some(999))));
}

#[test]
fn test_constraint_or_newer() {
    let c = VersionConstraint::parse("26.1 or newer").unwrap();
    assert!(c.matches(&Version::new(26, 1, Some(0))));
    assert!(c.matches(&Version::new(26, 2, Some(0))));
    assert!(c.matches(&Version::new(27, 1, Some(0))));
    assert!(!c.matches(&Version::new(25, 12, Some(0))));
}

#[test]
fn test_constraint_exactly_with_build() {
    let c = VersionConstraint::parse("26.1.3 exactly").unwrap();
    assert!(c.matches(&Version::new(26, 1, Some(3))));
    assert!(!c.matches(&Version::new(26, 1, Some(4))));
    assert!(!c.matches(&Version::new(26, 1, Some(2))));
}

#[test]
fn test_constraint_exactly_without_build_matches_prefix() {
    let c = VersionConstraint::parse("26.1 exactly").unwrap();
    assert!(c.matches(&Version::new(26, 1, Some(0))));
    assert!(c.matches(&Version::new(26, 1, Some(5))));
    assert!(!c.matches(&Version::new(26, 2, Some(0))));
}

#[test]
fn test_constraint_between() {
    let c = VersionConstraint::parse("between 25.12 and 26.2").unwrap();
    assert!(c.matches(&Version::new(26, 1, Some(0))));
    assert!(c.matches(&Version::new(25, 12, Some(0))));
    assert!(c.matches(&Version::new(26, 2, Some(0))));
    assert!(!c.matches(&Version::new(25, 11, Some(0))));
    assert!(!c.matches(&Version::new(26, 3, Some(0))));
}

#[test]
fn test_constraint_above() {
    let c = VersionConstraint::parse("above 25.6").unwrap();
    assert!(c.matches(&Version::new(25, 7, Some(0))));
    assert!(c.matches(&Version::new(26, 1, Some(0))));
    assert!(!c.matches(&Version::new(25, 6, Some(0))));
    assert!(!c.matches(&Version::new(25, 5, Some(0))));
}

#[test]
fn test_constraint_below() {
    let c = VersionConstraint::parse("below 27").unwrap();
    assert!(c.matches(&Version::new(26, 12, Some(99))));
    assert!(!c.matches(&Version::new(27, 1, Some(0))));
}

#[test]
fn test_constraint_above_below() {
    let c = VersionConstraint::parse("26.1 or newer but below 27").unwrap();
    assert!(c.matches(&Version::new(26, 5, Some(0))));
    assert!(c.matches(&Version::new(26, 1, Some(0))));
    assert!(!c.matches(&Version::new(25, 12, Some(0))));
    assert!(!c.matches(&Version::new(27, 1, Some(0))));
}

#[test]
fn test_constraint_parse_rejects_garbage() {
    assert!(VersionConstraint::parse("foo bar baz").is_err());
    assert!(VersionConstraint::parse("").is_err());
    assert!(VersionConstraint::parse("latest").is_err());
    assert!(VersionConstraint::parse(">=26.1").is_err());
}

#[test]
fn test_constraint_between_missing_and() {
    let result = VersionConstraint::parse("between 25.12");
    assert!(result.is_err());
}

#[test]
fn test_constraint_display_roundtrip() {
    let cases = vec![
        "any version",
        "26.1 or newer",
        "26.1.3 exactly",
        "between 25.12 and 26.2",
        "above 25.6",
        "below 27.1",
        "26.1 or newer but below 27.1",
    ];
    for input in cases {
        let c = VersionConstraint::parse(input).unwrap();
        let displayed = c.to_string();
        let reparsed = VersionConstraint::parse(&displayed).unwrap();
        assert_eq!(
            c, reparsed,
            "roundtrip failed for '{}' -> '{}' -> {:?}",
            input, displayed, reparsed
        );
    }
}

#[test]
fn test_constraint_parse_trims_whitespace() {
    let c = VersionConstraint::parse("  any version  ").unwrap();
    assert_eq!(c, VersionConstraint::AnyVersion);
}

// ===========================================================================
// Lockfile parser edge cases
// ===========================================================================

#[test]
fn test_lockfile_parse_valid() {
    let content = "\
// Auto-generated by WFL.
package http-client
  version is 26.1.3
  checksum is wflhash:a3f8b2c9d4e5f6a7

package json-parser
  version is 25.12.8
  checksum is wflhash:b4c5d6e7f8a9b0c1
  requires text-utils 25.11.2
";
    let lock = wflpkg::lockfile::parser::parse_lock_file(content).unwrap();
    assert_eq!(lock.packages.len(), 2);
    assert_eq!(lock.packages[0].name, "http-client");
    assert_eq!(lock.packages[0].version.to_string(), "26.1.3");
    assert_eq!(lock.packages[1].dependencies.len(), 1);
    assert_eq!(lock.packages[1].dependencies[0].name, "text-utils");
}

#[test]
fn test_lockfile_parse_empty() {
    let lock = wflpkg::lockfile::parser::parse_lock_file("").unwrap();
    assert!(lock.packages.is_empty());
}

#[test]
fn test_lockfile_parse_comments_only() {
    let content = "// just comments\n// nothing else\n";
    let lock = wflpkg::lockfile::parser::parse_lock_file(content).unwrap();
    assert!(lock.packages.is_empty());
}

#[test]
fn test_lockfile_parse_rejects_unrecognized_field() {
    let content = "\
package my-pkg
  version is 26.1.0
  unknown_field is something
";
    let result = wflpkg::lockfile::parser::parse_lock_file(content);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Unrecognized field"),
        "expected 'Unrecognized field', got: {msg}"
    );
}

#[test]
fn test_lockfile_parse_rejects_indented_without_package() {
    let content = "  version is 26.1.0\n";
    let result = wflpkg::lockfile::parser::parse_lock_file(content);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("without a preceding package"),
        "expected 'without a preceding package', got: {msg}"
    );
}

#[test]
fn test_lockfile_parse_rejects_malformed_requires() {
    let content = "\
package my-pkg
  version is 26.1.0
  requires only-name
";
    let result = wflpkg::lockfile::parser::parse_lock_file(content);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Malformed requires"),
        "expected 'Malformed requires', got: {msg}"
    );
}

#[test]
fn test_lockfile_parse_rejects_invalid_version() {
    let content = "\
package my-pkg
  version is not-a-version
";
    let result = wflpkg::lockfile::parser::parse_lock_file(content);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Invalid version"),
        "expected 'Invalid version', got: {msg}"
    );
}

#[test]
fn test_lockfile_parse_rejects_unexpected_line() {
    let content = "this is not valid\n";
    let result = wflpkg::lockfile::parser::parse_lock_file(content);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("Unexpected line"),
        "expected 'Unexpected line', got: {msg}"
    );
}

// ===========================================================================
// Manifest loading edge cases
// ===========================================================================

#[test]
fn test_manifest_load_missing_file() {
    let temp = TempDir::new().unwrap();
    let result = wflpkg::ProjectManifest::load(&temp.path().join("nonexistent.wfl"));
    assert!(result.is_err());
}

#[test]
fn test_manifest_load_valid() {
    let temp = TempDir::new().unwrap();
    let manifest_path = temp.path().join("project.wfl");
    std::fs::write(
        &manifest_path,
        "name is my-project\nversion is 26.1.0\ndescription is A test project",
    )
    .unwrap();

    let manifest = wflpkg::ProjectManifest::load(&manifest_path).unwrap();
    assert_eq!(manifest.name, "my-project");
    assert_eq!(manifest.version_string, "26.1.0");
    assert_eq!(manifest.description, "A test project");
}

#[test]
fn test_manifest_entry_point_default() {
    let manifest = wflpkg::ProjectManifest::default();
    assert_eq!(manifest.entry_point(), "src/main.wfl");
}

#[test]
fn test_manifest_registry_url_default() {
    let manifest = wflpkg::ProjectManifest::default();
    assert_eq!(manifest.registry_url(), "wflhub.org");
}

#[test]
fn test_manifest_find_dependency() {
    let temp = TempDir::new().unwrap();
    let manifest_path = temp.path().join("project.wfl");
    std::fs::write(
        &manifest_path,
        "name is my-project\nversion is 26.1.0\ndescription is Test\nrequires json-parser 26.1 or newer",
    )
    .unwrap();

    let manifest = wflpkg::ProjectManifest::load(&manifest_path).unwrap();
    assert!(manifest.find_dependency("json-parser").is_some());
    assert!(manifest.find_dependency("nonexistent").is_none());
}

#[test]
fn test_manifest_add_and_remove_dependency() {
    let mut manifest = wflpkg::ProjectManifest::default();
    manifest.name = "test".to_string();

    let dep = wflpkg::manifest::Dependency {
        name: "my-dep".to_string(),
        constraint: VersionConstraint::AnyVersion,
        dev_only: false,
    };
    manifest.add_dependency(dep);
    assert!(manifest.find_dependency("my-dep").is_some());
    assert_eq!(manifest.dependencies.len(), 1);

    assert!(manifest.remove_dependency("my-dep"));
    assert!(manifest.find_dependency("my-dep").is_none());
    assert_eq!(manifest.dependencies.len(), 0);
}

#[test]
fn test_manifest_remove_nonexistent_dependency() {
    let mut manifest = wflpkg::ProjectManifest::default();
    assert!(!manifest.remove_dependency("nonexistent"));
}

// ===========================================================================
// Error display formatting
// ===========================================================================

#[test]
fn test_error_display_manifest_not_found() {
    let err = PackageError::ManifestNotFound("/some/path".to_string());
    let msg = err.to_string();
    assert!(msg.contains("/some/path"));
    assert!(msg.contains("project.wfl"));
}

#[test]
fn test_error_display_invalid_version() {
    let err = PackageError::InvalidVersion("abc".to_string());
    let msg = err.to_string();
    assert!(msg.contains("abc"));
    assert!(msg.contains("YY.MM.BUILD"));
}

#[test]
fn test_error_display_invalid_version_constraint() {
    let err = PackageError::InvalidVersionConstraint("garbage".to_string());
    let msg = err.to_string();
    assert!(msg.contains("garbage"));
}

#[test]
fn test_error_display_not_authenticated() {
    let err = PackageError::NotAuthenticated;
    let msg = err.to_string();
    assert!(msg.contains("not logged in"));
    assert!(msg.contains("wfl login"));
}

#[test]
fn test_error_display_package_not_found_with_suggestions() {
    let err = PackageError::PackageNotFound {
        name: "jso-parser".to_string(),
        suggestions: vec!["json-parser".to_string(), "json-reader".to_string()],
    };
    let msg = err.to_string();
    assert!(msg.contains("jso-parser"));
    assert!(msg.contains("json-parser"));
    assert!(msg.contains("json-reader"));
    assert!(msg.contains("Did you mean"));
}

#[test]
fn test_error_display_package_not_found_without_suggestions() {
    let err = PackageError::PackageNotFound {
        name: "nonexistent".to_string(),
        suggestions: Vec::new(),
    };
    let msg = err.to_string();
    assert!(msg.contains("nonexistent"));
    assert!(!msg.contains("Did you mean"));
}

#[test]
fn test_error_display_checksum_mismatch() {
    let err = PackageError::ChecksumMismatch {
        package: "my-pkg".to_string(),
        expected: "wflhash:aaa".to_string(),
        actual: "wflhash:bbb".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("my-pkg"));
    assert!(msg.contains("wflhash:aaa"));
    assert!(msg.contains("wflhash:bbb"));
}

#[test]
fn test_error_display_version_conflict() {
    let err = PackageError::VersionConflict {
        package: "shared-dep".to_string(),
        constraint_a: "26.1 or newer".to_string(),
        source_a: "pkg-a".to_string(),
        constraint_b: "below 26".to_string(),
        source_b: "pkg-b".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("shared-dep"));
    assert!(msg.contains("pkg-a"));
    assert!(msg.contains("pkg-b"));
}

#[test]
fn test_error_display_lockfile_parse_error() {
    let err = PackageError::LockFileParseError {
        line: 42,
        message: "unexpected token".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("42"));
    assert!(msg.contains("unexpected token"));
}

#[test]
fn test_error_display_workspace_error() {
    let err = PackageError::WorkspaceError("missing member".to_string());
    let msg = err.to_string();
    assert!(msg.contains("missing member"));
}

#[test]
fn test_error_display_registry_unreachable() {
    let err = PackageError::RegistryUnreachable("https://wflhub.org".to_string());
    let msg = err.to_string();
    assert!(msg.contains("wflhub.org"));
    assert!(msg.contains("network"));
}

#[test]
fn test_error_display_permission_required() {
    let err = PackageError::PermissionRequired {
        package: "my-pkg".to_string(),
        permissions: vec!["file-access".to_string(), "network-access".to_string()],
    };
    let msg = err.to_string();
    assert!(msg.contains("my-pkg"));
    assert!(msg.contains("file-access"));
    assert!(msg.contains("network-access"));
}

#[test]
fn test_error_display_security_advisory() {
    let err = PackageError::SecurityAdvisory {
        package: "vuln-pkg".to_string(),
        severity: "high".to_string(),
        description: "Remote code execution".to_string(),
        fixed_in: Some("26.2.0".to_string()),
    };
    let msg = err.to_string();
    assert!(msg.contains("vuln-pkg"));
    assert!(msg.contains("high"));
    assert!(msg.contains("Remote code execution"));
    assert!(msg.contains("26.2.0"));
}

#[test]
fn test_error_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let pkg_err: PackageError = io_err.into();
    let msg = pkg_err.to_string();
    assert!(msg.contains("file not found"));
}
