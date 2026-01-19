use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn setup_test_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

fn create_ssl_dir(base_dir: &std::path::Path) -> PathBuf {
    let ssl_dir = base_dir.join(".ssl");
    fs::create_dir_all(&ssl_dir).expect("Failed to create .ssl directory");
    ssl_dir
}

fn generate_test_certificates(ssl_dir: &std::path::Path) {
    // Generate self-signed certificate for testing
    let cert_output = Command::new("openssl")
        .args(&[
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            ssl_dir.join("key.pem").to_str().unwrap(),
            "-out",
            ssl_dir.join("cert.pem").to_str().unwrap(),
            "-days",
            "1",
            "-subj",
            "/CN=localhost",
        ])
        .output();

    if cert_output.is_err() || !cert_output.unwrap().status.success() {
        panic!("Failed to generate test certificates. Make sure OpenSSL is installed.");
    }
}

fn wfl_binary_path() -> PathBuf {
    // Assuming the binary is in target/release/wfl or target/debug/wfl
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("release");
    path.push("wfl");

    if !path.exists() {
        path.pop();
        path.pop();
        path.push("debug");
        path.push("wfl");
    }

    #[cfg(windows)]
    {
        path.set_extension("exe");
    }

    path
}

#[test]
#[ignore] // Requires OpenSSL and full build
fn test_https_server_with_valid_certificates() {
    let temp_dir = setup_test_dir();
    let ssl_dir = create_ssl_dir(temp_dir.path());
    generate_test_certificates(&ssl_dir);

    // Create a simple HTTPS server script
    let script_path = temp_dir.path().join("https_server.wfl");
    let script_content = r#"
listen on secure port 8443 as https_server
print "HTTPS server started successfully"
"#;
    fs::write(&script_path, script_content).expect("Failed to write test script");

    // Run the script (it should start without errors)
    let output = Command::new(wfl_binary_path())
        .arg(&script_path)
        .current_dir(temp_dir.path())
        .env("WFL_TEST_MODE", "1")
        .output()
        .expect("Failed to execute WFL");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT: {}", stdout);
    println!("STDERR: {}", stderr);

    assert!(
        stdout.contains("HTTPS server started successfully")
            || stdout.contains("Secure server is listening"),
        "Expected HTTPS server to start successfully"
    );
}

#[test]
#[ignore] // Requires full build
fn test_missing_cert_pem_error() {
    let temp_dir = setup_test_dir();
    let ssl_dir = create_ssl_dir(temp_dir.path());

    // Only create key.pem, not cert.pem
    generate_test_certificates(&ssl_dir);
    fs::remove_file(ssl_dir.join("cert.pem")).expect("Failed to remove cert.pem");

    let script_path = temp_dir.path().join("https_server.wfl");
    let script_content = r#"
listen on secure port 8443 as https_server
"#;
    fs::write(&script_path, script_content).expect("Failed to write test script");

    let output = Command::new(wfl_binary_path())
        .arg(&script_path)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute WFL");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stderr.contains("HTTPS certificate not found") || stderr.contains("cert.pem"),
        "Expected error about missing certificate. Got: {}",
        stderr
    );
}

#[test]
#[ignore] // Requires full build
fn test_missing_key_pem_error() {
    let temp_dir = setup_test_dir();
    let ssl_dir = create_ssl_dir(temp_dir.path());

    // Only create cert.pem, not key.pem
    generate_test_certificates(&ssl_dir);
    fs::remove_file(ssl_dir.join("key.pem")).expect("Failed to remove key.pem");

    let script_path = temp_dir.path().join("https_server.wfl");
    let script_content = r#"
listen on secure port 8443 as https_server
"#;
    fs::write(&script_path, script_content).expect("Failed to write test script");

    let output = Command::new(wfl_binary_path())
        .arg(&script_path)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute WFL");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stderr.contains("HTTPS private key not found") || stderr.contains("key.pem"),
        "Expected error about missing private key. Got: {}",
        stderr
    );
}

#[test]
#[ignore] // Requires full build
fn test_invalid_certificate_error() {
    let temp_dir = setup_test_dir();
    let ssl_dir = create_ssl_dir(temp_dir.path());

    // Create invalid certificate files
    fs::write(ssl_dir.join("cert.pem"), "invalid certificate content")
        .expect("Failed to write invalid cert");
    fs::write(ssl_dir.join("key.pem"), "invalid key content").expect("Failed to write invalid key");

    let script_path = temp_dir.path().join("https_server.wfl");
    let script_content = r#"
listen on secure port 8443 as https_server
"#;
    fs::write(&script_path, script_content).expect("Failed to write test script");

    let output = Command::new(wfl_binary_path())
        .arg(&script_path)
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute WFL");

    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        stderr.contains("Failed to parse") || stderr.contains("invalid") || stderr.contains("TLS"),
        "Expected error about invalid certificate. Got: {}",
        stderr
    );
}

#[test]
#[ignore] // Requires OpenSSL, full build, and network testing
fn test_http_and_https_coexist() {
    let temp_dir = setup_test_dir();
    let ssl_dir = create_ssl_dir(temp_dir.path());
    generate_test_certificates(&ssl_dir);

    // Create a script that runs both HTTP and HTTPS servers
    let script_path = temp_dir.path().join("both_servers.wfl");
    let script_content = r#"
listen on port 8080 as http_server
listen on secure port 8443 as https_server
print "Both servers started"
"#;
    fs::write(&script_path, script_content).expect("Failed to write test script");

    let output = Command::new(wfl_binary_path())
        .arg(&script_path)
        .current_dir(temp_dir.path())
        .env("WFL_TEST_MODE", "1")
        .output()
        .expect("Failed to execute WFL");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    println!("STDOUT: {}", stdout);
    println!("STDERR: {}", stderr);

    // Both servers should start successfully
    assert!(
        (stdout.contains("Server is listening") || stdout.contains("Secure server is listening"))
            && stdout.contains("Both servers started"),
        "Expected both HTTP and HTTPS servers to start"
    );
}

#[test]
fn test_secure_keyword_parsing() {
    // This is a lightweight test that doesn't require certificates
    // It just verifies that the "secure" keyword is recognized
    let temp_dir = setup_test_dir();
    let script_path = temp_dir.path().join("parse_test.wfl");

    // This should parse successfully even without certs (will fail at runtime)
    let script_content = r#"
check if true:
    listen on secure port 8443 as https_server
end check
"#;
    fs::write(&script_path, script_content).expect("Failed to write test script");

    let output = Command::new(wfl_binary_path())
        .arg("--parse")
        .arg(&script_path)
        .output()
        .expect("Failed to execute WFL");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not have parse errors
    assert!(
        !stderr.contains("parse error") && !stderr.contains("Expected 'port'"),
        "Should parse 'secure' keyword without errors. Got: {}",
        stderr
    );
}
