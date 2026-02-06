pub mod parser;

/// A workspace definition parsed from `workspace.wfl`.
#[derive(Debug, Clone)]
pub struct Workspace {
    pub name: String,
    pub members: Vec<String>,
}
