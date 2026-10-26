//! Signature des binaires de l'agent : Ed25519, clés compilées dans l'agent.
//!
//! L'empreinte SHA-256 vient du serveur, avec le binaire : elle détecte un
//! téléchargement abîmé, pas un serveur (ou un intermédiaire) qui servirait
//! autre chose. La signature, elle, ne peut venir que de qui détient la clé
//! privée de publication — un secret de la CI, jamais sur un serveur
//! DumbMonit. Les clés publiques sont lues dans `signing/release-keys.pem` à
//! la compilation : un agent n'installe que ce que l'une d'elles a signé.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use rustls::SignatureScheme;
use rustls::pki_types::SignatureVerificationAlgorithm;

/// Clés publiques de publication, au format PEM (`BEGIN PUBLIC KEY`).
const RELEASE_KEYS: &str = include_str!("../../signing/release-keys.pem");

/// En-tête DER d'une clé publique Ed25519 (SubjectPublicKeyInfo, RFC 8410),
/// suivi des 32 octets de la clé.
const ED25519_SPKI_PREFIX: [u8; 12] =
    [0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00];

/// Longueur d'une signature Ed25519.
const SIGNATURE_LEN: usize = 64;

/// Clés publiques Ed25519 (32 octets chacune) d'un texte PEM. Les blocs
/// illisibles, ou d'un autre algorithme, sont ignorés.
pub fn parse_keys(pem: &str) -> Vec<[u8; 32]> {
    let mut keys = Vec::new();
    let mut body: Option<String> = None;
    for line in pem.lines().map(str::trim) {
        if line == "-----BEGIN PUBLIC KEY-----" {
            body = Some(String::new());
        } else if line == "-----END PUBLIC KEY-----" {
            if let Some(key) = body.take().and_then(|text| decode_key(&text)) {
                keys.push(key);
            }
        } else if let Some(text) = body.as_mut() {
            text.push_str(line);
        }
    }
    keys
}

fn decode_key(base64: &str) -> Option<[u8; 32]> {
    let der = BASE64.decode(base64).ok()?;
    let raw = der.strip_prefix(ED25519_SPKI_PREFIX.as_slice())?;
    raw.try_into().ok()
}

/// Clés de publication compilées dans ce binaire.
pub fn release_keys() -> Vec<[u8; 32]> {
    parse_keys(RELEASE_KEYS)
}

/// Verdict sur la signature d'un binaire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Signée par l'une des clés.
    Valid,
    /// Une signature, mais d'aucune des clés connues (ou illisible) : refus,
    /// quels que soient les réglages.
    Invalid,
    /// Pas de signature, ou pas de clé pour la vérifier.
    Unsigned,
}

/// Vérifie `signature` (base64) de `binary` contre `keys`.
pub fn verify(binary: &[u8], signature: Option<&str>, keys: &[[u8; 32]]) -> Verdict {
    let Some(signature) = signature.map(str::trim).filter(|s| !s.is_empty()) else {
        return Verdict::Unsigned;
    };
    if keys.is_empty() {
        return Verdict::Unsigned;
    }
    let Ok(signature) = BASE64.decode(signature) else { return Verdict::Invalid };
    if signature.len() != SIGNATURE_LEN {
        return Verdict::Invalid;
    }
    let Some(algorithm) = ed25519() else { return Verdict::Invalid };
    let valid = keys.iter().any(|key| algorithm.verify_signature(key, binary, &signature).is_ok());
    if valid { Verdict::Valid } else { Verdict::Invalid }
}

/// Table des algorithmes de vérification d'un fournisseur rustls.
type Mapping = [(SignatureScheme, &'static [&'static dyn SignatureVerificationAlgorithm])];

/// L'algorithme Ed25519 du fournisseur aws-lc-rs, celui du canal TLS : pas de
/// seconde bibliothèque cryptographique dans l'agent.
fn ed25519() -> Option<&'static dyn SignatureVerificationAlgorithm> {
    let mapping: &'static Mapping =
        rustls::crypto::aws_lc_rs::default_provider().signature_verification_algorithms.mapping;
    mapping
        .iter()
        .find(|(scheme, _)| *scheme == SignatureScheme::ED25519)
        .and_then(|(_, algorithms)| algorithms.first().copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Paire de test, produite par `openssl genpkey -algorithm ed25519`, et
    /// signature de [`MESSAGE`] par `openssl pkeyutl -sign -rawin` — l'outil
    /// même qu'emploie la CI.
    const TEST_KEY: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA8/ZUyYZRHvnPqoa1ZNbEvPh85mZz0sKRhzS3dgloEHU=
-----END PUBLIC KEY-----
";
    const OTHER_KEY: &str = "-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEARtQ6ZcijPmkDM668vQ6bWt2gu39e0udddirvIG12kQE=
-----END PUBLIC KEY-----
";
    const MESSAGE: &[u8] = b"dumbmonit-agent test binary\n";
    const SIGNATURE: &str =
        "MklaAxztG+jnCZhMAvBGIWI9TvW+cy8aw01ahKBP/SlGVjouUpUVgiA2M11nNfarU5E6Eb74UFsrZu2VQUQHAA==";

    #[test]
    fn a_binary_signed_by_a_known_key_is_valid() {
        let keys = parse_keys(TEST_KEY);
        assert_eq!(keys.len(), 1);
        assert_eq!(verify(MESSAGE, Some(SIGNATURE), &keys), Verdict::Valid);
        // Plusieurs clés (rotation) : il suffit que l'une d'elles signe.
        let both = parse_keys(&format!("{OTHER_KEY}\n# commentaire\n{TEST_KEY}"));
        assert_eq!(both.len(), 2);
        assert_eq!(verify(MESSAGE, Some(SIGNATURE), &both), Verdict::Valid);
    }

    #[test]
    fn a_modified_binary_or_another_key_is_refused() {
        let keys = parse_keys(TEST_KEY);
        assert_eq!(
            verify(b"dumbmonit-agent test binarY\n", Some(SIGNATURE), &keys),
            Verdict::Invalid
        );
        assert_eq!(verify(MESSAGE, Some(SIGNATURE), &parse_keys(OTHER_KEY)), Verdict::Invalid);
        assert_eq!(verify(MESSAGE, Some("pas du base64 !"), &keys), Verdict::Invalid);
        assert_eq!(verify(MESSAGE, Some("AAAA"), &keys), Verdict::Invalid, "trop courte");
    }

    #[test]
    fn a_missing_signature_or_key_means_unsigned() {
        let keys = parse_keys(TEST_KEY);
        assert_eq!(verify(MESSAGE, None, &keys), Verdict::Unsigned);
        assert_eq!(verify(MESSAGE, Some("  "), &keys), Verdict::Unsigned);
        assert_eq!(verify(MESSAGE, Some(SIGNATURE), &[]), Verdict::Unsigned);
    }

    #[test]
    fn the_key_file_only_yields_ed25519_keys() {
        // Une clé RSA, ou un bloc abîmé, ne compte pas : ce n'est pas une clé
        // de publication.
        let rsa = "MFwwDQYJKoZIhvcNAQEBBQADSwAwSAJBAK";
        let rsa = format!("-----BEGIN PUBLIC KEY-----\n{rsa}\n-----END PUBLIC KEY-----\n");
        assert!(parse_keys(&rsa).is_empty());
        assert!(parse_keys("-----BEGIN PUBLIC KEY-----\nMCowBQ\n").is_empty(), "bloc non fermé");
        // Le fichier livré se lit sans erreur, avec ou sans clé.
        let _ = release_keys();
    }
}
