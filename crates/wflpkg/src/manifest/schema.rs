//! The **schema layer** over the frozen data-literal grammar (grammar §3).
//!
//! The grammar defines *syntax* (which records/values are well-formed); this
//! layer defines *which record kinds exist and how they map to wflpkg's data
//! model* — `wflpkg` (the version envelope), `package`, and `dependency`. The
//! relationships between records live here, never as in-document references.

use crate::datalit::version::SemVer;
use crate::datalit::{Document, Entry, Record, Scalar, Value};
use crate::error::PackageError;
use crate::manifest::version::VersionConstraint;
use crate::manifest::{Dependency, ProjectManifest};

/// Grammar-envelope major version this build understands.
const SUPPORTED_GRAMMAR_MAJOR: u16 = 1;

fn schema_err(message: impl Into<String>) -> PackageError {
    PackageError::ManifestParseError {
        line: 0,
        message: message.into(),
    }
}

/// Validate the `create map wflpkg:` version envelope required as the first
/// record of every wflpkg file (grammar §9). Returns the declared grammar
/// version string.
pub fn check_envelope(doc: &Document) -> Result<String, PackageError> {
    let first = doc
        .records
        .first()
        .ok_or_else(|| schema_err("This file is empty."))?;
    if first.kind != "wflpkg" {
        return Err(schema_err(
            "A wflpkg file must begin with a version block:\n\
             \x20 create map wflpkg:\n\
             \x20     grammar is \"1.0.0\"\n\
             \x20 end map",
        ));
    }
    let grammar = first
        .get_string("grammar")
        .ok_or_else(|| schema_err("The `wflpkg` block needs a `grammar` version string."))?;
    let version = SemVer::parse_exact(grammar).map_err(|e| {
        schema_err(format!(
            "The grammar version \"{grammar}\" is invalid: {}.",
            e.message
        ))
    })?;
    if version.major != SUPPORTED_GRAMMAR_MAJOR {
        return Err(schema_err(format!(
            "This file declares grammar {grammar}, but this build understands grammar \
             {SUPPORTED_GRAMMAR_MAJOR}.x only."
        )));
    }
    Ok(grammar.to_string())
}

/// Map a parsed [`Document`] onto a [`ProjectManifest`].
pub fn manifest_from_document(doc: &Document) -> Result<ProjectManifest, PackageError> {
    check_envelope(doc)?;

    let package = match doc.records_of("package").count() {
        1 => doc.single("package").unwrap(),
        0 => {
            return Err(schema_err(
                "This manifest has no `create map package:` block.",
            ));
        }
        n => {
            return Err(schema_err(format!(
                "A manifest must have exactly one `package` block; found {n}."
            )));
        }
    };

    let mut manifest = ProjectManifest {
        name: req_string(package, "name")?,
        scope: opt_string(package, "scope")?,
        version_string: req_string(package, "version")?,
        description: opt_string(package, "description")?.unwrap_or_default(),
        authors: opt_string_list(package, "authors")?,
        license: opt_string(package, "license")?,
        keywords: opt_string_list(package, "keywords")?,
        entry: opt_string(package, "entry")?,
        repository: opt_string(package, "repository")?,
        registry: opt_string(package, "registry")?,
        notes: opt_string(package, "notes")?,
        permissions: opt_string_list(package, "permissions")?,
        dependencies: Vec::new(),
    };

    for dep in doc.records_of("dependency") {
        manifest.dependencies.push(dependency_from_record(dep)?);
    }

    Ok(manifest)
}

fn dependency_from_record(rec: &Record) -> Result<Dependency, PackageError> {
    let name = req_string(rec, "name")?;
    let constraint_str = req_string(rec, "version")?;
    let constraint = VersionConstraint::parse(&constraint_str)?;
    let scope = opt_string(rec, "scope")?;
    // Convention: `scope is "dev"` marks a development-only dependency.
    let dev_only = scope.as_deref() == Some("dev");
    Ok(Dependency {
        name,
        scope: if dev_only { None } else { scope },
        constraint,
        dev_only,
    })
}

/// Build the canonical [`Document`] for a [`ProjectManifest`], in schema
/// key order. The version envelope is always first.
pub fn manifest_to_document(m: &ProjectManifest) -> Document {
    let mut records = vec![envelope_record()];

    let mut pkg = Vec::new();
    push_str(&mut pkg, "name", &m.name);
    if let Some(scope) = &m.scope {
        push_str(&mut pkg, "scope", scope);
    }
    push_str(&mut pkg, "version", &m.version_string);
    if !m.description.is_empty() {
        push_str(&mut pkg, "description", &m.description);
    }
    if !m.authors.is_empty() {
        push_list(&mut pkg, "authors", &m.authors);
    }
    if let Some(license) = &m.license {
        push_str(&mut pkg, "license", license);
    }
    if !m.keywords.is_empty() {
        push_list(&mut pkg, "keywords", &m.keywords);
    }
    if let Some(entry) = &m.entry {
        push_str(&mut pkg, "entry", entry);
    }
    if let Some(repo) = &m.repository {
        push_str(&mut pkg, "repository", repo);
    }
    if let Some(reg) = &m.registry {
        push_str(&mut pkg, "registry", reg);
    }
    if !m.permissions.is_empty() {
        push_list(&mut pkg, "permissions", &m.permissions);
    }
    if let Some(notes) = &m.notes {
        push_str(&mut pkg, "notes", notes);
    }
    records.push(record("package", pkg));

    for dep in &m.dependencies {
        let mut d = Vec::new();
        push_str(&mut d, "name", &dep.name);
        push_str(&mut d, "version", &dep.constraint.to_string());
        let scope = if dep.dev_only {
            Some("dev".to_string())
        } else {
            dep.scope.clone()
        };
        if let Some(scope) = scope {
            push_str(&mut d, "scope", &scope);
        }
        records.push(record("dependency", d));
    }

    Document { records }
}

/// The `create map wflpkg: grammar is "1.0.0" end map` version envelope.
pub fn envelope_record() -> Record {
    let mut entries = Vec::new();
    push_str(&mut entries, "grammar", crate::datalit::GRAMMAR_VERSION);
    record("wflpkg", entries)
}

// ---- small builders ----

fn record(kind: &str, entries: Vec<Entry>) -> Record {
    Record {
        kind: kind.to_string(),
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

fn push_list(entries: &mut Vec<Entry>, key: &str, items: &[String]) {
    entries.push(Entry {
        key: key.to_string(),
        value: Value::List(items.iter().map(|s| Scalar::String(s.clone())).collect()),
        offset: 0,
    });
}

// ---- typed field extraction ----

fn req_string(rec: &Record, key: &str) -> Result<String, PackageError> {
    match rec.get(key) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(schema_err(format!(
            "In the `{}` block, `{key}` must be a quoted string.",
            rec.kind
        ))),
        None => Err(schema_err(format!(
            "The `{}` block is missing a required `{key}` field.",
            rec.kind
        ))),
    }
}

fn opt_string(rec: &Record, key: &str) -> Result<Option<String>, PackageError> {
    match rec.get(key) {
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(schema_err(format!(
            "In the `{}` block, `{key}` must be a quoted string.",
            rec.kind
        ))),
        None => Ok(None),
    }
}

fn opt_string_list(rec: &Record, key: &str) -> Result<Vec<String>, PackageError> {
    match rec.get(key) {
        Some(Value::List(items)) => items
            .iter()
            .map(|s| match s {
                Scalar::String(s) => Ok(s.clone()),
                _ => Err(schema_err(format!(
                    "In the `{}` block, every element of `{key}` must be a quoted string.",
                    rec.kind
                ))),
            })
            .collect(),
        Some(_) => Err(schema_err(format!(
            "In the `{}` block, `{key}` must be a list of quoted strings.",
            rec.kind
        ))),
        None => Ok(Vec::new()),
    }
}
