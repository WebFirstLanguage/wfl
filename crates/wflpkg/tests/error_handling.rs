//! Tests for PackageError Display formatting, error conversions, and edge cases.

use wflpkg::PackageError;

// ---------------------------------------------------------------------------
// Error Display formatting (one per variant)
// ---------------------------------------------------------------------------

#[test]
fn test_display_manifest_not_found() {
    let err = PackageError::ManifestNotFound("/some/dir".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("could not find a project.wfl"),
        "expected 'could not find a project.wfl', got: {msg}"
    );
    assert!(msg.contains("/some/dir"));
}

#[test]
fn test_display_manifest_parse_error() {
    let err = PackageError::ManifestParseError {
        line: 42,
        message: "unexpected token".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("42"), "should contain line number 42: {msg}");
    assert!(msg.contains("unexpected token"));
}

#[test]
fn test_display_invalid_package_name() {
    let err = PackageError::InvalidPackageName("BAD-NAME".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("not valid"),
        "expected 'not valid', got: {msg}"
    );
    assert!(msg.contains("BAD-NAME"));
}

#[test]
fn test_display_invalid_version() {
    let err = PackageError::InvalidVersion("abc.def".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("not a valid WFL version"),
        "expected version error, got: {msg}"
    );
    assert!(msg.contains("abc.def"));
}

#[test]
fn test_display_invalid_version_constraint() {
    let err = PackageError::InvalidVersionConstraint("xyzzy".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("could not understand"),
        "expected 'could not understand', got: {msg}"
    );
    assert!(msg.contains("xyzzy"));
}

#[test]
fn test_display_package_not_found_no_suggestions() {
    let err = PackageError::PackageNotFound {
        name: "nonexistent".to_string(),
        suggestions: vec![],
    };
    let msg = err.to_string();
    assert!(msg.contains("nonexistent"));
    assert!(
        !msg.contains("Did you mean"),
        "should not suggest with empty suggestions: {msg}"
    );
}

#[test]
fn test_display_package_not_found_with_suggestions() {
    let err = PackageError::PackageNotFound {
        name: "htto-client".to_string(),
        suggestions: vec!["http-client".to_string()],
    };
    let msg = err.to_string();
    assert!(msg.contains("htto-client"));
    assert!(
        msg.contains("Did you mean"),
        "expected 'Did you mean', got: {msg}"
    );
    assert!(msg.contains("http-client"));
}

#[test]
fn test_display_version_conflict() {
    let err = PackageError::VersionConflict {
        package: "json-parser".to_string(),
        constraint_a: "26.1 or newer".to_string(),
        source_a: "my-app".to_string(),
        constraint_b: "below 26".to_string(),
        source_b: "other-pkg".to_string(),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("version conflict") || msg.contains("conflict"),
        "expected conflict mention, got: {msg}"
    );
    assert!(msg.contains("my-app"));
    assert!(msg.contains("other-pkg"));
}

#[test]
fn test_display_registry_unreachable() {
    let err = PackageError::RegistryUnreachable("https://registry.example.com".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("could not connect"),
        "expected 'could not connect', got: {msg}"
    );
    assert!(msg.contains("registry.example.com"));
}

#[test]
fn test_display_not_authenticated() {
    let err = PackageError::NotAuthenticated;
    let msg = err.to_string();
    assert!(
        msg.contains("not logged in"),
        "expected 'not logged in', got: {msg}"
    );
}

#[test]
fn test_display_lockfile_parse_error() {
    let err = PackageError::LockFileParseError {
        line: 7,
        message: "bad checksum".to_string(),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("project.lock"),
        "expected 'project.lock', got: {msg}"
    );
    assert!(msg.contains("7"));
    assert!(msg.contains("bad checksum"));
}

#[test]
fn test_display_checksum_mismatch() {
    let err = PackageError::ChecksumMismatch {
        package: "http-client".to_string(),
        expected: "abc123".to_string(),
        actual: "def456".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains("abc123"), "expected hash missing: {msg}");
    assert!(msg.contains("def456"), "actual hash missing: {msg}");
    assert!(msg.contains("http-client"));
}

#[test]
fn test_display_security_advisory_with_fix() {
    let err = PackageError::SecurityAdvisory {
        package: "crypto-lib".to_string(),
        severity: "HIGH".to_string(),
        description: "Buffer overflow".to_string(),
        fixed_in: Some("26.2.1".to_string()),
    };
    let msg = err.to_string();
    assert!(msg.contains("Fixed in"), "expected 'Fixed in', got: {msg}");
    assert!(msg.contains("26.2.1"));
    assert!(msg.contains("crypto-lib"));
}

#[test]
fn test_display_security_advisory_no_fix() {
    let err = PackageError::SecurityAdvisory {
        package: "crypto-lib".to_string(),
        severity: "LOW".to_string(),
        description: "Minor issue".to_string(),
        fixed_in: None,
    };
    let msg = err.to_string();
    assert!(
        !msg.contains("Fixed in"),
        "should not contain 'Fixed in' with no fix: {msg}"
    );
}

#[test]
fn test_display_permission_required() {
    let err = PackageError::PermissionRequired {
        package: "fs-tools".to_string(),
        permissions: vec!["file-access".to_string(), "network-access".to_string()],
    };
    let msg = err.to_string();
    assert!(msg.contains("file-access"));
    assert!(msg.contains("network-access"));
    assert!(
        msg.contains("Can read and write files"),
        "expected permission description, got: {msg}"
    );
}

#[test]
fn test_display_workspace_error() {
    let err = PackageError::WorkspaceError("bad config".to_string());
    let msg = err.to_string();
    assert!(
        msg.contains("Workspace error"),
        "expected 'Workspace error', got: {msg}"
    );
    assert!(msg.contains("bad config"));
}

#[test]
fn test_display_general() {
    let err = PackageError::General("custom message".to_string());
    let msg = err.to_string();
    assert_eq!(msg, "custom message");
}

// ---------------------------------------------------------------------------
// Error conversions
// ---------------------------------------------------------------------------

#[test]
fn test_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file gone");
    let pkg_err: PackageError = io_err.into();
    let msg = pkg_err.to_string();
    assert!(msg.contains("file gone"), "IO message preserved: {msg}");
}

// ---------------------------------------------------------------------------
// Manifest edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_manifest_empty_content() {
    let result = wflpkg::manifest::parser::parse_manifest("");
    assert!(result.is_err(), "empty manifest should fail");
}

#[test]
fn test_manifest_missing_version() {
    let content = "name is foo\ndescription is bar";
    let result = wflpkg::manifest::parser::parse_manifest(content);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("version"),
        "error should mention version: {msg}"
    );
}

#[test]
fn test_manifest_missing_description() {
    let content = "name is foo\nversion is 26.1.1";
    let result = wflpkg::manifest::parser::parse_manifest(content);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("description"),
        "error should mention description: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Version edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_version_parse_non_numeric() {
    let result = wflpkg::Version::parse("abc.def");
    assert!(result.is_err(), "non-numeric version should fail");
}

#[test]
fn test_version_constraint_parse_gibberish() {
    let result = wflpkg::VersionConstraint::parse("xyzzy");
    assert!(result.is_err(), "gibberish constraint should fail");
}
