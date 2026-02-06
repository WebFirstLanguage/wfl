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

/// Re-export key types for convenience.
pub use error::PackageError;
pub use manifest::ProjectManifest;
pub use manifest::version::{Version, VersionConstraint};
