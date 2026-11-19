//! Ordered DNS resolution through `.wflcfg` (PRD:
//! `Engineering/designs/2026-11-18-ordered-dns-resolution-design.md`).
//!
//! The Linux release artifact is a static musl binary, and musl asks *every*
//! nameserver at once and takes the first reply. On a network whose resolver
//! list is `10.0.0.53, 10.0.0.54, 8.8.8.8`, Google's "no such name" often
//! wins the race and internal names fail. These tests pin the replacement: WFL
//! asks the servers it is given one at a time, in order, with glibc's rules,
//! and a developer can list those servers in `.wflcfg`.
//!
//! Every test configures WFL the way a user would — a real `.wflcfg` file in
//! the program's directory — and talks to real DNS servers on `127.0.0.1`
//! (`dns_fake_server`). Nothing here reaches the public internet once the
//! feature exists: with `dns_servers` set, WFL asks only those servers.

use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tempfile::TempDir;
use wfl::interpreter::Interpreter;
use wfl::interpreter::dns::{DnsSettings, OrderedResolver, SystemSources, plan};
use wfl::lexer::lex_wfl_with_positions;
use wfl::parser::Parser;

use crate::dns_fake_server::{
    FakeDns, FakeHttp, RCODE_FORMERR, RCODE_NOTIMP, RCODE_REFUSED, RCODE_SERVFAIL, Reply, TYPE_A,
    TYPE_AAAA, Transport,
};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn server_list(servers: &[&FakeDns]) -> String {
    servers
        .iter()
        .map(|s| s.addr.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse(code: &str) -> wfl::parser::ast::Program {
    let tokens = lex_wfl_with_positions(code);
    Parser::new(&tokens)
        .parse()
        .unwrap_or_else(|e| panic!("Parse error: {e:?}"))
}

/// Run `program` with `cfg` as the `.wflcfg` next to it, loaded through the
/// same `load_config` the CLI uses. Returns the value of `body` on success, or
/// every runtime error message joined on failure.
async fn run_with_config(cfg: &str, program: &str) -> Result<String, String> {
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(dir.path().join(".wflcfg"), cfg).expect("write .wflcfg");
    let config = wfl::config::load_config(dir.path());
    let mut interpreter = Interpreter::with_config(Arc::new(config));
    match interpreter.interpret(&parse(program)).await {
        Ok(_) => match interpreter.global_env().borrow().get("body") {
            Some(wfl::interpreter::value::Value::Text(t)) => Ok(t.to_string()),
            other => Ok(format!("{other:?}")),
        },
        Err(errors) => Err(errors
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
            .join("; ")),
    }
}

/// Fetch `url` with `cfg` as the program's `.wflcfg`.
async fn fetch(cfg: &str, url: &str) -> Result<String, String> {
    run_with_config(
        cfg,
        &format!("open url at \"{url}\" and read content as body\n"),
    )
    .await
}

fn wfl_binary() -> &'static str {
    crate::common::wfl_exe()
}

/// Run the real `wfl` binary with `args`, inside `dir`, with the global config
/// pointed at `global` (or at a path that does not exist).
fn run_binary(dir: &Path, args: &[&str], global: Option<&Path>) -> (String, Option<i32>) {
    let missing_global = dir.join("no-global-config.cfg");
    let output = Command::new(wfl_binary())
        .args(args)
        .current_dir(dir)
        .env("WFL_GLOBAL_CONFIG_PATH", global.unwrap_or(&missing_global))
        .output()
        .expect("run wfl");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (combined, output.status.code())
}

// ---------------------------------------------------------------------------
// FR-01: one server at a time, in order
// ---------------------------------------------------------------------------

/// FR-01 AC1/AC3: the first listed server's answer is used every time, even
/// though the second server always answers "no such name" much sooner. This is
/// exactly the musl race (the second server plays `8.8.8.8`).
#[tokio::test]
async fn first_listed_server_wins_even_when_a_later_server_answers_first() {
    let http = FakeHttp::start("ok").await;
    let internal = FakeDns::start(|_| Reply::delayed(100, Reply::loopback())).await;
    let public = FakeDns::start(|_| Reply::NxDomain).await;
    let cfg = format!("dns_servers = {}\n", server_list(&[&internal, &public]));

    for run in 0..100 {
        let url = format!("http://order{run}.wfl.test:{}/", http.port);
        let body = fetch(&cfg, &url).await;
        assert_eq!(body.as_deref(), Ok("ok"), "run {run}: {body:?}");
    }

    assert_eq!(http.request_count(), 100);
    assert_eq!(
        public.count_matching(|name| name.starts_with("order")),
        0,
        "the second server must never be asked while the first one gives a final answer"
    );
    assert!(internal.count_matching(|name| name.starts_with("order")) >= 100);
}

/// The same race, end to end through the real `wfl` binary.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_binary_uses_first_listed_server() {
    let http = FakeHttp::start("ok").await;
    let internal = FakeDns::start(|_| Reply::delayed(100, Reply::loopback())).await;
    let public = FakeDns::start(|_| Reply::NxDomain).await;

    for run in 0..10 {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join(".wflcfg"),
            format!("dns_servers = {}\n", server_list(&[&internal, &public])),
        )
        .unwrap();
        std::fs::write(
            dir.path().join("main.wfl"),
            format!(
                "open url at \"http://e2e{run}.wfl.test:{}/\" and read content as body\ndisplay \"BODY=\" with body\n",
                http.port
            ),
        )
        .unwrap();
        let path = dir.path().to_path_buf();
        let (out, code) =
            tokio::task::spawn_blocking(move || run_binary(&path, &["main.wfl"], None))
                .await
                .unwrap();
        assert!(out.contains("BODY=ok"), "run {run}: exit {code:?}\n{out}");
        assert_eq!(code, Some(0), "run {run}:\n{out}");
    }
    assert_eq!(public.count_matching(|name| name.starts_with("e2e")), 0);
}

// ---------------------------------------------------------------------------
// FR-02: glibc failover rules
// ---------------------------------------------------------------------------

/// FR-02 AC1: "no such name" from the first server is final.
#[tokio::test]
async fn no_such_name_from_first_server_is_final() {
    let first = FakeDns::start(|_| Reply::NxDomain).await;
    let second = FakeDns::start(|_| Reply::loopback()).await;
    let cfg = format!("dns_servers = {}\n", server_list(&[&first, &second]));

    let result = fetch(&cfg, "http://nx.wfl.test:9/").await;
    let message = result.expect_err("a final 'no such name' must fail the lookup");
    assert!(
        message.contains("Could not find the address of \"nx.wfl.test\""),
        "{message}"
    );
    assert!(message.contains("no such name"), "{message}");
    assert!(
        second.queries_for("nx.wfl.test").is_empty(),
        "a final answer must stop the search"
    );
}

/// FR-02 AC2 + FR-06 AC4: an empty NOERROR for both families is final.
#[tokio::test]
async fn empty_answer_for_both_families_is_final() {
    let first = FakeDns::start(|_| Reply::NoData).await;
    let second = FakeDns::start(|_| Reply::loopback()).await;
    let cfg = format!("dns_servers = {}\n", server_list(&[&first, &second]));

    let message = fetch(&cfg, "http://empty.wfl.test:9/")
        .await
        .expect_err("no addresses must fail the lookup");
    assert!(message.contains("no addresses"), "{message}");
    assert!(second.queries_for("empty.wfl.test").is_empty());
}

/// FR-02 AC3 + FR-09 AC1: a silent first server is waited for, for exactly the
/// `dns_timeout_seconds` budget, and then the next server answers.
#[tokio::test]
async fn silent_first_server_falls_through_after_the_configured_timeout() {
    let http = FakeHttp::start("ok").await;
    let silent = FakeDns::start(|_| Reply::Silent).await;
    let backup = FakeDns::start(|_| Reply::loopback()).await;
    let cfg = format!(
        "dns_servers = {}\ndns_timeout_seconds = 1\n",
        server_list(&[&silent, &backup])
    );

    let started = Instant::now();
    let body = fetch(&cfg, &format!("http://slow.wfl.test:{}/", http.port)).await;
    let elapsed = started.elapsed();

    assert_eq!(body.as_deref(), Ok("ok"), "{body:?}");
    assert!(
        silent.query_count() >= 1,
        "the first server must be asked before the second"
    );
    assert!(
        elapsed >= Duration::from_millis(900),
        "the first server was not waited for: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(4),
        "dns_timeout_seconds = 1 was not honored: {elapsed:?}"
    );
}

/// FR-02 AC4: SERVFAIL, REFUSED, NOTIMP and FORMERR move on to the next server
/// immediately, without waiting for the timeout.
#[tokio::test]
async fn failure_codes_move_on_to_the_next_server_immediately() {
    let http = FakeHttp::start("ok").await;
    for (label, code) in [
        ("servfail", RCODE_SERVFAIL),
        ("refused", RCODE_REFUSED),
        ("notimp", RCODE_NOTIMP),
        ("formerr", RCODE_FORMERR),
    ] {
        let failing = FakeDns::start(move |_| Reply::Rcode(code)).await;
        let backup = FakeDns::start(|_| Reply::loopback()).await;
        let cfg = format!(
            "dns_servers = {}\ndns_timeout_seconds = 5\n",
            server_list(&[&failing, &backup])
        );

        let started = Instant::now();
        let body = fetch(&cfg, &format!("http://{label}.wfl.test:{}/", http.port)).await;
        assert_eq!(body.as_deref(), Ok("ok"), "{label}: {body:?}");
        assert!(
            failing.query_count() >= 1,
            "{label}: first server not asked"
        );
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "{label}: waited for a timeout instead of moving on"
        );
    }
}

/// FR-02 AC5: a reply that is not a DNS message counts as a failure of that
/// server, and the next server answers.
#[tokio::test]
async fn unparseable_reply_moves_on_to_the_next_server() {
    let http = FakeHttp::start("ok").await;
    let garbage = FakeDns::start(|_| Reply::Garbage).await;
    let backup = FakeDns::start(|_| Reply::loopback()).await;
    let cfg = format!(
        "dns_servers = {}\ndns_timeout_seconds = 1\n",
        server_list(&[&garbage, &backup])
    );

    let body = fetch(&cfg, &format!("http://garbage.wfl.test:{}/", http.port)).await;
    assert_eq!(body.as_deref(), Ok("ok"), "{body:?}");
    assert!(garbage.query_count() >= 1);
}

/// FR-02 AC6: a truncated UDP reply is retried over TCP on the SAME server,
/// before any other server is asked.
#[tokio::test]
async fn truncated_reply_is_retried_over_tcp_on_the_same_server() {
    let http = FakeHttp::start("ok").await;
    let big = FakeDns::start(|_| {
        Reply::TruncatedThenTcp(vec![std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)])
    })
    .await;
    let other = FakeDns::start(|_| Reply::NxDomain).await;
    let cfg = format!("dns_servers = {}\n", server_list(&[&big, &other]));

    let body = fetch(&cfg, &format!("http://big.wfl.test:{}/", http.port)).await;
    assert_eq!(body.as_deref(), Ok("ok"), "{body:?}");
    assert!(
        big.queries_for("big.wfl.test")
            .iter()
            .any(|q| q.transport == Transport::Tcp),
        "the truncated answer was not retried over TCP"
    );
    assert!(other.queries_for("big.wfl.test").is_empty());
}

/// FR-02 AC7 + SEC-01: a reply carrying the wrong query ID is ignored; it must
/// neither be used nor end the wait for the genuine reply.
#[tokio::test]
async fn reply_with_the_wrong_query_id_is_ignored() {
    let http = FakeHttp::start("ok").await;
    let server = FakeDns::start(|_| Reply::WrongIdThen {
        decoy: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 2)),
        then: Box::new(Reply::loopback()),
    })
    .await;
    let cfg = format!("dns_servers = {}\n", server.addr);

    let body = fetch(&cfg, &format!("http://spoof.wfl.test:{}/", http.port)).await;
    assert_eq!(body.as_deref(), Ok("ok"), "{body:?}");
}

// ---------------------------------------------------------------------------
// FR-06: combining A and AAAA from one server
// ---------------------------------------------------------------------------

/// FR-06 AC3 (case 4): A fails while AAAA is an empty NOERROR. That is NOT a
/// final answer, so the next server is asked — this is what keeps an IPv4-only
/// internal host working when a server is flaky for A records.
#[tokio::test]
async fn one_family_failed_and_the_other_empty_asks_the_next_server() {
    let http = FakeHttp::start("ok").await;
    for (label, a_reply) in [
        ("servfail", Reply::Rcode(RCODE_SERVFAIL)),
        ("silent", Reply::Silent),
    ] {
        let flaky = FakeDns::start(move |q| {
            if q.qtype == TYPE_A {
                a_reply.clone()
            } else {
                Reply::NoData
            }
        })
        .await;
        let backup = FakeDns::start(|_| Reply::loopback()).await;
        let cfg = format!(
            "dns_servers = {}\ndns_timeout_seconds = 1\n",
            server_list(&[&flaky, &backup])
        );

        let body = fetch(
            &cfg,
            &format!("http://v4only-{label}.wfl.test:{}/", http.port),
        )
        .await;
        assert_eq!(body.as_deref(), Ok("ok"), "{label}: {body:?}");
        assert!(
            backup.query_count() >= 1,
            "{label}: the next server was never asked"
        );
    }
}

/// FR-06 AC2 (case 1): addresses for either family are final, even if the other
/// family failed on that server.
#[tokio::test]
async fn addresses_for_one_family_are_final() {
    let http = FakeHttp::start("ok").await;
    let first = FakeDns::start(|q| {
        if q.qtype == TYPE_AAAA {
            Reply::Rcode(RCODE_SERVFAIL)
        } else {
            Reply::loopback()
        }
    })
    .await;
    let second = FakeDns::start(|_| Reply::NxDomain).await;
    let cfg = format!("dns_servers = {}\n", server_list(&[&first, &second]));

    let body = fetch(&cfg, &format!("http://mixed.wfl.test:{}/", http.port)).await;
    assert_eq!(body.as_deref(), Ok("ok"), "{body:?}");
    assert!(second.queries_for("mixed.wfl.test").is_empty());
}

/// FR-06 AC2 item 1 + NFR-01: once the first server returns addresses for
/// either family, the lookup is final. A silent AAAA query must not hold the
/// result for the full per-server timeout.
#[tokio::test]
async fn lookup_returns_without_waiting_for_a_silent_aaaa_query() {
    let server = FakeDns::start(|q| {
        if q.qtype == TYPE_AAAA {
            Reply::Silent
        } else {
            Reply::loopback()
        }
    })
    .await;
    let mut settings = DnsSettings::default();
    assert!(settings.apply(
        "dns_servers",
        &server.addr.to_string(),
        Path::new("/project/.wflcfg")
    ));
    assert!(settings.apply(
        "dns_timeout_seconds",
        "2",
        Path::new("/project/.wflcfg")
    ));
    let resolver = OrderedResolver::from_plan_with_sources(
        &plan(&settings),
        SystemSources {
            resolv_conf: None,
            hosts: None,
            port: 53,
        },
    )
    .expect("WFL resolver");
    let started = Instant::now();
    let found = resolver.lookup("v4only.wfl.test").await;
    let elapsed = started.elapsed();
    assert_eq!(
        found.map_err(|e| e.to_string()),
        Ok(vec![IpAddr::V4(Ipv4Addr::LOCALHOST)])
    );
    assert!(
        elapsed < Duration::from_millis(500),
        "a silent AAAA query must not delay a usable A answer; elapsed {elapsed:?}"
    );
}

// ---------------------------------------------------------------------------
// FR-05: localhost never goes to a DNS server
// ---------------------------------------------------------------------------

/// FR-05 AC2 + SEC-16: `localhost` and `*.localhost` are loopback and are never
/// sent to a DNS server, even one that would answer with another address.
#[tokio::test]
async fn localhost_names_never_reach_a_dns_server() {
    let http = FakeHttp::start("ok").await;
    let hostile = FakeDns::start(|_| Reply::v4(127, 0, 0, 2)).await;
    let cfg = format!("dns_servers = {}\n", hostile.addr);

    for host in ["localhost", "app.localhost", "LOCALHOST"] {
        let body = fetch(&cfg, &format!("http://{host}:{}/", http.port)).await;
        assert_eq!(body.as_deref(), Ok("ok"), "{host}: {body:?}");
    }
    assert_eq!(
        hostile.count_matching(|name| name.ends_with("localhost")),
        0
    );
}

// ---------------------------------------------------------------------------
// FR-07 / FR-08 / FR-09 / FR-10 / FR-13: configuration
// ---------------------------------------------------------------------------

/// FR-07 AC3: a server listed twice is asked once, at its first position.
#[tokio::test]
async fn duplicate_servers_are_asked_once() {
    let http = FakeHttp::start("ok").await;
    let failing = FakeDns::start(|_| Reply::Rcode(RCODE_SERVFAIL)).await;
    let backup = FakeDns::start(|_| Reply::loopback()).await;
    let cfg = format!(
        "dns_servers = {a}, {a}, {b}\n",
        a = failing.addr,
        b = backup.addr
    );

    let body = fetch(&cfg, &format!("http://dup.wfl.test:{}/", http.port)).await;
    assert_eq!(body.as_deref(), Ok("ok"), "{body:?}");
    let asked_a = failing
        .queries_for("dup.wfl.test")
        .iter()
        .filter(|q| q.qtype == TYPE_A)
        .count();
    assert_eq!(asked_a, 1, "the duplicate entry was asked again");
}

/// FR-08 AC1: `dns_servers_mode = only` (written explicitly, any case) behaves
/// like the default.
#[tokio::test]
async fn explicit_only_mode_uses_the_listed_servers() {
    let http = FakeHttp::start("ok").await;
    let server = FakeDns::start(|_| Reply::loopback()).await;
    let cfg = format!("dns_servers = {}\ndns_servers_mode =  Only \n", server.addr);

    let body = fetch(&cfg, &format!("http://only.wfl.test:{}/", http.port)).await;
    assert_eq!(body.as_deref(), Ok("ok"), "{body:?}");
    assert!(server.query_count() >= 1);
}

/// FR-09 AC3: an invalid timeout is a warning; lookups still work with the
/// default timeout.
#[tokio::test]
async fn invalid_timeout_falls_back_to_the_default() {
    let http = FakeHttp::start("ok").await;
    let server = FakeDns::start(|_| Reply::loopback()).await;
    for bad in ["0", "31", "-1", "soon"] {
        let cfg = format!(
            "dns_servers = {}\ndns_timeout_seconds = {bad}\n",
            server.addr
        );
        let body = fetch(
            &cfg,
            &format!("http://t{}.wfl.test:{}/", bad.len(), http.port),
        )
        .await;
        assert_eq!(
            body.as_deref(),
            Ok("ok"),
            "dns_timeout_seconds = {bad}: {body:?}"
        );
    }
}

/// FR-13 + FR-07 AC2/AC7 + SEC-15: invalid DNS configuration fails closed. The
/// lookup errors with an explanation that names the setting and the bad value;
/// WFL never quietly uses some other DNS server instead.
#[tokio::test]
async fn invalid_dns_configuration_fails_closed_with_an_explanation() {
    let cases = [
        ("dns_servers = dns.example.com", "dns.example.com"),
        ("dns_servers = fe80::1%eth0", "fe80::1%eth0"),
        ("dns_servers = 127.0.0.1:0", "127.0.0.1:0"),
        ("dns_servers = 127.0.0.1:65536", "127.0.0.1:65536"),
        ("dns_servers = 127.0.0.1,,127.0.0.2", "dns_servers"),
        (
            "dns_servers = 127.0.0.1\ndns_servers_mode = sometimes",
            "sometimes",
        ),
        ("dns_servers = 127.0.0.1\ndns_resolver = magic", "magic"),
    ];
    for (cfg, offending) in cases {
        let message = fetch(cfg, "http://cfg.wfl.test:9/")
            .await
            .expect_err("invalid DNS configuration must fail the lookup");
        assert!(
            message.contains(offending),
            "{cfg:?}: the error does not name {offending:?}: {message}"
        );
        assert!(
            message.contains(".wflcfg"),
            "{cfg:?}: the error does not say which file to fix: {message}"
        );
    }
}

/// FR-13 AC3: a program that never touches the network is unaffected by
/// invalid DNS settings.
#[tokio::test]
async fn invalid_dns_configuration_does_not_affect_offline_programs() {
    let result = run_with_config(
        "dns_servers = not-an-ip\ndns_resolver = magic\n",
        "store body as \"offline\"\n",
    )
    .await;
    assert_eq!(result.as_deref(), Ok("offline"));
}

/// FR-07 hardening (FR-13 AC4): oversized and adversarial values never panic,
/// and always fail closed.
#[tokio::test]
async fn adversarial_dns_servers_values_fail_closed() {
    let huge = "127.0.0.1, ".repeat(5000);
    let control = "127.0.0.1\u{7}";
    for value in [
        huge.as_str(),
        control,
        "[::1",
        "::1]:53",
        "127.0.0.1:53:53",
        "[]",
    ] {
        let cfg = format!("dns_servers = {value}\n");
        let result = fetch(&cfg, "http://adv.wfl.test:9/").await;
        let message = result.expect_err("adversarial dns_servers must fail closed");
        assert!(message.contains("dns_servers"), "{value:?}: {message}");
    }
}

// ---------------------------------------------------------------------------
// FR-14 + SEC-08: the error message
// ---------------------------------------------------------------------------

/// FR-14 AC1/AC2: a failed lookup lists every server asked, in order, where it
/// came from, and what it said — and never repeats the URL's path or query.
#[tokio::test]
async fn failed_lookup_lists_servers_in_order_and_hides_the_url_path() {
    let silent = FakeDns::start(|_| Reply::Silent).await;
    let failing = FakeDns::start(|_| Reply::Rcode(RCODE_SERVFAIL)).await;
    let nx = FakeDns::start(|_| Reply::NxDomain).await;
    let cfg = format!(
        "dns_servers = {}\ndns_timeout_seconds = 1\n",
        server_list(&[&silent, &failing, &nx])
    );

    let message = fetch(&cfg, "http://gone.wfl.test:9/private/path?token=s3cret")
        .await
        .expect_err("lookup must fail");

    assert!(
        message.contains("Could not find the address of \"gone.wfl.test\""),
        "{message}"
    );
    let pos = |needle: &str| {
        message
            .find(needle)
            .unwrap_or_else(|| panic!("missing {needle:?} in: {message}"))
    };
    let (a, b, c) = (
        pos(&silent.addr.to_string()),
        pos(&failing.addr.to_string()),
        pos(&nx.addr.to_string()),
    );
    assert!(a < b && b < c, "servers are not listed in order: {message}");
    assert!(message.contains("no reply within 1 second"), "{message}");
    assert!(message.contains("server failure"), "{message}");
    assert!(message.contains("no such name"), "{message}");
    assert!(
        message.contains(".wflcfg"),
        "the source file is missing: {message}"
    );
    assert!(message.contains("Hint"), "{message}");
    assert!(
        !message.contains("private/path"),
        "URL path leaked: {message}"
    );
    assert!(!message.contains("s3cret"), "URL query leaked: {message}");
}

// ---------------------------------------------------------------------------
// NFR-02: bounded waiting and cancellation
// ---------------------------------------------------------------------------

/// Every query a fake server received for `name`, as "TYPE/transport at
/// +N ms" relative to `start`, for failure messages.
fn timeline(server: &FakeDns, name: &str, start: Instant) -> String {
    let entries: Vec<String> = server
        .queries_for(name)
        .iter()
        .map(|q| {
            let kind = match q.qtype {
                TYPE_A => "A".to_string(),
                TYPE_AAAA => "AAAA".to_string(),
                other => format!("type {other}"),
            };
            let at = q.at.saturating_duration_since(start).as_millis();
            format!("{kind}/{:?} at +{at} ms", q.transport)
        })
        .collect();
    if entries.is_empty() {
        "nothing".to_string()
    } else {
        entries.join(", ")
    }
}

/// NFR-02 + SEC-09: the program's execution deadline caps the whole lookup,
/// and once the request is abandoned no further server is asked.
///
/// The assertions are scoped to this lookup's name. Tests run in parallel on
/// ephemeral ports, so counting every datagram that reaches a port could also
/// count a stray query from another test whose server had just released the
/// same port.
#[tokio::test]
async fn execution_deadline_caps_the_lookup_and_stops_further_queries() {
    const NAME: &str = "deadline-stuck.wfl.test";
    let first = FakeDns::start(|_| Reply::Silent).await;
    let second = FakeDns::start(|_| Reply::Silent).await;
    let cfg = format!(
        "timeout_seconds = 2\ndns_servers = {}\ndns_timeout_seconds = 5\n",
        server_list(&[&first, &second])
    );

    let started = Instant::now();
    let result = fetch(&cfg, &format!("http://{NAME}:9/")).await;
    let elapsed = started.elapsed();
    assert!(result.is_err(), "{result:?}");
    assert!(
        !first.queries_for(NAME).is_empty(),
        "the first server was never asked"
    );
    assert!(
        elapsed < Duration::from_secs(4),
        "the lookup outlived the 2 s deadline: {elapsed:?}"
    );

    // The first server is silent and its timeout is 5 s, so the second server
    // must not be asked before the 2 s deadline; and once the request is
    // abandoned it must not be asked at all. (It would be asked at about
    // t = 5 s if the lookup were still running.)
    tokio::time::sleep(Duration::from_secs(4)).await;
    assert!(
        second.queries_for(NAME).is_empty(),
        "the second server was asked for {NAME}, but the first server never \
         answered and the request was abandoned at +{} ms.\n\
         first server saw: {}\nsecond server saw: {}\nlookup result: {result:?}",
        elapsed.as_millis(),
        timeline(&first, NAME, started),
        timeline(&second, NAME, started),
    );
}

/// SEC-09 on every platform: abandoning a request (dropping its future,
/// which is what WFL's execution deadline does in `run_http_with_budget`)
/// also drops the DNS lookup reqwest started for it. If reqwest or hyper ever
/// moved name resolution onto a detached task, an abandoned WFL lookup could
/// keep asking servers in the background; this test would then fail.
///
/// Deterministic: no network and no fake DNS server. The resolver's future
/// never finishes and records when it is dropped.
#[tokio::test]
async fn abandoning_a_request_drops_its_dns_lookup() {
    use std::sync::atomic::{AtomicBool, Ordering};

    struct DropFlag(Arc<AtomicBool>);
    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    struct NeverResolves {
        started: Arc<AtomicBool>,
        dropped: Arc<AtomicBool>,
    }
    impl reqwest::dns::Resolve for NeverResolves {
        fn resolve(&self, _name: reqwest::dns::Name) -> reqwest::dns::Resolving {
            self.started.store(true, Ordering::SeqCst);
            let flag = DropFlag(Arc::clone(&self.dropped));
            Box::pin(async move {
                let _flag = flag;
                std::future::pending::<
                    Result<reqwest::dns::Addrs, Box<dyn std::error::Error + Send + Sync>>,
                >()
                .await
            })
        }
    }

    let started = Arc::new(AtomicBool::new(false));
    let dropped = Arc::new(AtomicBool::new(false));
    let client = reqwest::Client::builder()
        .dns_resolver(Arc::new(NeverResolves {
            started: Arc::clone(&started),
            dropped: Arc::clone(&dropped),
        }))
        .build()
        .expect("client");

    let abandoned = tokio::time::timeout(
        Duration::from_millis(200),
        client.get("http://abandoned.wfl.test/").send(),
    )
    .await;
    assert!(
        abandoned.is_err(),
        "the request should still have been resolving"
    );
    assert!(
        started.load(Ordering::SeqCst),
        "the resolver was never asked"
    );
    assert!(
        dropped.load(Ordering::SeqCst),
        "the DNS lookup kept running after its request was abandoned"
    );
}

// ---------------------------------------------------------------------------
// FR-16 / FR-17 / FR-10: visibility through the real binary
// ---------------------------------------------------------------------------

/// FR-16: `wfl --dump-env` shows the DNS setup in use, with the source file of
/// every server, and flags a mode that has no servers to apply to (FR-08 AC4).
#[test]
fn dump_env_shows_the_dns_setup() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join(".wflcfg"),
        "dns_servers = 127.0.0.1:5353, [fd00::1]:53\ndns_timeout_seconds = 3\n",
    )
    .unwrap();
    let (out, code) = run_binary(dir.path(), &["--dump-env"], None);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("=== DNS ==="), "{out}");
    assert!(out.contains("Resolver: wfl"), "{out}");
    assert!(out.contains("Mode: only"), "{out}");
    assert!(out.contains("1. 127.0.0.1:5353"), "{out}");
    assert!(out.contains("2. [fd00::1]:53"), "{out}");
    assert!(out.contains("Timeout: 3 seconds"), "{out}");
    let section = &out[out.find("=== DNS ===").unwrap()..];
    assert!(
        section.contains(".wflcfg"),
        "server sources missing: {section}"
    );

    let lonely = TempDir::new().unwrap();
    std::fs::write(
        lonely.path().join(".wflcfg"),
        "dns_servers_mode = before_system\n",
    )
    .unwrap();
    let (out, _) = run_binary(lonely.path(), &["--dump-env"], None);
    let section = &out[out.find("=== DNS ===").unwrap_or(0)..];
    assert!(
        section.contains("dns_servers_mode is set but dns_servers is not"),
        "{out}"
    );
}

/// FR-10 AC3: an explicit `dns_resolver = system` wins over `dns_servers`, and
/// the conflict is reported.
#[test]
fn off_switch_wins_and_is_reported() {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join(".wflcfg"),
        "dns_servers = 127.0.0.1:5353\ndns_resolver = system\n",
    )
    .unwrap();
    let (out, code) = run_binary(dir.path(), &["--dump-env"], None);
    assert_eq!(code, Some(0), "{out}");
    let section = &out[out.find("=== DNS ===").unwrap_or(0)..];
    assert!(section.contains("Resolver: system"), "{out}");
    assert!(
        section.contains("dns_servers is ignored because dns_resolver = system"),
        "{out}"
    );
}

/// FR-07 AC5: an empty local `dns_servers =` clears a value inherited from the
/// global configuration file.
#[test]
fn empty_local_value_clears_inherited_global_servers() {
    let dir = TempDir::new().unwrap();
    let global = dir.path().join("global.cfg");
    std::fs::write(&global, "dns_servers = 127.0.0.1:5353\n").unwrap();
    let project = dir.path().join("project");
    std::fs::create_dir(&project).unwrap();

    let (out, _) = run_binary(&project, &["--dump-env"], Some(&global));
    let section = &out[out.find("=== DNS ===").unwrap_or(0)..];
    assert!(
        section.contains("127.0.0.1:5353"),
        "global value not shown: {out}"
    );

    std::fs::write(project.join(".wflcfg"), "dns_servers =\n").unwrap();
    let (out, _) = run_binary(&project, &["--dump-env"], Some(&global));
    let section = &out[out.find("=== DNS ===").unwrap_or(0)..];
    assert!(section.contains("=== DNS ==="), "{out}");
    assert!(
        !section.contains("127.0.0.1:5353"),
        "the empty local value did not clear the global one: {out}"
    );
}

/// FR-17: `wfl --configCheck` knows the four DNS keys and validates them with
/// the same rules as the loader; `--configFix` never invents DNS servers.
#[test]
fn config_check_knows_and_validates_the_dns_keys() {
    let good = TempDir::new().unwrap();
    std::fs::write(
        good.path().join(".wflcfg"),
        "dns_servers = 10.0.0.53, 10.0.0.54\ndns_servers_mode = only\ndns_timeout_seconds = 2\ndns_resolver = wfl\n",
    )
    .unwrap();
    let (out, _) = run_binary(good.path(), &["--configCheck", "."], None);
    for key in [
        "dns_servers",
        "dns_servers_mode",
        "dns_timeout_seconds",
        "dns_resolver",
    ] {
        assert!(
            !out.contains(&format!("Unknown configuration key: {key}"))
                && !out.contains(&format!("Unknown key '{key}'"))
                && !out.contains(&format!("unknown key: {key}")),
            "{key} reported as unknown:\n{out}"
        );
    }
    assert!(!out.to_lowercase().contains("unknown"), "{out}");

    let bad = TempDir::new().unwrap();
    std::fs::write(
        bad.path().join(".wflcfg"),
        "dns_servers = dns.example.com\ndns_servers_mode = sometimes\ndns_timeout_seconds = 31\ndns_resolver = magic\n",
    )
    .unwrap();
    let (out, _) = run_binary(bad.path(), &["--configCheck", "."], None);
    for needle in [
        "dns.example.com",
        "sometimes",
        "dns_timeout_seconds",
        "magic",
    ] {
        assert!(
            out.contains(needle),
            "configCheck missed {needle:?}:\n{out}"
        );
    }

    let (out, _) = run_binary(bad.path(), &["--configFix", "."], None);
    let fixed = std::fs::read_to_string(bad.path().join(".wflcfg")).unwrap();
    assert!(
        !fixed.contains("8.8.8.8") && !fixed.contains("1.1.1.1"),
        "--configFix invented DNS servers:\n{fixed}\n{out}"
    );
}

// ---------------------------------------------------------------------------
// SEC-07: reqwest's own hickory feature stays off
// ---------------------------------------------------------------------------

/// SEC-07: reqwest's built-in `hickory-dns` integration silently falls back to
/// Google DNS when `/etc/resolv.conf` cannot be read. WFL must never turn it on.
#[test]
fn reqwest_hickory_dns_feature_is_never_enabled() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let reqwest_line = manifest
        .lines()
        .find(|l| l.trim_start().starts_with("reqwest"))
        .expect("reqwest dependency");
    assert!(!reqwest_line.contains("hickory"), "{reqwest_line}");

    let lock = std::fs::read_to_string(root.join("Cargo.lock")).unwrap();
    let reqwest_block = lock
        .split("[[package]]")
        .find(|b| b.contains("name = \"reqwest\""))
        .expect("reqwest in Cargo.lock");
    assert!(
        !reqwest_block.contains("hickory"),
        "reqwest was resolved with a hickory dependency:\n{reqwest_block}"
    );
}
