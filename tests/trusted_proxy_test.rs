//! Real binary + HTTP/TLS + configuration boundary coverage for request identity.
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

mod common;

struct Server {
    child: Child,
    directory: tempfile::TempDir,
    url: String,
    client: reqwest::Client,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Server {
    async fn start(proxies: &str, tls: bool) -> Self {
        Self::start_with_handler(
            proxies,
            tls,
            "store identity as [req[\"client_ip\"], req[\"originating_ip\"]]\n        respond to req with stringify_json of identity",
        ).await
    }

    async fn start_with_handler(proxies: &str, tls: bool, handler: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let port = common::free_tcp_port();
        std::fs::write(
            directory.path().join(".wflcfg"),
            format!(
                "web_server_bind_address = 127.0.0.1\nweb_server_trusted_proxies = {proxies}\nexecution_logging = false\nlogging_enabled = false\ndebug_report_enabled = false\n"
            ),
        )
        .unwrap();
        let secured = if tls {
            let certificate = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()]).unwrap();
            std::fs::write(directory.path().join("cert.pem"), certificate.cert.pem()).unwrap();
            std::fs::write(
                directory.path().join("key.pem"),
                certificate.signing_key.serialize_pem(),
            )
            .unwrap();
            " secured with certificate \"cert.pem\" and key \"key.pem\""
        } else {
            ""
        };
        std::fs::write(
            directory.path().join("server.wfl"),
            format!(
                r#"listen on port {port}{secured} as server_handle
main loop:
    wait for request comes in on server_handle as req with timeout 10000
    check if path is equal to "/shutdown":
        respond to req with "closed"
        close server server_handle
        break
    otherwise:
        {handler}
    end check
end loop
"#
            ),
        )
        .unwrap();
        std::fs::write(
            directory.path().join("identity.wfl"),
            "store identity as [client_ip, originating_ip]\ndisplay stringify_json of identity\n",
        )
        .unwrap();
        let log = std::fs::File::create(directory.path().join("process.log")).unwrap();
        let child = Command::new(common::wfl_exe())
            .arg("server.wfl")
            .env(
                "WFL_GLOBAL_CONFIG_PATH",
                directory.path().join("no-global-config"),
            )
            .current_dir(directory.path())
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let mut server = Self {
            child,
            directory,
            url: format!("{}://127.0.0.1:{port}", if tls { "https" } else { "http" }),
            client: reqwest::Client::builder()
                .no_proxy()
                .danger_accept_invalid_certs(tls)
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            assert!(
                server.child.try_wait().unwrap().is_none(),
                "server exited before readiness: {}",
                server.log()
            );
            if tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .is_ok()
            {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "server readiness timeout: {}",
                server.log()
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        server
    }

    fn log(&self) -> String {
        std::fs::read_to_string(self.directory.path().join("process.log")).unwrap()
    }

    async fn identity(&self, headers: &[(&str, &str)]) -> serde_json::Value {
        let mut request = self.client.get(&self.url);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        let response = request.send().await.unwrap();
        assert_eq!(response.status(), 200, "{}", self.log());
        serde_json::from_str(&response.text().await.unwrap()).unwrap()
    }

    async fn close(mut self) {
        let response = self
            .client
            .get(format!("{}/shutdown", self.url))
            .send()
            .await
            .unwrap();
        assert_eq!(response.text().await.unwrap(), "closed");
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success(), "server exit failed: {}", self.log());
                break;
            }
            assert!(
                Instant::now() < deadline,
                "server did not shut down: {}",
                self.log()
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
}

fn assert_identity(identity: &serde_json::Value, origin: &str) {
    assert_eq!(
        identity[0], "127.0.0.1",
        "socket identity must stay unchanged"
    );
    assert_eq!(identity[1], origin, "validated originating identity");
}

#[tokio::test]
async fn default_and_untrusted_peers_ignore_forged_forwarding() {
    for proxies in ["", "10.0.0.0/8"] {
        let server = Server::start(proxies, false).await;
        let identity = server
            .identity(&[("x-forwarded-for", "198.51.100.7")])
            .await;
        server.close().await;
        assert_identity(&identity, "127.0.0.1");
    }
}

#[tokio::test]
async fn trusted_proxy_chain_stops_at_rightmost_untrusted_hop() {
    let server = Server::start("127.0.0.1, 10.0.0.0/8, 2001:db8:1::/48", false).await;
    let cases = [
        ("198.51.100.7", "198.51.100.7"),
        ("198.51.100.7, 10.20.30.40", "198.51.100.7"),
        ("203.0.113.99, 198.51.100.7, 10.20.30.40", "198.51.100.7"),
        ("2001:db8:2::7, 2001:db8:1::10", "2001:db8:2::7"),
        ("198.51.100.7, ::ffff:10.20.30.40", "198.51.100.7"),
        (" 198.51.100.7 ,\t10.20.30.40 ", "198.51.100.7"),
    ];
    let mut identities = Vec::new();
    for (forwarded, expected) in cases {
        identities.push((
            server.identity(&[("x-forwarded-for", forwarded)]).await,
            expected,
        ));
    }
    server.close().await;
    for (identity, expected) in identities {
        assert_identity(&identity, expected);
    }
}

#[tokio::test]
async fn malformed_ambiguous_and_oversized_forwarding_falls_back_to_peer() {
    let server = Server::start("127.0.0.1", false).await;
    let mut cases = vec![
        "".into(),
        "unknown".into(),
        "198.51.100.7,".into(),
        ",198.51.100.7".into(),
        "spoofed,198.51.100.7".into(),
        "198.51.100.7:1234".into(),
        "[2001:db8::7]".into(),
        "fe80::1%eth0".into(),
        "\"198.51.100.7\"".into(),
    ];
    cases.push(
        std::iter::repeat_n("198.51.100.7", 33)
            .collect::<Vec<_>>()
            .join(","),
    );
    cases.push(format!("198.51.100.7,{}127.0.0.1", " ".repeat(4096)));
    let mut identities = Vec::new();
    for value in &cases {
        identities.push(server.identity(&[("x-forwarded-for", value)]).await);
    }
    identities.push(
        server
            .identity(&[
                ("x-forwarded-for", "198.51.100.7"),
                ("x-forwarded-for", "203.0.113.9"),
            ])
            .await,
    );
    identities.push(
        server
            .identity(&[
                ("forwarded", "for=198.51.100.7"),
                ("x-real-ip", "198.51.100.7"),
            ])
            .await,
    );
    identities.push(server.identity(&[]).await);
    server.close().await;
    for identity in identities {
        assert_identity(&identity, "127.0.0.1");
    }
}

#[tokio::test]
async fn invalid_proxy_configuration_clears_trust_atomically() {
    for proxies in [
        "127.0.0.1,not-an-address",
        "127.0.0.1/33",
        "127.0.0.1,",
        "127.0.0.1\nweb_server_trusted_proxies = invalid",
    ] {
        let server = Server::start(proxies, false).await;
        let identity = server
            .identity(&[("x-forwarded-for", "198.51.100.7")])
            .await;
        server.close().await;
        assert_identity(&identity, "127.0.0.1");
    }
}

#[tokio::test]
async fn trusted_proxy_identity_reaches_tls_requests() {
    let server = Server::start("127.0.0.0/8", true).await;
    let identity = server
        .identity(&[("x-forwarded-for", "198.51.100.7")])
        .await;
    server.close().await;
    assert_identity(&identity, "198.51.100.7");
}

#[tokio::test]
async fn originating_identity_is_refreshed_and_passed_to_executed_files() {
    for handler in [
        "store identity as [client_ip, originating_ip]\n        respond to req with stringify_json of identity",
        "execute wfl file at \"identity.wfl\" with req and read output as identity\n        respond to req with identity",
    ] {
        let server = Server::start_with_handler("127.0.0.1", false, handler).await;
        let forwarded = server
            .identity(&[("x-forwarded-for", "198.51.100.7")])
            .await;
        let direct = server.identity(&[]).await;
        server.close().await;
        assert_identity(&forwarded, "198.51.100.7");
        assert_identity(&direct, "127.0.0.1");
    }
}

#[test]
fn config_checker_preserves_and_validates_trusted_proxy_policy() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(".wflcfg");
    let checker = wfl::wfl_config::checker::ConfigChecker::new();
    let valid = "web_server_trusted_proxies = 127.0.0.1, 2001:db8::/32\n";
    std::fs::write(&path, valid).unwrap();
    assert!(
        checker.check_config_file(&path).unwrap().is_empty(),
        "proxy policy must be recognized by --configCheck"
    );
    checker.fix_config_file(&path).unwrap();
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains(valid.trim()),
        "--configFix must preserve a valid trusted-proxy policy"
    );
    for value in ["127.0.0.1,invalid", "127.0.0.1/33", "::1/129", "127.0.0.1,"] {
        std::fs::write(&path, format!("web_server_trusted_proxies = {value}\n")).unwrap();
        let issues = checker.check_config_file(&path).unwrap();
        assert!(
            issues
                .iter()
                .any(|issue| issue.kind == wfl::wfl_config::checker::ConfigIssueKind::InvalidValue),
            "invalid proxy policy must be diagnosed: {value}"
        );
    }
}
