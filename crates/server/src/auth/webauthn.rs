//! WebAuthn (clés d'accès) : vérification des cérémonies d'enregistrement et de
//! connexion, sans dépendance de plus.
//!
//! La crate `webauthn-rs` s'appuie sur OpenSSL (`openssl`/`openssl-sys`), ce que
//! DumbMonit s'interdit : image statique musl, TLS par rustls seulement. Ce module
//! implémente donc la part de la spécification dont une clé d'accès a besoin, sur
//! les briques déjà présentes (`p256`, `sha2`, `base64`) :
//!
//! - algorithme **ES256** (ECDSA P-256 / SHA-256, COSE −7) seulement : c'est celui
//!   de tous les trousseaux (iCloud, Google, 1Password, Bitwarden…) et des clés
//!   FIDO2 courantes ; il est annoncé seul à l'enrôlement, si bien qu'un
//!   authentificateur ne propose rien d'autre ;
//! - attestation **`none`** : la clé n'a pas à prouver sa marque, seulement à
//!   posséder la clé privée ;
//! - vérification utilisateur (**UV**, code ou biométrie) exigée : une clé d'accès
//!   vaut alors à elle seule mot de passe et second facteur ;
//! - clé « découvrable » (résidente) : la connexion se fait sans saisir de nom.
//!
//! Les contrôles sont ceux du §7.1 (enregistrement) et §7.2 (assertion) de la
//! spécification, dans l'ordre : type, défi, origine, empreinte de l'identifiant
//! de partie de confiance (RP ID), drapeaux, signature, compteur.

use p256::ecdsa::signature::Verifier;
use p256::ecdsa::{Signature, VerifyingKey};
use serde_json::Value;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Identité sous laquelle la cérémonie se déroule : le domaine (RP ID) et
/// l'origine exacte (schéma, hôte, port) que le navigateur doit annoncer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelyingParty {
    pub id: String,
    pub origin: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WebAuthnError {
    #[error("malformed WebAuthn data: {0}")]
    Malformed(&'static str),
    #[error("WebAuthn check failed: {0}")]
    Rejected(&'static str),
}

type Result<T> = std::result::Result<T, WebAuthnError>;

const FLAG_UP: u8 = 0x01;
const FLAG_UV: u8 = 0x04;
const FLAG_BE: u8 = 0x08;
const FLAG_BS: u8 = 0x10;
const FLAG_AT: u8 = 0x40;

/// COSE : clé EC2, algorithme ES256, courbe P-256.
const COSE_KTY_EC2: i64 = 2;
const COSE_ALG_ES256: i64 = -7;
const COSE_CRV_P256: i64 = 1;

/// Longueur maximale d'un identifiant de clé (spécification : 1023 octets).
const MAX_CREDENTIAL_ID: usize = 1023;

// --------------------------------------------------------------------- CBOR

/// Valeur CBOR, réduite à ce que WebAuthn emploie.
#[derive(Debug, Clone, PartialEq)]
enum Cbor {
    Int(i64),
    Bytes(Vec<u8>),
    Text(String),
    Map(Vec<(Cbor, Cbor)>),
    Other,
}

const MAX_DEPTH: usize = 8;
const MAX_ITEMS: u64 = 256;

/// Lit une valeur CBOR à `pos` et avance. Longueurs définies seulement (le CBOR
/// canonique de CTAP2 n'en connaît pas d'autres).
fn decode(data: &[u8], pos: &mut usize, depth: usize) -> Result<Cbor> {
    if depth > MAX_DEPTH {
        return Err(WebAuthnError::Malformed("CBOR nested too deeply"));
    }
    let initial = *data.get(*pos).ok_or(WebAuthnError::Malformed("CBOR truncated"))?;
    *pos += 1;
    let major = initial >> 5;
    let info = initial & 0x1f;
    let argument = match info {
        0..=23 => u64::from(info),
        24 => u64::from(take(data, pos, 1)?[0]),
        25 => u64::from(u16::from_be_bytes(take(data, pos, 2)?.try_into().unwrap_or_default())),
        26 => u64::from(u32::from_be_bytes(take(data, pos, 4)?.try_into().unwrap_or_default())),
        27 => u64::from_be_bytes(take(data, pos, 8)?.try_into().unwrap_or_default()),
        _ => return Err(WebAuthnError::Malformed("CBOR indefinite length")),
    };
    match major {
        0 => Ok(Cbor::Int(i64::try_from(argument).map_err(|_| bad_int())?)),
        1 => {
            let value = i64::try_from(argument).map_err(|_| bad_int())?;
            Ok(Cbor::Int(-1 - value))
        }
        2 => Ok(Cbor::Bytes(take(data, pos, argument)?.to_vec())),
        3 => Ok(Cbor::Text(
            String::from_utf8(take(data, pos, argument)?.to_vec())
                .map_err(|_| WebAuthnError::Malformed("CBOR text is not UTF-8"))?,
        )),
        4 => {
            if argument > MAX_ITEMS {
                return Err(WebAuthnError::Malformed("CBOR array too long"));
            }
            // Aucun tableau n'est lu par WebAuthn ici : on l'enjambe.
            for _ in 0..argument {
                decode(data, pos, depth + 1)?;
            }
            Ok(Cbor::Other)
        }
        5 => {
            if argument > MAX_ITEMS {
                return Err(WebAuthnError::Malformed("CBOR map too large"));
            }
            let mut entries = Vec::new();
            for _ in 0..argument {
                let key = decode(data, pos, depth + 1)?;
                let value = decode(data, pos, depth + 1)?;
                entries.push((key, value));
            }
            Ok(Cbor::Map(entries))
        }
        // Étiquette : on lit la valeur qu'elle précède.
        6 => decode(data, pos, depth + 1),
        // Booléens, nul, flottants : sans usage ici, mais il faut les enjamber.
        _ => Ok(Cbor::Other),
    }
}

fn bad_int() -> WebAuthnError {
    WebAuthnError::Malformed("CBOR integer out of range")
}

fn take<'a>(data: &'a [u8], pos: &mut usize, len: impl TryInto<usize>) -> Result<&'a [u8]> {
    let len = len.try_into().map_err(|_| WebAuthnError::Malformed("CBOR length"))?;
    let end = pos.checked_add(len).ok_or(WebAuthnError::Malformed("CBOR length"))?;
    let slice = data.get(*pos..end).ok_or(WebAuthnError::Malformed("CBOR truncated"))?;
    *pos = end;
    Ok(slice)
}

impl Cbor {
    fn get_text(&self, key: &str) -> Option<&Cbor> {
        match self {
            Cbor::Map(entries) => entries
                .iter()
                .find(|(k, _)| matches!(k, Cbor::Text(t) if t == key))
                .map(|(_, v)| v),
            _ => None,
        }
    }

    fn get_int(&self, key: i64) -> Option<&Cbor> {
        match self {
            Cbor::Map(entries) => entries
                .iter()
                .find(|(k, _)| matches!(k, Cbor::Int(i) if *i == key))
                .map(|(_, v)| v),
            _ => None,
        }
    }

    fn as_int(&self) -> Option<i64> {
        match self {
            Cbor::Int(i) => Some(*i),
            _ => None,
        }
    }

    fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Cbor::Bytes(b) => Some(b),
            _ => None,
        }
    }
}

// ------------------------------------------------------ données de l'authentificateur

/// Données d'authentificateur (`authenticatorData`), décodées.
#[derive(Debug)]
struct AuthenticatorData {
    rp_id_hash: [u8; 32],
    flags: u8,
    sign_count: u32,
    credential: Option<AttestedCredential>,
}

#[derive(Debug)]
struct AttestedCredential {
    id: Vec<u8>,
    /// Clé publique COSE, déjà convertie en point SEC1 non compressé.
    public_key: Vec<u8>,
}

fn parse_authenticator_data(data: &[u8], expect_credential: bool) -> Result<AuthenticatorData> {
    if data.len() < 37 {
        return Err(WebAuthnError::Malformed("authenticator data too short"));
    }
    let mut rp_id_hash = [0u8; 32];
    rp_id_hash.copy_from_slice(&data[..32]);
    let flags = data[32];
    let sign_count = u32::from_be_bytes([data[33], data[34], data[35], data[36]]);
    let mut credential = None;
    if expect_credential {
        if flags & FLAG_AT == 0 {
            return Err(WebAuthnError::Rejected("no credential in the attestation"));
        }
        // aaguid (16) + longueur de l'identifiant (2) + identifiant + clé COSE.
        let mut pos = 37 + 16;
        let len = take(data, &mut pos, 2)?;
        let len = usize::from(u16::from_be_bytes([len[0], len[1]]));
        if len == 0 || len > MAX_CREDENTIAL_ID {
            return Err(WebAuthnError::Malformed("credential id length"));
        }
        let id = take(data, &mut pos, len)?.to_vec();
        let cose = decode(data, &mut pos, 0)?;
        credential = Some(AttestedCredential { id, public_key: cose_to_sec1(&cose)? });
    }
    Ok(AuthenticatorData { rp_id_hash, flags, sign_count, credential })
}

/// Convertit une clé COSE EC2/ES256 en point P-256 non compressé, et vérifie
/// que c'est bien un point de la courbe.
fn cose_to_sec1(cose: &Cbor) -> Result<Vec<u8>> {
    let int = |key: i64| cose.get_int(key).and_then(Cbor::as_int);
    if int(1) != Some(COSE_KTY_EC2) || int(3) != Some(COSE_ALG_ES256) || int(-1) != Some(COSE_CRV_P256)
    {
        return Err(WebAuthnError::Rejected("only ES256 (P-256) passkeys are supported"));
    }
    let coordinate = |key: i64| {
        cose.get_int(key)
            .and_then(Cbor::as_bytes)
            .filter(|b| b.len() == 32)
            .ok_or(WebAuthnError::Malformed("public key coordinate"))
    };
    let (x, y) = (coordinate(-2)?, coordinate(-3)?);
    let mut sec1 = Vec::with_capacity(65);
    sec1.push(0x04);
    sec1.extend_from_slice(x);
    sec1.extend_from_slice(y);
    VerifyingKey::from_sec1_bytes(&sec1)
        .map_err(|_| WebAuthnError::Rejected("the public key is not a point of P-256"))?;
    Ok(sec1)
}

// --------------------------------------------------------------- client data

/// Contrôles communs : type de cérémonie, défi, origine.
fn check_client_data(
    client_data_json: &[u8],
    ceremony: &str,
    challenge: &[u8],
    rp: &RelyingParty,
) -> Result<()> {
    let value: Value = serde_json::from_slice(client_data_json)
        .map_err(|_| WebAuthnError::Malformed("clientDataJSON"))?;
    let field = |name: &str| value.get(name).and_then(Value::as_str).unwrap_or_default();
    if field("type") != ceremony {
        return Err(WebAuthnError::Rejected("wrong ceremony type"));
    }
    let given = base64url_decode(field("challenge"))
        .ok_or(WebAuthnError::Malformed("challenge encoding"))?;
    if given.len() != challenge.len() || !bool::from(given.ct_eq(challenge)) {
        return Err(WebAuthnError::Rejected("the challenge does not match"));
    }
    if field("origin") != rp.origin {
        return Err(WebAuthnError::Rejected("the origin does not match"));
    }
    // Un iframe d'un autre site ne peut pas mener la cérémonie.
    if value.get("crossOrigin").and_then(Value::as_bool) == Some(true) {
        return Err(WebAuthnError::Rejected("cross-origin ceremony"));
    }
    Ok(())
}

fn check_rp_hash(data: &AuthenticatorData, rp: &RelyingParty) -> Result<()> {
    let expected = Sha256::digest(rp.id.as_bytes());
    if !bool::from(data.rp_id_hash[..].ct_eq(expected.as_slice())) {
        return Err(WebAuthnError::Rejected("the relying party does not match"));
    }
    Ok(())
}

pub fn base64url_decode(text: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(text.trim_end_matches('=')).ok()
}

pub fn base64url_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

// ------------------------------------------------------------ enregistrement

/// Une clé d'accès vérifiée, prête à être rangée.
#[derive(Debug, PartialEq, Eq)]
pub struct NewCredential {
    pub credential_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub sign_count: u32,
    pub backup_eligible: bool,
    pub backup_state: bool,
}

/// Vérifie la réponse d'un `navigator.credentials.create()` (attestation `none`).
pub fn verify_registration(
    rp: &RelyingParty,
    challenge: &[u8],
    client_data_json: &[u8],
    attestation_object: &[u8],
) -> Result<NewCredential> {
    check_client_data(client_data_json, "webauthn.create", challenge, rp)?;
    let mut pos = 0;
    let attestation = decode(attestation_object, &mut pos, 0)?;
    let auth_data = attestation
        .get_text("authData")
        .and_then(Cbor::as_bytes)
        .ok_or(WebAuthnError::Malformed("attestation object"))?;
    let data = parse_authenticator_data(auth_data, true)?;
    check_rp_hash(&data, rp)?;
    if data.flags & FLAG_UP == 0 {
        return Err(WebAuthnError::Rejected("user presence is missing"));
    }
    if data.flags & FLAG_UV == 0 {
        return Err(WebAuthnError::Rejected("user verification is required"));
    }
    let credential = data.credential.ok_or(WebAuthnError::Malformed("credential"))?;
    Ok(NewCredential {
        credential_id: credential.id,
        public_key: credential.public_key,
        sign_count: data.sign_count,
        backup_eligible: data.flags & FLAG_BE != 0,
        backup_state: data.flags & FLAG_BS != 0,
    })
}

// ----------------------------------------------------------------- assertion

/// Résultat d'une assertion valide.
#[derive(Debug, PartialEq, Eq)]
pub struct Assertion {
    pub sign_count: u32,
    pub backup_state: bool,
}

/// Vérifie la réponse d'un `navigator.credentials.get()` contre la clé stockée.
pub fn verify_assertion(
    rp: &RelyingParty,
    challenge: &[u8],
    client_data_json: &[u8],
    authenticator_data: &[u8],
    signature_der: &[u8],
    public_key: &[u8],
    stored_sign_count: u32,
) -> Result<Assertion> {
    check_client_data(client_data_json, "webauthn.get", challenge, rp)?;
    let data = parse_authenticator_data(authenticator_data, false)?;
    check_rp_hash(&data, rp)?;
    if data.flags & FLAG_UP == 0 {
        return Err(WebAuthnError::Rejected("user presence is missing"));
    }
    if data.flags & FLAG_UV == 0 {
        return Err(WebAuthnError::Rejected("user verification is required"));
    }

    let key = VerifyingKey::from_sec1_bytes(public_key)
        .map_err(|_| WebAuthnError::Malformed("stored public key"))?;
    let raw = der_signature_to_raw(signature_der)
        .ok_or(WebAuthnError::Malformed("signature encoding"))?;
    let signature =
        Signature::from_slice(&raw).map_err(|_| WebAuthnError::Malformed("signature"))?;
    let mut signed = authenticator_data.to_vec();
    signed.extend_from_slice(&Sha256::digest(client_data_json));
    key.verify(&signed, &signature)
        .map_err(|_| WebAuthnError::Rejected("the signature is not valid"))?;

    // Un compteur qui n'avance pas trahit une clé clonée. Les clés synchronisées
    // (trousseaux) renvoient toujours 0 : on ne l'exige donc que d'une clé qui
    // en a déjà donné un.
    if (data.sign_count != 0 || stored_sign_count != 0) && data.sign_count <= stored_sign_count {
        return Err(WebAuthnError::Rejected("the signature counter went backwards (cloned key?)"));
    }
    Ok(Assertion { sign_count: data.sign_count, backup_state: data.flags & FLAG_BS != 0 })
}

/// Signature ECDSA DER (`SEQUENCE { INTEGER r, INTEGER s }`) en `r || s`, 64 octets.
fn der_signature_to_raw(der: &[u8]) -> Option<[u8; 64]> {
    let (&tag, rest) = der.split_first()?;
    if tag != 0x30 {
        return None;
    }
    let (len, rest) = der_length(rest)?;
    if rest.len() != len {
        return None;
    }
    let (r, rest) = der_integer(rest)?;
    let (s, rest) = der_integer(rest)?;
    if !rest.is_empty() {
        return None;
    }
    let mut raw = [0u8; 64];
    raw[32 - r.len()..32].copy_from_slice(r);
    raw[64 - s.len()..].copy_from_slice(s);
    Some(raw)
}

fn der_length(data: &[u8]) -> Option<(usize, &[u8])> {
    let (&first, rest) = data.split_first()?;
    if first < 0x80 {
        return Some((usize::from(first), rest));
    }
    if first == 0x81 {
        let (&len, rest) = rest.split_first()?;
        return Some((usize::from(len), rest));
    }
    None
}

/// Entier DER non signé, sans l'octet de tête nul : 1 à 32 octets.
fn der_integer(data: &[u8]) -> Option<(&[u8], &[u8])> {
    let (&tag, rest) = data.split_first()?;
    if tag != 0x02 {
        return None;
    }
    let (len, rest) = der_length(rest)?;
    let value = rest.get(..len)?;
    let remainder = &rest[len..];
    let value = match value {
        [0, tail @ ..] => tail,
        other => other,
    };
    (!value.is_empty() && value.len() <= 32).then_some((value, remainder))
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::SigningKey;
    use p256::ecdsa::signature::Signer;

    const RP_ID: &str = "monit.example.org";
    const ORIGIN: &str = "https://monit.example.org";

    fn rp() -> RelyingParty {
        RelyingParty { id: RP_ID.into(), origin: ORIGIN.into() }
    }

    fn signing_key() -> SigningKey {
        SigningKey::from_slice(&[7u8; 32]).unwrap()
    }

    fn client_data(ceremony: &str, challenge: &[u8], origin: &str) -> Vec<u8> {
        serde_json::json!({
            "type": ceremony,
            "challenge": base64url_encode(challenge),
            "origin": origin,
            "crossOrigin": false,
        })
        .to_string()
        .into_bytes()
    }

    /// COSE EC2/ES256 de la clé de test, en CBOR canonique.
    fn cose_key(key: &SigningKey) -> Vec<u8> {
        let point = key.verifying_key().to_encoded_point(false);
        let mut out = vec![0xa5, 0x01, 0x02, 0x03, 0x26, 0x20, 0x01];
        out.extend_from_slice(&[0x21, 0x58, 0x20]);
        out.extend_from_slice(point.x().unwrap());
        out.extend_from_slice(&[0x22, 0x58, 0x20]);
        out.extend_from_slice(point.y().unwrap());
        out
    }

    fn auth_data(flags: u8, count: u32, credential: Option<(&[u8], Vec<u8>)>) -> Vec<u8> {
        let mut out = Sha256::digest(RP_ID.as_bytes()).to_vec();
        out.push(flags);
        out.extend_from_slice(&count.to_be_bytes());
        if let Some((id, cose)) = credential {
            out.extend_from_slice(&[0u8; 16]);
            out.extend_from_slice(&(id.len() as u16).to_be_bytes());
            out.extend_from_slice(id);
            out.extend_from_slice(&cose);
        }
        out
    }

    /// `{"fmt": "none", "attStmt": {}, "authData": <bytes>}`.
    fn attestation(auth: &[u8]) -> Vec<u8> {
        let mut out = vec![0xa3];
        out.extend_from_slice(&[0x63, b'f', b'm', b't', 0x64, b'n', b'o', b'n', b'e']);
        out.extend_from_slice(&[0x67, b'a', b't', b't', b'S', b't', b'm', b't', 0xa0]);
        out.extend_from_slice(&[0x68, b'a', b'u', b't', b'h', b'D', b'a', b't', b'a']);
        out.push(0x59);
        out.extend_from_slice(&(auth.len() as u16).to_be_bytes());
        out.extend_from_slice(auth);
        out
    }

    const CHALLENGE: [u8; 32] = [9u8; 32];
    const UP_UV: u8 = FLAG_UP | FLAG_UV;

    #[test]
    fn un_enregistrement_valide_donne_la_cle_publique() {
        let key = signing_key();
        let auth = auth_data(UP_UV | FLAG_AT | FLAG_BE | FLAG_BS, 0, Some((b"cred-1", cose_key(&key))));
        let result = verify_registration(
            &rp(),
            &CHALLENGE,
            &client_data("webauthn.create", &CHALLENGE, ORIGIN),
            &attestation(&auth),
        )
        .unwrap();
        assert_eq!(result.credential_id, b"cred-1");
        assert_eq!(
            result.public_key,
            key.verifying_key().to_encoded_point(false).as_bytes().to_vec()
        );
        assert!(result.backup_eligible && result.backup_state);
    }

    #[test]
    fn un_enregistrement_est_refuse_si_defi_origine_ou_drapeaux_different() {
        let key = signing_key();
        let good = auth_data(UP_UV | FLAG_AT, 0, Some((b"c", cose_key(&key))));
        let run = |challenge: &[u8], origin: &str, auth: &[u8]| {
            verify_registration(
                &rp(),
                &CHALLENGE,
                &client_data("webauthn.create", challenge, origin),
                &attestation(auth),
            )
        };
        assert!(run(&CHALLENGE, ORIGIN, &good).is_ok());
        assert_eq!(
            run(&[1u8; 32], ORIGIN, &good).unwrap_err(),
            WebAuthnError::Rejected("the challenge does not match")
        );
        assert_eq!(
            run(&CHALLENGE, "https://evil.example", &good).unwrap_err(),
            WebAuthnError::Rejected("the origin does not match")
        );
        let no_uv = auth_data(FLAG_UP | FLAG_AT, 0, Some((b"c", cose_key(&key))));
        assert_eq!(
            run(&CHALLENGE, ORIGIN, &no_uv).unwrap_err(),
            WebAuthnError::Rejected("user verification is required")
        );
        let no_up = auth_data(FLAG_UV | FLAG_AT, 0, Some((b"c", cose_key(&key))));
        assert!(run(&CHALLENGE, ORIGIN, &no_up).is_err());
        let no_at = auth_data(UP_UV, 0, None);
        assert!(run(&CHALLENGE, ORIGIN, &no_at).is_err());
        // Mauvais domaine : l'empreinte du RP ID ne correspond pas.
        let other_rp = RelyingParty { id: "other.example".into(), origin: ORIGIN.into() };
        assert_eq!(
            verify_registration(
                &other_rp,
                &CHALLENGE,
                &client_data("webauthn.create", &CHALLENGE, ORIGIN),
                &attestation(&good),
            )
            .unwrap_err(),
            WebAuthnError::Rejected("the relying party does not match")
        );
        // Mauvais type de cérémonie.
        assert!(
            verify_registration(
                &rp(),
                &CHALLENGE,
                &client_data("webauthn.get", &CHALLENGE, ORIGIN),
                &attestation(&good),
            )
            .is_err()
        );
    }

    #[test]
    fn un_algorithme_autre_qu_es256_est_refuse() {
        let key = signing_key();
        let mut cose = cose_key(&key);
        cose[4] = 0x27; // alg -8 (EdDSA)
        let auth = auth_data(UP_UV | FLAG_AT, 0, Some((b"c", cose)));
        let error = verify_registration(
            &rp(),
            &CHALLENGE,
            &client_data("webauthn.create", &CHALLENGE, ORIGIN),
            &attestation(&auth),
        )
        .unwrap_err();
        assert_eq!(error, WebAuthnError::Rejected("only ES256 (P-256) passkeys are supported"));
    }

    /// `r || s` en DER : le format que renvoie un authentificateur.
    fn raw_to_der(raw: &[u8]) -> Vec<u8> {
        let integer = |bytes: &[u8]| {
            let mut value: Vec<u8> = bytes.iter().copied().skip_while(|b| *b == 0).collect();
            if value.first().is_none_or(|b| b & 0x80 != 0) {
                value.insert(0, 0);
            }
            let mut out = vec![0x02, value.len() as u8];
            out.extend(value);
            out
        };
        let mut body = integer(&raw[..32]);
        body.extend(integer(&raw[32..]));
        let mut out = vec![0x30];
        if body.len() >= 0x80 {
            out.push(0x81);
        }
        out.push(body.len() as u8);
        out.extend(body);
        out
    }

    fn assertion(count: u32, flags: u8, challenge: &[u8]) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let key = signing_key();
        let auth = auth_data(flags, count, None);
        let client = client_data("webauthn.get", challenge, ORIGIN);
        let mut signed = auth.clone();
        signed.extend_from_slice(&Sha256::digest(&client));
        let signature: Signature = key.sign(&signed);
        (client, auth, raw_to_der(&signature.to_bytes()))
    }

    fn public_key() -> Vec<u8> {
        signing_key().verifying_key().to_encoded_point(false).as_bytes().to_vec()
    }

    #[test]
    fn une_assertion_valide_passe_et_rend_le_compteur() {
        let (client, auth, signature) = assertion(5, UP_UV, &CHALLENGE);
        let result =
            verify_assertion(&rp(), &CHALLENGE, &client, &auth, &signature, &public_key(), 4)
                .unwrap();
        assert_eq!(result, Assertion { sign_count: 5, backup_state: false });
    }

    #[test]
    fn une_assertion_trafiquee_est_refusee() {
        let (client, auth, signature) = assertion(5, UP_UV, &CHALLENGE);
        let check = |client: &[u8], auth: &[u8], sig: &[u8], stored: u32| {
            verify_assertion(&rp(), &CHALLENGE, client, auth, sig, &public_key(), stored)
        };
        assert!(check(&client, &auth, &signature, 4).is_ok());

        // Signature d'une autre donnée.
        let (_, other_auth, _) = assertion(6, UP_UV, &CHALLENGE);
        assert_eq!(
            check(&client, &other_auth, &signature, 4).unwrap_err(),
            WebAuthnError::Rejected("the signature is not valid")
        );
        // Défi rejoué d'une autre cérémonie.
        let (other_client, other_auth, other_sig) = assertion(5, UP_UV, &[1u8; 32]);
        assert_eq!(
            check(&other_client, &other_auth, &other_sig, 4).unwrap_err(),
            WebAuthnError::Rejected("the challenge does not match")
        );
        // Sans vérification de l'utilisateur.
        let (c, a, s) = assertion(5, FLAG_UP, &CHALLENGE);
        assert_eq!(
            check(&c, &a, &s, 4).unwrap_err(),
            WebAuthnError::Rejected("user verification is required")
        );
        // Compteur qui recule.
        assert!(check(&client, &auth, &signature, 5).is_err());
        assert!(check(&client, &auth, &signature, 9).is_err());
        // Signature tronquée.
        assert!(check(&client, &auth, &signature[..10], 4).is_err());
    }

    #[test]
    fn les_cles_synchronisees_a_compteur_nul_sont_acceptees() {
        let (client, auth, signature) = assertion(0, UP_UV | FLAG_BE | FLAG_BS, &CHALLENGE);
        let result =
            verify_assertion(&rp(), &CHALLENGE, &client, &auth, &signature, &public_key(), 0)
                .unwrap();
        assert_eq!(result, Assertion { sign_count: 0, backup_state: true });
    }

    #[test]
    fn la_signature_der_est_convertie_avec_ou_sans_octet_de_tete() {
        // r avec bit de poids fort (octet de tête 00), s court.
        let mut der = vec![0x30, 0x26, 0x02, 0x21, 0x00];
        der.extend_from_slice(&[0xff; 32]);
        der.extend_from_slice(&[0x02, 0x01, 0x05]);
        let raw = der_signature_to_raw(&der).unwrap();
        assert_eq!(&raw[..32], &[0xff; 32]);
        assert_eq!(raw[63], 5);
        assert_eq!(&raw[32..63], &[0u8; 31]);
        assert!(der_signature_to_raw(&[0x31, 0x00]).is_none());
        assert!(der_signature_to_raw(&[]).is_none());
    }

    #[test]
    fn le_cbor_hostile_est_borne() {
        // Tableaux imbriqués sur 40 niveaux.
        let nested = vec![0x81u8; 40];
        let mut pos = 0;
        assert!(decode(&nested, &mut pos, 0).is_err());
        // Longueur indéfinie, texte tronqué.
        let mut pos = 0;
        assert!(decode(&[0x5f], &mut pos, 0).is_err());
        let mut pos = 0;
        assert!(decode(&[0x42, 0x01], &mut pos, 0).is_err());
    }
}
