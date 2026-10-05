//! Options de collecte lues sur la cible.
//!
//! Même contrat que les autres intégrations HTTP : tout passe par
//! `Target::tags`, et une cible sans étiquette fonctionne sur un routeur où le
//! service `www-ssl` est activé. RouterOS le livre désactivé et sans
//! certificat : la notice explique comment l'allumer, d'où `https` par défaut
//! malgré tout — le mot de passe du compte de lecture voyage à chaque requête.

use std::time::Duration;

use dumbmonit_proto::{ProbeError, Target};

/// Délai appliqué à chaque requête. Un hAP au processeur occupé met parfois
/// plusieurs secondes à répondre sur `/rest/interface`.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Interfaces lues au plus. Un commutateur CRS en a une cinquantaine, un
/// concentrateur VPN des centaines (chaque tunnel en est une) : au-delà, les
/// suivantes sont comptées et laissées de côté.
pub const DEFAULT_MAX_INTERFACES: usize = 64;

#[derive(Debug, Clone)]
pub struct Options {
    /// Racine du routeur, sans barre oblique finale : `https://router.lan:443`.
    pub base_url: String,
    pub insecure_tls: bool,
    pub request_timeout: Duration,
    pub max_interfaces: usize,
    /// Lire les interfaces ; décoché, seuls le système et les capteurs le sont.
    pub interfaces: bool,
}

impl Options {
    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let scheme = parse_scheme(tag(target, "scheme"))?;
        let default_port = if scheme == "https" { 443 } else { 80 };
        let port = parse_port(tag(target, "port"), default_port)?;
        Ok(Self {
            base_url: base_url(&target.address, scheme, port)?,
            insecure_tls: parse_bool(tag(target, "insecure_tls"), false)?,
            request_timeout: parse_timeout(tag(target, "request_timeout_seconds"))?,
            max_interfaces: parse_max(tag(target, "max_interfaces"))?,
            interfaces: parse_bool(tag(target, "interfaces"), true)?,
        })
    }
}

fn tag<'a>(target: &'a Target, key: &str) -> Option<&'a str> {
    target.tags.get(key).map(|value| value.trim()).filter(|value| !value.is_empty())
}

fn parse_bool(value: Option<&str>, default: bool) -> Result<bool, ProbeError> {
    match value {
        None => Ok(default),
        Some(raw) => match raw.to_ascii_lowercase().as_str() {
            "true" | "1" | "yes" | "oui" | "on" => Ok(true),
            "false" | "0" | "no" | "non" | "off" => Ok(false),
            other => Err(ProbeError::Config(format!(
                "Expected a boolean value (true/false), got \"{other}\""
            ))),
        },
    }
}

fn parse_scheme(value: Option<&str>) -> Result<&'static str, ProbeError> {
    match value.map(str::to_ascii_lowercase).as_deref() {
        None | Some("https") => Ok("https"),
        Some("http") => Ok("http"),
        Some(other) => {
            Err(ProbeError::Config(format!("Unknown protocol \"{other}\": https or http")))
        }
    }
}

fn parse_port(value: Option<&str>, default: u16) -> Result<u16, ProbeError> {
    match value {
        None => Ok(default),
        Some(raw) => raw
            .parse()
            .ok()
            .filter(|port| *port > 0)
            .ok_or_else(|| ProbeError::Config(format!("Invalid port: \"{raw}\""))),
    }
}

fn parse_timeout(value: Option<&str>) -> Result<Duration, ProbeError> {
    let Some(raw) = value else { return Ok(DEFAULT_REQUEST_TIMEOUT) };
    let seconds: u64 =
        raw.parse().map_err(|_| ProbeError::Config(format!("Invalid timeout: \"{raw}\"")))?;
    if !(1..=60).contains(&seconds) {
        return Err(ProbeError::Config(
            "Request timeout must be between 1 and 60 seconds".to_string(),
        ));
    }
    Ok(Duration::from_secs(seconds))
}

fn parse_max(value: Option<&str>) -> Result<usize, ProbeError> {
    let Some(raw) = value else { return Ok(DEFAULT_MAX_INTERFACES) };
    raw.parse::<usize>().ok().filter(|n| (1..=1000).contains(n)).ok_or_else(|| {
        ProbeError::Config(format!("Invalid interface limit \"{raw}\": between 1 and 1000"))
    })
}

/// Compose la racine à partir de l'adresse saisie : `router.lan`,
/// `router.lan:8443`, une IPv6 avec ou sans crochets, ou une URL complète.
fn base_url(address: &str, scheme: &str, port: u16) -> Result<String, ProbeError> {
    let address = address.trim().trim_end_matches('/');
    // Une adresse copiée depuis la documentation se termine parfois par `/rest`.
    let address = address.strip_suffix("/rest").unwrap_or(address);
    if address.is_empty() {
        return Err(ProbeError::Config("Device address is empty".to_string()));
    }
    if address.starts_with("https://") || address.starts_with("http://") {
        return Ok(address.to_string());
    }
    if address.starts_with('[') {
        return Ok(match address.rfind("]:") {
            Some(_) => format!("{scheme}://{address}"),
            None => format!("{scheme}://{address}:{port}"),
        });
    }
    if address.matches(':').count() > 1 {
        return Ok(format!("{scheme}://[{address}]:{port}"));
    }
    if address.contains(':') {
        return Ok(format!("{scheme}://{address}"));
    }
    Ok(format!("{scheme}://{address}:{port}"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use dumbmonit_proto::Credential;

    use super::*;

    fn target(address: &str, tags: &[(&str, &str)]) -> Target {
        Target {
            id: 1,
            name: "routeur".into(),
            address: address.into(),
            kind: "mikrotik".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: tags
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
            credential: Credential::None,
            group_name: String::new(),
            position: 0,
        }
    }

    #[test]
    fn une_cible_sans_etiquette_parle_https_sur_le_port_443() {
        let options = Options::from_target(&target("router.lan", &[])).unwrap();
        assert_eq!(options.base_url, "https://router.lan:443");
        assert!(!options.insecure_tls);
        assert!(options.interfaces);
        assert_eq!(options.request_timeout, DEFAULT_REQUEST_TIMEOUT);
        assert_eq!(options.max_interfaces, DEFAULT_MAX_INTERFACES);
    }

    #[test]
    fn l_adresse_accepte_les_formes_usuelles() {
        let cases = [
            ("10.0.0.1", &[][..], "https://10.0.0.1:443"),
            ("router.lan:8443", &[], "https://router.lan:8443"),
            ("https://router.lan/rest/", &[], "https://router.lan"),
            ("fd00::1", &[], "https://[fd00::1]:443"),
            ("router.lan", &[("scheme", "http")], "http://router.lan:80"),
            ("router.lan", &[("scheme", "http"), ("port", "8080")], "http://router.lan:8080"),
        ];
        for (address, tags, expected) in cases {
            let options = Options::from_target(&target(address, tags)).unwrap();
            assert_eq!(options.base_url, expected, "adresse « {address} »");
        }
    }

    #[test]
    fn une_option_invalide_est_une_erreur_de_configuration() {
        for (key, value) in [
            ("insecure_tls", "peut-être"),
            ("interfaces", "toutes"),
            ("port", "0"),
            ("request_timeout_seconds", "900"),
            ("max_interfaces", "0"),
            ("scheme", "ftp"),
        ] {
            let error = Options::from_target(&target("x.lan", &[(key, value)])).unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "« {key} = {value} »");
        }
        assert!(Options::from_target(&target("  ", &[])).is_err());
    }
}
