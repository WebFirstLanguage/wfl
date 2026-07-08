//! Manifest reading — the single shared parser path.
//!
//! `project.wfl` is parsed by the frozen data-literal grammar
//! ([`crate::datalit`]) over WFL's one shared lexer, then mapped onto a
//! [`ProjectManifest`] by the schema layer. The old hand-rolled prose parser is
//! gone: there is exactly one parser, byte-identical to the compiler's
//! (ADR-001, condition 5).

use crate::datalit::{self, GrammarError};
use crate::error::PackageError;
use crate::manifest::{ProjectManifest, schema};

/// Parse a `project.wfl` manifest from its text content.
pub fn parse_manifest(content: &str) -> Result<ProjectManifest, PackageError> {
    let doc = datalit::parse(content.as_bytes()).map_err(|e| grammar_to_pkg_err(content, e))?;
    schema::manifest_from_document(&doc)
}

/// Map a frozen-grammar [`GrammarError`] to a user-facing [`PackageError`],
/// converting the byte offset into a 1-based line number and surfacing the
/// stable `MG-*` code.
pub(crate) fn grammar_to_pkg_err(content: &str, e: GrammarError) -> PackageError {
    PackageError::ManifestParseError {
        line: line_of(content, e.offset),
        message: format!("[{}] {}", e.code, e.message),
    }
}

/// 1-based line number of a byte offset.
pub(crate) fn line_of(content: &str, offset: usize) -> usize {
    let capped = offset.min(content.len());
    1 + content.as_bytes()[..capped]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = "\
create map wflpkg:
    grammar is \"1.0.0\"
end map

create map package:
    name is \"my-app\"
    version is \"26.1.1\"
    description is \"A test application\"
end map
";

    #[test]
    fn test_parse_minimal_manifest() {
        let manifest = parse_manifest(MINIMAL).unwrap();
        assert_eq!(manifest.name, "my-app");
        assert_eq!(manifest.version_string, "26.1.1");
        assert_eq!(manifest.description, "A test application");
    }

    #[test]
    fn test_parse_full_manifest() {
        let content = "\
create map wflpkg:
    grammar is \"1.0.0\"
end map

create map package:
    name is \"greeting\"
    version is \"26.2.1\"
    description is \"A web application that greets visitors\"
    authors is [\"Alice Smith\"]
    license is \"MIT\"
    entry is \"src/main.wfl\"
end map

create map dependency:
    name is \"http-client\"
    version is \"26.1 or newer\"
end map

create map dependency:
    name is \"json-parser\"
    version is \"25.12 or newer\"
end map

create map dependency:
    name is \"text-utils\"
    version is \"any version\"
end map

create map dependency:
    name is \"test-runner\"
    version is \"26.1 or newer\"
    scope is \"dev\"
end map
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
    fn test_parse_multiple_authors_and_permissions() {
        let content = "\
create map wflpkg:
    grammar is \"1.0.0\"
end map

create map package:
    name is \"my-app\"
    version is \"26.1.1\"
    description is \"Test\"
    authors is [\"Alice Smith\", \"Bob Jones\"]
    permissions is [\"file-access\", \"network-access\"]
end map
";
        let manifest = parse_manifest(content).unwrap();
        assert_eq!(manifest.authors, vec!["Alice Smith", "Bob Jones"]);
        assert_eq!(manifest.permissions, vec!["file-access", "network-access"]);
    }

    #[test]
    fn test_missing_name_error() {
        let content = "\
create map wflpkg:
    grammar is \"1.0.0\"
end map

create map package:
    version is \"26.1.1\"
end map
";
        let err = parse_manifest(content).unwrap_err();
        assert!(err.to_string().contains("missing a required `name`"));
    }

    #[test]
    fn test_missing_envelope_error() {
        let content =
            "create map package:\n    name is \"x\"\n    version is \"26.1.1\"\nend map\n";
        let err = parse_manifest(content).unwrap_err();
        assert!(err.to_string().contains("must begin with a version block"));
    }

    #[test]
    fn test_line_of() {
        let text = "a\nbb\nccc";
        assert_eq!(line_of(text, 0), 1);
        assert_eq!(line_of(text, 2), 2);
        assert_eq!(line_of(text, 5), 3);
    }
}
