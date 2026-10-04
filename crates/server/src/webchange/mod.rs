//! Collecteur « webchange » : les changements du contenu d'un site web.
//!
//! On surveille une page, ou tout un site (les pages sous un chemin), et l'on
//! veut savoir quand son *contenu* change : un tarif, des conditions, une
//! documentation, la page d'un concurrent, l'avis de maintenance d'un
//! fournisseur. Le collecteur lit le texte visible de chaque page (pas le
//! HTML, qui change à chaque déploiement), retire les lignes que l'utilisateur
//! a déclarées sans intérêt (horodatages, compteurs), et compare. Chaque texte
//! nouveau est gardé comme instantané, avec une capture d'écran si un
//! navigateur est configuré, et chaque différence devient un changement daté
//! que l'interface montre en « avant / après ».
//!
//! # Rythme
//!
//! La règle « équipement injoignable » attend un point toutes les trois
//! minutes au plus : le planificateur interroge donc la cible à sa période
//! habituelle (une minute par défaut), et `probe` ne fait que relire l'état en
//! base. La vérification elle-même — le parcours, qui peut durer plusieurs
//! minutes — tourne en tâche de fond, toutes les `check_interval_minutes`
//! (une heure par défaut), comme le collecteur `domain` ne réinterroge le
//! registre que toutes les `refresh_hours`. Une page de départ injoignable est
//! retentée à chaque passage, pour que le retour soit constaté vite.
//!
//! Deux vérifications ne se chevauchent jamais : un verrou en base
//! (`webchange_state.running_since`) est pris avant de partir.
//!
//! # Ce qui alerte
//!
//! - La page de départ injoignable (réseau, délai, 5xx) : `probe` échoue avec
//!   une erreur « hors service », la série `up` s'interrompt et la règle
//!   « Device unreachable » parle, comme pour tout équipement.
//! - Un changement : la règle livrée `webchange_detected` lit
//!   `webchange_last_check_changes`, le nombre de changements trouvés par la
//!   dernière vérification. Elle se déclenche après une vérification qui en a
//!   trouvé, et se résout à la suivante qui n'en trouve aucun.
//!
//! La première vérification sert de référence : aucun changement n'y est
//! compté. Changer l'adresse, le périmètre ou les exclusions refait une
//! référence plutôt que de signaler toutes les pages comme modifiées.
//!
//! # Métriques produites
//!
//! | Métrique | Sens |
//! |---|---|
//! | `webchange_pages` | Pages suivies (hors pages disparues). |
//! | `webchange_pages_failed` | Pages suivies dont la dernière lecture a échoué. |
//! | `webchange_last_check_changes` | Changements trouvés par la dernière vérification. |
//!
//! Absentes tant qu'aucune vérification n'a abouti.
//!
//! Le type ne tourne que sur le serveur : il écrit en base et sur le disque,
//! ce qu'un agent relais ne peut pas faire (`api/targets.rs` refuse le relais).

pub mod cdp;
pub mod crawl;
pub mod diff;
pub mod html;
pub mod options;
pub mod robots;
pub mod scope;
pub mod sitemap;
pub mod store;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use dumbmonit_proto::{Collector, MetricKind, ProbeError, Sample, Target};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tracing::{debug, info, warn};

use crate::state::AppState;
use options::{Mode, Options};

/// Type de cible, tel qu'enregistré dans `Target::kind`.
pub const KIND: &str = "webchange";

/// Durée maximale d'une vérification, captures comprises.
const CHECK_BUDGET: Duration = Duration::from_secs(20 * 60);
/// Temps consacré au plus aux captures d'une vérification. Au-delà, les
/// instantanés restants n'ont que leur texte.
const SCREENSHOT_BUDGET: Duration = Duration::from_secs(8 * 60);
/// Délai d'une capture.
const SCREENSHOT_TIMEOUT: Duration = Duration::from_secs(60);
/// Erreur d'une page qu'un parcours complet n'atteint plus.
const NO_LONGER_LINKED: &str = "No longer linked";

/// Ce dont une vérification a besoin.
#[derive(Clone)]
pub struct Context {
    pub pool: SqlitePool,
    pub data_dir: PathBuf,
    /// Navigateur des captures d'écran (`DUMBMONIT_BROWSER_URL`).
    pub browser_url: Option<String>,
}

impl Context {
    pub fn from_state(state: &AppState) -> Self {
        Self {
            pool: state.pool.clone(),
            data_dir: state.config.data_dir.clone(),
            browser_url: state.config.browser_url.clone(),
        }
    }

    /// Les captures sont-elles possibles en ce moment ?
    pub async fn screenshots_available(&self) -> bool {
        match &self.browser_url {
            Some(url) => cdp::available(url).await,
            None => false,
        }
    }
}

pub struct WebchangeCollector {
    context: Context,
}

impl WebchangeCollector {
    pub fn new(context: Context) -> Self {
        Self { context }
    }
}

#[async_trait]
impl Collector for WebchangeCollector {
    fn kind(&self) -> &str {
        KIND
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let options = Options::from_target(target)?;
        let pool = &self.context.pool;
        let state = store::state(pool, target.id).await.map_err(ProbeError::Other)?;

        let due = match &state {
            None => true,
            Some(state) => {
                state.fingerprint.as_deref() != Some(options.fingerprint.as_str())
                    || state.last_error_kind.as_deref() == Some("down")
                    || store::last_check_age(pool, target.id)
                        .await
                        .map_err(ProbeError::Other)?
                        .is_none_or(|age| age >= options.check_interval.as_secs() as i64)
            }
        };
        // Une vérification déjà en cours garde le verrou : `start_check` le
        // constate et ne fait rien.
        if due {
            match start_check(self.context.clone(), target.clone()).await {
                Ok(true) => debug!(target = target.id, "website check started"),
                Ok(false) => {}
                Err(error) => warn!(target = target.id, ?error, "website check not started"),
            }
        }

        let Some(state) = state else { return Ok(Vec::new()) };
        if let (Some(message), Some(kind)) = (state.last_error, state.last_error_kind.as_deref()) {
            return Err(match kind {
                "down" => ProbeError::Unreachable(message),
                "auth" => ProbeError::Auth(message),
                _ => ProbeError::Config(message),
            });
        }
        if state.last_check_at.is_none() {
            return Ok(Vec::new());
        }
        let (pages, failed) =
            store::page_counts(pool, target.id).await.map_err(ProbeError::Other)?;
        let now = chrono::Utc::now().timestamp_millis();
        Ok(vec![
            Sample::new("webchange_pages", pages as f64, MetricKind::Gauge, now),
            Sample::new("webchange_pages_failed", failed as f64, MetricKind::Gauge, now),
            Sample::new(
                "webchange_last_check_changes",
                state.last_check_changes as f64,
                MetricKind::Gauge,
                now,
            ),
        ])
    }
}

/// Lance une vérification en tâche de fond, sauf si une autre est en cours.
/// Vrai si elle a été lancée.
pub async fn start_check(context: Context, target: Target) -> anyhow::Result<bool> {
    if !store::try_lock(&context.pool, target.id).await? {
        return Ok(false);
    }
    tokio::spawn(async move {
        let started = Instant::now();
        match tokio::time::timeout(CHECK_BUDGET, run_check(&context, &target)).await {
            Ok(Ok(changes)) => info!(
                target = target.id,
                changes,
                elapsed = ?started.elapsed(),
                "website check done"
            ),
            Ok(Err(error)) => warn!(target = target.id, ?error, "website check failed"),
            Err(_) => {
                warn!(target = target.id, "website check abandoned after {CHECK_BUDGET:?}");
                let message = format!("the check did not finish within {CHECK_BUDGET:?}");
                let _ = store::finish_failed(&context.pool, target.id, "down", &message).await;
            }
        }
        if let Err(error) = store::unlock(&context.pool, target.id).await {
            warn!(target = target.id, ?error, "website check lock not released");
        }
    });
    Ok(true)
}

/// Sépare une erreur de sonde en sa nature (`down`, `auth`, `config`) et son
/// message, tel que `probe` le rendra.
fn classify(error: ProbeError) -> (&'static str, String) {
    match error {
        ProbeError::Timeout(after) => ("down", format!("no answer within {}s", after.as_secs())),
        ProbeError::Unreachable(message) => ("down", message),
        ProbeError::Auth(message) => ("auth", message),
        ProbeError::Protocol(message) | ProbeError::Config(message) => ("config", message),
        ProbeError::Other(error) => ("config", format!("{error:#}")),
    }
}

fn fingerprint(content: &str) -> String {
    hex::encode(Sha256::digest(content.as_bytes()))
}

/// Une vérification complète ; rend le nombre de changements trouvés.
async fn run_check(context: &Context, target: &Target) -> anyhow::Result<i64> {
    let pool = &context.pool;
    let id = target.id;
    let options = match Options::from_target(target) {
        Ok(options) => options,
        Err(error) => {
            let (kind, message) = classify(error);
            store::finish_failed(pool, id, kind, &message).await?;
            return Ok(0);
        }
    };
    let state = store::state(pool, id).await?.unwrap_or_default();
    let baseline = state.fingerprint.as_deref() != Some(options.fingerprint.as_str());

    let crawl = match crawl::crawl(&options).await {
        Ok(crawl) => crawl,
        Err(error) => {
            let (kind, message) = classify(error);
            store::finish_failed(pool, id, kind, &message).await?;
            return Ok(0);
        }
    };

    if baseline {
        // Nouvelle référence : ce que les anciens réglages suivaient et que
        // les nouveaux n'atteignent plus n'en fait plus partie.
        let urls: Vec<String> = crawl.pages.iter().map(|p| p.url.to_string()).collect();
        store::forget_pages_except(pool, id, &urls).await?;
        if state.fingerprint.is_none() {
            // Jamais de référence : d'éventuels fichiers sous ce numéro sont
            // ceux d'une cible supprimée dont le numéro a été repris.
            store::forget_files(&context.data_dir, id).await;
        }
    }

    let known: HashMap<String, store::PageRow> =
        store::pages(pool, id).await?.into_iter().map(|page| (page.url.clone(), page)).collect();
    let mut changes = 0i64;
    let mut to_capture: Vec<(i64, String)> = Vec::new();
    let mut seen = HashSet::new();

    for page in &crawl.pages {
        let url = page.url.as_str();
        seen.insert(url.to_string());
        let stored = known.get(url);
        let status = page.status.map(i64::from);

        let Some(lines) = &page.lines else {
            // Lecture en échec. Une page disparue (404/410) qu'on connaissait
            // devient un changement ; toute autre erreur reste sur la page.
            let was_live = stored.is_some_and(|s| !s.removed && s.hash.is_some());
            if page.gone && was_live && !baseline {
                let before = stored.and_then(|s| s.latest_snapshot);
                let removed = line_count(pool, before).await?;
                store::insert_change(
                    pool,
                    id,
                    &store::NewChange {
                        url,
                        kind: "removed_page",
                        added: 0,
                        removed,
                        before,
                        after: None,
                    },
                )
                .await?;
                changes += 1;
            }
            let removed = page.gone || stored.is_some_and(|s| s.removed);
            store::save_page(
                pool,
                id,
                &store::PageUpdate {
                    url,
                    title: None,
                    // Contrat avec l'interface : une page disparue s'affiche
                    // « Removed » sur un statut 404, 410 compris.
                    status: if page.gone { Some(404) } else { status },
                    error: page.error.as_deref(),
                    hash: None,
                    removed,
                    snapshot: None,
                    changed: page.gone && was_live && !baseline,
                },
            )
            .await?;
            continue;
        };

        let lines = options.keep_lines(lines.clone());
        let content = lines.join("\n");
        let hash = fingerprint(&content);
        let reappeared = stored.is_none_or(|s| s.removed);
        let same = stored.and_then(|s| s.hash.as_deref()) == Some(hash.as_str());
        if same && !reappeared {
            store::save_page(
                pool,
                id,
                &store::PageUpdate {
                    url,
                    title: page.title.as_deref(),
                    status,
                    error: None,
                    hash: None,
                    removed: false,
                    snapshot: None,
                    changed: false,
                },
            )
            .await?;
            continue;
        }

        let snapshot =
            store::insert_snapshot(pool, id, url, page.title.as_deref(), &hash, &content).await?;
        to_capture.push((snapshot, url.to_string()));
        let mut recorded = false;
        if !baseline {
            if reappeared {
                store::insert_change(
                    pool,
                    id,
                    &store::NewChange {
                        url,
                        kind: "new_page",
                        added: lines.len() as i64,
                        removed: 0,
                        before: None,
                        after: Some(snapshot),
                    },
                )
                .await?;
                recorded = true;
            } else if let Some(previous) = stored.filter(|s| s.hash.is_some()) {
                let before = previous.latest_snapshot;
                let old = match before {
                    Some(before) => {
                        store::snapshot_content(pool, before).await?.unwrap_or_default()
                    }
                    None => String::new(),
                };
                let old_lines: Vec<&str> = old.lines().collect();
                let new_lines: Vec<&str> = lines.iter().map(String::as_str).collect();
                let (added, removed) = diff::counts(&diff::diff(&old_lines, &new_lines));
                store::insert_change(
                    pool,
                    id,
                    &store::NewChange {
                        url,
                        kind: "changed",
                        added: added as i64,
                        removed: removed as i64,
                        before,
                        after: Some(snapshot),
                    },
                )
                .await?;
                recorded = true;
            }
        }
        if recorded {
            changes += 1;
        }
        store::save_page(
            pool,
            id,
            &store::PageUpdate {
                url,
                title: page.title.as_deref(),
                status,
                error: None,
                hash: Some(&hash),
                removed: false,
                snapshot: Some(snapshot),
                changed: recorded,
            },
        )
        .await?;
    }

    // Un parcours complet du site qui n'atteint plus une page connue : elle a
    // disparu (lien retiré, plan de site raccourci).
    if options.mode == Mode::Site && crawl.complete && !baseline {
        for (url, page) in known.iter().filter(|(url, page)| !page.removed && !seen.contains(*url))
        {
            if page.hash.is_some() {
                let removed = line_count(pool, page.latest_snapshot).await?;
                store::insert_change(
                    pool,
                    id,
                    &store::NewChange {
                        url,
                        kind: "removed_page",
                        added: 0,
                        removed,
                        before: page.latest_snapshot,
                        after: None,
                    },
                )
                .await?;
                changes += 1;
            }
            store::save_page(
                pool,
                id,
                &store::PageUpdate {
                    url,
                    title: None,
                    // Même contrat : 404, sans que le site l'ait répondu.
                    status: Some(404),
                    error: Some(NO_LONGER_LINKED),
                    hash: None,
                    removed: true,
                    snapshot: None,
                    changed: page.hash.is_some(),
                },
            )
            .await?;
        }
    }

    if options.screenshots
        && let Some(browser) = &context.browser_url
    {
        capture_all(context, id, browser, options.allow_private, &to_capture).await;
    }

    store::prune(pool, &context.data_dir, id).await?;
    // Une page de départ qui refuse l'accès (401, 403…) est une erreur de
    // réglage à montrer. Une page de départ disparue (404, 410), elle, est un
    // changement comme un autre : il est déjà compté, et une erreur de sonde
    // empêcherait justement la mesure qui le notifie d'être écrite.
    let start_error = crawl.pages.first().filter(|start| !start.gone).and_then(|start| {
        let status = start.status?;
        (status >= 400).then(|| {
            let kind = if matches!(status, 401 | 403) { "auth" } else { "config" };
            (kind, format!("the start page answers HTTP {status}"))
        })
    });
    store::finish_ok(
        pool,
        id,
        &options.fingerprint,
        changes,
        start_error.as_ref().map(|(kind, message)| (*kind, message.as_str())),
    )
    .await?;
    Ok(changes)
}

/// Lignes d'un instantané (0 s'il n'y en a pas).
async fn line_count(pool: &SqlitePool, snapshot: Option<i64>) -> anyhow::Result<i64> {
    let Some(snapshot) = snapshot else { return Ok(0) };
    let content = store::snapshot_content(pool, snapshot).await?.unwrap_or_default();
    Ok(content.lines().count() as i64)
}

/// Capture les pages des nouveaux instantanés, une à la fois. Le premier
/// échec arrête la série : un navigateur absent ne coûte qu'un délai, pas un
/// par page.
async fn capture_all(
    context: &Context,
    target: dumbmonit_proto::TargetId,
    browser: &str,
    allow_private: bool,
    snapshots: &[(i64, String)],
) {
    let started = Instant::now();
    for (snapshot, url) in snapshots {
        if started.elapsed() > SCREENSHOT_BUDGET {
            debug!(
                target = target,
                "screenshot budget spent, remaining snapshots keep their text only"
            );
            return;
        }
        let image = match tokio::time::timeout(
            SCREENSHOT_TIMEOUT,
            cdp::capture(browser, url, allow_private),
        )
        .await
        {
            Ok(Ok(image)) => image,
            Ok(Err(error)) => {
                warn!(target = target, url = %url, %error, "screenshot failed");
                return;
            }
            Err(_) => {
                warn!(target = target, url = %url, "screenshot timed out");
                return;
            }
        };
        let path = store::screenshot_path(&context.data_dir, target, *snapshot);
        let written = async {
            if let Some(dir) = path.parent() {
                tokio::fs::create_dir_all(dir).await?;
            }
            tokio::fs::write(&path, &image).await
        }
        .await;
        match written {
            Ok(()) => {
                if let Err(error) = store::mark_screenshot(&context.pool, *snapshot).await {
                    warn!(target = target, ?error, "screenshot saved but not recorded");
                }
            }
            Err(error) => {
                warn!(target = target, path = %path.display(), %error, "screenshot not saved");
                return;
            }
        }
    }
}
