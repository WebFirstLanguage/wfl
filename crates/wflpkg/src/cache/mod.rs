use std::path::{Path, PathBuf};

use crate::error::PackageError;
use crate::manifest::version::Version;

/// Manages the global package cache at `~/.wfl/packages/`.
pub struct PackageCache {
    cache_dir: PathBuf,
}

impl PackageCache {
    /// Create a new cache manager using the default global cache directory.
    pub fn new() -> Result<Self, PackageError> {
        let home = dirs_home()?;
        let cache_dir = home.join(".wfl").join("packages");
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
    }

    /// Create a cache manager with a custom directory (for testing).
    pub fn with_dir(cache_dir: PathBuf) -> Result<Self, PackageError> {
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
    }

    /// Get the path for a specific package version in the cache.
    pub fn package_path(&self, name: &str, version: &Version) -> PathBuf {
        self.cache_dir.join(name).join(version.to_string())
    }

    /// Check if a package version is cached.
    pub fn is_cached(&self, name: &str, version: &Version) -> bool {
        self.package_path(name, version).exists()
    }

    /// Store a package in the cache by copying from a source directory.
    pub fn store(&self, name: &str, version: &Version, source: &Path) -> Result<(), PackageError> {
        let dest = self.package_path(name, version);
        if dest.exists() {
            std::fs::remove_dir_all(&dest)?;
        }
        copy_dir_recursive(source, &dest)?;
        Ok(())
    }

    /// Install a cached package into the project's `packages/` directory.
    pub fn install_to_project(
        &self,
        name: &str,
        version: &Version,
        project_dir: &Path,
    ) -> Result<(), PackageError> {
        let cache_path = self.package_path(name, version);
        if !cache_path.exists() {
            return Err(PackageError::General(format!(
                "Package {} {} is not in the cache.",
                name, version
            )));
        }

        let dest = project_dir.join("packages").join(name);
        if dest.exists() {
            std::fs::remove_dir_all(&dest)?;
        }
        std::fs::create_dir_all(&dest)?;
        copy_dir_recursive(&cache_path, &dest)?;
        Ok(())
    }

    /// List all cached versions of a package.
    pub fn list_versions(&self, name: &str) -> Result<Vec<Version>, PackageError> {
        let pkg_dir = self.cache_dir.join(name);
        if !pkg_dir.exists() {
            return Ok(Vec::new());
        }

        let mut versions = Vec::new();
        for entry in std::fs::read_dir(&pkg_dir)? {
            let entry = entry?;
            if entry.path().is_dir()
                && let Ok(v) = Version::parse(&entry.file_name().to_string_lossy())
            {
                versions.push(v);
            }
        }
        versions.sort();
        Ok(versions)
    }

    /// Get the cache directory path.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), PackageError> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        // Reject symlinks to prevent path-traversal from untrusted packages
        let metadata = std::fs::symlink_metadata(&src_path)?;
        if metadata.file_type().is_symlink() {
            return Err(PackageError::General(format!(
                "Symbolic link found in package: {}",
                src_path.display()
            )));
        }

        if metadata.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Get the user's home directory.
fn dirs_home() -> Result<PathBuf, PackageError> {
    // Use HOME env var on Unix, USERPROFILE on Windows
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .map_err(|_| PackageError::General("Could not determine home directory".to_string()))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| PackageError::General("Could not determine home directory".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_store_and_check() {
        let temp = TempDir::new().unwrap();
        let cache = PackageCache::with_dir(temp.path().join("cache")).unwrap();

        let src = temp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("main.wfl"), "display \"hello\"").unwrap();

        let version = Version::new(26, 1, Some(1));
        assert!(!cache.is_cached("my-pkg", &version));

        cache.store("my-pkg", &version, &src).unwrap();
        assert!(cache.is_cached("my-pkg", &version));
    }

    #[test]
    fn test_install_to_project() {
        let temp = TempDir::new().unwrap();
        let cache = PackageCache::with_dir(temp.path().join("cache")).unwrap();

        let src = temp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("main.wfl"), "display \"hello\"").unwrap();

        let version = Version::new(26, 1, Some(1));
        cache.store("my-pkg", &version, &src).unwrap();

        let project = temp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        cache
            .install_to_project("my-pkg", &version, &project)
            .unwrap();

        assert!(
            project
                .join("packages")
                .join("my-pkg")
                .join("main.wfl")
                .exists()
        );
    }

    #[test]
    fn test_list_versions() {
        let temp = TempDir::new().unwrap();
        let cache = PackageCache::with_dir(temp.path().join("cache")).unwrap();

        let src = temp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("main.wfl"), "// test").unwrap();

        cache
            .store("my-pkg", &Version::new(26, 1, Some(1)), &src)
            .unwrap();
        cache
            .store("my-pkg", &Version::new(26, 1, Some(2)), &src)
            .unwrap();

        let versions = cache.list_versions("my-pkg").unwrap();
        assert_eq!(versions.len(), 2);
    }
}
