use std::path::Path;

use crate::archive;
use crate::checksum;
use crate::error::PackageError;
use crate::manifest::ProjectManifest;
use crate::manifest::version::Version;
use crate::registry::api::RegistryClient;
use crate::registry::auth::AuthManager;

/// Share (publish) the current project to the registry.
pub async fn share_package(project_dir: &Path) -> Result<(), PackageError> {
    let manifest_path = project_dir.join("project.wfl");
    if !manifest_path.exists() {
        return Err(PackageError::ManifestNotFound(
            project_dir.display().to_string(),
        ));
    }

    let manifest = ProjectManifest::load(&manifest_path)?;
    let version = Version::parse(&manifest.version_string)?;

    // Check authentication
    let auth = AuthManager::new()?;
    let token = auth.get_token()?.ok_or(PackageError::NotAuthenticated)?;

    // Validate manifest
    if manifest.name.is_empty() || manifest.description.is_empty() {
        return Err(PackageError::General(
            "Your project.wfl must have a name and description before sharing.".to_string(),
        ));
    }

    // Verify entry point exists
    let entry_path = project_dir.join(manifest.entry_point());
    if !entry_path.exists() {
        return Err(PackageError::General(format!(
            "The entry point \"{}\" does not exist.\n\
             Update the entry field in project.wfl or create the file.",
            manifest.entry_point()
        )));
    }

    println!("Preparing to share {} {}...", manifest.name, version);

    // Create archive
    let archive_path = project_dir.join(format!("{}-{}.wflpkg", manifest.name, version));
    archive::create_archive(project_dir, &archive_path)?;

    // Compute checksum
    let checksum = checksum::compute_checksum(&archive_path)?;

    // Upload to registry
    let mut client = RegistryClient::new(&format!("https://{}", manifest.registry_url()));
    client.set_auth_token(token);

    client
        .publish(&manifest.name, &version, &archive_path, &checksum)
        .await?;

    // Clean up archive
    let _ = std::fs::remove_file(&archive_path);

    println!(
        "Shared {} {} to {}",
        manifest.name,
        version,
        manifest.registry_url()
    );

    Ok(())
}
