//! Identification du serveur auprès des services de push (VAPID, RFC 8292).
//!
//! Chaque instance a sa paire de clés P-256, générée au premier démarrage. La
//! clé publique est remise aux navigateurs au moment où ils s'abonnent : le
//! service de push n'acceptera ensuite, pour cet abonnement, que des messages
//! signés par la clé privée correspondante. La clé privée ne quitte jamais le
//! serveur : chiffrée en base, déchiffrée en mémoire pour signer, et absente de
//! toute réponse d'API.

use anyhow::{Context, Result, anyhow};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{Signature, SigningKey};
use reqwest::Url;
use serde_json::json;
use sqlx::{Row, SqlitePool};

use crate::crypto::Cipher;

/// Durée de validité d'une signature. La RFC 8292 plafonne à 24 heures ; douze
/// laissent de la marge aux horloges mal réglées sans rien coûter, une
/// signature étant refaite à chaque envoi.
pub const SIGNATURE_LIFETIME_SECS: i64 = 12 * 3600;

/// Paire de clés VAPID de l'instance.
pub struct VapidKeys {
    signing: SigningKey,
    public_b64: String,
}

impl std::fmt::Debug for VapidKeys {
    // La clé privée n'apparaît jamais, pas même dans une trace de débogage.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VapidKeys").field("public", &self.public_b64).finish_non_exhaustive()
    }
}

impl VapidKeys {
    /// Nouvelle paire tirée de l'aléa du système.
    pub fn generate() -> Self {
        loop {
            // Un scalaire nul ou supérieur à l'ordre de la courbe est refusé :
            // la probabilité est infime (≈ 2⁻³²), mais la boucle la couvre.
            let bytes: [u8; 32] = rand::random();
            if let Ok(keys) = Self::from_secret_bytes(&bytes) {
                return keys;
            }
        }
    }

    /// Reconstruit la paire depuis le scalaire privé (32 octets).
    pub fn from_secret_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            return Err(anyhow!("VAPID private key must be 32 bytes, got {}", bytes.len()));
        }
        let signing = SigningKey::from_slice(bytes).map_err(|_| anyhow!("invalid VAPID key"))?;
        let point = signing.verifying_key().to_encoded_point(false);
        let public_b64 = URL_SAFE_NO_PAD.encode(point.as_bytes());
        Ok(Self { signing, public_b64 })
    }

    /// Scalaire privé, pour le chiffrer avant de l'écrire en base.
    fn secret_bytes(&self) -> Vec<u8> {
        self.signing.to_bytes().to_vec()
    }

    /// Clé publique en base64url (point non compressé, 65 octets).
    pub fn public_key(&self) -> &str {
        &self.public_b64
    }

    /// En-tête `Authorization` d'un envoi vers `endpoint` :
    /// `vapid t=<JWT ES256>, k=<clé publique>`.
    ///
    /// L'audience est l'origine du service de push (schéma, hôte, port), comme
    /// l'exige la RFC 8292 ; `subject` est le contact de l'opérateur (`mailto:` ou
    /// `https:`), que les services utilisent pour joindre l'expéditeur d'un abus.
    pub fn authorization(&self, endpoint: &Url, subject: &str, now: i64) -> String {
        let audience = endpoint.origin().ascii_serialization();
        let header = URL_SAFE_NO_PAD.encode(br#"{"typ":"JWT","alg":"ES256"}"#);
        let claims =
            json!({ "aud": audience, "exp": now + SIGNATURE_LIFETIME_SECS, "sub": subject });
        let claims = URL_SAFE_NO_PAD.encode(claims.to_string());
        let signing_input = format!("{header}.{claims}");
        // ES256 dans un JWT : r ‖ s sur 64 octets (JWS, RFC 7515), et non le DER.
        let signature: Signature = self.signing.sign(signing_input.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(signature.to_bytes());
        format!("vapid t={signing_input}.{signature}, k={}", self.public_b64)
    }
}

/// Lit la paire de clés de l'instance, et la crée si elle n'existe pas encore.
///
/// Appelée au démarrage, puis à chaque besoin : `INSERT OR IGNORE` suivi d'une
/// relecture fait que deux appels simultanés finissent sur la même paire.
pub async fn load_or_create(pool: &SqlitePool, cipher: &Cipher) -> Result<VapidKeys> {
    if let Some(keys) = load(pool, cipher).await? {
        return Ok(keys);
    }
    let keys = VapidKeys::generate();
    let encrypted = cipher.encrypt(&keys.secret_bytes())?;
    sqlx::query(
        "INSERT OR IGNORE INTO webpush_vapid (id, public_key, private_key_enc) VALUES (1, ?, ?)",
    )
    .bind(keys.public_key())
    .bind(&encrypted)
    .execute(pool)
    .await
    .context("storing the VAPID key pair")?;
    load(pool, cipher).await?.ok_or_else(|| anyhow!("VAPID key pair missing after creation"))
}

async fn load(pool: &SqlitePool, cipher: &Cipher) -> Result<Option<VapidKeys>> {
    let Some(row) = sqlx::query("SELECT private_key_enc FROM webpush_vapid WHERE id = 1")
        .fetch_optional(pool)
        .await
        .context("reading the VAPID key pair")?
    else {
        return Ok(None);
    };
    let encrypted: Vec<u8> = row.try_get("private_key_enc")?;
    let secret = cipher.decrypt(&encrypted).context("decrypting the VAPID private key")?;
    VapidKeys::from_secret_bytes(&secret).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::VerifyingKey;
    use p256::ecdsa::signature::Verifier;

    #[test]
    fn la_cle_publique_a_la_forme_attendue_par_les_navigateurs() {
        let keys = VapidKeys::generate();
        let raw = URL_SAFE_NO_PAD.decode(keys.public_key()).expect("base64url");
        assert_eq!(raw.len(), 65, "uncompressed P-256 point");
        assert_eq!(raw[0], 0x04);
    }

    #[test]
    fn la_paire_se_reconstruit_a_l_identique() {
        let keys = VapidKeys::generate();
        let again = VapidKeys::from_secret_bytes(&keys.secret_bytes()).expect("same key");
        assert_eq!(keys.public_key(), again.public_key());
        assert!(VapidKeys::from_secret_bytes(&[0u8; 32]).is_err(), "zero is not a key");
        assert!(VapidKeys::from_secret_bytes(&[1u8; 16]).is_err(), "wrong length");
    }

    #[test]
    fn la_trace_de_debogage_ne_montre_pas_la_cle_privee() {
        let keys = VapidKeys::generate();
        let secret = hex::encode(keys.secret_bytes());
        let debug = format!("{keys:?}");
        assert!(!debug.contains(&secret));
        assert!(debug.contains(keys.public_key()));
    }

    #[test]
    fn la_signature_est_un_jwt_es256_verifiable() {
        let keys = VapidKeys::generate();
        let endpoint: Url = "https://fcm.googleapis.com:443/fcm/send/abc".parse().unwrap();
        let header = keys.authorization(&endpoint, "mailto:ops@example.com", 1_000);

        let rest = header.strip_prefix("vapid t=").expect("vapid scheme");
        let (token, key) = rest.split_once(", k=").expect("token and key");
        assert_eq!(key, keys.public_key());

        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
        let claims: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
        assert_eq!(claims["aud"], "https://fcm.googleapis.com", "origin, default port dropped");
        assert_eq!(claims["exp"], 1_000 + SIGNATURE_LIFETIME_SECS);
        assert_eq!(claims["sub"], "mailto:ops@example.com");

        let public = URL_SAFE_NO_PAD.decode(key).unwrap();
        let verifying = VerifyingKey::from_sec1_bytes(&public).unwrap();
        let signature = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
        assert_eq!(signature.len(), 64, "raw r||s, not DER");
        let signature = Signature::from_slice(&signature).unwrap();
        let input = format!("{}.{}", parts[0], parts[1]);
        verifying.verify(input.as_bytes(), &signature).expect("valid signature");
    }

    #[tokio::test]
    async fn la_paire_est_creee_une_fois_chiffree_puis_relue() {
        let dir = tempfile::tempdir().unwrap();
        let pool = crate::db::open(&dir.path().join("db.sqlite")).await.unwrap();
        let cipher =
            crate::db::init_cipher(&pool, "a-test-secret-that-is-long-enough").await.unwrap();

        let first = load_or_create(&pool, &cipher).await.unwrap();
        let second = load_or_create(&pool, &cipher).await.unwrap();
        assert_eq!(first.public_key(), second.public_key(), "generated once");

        let stored: Vec<u8> =
            sqlx::query_scalar("SELECT private_key_enc FROM webpush_vapid WHERE id = 1")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_ne!(stored, first.secret_bytes(), "never stored in clear");
        assert!(!stored.windows(32).any(|w| w == first.secret_bytes().as_slice()));

        // Un autre secret d'instance ne lit pas la clé : le canari l'aurait refusé
        // au démarrage, et la lecture échoue ici plutôt que de signer avec n'importe quoi.
        let other = Cipher::derive("another-secret-long-enough", b"saltsalt").unwrap();
        assert!(load_or_create(&pool, &other).await.is_err());
    }
}
