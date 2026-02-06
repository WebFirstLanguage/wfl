use std::cmp::Ordering;
use std::fmt;

use crate::error::PackageError;

/// A WFL version following YY.MM.BUILD calendar versioning.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Version {
    pub year: u32,
    pub month: u32,
    pub build: Option<u32>,
}

impl Version {
    pub fn new(year: u32, month: u32, build: Option<u32>) -> Self {
        Self { year, month, build }
    }

    /// Parse a version string like "26.1.3", "26.1", or "27" (year only).
    pub fn parse(s: &str) -> Result<Self, PackageError> {
        let parts: Vec<&str> = s.trim().split('.').collect();
        match parts.len() {
            1 => {
                // Year only: "27" means the start of year 27 (27.1.0)
                let year = parts[0]
                    .parse::<u32>()
                    .map_err(|_| PackageError::InvalidVersion(s.to_string()))?;
                Ok(Self::new(year, 1, None))
            }
            2 => {
                let year = parts[0]
                    .parse::<u32>()
                    .map_err(|_| PackageError::InvalidVersion(s.to_string()))?;
                let month = parts[1]
                    .parse::<u32>()
                    .map_err(|_| PackageError::InvalidVersion(s.to_string()))?;
                if !(1..=12).contains(&month) {
                    return Err(PackageError::InvalidVersion(s.to_string()));
                }
                Ok(Self::new(year, month, None))
            }
            3 => {
                let year = parts[0]
                    .parse::<u32>()
                    .map_err(|_| PackageError::InvalidVersion(s.to_string()))?;
                let month = parts[1]
                    .parse::<u32>()
                    .map_err(|_| PackageError::InvalidVersion(s.to_string()))?;
                let build = parts[2]
                    .parse::<u32>()
                    .map_err(|_| PackageError::InvalidVersion(s.to_string()))?;
                if !(1..=12).contains(&month) {
                    return Err(PackageError::InvalidVersion(s.to_string()));
                }
                Ok(Self::new(year, month, Some(build)))
            }
            _ => Err(PackageError::InvalidVersion(s.to_string())),
        }
    }

    /// Return the version with build defaulting to 0 for comparisons.
    fn build_or_zero(&self) -> u32 {
        self.build.unwrap_or(0)
    }

    /// Check if this version matches a version with no build specified
    /// (i.e. "26.1" matches any "26.1.x").
    pub fn matches_prefix(&self, prefix: &Version) -> bool {
        self.year == prefix.year && self.month == prefix.month
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.build {
            Some(build) => write!(f, "{}.{}.{}", self.year, self.month, build),
            None => write!(f, "{}.{}", self.year, self.month),
        }
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.year
            .cmp(&other.year)
            .then(self.month.cmp(&other.month))
            .then(self.build_or_zero().cmp(&other.build_or_zero()))
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Version constraint types for dependency resolution.
#[derive(Debug, Clone, PartialEq)]
pub enum VersionConstraint {
    /// `26.1 or newer` — >= 26.1.0
    OrNewer(Version),
    /// `26.1.3 exactly` — == 26.1.3
    Exactly(Version),
    /// `between 25.12 and 26.2` — >= 25.12.0, <= 26.2.x
    Between(Version, Version),
    /// `any version` — no constraint
    AnyVersion,
    /// `above 25.6` — > 25.6.x
    Above(Version),
    /// `below 27` — < 27.0.0
    Below(Version),
    /// `26.1 or newer but below 27` — >= 26.1.0, < 27.0.0
    AboveBelow(Version, Version),
}

impl VersionConstraint {
    /// Check if a version satisfies this constraint.
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            VersionConstraint::OrNewer(min) => version >= min,
            VersionConstraint::Exactly(exact) => {
                if exact.build.is_some() {
                    version == exact
                } else {
                    version.matches_prefix(exact)
                }
            }
            VersionConstraint::Between(min, max) => version >= min && version <= max,
            VersionConstraint::AnyVersion => true,
            VersionConstraint::Above(min) => version > min,
            VersionConstraint::Below(max) => version < max,
            VersionConstraint::AboveBelow(min, max) => version >= min && version < max,
        }
    }

    /// Parse a version constraint from a string like "26.1 or newer".
    pub fn parse(s: &str) -> Result<Self, PackageError> {
        let s = s.trim();

        if s == "any version" {
            return Ok(VersionConstraint::AnyVersion);
        }

        // "between X and Y"
        if let Some(rest) = s.strip_prefix("between ") {
            let parts: Vec<&str> = rest.splitn(2, " and ").collect();
            if parts.len() != 2 {
                return Err(PackageError::InvalidVersionConstraint(s.to_string()));
            }
            let min = Version::parse(parts[0])?;
            let max = Version::parse(parts[1])?;
            return Ok(VersionConstraint::Between(min, max));
        }

        // "above X"
        if let Some(rest) = s.strip_prefix("above ") {
            let version = Version::parse(rest)?;
            return Ok(VersionConstraint::Above(version));
        }

        // "below X"
        if let Some(rest) = s.strip_prefix("below ") {
            let version = Version::parse(rest)?;
            return Ok(VersionConstraint::Below(version));
        }

        // "X or newer but below Y"
        if s.contains(" or newer but below ") {
            let parts: Vec<&str> = s.splitn(2, " or newer but below ").collect();
            if parts.len() != 2 {
                return Err(PackageError::InvalidVersionConstraint(s.to_string()));
            }
            let min = Version::parse(parts[0])?;
            let max = Version::parse(parts[1])?;
            return Ok(VersionConstraint::AboveBelow(min, max));
        }

        // "X or newer"
        if let Some(version_str) = s.strip_suffix(" or newer") {
            let version = Version::parse(version_str)?;
            return Ok(VersionConstraint::OrNewer(version));
        }

        // "X exactly"
        if let Some(version_str) = s.strip_suffix(" exactly") {
            let version = Version::parse(version_str)?;
            return Ok(VersionConstraint::Exactly(version));
        }

        Err(PackageError::InvalidVersionConstraint(s.to_string()))
    }
}

impl fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionConstraint::OrNewer(v) => write!(f, "{} or newer", v),
            VersionConstraint::Exactly(v) => write!(f, "{} exactly", v),
            VersionConstraint::Between(min, max) => write!(f, "between {} and {}", min, max),
            VersionConstraint::AnyVersion => write!(f, "any version"),
            VersionConstraint::Above(v) => write!(f, "above {}", v),
            VersionConstraint::Below(v) => write!(f, "below {}", v),
            VersionConstraint::AboveBelow(min, max) => {
                write!(f, "{} or newer but below {}", min, max)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parse() {
        let v = Version::parse("26.1.3").unwrap();
        assert_eq!(v.year, 26);
        assert_eq!(v.month, 1);
        assert_eq!(v.build, Some(3));

        let v = Version::parse("26.1").unwrap();
        assert_eq!(v.year, 26);
        assert_eq!(v.month, 1);
        assert_eq!(v.build, None);
    }

    #[test]
    fn test_version_ordering() {
        let v1 = Version::parse("25.12.1").unwrap();
        let v2 = Version::parse("26.1.0").unwrap();
        let v3 = Version::parse("26.1.3").unwrap();
        assert!(v1 < v2);
        assert!(v2 < v3);
    }

    #[test]
    fn test_version_display() {
        assert_eq!(Version::new(26, 1, Some(3)).to_string(), "26.1.3");
        assert_eq!(Version::new(26, 1, None).to_string(), "26.1");
    }

    #[test]
    fn test_constraint_parse_and_match() {
        let c = VersionConstraint::parse("26.1 or newer").unwrap();
        assert!(c.matches(&Version::parse("26.1.0").unwrap()));
        assert!(c.matches(&Version::parse("26.2.0").unwrap()));
        assert!(!c.matches(&Version::parse("25.12.0").unwrap()));

        let c = VersionConstraint::parse("26.1.3 exactly").unwrap();
        assert!(c.matches(&Version::parse("26.1.3").unwrap()));
        assert!(!c.matches(&Version::parse("26.1.4").unwrap()));

        let c = VersionConstraint::parse("any version").unwrap();
        assert!(c.matches(&Version::parse("1.1.0").unwrap()));

        let c = VersionConstraint::parse("between 25.12 and 26.2").unwrap();
        assert!(c.matches(&Version::parse("26.1.0").unwrap()));
        assert!(!c.matches(&Version::parse("26.3.0").unwrap()));

        let c = VersionConstraint::parse("above 25.6").unwrap();
        assert!(c.matches(&Version::parse("25.7.0").unwrap()));
        assert!(!c.matches(&Version::parse("25.6.0").unwrap()));

        let c = VersionConstraint::parse("below 27").unwrap();
        assert!(c.matches(&Version::parse("26.12.99").unwrap()));
        assert!(!c.matches(&Version::parse("27.1.0").unwrap()));

        let c = VersionConstraint::parse("26.1 or newer but below 27").unwrap();
        assert!(c.matches(&Version::parse("26.5.0").unwrap()));
        assert!(!c.matches(&Version::parse("25.12.0").unwrap()));
        assert!(!c.matches(&Version::parse("27.1.0").unwrap()));
    }
}
