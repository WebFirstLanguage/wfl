use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::error::PackageError;
use crate::manifest::parser::validate_package_name;
use crate::manifest::version::Version;

fn verify_directory(path: &Path, description: &str) -> Result<PathBuf, PackageError> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(PackageError::General(format!(
            "Refusing to use {} \"{}\": symbolic links are not allowed.",
            description,
            path.display()
        )));
    }
    if !metadata.is_dir() {
        return Err(PackageError::General(format!(
            "Refusing to use {} \"{}\": it is not a directory.",
            description,
            path.display()
        )));
    }
    path.canonicalize().map_err(|error| {
        PackageError::General(format!(
            "Could not verify {} \"{}\": {}",
            description,
            path.display(),
            error
        ))
    })
}

fn existing_child_directory(
    parent: &Path,
    child: &Path,
    description: &str,
) -> Result<Option<PathBuf>, PackageError> {
    let metadata = match std::fs::symlink_metadata(child) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() {
        return Err(PackageError::General(format!(
            "Refusing to use {} \"{}\": symbolic links are not allowed.",
            description,
            child.display()
        )));
    }
    if !metadata.is_dir() {
        return Err(PackageError::General(format!(
            "Refusing to use {} \"{}\": it is not a directory.",
            description,
            child.display()
        )));
    }

    let canonical_child = child.canonicalize().map_err(|error| {
        PackageError::General(format!(
            "Could not verify {} \"{}\": {}",
            description,
            child.display(),
            error
        ))
    })?;
    if canonical_child.parent() != Some(parent) {
        return Err(PackageError::General(format!(
            "Refusing to use {} \"{}\": it escapes its expected parent directory.",
            description,
            child.display()
        )));
    }

    Ok(Some(canonical_child))
}

fn ensure_child_directory(
    parent: &Path,
    child: &Path,
    description: &str,
) -> Result<PathBuf, PackageError> {
    if let Some(existing) = existing_child_directory(parent, child, description)? {
        return Ok(existing);
    }

    match std::fs::create_dir(child) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    existing_child_directory(parent, child, description)?.ok_or_else(|| {
        PackageError::General(format!(
            "Could not create {} \"{}\".",
            description,
            child.display()
        ))
    })
}

/// Manages the global package cache at `~/.wfl/packages/`.
pub struct PackageCache {
    cache_dir: PathBuf,
}

impl PackageCache {
    /// Create a new cache manager using the default global cache directory.
    pub fn new() -> Result<Self, PackageError> {
        let home = dirs_home()?;
        let cache_dir = home.join(".wfl").join("packages");
        Self::with_dir(cache_dir)
    }

    /// Create a cache manager with a custom directory (for testing).
    pub fn with_dir(cache_dir: PathBuf) -> Result<Self, PackageError> {
        match std::fs::symlink_metadata(&cache_dir) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(PackageError::General(format!(
                    "Refusing to use package cache \"{}\": symbolic links are not allowed.",
                    cache_dir.display()
                )));
            }
            Ok(metadata) if !metadata.is_dir() => {
                return Err(PackageError::General(format!(
                    "Refusing to use package cache \"{}\": it is not a directory.",
                    cache_dir.display()
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {
                std::fs::create_dir_all(&cache_dir)?;
            }
            Err(error) => return Err(error.into()),
        }

        Ok(Self {
            cache_dir: verify_directory(&cache_dir, "package cache")?,
        })
    }

    /// Get the path for a specific package version in the cache.
    pub fn package_path(&self, name: &str, version: &Version) -> PathBuf {
        if validate_package_name(name).is_err() {
            return self
                .cache_dir
                .join(".invalid-package-name")
                .join(version.to_string());
        }
        self.cache_dir.join(name).join(version.to_string())
    }

    /// Check if a package version is cached.
    pub fn is_cached(&self, name: &str, version: &Version) -> bool {
        if validate_package_name(name).is_err() {
            return false;
        }
        self.cached_package_directory(name, version)
            .ok()
            .flatten()
            .is_some()
    }

    /// Store a package in the cache by copying from a source directory.
    pub fn store(&self, name: &str, version: &Version, source: &Path) -> Result<(), PackageError> {
        validate_package_name(name)?;
        let source = verify_directory(source, "package source")?;
        let cache_root = self.verified_cache_root()?;
        let package_root = ensure_child_directory(
            &cache_root,
            &cache_root.join(name),
            "package cache directory",
        )?;
        let dest = package_root.join(version.to_string());
        if let Some(existing) =
            existing_child_directory(&package_root, &dest, "cached package version")?
        {
            if existing == source {
                return Err(PackageError::General(
                    "Refusing to replace a cached package with itself.".to_string(),
                ));
            }
            std::fs::remove_dir_all(existing)?;
        }

        copy_dir_recursive(&source, &dest)?;
        Ok(())
    }

    /// Install a cached package into the project's `packages/` directory.
    pub fn install_to_project(
        &self,
        name: &str,
        version: &Version,
        project_dir: &Path,
    ) -> Result<(), PackageError> {
        validate_package_name(name)?;
        let cache_path = self
            .cached_package_directory(name, version)?
            .ok_or_else(|| {
                PackageError::General(format!("Package {} {} is not in the cache.", name, version))
            })?;

        let project_root = project_dir.canonicalize().map_err(|error| {
            PackageError::General(format!(
                "Could not verify project directory \"{}\": {}",
                project_dir.display(),
                error
            ))
        })?;
        let packages_path = project_root.join("packages");
        let packages_root =
            ensure_child_directory(&project_root, &packages_path, "project packages directory")?;
        let dest = packages_root.join(name);
        if let Some(existing) =
            existing_child_directory(&packages_root, &dest, "installed package directory")?
        {
            std::fs::remove_dir_all(existing)?;
        }
        copy_dir_recursive(&cache_path, &dest)?;
        Ok(())
    }

    /// List all cached versions of a package.
    pub fn list_versions(&self, name: &str) -> Result<Vec<Version>, PackageError> {
        validate_package_name(name)?;
        let cache_root = self.verified_cache_root()?;
        let pkg_dir = match existing_child_directory(
            &cache_root,
            &cache_root.join(name),
            "package cache directory",
        )? {
            Some(path) => path,
            None => return Ok(Vec::new()),
        };

        let mut versions = Vec::new();
        for entry in std::fs::read_dir(&pkg_dir)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                return Err(PackageError::General(format!(
                    "Refusing to inspect cached package version \"{}\": symbolic links are not allowed.",
                    entry.path().display()
                )));
            }
            if file_type.is_dir()
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

    fn verified_cache_root(&self) -> Result<PathBuf, PackageError> {
        let canonical = verify_directory(&self.cache_dir, "package cache")?;
        if canonical != self.cache_dir {
            return Err(PackageError::General(format!(
                "Refusing to use package cache \"{}\": its filesystem location changed.",
                self.cache_dir.display()
            )));
        }
        Ok(canonical)
    }

    fn cached_package_directory(
        &self,
        name: &str,
        version: &Version,
    ) -> Result<Option<PathBuf>, PackageError> {
        let cache_root = self.verified_cache_root()?;
        let package_root = match existing_child_directory(
            &cache_root,
            &cache_root.join(name),
            "package cache directory",
        )? {
            Some(path) => path,
            None => return Ok(None),
        };
        existing_child_directory(
            &package_root,
            &package_root.join(version.to_string()),
            "cached package version",
        )
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
        } else if metadata.is_file() {
            std::fs::copy(&src_path, &dst_path)?;
        } else {
            return Err(PackageError::General(format!(
                "Unsupported filesystem entry found in package: {}",
                src_path.display()
            )));
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
