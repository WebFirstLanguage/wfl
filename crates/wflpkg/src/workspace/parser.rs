use std::path::Path;

use crate::error::PackageError;
use crate::workspace::Workspace;

/// Parse a `workspace.wfl` file from the given directory.
pub fn parse_workspace_file(workspace_dir: &Path) -> Result<Workspace, PackageError> {
    let path = workspace_dir.join("workspace.wfl");
    if !path.exists() {
        return Err(PackageError::WorkspaceError(
            "No workspace.wfl file found in this directory.".to_string(),
        ));
    }

    let content = std::fs::read_to_string(&path)?;
    parse_workspace(&content)
}

/// Parse workspace content.
pub fn parse_workspace(content: &str) -> Result<Workspace, PackageError> {
    let mut name = String::new();
    let mut members = Vec::new();
    let mut line_num = 0;

    for raw_line in content.lines() {
        line_num += 1;
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        if let Some(rest) = line.strip_prefix("name is ") {
            name = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("member is ") {
            members.push(rest.trim().to_string());
        } else {
            return Err(PackageError::WorkspaceError(format!(
                "Unexpected line {} in workspace.wfl: {}",
                line_num, line
            )));
        }
    }

    if name.is_empty() {
        return Err(PackageError::WorkspaceError(
            "workspace.wfl is missing the 'name is ...' field.".to_string(),
        ));
    }

    if members.is_empty() {
        return Err(PackageError::WorkspaceError(
            "workspace.wfl has no members defined. Add 'member is path/to/package'.".to_string(),
        ));
    }

    Ok(Workspace { name, members })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_workspace() {
        let content = "\
// workspace.wfl

name is my-organization

member is packages/core
member is packages/web-server
member is packages/cli-tool
";
        let ws = parse_workspace(content).unwrap();
        assert_eq!(ws.name, "my-organization");
        assert_eq!(ws.members.len(), 3);
        assert_eq!(ws.members[0], "packages/core");
        assert_eq!(ws.members[1], "packages/web-server");
        assert_eq!(ws.members[2], "packages/cli-tool");
    }

    #[test]
    fn test_parse_workspace_missing_name() {
        let content = "member is packages/core";
        assert!(parse_workspace(content).is_err());
    }

    #[test]
    fn test_parse_workspace_no_members() {
        let content = "name is my-org";
        assert!(parse_workspace(content).is_err());
    }
}
