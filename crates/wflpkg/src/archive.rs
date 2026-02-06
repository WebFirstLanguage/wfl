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

        // Skip excluded directories and files
        if name_str == "packages"
            || name_str == ".git"
            || name_str == "node_modules"
            || name_str == "target"
            || name_str == ".gitignore"
            || name_str == "project.lock"
        {
            continue;
        }

        let relative = path.strip_prefix(base).unwrap_or(&path);

        if path.is_dir() {
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
pub fn extract_archive(archive_path: &Path, dest_dir: &Path) -> Result<(), PackageError> {
    let file = std::fs::File::open(archive_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    std::fs::create_dir_all(dest_dir)?;
    archive
        .unpack(dest_dir)
        .map_err(|e| PackageError::General(format!("Failed to extract archive: {}", e)))?;

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
