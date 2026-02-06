use sha2::{Digest, Sha256};
use std::path::Path;

use crate::error::PackageError;

/// Compute a WFLHASH checksum over a directory's contents.
/// Uses SHA-256 internally, prefixed with "wflhash:" for display.
pub fn compute_checksum(path: &Path) -> Result<String, PackageError> {
    let mut hasher = Sha256::new();
    hash_directory(path, path, &mut hasher)?;
    let result = hasher.finalize();
    Ok(format!("wflhash:{:x}", result))
}

/// Recursively hash directory contents in sorted order for determinism.
fn hash_directory(base: &Path, path: &Path, hasher: &mut Sha256) -> Result<(), PackageError> {
    if !path.is_dir() {
        // Hash a single file
        let content = std::fs::read(path)?;
        let relative = path.strip_prefix(base).unwrap_or(path);
        let rel_bytes = relative.to_string_lossy();
        hasher.update((rel_bytes.len() as u64).to_le_bytes());
        hasher.update(rel_bytes.as_bytes());
        hasher.update(&content);
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let entry_path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        if crate::is_excluded(&name) {
            continue;
        }

        if entry_path.is_dir() {
            hash_directory(base, &entry_path, hasher)?;
        } else {
            let content = std::fs::read(&entry_path)?;
            let relative = entry_path.strip_prefix(base).unwrap_or(&entry_path);
            let rel_bytes = relative.to_string_lossy();
            hasher.update((rel_bytes.len() as u64).to_le_bytes());
            hasher.update(rel_bytes.as_bytes());
            hasher.update(&content);
        }
    }

    Ok(())
}

/// Verify a checksum against an expected value.
pub fn verify_checksum(path: &Path, expected: &str) -> Result<bool, PackageError> {
    let actual = compute_checksum(path)?;
    Ok(actual == expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_compute_checksum() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("test.wfl"), "display \"hello\"").unwrap();
        let checksum = compute_checksum(temp.path()).unwrap();
        assert!(checksum.starts_with("wflhash:"));
        assert_eq!(checksum.len(), 8 + 64); // "wflhash:" (8) + 64 hex chars
    }

    #[test]
    fn test_verify_checksum() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("test.wfl"), "display \"hello\"").unwrap();
        let checksum = compute_checksum(temp.path()).unwrap();
        assert!(verify_checksum(temp.path(), &checksum).unwrap());
        assert!(
            !verify_checksum(
                temp.path(),
                "wflhash:0000000000000000000000000000000000000000000000000000000000000000"
            )
            .unwrap()
        );
    }

    #[test]
    fn test_deterministic() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("a.wfl"), "store x as 1").unwrap();
        std::fs::write(temp.path().join("b.wfl"), "store y as 2").unwrap();
        let c1 = compute_checksum(temp.path()).unwrap();
        let c2 = compute_checksum(temp.path()).unwrap();
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_wflpkg_files_excluded_from_checksum() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("main.wfl"), "display \"hello\"").unwrap();

        let checksum_before = compute_checksum(temp.path()).unwrap();

        std::fs::write(temp.path().join("myproject.wflpkg"), b"archive data").unwrap();

        let checksum_after = compute_checksum(temp.path()).unwrap();

        assert_eq!(
            checksum_before, checksum_after,
            "Checksum should not change when a .wflpkg file is added"
        );
    }
}
