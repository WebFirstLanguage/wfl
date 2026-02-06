use std::path::{Path, PathBuf};

use crate::error::PackageError;
use crate::manifest::ProjectManifest;

/// Resolve the entry point for a package installed in `packages/<name>/`.
pub fn resolve_package_entry(name: &str, project_dir: &Path) -> Result<PathBuf, PackageError> {
    if name.contains('/') || name.contains('\\') || name.contains("..") || name.is_empty() {
        return Err(PackageError::General(format!(
            "Invalid package name \"{}\": must not contain path separators or '..' components.",
            name
        )));
    }
    let package_dir = project_dir.join("packages").join(name);

    if !package_dir.exists() {
        return Err(PackageError::General(format!(
            "The package \"{}\" is not installed.\n\n\
             To install it, run:\n\
             \x20 wfl add {}",
            name, name
        )));
    }

    // Look for project.wfl in the package directory
    let manifest_path = package_dir.join("project.wfl");
    if manifest_path.exists() {
        let manifest = ProjectManifest::load(&manifest_path)?;
        let entry = manifest.entry_point();
        let entry_path = package_dir.join(entry);
        if entry_path.exists() {
            // Canonicalize both paths to resolve symlinks and `..` components,
            // then verify the entry point is inside the package directory.
            let canon_pkg = package_dir.canonicalize().map_err(|e| {
                PackageError::General(format!(
                    "Could not canonicalize package directory \"{}\": {}",
                    package_dir.display(),
                    e
                ))
            })?;
            let canon_entry = entry_path.canonicalize().map_err(|e| {
                PackageError::General(format!(
                    "Could not canonicalize entry point \"{}\": {}",
                    entry_path.display(),
                    e
                ))
            })?;
            if !canon_entry.starts_with(&canon_pkg) {
                return Err(PackageError::General(format!(
                    "The package \"{}\" declares an unsafe entry point \"{}\" \
                     that escapes the package directory.",
                    name, entry
                )));
            }
            return Ok(canon_entry);
        }
        return Err(PackageError::General(format!(
            "The package \"{}\" declares entry point \"{}\" but the file does not exist.",
            name, entry
        )));
    }

    // Fallback: look for src/main.wfl
    let default_entry = package_dir.join("src").join("main.wfl");
    if default_entry.exists() {
        let canon = default_entry.canonicalize().map_err(|e| {
            PackageError::General(format!(
                "Could not canonicalize fallback entry \"{}\": {}",
                default_entry.display(),
                e
            ))
        })?;
        let canon_pkg = package_dir.canonicalize().map_err(|e| {
            PackageError::General(format!(
                "Could not canonicalize package directory \"{}\": {}",
                package_dir.display(),
                e
            ))
        })?;
        if !canon.starts_with(&canon_pkg) {
            return Err(PackageError::General(format!(
                "Fallback entry point escapes the package directory for \"{}\".",
                name
            )));
        }
        return Ok(canon);
    }

    // Fallback: look for main.wfl in package root
    let root_main = package_dir.join("main.wfl");
    if root_main.exists() {
        let canon = root_main.canonicalize().map_err(|e| {
            PackageError::General(format!(
                "Could not canonicalize fallback entry \"{}\": {}",
                root_main.display(),
                e
            ))
        })?;
        let canon_pkg = package_dir.canonicalize().map_err(|e| {
            PackageError::General(format!(
                "Could not canonicalize package directory \"{}\": {}",
                package_dir.display(),
                e
            ))
        })?;
        if !canon.starts_with(&canon_pkg) {
            return Err(PackageError::General(format!(
                "Fallback entry point escapes the package directory for \"{}\".",
                name
            )));
        }
        return Ok(canon);
    }

    Err(PackageError::General(format!(
        "I could not find an entry point for the package \"{}\".\n\n\
         The package directory exists at {}\n\
         but does not contain a project.wfl, src/main.wfl, or main.wfl file.",
        name,
        package_dir.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_with_manifest() {
        let temp = TempDir::new().unwrap();
        let pkg_dir = temp.path().join("packages").join("my-lib");
        std::fs::create_dir_all(pkg_dir.join("src")).unwrap();
        std::fs::write(
            pkg_dir.join("project.wfl"),
            "name is my-lib\nversion is 26.1.1\ndescription is Test\nentry is src/main.wfl",
        )
        .unwrap();
        std::fs::write(pkg_dir.join("src").join("main.wfl"), "// entry").unwrap();

        let result = resolve_package_entry("my-lib", temp.path()).unwrap();
        assert!(result.ends_with("main.wfl"));
    }

    #[test]
    fn test_resolve_not_installed() {
        let temp = TempDir::new().unwrap();
        let result = resolve_package_entry("missing-pkg", temp.path());
        assert!(result.is_err());
    }
}
