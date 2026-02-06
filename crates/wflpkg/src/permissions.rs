use crate::error::PackageError;

/// Permission types that packages can declare with `needs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Permission {
    FileAccess,
    NetworkAccess,
    SystemAccess,
    Unknown(String),
}

impl Permission {
    /// Parse a permission string.
    pub fn parse(s: &str) -> Self {
        match s.trim() {
            "file-access" => Permission::FileAccess,
            "network-access" => Permission::NetworkAccess,
            "system-access" => Permission::SystemAccess,
            other => Permission::Unknown(other.to_string()),
        }
    }

    /// Get a human-readable description of the permission.
    pub fn description(&self) -> &str {
        match self {
            Permission::FileAccess => "Can read and write files on disk",
            Permission::NetworkAccess => "Can make HTTP requests",
            Permission::SystemAccess => "Can execute system commands",
            Permission::Unknown(_) => "Unknown permission",
        }
    }

    /// Get the permission identifier string.
    pub fn name(&self) -> &str {
        match self {
            Permission::FileAccess => "file-access",
            Permission::NetworkAccess => "network-access",
            Permission::SystemAccess => "system-access",
            Permission::Unknown(s) => s,
        }
    }
}

/// Prompt the user to confirm permissions for a package.
/// Returns Ok(true) if confirmed, Ok(false) if denied.
pub fn confirm_permissions(
    package_name: &str,
    permissions: &[String],
) -> Result<bool, PackageError> {
    if permissions.is_empty() {
        return Ok(true);
    }

    let parsed: Vec<Permission> = permissions.iter().map(|s| Permission::parse(s)).collect();

    println!(
        "The package \"{}\" needs the following permissions:",
        package_name
    );
    for perm in &parsed {
        println!("  - {}: {}", perm.name(), perm.description());
    }
    println!();

    // Use rustyline for input
    use rustyline::DefaultEditor;
    let mut editor =
        DefaultEditor::new().map_err(|e| PackageError::General(format!("Input error: {}", e)))?;

    let response = editor
        .readline("Do you want to add this package? (yes/no): ")
        .map_err(|e| PackageError::General(format!("Input error: {}", e)))?;

    Ok(response.trim().to_lowercase() == "yes" || response.trim().to_lowercase() == "y")
}
