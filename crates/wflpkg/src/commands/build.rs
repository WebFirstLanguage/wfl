use std::path::Path;
use std::process::Command;

use crate::error::PackageError;
use crate::manifest::ProjectManifest;
use crate::workspace;

/// Build the project: ensure all dependencies are installed and validate.
pub async fn build_project(project_dir: &Path) -> Result<(), PackageError> {
    // Check for workspace
    let workspace_path = project_dir.join("workspace.wfl");
    if workspace_path.exists() {
        return build_workspace(project_dir).await;
    }

    let manifest_path = project_dir.join("project.wfl");
    if !manifest_path.exists() {
        return Err(PackageError::ManifestNotFound(
            project_dir.display().to_string(),
        ));
    }

    let manifest = ProjectManifest::load(&manifest_path)?;

    // Verify entry point exists
    let entry_path = project_dir.join(manifest.entry_point());
    if !entry_path.exists() {
        return Err(PackageError::General(format!(
            "The entry point \"{}\" does not exist.\n\
             Update the entry field in project.wfl or create the file.",
            manifest.entry_point()
        )));
    }

    // TODO: Ensure all dependencies are installed from lock file
    // TODO: Download missing dependencies from registry

    // Verify project can be parsed by invoking wfl --parse
    let status = Command::new("wfl")
        .arg("--parse")
        .arg(&entry_path)
        .current_dir(project_dir)
        .status();

    match status {
        Ok(s) if s.success() => {
            println!(
                "Build successful: {} {}",
                manifest.name, manifest.version_string
            );
            Ok(())
        }
        Ok(_) => Err(PackageError::General(format!(
            "Build failed: parse errors in {}",
            manifest.entry_point()
        ))),
        Err(e) => Err(PackageError::General(format!(
            "Could not run wfl to verify the build: {}\n\
             Make sure wfl is installed and in your PATH.",
            e
        ))),
    }
}

/// Build all members of a workspace.
async fn build_workspace(workspace_dir: &Path) -> Result<(), PackageError> {
    let ws = workspace::parser::parse_workspace_file(workspace_dir)?;

    println!(
        "Building workspace \"{}\" ({} members)...",
        ws.name,
        ws.members.len()
    );

    for member_path in &ws.members {
        let member_dir = workspace_dir.join(member_path);
        if !member_dir.exists() {
            return Err(PackageError::WorkspaceError(format!(
                "Workspace member directory does not exist: {}",
                member_path
            )));
        }

        println!("  Building {}...", member_path);
        Box::pin(build_project(&member_dir)).await?;
    }

    println!("Workspace build complete.");
    Ok(())
}
