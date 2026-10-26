//! Autorité interne du canal des agents : création, rotation, certificats.
//!
//! Tout est en ECDSA P-256 (clés générées par aws-lc-rs) : la courbe que tous
//! les vérificateurs connaissent, avec des signatures courtes. L'autorité vit
//! dix ans ; ce qu'elle signe vit trente jours, renouvelé bien avant.
//!
//! La clé privée de l'autorité est chiffrée par la clé de l'instance
//! (`crypto.rs`) et n'existe en clair qu'en mémoire, le temps de signer.

use anyhow::{Context, Result, bail};
use chrono::{Datelike, Days, Utc};
use dumbmonit_proto::{AGENT_TLS_SERVER_NAME, CLIENT_CERT_DAYS, TargetId};
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose, IsCa,
    Issuer, KeyPair, KeyUsagePurpose, PKCS_ECDSA_P256_SHA256, PublicKeyData, SanType, SerialNumber,
    SignatureAlgorithm, date_time_ymd,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};

use crate::crypto::Cipher;

/// Durée de vie d'une autorité, en jours.
const CA_DAYS: u64 = 3650;

/// Durée de vie du certificat du serveur : il est refait chaque jour et à
/// chaque démarrage, cette durée n'est qu'un filet.
const SERVER_CERT_DAYS: u64 = 30;

/// Une autorité retirée reste reconnue pour les certificats clients qu'elle a
/// signés tant qu'ils peuvent encore être valides.
const RETIRED_TRUST_MS: i64 = (CLIENT_CERT_DAYS + 2) * 24 * 3600 * 1000;

/// Temps pendant lequel un certificat remplacé par un renouvellement reste
/// accepté : celui des requêtes encore en vol sous l'ancien.
const REPLACED_GRACE_MS: i64 = 10 * 60 * 1000;

/// Empreinte SHA-256 (hexadécimal) d'un certificat DER.
pub fn fingerprint(der: &[u8]) -> String {
    hex::encode(Sha256::digest(der))
}

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

/// Une autorité telle que la base la garde (clé encore chiffrée).
#[derive(Clone)]
pub struct Authority {
    pub id: i64,
    pub name: String,
    pub cert_der: Vec<u8>,
    pub fingerprint: String,
    /// `active`, `next` (rotation en cours) ou `retired`.
    pub state: String,
    pub created_ms: i64,
    key_enc: Vec<u8>,
}

impl std::fmt::Debug for Authority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Authority")
            .field("id", &self.id)
            .field("fingerprint", &self.fingerprint)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl Authority {
    /// Clé de signature, déchiffrée pour la durée d'une signature.
    fn signing_key(&self, cipher: &Cipher) -> Result<KeyPair> {
        let der = cipher.decrypt(&self.key_enc).context("decrypting the agent authority key")?;
        KeyPair::from_pkcs8_der_and_sign_algo(
            &PrivatePkcs8KeyDer::from(der),
            &PKCS_ECDSA_P256_SHA256,
        )
        .context("reading the agent authority key")
    }

    /// Émetteur des certificats qu'elle signe : même sujet, même méthode
    /// d'identifiant de clé que son propre certificat.
    fn issuer(&self, cipher: &Cipher) -> Result<Issuer<'static, KeyPair>> {
        Ok(Issuer::new(authority_params(&self.name), self.signing_key(cipher)?))
    }
}

/// Paramètres d'une autorité. Rejoués à l'identique pour signer : le sujet
/// des certificats émis doit reproduire octet par octet le sien.
fn authority_params(name: &str) -> CertificateParams {
    let mut params = CertificateParams::default();
    params.distinguished_name = distinguished_name(name);
    // Longueur de chemin nulle : elle signe des certificats finaux, jamais une
    // autre autorité.
    params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    params
}

fn distinguished_name(common_name: &str) -> DistinguishedName {
    let mut name = DistinguishedName::new();
    name.push(DnType::OrganizationName, "DumbMonit");
    name.push(DnType::CommonName, common_name);
    name
}

/// Numéro de série aléatoire de 128 bits, positif et non nul.
fn serial() -> SerialNumber {
    let mut bytes: [u8; 16] = rand::random();
    bytes[0] &= 0x7f;
    bytes[0] |= 0x01;
    SerialNumber::from_slice(&bytes)
}

/// Fixe la validité, d'hier (dérive d'horloge) à `days` jours d'ici, et rend
/// l'échéance en millisecondes.
fn set_validity(params: &mut CertificateParams, days: u64) -> i64 {
    let today = Utc::now().date_naive();
    let from = today.checked_sub_days(Days::new(1)).unwrap_or(today);
    let until = today.checked_add_days(Days::new(days)).unwrap_or(today);
    params.not_before = date_time_ymd(from.year(), from.month() as u8, from.day() as u8);
    params.not_after = date_time_ymd(until.year(), until.month() as u8, until.day() as u8);
    until.and_hms_opt(0, 0, 0).map_or(0, |at| at.and_utc().timestamp_millis())
}

/// Clé publique d'un agent : point P-256 non compressé, tel qu'il l'envoie.
struct AgentPublicKey(Vec<u8>);

impl PublicKeyData for AgentPublicKey {
    fn der_bytes(&self) -> &[u8] {
        &self.0
    }

    fn algorithm(&self) -> &'static SignatureAlgorithm {
        &PKCS_ECDSA_P256_SHA256
    }
}

/// Vérifie qu'une clé publique reçue est bien un point P-256 non compressé
/// valide, avant d'y mettre la signature de l'autorité.
pub fn check_public_key(raw: &[u8]) -> Result<()> {
    if raw.len() != 65 || raw[0] != 0x04 {
        bail!("an uncompressed P-256 public key (65 bytes) is expected");
    }
    p256::PublicKey::from_sec1_bytes(raw)
        .map(|_| ())
        .map_err(|_| anyhow::anyhow!("the public key is not a point of P-256"))
}

// ------------------------------------------------------------------ autorités

/// Autorités encore utiles : active, suivante, et retirées depuis peu.
pub async fn authorities(pool: &SqlitePool) -> Result<Vec<Authority>> {
    let rows = sqlx::query(
        "SELECT id, name, cert_der, key_enc, fingerprint, state, created_ms
         FROM agent_ca
         WHERE state != 'retired' OR retired_ms > ?
         ORDER BY id",
    )
    .bind(now_ms() - RETIRED_TRUST_MS)
    .fetch_all(pool)
    .await
    .context("reading the agent authorities")?;
    rows.iter()
        .map(|row| {
            Ok(Authority {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                cert_der: row.try_get("cert_der")?,
                key_enc: row.try_get("key_enc")?,
                fingerprint: row.try_get("fingerprint")?,
                state: row.try_get("state")?,
                created_ms: row.try_get("created_ms")?,
            })
        })
        .collect()
}

/// L'autorité qui signe aujourd'hui.
pub async fn active(pool: &SqlitePool) -> Result<Option<Authority>> {
    Ok(authorities(pool).await?.into_iter().find(|ca| ca.state == "active"))
}

/// Crée une autorité dans l'état donné et la range, clé chiffrée.
async fn create(pool: &SqlitePool, cipher: &Cipher, state: &str) -> Result<Authority> {
    let key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)
        .context("generating the agent authority key")?;
    let suffix: [u8; 4] = rand::random();
    let name = format!("DumbMonit agent CA {}", hex::encode(suffix));
    let mut params = authority_params(&name);
    params.serial_number = Some(serial());
    set_validity(&mut params, CA_DAYS);
    let cert = params.self_signed(&key).context("signing the agent authority")?;
    let cert_der = cert.der().to_vec();
    let key_enc = cipher.encrypt(&key.serialize_der())?;
    let fingerprint = fingerprint(&cert_der);
    let created_ms = now_ms();
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO agent_ca (name, cert_der, key_enc, fingerprint, state, created_ms)
         VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(&name)
    .bind(&cert_der)
    .bind(&key_enc)
    .bind(&fingerprint)
    .bind(state)
    .bind(created_ms)
    .fetch_one(pool)
    .await
    .context("storing the agent authority")?;
    Ok(Authority { id, name, cert_der, fingerprint, state: state.to_string(), created_ms, key_enc })
}

/// Garantit qu'une autorité active existe ; rend vrai si elle vient d'être
/// créée (premier démarrage de cette version).
pub async fn ensure_active(pool: &SqlitePool, cipher: &Cipher) -> Result<bool> {
    if let Some(active) = active(pool).await? {
        // Une clé qui ne se déchiffre plus (secret d'instance changé) se dit
        // ici, au démarrage, plutôt qu'au premier agent refusé.
        active.signing_key(cipher)?;
        return Ok(false);
    }
    create(pool, cipher, "active").await?;
    Ok(true)
}

/// Début de rotation : une autorité suivante, que les agents épinglent avant
/// qu'elle ne signe quoi que ce soit.
pub async fn start_rotation(pool: &SqlitePool, cipher: &Cipher) -> Result<Authority> {
    if let Some(next) = authorities(pool).await?.into_iter().find(|ca| ca.state == "next") {
        return Ok(next);
    }
    create(pool, cipher, "next").await
}

/// Fin de rotation : la suivante devient active, l'ancienne est retirée (elle
/// reste reconnue pour les certificats clients qu'elle a signés).
pub async fn complete_rotation(pool: &SqlitePool) -> Result<bool> {
    let mut tx = pool.begin().await?;
    let next: Option<i64> = sqlx::query_scalar("SELECT id FROM agent_ca WHERE state = 'next'")
        .fetch_optional(&mut *tx)
        .await?;
    let Some(next) = next else { return Ok(false) };
    sqlx::query("UPDATE agent_ca SET state = 'retired', retired_ms = ? WHERE state = 'active'")
        .bind(now_ms())
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE agent_ca SET state = 'active' WHERE id = ?")
        .bind(next)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(true)
}

/// Abandon d'une rotation commencée.
pub async fn cancel_rotation(pool: &SqlitePool) -> Result<bool> {
    let done = sqlx::query("DELETE FROM agent_ca WHERE state = 'next'").execute(pool).await?;
    Ok(done.rows_affected() > 0)
}

// --------------------------------------------------------------- certificats

/// Certificat du serveur sur le canal des agents, et sa clé.
pub struct ServerIdentity {
    /// Le certificat, puis l'autorité : l'agent retrouve dans la chaîne
    /// l'autorité dont il a épinglé l'empreinte.
    pub chain: Vec<CertificateDer<'static>>,
    pub key: PrivateKeyDer<'static>,
}

/// Fabrique un certificat de serveur neuf (nouvelle clé) sous l'autorité active.
pub fn issue_server(authority: &Authority, cipher: &Cipher) -> Result<ServerIdentity> {
    let issuer = authority.issuer(cipher)?;
    let key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)
        .context("generating the agent channel key")?;
    let mut params = CertificateParams::default();
    params.distinguished_name = distinguished_name("DumbMonit agent channel");
    params.subject_alt_names = vec![SanType::DnsName(
        AGENT_TLS_SERVER_NAME.try_into().context("agent channel server name")?,
    )];
    params.is_ca = IsCa::ExplicitNoCa;
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    params.use_authority_key_identifier_extension = true;
    params.serial_number = Some(serial());
    set_validity(&mut params, SERVER_CERT_DAYS);
    let cert = params.signed_by(&key, &issuer).context("signing the agent channel certificate")?;
    Ok(ServerIdentity {
        chain: vec![cert.der().clone(), CertificateDer::from(authority.cert_der.clone())],
        key: PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der())),
    })
}

/// Certificat client délivré à un agent.
#[derive(Debug, Clone)]
pub struct IssuedClient {
    pub der: Vec<u8>,
    pub not_after_ms: i64,
}

/// Signe la clé publique d'un agent pour sa cible, et range le certificat.
/// Les certificats précédents de la machine sont marqués remplacés.
pub async fn issue_client(
    pool: &SqlitePool,
    cipher: &Cipher,
    target_id: TargetId,
    public_key: &[u8],
) -> Result<IssuedClient> {
    check_public_key(public_key)?;
    let authority = active(pool).await?.context("no active agent authority")?;
    let issuer = authority.issuer(cipher)?;
    let mut params = CertificateParams::default();
    params.distinguished_name = distinguished_name(&format!("dumbmonit-agent-{target_id}"));
    params.is_ca = IsCa::ExplicitNoCa;
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
    params.use_authority_key_identifier_extension = true;
    params.serial_number = Some(serial());
    let not_after_ms = set_validity(&mut params, CLIENT_CERT_DAYS as u64);
    let cert = params
        .signed_by(&AgentPublicKey(public_key.to_vec()), &issuer)
        .context("signing the agent certificate")?;
    let der = cert.der().to_vec();

    let now = now_ms();
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM agent_certs WHERE target_id = ? AND replaced_ms < ?")
        .bind(target_id)
        .bind(now - REPLACED_GRACE_MS)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE agent_certs SET replaced_ms = ? WHERE target_id = ? AND replaced_ms IS NULL",
    )
    .bind(now)
    .bind(target_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO agent_certs (fingerprint, target_id, ca_id, issued_ms, not_after_ms)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(fingerprint(&der))
    .bind(target_id)
    .bind(authority.id)
    .bind(now)
    .bind(not_after_ms)
    .execute(&mut *tx)
    .await
    .context("recording the agent certificate")?;
    tx.commit().await?;
    Ok(IssuedClient { der, not_after_ms })
}

/// Machine à laquelle appartient un certificat client présenté, s'il est
/// encore accepté : délivré ici, non échu, non remplacé depuis trop
/// longtemps, et sa machine toujours là (la suppression le révoque).
pub async fn certificate_owner(pool: &SqlitePool, fingerprint: &str) -> Result<Option<TargetId>> {
    let now = now_ms();
    let owner: Option<TargetId> = sqlx::query_scalar(
        "SELECT c.target_id FROM agent_certs c
         JOIN agent_hosts h ON h.target_id = c.target_id
         WHERE c.fingerprint = ? AND c.not_after_ms > ?
           AND (c.replaced_ms IS NULL OR c.replaced_ms > ?)",
    )
    .bind(fingerprint)
    .bind(now)
    .bind(now - REPLACED_GRACE_MS)
    .fetch_optional(pool)
    .await
    .context("checking the agent certificate")?;
    Ok(owner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_public_key_must_be_a_real_p256_point() {
        let key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap();
        assert!(check_public_key(key.public_key_raw()).is_ok());
        assert!(check_public_key(&[4u8; 65]).is_err(), "pas un point de la courbe");
        assert!(check_public_key(&key.public_key_raw()[..33]).is_err());
    }

    #[test]
    fn a_serial_number_is_positive_and_never_zero() {
        for _ in 0..64 {
            let bytes = serial().to_bytes();
            assert_eq!(bytes.len(), 16);
            assert_eq!(bytes[0] & 0x80, 0);
            assert_ne!(bytes[0], 0);
        }
    }
}
