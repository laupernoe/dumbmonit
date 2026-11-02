//! Mode « sentinelle » : un agent surveille que le serveur (et, au besoin, une
//! seconde instance exposée) répond, et alerte lui-même quand ce n'est plus le
//! cas — le serveur étant justement celui qui est tombé.
//!
//! Le réglage voyage dans l'accusé de réception des lots
//! ([`crate::PushAck::sentinel`]), uniquement vers la machine désignée, et
//! uniquement sur le canal chiffré : il porte le seul secret que la sentinelle
//! ait à connaître, celui de son canal de notification.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Adresses sondées au plus.
pub const SENTINEL_MAX_URLS: usize = 5;
/// Période de sondage minimale et maximale, en secondes.
pub const SENTINEL_MIN_INTERVAL_SECS: u64 = 15;
pub const SENTINEL_MAX_INTERVAL_SECS: u64 = 3_600;
/// Nombre d'échecs consécutifs exigé : borne haute acceptée.
pub const SENTINEL_MAX_THRESHOLD: u32 = 20;
/// Longueur maximale d'une adresse.
pub const SENTINEL_MAX_URL_LEN: usize = 500;

/// Canal de notification de la sentinelle : un seul, volontairement restreint.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SentinelChannel {
    /// Webhook HTTPS générique : un `POST` JSON.
    Webhook { url: String },
    /// Sujet ntfy : un `POST` en texte brut sur `{server}/{topic}`.
    Ntfy { server: String, topic: String },
}

impl SentinelChannel {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Webhook { .. } => "webhook",
            Self::Ntfy { .. } => "ntfy",
        }
    }
}

// L'adresse d'un webhook ou le sujet d'un ntfy sont des secrets : jamais dans
// un journal, même par un `{:?}` distrait.
impl fmt::Debug for SentinelChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SentinelChannel::{}(…)", self.kind())
    }
}

/// Réglage de la sentinelle d'une machine.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SentinelSetting {
    pub enabled: bool,
    /// Adresses HTTPS à sonder (le serveur principal, la passerelle exposée).
    #[serde(default)]
    pub urls: Vec<String>,
    /// Période entre deux sondages, en secondes.
    #[serde(default)]
    pub interval_secs: u64,
    /// Échecs consécutifs d'une adresse avant d'alerter.
    #[serde(default)]
    pub threshold: u32,
    /// Absent tant que le canal n'est pas réglé, et quand le serveur refuse de
    /// le confier à un canal non chiffré.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<SentinelChannel>,
}

/// Hôte (avec port éventuel) d'une adresse `https://`, ou `None` si elle n'est
/// pas une adresse HTTPS acceptable : autre schéma, hôte vide, identifiants
/// dans l'adresse, espaces ou caractères de contrôle.
pub fn https_authority(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let unusable = |c: char| c.is_whitespace() || c.is_control();
    if url.len() > SENTINEL_MAX_URL_LEN || url.chars().any(unusable) {
        return None;
    }
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..end];
    if authority.is_empty() || authority.contains('@') || authority.starts_with(':') {
        return None;
    }
    Some(authority)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_plain_https_addresses_are_accepted() {
        assert_eq!(https_authority("https://mon.example.org/health"), Some("mon.example.org"));
        assert_eq!(https_authority("https://10.0.0.1:8443"), Some("10.0.0.1:8443"));
        assert_eq!(https_authority("http://mon.example.org"), None);
        assert_eq!(https_authority("https://"), None);
        assert_eq!(https_authority("https://user:pw@mon.example.org"), None);
        assert_eq!(https_authority("https://mon example.org"), None);
        assert_eq!(https_authority("ftp://mon.example.org"), None);
    }

    #[test]
    fn the_channel_secret_never_reaches_a_debug_output() {
        let channel = SentinelChannel::Webhook { url: "https://hooks.example.org/s3cret".into() };
        assert!(!format!("{channel:?}").contains("s3cret"));
        let setting = SentinelSetting { channel: Some(channel), ..SentinelSetting::default() };
        assert!(!format!("{setting:?}").contains("s3cret"));
    }

    #[test]
    fn a_setting_survives_a_round_trip() {
        let setting = SentinelSetting {
            enabled: true,
            urls: vec!["https://a.example.org".into()],
            interval_secs: 60,
            threshold: 3,
            channel: Some(SentinelChannel::Ntfy {
                server: "https://ntfy.sh".into(),
                topic: "dumbmonit".into(),
            }),
        };
        let json = serde_json::to_string(&setting).unwrap();
        assert!(json.contains(r#""kind":"ntfy""#));
        assert_eq!(serde_json::from_str::<SentinelSetting>(&json).unwrap(), setting);
    }
}
