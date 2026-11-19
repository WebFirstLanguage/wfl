//! Ordered DNS resolution: the system's own DNS settings (cycle 2 of the PRD,
//! `Engineering/designs/2026-11-18-ordered-dns-resolution-design.md`).
//!
//! These tests point WFL's resolver at a temporary `resolv.conf` and hosts
//! file through `OrderedResolver::from_plan_with_sources` — a Rust library
//! parameter that no `.wflcfg` key, environment variable, flag, or WFL program
//! can reach (SEC-17) — and at real DNS servers on loopback
//! (`dns_fake_server`). They cover: reading `/etc/resolv.conf` and asking its
//! servers in order (the Linux default), `before_system` / `after_system`,
//! search domains and `ndots`, the hosts file, the answer cache, and the "no
//! servers at all" error.
//!
//! `resolv.conf` cannot name a port, so the system-list tests start their
//! servers on `127.0.0.1`, `127.0.0.2`, ... sharing one port and tell the
//! resolver that port. Those tests are Linux-only, because only Linux treats
//! the whole `127.0.0.0/8` block as loopback.

// Several helpers serve only the Linux-only system-list tests.
#![cfg_attr(not(target_os = "linux"), allow(dead_code, unused_imports))]

use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tempfile::TempDir;
use wfl::interpreter::dns::{DnsPlan, DnsSettings, OrderedResolver, SystemSources, plan};

use crate::dns_fake_server::{FakeDns, Query, RCODE_SERVFAIL, Reply, TYPE_A, handler};

const LOCAL: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

fn settings(lines: &[(&str, &str)]) -> DnsSettings {
    let mut settings = DnsSettings::default();
    for (key, value) in lines {
        assert!(settings.apply(key, value, Path::new("/project/.wflcfg")));
    }
    settings
}

/// A temporary `resolv.conf` and hosts file.
struct SystemFiles {
    dir: TempDir,
}

impl SystemFiles {
    fn new(resolv_conf: &str, hosts: &str) -> Self {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("resolv.conf"), resolv_conf).unwrap();
        std::fs::write(dir.path().join("hosts"), hosts).unwrap();
        Self { dir }
    }

    fn resolv_conf(&self) -> PathBuf {
        self.dir.path().join("resolv.conf")
    }

    fn hosts(&self) -> PathBuf {
        self.dir.path().join("hosts")
    }

    fn sources(&self, port: u16) -> SystemSources {
        SystemSources {
            resolv_conf: Some(self.resolv_conf()),
            hosts: Some(self.hosts()),
            port,
        }
    }
}

fn resolver(settings: &DnsSettings, sources: SystemSources) -> OrderedResolver {
    OrderedResolver::from_plan_with_sources(&plan(settings), sources)
        .expect("these settings select WFL's resolver")
}

fn explicit_wfl() -> DnsSettings {
    settings(&[("dns_resolver", "wfl")])
}

/// Queries of type A, across several servers, in the order they arrived.
fn a_queries_in_order(servers: &[(&str, &FakeDns)]) -> Vec<(String, String)> {
    let mut all: Vec<(Instant, String, String)> = servers
        .iter()
        .flat_map(|(label, server)| {
            server
                .queries()
                .into_iter()
                .filter(|q: &Query| q.qtype == TYPE_A)
                .map(|q| (q.at, label.to_string(), q.name))
                .collect::<Vec<_>>()
        })
        .collect();
    all.sort_by_key(|(at, _, _)| *at);
    all.into_iter()
        .map(|(_, label, name)| (label, name))
        .collect()
}

fn a_count(server: &FakeDns, name: &str) -> usize {
    server
        .queries_for(name)
        .iter()
        .filter(|q| q.qtype == TYPE_A)
        .count()
}

// ---------------------------------------------------------------------------
// The Linux default and the system list (FR-01, FR-03, FR-09, A12)
// ---------------------------------------------------------------------------

/// A12: on Linux, WFL's ordered resolver is the default, with no settings.
#[cfg(target_os = "linux")]
#[test]
fn linux_default_is_the_ordered_resolver() {
    assert!(
        matches!(plan(&DnsSettings::default()), DnsPlan::Wfl { .. }),
        "{:?}",
        plan(&DnsSettings::default())
    );
}

/// NG1 / A1: elsewhere the OS resolver stays the default.
#[cfg(not(target_os = "linux"))]
#[test]
fn other_platforms_default_to_the_system_resolver() {
    assert!(matches!(
        plan(&DnsSettings::default()),
        DnsPlan::System { .. }
    ));
}

/// FR-01 on the default path: the servers in `/etc/resolv.conf` are asked one
/// at a time, in order — the second never wins the race.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn resolv_conf_servers_are_asked_in_order() {
    let (port, servers) = FakeDns::start_group(
        60,
        vec![
            handler(|_| Reply::delayed(100, Reply::loopback())),
            handler(|_| Reply::NxDomain),
        ],
    )
    .await;
    let files = SystemFiles::new("nameserver 127.0.0.1\nnameserver 127.0.0.2\n", "");
    let resolver = resolver(&explicit_wfl(), files.sources(port));

    for run in 0..30 {
        let name = format!("sysorder{run}.wfl.test");
        assert_eq!(
            resolver.lookup(&name).await.map_err(|e| e.to_string()),
            Ok(vec![LOCAL]),
            "run {run}"
        );
    }
    assert_eq!(
        servers[1].count_matching(|name| name.starts_with("sysorder")),
        0,
        "the second server was asked"
    );
}

/// FR-03 + FR-09: `options timeout:N attempts:N` in resolv.conf are honored.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn resolv_conf_timeout_and_attempts_are_honored() {
    let (port, servers) = FakeDns::start_group(
        60,
        vec![handler(|_| Reply::Silent), handler(|_| Reply::loopback())],
    )
    .await;
    let files = SystemFiles::new(
        "nameserver 127.0.0.1\nnameserver 127.0.0.2\noptions timeout:1 attempts:1\n",
        "",
    );
    let started = Instant::now();
    let found = resolver(&explicit_wfl(), files.sources(port))
        .lookup("slow.wfl.test")
        .await;
    let elapsed = started.elapsed();
    assert_eq!(found.map_err(|e| e.to_string()), Ok(vec![LOCAL]));
    assert!(elapsed >= Duration::from_millis(900), "{elapsed:?}");
    assert!(
        elapsed < Duration::from_secs(3),
        "timeout:1 ignored: {elapsed:?}"
    );
    assert!(servers[0].query_count() >= 1);

    // attempts:2 with one silent server: the list is tried exactly twice, so
    // the lookup gives up after two one-second rounds. Rounds are measured by
    // time, not by counting packets: within one round hickory re-sends the
    // same question to the same server over UDP (after about 1.2x the measured
    // round-trip time, at least ~333 ms, up to 3 times). Those re-sends are
    // not extra rounds, and no other server is contacted meanwhile.
    let (port, silent) = FakeDns::start_group(60, vec![handler(|_| Reply::Silent)]).await;
    let files = SystemFiles::new("nameserver 127.0.0.1\noptions timeout:1 attempts:2\n", "");
    let started = Instant::now();
    let result = resolver(&explicit_wfl(), files.sources(port))
        .lookup("never.wfl.test")
        .await;
    let elapsed = started.elapsed();
    assert!(result.is_err());
    assert!(
        elapsed >= Duration::from_millis(1800) && elapsed < Duration::from_millis(2900),
        "attempts:2 x timeout:1 should take about 2 s, took {elapsed:?}"
    );
    assert!(a_count(&silent[0], "never.wfl.test") >= 2);
}

/// FR-09 AC1: `dns_timeout_seconds` overrides the resolv.conf timeout.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn dns_timeout_seconds_overrides_the_resolv_conf_timeout() {
    let (port, _servers) = FakeDns::start_group(
        60,
        vec![handler(|_| Reply::Silent), handler(|_| Reply::loopback())],
    )
    .await;
    let files = SystemFiles::new(
        "nameserver 127.0.0.1\nnameserver 127.0.0.2\noptions timeout:5\n",
        "",
    );
    let started = Instant::now();
    let found = resolver(
        &settings(&[("dns_resolver", "wfl"), ("dns_timeout_seconds", "1")]),
        files.sources(port),
    )
    .lookup("override.wfl.test")
    .await;
    assert_eq!(found.map_err(|e| e.to_string()), Ok(vec![LOCAL]));
    assert!(started.elapsed() < Duration::from_secs(3));
}

/// FR-14: servers read from resolv.conf are named with their source.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn system_servers_are_named_with_their_source_in_errors() {
    let (port, _servers) = FakeDns::start_group(
        60,
        vec![
            handler(|_| Reply::Rcode(RCODE_SERVFAIL)),
            handler(|_| Reply::NxDomain),
        ],
    )
    .await;
    let files = SystemFiles::new("nameserver 127.0.0.1\nnameserver 127.0.0.2\n", "");
    let error = resolver(&explicit_wfl(), files.sources(port))
        .lookup("missing.wfl.test")
        .await
        .expect_err("no such name");
    let message = error.to_string();
    let source = format!("(from {})", files.resolv_conf().display());
    assert!(message.contains(&source), "{message}");
    assert!(message.contains("server failure"), "{message}");
    assert!(message.contains("no such name"), "{message}");
}

// ---------------------------------------------------------------------------
// dns_servers_mode (FR-08, A26)
// ---------------------------------------------------------------------------

/// FR-08 AC2: `only` never contacts a resolv.conf server.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn only_mode_never_asks_resolv_conf_servers() {
    let (port, system) = FakeDns::start_group(60, vec![handler(|_| Reply::v4(10, 9, 9, 9))]).await;
    let mine = FakeDns::start(|_| Reply::loopback()).await;
    let files = SystemFiles::new("nameserver 127.0.0.1\n", "");
    let found = resolver(
        &settings(&[("dns_servers", &mine.addr.to_string())]),
        files.sources(port),
    )
    .lookup("only.wfl.test")
    .await;
    assert_eq!(found.map_err(|e| e.to_string()), Ok(vec![LOCAL]));
    assert!(system[0].queries_for("only.wfl.test").is_empty());
}

/// FR-08 AC3: `before_system` asks the .wflcfg servers, then the system list.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn before_system_asks_mine_first_then_the_system_list() {
    let (port, system) = FakeDns::start_group(60, vec![handler(|_| Reply::loopback())]).await;
    let mine = FakeDns::start(|_| Reply::Rcode(RCODE_SERVFAIL)).await;
    let files = SystemFiles::new("nameserver 127.0.0.1\n", "");
    let found = resolver(
        &settings(&[
            ("dns_servers", &mine.addr.to_string()),
            ("dns_servers_mode", "before_system"),
        ]),
        files.sources(port),
    )
    .lookup("before.wfl.test")
    .await;
    assert_eq!(found.map_err(|e| e.to_string()), Ok(vec![LOCAL]));
    let order = a_queries_in_order(&[("mine", &mine), ("system", &system[0])]);
    assert_eq!(
        order
            .iter()
            .map(|(who, _)| who.as_str())
            .collect::<Vec<_>>(),
        ["mine", "system"]
    );
}

/// FR-08 AC3: `after_system` asks the system list first...
#[cfg(target_os = "linux")]
#[tokio::test]
async fn after_system_asks_the_system_list_first() {
    let (port, system) =
        FakeDns::start_group(60, vec![handler(|_| Reply::Rcode(RCODE_SERVFAIL))]).await;
    let mine = FakeDns::start(|_| Reply::loopback()).await;
    let files = SystemFiles::new("nameserver 127.0.0.1\n", "");
    let found = resolver(
        &settings(&[
            ("dns_servers", &mine.addr.to_string()),
            ("dns_servers_mode", "after_system"),
        ]),
        files.sources(port),
    )
    .lookup("after.wfl.test")
    .await;
    assert_eq!(found.map_err(|e| e.to_string()), Ok(vec![LOCAL]));
    let order = a_queries_in_order(&[("mine", &mine), ("system", &system[0])]);
    assert_eq!(
        order
            .iter()
            .map(|(who, _)| who.as_str())
            .collect::<Vec<_>>(),
        ["system", "mine"]
    );
}

/// ...and so (A9) it cannot override a final "no such name" from the system.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn after_system_cannot_override_a_final_system_answer() {
    let (port, _system) = FakeDns::start_group(60, vec![handler(|_| Reply::NxDomain)]).await;
    let mine = FakeDns::start(|_| Reply::loopback()).await;
    let files = SystemFiles::new("nameserver 127.0.0.1\n", "");
    let result = resolver(
        &settings(&[
            ("dns_servers", &mine.addr.to_string()),
            ("dns_servers_mode", "after_system"),
        ]),
        files.sources(port),
    )
    .lookup("internal.wfl.test")
    .await;
    assert!(result.is_err(), "{result:?}");
    assert!(mine.queries_for("internal.wfl.test").is_empty());
}

/// A26: a server in both lists is asked once, at its first position.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn a_server_in_both_lists_is_asked_once() {
    let (port, system) = FakeDns::start_group(
        60,
        vec![
            handler(|_| Reply::Rcode(RCODE_SERVFAIL)),
            handler(|_| Reply::loopback()),
        ],
    )
    .await;
    let files = SystemFiles::new("nameserver 127.0.0.1\nnameserver 127.0.0.2\n", "");
    let found = resolver(
        &settings(&[
            ("dns_servers", &format!("127.0.0.1:{port}")),
            ("dns_servers_mode", "before_system"),
        ]),
        files.sources(port),
    )
    .lookup("both.wfl.test")
    .await;
    assert_eq!(found.map_err(|e| e.to_string()), Ok(vec![LOCAL]));
    assert_eq!(a_count(&system[0], "both.wfl.test"), 1);
}

/// FR-12 AC4: a mixing mode with no usable system list uses dns_servers alone.
#[cfg(unix)]
#[tokio::test]
async fn mixing_mode_without_a_system_list_uses_dns_servers_alone() {
    let mine = FakeDns::start(|_| Reply::loopback()).await;
    let dir = TempDir::new().unwrap();
    let sources = SystemSources {
        resolv_conf: Some(dir.path().join("missing-resolv.conf")),
        hosts: None,
        port: 53,
    };
    let found = resolver(
        &settings(&[
            ("dns_servers", &mine.addr.to_string()),
            ("dns_servers_mode", "before_system"),
        ]),
        sources,
    )
    .lookup("alone.wfl.test")
    .await;
    assert_eq!(found.map_err(|e| e.to_string()), Ok(vec![LOCAL]));
}

/// FR-11 AC2: off Linux, the mixing modes are not supported yet.
#[cfg(not(target_os = "linux"))]
#[test]
fn mixing_modes_are_rejected_off_linux() {
    for mode in ["before_system", "after_system"] {
        match plan(&settings(&[
            ("dns_servers", "10.0.0.1"),
            ("dns_servers_mode", mode),
        ])) {
            DnsPlan::Misconfigured { message } => {
                assert!(
                    message.contains("not supported on this platform yet"),
                    "{message}"
                )
            }
            other => panic!("{mode}: {other:?}"),
        }
    }
}

// ---------------------------------------------------------------------------
// FR-12: no servers at all
// ---------------------------------------------------------------------------

/// FR-12 AC1/AC2 + SEC-07: with no servers anywhere, the lookup fails at once,
/// names every place WFL looked, and never falls back to a server nobody chose.
#[cfg(unix)]
#[tokio::test]
async fn no_servers_at_all_fails_immediately_and_names_the_sources() {
    let dir = TempDir::new().unwrap();
    let empty = dir.path().join("empty-resolv.conf");
    std::fs::write(&empty, "# no nameserver lines\nsearch corp.test\n").unwrap();
    let directory = dir.path().join("a-directory");
    std::fs::create_dir(&directory).unwrap();
    for resolv_conf in [dir.path().join("missing-resolv.conf"), empty, directory] {
        let sources = SystemSources {
            resolv_conf: Some(resolv_conf.clone()),
            hosts: None,
            port: 53,
        };
        let started = Instant::now();
        let error = resolver(&explicit_wfl(), sources)
            .lookup("anything.wfl.test")
            .await
            .expect_err("no servers");
        assert!(started.elapsed() < Duration::from_millis(500));
        let message = error.to_string();
        assert!(
            message.contains(&resolv_conf.display().to_string()),
            "{}: {message}",
            resolv_conf.display()
        );
        assert!(message.contains("dns_servers"), "{message}");
        assert!(message.contains("no DNS servers"), "{message}");
    }
}

// ---------------------------------------------------------------------------
// FR-04: search domains and ndots, glibc order
// ---------------------------------------------------------------------------

/// FR-04 AC1/AC2: an unqualified name tries the search list first; each
/// candidate is asked of EVERY server before the next candidate; "no such
/// name" moves to the next candidate.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn unqualified_names_try_the_search_list_in_glibc_order() {
    let (port, servers) = FakeDns::start_group(
        60,
        vec![
            handler(|_| Reply::Rcode(RCODE_SERVFAIL)),
            handler(|q| {
                if q.name == "app.lab.test" {
                    Reply::loopback()
                } else {
                    Reply::NxDomain
                }
            }),
        ],
    )
    .await;
    let files = SystemFiles::new(
        "search corp.test lab.test\nnameserver 127.0.0.1\nnameserver 127.0.0.2\n",
        "",
    );
    let found = resolver(&explicit_wfl(), files.sources(port))
        .lookup("app")
        .await;
    assert_eq!(found.map_err(|e| e.to_string()), Ok(vec![LOCAL]));
    assert_eq!(
        a_queries_in_order(&[("s1", &servers[0]), ("s2", &servers[1])]),
        [
            ("s1".to_string(), "app.corp.test".to_string()),
            ("s2".to_string(), "app.corp.test".to_string()),
            ("s1".to_string(), "app.lab.test".to_string()),
            ("s2".to_string(), "app.lab.test".to_string()),
        ]
    );
}

/// FR-04: a name with at least `ndots` dots is tried as-is first; a trailing
/// dot means "as-is only"; `ndots` changes the order; `domain` is used when
/// there is no `search` line.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn ndots_trailing_dot_and_domain_follow_glibc() {
    async fn asked(resolv_conf: &str, name: &str) -> Vec<String> {
        let (port, servers) = FakeDns::start_group(60, vec![handler(|_| Reply::NxDomain)]).await;
        let files = SystemFiles::new(&format!("nameserver 127.0.0.1\n{resolv_conf}"), "");
        let _ = resolver(&explicit_wfl(), files.sources(port))
            .lookup(name)
            .await;
        a_queries_in_order(&[("s", &servers[0])])
            .into_iter()
            .map(|(_, n)| n)
            .collect()
    }

    assert_eq!(
        asked("search corp.test\n", "db.internal").await,
        ["db.internal", "db.internal.corp.test"]
    );
    assert_eq!(
        asked("search corp.test\noptions ndots:2\n", "db.internal").await,
        ["db.internal.corp.test", "db.internal"]
    );
    assert_eq!(asked("search corp.test\n", "app.").await, ["app"]);
    assert_eq!(
        asked("domain corp.test\n", "app").await,
        ["app.corp.test", "app"]
    );
}

/// A8: the search list from resolv.conf also applies to dns_servers.
#[cfg(unix)]
#[tokio::test]
async fn the_search_list_also_applies_to_dns_servers() {
    let mine = FakeDns::start(|q| {
        if q.name == "app.corp.test" {
            Reply::loopback()
        } else {
            Reply::NxDomain
        }
    })
    .await;
    let files = SystemFiles::new("search corp.test\n", "");
    let found = resolver(
        &settings(&[("dns_servers", &mine.addr.to_string())]),
        files.sources(53),
    )
    .lookup("app")
    .await;
    assert_eq!(found.map_err(|e| e.to_string()), Ok(vec![LOCAL]));
    assert_eq!(a_count(&mine, "app.corp.test"), 1);
}

// ---------------------------------------------------------------------------
// FR-05: the hosts file
// ---------------------------------------------------------------------------

fn with_hosts(hosts: &str) -> SystemFiles {
    SystemFiles::new("", hosts)
}

/// FR-05 AC1: a name in the hosts file is answered from it, with no DNS query.
#[tokio::test]
async fn hosts_file_names_are_answered_without_dns() {
    let dns = FakeDns::start(|_| Reply::v4(127, 0, 0, 2)).await;
    let files = with_hosts("127.0.0.1 intranet.wfl.test\n");
    let found = resolver(
        &settings(&[("dns_servers", &dns.addr.to_string())]),
        files.sources(53),
    )
    .lookup("intranet.wfl.test")
    .await;
    assert_eq!(found.map_err(|e| e.to_string()), Ok(vec![LOCAL]));
    assert!(dns.queries_for("intranet.wfl.test").is_empty());
}

/// FR-05: aliases, several lines for one name, IPv6, comments, and case.
#[tokio::test]
async fn hosts_file_aliases_lines_ipv6_and_comments() {
    let dns = FakeDns::start(|_| Reply::v4(127, 0, 0, 2)).await;
    let files = with_hosts(
        "# a comment line\n\
         10.0.0.5\tcanonical.wfl.test alias.wfl.test   # trailing comment\n\
         10.0.0.6 alias.wfl.test\n\
         fd00::5 alias.wfl.test\n\
         not-an-address alias.wfl.test\n",
    );
    let found = resolver(
        &settings(&[("dns_servers", &dns.addr.to_string())]),
        files.sources(53),
    )
    .lookup("ALIAS.wfl.test")
    .await;
    assert_eq!(
        found.map_err(|e| e.to_string()),
        Ok(vec![
            "10.0.0.5".parse::<IpAddr>().unwrap(),
            "10.0.0.6".parse().unwrap(),
            "fd00::5".parse().unwrap(),
        ])
    );
    assert!(dns.queries_for("alias.wfl.test").is_empty());
}

/// FR-05 AC1: edits to the hosts file are seen by a running program.
#[tokio::test]
async fn hosts_file_changes_are_seen_without_a_restart() {
    let dns = FakeDns::start(|_| Reply::v4(127, 0, 0, 2)).await;
    let files = with_hosts("10.0.0.1 moving.wfl.test\n");
    let resolver = resolver(
        &settings(&[("dns_servers", &dns.addr.to_string())]),
        files.sources(53),
    );
    assert_eq!(
        resolver
            .lookup("moving.wfl.test")
            .await
            .map_err(|e| e.to_string()),
        Ok(vec!["10.0.0.1".parse::<IpAddr>().unwrap()])
    );
    // Filesystem timestamps can be as coarse as one second.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    std::fs::write(files.hosts(), "10.0.0.2 moving.wfl.test\n").unwrap();
    assert_eq!(
        resolver
            .lookup("moving.wfl.test")
            .await
            .map_err(|e| e.to_string()),
        Ok(vec!["10.0.0.2".parse::<IpAddr>().unwrap()])
    );
}

// ---------------------------------------------------------------------------
// FR-15: the answer cache
// ---------------------------------------------------------------------------

fn only(dns: &FakeDns) -> DnsSettings {
    settings(&[("dns_servers", &dns.addr.to_string())])
}

fn no_system_files() -> SystemSources {
    SystemSources {
        resolv_conf: None,
        hosts: None,
        port: 53,
    }
}

/// FR-15 AC1/AC2: answers are reused until their TTL runs out.
#[tokio::test]
async fn answers_are_cached_until_their_ttl_expires() {
    let dns = FakeDns::start_with_ttl(1, |_| Reply::loopback()).await;
    let resolver = resolver(&only(&dns), no_system_files());

    resolver.lookup("ttl.wfl.test").await.unwrap();
    assert_eq!(a_count(&dns, "ttl.wfl.test"), 1);
    resolver.lookup("ttl.wfl.test").await.unwrap();
    assert_eq!(
        a_count(&dns, "ttl.wfl.test"),
        1,
        "the cached answer was not used"
    );

    tokio::time::sleep(Duration::from_millis(1600)).await;
    resolver.lookup("ttl.wfl.test").await.unwrap();
    assert_eq!(
        a_count(&dns, "ttl.wfl.test"),
        2,
        "an expired answer was reused"
    );
}

/// FR-15 AC1: a cached answer skips a dead first server entirely.
#[tokio::test]
async fn a_cached_answer_skips_a_dead_first_server() {
    let dead = FakeDns::start(|_| Reply::Silent).await;
    let alive = FakeDns::start(|_| Reply::loopback()).await;
    let resolver = resolver(
        &settings(&[
            ("dns_servers", &format!("{}, {}", dead.addr, alive.addr)),
            ("dns_timeout_seconds", "1"),
        ]),
        no_system_files(),
    );
    resolver.lookup("cached.wfl.test").await.unwrap();
    let dead_before = dead.query_count();

    let started = Instant::now();
    resolver.lookup("cached.wfl.test").await.unwrap();
    assert!(started.elapsed() < Duration::from_millis(300));
    assert_eq!(dead.query_count(), dead_before);
}

/// FR-15 AC2: "no such name" is cached too (briefly).
#[tokio::test]
async fn no_such_name_is_cached() {
    let dns = FakeDns::start(|_| Reply::NxDomain).await;
    let resolver = resolver(&only(&dns), no_system_files());
    assert!(resolver.lookup("nx.wfl.test").await.is_err());
    assert!(resolver.lookup("nx.wfl.test").await.is_err());
    assert_eq!(a_count(&dns, "nx.wfl.test"), 1);
}

/// FR-15 AC3 + SEC-11: the cache holds at most 1,024 names; the least recently
/// used one is dropped first.
#[tokio::test]
async fn the_cache_holds_at_most_1024_names() {
    let dns = FakeDns::start_with_ttl(3600, |_| Reply::loopback()).await;
    let resolver = resolver(&only(&dns), no_system_files());
    for i in 0..=1024 {
        resolver.lookup(&format!("n{i}.wfl.test")).await.unwrap();
    }
    // n0 was the least recently used when n1024 arrived, so it is gone...
    resolver.lookup("n0.wfl.test").await.unwrap();
    assert_eq!(a_count(&dns, "n0.wfl.test"), 2, "n0 was not evicted");
    // ...while the most recent name is still cached.
    resolver.lookup("n1024.wfl.test").await.unwrap();
    assert_eq!(
        a_count(&dns, "n1024.wfl.test"),
        1,
        "the cache did not hold n1024"
    );
}

/// FR-15 + Assumption 36: when resolv.conf changes (VPN up or down, a DHCP
/// renewal), answers cached from the old servers are dropped, so the next
/// lookup asks the new servers instead of reusing a stale answer for up to a
/// day.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn changing_resolv_conf_clears_cached_answers() {
    let (port, _servers) = FakeDns::start_group(
        3600,
        vec![
            handler(|_| Reply::v4(10, 0, 0, 1)),
            handler(|_| Reply::v4(10, 0, 0, 2)),
        ],
    )
    .await;
    let files = SystemFiles::new("nameserver 127.0.0.1\n", "");
    let resolver = resolver(&explicit_wfl(), files.sources(port));
    assert_eq!(
        resolver
            .lookup("vpn.wfl.test")
            .await
            .map_err(|e| e.to_string()),
        Ok(vec!["10.0.0.1".parse::<IpAddr>().unwrap()])
    );

    // Filesystem timestamps can be as coarse as one second.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    std::fs::write(files.resolv_conf(), "nameserver 127.0.0.2\n").unwrap();
    assert_eq!(
        resolver
            .lookup("vpn.wfl.test")
            .await
            .map_err(|e| e.to_string()),
        Ok(vec!["10.0.0.2".parse::<IpAddr>().unwrap()]),
        "a cached answer from the old resolv.conf was reused"
    );
}

/// Review round 4: a lookup that started under the OLD resolv.conf and
/// finishes after the change must not put its answer back into the cache.
/// Without that, one stale answer from the old servers survives the change.
#[cfg(target_os = "linux")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_lookup_in_flight_during_a_resolv_conf_change_is_not_cached() {
    let (port, _servers) = FakeDns::start_group(
        3600,
        vec![
            // The old server answers slowly, so its lookup is still running
            // when resolv.conf changes.
            handler(|_| Reply::delayed(2500, Reply::v4(10, 0, 0, 1))),
            handler(|_| Reply::v4(10, 0, 0, 2)),
        ],
    )
    .await;
    let files = SystemFiles::new("nameserver 127.0.0.1\noptions timeout:5\n", "");
    let resolver = resolver(&explicit_wfl(), files.sources(port));

    let old_lookup = {
        let resolver = resolver.clone();
        tokio::spawn(async move { resolver.lookup("race.wfl.test").await })
    };
    // Filesystem timestamps can be as coarse as one second.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    std::fs::write(
        files.resolv_conf(),
        "nameserver 127.0.0.2\noptions timeout:5\n",
    )
    .unwrap();
    let new_server: Vec<IpAddr> = vec!["10.0.0.2".parse().unwrap()];
    assert_eq!(
        resolver
            .lookup("race.wfl.test")
            .await
            .map_err(|e| e.to_string()),
        Ok(new_server.clone())
    );

    // The old lookup finishes after the change, with the old server's answer.
    let old = old_lookup.await.unwrap();
    assert_eq!(
        old.map_err(|e| e.to_string()),
        Ok(vec!["10.0.0.1".parse::<IpAddr>().unwrap()])
    );
    // ...but that answer must not replace the fresh one in the cache.
    assert_eq!(
        resolver
            .lookup("race.wfl.test")
            .await
            .map_err(|e| e.to_string()),
        Ok(new_server),
        "an answer from the old resolv.conf was cached after the change"
    );
}

/// NFR-02 + SEC-09: abandoning a lookup (as the request deadline does) stops
/// it across every search-list name, so a long search list cannot keep
/// sending queries after the request has given up.
///
/// This uses WFL's REAL ordered resolver against a silent server, so it also
/// proves that `dns.rs` keeps all of its per-server work inside the lookup's
/// own future: a per-server query detached with `tokio::spawn` would keep
/// sending after the drop and fail this test. Not cfg-gated: the temporary
/// resolv.conf is read on every platform, so this runs on Windows CI too
/// (`abandoning_a_request_drops_its_dns_lookup` covers the reqwest side).
#[tokio::test]
async fn abandoning_a_lookup_stops_a_long_search() {
    let silent = FakeDns::start(|_| Reply::Silent).await;
    let search = (1..=8)
        .map(|i| format!("d{i}.wfl.test"))
        .collect::<Vec<_>>()
        .join(" ");
    let files = SystemFiles::new(&format!("search {search}\n"), "");
    let resolver = resolver(
        &settings(&[
            ("dns_servers", &silent.addr.to_string()),
            ("dns_timeout_seconds", "1"),
        ]),
        files.sources(53),
    );
    let gave_up = tokio::time::timeout(Duration::from_millis(1500), resolver.lookup("app")).await;
    assert!(
        gave_up.is_err(),
        "the lookup should still have been running"
    );
    // A datagram sent just before the lookup was dropped may still be on its
    // way to the fake server's log; let it land before taking the baseline,
    // so only queries sent AFTER the drop can change the count.
    tokio::time::sleep(Duration::from_millis(300)).await;
    let ours = |name: &str| name.starts_with("app");
    let asked = silent.count_matching(ours);
    assert!(asked >= 1);
    // Long enough for several more rounds (1 s each) had the lookup survived.
    tokio::time::sleep(Duration::from_millis(3000)).await;
    assert_eq!(
        silent.count_matching(ours),
        asked,
        "queries were sent after the lookup was abandoned"
    );
}

// ---------------------------------------------------------------------------
// NFR-03: concurrency
// ---------------------------------------------------------------------------

/// NFR-03 + SEC-10: one stuck lookup does not delay an unrelated one.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_stuck_lookup_does_not_delay_others() {
    let dns = FakeDns::start(|q| {
        if q.name.starts_with("stuck") {
            Reply::Silent
        } else {
            Reply::loopback()
        }
    })
    .await;
    let resolver = resolver(
        &settings(&[
            ("dns_servers", &dns.addr.to_string()),
            ("dns_timeout_seconds", "5"),
        ]),
        no_system_files(),
    );
    let stuck = {
        let resolver = resolver.clone();
        tokio::spawn(async move { resolver.lookup("stuck.wfl.test").await })
    };
    tokio::time::sleep(Duration::from_millis(100)).await;
    let started = Instant::now();
    assert_eq!(
        resolver
            .lookup("fast.wfl.test")
            .await
            .map_err(|e| e.to_string()),
        Ok(vec![LOCAL])
    );
    assert!(started.elapsed() < Duration::from_millis(500));
    assert!(
        !stuck.is_finished(),
        "the stuck lookup should still be waiting"
    );
    stuck.abort();
}

// ---------------------------------------------------------------------------
// FR-16 on the default path, through the real binary
// ---------------------------------------------------------------------------

/// FR-16: with no DNS settings, `wfl --dump-env` on Linux shows the ordered
/// resolver and the servers it read from /etc/resolv.conf.
#[cfg(target_os = "linux")]
#[test]
fn dump_env_shows_the_linux_default() {
    let dir = TempDir::new().unwrap();
    let output = std::process::Command::new(crate::common::wfl_exe())
        .arg("--dump-env")
        .current_dir(dir.path())
        .env("WFL_GLOBAL_CONFIG_PATH", dir.path().join("none.cfg"))
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&output.stdout);
    let section = &out[out.find("=== DNS ===").expect("DNS section")..];
    assert!(section.contains("Resolver: wfl"), "{section}");
    assert!(section.contains("/etc/resolv.conf"), "{section}");
}
