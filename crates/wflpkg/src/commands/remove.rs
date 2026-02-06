use std::path::Path;

use crate::error::PackageError;
use crate::manifest::ProjectManifest;

/// Remove a dependency from the project.
pub fn remove_dependency(name: &str, project_dir: &Path) -> Result<(), PackageError> {
    let manifest_path = project_dir.join("project.wfl");
    if !manifest_path.exists() {
        return Err(PackageError::ManifestNotFound(
            project_dir.display().to_string(),
        ));
    }

    let mut manifest = ProjectManifest::load(&manifest_path)?;

    if !manifest.remove_dependency(name) {
        return Err(PackageError::General(format!(
            "The package \"{}\" is not listed as a dependency in your project.wfl.\n\n\
             To see your current dependencies, open project.wfl and look for\n\
             lines starting with \"requires\".",
            name
        )));
    }

    manifest.save(&manifest_path)?;

    // Clean up local packages directory
    let pkg_dir = project_dir.join("packages").join(name);
    if pkg_dir.exists() {
        std::fs::remove_dir_all(&pkg_dir)?;
    }

    // TODO: Update lock file

    println!("Removed {} from project.wfl", name);

    Ok(())
}
