//! Clés d'accès : défis en cours, identité de la partie de confiance et stockage.
//!
//! La vérification cryptographique est dans [`super::webauthn`] ; ce module garde
//! ce qui lui est extérieur.
//!
//! **Défis.** Un défi est tiré au hasard à chaque cérémonie, gardé en mémoire
//! cinq minutes et consommé par son premier usage, réussi ou non : il ne peut
//! pas être rejoué. Un redémarrage les annule, l'utilisateur recommence.
//!
//! **Identité.** Le RP ID est le nom d'hôte sous lequel l'instance est ouverte,
//! l'origine est celle que le navigateur annonce. Si `DUMBMONIT_PUBLIC_URL` est
//! réglée, c'est elle qui fait foi : ouvrir l'instance par une autre adresse
//! (IP, autre nom) est refusé plutôt que d'enregistrer une clé qui ne marcherait
//! qu'ici. Sans elle, l'en-tête `Origin` — que seul le navigateur peut poser —
//! est utilisé ; le navigateur lie de toute façon la clé au domaine de la page,
//! si bien qu'un site tiers ne peut obtenir aucune assertion pour le nôtre.
//! HTTPS est exigé, sauf pour `localhost`.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use axum::http::{HeaderMap, header};
use serde::Serialize;
use sqlx::{Row, SqlitePool};

use super::AuthError;
use super::webauthn::RelyingParty;

const TTL: Duration = Duration::from_secs(5 * 60);
const MAX_PENDING: usize = 1_000;
/// Nombre maximal de clés d'accès par compte.
pub const MAX_PER_USER: i64 = 20;
/// Longueur maximale du nom d'une clé.
pub const MAX_NAME_CHARS: usize = 60;

/// Cérémonie en cours.
pub enum Ceremony {
    /// Enregistrement d'une clé pour ce compte.
    Register { user_id: i64 },
    /// Connexion sans nom d'utilisateur (clé découvrable).
    Login,
}

pub struct Pending {
    pub ceremony: Ceremony,
    pub challenge: [u8; 32],
    pub rp: RelyingParty,
    started: Instant,
}

static PENDING: LazyLock<Mutex<HashMap<String, Pending>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Ouvre une cérémonie : rend son identifiant et le défi.
pub fn start(ceremony: Ceremony, rp: RelyingParty) -> (String, [u8; 32]) {
    let id = hex::encode(rand::random::<[u8; 16]>());
    let challenge: [u8; 32] = rand::random();
    let now = Instant::now();
    if let Ok(mut pending) = PENDING.lock() {
        pending.retain(|_, p| now.duration_since(p.started) < TTL);
        if pending.len() >= MAX_PENDING
            && let Some(oldest) =
                pending.iter().min_by_key(|(_, p)| p.started).map(|(k, _)| k.clone())
        {
            pending.remove(&oldest);
        }
        pending.insert(id.clone(), Pending { ceremony, challenge, rp, started: now });
    }
    (id, challenge)
}

/// Reprend une cérémonie, une seule fois. `None` : inconnue ou expirée.
pub fn take(id: &str) -> Option<Pending> {
    let mut pending = PENDING.lock().ok()?;
    let entry = pending.remove(id)?;
    (entry.started.elapsed() < TTL).then_some(entry)
}

/// Identité de la partie de confiance pour une requête.
pub fn relying_party(headers: &HeaderMap, public_url: Option<&str>) -> Result<RelyingParty, AuthError> {
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty() && value != "null")
        .ok_or_else(|| {
            AuthError::Invalid("Passkeys need a browser request that carries its origin.".into())
        })?;
    let (scheme, authority) = origin
        .split_once("://")
        .ok_or_else(|| AuthError::Invalid("Unreadable origin.".into()))?;
    if authority.contains(['/', '?', '#', '@']) || !matches!(scheme, "http" | "https") {
        return Err(AuthError::Invalid("Unreadable origin.".into()));
    }
    let host = host_of(authority);
    if host.is_empty() {
        return Err(AuthError::Invalid("Unreadable origin.".into()));
    }
    let local = host == "localhost" || host.ends_with(".localhost");
    if scheme != "https" && !local {
        return Err(AuthError::Invalid(
            "Passkeys need HTTPS (or http://localhost): open DumbMonit through its HTTPS address."
                .into(),
        ));
    }
    if host.parse::<std::net::IpAddr>().is_ok() || host.starts_with('[') {
        return Err(AuthError::Invalid(
            "Passkeys are tied to a domain name, not an IP address: open DumbMonit by its name."
                .into(),
        ));
    }
    if let Some(configured) = public_url.map(origin_of)
        && !configured.is_empty()
        && !configured.eq_ignore_ascii_case(&origin)
    {
        return Err(AuthError::Invalid(format!(
            "Open DumbMonit at its public address ({configured}) to use passkeys."
        )));
    }
    Ok(RelyingParty { id: host.to_ascii_lowercase(), origin })
}

/// `schéma://hôte[:port]` d'une adresse, chemin retiré.
fn origin_of(url: &str) -> String {
    let url = url.trim();
    match url.split_once("://") {
        Some((scheme, rest)) => {
            format!("{scheme}://{}", rest.split(['/', '?', '#']).next().unwrap_or_default())
        }
        None => String::new(),
    }
}

/// Nom d'hôte d'une autorité `hôte[:port]`.
fn host_of(authority: &str) -> String {
    if authority.starts_with('[') {
        return authority.to_string();
    }
    authority.rsplit_once(':').map_or(authority, |(host, _)| host).to_string()
}

// ------------------------------------------------------------------- stockage

/// Une clé d'accès, telle que l'interface la voit.
#[derive(Debug, Clone, Serialize)]
pub struct PasskeyView {
    pub id: i64,
    pub name: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    /// Copiée dans un trousseau synchronisé (iCloud, Google, gestionnaire de mots de passe).
    pub synced: bool,
}

/// Une clé d'accès avec ce qu'il faut pour vérifier une assertion.
pub struct Stored {
    pub id: i64,
    pub user_id: i64,
    pub public_key: Vec<u8>,
    pub sign_count: u32,
}

pub async fn list(pool: &SqlitePool, user_id: i64) -> Result<Vec<PasskeyView>> {
    let rows = sqlx::query(
        "SELECT id, name, created_at, last_used_at, backup_state
         FROM passkeys WHERE user_id = ? ORDER BY created_at, id",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .context("liste des clés d'accès")?;
    rows.iter()
        .map(|row| {
            Ok(PasskeyView {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                created_at: row.try_get("created_at")?,
                last_used_at: row.try_get("last_used_at")?,
                synced: row.try_get::<i64, _>("backup_state")? != 0,
            })
        })
        .collect()
}

pub async fn credential_ids(pool: &SqlitePool, user_id: i64) -> Result<Vec<Vec<u8>>> {
    let rows = sqlx::query("SELECT credential_id FROM passkeys WHERE user_id = ?")
        .bind(user_id)
        .fetch_all(pool)
        .await
        .context("identifiants des clés d'accès")?;
    rows.iter().map(|row| Ok(row.try_get("credential_id")?)).collect()
}

pub async fn count(pool: &SqlitePool, user_id: Option<i64>) -> Result<i64> {
    let row = match user_id {
        Some(id) => sqlx::query("SELECT COUNT(*) AS n FROM passkeys WHERE user_id = ?").bind(id),
        None => sqlx::query("SELECT COUNT(*) AS n FROM passkeys"),
    }
    .fetch_one(pool)
    .await
    .context("décompte des clés d'accès")?;
    Ok(row.try_get("n")?)
}

pub struct NewPasskey<'a> {
    pub user_id: i64,
    pub name: &'a str,
    pub credential_id: &'a [u8],
    pub public_key: &'a [u8],
    pub sign_count: u32,
    pub backup_eligible: bool,
    pub backup_state: bool,
    pub transports: &'a [String],
}

/// Range une clé. Faux si cet identifiant existe déjà (même clé, autre compte).
pub async fn insert(pool: &SqlitePool, key: &NewPasskey<'_>) -> Result<bool> {
    let result = sqlx::query(
        "INSERT OR IGNORE INTO passkeys
             (user_id, name, credential_id, public_key, sign_count, backup_eligible, backup_state,
              transports)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(key.user_id)
    .bind(key.name)
    .bind(key.credential_id)
    .bind(key.public_key)
    .bind(i64::from(key.sign_count))
    .bind(i64::from(key.backup_eligible))
    .bind(i64::from(key.backup_state))
    .bind(serde_json::to_string(key.transports)?)
    .execute(pool)
    .await
    .context("enregistrement d'une clé d'accès")?;
    Ok(result.rows_affected() > 0)
}

pub async fn find(pool: &SqlitePool, credential_id: &[u8]) -> Result<Option<Stored>> {
    let row = sqlx::query(
        "SELECT id, user_id, public_key, sign_count FROM passkeys WHERE credential_id = ?",
    )
    .bind(credential_id)
    .fetch_optional(pool)
    .await
    .context("recherche d'une clé d'accès")?;
    row.map(|row| {
        Ok(Stored {
            id: row.try_get("id")?,
            user_id: row.try_get("user_id")?,
            public_key: row.try_get("public_key")?,
            sign_count: u32::try_from(row.try_get::<i64, _>("sign_count")?).unwrap_or(u32::MAX),
        })
    })
    .transpose()
}

pub async fn record_use(pool: &SqlitePool, id: i64, sign_count: u32, backup_state: bool) -> Result<()> {
    sqlx::query(
        "UPDATE passkeys SET sign_count = ?, backup_state = ?,
             last_used_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ?",
    )
    .bind(i64::from(sign_count))
    .bind(i64::from(backup_state))
    .bind(id)
    .execute(pool)
    .await
    .context("usage d'une clé d'accès")?;
    Ok(())
}

pub async fn rename(pool: &SqlitePool, user_id: i64, id: i64, name: &str) -> Result<bool> {
    let result = sqlx::query("UPDATE passkeys SET name = ? WHERE id = ? AND user_id = ?")
        .bind(name)
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .context("renommage d'une clé d'accès")?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete(pool: &SqlitePool, user_id: i64, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM passkeys WHERE id = ? AND user_id = ?")
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await
        .context("suppression d'une clé d'accès")?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(origin: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::ORIGIN, origin.parse().unwrap());
        headers
    }

    #[test]
    fn le_domaine_vient_de_l_origine() {
        let rp = relying_party(&headers("https://Monit.Example.org"), None).unwrap();
        assert_eq!(rp.id, "monit.example.org");
        assert_eq!(rp.origin, "https://Monit.Example.org");
        let rp = relying_party(&headers("https://monit.example.org:8443"), None).unwrap();
        assert_eq!((rp.id.as_str(), rp.origin.as_str()), ("monit.example.org", "https://monit.example.org:8443"));
    }

    #[test]
    fn https_est_exige_sauf_localhost() {
        assert!(relying_party(&headers("http://monit.lan"), None).is_err());
        let local = relying_party(&headers("http://localhost:8080"), None).unwrap();
        assert_eq!(local.id, "localhost");
    }

    #[test]
    fn une_adresse_ip_est_refusee() {
        assert!(relying_party(&headers("https://192.168.1.10:8080"), None).is_err());
        assert!(relying_party(&headers("https://[fd00::1]"), None).is_err());
    }

    #[test]
    fn l_adresse_publique_configuree_fait_foi() {
        let configured = Some("https://monit.example.org");
        assert!(relying_party(&headers("https://monit.example.org"), configured).is_ok());
        assert!(relying_party(&headers("https://other.example.org"), configured).is_err());
        assert!(relying_party(&HeaderMap::new(), configured).is_err());
        // Une adresse publique avec un chemin garde son origine.
        let with_path = Some("https://monit.example.org/status/");
        assert!(relying_party(&headers("https://monit.example.org"), with_path).is_ok());
    }

    #[test]
    fn un_defi_ne_sert_qu_une_fois() {
        let rp = RelyingParty { id: "a.example".into(), origin: "https://a.example".into() };
        let (id, challenge) = start(Ceremony::Login, rp);
        let first = take(&id).expect("défi connu");
        assert_eq!(first.challenge, challenge);
        assert!(take(&id).is_none(), "a challenge cannot be replayed");
        assert!(take("inconnu").is_none());
    }
}
