use std::process::Command;

fn main() {
    // Capture rustc version
    let output = Command::new("rustc")
        .arg("--version")
        .output()
        .expect("Failed to execute rustc");

    let version = String::from_utf8(output.stdout)
        .expect("Failed to parse rustc version")
        .trim()
        .to_string();

    println!("cargo:rustc-env=RUSTC_VERSION={}", version);
    println!("cargo:rerun-if-changed=build.rs");
}
