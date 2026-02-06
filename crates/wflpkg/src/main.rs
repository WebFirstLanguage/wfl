use std::env;
use std::path::Path;
use std::process;

/// Standalone `wflpkg` binary entry point.
/// Delegates to the same library functions used by `wfl` subcommands.
#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return;
    }

    let cwd = env::current_dir().unwrap_or_else(|_| {
        eprintln!("Error: Could not determine current directory.");
        process::exit(1);
    });

    let result = run_command(&args[1..], &cwd).await;
    if let Err(e) = result {
        eprintln!("{}", e);
        process::exit(1);
    }
}

async fn run_command(args: &[String], cwd: &Path) -> Result<(), wflpkg::PackageError> {
    match args[0].as_str() {
        "create" => {
            let name = parse_create_args(&args[1..]);
            wflpkg::commands::create::create_project(name.as_deref(), cwd)?;
        }
        "add" => {
            wflpkg::commands::add::add_dependency(&args[1..], cwd)?;
        }
        "remove" => {
            if args.len() < 2 {
                return Err(wflpkg::PackageError::General(
                    "Usage: wflpkg remove <package-name>".to_string(),
                ));
            }
            wflpkg::commands::remove::remove_dependency(&args[1], cwd)?;
        }
        "update" => {
            let pkg = if args.len() > 1 {
                Some(args[1].as_str())
            } else {
                None
            };
            wflpkg::commands::update::update_dependencies(pkg, cwd)?;
        }
        "build" => {
            wflpkg::commands::build::build_project(cwd).await?;
        }
        "run" => {
            wflpkg::commands::run::run_project(cwd).await?;
        }
        "share" => {
            wflpkg::commands::share::share_package(cwd).await?;
        }
        "search" => {
            if args.len() < 2 {
                return Err(wflpkg::PackageError::General(
                    "Usage: wflpkg search <query>".to_string(),
                ));
            }
            wflpkg::commands::search::search_packages(&args[1], "wflhub.org").await?;
        }
        "info" => {
            if args.len() < 2 {
                return Err(wflpkg::PackageError::General(
                    "Usage: wflpkg info <package-name>".to_string(),
                ));
            }
            wflpkg::commands::info::show_package_info(&args[1], "wflhub.org").await?;
        }
        "login" => {
            wflpkg::commands::login::login("wflhub.org")?;
        }
        "logout" => {
            wflpkg::commands::login::logout()?;
        }
        "check" => {
            if args.len() >= 2 {
                match args[1].as_str() {
                    "security" => {
                        wflpkg::commands::check::check_security(cwd).await?;
                    }
                    "compatibility" => {
                        wflpkg::commands::check::check_compatibility(cwd).await?;
                    }
                    _ => {
                        return Err(wflpkg::PackageError::General(format!(
                            "Unknown check type: \"{}\"\n\nValid options:\n  wflpkg check security\n  wflpkg check compatibility",
                            args[1]
                        )));
                    }
                }
            } else {
                return Err(wflpkg::PackageError::General(
                    "Usage: wflpkg check <security|compatibility>".to_string(),
                ));
            }
        }
        "help" | "--help" | "-h" => {
            print_help();
        }
        other => {
            return Err(wflpkg::PackageError::General(format!(
                "Unknown command: \"{}\"\n\nRun 'wflpkg help' for a list of commands.",
                other
            )));
        }
    }

    Ok(())
}

/// Parse "create project called <name>" or "create project" from args.
fn parse_create_args(args: &[String]) -> Option<String> {
    // Skip "project" keyword if present
    let args = if !args.is_empty() && args[0] == "project" {
        &args[1..]
    } else {
        args
    };

    // Look for "called <name>"
    if args.len() >= 2 && args[0] == "called" {
        Some(args[1].clone())
    } else if args.len() == 1 && args[0] != "called" {
        // Direct name without "called"
        Some(args[0].clone())
    } else {
        None
    }
}

fn print_help() {
    println!("WFL Package Manager (wflpkg)");
    println!();
    println!("USAGE:");
    println!("    wflpkg <command> [args]");
    println!();
    println!("COMMANDS:");
    println!("    create [project] [called <name>]   Create a new WFL project");
    println!("    add <package> [constraint]          Add a dependency");
    println!("    remove <package>                    Remove a dependency");
    println!("    update [package]                    Update dependencies");
    println!("    build                               Build the project");
    println!("    run                                 Run the project");
    println!("    share                               Share (publish) to the registry");
    println!("    search <query>                      Search for packages");
    println!("    info <package>                      Show package details");
    println!("    login                               Log in to the registry");
    println!("    logout                              Log out from the registry");
    println!("    check security                      Check for security advisories");
    println!("    check compatibility                 Check API compatibility");
    println!("    help                                Show this help message");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(val: &str) -> String {
        val.to_string()
    }

    // --- parse_create_args tests ---

    #[test]
    fn test_parse_create_args_with_called_name() {
        let args = vec![s("project"), s("called"), s("my-app")];
        assert_eq!(parse_create_args(&args), Some(s("my-app")));
    }

    #[test]
    fn test_parse_create_args_direct_name() {
        let args = vec![s("project"), s("my-app")];
        assert_eq!(parse_create_args(&args), Some(s("my-app")));
    }

    #[test]
    fn test_parse_create_args_no_name() {
        let args = vec![s("project")];
        assert_eq!(parse_create_args(&args), None);
    }

    #[test]
    fn test_parse_create_args_empty() {
        let args: Vec<String> = vec![];
        assert_eq!(parse_create_args(&args), None);
    }

    #[test]
    fn test_parse_create_args_called_without_name() {
        let args = vec![s("project"), s("called")];
        assert_eq!(parse_create_args(&args), None);
    }

    // --- run_command tests ---

    #[tokio::test]
    async fn test_run_command_unknown_subcommand() {
        let temp = tempfile::TempDir::new().unwrap();
        let args = vec![s("bogus")];
        let result = run_command(&args, temp.path()).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Unknown command"),
            "expected 'Unknown command', got: {msg}"
        );
    }

    #[tokio::test]
    async fn test_run_command_remove_missing_arg() {
        let temp = tempfile::TempDir::new().unwrap();
        let args = vec![s("remove")];
        let result = run_command(&args, temp.path()).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Usage:"), "expected 'Usage:', got: {msg}");
    }

    #[tokio::test]
    async fn test_run_command_search_missing_arg() {
        let temp = tempfile::TempDir::new().unwrap();
        let args = vec![s("search")];
        let result = run_command(&args, temp.path()).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Usage:"), "expected 'Usage:', got: {msg}");
    }

    #[tokio::test]
    async fn test_run_command_check_invalid_type() {
        let temp = tempfile::TempDir::new().unwrap();
        let args = vec![s("check"), s("bogus")];
        let result = run_command(&args, temp.path()).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Unknown check type"),
            "expected 'Unknown check type', got: {msg}"
        );
    }
}
