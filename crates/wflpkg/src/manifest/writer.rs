//! Manifest writing — served from the same library path as reading.
//!
//! `write_manifest` builds the canonical [`crate::datalit::Document`] for a
//! [`ProjectManifest`] and renders it with `wfl fmt`, so on-disk output is
//! always byte-deterministic and round-trips through the parser.

use crate::datalit::fmt;
use crate::manifest::{ProjectManifest, schema};

/// Serialize a `ProjectManifest` to the canonical `project.wfl` byte form.
pub fn write_manifest(manifest: &ProjectManifest) -> String {
    let doc = schema::manifest_to_document(manifest);
    fmt::to_canonical(&doc)
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
        assert!(output.contains("create map wflpkg:"));
        assert!(output.contains("grammar is \"1.0.0\""));
        assert!(output.contains("name is \"my-app\""));
        assert!(output.contains("version is \"26.1.1\""));
        assert!(output.contains("description is \"A test app\""));
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
                    scope: None,
                    constraint: VersionConstraint::OrNewer(Version::new(26, 1, None)),
                    dev_only: false,
                },
                Dependency {
                    name: "test-runner".to_string(),
                    scope: None,
                    constraint: VersionConstraint::OrNewer(Version::new(26, 1, None)),
                    dev_only: true,
                },
            ],
            permissions: vec!["network-access".to_string()],
            ..Default::default()
        };

        let output = write_manifest(&manifest);

        // Re-parse and confirm the fields survive the round trip.
        let parsed = crate::manifest::parser::parse_manifest(&output).unwrap();
        assert_eq!(parsed.name, manifest.name);
        assert_eq!(parsed.version_string, manifest.version_string);
        assert_eq!(parsed.description, manifest.description);
        assert_eq!(parsed.authors, manifest.authors);
        assert_eq!(parsed.license, manifest.license);
        assert_eq!(parsed.entry, manifest.entry);
        assert_eq!(parsed.dependencies.len(), 2);
        assert_eq!(parsed.permissions, manifest.permissions);
        let dev = parsed
            .dependencies
            .iter()
            .find(|d| d.name == "test-runner")
            .unwrap();
        assert!(dev.dev_only);
    }

    #[test]
    fn test_output_is_canonical_and_reparses() {
        let manifest = ProjectManifest {
            name: "greeting".to_string(),
            version_string: "26.2.1".to_string(),
            description: "hi".to_string(),
            ..Default::default()
        };
        let output = write_manifest(&manifest);
        // Byte-deterministic: writing again yields identical bytes.
        assert_eq!(output, write_manifest(&manifest));
        // And the output is itself a valid manifest.
        assert!(crate::manifest::parser::parse_manifest(&output).is_ok());
    }
}
