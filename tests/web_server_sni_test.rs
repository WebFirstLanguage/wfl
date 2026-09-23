//! SNI contract tests: real certificates, verified TLS, and the real WFL binary.
use rustls::pki_types::{CertificateDer, ServerName};
use std::{net::SocketAddr, path::Path, process::Stdio, sync::Arc, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use wfl::{
    Interpreter, analyzer::Analyzer, lexer::lex_wfl_with_positions, parser::Parser,
    typechecker::TypeChecker,
};

fn parse(code: &str) -> wfl::parser::ast::Program {
    Parser::new(&lex_wfl_with_positions(code))
        .parse()
        .expect("SNI syntax must parse")
}

struct Cert {
    cert: String,
    key: String,
    der: CertificateDer<'static>,
}

impl Cert {
    fn new(dir: &Path, name: &str) -> Self {
        let generated = rcgen::generate_simple_self_signed(vec![name.to_owned()]).unwrap();
        let cert = dir.join(format!("{name}.pem"));
        let key = dir.join(format!("{name}.key"));
        std::fs::write(&cert, generated.cert.pem()).unwrap();
        std::fs::write(&key, generated.signing_key.serialize_pem()).unwrap();
        Self {
            cert: cert.to_string_lossy().replace('\\', "/"),
            key: key.to_string_lossy().replace('\\', "/"),
            der: generated.cert.der().clone(),
        }
    }
    fn clause(&self, name: &str) -> String {
        format!(
            r#"certificate "{}" and key "{}" for "{name}""#,
            self.cert, self.key
        )
    }
}

fn connector(certs: &[&Cert], sni: bool) -> tokio_rustls::TlsConnector {
    let mut roots = rustls::RootCertStore::empty();
    for cert in certs {
        roots.add(cert.der.clone()).unwrap();
    }
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth();
    config.enable_sni = sni;
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    tokio_rustls::TlsConnector::from(Arc::new(config))
}

async fn connect(
    addr: SocketAddr,
    name: &str,
    connector: &tokio_rustls::TlsConnector,
) -> std::io::Result<tokio_rustls::client::TlsStream<tokio::net::TcpStream>> {
    let tcp = tokio::net::TcpStream::connect(addr).await?;
    tokio::time::timeout(
        Duration::from_secs(3),
        connector.connect(ServerName::try_from(name.to_owned()).unwrap(), tcp),
    )
    .await
    .expect("TLS handshake must be bounded")
}

async fn start_binary(
    dir: &Path,
    clauses: &str,
    requests: usize,
) -> (tokio::process::Child, SocketAddr) {
    let mut code = format!(
        "listen on port 0 secured with {clauses} as secure_server\ndisplay secure_server\n"
    );
    for _ in 0..requests {
        code.push_str("wait for request comes in on secure_server as req with timeout 5000\nrespond to req with header \"Host\" of req\n");
    }
    code.push_str("close server secure_server\n");
    std::fs::write(dir.join("server.wfl"), code).unwrap();
    std::fs::write(dir.join(".wflcfg"), "web_server_bind_address = 127.0.0.1\n").unwrap();
    let mut child = tokio::process::Command::new(env!("CARGO_BIN_EXE_wfl"))
        .args(["--execution-timeout", "15", "server.wfl"])
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let addr = tokio::time::timeout(Duration::from_secs(10), async {
        while let Some(line) = lines.next_line().await.unwrap() {
            if let Some(addr) = line.trim().strip_prefix("WebServer::") {
                return addr.parse().unwrap();
            }
        }
        panic!("WFL exited before publishing the listener address");
    })
    .await
    .expect("WFL startup must be bounded");
    (child, addr)
}

async fn request(
    addr: SocketAddr,
    name: &str,
    cert: &Cert,
    connector: &tokio_rustls::TlsConnector,
) {
    let mut stream = connect(addr, name, connector)
        .await
        .expect("verified SNI handshake");
    assert_eq!(
        stream.get_ref().1.peer_certificates().unwrap()[0],
        cert.der,
        "SNI must choose this domain's certificate"
    );
    stream
        .write_all(
            format!("GET / HTTP/1.1\r\nHost: {name}\r\nConnection: close\r\n\r\n").as_bytes(),
        )
        .await
        .unwrap();
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(3), stream.read_to_string(&mut response))
        .await
        .unwrap()
        .unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(response.ends_with(name), "{response}");
}

#[test]
fn sni_syntax_accepts_named_certificates_and_variables() {
    let ast = parse(
        r#"
store hostname as "one.test"
store cert_path as "one.pem"
store key_path as "one.key"
store listen_port as 0
listen on port listen_port secured with certificate cert_path and key key_path for hostname
    and certificate "two.pem" and key "two.key" for "two.test" as secure_server
close server secure_server
"#,
    );
    Analyzer::new()
        .analyze(&ast)
        .expect("SNI expressions must be analyzed");
    TypeChecker::new()
        .check_types(&ast)
        .expect("SNI text operands must typecheck");
}

#[test]
fn sni_operands_are_analyzed_and_typechecked() {
    for operand in [
        "certificate 42 and key \"k\" for \"one.test\"",
        "certificate \"c\" and key 42 for \"one.test\"",
        "certificate \"c\" and key \"k\" for 42",
    ] {
        let ast = parse(&format!(
            "listen on port 0 secured with {operand} as secure_server"
        ));
        assert!(TypeChecker::new().check_types(&ast).is_err(), "{operand}");
    }
    for operand in [
        "certificate missing_cert and key \"k\" for \"one.test\"",
        "certificate \"c\" and key missing_key for \"one.test\"",
        "certificate \"c\" and key \"k\" for missing_host",
    ] {
        let ast = parse(&format!(
            "listen on port 0 secured with {operand} as secure_server"
        ));
        assert!(Analyzer::new().analyze(&ast).is_err(), "{operand}");
    }
}

#[tokio::test]
async fn sni_binary_selects_two_certs_rejects_unknown_and_survives_stalled_clients() {
    let dir = tempfile::tempdir().unwrap();
    let one = Cert::new(dir.path(), "one.test");
    let two = Cert::new(dir.path(), "two.test");
    let (mut child, addr) = start_binary(
        dir.path(),
        &format!("{} and {}", one.clause("ONE.test"), two.clause("two.test")),
        2,
    )
    .await;
    let client = connector(&[&one, &two], true);
    let mut stalled = tokio::net::TcpStream::connect(addr).await.unwrap();
    // Unknown/missing SNI fails before any HTTP request reaches the application.
    let rejected = connect(addr, "unknown.test", &client).await.unwrap_err();
    assert!(
        matches!(
            rejected
                .get_ref()
                .and_then(|e| e.downcast_ref::<rustls::Error>()),
            Some(rustls::Error::AlertReceived(_))
        ),
        "The server must reject unknown SNI, not merely present a mismatched certificate: {rejected}"
    );
    assert!(
        connect(addr, "one.test", &connector(&[&one], false))
            .await
            .is_err()
    );
    let mut idle = connect(addr, "one.test", &client).await.unwrap();
    let ((), ()) = tokio::join!(
        request(addr, "one.test", &one, &client),
        request(addr, "two.test", &two, &client)
    );
    assert!(
        tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    let mut byte = [0];
    for result in [
        tokio::time::timeout(Duration::from_secs(2), stalled.read(&mut byte)).await,
        tokio::time::timeout(Duration::from_secs(2), idle.read(&mut byte)).await,
    ] {
        assert!(
            matches!(result, Ok(Ok(0)) | Ok(Err(_))),
            "close must cancel established and partial TLS connections: {result:?}"
        );
    }
    assert!(tokio::net::TcpStream::connect(addr).await.is_err());
}

#[tokio::test]
async fn sni_explicit_default_handles_unknown_and_missing_sni() {
    let dir = tempfile::tempdir().unwrap();
    let default = Cert::new(dir.path(), "default.test");
    let named = Cert::new(dir.path(), "named.test");
    let clauses = format!(
        r#"certificate "{}" and key "{}" and {}"#,
        default.cert,
        default.key,
        named.clause("named.test")
    );
    let (mut child, addr) = start_binary(dir.path(), &clauses, 3).await;
    let client = connector(&[&default, &named], true);
    request(addr, "default.test", &default, &client).await;
    request(addr, "named.test", &named, &client).await;
    request(
        addr,
        "default.test",
        &default,
        &connector(&[&default], false),
    )
    .await;
    assert!(
        tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
}

#[tokio::test]
async fn sni_invalid_configuration_fails_before_binding() {
    let dir = tempfile::tempdir().unwrap();
    let one = Cert::new(dir.path(), "one.test");
    let two = Cert::new(dir.path(), "two.test");
    let cases = [
        (
            format!("{} and {}", one.clause("one.test"), one.clause("ONE.TEST")),
            "Duplicate",
        ),
        (one.clause("wrong.test"), "one.test.pem"),
        (one.clause("*.test"), "DNS"),
        (one.clause("one.test:443"), "DNS"),
        (one.clause("one.test."), "DNS"),
        (one.clause("127.0.0.1"), "DNS"),
        (one.clause("https://one.test"), "DNS"),
        (one.clause(""), "DNS"),
        (one.clause(&format!("{}.test", "a".repeat(64))), "DNS"),
        (
            format!(
                r#"certificate "{}" and key "{}" for "one.test""#,
                one.cert, two.key
            ),
            "not a valid pair",
        ),
        (
            format!(
                r#"certificate "missing.pem" and key "{}" for "one.test""#,
                one.key
            ),
            "Cannot open",
        ),
    ];
    for (clause, expected) in cases {
        let ast = parse(&format!(
            "listen on port 0 secured with {clause} as secure_server"
        ));
        let mut interpreter = Interpreter::new();
        let error = format!(
            "{:?}",
            interpreter
                .interpret(&ast)
                .await
                .expect_err("invalid SNI configuration must fail")
        );
        assert!(error.contains(expected), "expected {expected}: {error}");
        assert!(
            interpreter
                .global_env()
                .borrow()
                .get("secure_server")
                .is_none(),
            "invalid listener must not be published"
        );
    }
}

#[test]
fn sni_parser_rejects_incomplete_ambiguous_and_oversized_lists() {
    for clause in [
        r#"certificate "c" and key "k" for"#,
        r#"certificate "c" and key "k" for "one.test" and"#,
        r#"certificate "c" and key "k" for "one.test" and certificate "c" for "two.test""#,
        r#"certificate "c" and key "k" for "one.test" and certificate "d" and key "e""#,
        r#"certificate "c" and key "k" and certificate "d" and key "e""#,
    ] {
        let code = format!("listen on port 0 secured with {clause} as secure_server");
        assert!(
            Parser::new(&lex_wfl_with_positions(&code)).parse().is_err(),
            "{code}"
        );
    }
    let clauses = (0..129)
        .map(|i| format!(r#"certificate "c" and key "k" for "host{i}.test""#))
        .collect::<Vec<_>>();
    parse(&format!(
        "listen on port 0 secured with {} as secure_server",
        clauses[..128].join(" and ")
    ));
    let code = format!(
        "listen on port 0 secured with {} as secure_server",
        clauses.join(" and ")
    );
    let error = Parser::new(&lex_wfl_with_positions(&code))
        .parse()
        .unwrap_err();
    assert!(format!("{error:?}").contains("at most 128"));
}

#[tokio::test]
async fn sni_runtime_checks_dynamic_operands_and_ignores_implicit_default() {
    for operand in [
        "certificate 42 and key \"k\" for \"one.test\"",
        "certificate \"c\" and key 42 for \"one.test\"",
        "certificate \"c\" and key \"k\" for 42",
    ] {
        let ast = parse(&format!(
            "listen on port 0 secured with {operand} as secure_server"
        ));
        let error = Interpreter::new().interpret(&ast).await.unwrap_err();
        assert!(format!("{error:?}").contains("Expected text for TLS"));
    }
    let dir = tempfile::tempdir().unwrap();
    let one = Cert::new(dir.path(), "one.test");
    let config = wfl::config::WflConfig {
        web_server_tls_cert_file: Some("missing-default.pem".to_owned()),
        web_server_tls_key_file: Some("missing-default.key".to_owned()),
        ..Default::default()
    };
    let mut interpreter = Interpreter::with_config(Arc::new(config));
    interpreter
        .interpret(&parse(&format!(
            "listen on port 0 secured with {} as secure_server\nclose server secure_server",
            one.clause("one.test")
        )))
        .await
        .unwrap();
}

#[tokio::test]
async fn sni_evaluates_operands_once_in_source_order() {
    let dir = tempfile::tempdir().unwrap();
    let cert = Cert::new(dir.path(), "one.test");
    let source = format!(
        r#"
store evaluation_order as ""
define action called choose_certificate:
    change evaluation_order to evaluation_order with "cert;"
    return "{}"
end action
define action called choose_key:
    change evaluation_order to evaluation_order with "key;"
    return "{}"
end action
define action called choose_hostname:
    change evaluation_order to evaluation_order with "host;"
    return "one.test"
end action
listen on port 0 secured with certificate choose_certificate and key choose_key for choose_hostname as secure_server
close server secure_server
"#,
        cert.cert, cert.key
    );
    let mut interpreter = Interpreter::new();
    interpreter.interpret(&parse(&source)).await.unwrap();
    let order = interpreter
        .global_env()
        .borrow()
        .get("evaluation_order")
        .unwrap();
    let wfl::interpreter::value::Value::Text(order) = order else {
        panic!("order must be text")
    };
    assert_eq!(order.as_ref(), "cert;key;host;");

    let invalid = source.replace(&format!("return \"{}\"", cert.cert), "return 42");
    let mut interpreter = Interpreter::new();
    let error = interpreter.interpret(&parse(&invalid)).await.unwrap_err();
    assert!(format!("{error:?}").contains("Expected text for TLS certificate path"));
    let order = interpreter
        .global_env()
        .borrow()
        .get("evaluation_order")
        .unwrap();
    let wfl::interpreter::value::Value::Text(order) = order else {
        panic!("order must be text")
    };
    assert_eq!(
        order.as_ref(),
        "cert;",
        "later operands must not run after an invalid certificate path"
    );
}

#[tokio::test]
async fn sni_cli_analyzer_counts_certificate_and_hostname_variables_as_used() {
    let dir = tempfile::tempdir().unwrap();
    let code = r#"
store default_cert as "default.pem"
store default_key as "default.key"
store named_cert as "one.pem"
store named_key as "one.key"
store hostname as "one.test"
listen on port 0 secured with certificate default_cert and key default_key
    and certificate named_cert and key named_key for hostname as secure_server
close server secure_server
"#;
    std::fs::write(dir.path().join("analyze.wfl"), code).unwrap();
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        tokio::process::Command::new(env!("CARGO_BIN_EXE_wfl"))
            .args(["--analyze", "analyze.wfl"])
            .current_dir(dir.path())
            .env(
                "WFL_GLOBAL_CONFIG_PATH",
                dir.path().join("absent-global-config"),
            )
            .env("NO_COLOR", "1")
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .unwrap()
    .unwrap();
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(
        result.status.success(),
        "--analyze must accept used TLS operands: {output}"
    );
    assert!(!output.contains("ANALYZE-UNUSED"), "{output}");
}

#[tokio::test]
async fn sni_incompatible_named_key_does_not_select_compatible_default() {
    let dir = tempfile::tempdir().unwrap();
    let named = Cert::new(dir.path(), "one.test");
    let fallback_key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ED25519).unwrap();
    let fallback_cert = rcgen::CertificateParams::new(vec!["one.test".to_owned()])
        .unwrap()
        .self_signed(&fallback_key)
        .unwrap();
    let fallback = Cert {
        cert: dir
            .path()
            .join("fallback.pem")
            .to_string_lossy()
            .replace('\\', "/"),
        key: dir
            .path()
            .join("fallback.key")
            .to_string_lossy()
            .replace('\\', "/"),
        der: fallback_cert.der().clone(),
    };
    std::fs::write(&fallback.cert, fallback_cert.pem()).unwrap();
    std::fs::write(&fallback.key, fallback_key.serialize_pem()).unwrap();
    let clauses = format!(
        r#"certificate "{}" and key "{}" and {}"#,
        fallback.cert,
        fallback.key,
        named.clause("one.test")
    );
    let (mut child, addr) = start_binary(dir.path(), &clauses, 2).await;
    let mut roots = rustls::RootCertStore::empty();
    roots.add(named.der.clone()).unwrap();
    roots.add(fallback.der.clone()).unwrap();
    let mut provider = rustls::crypto::ring::default_provider();
    let mapping = provider.signature_verification_algorithms.mapping;
    let index = mapping
        .iter()
        .position(|(scheme, _)| *scheme == rustls::SignatureScheme::ED25519)
        .unwrap();
    provider.signature_verification_algorithms.mapping = &mapping[index..index + 1];
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(provider))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let restricted = tokio_rustls::TlsConnector::from(Arc::new(config.clone()));
    let rejected = connect(addr, "one.test", &restricted).await.unwrap_err();
    assert!(
        matches!(
            rejected
                .get_ref()
                .and_then(|e| e.downcast_ref::<rustls::Error>()),
            Some(rustls::Error::AlertReceived(_))
        ),
        "a known incompatible named key must fail, not fall back: {rejected}"
    );
    let mut no_sni = config;
    no_sni.enable_sni = false;
    request(
        addr,
        "one.test",
        &fallback,
        &tokio_rustls::TlsConnector::from(Arc::new(no_sni)),
    )
    .await;
    request(
        addr,
        "one.test",
        &named,
        &connector(&[&named, &fallback], true),
    )
    .await;
    assert!(
        tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
}
