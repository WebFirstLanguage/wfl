use std::path::Path;
use tokio::process::Command;

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

    let canon_project = project_dir.canonicalize().map_err(|e| {
        PackageError::General(format!("Could not canonicalize project directory: {}", e))
    })?;
    let canon_entry = entry_path
        .canonicalize()
        .map_err(|e| PackageError::General(format!("Could not canonicalize entry point: {}", e)))?;
    if !canon_entry.starts_with(&canon_project) {
        return Err(PackageError::General(format!(
            "The entry point \"{}\" resolves outside the project directory.",
            manifest.entry_point()
        )));
    }

    let status = Command::new("wfl")
        .arg(&canon_entry)
        .current_dir(project_dir)
        .status()
        .await;

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(PackageError::General(format!(
            "wfl exited with status code {}",
            s.code().unwrap_or(1)
        ))),
        Err(e) => Err(PackageError::General(format!(
            "Could not run wfl: {}\n\
             Make sure wfl is installed and in your PATH.",
            e
        ))),
    }
}
