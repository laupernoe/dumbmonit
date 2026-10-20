//! Expurgation des journaux conservés ou expédiés.
//!
//! Le code du serveur prend déjà soin de ne rien journaliser de secret (les
//! erreurs reqwest passent par `without_url`, les spans HTTP ne portent que le
//! chemin). Ce module est le second filet : un fichier gardé deux semaines et
//! copié chez un service tiers mérite qu'on masque ce qui *ressemble* à un
//! secret, même si personne ne l'y a mis exprès — un identifiant dans une URL,
//! un jeton dans une chaîne de requête, un en-tête `Authorization` recopié dans
//! un message d'erreur, le jeton d'un bot Telegram dans son chemin.

use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value;

/// Marqueur substitué aux secrets.
pub const MASK: &str = "***";

/// Clés de champ dont la valeur est toujours masquée.
const SECRET_FIELDS: [&str; 10] = [
    "password",
    "passwd",
    "passphrase",
    "secret",
    "api_key",
    "apikey",
    "authorization",
    "cookie",
    "credential",
    "private_key",
];

struct Rules(Vec<(Regex, &'static str)>);

fn rules() -> &'static Rules {
    static RULES: OnceLock<Rules> = OnceLock::new();
    RULES.get_or_init(|| {
        let rule = |pattern: &str, replacement: &'static str| {
            (Regex::new(pattern).expect("motif d'expurgation valide"), replacement)
        };
        Rules(vec![
            // Identifiants dans une URL : `scheme://user:pass@host`.
            rule(r"(?i)\b([a-z][a-z0-9+.\-]*://)[^/\s:@]+:[^/\s@]+@", "${1}***@"),
            // Jeton d'un bot Telegram dans le chemin.
            rule(r"/bot\d+:[A-Za-z0-9_\-]+", "/bot***"),
            // Paramètres de requête sensibles.
            rule(
                r"(?i)([?&](?:access_token|refresh_token|id_token|token|api[_\-]?key|key|password|passwd|pass|secret|auth|sig|signature|code|client_secret)=)[^&\s#'\x22]+",
                "${1}***",
            ),
            // `password=…`, `secret: …` dans un texte libre.
            rule(
                r#"(?i)\b(password|passwd|passphrase|secret|api_key|apikey|client_secret)(["']?\s*[:=]\s*["']?)[^\s"',;&]+"#,
                "${1}${2}***",
            ),
        ])
    })
}

/// Masque ce qui ressemble à un secret dans un texte.
pub fn text(input: &str) -> String {
    let mut out = std::borrow::Cow::Borrowed(input);
    for (regex, replacement) in &rules().0 {
        if regex.is_match(&out) {
            out = std::borrow::Cow::Owned(regex.replace_all(&out, *replacement).into_owned());
        }
    }
    // En-têtes d'autorisation recopiés dans un message : seulement si la valeur
    // a l'allure d'un jeton (un chiffre ou un signe base64), pour ne pas masquer
    // « basic authentication ».
    static AUTH: OnceLock<Regex> = OnceLock::new();
    let auth = AUTH.get_or_init(|| {
        Regex::new(r"(?i)\b(bearer|basic|apikey)\s+([A-Za-z0-9._~+/=\-]{8,})")
            .expect("motif d'expurgation valide")
    });
    if auth.is_match(&out) {
        let replaced = auth.replace_all(&out, |caps: &regex::Captures<'_>| {
            let looks_like_token = caps[2].bytes().any(|b| {
                b.is_ascii_digit() || matches!(b, b'+' | b'/' | b'=' | b'.' | b'_' | b'-')
            });
            if looks_like_token { format!("{} {MASK}", &caps[1]) } else { caps[0].to_string() }
        });
        out = std::borrow::Cow::Owned(replaced.into_owned());
    }
    out.into_owned()
}

/// Masque la valeur d'un champ dont le nom désigne un secret.
pub fn field(key: &str, value: Value) -> Value {
    let lower = key.to_ascii_lowercase();
    if SECRET_FIELDS.iter().any(|secret| lower == *secret || lower.ends_with(&format!("_{secret}")))
    {
        Value::String(MASK.into())
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masque_les_formes_courantes() {
        let cases = [
            ("https://bob:hunter22@nas.lan:5001/x", "https://***@nas.lan:5001/x"),
            ("GET /api?token=abc123&page=2", "GET /api?token=***&page=2"),
            (
                "https://api.telegram.org/bot42:AAE_x-y/sendMessage",
                "https://api.telegram.org/bot***/sendMessage",
            ),
            ("header Authorization: Bearer abcdefgh1jkl", "header Authorization: Bearer ***"),
            ("uses basic authentication", "uses basic authentication"),
            ("login failed password=topsecret for bob", "login failed password=*** for bob"),
        ];
        for (input, expected) in cases {
            assert_eq!(text(input), expected);
        }
    }

    #[test]
    fn laisse_le_reste_intact() {
        let plain = "probe failed: connection refused (10.0.0.4:161), token rotation scheduled";
        assert_eq!(text(plain), plain);
        assert_eq!(field("token_name", Value::from("ci")), Value::from("ci"));
        assert_eq!(field("smtp_password", Value::from("x")), Value::from(MASK));
    }
}
