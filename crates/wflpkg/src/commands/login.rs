use crate::error::PackageError;
use crate::registry::auth::AuthManager;

/// Log in to the registry.
pub fn login(registry_url: &str) -> Result<(), PackageError> {
    let auth = AuthManager::new()?;

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
    use rustyline::DefaultEditor;
    let mut editor =
        DefaultEditor::new().map_err(|e| PackageError::General(format!("Input error: {}", e)))?;

    println!(
        "Visit https://{}/settings/tokens to generate an API token.",
        bare_host
    );
    println!();

    let token = editor
        .readline("Enter your API token: ")
        .map_err(|e| PackageError::General(format!("Input error: {}", e)))?;

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
