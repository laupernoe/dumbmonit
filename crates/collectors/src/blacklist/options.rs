//! Réglages des deux moniteurs, lus dans les étiquettes de la cible.

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use dumbmonit_proto::{Credential, ProbeError, Target};

use super::lists::{self, List, Subject};
use crate::uptime::tags;

pub const DEFAULT_REFRESH_HOURS: u32 = 6;
pub const DEFAULT_QUERY_TIMEOUT_SECONDS: u32 = 5;
/// Questions par seconde, toutes listes confondues : de quoi vérifier une
/// adresse de serveur en une seconde, et une plage /24 en trois minutes, sans
/// ressembler à un balayage aux yeux des listes.
pub const DEFAULT_QUERIES_PER_SECOND: u32 = 10;
pub const MAX_QUERIES_PER_SECOND: u32 = 50;
/// Questions en vol au même moment, au plus.
pub const MAX_IN_FLIGHT: usize = 8;

#[derive(Debug, Clone)]
pub struct Settings {
    pub resolver: Option<SocketAddr>,
    pub query_timeout: Duration,
    pub refresh: Duration,
    pub queries_per_second: u32,
    /// Listes retenues, dans l'ordre du catalogue.
    pub lists: Vec<&'static List>,
    /// Clé du Data Query Service de Spamhaus (secret de la cible).
    pub dqs_key: Option<String>,
    /// Adresses à vérifier en plus (moniteur de serveur mail).
    pub extra: Vec<IpAddr>,
}

impl Settings {
    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let dqs_key = match &target.credential {
            Credential::None => None,
            Credential::ApiToken { token } => Some(dqs_key(token)?),
            _ => {
                return Err(ProbeError::Config(
                    "a blocklist check only takes a Spamhaus DQS key, or no credential".to_string(),
                ));
            }
        };
        let refresh = tags::parse_u32(target, "refresh_hours", DEFAULT_REFRESH_HOURS, 1..=168)?;
        let timeout = tags::parse_u32(
            target,
            "query_timeout_seconds",
            DEFAULT_QUERY_TIMEOUT_SECONDS,
            1..=30,
        )?;
        let queries_per_second = tags::parse_u32(
            target,
            "queries_per_second",
            DEFAULT_QUERIES_PER_SECOND,
            1..=MAX_QUERIES_PER_SECOND,
        )?;
        Ok(Self {
            resolver: tags::tag(target, "resolver").map(resolver).transpose()?,
            query_timeout: Duration::from_secs(u64::from(timeout)),
            refresh: Duration::from_secs(u64::from(refresh) * 3600),
            queries_per_second,
            lists: selected(tags::tag(target, "lists"), tags::tag(target, "custom_lists"))?,
            dqs_key,
            extra: tags::tag(target, "extra_addresses")
                .map(addresses)
                .transpose()?
                .unwrap_or_default(),
        })
    }

    pub fn lists_for(&self, subject: Subject) -> impl Iterator<Item = &'static List> + '_ {
        self.lists.iter().copied().filter(move |list| list.subject == subject)
    }
}

/// Une clé DQS : 26 caractères minuscules et chiffres chez Spamhaus ; on
/// tolère une longueur voisine, mais rien qui puisse casser un nom DNS.
fn dqs_key(raw: &str) -> Result<String, ProbeError> {
    let key = raw.trim().to_ascii_lowercase();
    if (8..=64).contains(&key.len()) && key.chars().all(|c| c.is_ascii_alphanumeric()) {
        Ok(key)
    } else {
        Err(ProbeError::Config(
            "the Spamhaus DQS key is the string of letters and digits shown in your Spamhaus \
             account (26 characters)"
                .to_string(),
        ))
    }
}

/// `9.9.9.9`, `10.0.0.1:5353`, `[2001:db8::53]:53`.
fn resolver(raw: &str) -> Result<SocketAddr, ProbeError> {
    if let Ok(address) = raw.parse::<SocketAddr>() {
        return Ok(address);
    }
    raw.parse::<IpAddr>().map(|ip| SocketAddr::new(ip, 53)).map_err(|_| {
        ProbeError::Config(format!(
            "\"resolver\" expects the IP address of a resolver, port optional (\"10.0.0.53\", \
             \"10.0.0.53:5353\"), got \"{raw}\""
        ))
    })
}

/// Les listes à interroger : celles du catalogue que `lists` retient (toutes
/// sans réglage, aucune pour `-`), suivies des listes personnalisées.
fn selected(raw: Option<&str>, custom: Option<&str>) -> Result<Vec<&'static List>, ProbeError> {
    let mut chosen: Vec<&'static List> = match raw.map(str::trim) {
        None | Some("") => lists::LISTS.iter().collect(),
        // Le formulaire écrit `-` quand plus aucune liste du catalogue n'est cochée.
        Some("-") => Vec::new(),
        Some(raw) => {
            let wanted = tags::split_list(raw);
            for id in &wanted {
                if lists::by_id(id).is_none() {
                    return Err(ProbeError::Config(format!(
                        "unknown blocklist \"{id}\" in Lists; known lists: {}",
                        lists::ids()
                    )));
                }
            }
            lists::LISTS.iter().filter(|list| wanted.iter().any(|id| id == list.id)).collect()
        }
    };
    let extra = lists::parse_custom(custom.unwrap_or_default())
        .map_err(|why| ProbeError::Config(format!("Custom lists: {why}")))?;
    chosen.extend(extra);
    if chosen.is_empty() {
        return Err(ProbeError::Config(format!(
            "No blocklist is selected: tick at least one list or add a custom list; known \
             lists: {}",
            lists::ids()
        )));
    }
    Ok(chosen)
}

fn addresses(raw: &str) -> Result<Vec<IpAddr>, ProbeError> {
    tags::split_list(raw)
        .iter()
        .map(|item| {
            item.parse::<IpAddr>().map_err(|_| {
                ProbeError::Config(format!(
                    "\"extra_addresses\" expects IP addresses separated by commas, got \"{item}\""
                ))
            })
        })
        .collect()
}

/// Les listes ne connaissent que l'Internet public. `127.0.0.2` est l'adresse
/// d'essai que toutes listent : elle sert à vérifier la configuration.
pub fn public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            if v4 == std::net::Ipv4Addr::new(127, 0, 0, 2) {
                return true;
            }
            let [a, b, ..] = v4.octets();
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_unspecified()
                || v4.is_multicast()
                || a == 0
                || a >= 240
                // 100.64.0.0/10 : partage d'adresses des opérateurs (CGNAT).
                || (a == 100 && (64..128).contains(&b)))
        }
        IpAddr::V6(v6) => {
            let first = v6.segments()[0];
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || (first & 0xfe00) == 0xfc00
                || (first & 0xffc0) == 0xfe80
                || (first == 0x2001 && v6.segments()[1] == 0x0db8))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uptime::tags::test_support::cible;

    #[test]
    fn defauts_et_bornes() {
        let settings = Settings::from_target(&cible("mail_blacklist", "example.com", &[])).unwrap();
        assert_eq!(settings.lists.len(), lists::LISTS.len());
        assert_eq!(settings.refresh, Duration::from_secs(6 * 3600));
        assert_eq!(settings.queries_per_second, DEFAULT_QUERIES_PER_SECOND);
        assert!(settings.resolver.is_none());

        let tags = [("lists", "spamcop, spamhaus_zen"), ("resolver", "10.0.0.53")];
        let settings = Settings::from_target(&cible("ip_blacklist", "x", &tags)).unwrap();
        let ids: Vec<&str> = settings.lists.iter().map(|l| l.id).collect();
        assert_eq!(ids, ["spamhaus_zen", "spamcop"], "ordre du catalogue");
        assert_eq!(settings.resolver.unwrap().port(), 53);

        for tags in [
            [("lists", "sorbs")],
            [("queries_per_second", "500")],
            [("resolver", "dns.google")],
            [("extra_addresses", "mail.example.com")],
        ] {
            let error = Settings::from_target(&cible("mail_blacklist", "x", &tags)).unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "{tags:?}");
        }
    }

    #[test]
    fn les_listes_choisies_et_les_listes_personnalisees() {
        let settings = |tags: &[(&str, &str)]| {
            Settings::from_target(&cible("mail_blacklist", "example.com", tags))
        };
        let ids = |s: &Settings| s.lists.iter().map(|l| l.id.to_string()).collect::<Vec<_>>();

        // Sans réglage : tout le catalogue (compatibilité des moniteurs existants).
        assert_eq!(settings(&[]).unwrap().lists.len(), lists::LISTS.len());
        // Une sélection, dans l'ordre du catalogue, puis les listes personnalisées.
        let custom = r#"[{"zone":"DNSBL.Example.org.","subject":"domain","major":true},
            {"zone":"ip.example.net"}]"#;
        let chosen = settings(&[("lists", "spamcop,spamhaus_zen"), ("custom_lists", custom)]);
        let chosen = chosen.unwrap();
        assert_eq!(
            ids(&chosen),
            ["spamhaus_zen", "spamcop", "custom:dnsbl.example.org", "custom:ip.example.net"]
        );
        let domain = chosen.lists_for(Subject::Domain).collect::<Vec<_>>();
        assert_eq!(domain.len(), 1);
        assert!(domain[0].major && domain[0].zone == "dnsbl.example.org");
        let ip = chosen.lists_for(Subject::Ip).map(|l| l.id).collect::<Vec<_>>();
        assert_eq!(ip, ["spamhaus_zen", "spamcop", "custom:ip.example.net"]);

        // `-` : aucune liste du catalogue, seulement les personnalisées.
        let only = settings(&[("lists", "-"), ("custom_lists", r#"[{"zone":"a.example.org"}]"#)]);
        assert_eq!(ids(&only.unwrap()), ["custom:a.example.org"]);
        // Rien du tout : erreur de configuration.
        assert!(matches!(settings(&[("lists", "-")]), Err(ProbeError::Config(_))));

        // Un format invalide, une adresse IP, un doublon, un type inconnu.
        for bad in [
            "not json",
            r#"[{"zone":"192.0.2.1"}]"#,
            r#"[{"zone":"nodot"}]"#,
            r#"[{"zone":"bad_.-example.org "}]"#,
            r#"[{"zone":"1.2.3.4"}]"#,
            r#"[{"zone":"a.example.org"},{"zone":"A.example.org."}]"#,
            r#"[{"zone":"zen.spamhaus.org"}]"#,
            r#"[{"zone":"a.example.org","subject":"url"}]"#,
        ] {
            let error = settings(&[("custom_lists", bad)]).unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "{bad}");
        }
    }

    #[test]
    fn la_cle_dqs_vient_du_secret() {
        let mut target = cible("mail_blacklist", "example.com", &[]);
        target.credential = Credential::ApiToken { token: " ABCDEFGHIJ0123456789KLMNOP ".into() };
        let settings = Settings::from_target(&target).unwrap();
        assert_eq!(settings.dqs_key.as_deref(), Some("abcdefghij0123456789klmnop"));
        target.credential = Credential::ApiToken { token: "abc.def/../x".into() };
        assert!(Settings::from_target(&target).is_err());
    }

    #[test]
    fn seules_les_adresses_publiques_sont_verifiees() {
        for ok in ["8.8.8.8", "127.0.0.2", "2a00:1450::1"] {
            assert!(public(ok.parse().unwrap()), "{ok}");
        }
        for ko in
            ["10.1.2.3", "192.168.1.1", "100.64.0.1", "127.0.0.1", "203.0.113.5", "fd00::1", "::1"]
        {
            assert!(!public(ko.parse().unwrap()), "{ko}");
        }
    }
}
