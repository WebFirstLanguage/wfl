use std::path::{Path, PathBuf};

use crate::error::PackageError;
use crate::manifest::ProjectManifest;

/// Resolve the entry point for a package installed in `packages/<name>/`.
pub fn resolve_package_entry(name: &str, project_dir: &Path) -> Result<PathBuf, PackageError> {
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
            return Ok(entry_path);
        }
        return Err(PackageError::General(format!(
            "The package \"{}\" declares entry point \"{}\" but the file does not exist.",
            name, entry
        )));
    }

    // Fallback: look for src/main.wfl
    let default_entry = package_dir.join("src").join("main.wfl");
    if default_entry.exists() {
        return Ok(default_entry);
    }

    // Fallback: look for main.wfl in package root
    let root_main = package_dir.join("main.wfl");
    if root_main.exists() {
        return Ok(root_main);
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
