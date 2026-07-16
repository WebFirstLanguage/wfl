use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Component, Path};
use tar::Archive;

use crate::error::PackageError;
use crate::package_files::IgnoreStack;

const CHECKSUM_PREFIX: &str = "wflhash:v2:";
const CHECKSUM_DOMAIN: &[u8] = b"WFL package checksum\0v2\0";

/// Compute a v2 WFL package checksum over selected source contents.
///
/// Publishing uses `compute_archive_checksum` after the immutable archive has
/// been created. This public helper remains useful for source trees and uses
/// the same selection rules as archive creation.
pub fn compute_checksum(path: &Path) -> Result<String, PackageError> {
    let metadata = std::fs::symlink_metadata(path)?;
    let mut hasher = new_hasher();
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        return Err(unsupported_object(path));
    }
    if file_type.is_dir() {
        let mut ignore_stack = IgnoreStack::new(path)?;
        hash_source_directory(path, path, &mut hasher, &mut ignore_stack)?;
    } else if file_type.is_file() {
        hash_file(path, path, &mut hasher)?;
    } else {
        return Err(unsupported_object(path));
    }
    Ok(finish_checksum(hasher))
}

/// Compute the checksum from the completed upload archive itself.
///
/// This prevents a concurrent source edit from producing archive A with the
/// checksum of source state B. The required entry point must be present as a
/// regular file in that exact archive.
pub(crate) fn compute_archive_checksum(
    archive_path: &Path,
    required_entry: &Path,
) -> Result<String, PackageError> {
    let file = std::fs::File::open(archive_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    let required = portable_relative_path(required_entry)?;
    if required.is_empty() {
        return Err(PackageError::General(
            "The project entry point cannot be empty.".to_string(),
        ));
    }

    let mut hasher = new_hasher();
    let mut found_entry = false;
    let mut found_manifest = false;

    for item in archive
        .entries()
        .map_err(|error| PackageError::General(format!("Failed to read archive: {}", error)))?
    {
        let mut entry = item
            .map_err(|error| PackageError::General(format!("Invalid archive entry: {}", error)))?;
        if !entry.header().entry_type().is_file() {
            return Err(PackageError::General(
                "The generated package archive contains a non-regular entry.".to_string(),
            ));
        }
        let entry_path = entry
            .path()
            .map_err(|error| PackageError::General(format!("Invalid archive path: {}", error)))?;
        let portable = portable_relative_path(&entry_path)?;
        if portable.is_empty() {
            return Err(PackageError::General(
                "The generated package archive contains an empty path.".to_string(),
            ));
        }
        found_entry |= portable == required;
        found_manifest |= portable == "project.wfl";
        let size = entry.size();
        hash_record(&portable, size, &mut entry, &mut hasher)?;
    }

    if !found_manifest {
        return Err(PackageError::General(
            "project.wfl is not included in the package archive. Check .gitignore and try again."
                .to_string(),
        ));
    }
    if !found_entry {
        return Err(PackageError::General(format!(
            "The entry point \"{}\" is not included in the package archive. Check .gitignore and make sure it is a regular file.",
            required_entry.display()
        )));
    }
    Ok(finish_checksum(hasher))
}

/// Recursively hash source contents in deterministic order.
fn hash_source_directory(
    base: &Path,
    path: &Path,
    hasher: &mut Sha256,
    ignore_stack: &mut IgnoreStack,
) -> Result<(), PackageError> {
    let entries = sorted_entries(path)?;

    for entry in entries {
        let entry_path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_str().ok_or_else(|| {
            PackageError::General(format!(
                "Package path is not valid Unicode: {}",
                entry_path.display()
            ))
        })?;

        if crate::is_excluded(name) {
            continue;
        }

        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        let relative = entry_path.strip_prefix(base).unwrap_or(&entry_path);
        if ignore_stack.is_ignored(relative, file_type.is_dir())? {
            continue;
        }
        if !file_type.is_dir() && !file_type.is_file() {
            return Err(unsupported_object(&entry_path));
        }

        if file_type.is_dir() {
            let added_rules = ignore_stack.push_for_dir(&entry_path, relative)?;
            let result = hash_source_directory(base, &entry_path, hasher, ignore_stack);
            ignore_stack.pop(added_rules);
            result?;
        } else {
            hash_file(base, &entry_path, hasher)?;
        }
    }

    Ok(())
}

/// Hash every regular file in an installed package. Unlike source selection,
/// verification deliberately honors neither fixed exclusions nor a package's
/// `.gitignore`, so an added payload can never be invisible to verification.
fn hash_verified_directory(
    base: &Path,
    path: &Path,
    hasher: &mut Sha256,
) -> Result<(), PackageError> {
    for entry in sorted_entries(path)? {
        let entry_path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() || (!file_type.is_dir() && !file_type.is_file()) {
            return Err(unsupported_object(&entry_path));
        }
        if file_type.is_dir() {
            hash_verified_directory(base, &entry_path, hasher)?;
        } else {
            hash_file(base, &entry_path, hasher)?;
        }
    }
    Ok(())
}

fn sorted_entries(path: &Path) -> Result<Vec<std::fs::DirEntry>, PackageError> {
    let mut entries: Vec<_> = std::fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
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
    Ok(entries)
}

fn hash_file(base: &Path, path: &Path, hasher: &mut Sha256) -> Result<(), PackageError> {
    let path_metadata = std::fs::symlink_metadata(path)?;
    if path_metadata.file_type().is_symlink() || !path_metadata.is_file() {
        return Err(unsupported_object(path));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(unsupported_object(path));
    }
    let relative = path.strip_prefix(base).unwrap_or(path);
    let portable = portable_relative_path(relative)?;
    hash_record(&portable, metadata.len(), &mut file, hasher)?;

    // Refuse a file that grew after its length was recorded. A short read is
    // already rejected inside `hash_record`.
    let mut extra = [0_u8; 1];
    if file.read(&mut extra)? != 0 {
        return Err(PackageError::General(format!(
            "Package file changed while it was being checksummed: {}",
            path.display()
        )));
    }
    Ok(())
}

fn hash_record<R: Read>(
    portable_path: &str,
    content_len: u64,
    reader: &mut R,
    hasher: &mut Sha256,
) -> Result<(), PackageError> {
    let path_bytes = portable_path.as_bytes();
    hasher.update([0x01]);
    hasher.update((path_bytes.len() as u64).to_le_bytes());
    hasher.update(path_bytes);
    hasher.update(content_len.to_le_bytes());

    let mut remaining = content_len;
    let mut buffer = [0_u8; 64 * 1024];
    while remaining > 0 {
        let wanted = usize::try_from(remaining.min(buffer.len() as u64)).unwrap_or(buffer.len());
        let read = reader.read(&mut buffer[..wanted])?;
        if read == 0 {
            return Err(PackageError::General(
                "Package file changed while it was being checksummed.".to_string(),
            ));
        }
        hasher.update(&buffer[..read]);
        remaining -= read as u64;
    }
    Ok(())
}

fn portable_relative_path(path: &Path) -> Result<String, PackageError> {
    let mut portable = String::new();
    for component in path.components() {
        let Component::Normal(value) = component else {
            return Err(PackageError::General(format!(
                "Package path is not a normalized relative path: {}",
                path.display()
            )));
        };
        let value = value.to_str().ok_or_else(|| {
            PackageError::General(format!(
                "Package path is not valid Unicode: {}",
                path.display()
            ))
        })?;
        if !portable.is_empty() {
            portable.push('/');
        }
        portable.push_str(value);
    }
    Ok(portable)
}

fn new_hasher() -> Sha256 {
    let mut hasher = Sha256::new();
    hasher.update(CHECKSUM_DOMAIN);
    hasher
}

fn finish_checksum(hasher: Sha256) -> String {
    format!("{}{:x}", CHECKSUM_PREFIX, hasher.finalize())
}

fn unsupported_object(path: &Path) -> PackageError {
    PackageError::General(format!(
        "Package contains an unsupported filesystem object: {}",
        path.display()
    ))
}

/// Verify an installed package against an expected checksum.
pub fn verify_checksum(path: &Path, expected: &str) -> Result<bool, PackageError> {
    let metadata = std::fs::symlink_metadata(path)?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() || (!file_type.is_dir() && !file_type.is_file()) {
        return Err(unsupported_object(path));
    }

    let mut hasher = new_hasher();
    if file_type.is_dir() {
        hash_verified_directory(path, path, &mut hasher)?;
    } else {
        hash_file(path, path, &mut hasher)?;
    }
    Ok(finish_checksum(hasher) == expected)
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
        assert!(checksum.starts_with("wflhash:v2:"));
        assert_eq!(checksum.len(), 11 + 64);
    }

    #[test]
    fn test_verify_checksum() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("test.wfl"), "display \"hello\"").unwrap();
        let checksum = compute_checksum(temp.path()).unwrap();
        assert!(verify_checksum(temp.path(), &checksum).unwrap());
        assert!(!verify_checksum(temp.path(), "wflhash:v2:invalid").unwrap());
    }

    #[test]
    fn test_deterministic() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("a.wfl"), "store x as 1").unwrap();
        std::fs::write(temp.path().join("b.wfl"), "store y as 2").unwrap();
        assert_eq!(
            compute_checksum(temp.path()).unwrap(),
            compute_checksum(temp.path()).unwrap()
        );
    }

    #[test]
    fn record_boundaries_cannot_collide() {
        let one = TempDir::new().unwrap();
        let two = TempDir::new().unwrap();
        let mut collision_payload = 1_u64.to_le_bytes().to_vec();
        collision_payload.push(b'b');
        std::fs::write(one.path().join("a"), collision_payload).unwrap();
        std::fs::write(two.path().join("a"), []).unwrap();
        std::fs::write(two.path().join("b"), []).unwrap();

        assert_ne!(
            compute_checksum(one.path()).unwrap(),
            compute_checksum(two.path()).unwrap()
        );
    }

    #[test]
    fn verification_does_not_hide_source_exclusions() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("main.wfl"), "display \"hello\"").unwrap();
        let expected = compute_checksum(temp.path()).unwrap();
        std::fs::create_dir(temp.path().join("packages")).unwrap();
        std::fs::write(temp.path().join("packages/evil.wfl"), "display \"evil\"").unwrap();

        assert!(!verify_checksum(temp.path(), &expected).unwrap());
    }

    #[test]
    fn completed_archive_checksum_matches_strict_extracted_verification() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        std::fs::create_dir_all(source.join("a")).unwrap();
        std::fs::write(source.join("a/nested.wfl"), "display \"nested\"").unwrap();
        std::fs::write(source.join("a.txt"), "asset").unwrap();
        std::fs::write(source.join("main.wfl"), "display \"main\"").unwrap();
        std::fs::write(
            source.join("project.wfl"),
            "name is demo\nversion is 26.1.1\ndescription is demo\nentry is main.wfl\n",
        )
        .unwrap();
        std::fs::write(source.join(".gitignore"), "secret.env\n").unwrap();
        std::fs::write(source.join("secret.env"), "TOKEN=secret").unwrap();

        let archive = temp.path().join("package.wflpkg");
        crate::archive::create_archive(&source, &archive).unwrap();
        let checksum = compute_archive_checksum(&archive, Path::new("main.wfl")).unwrap();
        assert_eq!(checksum, compute_checksum(&source).unwrap());

        let extracted = temp.path().join("extracted");
        crate::archive::extract_archive(&archive, &extracted).unwrap();
        assert!(verify_checksum(&extracted, &checksum).unwrap());
    }

    #[test]
    fn test_wflpkg_files_excluded_from_source_checksum() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("main.wfl"), "display \"hello\"").unwrap();
        let before = compute_checksum(temp.path()).unwrap();
        std::fs::write(temp.path().join("myproject.wflpkg"), b"archive data").unwrap();
        assert_eq!(before, compute_checksum(temp.path()).unwrap());
    }

    #[test]
    fn test_gitignored_files_excluded_from_source_checksum() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join("nested")).unwrap();
        std::fs::write(temp.path().join("main.wfl"), "display \"hello\"").unwrap();
        std::fs::write(temp.path().join(".gitignore"), ".env\n*.log\n!nested/\n").unwrap();
        let before = compute_checksum(temp.path()).unwrap();
        std::fs::write(temp.path().join(".env"), "API_TOKEN=secret").unwrap();
        std::fs::write(temp.path().join("debug.log"), "credential output").unwrap();
        std::fs::write(temp.path().join("nested/debug.log"), "credential output").unwrap();
        assert_eq!(before, compute_checksum(temp.path()).unwrap());
    }

    #[test]
    fn portable_path_uses_forward_slashes() {
        let path = Path::new("first").join("second").join("file.wfl");
        assert_eq!(
            portable_relative_path(&path).unwrap(),
            "first/second/file.wfl"
        );
    }

    #[cfg(unix)]
    #[test]
    fn root_symlinks_and_special_files_are_rejected() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target");
        std::fs::write(&target, "content").unwrap();
        let link = temp.path().join("link");
        symlink(&target, &link).unwrap();
        assert!(compute_checksum(&link).is_err());

        assert!(compute_checksum(Path::new("/dev/null")).is_err());
    }
}
