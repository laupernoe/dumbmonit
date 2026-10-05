//! Code de première configuration.
//!
//! Tant qu'aucun administrateur n'existe, `POST /api/auth/setup` est ouvert à
//! qui joint le serveur — sur un réseau local partagé, ou une instance exposée
//! trop tôt, le premier venu prendrait l'instance. Le code ferme cette porte :
//! tiré au démarrage, gardé en mémoire seulement, affiché dans le journal du
//! serveur. Le lire prouve qu'on a la main sur la machine qui fait tourner
//! DumbMonit, exactement comme pour `DUMBMONIT_RESET_PASSWORD`.
//!
//! Il change à chaque démarrage tant que la configuration n'est pas faite ;
//! `DUMBMONIT_SETUP_CODE` le fixe pour un déploiement automatisé.

use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Alphabet sans les caractères qu'on confond à la lecture (0/O, 1/I/L).
const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

/// Longueur du code, hors tiret : 10 caractères de 31, environ 50 bits —
/// largement assez derrière le compteur de tentatives.
const LENGTH: usize = 10;

/// Tire un code lisible, `XXXXX-XXXXX`.
pub fn generate() -> String {
    let raw: String = (0..LENGTH)
        .map(|_| {
            // Rejet des tirages au-delà du dernier multiple : pas de biais.
            loop {
                let byte = rand::random::<u8>() as usize;
                if byte < 256 - 256 % ALPHABET.len() {
                    return ALPHABET[byte % ALPHABET.len()] as char;
                }
            }
        })
        .collect();
    format!("{}-{}", &raw[..LENGTH / 2], &raw[LENGTH / 2..])
}

/// Forme canonique d'un code saisi : majuscules, sans tiret ni espace.
pub fn normalize(code: &str) -> String {
    code.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect()
}

/// Compare un code saisi au code attendu, en temps constant.
///
/// Un code vide ne correspond jamais, même à un code attendu vide.
pub fn matches(expected: &str, given: &str) -> bool {
    let given = normalize(given);
    if given.is_empty() {
        return false;
    }
    let expected = Sha256::digest(normalize(expected).as_bytes());
    let given = Sha256::digest(given.as_bytes());
    bool::from(expected.as_slice().ct_eq(given.as_slice()))
}

/// Affiche le code dans le journal, bien en vue.
pub fn announce(code: &str) {
    tracing::warn!(
        "\n\n  ============================================================\n  \
         First-run setup code: {code}\n  \
         Enter it on the setup page to create the admin account.\n  \
         It changes at every restart until an admin exists.\n  \
         ============================================================\n"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_generated_code_is_readable_and_unambiguous() {
        let code = generate();
        assert_eq!(code.len(), LENGTH + 1);
        assert_eq!(&code[5..6], "-");
        assert!(normalize(&code).bytes().all(|b| ALPHABET.contains(&b)), "{code}");
        assert_ne!(generate(), generate());
    }

    #[test]
    fn the_comparison_ignores_case_dashes_and_spaces() {
        assert!(matches("ABCDE-FGHJK", "abcde fghjk"));
        assert!(matches("ABCDE-FGHJK", " ABCDEFGHJK "));
        assert!(!matches("ABCDE-FGHJK", "ABCDE-FGHJM"));
        assert!(!matches("ABCDE-FGHJK", ""));
    }
}
