use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

pub fn dump_env(output_path: Option<&str>) -> io::Result<()> {
    let mut report = String::new();

    report.push_str("=== WFL Environment Dump ===\n\n");

    // WFL Version
    report.push_str(&format!("WFL Version: {}\n", crate::version::VERSION));

    // Build Rust Version (captured in build.rs)
    report.push_str(&format!(
        "Build Rust Version: {}\n",
        env!("RUSTC_VERSION")
    ));

    // OS and Arch
    report.push_str(&format!("OS: {}\n", env::consts::OS));
    report.push_str(&format!("Architecture: {}\n", env::consts::ARCH));

    // LSP Detection
    if let Ok(exe_path) = env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            let lsp_name = if cfg!(windows) {
                "wfl-lsp.exe"
            } else {
                "wfl-lsp"
            };
            let lsp_path = dir.join(lsp_name);
            let lsp_status = if lsp_path.exists() {
                format!("Detected at {}", lsp_path.display())
            } else {
                "Not detected".to_string()
            };
            report.push_str(&format!("WFL LSP Server: {}\n", lsp_status));
        } else {
             report.push_str("WFL LSP Server: Unable to determine executable directory\n");
        }
    } else {
        report.push_str("WFL LSP Server: Unable to determine executable path\n");
    }

    report.push('\n');

    // Loaded Configuration
    report.push_str("=== Configuration ===\n");
    // We need to load the config to show it.
    // The load_config function takes a path to look for .wflcfg
    // We'll use current dir as a best effort or try to match main.rs logic
    // In main.rs:
    // let script_dir = Path::new(&file_path).parent().unwrap_or(Path::new("."));
    // let config = config::load_config(script_dir);
    // Since we don't have a script file here, we can use current dir.
    let current_dir = env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let config = crate::config::load_config(&current_dir);

    report.push_str(&format!("{:#?}\n", config));
    report.push('\n');

    // Environment Variables
    report.push_str("=== Environment Variables (WFL_*) ===\n");
    for (key, value) in env::vars() {
        if key.starts_with("WFL_") {
            report.push_str(&format!("{}={}\n", key, value));
        }
    }

    // Output
    if let Some(path) = output_path {
        let mut file = fs::File::create(path)?;
        file.write_all(report.as_bytes())?;
        println!("Environment details dumped to {}", path);
    } else {
        println!("{}", report);
    }

    Ok(())
}
