use std::collections::HashMap;

use crate::error::PackageError;
use crate::manifest::Dependency;
use crate::manifest::version::{Version, VersionConstraint};
use crate::resolver::ResolvedSet;

/// A dependency resolver that uses a greedy algorithm.
///
/// Algorithm:
/// 1. Collect all constraints for each package from the manifest and transitive deps
/// 2. Topologically sort the dependency graph
/// 3. For each package, find the highest version satisfying all constraints
/// 4. Report conflicts with actionable error messages
#[derive(Default)]
pub struct DependencyResolver {
    /// Available versions per package (from registry/cache)
    available: HashMap<String, Vec<Version>>,
    /// Workspace member names (prefer local resolution)
    workspace_members: Vec<String>,
}

impl DependencyResolver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register available versions for a package.
    pub fn add_available(&mut self, name: &str, versions: Vec<Version>) {
        self.available.insert(name.to_string(), versions);
    }

    /// Register workspace members for local resolution preference.
    pub fn set_workspace_members(&mut self, members: Vec<String>) {
        self.workspace_members = members;
    }

    /// Resolve dependencies from a list of direct dependencies.
    pub fn resolve(&self, dependencies: &[Dependency]) -> Result<ResolvedSet, PackageError> {
        let mut constraints: HashMap<String, Vec<(VersionConstraint, String)>> = HashMap::new();

        // Collect all constraints (source is "project" for direct deps)
        for dep in dependencies {
            constraints
                .entry(dep.name.clone())
                .or_default()
                .push((dep.constraint.clone(), "your project".to_string()));
        }

        // Resolve each package
        let mut resolved = ResolvedSet::default();

        for (name, pkg_constraints) in &constraints {
            // Check if it's a workspace member
            if self.workspace_members.contains(name) {
                // Workspace members are resolved locally; skip registry resolution
                continue;
            }

            let versions =
                self.available
                    .get(name)
                    .ok_or_else(|| PackageError::PackageNotFound {
                        name: name.clone(),
                        suggestions: self.find_similar_names(name),
                    })?;

            // Find highest version satisfying all constraints
            let mut matching: Vec<&Version> = versions
                .iter()
                .filter(|v| pkg_constraints.iter().all(|(c, _)| c.matches(v)))
                .collect();

            matching.sort();

            if let Some(best) = matching.last() {
                resolved.packages.insert(name.clone(), (*best).clone());
            } else {
                // Find which constraints conflict
                if pkg_constraints.len() >= 2 {
                    let (c_a, src_a) = &pkg_constraints[0];
                    let (c_b, src_b) = &pkg_constraints[1];
                    return Err(PackageError::VersionConflict {
                        package: name.clone(),
                        constraint_a: c_a.to_string(),
                        source_a: src_a.clone(),
                        constraint_b: c_b.to_string(),
                        source_b: src_b.clone(),
                    });
                } else {
                    return Err(PackageError::General(format!(
                        "I could not find a version of \"{}\" matching {}.\n\n\
                         Available versions: {}",
                        name,
                        pkg_constraints[0].0,
                        versions
                            .iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )));
                }
            }
        }

        Ok(resolved)
    }

    /// Find package names similar to the given name (for suggestions).
    fn find_similar_names(&self, name: &str) -> Vec<String> {
        self.available
            .keys()
            .filter(|k| {
                // Simple similarity: shared prefix or edit distance heuristic
                let shorter = name.len().min(k.len());
                if shorter < 3 {
                    return false;
                }
                let shared = name
                    .chars()
                    .zip(k.chars())
                    .take_while(|(a, b)| a == b)
                    .count();
                shared >= shorter / 2 || k.contains(name) || name.contains(k.as_str())
            })
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_simple() {
        let mut resolver = DependencyResolver::new();
        resolver.add_available(
            "http-client",
            vec![
                Version::new(26, 1, Some(1)),
                Version::new(26, 1, Some(2)),
                Version::new(26, 1, Some(3)),
            ],
        );

        let deps = vec![Dependency {
            name: "http-client".to_string(),
            constraint: VersionConstraint::OrNewer(Version::new(26, 1, None)),
            dev_only: false,
        }];

        let result = resolver.resolve(&deps).unwrap();
        assert_eq!(
            result.packages.get("http-client").unwrap(),
            &Version::new(26, 1, Some(3))
        );
    }

    #[test]
    fn test_resolve_exact() {
        let mut resolver = DependencyResolver::new();
        resolver.add_available(
            "json-parser",
            vec![Version::new(25, 12, Some(5)), Version::new(25, 12, Some(8))],
        );

        let deps = vec![Dependency {
            name: "json-parser".to_string(),
            constraint: VersionConstraint::Exactly(Version::new(25, 12, Some(5))),
            dev_only: false,
        }];

        let result = resolver.resolve(&deps).unwrap();
        assert_eq!(
            result.packages.get("json-parser").unwrap(),
            &Version::new(25, 12, Some(5))
        );
    }

    #[test]
    fn test_resolve_not_found() {
        let resolver = DependencyResolver::new();
        let deps = vec![Dependency {
            name: "nonexistent".to_string(),
            constraint: VersionConstraint::AnyVersion,
            dev_only: false,
        }];
        assert!(resolver.resolve(&deps).is_err());
    }

    #[test]
    fn test_resolve_no_matching_version() {
        let mut resolver = DependencyResolver::new();
        resolver.add_available("old-pkg", vec![Version::new(24, 1, Some(1))]);

        let deps = vec![Dependency {
            name: "old-pkg".to_string(),
            constraint: VersionConstraint::OrNewer(Version::new(26, 1, None)),
            dev_only: false,
        }];

        assert!(resolver.resolve(&deps).is_err());
    }
}
