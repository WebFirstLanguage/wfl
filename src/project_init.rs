//! Bundled application scaffolding, created without replacing project files.

use std::fs;
use std::io::{self, Write};
use std::path::Path;
use tempfile::NamedTempFile;

const CONFIG: &str = "\
# Project settings created by wfl init.
# Reference: https://github.com/WebFirstLanguage/wfl/blob/main/Docs/reference/configuration-reference.md
timeout_seconds = 60
logging_enabled = false
execution_logging = false
debug_report_enabled = false
";

const TEMPLATES: [(&str, &str); 3] = [
    (".wflcfg", CONFIG),
    ("AGENTS.md", include_str!("project_init/AGENTS.md")),
    ("CLAUDE.md", include_str!("project_init/CLAUDE.md")),
];

#[derive(Default)]
pub struct InitReport {
    pub created: Vec<&'static str>,
    pub skipped: Vec<&'static str>,
}

fn path_error(path: &Path, error: io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{}: {error}", path.display()))
}

/// Inspect the entry itself: following a link here could hide a collision or
/// make a dangling link look like an available destination.
fn existing_regular_file(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(true),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "{} is not a regular file; resolve this entry before running wfl init",
                path.display()
            ),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(path_error(path, error)),
    }
}

/// Fill missing files in an existing directory. Preflight every name before
/// writing, then stage complete contents beside their destinations. Publishing
/// never overwrites an existing entry, including one created after preflight.
/// A late I/O failure may leave earlier completed files; rerunning is safe.
pub fn initialize(directory: &Path) -> io::Result<InitReport> {
    let mut report = InitReport::default();
    let mut missing = Vec::new();
    for (name, contents) in TEMPLATES {
        if existing_regular_file(&directory.join(name))? {
            report.skipped.push(name);
        } else {
            missing.push((name, contents));
        }
    }

    let mut staged = Vec::new();
    for (name, contents) in missing {
        let path = directory.join(name);
        let mut temporary =
            NamedTempFile::new_in(directory).map_err(|error| path_error(&path, error))?;
        temporary
            .write_all(contents.as_bytes())
            .map_err(|error| path_error(&path, error))?;
        staged.push((name, temporary));
    }

    for (name, temporary) in staged {
        let path = directory.join(name);
        match temporary.persist_noclobber(&path) {
            Ok(_) => report.created.push(name),
            Err(error) => {
                // Another initializer or editor may have created this file
                // since preflight. Preserve it under the same regular-file rule.
                if error.error.kind() == io::ErrorKind::AlreadyExists
                    && existing_regular_file(&path)?
                {
                    report.skipped.push(name);
                } else {
                    return Err(path_error(&path, error.error));
                }
            }
        }
    }
    Ok(report)
}
