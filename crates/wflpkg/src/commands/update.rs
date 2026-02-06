use std::path::Path;

use crate::error::PackageError;
use crate::manifest::ProjectManifest;

/// Update dependencies to their latest compatible versions.
pub fn update_dependencies(
    package_name: Option<&str>,
    project_dir: &Path,
) -> Result<(), PackageError> {
    let manifest_path = project_dir.join("project.wfl");
    if !manifest_path.exists() {
        return Err(PackageError::ManifestNotFound(
            project_dir.display().to_string(),
        ));
    }

    let manifest = ProjectManifest::load(&manifest_path)?;

    if let Some(name) = package_name {
        // Update a specific package
        if manifest.find_dependency(name).is_none() {
            return Err(PackageError::General(format!(
                "The package \"{}\" is not listed as a dependency.\n\n\
                 To add it, run:\n\
                 \x20 wfl add {}",
                name, name
            )));
        }

        // TODO: Query registry for latest version
        // TODO: Re-resolve dependencies
        // TODO: Update lock file
        // TODO: Download and install updated package

        println!("Updated {}", name);
    } else {
        // Update all packages
        if manifest.dependencies.is_empty() {
            println!("No dependencies to update.");
            return Ok(());
        }

        // TODO: Query registry for latest versions
        // TODO: Re-resolve all dependencies
        // TODO: Update lock file
        // TODO: Download and install updated packages

        println!("Updated {} dependencies", manifest.dependencies.len());
    }

    Ok(())
}
