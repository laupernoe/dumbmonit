//! Réglages d'une surveillance, lus dans l'adresse et les étiquettes de la
//! cible. Toute erreur ici est une erreur de configuration : elle s'affiche à
//! côté de l'équipement et ne déclenche aucune alerte.

use std::time::Duration;

use dumbmonit_proto::{ProbeError, Target};
use regex::Regex;
use reqwest::Url;
use sha2::{Digest, Sha256};

use super::scope::Scope;

pub const OPTION_SCOPE: &str = "scope";
pub const OPTION_MAX_PAGES: &str = "max_pages";
pub const OPTION_PATH_PREFIX: &str = "path_prefix";
pub const OPTION_IGNORE: &str = "ignore";
pub const OPTION_SCREENSHOTS: &str = "screenshots";
pub const OPTION_CHECK_INTERVAL: &str = "check_interval_minutes";
pub const OPTION_INSECURE_TLS: &str = "insecure_tls";
pub const OPTION_ALLOW_PRIVATE: &str = dumbmonit_collectors::uptime::guard::OPTION;

pub const DEFAULT_MAX_PAGES: u32 = 25;
pub const MAX_MAX_PAGES: u32 = 200;
pub const DEFAULT_CHECK_INTERVAL_MINUTES: u32 = 60;
/// Cinq minutes au plus souvent : en deçà, on n'observe plus un site, on le
/// charge — et c'est souvent celui de quelqu'un d'autre.
pub const MIN_CHECK_INTERVAL_MINUTES: u32 = 5;
/// Une semaine au plus.
pub const MAX_CHECK_INTERVAL_MINUTES: u32 = 7 * 24 * 60;
/// Expressions d'exclusion au plus : au-delà, c'est la page qu'il faut
/// choisir autrement.
const MAX_IGNORE_PATTERNS: usize = 50;
/// Taille maximale d'une expression compilée.
const REGEX_SIZE_LIMIT: usize = 1 << 20;

/// Page seule ou site entier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Page,
    Site,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub mode: Mode,
    pub scope: Scope,
    pub max_pages: usize,
    pub ignore: Vec<Regex>,
    pub screenshots: bool,
    pub check_interval: Duration,
    pub insecure_tls: bool,
    pub allow_private: bool,
    /// Empreinte de ce qui change la comparaison elle-même : adresse,
    /// périmètre, exclusions. Quand elle change, la vérification suivante
    /// repart d'une référence neuve au lieu de tout signaler comme modifié.
    pub fingerprint: String,
}

impl Options {
    pub fn from_target(target: &Target) -> Result<Self, ProbeError> {
        let start = start_url(&target.address)?;
        let mode = match tag(target, OPTION_SCOPE).map(str::to_ascii_lowercase).as_deref() {
            None | Some("page") => Mode::Page,
            Some("site") => Mode::Site,
            Some(other) => {
                return Err(ProbeError::Config(format!(
                    "\"{OPTION_SCOPE}\" must be \"page\" or \"site\", got \"{other}\""
                )));
            }
        };
        let max_pages =
            parse_u32(target, OPTION_MAX_PAGES, DEFAULT_MAX_PAGES, 1..=MAX_MAX_PAGES)? as usize;
        let prefix = tag(target, OPTION_PATH_PREFIX);
        if let Some(prefix) = prefix
            && (prefix.contains("://") || prefix.contains(char::is_whitespace))
        {
            return Err(ProbeError::Config(format!(
                "\"{OPTION_PATH_PREFIX}\" is a path such as /docs/, got \"{prefix}\""
            )));
        }
        let scope = Scope::new(&start, prefix);
        let raw_ignore = tag(target, OPTION_IGNORE).unwrap_or_default();
        let ignore = parse_ignore(raw_ignore)?;
        let minutes = parse_u32(
            target,
            OPTION_CHECK_INTERVAL,
            DEFAULT_CHECK_INTERVAL_MINUTES,
            MIN_CHECK_INTERVAL_MINUTES..=MAX_CHECK_INTERVAL_MINUTES,
        )?;

        let mut hasher = Sha256::new();
        for part in [
            scope.start().as_str(),
            if mode == Mode::Site { "site" } else { "page" },
            scope.prefix(),
            raw_ignore.trim(),
        ] {
            hasher.update(part.as_bytes());
            hasher.update([0]);
        }
        let fingerprint = hex::encode(hasher.finalize());

        Ok(Self {
            mode,
            scope,
            max_pages: if mode == Mode::Page { 1 } else { max_pages },
            ignore,
            screenshots: parse_bool(target, OPTION_SCREENSHOTS, true)?,
            check_interval: Duration::from_secs(u64::from(minutes) * 60),
            insecure_tls: parse_bool(target, OPTION_INSECURE_TLS, false)?,
            allow_private: parse_bool(target, OPTION_ALLOW_PRIVATE, false)?,
            fingerprint,
        })
    }

    /// Lignes de texte une fois les lignes exclues retirées.
    pub fn keep_lines(&self, lines: Vec<String>) -> Vec<String> {
        if self.ignore.is_empty() {
            return lines;
        }
        lines.into_iter().filter(|line| !self.ignore.iter().any(|re| re.is_match(line))).collect()
    }
}

/// L'adresse de départ : `http(s)://…`, ou un nom nu auquel on prête `https`.
pub fn start_url(address: &str) -> Result<Url, ProbeError> {
    let address = address.trim();
    let with_scheme =
        if address.contains("://") { address.to_string() } else { format!("https://{address}") };
    let url = Url::parse(&with_scheme).map_err(|error| {
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
    Ok(super::scope::normalise(url))
}

/// Une expression par ligne ; les lignes vides sont ignorées.
pub fn parse_ignore(raw: &str) -> Result<Vec<Regex>, ProbeError> {
    let patterns: Vec<&str> = raw.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    if patterns.len() > MAX_IGNORE_PATTERNS {
        return Err(ProbeError::Config(format!(
            "\"{OPTION_IGNORE}\" holds {} expressions, {MAX_IGNORE_PATTERNS} at most",
            patterns.len()
        )));
    }
    patterns
        .into_iter()
        .map(|pattern| {
            regex::RegexBuilder::new(pattern).size_limit(REGEX_SIZE_LIMIT).build().map_err(
                |error| {
                    ProbeError::Config(format!(
                        "\"{OPTION_IGNORE}\": \"{pattern}\" is not a valid regular expression: {error}"
                    ))
                },
            )
        })
        .collect()
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
    let Some(raw) = tag(target, key) else { return Ok(default) };
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
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(address: &str, tags: &[(&str, &str)]) -> Target {
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
        }
    }

    #[test]
    fn les_defauts() {
        let options = Options::from_target(&target("ex.fr/docs/intro.html", &[])).unwrap();
        assert_eq!(options.mode, Mode::Page);
        assert_eq!(options.scope.start().as_str(), "https://ex.fr/docs/intro.html");
        assert_eq!(options.scope.prefix(), "/docs/");
        assert_eq!(options.max_pages, 1, "une page seule");
        assert!(options.screenshots);
        assert!(options.ignore.is_empty());
        assert_eq!(options.check_interval, Duration::from_secs(3600));
        assert!(!options.allow_private && !options.insecure_tls);

        let site = Options::from_target(&target("https://ex.fr/", &[("scope", "site")])).unwrap();
        assert_eq!(site.mode, Mode::Site);
        assert_eq!(site.max_pages, 25);
        assert_ne!(site.fingerprint, options.fingerprint);
    }

    #[test]
    fn les_valeurs_aberrantes_sont_des_erreurs_de_configuration() {
        for (address, tags) in [
            ("ftp://ex.fr/", &[][..]),
            ("https://", &[][..]),
            ("https://ex.fr/", &[("scope", "tout")][..]),
            ("https://ex.fr/", &[("max_pages", "0")][..]),
            ("https://ex.fr/", &[("max_pages", "201")][..]),
            ("https://ex.fr/", &[("check_interval_minutes", "1")][..]),
            ("https://ex.fr/", &[("screenshots", "peut-être")][..]),
            ("https://ex.fr/", &[("ignore", "(non fermée")][..]),
            ("https://ex.fr/", &[("path_prefix", "https://ex.fr/docs")][..]),
        ] {
            let error = Options::from_target(&target(address, tags)).unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)), "{address} {tags:?} : {error}");
        }
    }

    #[test]
    fn les_lignes_exclues_disparaissent_avant_la_comparaison() {
        let options = Options::from_target(&target(
            "https://ex.fr/",
            &[("ignore", "^Updated \\d+ minutes ago$\n\n  Visitors: \\d+|Generated at")],
        ))
        .unwrap();
        assert_eq!(options.ignore.len(), 2);
        let kept = options.keep_lines(vec![
            "Welcome".into(),
            "Updated 5 minutes ago".into(),
            "Visitors: 1234 today".into(),
            "Page Generated at 12:00".into(),
            "Updated 5 minutes ago by Bob".into(),
        ]);
        assert_eq!(kept, ["Welcome", "Updated 5 minutes ago by Bob"]);
    }

    #[test]
    fn changer_les_exclusions_change_lempreinte() {
        let a = Options::from_target(&target("https://ex.fr/", &[])).unwrap();
        let b = Options::from_target(&target("https://ex.fr/", &[("ignore", "x")])).unwrap();
        let c =
            Options::from_target(&target("https://ex.fr/", &[("screenshots", "false")])).unwrap();
        assert_ne!(a.fingerprint, b.fingerprint);
        assert_eq!(a.fingerprint, c.fingerprint, "les captures ne changent pas la comparaison");
    }
}
