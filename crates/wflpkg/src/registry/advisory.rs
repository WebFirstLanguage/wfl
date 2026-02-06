use crate::error::PackageError;
use crate::manifest::version::Version;

/// A security advisory for a package.
#[derive(Debug, Clone)]
pub struct Advisory {
    pub package: String,
    pub severity: String,
    pub description: String,
    pub affected_versions: String,
    pub fixed_in: Option<Version>,
}

/// Query the registry's advisory database for known vulnerabilities.
pub async fn check_advisories(
    registry_url: &str,
    packages: &[(String, Version)],
) -> Result<Vec<Advisory>, PackageError> {
    if packages.is_empty() {
        return Ok(Vec::new());
    }

    let url = format!("{}/api/v1/advisories", registry_url.trim_end_matches('/'));

    let package_list: Vec<serde_json::Value> = packages
        .iter()
        .map(|(name, version)| {
            serde_json::json!({
                "name": name,
                "version": version.to_string()
            })
        })
        .collect();

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .unwrap_or_default();
    let response = client
        .post(&url)
        .json(&serde_json::json!({ "packages": package_list }))
        .send()
        .await
        .map_err(|e| PackageError::RegistryUnreachable(format!("{}: {}", registry_url, e)))?;

    if !response.status().is_success() {
        return Err(PackageError::Http(format!(
            "Advisory check returned status {}",
            response.status()
        )));
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| PackageError::Http(format!("Failed to parse advisory response: {}", e)))?;

    let advisories = body
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| {
            Some(Advisory {
                package: v["package"].as_str()?.to_string(),
                severity: v["severity"].as_str().unwrap_or("UNKNOWN").to_string(),
                description: v["description"].as_str().unwrap_or("").to_string(),
                affected_versions: v["affected"].as_str().unwrap_or("").to_string(),
                fixed_in: v["fixed_in"].as_str().and_then(|s| Version::parse(s).ok()),
            })
        })
        .collect();

    Ok(advisories)
}
