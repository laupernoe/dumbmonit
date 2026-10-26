//! Canal chiffré vers le serveur : TLS 1.3, certificat client, autorité épinglée.
//!
//! - La paire de clés de l'agent est générée ici (ECDSA P-256) ; la clé privée
//!   ne quitte jamais la machine. Le serveur signe la clé publique contre le
//!   jeton d'enrôlement, une fois, puis sous le certificat en cours à chaque
//!   renouvellement (une nouvelle clé à chaque fois).
//! - Le serveur n'est reconnu qu'à l'empreinte SHA-256 de son autorité : celle
//!   de la commande d'installation, ou, pour un agent déjà installé qui migre
//!   depuis l'ancien canal, celle que le serveur annonce (confiance au premier
//!   usage). Un certificat public valide présenté par un intermédiaire ne
//!   correspond à aucune empreinte : la connexion est refusée.
//! - Échange de clés hybride post-quantique **X25519MLKEM768** seul, TLS 1.3
//!   seul : pas de négociation vers le bas possible.
//!
//! Tout est rangé dans `agent-tls.json`, à côté du secret de liaison, lisible
//! par le seul propriétaire (0600 sous Unix ; sous Windows, le dossier de
//! configuration est réservé à SYSTEM et aux administrateurs).

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use anyhow::{Context, Result};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use dumbmonit_proto::{AGENT_TLS_SERVER_NAME, RENEW_BEFORE_DAYS};
use rustls::client::WebPkiServerVerifier;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls12_signature, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::{
    CertificateError, ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::warn;

/// Nom du fichier d'état, à côté du secret de liaison.
pub const STATE_FILE: &str = "agent-tls.json";

/// Empreintes épinglées, partagées entre le vérificateur et le client : une
/// mise à jour (rotation de l'autorité) vaut pour la poignée de main suivante.
pub type Pins = Arc<RwLock<Vec<String>>>;

/// Ce que l'agent garde du canal chiffré.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TlsState {
    /// `https://hôte:port` du canal des agents.
    pub url: String,
    /// Empreintes SHA-256 des autorités reconnues.
    pub pins: Vec<String>,
    /// Certificat client (DER, base64).
    #[serde(default)]
    pub certificate: Option<String>,
    /// Clé privée PKCS#8 (base64).
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub not_after_ms: Option<i64>,
    /// Vrai dès le premier enrôlement réussi : l'agent ne reparlera plus
    /// jamais en clair, même s'il doit se réenrôler.
    #[serde(default)]
    pub locked: bool,
}

/// La clé privée n'apparaît jamais dans une trace.
impl std::fmt::Debug for TlsState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TlsState")
            .field("url", &self.url)
            .field("pins", &self.pins)
            .field("enrolled", &self.certificate.is_some())
            .field("not_after_ms", &self.not_after_ms)
            .field("locked", &self.locked)
            .finish_non_exhaustive()
    }
}

impl TlsState {
    pub fn new(url: String, pins: Vec<String>) -> Self {
        Self { url, pins, ..Self::default() }
    }

    /// Relit l'état ; absent ou illisible, `None` (un état illisible est
    /// signalé : l'agent se réenrôlera).
    pub fn load(path: &Path) -> Option<Self> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
            Err(error) => {
                warn!(path = %path.display(), %error, "encrypted channel state unreadable");
                return None;
            }
        };
        match serde_json::from_str::<Self>(&text) {
            Ok(state) => Some(state),
            Err(error) => {
                warn!(path = %path.display(), %error, "encrypted channel state unreadable");
                None
            }
        }
    }

    /// Range l'état, lisible par le seul propriétaire.
    pub fn save(&self, path: &Path) -> Result<()> {
        let text = serde_json::to_string_pretty(self).context("encoding the channel state")?;
        crate::binding::write_private(path, &text)
    }

    /// Certificat et clé, décodés, si l'agent en a.
    pub fn identity(&self) -> Option<(Vec<u8>, Vec<u8>)> {
        let certificate = BASE64.decode(self.certificate.as_deref()?).ok()?;
        let key = BASE64.decode(self.key.as_deref()?).ok()?;
        Some((certificate, key))
    }

    /// Certificat présent et pas encore échu.
    pub fn usable(&self, now_ms: i64) -> bool {
        self.identity().is_some() && self.not_after_ms.is_some_and(|until| until > now_ms)
    }

    /// Moins de [`RENEW_BEFORE_DAYS`] jours avant l'échéance.
    pub fn needs_renewal(&self, now_ms: i64) -> bool {
        let margin = RENEW_BEFORE_DAYS * 24 * 3600 * 1000;
        self.not_after_ms.is_some_and(|until| until - now_ms < margin)
    }

    /// Oublie le certificat (refusé par le serveur) : l'agent se réenrôlera.
    pub fn drop_certificate(&mut self) {
        self.certificate = None;
        self.key = None;
        self.not_after_ms = None;
    }
}

/// Emplacement de l'état pour un secret de liaison donné : juste à côté.
pub fn beside(secret_path: &Path) -> PathBuf {
    secret_path.with_file_name(STATE_FILE)
}

/// Nouvelle paire de clés : la clé publique (point non compressé, base64) à
/// envoyer, la clé privée PKCS#8 à garder.
pub fn new_key() -> Result<(String, Vec<u8>)> {
    let key = rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256)
        .context("generating the agent key")?;
    Ok((BASE64.encode(key.public_key_raw()), key.serialize_der()))
}

/// Empreinte SHA-256 (hexadécimal) d'un certificat DER.
pub fn fingerprint(der: &[u8]) -> String {
    hex::encode(Sha256::digest(der))
}

/// Fournisseur cryptographique de l'agent : aws-lc-rs, échange de clés
/// hybride post-quantique seul.
pub fn provider() -> Arc<CryptoProvider> {
    use rustls::crypto::aws_lc_rs::{self, kx_group};
    Arc::new(CryptoProvider {
        kx_groups: vec![kx_group::X25519MLKEM768],
        ..aws_lc_rs::default_provider()
    })
}

/// Configuration TLS du canal : TLS 1.3, autorité épinglée, certificat client
/// s'il y en a un.
pub fn client_config(pins: Pins, identity: Option<(Vec<u8>, Vec<u8>)>) -> Result<ClientConfig> {
    let provider = provider();
    let verifier = Arc::new(PinnedVerifier { pins, provider: provider.clone() });
    let builder = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .context("TLS 1.3")?
        .dangerous()
        .with_custom_certificate_verifier(verifier);
    let mut config = match identity {
        Some((certificate, key)) => builder
            .with_client_auth_cert(
                vec![CertificateDer::from(certificate)],
                PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key)),
            )
            .context("agent certificate")?,
        None => builder.with_no_client_auth(),
    };
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(config)
}

/// Vérifie le serveur par l'empreinte de son autorité.
///
/// Le serveur envoie son certificat suivi de l'autorité qui l'a signé. Celle-ci
/// doit avoir une empreinte épinglée ; le certificat du serveur est ensuite
/// vérifié contre elle seule (signature, échéance, usage `serverAuth`, nom
/// [`AGENT_TLS_SERVER_NAME`]). L'adresse composée n'entre pas en compte :
/// IP, nom local ou public, c'est l'autorité épinglée qui fait foi.
#[derive(Debug)]
struct PinnedVerifier {
    pins: Pins,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for PinnedVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let pins = self.pins.read().map(|pins| pins.clone()).unwrap_or_default();
        let Some(anchor) =
            intermediates.iter().find(|candidate| pins.contains(&fingerprint(candidate.as_ref())))
        else {
            return Err(rustls::Error::InvalidCertificate(CertificateError::UnknownIssuer));
        };
        let mut roots = RootCertStore::empty();
        roots.add(anchor.clone())?;
        let inner =
            WebPkiServerVerifier::builder_with_provider(Arc::new(roots), self.provider.clone())
                .build()
                .map_err(|error| rustls::Error::General(error.to_string()))?;
        let name = ServerName::try_from(AGENT_TLS_SERVER_NAME)
            .map_err(|error| rustls::Error::General(error.to_string()))?;
        inner.verify_server_cert(end_entity, intermediates, &name, ocsp_response, now)
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls12_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(message, cert, dss, &self.provider.signature_verification_algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider.signature_verification_algorithms.supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rcgen::{
        BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair,
        KeyUsagePurpose, PKCS_ECDSA_P256_SHA256, SanType,
    };

    /// Une autorité et un certificat de serveur signé par elle, comme ceux
    /// que fabrique le serveur.
    fn authority_and_server(name: &str) -> (CertificateDer<'static>, CertificateDer<'static>) {
        let ca_key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap();
        let mut ca = CertificateParams::default();
        ca.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
        ca.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::DigitalSignature];
        let ca_cert = ca.self_signed(&ca_key).unwrap();
        let issuer = Issuer::new(ca, ca_key);

        let key = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).unwrap();
        let mut leaf = CertificateParams::default();
        leaf.subject_alt_names = vec![SanType::DnsName(name.try_into().unwrap())];
        leaf.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let leaf_cert = leaf.signed_by(&key, &issuer).unwrap();
        (ca_cert.der().clone(), leaf_cert.der().clone())
    }

    fn verify(pins: Vec<String>, ca: &CertificateDer<'_>, leaf: &CertificateDer<'_>) -> bool {
        let verifier = PinnedVerifier { pins: Arc::new(RwLock::new(pins)), provider: provider() };
        let dialled = ServerName::try_from("192.0.2.7").unwrap();
        verifier
            .verify_server_cert(leaf, std::slice::from_ref(ca), &dialled, &[], UnixTime::now())
            .is_ok()
    }

    #[test]
    fn the_server_is_accepted_under_its_pinned_authority_whatever_address_was_dialled() {
        let (ca, leaf) = authority_and_server(AGENT_TLS_SERVER_NAME);
        assert!(verify(vec![fingerprint(ca.as_ref())], &ca, &leaf));
    }

    #[test]
    fn a_wrong_fingerprint_is_refused_even_for_a_valid_chain() {
        // Exactement la position d'un intermédiaire : une chaîne valide, mais
        // sous une autorité qui n'est pas celle de la commande d'installation.
        let (ca, leaf) = authority_and_server(AGENT_TLS_SERVER_NAME);
        let (other_ca, _) = authority_and_server(AGENT_TLS_SERVER_NAME);
        assert!(!verify(vec![fingerprint(other_ca.as_ref())], &ca, &leaf));
        assert!(!verify(Vec::new(), &ca, &leaf), "rien d'épinglé : rien d'accepté");
    }

    #[test]
    fn a_certificate_for_another_name_under_the_pinned_authority_is_refused() {
        // Un certificat client d'agent (ou tout autre) signé par la même
        // autorité ne peut pas se faire passer pour le serveur.
        let (ca, leaf) = authority_and_server("autre.exemple");
        assert!(!verify(vec![fingerprint(ca.as_ref())], &ca, &leaf));
    }

    #[test]
    fn the_client_offers_tls13_and_post_quantum_key_exchange_only() {
        let config = client_config(Arc::new(RwLock::new(Vec::new())), None).unwrap();
        let groups: Vec<String> =
            config.crypto_provider().kx_groups.iter().map(|g| format!("{:?}", g.name())).collect();
        assert_eq!(groups, ["X25519MLKEM768"]);
        assert_eq!(config.alpn_protocols, vec![b"http/1.1".to_vec()]);
    }

    #[test]
    fn the_state_is_stored_privately_and_never_shows_its_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STATE_FILE);
        let (_, key) = new_key().unwrap();
        let mut state = TlsState::new("https://s:8443".into(), vec!["a".repeat(64)]);
        state.certificate = Some(BASE64.encode(b"cert"));
        state.key = Some(BASE64.encode(&key));
        state.not_after_ms = Some(i64::MAX);
        state.save(&path).unwrap();
        assert_eq!(TlsState::load(&path), Some(state.clone()));
        assert!(!format!("{state:?}").contains(state.key.as_deref().unwrap()));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn renewal_starts_ten_days_before_expiry() {
        let day = 24 * 3600 * 1000;
        let mut state = TlsState::new("https://s:8443".into(), Vec::new());
        state.not_after_ms = Some(30 * day);
        assert!(!state.needs_renewal(0));
        assert!(!state.needs_renewal(19 * day));
        assert!(state.needs_renewal(21 * day));
        assert!(!state.usable(0), "sans certificat, rien d'utilisable");
    }
}
