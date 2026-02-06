use crate::error::PackageError;
use crate::manifest::version::Version;

/// A client for communicating with the WFL package registry.
pub struct RegistryClient {
    base_url: String,
    auth_token: Option<String>,
}

/// Package metadata returned from the registry.
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub name: String,
    pub description: String,
    pub latest_version: Version,
    pub versions: Vec<Version>,
    pub author: String,
    pub license: String,
    pub downloads: u64,
}

/// Search result from the registry.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub name: String,
    pub description: String,
    pub version: Version,
    pub downloads: u64,
}

impl RegistryClient {
    /// Create a new registry client for the given base URL.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_token: None,
        }
    }

    /// Set the authentication token.
    pub fn set_auth_token(&mut self, token: String) {
        self.auth_token = Some(token);
    }

    /// Search for packages matching a query.
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>, PackageError> {
        let url = format!("{}/api/v1/search?q={}", self.base_url, query);
        let client = reqwest::Client::new();
        let response =
            client.get(&url).send().await.map_err(|e| {
                PackageError::RegistryUnreachable(format!("{}: {}", self.base_url, e))
            })?;

        if !response.status().is_success() {
            return Err(PackageError::Http(format!(
                "Registry returned status {}",
                response.status()
            )));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| PackageError::Http(format!("Failed to parse response: {}", e)))?;

        let results = body
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| {
                Some(SearchResult {
                    name: v["name"].as_str()?.to_string(),
                    description: v["description"].as_str().unwrap_or("").to_string(),
                    version: Version::parse(v["version"].as_str()?).ok()?,
                    downloads: v["downloads"].as_u64().unwrap_or(0),
                })
            })
            .collect();

        Ok(results)
    }

    /// Get detailed information about a package.
    pub async fn get_package_info(&self, name: &str) -> Result<PackageInfo, PackageError> {
        let url = format!("{}/api/v1/packages/{}", self.base_url, name);
        let client = reqwest::Client::new();
        let response =
            client.get(&url).send().await.map_err(|e| {
                PackageError::RegistryUnreachable(format!("{}: {}", self.base_url, e))
            })?;

        if response.status().as_u16() == 404 {
            return Err(PackageError::PackageNotFound {
                name: name.to_string(),
                suggestions: Vec::new(),
            });
        }

        if !response.status().is_success() {
            return Err(PackageError::Http(format!(
                "Registry returned status {}",
                response.status()
            )));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| PackageError::Http(format!("Failed to parse response: {}", e)))?;

        let versions: Vec<Version> = body["versions"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| Version::parse(v.as_str()?).ok())
            .collect();

        let latest = versions
            .last()
            .cloned()
            .unwrap_or(Version::new(0, 1, Some(0)));

        Ok(PackageInfo {
            name: body["name"].as_str().unwrap_or(name).to_string(),
            description: body["description"].as_str().unwrap_or("").to_string(),
            latest_version: latest,
            versions,
            author: body["author"].as_str().unwrap_or("").to_string(),
            license: body["license"].as_str().unwrap_or("").to_string(),
            downloads: body["downloads"].as_u64().unwrap_or(0),
        })
    }

    /// Get available versions for a package.
    pub async fn get_versions(&self, name: &str) -> Result<Vec<Version>, PackageError> {
        let info = self.get_package_info(name).await?;
        Ok(info.versions)
    }

    /// Upload a package archive to the registry.
    pub async fn publish(
        &self,
        name: &str,
        version: &Version,
        archive_path: &std::path::Path,
        checksum: &str,
    ) -> Result<(), PackageError> {
        let token = self
            .auth_token
            .as_ref()
            .ok_or(PackageError::NotAuthenticated)?;

        let url = format!("{}/api/v1/packages", self.base_url);
        let archive_data = tokio::fs::read(archive_path)
            .await
            .map_err(PackageError::Io)?;

        let client = reqwest::Client::new();
        let form = reqwest::multipart::Form::new()
            .text("name", name.to_string())
            .text("version", version.to_string())
            .text("checksum", checksum.to_string())
            .part(
                "archive",
                reqwest::multipart::Part::bytes(archive_data)
                    .file_name(format!("{}-{}.wflpkg", name, version)),
            );

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .multipart(form)
            .send()
            .await
            .map_err(|e| PackageError::RegistryUnreachable(format!("{}: {}", self.base_url, e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(PackageError::Http(format!("Failed to publish: {}", body)));
        }

        Ok(())
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}
