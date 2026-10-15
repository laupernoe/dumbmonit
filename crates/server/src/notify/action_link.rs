//! Liens d'action signés : acquitter ou mettre en pause une alerte depuis la
//! notification elle-même, sans ouvrir de session.
//!
//! Un lien est une capacité, pas une identité : il ne vaut que pour un geste
//! (`ack` ou `snooze`), sur les alertes qu'il nomme, pendant [`TTL`], une seule
//! fois. Il est signé par HMAC-SHA256 avec une sous-clé du secret d'instance
//! ([`crate::crypto::Cipher::subkey`]) : rien à stocker pour l'émettre, et
//! changer le secret les révoque tous.
//!
//! Forme : `base64url(charge) "." base64url(étiquette)`, la charge étant
//! `version (1) | geste (1) | échéance unix (8, gros-boutiste) | aléa (12) |
//! empreintes séparées par '\n'`. L'aléa identifie le lien pour l'usage unique
//! (table `alert_action_uses`).
//!
//! Le jeton voyage dans le *fragment* de l'URL (`/alert-action#<jeton>`) : un
//! navigateur ne l'envoie jamais au serveur ni dans un `Referer`, si bien qu'il
//! n'apparaît dans aucun journal d'accès, de mandataire ou de ce serveur. La
//! page le lit et le poste dans un corps JSON.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, TimeDelta, Utc};
use hmac::{Hmac, KeyInit, Mac};
use serde::Serialize;
use sha2::Sha256;
use subtle::ConstantTimeEq;

use crate::crypto::Cipher;

/// Durée de validité d'un lien.
pub const TTL: TimeDelta = TimeDelta::hours(24);
/// Pause d'un « Snooze ».
pub const SNOOZE: TimeDelta = TimeDelta::hours(1);
/// Alertes au plus par lien : au-delà, le message ne porte pas de boutons —
/// acquitter d'un geste une avalanche qu'on n'a pas lue n'est pas un service.
pub const MAX_FINGERPRINTS: usize = 10;
/// Longueur maximale d'un jeton reçu, vérifiée avant tout décodage.
pub const MAX_TOKEN_LEN: usize = 2048;
/// Ce que l'acquittement et le journal d'audit retiennent comme auteur.
pub const ACTOR: &str = "via notification link";
/// Page de l'interface qui confirme le geste.
pub const PAGE_PATH: &str = "/alert-action";

const VERSION: u8 = 1;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 32;
const HEADER_LEN: usize = 1 + 1 + 8 + NONCE_LEN;
/// Libellé de la sous-clé : un autre usage du secret ne pourra jamais
/// produire une signature valide ici.
const KEY_LABEL: &[u8] = b"dumbmonit/alert-action-key/v1";
/// Séparation de domaine dans le message signé.
const DOMAIN: &[u8] = b"dumbmonit/alert-action/v1\0";
/// Tolérance d'horloge sur une échéance trop lointaine.
const CLOCK_SKEW: TimeDelta = TimeDelta::minutes(5);

/// Geste porté par un lien.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Acquitte jusqu'à la résolution (la résolution efface l'acquittement ;
    /// le plafond de l'API, 30 jours, borne le reste).
    Ack,
    /// Acquitte une heure : les rappels reprennent ensuite.
    Snooze,
}

impl Action {
    pub const ALL: [Action; 2] = [Action::Ack, Action::Snooze];

    fn code(self) -> u8 {
        match self {
            Self::Ack => 1,
            Self::Snooze => 2,
        }
    }

    fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Ack),
            2 => Some(Self::Snooze),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ack => "ack",
            Self::Snooze => "snooze",
        }
    }

    /// Libellé du bouton, en anglais comme le reste des messages.
    pub fn label(self) -> &'static str {
        match self {
            Self::Ack => "Acknowledge",
            Self::Snooze => "Snooze 1 h",
        }
    }

    /// Durée de l'acquittement posé.
    pub fn duration(self) -> TimeDelta {
        match self {
            Self::Ack => TimeDelta::seconds(crate::api::alerts::MAX_ACK_SECS),
            Self::Snooze => SNOOZE,
        }
    }
}

/// Contenu d'un lien, une fois la signature vérifiée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claims {
    pub action: Action,
    pub fingerprints: Vec<String>,
    pub expires_at: DateTime<Utc>,
    pub nonce: [u8; NONCE_LEN],
}

impl Claims {
    /// Un lien neuf, valable [`TTL`] à partir de `now`.
    pub fn new(action: Action, fingerprints: Vec<String>, now: DateTime<Utc>) -> Self {
        Self { action, fingerprints, expires_at: now + TTL, nonce: rand::random() }
    }

    /// Identifiant d'usage unique, tel que stocké en base.
    pub fn nonce_hex(&self) -> String {
        hex::encode(self.nonce)
    }

    fn encode(&self) -> Vec<u8> {
        let joined = self.fingerprints.join("\n");
        let mut out = Vec::with_capacity(HEADER_LEN + joined.len());
        out.push(VERSION);
        out.push(self.action.code());
        out.extend_from_slice(&self.expires_at.timestamp().to_be_bytes());
        out.extend_from_slice(&self.nonce);
        out.extend_from_slice(joined.as_bytes());
        out
    }

    fn decode(payload: &[u8]) -> Option<Self> {
        if payload.len() <= HEADER_LEN || payload[0] != VERSION {
            return None;
        }
        let action = Action::from_code(payload[1])?;
        let expires = i64::from_be_bytes(payload[2..10].try_into().ok()?);
        let nonce: [u8; NONCE_LEN] = payload[10..HEADER_LEN].try_into().ok()?;
        let joined = std::str::from_utf8(&payload[HEADER_LEN..]).ok()?;
        let fingerprints: Vec<String> = joined.split('\n').map(str::to_string).collect();
        if fingerprints.len() > MAX_FINGERPRINTS || fingerprints.iter().any(String::is_empty) {
            return None;
        }
        Some(Self {
            action,
            fingerprints,
            expires_at: DateTime::from_timestamp(expires, 0)?,
            nonce,
        })
    }
}

/// Pourquoi un jeton est refusé. Ne sort jamais tel quel vers le client : la
/// réponse est la même pour les trois, seul le journal de débogage distingue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejected {
    /// Trop long, mal découpé, base64 invalide, charge illisible.
    Malformed,
    /// Étiquette fausse : jeton forgé, altéré, ou signé par un autre secret.
    BadSignature,
    /// Signature valide, échéance passée (ou absurde).
    Expired,
}

/// Clé de signature des liens, dérivée du secret d'instance.
pub struct ActionKey([u8; 32]);

impl std::fmt::Debug for ActionKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ActionKey(***)")
    }
}

impl ActionKey {
    pub fn from_cipher(cipher: &Cipher) -> Self {
        Self(cipher.subkey(KEY_LABEL))
    }

    #[cfg(test)]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    fn tag(&self, payload: &[u8]) -> [u8; TAG_LEN] {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.0).expect("HMAC accepts any key length");
        mac.update(DOMAIN);
        mac.update(payload);
        let mut out = [0u8; TAG_LEN];
        out.copy_from_slice(&mac.finalize().into_bytes());
        out
    }

    /// Jeton signé pour ces revendications.
    pub fn sign(&self, claims: &Claims) -> String {
        let payload = claims.encode();
        let tag = self.tag(&payload);
        format!("{}.{}", URL_SAFE_NO_PAD.encode(&payload), URL_SAFE_NO_PAD.encode(tag))
    }

    /// Vérifie un jeton : forme, signature (en temps constant), puis échéance.
    ///
    /// La charge n'est lue qu'après la signature : un jeton forgé ne fait
    /// jamais exécuter le décodeur de champs.
    pub fn verify(&self, token: &str, now: DateTime<Utc>) -> Result<Claims, Rejected> {
        let token = token.trim();
        if token.is_empty() || token.len() > MAX_TOKEN_LEN {
            return Err(Rejected::Malformed);
        }
        let (payload, tag) = token.split_once('.').ok_or(Rejected::Malformed)?;
        let payload = URL_SAFE_NO_PAD.decode(payload).map_err(|_| Rejected::Malformed)?;
        let tag = URL_SAFE_NO_PAD.decode(tag).map_err(|_| Rejected::Malformed)?;
        if tag.len() != TAG_LEN {
            return Err(Rejected::Malformed);
        }
        let expected = self.tag(&payload);
        if !bool::from(expected.as_slice().ct_eq(tag.as_slice())) {
            return Err(Rejected::BadSignature);
        }
        let claims = Claims::decode(&payload).ok_or(Rejected::Malformed)?;
        if claims.expires_at <= now || claims.expires_at > now + TTL + CLOCK_SKEW {
            return Err(Rejected::Expired);
        }
        Ok(claims)
    }
}

/// Un bouton prêt à être rendu par un canal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageAction {
    pub action: Action,
    pub label: &'static str,
    /// Adresse absolue, sur l'URL publique de l'instance.
    pub url: String,
    /// Même adresse, relative à l'instance (Web Push n'ouvre que celles-là).
    pub path: String,
}

/// Les boutons d'un message portant ces alertes, ou rien quand il n'y en a
/// aucune ou trop ([`MAX_FINGERPRINTS`]).
pub fn build(
    key: &ActionKey,
    public_url: &str,
    fingerprints: &[String],
    now: DateTime<Utc>,
) -> Vec<MessageAction> {
    let mut unique: Vec<String> = Vec::new();
    for fingerprint in fingerprints {
        if !fingerprint.is_empty() && !fingerprint.contains('\n') && !unique.contains(fingerprint) {
            unique.push(fingerprint.clone());
        }
    }
    if unique.is_empty() || unique.len() > MAX_FINGERPRINTS {
        return Vec::new();
    }
    let base = public_url.trim_end_matches('/');
    Action::ALL
        .into_iter()
        .map(|action| {
            let token = key.sign(&Claims::new(action, unique.clone(), now));
            let path = format!("{PAGE_PATH}#{token}");
            MessageAction { action, label: action.label(), url: format!("{base}{path}"), path }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> ActionKey {
        ActionKey::from_bytes([7; 32])
    }

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).unwrap()
    }

    fn claims() -> Claims {
        Claims::new(Action::Ack, vec!["cpu-high@00000000deadbeef".into()], now())
    }

    #[test]
    fn a_signed_token_round_trips() {
        let claims = claims();
        let token = key().sign(&claims);
        assert_eq!(key().verify(&token, now()), Ok(claims));
        assert!(token.len() < 200, "short enough for every channel: {token}");
        assert!(
            token.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)),
            "URL-safe without escaping: {token}"
        );
    }

    #[test]
    fn another_key_or_a_changed_byte_is_refused() {
        let token = key().sign(&claims());
        assert_eq!(
            ActionKey::from_bytes([8; 32]).verify(&token, now()),
            Err(Rejected::BadSignature)
        );
        // Chaque caractère altéré, d'un côté comme de l'autre du point, est refusé.
        for index in 0..token.len() {
            let mut bytes = token.clone().into_bytes();
            if bytes[index] == b'.' {
                continue;
            }
            bytes[index] = if bytes[index] == b'A' { b'B' } else { b'A' };
            let altered = String::from_utf8(bytes).unwrap();
            if altered == token {
                continue;
            }
            assert!(key().verify(&altered, now()).is_err(), "altered at {index}: accepted");
        }
    }

    #[test]
    fn the_action_cannot_be_swapped_without_the_key() {
        // Passer d'un « snooze » à un « ack » en changeant l'octet du geste.
        let token = key().sign(&Claims::new(Action::Snooze, vec!["a@1".into()], now()));
        let (payload, tag) = token.split_once('.').unwrap();
        let mut raw = URL_SAFE_NO_PAD.decode(payload).unwrap();
        raw[1] = Action::Ack.code();
        let forged = format!("{}.{tag}", URL_SAFE_NO_PAD.encode(raw));
        assert_eq!(key().verify(&forged, now()), Err(Rejected::BadSignature));
    }

    #[test]
    fn an_expired_or_far_future_token_is_refused() {
        let token = key().sign(&claims());
        assert!(key().verify(&token, now() + TTL - TimeDelta::seconds(1)).is_ok());
        assert_eq!(key().verify(&token, now() + TTL), Err(Rejected::Expired));
        let mut far = claims();
        far.expires_at = now() + TimeDelta::days(30);
        assert_eq!(key().verify(&key().sign(&far), now()), Err(Rejected::Expired));
    }

    #[test]
    fn garbage_is_refused_without_panicking() {
        let long = "a".repeat(MAX_TOKEN_LEN + 1);
        for token in ["", ".", "abc", "abc.def", "!!!.???", "a.b.c", long.as_str()] {
            assert!(key().verify(token, now()).is_err(), "{token:?} accepted");
        }
        // Une étiquette valide sur une charge illisible (signée par nous) reste refusée.
        for payload in [&b""[..], &[VERSION][..], &[9; 40][..]] {
            let token = format!(
                "{}.{}",
                URL_SAFE_NO_PAD.encode(payload),
                URL_SAFE_NO_PAD.encode(key().tag(payload))
            );
            assert_eq!(key().verify(&token, now()), Err(Rejected::Malformed));
        }
    }

    #[test]
    fn every_link_has_its_own_nonce() {
        let fps = vec!["a@1".to_string()];
        let links = build(&key(), "https://monit.lan/", &fps, now());
        assert_eq!(links.len(), 2);
        assert!(links[0].url.starts_with("https://monit.lan/alert-action#"));
        assert!(links[0].path.starts_with("/alert-action#"));
        let nonces: Vec<_> = links
            .iter()
            .map(|link| key().verify(link.path.split_once('#').unwrap().1, now()).unwrap().nonce)
            .collect();
        assert_ne!(nonces[0], nonces[1]);
    }

    #[test]
    fn no_buttons_without_alerts_or_for_an_avalanche() {
        assert!(build(&key(), "https://m", &[], now()).is_empty());
        let many: Vec<String> = (0..=MAX_FINGERPRINTS).map(|i| format!("r@{i}")).collect();
        assert!(build(&key(), "https://m", &many, now()).is_empty());
        let dup = vec!["a@1".to_string(), "a@1".to_string()];
        let link = &build(&key(), "https://m", &dup, now())[0];
        let claims = key().verify(link.path.split_once('#').unwrap().1, now()).unwrap();
        assert_eq!(claims.fingerprints, vec!["a@1"]);
    }

    #[test]
    fn the_key_never_shows_in_debug_output() {
        assert_eq!(format!("{:?}", key()), "ActionKey(***)");
    }
}
