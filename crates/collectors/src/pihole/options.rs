//! Options de collecte lues sur la cible.
//!
//! Même contrat que les autres intégrations HTTP : tout passe par
//! `Target::tags`, et une cible sans étiquette fonctionne sur une installation
//! standard. Pi-hole v6 sert son interface et son API en clair sur le port 80,
//! et en HTTPS sur le 443 avec un certificat qu'il signe lui-même : le défaut
//! est donc `http`, le seul qui fonctionne sans rien régler.

use std::time::Duration;

use dumbmonit_proto::{ProbeError, Target};

pub const DEFAULT_PORT: u16 = 80;

/// Délai par requête HTTP. Toutes les réponses lues ici tiennent en quelques
/// kilo-octets ; dix secondes couvrent un Raspberry Pi occupé à reconstruire
/// sa liste de blocage.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub struct Options {
    /// Racine du serveur, sans barre oblique finale ni `/admin` ou `/api`.
    pub base_url: String,
    pub insecure_tls: bool,
    pub request_timeout: Duration,
}

impl Options {
    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let scheme = parse_scheme(tag(target, "scheme"))?;
        let port = parse_port(tag(target, "port"))?;
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

/// Compose la racine à partir de l'adresse saisie : `pi.hole`, `pi.hole:8080`,
/// une IPv6, ou une URL complète. L'adresse copiée depuis le navigateur finit
/// souvent par `/admin` (l'interface) ou `/api` : les chemins l'ajoutent déjà.
fn base_url(address: &str, scheme: &str, port: u16) -> Result<String, ProbeError> {
    let mut address = address.trim().trim_end_matches('/');
    for suffix in ["/admin", "/api"] {
        address = address.strip_suffix(suffix).unwrap_or(address).trim_end_matches('/');
    }
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
            kind: "pihole".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: tags
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<BTreeMap<_, _>>(),
            credential: Credential::None,
        }
    }

    #[test]
    fn une_cible_sans_etiquette_parle_en_clair_sur_le_port_80() {
        let options = Options::from_target(&target("pi.hole", &[])).unwrap();
        assert_eq!(options.base_url, "http://pi.hole:80");
        assert!(!options.insecure_tls);
        assert_eq!(options.request_timeout, DEFAULT_REQUEST_TIMEOUT);
    }

    #[test]
    fn l_adresse_accepte_les_formes_usuelles() {
        let cases = [
            ("10.0.0.53", &[][..], "http://10.0.0.53:80"),
            ("pi.hole:8080", &[], "http://pi.hole:8080"),
            ("http://pi.hole/admin/", &[], "http://pi.hole"),
            ("https://dns.lan/api", &[], "https://dns.lan"),
            ("fd00::53", &[], "http://[fd00::53]:80"),
            ("pi.hole", &[("scheme", "https"), ("port", "443")], "https://pi.hole:443"),
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
            ("port", "0"),
            ("request_timeout_seconds", "900"),
            ("scheme", "ftp"),
        ] {
            let error = Options::from_target(&target("x.lan", &[(key, value)])).unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "« {key} = {value} »");
        }
        assert!(Options::from_target(&target(" /admin ", &[])).is_err());
    }
}
