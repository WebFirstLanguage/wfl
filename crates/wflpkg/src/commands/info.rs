use crate::error::PackageError;
use crate::registry::api::RegistryClient;

/// Show detailed information about a package from the registry.
pub async fn show_package_info(name: &str, registry_url: &str) -> Result<(), PackageError> {
    let client = RegistryClient::new(&format!("https://{}", registry_url))?;
    let info = client.get_package_info(name).await?;

    println!("{} ({})", info.name, info.latest_version);
    println!();
    if !info.description.is_empty() {
        println!("  {}", info.description);
        println!();
    }
    if !info.author.is_empty() {
        println!("  Author:  {}", info.author);
    }
    if !info.license.is_empty() {
        println!("  License: {}", info.license);
    }
    if info.downloads > 0 {
        println!("  Downloads: {}", info.downloads);
    }

    if !info.versions.is_empty() {
        println!();
        println!("  Versions:");
        for version in info.versions.iter().rev().take(10) {
            println!("    {}", version);
        }
        if info.versions.len() > 10 {
            println!("    ... and {} more", info.versions.len() - 10);
        }
    }

    println!();
    println!("To add this package, run:");
    println!("  wfl add {}", name);

    Ok(())
}
