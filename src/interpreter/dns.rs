//! Ordered DNS resolution for outbound HTTP requests.
//!
//! PRD: `Engineering/designs/2026-11-18-ordered-dns-resolution-design.md`.
//!
//! # Why this exists
//!
//! The Linux release artifact is a static musl binary. musl's resolver sends
//! every query to *all* nameservers at once and takes the first reply. On a
//! network whose resolver list is `10.3.0.100, 10.3.0.101, 8.8.8.8`, the
//! public server's "no such name" often wins that race, so internal names fail
//! — and every internal name leaks to the public server on every lookup.
//!
//! This module asks the servers **one at a time, in order**, with glibc's
//! rules:
//!
//! * a server's final answer (addresses, "no such name", or "no addresses")
//!   ends the search — a later server can never override it;
//! * WFL moves on to the next server only when a server does not answer, or
//!   answers "server failure", "refused", "not implemented", "format error",
//!   or something unreadable.
//!
//! Each server is queried through its own single-server hickory resolver, so
//! hickory never races or reorders servers: the loop here decides everything.
//! reqwest's built-in `hickory-dns` feature is deliberately **not** used (it
//! races two servers, reorders them by speed, and falls back to Google DNS when
//! `/etc/resolv.conf` is unreadable).
//!
//! # Configuration (`.wflcfg`)
//!
//! * `dns_servers` — IP addresses with optional ports, in the order to ask.
//! * `dns_servers_mode` — `only` (default), `before_system`, `after_system`.
//! * `dns_timeout_seconds` — whole seconds per server, 1–30.
//! * `dns_resolver` — `wfl` or `system` (the off switch).
//!
//! Invalid DNS configuration fails **closed**: every lookup reports the problem
//! and no query is sent anywhere, so a typo can never silently fall back to a
//! server the developer did not choose.

use std::collections::HashMap;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hickory_resolver::TokioResolver;
use hickory_resolver::config::{
    ConnectionConfig, NameServerConfig, ResolveHosts, ResolverConfig, ResolverOpts,
};
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::net::{DnsError, NetError};
use hickory_resolver::proto::op::ResponseCode;
use hickory_resolver::proto::rr::{RData, RecordType};

/// Wait per server when neither `.wflcfg` nor `/etc/resolv.conf` says otherwise
/// (glibc's default).
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 5;
/// Largest accepted per-server wait (glibc's own maximum, `RES_MAXRETRANS`).
pub const MAX_TIMEOUT_SECONDS: u64 = 30;
/// How many times the whole server list is tried (glibc's default).
pub const DEFAULT_ATTEMPTS: usize = 2;
/// Largest accepted number of rounds (glibc's own maximum, `RES_MAXRETRY`).
pub const MAX_ATTEMPTS: usize = 5;
/// Most servers accepted in `dns_servers`.
pub const MAX_SERVERS: usize = 32;
/// Longest accepted `dns_servers` value, in bytes.
const MAX_SERVERS_VALUE_BYTES: usize = 4096;

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// Which servers `dns_servers` replaces or joins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsServersMode {
    /// Ask only the `.wflcfg` servers (the default).
    Only,
    /// Ask the `.wflcfg` servers, then the system servers.
    BeforeSystem,
    /// Ask the system servers, then the `.wflcfg` servers.
    AfterSystem,
}

impl DnsServersMode {
    fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "only" => Some(Self::Only),
            "before_system" => Some(Self::BeforeSystem),
            "after_system" => Some(Self::AfterSystem),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Only => "only",
            Self::BeforeSystem => "before_system",
            Self::AfterSystem => "after_system",
        }
    }
}

/// `dns_resolver`: WFL's ordered resolver, or the operating system's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsResolverChoice {
    Wfl,
    System,
}

impl DnsResolverChoice {
    fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "wfl" => Some(Self::Wfl),
            "system" => Some(Self::System),
            _ => None,
        }
    }
}

/// A configuration value together with the file it came from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Setting<T> {
    #[default]
    Unset,
    Set {
        value: T,
        source: PathBuf,
    },
    /// The file holds a value WFL cannot use. Lookups fail closed.
    Invalid {
        reason: String,
        source: PathBuf,
    },
}

/// The four DNS keys, as loaded from the global and local configuration files
/// (the local file wins, key by key, like every other key).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DnsSettings {
    pub servers: Setting<Vec<SocketAddr>>,
    pub mode: Setting<DnsServersMode>,
    pub resolver: Setting<DnsResolverChoice>,
    /// `dns_timeout_seconds`, already validated (1–30), with its source file.
    pub timeout_seconds: Option<(u64, PathBuf)>,
}

impl DnsSettings {
    /// Apply one `key = value` line. Returns `false` for keys this module does
    /// not own.
    pub fn apply(&mut self, key: &str, value: &str, file: &Path) -> bool {
        let value = value.trim();
        match key {
            "dns_servers" => {
                self.servers = match parse_dns_servers(value) {
                    Ok(servers) if servers.is_empty() => Setting::Unset,
                    Ok(servers) => Setting::Set {
                        value: servers,
                        source: file.to_path_buf(),
                    },
                    Err(reason) => {
                        log::warn!("Invalid dns_servers in {}: {reason}", file.display());
                        Setting::Invalid {
                            reason,
                            source: file.to_path_buf(),
                        }
                    }
                };
            }
            "dns_servers_mode" => {
                self.mode = if value.is_empty() {
                    Setting::Unset
                } else {
                    match DnsServersMode::parse(value) {
                        Some(mode) => Setting::Set {
                            value: mode,
                            source: file.to_path_buf(),
                        },
                        None => {
                            log::warn!(
                                "Invalid dns_servers_mode '{}' in {}",
                                printable(value),
                                file.display()
                            );
                            Setting::Invalid {
                                reason: format!(
                                    "it is '{}', but it must be one of: only, before_system, after_system",
                                    printable(value)
                                ),
                                source: file.to_path_buf(),
                            }
                        }
                    }
                };
            }
            "dns_resolver" => {
                self.resolver = if value.is_empty() {
                    Setting::Unset
                } else {
                    match DnsResolverChoice::parse(value) {
                        Some(choice) => Setting::Set {
                            value: choice,
                            source: file.to_path_buf(),
                        },
                        None => {
                            log::warn!(
                                "Invalid dns_resolver '{}' in {}",
                                printable(value),
                                file.display()
                            );
                            Setting::Invalid {
                                reason: format!(
                                    "it is '{}', but it must be 'wfl' or 'system'",
                                    printable(value)
                                ),
                                source: file.to_path_buf(),
                            }
                        }
                    }
                };
            }
            "dns_timeout_seconds" => {
                self.timeout_seconds = match parse_timeout_seconds(value) {
                    Ok(seconds) => Some((seconds, file.to_path_buf())),
                    Err(reason) => {
                        // A wrong timeout cannot leak a query to an unchosen
                        // server, so this warns and uses the default instead of
                        // failing closed (FR-09 AC3).
                        log::warn!(
                            "Invalid dns_timeout_seconds in {}: {reason}; using the default",
                            file.display()
                        );
                        None
                    }
                };
            }
            _ => return false,
        }
        true
    }
}

/// Validate `dns_timeout_seconds`: a whole number from 1 to 30.
pub fn parse_timeout_seconds(value: &str) -> Result<u64, String> {
    match value.trim().parse::<u64>() {
        Ok(n) if (1..=MAX_TIMEOUT_SECONDS).contains(&n) => Ok(n),
        _ => Err(format!(
            "'{}' is not a whole number of seconds from 1 to {MAX_TIMEOUT_SECONDS}",
            printable(value)
        )),
    }
}

/// Parse a `dns_servers` value: comma-separated IP addresses, each with an
/// optional port (default 53). Duplicates are kept once, at their first
/// position. An empty value means "not set" and returns an empty list.
pub fn parse_dns_servers(value: &str) -> Result<Vec<SocketAddr>, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(Vec::new());
    }
    if value.len() > MAX_SERVERS_VALUE_BYTES {
        return Err(format!(
            "the value is {} bytes long; the limit is {MAX_SERVERS_VALUE_BYTES}",
            value.len()
        ));
    }
    if value.chars().any(char::is_control) {
        return Err("the value contains a control character".to_string());
    }
    let mut servers: Vec<SocketAddr> = Vec::new();
    for entry in value.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            return Err(
                "it has an empty entry (two commas in a row, or a comma at the start or end)"
                    .to_string(),
            );
        }
        let addr = parse_server_entry(entry)?;
        if !servers.contains(&addr) {
            servers.push(addr);
        }
        if servers.len() > MAX_SERVERS {
            return Err(format!("it lists more than {MAX_SERVERS} servers"));
        }
    }
    Ok(servers)
}

fn parse_server_entry(entry: &str) -> Result<SocketAddr, String> {
    if entry.contains('%') {
        return Err(format!(
            "'{entry}' has an IPv6 zone ID, which is not supported yet; use a global or unique-local IPv6 address instead"
        ));
    }
    if let Ok(ip) = entry.parse::<IpAddr>() {
        return Ok(SocketAddr::new(ip, 53));
    }
    if let Ok(addr) = entry.parse::<SocketAddr>() {
        if addr.port() == 0 {
            return Err(format!(
                "'{entry}' uses port 0, which is not a valid DNS port"
            ));
        }
        return Ok(addr);
    }
    let looks_like_hostname = entry
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        && entry.chars().any(|c| c.is_ascii_alphabetic());
    if looks_like_hostname {
        return Err(format!(
            "'{entry}' is a hostname, but dns_servers needs IP addresses: looking up a DNS server's own name would itself need DNS"
        ));
    }
    Err(format!(
        "'{entry}' is not an IP address with an optional port (examples: 10.3.0.100, 10.3.0.100:5353, [fd00::1]:53)"
    ))
}

/// Show a configuration value safely in a message: control characters are
/// escaped and very long values are shortened.
fn printable(value: &str) -> String {
    let escaped: String = value.chars().flat_map(char::escape_debug).collect();
    if escaped.chars().count() > 80 {
        let short: String = escaped.chars().take(77).collect();
        format!("{short}...")
    } else {
        escaped
    }
}

// ---------------------------------------------------------------------------
// The plan: which resolver, which servers
// ---------------------------------------------------------------------------

/// A DNS server WFL may ask, and the file that named it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerEntry {
    pub addr: SocketAddr,
    pub source: String,
}

/// What the DNS settings resolve to for this run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsPlan {
    /// Use the operating system's resolver (reqwest's default), exactly as
    /// before this feature.
    System { warnings: Vec<String> },
    /// Use WFL's ordered resolver.
    Wfl {
        settings: WflResolverSettings,
        warnings: Vec<String>,
    },
    /// The DNS settings are invalid: every lookup fails with this message, and
    /// no query is sent.
    Misconfigured { message: String },
}

impl DnsPlan {
    pub fn warnings(&self) -> &[String] {
        match self {
            DnsPlan::System { warnings } | DnsPlan::Wfl { warnings, .. } => warnings,
            DnsPlan::Misconfigured { .. } => &[],
        }
    }
}

/// The settings WFL's resolver runs with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WflResolverSettings {
    pub mode: DnsServersMode,
    /// Servers in the order they are asked.
    pub servers: Vec<ServerEntry>,
    pub timeout: Duration,
    /// Where the timeout came from (`None` = the built-in default).
    pub timeout_source: Option<String>,
    pub attempts: usize,
}

fn display_path(path: &Path) -> String {
    path.display().to_string()
}

fn misconfigured(key: &str, reason: &str, source: &Path) -> DnsPlan {
    DnsPlan::Misconfigured {
        message: format!(
            "The DNS setting {key} in {} is invalid: {reason}.\n\
             WFL does not fall back to other DNS servers when a DNS setting is wrong, \
             so no DNS query was sent. Fix the value in that file, or remove the line.",
            source.display()
        ),
    }
}

/// Decide which resolver to use and with which servers.
pub fn plan(settings: &DnsSettings) -> DnsPlan {
    let mut warnings = Vec::new();

    match &settings.resolver {
        Setting::Invalid { reason, source } => {
            return misconfigured("dns_resolver", reason, source);
        }
        Setting::Set {
            value: DnsResolverChoice::System,
            source,
        } => {
            if !matches!(settings.servers, Setting::Unset) {
                warnings.push(format!(
                    "dns_servers is ignored because dns_resolver = system (set in {}).",
                    source.display()
                ));
            }
            return DnsPlan::System { warnings };
        }
        _ => {}
    }

    let servers = match &settings.servers {
        Setting::Invalid { reason, source } => {
            return misconfigured("dns_servers", reason, source);
        }
        Setting::Set { value, source } => Some((value, source)),
        Setting::Unset => None,
    };

    let mode = match (&settings.mode, servers.is_some()) {
        (Setting::Invalid { reason, source }, true) => {
            return misconfigured("dns_servers_mode", reason, source);
        }
        (Setting::Set { value, .. }, true) => *value,
        (Setting::Unset, _) => DnsServersMode::Only,
        (_, false) => {
            warnings.push(
                "dns_servers_mode is set but dns_servers is not, so the mode is ignored."
                    .to_string(),
            );
            DnsServersMode::Only
        }
    };

    let explicit_wfl = matches!(
        settings.resolver,
        Setting::Set {
            value: DnsResolverChoice::Wfl,
            ..
        }
    );
    if servers.is_none() && !explicit_wfl {
        return DnsPlan::System { warnings };
    }

    let (timeout, timeout_source) = match &settings.timeout_seconds {
        Some((seconds, source)) => (Duration::from_secs(*seconds), Some(display_path(source))),
        None => (Duration::from_secs(DEFAULT_TIMEOUT_SECONDS), None),
    };

    let servers = servers
        .map(|(list, source)| {
            list.iter()
                .map(|addr| ServerEntry {
                    addr: *addr,
                    source: display_path(source),
                })
                .collect()
        })
        .unwrap_or_default();

    DnsPlan::Wfl {
        settings: WflResolverSettings {
            mode,
            servers,
            timeout,
            timeout_source,
            attempts: DEFAULT_ATTEMPTS,
        },
        warnings,
    }
}

/// The `=== DNS ===` section of `wfl --dump-env`.
pub fn describe(settings: &DnsSettings) -> String {
    let mut out = String::from("=== DNS ===\n");
    let plan = plan(settings);
    match &plan {
        DnsPlan::System { .. } => {
            out.push_str("Resolver: system (the operating system's resolver)\n");
        }
        DnsPlan::Misconfigured { message } => {
            out.push_str(
                "Resolver: wfl, but the DNS settings are invalid, so every lookup will fail:\n",
            );
            for line in message.lines() {
                out.push_str(&format!("  {line}\n"));
            }
        }
        DnsPlan::Wfl { settings, .. } => {
            out.push_str("Resolver: wfl (asks DNS servers one at a time, in order)\n");
            out.push_str(&format!("Mode: {}\n", settings.mode.as_str()));
            if settings.servers.is_empty() {
                out.push_str("Servers: none found, so every lookup will fail\n");
            } else {
                out.push_str("Servers, in the order WFL asks them:\n");
                for (i, server) in settings.servers.iter().enumerate() {
                    out.push_str(&format!(
                        "  {}. {}  (from {})\n",
                        i + 1,
                        server.addr,
                        server.source
                    ));
                }
            }
            let seconds = settings.timeout.as_secs();
            let unit = if seconds == 1 { "second" } else { "seconds" };
            match &settings.timeout_source {
                Some(source) => out.push_str(&format!(
                    "Timeout: {seconds} {unit} per server (from {source})\n"
                )),
                None => out.push_str(&format!("Timeout: {seconds} {unit} per server (default)\n")),
            }
            out.push_str(&format!(
                "Attempts: the whole list is tried up to {} times\n",
                settings.attempts
            ));
        }
    }
    if !plan.warnings().is_empty() {
        out.push_str("Warnings:\n");
        for warning in plan.warnings() {
            out.push_str(&format!("  - {warning}\n"));
        }
    }
    // Shown even when a setting is ignored, so a value inherited from the
    // global file is visible.
    if let Setting::Set { value, source } = &settings.servers
        && !matches!(plan, DnsPlan::Wfl { .. })
    {
        let list = value
            .iter()
            .map(SocketAddr::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "dns_servers (not used): {list}  (from {})\n",
            source.display()
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// The lookup error
// ---------------------------------------------------------------------------

/// A failed lookup, with a message written for the WFL program's author.
#[derive(Debug, Clone)]
pub struct DnsLookupError {
    message: String,
}

impl DnsLookupError {
    fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for DnsLookupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DnsLookupError {}

/// Find a [`DnsLookupError`] anywhere in an error's source chain.
pub fn find_lookup_error<'a>(
    error: &'a (dyn std::error::Error + 'static),
) -> Option<&'a DnsLookupError> {
    let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(error);
    while let Some(current) = cause {
        if let Some(found) = current.downcast_ref::<DnsLookupError>() {
            return Some(found);
        }
        cause = current.source();
    }
    None
}

// ---------------------------------------------------------------------------
// Per-server outcomes
// ---------------------------------------------------------------------------

/// What one query (one record type, one server) produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryOutcome {
    /// NOERROR with at least one address of the asked-for family.
    Addresses(Vec<IpAddr>),
    /// NOERROR without addresses of the asked-for family ("no data").
    NoData,
    /// NXDOMAIN: the name does not exist.
    NoSuchName,
    /// The server did not answer within the timeout.
    NoReply,
    /// A response code that means "this server cannot help; ask the next one".
    Failed(ResponseCode),
    /// The server could not be reached at all (for example, port unreachable).
    Unreachable(String),
    /// The reply could not be read as a DNS message.
    Unreadable(String),
}

impl QueryOutcome {
    fn is_final(&self) -> bool {
        matches!(
            self,
            QueryOutcome::Addresses(_) | QueryOutcome::NoData | QueryOutcome::NoSuchName
        )
    }

    fn describe(&self, timeout: Duration) -> String {
        match self {
            QueryOutcome::Addresses(_) => "answered with addresses".to_string(),
            QueryOutcome::NoData => "answered \"no addresses for this name\"".to_string(),
            QueryOutcome::NoSuchName => "answered \"no such name\"".to_string(),
            QueryOutcome::NoReply => {
                let seconds = timeout.as_secs();
                let unit = if seconds == 1 { "second" } else { "seconds" };
                format!("no reply within {seconds} {unit}")
            }
            QueryOutcome::Failed(code) => format!("answered \"{}\"", response_code_words(*code)),
            QueryOutcome::Unreachable(detail) => format!("could not be reached ({detail})"),
            QueryOutcome::Unreadable(_) => "sent a reply WFL could not read".to_string(),
        }
    }
}

fn response_code_words(code: ResponseCode) -> String {
    match code {
        ResponseCode::ServFail => "server failure".to_string(),
        ResponseCode::Refused => "refused".to_string(),
        ResponseCode::NotImp => "not implemented".to_string(),
        ResponseCode::FormErr => "format error".to_string(),
        other => format!("response code {}", u16::from(other)),
    }
}

/// Map hickory's result for one single-server query to a WFL outcome.
///
/// This mapping is the heart of the glibc rules, so it is pinned by tests
/// (FR-02 AC8): a hickory upgrade that changes how a response code surfaces
/// must fail those tests rather than silently turn, say, SERVFAIL into a final
/// answer.
pub fn classify(result: Result<Vec<IpAddr>, &NetError>) -> QueryOutcome {
    match result {
        Ok(addrs) if addrs.is_empty() => QueryOutcome::NoData,
        Ok(addrs) => QueryOutcome::Addresses(addrs),
        Err(NetError::Dns(DnsError::NoRecordsFound(no_records))) => {
            match no_records.response_code {
                ResponseCode::NXDomain => QueryOutcome::NoSuchName,
                ResponseCode::NoError => QueryOutcome::NoData,
                code => QueryOutcome::Failed(code),
            }
        }
        Err(NetError::Dns(DnsError::ResponseCode(ResponseCode::NXDomain))) => {
            QueryOutcome::NoSuchName
        }
        Err(NetError::Dns(DnsError::ResponseCode(code))) => QueryOutcome::Failed(*code),
        Err(NetError::Timeout) => QueryOutcome::NoReply,
        Err(NetError::Io(error)) => QueryOutcome::Unreachable(error.to_string()),
        Err(NetError::NoConnections) => QueryOutcome::Unreachable("no connection".to_string()),
        Err(other) => QueryOutcome::Unreadable(other.to_string()),
    }
}

/// Combine one server's A and AAAA outcomes (FR-06 AC2).
///
/// 1. Addresses for either family are final.
/// 2. Otherwise "no such name" from either query is final.
/// 3. Otherwise empty answers for both families are final ("no addresses").
/// 4. Otherwise (one family empty while the other failed, or both failed) the
///    server could not help: ask the next one. This is slightly more forgiving
///    than glibc, and it keeps an IPv4-only internal host working when a server
///    fails only for A records.
pub fn combine(v4: &QueryOutcome, v6: &QueryOutcome) -> ServerAnswer {
    let mut addrs = Vec::new();
    for outcome in [v4, v6] {
        if let QueryOutcome::Addresses(found) = outcome {
            addrs.extend(found.iter().copied());
        }
    }
    if !addrs.is_empty() {
        return ServerAnswer::Addresses(addrs);
    }
    if matches!(v4, QueryOutcome::NoSuchName) || matches!(v6, QueryOutcome::NoSuchName) {
        return ServerAnswer::NoSuchName;
    }
    if *v4 == QueryOutcome::NoData && *v6 == QueryOutcome::NoData {
        return ServerAnswer::NoAddresses;
    }
    // Report the failure that made this server unusable: IPv4 first.
    let failed = if v4.is_final() { v6 } else { v4 };
    let family = if v4.is_final() || v6.is_final() {
        if std::ptr::eq(failed, v4) {
            Some("IPv4")
        } else {
            Some("IPv6")
        }
    } else {
        None
    };
    ServerAnswer::NotFinal {
        outcome: failed.clone(),
        family,
    }
}

/// One server's combined answer for one name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerAnswer {
    Addresses(Vec<IpAddr>),
    NoSuchName,
    NoAddresses,
    /// The server could not help; `family` names the address family that
    /// failed when the other one came back empty.
    NotFinal {
        outcome: QueryOutcome,
        family: Option<&'static str>,
    },
}

impl ServerAnswer {
    fn describe(&self, timeout: Duration) -> String {
        match self {
            ServerAnswer::Addresses(_) => "answered with addresses".to_string(),
            ServerAnswer::NoSuchName => "answered \"no such name\" (final)".to_string(),
            ServerAnswer::NoAddresses => {
                "answered \"no addresses for this name\" (final)".to_string()
            }
            ServerAnswer::NotFinal { outcome, family } => match family {
                Some(family) => format!("{} for {family} addresses", outcome.describe(timeout)),
                None => outcome.describe(timeout),
            },
        }
    }
}

// ---------------------------------------------------------------------------
// The resolver
// ---------------------------------------------------------------------------

/// WFL's ordered resolver, plugged into reqwest with
/// `ClientBuilder::dns_resolver`.
#[derive(Clone)]
pub struct OrderedResolver {
    inner: Arc<Inner>,
}

impl fmt::Debug for OrderedResolver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderedResolver")
            .field("setup", &self.inner.setup)
            .field("sources", &self.inner.sources)
            .finish()
    }
}

/// Where WFL reads the operating system's DNS settings from.
///
/// Production always uses [`SystemSources::platform_default`]. The fields are
/// public so library callers (the test suite) can point a resolver at a
/// temporary `resolv.conf` and hosts file; nothing a WFL program, `.wflcfg`,
/// environment variable, or command-line flag controls can change them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemSources {
    /// The resolver configuration file: `/etc/resolv.conf` on Linux and macOS,
    /// `None` on Windows (which has no such file).
    pub resolv_conf: Option<PathBuf>,
    /// The hosts file: `/etc/hosts`, or the Windows hosts file.
    pub hosts: Option<PathBuf>,
    /// The port used for servers read from `resolv.conf`. Always 53 in
    /// production, because `resolv.conf` cannot name a port.
    pub port: u16,
}

impl SystemSources {
    /// The real system files for this platform.
    pub fn platform_default() -> Self {
        if cfg!(windows) {
            let root = std::env::var_os("SystemRoot")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
            Self {
                resolv_conf: None,
                hosts: Some(
                    root.join("System32")
                        .join("drivers")
                        .join("etc")
                        .join("hosts"),
                ),
                port: 53,
            }
        } else {
            Self {
                resolv_conf: Some(PathBuf::from("/etc/resolv.conf")),
                hosts: Some(PathBuf::from("/etc/hosts")),
                port: 53,
            }
        }
    }
}

#[derive(Debug)]
enum Setup {
    Ready(WflResolverSettings),
    Misconfigured(String),
}

struct Inner {
    setup: Setup,
    sources: SystemSources,
    /// One single-server hickory resolver per server, created on first use and
    /// reused (so TCP connections and hickory's per-server state persist).
    clients: Mutex<HashMap<SocketAddr, TokioResolver>>,
}

impl OrderedResolver {
    /// Build the resolver for a plan, reading the real system files. Returns
    /// `None` for [`DnsPlan::System`], which means "keep reqwest's default
    /// resolver".
    pub fn from_plan(plan: &DnsPlan) -> Option<Self> {
        Self::from_plan_with_sources(plan, SystemSources::platform_default())
    }

    /// Build the resolver for a plan, reading the system's DNS settings from
    /// `sources`.
    pub fn from_plan_with_sources(plan: &DnsPlan, sources: SystemSources) -> Option<Self> {
        let setup = match plan {
            DnsPlan::System { .. } => return None,
            DnsPlan::Wfl { settings, warnings } => {
                for warning in warnings {
                    log::warn!("DNS configuration: {warning}");
                }
                Setup::Ready(settings.clone())
            }
            DnsPlan::Misconfigured { message } => Setup::Misconfigured(message.clone()),
        };
        Some(Self {
            inner: Arc::new(Inner {
                setup,
                sources,
                clients: Mutex::new(HashMap::new()),
            }),
        })
    }

    /// Look up `host`, returning its addresses in the order to try them.
    pub async fn lookup(&self, host: &str) -> Result<Vec<IpAddr>, DnsLookupError> {
        self.inner.lookup(host).await
    }
}

impl reqwest::dns::Resolve for OrderedResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let inner = Arc::clone(&self.inner);
        let host = name.as_str().to_string();
        Box::pin(async move {
            let addrs = inner.lookup(&host).await?;
            let addrs: reqwest::dns::Addrs =
                Box::new(addrs.into_iter().map(|ip| SocketAddr::new(ip, 0)));
            Ok(addrs)
        })
    }
}

/// `localhost` and `*.localhost` are always loopback (RFC 6761) and are never
/// sent to a DNS server.
fn is_localhost(name: &str) -> bool {
    name == "localhost" || name.ends_with(".localhost")
}

const LOOPBACK: [IpAddr; 2] = [
    IpAddr::V4(Ipv4Addr::LOCALHOST),
    IpAddr::V6(Ipv6Addr::LOCALHOST),
];

/// One line of the "servers asked" transcript.
struct Asked {
    server: usize,
    answer: ServerAnswer,
}

impl Inner {
    async fn lookup(&self, host: &str) -> Result<Vec<IpAddr>, DnsLookupError> {
        let normalized = host.trim_end_matches('.').to_ascii_lowercase();
        if is_localhost(&normalized) {
            return Ok(LOOPBACK.to_vec());
        }
        let settings = match &self.setup {
            Setup::Misconfigured(message) => {
                return Err(DnsLookupError::new(format!(
                    "Could not look up \"{normalized}\".\n\n{message}"
                )));
            }
            Setup::Ready(settings) => settings,
        };
        if settings.servers.is_empty() {
            return Err(DnsLookupError::new(format!(
                "Could not look up \"{normalized}\": no DNS servers were found.\n\n\
                 WFL looked for servers in dns_servers (.wflcfg) and found none, \
                 so no DNS query was sent.\n\n\
                 Hint: list your DNS servers in .wflcfg, for example:\n    dns_servers = 10.3.0.100, 10.3.0.101"
            )));
        }

        let fqdn = format!("{normalized}.");
        let mut asked: Vec<Asked> = Vec::new();
        for _round in 0..settings.attempts {
            for (index, server) in settings.servers.iter().enumerate() {
                let answer = self.ask(server, &fqdn, settings).await;
                log::debug!(
                    "DNS: asked {} for {normalized}: {}",
                    server.addr,
                    answer.describe(settings.timeout)
                );
                let final_answer = !matches!(answer, ServerAnswer::NotFinal { .. });
                asked.push(Asked {
                    server: index,
                    answer: answer.clone(),
                });
                if final_answer {
                    return match answer {
                        ServerAnswer::Addresses(addrs) => Ok(addrs),
                        _ => Err(lookup_failed(&normalized, settings, &asked)),
                    };
                }
            }
        }
        Err(lookup_failed(&normalized, settings, &asked))
    }

    /// Ask one server for A and AAAA records at the same time.
    async fn ask(
        &self,
        server: &ServerEntry,
        fqdn: &str,
        settings: &WflResolverSettings,
    ) -> ServerAnswer {
        let client = match self.client_for(server.addr, settings.timeout) {
            Ok(client) => client,
            Err(error) => {
                return ServerAnswer::NotFinal {
                    outcome: QueryOutcome::Unreachable(error),
                    family: None,
                };
            }
        };
        let (v4, v6) = tokio::join!(
            query(&client, fqdn, RecordType::A, settings.timeout),
            query(&client, fqdn, RecordType::AAAA, settings.timeout),
        );
        combine(&v4, &v6)
    }

    fn client_for(&self, addr: SocketAddr, timeout: Duration) -> Result<TokioResolver, String> {
        let mut clients = self.clients.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(client) = clients.get(&addr) {
            return Ok(client.clone());
        }
        let client = single_server_resolver(addr, timeout)?;
        clients.insert(addr, client.clone());
        Ok(client)
    }
}

/// A hickory resolver that knows exactly one server and never retries, caches,
/// searches, or reads the hosts file: WFL's loop owns all of that.
fn single_server_resolver(addr: SocketAddr, timeout: Duration) -> Result<TokioResolver, String> {
    let mut udp = ConnectionConfig::udp();
    udp.port = addr.port();
    let mut tcp = ConnectionConfig::tcp();
    tcp.port = addr.port();
    // A "no such name" from the server WFL is asking is final; WFL's own loop
    // decides when to move on.
    let server = NameServerConfig::new(addr.ip(), true, vec![udp, tcp]);
    let config = ResolverConfig::from_parts(None, Vec::new(), vec![server]);
    let mut builder = TokioResolver::builder_with_config(config, TokioRuntimeProvider::default());
    let options: &mut ResolverOpts = builder.options_mut();
    options.timeout = timeout;
    options.attempts = 0;
    options.cache_size = 0;
    options.use_hosts_file = ResolveHosts::Never;
    options.ndots = 0;
    options.num_concurrent_reqs = 1;
    options.try_tcp_on_error = false;
    builder.build().map_err(|error| error.to_string())
}

/// One query for one record type. The outer timeout is a safety net in case
/// a lower layer ignores the per-request timeout.
async fn query(
    client: &TokioResolver,
    fqdn: &str,
    record_type: RecordType,
    timeout: Duration,
) -> QueryOutcome {
    let lookup = tokio::time::timeout(timeout + Duration::from_millis(250), async {
        client.lookup(fqdn, record_type).await
    })
    .await;
    match lookup {
        Err(_) => QueryOutcome::NoReply,
        Ok(Ok(found)) => {
            let addrs = found
                .answers()
                .iter()
                .filter_map(|record| match (&record.data, record_type) {
                    (RData::A(a), RecordType::A) => Some(IpAddr::V4(a.0)),
                    (RData::AAAA(aaaa), RecordType::AAAA) => Some(IpAddr::V6(aaaa.0)),
                    _ => None,
                })
                .collect();
            classify(Ok(addrs))
        }
        Ok(Err(error)) => classify(Err(&error)),
    }
}

fn lookup_failed(name: &str, settings: &WflResolverSettings, asked: &[Asked]) -> DnsLookupError {
    let rounds = asked.len().div_ceil(settings.servers.len().max(1));
    let mut message = format!("Could not find the address of \"{name}\".\n\n");
    message.push_str("I asked these DNS servers, one at a time, in this order:\n");
    // Each server once, with what it said the last time it was asked.
    let mut last: Vec<(usize, &ServerAnswer)> = Vec::new();
    for entry in asked {
        match last.iter_mut().find(|(server, _)| *server == entry.server) {
            Some(slot) => slot.1 = &entry.answer,
            None => last.push((entry.server, &entry.answer)),
        }
    }
    let width = last
        .iter()
        .map(|(server, _)| settings.servers[*server].addr.to_string().len())
        .max()
        .unwrap_or(0);
    for (position, (server, answer)) in last.iter().enumerate() {
        let entry = &settings.servers[*server];
        message.push_str(&format!(
            "  {}. {:<width$}  (from {})  {}\n",
            position + 1,
            entry.addr.to_string(),
            entry.source,
            answer.describe(settings.timeout),
        ));
    }
    let ended_with_answer = asked
        .last()
        .is_some_and(|entry| !matches!(entry.answer, ServerAnswer::NotFinal { .. }));
    if !ended_with_answer && rounds > 1 {
        message.push_str(&format!("Each server was asked up to {rounds} times.\n"));
    }
    message.push('\n');
    if ended_with_answer {
        message.push_str(
            "Hint: if this is an internal name, list your internal DNS servers first in .wflcfg, for example:\n    dns_servers = 10.3.0.100, 10.3.0.101",
        );
    } else {
        message.push_str(
            "Hint: none of these DNS servers gave an answer. Check that they are running and reachable, \
             or list other servers in .wflcfg, for example:\n    dns_servers = 10.3.0.100, 10.3.0.101",
        );
    }
    DnsLookupError::new(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v4(octets: [u8; 4]) -> IpAddr {
        IpAddr::V4(Ipv4Addr::from(octets))
    }

    #[test]
    fn server_entries_accept_addresses_with_and_without_ports() {
        let parsed =
            parse_dns_servers("10.3.0.100, 10.3.0.101:5353, fd00::1, [fd00::2]:53").unwrap();
        assert_eq!(
            parsed,
            vec![
                "10.3.0.100:53".parse().unwrap(),
                "10.3.0.101:5353".parse().unwrap(),
                "[fd00::1]:53".parse().unwrap(),
                "[fd00::2]:53".parse().unwrap(),
            ]
        );
    }

    #[test]
    fn duplicate_entries_keep_their_first_position() {
        let parsed = parse_dns_servers("10.0.0.2, 10.0.0.1, 10.0.0.2:53").unwrap();
        assert_eq!(
            parsed,
            vec![
                "10.0.0.2:53".parse().unwrap(),
                "10.0.0.1:53".parse().unwrap()
            ]
        );
    }

    #[test]
    fn invalid_entries_are_rejected_with_the_entry_named() {
        for (value, needle) in [
            ("dns.example.com", "hostname"),
            ("fe80::1%eth0", "zone ID"),
            ("10.0.0.1:0", "port 0"),
            ("10.0.0.1:65536", "10.0.0.1:65536"),
            ("10.0.0.1,", "empty entry"),
            (",10.0.0.1", "empty entry"),
            ("[::1", "[::1"),
            ("10.0.0.1\u{7}", "control character"),
        ] {
            let error = parse_dns_servers(value).expect_err(value);
            assert!(error.contains(needle), "{value:?}: {error}");
        }
        let many = (1..=33)
            .map(|i| format!("10.0.0.{i}"))
            .collect::<Vec<_>>()
            .join(",");
        assert!(
            parse_dns_servers(&many)
                .unwrap_err()
                .contains("more than 32")
        );
        assert!(
            parse_dns_servers(&"1".repeat(5000))
                .unwrap_err()
                .contains("limit")
        );
    }

    #[test]
    fn timeout_accepts_only_one_to_thirty() {
        assert_eq!(parse_timeout_seconds("1"), Ok(1));
        assert_eq!(parse_timeout_seconds(" 30 "), Ok(30));
        for bad in ["0", "31", "-1", "1.5", "soon", ""] {
            assert!(parse_timeout_seconds(bad).is_err(), "{bad:?}");
        }
    }

    /// FR-02 AC8: how each hickory error surfaces as a WFL outcome. If a
    /// hickory upgrade changes this, the glibc rules would silently change too.
    #[test]
    fn hickory_errors_map_to_glibc_outcomes() {
        use hickory_resolver::net::NoRecords;
        use hickory_resolver::proto::op::Query;

        let no_records = |code| {
            NetError::Dns(DnsError::NoRecordsFound(NoRecords::new(
                Query::default(),
                code,
            )))
        };
        let cases: Vec<(NetError, QueryOutcome, bool)> = vec![
            (
                no_records(ResponseCode::NXDomain),
                QueryOutcome::NoSuchName,
                true,
            ),
            (
                no_records(ResponseCode::NoError),
                QueryOutcome::NoData,
                true,
            ),
            (
                NetError::Dns(DnsError::ResponseCode(ResponseCode::ServFail)),
                QueryOutcome::Failed(ResponseCode::ServFail),
                false,
            ),
            (
                NetError::Dns(DnsError::ResponseCode(ResponseCode::Refused)),
                QueryOutcome::Failed(ResponseCode::Refused),
                false,
            ),
            (
                NetError::Dns(DnsError::ResponseCode(ResponseCode::NotImp)),
                QueryOutcome::Failed(ResponseCode::NotImp),
                false,
            ),
            (
                NetError::Dns(DnsError::ResponseCode(ResponseCode::FormErr)),
                QueryOutcome::Failed(ResponseCode::FormErr),
                false,
            ),
            (NetError::Timeout, QueryOutcome::NoReply, false),
        ];
        for (error, expected, is_final) in cases {
            let outcome = classify(Err(&error));
            assert_eq!(outcome, expected, "{error:?}");
            assert_eq!(outcome.is_final(), is_final, "{error:?}");
        }
        let io = NetError::Io(Arc::new(std::io::Error::from(
            std::io::ErrorKind::ConnectionRefused,
        )));
        assert!(matches!(classify(Err(&io)), QueryOutcome::Unreachable(_)));
        assert!(!classify(Err(&io)).is_final());
        let proto = NetError::Msg("malformed".to_string());
        assert!(matches!(classify(Err(&proto)), QueryOutcome::Unreadable(_)));
        assert!(!classify(Err(&proto)).is_final());
    }

    /// FR-06 AC2: the four rules for combining A and AAAA from one server.
    #[test]
    fn combining_a_and_aaaa_follows_the_four_rules() {
        let addrs = QueryOutcome::Addresses(vec![v4([10, 0, 0, 1])]);
        let servfail = QueryOutcome::Failed(ResponseCode::ServFail);
        // 1. addresses for either family are final
        assert_eq!(
            combine(&addrs, &servfail),
            ServerAnswer::Addresses(vec![v4([10, 0, 0, 1])])
        );
        assert!(matches!(
            combine(&QueryOutcome::NoReply, &addrs),
            ServerAnswer::Addresses(_)
        ));
        // 2. no such name is final
        assert_eq!(
            combine(&QueryOutcome::NoSuchName, &QueryOutcome::NoReply),
            ServerAnswer::NoSuchName
        );
        // 3. empty for both is final
        assert_eq!(
            combine(&QueryOutcome::NoData, &QueryOutcome::NoData),
            ServerAnswer::NoAddresses
        );
        // 4. one empty, the other failed: not final, and the failed family is named
        assert_eq!(
            combine(&servfail, &QueryOutcome::NoData),
            ServerAnswer::NotFinal {
                outcome: servfail.clone(),
                family: Some("IPv4"),
            }
        );
        assert_eq!(
            combine(&QueryOutcome::NoData, &QueryOutcome::NoReply),
            ServerAnswer::NotFinal {
                outcome: QueryOutcome::NoReply,
                family: Some("IPv6"),
            }
        );
        assert_eq!(
            combine(&QueryOutcome::NoReply, &servfail),
            ServerAnswer::NotFinal {
                outcome: QueryOutcome::NoReply,
                family: None,
            }
        );
    }

    fn settings_with(lines: &[(&str, &str)]) -> DnsSettings {
        let mut settings = DnsSettings::default();
        for (key, value) in lines {
            assert!(settings.apply(key, value, Path::new("/project/.wflcfg")));
        }
        settings
    }

    #[test]
    fn plan_follows_the_documented_precedence() {
        // Nothing set: the system resolver.
        assert!(matches!(
            plan(&DnsSettings::default()),
            DnsPlan::System { .. }
        ));

        // dns_servers set: WFL's resolver, mode only, servers in order.
        match plan(&settings_with(&[("dns_servers", "10.0.0.2, 10.0.0.1")])) {
            DnsPlan::Wfl { settings, warnings } => {
                assert_eq!(settings.mode, DnsServersMode::Only);
                assert_eq!(
                    settings
                        .servers
                        .iter()
                        .map(|s| s.addr.to_string())
                        .collect::<Vec<_>>(),
                    ["10.0.0.2:53", "10.0.0.1:53"]
                );
                assert_eq!(settings.servers[0].source, "/project/.wflcfg");
                assert_eq!(
                    settings.timeout,
                    Duration::from_secs(DEFAULT_TIMEOUT_SECONDS)
                );
                assert!(warnings.is_empty());
            }
            other => panic!("{other:?}"),
        }

        // The off switch wins, with a warning.
        let off = plan(&settings_with(&[
            ("dns_servers", "10.0.0.1"),
            ("dns_resolver", "system"),
        ]));
        assert!(matches!(&off, DnsPlan::System { warnings } if warnings[0].contains("ignored")));

        // Invalid values fail closed.
        for lines in [
            vec![("dns_servers", "nope")],
            vec![("dns_servers", "10.0.0.1"), ("dns_servers_mode", "x")],
            vec![("dns_resolver", "x")],
        ] {
            assert!(
                matches!(plan(&settings_with(&lines)), DnsPlan::Misconfigured { .. }),
                "{lines:?}"
            );
        }

        // A mode without servers is ignored with a warning.
        let lonely = plan(&settings_with(&[("dns_servers_mode", "before_system")]));
        assert!(lonely.warnings()[0].contains("dns_servers_mode is set"));
    }

    #[test]
    fn empty_value_clears_an_inherited_value() {
        let mut settings = settings_with(&[("dns_servers", "10.0.0.1")]);
        settings.apply("dns_servers", "", Path::new("/project/.wflcfg"));
        assert_eq!(settings.servers, Setting::Unset);
    }

    #[test]
    fn platform_default_sources_point_at_the_real_files() {
        let sources = SystemSources::platform_default();
        assert_eq!(sources.port, 53);
        let hosts = sources.hosts.expect("every platform has a hosts file");
        if cfg!(windows) {
            assert_eq!(sources.resolv_conf, None);
            assert!(
                hosts.ends_with(
                    Path::new("System32")
                        .join("drivers")
                        .join("etc")
                        .join("hosts")
                ),
                "{}",
                hosts.display()
            );
        } else {
            assert_eq!(sources.resolv_conf, Some(PathBuf::from("/etc/resolv.conf")));
            assert_eq!(hosts, PathBuf::from("/etc/hosts"));
        }
    }

    #[tokio::test]
    async fn localhost_never_needs_a_server() {
        let resolver = OrderedResolver::from_plan(&DnsPlan::Wfl {
            settings: WflResolverSettings {
                mode: DnsServersMode::Only,
                servers: Vec::new(),
                timeout: Duration::from_secs(1),
                timeout_source: None,
                attempts: 1,
            },
            warnings: Vec::new(),
        })
        .unwrap();
        for name in ["localhost", "LOCALHOST.", "api.localhost"] {
            assert_eq!(resolver.lookup(name).await.unwrap(), LOOPBACK.to_vec());
        }
    }
}
