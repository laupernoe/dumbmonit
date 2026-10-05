//! Options de collecte lues sur la cible, communes aux applications
//! auto-hébergées.
//!
//! Même contrat que les autres intégrations HTTP : aucun champ propre dans
//! `Target`, tout passe par `Target::tags`, et une cible sans étiquette
//! fonctionne sur une installation standard. Chaque produit a son protocole
//! et son port par défaut : Nextcloud se publie presque toujours en HTTPS,
//! les autres écoutent en clair sur leur port tant qu'aucun proxy inverse
//! n'est placé devant.

use std::time::Duration;

use dumbmonit_proto::{ProbeError, Target};

/// Délai appliqué à chaque requête HTTP. Les statistiques d'Immich et de
/// Nextcloud comptent en base à chaque appel : quinze secondes laissent la
/// marge sur une grosse instance.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub struct Options {
    /// Racine de l'application, sans barre oblique finale :
    /// `https://cloud.lan`, ou `https://home.lan/paperless` derrière un proxy.
    pub base_url: String,
    pub insecure_tls: bool,
    pub request_timeout: Duration,
}

impl Options {
    pub fn from_target(
        target: &Target,
        default_scheme: &'static str,
        default_port: u16,
    ) -> Result<Self, ProbeError> {
        let scheme = parse_scheme(tag(target, "scheme"), default_scheme)?;
        let default_port = match (tag(target, "scheme"), scheme) {
            // Un protocole choisi sans port : celui de ce protocole, pas celui
            // du produit (`https` sans port, c'est 443).
            (Some(_), "https") if default_scheme == "http" => 443,
            (Some(_), "http") if default_scheme == "https" => 80,
            _ => default_port,
        };
        let port = parse_port(tag(target, "port"), default_port)?;
        Ok(Self {
            base_url: base_url(&target.address, scheme, port)?,
            insecure_tls: parse_bool(tag(target, "insecure_tls"))?,
            request_timeout: parse_timeout(tag(target, "request_timeout_seconds"))?,
        })
    }
}

pub fn tag<'a>(target: &'a Target, key: &str) -> Option<&'a str> {
    target.tags.get(key).map(|value| value.trim()).filter(|value| !value.is_empty())
}

pub fn parse_bool(value: Option<&str>) -> Result<bool, ProbeError> {
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

fn parse_scheme(value: Option<&str>, default: &'static str) -> Result<&'static str, ProbeError> {
    match value.map(str::to_ascii_lowercase).as_deref() {
        None => Ok(default),
        Some("http") => Ok("http"),
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

/// Compose la racine à partir de l'adresse saisie : `cloud.lan`,
/// `cloud.lan:8443`, une IPv6 avec ou sans crochets, ou une URL complète —
/// chemin compris, pour une application publiée sous un préfixe.
///
/// Le port par défaut d'un protocole n'est pas écrit : `https://cloud.lan` et
/// non `https://cloud.lan:443`, ce qui compte pour Nextcloud, qui compare
/// l'hôte reçu à ses domaines de confiance.
fn base_url(address: &str, scheme: &str, port: u16) -> Result<String, ProbeError> {
    let address = address.trim().trim_end_matches('/');
    if address.is_empty() {
        return Err(ProbeError::Config("Device address is empty".to_string()));
    }
    if address.starts_with("https://") || address.starts_with("http://") {
        return Ok(address.to_string());
    }
    let implicit = matches!((scheme, port), ("https", 443) | ("http", 80));
    let with_port = |host: String| {
        if implicit { format!("{scheme}://{host}") } else { format!("{scheme}://{host}:{port}") }
    };
    if address.starts_with('[') {
        return Ok(match address.rfind("]:") {
            Some(_) => format!("{scheme}://{address}"),
            None => with_port(address.to_string()),
        });
    }
    if address.matches(':').count() > 1 {
        return Ok(with_port(format!("[{address}]")));
    }
    if address.contains(':') {
        return Ok(format!("{scheme}://{address}"));
    }
    Ok(with_port(address.to_string()))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use dumbmonit_proto::Credential;

    use super::*;

    fn target(address: &str, tags: &[(&str, &str)]) -> Target {
        Target {
            id: 1,
            name: "app".into(),
            address: address.into(),
            kind: "immich".into(),
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
    fn une_cible_sans_etiquette_prend_le_protocole_et_le_port_du_produit() {
        let options = Options::from_target(&target("immich.lan", &[]), "http", 2283).unwrap();
        assert_eq!(options.base_url, "http://immich.lan:2283");
        assert!(!options.insecure_tls);
        assert_eq!(options.request_timeout, DEFAULT_REQUEST_TIMEOUT);
        let options = Options::from_target(&target("cloud.lan", &[]), "https", 443).unwrap();
        assert_eq!(options.base_url, "https://cloud.lan");
    }

    #[test]
    fn l_adresse_accepte_les_formes_usuelles() {
        let cases = [
            ("10.0.0.5", &[][..], "http://10.0.0.5:8096"),
            ("media.lan:9999", &[], "http://media.lan:9999"),
            ("https://home.lan/jellyfin/", &[], "https://home.lan/jellyfin"),
            ("fd00::1", &[], "http://[fd00::1]:8096"),
            ("[fd00::1]:8920", &[], "http://[fd00::1]:8920"),
            ("media.lan", &[("scheme", "https")], "https://media.lan"),
            ("media.lan", &[("scheme", "https"), ("port", "8920")], "https://media.lan:8920"),
        ];
        for (address, tags, expected) in cases {
            let options = Options::from_target(&target(address, tags), "http", 8096).unwrap();
            assert_eq!(options.base_url, expected, "adresse « {address} »");
        }
        let options =
            Options::from_target(&target("cloud.lan", &[("scheme", "http")]), "https", 443)
                .unwrap();
        assert_eq!(options.base_url, "http://cloud.lan");
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
            let error =
                Options::from_target(&target("x.lan", &[(key, value)]), "http", 1).unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "« {key} = {value} »");
        }
        assert!(Options::from_target(&target("  ", &[]), "http", 1).is_err());
    }
}
