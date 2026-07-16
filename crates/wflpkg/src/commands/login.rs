use crate::error::PackageError;
use crate::registry::auth::{AuthManager, normalize_registry_origin};

/// Default token reader that uses rpassword to hide input.
fn default_token_reader(prompt: &str) -> Result<String, PackageError> {
    rpassword::prompt_password(prompt)
        .map_err(|e| PackageError::General(format!("Input error: {}", e)))
}

/// Log in to the registry.
pub fn login(registry_url: &str) -> Result<(), PackageError> {
    login_with_reader(registry_url, &AuthManager::new()?, default_token_reader)
}

/// Log in to the registry with an injectable token reader (for testability).
fn login_with_reader<F>(
    registry_url: &str,
    auth: &AuthManager,
    reader: F,
) -> Result<(), PackageError>
where
    F: FnOnce(&str) -> Result<String, PackageError>,
{
    let registry_origin = normalize_registry_origin(registry_url)?;
    if let Some(credentials) = auth.get_credentials()? {
        if credentials.registry_origin() != registry_origin {
            return Err(PackageError::General(format!(
                "You are logged in to {}, not {}. Run `wfl logout`, verify the registry address, then log in again.",
                credentials.registry_origin(),
                registry_origin
            )));
        }
        println!(
            "You are already logged in to {}.",
            credentials.registry_origin()
        );
        println!("To log out first, run: wfl logout");
        return Ok(());
    }

    println!("Logging in to {}...", registry_origin);
    println!();

    // For now, use a simple token-based login via CLI prompt.
    // In the future, this will use browser-based OAuth.
    println!(
        "Visit {}/settings/tokens to generate an API token.",
        registry_origin
    );
    println!();

    let token = reader("Enter your API token: ")?;

    let token = token.trim();
    if token.is_empty() {
        return Err(PackageError::General("No token provided.".to_string()));
    }

    auth.store_token(token, &registry_origin)?;
    println!("Logged in successfully to {}.", registry_origin);

    Ok(())
}

/// Log out from the registry.
pub fn logout() -> Result<(), PackageError> {
    let auth = AuthManager::new()?;
    logout_with_auth(&auth)
}

fn logout_with_auth(auth: &AuthManager) -> Result<(), PackageError> {
    if !auth.credentials_file_exists()? {
        println!("You are not currently logged in.");
        return Ok(());
    }

    auth.clear_token()?;
    println!("Logged out successfully.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::auth::AuthManager;
    use std::sync::atomic::{AtomicBool, Ordering};

    fn temp_auth() -> (AuthManager, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("credentials.json");
        (AuthManager::with_path(path), dir)
    }

    #[test]
    fn test_login_calls_provided_reader() {
        let (auth, _dir) = temp_auth();
        let called = AtomicBool::new(false);
        let reader = |_prompt: &str| -> Result<String, PackageError> {
            called.store(true, Ordering::SeqCst);
            Ok("test-token-123".to_string())
        };

        let result = login_with_reader("https://registry.example.com", &auth, reader);
        assert!(result.is_ok());
        assert!(
            called.load(Ordering::SeqCst),
            "reader function was not called"
        );
        assert!(auth.is_authenticated());
    }

    #[test]
    fn test_login_empty_token_rejected() {
        let (auth, _dir) = temp_auth();
        let reader = |_: &str| -> Result<String, PackageError> { Ok("".to_string()) };

        let result = login_with_reader("https://registry.example.com", &auth, reader);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No token provided"), "got: {}", err);
    }

    #[test]
    fn test_login_whitespace_token_rejected() {
        let (auth, _dir) = temp_auth();
        let reader = |_: &str| -> Result<String, PackageError> { Ok("   \n".to_string()) };

        let result = login_with_reader("https://registry.example.com", &auth, reader);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("No token provided"), "got: {}", err);
    }

    #[test]
    fn test_login_reader_error_propagates() {
        let (auth, _dir) = temp_auth();
        let reader = |_: &str| -> Result<String, PackageError> {
            Err(PackageError::General(
                "Input error: device lost".to_string(),
            ))
        };

        let result = login_with_reader("https://registry.example.com", &auth, reader);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Input error: device lost"), "got: {}", err);
    }

    #[test]
    fn test_login_already_authenticated() {
        let (auth, _dir) = temp_auth();
        auth.store_token("existing-token", "https://registry.example.com")
            .unwrap();

        let reader = |_: &str| -> Result<String, PackageError> {
            panic!("reader should not be called when already authenticated");
        };

        let result = login_with_reader("https://registry.example.com", &auth, reader);
        assert!(result.is_ok());
    }

    #[test]
    fn test_login_rejects_different_authenticated_registry() {
        let (auth, _dir) = temp_auth();
        auth.store_token("existing-token", "https://registry.example.com")
            .unwrap();

        let result = login_with_reader("https://other.example.com", &auth, |_| {
            panic!("reader should not be called for existing credentials")
        });
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not https://other.example.com")
        );
    }

    #[test]
    fn test_logout_recovers_malformed_auth_then_allows_login() {
        let (auth, dir) = temp_auth();
        let path = dir.path().join("credentials.json");

        for malformed in [
            "not json",
            r#"{"token":"secret"}"#,
            r#"{"token":"secret","registry":"http://registry.example"}"#,
        ] {
            std::fs::write(&path, malformed).unwrap();
            logout_with_auth(&auth).unwrap();
            assert!(!path.exists());
            login_with_reader("registry.example", &auth, |_| Ok("replacement".to_string()))
                .unwrap();
            auth.clear_token().unwrap();
        }
    }

    #[test]
    fn test_login_trims_token() {
        let (auth, _dir) = temp_auth();
        let reader = |_: &str| -> Result<String, PackageError> { Ok("  tok123  ".to_string()) };

        let result = login_with_reader("https://registry.example.com", &auth, reader);
        assert!(result.is_ok());
        let loaded = auth.get_token().unwrap().unwrap();
        assert_eq!(loaded, "tok123");
    }
}
