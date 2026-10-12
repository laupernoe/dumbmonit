//! Collecteur « website » : la santé et le référencement d'un site, en masse.
//!
//! Une cible par site (son adresse d'accueil, ou toute page à garder à
//! l'œil). Chaque sonde lit la page une fois, redirections suivies
//! ([`fetch`]), et en tire :
//!
//! - la réponse : statut final, temps total, chaîne de redirections. Une
//!   redirection n'est jamais une erreur en soi (`http` → `https`, domaine nu
//!   → `www.`) ; une boucle ou trop d'étapes en sont une, rapportée comme
//!   « hors service » avec la chaîne dans le message, comme un site qui ne
//!   répond pas ou un certificat refusé ;
//! - la page : taille du texte visible (une page blanche, c'est un thème
//!   cassé, une base de données en panne derrière un 200), texte attendu ;
//! - le référencement ([`seo`]) : titre, description, canonique, consignes
//!   `noindex` / `nofollow` (balise ou en-tête `X-Robots-Tag`), Open Graph,
//!   langue, `h1`, `hreflang` ; et, aux sondes de changement (au plus une
//!   fois l'heure), `robots.txt` (tout interdire à Google ?) et le plan de
//!   site ;
//! - la plateforme ([`platform`]) : WordPress, Shopify, Next.js…
//!
//! # Changements
//!
//! Les métadonnées et l'adresse finale sont comparées à une référence
//! confirmée ([`changes`]) : une valeur nouvelle n'est un changement que lue
//! deux sondes de suite (pas de fausse alerte sur un test A/B ou un cache qui
//! hésite).
//!
//! Cette comparaison, et la relecture de `robots.txt` et du plan de site qui
//! l'accompagne, n'a lieu qu'aux sondes de changement : toutes les
//! `change_check_minutes` (six heures par défaut,
//! `DUMBMONIT_WEBSITE_CHANGE_CHECK_MINUTES` pour l'instance), alors que la
//! disponibilité est lue à chaque sonde. Une valeur mise en attente est
//! revue dès la sonde suivante, pas six heures plus tard : la règle des deux
//! lectures de suite garde son sens, et un changement est signalé au plus
//! une période et une sonde après être apparu.
//!
//! Un changement confirmé est gardé dans l'historique, et sa jauge
//! (`website_title_changed`…) vaut 1 pendant 24 heures, ce qui laisse aux
//! règles le temps de le voir quelle que soit leur cadence.
//!
//! Changer l'adresse ou `ignore_query` refait une référence plutôt que de
//! signaler un changement.
//!
//! # Métriques produites
//!
//! | Métrique | Sens |
//! |---|---|
//! | `website_up` | 1 si la réponse finale est un succès (2xx/3xx), 0 sinon. |
//! | `website_status_code` | Statut HTTP de la réponse finale. |
//! | `website_response_seconds` | Durée totale, redirections comprises. |
//! | `website_redirect_hops` | Redirections traversées. |
//! | `website_info{platform}` | Toujours 1 ; l'étiquette porte la plateforme. |
//! | `website_text_chars` | Caractères de texte visible (réponse 2xx). |
//! | `website_blank` | 1 si ce texte est sous `min_text_chars`. |
//! | `website_expect_text_missing` | 1 si `expect_text` manque (option posée). |
//! | `website_noindex` / `website_nofollow` | Consignes d'indexation. |
//! | `website_h1_count` / `website_hreflang_count` | Titres `h1`, variantes de langue. |
//! | `website_{title,description,canonical,final_url}_changed` | 1 pendant 24 h après un changement confirmé. |
//! | `website_robots_blocks_all` | 1 si `robots.txt` interdit tout à Google. |
//! | `website_sitemap_ok` | 1 si un plan de site se lit. |
//!
//! Le type ne tourne que sur le serveur : il écrit en base, ce qu'un agent
//! relais ne peut pas faire (`api/targets.rs` refuse le relais).

pub mod changes;
pub mod fetch;
pub mod options;
pub mod platform;
pub mod seo;
pub mod store;
pub mod throttle;

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, MetricKind, ProbeError, Sample, Target};
use reqwest::Url;
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tracing::warn;

use crate::webchange::{html, robots::Robots, sitemap};
use changes::Tracker;
use fetch::{Clients, Hop};
use options::Options;
pub use platform::{Identity, Platform, detect_platform, identify};
use seo::Metadata;

/// Type de cible, tel qu'enregistré dans `Target::kind`.
pub const KIND: &str = "website";

/// Durée pendant laquelle la jauge d'un changement confirmé vaut 1.
pub const CHANGE_WINDOW_SECS: i64 = 24 * 3600;
/// `robots.txt` et le plan de site sont relus aux sondes de changement, et au
/// plus toutes les heures.
pub const ROBOTS_REFRESH_SECS: i64 = 3600;
/// Corps lu au plus pour `robots.txt` et le plan de site.
const MAX_AUX_BYTES: usize = 10 * 1024 * 1024;
/// Redirections suivies au plus pour `robots.txt` et le plan de site.
const AUX_REDIRECTS: usize = 5;

/// Champs dont un changement confirmé a sa jauge.
const CHANGE_GAUGES: &[(&str, &str)] = &[
    (changes::TITLE, "website_title_changed"),
    (changes::DESCRIPTION, "website_description_changed"),
    (changes::CANONICAL, "website_canonical_changed"),
    (changes::FINAL_URL, "website_final_url_changed"),
];

/// Le résultat d'une lecture, tel que gardé pour l'API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastProbe {
    pub requested_url: String,
    pub final_url: String,
    pub status: u16,
    pub response_ms: u64,
    /// Redirections traversées, dans l'ordre (la dernière mène à
    /// `final_url`).
    pub redirects: Vec<Hop>,
    pub platform: Platform,
    pub content_type: String,
    /// Absents si la réponse finale n'est pas un succès 2xx.
    pub text_chars: Option<usize>,
    pub blank: Option<bool>,
    /// Absent sans `expect_text`.
    pub expect_text_found: Option<bool>,
    pub metadata: Option<Metadata>,
}

pub struct WebsiteCollector {
    pool: SqlitePool,
    clients: Clients,
    /// Période de comparaison des métadonnées pour un site sans option
    /// `change_check_minutes`.
    change_check: Duration,
}

impl WebsiteCollector {
    pub fn new(pool: SqlitePool) -> Self {
        let change_check =
            Duration::from_secs(u64::from(options::DEFAULT_CHANGE_CHECK_MINUTES) * 60);
        Self { pool, clients: Clients::default(), change_check }
    }

    /// Période d'instance (`DUMBMONIT_WEBSITE_CHANGE_CHECK_MINUTES`).
    pub fn with_change_check(mut self, period: Duration) -> Self {
        self.change_check = period;
        self
    }
}

/// La comparaison des métadonnées est-elle due à cette sonde ?
///
/// Oui sans référence, avec une valeur en attente (la confirmer ou l'écarter
/// ne doit pas attendre une période entière), jamais faite, ou faite il y a
/// au moins `period`.
pub fn change_check_due(tracker: &Tracker, check_age: Option<i64>, period: Duration) -> bool {
    tracker.confirmed.is_empty()
        || !tracker.pending.is_empty()
        || check_age.is_none_or(|age| age < 0 || age as u64 >= period.as_secs())
}

#[async_trait]
impl Collector for WebsiteCollector {
    fn kind(&self) -> &str {
        KIND
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = Options::from_target(target)?;
        let client = self.clients.get(options.allow_private, options.user_agent.as_deref())?;
        let followed =
            match fetch::fetch(&client, &options.url, options.max_redirects, options.allow_private)
                .await
            {
                Ok(followed) => followed,
                Err(error) => {
                    if let Err(e) = store::save_error(&self.pool, target.id, &error.message()).await
                    {
                        warn!(target = target.id, error = ?e, "website error not recorded");
                    }
                    return Err(error.into());
                }
            };
        let reading = read(&options, &followed);

        let pool = &self.pool;
        let mut state = store::state(pool, target.id).await.map_err(ProbeError::Other)?;
        if state.as_ref().is_some_and(|s| s.fingerprint != options.fingerprint) {
            store::reset(pool, target.id).await.map_err(ProbeError::Other)?;
            state = None;
        }
        let (mut tracker, baseline) = match &state {
            Some(state) => (state.tracker(), false),
            None => (Tracker::default(), true),
        };
        let period = options.change_check.unwrap_or(self.change_check);
        let check_age = state.as_ref().and_then(|s| s.check_age);
        let check_due = change_check_due(&tracker, check_age, period);
        // La comparaison n'est faite que sur une page lue : une erreur ou une
        // réponse qui n'est pas du HTML la reporte à la sonde suivante.
        let mut checked = false;
        let confirmed = match &reading.probe.metadata {
            Some(metadata) if check_due => {
                checked = true;
                let current = changes::values(&followed.final_url, metadata);
                if baseline || tracker.confirmed.is_empty() {
                    tracker = Tracker::baseline(current);
                    Vec::new()
                } else {
                    tracker.observe(&current, options.ignore_query)
                }
            }
            _ => Vec::new(),
        };

        // `robots.txt` et plan de site, sur l'origine finale : aux sondes de
        // changement, au plus une fois l'heure, et dès qu'ils manquent.
        let robots_due = reading.probe.status < 500
            && state.as_ref().is_none_or(|s| {
                s.robots_blocks_all.is_none()
                    || (check_due && s.robots_age.is_none_or(|age| age >= ROBOTS_REFRESH_SECS))
            });
        let robots = if robots_due {
            Some(check_robots(&client, &followed.final_url, options.allow_private).await)
        } else {
            None
        };

        let last_probe = serde_json::to_string(&reading.probe)
            .map_err(|e| ProbeError::Other(anyhow::anyhow!(e)))?;
        store::save(
            pool,
            target.id,
            store::Saved {
                fingerprint: &options.fingerprint,
                tracker: &tracker,
                changes: &confirmed,
                last_probe: &last_probe,
                robots,
                checked,
            },
        )
        .await
        .map_err(ProbeError::Other)?;

        let state = store::state(pool, target.id).await.map_err(ProbeError::Other)?;
        let ages = state.as_ref().map(|s| change_ages(&s.changed_at())).unwrap_or_default();
        let robots = state.and_then(|s| Some((s.robots_blocks_all?, s.sitemap_ok?)));
        Ok(samples(&reading.probe, &ages, robots, chrono::Utc::now().timestamp_millis()))
    }
}

/// Une lecture interprétée.
struct Reading {
    probe: LastProbe,
}

/// Interprète la page lue : texte, métadonnées, plateforme.
fn read(options: &Options, followed: &fetch::Followed) -> Reading {
    let answer = &followed.answer;
    let content_type = answer
        .headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let essence = content_type.split(';').next().unwrap_or_default().trim().to_ascii_lowercase();
    let is_html =
        essence.is_empty() || essence == "text/html" || essence == "application/xhtml+xml";
    let success = (200..300).contains(&answer.status);

    let mut probe = LastProbe {
        requested_url: options.url.to_string(),
        final_url: followed.final_url.to_string(),
        status: answer.status,
        response_ms: followed.elapsed.as_millis() as u64,
        redirects: followed.chain.clone(),
        platform: detect_platform(&followed.final_url, &answer.headers, &answer.body),
        content_type,
        text_chars: None,
        blank: None,
        expect_text_found: None,
        metadata: None,
    };
    if !success {
        return Reading { probe };
    }
    let lines: Vec<String> = if is_html {
        html::parse(&answer.body).lines
    } else {
        answer.body.lines().map(html::collapse).filter(|l| !l.is_empty()).collect()
    };
    let text = lines.join(" ");
    let chars = text.chars().count();
    probe.text_chars = Some(chars);
    probe.blank = Some(chars < options.min_text_chars);
    probe.expect_text_found = options
        .expect_text
        .as_deref()
        .map(|expected| text.to_lowercase().contains(&html::collapse(expected).to_lowercase()));
    if is_html {
        probe.metadata = Some(seo::extract(&followed.final_url, &answer.headers, &answer.body));
    }
    Reading { probe }
}

/// Lit `robots.txt` et le plan de site de l'origine de `final_url` : rend
/// (`robots.txt` interdit tout à Google, un plan de site se lit).
async fn check_robots(
    client: &reqwest::Client,
    final_url: &Url,
    allow_private: bool,
) -> (bool, bool) {
    let get = |url: Url| async move {
        let followed = fetch::follow(url, AUX_REDIRECTS, |url| async move {
            fetch::get(client, &url, allow_private, MAX_AUX_BYTES).await
        })
        .await
        .ok()?;
        (followed.answer.status == 200).then_some((followed.final_url, followed.answer.body))
    };
    let robots = match final_url.join("/robots.txt") {
        Ok(url) => get(url).await.map(|(_, body)| Robots::parse_for(&body, "googlebot")),
        Err(_) => None,
    };
    let blocks_all = robots.as_ref().is_some_and(|robots| !robots.allows("/"));

    let mut candidates: Vec<Url> = robots
        .iter()
        .flat_map(|r| r.sitemaps.iter())
        .filter_map(|raw| Url::parse(raw).ok())
        .filter(|url| matches!(url.scheme(), "http" | "https"))
        .take(1)
        .collect();
    if candidates.is_empty() {
        candidates.extend(final_url.join("/sitemap.xml").ok());
    }
    let mut sitemap_ok = false;
    for candidate in candidates {
        if let Some((url, body)) = get(candidate).await {
            sitemap_ok = sitemap::parse(&body).is_some()
                || body.contains("<urlset")
                || body.contains("<sitemapindex")
                || url.path().ends_with(".gz");
        }
    }
    (blocks_all, sitemap_ok)
}

/// Âge en secondes du dernier changement confirmé de chaque champ.
pub fn change_ages(changed_at: &BTreeMap<String, String>) -> BTreeMap<String, i64> {
    let now = chrono::Utc::now().naive_utc();
    changed_at
        .iter()
        .filter_map(|(field, at)| {
            let at = chrono::NaiveDateTime::parse_from_str(at, "%Y-%m-%d %H:%M:%S").ok()?;
            Some((field.clone(), (now - at).num_seconds()))
        })
        .collect()
}

/// Les mesures d'une lecture.
fn samples(
    probe: &LastProbe,
    ages: &BTreeMap<String, i64>,
    robots: Option<(bool, bool)>,
    now: i64,
) -> Vec<Sample> {
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, now);
    let flag = |value: bool| if value { 1.0 } else { 0.0 };
    let mut samples = vec![
        gauge("website_up", flag((200..400).contains(&probe.status))),
        gauge("website_status_code", f64::from(probe.status)),
        gauge("website_response_seconds", probe.response_ms as f64 / 1000.0),
        gauge("website_redirect_hops", probe.redirects.len() as f64),
        gauge("website_info", 1.0).with_label("platform", probe.platform.as_str()),
    ];
    if let (Some(chars), Some(blank)) = (probe.text_chars, probe.blank) {
        samples.push(gauge("website_text_chars", chars as f64));
        samples.push(gauge("website_blank", flag(blank)));
    }
    if let Some(found) = probe.expect_text_found {
        samples.push(gauge("website_expect_text_missing", flag(!found)));
    }
    if let Some(meta) = &probe.metadata {
        samples.push(gauge("website_noindex", flag(meta.noindex)));
        samples.push(gauge("website_nofollow", flag(meta.nofollow)));
        samples.push(gauge("website_h1_count", meta.h1_count as f64));
        samples.push(gauge("website_hreflang_count", meta.hreflang_count as f64));
    }
    for (field, metric) in CHANGE_GAUGES {
        let recent = ages.get(*field).is_some_and(|age| *age < CHANGE_WINDOW_SECS);
        samples.push(gauge(metric, flag(recent)));
    }
    if let Some((blocks_all, sitemap_ok)) = robots {
        samples.push(gauge("website_robots_blocks_all", flag(blocks_all)));
        samples.push(gauge("website_sitemap_ok", flag(sitemap_ok)));
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::*;
    use options::tests::target;
    use reqwest::header::{HeaderMap, HeaderValue};

    fn followed(status: u16, content_type: &str, body: &str) -> fetch::Followed {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_str(content_type).unwrap());
        fetch::Followed {
            chain: vec![Hop { url: "http://ex.fr/".into(), status: 301 }],
            final_url: Url::parse("https://www.ex.fr/").unwrap(),
            answer: fetch::Answer { status, headers, body: body.into() },
            elapsed: Duration::from_millis(1500),
        }
    }

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    #[test]
    fn une_petite_page_legitime_nest_pas_blanche() {
        let options = Options::from_target(&target("ex.fr", &[])).unwrap();
        let petite = format!(
            "<html><body><h1>Example Domain</h1><p>{}</p></body></html>",
            "mot ".repeat(35)
        );
        let reading = read(&options, &followed(200, "text/html", &petite));
        assert_eq!(reading.probe.blank, Some(false));
        let vide = read(&options, &followed(200, "text/html", "<html><body></body></html>"));
        assert_eq!(vide.probe.blank, Some(true));
    }

    #[test]
    fn une_page_blanche_est_reconnue() {
        let options = Options::from_target(&target("ex.fr", &[])).unwrap();
        let blank = "<html><head><title>Boutique</title><script>app()</script></head>\
                     <body><div id=root></div></body></html>";
        let reading = read(&options, &followed(200, "text/html", blank));
        assert_eq!(reading.probe.blank, Some(true));
        assert_eq!(reading.probe.text_chars, Some(0), "ni le titre ni le script ne comptent");
        let samples = samples(&reading.probe, &BTreeMap::new(), None, 0);
        assert_eq!(value(&samples, "website_blank"), Some(1.0));
        assert_eq!(value(&samples, "website_up"), Some(1.0), "blanche, mais elle répond");
        assert_eq!(value(&samples, "website_redirect_hops"), Some(1.0));
        assert_eq!(value(&samples, "website_response_seconds"), Some(1.5));

        let full = format!("<html><body><p>{}</p></body></html>", "mot ".repeat(100));
        let reading = read(&options, &followed(200, "text/html; charset=utf-8", &full));
        assert_eq!(reading.probe.blank, Some(false));
    }

    #[test]
    fn texte_attendu() {
        let options =
            Options::from_target(&target("ex.fr", &[("expect_text", "  Ajouter  au panier ")]))
                .unwrap();
        let page = "<html><body><button>AJOUTER au\n panier</button></body></html>";
        let found = read(&options, &followed(200, "text/html", page));
        assert_eq!(found.probe.expect_text_found, Some(true));
        let missing = read(&options, &followed(200, "text/html", "<p>Maintenance</p>"));
        assert_eq!(missing.probe.expect_text_found, Some(false));
        let samples = samples(&missing.probe, &BTreeMap::new(), None, 0);
        assert_eq!(value(&samples, "website_expect_text_missing"), Some(1.0));
    }

    #[test]
    fn une_erreur_serveur_n_a_ni_texte_ni_metadonnees() {
        let options = Options::from_target(&target("ex.fr", &[])).unwrap();
        let reading = read(&options, &followed(503, "text/html", "<title>Erreur</title>"));
        assert!(reading.probe.metadata.is_none() && reading.probe.blank.is_none());
        let samples = samples(&reading.probe, &BTreeMap::new(), None, 0);
        assert_eq!(value(&samples, "website_up"), Some(0.0));
        assert_eq!(value(&samples, "website_status_code"), Some(503.0));
        assert_eq!(value(&samples, "website_blank"), None);
        assert_eq!(value(&samples, "website_noindex"), None);
    }

    #[test]
    fn jauges_de_changement_et_plateforme() {
        let options = Options::from_target(&target("ex.fr", &[])).unwrap();
        let page = r#"<html><head><meta name="generator" content="WordPress 6.6"></head></html>"#;
        let reading = read(&options, &followed(200, "text/html", page));
        let ages: BTreeMap<String, i64> =
            [("title".to_string(), 60), ("canonical".to_string(), CHANGE_WINDOW_SECS + 1)].into();
        let samples = samples(&reading.probe, &ages, Some((true, false)), 0);
        assert_eq!(value(&samples, "website_title_changed"), Some(1.0));
        assert_eq!(value(&samples, "website_canonical_changed"), Some(0.0), "fenêtre passée");
        assert_eq!(value(&samples, "website_description_changed"), Some(0.0));
        assert_eq!(value(&samples, "website_robots_blocks_all"), Some(1.0));
        assert_eq!(value(&samples, "website_sitemap_ok"), Some(0.0));
        let info = samples.iter().find(|s| s.metric == "website_info").unwrap();
        assert_eq!(info.labels.get("platform").map(String::as_str), Some("wordpress"));
    }

    #[test]
    fn cadence_de_la_comparaison() {
        let six_heures = Duration::from_secs(6 * 3600);
        let reference = Tracker::baseline([("title".to_string(), "A".to_string())].into());
        assert!(change_check_due(&Tracker::default(), Some(10), six_heures), "sans référence");
        assert!(change_check_due(&reference, None, six_heures), "jamais comparée");
        assert!(!change_check_due(&reference, Some(3600), six_heures), "une heure : pas encore");
        assert!(change_check_due(&reference, Some(6 * 3600), six_heures));
        assert!(change_check_due(&reference, Some(-5), six_heures), "horloge reculée");
        assert!(change_check_due(&reference, Some(0), Duration::ZERO), "zéro : à chaque sonde");
        // Une valeur en attente est revue dès la sonde suivante.
        let mut pending = reference.clone();
        pending.pending.insert("title".into(), "B".into());
        assert!(change_check_due(&pending, Some(60), six_heures));
    }

    #[test]
    fn ages_des_changements() {
        let now = chrono::Utc::now().naive_utc() - chrono::Duration::seconds(90);
        let changed: BTreeMap<String, String> = [
            ("title".to_string(), now.format("%Y-%m-%d %H:%M:%S").to_string()),
            ("lang".to_string(), "illisible".to_string()),
        ]
        .into();
        let ages = change_ages(&changed);
        assert!((89..=95).contains(&ages["title"]));
        assert!(!ages.contains_key("lang"));
    }
}
