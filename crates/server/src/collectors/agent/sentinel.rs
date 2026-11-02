//! Réglage du mode sentinelle d'une machine (migration 0230).
//!
//! Le canal de notification est un secret : chiffré au repos par la clé
//! d'instance, absent de toute réponse d'API, confié à la seule machine
//! désignée et seulement sur le canal chiffré des agents.

use anyhow::{Context, Result};
use dumbmonit_proto::{
    SENTINEL_MAX_INTERVAL_SECS, SENTINEL_MAX_THRESHOLD, SENTINEL_MAX_URLS,
    SENTINEL_MIN_INTERVAL_SECS, SentinelChannel, SentinelSetting, TargetId, https_authority,
};
use sqlx::{Row, SqlitePool};

use crate::crypto::Cipher;

/// Réglage tel que stocké, canal déchiffré.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stored {
    pub enabled: bool,
    pub urls: Vec<String>,
    pub interval_secs: u64,
    pub threshold: u32,
    pub channel: Option<SentinelChannel>,
}

impl Default for Stored {
    fn default() -> Self {
        Self { enabled: false, urls: Vec::new(), interval_secs: 60, threshold: 3, channel: None }
    }
}

/// Ce que l'interface envoie. `channel` absent : le canal en place est gardé ;
/// `clear_channel` : il est effacé.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Input {
    pub enabled: bool,
    #[serde(default)]
    pub urls: Vec<String>,
    pub interval_secs: u64,
    pub threshold: u32,
    #[serde(default)]
    pub channel: Option<SentinelChannel>,
    #[serde(default)]
    pub clear_channel: bool,
}

/// Réglage de la machine, s'il en existe un.
pub async fn load(pool: &SqlitePool, cipher: &Cipher, id: TargetId) -> Result<Option<Stored>> {
    let row = sqlx::query(
        "SELECT enabled, urls, interval_secs, threshold, channel_enc FROM agent_sentinel \
         WHERE target_id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("reading the sentinel setting")?;
    let Some(row) = row else { return Ok(None) };
    let urls: String = row.try_get("urls")?;
    let encrypted: Option<Vec<u8>> = row.try_get("channel_enc")?;
    // Un canal illisible (clé d'instance changée) vaut « pas de canal » : la
    // sentinelle ne démarre pas, l'interface dit qu'il faut le ressaisir.
    let channel = encrypted
        .and_then(|bytes| cipher.decrypt(&bytes).ok())
        .and_then(|plain| serde_json::from_slice(&plain).ok());
    Ok(Some(Stored {
        enabled: row.try_get::<i64, _>("enabled")? != 0,
        urls: serde_json::from_str(&urls).unwrap_or_default(),
        interval_secs: row.try_get::<i64, _>("interval_secs")?.max(0) as u64,
        threshold: row.try_get::<i64, _>("threshold")?.max(0) as u32,
        channel,
    }))
}

pub async fn save(
    pool: &SqlitePool,
    cipher: &Cipher,
    id: TargetId,
    stored: &Stored,
) -> Result<()> {
    let encrypted = match &stored.channel {
        Some(channel) => Some(cipher.encrypt(&serde_json::to_vec(channel)?)?),
        None => None,
    };
    sqlx::query(
        "INSERT INTO agent_sentinel \
             (target_id, enabled, urls, interval_secs, threshold, channel_enc) \
         VALUES (?, ?, ?, ?, ?, ?) \
         ON CONFLICT(target_id) DO UPDATE SET enabled = excluded.enabled, urls = excluded.urls, \
             interval_secs = excluded.interval_secs, threshold = excluded.threshold, \
             channel_enc = excluded.channel_enc, updated_at = datetime('now')",
    )
    .bind(id)
    .bind(i64::from(stored.enabled))
    .bind(serde_json::to_string(&stored.urls)?)
    .bind(stored.interval_secs as i64)
    .bind(i64::from(stored.threshold))
    .bind(encrypted)
    .execute(pool)
    .await
    .context("saving the sentinel setting")?;
    Ok(())
}

/// Canal valide : webhook HTTPS, ou serveur ntfy HTTPS et sujet sûr.
pub fn check_channel(channel: &SentinelChannel) -> Result<(), String> {
    match channel {
        SentinelChannel::Webhook { url } => {
            if https_authority(url).is_none() {
                return Err("The webhook must be a plain https:// address.".into());
            }
        }
        SentinelChannel::Ntfy { server, topic } => {
            if https_authority(server).is_none() {
                return Err("The ntfy server must be a plain https:// address.".into());
            }
            let safe = !topic.is_empty()
                && topic.len() <= 64
                && topic.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
            if !safe {
                return Err(
                    "The ntfy topic is 1 to 64 letters, digits, dashes or underscores.".into()
                );
            }
        }
    }
    Ok(())
}

/// Vérifie une saisie et la fond dans le réglage en place.
pub fn merge(current: Option<&Stored>, input: Input) -> Result<Stored, String> {
    let mut urls: Vec<String> = Vec::new();
    for raw in &input.urls {
        let url = raw.trim();
        if url.is_empty() || urls.iter().any(|u| u == url) {
            continue;
        }
        if https_authority(url).is_none() {
            return Err(format!("\"{url}\" is not a plain https:// address."));
        }
        urls.push(url.to_string());
    }
    if urls.len() > SENTINEL_MAX_URLS {
        return Err(format!("At most {SENTINEL_MAX_URLS} addresses."));
    }
    if !(SENTINEL_MIN_INTERVAL_SECS..=SENTINEL_MAX_INTERVAL_SECS).contains(&input.interval_secs) {
        return Err(format!(
            "The interval is {SENTINEL_MIN_INTERVAL_SECS} to {SENTINEL_MAX_INTERVAL_SECS} seconds."
        ));
    }
    if !(1..=SENTINEL_MAX_THRESHOLD).contains(&input.threshold) {
        return Err(format!("The threshold is 1 to {SENTINEL_MAX_THRESHOLD} failures."));
    }
    let channel = match (input.clear_channel, input.channel) {
        (_, Some(channel)) => {
            check_channel(&channel)?;
            Some(channel)
        }
        (true, None) => None,
        (false, None) => current.and_then(|c| c.channel.clone()),
    };
    if input.enabled && urls.is_empty() {
        return Err("Add at least one address to watch.".into());
    }
    if input.enabled && channel.is_none() {
        return Err("Set a webhook or an ntfy topic to notify.".into());
    }
    Ok(Stored {
        enabled: input.enabled,
        urls,
        interval_secs: input.interval_secs,
        threshold: input.threshold,
        channel,
    })
}

/// Ce que l'agent reçoit. Sur un canal non chiffré, le secret reste ici.
pub fn for_agent(stored: Stored, encrypted: bool) -> SentinelSetting {
    SentinelSetting {
        enabled: stored.enabled,
        urls: stored.urls,
        interval_secs: stored.interval_secs,
        threshold: stored.threshold,
        channel: if encrypted { stored.channel } else { None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> Input {
        Input {
            enabled: true,
            urls: vec!["https://dumbmonit.example.org/api/health".into()],
            interval_secs: 60,
            threshold: 3,
            channel: Some(SentinelChannel::Webhook { url: "https://hooks.example.org/x".into() }),
            clear_channel: false,
        }
    }

    #[test]
    fn a_valid_input_is_accepted() {
        let stored = merge(None, input()).expect("valid");
        assert!(stored.enabled);
        assert_eq!(stored.urls.len(), 1);
    }

    #[test]
    fn plain_http_addresses_are_refused() {
        let bad = Input { urls: vec!["http://dumbmonit.local".into()], ..input() };
        assert!(merge(None, bad).is_err());
        let bad = Input {
            channel: Some(SentinelChannel::Webhook { url: "http://hooks.example.org".into() }),
            ..input()
        };
        assert!(merge(None, bad).is_err());
    }

    #[test]
    fn the_bounds_are_enforced() {
        assert!(merge(None, Input { interval_secs: 5, ..input() }).is_err());
        assert!(merge(None, Input { threshold: 0, ..input() }).is_err());
        assert!(merge(None, Input { threshold: 99, ..input() }).is_err());
        let many = (0..6).map(|i| format!("https://h{i}.example.org")).collect();
        assert!(merge(None, Input { urls: many, ..input() }).is_err());
    }

    #[test]
    fn omitting_the_channel_keeps_it_and_clearing_removes_it() {
        let kept = merge(None, input()).unwrap();
        let again = merge(Some(&kept), Input { channel: None, ..input() }).unwrap();
        assert_eq!(again.channel, kept.channel, "omitted = unchanged");

        let cleared = merge(
            Some(&kept),
            Input { channel: None, clear_channel: true, enabled: false, ..input() },
        )
        .unwrap();
        assert_eq!(cleared.channel, None);

        // Activée sans canal : refusé.
        assert!(merge(Some(&cleared), Input { channel: None, ..input() }).is_err());
    }

    #[test]
    fn the_channel_only_leaves_for_an_encrypted_agent_channel() {
        let stored = merge(None, input()).unwrap();
        assert!(for_agent(stored.clone(), true).channel.is_some());
        assert!(for_agent(stored, false).channel.is_none());
    }

    #[test]
    fn an_unsafe_ntfy_topic_is_refused() {
        let channel =
            SentinelChannel::Ntfy { server: "https://ntfy.sh".into(), topic: "a/b".into() };
        assert!(check_channel(&channel).is_err());
    }
}
