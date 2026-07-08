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
    /// Integrity digest of the resolved package (`sha256:…`). This is the
    /// "the bytes I received are the bytes the world witnessed" anchor.
    pub checksum: String,
    /// Normalized-AST structure hash (Decision 2). The **schema field is
    /// committed from day one** — the regret-minimizing move — even though the
    /// AST normalizer that fills it is a tracked follow-up. `None` until then.
    pub ast_hash: Option<String>,
    /// Names of this package's direct dependencies. Their resolved versions are
    /// the top-level `locked` records elsewhere in the file.
    pub deps: Vec<String>,
}

impl LockFile {
    /// Load a lock file from a path.
    pub fn load(path: &std::path::Path) -> Result<Self, crate::error::PackageError> {
        let content = std::fs::read_to_string(path)?;
        parser::parse_lock_file(&content)
    }

    /// Save the lock file to a path, in the canonical `wfl fmt` byte form.
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
