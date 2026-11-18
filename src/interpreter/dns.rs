//! Ordered DNS resolution for outbound HTTP requests.
//!
//! PRD: `Engineering/designs/2026-11-18-ordered-dns-resolution-design.md`.
//!
//! # Why this exists
//!
//! The Linux release artifact is a static musl binary. musl's resolver sends
//! every query to *all* nameservers at once and takes the first reply. On a
//! network whose resolver list is `10.0.0.53, 10.0.0.54, 8.8.8.8`, the
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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

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
                                reason: invalid_mode_reason(value),
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
                                reason: invalid_resolver_reason(value),
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

fn invalid_mode_reason(value: &str) -> String {
    format!(
        "it is '{}', but it must be one of: only, before_system, after_system",
        printable(value)
    )
}

fn invalid_resolver_reason(value: &str) -> String {
    format!(
        "it is '{}', but it must be 'wfl' or 'system'",
        printable(value)
    )
}

/// Whether this platform supports `mode`. The mixing modes need the system's
/// own server list, which WFL reads only on Linux for now (FR-11).
fn mode_supported_here(mode: DnsServersMode) -> bool {
    mode == DnsServersMode::Only || cfg!(target_os = "linux")
}

fn unsupported_mode_reason(mode: DnsServersMode) -> String {
    format!(
        "'{}' is not supported on this platform yet; only 'only' is \
         (WFL cannot read this system's own DNS server list)",
        mode.as_str()
    )
}

/// Why a value for one of the four DNS keys is invalid, or `None` when it is
/// valid, using exactly the loader's parsers and messages. `wfl --configCheck`
/// calls this, so the checker and the loader cannot drift apart.
pub fn setting_problem(key: &str, value: &str) -> Option<String> {
    let value = value.trim();
    match key {
        "dns_servers" => parse_dns_servers(value).err(),
        "dns_timeout_seconds" => parse_timeout_seconds(value).err(),
        "dns_servers_mode" if value.is_empty() => None,
        "dns_servers_mode" => match DnsServersMode::parse(value) {
            None => Some(invalid_mode_reason(value)),
            Some(mode) if !mode_supported_here(mode) => Some(unsupported_mode_reason(mode)),
            Some(_) => None,
        },
        "dns_resolver" if value.is_empty() => None,
        "dns_resolver" => DnsResolverChoice::parse(value)
            .is_none()
            .then(|| invalid_resolver_reason(value)),
        _ => None,
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
        "'{entry}' is not an IP address with an optional port (examples: 10.0.0.53, 10.0.0.53:5353, [fd00::1]:53)"
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

/// The `.wflcfg` side of WFL's resolver. The system side (servers, search
/// list, timeout and attempts from `resolv.conf`) is read at lookup time from
/// [`SystemSources`], so edits to `/etc/resolv.conf` are picked up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WflResolverSettings {
    pub mode: DnsServersMode,
    /// Servers from `dns_servers`, in order (empty when it is not set).
    pub servers: Vec<ServerEntry>,
    /// `dns_timeout_seconds` and the file that set it.
    pub timeout_override: Option<(u64, String)>,
}

impl WflResolverSettings {
    /// Whether the system's servers take part: always when `dns_servers` is
    /// not set, and in the two mixing modes.
    fn uses_system_servers(&self) -> bool {
        self.servers.is_empty() || self.mode != DnsServersMode::Only
    }
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

/// Whether WFL's ordered resolver is the default when no DNS key is set.
///
/// Linux only: that is where the static musl artifact races nameservers.
/// Windows and macOS keep their own resolvers (macOS split DNS and the Windows
/// DNS client are things this resolver does not read).
pub const ORDERED_BY_DEFAULT: bool = cfg!(target_os = "linux");

/// Decide which resolver to use and with which `.wflcfg` servers.
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
        (Setting::Set { value, source }, true) => {
            if !mode_supported_here(*value) {
                return misconfigured("dns_servers_mode", &unsupported_mode_reason(*value), source);
            }
            *value
        }
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
    if servers.is_none() && !explicit_wfl && !ORDERED_BY_DEFAULT {
        return DnsPlan::System { warnings };
    }

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
            timeout_override: settings
                .timeout_seconds
                .as_ref()
                .map(|(seconds, source)| (*seconds, display_path(source))),
        },
        warnings,
    }
}

/// The `=== DNS ===` section of `wfl --dump-env`, read from the real system
/// files.
pub fn describe(settings: &DnsSettings) -> String {
    describe_with_sources(settings, &SystemSources::platform_default())
}

/// The `=== DNS ===` section, reading the system's DNS settings from `sources`.
pub fn describe_with_sources(settings: &DnsSettings, sources: &SystemSources) -> String {
    let mut out = String::from("=== DNS ===\n");
    let plan = plan(settings);
    let mut warnings: Vec<String> = plan.warnings().to_vec();
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
            let system = SystemConfig::read(sources);
            let effective = Effective::build(settings, &system, sources);
            warnings.extend(effective.warning.clone());
            out.push_str("Resolver: wfl (asks DNS servers one at a time, in order)\n");
            if settings.servers.is_empty() {
                out.push_str("Mode: system list (dns_servers is not set)\n");
            } else {
                out.push_str(&format!("Mode: {}\n", settings.mode.as_str()));
            }
            if effective.servers.is_empty() {
                out.push_str("Servers: none found, so every lookup will fail\n");
                for line in effective.no_servers_detail.lines() {
                    out.push_str(&format!("  {line}\n"));
                }
            } else {
                out.push_str("Servers, in the order WFL asks them:\n");
                for (i, server) in effective.servers.iter().enumerate() {
                    out.push_str(&format!(
                        "  {}. {}  (from {})\n",
                        i + 1,
                        server.addr,
                        server.source
                    ));
                }
            }
            if !effective.search.is_empty() {
                out.push_str(&format!(
                    "Search list: {} (ndots {})\n",
                    effective.search.join(" "),
                    effective.ndots
                ));
            }
            out.push_str(&format!(
                "Timeout: {} per server (from {})\n",
                seconds_words(effective.timeout.as_secs()),
                effective.timeout_source
            ));
            out.push_str(&format!(
                "Attempts: the whole list is tried up to {} times (from {})\n",
                effective.attempts, effective.attempts_source
            ));
        }
    }
    if !warnings.is_empty() {
        out.push_str("Warnings:\n");
        for warning in &warnings {
            out.push_str(&format!("  - {warning}\n"));
        }
    }
    // Shown even when the setting is not used, so a value inherited from the
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

fn seconds_words(seconds: u64) -> String {
    if seconds == 1 {
        "1 second".to_string()
    } else {
        format!("{seconds} seconds")
    }
}

// ---------------------------------------------------------------------------
// The system's own DNS files
// ---------------------------------------------------------------------------

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

/// glibc uses at most this many `nameserver` lines (`MAXNS`).
const MAX_SYSTEM_SERVERS: usize = 3;
/// glibc's cap on `options ndots:N`.
const MAX_NDOTS: usize = 15;
/// The largest system file WFL reads (resolv.conf or hosts).
const MAX_SYSTEM_FILE_BYTES: u64 = 8 * 1024 * 1024;

/// What `resolv.conf` says, as glibc would read it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SystemConfig {
    servers: Vec<IpAddr>,
    search: Vec<String>,
    ndots: usize,
    timeout: Option<u64>,
    attempts: Option<usize>,
    /// Why no servers came from the file (missing, unreadable, empty), for
    /// the "no DNS servers" error and `--dump-env`.
    problem: Option<String>,
    /// Lines WFL skipped, for `--dump-env`.
    skipped: Vec<String>,
}

impl SystemConfig {
    fn read(sources: &SystemSources) -> Self {
        let Some(path) = &sources.resolv_conf else {
            return Self {
                ndots: 1,
                problem: Some("this platform has no resolv.conf".to_string()),
                ..Self::default()
            };
        };
        match read_small_file(path) {
            Ok(text) => {
                let mut config = Self::parse(&text);
                if config.servers.is_empty() {
                    config.problem = Some(format!("{} has no nameserver lines", path.display()));
                }
                config
            }
            Err(reason) => Self {
                ndots: 1,
                problem: Some(format!("{} {reason}", path.display())),
                ..Self::default()
            },
        }
    }

    /// Parse `resolv.conf` text with glibc's rules: the first three
    /// `nameserver` lines, the last `search` or `domain` line, and `options
    /// ndots/timeout/attempts` capped at glibc's maximums. `#` and `;` start
    /// comments.
    fn parse(text: &str) -> Self {
        let mut config = Self {
            ndots: 1,
            ..Self::default()
        };
        for raw in text.lines() {
            let line = raw.split(['#', ';']).next().unwrap_or("").trim();
            let mut words = line.split_whitespace();
            match words.next() {
                Some("nameserver") => {
                    let Some(value) = words.next() else { continue };
                    match value.parse::<IpAddr>() {
                        Ok(ip) if config.servers.len() < MAX_SYSTEM_SERVERS => {
                            if !config.servers.contains(&ip) {
                                config.servers.push(ip);
                            }
                        }
                        Ok(_) => config.skipped.push(format!(
                            "nameserver {value} (glibc uses only the first {MAX_SYSTEM_SERVERS})"
                        )),
                        Err(_) => config.skipped.push(format!(
                            "nameserver {} (not an IP address WFL can use)",
                            printable(value)
                        )),
                    }
                }
                Some("search") | Some("domain") => {
                    config.search = words
                        .map(|domain| domain.trim_matches('.').to_ascii_lowercase())
                        .filter(|domain| !domain.is_empty())
                        .collect();
                }
                Some("options") => {
                    for option in words {
                        if let Some(n) = option.strip_prefix("ndots:") {
                            if let Ok(n) = n.parse::<usize>() {
                                config.ndots = n.min(MAX_NDOTS);
                            }
                        } else if let Some(n) = option.strip_prefix("timeout:") {
                            if let Ok(n) = n.parse::<u64>() {
                                config.timeout = Some(n.clamp(1, MAX_TIMEOUT_SECONDS));
                            }
                        } else if let Some(n) = option.strip_prefix("attempts:")
                            && let Ok(n) = n.parse::<usize>()
                        {
                            config.attempts = Some(n.clamp(1, MAX_ATTEMPTS));
                        }
                    }
                }
                _ => {}
            }
        }
        config
    }
}

/// Read a system file, refusing absurdly large ones.
fn read_small_file(path: &Path) -> Result<String, String> {
    let metadata = std::fs::metadata(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => "does not exist".to_string(),
        _ => format!("could not be read ({error})"),
    })?;
    if !metadata.is_file() {
        return Err("is not a file".to_string());
    }
    if metadata.len() > MAX_SYSTEM_FILE_BYTES {
        return Err(format!(
            "is larger than {} MiB",
            MAX_SYSTEM_FILE_BYTES / (1024 * 1024)
        ));
    }
    std::fs::read_to_string(path).map_err(|error| format!("could not be read ({error})"))
}

/// A file's identity for change detection: modification time and size.
fn file_stamp(path: &Path) -> Option<(SystemTime, u64)> {
    let metadata = std::fs::metadata(path).ok()?;
    Some((metadata.modified().ok()?, metadata.len()))
}

/// A parsed file, re-read when its modification time or size changes.
struct Watched<T> {
    stamp: Option<(SystemTime, u64)>,
    value: Arc<T>,
}

/// The hosts file: lower-cased name → addresses in file order.
#[derive(Debug, Default)]
struct HostsFile {
    names: HashMap<String, Vec<IpAddr>>,
}

impl HostsFile {
    fn read(path: &Path) -> Self {
        match read_small_file(path) {
            Ok(text) => Self::parse(&text),
            Err(_) => Self::default(),
        }
    }

    /// `address name [aliases...]` per line; `#` starts a comment; lines whose
    /// first field is not an IP address are skipped.
    fn parse(text: &str) -> Self {
        let mut hosts = Self::default();
        for raw in text.lines() {
            let line = raw.split('#').next().unwrap_or("").trim();
            let mut fields = line.split_whitespace();
            let Some(Ok(addr)) = fields.next().map(str::parse::<IpAddr>) else {
                continue;
            };
            for name in fields {
                let name = name.trim_end_matches('.').to_ascii_lowercase();
                let entry = hosts.names.entry(name).or_default();
                if !entry.contains(&addr) {
                    entry.push(addr);
                }
            }
        }
        hosts
    }
}

/// Everything one lookup needs: the servers in order, the search list, and the
/// timing, with where each came from.
#[derive(Debug, Clone)]
struct Effective {
    servers: Vec<ServerEntry>,
    search: Vec<String>,
    ndots: usize,
    timeout: Duration,
    timeout_source: String,
    attempts: usize,
    attempts_source: String,
    /// Shown when the server list is empty.
    no_servers_detail: String,
    /// Set when a mixing mode found no system servers (FR-12 AC4).
    warning: Option<String>,
}

impl Effective {
    fn build(
        settings: &WflResolverSettings,
        system: &SystemConfig,
        sources: &SystemSources,
    ) -> Self {
        let resolv_name = sources
            .resolv_conf
            .as_deref()
            .map(display_path)
            .unwrap_or_else(|| "resolv.conf".to_string());
        let system_servers: Vec<ServerEntry> = if settings.uses_system_servers() {
            system
                .servers
                .iter()
                .map(|ip| ServerEntry {
                    addr: SocketAddr::new(*ip, sources.port),
                    source: resolv_name.clone(),
                })
                .collect()
        } else {
            Vec::new()
        };
        let ordered: Vec<ServerEntry> = match settings.mode {
            DnsServersMode::AfterSystem => system_servers
                .iter()
                .chain(settings.servers.iter())
                .cloned()
                .collect(),
            DnsServersMode::Only | DnsServersMode::BeforeSystem => settings
                .servers
                .iter()
                .chain(system_servers.iter())
                .cloned()
                .collect(),
        };
        let mut servers: Vec<ServerEntry> = Vec::new();
        for entry in ordered {
            if !servers.iter().any(|seen| seen.addr == entry.addr) {
                servers.push(entry);
            }
        }

        let warning = (!settings.servers.is_empty()
            && settings.mode != DnsServersMode::Only
            && system.servers.is_empty())
        .then(|| {
            format!(
                "dns_servers_mode = {} found no system DNS servers ({}), so only dns_servers are used.",
                settings.mode.as_str(),
                system.problem.as_deref().unwrap_or("none listed")
            )
        });

        let mut no_servers_detail = String::from("WFL looked for DNS servers in:\n");
        no_servers_detail.push_str("  - dns_servers in .wflcfg: not set\n");
        no_servers_detail.push_str(&format!(
            "  - {}\n",
            system
                .problem
                .clone()
                .unwrap_or_else(|| format!("{resolv_name}: no usable servers"))
        ));

        let (timeout, timeout_source) = match (&settings.timeout_override, system.timeout) {
            (Some((seconds, source)), _) => (*seconds, source.clone()),
            (None, Some(seconds)) => (seconds, resolv_name.clone()),
            (None, None) => (DEFAULT_TIMEOUT_SECONDS, "the default".to_string()),
        };
        let (attempts, attempts_source) = match system.attempts {
            Some(attempts) => (attempts, resolv_name.clone()),
            None => (DEFAULT_ATTEMPTS, "the default".to_string()),
        };

        Self {
            servers,
            search: system.search.clone(),
            ndots: system.ndots,
            timeout: Duration::from_secs(timeout),
            timeout_source,
            attempts,
            attempts_source,
            no_servers_detail,
            warning,
        }
    }
}

/// The names to try for `name`, in glibc's order (FR-04): a name with a
/// trailing dot is absolute; a name with at least `ndots` dots is tried as-is
/// first and then with each search domain; otherwise the search domains come
/// first and the bare name last.
fn candidate_names(name: &str, absolute: bool, search: &[String], ndots: usize) -> Vec<String> {
    if absolute || search.is_empty() {
        return vec![name.to_string()];
    }
    let searched = search
        .iter()
        .map(|domain| format!("{name}.{domain}"))
        // A DNS name is at most 253 characters.
        .filter(|candidate| candidate.len() <= 253);
    if name.matches('.').count() >= ndots {
        std::iter::once(name.to_string()).chain(searched).collect()
    } else {
        searched.chain(std::iter::once(name.to_string())).collect()
    }
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
// The answer cache (FR-15)
// ---------------------------------------------------------------------------

/// Most names the cache holds; the least recently used one is dropped first.
pub const CACHE_CAPACITY: usize = 1024;
/// Longest time a positive answer is reused, whatever its TTL says.
pub const MAX_POSITIVE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
/// Longest time a "no such name" / "no addresses" answer is reused.
pub const MAX_NEGATIVE_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
enum Cached {
    Found(Vec<IpAddr>),
    Failed(DnsLookupError),
}

struct CacheEntry {
    value: Cached,
    expires: Instant,
    last_used: u64,
}

/// A small in-memory LRU cache of final answers, keyed by the name as asked.
#[derive(Default)]
struct AnswerCache {
    entries: HashMap<String, CacheEntry>,
    clock: u64,
}

impl AnswerCache {
    fn get(&mut self, key: &str, now: Instant) -> Option<Cached> {
        self.clock += 1;
        let clock = self.clock;
        match self.entries.get_mut(key) {
            Some(entry) if entry.expires > now => {
                entry.last_used = clock;
                Some(entry.value.clone())
            }
            Some(_) => {
                self.entries.remove(key);
                None
            }
            None => None,
        }
    }

    fn insert(&mut self, key: String, value: Cached, expires: Instant) {
        self.clock += 1;
        if !self.entries.contains_key(&key) && self.entries.len() >= CACHE_CAPACITY {
            let oldest = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(name, _)| name.clone());
            if let Some(oldest) = oldest {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(
            key,
            CacheEntry {
                value,
                expires,
                last_used: self.clock,
            },
        );
    }
}

/// The earlier of two optional expiry times.
fn earliest(a: Option<Instant>, b: Option<Instant>) -> Option<Instant> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (one, None) | (None, one) => one,
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

#[derive(Debug)]
enum Setup {
    Ready(WflResolverSettings),
    Misconfigured(String),
}

struct Inner {
    setup: Setup,
    sources: SystemSources,
    /// One single-server hickory resolver per (server, timeout), created on
    /// first use and reused, so TCP connections and hickory's per-server state
    /// persist.
    clients: Mutex<HashMap<(SocketAddr, Duration), TokioResolver>>,
    system: Mutex<Option<Watched<SystemConfig>>>,
    hosts: Mutex<Option<Watched<HostsFile>>>,
    cache: Mutex<AnswerCache>,
    /// The "mixing mode found no system servers" warning is logged once.
    warned_no_system_servers: AtomicBool,
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
                system: Mutex::new(None),
                hosts: Mutex::new(None),
                cache: Mutex::new(AnswerCache::default()),
                warned_no_system_servers: AtomicBool::new(false),
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
#[derive(Debug, Clone)]
struct Asked {
    server: usize,
    answer: ServerAnswer,
}

/// How the search for one candidate name ended.
#[derive(Debug, Clone)]
enum CandidateEnd {
    Found(Vec<IpAddr>),
    NoSuchName,
    NoAddresses,
    /// No server gave a final answer. `all_silent` means none replied at all.
    Unresolved {
        all_silent: bool,
    },
}

/// One candidate name and what happened to it, for the error message.
struct Tried {
    name: String,
    asked: Vec<Asked>,
    end: CandidateEnd,
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Inner {
    async fn lookup(&self, host: &str) -> Result<Vec<IpAddr>, DnsLookupError> {
        let absolute = host.ends_with('.');
        let normalized = host.trim_end_matches('.').to_ascii_lowercase();
        if normalized.is_empty() {
            return Err(DnsLookupError::new(
                "Cannot look up an empty name.".to_string(),
            ));
        }
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

        // The hosts file comes first, and is re-read when it changes.
        if let Some(addrs) = self.hosts_file().names.get(&normalized) {
            return Ok(addrs.clone());
        }

        let cache_key = if absolute {
            format!("{normalized}.")
        } else {
            normalized.clone()
        };
        if let Some(cached) = lock(&self.cache).get(&cache_key, Instant::now()) {
            return match cached {
                Cached::Found(addrs) => Ok(addrs),
                Cached::Failed(error) => Err(error),
            };
        }

        let system = self.system_config();
        let effective = Effective::build(settings, &system, &self.sources);
        if let Some(warning) = &effective.warning
            && !self.warned_no_system_servers.swap(true, Ordering::Relaxed)
        {
            log::warn!("DNS configuration: {warning}");
        }
        if effective.servers.is_empty() {
            return Err(DnsLookupError::new(format!(
                "Could not look up \"{normalized}\": no DNS servers were found, so no DNS query was sent.\n\n\
                 {}\n\
                 Hint: list your DNS servers in .wflcfg, for example:\n    dns_servers = 10.0.0.53, 10.0.0.54\n\
                 If a DNS resolver runs on this machine, use:\n    dns_servers = 127.0.0.1",
                effective.no_servers_detail
            )));
        }

        let mut tried: Vec<Tried> = Vec::new();
        let mut negative_expiry = Some(Instant::now() + MAX_NEGATIVE_TTL);
        for candidate in candidate_names(&normalized, absolute, &effective.search, effective.ndots)
        {
            let (end, asked, expiry) = self.search_one(&candidate, &effective).await;
            match &end {
                CandidateEnd::Found(addrs) => {
                    if let Some(expires) = expiry {
                        lock(&self.cache).insert(cache_key, Cached::Found(addrs.clone()), expires);
                    }
                    return Ok(addrs.clone());
                }
                CandidateEnd::NoSuchName | CandidateEnd::NoAddresses => {
                    negative_expiry = match (negative_expiry, expiry) {
                        (Some(so_far), Some(this)) => Some(so_far.min(this)),
                        _ => None,
                    };
                }
                CandidateEnd::Unresolved { .. } => negative_expiry = None,
            }
            let stop = matches!(end, CandidateEnd::Unresolved { all_silent: true });
            tried.push(Tried {
                name: candidate,
                asked,
                end,
            });
            // glibc stops searching when every server timed out.
            if stop {
                break;
            }
        }

        let error = lookup_failed(&normalized, &effective, &tried);
        if let Some(expires) = negative_expiry {
            lock(&self.cache).insert(cache_key, Cached::Failed(error.clone()), expires);
        }
        Err(error)
    }

    /// Ask every server, in order, for one candidate name, `attempts` times
    /// over, until one gives a final answer.
    async fn search_one(
        &self,
        candidate: &str,
        effective: &Effective,
    ) -> (CandidateEnd, Vec<Asked>, Option<Instant>) {
        let fqdn = format!("{candidate}.");
        let mut asked: Vec<Asked> = Vec::new();
        for _round in 0..effective.attempts {
            for (index, server) in effective.servers.iter().enumerate() {
                let (answer, expiry) = self.ask(server, &fqdn, effective.timeout).await;
                log::debug!(
                    "DNS: asked {} for {candidate}: {}",
                    server.addr,
                    answer.describe(effective.timeout)
                );
                asked.push(Asked {
                    server: index,
                    answer: answer.clone(),
                });
                let end = match answer {
                    ServerAnswer::Addresses(addrs) => CandidateEnd::Found(addrs),
                    ServerAnswer::NoSuchName => CandidateEnd::NoSuchName,
                    ServerAnswer::NoAddresses => CandidateEnd::NoAddresses,
                    ServerAnswer::NotFinal { .. } => continue,
                };
                return (end, asked, expiry);
            }
        }
        let all_silent = asked.iter().all(|entry| {
            matches!(
                entry.answer,
                ServerAnswer::NotFinal {
                    outcome: QueryOutcome::NoReply,
                    family: None
                }
            )
        });
        (CandidateEnd::Unresolved { all_silent }, asked, None)
    }

    /// Ask one server for A and AAAA records at the same time.
    async fn ask(
        &self,
        server: &ServerEntry,
        fqdn: &str,
        timeout: Duration,
    ) -> (ServerAnswer, Option<Instant>) {
        let client = match self.client_for(server.addr, timeout) {
            Ok(client) => client,
            Err(error) => {
                return (
                    ServerAnswer::NotFinal {
                        outcome: QueryOutcome::Unreachable(error),
                        family: None,
                    },
                    None,
                );
            }
        };
        let ((v4, v4_expiry), (v6, v6_expiry)) = tokio::join!(
            query(&client, fqdn, RecordType::A, timeout),
            query(&client, fqdn, RecordType::AAAA, timeout),
        );
        (combine(&v4, &v6), earliest(v4_expiry, v6_expiry))
    }

    fn client_for(&self, addr: SocketAddr, timeout: Duration) -> Result<TokioResolver, String> {
        let mut clients = lock(&self.clients);
        if let Some(client) = clients.get(&(addr, timeout)) {
            return Ok(client.clone());
        }
        let client = single_server_resolver(addr, timeout)?;
        clients.insert((addr, timeout), client.clone());
        Ok(client)
    }

    /// `resolv.conf`, re-read when it changes.
    fn system_config(&self) -> Arc<SystemConfig> {
        let stamp = self.sources.resolv_conf.as_deref().and_then(file_stamp);
        let mut slot = lock(&self.system);
        if let Some(watched) = slot.as_ref()
            && watched.stamp == stamp
        {
            return Arc::clone(&watched.value);
        }
        let value = Arc::new(SystemConfig::read(&self.sources));
        *slot = Some(Watched {
            stamp,
            value: Arc::clone(&value),
        });
        value
    }

    /// The hosts file, re-read when it changes.
    fn hosts_file(&self) -> Arc<HostsFile> {
        let Some(path) = self.sources.hosts.as_deref() else {
            return Arc::new(HostsFile::default());
        };
        let stamp = file_stamp(path);
        let mut slot = lock(&self.hosts);
        if let Some(watched) = slot.as_ref()
            && watched.stamp == stamp
        {
            return Arc::clone(&watched.value);
        }
        let value = Arc::new(HostsFile::read(path));
        *slot = Some(Watched {
            stamp,
            value: Arc::clone(&value),
        });
        value
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

/// One query for one record type, and until when its answer may be cached.
/// The outer timeout is a safety net in case a lower layer ignores the
/// per-request timeout.
async fn query(
    client: &TokioResolver,
    fqdn: &str,
    record_type: RecordType,
    timeout: Duration,
) -> (QueryOutcome, Option<Instant>) {
    let lookup = tokio::time::timeout(timeout + Duration::from_millis(250), async {
        client.lookup(fqdn, record_type).await
    })
    .await;
    let now = Instant::now();
    match lookup {
        Err(_) => (QueryOutcome::NoReply, None),
        Ok(Ok(found)) => {
            let addrs: Vec<IpAddr> = found
                .answers()
                .iter()
                .filter_map(|record| match (&record.data, record_type) {
                    (RData::A(a), RecordType::A) => Some(IpAddr::V4(a.0)),
                    (RData::AAAA(aaaa), RecordType::AAAA) => Some(IpAddr::V6(aaaa.0)),
                    _ => None,
                })
                .collect();
            let cap = if addrs.is_empty() {
                MAX_NEGATIVE_TTL
            } else {
                MAX_POSITIVE_TTL
            };
            let expiry = found.valid_until().min(now + cap);
            (classify(Ok(addrs)), Some(expiry))
        }
        Ok(Err(error)) => {
            let expiry = match &error {
                NetError::Dns(DnsError::NoRecordsFound(no_records)) => no_records
                    .negative_ttl
                    .map(|ttl| now + Duration::from_secs(u64::from(ttl)).min(MAX_NEGATIVE_TTL)),
                _ => None,
            };
            (classify(Err(&error)), expiry)
        }
    }
}

/// Write the "servers asked" lines for one candidate name.
fn write_asked(message: &mut String, effective: &Effective, asked: &[Asked], indent: &str) {
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
        .map(|(server, _)| effective.servers[*server].addr.to_string().len())
        .max()
        .unwrap_or(0);
    for (position, (server, answer)) in last.iter().enumerate() {
        let entry = &effective.servers[*server];
        message.push_str(&format!(
            "{indent}{}. {:<width$}  (from {})  {}\n",
            position + 1,
            entry.addr.to_string(),
            entry.source,
            answer.describe(effective.timeout),
        ));
    }
    let rounds = asked.len().div_ceil(effective.servers.len().max(1));
    let unresolved = asked
        .last()
        .is_none_or(|entry| matches!(entry.answer, ServerAnswer::NotFinal { .. }));
    if unresolved && rounds > 1 {
        message.push_str(&format!(
            "{indent}Each server was asked up to {rounds} times.\n"
        ));
    }
}

fn lookup_failed(name: &str, effective: &Effective, tried: &[Tried]) -> DnsLookupError {
    let mut message = format!("Could not find the address of \"{name}\".\n\n");
    match tried {
        [only] => {
            message.push_str("I asked these DNS servers, one at a time, in this order:\n");
            write_asked(&mut message, effective, &only.asked, "  ");
        }
        _ => {
            message.push_str(&format!(
                "I tried {} names, in this order (the search list comes from resolv.conf):\n",
                tried.len()
            ));
            for candidate in tried {
                message.push_str(&format!(
                    "\n  {}\n  I asked these DNS servers, one at a time, in this order:\n",
                    candidate.name
                ));
                write_asked(&mut message, effective, &candidate.asked, "    ");
            }
        }
    }
    message.push('\n');
    let all_answered = tried
        .iter()
        .all(|candidate| !matches!(candidate.end, CandidateEnd::Unresolved { .. }));
    if all_answered {
        message.push_str(
            "Hint: if this is an internal name, list your internal DNS servers first in .wflcfg, for example:\n    dns_servers = 10.0.0.53, 10.0.0.54",
        );
    } else {
        message.push_str(
            "Hint: none of these DNS servers gave an answer. Check that they are running and reachable, \
             or list other servers in .wflcfg, for example:\n    dns_servers = 10.0.0.53, 10.0.0.54",
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
        let parsed = parse_dns_servers("10.0.0.53, 10.0.0.54:5353, fd00::1, [fd00::2]:53").unwrap();
        assert_eq!(
            parsed,
            vec![
                "10.0.0.53:53".parse().unwrap(),
                "10.0.0.54:5353".parse().unwrap(),
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
        // Nothing set: WFL's resolver on Linux, the system's elsewhere.
        assert_eq!(
            matches!(plan(&DnsSettings::default()), DnsPlan::Wfl { .. }),
            ORDERED_BY_DEFAULT
        );

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
                assert_eq!(settings.timeout_override, None);
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

    /// `wfl --configCheck` (through `setting_problem`) rejects exactly what
    /// the loader rejects.
    #[test]
    fn checker_rules_match_the_loader() {
        for (key, value) in [
            ("dns_servers", "10.0.0.2"),
            ("dns_servers", "nope"),
            ("dns_servers", "10.0.0.2,,10.0.0.3"),
            ("dns_servers_mode", "only"),
            ("dns_servers_mode", "BEFORE_SYSTEM"),
            ("dns_servers_mode", "after_system"),
            ("dns_servers_mode", "sometimes"),
            ("dns_resolver", "wfl"),
            ("dns_resolver", "System"),
            ("dns_resolver", "magic"),
            ("dns_timeout_seconds", "3"),
            ("dns_timeout_seconds", "31"),
        ] {
            let mut settings = DnsSettings::default();
            // A server is set first, so a mode or resolver value is not ignored.
            settings.apply("dns_servers", "10.0.0.1", Path::new("/p/.wflcfg"));
            settings.apply(key, value, Path::new("/p/.wflcfg"));
            let loader_rejects = match key {
                "dns_timeout_seconds" => settings.timeout_seconds.is_none(),
                _ => matches!(plan(&settings), DnsPlan::Misconfigured { .. }),
            };
            assert_eq!(
                setting_problem(key, value).is_some(),
                loader_rejects,
                "{key} = {value}"
            );
        }
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
                timeout_override: None,
            },
            warnings: Vec::new(),
        })
        .unwrap();
        for name in ["localhost", "LOCALHOST.", "api.localhost"] {
            assert_eq!(resolver.lookup(name).await.unwrap(), LOOPBACK.to_vec());
        }
    }

    #[test]
    fn resolv_conf_is_read_with_glibc_rules() {
        let config = SystemConfig::parse(
            "# comment\n\
             ; another comment\n\
             nameserver 10.0.0.53\n\
             nameserver 10.0.0.54   # trailing comment\n\
             nameserver 10.0.0.53\n\
             nameserver fe80::1%eth0\n\
             nameserver 8.8.8.8\n\
             nameserver 1.1.1.1\n\
             search old.test\n\
             domain corp.test\n\
             search lab.test . corp.test.\n\
             options ndots:99 timeout:0 attempts:9 rotate edns0\n",
        );
        // glibc (MAXNS = 3) counts the first three usable `nameserver`
        // lines, duplicates included, so the repeated 10.0.0.53 uses up the
        // third slot and 8.8.8.8 is never asked. The zone-ID line is not
        // usable, so it does not count.
        assert_eq!(
            config.servers,
            ["10.0.0.53", "10.0.0.54"]
                .map(|ip| ip.parse::<IpAddr>().unwrap())
                .to_vec()
        );
        assert!(
            config
                .skipped
                .iter()
                .any(|line| line.contains("fe80::1%eth0"))
        );
        assert!(config.skipped.iter().any(|line| line.contains("8.8.8.8")));
        assert!(config.skipped.iter().any(|line| line.contains("1.1.1.1")));
        // The last search/domain line wins; "." entries are dropped.
        assert_eq!(config.search, ["lab.test", "corp.test"]);
        // Options are capped at glibc's limits.
        assert_eq!(config.ndots, MAX_NDOTS);
        assert_eq!(config.timeout, Some(1));
        assert_eq!(config.attempts, Some(MAX_ATTEMPTS));

        let defaults = SystemConfig::parse("");
        assert_eq!(defaults.ndots, 1);
        assert_eq!(defaults.timeout, None);
        assert_eq!(defaults.attempts, None);
    }

    #[test]
    fn candidate_names_follow_glibc_order() {
        let search = ["corp.test".to_string(), "lab.test".to_string()];
        assert_eq!(
            candidate_names("app", false, &search, 1),
            ["app.corp.test", "app.lab.test", "app"]
        );
        assert_eq!(
            candidate_names("db.internal", false, &search, 1),
            [
                "db.internal",
                "db.internal.corp.test",
                "db.internal.lab.test"
            ]
        );
        assert_eq!(
            candidate_names("db.internal", false, &search, 2),
            [
                "db.internal.corp.test",
                "db.internal.lab.test",
                "db.internal"
            ]
        );
        assert_eq!(candidate_names("app", true, &search, 1), ["app"]);
        assert_eq!(candidate_names("app", false, &[], 1), ["app"]);
        // At most six search domains are used (glibc's classic MAXDNSRCH), so
        // a long search list cannot multiply the worst-case wait without bound.
        let many: Vec<String> = (1..=8).map(|i| format!("d{i}.test")).collect();
        let tried = candidate_names("app", false, &many, 1);
        assert_eq!(tried.len(), 7, "{tried:?}");
        assert_eq!(tried[5], "app.d6.test");
        assert_eq!(tried[6], "app");
        let long = "x".repeat(250);
        assert_eq!(
            candidate_names(&long, false, &search, 1),
            std::slice::from_ref(&long)
        );
    }

    /// A hosts file that is not valid UTF-8 must still be read: one Latin-1
    /// byte in a comment, or a UTF-16 file saved by a Windows editor, must
    /// not silently send every pinned name to DNS instead.
    #[test]
    fn hosts_file_bytes_are_decoded_leniently() {
        let read = |bytes: &[u8]| {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("hosts");
            std::fs::write(&path, bytes).unwrap();
            HostsFile::read(&path)
        };
        let latin1 = read(b"# caf\xe9 comment\n10.0.0.7 latin.test\n");
        assert_eq!(
            latin1.names.get("latin.test"),
            Some(&vec!["10.0.0.7".parse::<IpAddr>().unwrap()])
        );
        let mut utf16 = vec![0xFF, 0xFE];
        for unit in "10.0.0.8 wide.test\r\n".encode_utf16() {
            utf16.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(
            read(&utf16).names.get("wide.test"),
            Some(&vec!["10.0.0.8".parse::<IpAddr>().unwrap()])
        );
        let utf8_bom = read(b"\xef\xbb\xbf10.0.0.9 bom.test\n");
        assert_eq!(
            utf8_bom.names.get("bom.test"),
            Some(&vec!["10.0.0.9".parse::<IpAddr>().unwrap()])
        );
    }

    #[test]
    fn hosts_file_is_parsed_like_libc() {
        let hosts = HostsFile::parse(
            "# comment\n\
             127.0.0.1 localhost\n\
             10.0.0.5\tCanonical.Test alias.test.  # trailing\n\
             10.0.0.6 alias.test\n\
             10.0.0.5 alias.test\n\
             fd00::5 alias.test\n\
             not-an-ip ignored.test\n",
        );
        assert_eq!(
            hosts.names["alias.test"],
            ["10.0.0.5", "10.0.0.6", "fd00::5"]
                .map(|ip| ip.parse::<IpAddr>().unwrap())
                .to_vec()
        );
        assert_eq!(hosts.names["canonical.test"].len(), 1);
        assert!(!hosts.names.contains_key("ignored.test"));
    }

    #[test]
    fn cache_expires_entries_and_drops_the_least_recently_used() {
        let now = Instant::now();
        let later = now + Duration::from_secs(60);
        let mut cache = AnswerCache::default();
        cache.insert("a".into(), Cached::Found(vec![v4([10, 0, 0, 1])]), later);
        assert!(matches!(cache.get("a", now), Some(Cached::Found(_))));
        assert!(cache.get("a", later + Duration::from_secs(1)).is_none());
        assert!(cache.entries.is_empty(), "an expired entry was kept");

        let mut cache = AnswerCache::default();
        for i in 0..CACHE_CAPACITY {
            cache.insert(format!("n{i}"), Cached::Found(Vec::new()), later);
        }
        // Touch n0, so n1 becomes the least recently used.
        assert!(cache.get("n0", now).is_some());
        cache.insert("new".into(), Cached::Found(Vec::new()), later);
        assert_eq!(cache.entries.len(), CACHE_CAPACITY);
        assert!(cache.entries.contains_key("n0"));
        assert!(!cache.entries.contains_key("n1"));
        assert!(cache.entries.contains_key("new"));
    }

    #[test]
    fn effective_servers_follow_the_mode_and_drop_duplicates() {
        let sources = SystemSources {
            resolv_conf: Some(PathBuf::from("/etc/resolv.conf")),
            hosts: None,
            port: 53,
        };
        let system = SystemConfig::parse(
            "nameserver 10.0.0.9\nnameserver 10.0.0.1\noptions timeout:3 attempts:4\n",
        );
        let mine = |mode| WflResolverSettings {
            mode,
            servers: vec![ServerEntry {
                addr: "10.0.0.1:53".parse().unwrap(),
                source: "/p/.wflcfg".into(),
            }],
            timeout_override: None,
        };
        let order = |settings: &WflResolverSettings| {
            Effective::build(settings, &system, &sources)
                .servers
                .iter()
                .map(|s| format!("{} {}", s.addr, s.source))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            order(&mine(DnsServersMode::Only)),
            ["10.0.0.1:53 /p/.wflcfg"]
        );
        assert_eq!(
            order(&mine(DnsServersMode::BeforeSystem)),
            ["10.0.0.1:53 /p/.wflcfg", "10.0.0.9:53 /etc/resolv.conf"]
        );
        assert_eq!(
            order(&mine(DnsServersMode::AfterSystem)),
            [
                "10.0.0.9:53 /etc/resolv.conf",
                "10.0.0.1:53 /etc/resolv.conf"
            ]
        );
        let none = WflResolverSettings {
            mode: DnsServersMode::Only,
            servers: Vec::new(),
            timeout_override: None,
        };
        assert_eq!(
            order(&none),
            [
                "10.0.0.9:53 /etc/resolv.conf",
                "10.0.0.1:53 /etc/resolv.conf"
            ]
        );

        // Timeout: dns_timeout_seconds, then resolv.conf, then 5 s.
        let effective = Effective::build(&none, &system, &sources);
        assert_eq!(effective.timeout, Duration::from_secs(3));
        assert_eq!(effective.attempts, 4);
        let overridden = WflResolverSettings {
            timeout_override: Some((1, "/p/.wflcfg".into())),
            ..none.clone()
        };
        let effective = Effective::build(&overridden, &system, &sources);
        assert_eq!(effective.timeout, Duration::from_secs(1));
        assert_eq!(effective.timeout_source, "/p/.wflcfg");
        let bare = Effective::build(&none, &SystemConfig::parse(""), &sources);
        assert_eq!(bare.timeout, Duration::from_secs(DEFAULT_TIMEOUT_SECONDS));
        assert_eq!(bare.attempts, DEFAULT_ATTEMPTS);
    }
}
