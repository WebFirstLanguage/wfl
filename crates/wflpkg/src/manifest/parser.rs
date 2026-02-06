use crate::error::PackageError;
use crate::manifest::version::VersionConstraint;
use crate::manifest::{Dependency, ProjectManifest};

/// Parse a `project.wfl` manifest from its text content.
pub fn parse_manifest(content: &str) -> Result<ProjectManifest, PackageError> {
    let mut manifest = ProjectManifest::default();
    let mut line_num = 0;

    for raw_line in content.lines() {
        line_num += 1;
        let line = raw_line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        // Try to parse "requires ..." dependency lines
        if let Some(rest) = line.strip_prefix("requires ") {
            let dep = parse_dependency(rest, line_num)?;
            manifest.dependencies.push(dep);
            continue;
        }

        // Try to parse "needs ..." permission lines
        if let Some(rest) = line.strip_prefix("needs ") {
            let perm = rest.trim().to_string();
            manifest.permissions.push(perm);
            continue;
        }

        // Try to parse "authors are ..." (multi-author)
        if let Some(authors_str) = line.strip_prefix("authors are ") {
            let authors: Vec<String> = authors_str
                .split(" and ")
                .flat_map(|part| part.split(','))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            manifest.authors = authors;
            continue;
        }

        // Parse "key is value" lines
        if let Some(pos) = line.find(" is ") {
            let key = line[..pos].trim();
            let value = line[pos + 4..].trim();

            match key {
                "name" => manifest.name = value.to_string(),
                "version" => manifest.version_string = value.to_string(),
                "description" => manifest.description = value.to_string(),
                "author" => manifest.authors = vec![value.to_string()],
                "license" => manifest.license = Some(value.to_string()),
                "entry" => manifest.entry = Some(value.to_string()),
                "repository" => manifest.repository = Some(value.to_string()),
                "registry" => manifest.registry = Some(value.to_string()),
                _ => {
                    return Err(PackageError::ManifestParseError {
                        line: line_num,
                        message: format!(
                            "I do not recognize the field \"{}\".\n\
                             Valid fields are: name, version, description, author, authors, \
                             license, entry, repository, registry, requires, needs",
                            key
                        ),
                    });
                }
            }
        } else {
            return Err(PackageError::ManifestParseError {
                line: line_num,
                message: format!(
                    "I could not understand this line:\n  {}\n\n\
                     Each line in project.wfl should use the format:\n\
                     \x20 field is value\n\
                     \x20 requires package-name version-constraint\n\
                     \x20 needs permission-name",
                    line
                ),
            });
        }
    }

    // Validate required fields
    if manifest.name.is_empty() {
        return Err(PackageError::ManifestParseError {
            line: 0,
            message: "The project.wfl file is missing a required field: name\n\
                     Add a line like: name is my-project"
                .to_string(),
        });
    }

    if manifest.version_string.is_empty() {
        return Err(PackageError::ManifestParseError {
            line: 0,
            message: "The project.wfl file is missing a required field: version\n\
                     Add a line like: version is 26.1.1"
                .to_string(),
        });
    }

    if manifest.description.is_empty() {
        return Err(PackageError::ManifestParseError {
            line: 0,
            message: "The project.wfl file is missing a required field: description\n\
                     Add a line like: description is A brief description of your project"
                .to_string(),
        });
    }

    // Validate package name
    validate_package_name(&manifest.name)?;

    Ok(manifest)
}

/// Parse a dependency line after the "requires " prefix.
/// Examples:
///   "http-client 26.1 or newer"
///   "test-runner 26.1 or newer for development"
///   "text-utils any version"
fn parse_dependency(s: &str, line_num: usize) -> Result<Dependency, PackageError> {
    let s = s.trim();

    // Check for "for development" suffix
    let (rest, dev_only) = if let Some(stripped) = s.strip_suffix(" for development") {
        (stripped, true)
    } else {
        (s, false)
    };

    // Split into package name and version constraint
    // The name is the first word, everything after is the constraint
    let first_space = rest
        .find(' ')
        .ok_or_else(|| PackageError::ManifestParseError {
            line: line_num,
            message: format!(
                "I expected a version constraint after the package name in:\n  requires {}\n\n\
             For example:\n\
             \x20 requires {} any version\n\
             \x20 requires {} 26.1 or newer",
                s, rest, rest
            ),
        })?;

    let name = rest[..first_space].trim().to_string();
    let constraint_str = rest[first_space + 1..].trim();

    validate_package_name(&name)?;
    let constraint = VersionConstraint::parse(constraint_str)?;

    Ok(Dependency {
        name,
        constraint,
        dev_only,
    })
}

/// Validate a package name.
fn validate_package_name(name: &str) -> Result<(), PackageError> {
    if name.is_empty() || name.len() > 64 {
        return Err(PackageError::InvalidPackageName(name.to_string()));
    }

    let first = name.chars().next().unwrap();
    if !first.is_ascii_lowercase() {
        return Err(PackageError::InvalidPackageName(name.to_string()));
    }

    for c in name.chars() {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' {
            return Err(PackageError::InvalidPackageName(name.to_string()));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_manifest() {
        let content = "\
// project.wfl
name is my-app
version is 26.1.1
description is A test application
";
        let manifest = parse_manifest(content).unwrap();
        assert_eq!(manifest.name, "my-app");
        assert_eq!(manifest.version_string, "26.1.1");
        assert_eq!(manifest.description, "A test application");
    }

    #[test]
    fn test_parse_full_manifest() {
        let content = "\
// project.wfl - Package manifest

name is greeting
version is 26.2.1
description is A web application that greets visitors
author is Alice Smith
license is MIT

entry is src/main.wfl

requires http-client 26.1 or newer
requires json-parser 25.12 or newer
requires text-utils any version

requires test-runner 26.1 or newer for development
";
        let manifest = parse_manifest(content).unwrap();
        assert_eq!(manifest.name, "greeting");
        assert_eq!(manifest.authors, vec!["Alice Smith"]);
        assert_eq!(manifest.license, Some("MIT".to_string()));
        assert_eq!(manifest.entry, Some("src/main.wfl".to_string()));
        assert_eq!(manifest.dependencies.len(), 4);
        assert!(!manifest.dependencies[0].dev_only);
        assert!(manifest.dependencies[3].dev_only);
        assert_eq!(manifest.dependencies[3].name, "test-runner");
    }

    #[test]
    fn test_parse_multiple_authors() {
        let content = "\
name is my-app
version is 26.1.1
description is Test
authors are Alice Smith and Bob Jones
";
        let manifest = parse_manifest(content).unwrap();
        assert_eq!(manifest.authors, vec!["Alice Smith", "Bob Jones"]);
    }

    #[test]
    fn test_parse_permissions() {
        let content = "\
name is my-app
version is 26.1.1
description is Test
needs file-access
needs network-access
";
        let manifest = parse_manifest(content).unwrap();
        assert_eq!(manifest.permissions, vec!["file-access", "network-access"]);
    }

    #[test]
    fn test_missing_name_error() {
        let content = "version is 26.1.1\ndescription is Test";
        let err = parse_manifest(content).unwrap_err();
        assert!(err.to_string().contains("missing a required field: name"));
    }

    #[test]
    fn test_invalid_package_name() {
        assert!(validate_package_name("my-app").is_ok());
        assert!(validate_package_name("http-client").is_ok());
        assert!(validate_package_name("a123").is_ok());
        assert!(validate_package_name("MyApp").is_err());
        assert!(validate_package_name("123abc").is_err());
        assert!(validate_package_name("").is_err());
    }
}
