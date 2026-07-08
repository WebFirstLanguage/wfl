//! Lockfile writing — canonical `wfl fmt` byte form via the datalit grammar.

use crate::datalit::{Document, Entry, Record, Scalar, Value, fmt};
use crate::lockfile::{LockFile, LockedPackage};
use crate::manifest::schema;

/// Serialize a `LockFile` to the canonical `project.lock` byte form.
pub fn write_lock_file(lock: &LockFile) -> String {
    let mut records = vec![schema::envelope_record()];
    for pkg in &lock.packages {
        records.push(locked_record(pkg));
    }
    fmt::to_canonical(&Document { records })
}

fn locked_record(pkg: &LockedPackage) -> Record {
    let mut entries = Vec::new();
    push_str(&mut entries, "name", &pkg.name);
    push_str(&mut entries, "version", &pkg.version.to_string());
    push_str(&mut entries, "hash", &pkg.checksum);
    if let Some(ast_hash) = &pkg.ast_hash {
        push_str(&mut entries, "ast_hash", ast_hash);
    }
    entries.push(Entry {
        key: "deps".to_string(),
        value: Value::List(pkg.deps.iter().map(|d| Scalar::String(d.clone())).collect()),
        offset: 0,
    });
    Record {
        kind: "locked".to_string(),
        entries,
        offset: 0,
    }
}

fn push_str(entries: &mut Vec<Entry>, key: &str, value: &str) {
    entries.push(Entry {
        key: key.to_string(),
        value: Value::String(value.to_string()),
        offset: 0,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::version::Version;

    fn sample() -> LockFile {
        LockFile {
            packages: vec![
                LockedPackage {
                    name: "http-client".to_string(),
                    version: Version::new(26, 1, Some(3)),
                    checksum:
                        "sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
                            .to_string(),
                    ast_hash: None,
                    deps: vec!["text-utils".to_string()],
                },
                LockedPackage {
                    name: "text-utils".to_string(),
                    version: Version::new(25, 11, Some(2)),
                    checksum:
                        "sha256:2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae"
                            .to_string(),
                    ast_hash: None,
                    deps: vec![],
                },
            ],
        }
    }

    #[test]
    fn test_write_lock_file() {
        let output = write_lock_file(&sample());
        assert!(output.contains("create map wflpkg:"));
        assert!(output.contains("create map locked:"));
        assert!(output.contains("name is \"http-client\""));
        assert!(output.contains("version is \"26.1.3\""));
        assert!(output.contains("deps is [\"text-utils\"]"));
    }

    #[test]
    fn test_roundtrip() {
        let lock = sample();
        let output = write_lock_file(&lock);
        // Byte-deterministic.
        assert_eq!(output, write_lock_file(&lock));
        let parsed = crate::lockfile::parser::parse_lock_file(&output).unwrap();
        assert_eq!(parsed.packages.len(), 2);
        assert_eq!(parsed.packages[0].name, "http-client");
        assert_eq!(parsed.packages[0].version.to_string(), "26.1.3");
        assert_eq!(parsed.packages[0].deps, vec!["text-utils"]);
    }

    #[test]
    fn test_ast_hash_field_round_trips() {
        let mut lock = sample();
        lock.packages[0].ast_hash = Some("sha256:deadbeef".to_string());
        let output = write_lock_file(&lock);
        assert!(output.contains("ast_hash is \"sha256:deadbeef\""));
        let parsed = crate::lockfile::parser::parse_lock_file(&output).unwrap();
        assert_eq!(
            parsed.packages[0].ast_hash,
            Some("sha256:deadbeef".to_string())
        );
    }
}
