//! First-party manifest tooling served from the single library path (`wfl fmt`,
//! `wfl manifest --json`, `wfl manifest --hash`).
//!
//! Both the `wfl` binary and the `wflpkg` alias call straight into these
//! functions, so there is one implementation and one parser (ADR-001 §5.5).

use std::path::Path;

use crate::datalit;
use crate::error::PackageError;
use crate::manifest::parser::grammar_to_pkg_err;

/// Parse a manifest/lockfile file through the frozen grammar, mapping a
/// [`datalit::GrammarError`] to a user-facing [`PackageError`] with a line
/// number and the stable `MG-*` code.
fn parse_file(path: &Path) -> Result<(String, datalit::Document), PackageError> {
    let content = std::fs::read_to_string(path)?;
    let doc = datalit::parse(content.as_bytes()).map_err(|e| grammar_to_pkg_err(&content, e))?;
    Ok((content, doc))
}

/// `wfl fmt [file]` — rewrite a manifest/lockfile in the canonical `wfl fmt`
/// byte form. With `check_only`, verify without writing (non-zero exit if not
/// canonical). `wfl fmt` is the only sanctioned writer and never runs on the
/// verification path (grammar §7.1).
pub fn fmt_file(path: &Path, check_only: bool) -> Result<(), PackageError> {
    let (content, doc) = parse_file(path)?;
    let canonical = datalit::fmt::to_canonical(&doc);
    let display = path.display();

    if content == canonical {
        println!("{display} is already in canonical form.");
        return Ok(());
    }

    if check_only {
        return Err(PackageError::General(format!(
            "{display} is not in canonical form.\n\nRun `wfl fmt {display}` to rewrite it."
        )));
    }

    std::fs::write(path, &canonical)?;
    println!("Formatted {display}.");
    Ok(())
}

/// `wfl manifest --json [file]` — emit the deterministic JCS JSON projection to
/// stdout. Lossless, derived; the WFL literal remains the source of truth
/// (grammar §7.2).
pub fn manifest_json(path: &Path) -> Result<(), PackageError> {
    let (_content, doc) = parse_file(path)?;
    println!("{}", datalit::json::to_jcs(&doc));
    Ok(())
}

/// `wfl manifest --hash [file]` — print the two SHA-256 digests: the `file_hash`
/// over the canonical bytes and the `content_hash` over the JCS projection
/// (grammar §7.3).
pub fn manifest_hash(path: &Path) -> Result<(), PackageError> {
    let (_content, doc) = parse_file(path)?;
    println!("file_hash    {}", datalit::hash::file_hash(&doc));
    println!("content_hash {}", datalit::hash::content_hash(&doc));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp(name: &str, content: &str) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        (dir, path)
    }

    const MANIFEST: &str = "\
create map wflpkg:
    grammar is \"1.0.0\"
end map

create map package:
    name is \"greeting\"
    version is \"26.2.1\"
    description is \"hi\"
end map
";

    #[test]
    fn fmt_rewrites_and_is_idempotent() {
        // Non-canonical input (extra blank line, compact list) still parses.
        let messy = "create map wflpkg:\n  grammar is \"1.0.0\"\nend map\n\ncreate map package:\n  name is \"greeting\"\n  version is \"26.2.1\"\n  description is \"hi\"\nend map\n";
        let (_d, path) = write_temp("project.wfl", messy);
        fmt_file(&path, false).unwrap();
        let after = std::fs::read_to_string(&path).unwrap();
        // Second run is a no-op (idempotent / already canonical).
        fmt_file(&path, true).unwrap();
        assert_eq!(after, std::fs::read_to_string(&path).unwrap());
    }

    #[test]
    fn json_projection_prints_ok() {
        let (_d, path) = write_temp("project.wfl", MANIFEST);
        manifest_json(&path).unwrap();
        manifest_hash(&path).unwrap();
    }

    #[test]
    fn rejects_bad_manifest_with_code() {
        let (_d, path) = write_temp("project.wfl", "create map package:\nx is true\nend map\n");
        let err = fmt_file(&path, false).unwrap_err();
        // Envelope missing / boolean spelling — either way it fails with a code.
        assert!(err.to_string().contains("MG-") || err.to_string().contains("version block"));
    }
}
