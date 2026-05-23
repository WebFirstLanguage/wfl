use crate::error::PackageError;
use crate::registry::auth::AuthManager;

/// Default token reader that uses rpassword to hide input.
fn default_token_reader(prompt: &str) -> Result<String, PackageError> {
    rpassword::prompt_password_stdout(prompt)
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
    if auth.is_authenticated() {
        println!("You are already logged in.");
        println!("To log out first, run: wfl logout");
        return Ok(());
    }

    let bare_host = registry_url
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    println!("Logging in to {}...", bare_host);
    println!();

    // For now, use a simple token-based login via CLI prompt.
    // In the future, this will use browser-based OAuth.
    println!(
        "Visit https://{}/settings/tokens to generate an API token.",
        bare_host
    );
    println!();

    let token = reader("Enter your API token: ")?;

    let token = token.trim();
    if token.is_empty() {
        return Err(PackageError::General("No token provided.".to_string()));
    }

    auth.store_token(token, registry_url)?;
    println!("Logged in successfully to {}.", bare_host);

    Ok(())
}

/// Log out from the registry.
pub fn logout() -> Result<(), PackageError> {
    let auth = AuthManager::new()?;

    if !auth.is_authenticated() {
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
    fn test_login_trims_token() {
        let (auth, _dir) = temp_auth();
        let reader = |_: &str| -> Result<String, PackageError> { Ok("  tok123  ".to_string()) };

        let result = login_with_reader("https://registry.example.com", &auth, reader);
        assert!(result.is_ok());
        let loaded = auth.get_token().unwrap().unwrap();
        assert_eq!(loaded, "tok123");
    }
}
