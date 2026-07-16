use crate::error::PackageError;
use crate::manifest::version::Version;

/// Default connect timeout for registry requests (30 seconds).
const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
/// Default request timeout for registry requests (5 minutes).
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);
/// Maximum compressed package accepted for one publish request (100 MiB).
const MAX_PUBLISH_ARCHIVE_BYTES: u64 = 100 * 1024 * 1024;
/// Maximum registry response body retained by the client (1 MiB).
const MAX_REGISTRY_RESPONSE_BYTES: usize = 1024 * 1024;

/// A client for communicating with the WFL package registry.
pub struct RegistryClient {
    base_url: String,
    auth_token: Option<String>,
    client: reqwest::Client,
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
    pub fn new(base_url: &str) -> Result<Self, PackageError> {
        let client = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| PackageError::General(format!("HTTP client error: {}", e)))?;
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_token: None,
            client,
        })
    }

    /// Set the authentication token.
    pub fn set_auth_token(&mut self, token: String) {
        self.auth_token = Some(token);
    }

    /// Search for packages matching a query.
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>, PackageError> {
        let url = build_search_url(&self.base_url, query)?;
        let response =
            self.client.get(&url).send().await.map_err(|e| {
                PackageError::RegistryUnreachable(format!("{}: {}", self.base_url, e))
            })?;

        if !response.status().is_success() {
            return Err(PackageError::Http(format!(
                "Registry returned status {}",
                response.status()
            )));
        }

        let bytes = read_response_bounded(response).await?;
        let body: serde_json::Value = serde_json::from_slice(&bytes)
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
        let url = build_package_url(&self.base_url, name)?;
        let response =
            self.client.get(&url).send().await.map_err(|e| {
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

        let bytes = read_response_bounded(response).await?;
        let body: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| PackageError::Http(format!("Failed to parse response: {}", e)))?;

        let versions: Vec<Version> = body["versions"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| Version::parse(v.as_str()?).ok())
            .collect();

        let latest = versions
            .iter()
            .max()
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
        let (archive_file, archive_len) = open_publish_archive(archive_path).await?;

        let form = reqwest::multipart::Form::new()
            .text("name", name.to_string())
            .text("version", version.to_string())
            .text("checksum", checksum.to_string())
            .part(
                "archive",
                reqwest::multipart::Part::stream_with_length(archive_file, archive_len)
                    .file_name(format!("{}-{}.wflpkg", name, version)),
            );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .multipart(form)
            .send()
            .await
            .map_err(|e| PackageError::RegistryUnreachable(format!("{}: {}", self.base_url, e)))?;

        if !response.status().is_success() {
            let body = read_response_bounded(response).await?;
            let body = String::from_utf8_lossy(&body);
            return Err(PackageError::Http(format!("Failed to publish: {}", body)));
        }

        Ok(())
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

async fn open_publish_archive(
    archive_path: &std::path::Path,
) -> Result<(tokio::fs::File, u64), PackageError> {
    let metadata = tokio::fs::symlink_metadata(archive_path)
        .await
        .map_err(PackageError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(PackageError::General(
            "The upload archive is not a regular file.".to_string(),
        ));
    }
    if metadata.len() > MAX_PUBLISH_ARCHIVE_BYTES {
        return Err(PackageError::General(format!(
            "The compressed package exceeds the {} MiB publish limit.",
            MAX_PUBLISH_ARCHIVE_BYTES / (1024 * 1024)
        )));
    }
    let file = tokio::fs::File::open(archive_path)
        .await
        .map_err(PackageError::Io)?;
    let opened_metadata = file.metadata().await.map_err(PackageError::Io)?;
    if !opened_metadata.is_file() || opened_metadata.len() != metadata.len() {
        return Err(PackageError::General(
            "The upload archive changed while it was being opened.".to_string(),
        ));
    }
    Ok((file, opened_metadata.len()))
}

async fn read_response_bounded(mut response: reqwest::Response) -> Result<Vec<u8>, PackageError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_REGISTRY_RESPONSE_BYTES as u64)
    {
        return Err(response_too_large_error());
    }

    let mut body = Vec::with_capacity(
        response
            .content_length()
            .unwrap_or(0)
            .min(MAX_REGISTRY_RESPONSE_BYTES as u64) as usize,
    );
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| PackageError::Http(format!("Failed to read response: {}", error)))?
    {
        append_response_chunk(&mut body, &chunk)?;
    }
    Ok(body)
}

fn append_response_chunk(body: &mut Vec<u8>, chunk: &[u8]) -> Result<(), PackageError> {
    let next_len = body
        .len()
        .checked_add(chunk.len())
        .ok_or_else(response_too_large_error)?;
    if next_len > MAX_REGISTRY_RESPONSE_BYTES {
        return Err(response_too_large_error());
    }
    body.extend_from_slice(chunk);
    Ok(())
}

fn response_too_large_error() -> PackageError {
    PackageError::Http("Registry response exceeded the 1 MiB safety limit.".to_string())
}

/// Percent-encode a string for use as a URL path segment (RFC 3986).
/// Unreserved characters (A-Z, a-z, 0-9, `-`, `.`, `_`, `~`) pass through unchanged.
fn percent_encode_path_segment(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    encoded
}

/// Build a properly-encoded search URL with the query as a `q` parameter.
fn build_search_url(base_url: &str, query: &str) -> Result<String, PackageError> {
    let base = format!("{}/api/v1/search", base_url);
    let mut url = reqwest::Url::parse(&base)
        .map_err(|e| PackageError::Http(format!("Invalid base URL: {}", e)))?;
    url.query_pairs_mut().append_pair("q", query);
    Ok(url.to_string())
}

/// Build a properly-encoded package URL with the name as a path segment.
fn build_package_url(base_url: &str, name: &str) -> Result<String, PackageError> {
    let encoded_name = percent_encode_path_segment(name);
    let base = format!("{}/api/v1/packages/{}", base_url, encoded_name);
    // Validate the URL is well-formed
    reqwest::Url::parse(&base).map_err(|e| PackageError::Http(format!("Invalid URL: {}", e)))?;
    Ok(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "https://registry.example.com";

    #[tokio::test]
    async fn oversized_publish_archive_is_rejected_before_upload() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("oversized.wflpkg");
        let file = std::fs::File::create(&archive).unwrap();
        file.set_len(MAX_PUBLISH_ARCHIVE_BYTES + 1).unwrap();
        assert!(open_publish_archive(&archive).await.is_err());
    }

    #[test]
    fn oversized_registry_response_is_rejected() {
        let mut body = vec![0; MAX_REGISTRY_RESPONSE_BYTES];
        assert!(append_response_chunk(&mut body, &[1]).is_err());
        assert_eq!(body.len(), MAX_REGISTRY_RESPONSE_BYTES);
    }

    // --- search URL tests ---

    #[test]
    fn test_search_url_simple_query() {
        let url = build_search_url(BASE, "my-package").unwrap();
        assert_eq!(
            url,
            "https://registry.example.com/api/v1/search?q=my-package"
        );
    }

    #[test]
    fn test_search_url_encodes_spaces() {
        let url = build_search_url(BASE, "hello world").unwrap();
        assert!(url.contains("q=hello"));
        // Must not contain a raw space
        assert!(!url.contains("q=hello world"));
    }

    #[test]
    fn test_search_url_encodes_ampersand() {
        let url = build_search_url(BASE, "foo&bar=evil").unwrap();
        // The ampersand must be encoded — only one `q=` param should exist
        assert!(url.contains("q=foo"));
        assert!(!url.contains("&bar=evil"));
    }

    #[test]
    fn test_search_url_encodes_hash() {
        let url = build_search_url(BASE, "foo#fragment").unwrap();
        // Hash must not truncate the URL; query must contain the full value
        assert!(url.contains("q=foo"));
        assert!(!url.ends_with("#fragment"));
    }

    #[test]
    fn test_search_url_encodes_question_mark() {
        let url = build_search_url(BASE, "foo?extra").unwrap();
        // Only one `?` should appear (the query delimiter)
        let question_marks = url.matches('?').count();
        assert_eq!(question_marks, 1);
    }

    #[test]
    fn test_search_url_empty_query() {
        let url = build_search_url(BASE, "").unwrap();
        assert_eq!(url, "https://registry.example.com/api/v1/search?q=");
    }

    // --- package URL tests ---

    #[test]
    fn test_package_url_simple_name() {
        let url = build_package_url(BASE, "my-package").unwrap();
        assert_eq!(
            url,
            "https://registry.example.com/api/v1/packages/my-package"
        );
    }

    #[test]
    fn test_package_url_encodes_slash() {
        let url = build_package_url(BASE, "foo/bar").unwrap();
        assert!(url.contains("foo%2Fbar"));
        // Must not create a new path segment
        assert!(!url.contains("packages/foo/bar"));
    }

    #[test]
    fn test_package_url_encodes_dot_dot() {
        let url = build_package_url(BASE, "../secret").unwrap();
        assert!(url.contains("..%2Fsecret"));
    }

    #[test]
    fn test_package_url_encodes_hash() {
        let url = build_package_url(BASE, "pkg#frag").unwrap();
        assert!(url.contains("pkg%23frag"));
        assert!(!url.contains('#'));
    }

    #[test]
    fn test_package_url_encodes_question_mark() {
        let url = build_package_url(BASE, "pkg?q=evil").unwrap();
        assert!(url.contains("pkg%3Fq%3Devil"));
        assert!(!url.contains('?'));
    }

    // --- percent_encode_path_segment tests ---

    #[test]
    fn test_encode_unreserved_chars_unchanged() {
        let input = "ABCxyz019-._~";
        assert_eq!(percent_encode_path_segment(input), input);
    }

    #[test]
    fn test_encode_special_chars() {
        let encoded = percent_encode_path_segment("a/b?c#d&e f");
        assert_eq!(encoded, "a%2Fb%3Fc%23d%26e%20f");
    }

    // --- RegistryClient construction tests ---

    #[test]
    fn test_registry_client_strips_trailing_slash() {
        let client = RegistryClient::new("https://example.com/").unwrap();
        assert_eq!(client.base_url(), "https://example.com");
    }

    #[test]
    fn test_registry_client_preserves_clean_url() {
        let client = RegistryClient::new("https://example.com").unwrap();
        assert_eq!(client.base_url(), "https://example.com");
    }

    #[test]
    fn test_set_auth_token_does_not_panic() {
        let mut client = RegistryClient::new(BASE).unwrap();
        client.set_auth_token("secret-token".to_string());
        // Should still work after setting token
        assert_eq!(client.base_url(), BASE);
    }

    #[tokio::test]
    async fn test_publish_without_auth_returns_not_authenticated() {
        let client = RegistryClient::new(BASE).unwrap();
        let version = crate::manifest::version::Version::new(26, 1, Some(0));
        let fake_path = std::path::Path::new("/nonexistent/archive.wflpkg");
        let result = client
            .publish("test-pkg", &version, fake_path, "abc123")
            .await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("not logged in"),
            "expected NotAuthenticated, got: {msg}"
        );
    }

    #[test]
    fn test_build_search_url_invalid_base() {
        let result = build_search_url("not a url", "q");
        assert!(result.is_err(), "invalid base URL should fail");
    }
}
