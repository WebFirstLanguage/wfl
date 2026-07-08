//! Lockfile reading — the same shared parser path as the manifest.
//!
//! `project.lock` is `create map wflpkg:` envelope + one `create map locked:`
//! record per resolved package, parsed by the frozen data-literal grammar.

use crate::datalit::{self, Document, Record, Scalar, Value};
use crate::error::PackageError;
use crate::lockfile::{LockFile, LockedPackage};
use crate::manifest::parser::grammar_to_pkg_err;
use crate::manifest::schema;
use crate::manifest::version::Version;

/// Parse a `project.lock` file from its text content.
pub fn parse_lock_file(content: &str) -> Result<LockFile, PackageError> {
    let doc = datalit::parse(content.as_bytes()).map_err(|e| {
        // Reuse the manifest error mapping, but tag it as a lockfile problem.
        match grammar_to_pkg_err(content, e) {
            PackageError::ManifestParseError { line, message } => {
                PackageError::LockFileParseError { line, message }
            }
            other => other,
        }
    })?;
    lockfile_from_document(&doc)
}

fn lockfile_from_document(doc: &Document) -> Result<LockFile, PackageError> {
    schema::check_envelope(doc).map_err(as_lock_err)?;
    let mut packages = Vec::new();
    for rec in doc.records_of("locked") {
        packages.push(locked_from_record(rec)?);
    }
    Ok(LockFile { packages })
}

fn locked_from_record(rec: &Record) -> Result<LockedPackage, PackageError> {
    let name = req_string(rec, "name")?;
    let version_str = req_string(rec, "version")?;
    let version = Version::parse(&version_str).map_err(|_| PackageError::LockFileParseError {
        line: 0,
        message: format!("Invalid version \"{version_str}\" for `{name}`."),
    })?;
    let checksum = req_string(rec, "hash")?;
    let ast_hash = opt_string(rec, "ast_hash")?;
    let deps = opt_string_list(rec, "deps")?;
    Ok(LockedPackage {
        name,
        version,
        checksum,
        ast_hash,
        deps,
    })
}

fn as_lock_err(e: PackageError) -> PackageError {
    match e {
        PackageError::ManifestParseError { line, message } => {
            PackageError::LockFileParseError { line, message }
        }
        other => other,
    }
}

fn req_string(rec: &Record, key: &str) -> Result<String, PackageError> {
    match rec.get(key) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(PackageError::LockFileParseError {
            line: 0,
            message: format!("A `locked` record is missing its `{key}` string."),
        }),
    }
}

fn opt_string(rec: &Record, key: &str) -> Result<Option<String>, PackageError> {
    match rec.get(key) {
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(PackageError::LockFileParseError {
            line: 0,
            message: format!("`{key}` must be a string."),
        }),
        None => Ok(None),
    }
}

fn opt_string_list(rec: &Record, key: &str) -> Result<Vec<String>, PackageError> {
    match rec.get(key) {
        Some(Value::List(items)) => items
            .iter()
            .map(|s| match s {
                Scalar::String(s) => Ok(s.clone()),
                _ => Err(PackageError::LockFileParseError {
                    line: 0,
                    message: format!("Every element of `{key}` must be a string."),
                }),
            })
            .collect(),
        Some(_) => Err(PackageError::LockFileParseError {
            line: 0,
            message: format!("`{key}` must be a list of strings."),
        }),
        None => Ok(Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lock_file() {
        let content = "\
create map wflpkg:
    grammar is \"1.0.0\"
end map

create map locked:
    name is \"http-client\"
    version is \"26.1.3\"
    hash is \"sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08\"
    deps is [\"text-utils\"]
end map

create map locked:
    name is \"text-utils\"
    version is \"25.11.2\"
    hash is \"sha256:2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae\"
    deps is []
end map
";
        let lock = parse_lock_file(content).unwrap();
        assert_eq!(lock.packages.len(), 2);
        assert_eq!(lock.packages[0].name, "http-client");
        assert_eq!(lock.packages[0].version.to_string(), "26.1.3");
        assert!(lock.packages[0].checksum.starts_with("sha256:"));
        assert_eq!(lock.packages[0].deps, vec!["text-utils"]);
        assert_eq!(lock.packages[1].deps, Vec::<String>::new());
    }
}
