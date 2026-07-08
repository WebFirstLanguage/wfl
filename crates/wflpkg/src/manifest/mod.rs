pub mod parser;
pub mod schema;
pub mod version;
pub mod writer;

use version::VersionConstraint;

/// The project manifest parsed from `project.wfl`.
///
/// Since the manifest format switched to the frozen data-literal grammar
/// (`wflpkg-manifest-grammar-1.0.md`), this struct is the *schema view* of a
/// parsed [`crate::datalit::Document`]: the generic record/entry tree mapped
/// onto the fields wflpkg understands. The English aesthetic is preserved —
/// `project.wfl` reads as ordinary WFL `create map` blocks — while parsing goes
/// through the single shared lexer.
#[derive(Debug, Clone, Default)]
pub struct ProjectManifest {
    pub name: String,
    /// Optional publishing scope (namespace), e.g. `acme` in `@acme/greeting`.
    pub scope: Option<String>,
    pub version_string: String,
    pub description: String,
    pub authors: Vec<String>,
    pub license: Option<String>,
    pub keywords: Vec<String>,
    pub entry: Option<String>,
    pub repository: Option<String>,
    pub registry: Option<String>,
    /// Free-text annotations — first-class and **hashed** (grammar §12, F3), so
    /// they cannot be a review-differential channel the way comments would be.
    pub notes: Option<String>,
    pub dependencies: Vec<Dependency>,
    pub permissions: Vec<String>,
}

impl ProjectManifest {
    /// Load a manifest from a file path.
    pub fn load(path: &std::path::Path) -> Result<Self, crate::error::PackageError> {
        let content = std::fs::read_to_string(path)?;
        parser::parse_manifest(&content)
    }

    /// Save the manifest to a file path, in the canonical `wfl fmt` byte form.
    pub fn save(&self, path: &std::path::Path) -> Result<(), crate::error::PackageError> {
        let content = writer::write_manifest(self);
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Get the entry point, defaulting to "src/main.wfl".
    pub fn entry_point(&self) -> &str {
        self.entry.as_deref().unwrap_or("src/main.wfl")
    }

    /// Get the registry URL, defaulting to "wflhub.org".
    pub fn registry_url(&self) -> &str {
        self.registry.as_deref().unwrap_or("wflhub.org")
    }

    /// Find a dependency by name.
    pub fn find_dependency(&self, name: &str) -> Option<&Dependency> {
        self.dependencies.iter().find(|d| d.name == name)
    }

    /// Add or update a dependency.
    pub fn add_dependency(&mut self, dep: Dependency) {
        if let Some(existing) = self.dependencies.iter_mut().find(|d| d.name == dep.name) {
            *existing = dep;
        } else {
            self.dependencies.push(dep);
        }
    }

    /// Remove a dependency by name. Returns true if it was found and removed.
    pub fn remove_dependency(&mut self, name: &str) -> bool {
        let len_before = self.dependencies.len();
        self.dependencies.retain(|d| d.name != name);
        self.dependencies.len() < len_before
    }
}

/// A dependency declaration from the manifest.
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    /// Optional publishing scope (namespace) of the dependency.
    pub scope: Option<String>,
    pub constraint: VersionConstraint,
    pub dev_only: bool,
}
