//! Lecture des étiquettes communes aux intégrations REST et SOAP récentes
//! (UniFi, Home Assistant, vSphere).
//!
//! Même contrat que les `options.rs` plus anciens : aucun champ propre dans
//! `Target`, tout passe par `Target::tags`, et une cible sans étiquette
//! fonctionne sur une installation standard. Ce module factorise ce que ces
//! trois intégrations lisaient à l'identique — protocole, port, certificat,
//! délai — et les petites lectures typées dont elles ont besoin en plus.

use std::time::Duration;

use dumbmonit_proto::{ProbeError, Target};

/// Réglages de connexion d'une cible HTTP.
#[derive(Debug, Clone)]
pub struct Connection {
    /// Racine du service, sans barre oblique finale : `https://unifi.lan:8443`,
    /// ou `https://proxy.lan/hass` derrière un proxy inverse.
    pub base_url: String,
    pub insecure_tls: bool,
    pub request_timeout: Duration,
}

impl Connection {
    /// `default_scheme` est `http` ou `https` ; le port par défaut suit le
    /// protocole choisi (`http_port` ou `https_port`).
    pub fn from_target(
        target: &Target,
        default_scheme: &'static str,
        http_port: u16,
        https_port: u16,
        default_timeout: Duration,
    ) -> Result<Self, ProbeError> {
        let scheme = match tag(target, "scheme").map(str::to_ascii_lowercase).as_deref() {
            None => default_scheme,
            Some("http") => "http",
            Some("https") => "https",
            Some(other) => {
                return Err(ProbeError::Config(format!(
                    "Unknown protocol \"{other}\": http or https"
                )));
            }
        };
        let default_port = if scheme == "http" { http_port } else { https_port };
        let port = match tag(target, "port") {
            None => default_port,
            Some(raw) => raw
                .parse()
                .ok()
                .filter(|port| *port > 0)
                .ok_or_else(|| ProbeError::Config(format!("Invalid port: \"{raw}\"")))?,
        };
        let request_timeout = match tag(target, "request_timeout_seconds") {
            None => default_timeout,
            Some(raw) => Duration::from_secs(parse_u64_in(
                Some(raw),
                0,
                1..=120,
                "Request timeout (seconds)",
            )?),
        };
        Ok(Self {
            base_url: base_url(&target.address, scheme, port)?,
            insecure_tls: parse_bool(tag(target, "insecure_tls"), false)?,
            request_timeout,
        })
    }
}

/// Valeur d'une étiquette, sans espaces, ou `None` si elle est absente ou vide.
pub fn tag<'a>(target: &'a Target, key: &str) -> Option<&'a str> {
    target.tags.get(key).map(|value| value.trim()).filter(|value| !value.is_empty())
}

pub fn parse_bool(value: Option<&str>, default: bool) -> Result<bool, ProbeError> {
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

/// Un entier borné ; `what` nomme l'option dans le message d'erreur.
pub fn parse_u64_in(
    value: Option<&str>,
    default: u64,
    range: std::ops::RangeInclusive<u64>,
    what: &str,
) -> Result<u64, ProbeError> {
    let Some(raw) = value else { return Ok(default) };
    let parsed: u64 = raw
        .parse()
        .map_err(|_| ProbeError::Config(format!("{what}: \"{raw}\" is not a number")))?;
    if !range.contains(&parsed) {
        return Err(ProbeError::Config(format!(
            "{what} must be between {} and {}, got {parsed}",
            range.start(),
            range.end()
        )));
    }
    Ok(parsed)
}

/// Une liste séparée par des virgules ou des espaces, en minuscules.
pub fn parse_list(value: Option<&str>) -> Vec<String> {
    value
        .unwrap_or_default()
        .split([',', ' ', ';'])
        .map(|item| item.trim().to_ascii_lowercase())
        .filter(|item| !item.is_empty())
        .collect()
}

/// Compose la racine à partir de l'adresse saisie : `hote`, `hote:port`, une
/// IPv6 avec ou sans crochets, ou une URL complète — chemin compris, pour un
/// service publié sous un préfixe par un proxy inverse.
pub fn base_url(address: &str, scheme: &str, port: u16) -> Result<String, ProbeError> {
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
pub mod test_support {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use dumbmonit_proto::{Credential, Target};

    /// Une cible de test : adresse, étiquettes et identifiant choisis.
    pub fn target(
        kind: &str,
        address: &str,
        tags: &[(&str, &str)],
        credential: Credential,
    ) -> Target {
        Target {
            id: 7,
            name: kind.into(),
            address: address.into(),
            kind: kind.into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: tags
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
            credential,
            group_name: String::new(),
            position: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use dumbmonit_proto::Credential;

    use super::test_support::target;
    use super::*;

    fn connection(address: &str, tags: &[(&str, &str)]) -> Result<Connection, ProbeError> {
        Connection::from_target(
            &target("x", address, tags, Credential::None),
            "https",
            80,
            443,
            Duration::from_secs(10),
        )
    }

    #[test]
    fn une_cible_sans_etiquette_prend_le_protocole_et_le_port_par_defaut() {
        let options = connection("host.lan", &[]).unwrap();
        assert_eq!(options.base_url, "https://host.lan:443");
        assert!(!options.insecure_tls);
        assert_eq!(options.request_timeout, Duration::from_secs(10));
        let options = connection("host.lan", &[("scheme", "http")]).unwrap();
        assert_eq!(options.base_url, "http://host.lan:80");
    }

    #[test]
    fn l_adresse_accepte_les_formes_usuelles() {
        let cases = [
            ("10.0.0.5", &[][..], "https://10.0.0.5:443"),
            ("host.lan:8443", &[], "https://host.lan:8443"),
            ("https://proxy.lan/hass/", &[], "https://proxy.lan/hass"),
            ("fd00::1", &[], "https://[fd00::1]:443"),
            ("[fd00::1]:8123", &[], "https://[fd00::1]:8123"),
            ("host.lan", &[("port", "8443")], "https://host.lan:8443"),
        ];
        for (address, tags, expected) in cases {
            assert_eq!(connection(address, tags).unwrap().base_url, expected, "« {address} »");
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
            let error = connection("x.lan", &[(key, value)]).unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "« {key} = {value} »");
        }
        assert!(connection("  ", &[]).is_err());
    }

    #[test]
    fn les_listes_et_les_entiers_bornes() {
        assert_eq!(parse_list(Some("Button, scene ;event")), ["button", "scene", "event"]);
        assert!(parse_list(None).is_empty());
        assert_eq!(parse_u64_in(None, 20, 1..=100, "x").unwrap(), 20);
        assert_eq!(parse_u64_in(Some("35"), 20, 1..=100, "x").unwrap(), 35);
        assert!(parse_u64_in(Some("0"), 20, 1..=100, "x").is_err());
        assert!(parse_u64_in(Some("abc"), 20, 1..=100, "x").is_err());
    }
}
