use std::path::Path;
use tokio::process::Command;

use crate::error::PackageError;
use crate::lockfile::LockFile;
use crate::manifest::ProjectManifest;
use crate::manifest::version::Version;
use crate::registry::advisory;

/// Check for security advisories affecting project dependencies.
pub async fn check_security(project_dir: &Path) -> Result<(), PackageError> {
    let manifest_path = project_dir.join("project.wfl");
    if !manifest_path.exists() {
        return Err(PackageError::ManifestNotFound(
            project_dir.display().to_string(),
        ));
    }

    let manifest = ProjectManifest::load(&manifest_path)?;
    let lock_path = project_dir.join("project.lock");

    // Collect packages to check
    let packages: Vec<(String, Version)> = if lock_path.exists() {
        let lock = LockFile::load(&lock_path)?;
        lock.packages
            .iter()
            .map(|p| (p.name.clone(), p.version.clone()))
            .collect()
    } else {
        Vec::new()
    };

    if packages.is_empty() {
        println!("No locked dependencies to check.");
        println!("Run 'wfl update' to generate a lock file first.");
        return Ok(());
    }

    let registry_url = format!("https://{}", manifest.registry_url());
    let advisories = advisory::check_advisories(&registry_url, &packages).await?;

    if advisories.is_empty() {
        println!("No security advisories found. Your dependencies are up to date.");
    } else {
        println!(
            "I found {} security {} affecting your dependencies.\n",
            advisories.len(),
            if advisories.len() == 1 {
                "advisory"
            } else {
                "advisories"
            }
        );
        for adv in &advisories {
            println!("  {} — {}: {}", adv.package, adv.severity, adv.description);
            if let Some(ref fixed) = adv.fixed_in {
                println!("    Fixed in {}. Run: wfl update {}", fixed, adv.package);
            }
        }
        println!("\nTo update all affected packages at once, run:");
        println!("  wfl update");
    }

    Ok(())
}

/// Check compatibility of the current package version.
/// Uses `wfl --parse` to extract the public API (action and container names).
pub async fn check_compatibility(project_dir: &Path) -> Result<(), PackageError> {
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
            "The entry point \"{}\" does not exist.",
            manifest.entry_point()
        )));
    }

    // Use wfl --parse to verify the file parses, then do a basic scan
    // of the source for action/container definitions
    let status = Command::new("wfl")
        .arg("--parse")
        .arg(&entry_path)
        .current_dir(project_dir)
        .status()
        .await;

    match status {
        Ok(s) if !s.success() => {
            return Err(PackageError::General(format!(
                "Cannot check compatibility: {} has parse errors.",
                manifest.entry_point()
            )));
        }
        Err(e) => {
            return Err(PackageError::General(format!("Could not run wfl: {}", e)));
        }
        _ => {}
    }

    // Do a simple text-based scan for public API elements
    let content = std::fs::read_to_string(&entry_path)?;
    let mut actions = Vec::new();
    let mut containers = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("define action called ") {
            let name = trimmed
                .strip_prefix("define action called ")
                .unwrap_or("")
                .split_whitespace()
                .next()
                .unwrap_or("");
            if !name.is_empty() {
                actions.push(name.to_string());
            }
        } else if trimmed.starts_with("create container called ") {
            let name = trimmed
                .strip_prefix("create container called ")
                .unwrap_or("")
                .split_whitespace()
                .next()
                .unwrap_or("");
            if !name.is_empty() {
                containers.push(name.to_string());
            }
        }
    }

    println!(
        "Compatibility check for {} {}:",
        manifest.name, manifest.version_string
    );
    println!();
    println!("  Public API:");
    if !actions.is_empty() {
        println!("    Actions: {}", actions.join(", "));
    }
    if !containers.is_empty() {
        println!("    Containers: {}", containers.join(", "));
    }
    if actions.is_empty() && containers.is_empty() {
        println!("    (no public actions or containers found)");
    }

    // TODO: Compare with previous published version from registry
    println!();
    println!("Note: Full compatibility comparison with the previously published version");
    println!("will be available once the registry is live.");

    Ok(())
}
