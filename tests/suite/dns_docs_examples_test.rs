//! The DNS documentation must match the code (CLAUDE.md "Docs Must Be Honest";
//! PRD FR-19 AC5).
//!
//! The DNS docs contain no WFL code blocks, only `.wflcfg` snippets and an
//! example error message, which `scripts/validate_docs_examples.py` does not
//! check. These tests do: every snippet that sets a DNS key is loaded through
//! the real configuration loader, and the example error message must use the
//! exact phrases the resolver produces.

use std::path::Path;

use tempfile::TempDir;
use wfl::interpreter::dns::{DnsPlan, plan};

use crate::dns_fake_server::{FakeDns, RCODE_SERVFAIL, Reply};

const DOCS: [&str; 2] = [
    "Docs/reference/configuration-reference.md",
    "Docs/04-advanced-features/interoperability.md",
];

fn read_doc(relative: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(relative))
        .unwrap_or_else(|e| panic!("{relative}: {e}"))
}

/// The bodies of every fenced block with the given info string.
fn fenced_blocks(text: &str, info: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        match &mut current {
            None if line.trim_start() == format!("```{info}") => current = Some(String::new()),
            Some(body) if line.trim_start() == "```" => {
                blocks.push(std::mem::take(body));
                current = None;
            }
            Some(body) => {
                body.push_str(line);
                body.push('\n');
            }
            None => {}
        }
    }
    blocks
}

#[test]
fn every_dns_snippet_in_the_docs_is_a_valid_configuration() {
    let mut checked = 0;
    for doc in DOCS {
        for block in fenced_blocks(&read_doc(doc), "ini") {
            if !block.contains("dns_") {
                continue;
            }
            checked += 1;
            let dir = TempDir::new().unwrap();
            std::fs::write(dir.path().join(".wflcfg"), &block).unwrap();
            let config = wfl::config::load_local_config(dir.path());
            let mixing = block.contains("before_system") || block.contains("after_system");
            match plan(&config.dns) {
                DnsPlan::Misconfigured { message } if mixing && !cfg!(target_os = "linux") => {
                    assert!(
                        message.contains("not supported on this platform yet"),
                        "{doc}: {message}"
                    );
                }
                DnsPlan::Misconfigured { message } => {
                    panic!("{doc}: this documented snippet is invalid:\n{block}\n{message}")
                }
                DnsPlan::System { .. } | DnsPlan::Wfl { .. } => {}
            }
            for line in block.lines().filter(|l| l.trim_start().starts_with("dns_")) {
                let key = line.split('=').next().unwrap().trim();
                assert!(
                    [
                        "dns_servers",
                        "dns_servers_mode",
                        "dns_timeout_seconds",
                        "dns_resolver"
                    ]
                    .contains(&key),
                    "{doc}: unknown DNS key {key:?}"
                );
            }
        }
    }
    assert!(checked >= 4, "expected the DNS snippets, found {checked}");
}

/// The documented error message uses the resolver's real wording.
#[tokio::test]
async fn documented_error_message_matches_the_resolver() {
    let silent = FakeDns::start(|_| Reply::Silent).await;
    let failing = FakeDns::start(|_| Reply::Rcode(RCODE_SERVFAIL)).await;
    let nx = FakeDns::start(|_| Reply::NxDomain).await;

    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join(".wflcfg"),
        format!(
            "dns_servers = {}, {}, {}\ndns_timeout_seconds = 1\n",
            silent.addr, failing.addr, nx.addr
        ),
    )
    .unwrap();
    let config = wfl::config::load_local_config(dir.path());
    let resolver = wfl::interpreter::dns::OrderedResolver::from_plan_with_sources(
        &plan(&config.dns),
        wfl::interpreter::dns::SystemSources {
            resolv_conf: None,
            hosts: None,
            port: 53,
        },
    )
    .expect("WFL resolver");
    let message = resolver
        .lookup("knowledge.lan")
        .await
        .expect_err("lookup fails")
        .to_string();

    let docs = read_doc(DOCS[0]);
    let example = fenced_blocks(&docs, "text")
        .into_iter()
        .find(|block| block.contains("Could not find the address of"))
        .expect("the documented example error");
    for phrase in [
        "Could not find the address of \"knowledge.lan\".",
        "I asked these DNS servers, one at a time, in this order:",
        "no reply within",
        "answered \"server failure\"",
        "answered \"no such name\" (final)",
        "Hint: if this is an internal name, list your internal DNS servers first in .wflcfg, for example:",
    ] {
        assert!(
            message.contains(phrase),
            "the resolver no longer says {phrase:?}:\n{message}"
        );
        assert!(
            example.contains(phrase),
            "the docs no longer show {phrase:?}:\n{example}"
        );
    }
}
