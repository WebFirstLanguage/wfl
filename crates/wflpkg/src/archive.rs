use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use std::path::Path;
use tar::Archive;

use crate::error::PackageError;

/// Create a `.wflpkg` archive (tar.gz) from a project directory.
/// Excludes `packages/`, `.git/`, `node_modules/`, and other non-source files.
pub fn create_archive(project_dir: &Path, output_path: &Path) -> Result<(), PackageError> {
    let file = std::fs::File::create(output_path)?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = tar::Builder::new(enc);

    add_dir_to_archive(&mut tar, project_dir, project_dir)?;

    tar.finish()
        .map_err(|e| PackageError::General(format!("Failed to create archive: {}", e)))?;

    Ok(())
}

/// Recursively add directory contents to the archive.
fn add_dir_to_archive<W: std::io::Write>(
    tar: &mut tar::Builder<W>,
    base: &Path,
    dir: &Path,
) -> Result<(), PackageError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if crate::EXCLUDED_NAMES.contains(&name_str.as_ref()) {
            continue;
        }

        let ft = entry
            .file_type()
            .map_err(|e| PackageError::General(format!("Failed to read file type: {}", e)))?;

        // Skip symlinks to avoid following links outside the project
        if ft.is_symlink() {
            continue;
        }

        let relative = path.strip_prefix(base).unwrap_or(&path);

        if ft.is_dir() {
            add_dir_to_archive(tar, base, &path)?;
        } else {
            tar.append_path_with_name(&path, relative).map_err(|e| {
                PackageError::General(format!("Failed to add file to archive: {}", e))
            })?;
        }
    }
    Ok(())
}

/// Extract a `.wflpkg` archive to a destination directory.
///
/// Validates that all extracted paths stay within `dest_dir` to prevent
/// directory traversal attacks from malicious archives.
pub fn extract_archive(archive_path: &Path, dest_dir: &Path) -> Result<(), PackageError> {
    let file = std::fs::File::open(archive_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    let dest_canonical = std::fs::canonicalize(
        std::fs::create_dir_all(dest_dir)
            .map(|_| dest_dir)
            .map_err(PackageError::Io)?,
    )
    .map_err(PackageError::Io)?;

    for entry in archive
        .entries()
        .map_err(|e| PackageError::General(format!("Failed to read archive entries: {}", e)))?
    {
        let mut entry =
            entry.map_err(|e| PackageError::General(format!("Invalid archive entry: {}", e)))?;

        let entry_path = entry
            .path()
            .map_err(|e| PackageError::General(format!("Invalid entry path: {}", e)))?
            .into_owned();

        // Reject absolute paths
        if entry_path.is_absolute() {
            return Err(PackageError::General(format!(
                "Archive contains absolute path: {}",
                entry_path.display()
            )));
        }

        // Reject paths with .. components
        for component in entry_path.components() {
            if matches!(component, std::path::Component::ParentDir) {
                return Err(PackageError::General(format!(
                    "Archive contains path traversal: {}",
                    entry_path.display()
                )));
            }
        }

        // Verify resolved path stays within dest_dir
        let target = dest_canonical.join(&entry_path);
        if !target.starts_with(&dest_canonical) {
            return Err(PackageError::General(format!(
                "Archive entry escapes destination: {}",
                entry_path.display()
            )));
        }

        // Reject symlink and hard link entries to prevent symlink-based attacks
        if entry.header().entry_type().is_symlink() || entry.header().entry_type().is_hard_link() {
            return Err(PackageError::General(format!(
                "Archive contains a symlink or hard link: {}",
                entry_path.display()
            )));
        }

        // Create parent directories as needed
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }

        entry.unpack(&target).map_err(|e| {
            PackageError::General(format!("Failed to extract {}: {}", entry_path.display(), e))
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_and_extract_archive() {
        let temp = TempDir::new().unwrap();

        // Create source project
        let src = temp.path().join("project");
        std::fs::create_dir_all(src.join("src")).unwrap();
        std::fs::write(
            src.join("project.wfl"),
            "name is test\nversion is 26.1.1\ndescription is Test",
        )
        .unwrap();
        std::fs::write(src.join("src").join("main.wfl"), "display \"hello\"").unwrap();

        // Create directories that should be excluded
        std::fs::create_dir_all(src.join("packages")).unwrap();
        std::fs::create_dir_all(src.join(".git")).unwrap();

        // Create archive
        let archive_path = temp.path().join("test.wflpkg");
        create_archive(&src, &archive_path).unwrap();
        assert!(archive_path.exists());

        // Extract archive
        let dest = temp.path().join("extracted");
        extract_archive(&archive_path, &dest).unwrap();

        // Verify contents
        assert!(dest.join("project.wfl").exists());
        assert!(dest.join("src").join("main.wfl").exists());
        // Excluded directories should not be present
        assert!(!dest.join("packages").exists());
        assert!(!dest.join(".git").exists());
    }
}
