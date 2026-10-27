//! Sauvegarde des configurations réseau (type `netconfig`), à la manière
//! d'Oxidized ou de RANCID : relevé de la configuration d'un commutateur, d'un
//! routeur ou d'un pare-feu, par SSH ou par son API.
//!
//! Ce module ne fait que le relevé, partagé entre le serveur et l'agent
//! relais : versions, chiffrement, comparaison et masquage à l'affichage sont
//! l'affaire du serveur (`crates/server/src/netconfig`). Rien n'est écrit sur
//! disque ici.
//!
//! Les profils (`profiles/netconfig/*.yaml`, voir [`profile`]) disent quelles
//! commandes envoyer : l'interface ne peut en choisir aucune autre, et aucun
//! champ saisi n'est interpolé dans une commande.

pub mod http;
pub mod profile;
pub mod ssh;
pub mod text;

use std::time::Duration;

use dumbmonit_proto::{Credential, NetconfigCapture, ProbeError, Target};

pub use profile::{PROFILE_IDS, Profile, Transport};

/// Type de cible, tel qu'enregistré dans `Target::kind`.
pub const KIND: &str = "netconfig";

/// Taille maximale d'une configuration relevée. Au-delà, le relevé échoue :
/// mieux vaut une erreur visible qu'une version tronquée qui passerait pour
/// un changement.
pub const MAX_CONFIG_BYTES: usize = 4 * 1024 * 1024;

/// Ajoute `data` au tampon d'un relevé, sous la borne [`MAX_CONFIG_BYTES`]
/// comptée sur tout ce que le relevé a reçu (`total`). Au-delà, l'erreur :
/// rien n'est ajouté.
pub fn append_bounded(buf: &mut Vec<u8>, total: &mut usize, data: &[u8]) -> Result<(), ProbeError> {
    let next = total.saturating_add(data.len());
    if next > MAX_CONFIG_BYTES {
        return Err(too_large());
    }
    *total = next;
    buf.extend_from_slice(data);
    Ok(())
}

/// L'erreur d'une configuration plus grande que [`MAX_CONFIG_BYTES`].
pub fn too_large() -> ProbeError {
    ProbeError::Protocol(format!(
        "the configuration is larger than {} MiB, which is refused",
        MAX_CONFIG_BYTES / (1024 * 1024)
    ))
}

/// Délai par défaut d'un relevé complet, et ses bornes (option
/// `fetch_timeout_seconds`).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
pub const MIN_TIMEOUT_SECS: u64 = 10;
pub const MAX_TIMEOUT_SECS: u64 = 110;

/// Étiquette qu'ajoute le serveur à la copie de la cible confiée à un agent
/// relais : l'empreinte de clé d'hôte mémorisée. Toujours écrasée ou retirée
/// par le serveur, jamais lue sur la cible enregistrée.
pub const RELAY_HOST_KEY_TAG: &str = "netconfig_host_key";

/// Échec d'un relevé.
#[derive(Debug)]
pub enum Failure {
    /// La clé d'hôte présentée n'est pas celle mémorisée : connexion coupée
    /// avant l'authentification.
    HostKeyMismatch {
        seen: String,
    },
    Probe(ProbeError),
}

impl From<ProbeError> for Failure {
    fn from(error: ProbeError) -> Self {
        Self::Probe(error)
    }
}

/// Un relevé réussi.
pub struct Capture {
    pub config: String,
    /// Empreinte de la clé d'hôte SSH ; `None` pour un profil HTTP.
    pub host_key: Option<String>,
}

/// Jamais le texte : une configuration contient des secrets.
impl std::fmt::Debug for Capture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Capture")
            .field("config_bytes", &self.config.len())
            .field("host_key", &self.host_key)
            .finish()
    }
}

/// Réglages d'un relevé lus dans les options de la cible.
pub struct FetchOptions {
    pub profile: &'static Profile,
    pub timeout: Duration,
    pub insecure_tls: bool,
}

impl FetchOptions {
    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let id = target.tags.get("profile").map(|s| s.trim()).unwrap_or("");
        if id.is_empty() {
            return Err(ProbeError::Config("choose the device model (\"Profile\")".into()));
        }
        let profile = profile::find(id)
            .ok_or_else(|| ProbeError::Config(format!("unknown device profile \"{id}\"")))?;
        let timeout = match target.tags.get("fetch_timeout_seconds").map(|s| s.trim()) {
            None | Some("") => DEFAULT_TIMEOUT,
            Some(raw) => match raw.parse::<u64>() {
                Ok(secs) if (MIN_TIMEOUT_SECS..=MAX_TIMEOUT_SECS).contains(&secs) => {
                    Duration::from_secs(secs)
                }
                _ => {
                    return Err(ProbeError::Config(format!(
                        "fetch timeout must be between {MIN_TIMEOUT_SECS} and {MAX_TIMEOUT_SECS} \
                         seconds"
                    )));
                }
            },
        };
        let insecure_tls = matches!(
            target.tags.get("insecure_tls").map(|s| s.trim().to_ascii_lowercase()).as_deref(),
            Some("true" | "1" | "yes" | "on")
        );
        Ok(Self { profile, timeout, insecure_tls })
    }
}

/// Relève la configuration de `target`. `pinned` : l'empreinte de clé d'hôte
/// mémorisée (`None` au premier relevé, qui l'établit).
pub async fn fetch(target: &Target, pinned: Option<&str>) -> Result<Capture, Failure> {
    let options = FetchOptions::from_target(target)?;
    let profile = options.profile;
    let (host, port) =
        crate::uptime::tags::split_host_port(&target.address, profile.default_port())?;
    // Chaque étape (connexion, invite, commande) a droit au délai entier ;
    // l'ensemble est borné par le même délai, doublé.
    let budget = options.timeout * 2;
    let work = fetch_raw(target, &options, &host, port, pinned);
    let capture = match tokio::time::timeout(budget, work).await {
        Ok(result) => result?,
        Err(_) => return Err(Failure::Probe(ProbeError::Timeout(budget))),
    };
    let strip = profile.strip_rules();
    let config = text::strip_lines(&capture.config, &strip);
    if config.trim().is_empty() {
        return Err(Failure::Probe(ProbeError::Protocol(
            "the device returned an empty configuration".into(),
        )));
    }
    Ok(Capture { config, host_key: capture.host_key })
}

async fn fetch_raw(
    target: &Target,
    options: &FetchOptions,
    host: &str,
    port: u16,
    pinned: Option<&str>,
) -> Result<Capture, Failure> {
    let profile = options.profile;
    if profile.uses_ssh() {
        let login = match &target.credential {
            Credential::SshLogin { username, password, enable_secret } => {
                ssh::Login { username, password, enable_secret }
            }
            Credential::UsernamePassword { username, password } => {
                ssh::Login { username, password, enable_secret: "" }
            }
            _ => {
                return Err(Failure::Probe(ProbeError::Config(
                    "this profile needs an SSH login (user name and password)".into(),
                )));
            }
        };
        let fetched = ssh::fetch(host, port, &login, profile, pinned, options.timeout).await?;
        return Ok(Capture { config: fetched.config, host_key: Some(fetched.host_key) });
    }
    let Some(spec) = &profile.http else {
        return Err(Failure::Probe(ProbeError::Config("profile without HTTP".into())));
    };
    let credential = match &target.credential {
        Credential::UsernamePassword { username, password } => {
            http::HttpCredential::Basic { username, password }
        }
        Credential::ApiToken { token } => http::HttpCredential::Token(token),
        _ => {
            return Err(Failure::Probe(ProbeError::Config(
                "this profile needs an API key and secret, or an API token".into(),
            )));
        }
    };
    let config =
        http::fetch(host, port, spec, &credential, options.insecure_tls, options.timeout).await?;
    Ok(Capture { config, host_key: None })
}

/// Relevé fait par un agent relais pour le serveur : l'empreinte mémorisée
/// arrive dans [`RELAY_HOST_KEY_TAG`]. Rend l'erreur éventuelle (texte de
/// `ProbeError`, comme toute sonde relayée) et ce qu'il faut renvoyer.
pub async fn relay_capture(target: &Target) -> (Option<String>, NetconfigCapture) {
    let pinned = target.tags.get(RELAY_HOST_KEY_TAG).map(|s| s.trim()).filter(|s| !s.is_empty());
    match fetch(target, pinned).await {
        Ok(capture) => (
            None,
            NetconfigCapture {
                config: Some(capture.config),
                host_key: capture.host_key,
                host_key_mismatch: false,
            },
        ),
        Err(Failure::HostKeyMismatch { seen }) => (
            Some(
                ProbeError::Protocol("the SSH host key changed: connection refused".into())
                    .to_string(),
            ),
            NetconfigCapture { config: None, host_key: Some(seen), host_key_mismatch: true },
        ),
        Err(Failure::Probe(error)) => (Some(error.to_string()), NetconfigCapture::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(tags: &[(&str, &str)]) -> Target {
        Target {
            id: 1,
            name: "sw1".into(),
            address: "192.0.2.1".into(),
            kind: KIND.into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: tags.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            credential: Credential::None,
            group_name: String::new(),
            position: 0,
        }
    }

    #[test]
    fn options_need_a_known_profile_and_a_bounded_timeout() {
        assert!(FetchOptions::from_target(&target(&[])).is_err());
        assert!(FetchOptions::from_target(&target(&[("profile", "nope")])).is_err());
        let ok = FetchOptions::from_target(&target(&[("profile", "cisco_ios")])).unwrap();
        assert_eq!(ok.timeout, DEFAULT_TIMEOUT);
        assert!(
            FetchOptions::from_target(&target(&[
                ("profile", "cisco_ios"),
                ("fetch_timeout_seconds", "3600")
            ]))
            .is_err()
        );
    }

    #[tokio::test]
    async fn the_wrong_credential_type_is_a_configuration_error() {
        let result = fetch(&target(&[("profile", "cisco_ios")]), None).await;
        assert!(matches!(result, Err(Failure::Probe(ProbeError::Config(_)))));
    }

    /// Borne de taille : ce qui la dépasserait n'est jamais ajouté, et le
    /// compte porte sur tout le relevé, pas sur un seul morceau.
    #[test]
    fn the_size_bound_counts_the_whole_fetch() {
        let mut buf = Vec::new();
        let mut total = 0usize;
        let chunk = vec![b'x'; MAX_CONFIG_BYTES / 2];
        append_bounded(&mut buf, &mut total, &chunk).unwrap();
        append_bounded(&mut buf, &mut total, &chunk).unwrap();
        assert_eq!(total, MAX_CONFIG_BYTES);
        assert!(append_bounded(&mut buf, &mut total, b"y").is_err());
        assert_eq!(buf.len(), MAX_CONFIG_BYTES, "nothing past the bound is kept");
    }

    /// Confiance au premier usage, puis l'empreinte exacte et elle seule.
    #[test]
    fn a_changed_host_key_is_never_accepted() {
        let pinned = "SHA256:Nh0Me49Zh9fDw/VYUfq43IJmI1T+XrjiYONPND8GzaM";
        assert!(ssh::accepts_host_key(None, pinned), "first use");
        assert!(ssh::accepts_host_key(Some(pinned), pinned));
        let other = "SHA256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        assert!(!ssh::accepts_host_key(Some(pinned), other));
        assert!(!ssh::accepts_host_key(Some(pinned), ""));
        assert!(!ssh::accepts_host_key(Some(pinned), &pinned.to_lowercase()));
    }

    /// Une adresse qui ferait partir l'identifiant ailleurs est refusée avant
    /// toute requête.
    #[tokio::test]
    async fn an_api_address_cannot_redirect_the_credential() {
        let spec = profile::HttpSpec {
            path: "/api/core/backup/download/this".into(),
            auth: profile::HttpAuth::Basic,
            header: None,
        };
        let credential = http::HttpCredential::Basic { username: "key", password: "secret" };
        for host in ["evil.example@192.0.2.1", "192.0.2.1/x", "192.0.2.1?a", "a b", ""] {
            let result =
                http::fetch(host, 443, &spec, &credential, false, Duration::from_secs(1)).await;
            assert!(matches!(result, Err(ProbeError::Config(_))), "{host}: {result:?}");
        }
    }

    #[tokio::test]
    async fn an_enable_secret_with_a_line_break_is_refused_before_connecting() {
        let mut t = target(&[("profile", "cisco_ios")]);
        // Adresse de documentation : la vérification doit échouer avant toute connexion.
        t.credential = Credential::SshLogin {
            username: "backup".into(),
            password: "pw".into(),
            enable_secret: "x\nreload".into(),
        };
        let result = fetch(&t, None).await;
        assert!(matches!(result, Err(Failure::Probe(ProbeError::Config(_)))), "{result:?}");
    }
}
