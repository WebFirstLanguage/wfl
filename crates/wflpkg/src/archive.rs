use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use std::io::{Read, Write};
use std::path::Path;
use tar::{Archive, Header};

use crate::error::PackageError;
use crate::package_files::IgnoreStack;

const MAX_PACKAGE_ENTRIES: u64 = 10_000;
const MAX_PACKAGE_SOURCE_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_PACKAGE_DEPTH: usize = 128;

/// Create a `.wflpkg` archive (tar.gz) from a project directory.
/// Excludes `packages/`, `.git/`, `node_modules/`, and other non-source files.
pub fn create_archive(project_dir: &Path, output_path: &Path) -> Result<(), PackageError> {
    let file = create_new_private_file(output_path)?;
    let mut cleanup = RemoveOnDrop::new(output_path);
    create_archive_to_writer(project_dir, file)?;
    cleanup.keep();
    Ok(())
}

/// Create an archive on a caller-owned output stream.
///
/// The share command uses this with an already-open private temporary file, so
/// an untrusted project cannot redirect archive creation through a symlink.
pub(crate) fn create_archive_to_writer<W: Write>(
    project_dir: &Path,
    writer: W,
) -> Result<(), PackageError> {
    let enc = GzEncoder::new(writer, Compression::default());
    let mut tar = tar::Builder::new(enc);
    // A path that is swapped to a symlink while packaging must never cause
    // tar to follow and disclose its target.
    tar.follow_symlinks(false);
    let mut ignore_stack = IgnoreStack::new(project_dir)?;
    let mut budget = TraversalBudget::default();

    add_dir_to_archive(
        &mut tar,
        project_dir,
        project_dir,
        &mut ignore_stack,
        &mut budget,
        0,
    )?;

    let enc = tar
        .into_inner()
        .map_err(|e| PackageError::General(format!("Failed to create archive: {}", e)))?;
    enc.finish()
        .map_err(|e| PackageError::General(format!("Failed to finish archive: {}", e)))?;

    Ok(())
}

/// Recursively add directory contents to the archive.
fn add_dir_to_archive<W: std::io::Write>(
    tar: &mut tar::Builder<W>,
    base: &Path,
    dir: &Path,
    ignore_stack: &mut IgnoreStack,
    budget: &mut TraversalBudget,
    depth: usize,
) -> Result<(), PackageError> {
    if depth > MAX_PACKAGE_DEPTH {
        return Err(package_budget_error());
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        budget.add_entry()?;
        entries.push(entry?);
    }
    for entry in &entries {
        if entry.file_name().to_str().is_none() {
            return Err(PackageError::General(format!(
                "Package path is not valid Unicode: {}",
                entry.path().display()
            )));
        }
    }
    entries.sort_by(|left, right| {
        left.file_name()
            .to_str()
            .expect("validated above")
            .cmp(right.file_name().to_str().expect("validated above"))
    });

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_str().expect("validated above");

        if crate::is_excluded(name_str) {
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
        if ignore_stack.is_ignored(relative, ft.is_dir())? {
            continue;
        }
        if !ft.is_dir() && !ft.is_file() {
            return Err(PackageError::General(format!(
                "Package contains an unsupported filesystem object: {}",
                path.display()
            )));
        }
        if ft.is_dir() {
            let added_rules = ignore_stack.push_for_dir(&path, relative)?;
            let result = add_dir_to_archive(tar, base, &path, ignore_stack, budget, depth + 1);
            ignore_stack.pop(added_rules);
            result?;
        } else {
            let mut options = std::fs::OpenOptions::new();
            options.read(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.custom_flags(libc::O_NOFOLLOW);
            }
            let mut file = options.open(&path)?;
            let metadata = file.metadata()?;
            if !metadata.is_file() {
                return Err(PackageError::General(format!(
                    "Package file changed type while it was being archived: {}",
                    path.display()
                )));
            }
            budget.add_bytes(metadata.len())?;
            let captured_size = metadata.len();
            let mut header = Header::new_gnu();
            header.set_metadata(&metadata);
            header.set_size(captured_size);
            {
                let mut limited = (&mut file).take(captured_size);
                tar.append_data(&mut header, relative, &mut limited)
                    .map_err(|e| {
                        PackageError::General(format!("Failed to add file to archive: {}", e))
                    })?;
                if limited.limit() != 0 {
                    return Err(PackageError::General(format!(
                        "Package file changed while it was being archived: {}",
                        path.display()
                    )));
                }
            }
            let mut extra = [0_u8; 1];
            if file.read(&mut extra)? != 0 {
                return Err(PackageError::General(format!(
                    "Package file changed while it was being archived: {}",
                    path.display()
                )));
            }
        }
    }
    Ok(())
}

#[derive(Default)]
struct TraversalBudget {
    entries: u64,
    source_bytes: u64,
}

impl TraversalBudget {
    fn add_entry(&mut self) -> Result<(), PackageError> {
        self.entries = self
            .entries
            .checked_add(1)
            .ok_or_else(package_budget_error)?;
        if self.entries > MAX_PACKAGE_ENTRIES {
            return Err(package_budget_error());
        }
        Ok(())
    }

    fn add_bytes(&mut self, bytes: u64) -> Result<(), PackageError> {
        self.source_bytes = self
            .source_bytes
            .checked_add(bytes)
            .ok_or_else(package_budget_error)?;
        if self.source_bytes > MAX_PACKAGE_SOURCE_BYTES {
            return Err(package_budget_error());
        }
        Ok(())
    }
}

fn package_budget_error() -> PackageError {
    PackageError::General(
        "The project exceeds the safe package size, file-count, or directory-depth limit."
            .to_string(),
    )
}

fn create_new_private_file(path: &Path) -> Result<std::fs::File, PackageError> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).map_err(PackageError::Io)
}

struct RemoveOnDrop<'a> {
    path: &'a Path,
    keep: bool,
}

impl<'a> RemoveOnDrop<'a> {
    fn new(path: &'a Path) -> Self {
        Self { path, keep: false }
    }

    fn keep(&mut self) {
        self.keep = true;
    }
}

impl Drop for RemoveOnDrop<'_> {
    fn drop(&mut self) {
        if !self.keep {
            let _ = std::fs::remove_file(self.path);
        }
    }
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
    fn test_wflpkg_files_excluded_from_archive() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("project");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("main.wfl"), "display \"hello\"").unwrap();
        std::fs::write(src.join("project.wflpkg"), b"archive data").unwrap();

        let archive_path = temp.path().join("test.wflpkg");
        create_archive(&src, &archive_path).unwrap();

        let dest = temp.path().join("extracted");
        extract_archive(&archive_path, &dest).unwrap();

        assert!(dest.join("main.wfl").exists());
        assert!(
            !dest.join("project.wflpkg").exists(),
            ".wflpkg files should be excluded from archive"
        );
    }

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

    #[test]
    fn test_create_archive_honors_gitignore_rules() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("project");
        std::fs::create_dir_all(src.join("nested")).unwrap();
        std::fs::write(src.join("main.wfl"), "display \"hello\"").unwrap();
        std::fs::write(
            src.join(".gitignore"),
            ".env\n*.log\nsecret-dir/\n!important.log\n!nested/\n",
        )
        .unwrap();
        std::fs::write(src.join(".env"), "API_TOKEN=secret").unwrap();
        std::fs::write(src.join("debug.log"), "credentials").unwrap();
        std::fs::write(src.join("important.log"), "safe asset").unwrap();
        std::fs::create_dir_all(src.join("secret-dir")).unwrap();
        std::fs::write(src.join("secret-dir/token.txt"), "secret").unwrap();
        std::fs::write(src.join("nested/debug.log"), "credentials").unwrap();

        let archive_path = temp.path().join("test.wflpkg");
        create_archive(&src, &archive_path).unwrap();
        let dest = temp.path().join("extracted");
        extract_archive(&archive_path, &dest).unwrap();

        assert!(dest.join("main.wfl").exists());
        assert!(dest.join("important.log").exists());
        assert!(!dest.join(".env").exists());
        assert!(!dest.join("debug.log").exists());
        assert!(!dest.join("nested/debug.log").exists());
        assert!(!dest.join("secret-dir").exists());
    }

    #[test]
    fn test_create_archive_honors_nested_gitignore_rules() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("project");
        std::fs::create_dir_all(src.join("nested")).unwrap();
        std::fs::write(src.join("main.wfl"), "display \"hello\"").unwrap();
        std::fs::write(src.join("nested/.gitignore"), "private.txt\n").unwrap();
        std::fs::write(src.join("nested/private.txt"), "secret").unwrap();
        std::fs::write(src.join("nested/public.txt"), "public").unwrap();

        let archive_path = temp.path().join("test.wflpkg");
        create_archive(&src, &archive_path).unwrap();
        let dest = temp.path().join("extracted");
        extract_archive(&archive_path, &dest).unwrap();

        assert!(!dest.join("nested/private.txt").exists());
        assert!(dest.join("nested/public.txt").exists());
    }

    #[test]
    fn test_create_archive_refuses_existing_output() {
        let temp = TempDir::new().unwrap();
        let src = temp.path().join("project");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("main.wfl"), "display \"hello\"").unwrap();
        let archive_path = temp.path().join("existing.wflpkg");
        std::fs::write(&archive_path, "do not overwrite").unwrap();

        assert!(create_archive(&src, &archive_path).is_err());
        assert_eq!(
            std::fs::read_to_string(&archive_path).unwrap(),
            "do not overwrite"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_create_archive_refuses_symlink_output() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let src = temp.path().join("project");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("main.wfl"), "display \"hello\"").unwrap();
        let target = temp.path().join("target.txt");
        std::fs::write(&target, "do not overwrite").unwrap();
        let archive_path = temp.path().join("linked.wflpkg");
        symlink(&target, &archive_path).unwrap();

        assert!(create_archive(&src, &archive_path).is_err());
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "do not overwrite"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_package_traversal_rejects_special_files() {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let temp = TempDir::new().unwrap();
        let src = temp.path().join("project");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("main.wfl"), "display \"hello\"").unwrap();
        let special = src.join("pipe");
        let special_c = CString::new(special.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(special_c.as_ptr(), 0o600) }, 0);

        let archive_path = temp.path().join("test.wflpkg");
        assert!(create_archive(&src, &archive_path).is_err());
        assert!(!archive_path.exists());
        assert!(crate::checksum::compute_checksum(&src).is_err());

        std::fs::write(src.join(".gitignore"), "pipe\n").unwrap();
        create_archive(&src, &archive_path).unwrap();
        assert!(crate::checksum::compute_checksum(&src).is_ok());
    }
}
