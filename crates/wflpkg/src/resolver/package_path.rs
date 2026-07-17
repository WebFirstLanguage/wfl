use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::error::PackageError;
use crate::manifest::ProjectManifest;
use crate::manifest::parser::validate_package_name;

fn not_installed(name: &str) -> PackageError {
    PackageError::General(format!(
        "The package \"{}\" is not installed.\n\n\
         To install it, run:\n\
         \x20 wfl add {}",
        name, name
    ))
}

fn verified_package_directory(name: &str, project_dir: &Path) -> Result<PathBuf, PackageError> {
    let canonical_project = project_dir.canonicalize().map_err(|error| {
        PackageError::General(format!(
            "Could not verify project directory \"{}\": {}",
            project_dir.display(),
            error
        ))
    })?;
    let packages_dir = canonical_project.join("packages");
    let packages_metadata = match std::fs::symlink_metadata(&packages_dir) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Err(not_installed(name)),
        Err(error) => return Err(error.into()),
    };
    if packages_metadata.file_type().is_symlink() {
        return Err(PackageError::General(format!(
            "Refusing to resolve package \"{}\": packages directory \"{}\" is a symbolic link.",
            name,
            packages_dir.display()
        )));
    }
    if !packages_metadata.is_dir() {
        return Err(PackageError::General(format!(
            "Refusing to resolve package \"{}\": \"{}\" is not a directory.",
            name,
            packages_dir.display()
        )));
    }

    let canonical_packages = packages_dir.canonicalize().map_err(|error| {
        PackageError::General(format!(
            "Could not verify packages directory \"{}\": {}",
            packages_dir.display(),
            error
        ))
    })?;
    if canonical_packages.parent() != Some(canonical_project.as_path()) {
        return Err(PackageError::General(format!(
            "Refusing to resolve package \"{}\": packages directory escapes the project.",
            name
        )));
    }

    let package_dir = canonical_packages.join(name);
    let package_metadata = match std::fs::symlink_metadata(&package_dir) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Err(not_installed(name)),
        Err(error) => return Err(error.into()),
    };
    if package_metadata.file_type().is_symlink() {
        return Err(PackageError::General(format!(
            "Refusing to resolve package \"{}\": package directory \"{}\" is a symbolic link.",
            name,
            package_dir.display()
        )));
    }
    if !package_metadata.is_dir() {
        return Err(PackageError::General(format!(
            "Refusing to resolve package \"{}\": \"{}\" is not a directory.",
            name,
            package_dir.display()
        )));
    }

    let canonical_package = package_dir.canonicalize().map_err(|error| {
        PackageError::General(format!(
            "Could not verify package directory \"{}\": {}",
            package_dir.display(),
            error
        ))
    })?;
    if canonical_package.parent() != Some(canonical_packages.as_path()) {
        return Err(PackageError::General(format!(
            "Refusing to resolve package \"{}\": package directory escapes the packages directory.",
            name
        )));
    }

    Ok(canonical_package)
}

/// Resolve the entry point for a package installed in `packages/<name>/`.
pub fn resolve_package_entry(name: &str, project_dir: &Path) -> Result<PathBuf, PackageError> {
    validate_package_name(name)?;
    let package_dir = verified_package_directory(name, project_dir)?;

    // Look for project.wfl in the package directory
    let manifest_path = package_dir.join("project.wfl");
    let manifest_metadata = match std::fs::symlink_metadata(&manifest_path) {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    if let Some(manifest_metadata) = manifest_metadata {
        if manifest_metadata.file_type().is_symlink() {
            return Err(PackageError::General(format!(
                "Refusing to resolve package \"{}\": package manifest \"{}\" is a symbolic link.",
                name,
                manifest_path.display()
            )));
        }
        if !manifest_metadata.is_file() {
            return Err(PackageError::General(format!(
                "Refusing to resolve package \"{}\": package manifest \"{}\" is not a regular file.",
                name,
                manifest_path.display()
            )));
        }
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
