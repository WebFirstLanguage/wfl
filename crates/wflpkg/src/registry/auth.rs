use std::path::PathBuf;

use crate::error::PackageError;

/// Manages authentication tokens for registry access.
pub struct AuthManager {
    auth_file: PathBuf,
}

/// Stored authentication data.
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct AuthData {
    token: Option<String>,
    registry: Option<String>,
}

impl AuthManager {
    /// Create a new auth manager using the default auth file location.
    pub fn new() -> Result<Self, PackageError> {
        let home = get_home_dir()?;
        let auth_file = home.join(".wfl").join("auth.json");
        Ok(Self { auth_file })
    }

    /// Create an auth manager with a custom path (for testing).
    pub fn with_path(path: PathBuf) -> Self {
        Self { auth_file: path }
    }

    /// Get the stored authentication token.
    pub fn get_token(&self) -> Result<Option<String>, PackageError> {
        if !self.auth_file.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&self.auth_file)?;
        let data: AuthData = serde_json::from_str(&content)
            .map_err(|e| PackageError::General(format!("Failed to parse auth file: {}", e)))?;
        Ok(data.token)
    }

    /// Store an authentication token.
    pub fn store_token(&self, token: &str, registry: &str) -> Result<(), PackageError> {
        use zeroize::Zeroize;

        if let Some(parent) = self.auth_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut token_owned = token.to_string();
        let mut data = AuthData {
            token: Some(token_owned.clone()),
            registry: Some(registry.to_string()),
        };

        let mut content = serde_json::to_string_pretty(&data)
            .map_err(|e| PackageError::General(format!("Failed to serialize auth data: {}", e)))?;

        std::fs::write(&self.auth_file, &content)?;

        // Set restrictive permissions on Unix (owner read/write only).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&self.auth_file, perms)?;
        }

        // Zeroize secrets to prevent them lingering in memory.
        content.zeroize();
        token_owned.zeroize();
        if let Some(ref mut t) = data.token {
            t.zeroize();
        }
        if let Some(ref mut r) = data.registry {
            r.zeroize();
        }

        Ok(())
    }

    /// Remove the stored authentication token.
    pub fn clear_token(&self) -> Result<(), PackageError> {
        if self.auth_file.exists() {
            std::fs::remove_file(&self.auth_file)?;
        }
        Ok(())
    }

    /// Check if we have a stored token.
    pub fn is_authenticated(&self) -> bool {
        self.get_token().ok().flatten().is_some()
    }
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
}
