//! Security-focused tests for archive extraction, path traversal,
//! package name validation, and entry-point containment.

use std::fs;
use tempfile::TempDir;

// ===========================================================================
// Archive extraction security
// ===========================================================================

fn build_malicious_archive(
    archive_path: &std::path::Path,
    entry_type: tar::EntryType,
    name: &[u8],
    link: &[u8],
    data: &[u8],
) {
    let file = fs::File::create(archive_path).unwrap();
    let enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar = tar::Builder::new(enc);

    let mut header = tar::Header::new_gnu();
    header.set_entry_type(entry_type);
    header.set_size(data.len() as u64);
    header.set_mode(0o644);
    {
        let gnu = header.as_gnu_mut().unwrap();
        gnu.name[..name.len()].copy_from_slice(name);
        if !link.is_empty() {
            gnu.linkname[..link.len()].copy_from_slice(link);
        }
    }
    header.set_cksum();
    tar.append(&header, data).unwrap();
    let enc = tar.into_inner().unwrap();
    enc.finish().unwrap();
}

#[test]
fn test_extract_archive_rejects_absolute_path() {
    let temp = TempDir::new().unwrap();
    let archive_path = temp.path().join("bad.wflpkg");
    build_malicious_archive(
        &archive_path,
        tar::EntryType::Regular,
        b"/etc/shadow",
        b"",
        b"malicious content",
    );

    let dest = temp.path().join("output");
    fs::create_dir_all(&dest).unwrap();
    let result = wflpkg::archive::extract_archive(&archive_path, &dest);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("absolute path"),
        "expected 'absolute path', got: {msg}"
    );
}

#[test]
fn test_extract_archive_rejects_parent_traversal() {
    let temp = TempDir::new().unwrap();
    let archive_path = temp.path().join("bad.wflpkg");
    build_malicious_archive(
        &archive_path,
        tar::EntryType::Regular,
        b"../../etc/passwd",
        b"",
        b"escape",
    );

    let dest = temp.path().join("output");
    fs::create_dir_all(&dest).unwrap();
    let result = wflpkg::archive::extract_archive(&archive_path, &dest);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("path traversal"),
        "expected 'path traversal', got: {msg}"
    );
}

#[test]
fn test_extract_archive_rejects_symlink_entry() {
    let temp = TempDir::new().unwrap();
    let archive_path = temp.path().join("bad.wflpkg");
    build_malicious_archive(
        &archive_path,
        tar::EntryType::Symlink,
        b"evil-link",
        b"/etc/passwd",
        b"",
    );

    let dest = temp.path().join("output");
    fs::create_dir_all(&dest).unwrap();
    let result = wflpkg::archive::extract_archive(&archive_path, &dest);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("symlink or hard link"),
        "expected 'symlink or hard link', got: {msg}"
    );
}

#[test]
fn test_extract_archive_rejects_hardlink_entry() {
    let temp = TempDir::new().unwrap();
    let archive_path = temp.path().join("bad.wflpkg");
    build_malicious_archive(
        &archive_path,
        tar::EntryType::Link,
        b"hard-link",
        b"/etc/shadow",
        b"",
    );

    let dest = temp.path().join("output");
    fs::create_dir_all(&dest).unwrap();
    let result = wflpkg::archive::extract_archive(&archive_path, &dest);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("symlink or hard link"),
        "expected 'symlink or hard link', got: {msg}"
    );
}

#[cfg(unix)]
#[test]
fn test_extract_archive_rejects_preexisting_symlink_ancestor() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let archive_path = temp.path().join("bad.wflpkg");
    build_malicious_archive(
        &archive_path,
        tar::EntryType::Regular,
        b"linked/escaped.txt",
        b"",
        b"escape",
    );

    let dest = temp.path().join("output");
    let outside = temp.path().join("outside");
    fs::create_dir_all(&dest).unwrap();
    fs::create_dir_all(&outside).unwrap();
    let sentinel = outside.join("sentinel.txt");
    fs::write(&sentinel, "must survive").unwrap();
    symlink(&outside, dest.join("linked")).unwrap();

    let result = wflpkg::archive::extract_archive(&archive_path, &dest);
    assert!(
        result.is_err(),
        "a pre-existing symlink ancestor must be rejected"
    );
    assert!(sentinel.exists(), "outside content must survive extraction");
    assert!(
        !outside.join("escaped.txt").exists(),
        "archive content must not escape the destination"
    );
}

#[test]
fn test_create_archive_excludes_expected_dirs() {
    let temp = TempDir::new().unwrap();
    let src = temp.path().join("project");
    fs::create_dir_all(src.join("src")).unwrap();
    fs::create_dir_all(src.join("packages")).unwrap();
    fs::create_dir_all(src.join(".git")).unwrap();
    fs::create_dir_all(src.join("node_modules")).unwrap();
    fs::create_dir_all(src.join("target")).unwrap();
    fs::write(
        src.join("project.wfl"),
        "name is test\nversion is 26.1.0\ndescription is Test",
    )
    .unwrap();
    fs::write(src.join("src/main.wfl"), "display \"hi\"").unwrap();
    fs::write(src.join(".gitignore"), "target/").unwrap();
    fs::write(src.join("project.lock"), "// lock").unwrap();
    fs::write(src.join("packages/dep.wfl"), "// dep").unwrap();

    let archive_path = temp.path().join("test.wflpkg");
    wflpkg::archive::create_archive(&src, &archive_path).unwrap();

    let dest = temp.path().join("extracted");
    wflpkg::archive::extract_archive(&archive_path, &dest).unwrap();

    assert!(dest.join("project.wfl").exists());
    assert!(dest.join("src/main.wfl").exists());
    assert!(!dest.join("packages").exists());
    assert!(!dest.join(".git").exists());
    assert!(!dest.join("node_modules").exists());
    assert!(!dest.join("target").exists());
    assert!(!dest.join(".gitignore").exists());
    assert!(!dest.join("project.lock").exists());
}

/// Regression test: the checksum published alongside a package must be
/// computed over the *project directory* (minus excluded dirs), NOT over
/// the archive file.  When the recipient extracts the archive and runs
/// `compute_checksum` on the extracted tree the result must match.
#[test]
fn test_checksum_of_project_dir_matches_extracted_archive() {
    let temp = TempDir::new().unwrap();

    let src = temp.path().join("project");
    fs::create_dir_all(src.join("src")).unwrap();
    fs::write(
        src.join("project.wfl"),
        "name is test\nversion is 26.1.0\ndescription is Test",
    )
    .unwrap();
    fs::write(src.join("src/main.wfl"), "display \"hello\"").unwrap();

    // Add every entry from EXCLUDED_NAMES so checksum and archive both skip them
    fs::create_dir_all(src.join("packages")).unwrap();
    fs::write(src.join("packages/dep.wfl"), "// dep").unwrap();
    fs::create_dir_all(src.join(".git")).unwrap();
    fs::write(src.join(".git/HEAD"), "ref: refs/heads/main").unwrap();
    fs::create_dir_all(src.join("node_modules")).unwrap();
    fs::write(src.join("node_modules/mod.js"), "//mod").unwrap();
    fs::create_dir_all(src.join("target")).unwrap();
    fs::write(src.join("target/debug"), "bin").unwrap();
    fs::write(src.join(".gitignore"), "target/\n").unwrap();
    fs::write(src.join("project.lock"), "// lock\n").unwrap();

    // Checksum over the project directory (skips EXCLUDED_NAMES)
    let checksum_before = wflpkg::checksum::compute_checksum(&src).unwrap();

    // Create archive and extract it
    let archive_path = temp.path().join("test.wflpkg");
    wflpkg::archive::create_archive(&src, &archive_path).unwrap();

    let dest = temp.path().join("extracted");
    wflpkg::archive::extract_archive(&archive_path, &dest).unwrap();

    // Checksum over the extracted directory must equal the original
    let checksum_after = wflpkg::checksum::compute_checksum(&dest).unwrap();

    assert_eq!(
        checksum_before, checksum_after,
        "checksum of project dir should match checksum of extracted archive"
    );

    // Sanity: computing checksum over the archive *file* gives a different value
    let checksum_archive = wflpkg::checksum::compute_checksum(&archive_path).unwrap();
    assert_ne!(
        checksum_before, checksum_archive,
        "checksum of archive file should differ from checksum of project dir"
    );
}

// ===========================================================================
// Package name validation (path traversal)
// ===========================================================================

#[test]
fn test_resolve_package_rejects_slash_in_name() {
    let temp = TempDir::new().unwrap();
    let result = wflpkg::resolver::package_path::resolve_package_entry("../escape", temp.path());
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("not valid"),
        "expected exact package-name validation, got: {msg}"
    );
}

#[test]
fn test_resolve_package_rejects_backslash_in_name() {
    let temp = TempDir::new().unwrap();
    let result = wflpkg::resolver::package_path::resolve_package_entry("foo\\bar", temp.path());
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("not valid"),
        "expected exact package-name validation, got: {msg}"
    );
}

#[test]
fn test_resolve_package_rejects_dotdot_in_name() {
    let temp = TempDir::new().unwrap();
    let result = wflpkg::resolver::package_path::resolve_package_entry("..", temp.path());
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("not valid"),
        "expected exact package-name validation, got: {msg}"
    );
}

#[test]
fn test_resolve_package_rejects_empty_name() {
    let temp = TempDir::new().unwrap();
    let result = wflpkg::resolver::package_path::resolve_package_entry("", temp.path());
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("not valid"),
        "expected exact package-name validation, got: {msg}"
    );
}

#[test]
fn test_resolve_package_rejects_windows_path_prefix() {
    let temp = TempDir::new().unwrap();
    let result = wflpkg::resolver::package_path::resolve_package_entry("c:escape", temp.path());
    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("not valid"),
        "drive-prefixed names must fail the manifest's package-name rules"
    );
}

#[cfg(unix)]
#[test]
fn test_resolve_package_rejects_symlinked_packages_root() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let outside = temp.path().join("outside");
    let package = outside.join("my-lib");
    fs::create_dir_all(&package).unwrap();
    fs::write(package.join("main.wfl"), "// outside").unwrap();
    symlink(&outside, temp.path().join("packages")).unwrap();

    let result = wflpkg::resolver::package_path::resolve_package_entry("my-lib", temp.path());
    assert!(
        result.is_err(),
        "a symlinked packages root must be rejected"
    );
    assert!(
        result.unwrap_err().to_string().contains("symbolic link"),
        "the error should identify the unsafe package boundary"
    );
}

#[cfg(unix)]
#[test]
fn test_resolve_package_rejects_symlinked_manifest() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let package = temp.path().join("packages/my-lib");
    fs::create_dir_all(package.join("src")).unwrap();
    fs::write(package.join("src/main.wfl"), "// entry").unwrap();
    let outside_manifest = temp.path().join("outside-project.wfl");
    fs::write(
        &outside_manifest,
        "name is my-lib\nversion is 26.1.1\ndescription is Outside",
    )
    .unwrap();
    symlink(&outside_manifest, package.join("project.wfl")).unwrap();

    let result = wflpkg::resolver::package_path::resolve_package_entry("my-lib", temp.path());
    assert!(
        result.is_err(),
        "a symlinked package manifest must be rejected"
    );
    assert!(
        result.unwrap_err().to_string().contains("symbolic link"),
        "the error should identify the unsafe manifest"
    );
}

#[test]
fn test_resolve_package_fallback_src_main() {
    let temp = TempDir::new().unwrap();
    let pkg_dir = temp.path().join("packages").join("my-lib");
    fs::create_dir_all(pkg_dir.join("src")).unwrap();
    fs::write(pkg_dir.join("src/main.wfl"), "// entry").unwrap();

    let result = wflpkg::resolver::package_path::resolve_package_entry("my-lib", temp.path());
    assert!(result.is_ok());
    let path = result.unwrap();
    assert!(path.ends_with("main.wfl"));
}

#[test]
fn test_resolve_package_fallback_root_main() {
    let temp = TempDir::new().unwrap();
    let pkg_dir = temp.path().join("packages").join("my-lib");
    fs::create_dir_all(&pkg_dir).unwrap();
    fs::write(pkg_dir.join("main.wfl"), "// entry").unwrap();

    let result = wflpkg::resolver::package_path::resolve_package_entry("my-lib", temp.path());
    assert!(result.is_ok());
    let path = result.unwrap();
    assert!(path.ends_with("main.wfl"));
}

#[test]
fn test_resolve_package_no_entry_point_found() {
    let temp = TempDir::new().unwrap();
    let pkg_dir = temp.path().join("packages").join("empty-pkg");
    fs::create_dir_all(&pkg_dir).unwrap();

    let result = wflpkg::resolver::package_path::resolve_package_entry("empty-pkg", temp.path());
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("could not find an entry point"),
        "expected 'could not find an entry point', got: {msg}"
    );
}

// ===========================================================================
// Checksum tests
// ===========================================================================

#[test]
fn test_checksum_deterministic_across_calls() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path().join("project");
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(dir.join("file.txt"), "hello world").unwrap();
    fs::write(dir.join("src/code.wfl"), "display \"hi\"").unwrap();

    let sum1 = wflpkg::checksum::compute_checksum(&dir).unwrap();
    let sum2 = wflpkg::checksum::compute_checksum(&dir).unwrap();
    assert_eq!(sum1, sum2, "checksum should be deterministic");
}

#[test]
fn test_checksum_changes_with_content() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path().join("project");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("file.txt"), "hello").unwrap();

    let sum1 = wflpkg::checksum::compute_checksum(&dir).unwrap();

    fs::write(dir.join("file.txt"), "world").unwrap();
    let sum2 = wflpkg::checksum::compute_checksum(&dir).unwrap();

    assert_ne!(sum1, sum2, "checksum should change with content");
}

#[test]
fn test_checksum_verify_roundtrip() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path().join("project");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("file.txt"), "content").unwrap();

    let checksum = wflpkg::checksum::compute_checksum(&dir).unwrap();
    assert!(wflpkg::checksum::verify_checksum(&dir, &checksum).unwrap());
}

#[test]
fn test_checksum_verify_fails_on_tamper() {
    let temp = TempDir::new().unwrap();
    let dir = temp.path().join("project");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("file.txt"), "content").unwrap();

    let checksum = wflpkg::checksum::compute_checksum(&dir).unwrap();

    fs::write(dir.join("file.txt"), "tampered").unwrap();
    assert!(!wflpkg::checksum::verify_checksum(&dir, &checksum).unwrap());
}

#[test]
fn test_checksum_no_hash_ambiguity() {
    let temp = TempDir::new().unwrap();

    let dir1 = temp.path().join("proj1");
    fs::create_dir_all(&dir1).unwrap();
    fs::write(dir1.join("ab"), "cd").unwrap();

    let dir2 = temp.path().join("proj2");
    fs::create_dir_all(&dir2).unwrap();
    fs::write(dir2.join("a"), "bcd").unwrap();

    let sum1 = wflpkg::checksum::compute_checksum(&dir1).unwrap();
    let sum2 = wflpkg::checksum::compute_checksum(&dir2).unwrap();
    assert_ne!(
        sum1, sum2,
        "different file name + content splits should produce different checksums"
    );
}

// ===========================================================================
// Registry client construction
// ===========================================================================

#[test]
fn test_registry_client_new_returns_result() {
    let client = wflpkg::registry::api::RegistryClient::new("https://example.com");
    assert!(client.is_ok());
}

#[test]
fn test_registry_client_strips_trailing_slash() {
    let client = wflpkg::registry::api::RegistryClient::new("https://example.com/").unwrap();
    assert_eq!(client.base_url(), "https://example.com");
}

// ===========================================================================
// Auth token zeroization (basic structural tests)
// ===========================================================================

#[test]
fn test_auth_store_and_retrieve() {
    let temp = TempDir::new().unwrap();
    let auth_file = temp.path().join("auth.json");
    let auth = wflpkg::registry::auth::AuthManager::with_path(auth_file);
    auth.store_token("secret-token-123", "example.com").unwrap();

    let token = auth.get_token().unwrap();
    assert_eq!(token, Some("secret-token-123".to_string()));
}

#[test]
fn test_auth_clear_token() {
    let temp = TempDir::new().unwrap();
    let auth_file = temp.path().join("auth.json");
    let auth = wflpkg::registry::auth::AuthManager::with_path(auth_file);
    auth.store_token("secret", "example.com").unwrap();
    auth.clear_token().unwrap();

    let token = auth.get_token().unwrap();
    assert_eq!(token, None);
}

#[test]
fn test_auth_is_authenticated() {
    let temp = TempDir::new().unwrap();
    let auth_file = temp.path().join("auth.json");
    let auth = wflpkg::registry::auth::AuthManager::with_path(auth_file);

    assert!(!auth.is_authenticated());
    auth.store_token("token", "example.com").unwrap();
    assert!(auth.is_authenticated());
}

#[cfg(unix)]
#[test]
fn test_auth_file_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempDir::new().unwrap();
    let auth_file = temp.path().join("auth.json");
    let auth = wflpkg::registry::auth::AuthManager::with_path(auth_file.clone());
    auth.store_token("secret", "example.com").unwrap();

    let perms = fs::metadata(&auth_file).unwrap().permissions();
    assert_eq!(
        perms.mode() & 0o777,
        0o600,
        "auth file should have 0600 permissions"
    );
}

// ===========================================================================
// Permissions module
// ===========================================================================

#[test]
fn test_permission_parse_known() {
    assert_eq!(
        wflpkg::permissions::Permission::parse("file-access"),
        wflpkg::permissions::Permission::FileAccess
    );
    assert_eq!(
        wflpkg::permissions::Permission::parse("network-access"),
        wflpkg::permissions::Permission::NetworkAccess
    );
    assert_eq!(
        wflpkg::permissions::Permission::parse("system-access"),
        wflpkg::permissions::Permission::SystemAccess
    );
}

#[test]
fn test_permission_parse_unknown() {
    let perm = wflpkg::permissions::Permission::parse("custom-perm");
    assert_eq!(
        perm,
        wflpkg::permissions::Permission::Unknown("custom-perm".to_string())
    );
    assert_eq!(perm.name(), "custom-perm");
    assert_eq!(perm.description(), "Unknown permission");
}

// ===========================================================================
// Cache module
// ===========================================================================

#[test]
fn test_cache_store_and_retrieve() {
    let temp = TempDir::new().unwrap();
    let cache = wflpkg::cache::PackageCache::with_dir(temp.path().join("cache")).unwrap();

    let src = temp.path().join("src-pkg");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("main.wfl"), "display \"hi\"").unwrap();

    let version = wflpkg::Version::new(26, 1, Some(0));
    cache.store("my-pkg", &version, &src).unwrap();
    assert!(cache.is_cached("my-pkg", &version));

    let versions = cache.list_versions("my-pkg").unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0], version);
}

#[test]
fn test_cache_install_to_project() {
    let temp = TempDir::new().unwrap();
    let cache = wflpkg::cache::PackageCache::with_dir(temp.path().join("cache")).unwrap();

    let src = temp.path().join("src-pkg");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("main.wfl"), "display \"hi\"").unwrap();

    let version = wflpkg::Version::new(26, 1, Some(0));
    cache.store("my-pkg", &version, &src).unwrap();

    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    cache
        .install_to_project("my-pkg", &version, &project)
        .unwrap();

    assert!(project.join("packages/my-pkg/main.wfl").exists());
}

#[test]
fn test_cache_list_versions_not_cached() {
    let temp = TempDir::new().unwrap();
    let cache = wflpkg::cache::PackageCache::with_dir(temp.path().join("cache")).unwrap();
    let versions = cache.list_versions("nonexistent").unwrap();
    assert!(versions.is_empty());
}

#[test]
fn test_cache_install_not_cached_fails() {
    let temp = TempDir::new().unwrap();
    let cache = wflpkg::cache::PackageCache::with_dir(temp.path().join("cache")).unwrap();
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    let version = wflpkg::Version::new(99, 1, Some(0));
    let result = cache.install_to_project("not-cached", &version, &project);
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("not in the cache"),
        "expected 'not in the cache', got: {msg}"
    );
}

#[test]
fn test_cache_rejects_invalid_package_names() {
    let temp = TempDir::new().unwrap();
    let cache = wflpkg::cache::PackageCache::with_dir(temp.path().join("cache")).unwrap();
    let version = wflpkg::Version::new(26, 1, Some(0));
    let invalid_name = temp.path().join("outside").to_string_lossy().into_owned();

    let src = temp.path().join("src-pkg");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("main.wfl"), "display \"hi\"").unwrap();
    assert!(!cache.is_cached(&invalid_name, &version));
    assert!(cache.store(&invalid_name, &version, &src).is_err());
    assert!(cache.list_versions(&invalid_name).is_err());

    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    assert!(
        cache
            .install_to_project(&invalid_name, &version, &project)
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn test_cache_rejects_symlinked_root() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let outside = temp.path().join("outside-cache");
    fs::create_dir_all(&outside).unwrap();
    let cache_link = temp.path().join("cache-link");
    symlink(&outside, &cache_link).unwrap();

    match wflpkg::cache::PackageCache::with_dir(cache_link) {
        Err(error) => assert!(error.to_string().contains("symbolic link")),
        Ok(_) => panic!("a symlinked package cache root must be rejected"),
    }
}

#[cfg(unix)]
#[test]
fn test_cache_store_rejects_symlinked_package_target() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let cache = wflpkg::cache::PackageCache::with_dir(temp.path().join("cache")).unwrap();
    let version = wflpkg::Version::new(26, 1, Some(0));
    let outside = temp.path().join("outside-package");
    let outside_version = outside.join(version.to_string());
    fs::create_dir_all(&outside_version).unwrap();
    let sentinel = outside_version.join("sentinel.txt");
    fs::write(&sentinel, "must survive").unwrap();
    symlink(&outside, cache.cache_dir().join("my-pkg")).unwrap();

    let src = temp.path().join("src-pkg");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("main.wfl"), "display \"hi\"").unwrap();
    assert!(!cache.is_cached("my-pkg", &version));
    let result = cache.store("my-pkg", &version, &src);
    assert!(result.is_err(), "a symlinked cache target must be rejected");
    assert!(sentinel.exists(), "outside cached content must survive");
}

#[cfg(unix)]
#[test]
fn test_cache_install_rejects_symlinked_packages_root() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap();
    let cache = wflpkg::cache::PackageCache::with_dir(temp.path().join("cache")).unwrap();
    let version = wflpkg::Version::new(26, 1, Some(0));
    let src = temp.path().join("src-pkg");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("main.wfl"), "display \"hi\"").unwrap();
    cache.store("my-pkg", &version, &src).unwrap();

    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let outside = temp.path().join("outside-packages");
    let outside_package = outside.join("my-pkg");
    fs::create_dir_all(&outside_package).unwrap();
    let sentinel = outside_package.join("sentinel.txt");
    fs::write(&sentinel, "must survive").unwrap();
    symlink(&outside, project.join("packages")).unwrap();

    let result = cache.install_to_project("my-pkg", &version, &project);
    assert!(
        result.is_err(),
        "a symlinked project packages root must be rejected"
    );
    assert!(sentinel.exists(), "outside installed content must survive");
}
