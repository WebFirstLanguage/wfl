use crate::error::PackageError;
use crate::registry::api::RegistryClient;

/// Search the registry for packages matching a query.
pub async fn search_packages(query: &str, registry_url: &str) -> Result<(), PackageError> {
    let client = RegistryClient::new(&format!("https://{}", registry_url))?;
    let results = client.search(query).await?;

    if results.is_empty() {
        println!("No packages found matching \"{}\".", query);
        return Ok(());
    }

    println!("Packages matching \"{}\":\n", query);
    for result in &results {
        println!(
            "  {} ({}) — {}",
            result.name, result.version, result.description
        );
        if result.downloads > 0 {
            println!("    {} downloads", result.downloads);
        }
    }
    println!("\nFound {} packages.", results.len());

    Ok(())
}
