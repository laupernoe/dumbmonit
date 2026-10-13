//! Réglages d'un site surveillé, lus dans l'adresse et les étiquettes de la
//! cible. Toute erreur ici est une erreur de configuration : elle s'affiche à
//! côté de l'équipement et ne déclenche aucune alerte.

use std::collections::BTreeSet;
use std::time::Duration;

use dumbmonit_proto::{ProbeError, Target};
use reqwest::Url;
use sha2::{Digest, Sha256};

pub const OPTION_EXPECT_TEXT: &str = "expect_text";
pub const OPTION_MIN_TEXT_CHARS: &str = "min_text_chars";
pub const OPTION_MAX_REDIRECTS: &str = "max_redirects";
pub const OPTION_CATEGORY: &str = "category";
pub const OPTION_USER_AGENT: &str = "user_agent";
pub const OPTION_IGNORE_QUERY: &str = "ignore_query";
pub const OPTION_ALLOW_PRIVATE: &str = dumbmonit_collectors::uptime::guard::OPTION;
pub const OPTION_CHANGE_CHECK: &str = "change_check_minutes";
pub const OPTION_HOST_SPACING: &str = "host_spacing_seconds";
pub const OPTION_CONFIRM_CHECKS: &str = "confirm_checks";
pub const OPTION_TEXT_DROP: &str = "text_drop_percent";

pub const DEFAULT_MIN_TEXT_CHARS: u32 = 20;
pub const MAX_MIN_TEXT_CHARS: u32 = 100_000;
pub const DEFAULT_MAX_REDIRECTS: u32 = 10;
pub const MAX_MAX_REDIRECTS: u32 = 30;
/// Lectures de suite qui confirment un changement.
pub const DEFAULT_CONFIRM_CHECKS: u32 = 2;
pub const MAX_CONFIRM_CHECKS: u32 = 10;
/// Chute du texte visible, en % de la référence, qui compte comme une page
/// quasi vide ; 0 coupe le suivi.
pub const DEFAULT_TEXT_DROP_PERCENT: u32 = 80;
pub const MAX_TEXT_DROP_PERCENT: u32 = 99;
pub const DEFAULT_IGNORE_QUERY: bool = true;
/// Défaut de `DUMBMONIT_WEBSITE_CHANGE_CHECK_MINUTES` : six heures.
pub const DEFAULT_CHANGE_CHECK_MINUTES: u32 = 360;
/// Une semaine.
pub const MAX_CHANGE_CHECK_MINUTES: u32 = 10_080;
/// Défaut de `DUMBMONIT_WEBSITE_HOST_SPACING_SECS`.
pub const DEFAULT_HOST_SPACING_SECS: u32 = 2;
pub const MAX_HOST_SPACING_SECS: u32 = 3600;

#[derive(Debug, Clone)]
pub struct Options {
    pub url: Url,
    /// Texte qui doit figurer dans le texte visible de la page finale.
    pub expect_text: Option<String>,
    /// En deçà, la page est tenue pour blanche.
    pub min_text_chars: usize,
    pub max_redirects: usize,
    pub user_agent: Option<String>,
    /// Catégorie posée à la main (texte libre).
    pub category: Option<String>,
    /// Les paramètres d'URL (`?utm_source=…`) ne comptent pas dans les
    /// comparaisons d'adresses (adresse finale, canonique).
    pub ignore_query: bool,
    pub allow_private: bool,
    /// Période de la comparaison des métadonnées ; `None` : celle de
    /// l'instance. Zéro : à chaque sonde.
    pub change_check: Option<Duration>,
    /// Écart minimal avant la requête suivante vers le même domaine ; `None` :
    /// celui de l'instance.
    pub host_spacing: Option<Duration>,
    /// Lectures de suite qui confirment un changement de métadonnée.
    pub confirm_checks: u32,
    /// Chute du texte visible (% de la référence) à signaler ; 0 : coupé.
    pub text_drop_percent: u32,
    /// Champs de métadonnées comparés (les autres sont ignorés).
    pub watched: BTreeSet<&'static str>,
    /// Empreinte de ce qui change la comparaison elle-même (adresse,
    /// `ignore_query`) : quand elle change, la référence repart de zéro au lieu
    /// de signaler un changement.
    pub fingerprint: String,
}

impl Options {
    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let url = parse_url(&target.address)?;
        let ignore_query = parse_bool(target, OPTION_IGNORE_QUERY, DEFAULT_IGNORE_QUERY)?;
        let mut watched = BTreeSet::new();
        for (option, fields) in super::changes::WATCH_GROUPS {
            if parse_bool(target, option, true)? {
                watched.extend(fields.iter().copied());
            }
        }
        let mut hasher = Sha256::new();
        hasher.update(url.as_str().as_bytes());
        hasher.update([0, u8::from(ignore_query)]);
        Ok(Self {
            expect_text: tag(target, OPTION_EXPECT_TEXT).map(str::to_string),
            min_text_chars: parse_u32(
                target,
                OPTION_MIN_TEXT_CHARS,
                DEFAULT_MIN_TEXT_CHARS,
                0..=MAX_MIN_TEXT_CHARS,
            )? as usize,
            max_redirects: parse_u32(
                target,
                OPTION_MAX_REDIRECTS,
                DEFAULT_MAX_REDIRECTS,
                0..=MAX_MAX_REDIRECTS,
            )? as usize,
            user_agent: tag(target, OPTION_USER_AGENT).map(str::to_string),
            category: tag(target, OPTION_CATEGORY).map(str::to_string),
            ignore_query,
            allow_private: parse_bool(target, OPTION_ALLOW_PRIVATE, false)?,
            change_check: change_check(target)?,
            host_spacing: host_spacing(target)?,
            confirm_checks: parse_u32(
                target,
                OPTION_CONFIRM_CHECKS,
                DEFAULT_CONFIRM_CHECKS,
                1..=MAX_CONFIRM_CHECKS,
            )?,
            text_drop_percent: parse_u32(
                target,
                OPTION_TEXT_DROP,
                DEFAULT_TEXT_DROP_PERCENT,
                0..=MAX_TEXT_DROP_PERCENT,
            )?,
            watched,
            fingerprint: hex::encode(hasher.finalize()),
            url,
        })
    }
}

/// L'option `change_check_minutes`, si elle est posée.
pub fn change_check(target: &Target) -> Result<Option<Duration>, ProbeError> {
    let minutes = parse_opt_u32(target, OPTION_CHANGE_CHECK, 0..=MAX_CHANGE_CHECK_MINUTES)?;
    Ok(minutes.map(|m| Duration::from_secs(u64::from(m) * 60)))
}

/// L'option `host_spacing_seconds`, si elle est posée. Le planificateur la lit
/// avant la sonde, sans s'arrêter à une valeur fausse : c'est la sonde qui la
/// signale.
pub fn host_spacing(target: &Target) -> Result<Option<Duration>, ProbeError> {
    let seconds = parse_opt_u32(target, OPTION_HOST_SPACING, 0..=MAX_HOST_SPACING_SECS)?;
    Ok(seconds.map(|s| Duration::from_secs(u64::from(s))))
}

/// Adresse d'un site : `https://` est supposé quand le schéma manque.
pub fn parse_url(address: &str) -> Result<Url, ProbeError> {
    let address = address.trim();
    let with_scheme =
        if address.contains("://") { address.to_string() } else { format!("https://{address}") };
    let mut url = Url::parse(&with_scheme).map_err(|error| {
        ProbeError::Config(format!("\"{address}\" is not a valid web address: {error}"))
    })?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(ProbeError::Config(format!(
            "only http:// and https:// addresses can be watched, got \"{address}\""
        )));
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(ProbeError::Config(format!("\"{address}\" has no host name")));
    }
    url.set_fragment(None);
    Ok(url)
}

fn tag<'a>(target: &'a Target, key: &str) -> Option<&'a str> {
    target.tags.get(key).map(String::as_str).map(str::trim).filter(|v| !v.is_empty())
}

fn parse_bool(target: &Target, key: &str, default: bool) -> Result<bool, ProbeError> {
    match tag(target, key).map(str::to_ascii_lowercase).as_deref() {
        None => Ok(default),
        Some("true" | "1" | "yes" | "on") => Ok(true),
        Some("false" | "0" | "no" | "off") => Ok(false),
        Some(other) => Err(ProbeError::Config(format!(
            "\"{key}\" expects a boolean (true/false), got \"{other}\""
        ))),
    }
}

fn parse_u32(
    target: &Target,
    key: &str,
    default: u32,
    range: std::ops::RangeInclusive<u32>,
) -> Result<u32, ProbeError> {
    Ok(parse_opt_u32(target, key, range)?.unwrap_or(default))
}

fn parse_opt_u32(
    target: &Target,
    key: &str,
    range: std::ops::RangeInclusive<u32>,
) -> Result<Option<u32>, ProbeError> {
    let Some(raw) = tag(target, key) else { return Ok(None) };
    let value: u32 = raw
        .parse()
        .map_err(|_| ProbeError::Config(format!("\"{key}\" expects an integer, got \"{raw}\"")))?;
    if !range.contains(&value) {
        return Err(ProbeError::Config(format!(
            "\"{key}\" must be between {} and {}, got {value}",
            range.start(),
            range.end()
        )));
    }
    Ok(Some(value))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn target(address: &str, tags: &[(&str, &str)]) -> Target {
        Target {
            id: 1,
            name: "site".into(),
            address: address.into(),
            kind: super::super::KIND.into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: tags.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            credential: dumbmonit_proto::Credential::None,
            group_name: String::new(),
            position: 0,
        }
    }

    #[test]
    fn les_defauts() {
        let options = Options::from_target(&target("example.com", &[])).unwrap();
        assert_eq!(options.url.as_str(), "https://example.com/");
        assert_eq!(options.min_text_chars, 20);
        assert_eq!(options.max_redirects, 10);
        assert!(options.ignore_query);
        assert!(options.expect_text.is_none() && options.user_agent.is_none());
        assert!(!options.allow_private);
        assert!(options.change_check.is_none() && options.host_spacing.is_none());
        assert_eq!((options.confirm_checks, options.text_drop_percent), (2, 80));
        assert_eq!(options.watched.len(), super::super::changes::FIELDS.len(), "tout est suivi");
    }

    #[test]
    fn controles_activables_un_par_un() {
        let options = Options::from_target(&target(
            "ex.fr",
            &[
                ("watch_title", "off"),
                ("watch_open_graph", "false"),
                ("watch_hreflang", "0"),
                ("confirm_checks", "4"),
                ("text_drop_percent", "0"),
            ],
        ))
        .unwrap();
        for field in ["title", "og_title", "og_description", "hreflang", "hreflang_count"] {
            assert!(!options.watched.contains(field), "{field}");
        }
        assert!(options.watched.contains("canonical") && options.watched.contains("final_url"));
        assert_eq!((options.confirm_checks, options.text_drop_percent), (4, 0));
        for (key, value) in [
            ("confirm_checks", "0"),
            ("confirm_checks", "11"),
            ("text_drop_percent", "100"),
            ("watch_title", "peut-être"),
        ] {
            assert!(
                matches!(
                    Options::from_target(&target("ex.fr", &[(key, value)])),
                    Err(ProbeError::Config(_))
                ),
                "{key}={value}"
            );
        }
    }

    #[test]
    fn cadence_des_changements_et_ecart_par_domaine() {
        let options = Options::from_target(&target(
            "ex.fr",
            &[("change_check_minutes", "0"), ("host_spacing_seconds", "15")],
        ))
        .unwrap();
        assert_eq!(options.change_check, Some(Duration::ZERO), "zéro : à chaque sonde");
        assert_eq!(options.host_spacing, Some(Duration::from_secs(15)));
        let options =
            Options::from_target(&target("ex.fr", &[("change_check_minutes", "1440")])).unwrap();
        assert_eq!(options.change_check, Some(Duration::from_secs(86_400)));
        for (key, value) in [
            ("change_check_minutes", "10081"),
            ("change_check_minutes", "-1"),
            ("host_spacing_seconds", "3601"),
            ("host_spacing_seconds", "deux"),
        ] {
            assert!(
                matches!(
                    Options::from_target(&target("ex.fr", &[(key, value)])),
                    Err(ProbeError::Config(_))
                ),
                "{key}={value}"
            );
        }
    }

    #[test]
    fn options_typees_et_erreurs() {
        let options = Options::from_target(&target(
            "http://ex.fr/page#ancre",
            &[
                ("expect_text", "Bienvenue"),
                ("min_text_chars", "50"),
                ("max_redirects", "3"),
                ("ignore_query", "false"),
                ("user_agent", "Mozilla/5.0 test"),
            ],
        ))
        .unwrap();
        assert_eq!(options.url.as_str(), "http://ex.fr/page");
        assert_eq!(options.expect_text.as_deref(), Some("Bienvenue"));
        assert_eq!((options.min_text_chars, options.max_redirects), (50, 3));
        assert!(!options.ignore_query);
        assert_eq!(options.user_agent.as_deref(), Some("Mozilla/5.0 test"));

        let base = Options::from_target(&target("http://ex.fr/page", &[])).unwrap();
        assert_ne!(base.fingerprint, options.fingerprint, "ignore_query change la référence");

        for (key, value) in
            [("max_redirects", "99"), ("min_text_chars", "abc"), ("ignore_query", "peut-être")]
        {
            assert!(matches!(
                Options::from_target(&target("ex.fr", &[(key, value)])),
                Err(ProbeError::Config(_))
            ));
        }
        assert!(Options::from_target(&target("ftp://ex.fr", &[])).is_err());
    }
}
