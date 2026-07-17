use std::path::PathBuf;

use crate::error::PackageError;

const MAX_AUTH_FILE_BYTES: u64 = 64 * 1024;
const MAX_TOKEN_BYTES: usize = 16 * 1024;

/// Manages authentication tokens for registry access.
pub struct AuthManager {
    auth_file: PathBuf,
    // Only consulted when tightening directory permissions, which is a
    // Unix-only concern. On non-Unix targets the field is intentionally unread.
    #[cfg_attr(not(unix), allow(dead_code))]
    manage_parent_permissions: bool,
}

/// Stored authentication data.
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct AuthData {
    token: Option<String>,
    registry: Option<String>,
}

/// Authentication data bound to one canonical HTTPS registry origin.
pub struct RegistryCredentials {
    token: String,
    registry_origin: String,
}

impl RegistryCredentials {
    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn registry_origin(&self) -> &str {
        &self.registry_origin
    }
}

impl AuthManager {
    /// Create a new auth manager using the default auth file location.
    pub fn new() -> Result<Self, PackageError> {
        let home = get_home_dir()?;
        let auth_file = home.join(".wfl").join("auth.json");
        Ok(Self {
            auth_file,
            manage_parent_permissions: true,
        })
    }

    /// Create an auth manager with a custom path (for testing).
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            auth_file: path,
            manage_parent_permissions: false,
        }
    }

    /// Get the stored authentication token.
    pub fn get_token(&self) -> Result<Option<String>, PackageError> {
        Ok(self.get_credentials()?.map(|credentials| credentials.token))
    }

    /// Get the stored token together with its canonical registry origin.
    pub fn get_credentials(&self) -> Result<Option<RegistryCredentials>, PackageError> {
        match std::fs::symlink_metadata(&self.auth_file) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(PackageError::General(
                    "The credentials path is not a regular file. Run `wfl logout`, then `wfl login` again."
                        .to_string(),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(PackageError::Io(error)),
        }
        let content = read_auth_file_no_follow(&self.auth_file)?;
        let data: AuthData = serde_json::from_str(&content)
            .map_err(|e| PackageError::General(format!("Failed to parse auth file: {}", e)))?;
        match (data.token, data.registry) {
            (Some(token), Some(registry)) if !token.trim().is_empty() => {
                let registry_origin = normalize_registry_origin(&registry).map_err(|_| {
                    PackageError::General(
                        "Stored credentials have an invalid registry scope. Run `wfl logout`, then `wfl login` again."
                            .to_string(),
                    )
                })?;
                Ok(Some(RegistryCredentials {
                    token,
                    registry_origin,
                }))
            }
            (None, None) => Ok(None),
            _ => Err(PackageError::General(
                "Stored credentials are incomplete. Run `wfl logout`, then `wfl login` again."
                    .to_string(),
            )),
        }
    }

    /// Store an authentication token.
    pub fn store_token(&self, token: &str, registry: &str) -> Result<(), PackageError> {
        use zeroize::Zeroize;

        if token.trim().is_empty() {
            return Err(PackageError::General(
                "Cannot store an empty authentication token.".to_string(),
            ));
        }
        if token.len() > MAX_TOKEN_BYTES {
            return Err(PackageError::General(
                "Authentication token exceeds the 16 KiB safety limit.".to_string(),
            ));
        }
        let registry_origin = normalize_registry_origin(registry)?;

        {
            let parent = self
                .auth_file
                .parent()
                .filter(|path| !path.as_os_str().is_empty())
                .unwrap_or_else(|| std::path::Path::new("."));
            // Whether we just created the directory. Only read on Unix, where a
            // freshly created credentials directory is locked down to 0o700.
            #[cfg_attr(not(unix), allow(unused_variables))]
            let created_parent = match std::fs::symlink_metadata(parent) {
                Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                    return Err(PackageError::General(
                        "The credentials directory must be a real directory, not a symlink."
                            .to_string(),
                    ));
                }
                Ok(_) => false,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    std::fs::create_dir_all(parent)?;
                    true
                }
                Err(error) => return Err(PackageError::Io(error)),
            };

            #[cfg(unix)]
            if created_parent || self.manage_parent_permissions {
                use std::os::unix::fs::PermissionsExt;
                let parent_handle = open_directory_no_follow(parent)?;
                parent_handle.set_permissions(std::fs::Permissions::from_mode(0o700))?;
            }
        }

        let mut token_owned = token.to_string();
        let mut data = AuthData {
            token: Some(token_owned.clone()),
            registry: Some(registry_origin),
        };

        let mut content = serde_json::to_string_pretty(&data)
            .map_err(|e| PackageError::General(format!("Failed to serialize auth data: {}", e)))?;

        let write_result = self.write_auth_file(content.as_bytes());

        // Zeroize secrets to prevent them lingering in memory.
        content.zeroize();
        token_owned.zeroize();
        if let Some(ref mut t) = data.token {
            t.zeroize();
        }
        if let Some(ref mut r) = data.registry {
            r.zeroize();
        }

        write_result
    }

    /// Remove the stored authentication token.
    pub fn clear_token(&self) -> Result<(), PackageError> {
        match std::fs::symlink_metadata(&self.auth_file) {
            Ok(_) => std::fs::remove_file(&self.auth_file)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(PackageError::Io(error)),
        }
        Ok(())
    }

    /// Check whether anything exists at the credentials path without parsing
    /// or following it. This lets logout recover malformed and symlinked auth.
    pub fn credentials_file_exists(&self) -> Result<bool, PackageError> {
        match std::fs::symlink_metadata(&self.auth_file) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(PackageError::Io(error)),
        }
    }

    /// Check if we have a stored token.
    pub fn is_authenticated(&self) -> bool {
        self.get_credentials().ok().flatten().is_some()
    }

    fn write_auth_file(&self, content: &[u8]) -> Result<(), PackageError> {
        use std::io::Write;

        let parent = self
            .auth_file
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new("."));
        let mut temp = tempfile::Builder::new()
            .prefix(".auth-")
            .tempfile_in(parent)?;

        // NamedTempFile is private by default. Set the mode before writing as
        // an explicit invariant so the token is never briefly world-readable.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            temp.as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }

        temp.write_all(content)?;
        temp.as_file().sync_all()?;
        let persisted = temp
            .persist(&self.auth_file)
            .map_err(|error| PackageError::Io(error.error))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            persisted.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        persisted.sync_all()?;
        Ok(())
    }
}

fn read_auth_file_no_follow(path: &std::path::Path) -> Result<String, PackageError> {
    use std::io::Read;

    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || metadata.len() > MAX_AUTH_FILE_BYTES {
        return Err(PackageError::General(
            "The credentials file is not a safe regular file.".to_string(),
        ));
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_AUTH_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_AUTH_FILE_BYTES {
        return Err(PackageError::General(
            "The credentials file exceeds the 64 KiB safety limit.".to_string(),
        ));
    }
    String::from_utf8(bytes)
        .map_err(|_| PackageError::General("The credentials file is not valid UTF-8.".to_string()))
}

#[cfg(unix)]
fn open_directory_no_follow(path: &std::path::Path) -> Result<std::fs::File, PackageError> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    use std::os::unix::fs::OpenOptionsExt;
    options.custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW);
    let directory = options.open(path)?;
    if !directory.metadata()?.is_dir() {
        return Err(PackageError::General(
            "The credentials directory is not a directory.".to_string(),
        ));
    }
    Ok(directory)
}

/// Parse a registry setting and reduce it to a canonical HTTPS origin.
///
/// Manifests traditionally contain a bare host (for example `wflhub.org`),
/// while auth files may contain either that form or a full HTTPS URL. Paths,
/// userinfo, queries, and fragments are rejected because credentials are
/// scoped to an origin, not to an attacker-controlled URL string.
pub fn normalize_registry_origin(registry: &str) -> Result<String, PackageError> {
    let registry = registry.trim();
    if registry.is_empty() {
        return Err(PackageError::General(
            "Registry address cannot be empty.".to_string(),
        ));
    }

    let candidate = if registry.contains("://") {
        registry.to_string()
    } else {
        format!("https://{}", registry)
    };
    let url = reqwest::Url::parse(&candidate)
        .map_err(|_| PackageError::General("Registry address is not a valid URL.".to_string()))?;

    if url.scheme() != "https" {
        return Err(PackageError::General(
            "Registry credentials may only be sent to an HTTPS registry.".to_string(),
        ));
    }
    if url.host_str().is_none() {
        return Err(PackageError::General(
            "Registry address must include a host.".to_string(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(PackageError::General(
            "Registry address must not contain a username or password.".to_string(),
        ));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(PackageError::General(
            "Registry address must not contain a query or fragment.".to_string(),
        ));
    }
    if url.path() != "/" && !url.path().is_empty() {
        return Err(PackageError::General(
            "Registry address must not contain a path.".to_string(),
        ));
    }

    Ok(url.origin().ascii_serialization())
}

/// Get the user's home directory.
fn get_home_dir() -> Result<PathBuf, PackageError> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .map_err(|_| PackageError::General("Could not determine home directory".to_string()))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| PackageError::General("Could not determine home directory".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_store_and_retrieve_token() {
        let temp = TempDir::new().unwrap();
        let auth = AuthManager::with_path(temp.path().join("auth.json"));

        assert!(!auth.is_authenticated());
        assert!(auth.get_token().unwrap().is_none());

        auth.store_token("test-token-123", "wflhub.org").unwrap();
        assert!(auth.is_authenticated());
        assert_eq!(auth.get_token().unwrap().unwrap(), "test-token-123");
        assert_eq!(
            auth.get_credentials().unwrap().unwrap().registry_origin(),
            "https://wflhub.org"
        );
    }

    #[test]
    fn test_clear_token() {
        let temp = TempDir::new().unwrap();
        let auth = AuthManager::with_path(temp.path().join("auth.json"));

        auth.store_token("token", "wflhub.org").unwrap();
        assert!(auth.is_authenticated());

        auth.clear_token().unwrap();
        assert!(!auth.is_authenticated());
    }

    #[test]
    fn test_clear_token_removes_malformed_credentials() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("auth.json");
        let auth = AuthManager::with_path(path.clone());

        for malformed in [
            "not json",
            r#"{"token":"secret"}"#,
            r#"{"token":"secret","registry":"http://registry.example"}"#,
        ] {
            std::fs::write(&path, malformed).unwrap();
            assert!(auth.get_credentials().is_err());
            assert!(auth.credentials_file_exists().unwrap());
            auth.clear_token().unwrap();
            assert!(!auth.credentials_file_exists().unwrap());
            auth.store_token("replacement", "registry.example").unwrap();
            assert!(auth.is_authenticated());
            auth.clear_token().unwrap();
        }
    }

    #[test]
    fn test_normalize_registry_origin() {
        assert_eq!(
            normalize_registry_origin("wflhub.org").unwrap(),
            "https://wflhub.org"
        );
        assert_eq!(
            normalize_registry_origin("HTTPS://WFLHUB.ORG:443/").unwrap(),
            "https://wflhub.org"
        );
        assert!(normalize_registry_origin("http://wflhub.org").is_err());
        assert!(normalize_registry_origin("wflhub.org@evil.example").is_err());
        assert!(normalize_registry_origin("wflhub.org/api").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn test_store_token_replaces_symlink_without_touching_target() {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target.txt");
        std::fs::write(&target, "keep me").unwrap();
        let auth_path = temp.path().join("auth.json");
        symlink(&target, &auth_path).unwrap();
        let auth = AuthManager::with_path(auth_path.clone());

        auth.store_token("secret", "wflhub.org").unwrap();

        assert_eq!(std::fs::read_to_string(&target).unwrap(), "keep me");
        assert!(
            !std::fs::symlink_metadata(&auth_path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(auth.get_token().unwrap().as_deref(), Some("secret"));
    }

    #[cfg(unix)]
    #[test]
    fn test_store_token_creates_private_file_and_directory() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempDir::new().unwrap();
        let auth_dir = temp.path().join("credentials");
        let auth_path = auth_dir.join("auth.json");
        let auth = AuthManager::with_path(auth_path.clone());

        auth.store_token("secret", "wflhub.org").unwrap();

        assert_eq!(
            std::fs::metadata(&auth_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(&auth_dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_custom_auth_path_does_not_chmod_existing_parent() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempDir::new().unwrap();
        let parent = temp.path().join("custom");
        std::fs::create_dir(&parent).unwrap();
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o750)).unwrap();
        let auth = AuthManager::with_path(parent.join("auth.json"));

        auth.store_token("secret", "wflhub.org").unwrap();
        assert_eq!(
            std::fs::metadata(&parent).unwrap().permissions().mode() & 0o777,
            0o750
        );
    }
}
