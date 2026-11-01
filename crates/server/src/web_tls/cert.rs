//! Certificat présenté par l'interface : fourni en fichiers, ou autosigné.
//!
//! - **Fourni** (`DUMBMONIT_TLS_CERT` / `DUMBMONIT_TLS_KEY`) : une chaîne PEM
//!   et sa clé (PKCS#8, PKCS#1 ou SEC1), relues dès qu'un des deux fichiers
//!   change — un renouvellement Let's Encrypt ou d'une autorité interne se
//!   prend sans redémarrer.
//! - **Autosigné** : créé au premier démarrage, ECDSA P-256, 397 jours (la
//!   durée maximale qu'acceptent tous les navigateurs, macOS et iOS
//!   compris), renouvelé trente jours avant l'échéance **avec la même clé** :
//!   l'empreinte de la clé publique reste stable d'un renouvellement à
//!   l'autre. La clé privée est rangée chiffrée par la clé de l'instance
//!   (`crypto.rs`), comme celle de l'autorité des agents.

use std::net::IpAddr;
use std::path::Path;

use anyhow::{Context, Result, bail};
use base64::Engine as _;
use rcgen::{
    CertificateParams, ExtendedKeyUsagePurpose, IsCa, KeyPair, KeyUsagePurpose,
    PKCS_ECDSA_P256_SHA256, SanType,
};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::agent_tls::pki;

/// Durée de vie du certificat autosigné, en jours.
pub const SELF_SIGNED_DAYS: u64 = 397;

/// Renouvellement du certificat autosigné quand il lui reste moins que ça.
pub const RENEW_BEFORE_MS: i64 = 30 * 24 * 3600 * 1000;

/// Certificat autosigné tel que la base le garde (la clé à part, chiffrée).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredCert {
    /// Certificat DER, en hexadécimal.
    pub der_hex: String,
    /// Noms et adresses du certificat.
    pub names: Vec<String>,
    pub not_after_ms: i64,
}

/// Certificat autosigné tout juste fabriqué.
pub struct SelfSigned {
    pub cert_der: Vec<u8>,
    /// Clé PKCS#8 DER : à chiffrer avant de la ranger.
    pub key_der: Vec<u8>,
    pub stored: StoredCert,
}

/// Identité prête pour rustls.
pub struct Identity {
    pub chain: Vec<CertificateDer<'static>>,
    pub key: PrivateKeyDer<'static>,
}

/// Vrai pour un nom DNS (lettres, chiffres, tirets, points, joker en tête) ou
/// une adresse IP.
pub fn is_valid_name(name: &str) -> bool {
    if name.parse::<IpAddr>().is_ok() {
        return true;
    }
    let host = name.strip_prefix("*.").unwrap_or(name);
    !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

/// Nom d'hôte par défaut d'un conteneur Docker : douze caractères
/// hexadécimaux, les premiers de son identifiant.
pub fn is_container_id(name: &str) -> bool {
    name.len() == 12 && name.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Noms du certificat autosigné : la boucle locale, le nom de la machine,
/// l'hôte de l'URL publique et ceux de `DUMBMONIT_TLS_NAMES`.
pub fn default_names(
    hostname: Option<&str>,
    public_url: Option<&str>,
    extra: &[String],
) -> Vec<String> {
    let mut names: Vec<String> = vec!["localhost".into(), "127.0.0.1".into(), "::1".into()];
    let public_host = public_url
        .and_then(|url| reqwest::Url::parse(url.trim()).ok())
        .and_then(|url| url.host_str().map(|host| host.trim_matches(['[', ']']).to_string()));
    let candidates =
        hostname.map(str::to_string).into_iter().chain(public_host).chain(extra.iter().cloned());
    for name in candidates {
        let name = name.trim().to_ascii_lowercase();
        if is_valid_name(&name) && !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

fn san(name: &str) -> Result<SanType> {
    Ok(match name.parse::<IpAddr>() {
        Ok(ip) => SanType::IpAddress(ip),
        Err(_) => {
            SanType::DnsName(name.try_into().with_context(|| format!("certificate name {name}"))?)
        }
    })
}

/// Fabrique (ou refait, avec la clé donnée) le certificat autosigné.
pub fn self_signed(key_der: Option<&[u8]>, names: &[String]) -> Result<SelfSigned> {
    let key = match key_der {
        Some(der) => KeyPair::from_pkcs8_der_and_sign_algo(
            &PrivatePkcs8KeyDer::from(der.to_vec()),
            &PKCS_ECDSA_P256_SHA256,
        )
        .context("reading the HTTPS key")?,
        None => {
            KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).context("generating the HTTPS key")?
        }
    };
    let mut params = CertificateParams::default();
    params.distinguished_name = pki::distinguished_name("DumbMonit (self-signed)");
    params.subject_alt_names = names.iter().map(|name| san(name)).collect::<Result<_>>()?;
    params.is_ca = IsCa::ExplicitNoCa;
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    params.serial_number = Some(pki::serial());
    let not_after_ms = pki::set_validity(&mut params, SELF_SIGNED_DAYS);
    let cert = params.self_signed(&key).context("signing the HTTPS certificate")?;
    let cert_der = cert.der().to_vec();
    Ok(SelfSigned {
        stored: StoredCert { der_hex: hex::encode(&cert_der), names: names.to_vec(), not_after_ms },
        cert_der,
        key_der: key.serialize_der(),
    })
}

impl StoredCert {
    /// Faut-il le refaire : échéance proche, ou noms changés ?
    pub fn needs_renewal(&self, names: &[String], now_ms: i64) -> bool {
        self.not_after_ms - now_ms < RENEW_BEFORE_MS || self.names != names
    }

    pub fn der(&self) -> Result<Vec<u8>> {
        hex::decode(&self.der_hex).context("stored HTTPS certificate")
    }
}

/// Identité rustls d'un certificat autosigné rangé.
pub fn identity_from_parts(cert_der: Vec<u8>, key_der: Vec<u8>) -> Identity {
    Identity {
        chain: vec![CertificateDer::from(cert_der)],
        key: PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_der)),
    }
}

/// Lit un certificat fourni : chaîne PEM (certificat du serveur d'abord) et
/// clé privée PEM.
pub fn read_provided(cert_path: &Path, key_path: &Path) -> Result<Identity> {
    let chain = CertificateDer::pem_file_iter(cert_path)
        .with_context(|| format!("reading {}", cert_path.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("reading the certificates of {}", cert_path.display()))?;
    if chain.is_empty() {
        bail!("{} holds no PEM certificate", cert_path.display());
    }
    let key = PrivateKeyDer::from_pem_file(key_path)
        .with_context(|| format!("reading the private key in {}", key_path.display()))?;
    Ok(Identity { chain, key })
}

/// Empreinte SHA-256 d'un certificat, telle que la montrent les navigateurs :
/// hexadécimal majuscule, octets séparés par des deux-points.
pub fn display_fingerprint(der: &[u8]) -> String {
    Sha256::digest(der).iter().map(|byte| format!("{byte:02X}")).collect::<Vec<_>>().join(":")
}

/// Épingle de la clé publique (`sha256//<base64>`), le format de
/// `curl --pinnedpubkey` : stable d'un renouvellement à l'autre du
/// certificat autosigné, qui garde sa clé.
pub fn spki_pin(key_der: &[u8]) -> Result<String> {
    let key = KeyPair::from_pkcs8_der_and_sign_algo(
        &PrivatePkcs8KeyDer::from(key_der.to_vec()),
        &PKCS_ECDSA_P256_SHA256,
    )
    .context("reading the HTTPS key")?;
    // SubjectPublicKeyInfo d'une clé P-256 : en-tête fixe (id-ecPublicKey,
    // prime256v1), puis le point non compressé de 65 octets.
    const P256_SPKI_PREFIX: [u8; 26] = [
        0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08,
        0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
    ];
    let point = rcgen::PublicKeyData::der_bytes(&key);
    if point.len() != 65 || point[0] != 0x04 {
        bail!("unexpected HTTPS public key");
    }
    let mut spki = P256_SPKI_PREFIX.to_vec();
    spki.extend_from_slice(point);
    Ok(format!(
        "sha256//{}",
        base64::engine::general_purpose::STANDARD.encode(Sha256::digest(spki))
    ))
}

/// Certificat en PEM, pour le téléchargement depuis les réglages.
pub fn to_pem(der: &[u8]) -> String {
    let body = base64::engine::general_purpose::STANDARD.encode(der);
    let mut pem = String::from("-----BEGIN CERTIFICATE-----\n");
    for line in body.as_bytes().chunks(64) {
        pem.push_str(&String::from_utf8_lossy(line));
        pem.push('\n');
    }
    pem.push_str("-----END CERTIFICATE-----\n");
    pem
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_host_names_or_addresses() {
        assert!(is_valid_name("nas.lan"));
        assert!(is_valid_name("*.home.arpa"));
        assert!(is_valid_name("192.168.1.10"));
        assert!(is_valid_name("fe80::1"));
        assert!(!is_valid_name(""));
        assert!(!is_valid_name("nas lan"));
        assert!(!is_valid_name("-nas.lan"));
        assert!(!is_valid_name("nas..lan"));
    }

    #[test]
    fn a_default_container_hostname_is_recognised() {
        assert!(is_container_id("3f2a9b1c7d8e"));
        assert!(!is_container_id("nas"));
        assert!(!is_container_id("3f2a9b1c7d8e1"));
    }

    #[test]
    fn the_self_signed_certificate_covers_the_loopback_the_host_and_the_public_url() {
        let names = default_names(
            Some("NAS"),
            Some("https://monit.example.org:8443/"),
            &["192.168.1.10".into(), "nas".into()],
        );
        let expected =
            ["localhost", "127.0.0.1", "::1", "nas", "monit.example.org", "192.168.1.10"];
        assert_eq!(names, expected);
    }

    #[test]
    fn a_renewal_keeps_the_key_and_the_public_key_pin() {
        let names = default_names(None, None, &[]);
        let first = self_signed(None, &names).unwrap();
        let second = self_signed(Some(&first.key_der), &names).unwrap();
        assert_ne!(first.cert_der, second.cert_der, "nouveau certificat");
        assert_eq!(first.key_der, second.key_der, "même clé");
        assert_eq!(spki_pin(&first.key_der).unwrap(), spki_pin(&second.key_der).unwrap());
        assert!(spki_pin(&first.key_der).unwrap().starts_with("sha256//"));
        // Une identité rustls se construit à partir de ce qui est rangé.
        let identity = identity_from_parts(second.cert_der.clone(), second.key_der.clone());
        assert_eq!(identity.chain.len(), 1);
    }

    const DAY: i64 = 86_400_000;

    #[test]
    fn the_certificate_is_renewed_a_month_before_it_expires_or_when_its_names_change() {
        let names = vec!["localhost".to_string()];
        let stored =
            StoredCert { der_hex: String::new(), names: names.clone(), not_after_ms: 100 * DAY };
        assert!(!stored.needs_renewal(&names, 0));
        assert!(stored.needs_renewal(&names, 71 * DAY));
        assert!(stored.needs_renewal(&["nas.lan".to_string()], 0));
    }

    #[test]
    fn the_fingerprint_reads_like_a_browser_shows_it() {
        let shown = display_fingerprint(b"abc");
        assert_eq!(shown.len(), 32 * 3 - 1);
        assert!(shown.starts_with("BA:78:16:BF"));
        let pem = to_pem(&[0u8; 100]);
        assert!(pem.starts_with("-----BEGIN CERTIFICATE-----\n"));
        assert!(pem.trim_end().ends_with("-----END CERTIFICATE-----"));
    }

    #[test]
    fn a_provided_certificate_is_read_from_pem_files() {
        let dir = tempfile::tempdir().unwrap();
        let made = self_signed(None, &["nas.lan".to_string()]).unwrap();
        let cert = dir.path().join("cert.pem");
        let key = dir.path().join("key.pem");
        std::fs::write(&cert, to_pem(&made.cert_der)).unwrap();
        let body = base64::engine::general_purpose::STANDARD.encode(&made.key_der);
        let pem = format!("-----BEGIN PRIVATE KEY-----\n{body}\n-----END PRIVATE KEY-----\n");
        std::fs::write(&key, pem).unwrap();
        let identity = read_provided(&cert, &key).unwrap();
        assert_eq!(identity.chain[0].as_ref(), made.cert_der.as_slice());
        std::fs::write(&cert, "pas un certificat").unwrap();
        assert!(read_provided(&cert, &key).is_err());
    }
}
