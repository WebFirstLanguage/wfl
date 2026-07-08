//! Version and version-constraint grammar (grammar §5, §8).
//!
//! Per the resolved design (`wflpkg-brainstorm-results.md`), resolution is
//! **SemVer**: `MAJOR.MINOR.PATCH`, `MAJOR < 256` (WFL's MSI rule), optional
//! `-prerelease` / `+build`, ASCII-only, no leading zeros. Constraints keep
//! WFL's natural-language surface (`"26.1 or newer"`, `"between 25.12 and
//! 26.2"`). This module is the single validator used both by Gate I (`MG-I02`,
//! string-level) and by the schema/resolver layer (semantic).

use std::cmp::Ordering;
use std::fmt;

/// A validation failure. Carries an offset delta (0-based within the version
/// string) so Gate I can point at the right byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionError {
    pub message: String,
}

impl VersionError {
    fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

/// A fully-specified `MAJOR.MINOR.PATCH[-pre][+build]` version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVer {
    pub major: u16,
    pub minor: u64,
    pub patch: u64,
    pub prerelease: Option<String>,
    pub build: Option<String>,
}

impl SemVer {
    /// Parse an exact version: exactly three numeric components, `MAJOR < 256`,
    /// no leading zeros, optional `-pre` / `+build` of `[0-9A-Za-z.-]`.
    pub fn parse_exact(s: &str) -> Result<SemVer, VersionError> {
        if !s.is_ascii() {
            return Err(VersionError::new("versions must be ASCII"));
        }
        // Split off build (+) then prerelease (-).
        let (core_pre, build) = match s.split_once('+') {
            Some((a, b)) => {
                validate_alnumdot(b, "build")?;
                (a, Some(b.to_string()))
            }
            None => (s, None),
        };
        let (core, prerelease) = match core_pre.split_once('-') {
            Some((a, b)) => {
                validate_alnumdot(b, "prerelease")?;
                (a, Some(b.to_string()))
            }
            None => (core_pre, None),
        };
        let parts: Vec<&str> = core.split('.').collect();
        if parts.len() != 3 {
            return Err(VersionError::new(
                "an exact version needs three parts: MAJOR.MINOR.PATCH",
            ));
        }
        let major = parse_numeric(parts[0])?;
        let minor = parse_numeric(parts[1])?;
        let patch = parse_numeric(parts[2])?;
        if major > 255 {
            return Err(VersionError::new("MAJOR must be below 256"));
        }
        Ok(SemVer {
            major: major as u16,
            minor,
            patch,
            prerelease,
            build,
        })
    }

    fn core_tuple(&self) -> (u16, u64, u64) {
        (self.major, self.minor, self.patch)
    }
}

impl fmt::Display for SemVer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.prerelease {
            write!(f, "-{pre}")?;
        }
        if let Some(b) = &self.build {
            write!(f, "+{b}")?;
        }
        Ok(())
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        // Core precedence first; build metadata is ignored (SemVer §10).
        self.core_tuple()
            .cmp(&other.core_tuple())
            .then_with(|| cmp_prerelease(&self.prerelease, &other.prerelease))
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A prefix of a version used inside constraints: `26`, `26.1`, or `26.1.3`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialVersion {
    major: u16,
    minor: Option<u64>,
    patch: Option<u64>,
}

impl PartialVersion {
    fn parse(s: &str) -> Result<PartialVersion, VersionError> {
        if !s.is_ascii() {
            return Err(VersionError::new("versions must be ASCII"));
        }
        // Constraints do not carry prerelease/build.
        if s.contains(['-', '+']) {
            return Err(VersionError::new(
                "version constraints use plain MAJOR[.MINOR[.PATCH]] numbers",
            ));
        }
        let parts: Vec<&str> = s.split('.').collect();
        if parts.is_empty() || parts.len() > 3 {
            return Err(VersionError::new(
                "a version has one to three numeric parts",
            ));
        }
        let major = parse_numeric(parts[0])?;
        if major > 255 {
            return Err(VersionError::new("MAJOR must be below 256"));
        }
        let minor = parts.get(1).map(|p| parse_numeric(p)).transpose()?;
        let patch = parts.get(2).map(|p| parse_numeric(p)).transpose()?;
        Ok(PartialVersion {
            major: major as u16,
            minor,
            patch,
        })
    }

    /// Smallest concrete version matching this prefix (unspecified parts → 0).
    fn floor(&self) -> (u16, u64, u64) {
        (self.major, self.minor.unwrap_or(0), self.patch.unwrap_or(0))
    }

    /// Largest concrete version matching this prefix (unspecified parts → max).
    fn ceil(&self) -> (u16, u64, u64) {
        (
            self.major,
            self.minor.unwrap_or(u64::MAX),
            self.patch.unwrap_or(u64::MAX),
        )
    }
}

/// A dependency version constraint, natural-language surface preserved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionConstraint {
    /// `26.1 or newer`
    OrNewer(PartialVersion),
    /// `26.1.3 exactly`
    Exactly(PartialVersion),
    /// `between 25.12 and 26.2`
    Between(PartialVersion, PartialVersion),
    /// `any version`
    Any,
    /// `above 25.6`
    Above(PartialVersion),
    /// `below 27`
    Below(PartialVersion),
    /// `26.1 or newer but below 27`
    AboveBelow(PartialVersion, PartialVersion),
}

impl VersionConstraint {
    /// Parse a constraint string. This is the exact set of natural-language
    /// forms admitted by the grammar (§5).
    pub fn parse(s: &str) -> Result<VersionConstraint, VersionError> {
        let s = s.trim();
        if s == "any version" {
            return Ok(VersionConstraint::Any);
        }
        if let Some(rest) = s.strip_prefix("between ") {
            let (a, b) = rest
                .split_once(" and ")
                .ok_or_else(|| VersionError::new("expected `between X and Y`"))?;
            return Ok(VersionConstraint::Between(
                PartialVersion::parse(a.trim())?,
                PartialVersion::parse(b.trim())?,
            ));
        }
        if let Some((a, b)) = s.split_once(" or newer but below ") {
            return Ok(VersionConstraint::AboveBelow(
                PartialVersion::parse(a.trim())?,
                PartialVersion::parse(b.trim())?,
            ));
        }
        if let Some(rest) = s.strip_suffix(" or newer") {
            return Ok(VersionConstraint::OrNewer(PartialVersion::parse(
                rest.trim(),
            )?));
        }
        if let Some(rest) = s.strip_suffix(" exactly") {
            return Ok(VersionConstraint::Exactly(PartialVersion::parse(
                rest.trim(),
            )?));
        }
        if let Some(rest) = s.strip_prefix("above ") {
            return Ok(VersionConstraint::Above(PartialVersion::parse(
                rest.trim(),
            )?));
        }
        if let Some(rest) = s.strip_prefix("below ") {
            return Ok(VersionConstraint::Below(PartialVersion::parse(
                rest.trim(),
            )?));
        }
        Err(VersionError::new(
            "I could not understand this version constraint",
        ))
    }

    /// Does a concrete version satisfy this constraint?
    pub fn matches(&self, v: &SemVer) -> bool {
        let t = v.core_tuple();
        match self {
            VersionConstraint::Any => true,
            VersionConstraint::OrNewer(p) => t >= p.floor(),
            VersionConstraint::Exactly(p) => t >= p.floor() && t <= p.ceil(),
            VersionConstraint::Between(a, b) => t >= a.floor() && t <= b.ceil(),
            VersionConstraint::Above(p) => t > p.ceil(),
            VersionConstraint::Below(p) => t < p.floor(),
            VersionConstraint::AboveBelow(a, b) => t >= a.floor() && t < b.floor(),
        }
    }
}

impl fmt::Display for PartialVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.major)?;
        if let Some(m) = self.minor {
            write!(f, ".{m}")?;
        }
        if let Some(p) = self.patch {
            write!(f, ".{p}")?;
        }
        Ok(())
    }
}

impl fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionConstraint::OrNewer(v) => write!(f, "{v} or newer"),
            VersionConstraint::Exactly(v) => write!(f, "{v} exactly"),
            VersionConstraint::Between(a, b) => write!(f, "between {a} and {b}"),
            VersionConstraint::Any => write!(f, "any version"),
            VersionConstraint::Above(v) => write!(f, "above {v}"),
            VersionConstraint::Below(v) => write!(f, "below {v}"),
            VersionConstraint::AboveBelow(a, b) => write!(f, "{a} or newer but below {b}"),
        }
    }
}

/// A single numeric component: `[0-9]+`, no leading zeros (except the bare `0`).
fn parse_numeric(s: &str) -> Result<u64, VersionError> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(VersionError::new("version parts must be numbers"));
    }
    if s.len() > 1 && s.starts_with('0') {
        return Err(VersionError::new(
            "version parts must not have leading zeros",
        ));
    }
    s.parse::<u64>()
        .map_err(|_| VersionError::new("version part is too large"))
}

/// `alnumdot = 1*( ALPHA / DIGIT / "-" / "." )` for prerelease / build (§5).
fn validate_alnumdot(s: &str, what: &str) -> Result<(), VersionError> {
    if s.is_empty()
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
    {
        return Err(VersionError::new(format!(
            "{what} may contain only letters, digits, hyphens and dots"
        )));
    }
    Ok(())
}

/// SemVer prerelease precedence: a version with a prerelease is *lower* than one
/// without; otherwise compare identifiers dot-by-dot (numeric < alphanumeric).
fn cmp_prerelease(a: &Option<String>, b: &Option<String>) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => {
            let mut ai = a.split('.');
            let mut bi = b.split('.');
            loop {
                match (ai.next(), bi.next()) {
                    (None, None) => return Ordering::Equal,
                    (None, Some(_)) => return Ordering::Less,
                    (Some(_), None) => return Ordering::Greater,
                    (Some(x), Some(y)) => {
                        let ord = match (x.parse::<u64>(), y.parse::<u64>()) {
                            (Ok(xn), Ok(yn)) => xn.cmp(&yn),
                            (Ok(_), Err(_)) => Ordering::Less,
                            (Err(_), Ok(_)) => Ordering::Greater,
                            (Err(_), Err(_)) => x.cmp(y),
                        };
                        if ord != Ordering::Equal {
                            return ord;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_versions() {
        assert!(SemVer::parse_exact("26.2.1").is_ok());
        assert!(SemVer::parse_exact("0.0.0").is_ok());
        assert!(SemVer::parse_exact("26.2.1-alpha.1").is_ok());
        assert!(SemVer::parse_exact("26.2.1+build.5").is_ok());
        // Rejections
        assert!(SemVer::parse_exact("26.2").is_err()); // needs 3 parts
        assert!(SemVer::parse_exact("256.0.0").is_err()); // MAJOR < 256
        assert!(SemVer::parse_exact("26.02.1").is_err()); // leading zero
        assert!(SemVer::parse_exact("26.2.1.0").is_err()); // 4 parts
    }

    #[test]
    fn ordering() {
        let a = SemVer::parse_exact("25.12.1").unwrap();
        let b = SemVer::parse_exact("26.1.0").unwrap();
        let c = SemVer::parse_exact("26.1.3").unwrap();
        assert!(a < b && b < c);
        // Prerelease is lower than release.
        let pre = SemVer::parse_exact("26.1.0-rc.1").unwrap();
        assert!(pre < b);
    }

    #[test]
    fn constraints() {
        let newer = VersionConstraint::parse("26.1 or newer").unwrap();
        assert!(newer.matches(&SemVer::parse_exact("26.1.0").unwrap()));
        assert!(newer.matches(&SemVer::parse_exact("26.5.9").unwrap()));
        assert!(!newer.matches(&SemVer::parse_exact("25.12.0").unwrap()));

        let between = VersionConstraint::parse("between 25.12 and 26.2").unwrap();
        assert!(between.matches(&SemVer::parse_exact("26.2.9").unwrap()));
        assert!(!between.matches(&SemVer::parse_exact("26.3.0").unwrap()));

        let below = VersionConstraint::parse("below 27").unwrap();
        assert!(below.matches(&SemVer::parse_exact("26.12.99").unwrap()));
        assert!(!below.matches(&SemVer::parse_exact("27.0.0").unwrap()));

        assert!(
            VersionConstraint::parse("any version")
                .unwrap()
                .matches(&SemVer::parse_exact("1.2.3").unwrap())
        );
        assert!(VersionConstraint::parse("nonsense").is_err());
    }
}
