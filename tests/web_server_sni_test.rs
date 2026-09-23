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
            "listen on port 0 secured with {operand} as server"
        ));
        assert!(TypeChecker::new().check_types(&ast).is_err(), "{operand}");
    }
    for operand in [
        "certificate missing_cert and key \"k\" for \"one.test\"",
        "certificate \"c\" and key missing_key for \"one.test\"",
        "certificate \"c\" and key \"k\" for missing_host",
    ] {
        let ast = parse(&format!(
            "listen on port 0 secured with {operand} as server"
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
    assert!(connect(addr, "unknown.test", &client).await.is_err());
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
