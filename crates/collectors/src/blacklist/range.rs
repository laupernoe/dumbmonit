//! Liste noire d'une plage d'adresses publiques (`kind = "ip_blacklist"`).
//!
//! Adresse : une IPv4 (`198.51.100.7`) ou une plage CIDR jusqu'à /24
//! (`198.51.100.0/24`, 256 adresses). Chaque adresse passe devant les listes
//! d'adresses retenues, au rythme de `queries_per_second`. L'IPv6 est refusée :
//! sur les listes retenues, seule Spamhaus ZEN la couvre, et une plage IPv6 ne
//! se parcourt pas adresse par adresse.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Instant;

use dumbmonit_proto::ProbeError;

use super::check::{self, Pace};
use super::dns::Dns;
use super::lists::{Subject, Target};
use super::options::{Settings, public};
use super::scan::Scan;

/// Adresses d'une plage, au plus : un /24.
pub const MAX_ADDRESSES: u32 = 256;

pub fn parse(address: &str) -> Result<Vec<Ipv4Addr>, ProbeError> {
    let raw = address.trim();
    let (ip, prefix) = match raw.split_once('/') {
        Some((ip, prefix)) => (ip.trim(), Some(prefix.trim())),
        None => (raw, None),
    };
    let ip: IpAddr = ip.parse().map_err(|_| {
        ProbeError::Config(format!(
            "\"{address}\" is not an IPv4 address or range: enter an address (198.51.100.7) or a \
             CIDR range up to /24 (198.51.100.0/24)"
        ))
    })?;
    let IpAddr::V4(ip) = ip else {
        return Err(ProbeError::Config(
            "IPv6 ranges are not checked: of the blocklists DumbMonit queries only Spamhaus ZEN \
             answers for IPv6, and an IPv6 range cannot be walked address by address. Check a \
             single IPv6 address with the mail server reputation monitor instead."
                .to_string(),
        ));
    };
    let prefix: u8 = match prefix {
        None => 32,
        Some(text) => text.parse().ok().filter(|p| *p <= 32).ok_or_else(|| {
            ProbeError::Config(format!("\"/{text}\" is not a CIDR prefix length (0 to 32)"))
        })?,
    };
    let size = 1u64 << (32 - u32::from(prefix));
    if size > u64::from(MAX_ADDRESSES) {
        return Err(ProbeError::Config(format!(
            "{raw} holds {size} addresses; one monitor checks at most {MAX_ADDRESSES} (a /24). \
             Split the range into several /24 monitors: each address costs several queries to \
             every blocklist, and a larger sweep looks like abuse to them."
        )));
    }
    let start = u32::from(ip) & (u32::MAX.checked_shl(32 - u32::from(prefix)).unwrap_or(0));
    let addresses: Vec<Ipv4Addr> = (0..size as u32).map(|i| Ipv4Addr::from(start + i)).collect();
    if let Some(private) = addresses.iter().find(|ip| !public(IpAddr::V4(**ip))) {
        return Err(ProbeError::Config(format!(
            "{raw} includes {private}, which is not a public address: blocklists only know \
             addresses reachable from the Internet"
        )));
    }
    Ok(addresses)
}

pub async fn scan(
    dns: Arc<dyn Dns>,
    addresses: Vec<Ipv4Addr>,
    settings: Settings,
) -> Result<Scan, ProbeError> {
    let started = Instant::now();
    let pace = Arc::new(Pace::per_second(settings.queries_per_second));
    let lists: Vec<_> = settings.lists_for(Subject::Ip).collect();
    if lists.is_empty() {
        return Err(ProbeError::Config(
            "Lists names only domain blocklists: an address range needs address blocklists"
                .to_string(),
        ));
    }
    let jobs = addresses
        .iter()
        .flat_map(|ip| lists.iter().map(move |list| (Target::V4(*ip), *list)))
        .collect();
    let outcomes = check::run(dns, pace, jobs, settings.dqs_key.clone()).await;
    super::conclude(&outcomes)?;
    Ok(Scan {
        addresses_checked: addresses.len(),
        outcomes,
        addresses: Vec::new(),
        domain: None,
        auth: None,
        lists,
        duration: started.elapsed(),
        every_check: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plages_acceptees_et_refusees() {
        assert_eq!(parse("8.8.8.8").unwrap(), [Ipv4Addr::new(8, 8, 8, 8)]);
        let range = parse("8.8.8.13/30").unwrap();
        assert_eq!(range, [12, 13, 14, 15].map(|d| Ipv4Addr::new(8, 8, 8, d)));
        assert_eq!(parse("8.8.8.0/24").unwrap().len(), 256);

        let too_big = parse("8.8.0.0/23").unwrap_err();
        assert!(
            matches!(too_big, ProbeError::Config(ref m) if m.contains("512") && m.contains("/24"))
        );
        assert!(!too_big.means_down());
        let v6 = parse("2a00:1450::/120").unwrap_err();
        assert!(matches!(v6, ProbeError::Config(ref m) if m.contains("IPv6")));
        assert!(matches!(parse("10.0.0.0/30"), Err(ProbeError::Config(_))));
        assert!(matches!(parse("8.8.8.0/33"), Err(ProbeError::Config(_))));
        assert!(matches!(parse("example.com"), Err(ProbeError::Config(_))));
    }
}
