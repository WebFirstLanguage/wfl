pub mod archive;
pub mod cache;
pub mod checksum;
pub mod commands;
pub mod error;
pub mod lockfile;
pub mod manifest;
pub mod permissions;
pub mod registry;
pub mod resolver;
pub mod workspace;

/// Names excluded from both archive creation and checksum computation.
/// Kept in one place so the two always stay in sync.
pub const EXCLUDED_NAMES: &[&str] = &[
    "packages",
    ".git",
    "node_modules",
    "target",
    ".gitignore",
    "project.lock",
];

/// File extensions (without leading dot) excluded from both archive creation
/// and checksum computation.
pub const EXCLUDED_EXTENSIONS: &[&str] = &["wflpkg"];

/// Check whether a file or directory name should be excluded from archive/checksum.
pub fn is_excluded(name: &str) -> bool {
    if EXCLUDED_NAMES.contains(&name) {
        return true;
    }
    std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXCLUDED_EXTENSIONS.contains(&e))
}

/// Re-export key types for convenience.
pub use error::PackageError;
pub use manifest::ProjectManifest;
pub use manifest::version::{Version, VersionConstraint};
