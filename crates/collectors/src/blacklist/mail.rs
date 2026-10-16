//! Réputation d'un serveur mail (`kind = "mail_blacklist"`).
//!
//! Adresse : une IP publique, ou un domaine de messagerie. Pour un domaine, les
//! adresses vérifiées sont celles de ses MX (A et AAAA), celles du domaine
//! lui-même, et les `extra_addresses` (un relais d'envoi qui n'est pas un MX).
//! Chacune passe devant les listes d'adresses ; le domaine devant les listes de
//! domaines. S'y ajoutent ce que regardent Gmail, Outlook.com et Yahoo, qui ne
//! publient aucune liste : le DNS inverse de chaque adresse et sa cohérence
//! (FCrDNS), et pour un domaine ses enregistrements SPF et DMARC.

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;

use dumbmonit_proto::ProbeError;

use super::check::{self, Pace};
use super::dns::{Dns, Failure, Qtype};
use super::lists::{Subject, Target, reverse_v4, reverse_v6};
use super::options::{Settings, public};
use super::scan::{Address, MailAuth, Scan};

/// Hôtes MX suivis, au plus.
const MAX_MX_HOSTS: usize = 10;
/// Adresses vérifiées, au plus.
pub const MAX_ADDRESSES: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Ip(IpAddr),
    Domain(String),
}

pub fn parse(address: &str) -> Result<Input, ProbeError> {
    let raw = address.trim().trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = raw.parse::<IpAddr>() {
        if !public(ip) {
            return Err(ProbeError::Config(format!(
                "{ip} is not a public address: blocklists only know addresses reachable from \
                 the Internet. Enter the public address your mail server sends from."
            )));
        }
        return Ok(Input::Ip(ip));
    }
    crate::domain::normalize(raw).map(Input::Domain).map_err(|_| {
        ProbeError::Config(format!(
            "\"{address}\" is neither an IP address nor a domain name: enter the public address \
             of the mail server, or the domain it sends for (example.com)"
        ))
    })
}

fn fqdn(name: &str) -> String {
    format!("{}.", name.trim_end_matches('.'))
}

fn ptr_name(ip: IpAddr) -> String {
    match ip {
        IpAddr::V4(v4) => format!("{}.in-addr.arpa.", reverse_v4(v4)),
        IpAddr::V6(v6) => format!("{}.ip6.arpa.", reverse_v6(v6)),
    }
}

fn silent(domain: &str, what: &str, failure: Failure) -> ProbeError {
    ProbeError::Unreachable(format!(
        "the resolver did not answer the {what} query for {domain}: {failure}"
    ))
}

/// `v=spf1` en tête, suivi d'un espace ou de rien (RFC 7208, section 4.5).
fn is_spf(record: &str) -> bool {
    let lower = record.trim().to_ascii_lowercase();
    lower == "v=spf1" || lower.starts_with("v=spf1 ")
}

fn is_dmarc(record: &str) -> bool {
    record.trim().to_ascii_lowercase().replace(' ', "").starts_with("v=dmarc1;")
        || record.trim().eq_ignore_ascii_case("v=DMARC1")
}

/// `p=` d'un enregistrement DMARC.
fn dmarc_policy(record: &str) -> Option<String> {
    record.split(';').find_map(|part| {
        let (key, value) = part.split_once('=')?;
        key.trim().eq_ignore_ascii_case("p").then(|| value.trim().to_ascii_lowercase())
    })
}

/// Les hôtes d'une réponse MX, du plus prioritaire au moins prioritaire. Un « MX
/// nul » (`0 .`, RFC 7505) ne donne aucun hôte.
fn mx_hosts(values: &[String]) -> Vec<String> {
    let mut hosts: Vec<(u16, String)> = values
        .iter()
        .filter_map(|value| {
            let (preference, host) = value.trim().split_once(' ')?;
            let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
            (!host.is_empty()).then(|| (preference.parse().unwrap_or(u16::MAX), host))
        })
        .collect();
    hosts.sort();
    let mut seen = Vec::new();
    for (_, host) in hosts {
        if !seen.contains(&host) {
            seen.push(host);
        }
    }
    seen.truncate(MAX_MX_HOSTS);
    seen
}

fn add(addresses: &mut Vec<Address>, ip: IpAddr, source: &'static str, host: Option<&str>) {
    if !public(ip) || addresses.iter().any(|a| a.ip == ip) || addresses.len() >= MAX_ADDRESSES {
        return;
    }
    addresses.push(Address { ip, source, host: host.map(str::to_string), ptr: None, fcrdns: None });
}

/// Les adresses A et AAAA d'un nom. Un nom muet n'en donne aucune.
async fn resolve(dns: &dyn Dns, pace: &Pace, name: &str) -> Vec<IpAddr> {
    let mut out = Vec::new();
    for qtype in [Qtype::A, Qtype::Aaaa] {
        pace.wait().await;
        if let Ok(values) = dns.query(&fqdn(name), qtype).await {
            out.extend(values.iter().filter_map(|v| v.parse::<IpAddr>().ok()));
        }
    }
    out
}

/// Le DNS inverse d'une adresse, et s'il revient vers elle (FCrDNS).
async fn reverse(dns: &dyn Dns, pace: &Pace, address: &mut Address) {
    pace.wait().await;
    let names = match dns.query(&ptr_name(address.ip), Qtype::Ptr).await {
        Ok(names) => names,
        Err(_) => return,
    };
    address.ptr = names.first().cloned();
    if names.is_empty() {
        address.fcrdns = Some(false);
        return;
    }
    let qtype = if address.ip.is_ipv4() { Qtype::A } else { Qtype::Aaaa };
    let mut answered = false;
    for name in names.iter().take(3) {
        pace.wait().await;
        if let Ok(values) = dns.query(&fqdn(name), qtype).await {
            answered = true;
            if values.iter().any(|v| v.parse::<IpAddr>().ok() == Some(address.ip)) {
                address.ptr = Some(name.clone());
                address.fcrdns = Some(true);
                return;
            }
        }
    }
    address.fcrdns = answered.then_some(false);
}

async fn txt(dns: &dyn Dns, pace: &Pace, name: &str) -> Option<Vec<String>> {
    pace.wait().await;
    dns.query(&fqdn(name), Qtype::Txt).await.ok()
}

pub async fn scan(dns: Arc<dyn Dns>, input: Input, settings: Settings) -> Result<Scan, ProbeError> {
    let started = Instant::now();
    let pace = Arc::new(Pace::per_second(settings.queries_per_second));
    let mut addresses = Vec::new();
    let mut auth = None;
    let mut domain = None;

    match &input {
        Input::Ip(ip) => add(&mut addresses, *ip, "input", None),
        Input::Domain(name) => {
            pace.wait().await;
            let mx = dns.query(&fqdn(name), Qtype::Mx).await.map_err(|f| silent(name, "MX", f))?;
            let hosts = mx_hosts(&mx);
            for host in &hosts {
                for ip in resolve(dns.as_ref(), &pace, host).await {
                    add(&mut addresses, ip, "mx", Some(host.as_str()));
                }
            }
            for ip in resolve(dns.as_ref(), &pace, name).await {
                add(&mut addresses, ip, "domain", None);
            }
            let spf = txt(dns.as_ref(), &pace, name).await;
            let dmarc = txt(dns.as_ref(), &pace, &format!("_dmarc.{name}")).await;
            let dmarc_records: Option<Vec<&String>> =
                dmarc.as_ref().map(|values| values.iter().filter(|v| is_dmarc(v)).collect());
            auth = Some(MailAuth {
                mx_hosts: hosts.len(),
                spf_records: spf.map(|values| values.iter().filter(|v| is_spf(v)).count()),
                dmarc_records: dmarc_records.as_ref().map(Vec::len),
                dmarc_policy: dmarc_records.and_then(|r| r.first().and_then(|v| dmarc_policy(v))),
            });
            domain = Some(name.clone());
        }
    }
    for ip in &settings.extra {
        add(&mut addresses, *ip, "extra", None);
    }
    if addresses.is_empty() {
        let name = domain.as_deref().unwrap_or_default();
        return Err(ProbeError::Config(format!(
            "{name} has no MX, A or AAAA record pointing to a public address: nothing to check. \
             Enter the public address the mail server sends from, or add it to Extra addresses."
        )));
    }
    for address in &mut addresses {
        reverse(dns.as_ref(), &pace, address).await;
    }

    let mut jobs = Vec::new();
    for address in &addresses {
        let subject = match address.ip {
            IpAddr::V4(v4) => Target::V4(v4),
            IpAddr::V6(v6) => Target::V6(v6),
        };
        jobs.extend(settings.lists_for(Subject::Ip).map(|list| (subject.clone(), list)));
    }
    if let Some(name) = &domain {
        jobs.extend(
            settings.lists_for(Subject::Domain).map(|list| (Target::Domain(name.clone()), list)),
        );
    }
    let outcomes = check::run(dns, pace, jobs, settings.dqs_key.clone()).await;
    super::conclude(&outcomes)?;
    Ok(Scan {
        addresses_checked: addresses.len(),
        outcomes,
        addresses,
        domain,
        auth,
        lists: settings.lists.clone(),
        duration: started.elapsed(),
        every_check: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresse_ou_domaine() {
        assert!(parse("192.0.2.1").unwrap_err().to_string().contains("public"));
        assert_eq!(parse("8.8.4.4").unwrap(), Input::Ip("8.8.4.4".parse().unwrap()));
        assert_eq!(parse("Example.COM.").unwrap(), Input::Domain("example.com".into()));
        assert_eq!(parse("[2a00:1450::1]").unwrap(), Input::Ip("2a00:1450::1".parse().unwrap()));
        assert!(matches!(parse("not a host"), Err(ProbeError::Config(_))));
    }

    #[test]
    fn mx_spf_dmarc() {
        let mx = ["20 mx2.example.com.".to_string(), "10 MX1.example.com".into(), "0 .".into()];
        assert_eq!(mx_hosts(&mx), ["mx1.example.com", "mx2.example.com"]);
        assert!(is_spf("v=spf1 mx -all"));
        assert!(!is_spf("v=spf10 nonsense"));
        assert!(is_dmarc("v=DMARC1; p=reject; rua=mailto:d@example.com"));
        assert_eq!(dmarc_policy("v=DMARC1; p=Quarantine; pct=100").as_deref(), Some("quarantine"));
        assert_eq!(ptr_name("192.0.2.1".parse().unwrap()), "1.2.0.192.in-addr.arpa.");
    }
}
