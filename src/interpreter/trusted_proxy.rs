//! Bounded, explicit trust for HTTP forwarding metadata.
use std::net::IpAddr;

const MAX_PROXIES: usize = 128;
const MAX_CONFIG_BYTES: usize = 8192;
const MAX_FORWARDED_BYTES: usize = 4096;
const MAX_FORWARDED_HOPS: usize = 32;

#[derive(Clone, Debug, Default)]
pub(crate) struct TrustedProxyPolicy {
    networks: Vec<(IpAddr, u8)>,
}

fn canonical_ip(address: IpAddr) -> IpAddr {
    match address {
        IpAddr::V6(address) => address
            .to_ipv4_mapped()
            .map_or(IpAddr::V6(address), IpAddr::V4),
        address => address,
    }
}

impl TrustedProxyPolicy {
    /// Shared loader/checker parsing so config repair cannot remove valid trust.
    pub(crate) fn parse_config(value: &str) -> Result<Vec<String>, &'static str> {
        if value.len() > MAX_CONFIG_BYTES {
            return Err("trusted proxies exceed 8192 bytes");
        }
        let entries = if value.is_empty() {
            Vec::new()
        } else {
            value
                .split(',')
                .take(MAX_PROXIES + 1)
                .map(|entry| entry.trim().to_string())
                .collect()
        };
        Self::parse(&entries)?;
        Ok(entries)
    }

    /// Reject the entire policy on an invalid entry; partial trust is unsafe.
    pub(crate) fn parse(entries: &[String]) -> Result<Self, &'static str> {
        if entries.len() > MAX_PROXIES
            || entries
                .iter()
                .try_fold(entries.len().saturating_sub(1), |size, entry| {
                    size.checked_add(entry.len())
                })
                .is_none_or(|size| size > MAX_CONFIG_BYTES)
        {
            return Err("trusted proxies exceed 128 entries or 8192 bytes");
        }
        let mut networks = Vec::with_capacity(entries.len());
        for entry in entries {
            let (address, prefix) = match entry.trim().split_once('/') {
                Some((address, prefix)) => {
                    if prefix.is_empty() || !prefix.bytes().all(|byte| byte.is_ascii_digit()) {
                        return Err("trusted proxy CIDR prefix must be a decimal number");
                    }
                    let prefix = prefix
                        .parse::<u8>()
                        .map_err(|_| "invalid trusted proxy CIDR prefix")?;
                    (address, Some(prefix))
                }
                None => (entry.trim(), None),
            };
            let address = address
                .parse::<IpAddr>()
                .map_err(|_| "trusted proxies must be IP addresses or CIDRs")?;
            let width = if address.is_ipv4() { 32 } else { 128 };
            let mut prefix = prefix.unwrap_or(width);
            if prefix > width {
                return Err("trusted proxy CIDR prefix is too large for its address family");
            }
            let canonical = canonical_ip(address);
            if address.is_ipv6() && canonical.is_ipv4() {
                prefix = prefix
                    .checked_sub(96)
                    .ok_or("IPv4-mapped proxy CIDRs require a prefix of at least 96")?;
            }
            networks.push((canonical, prefix));
        }
        Ok(Self { networks })
    }

    fn trusts(&self, address: IpAddr) -> bool {
        let address = canonical_ip(address);
        self.networks
            .iter()
            .any(|(network, prefix)| match (network, address) {
                (IpAddr::V4(network), IpAddr::V4(address)) => {
                    let mask = u32::MAX.checked_shl(u32::from(32 - prefix)).unwrap_or(0);
                    u32::from(*network) & mask == u32::from(address) & mask
                }
                (IpAddr::V6(network), IpAddr::V6(address)) => {
                    let mask = u128::MAX.checked_shl(u32::from(128 - prefix)).unwrap_or(0);
                    u128::from(*network) & mask == u128::from(address) & mask
                }
                _ => false,
            })
    }

    /// The socket peer is authoritative unless it is explicitly trusted.
    /// Validate the complete header before taking the rightmost untrusted hop.
    /// Duplicate physical header fields are ambiguous and are never combined.
    pub(crate) fn originating_ip(
        &self,
        peer: Option<IpAddr>,
        headers: &warp::http::HeaderMap,
    ) -> Option<IpAddr> {
        let peer = canonical_ip(peer?);
        if !self.trusts(peer) {
            return Some(peer);
        }
        let mut values = headers.get_all("x-forwarded-for").iter();
        let Some(value) = values.next() else {
            return Some(peer);
        };
        if values.next().is_some() || value.as_bytes().len() > MAX_FORWARDED_BYTES {
            return Some(peer);
        }
        let Ok(value) = value.to_str() else {
            return Some(peer);
        };
        let mut chain = Vec::with_capacity(MAX_FORWARDED_HOPS);
        for entry in value.split(',') {
            if chain.len() == MAX_FORWARDED_HOPS {
                return Some(peer);
            }
            // HTTP optional whitespace is SP/HTAB, not arbitrary Unicode.
            let Ok(address) = entry.trim_matches([' ', '\t']).parse::<IpAddr>() else {
                return Some(peer);
            };
            chain.push(canonical_ip(address));
        }
        let mut origin = peer;
        for address in chain.into_iter().rev() {
            if !self.trusts(origin) {
                break;
            }
            origin = address;
        }
        Some(origin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(entries: &[&str]) -> TrustedProxyPolicy {
        TrustedProxyPolicy::parse(
            &entries
                .iter()
                .map(|entry| entry.to_string())
                .collect::<Vec<_>>(),
        )
        .unwrap()
    }

    #[test]
    fn cidr_boundaries_and_address_families_are_exact() {
        let policy = policy(&["10.0.0.1/24", "2001:db8::/127", "::ffff:192.0.2.0/120"]);
        for address in [
            "10.0.0.0",
            "10.0.0.255",
            "2001:db8::",
            "2001:db8::1",
            "192.0.2.1",
            "::ffff:10.0.0.1",
        ] {
            assert!(policy.trusts(address.parse().unwrap()), "{address}");
        }
        for address in ["10.0.1.0", "9.255.255.255", "2001:db8::2", "192.0.3.0"] {
            assert!(!policy.trusts(address.parse().unwrap()), "{address}");
        }
        assert!(
            TrustedProxyPolicy::parse(&["0.0.0.0/0".into()])
                .unwrap()
                .trusts("198.51.100.7".parse().unwrap())
        );
    }

    #[test]
    fn bad_and_oversized_policies_are_rejected_atomically() {
        for entry in [
            "",
            "localhost",
            "127.0.0.1/33",
            "::1/129",
            "::1/-1",
            "::1/+1",
            "::1/1/2",
            "::ffff:127.0.0.1/95",
        ] {
            assert!(
                TrustedProxyPolicy::parse(&["127.0.0.1".into(), entry.into()]).is_err(),
                "{entry}"
            );
        }
        assert!(TrustedProxyPolicy::parse(&vec!["127.0.0.1".into(); 129]).is_err());
        assert!(TrustedProxyPolicy::parse(&[" ".repeat(8193)]).is_err());
        assert!(TrustedProxyPolicy::parse(&vec!["127.0.0.1".into(); 128]).is_ok());
    }

    #[test]
    fn missing_peer_and_non_ascii_forwarding_never_create_an_identity() {
        let policy = policy(&["127.0.0.1"]);
        let mut headers = warp::http::HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            warp::http::HeaderValue::from_bytes(b"198.51.100.7,\xff").unwrap(),
        );
        let peer = "127.0.0.1".parse().unwrap();
        assert_eq!(policy.originating_ip(None, &headers), None);
        assert_eq!(policy.originating_ip(Some(peer), &headers), Some(peer));
    }

    #[test]
    fn exactly_32_hops_are_accepted_but_33_are_not() {
        let policy = policy(&["127.0.0.1"]);
        let peer = "127.0.0.1".parse().unwrap();
        for hops in [32, 33] {
            let mut headers = warp::http::HeaderMap::new();
            let mut chain = vec!["198.51.100.7"];
            chain.extend(std::iter::repeat_n("127.0.0.1", hops - 1));
            headers.insert("x-forwarded-for", chain.join(",").parse().unwrap());
            assert_eq!(
                policy.originating_ip(Some(peer), &headers),
                Some(if hops == 32 {
                    "198.51.100.7".parse().unwrap()
                } else {
                    peer
                })
            );
        }
    }
}
