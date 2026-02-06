use std::path::Path;

use crate::error::PackageError;
use crate::manifest::version::VersionConstraint;
use crate::manifest::{Dependency, ProjectManifest};

/// Add a dependency to the project.
///
/// Parses CLI args like:
///   `wfl add http-client`
///   `wfl add http-client 26.1 or newer`
///   `wfl add http-client 26.1 or newer for development`
pub fn add_dependency(args: &[String], project_dir: &Path) -> Result<(), PackageError> {
    let manifest_path = project_dir.join("project.wfl");
    if !manifest_path.exists() {
        return Err(PackageError::ManifestNotFound(
            project_dir.display().to_string(),
        ));
    }

    let mut manifest = ProjectManifest::load(&manifest_path)?;

    // Parse the arguments
    let (dep, _) = parse_add_args(args)?;

    // TODO: Check permissions from registry metadata
    // TODO: Download and cache the package

    // Add to manifest
    manifest.add_dependency(dep.clone());
    manifest.save(&manifest_path)?;

    // TODO: Resolve and update lock file
    // TODO: Install package to packages/

    println!("Added {} {} to project.wfl", dep.name, dep.constraint);
    if dep.dev_only {
        println!("  (development dependency)");
    }

    Ok(())
}

/// Parse add command arguments into a Dependency.
fn parse_add_args(args: &[String]) -> Result<(Dependency, bool), PackageError> {
    if args.is_empty() {
        return Err(PackageError::General(
            "I need a package name to add.\n\n\
             Usage:\n\
             \x20 wfl add <package-name>\n\
             \x20 wfl add <package-name> <version-constraint>\n\
             \x20 wfl add <package-name> <version-constraint> for development"
                .to_string(),
        ));
    }

    let name = args[0].clone();

    // Check for "for development" at the end
    let (constraint_args, dev_only) = if args.len() >= 3
        && args[args.len() - 2] == "for"
        && args[args.len() - 1] == "development"
    {
        (&args[1..args.len() - 2], true)
    } else {
        (&args[1..], false)
    };

    // Parse version constraint
    let constraint = if constraint_args.is_empty() {
        // Default: any version (will pick latest)
        VersionConstraint::AnyVersion
    } else {
        let constraint_str = constraint_args.join(" ");
        VersionConstraint::parse(&constraint_str)?
    };

    Ok((
        Dependency {
            name,
            constraint,
            dev_only,
        },
        dev_only,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::version::Version;

    #[test]
    fn test_parse_add_simple() {
        let args = vec!["http-client".to_string()];
        let (dep, _) = parse_add_args(&args).unwrap();
        assert_eq!(dep.name, "http-client");
        assert_eq!(dep.constraint, VersionConstraint::AnyVersion);
        assert!(!dep.dev_only);
    }

    #[test]
    fn test_parse_add_with_constraint() {
        let args: Vec<String> = "http-client 26.1 or newer"
            .split_whitespace()
            .map(String::from)
            .collect();
        let (dep, _) = parse_add_args(&args).unwrap();
        assert_eq!(dep.name, "http-client");
        assert_eq!(
            dep.constraint,
            VersionConstraint::OrNewer(Version::new(26, 1, None))
        );
    }

    #[test]
    fn test_parse_add_dev_dependency() {
        let args: Vec<String> = "test-runner 26.1 or newer for development"
            .split_whitespace()
            .map(String::from)
            .collect();
        let (dep, _) = parse_add_args(&args).unwrap();
        assert_eq!(dep.name, "test-runner");
        assert!(dep.dev_only);
    }

    #[test]
    fn test_parse_add_empty() {
        let args: Vec<String> = vec![];
        assert!(parse_add_args(&args).is_err());
    }
}
