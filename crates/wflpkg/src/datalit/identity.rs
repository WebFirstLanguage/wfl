//! Gate I — identity / semantic checks (grammar §8).
//!
//! Runs post-parse on the identity-bearing fields: the string values of the
//! keys `scope`, `name`, and `version`, wherever a record carries them.
//!
//! **Restriction level: ASCII-Only.** `scope`/`name` are constrained to
//! `[a-z][a-z0-9]*(-[a-z0-9]+)*` — the strongest UTS #39 level, matching npm /
//! crates.io / PyPI. Because ASCII confusables are Unicode-version-stable, the
//! identity path's security does not depend on the Unicode pin at all. The
//! UTS #39 mixed-script / restriction-level check ([`uts39_tripwire`]) runs as a
//! tripwire that is *vacuous under ASCII-only* but is already wired, so any
//! future relaxation off ASCII cannot silently skip it (`MG-I03`).

use super::error::{Code, GrammarError, GrammarResult};
use super::limits::MAX_IDENTITY_LEN;
use super::version::{SemVer, VersionConstraint};
use super::{Document, Value};

/// Run Gate I over a parsed document.
pub fn check(doc: &Document) -> GrammarResult<()> {
    for record in &doc.records {
        for entry in &record.entries {
            let Value::String(s) = &entry.value else {
                continue; // identity policy governs string-valued fields only
            };
            match entry.key.as_str() {
                "scope" | "name" => check_identity(s, entry.offset)?,
                "version" => check_version(s, &record.kind, entry.offset)?,
                _ => {}
            }
        }
    }
    Ok(())
}

/// `MG-I01` allowlist + `MG-I03` tripwire for `scope` / `name`.
fn check_identity(s: &str, offset: usize) -> GrammarResult<()> {
    if !valid_identity(s) {
        return Err(GrammarError::new(
            Code::MgI01,
            offset,
            format!(
                "`{s}` is not a valid name. Use lowercase letters, digits and single hyphens \
                 (a-z, 0-9, -), 1–{MAX_IDENTITY_LEN} characters, no leading/trailing/double \
                 hyphen and no underscore."
            ),
        ));
    }
    // Wired-but-vacuous under ASCII-only (see module docs).
    if !uts39_tripwire(s) {
        return Err(GrammarError::new(
            Code::MgI03,
            offset,
            format!("`{s}` mixes scripts in a confusable way."),
        ));
    }
    Ok(())
}

/// `MG-I02` version validation. A `version` in a `dependency` record is a
/// constraint; anywhere else it is an exact `MAJOR.MINOR.PATCH`.
fn check_version(s: &str, kind: &str, offset: usize) -> GrammarResult<()> {
    let result = if kind == "dependency" {
        VersionConstraint::parse(s).map(|_| ())
    } else {
        SemVer::parse_exact(s).map(|_| ())
    };
    result.map_err(|e| {
        GrammarError::new(
            Code::MgI02,
            offset,
            format!("`{s}` is not a valid version: {}.", e.message),
        )
    })
}

/// Public predicate for the identity allowlist, so writers (`wfl create`) can
/// reject an invalid package name *before* emitting a manifest the parser would
/// then reject on read.
pub fn is_valid_identity(s: &str) -> bool {
    valid_identity(s)
}

/// The identity allowlist `[a-z][a-z0-9]*(-[a-z0-9]+)*`, `len ≤ 64`, underscore
/// forbidden (kills the `-`/`_` collision class), no leading/trailing/double
/// hyphen.
fn valid_identity(s: &str) -> bool {
    if s.is_empty() || s.len() > MAX_IDENTITY_LEN {
        return false;
    }
    let bytes = s.as_bytes();
    if !bytes[0].is_ascii_lowercase() {
        return false; // must start with a letter
    }
    let mut prev_hyphen = false;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'-' {
            if i == 0 || prev_hyphen {
                return false; // no leading or double hyphen
            }
            prev_hyphen = true;
        } else if b.is_ascii_lowercase() || b.is_ascii_digit() {
            prev_hyphen = false;
        } else {
            return false; // underscore, uppercase, non-ASCII, etc.
        }
    }
    !prev_hyphen // no trailing hyphen
}

/// UTS #39 mixed-script / restriction-level tripwire (§8, `MG-I03`).
///
/// Vacuous under the ASCII-Only allowlist: any string that passed
/// [`valid_identity`] is single-script (Latin/Common) and thus at the
/// ASCII-Only restriction level. The check is nonetheless expressed here so a
/// future relaxation off ASCII must route through it (and wire in the vendored
/// `unicode-security` tables) rather than skip it.
fn uts39_tripwire(s: &str) -> bool {
    // ASCII text is trivially single-script and non-confusable.
    s.is_ascii()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist() {
        assert!(valid_identity("http-client"));
        assert!(valid_identity("a"));
        assert!(valid_identity("json2"));
        assert!(valid_identity("a-b-c"));
        // Rejections
        assert!(!valid_identity("Http")); // uppercase
        assert!(!valid_identity("my_pkg")); // underscore
        assert!(!valid_identity("-lead")); // leading hyphen
        assert!(!valid_identity("trail-")); // trailing hyphen
        assert!(!valid_identity("double--hyphen"));
        assert!(!valid_identity("1abc")); // must start with a letter
        assert!(!valid_identity(&"a".repeat(65))); // too long
    }
}
