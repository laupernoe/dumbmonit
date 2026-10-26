//! Provisionnement SCIM 2.0 : réglages et jeton du fournisseur d'identité.
//!
//! Un fournisseur (Entra ID, Okta…) crée, met à jour, désactive et supprime des
//! comptes et des équipes par `/api/scim/v2` (voir `api/scim.rs`). Il présente
//! un jeton porteur dédié, `Authorization: Bearer dmscim_…`, distinct des jetons
//! d'API (`dmt_`) : aucun des deux n'ouvre la porte de l'autre.
//!
//! Le jeton est un aléa de 256 bits, montré une seule fois à l'administrateur.
//! Seule son empreinte SHA-256 est gardée, dans `settings`, sous une clé à part
//! des réglages : enregistrer les réglages ne peut donc jamais ressusciter un
//! jeton révoqué entre-temps. La comparaison se fait en temps constant.
//!
//! Ce que SCIM ne fait jamais, quels que soient le jeton et la requête : créer,
//! modifier, désactiver ou supprimer un administrateur, poser un mot de passe,
//! ou toucher aux droits des équipes sur les dossiers.

pub mod filter;

use anyhow::Result;
use axum::http::{HeaderMap, header};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use subtle::ConstantTimeEq;

use crate::auth::users::Role;

/// Préfixe du jeton SCIM : reconnaissable dans une configuration et par un
/// scanner de secrets, et jamais pris pour un jeton d'API.
pub const TOKEN_PREFIX: &str = "dmscim_";

/// Octets d'aléa du jeton : 256 bits.
const TOKEN_BYTES: usize = 32;

/// Réglages (activation, rôle des comptes créés).
pub const SETTINGS_KEY: &str = "scim.config";
/// Empreinte du jeton et sa date de création.
pub const TOKEN_KEY: &str = "scim.token";

/// Rôle donné aux comptes que SCIM crée. Jamais administrateur : le type ne
/// le permet pas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DefaultRole {
    #[default]
    Viewer,
    Operator,
}

impl DefaultRole {
    pub fn role(self) -> Role {
        match self {
            Self::Viewer => Role::Viewer,
            Self::Operator => Role::Operator,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Operator => "operator",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "viewer" => Some(Self::Viewer),
            "operator" => Some(Self::Operator),
            _ => None,
        }
    }
}

/// Réglages SCIM, stockés en clair sous [`SETTINGS_KEY`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScimConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub default_role: DefaultRole,
}

/// Le jeton tel qu'il est gardé : son empreinte, jamais lui.
#[derive(Clone, Serialize, Deserialize)]
pub struct StoredToken {
    /// SHA-256 du jeton complet, en hexadécimal.
    pub hash: String,
    /// RFC 3339 UTC.
    pub created_at: String,
}

impl std::fmt::Debug for StoredToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StoredToken {{ hash: <redacted>, created_at: {:?} }}", self.created_at)
    }
}

pub async fn config(pool: &SqlitePool) -> Result<ScimConfig> {
    Ok(crate::auth::settings::get(pool, SETTINGS_KEY).await?.unwrap_or_default())
}

pub async fn save_config(pool: &SqlitePool, config: &ScimConfig) -> Result<()> {
    crate::auth::settings::set(pool, SETTINGS_KEY, config).await
}

pub async fn token(pool: &SqlitePool) -> Result<Option<StoredToken>> {
    crate::auth::settings::get(pool, TOKEN_KEY).await
}

/// Fabrique un nouveau jeton, remplace l'ancien (qui cesse aussitôt de
/// valoir) et rend le jeton en clair — la seule et unique fois.
pub async fn rotate_token(pool: &SqlitePool) -> Result<(StoredToken, String)> {
    let clear = generate();
    let stored = StoredToken {
        hash: fingerprint(&clear),
        created_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    };
    crate::auth::settings::set(pool, TOKEN_KEY, &stored).await?;
    Ok((stored, clear))
}

/// Oublie le jeton : plus aucune requête SCIM n'est acceptée.
pub async fn revoke_token(pool: &SqlitePool) -> Result<()> {
    crate::auth::settings::delete(pool, TOKEN_KEY).await
}

pub fn generate() -> String {
    let bytes: [u8; TOKEN_BYTES] = rand::random();
    format!("{TOKEN_PREFIX}{}", hex::encode(bytes))
}

/// Empreinte gardée : SHA-256 du jeton complet. SHA-256 et non Argon2 : le
/// jeton est un aléa de 256 bits, pas un mot de passe humain.
pub fn fingerprint(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Le jeton SCIM d'un en-tête `Authorization`. Strict sur le schéma (`Bearer`)
/// et sur le préfixe : un jeton d'API ou un mot de passe `Basic` n'est pas lu.
pub fn extract_bearer(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?.trim();
    let (scheme, token) = value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = token.trim();
    token.starts_with(TOKEN_PREFIX).then_some(token)
}

/// Vrai si `presented` est le jeton dont `stored` est l'empreinte. Les deux
/// empreintes sont comparées en temps constant.
pub fn token_matches(stored: &StoredToken, presented: &str) -> bool {
    let Ok(expected) = hex::decode(stored.hash.trim()) else { return false };
    let actual = Sha256::digest(presented.as_bytes());
    expected.len() == actual.len() && bool::from(expected.as_slice().ct_eq(actual.as_slice()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn a_generated_token_is_long_recognisable_and_unique() {
        let a = generate();
        let b = generate();
        assert!(a.starts_with(TOKEN_PREFIX));
        assert_eq!(a.len(), TOKEN_PREFIX.len() + 2 * TOKEN_BYTES);
        assert_ne!(a, b);
    }

    #[test]
    fn only_the_right_token_matches_its_fingerprint() {
        let clear = generate();
        let stored = StoredToken { hash: fingerprint(&clear), created_at: String::new() };
        assert!(token_matches(&stored, &clear));
        assert!(!token_matches(&stored, &generate()));
        assert!(!token_matches(&stored, ""));
        let broken = StoredToken { hash: "zz".into(), created_at: String::new() };
        assert!(!token_matches(&broken, &clear));
        assert!(!fingerprint(&clear).contains(&clear[TOKEN_PREFIX.len()..]));
    }

    #[test]
    fn only_a_bearer_with_the_scim_prefix_is_read() {
        let mut headers = HeaderMap::new();
        let mut with = |value: &str| {
            headers.insert(header::AUTHORIZATION, HeaderValue::from_str(value).unwrap());
            extract_bearer(&headers).map(str::to_string)
        };
        assert_eq!(with("Bearer dmscim_abc").as_deref(), Some("dmscim_abc"));
        assert_eq!(with("bearer  dmscim_abc ").as_deref(), Some("dmscim_abc"));
        assert_eq!(with("Bearer dmt_abc"), None);
        assert_eq!(with("Basic dmscim_abc"), None);
        assert_eq!(with("dmscim_abc"), None);
    }

    #[test]
    fn the_default_role_is_never_admin() {
        assert_eq!(DefaultRole::parse("viewer"), Some(DefaultRole::Viewer));
        assert_eq!(DefaultRole::parse("operator"), Some(DefaultRole::Operator));
        assert_eq!(DefaultRole::parse("admin"), None);
        assert_eq!(DefaultRole::default().role(), Role::Viewer);
        assert!(serde_json::from_str::<DefaultRole>("\"admin\"").is_err());
    }
}
