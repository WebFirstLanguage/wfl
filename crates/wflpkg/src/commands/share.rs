use std::path::{Component, Path, PathBuf};

use crate::archive;
use crate::checksum;
use crate::error::PackageError;
use crate::manifest::ProjectManifest;
use crate::manifest::version::Version;
use crate::registry::api::RegistryClient;
use crate::registry::auth::{AuthManager, normalize_registry_origin};

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

/// Share (publish) the current project to the registry.
pub async fn share_package(project_dir: &Path) -> Result<(), PackageError> {
    let project_dir = std::fs::canonicalize(project_dir)?;
    let project_dir = project_dir.as_path();
    let manifest_path = project_dir.join("project.wfl");
    let manifest = load_manifest_no_follow(&manifest_path, project_dir)?;
    let version = Version::parse(&manifest.version_string)?;

    // Check authentication
    let auth = AuthManager::new()?;
    let credentials = auth
        .get_credentials()?
        .ok_or(PackageError::NotAuthenticated)?;
    let registry_origin =
        registry_for_credentials(manifest.registry_url(), credentials.registry_origin())?;

    // Validate manifest
    if manifest.name.is_empty() || manifest.description.is_empty() {
        return Err(PackageError::General(
            "Your project.wfl must have a name and description before sharing.".to_string(),
        ));
    }

    // Reject absolute/traversing/symlink/directory entry points before doing
    // any packaging work. Presence in the completed archive is checked below
    // as the authoritative validation against ignore rules and races.
    let entry_point = validate_entry_point(project_dir, manifest.entry_point())?;

    println!("Preparing to share {} {}...", manifest.name, version);

    // Create the upload in a private, external temporary directory. Keeping
    // the TempDir alive provides automatic cleanup on every return path and
    // prevents a project-controlled symlink from redirecting archive output.
    let archive_file = create_upload_archive(project_dir)?;

    // Derive the checksum from the completed immutable archive. This also
    // verifies that the declared regular entry point is actually included.
    let checksum = checksum::compute_archive_checksum(archive_file.path(), &entry_point)?;

    // Upload to registry
    let mut client = RegistryClient::new(&registry_origin)?;
    client.set_auth_token(credentials.token().to_string());

    client
        .publish(&manifest.name, &version, archive_file.path(), &checksum)
        .await?;

    println!(
        "Shared {} {} to {}",
        manifest.name, version, registry_origin
    );

    Ok(())
}

fn load_manifest_no_follow(
    manifest_path: &Path,
    project_dir: &Path,
) -> Result<ProjectManifest, PackageError> {
    use std::io::Read;

    match std::fs::symlink_metadata(manifest_path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(PackageError::General(
                "project.wfl must be a regular, non-symlink file.".to_string(),
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(PackageError::ManifestNotFound(
                project_dir.display().to_string(),
            ));
        }
        Err(error) => return Err(PackageError::Io(error)),
    }

    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = match options.open(manifest_path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(PackageError::ManifestNotFound(
                project_dir.display().to_string(),
            ));
        }
        Err(error) => return Err(PackageError::Io(error)),
    };
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_MANIFEST_BYTES {
        return Err(PackageError::General(
            "project.wfl must be a regular file no larger than 1 MiB.".to_string(),
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_MANIFEST_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(PackageError::General(
            "project.wfl must be no larger than 1 MiB.".to_string(),
        ));
    }
    let content = String::from_utf8(bytes)
        .map_err(|_| PackageError::General("project.wfl must contain valid UTF-8.".to_string()))?;
    crate::manifest::parser::parse_manifest(&content)
}

fn validate_entry_point(project_dir: &Path, entry: &str) -> Result<PathBuf, PackageError> {
    let entry_path = Path::new(entry);
    if entry_path.as_os_str().is_empty() || entry_path.is_absolute() {
        return Err(invalid_entry_point(entry));
    }
    for component in entry_path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(invalid_entry_point(entry));
        }
    }

    let metadata = std::fs::symlink_metadata(project_dir.join(entry_path)).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            invalid_entry_point(entry)
        } else {
            PackageError::Io(error)
        }
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(invalid_entry_point(entry));
    }
    Ok(entry_path.to_path_buf())
}

fn invalid_entry_point(entry: &str) -> PackageError {
    PackageError::General(format!(
        "The entry point \"{}\" must be a normalized relative path to a regular, non-symlink file inside the project.",
        entry
    ))
}

fn registry_for_credentials(
    manifest_registry: &str,
    authenticated_registry: &str,
) -> Result<String, PackageError> {
    let requested_origin = normalize_registry_origin(manifest_registry)?;
    let authenticated_origin = normalize_registry_origin(authenticated_registry)?;
    if requested_origin != authenticated_origin {
        return Err(PackageError::General(format!(
            "This project is configured to use {}, but your saved credentials belong to {}.\n\n\
             Refusing to send that token to a different registry. Log out, verify the project's registry setting, and log in to the intended registry.",
            requested_origin, authenticated_origin
        )));
    }
    Ok(requested_origin)
}

struct UploadArchive {
    _directory: tempfile::TempDir,
    path: std::path::PathBuf,
}

impl UploadArchive {
    fn path(&self) -> &Path {
        &self.path
    }
}

fn create_upload_archive(project_dir: &Path) -> Result<UploadArchive, PackageError> {
    let directory = tempfile::Builder::new()
        .prefix("wflpkg-upload-")
        .tempdir()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700))?;
    }
    let path = directory.path().join("package.wflpkg");
    archive::create_archive(project_dir, &path)?;
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)?
        .sync_all()?;
    Ok(UploadArchive {
        _directory: directory,
        path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_are_bound_to_the_same_origin() {
        assert_eq!(
            registry_for_credentials("wflhub.org", "https://wflhub.org/").unwrap(),
            "https://wflhub.org"
        );
        assert!(registry_for_credentials("wflhub.org@evil.example", "wflhub.org").is_err());
        assert!(registry_for_credentials("evil.example", "wflhub.org").is_err());
    }

    #[test]
    fn upload_archive_is_external_and_removed_on_drop() {
        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("main.wfl"), "display \"hello\"").unwrap();

        let archive_path = {
            let archive = create_upload_archive(project.path()).unwrap();
            assert!(!archive.path().starts_with(project.path()));
            assert!(archive.path().exists());
            archive.path().to_path_buf()
        };
        assert!(!archive_path.exists());
    }

    #[test]
    fn entry_point_must_be_an_in_project_regular_file() {
        let project = tempfile::tempdir().unwrap();
        std::fs::create_dir(project.path().join("src")).unwrap();
        std::fs::write(project.path().join("src/main.wfl"), "display \"hello\"").unwrap();

        assert_eq!(
            validate_entry_point(project.path(), "src/main.wfl").unwrap(),
            Path::new("src/main.wfl")
        );
        assert!(validate_entry_point(project.path(), "../outside.wfl").is_err());
        assert!(validate_entry_point(project.path(), project.path().to_str().unwrap()).is_err());
        assert!(validate_entry_point(project.path(), "src").is_err());
    }

    #[test]
    fn ignored_entry_point_is_rejected_from_completed_archive() {
        let project = tempfile::tempdir().unwrap();
        std::fs::create_dir(project.path().join("src")).unwrap();
        std::fs::write(project.path().join("src/main.wfl"), "display \"hello\"").unwrap();
        std::fs::write(
            project.path().join("project.wfl"),
            "name is demo\nversion is 26.1.1\ndescription is demo\nentry is src/main.wfl\n",
        )
        .unwrap();
        std::fs::write(project.path().join(".gitignore"), "src/main.wfl\n").unwrap();

        let entry = validate_entry_point(project.path(), "src/main.wfl").unwrap();
        let archive = create_upload_archive(project.path()).unwrap();
        assert!(checksum::compute_archive_checksum(archive.path(), &entry).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_manifest_is_rejected() {
        use std::os::unix::fs::symlink;

        let project = tempfile::tempdir().unwrap();
        let target = project.path().join("manifest-target");
        std::fs::write(
            &target,
            "name is demo\nversion is 26.1.1\ndescription is demo\n",
        )
        .unwrap();
        let manifest = project.path().join("project.wfl");
        symlink(&target, &manifest).unwrap();
        assert!(load_manifest_no_follow(&manifest, project.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_entry_point_is_rejected() {
        use std::os::unix::fs::symlink;

        let project = tempfile::tempdir().unwrap();
        std::fs::write(project.path().join("target.wfl"), "display \"hello\"").unwrap();
        symlink("target.wfl", project.path().join("main.wfl")).unwrap();
        assert!(validate_entry_point(project.path(), "main.wfl").is_err());
    }
}
