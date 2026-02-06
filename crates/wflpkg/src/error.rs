use std::fmt;

/// Package manager error types with Elm-style first-person error messages.
#[derive(Debug)]
pub enum PackageError {
    /// Manifest file not found
    ManifestNotFound(String),
    /// Manifest parse error
    ManifestParseError { line: usize, message: String },
    /// Invalid package name
    InvalidPackageName(String),
    /// Invalid version string
    InvalidVersion(String),
    /// Invalid version constraint
    InvalidVersionConstraint(String),
    /// Package not found in registry
    PackageNotFound {
        name: String,
        suggestions: Vec<String>,
    },
    /// Version conflict between dependencies
    VersionConflict {
        package: String,
        constraint_a: String,
        source_a: String,
        constraint_b: String,
        source_b: String,
    },
    /// Registry unreachable
    RegistryUnreachable(String),
    /// Not authenticated
    NotAuthenticated,
    /// Lock file parse error
    LockFileParseError { line: usize, message: String },
    /// Checksum mismatch
    ChecksumMismatch {
        package: String,
        expected: String,
        actual: String,
    },
    /// IO error
    Io(std::io::Error),
    /// HTTP error
    Http(String),
    /// Security advisory found
    SecurityAdvisory {
        package: String,
        severity: String,
        description: String,
        fixed_in: Option<String>,
    },
    /// Permission required
    PermissionRequired {
        package: String,
        permissions: Vec<String>,
    },
    /// Workspace error
    WorkspaceError(String),
    /// General error
    General(String),
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageError::ManifestNotFound(dir) => {
                write!(
                    f,
                    "I could not find a project.wfl file in {}.\n\n\
                     Every WFL project needs a project.wfl file to manage dependencies.\n\
                     To create one interactively, run:\n\
                     \x20 wfl create project\n\n\
                     Or create one manually — here is a minimal example:\n\
                     \x20 name is my-project\n\
                     \x20 version is 26.1.1\n\
                     \x20 description is A new WFL project",
                    dir
                )
            }
            PackageError::ManifestParseError { line, message } => {
                write!(
                    f,
                    "I found a problem in your project.wfl file at line {}.\n\n{}",
                    line, message
                )
            }
            PackageError::InvalidPackageName(name) => {
                write!(
                    f,
                    "The package name \"{}\" is not valid.\n\n\
                     Package names must:\n\
                     \x20 - Start with a lowercase letter\n\
                     \x20 - Contain only lowercase letters, numbers, and hyphens\n\
                     \x20 - Be between 1 and 64 characters long",
                    name
                )
            }
            PackageError::InvalidVersion(version) => {
                write!(
                    f,
                    "The version \"{}\" is not a valid WFL version.\n\n\
                     WFL uses calendar-based versioning: YY.MM.BUILD\n\
                     For example: 26.1.1, 26.2.3, 25.12.15",
                    version
                )
            }
            PackageError::InvalidVersionConstraint(constraint) => {
                write!(
                    f,
                    "I could not understand the version constraint \"{}\".\n\n\
                     Valid version constraints:\n\
                     \x20 26.1 or newer\n\
                     \x20 26.1.3 exactly\n\
                     \x20 between 25.12 and 26.2\n\
                     \x20 any version\n\
                     \x20 above 25.6\n\
                     \x20 below 27",
                    constraint
                )
            }
            PackageError::PackageNotFound { name, suggestions } => {
                let mut msg = format!(
                    "I could not find a package called \"{}\" in the registry.",
                    name
                );
                if !suggestions.is_empty() {
                    msg.push_str("\n\nDid you mean one of these?");
                    for suggestion in suggestions {
                        msg.push_str(&format!("\n  - {}", suggestion));
                    }
                }
                write!(f, "{}", msg)
            }
            PackageError::VersionConflict {
                package,
                constraint_a,
                source_a,
                constraint_b,
                source_b,
            } => {
                write!(
                    f,
                    "I found a version conflict while resolving dependencies.\n\n\
                     The package \"{}\" requires {} {},\n\
                     but \"{}\" requires {} {}.\n\n\
                     These two constraints cannot be satisfied at the same time.\n\n\
                     You can:\n\
                     \x20 1. Update \"{}\" to a version that supports {} {}:\n\
                     \x20      wfl update {}\n\
                     \x20 2. Remove \"{}\" if you no longer need it:\n\
                     \x20      wfl remove {}",
                    source_a,
                    package,
                    constraint_a,
                    source_b,
                    package,
                    constraint_b,
                    source_b,
                    package,
                    constraint_a,
                    source_b,
                    source_b,
                    source_b,
                )
            }
            PackageError::RegistryUnreachable(url) => {
                write!(
                    f,
                    "I could not connect to the registry at {}.\n\n\
                     This might be a network issue, or the registry might be temporarily\n\
                     unavailable.\n\n\
                     Your project can still build using cached packages. To build offline:\n\
                     \x20 wfl build\n\n\
                     To retry connecting:\n\
                     \x20 wfl update",
                    url
                )
            }
            PackageError::NotAuthenticated => {
                write!(
                    f,
                    "I could not complete this action because you are not logged in.\n\n\
                     To log in to the registry, run:\n\
                     \x20 wfl login\n\n\
                     Then try again."
                )
            }
            PackageError::LockFileParseError { line, message } => {
                write!(
                    f,
                    "I found a problem in your project.lock file at line {}.\n\n\
                     {}\n\n\
                     You can regenerate the lock file by running:\n\
                     \x20 wfl update",
                    line, message
                )
            }
            PackageError::ChecksumMismatch {
                package,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "The checksum for \"{}\" does not match.\n\n\
                     Expected: {}\n\
                     Got:      {}\n\n\
                     This package may have been modified or corrupted.\n\
                     To re-download it, run:\n\
                     \x20 wfl update {}",
                    package, expected, actual, package
                )
            }
            PackageError::Io(err) => write!(f, "I/O error: {}", err),
            PackageError::Http(msg) => {
                write!(f, "HTTP error: {}", msg)
            }
            PackageError::SecurityAdvisory {
                package,
                severity,
                description,
                fixed_in,
            } => {
                let mut msg = format!("  {} — {}: {}\n", package, severity, description);
                if let Some(version) = fixed_in {
                    msg.push_str(&format!(
                        "    Fixed in {}. Run: wfl update {}",
                        version, package
                    ));
                }
                write!(f, "{}", msg)
            }
            PackageError::PermissionRequired {
                package,
                permissions,
            } => {
                let mut msg = format!(
                    "The package \"{}\" needs the following permissions:\n",
                    package
                );
                for perm in permissions {
                    let description = match perm.as_str() {
                        "file-access" => "Can read and write files on disk",
                        "network-access" => "Can make HTTP requests",
                        "system-access" => "Can execute system commands",
                        _ => "Unknown permission",
                    };
                    msg.push_str(&format!("  - {}: {}\n", perm, description));
                }
                msg.push_str("\nDo you want to add this package? (yes/no)");
                write!(f, "{}", msg)
            }
            PackageError::WorkspaceError(msg) => {
                write!(f, "Workspace error: {}", msg)
            }
            PackageError::General(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for PackageError {}

impl From<std::io::Error> for PackageError {
    fn from(err: std::io::Error) -> Self {
        PackageError::Io(err)
    }
}
