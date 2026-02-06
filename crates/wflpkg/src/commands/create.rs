use std::path::{Path, PathBuf};

use crate::error::PackageError;
use crate::manifest::ProjectManifest;

/// Create a new WFL project, either interactively or with a given name.
pub fn create_project(name: Option<&str>, base_path: &Path) -> Result<PathBuf, PackageError> {
    let (project_name, description, author, license) = if let Some(name) = name {
        // Non-interactive mode
        (
            name.to_string(),
            "A new WFL project".to_string(),
            String::new(),
            "MIT".to_string(),
        )
    } else {
        // Interactive wizard
        run_wizard()?
    };

    // Validate name
    validate_project_name(&project_name)?;

    // Create project directory
    let project_dir = base_path.join(&project_name);
    if project_dir.exists() {
        return Err(PackageError::General(format!(
            "A directory called \"{}\" already exists.\n\n\
             Choose a different name, or delete the existing directory first.",
            project_name
        )));
    }

    std::fs::create_dir_all(project_dir.join("src"))?;

    // Create project.wfl
    let mut manifest = ProjectManifest {
        name: project_name.clone(),
        version_string: default_version(),
        description,
        license: Some(license),
        entry: Some("src/main.wfl".to_string()),
        ..Default::default()
    };

    if !author.is_empty() {
        manifest.authors = vec![author];
    }

    manifest.save(&project_dir.join("project.wfl"))?;

    // Create .wflcfg
    let wflcfg_content = "# WebFirst Language Configuration\n\
         # Created by wfl create project\n\n\
         timeout_seconds = 60\n\
         logging_enabled = false\n"
        .to_string();
    std::fs::write(project_dir.join(".wflcfg"), wflcfg_content)?;

    // Create src/main.wfl
    let main_content = format!(
        "// {} - {}\n\n\
         display \"Hello from {}!\"\n",
        project_name, manifest.description, project_name,
    );
    std::fs::write(project_dir.join("src").join("main.wfl"), main_content)?;

    // Create .gitignore
    let gitignore_content = "packages/\n*.log\nwfl-debug-report-*.txt\n";
    std::fs::write(project_dir.join(".gitignore"), gitignore_content)?;

    println!(
        "Created project \"{}\" at {}",
        project_name,
        project_dir.display()
    );
    println!();
    println!("To get started:");
    println!("  cd {}", project_name);
    println!("  wfl run");

    Ok(project_dir)
}

/// Run the interactive project creation wizard.
fn run_wizard() -> Result<(String, String, String, String), PackageError> {
    use rustyline::DefaultEditor;

    let mut editor = DefaultEditor::new()
        .map_err(|e| PackageError::General(format!("Failed to start wizard: {}", e)))?;

    println!();
    println!("WFL Project Creation Wizard");
    println!("===========================");
    println!();
    println!("I will help you create a new WFL project.");
    println!("Press Enter to accept the default value shown in brackets.");
    println!();

    // Project name
    let name = loop {
        let input = editor
            .readline("Project name: ")
            .map_err(|e| PackageError::General(format!("Input error: {}", e)))?;
        let input = input.trim().to_string();
        if input.is_empty() {
            println!("  A project name is required.");
            continue;
        }
        if let Err(e) = validate_project_name(&input) {
            println!("  {}", e);
            continue;
        }
        break input;
    };

    // Description
    let description = editor
        .readline("Description [A new WFL project]: ")
        .map_err(|e| PackageError::General(format!("Input error: {}", e)))?;
    let description = if description.trim().is_empty() {
        "A new WFL project".to_string()
    } else {
        description.trim().to_string()
    };

    // Author
    let author = editor
        .readline("Author (optional): ")
        .map_err(|e| PackageError::General(format!("Input error: {}", e)))?;
    let author = author.trim().to_string();

    // License
    let license = editor
        .readline("License [MIT]: ")
        .map_err(|e| PackageError::General(format!("Input error: {}", e)))?;
    let license = if license.trim().is_empty() {
        "MIT".to_string()
    } else {
        license.trim().to_string()
    };

    Ok((name, description, author, license))
}

/// Validate a project name (same rules as package names).
fn validate_project_name(name: &str) -> Result<(), PackageError> {
    if name.is_empty() || name.len() > 64 {
        return Err(PackageError::InvalidPackageName(name.to_string()));
    }

    let first = name.chars().next().unwrap();
    if !first.is_ascii_lowercase() {
        return Err(PackageError::InvalidPackageName(name.to_string()));
    }

    for c in name.chars() {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' {
            return Err(PackageError::InvalidPackageName(name.to_string()));
        }
    }

    Ok(())
}

/// Get the default version based on current date.
fn default_version() -> String {
    let now = chrono::Local::now();
    format!("{}.{}.1", now.format("%y"), now.format("%-m"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_project_non_interactive() {
        let temp = TempDir::new().unwrap();
        let result = create_project(Some("test-app"), temp.path());
        assert!(result.is_ok());

        let project_dir = result.unwrap();
        assert!(project_dir.join("project.wfl").exists());
        assert!(project_dir.join("src/main.wfl").exists());
        assert!(project_dir.join(".wflcfg").exists());
        assert!(project_dir.join(".gitignore").exists());

        // Verify manifest can be parsed
        let manifest = ProjectManifest::load(&project_dir.join("project.wfl")).unwrap();
        assert_eq!(manifest.name, "test-app");
        assert_eq!(manifest.entry, Some("src/main.wfl".to_string()));
    }

    #[test]
    fn test_create_project_already_exists() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir(temp.path().join("existing-app")).unwrap();
        let result = create_project(Some("existing-app"), temp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_project_name() {
        assert!(validate_project_name("my-app").is_ok());
        assert!(validate_project_name("a").is_ok());
        assert!(validate_project_name("MyApp").is_err());
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("1app").is_err());
    }
}
