pub mod parser;
pub mod writer;

use crate::manifest::version::Version;

/// A lock file representing exact resolved dependency versions.
#[derive(Debug, Clone, Default)]
pub struct LockFile {
    pub packages: Vec<LockedPackage>,
}

/// A single locked package entry.
#[derive(Debug, Clone)]
pub struct LockedPackage {
    pub name: String,
    pub version: Version,
    pub checksum: String,
    pub dependencies: Vec<LockedDependency>,
}

/// A dependency reference within a locked package.
#[derive(Debug, Clone)]
pub struct LockedDependency {
    pub name: String,
    pub version: Version,
}

impl LockFile {
    /// Load a lock file from a path.
    pub fn load(path: &std::path::Path) -> Result<Self, crate::error::PackageError> {
        let content = std::fs::read_to_string(path)?;
        parser::parse_lock_file(&content)
    }

    /// Save the lock file to a path.
    pub fn save(&self, path: &std::path::Path) -> Result<(), crate::error::PackageError> {
        let content = writer::write_lock_file(self);
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Find a locked package by name.
    pub fn find_package(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.iter().find(|p| p.name == name)
    }
}
