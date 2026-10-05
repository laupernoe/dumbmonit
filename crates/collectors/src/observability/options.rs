//! Options de collecte lues sur la cible, communes aux quatre serveurs.
//!
//! Même contrat que les autres intégrations HTTP : aucun champ propre dans
//! `Target`, tout passe par `Target::tags`, et une cible sans étiquette
//! fonctionne sur une installation standard. La différence est le protocole par
//! défaut : ces serveurs écoutent en clair tant qu'on ne les a pas placés
//! derrière un proxy inverse, d'où `http` et non `https`.

use std::time::Duration;

use dumbmonit_proto::{ProbeError, Target};

/// Délai appliqué à chaque requête HTTP. `/metrics` d'un Loki chargé pèse
/// quelques centaines de kilo-octets : dix secondes laissent la marge.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct Options {
    /// Racine du serveur, sans barre oblique finale : `http://loki.lan:3100`,
    /// ou `https://logs.lan/loki` derrière un proxy inverse.
    pub base_url: String,
    pub insecure_tls: bool,
    pub request_timeout: Duration,
}

impl Options {
    pub fn from_target(target: &Target, default_port: u16) -> Result<Self, ProbeError> {
        let scheme = parse_scheme(tag(target, "scheme"))?;
        let port = parse_port(tag(target, "port"), default_port)?;
        Ok(Self {
            base_url: base_url(&target.address, scheme, port)?,
            insecure_tls: parse_bool(tag(target, "insecure_tls"))?,
            request_timeout: parse_timeout(tag(target, "request_timeout_seconds"))?,
        })
    }
}

fn tag<'a>(target: &'a Target, key: &str) -> Option<&'a str> {
    target.tags.get(key).map(|value| value.trim()).filter(|value| !value.is_empty())
}

fn parse_bool(value: Option<&str>) -> Result<bool, ProbeError> {
    match value {
        None => Ok(false),
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
    if !(1..=120).contains(&seconds) {
        return Err(ProbeError::Config(
            "Request timeout must be between 1 and 120 seconds".to_string(),
        ));
    }
    Ok(Duration::from_secs(seconds))
}

/// Compose la racine à partir de l'adresse saisie : `loki.lan`, `loki.lan:3100`,
/// une IPv6 avec ou sans crochets, ou une URL complète — chemin compris, pour
/// un serveur publié sous un préfixe par un proxy inverse.
fn base_url(address: &str, scheme: &str, port: u16) -> Result<String, ProbeError> {
    let address = address.trim().trim_end_matches('/');
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
            name: "logs".into(),
            address: address.into(),
            kind: "loki".into(),
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
    fn une_cible_sans_etiquette_parle_en_clair_sur_le_port_du_produit() {
        let options = Options::from_target(&target("loki.lan", &[]), 3100).unwrap();
        assert_eq!(options.base_url, "http://loki.lan:3100");
        assert!(!options.insecure_tls);
        assert_eq!(options.request_timeout, DEFAULT_REQUEST_TIMEOUT);
    }

    #[test]
    fn l_adresse_accepte_les_formes_usuelles() {
        let cases = [
            ("10.0.0.5", &[][..], "http://10.0.0.5:8428"),
            ("vm.lan:9999", &[], "http://vm.lan:9999"),
            ("https://logs.lan/victoria/", &[], "https://logs.lan/victoria"),
            ("fd00::1", &[], "http://[fd00::1]:8428"),
            ("[fd00::1]:8429", &[], "http://[fd00::1]:8429"),
            ("vm.lan", &[("scheme", "https"), ("port", "443")], "https://vm.lan:443"),
        ];
        for (address, tags, expected) in cases {
            let options = Options::from_target(&target(address, tags), 8428).unwrap();
            assert_eq!(options.base_url, expected, "adresse « {address} »");
        }
    }

    #[test]
    fn une_option_invalide_est_une_erreur_de_configuration() {
        for (key, value) in [
            ("insecure_tls", "peut-être"),
            ("port", "0"),
            ("port", "huit"),
            ("request_timeout_seconds", "900"),
            ("scheme", "ftp"),
        ] {
            let error = Options::from_target(&target("x.lan", &[(key, value)]), 1).unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "« {key} = {value} »");
        }
        assert!(Options::from_target(&target("  ", &[]), 1).is_err());
    }
}
