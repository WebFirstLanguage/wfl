use crate::manifest::ProjectManifest;

/// Serialize a `ProjectManifest` back to the `project.wfl` format.
pub fn write_manifest(manifest: &ProjectManifest) -> String {
    let mut lines = Vec::new();

    lines.push("// project.wfl".to_string());
    lines.push(String::new());

    lines.push(format!("name is {}", manifest.name));
    lines.push(format!("version is {}", manifest.version_string));
    lines.push(format!("description is {}", manifest.description));

    if manifest.authors.len() == 1 {
        lines.push(format!("author is {}", manifest.authors[0]));
    } else if manifest.authors.len() > 1 {
        lines.push(format!("authors are {}", manifest.authors.join(" and ")));
    }

    if let Some(license) = &manifest.license {
        lines.push(format!("license is {}", license));
    }

    if let Some(entry) = &manifest.entry {
        lines.push(String::new());
        lines.push(format!("entry is {}", entry));
    }

    if let Some(repository) = &manifest.repository {
        lines.push(format!("repository is {}", repository));
    }

    if let Some(registry) = &manifest.registry {
        lines.push(format!("registry is {}", registry));
    }

    // Dependencies
    let regular_deps: Vec<_> = manifest
        .dependencies
        .iter()
        .filter(|d| !d.dev_only)
        .collect();
    let dev_deps: Vec<_> = manifest
        .dependencies
        .iter()
        .filter(|d| d.dev_only)
        .collect();

    if !regular_deps.is_empty() || !dev_deps.is_empty() {
        lines.push(String::new());
    }

    for dep in &regular_deps {
        lines.push(format!("requires {} {}", dep.name, dep.constraint));
    }

    if !dev_deps.is_empty() && !regular_deps.is_empty() {
        lines.push(String::new());
    }

    for dep in &dev_deps {
        lines.push(format!(
            "requires {} {} for development",
            dep.name, dep.constraint
        ));
    }

    // Permissions
    if !manifest.permissions.is_empty() {
        lines.push(String::new());
        for perm in &manifest.permissions {
            lines.push(format!("needs {}", perm));
        }
    }

    lines.push(String::new());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::Dependency;
    use crate::manifest::version::{Version, VersionConstraint};

    #[test]
    fn test_write_minimal_manifest() {
        let manifest = ProjectManifest {
            name: "my-app".to_string(),
            version_string: "26.1.1".to_string(),
            description: "A test app".to_string(),
            ..Default::default()
        };
        let output = write_manifest(&manifest);
        assert!(output.contains("name is my-app"));
        assert!(output.contains("version is 26.1.1"));
        assert!(output.contains("description is A test app"));
    }

    #[test]
    fn test_roundtrip() {
        let manifest = ProjectManifest {
            name: "greeting".to_string(),
            version_string: "26.2.1".to_string(),
            description: "A web application".to_string(),
            authors: vec!["Alice Smith".to_string()],
            license: Some("MIT".to_string()),
            entry: Some("src/main.wfl".to_string()),
            dependencies: vec![
                Dependency {
                    name: "http-client".to_string(),
                    constraint: VersionConstraint::OrNewer(Version::new(26, 1, None)),
                    dev_only: false,
                },
                Dependency {
                    name: "test-runner".to_string(),
                    constraint: VersionConstraint::OrNewer(Version::new(26, 1, None)),
                    dev_only: true,
                },
            ],
            permissions: vec!["network-access".to_string()],
            ..Default::default()
        };

        let output = write_manifest(&manifest);

        // Re-parse
        let parsed = crate::manifest::parser::parse_manifest(&output).unwrap();
        assert_eq!(parsed.name, manifest.name);
        assert_eq!(parsed.version_string, manifest.version_string);
        assert_eq!(parsed.description, manifest.description);
        assert_eq!(parsed.authors, manifest.authors);
        assert_eq!(parsed.license, manifest.license);
        assert_eq!(parsed.entry, manifest.entry);
        assert_eq!(parsed.dependencies.len(), 2);
        assert_eq!(parsed.permissions, manifest.permissions);
    }
}
