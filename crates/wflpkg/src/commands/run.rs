use std::path::Path;
use std::process::Command;

use crate::error::PackageError;
use crate::manifest::ProjectManifest;

/// Run the project's entry point by invoking `wfl`.
pub async fn run_project(project_dir: &Path) -> Result<(), PackageError> {
    let manifest_path = project_dir.join("project.wfl");
    if !manifest_path.exists() {
        return Err(PackageError::ManifestNotFound(
            project_dir.display().to_string(),
        ));
    }

    let manifest = ProjectManifest::load(&manifest_path)?;
    let entry_path = project_dir.join(manifest.entry_point());

    if !entry_path.exists() {
        return Err(PackageError::General(format!(
            "The entry point \"{}\" does not exist.\n\
             Update the entry field in project.wfl or create the file.",
            manifest.entry_point()
        )));
    }

    // Execute the entry point using the wfl binary
    let status = Command::new("wfl")
        .arg(&entry_path)
        .current_dir(project_dir)
        .status();

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => {
            std::process::exit(s.code().unwrap_or(1));
        }
        Err(e) => Err(PackageError::General(format!(
            "Could not run wfl: {}\n\
             Make sure wfl is installed and in your PATH.",
            e
        ))),
    }
}
