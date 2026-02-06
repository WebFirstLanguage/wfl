pub mod algorithm;
pub mod package_path;

use std::collections::HashMap;

use crate::manifest::version::Version;

/// The result of dependency resolution: a map of package name to exact version.
#[derive(Debug, Clone, Default)]
pub struct ResolvedSet {
    pub packages: HashMap<String, Version>,
}
