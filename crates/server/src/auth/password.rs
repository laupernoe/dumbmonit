//! Hachage et vérification du mot de passe de l'instance.
//!
//! Argon2id avec les paramètres de l'OWASP (fixés ici plutôt que laissés au
//! défaut de la bibliothèque, qui pourrait changer), et un sel aléatoire tiré
//! à chaque enregistrement : deux instances configurées avec le même mot de
//! passe ne partagent aucune empreinte.

use anyhow::{Context, Result};
use argon2::password_hash::phc::PasswordHash;
use argon2::{Argon2, PasswordHasher, PasswordVerifier};

use crate::auth::{AuthError, AuthResult};

/// Longueur minimale, comptée en caractères et non en octets — sinon « éàü » vaudrait
/// six caractères pour l'utilisateur et trois pour nous.
///
/// Douze caractères, parce que ce mot de passe protège tout : il n'y a ni second
/// facteur, ni verrouillage de compte à opposer à une recherche exhaustive.
pub const MIN_LENGTH: usize = 12;

/// Mémoire (Kio), passes et parallélisme d'Argon2id : 19 Mio, 2, 1 — la
/// configuration de référence de l'OWASP Password Storage Cheat Sheet
/// (révision 2025), environ 30 ms par vérification sur un petit serveur.
const M_COST_KIB: u32 = 19 * 1024;
const T_COST: u32 = 2;
const P_COST: u32 = 1;

fn argon2() -> Argon2<'static> {
    let params =
        argon2::Params::new(M_COST_KIB, T_COST, P_COST, None).expect("paramètres Argon2id valides");
    Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params)
}

/// Refuse un mot de passe trop court, avec un message qui dit quoi faire.
pub fn validate(password: &str) -> AuthResult<()> {
    if password.chars().count() < MIN_LENGTH {
        return Err(AuthError::Invalid(format!(
            "The password must be at least {MIN_LENGTH} characters long."
        )));
    }
    Ok(())
}

/// Hache un mot de passe et renvoie sa chaîne PHC (algorithme, paramètres, sel,
/// empreinte), telle qu'elle sera stockée.
///
/// Le calcul est délibérément coûteux — c'est tout l'intérêt d'Argon2id — donc il
/// est confié à un fil bloquant : le tenir sur un exécuteur Tokio gèlerait les
/// autres requêtes pendant plusieurs dizaines de millisecondes.
pub async fn hash(password: String) -> Result<String> {
    tokio::task::spawn_blocking(move || {
        let hash: PasswordHash = argon2()
            .hash_password(password.as_bytes())
            .map_err(|error| anyhow::anyhow!("hachage Argon2id impossible : {error}"))?;
        Ok(hash.to_string())
    })
    .await
    .context("tâche de hachage interrompue")?
}

/// Vérifie un mot de passe contre une chaîne PHC stockée.
///
/// Un `false` signifie « mot de passe incorrect » ; une erreur signifie « empreinte
/// stockée illisible », ce qui est un problème d'exploitation et non une tentative
/// de connexion ratée. La comparaison finale est faite en temps constant par la
/// bibliothèque.
pub async fn verify(password: String, stored: String) -> Result<bool> {
    tokio::task::spawn_blocking(move || {
        let parsed = PasswordHash::new(&stored).map_err(|error| {
            anyhow::anyhow!("empreinte de mot de passe illisible en base : {error}")
        })?;
        match argon2().verify_password(password.as_bytes(), &parsed) {
            Ok(()) => Ok(true),
            Err(argon2::password_hash::Error::PasswordInvalid) => Ok(false),
            Err(error) => Err(anyhow::anyhow!("vérification Argon2id impossible : {error}")),
        }
    })
    .await
    .context("tâche de vérification interrompue")?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_password_verifies_against_its_own_hash() {
        let stored = hash("mot-de-passe-du-homelab".to_string()).await.unwrap();
        assert!(verify("mot-de-passe-du-homelab".to_string(), stored.clone()).await.unwrap());
        assert!(!verify("mot-de-passe-du-voisin".to_string(), stored).await.unwrap());
    }

    #[tokio::test]
    async fn the_stored_hash_never_contains_the_password() {
        let stored = hash("motdepasse-très-secret".to_string()).await.unwrap();
        assert!(!stored.contains("motdepasse"), "{stored}");
        assert!(stored.starts_with("$argon2id$"), "{stored}");
    }

    #[tokio::test]
    async fn the_hash_uses_the_owasp_argon2id_parameters() {
        let secret = ["mot", "de", "passe", "du", "homelab"].join("-");
        let stored = hash(secret).await.unwrap();
        assert!(stored.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"), "{stored}");
    }

    #[tokio::test]
    async fn the_same_password_yields_two_different_hashes() {
        // Le sel est tiré au hasard à chaque fois : deux instances configurées avec
        // le même mot de passe ne se reconnaissent pas dans leurs empreintes.
        let first = hash("mot-de-passe-identique".to_string()).await.unwrap();
        let second = hash("mot-de-passe-identique".to_string()).await.unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn a_short_password_is_refused_with_a_readable_message() {
        let error = validate("trop-court").unwrap_err();
        match error {
            AuthError::Invalid(message) => assert!(message.contains("12 characters"), "{message}"),
            _ => panic!("un mot de passe trop court doit être une erreur de requête"),
        }
        // Les caractères accentués comptent pour un, comme l'utilisateur les compte.
        assert!(validate("éèàùçéèàùçéè").is_ok());
    }
}
