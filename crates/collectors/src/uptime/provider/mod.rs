//! État d'un fournisseur tiers, lu sur sa page de statut publique
//! (`kind = "provider"`).
//!
//! Cloudflare, GitHub, Atlassian, AWS… publient l'état de leurs services : une
//! sonde qui lit cette page prévient avant les utilisateurs, et la page de
//! statut d'un homelab peut afficher le fournisseur comme composant externe.
//!
//! Deux formats sont lus (voir [`parse`]) :
//!
//! - le JSON d'une Atlassian Statuspage, `/api/v2/summary.json` — la plupart
//!   des pages de statut publiques (Cloudflare, GitHub, Atlassian, Discord,
//!   DigitalOcean…) ;
//! - un flux RSS ou Atom d'incidents — AWS Health
//!   (`https://status.aws.amazon.com/rss/all.rss` ou un flux par service) et le
//!   `history.rss` d'une Statuspage.
//!
//! Microsoft 365 ne publie pas d'état lisible sans compte : son état de service
//! passe par l'API Graph (`serviceAnnouncement`) avec une application déclarée
//! dans le tenant. Cette sonde ne le couvre donc pas ; tout flux public qui
//! convient au format RSS/Atom peut en revanche être donné en adresse.
//!
//! # Une réponse partagée
//!
//! Plusieurs cibles, ou plusieurs pages de statut, regardent souvent le même
//! fournisseur : la réponse est gardée en mémoire par adresse pendant
//! `refresh_seconds` et partagée. Si le fournisseur ne répond plus, la dernière
//! réponse sert encore quinze minutes avant que la sonde ne le signale.
//!
//! # Adresse et étiquettes
//!
//! Adresse : un nom de la liste ([`PRESETS`]), l'adresse d'une page Statuspage
//! (`https://www.cloudflarestatus.com`) ou celle d'un flux.
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `component` | — | Ne regarder que les composants (ou entrées de flux) dont le nom contient ce texte. |
//! | `fail_on` | `minor` | Niveau d'échec : `minor` (toute dégradation), `major`, `critical`. |
//! | `refresh_seconds` | `60` | Durée de réutilisation de la réponse (30 à 3600). |
//! | `window_hours` | `24` | Flux seulement : âge au-delà duquel une entrée ne compte plus (1 à 168). |
//! | `allow_private_targets` | `false` | Voir `guard`. |
//! | `timeout_seconds` | `5` | Budget de la sonde (1 à 60). |
//!
//! Métriques, en plus des métriques communes de [`outcome`](super::outcome) :
//! `probe_provider_impact` (0 à 3), `probe_provider_incidents`,
//! `probe_provider_components_affected`.

mod parse;

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, OnceLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::{TimeDelta, Utc};
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use reqwest::redirect::Policy;
use reqwest::{Client, Url};

use super::guard;
use super::outcome::{Failure, Report};
use super::tags;
use parse::{Impact, ReadError};

pub const DEFAULT_REFRESH_SECONDS: u32 = 60;
pub const DEFAULT_WINDOW_HOURS: u32 = 24;
/// Une réponse plus ancienne n'est plus servie à la place d'une neuve.
const STALE_LIMIT: Duration = Duration::from_secs(15 * 60);
/// Taille maximale d'une réponse lue.
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
/// Redirections suivies (`http` vers `https`, `www`).
const MAX_REDIRECTS: usize = 3;

/// Fournisseurs connus : nom court, adresse. Toutes les pages de la liste sont
/// des Atlassian Statuspage, sauf AWS (flux RSS).
pub const PRESETS: &[(&str, &str)] = &[
    ("cloudflare", "https://www.cloudflarestatus.com"),
    ("github", "https://www.githubstatus.com"),
    ("atlassian", "https://status.atlassian.com"),
    ("discord", "https://discordstatus.com"),
    ("dropbox", "https://status.dropbox.com"),
    ("npm", "https://status.npmjs.org"),
    ("digitalocean", "https://status.digitalocean.com"),
    ("twilio", "https://status.twilio.com"),
    ("zoom", "https://status.zoom.us"),
    ("vercel", "https://www.vercel-status.com"),
    ("netlify", "https://www.netlifystatus.com"),
    ("datadog", "https://status.datadoghq.com"),
    ("aws", "https://status.aws.amazon.com/rss/all.rss"),
];

#[derive(Default)]
pub struct ProviderCollector;

impl ProviderCollector {
    pub fn new() -> Self {
        Self
    }
}

/// Où et comment lire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Statuspage(Url),
    Feed(Url),
}

impl Source {
    fn url(&self) -> &Url {
        match self {
            Self::Statuspage(url) | Self::Feed(url) => url,
        }
    }
}

/// Transforme l'adresse saisie en source, et en nom court pour l'étiquette
/// `provider`. Un nom de la liste, une URL de page (on y ajoute le chemin de
/// l'API) ou une URL de flux ou de JSON (prise telle quelle).
pub fn resolve(address: &str) -> Result<(Source, String), ProbeError> {
    let invalid = || {
        ProbeError::Config(format!(
            "\"{address}\" is neither a provider name ({}) nor the address of a status page or feed",
            PRESETS.iter().map(|(name, _)| *name).collect::<Vec<_>>().join(", ")
        ))
    };
    let raw = address.trim();
    if raw.is_empty() {
        return Err(invalid());
    }
    let preset = PRESETS.iter().find(|(name, _)| name.eq_ignore_ascii_case(raw));
    let (label, text) = match preset {
        Some((name, url)) => ((*name).to_string(), (*url).to_string()),
        None if raw.contains("://") => (String::new(), raw.to_string()),
        None => (String::new(), format!("https://{raw}")),
    };
    let mut url = Url::parse(&text).map_err(|_| invalid())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(invalid());
    }
    let label =
        if label.is_empty() { url.host_str().unwrap_or_default().to_string() } else { label };
    let path = url.path().to_ascii_lowercase();
    let feed = [".rss", ".xml", ".atom"].iter().any(|suffix| path.ends_with(suffix))
        || path.ends_with("/feed")
        || path.ends_with("/rss");
    if feed {
        return Ok((Source::Feed(url), label));
    }
    if !path.ends_with(".json") {
        let base = url.path().trim_end_matches('/').to_string();
        url.set_path(&format!("{base}/api/v2/summary.json"));
    }
    url.set_query(None);
    Ok((Source::Statuspage(url), label))
}

struct Settings {
    source: Source,
    label: String,
    component: Option<String>,
    fail_on: Impact,
    refresh: Duration,
    window: TimeDelta,
    allow_private: bool,
    timeout: Duration,
}

fn settings(target: &Target) -> Result<Settings, ProbeError> {
    if !matches!(target.credential, Credential::None) {
        return Err(ProbeError::Config(
            "a provider's public status page takes no credential".to_string(),
        ));
    }
    let (source, label) = resolve(&target.address)?;
    let fail_on = match tags::tag(target, "fail_on") {
        None => Impact::Minor,
        Some(raw) => Impact::parse_level(raw).ok_or_else(|| {
            ProbeError::Config(format!(
                "\"fail_on\" expects minor, major or critical, got \"{raw}\""
            ))
        })?,
    };
    let refresh = tags::parse_u32(target, "refresh_seconds", DEFAULT_REFRESH_SECONDS, 30..=3600)?;
    let window = tags::parse_u32(target, "window_hours", DEFAULT_WINDOW_HOURS, 1..=168)?;
    Ok(Settings {
        source,
        label,
        component: tags::tag(target, "component").map(str::to_string),
        fail_on,
        refresh: Duration::from_secs(u64::from(refresh)),
        window: TimeDelta::hours(i64::from(window)),
        allow_private: guard::allowed(target)?,
        timeout: tags::parse_timeout(target, "timeout_seconds")?,
    })
}

struct Cached {
    at: Instant,
    body: Arc<String>,
}

/// Réponses par adresse, partagées entre les cibles.
static CACHE: LazyLock<Mutex<HashMap<String, Cached>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn client(allow_private: bool) -> Result<Client, ProbeError> {
    static CLIENTS: [OnceLock<Client>; 2] = [OnceLock::new(), OnceLock::new()];
    let cell = &CLIENTS[usize::from(allow_private)];
    if let Some(existing) = cell.get() {
        return Ok(existing.clone());
    }
    let built = Client::builder()
        .dns_resolver(Arc::new(guard::GuardedResolver { allow_private }))
        .redirect(Policy::limited(MAX_REDIRECTS))
        .user_agent(concat!("DumbMonit/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| ProbeError::Config(format!("HTTP client unusable: {error}")))?;
    Ok(cell.get_or_init(|| built).clone())
}

type Fetch = Result<Arc<String>, (Failure, String)>;

async fn download(settings: &Settings) -> Fetch {
    let url = settings.source.url();
    let http = client(settings.allow_private).map_err(|e| (Failure::Connect, e.to_string()))?;
    let accept = match settings.source {
        Source::Statuspage(_) => "application/json",
        Source::Feed(_) => "application/rss+xml, application/atom+xml, application/xml, text/xml",
    };
    let response = http
        .get(url.clone())
        .header(reqwest::header::ACCEPT, accept)
        .timeout(settings.timeout)
        .send()
        .await
        .map_err(|error| {
            let failure = if error.is_timeout() { Failure::Timeout } else { Failure::Connect };
            (failure, format!("{url}: {error}"))
        })?;
    if !response.status().is_success() {
        return Err((Failure::Status, format!("{url} answered {}", response.status())));
    }
    let bytes =
        response.bytes().await.map_err(|error| (Failure::Body, format!("{url}: {error}")))?;
    if bytes.len() > MAX_BODY_BYTES {
        return Err((Failure::Body, format!("{url}: answer larger than {MAX_BODY_BYTES} bytes")));
    }
    Ok(Arc::new(String::from_utf8_lossy(&bytes).into_owned()))
}

/// La réponse du fournisseur : celle du cache si elle a moins de `refresh`,
/// sinon une neuve ; à défaut la dernière connue, tant qu'elle a moins de
/// quinze minutes.
async fn fetch(settings: &Settings) -> Fetch {
    let key = settings.source.url().to_string();
    let cached = {
        let cache = CACHE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        cache.get(&key).map(|entry| (entry.at, Arc::clone(&entry.body)))
    };
    if let Some((at, body)) = &cached
        && at.elapsed() < settings.refresh
    {
        return Ok(Arc::clone(body));
    }
    match download(settings).await {
        Ok(body) => {
            let mut cache = CACHE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            cache.insert(key, Cached { at: Instant::now(), body: Arc::clone(&body) });
            Ok(body)
        }
        Err(error) => match cached {
            Some((at, body)) if at.elapsed() < STALE_LIMIT => {
                tracing::debug!(provider = %settings.label, detail = %error.1, "status page silent, cached answer");
                Ok(body)
            }
            _ => Err(error),
        },
    }
}

#[async_trait]
impl Collector for ProviderCollector {
    fn kind(&self) -> &'static str {
        "provider"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let settings = settings(target)?;
        guard::vet_url(settings.source.url(), settings.allow_private, settings.timeout).await?;
        let mut report = Report::new("provider").label("provider", settings.label.clone());

        let body = match fetch(&settings).await {
            Ok(body) => body,
            Err((failure, detail)) => {
                report.fail(failure, detail);
                return Ok(report.finish());
            }
        };
        let read = match &settings.source {
            Source::Statuspage(_) => parse::read_statuspage(&body, settings.component.as_deref()),
            Source::Feed(_) => {
                parse::read_feed(&body, settings.component.as_deref(), Utc::now(), settings.window)
            }
        };
        match read {
            Ok(reading) => {
                report.gauge("provider_impact", f64::from(reading.impact as u8));
                report.gauge("provider_incidents", f64::from(reading.incidents));
                report.gauge("provider_components_affected", f64::from(reading.affected));
                if reading.impact >= settings.fail_on {
                    report.fail(Failure::Status, reading.summary);
                }
            }
            // Un filtre qui ne retient rien est à corriger, pas une panne du fournisseur.
            Err(ReadError::NoMatch(why)) => return Err(ProbeError::Config(why)),
            Err(ReadError::Format(why)) => {
                report.fail(Failure::Body, format!("{}: {why}", settings.source.url()));
            }
        }
        Ok(report.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uptime::tags::test_support::cible;

    fn statuspage(address: &str) -> String {
        match resolve(address).unwrap().0 {
            Source::Statuspage(url) => url.to_string(),
            Source::Feed(url) => panic!("a feed: {url}"),
        }
    }

    #[test]
    fn a_provider_name_resolves_to_its_status_page() {
        assert_eq!(
            statuspage("cloudflare"),
            "https://www.cloudflarestatus.com/api/v2/summary.json"
        );
        assert_eq!(statuspage(" GitHub "), "https://www.githubstatus.com/api/v2/summary.json");
        let (source, label) = resolve("aws").unwrap();
        assert!(matches!(source, Source::Feed(_)));
        assert_eq!(label, "aws");
    }

    #[test]
    fn a_page_address_gets_the_api_path_and_a_feed_is_kept() {
        assert_eq!(
            statuspage("status.example.com"),
            "https://status.example.com/api/v2/summary.json"
        );
        assert_eq!(
            statuspage("https://status.example.com/"),
            "https://status.example.com/api/v2/summary.json"
        );
        assert_eq!(
            statuspage("http://x.example/api/v2/summary.json?foo=1"),
            "http://x.example/api/v2/summary.json"
        );
        assert!(matches!(
            resolve("https://status.example.com/history.rss").unwrap().0,
            Source::Feed(_)
        ));
        assert_eq!(resolve("https://status.example.com").unwrap().1, "status.example.com");
    }

    #[test]
    fn nonsense_is_a_configuration_error() {
        for bad in ["", "   ", "ftp://x.example/", "https://"] {
            let error = resolve(bad).unwrap_err();
            assert!(matches!(error, ProbeError::Config(_)) && !error.means_down(), "{bad}");
        }
    }

    #[test]
    fn options_are_read_and_checked() {
        let ok = settings(&cible(
            "provider",
            "cloudflare",
            &[("component", "Workers"), ("fail_on", "major"), ("refresh_seconds", "120")],
        ))
        .unwrap();
        assert_eq!(ok.component.as_deref(), Some("Workers"));
        assert_eq!(ok.fail_on, Impact::Major);
        assert_eq!(ok.refresh, Duration::from_secs(120));
        assert_eq!(ok.window, TimeDelta::hours(24));
        assert!(!ok.allow_private);

        for tags in [[("fail_on", "none")], [("refresh_seconds", "5")], [("window_hours", "0")]] {
            assert!(settings(&cible("provider", "github", &tags)).is_err(), "{tags:?}");
        }
    }

    #[test]
    fn the_kind_is_the_announced_one() {
        assert_eq!(ProviderCollector::new().kind(), "provider");
    }
}
