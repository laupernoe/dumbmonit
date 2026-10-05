//! Options de collecte AdGuard Home, lues dans les étiquettes de la cible.
//!
//! Même contrat que les autres intégrations HTTP : aucun champ propre dans
//! `Target`, une cible sans étiquette fonctionne sur une installation
//! standard. AdGuard Home sert son interface en clair sur le port choisi à
//! l'installation (80 par défaut) tant qu'on ne lui a pas donné de certificat,
//! d'où `http` par défaut.

use std::time::Duration;

use dumbmonit_proto::{ProbeError, Target};

/// Port proposé par l'assistant d'installation pour l'interface web.
pub const DEFAULT_PORT: u16 = 80;

/// Délai appliqué à chaque requête. Les réponses tiennent en quelques
/// kilo-octets : cinq secondes suffisent largement.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct Options {
    /// Racine de l'interface, sans barre oblique finale ni `/control`.
    pub base_url: String,
    pub insecure_tls: bool,
    pub request_timeout: Duration,
    /// Fait tester chaque serveur DNS amont par AdGuard Home à chaque collecte.
    pub upstream_check: bool,
}

impl Options {
    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let scheme = parse_scheme(tag(target, "scheme"))?;
        let port = parse_port(tag(target, "port"))?;
        Ok(Self {
            base_url: base_url(&target.address, scheme, port)?,
            insecure_tls: parse_bool(tag(target, "insecure_tls"), false)?,
            request_timeout: parse_timeout(tag(target, "request_timeout_seconds"))?,
            upstream_check: parse_bool(tag(target, "upstream_check"), true)?,
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
        None | Some("http") => Ok("http"),
        Some("https") => Ok("https"),
        Some(other) => {
            Err(ProbeError::Config(format!("Unknown protocol \"{other}\": http or https")))
        }
    }
}

fn parse_port(value: Option<&str>) -> Result<u16, ProbeError> {
    match value {
        None => Ok(DEFAULT_PORT),
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

/// Compose la racine à partir de l'adresse saisie : `adguard.lan`,
/// `adguard.lan:3000`, une IPv6, ou une URL complète — chemin compris derrière
/// un proxy inverse. Un `/control` final, copié depuis la documentation de
/// l'API, est retiré : les chemins l'ajoutent déjà.
fn base_url(address: &str, scheme: &str, port: u16) -> Result<String, ProbeError> {
    let address = address.trim().trim_end_matches('/');
    let address = address.strip_suffix("/control").unwrap_or(address).trim_end_matches('/');
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
            name: "dns".into(),
            address: address.into(),
            kind: "adguard".into(),
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
    fn une_cible_sans_etiquette_parle_en_clair_sur_le_port_80() {
        let options = Options::from_target(&target("adguard.lan", &[])).unwrap();
        assert_eq!(options.base_url, "http://adguard.lan:80");
        assert!(!options.insecure_tls);
        assert!(options.upstream_check);
        assert_eq!(options.request_timeout, DEFAULT_REQUEST_TIMEOUT);
    }

    #[test]
    fn l_adresse_accepte_les_formes_usuelles() {
        let cases = [
            ("10.0.0.53", &[][..], "http://10.0.0.53:80"),
            ("adguard.lan:3000", &[], "http://adguard.lan:3000"),
            ("https://dns.lan/adguard/control/", &[], "https://dns.lan/adguard"),
            ("fd00::53", &[], "http://[fd00::53]:80"),
            ("adguard.lan", &[("scheme", "https"), ("port", "8443")], "https://adguard.lan:8443"),
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
            ("upstream_check", "souvent"),
            ("port", "0"),
            ("request_timeout_seconds", "90"),
            ("scheme", "ftp"),
        ] {
            let error = Options::from_target(&target("x.lan", &[(key, value)])).unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "« {key} = {value} »");
        }
        assert!(Options::from_target(&target(" ", &[])).is_err());
    }
}
