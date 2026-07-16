use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::error::PackageError;
use crate::manifest::ProjectManifest;
use crate::manifest::parser::validate_package_name;

fn directory_to_remove(
    name: &str,
    canonical_project: &Path,
) -> Result<Option<PathBuf>, PackageError> {
    let packages_dir = canonical_project.join("packages");
    let packages_metadata = match std::fs::symlink_metadata(&packages_dir) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };

    if packages_metadata.file_type().is_symlink() {
        return Err(PackageError::General(format!(
            "Refusing to remove package \"{}\": the packages directory \"{}\" is a symbolic link.",
            name,
            packages_dir.display()
        )));
    }
    if !packages_metadata.is_dir() {
        return Err(PackageError::General(format!(
            "Refusing to remove package \"{}\": \"{}\" is not a directory.",
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
    if canonical_packages.parent() != Some(canonical_project) {
        return Err(PackageError::General(format!(
            "Refusing to remove package \"{}\": packages directory escapes the project.",
            name
        )));
    }

    let package_dir = packages_dir.join(name);
    let package_metadata = match std::fs::symlink_metadata(&package_dir) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };

    if package_metadata.file_type().is_symlink() {
        return Err(PackageError::General(format!(
            "Refusing to remove package \"{}\": package directory \"{}\" is a symbolic link.",
            name,
            package_dir.display()
        )));
    }
    if !package_metadata.is_dir() {
        return Err(PackageError::General(format!(
            "Refusing to remove package \"{}\": \"{}\" is not a directory.",
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
            "Refusing to remove package \"{}\": package directory escapes the packages directory.",
            name
        )));
    }

    Ok(Some(canonical_package))
}

/// Remove a dependency from the project.
pub fn remove_dependency(name: &str, project_dir: &Path) -> Result<(), PackageError> {
    validate_package_name(name)?;

    let requested_manifest_path = project_dir.join("project.wfl");
    let manifest_metadata = match std::fs::symlink_metadata(&requested_manifest_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Err(PackageError::ManifestNotFound(
                project_dir.display().to_string(),
            ));
        }
        Err(error) => return Err(error.into()),
    };
    if manifest_metadata.file_type().is_symlink() {
        return Err(PackageError::General(format!(
            "Refusing to remove package \"{}\": project manifest \"{}\" is a symbolic link.",
            name,
            requested_manifest_path.display()
        )));
    }
    if !manifest_metadata.is_file() {
        return Err(PackageError::General(format!(
            "Refusing to remove package \"{}\": project manifest \"{}\" is not a regular file.",
            name,
            requested_manifest_path.display()
        )));
    }
    let canonical_project = project_dir.canonicalize().map_err(|error| {
        PackageError::General(format!(
            "Could not verify project directory \"{}\": {}",
            project_dir.display(),
            error
        ))
    })?;
    let manifest_path = canonical_project.join("project.wfl");

    let mut manifest = ProjectManifest::load(&manifest_path)?;

    if manifest.find_dependency(name).is_none() {
        return Err(PackageError::General(format!(
            "The package \"{}\" is not listed as a dependency in your project.wfl.\n\n\
             To see your current dependencies, open project.wfl and look for\n\
             lines starting with \"requires\".",
            name
        )));
    }

    // Resolve and verify every component before changing the manifest. In
    // particular, never pass a caller-controlled or symlinked path to the
    // recursive remover.
    let package_dir = directory_to_remove(name, &canonical_project)?;

    manifest.remove_dependency(name);
    manifest.save(&manifest_path)?;

    // Clean up local packages directory
    if let Some(package_dir) = package_dir {
        std::fs::remove_dir_all(package_dir)?;
    }

    // TODO: Update lock file

    println!("Removed {} from project.wfl", name);

    Ok(())
}
