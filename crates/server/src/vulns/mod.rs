//! Failles connues (CVE) dans le logiciel installé sur le parc.
//!
//! # Principe
//!
//! 1. **Inventaire** ([`inventory`]) : pour chaque équipement, les logiciels
//!    et leurs versions, tirés de ce que les collecteurs ont déjà relevé
//!    (séries `*_version_info`, paquets Proxmox, inventaire quotidien de
//!    l'agent). Rien n'est redemandé aux équipements.
//! 2. **Sources** : NVD par CPE (produits du [`catalog`]), Debian Security
//!    Tracker (paquets Debian et Proxmox, rétroportages compris), OSV.dev
//!    (Ubuntu, Alpine, AlmaLinux, Rocky Linux), catalogue KEV de la CISA
//!    (failles exploitées). Tout est gardé en base ([`store`]) : hors ligne,
//!    le dernier état reste servi.
//! 3. **Verdict** ([`verdict`]) : `vulnerable`, `patched` ou `unknown` (« à
//!    vérifier ») — le doute ne déclenche jamais d'alerte.
//!
//! # Rythme
//!
//! Une tâche de fond, hors du cycle des collecteurs : les séries
//! d'alerte sont republiées toutes les quinze minutes, l'inventaire et les
//! verdicts recalculés toutes les heures, les sources relues une fois par
//! jour (ou à la demande). Coupé par défaut : rien ne part vers l'extérieur
//! tant que l'administrateur ne l'a pas activé.

pub mod catalog;
pub mod debian;
mod fetch;
pub mod inventory;
pub mod kev;
pub mod nvd;
pub mod osv;
pub mod store;
pub mod verdict;
pub mod version;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use dumbmonit_proto::{MetricKind, Sample, TargetId};
use futures::StreamExt;
use sqlx::SqlitePool;
use tokio::sync::Notify;
use tracing::{debug, info, warn};

use crate::crypto::Cipher;
use crate::state::AppState;
use crate::tsdb::InstantSeries;
use fetch::Fetcher;
use inventory::{Builder, Component, Origin, Series};
use verdict::{Reason, State, Verdict};

/// Réglage `app_settings` : `on` / `off`, absent = coupé.
pub const SETTING_KEY: &str = "cve_scan";
/// Clé d'API NVD, chiffrée (hex).
pub const NVD_KEY_SETTING: &str = "cve_nvd_api_key";

const TICK: Duration = Duration::from_secs(15 * 60);
const RECOMPUTE_EVERY: Duration = Duration::from_secs(3600);
/// Les sources sont relues une fois par jour (un peu moins, pour ne pas
/// glisser d'un tour à chaque fois).
const SOURCES_EVERY: chrono::Duration = chrono::Duration::hours(23);
/// Fenêtre de lecture des séries de version : certaines ne sont relues que
/// toutes les quelques heures.
const LOOKBACK: &str = "1d";
/// Fiches NVD lues par tour pour noter les CVE venues de Debian et d'OSV.
const ENRICH_BUDGET: usize = 250;
const ENRICH_BUDGET_WITH_KEY: usize = 2500;
/// Détails d'avis OSV lus par tour.
const OSV_DETAIL_BUDGET: usize = 4000;
/// Séries d'alerte publiées au plus.
const MAX_PUBLISHED: usize = 2000;
/// Note à partir de laquelle une faille sans correctif, ou à vérifier, d'un
/// paquet de distribution est montrée.
const NOTABLE_CVSS: f64 = 7.0;

struct Runtime {
    refreshing: AtomicBool,
    forced: AtomicBool,
    wake: Notify,
}

fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| Runtime {
        refreshing: AtomicBool::new(false),
        forced: AtomicBool::new(false),
        wake: Notify::new(),
    })
}

/// Demande un rafraîchissement complet (sources comprises) au prochain tour.
pub fn request_refresh() {
    runtime().forced.store(true, Ordering::SeqCst);
    runtime().refreshing.store(true, Ordering::SeqCst);
    runtime().wake.notify_one();
}

pub fn is_refreshing() -> bool {
    runtime().refreshing.load(Ordering::SeqCst)
}

// ----------------------------------------------------------------- réglages

pub async fn enabled(pool: &SqlitePool) -> sqlx::Result<bool> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(SETTING_KEY)
        .fetch_optional(pool)
        .await?;
    Ok(value.as_deref() == Some("on"))
}

async fn set_setting(pool: &SqlitePool, key: &str, value: Option<&str>) -> sqlx::Result<()> {
    match value {
        Some(value) => {
            sqlx::query(
                "INSERT INTO app_settings (key, value) VALUES (?, ?) \
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            )
            .bind(key)
            .bind(value)
            .execute(pool)
            .await?;
        }
        None => {
            sqlx::query("DELETE FROM app_settings WHERE key = ?").bind(key).execute(pool).await?;
        }
    }
    Ok(())
}

pub async fn set_enabled(pool: &SqlitePool, on: bool) -> sqlx::Result<()> {
    set_setting(pool, SETTING_KEY, Some(if on { "on" } else { "off" })).await
}

/// La clé d'API NVD, déchiffrée ; `None` si absente ou illisible.
pub async fn nvd_key(pool: &SqlitePool, cipher: &Cipher) -> sqlx::Result<Option<String>> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(NVD_KEY_SETTING)
        .fetch_optional(pool)
        .await?;
    Ok(value
        .and_then(|hex| hex::decode(hex).ok())
        .and_then(|bytes| cipher.decrypt(&bytes).ok())
        .and_then(|plain| String::from_utf8(plain).ok()))
}

/// Pose (ou efface, avec une chaîne vide) la clé d'API NVD.
pub async fn set_nvd_key(pool: &SqlitePool, cipher: &Cipher, key: &str) -> anyhow::Result<()> {
    let key = key.trim();
    if key.is_empty() {
        set_setting(pool, NVD_KEY_SETTING, None).await?;
    } else {
        let encrypted = hex::encode(cipher.encrypt(key.as_bytes())?);
        set_setting(pool, NVD_KEY_SETTING, Some(&encrypted)).await?;
    }
    Ok(())
}

// ------------------------------------------------------------- inventaire

fn inventory_query(names: &[&str]) -> String {
    format!(
        r#"last_over_time({{__name__=~"dumbmonit_({})"}}[{LOOKBACK}]) keep_metric_names"#,
        names.join("|")
    )
}

/// Range les séries par cible (étiquette `target`), sans préfixe ni nom.
fn group_series(series: Vec<InstantSeries>) -> BTreeMap<TargetId, Vec<Series>> {
    let mut grouped: BTreeMap<TargetId, Vec<Series>> = BTreeMap::new();
    for one in series {
        let Some(id) = one.metric.get("target").and_then(|raw| raw.parse().ok()) else {
            continue;
        };
        let Some(name) = one.metric.get("__name__") else { continue };
        let name = name.strip_prefix("dumbmonit_").unwrap_or(name).to_string();
        let labels = one
            .metric
            .into_iter()
            .filter(|(key, _)| key != "__name__" && key != "target")
            .collect();
        grouped.entry(id).or_default().push(Series { name, labels });
    }
    grouped
}

/// L'inventaire de tous les équipements actifs.
pub async fn build_inventory(state: &AppState) -> anyhow::Result<Vec<Component>> {
    let catalog = catalog::catalog();
    let targets = crate::db::targets::list(&state.pool, &state.cipher).await?;
    let active: HashSet<TargetId> = targets.iter().filter(|t| t.enabled).map(|t| t.id).collect();
    let mut builder = Builder::new(catalog);
    // VictoriaMetrics indisponible : on garde au moins l'inventaire des agents.
    match state.victoria.query(&inventory_query(&catalog.metric_names())).await {
        Ok(series) => {
            for (target_id, list) in group_series(series) {
                if active.contains(&target_id) {
                    builder.add_series(target_id, &list);
                }
            }
        }
        Err(error) => warn!(?error, "software inventory: version series not read"),
    }
    for (target_id, inventory) in store::agent_inventories(&state.pool).await? {
        if active.contains(&target_id) {
            builder.add_agent(target_id, &inventory);
        }
    }
    Ok(builder.finish())
}

// ---------------------------------------------------------------- sources

fn parse_utc(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw).map(|t| t.to_utc()).ok().or_else(|| {
        NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S").ok().map(|t| t.and_utc())
    })
}

async fn sources_due(pool: &SqlitePool) -> sqlx::Result<bool> {
    let kev = store::source(pool, "kev").await?;
    let last = kev.last_attempt.as_deref().and_then(parse_utc);
    Ok(last.is_none_or(|last| Utc::now() - last >= SOURCES_EVERY))
}

async fn record(pool: &SqlitePool, name: &str, outcome: Result<Option<&str>, &str>) {
    if let Err(error) = store::record_source(pool, name, outcome).await {
        warn!(?error, source = name, "CVE source state not saved");
    }
}

/// Relit les sources utiles à l'inventaire. Chaque source peut échouer sans
/// empêcher les autres ; l'échec est noté et montré.
async fn refresh_sources(state: &AppState, fetcher: &Fetcher, components: &[Component]) {
    let pool = &state.pool;

    match fetcher.kev().await {
        Ok(entries) => match store::save_kev(pool, &entries).await {
            Ok(()) => record(pool, "kev", Ok(None)).await,
            Err(error) => warn!(?error, "KEV not saved"),
        },
        Err(error) => record(pool, "kev", Err(error.as_str())).await,
    }

    // NVD, un produit après l'autre (différentiel par produit).
    let mut products: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for component in components {
        if let Origin::Nvd { product, cpes, .. } = &component.origin {
            products.entry(product.clone()).or_insert_with(|| cpes.clone());
        }
    }
    let mut failures = Vec::new();
    for (product, cpes) in &products {
        let name = format!("nvd:{product}");
        let previous = store::source(pool, &name).await.unwrap_or_default();
        let since = previous.cursor.as_deref().and_then(parse_utc);
        // Une marge d'une heure : une CVE modifiée pendant la lecture précédente.
        let started = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        match fetcher.nvd_product(cpes, since).await {
            Ok(items) => match store::save_nvd(pool, product, &items).await {
                Ok(()) => record(pool, &name, Ok(Some(&started))).await,
                Err(error) => warn!(?error, product, "NVD answer not saved"),
            },
            Err(error) => {
                record(pool, &name, Err(error.as_str())).await;
                failures.push(format!("{product}: {error}"));
            }
        }
    }
    if !products.is_empty() {
        if failures.is_empty() {
            record(pool, "nvd", Ok(None)).await;
        } else {
            let message = failures.join("; ");
            record(pool, "nvd", Err(message.as_str())).await;
        }
    }

    // Debian Security Tracker.
    let mut packages = HashSet::new();
    let mut releases = HashSet::new();
    for component in components {
        if let Origin::Debian { package, release } = &component.origin {
            packages.insert(package.clone());
            releases.insert(release.clone());
        }
    }
    if !packages.is_empty() {
        let dir = state.config.data_dir.clone();
        match fetcher.debian(&dir, packages, releases.clone()).await {
            Ok(entries) => match store::save_debian(pool, &releases, &entries).await {
                Ok(()) => record(pool, "debian", Ok(None)).await,
                Err(error) => warn!(?error, "Debian tracker not saved"),
            },
            Err(error) => record(pool, "debian", Err(error.as_str())).await,
        }
    }

    // OSV.dev.
    let mut queries: Vec<osv::Query> = components
        .iter()
        .filter_map(|component| match &component.origin {
            Origin::Osv { ecosystem, package } => Some(osv::Query {
                ecosystem: ecosystem.clone(),
                package: package.clone(),
                version: component.version.clone(),
            }),
            _ => None,
        })
        .collect();
    queries.sort();
    queries.dedup();
    if !queries.is_empty() {
        match refresh_osv(pool, fetcher, &queries).await {
            Ok(()) => record(pool, "osv", Ok(None)).await,
            Err(error) => record(pool, "osv", Err(error.as_str())).await,
        }
    }
}

async fn refresh_osv(
    pool: &SqlitePool,
    fetcher: &Fetcher,
    queries: &[osv::Query],
) -> Result<(), String> {
    let db = |error: sqlx::Error| format!("database error: {error}");
    let mut wanted: HashMap<String, String> = HashMap::new();
    for chunk in queries.chunks(osv::BATCH) {
        let results = fetcher.osv_batch(chunk).await?;
        for (query, vulns) in chunk.iter().zip(results) {
            store::save_osv_query(pool, query, &vulns).await.map_err(db)?;
            for vuln in vulns {
                wanted.insert(vuln.id, vuln.modified);
            }
        }
    }
    let known = store::osv_modified(pool).await.map_err(db)?;
    let stale: Vec<String> = wanted
        .into_iter()
        .filter(|(id, modified)| known.get(id) != Some(modified))
        .map(|(id, _)| id)
        .take(OSV_DETAIL_BUDGET)
        .collect();
    let mut details = futures::stream::iter(stale)
        .map(|id| async move { (fetcher.osv_vuln(&id).await, id) })
        .buffer_unordered(8);
    let mut failed = 0usize;
    while let Some((result, id)) = details.next().await {
        match result {
            Ok(vuln) => store::save_osv_vuln(pool, &vuln).await.map_err(db)?,
            Err(error) => {
                failed += 1;
                debug!(id, error, "OSV advisory not read");
            }
        }
    }
    store::prune_osv(pool).await.map_err(db)?;
    if failed > 0 {
        return Err(format!("{failed} advisories could not be read"));
    }
    Ok(())
}

// ------------------------------------------------------------------ calcul

/// Tout ce que le calcul lit en base, chargé une fois.
#[derive(Default)]
pub struct Data {
    pub nvd: HashMap<String, Vec<(String, Vec<nvd::CpeRange>)>>,
    pub debian: HashMap<(String, String), Vec<store::DebianRow>>,
    pub osv_queries: HashMap<(String, String, String), Vec<String>>,
    pub osv_vulns: HashMap<String, osv::Vuln>,
    pub kev: HashMap<String, store::Kev>,
    pub infos: store::InfoMap,
}

impl Data {
    async fn load(pool: &SqlitePool, components: &[Component]) -> sqlx::Result<Self> {
        let mut nvd = HashMap::new();
        for component in components {
            if let Origin::Nvd { product, .. } = &component.origin
                && !nvd.contains_key(product)
            {
                nvd.insert(product.clone(), store::nvd_for(pool, product).await?);
            }
        }
        Ok(Self {
            nvd,
            debian: store::debian_all(pool).await?,
            osv_queries: store::osv_queries(pool).await?,
            osv_vulns: store::osv_vulns(pool).await?,
            kev: store::kev_all(pool).await?,
            infos: store::infos(pool).await?,
        })
    }
}

/// Un verdict avant tri : tout ce que les sources disent, montré ou non.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub target_id: TargetId,
    pub component: String,
    pub version: String,
    pub cve: String,
    pub verdict: Verdict,
    /// `nvd`, `debian`, `osv`.
    pub source: &'static str,
    /// Debian ne publiera pas de correctif de sécurité (« Minor issue »).
    pub nodsa: bool,
}

fn severity_rank(verdict: &Verdict) -> u8 {
    match verdict.state {
        State::Vulnerable => 0,
        State::Unknown => 1,
        State::Patched => 2,
    }
}

/// Les verdicts de tout l'inventaire, un par (équipement, composant, CVE).
pub fn evaluate(components: &[Component], data: &Data) -> Vec<Candidate> {
    let mut out: HashMap<(TargetId, String, String), Candidate> = HashMap::new();
    let mut push = |candidate: Candidate| {
        let key = (candidate.target_id, candidate.component.clone(), candidate.cve.clone());
        let worse = out.get(&key).is_none_or(|existing| {
            severity_rank(&existing.verdict) > severity_rank(&candidate.verdict)
        });
        if worse {
            out.insert(key, candidate);
        }
    };
    for component in components {
        let base = |cve: String, verdict: Verdict, source: &'static str, nodsa: bool| Candidate {
            target_id: component.target_id,
            component: component.name.clone(),
            version: component.version.clone(),
            cve,
            verdict,
            source,
            nodsa,
        };
        match &component.origin {
            Origin::Nvd { product, vendor_build, .. } => {
                for (cve, ranges) in data.nvd.get(product).into_iter().flatten() {
                    if let Some(verdict) = verdict::nvd(&component.version, *vendor_build, ranges) {
                        push(base(cve.clone(), verdict, "nvd", false));
                    }
                }
            }
            Origin::Debian { package, release } => {
                let key = (package.clone(), release.clone());
                for row in data.debian.get(&key).into_iter().flatten() {
                    let verdict = verdict::debian(
                        &component.version,
                        row.status,
                        row.fixed_version.as_deref(),
                    );
                    push(base(row.cve.clone(), verdict, "debian", row.nodsa));
                }
            }
            Origin::Osv { ecosystem, package } => {
                let key = (ecosystem.clone(), package.clone(), component.version.clone());
                for id in data.osv_queries.get(&key).into_iter().flatten() {
                    let Some(vuln) = data.osv_vulns.get(id) else { continue };
                    let verdict = verdict::osv(vuln.fixed_for(ecosystem, package));
                    for cve in vuln.ids() {
                        push(base(cve, verdict.clone(), "osv", false));
                    }
                }
            }
            Origin::None => {}
        }
    }
    let mut list: Vec<Candidate> = out.into_values().collect();
    list.sort_by(|a, b| {
        (a.target_id, &a.component, &a.cve).cmp(&(b.target_id, &b.component, &b.cve))
    });
    list
}

/// Une CVE récente : publiée depuis moins d'un an, ou, sans date, d'une
/// année d'identifiant récente.
fn recent(cve: &str, published: Option<&str>, now: DateTime<Utc>) -> bool {
    if let Some(published) = published.and_then(parse_utc) {
        return now - published <= chrono::Duration::days(365);
    }
    let year: Option<i32> =
        cve.strip_prefix("CVE-").and_then(|r| r.split('-').next()?.parse().ok());
    year.is_some_and(|year| year >= now.year() - 1)
}

/// Ce qui mérite d'être montré : tout ce qui est exploité ou corrigeable,
/// le reste seulement quand c'est grave ou récent. Sans ce tri, un noyau
/// Debian porterait des centaines de failles mineures sans correctif.
pub fn worth_listing(
    candidate: &Candidate,
    exploited: bool,
    cvss: Option<f64>,
    published: Option<&str>,
    now: DateTime<Utc>,
) -> bool {
    if exploited {
        return true;
    }
    let notable = cvss.is_some_and(|s| s >= NOTABLE_CVSS);
    let product = candidate.source == "nvd";
    match (candidate.verdict.state, candidate.verdict.reason) {
        (State::Vulnerable, Reason::NoFixAvailable) => notable && !candidate.nodsa,
        (State::Vulnerable, _) => true,
        (State::Unknown, _) => product || notable,
        (State::Patched, _) => recent(&candidate.cve, published, now) && (product || notable),
    }
}

/// Recalcule l'inventaire et les verdicts, et les enregistre.
async fn compute(state: &AppState, components: &[Component]) -> anyhow::Result<Vec<Candidate>> {
    let pool = &state.pool;
    let data = Data::load(pool, components).await?;
    let candidates = evaluate(components, &data);
    let now = Utc::now();
    let stamp = store::now();
    let findings: Vec<store::FindingRow> = candidates
        .iter()
        .filter(|c| {
            let info = data.infos.get(&c.cve);
            worth_listing(
                c,
                data.kev.contains_key(&c.cve),
                info.and_then(|i| i.1),
                info.and_then(|i| i.2.as_deref()),
                now,
            )
        })
        .map(|c| store::FindingRow {
            target_id: c.target_id,
            component: c.component.clone(),
            cve_id: c.cve.clone(),
            installed_version: c.version.clone(),
            fixed_version: c.verdict.fixed_version.clone(),
            state: c.verdict.state.as_str().to_string(),
            reason: c.verdict.reason.as_str().to_string(),
            source: c.source.to_string(),
            first_seen: stamp.clone(),
        })
        .collect();
    let rows: Vec<store::ComponentRow> = components
        .iter()
        .map(|c| store::ComponentRow {
            target_id: c.target_id,
            component: c.name.clone(),
            version: c.version.clone(),
            origin: c.origin.label(),
        })
        .collect();
    store::replace_results(pool, &rows, &findings).await?;
    record(pool, "scan", Ok(None)).await;
    debug!(components = rows.len(), findings = findings.len(), "CVE verdicts computed");
    Ok(candidates)
}

/// Note les CVE venues de Debian et d'OSV par leur fiche NVD, les plus
/// utiles d'abord. Renvoie le nombre de fiches lues.
async fn enrich(state: &AppState, fetcher: &Fetcher, candidates: &[Candidate]) -> usize {
    let infos = match store::infos(&state.pool).await {
        Ok(infos) => infos,
        Err(error) => {
            warn!(?error, "CVE details not read");
            return 0;
        }
    };
    let now = Utc::now();
    let mut wanted: Vec<(u8, &str)> = candidates
        .iter()
        .filter(|c| c.source != "nvd" && c.cve.starts_with("CVE-"))
        .filter(|c| !infos.get(&c.cve).is_some_and(|i| i.3))
        .filter_map(|c| {
            let published = infos.get(&c.cve).and_then(|i| i.2.as_deref());
            let rank = match (c.verdict.state, c.verdict.reason) {
                (State::Vulnerable, Reason::NoFixAvailable) if c.nodsa => return None,
                (State::Vulnerable, Reason::NoFixAvailable) => 1,
                (State::Vulnerable, _) => 0,
                (State::Unknown, _) => 2,
                (State::Patched, _) if recent(&c.cve, published, now) => 3,
                (State::Patched, _) => return None,
            };
            Some((rank, c.cve.as_str()))
        })
        .collect();
    wanted.sort();
    let mut seen = HashSet::new();
    let budget = if fetcher.has_nvd_key() { ENRICH_BUDGET_WITH_KEY } else { ENRICH_BUDGET };
    let ids: Vec<&str> =
        wanted.into_iter().map(|(_, id)| id).filter(|id| seen.insert(*id)).take(budget).collect();
    let mut done = 0;
    for id in ids {
        match fetcher.nvd_cve(id).await {
            Ok(Some(info)) => {
                if let Err(error) = store::upsert_info(&state.pool, &info, true).await {
                    warn!(?error, "CVE score not saved");
                    break;
                }
            }
            Ok(None) => {
                let _ = store::mark_scored(&state.pool, id).await;
            }
            Err(error) => {
                debug!(id, error, "NVD score lookup stopped");
                break;
            }
        }
        done += 1;
    }
    done
}

/// Un tour complet : inventaire, sources si dues, verdicts, notes.
async fn run_cycle(state: &AppState, forced: bool) -> anyhow::Result<()> {
    let key = nvd_key(&state.pool, &state.cipher).await?;
    let fetcher = Fetcher::new(key).map_err(anyhow::Error::msg)?;
    let components = build_inventory(state).await?;
    if forced || sources_due(&state.pool).await? {
        info!(components = components.len(), "refreshing CVE sources");
        refresh_sources(state, &fetcher, &components).await;
    }
    let candidates = compute(state, &components).await?;
    if enrich(state, &fetcher, &candidates).await > 0 {
        compute(state, &components).await?;
    }
    Ok(())
}

// --------------------------------------------------------------- alertes

/// Séries d'alerte : une par CVE non ignorée à laquelle un composant est
/// vulnérable (valeur : note CVSS, 0 si inconnue), et une de plus quand
/// elle est exploitée (KEV).
pub fn alert_samples(
    findings: &[store::FindingRow],
    ignored: &HashSet<(TargetId, String)>,
    kev: &HashMap<String, store::Kev>,
    infos: &store::InfoMap,
    ts_ms: i64,
) -> Vec<Sample> {
    let mut out = Vec::new();
    for finding in findings.iter().filter(|f| f.state == State::Vulnerable.as_str()) {
        if ignored.contains(&(finding.target_id, finding.cve_id.clone())) {
            continue;
        }
        let labelled = |metric: &str, value: f64| {
            Sample::new(metric, value, MetricKind::Gauge, ts_ms)
                .with_label("target", finding.target_id.to_string())
                .with_label("cve", finding.cve_id.clone())
                .with_label("component", finding.component.clone())
        };
        let cvss = infos.get(&finding.cve_id).and_then(|i| i.1).unwrap_or(0.0);
        out.push(labelled("cve_vulnerable", cvss));
        if kev.contains_key(&finding.cve_id) {
            out.push(labelled("cve_exploited_vulnerable", 1.0));
        }
        if out.len() >= MAX_PUBLISHED {
            break;
        }
    }
    out
}

async fn publish(state: &AppState) -> anyhow::Result<()> {
    let pool = &state.pool;
    let findings = store::findings(pool).await?;
    let ignored: HashSet<(TargetId, String)> = store::ignores(pool).await?.into_keys().collect();
    let samples = alert_samples(
        &findings,
        &ignored,
        &store::kev_all(pool).await?,
        &store::infos(pool).await?,
        Utc::now().timestamp_millis(),
    );
    if !samples.is_empty() {
        state.sink.send(samples).await;
    }
    Ok(())
}

/// La tâche de fond. Coupée : elle ne fait rien, et les séries s'éteignent.
pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        // Laisse aux collecteurs le temps d'une première passe.
        tokio::time::sleep(Duration::from_secs(150)).await;
        let mut last_compute: Option<Instant> = None;
        loop {
            let forced = runtime().forced.swap(false, Ordering::SeqCst);
            match enabled(&state.pool).await {
                Ok(true) => {
                    let due = last_compute.is_none_or(|t| t.elapsed() >= RECOMPUTE_EVERY);
                    // La démonstration publique a ses verdicts écrits d'avance
                    // (`demo::workspace`) : aucune source n'est jamais demandée.
                    if (forced || due) && !state.config.demo {
                        runtime().refreshing.store(true, Ordering::SeqCst);
                        if let Err(error) = run_cycle(&state, forced).await {
                            warn!(?error, "CVE scan failed");
                        }
                        last_compute = Some(Instant::now());
                    }
                    if let Err(error) = publish(&state).await {
                        warn!(?error, "CVE alert series not published");
                    }
                }
                Ok(false) => {}
                Err(error) => warn!(?error, "CVE scan setting not read"),
            }
            runtime().refreshing.store(false, Ordering::SeqCst);
            // Réveil au prochain quart d'heure, ou tout de suite sur demande.
            tokio::select! {
                () = tokio::time::sleep(TICK) => {}
                () = runtime().wake.notified() => {}
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vulns::debian::Status;
    use crate::vulns::nvd::CpeRange;

    fn component(name: &str, version: &str, origin: Origin) -> Component {
        Component { target_id: 1, name: name.into(), version: version.into(), origin }
    }

    fn data() -> Data {
        let mut data = Data::default();
        data.nvd.insert(
            "proxmox-ve".into(),
            vec![
                (
                    "CVE-2024-21545".into(),
                    vec![CpeRange {
                        cpe: "a:proxmox:virtual_environment".into(),
                        version: "*".into(),
                        update: "*".into(),
                        end_excluding: Some("8.2.5".into()),
                        ..CpeRange::default()
                    }],
                ),
                (
                    "CVE-2019-0001".into(),
                    vec![CpeRange {
                        cpe: "a:proxmox:virtual_environment".into(),
                        version: "*".into(),
                        update: "*".into(),
                        end_excluding: Some("5.0".into()),
                        ..CpeRange::default()
                    }],
                ),
            ],
        );
        let row = |cve: &str, status, fixed: Option<&str>, nodsa| store::DebianRow {
            cve: cve.into(),
            status,
            fixed_version: fixed.map(Into::into),
            nodsa,
        };
        data.debian.insert(
            ("openssl".into(), "bookworm".into()),
            vec![
                row("CVE-2026-0001", Status::Resolved, Some("3.0.17-1~deb12u1"), false),
                row("CVE-2026-0002", Status::Open, None, true),
                row("CVE-2026-0003", Status::Open, None, false),
            ],
        );
        data
    }

    #[test]
    fn les_verdicts_suivent_chaque_source() {
        let components = vec![
            component(
                "Proxmox VE (pve1)",
                "8.2.4",
                Origin::Nvd {
                    product: "proxmox-ve".into(),
                    cpes: vec!["a:proxmox:virtual_environment".into()],
                    vendor_build: false,
                },
            ),
            component(
                "openssl",
                "3.0.15-1~deb12u1",
                Origin::Debian { package: "openssl".into(), release: "bookworm".into() },
            ),
        ];
        let candidates = evaluate(&components, &data());
        let by_cve: HashMap<&str, &Candidate> =
            candidates.iter().map(|c| (c.cve.as_str(), c)).collect();
        assert_eq!(by_cve["CVE-2024-21545"].verdict.state, State::Vulnerable);
        assert_eq!(by_cve["CVE-2019-0001"].verdict.state, State::Patched);
        assert_eq!(by_cve["CVE-2026-0001"].verdict.reason, Reason::BelowFixedVersion);
        assert!(by_cve["CVE-2026-0002"].nodsa);
        assert_eq!(candidates.len(), 5);
    }

    #[test]
    fn le_tri_ecarte_le_bruit_mais_jamais_l_exploite() {
        let now = parse_utc("2026-10-22 12:00:00").unwrap();
        let candidates = evaluate(
            &[component(
                "openssl",
                "3.0.15-1~deb12u1",
                Origin::Debian { package: "openssl".into(), release: "bookworm".into() },
            )],
            &data(),
        );
        let find = |cve: &str| candidates.iter().find(|c| c.cve == cve).unwrap();
        // Corrigeable : toujours montrée.
        assert!(worth_listing(find("CVE-2026-0001"), false, None, None, now));
        // Sans correctif : seulement si grave, et jamais quand Debian la dit mineure…
        assert!(!worth_listing(find("CVE-2026-0003"), false, Some(5.0), None, now));
        assert!(worth_listing(find("CVE-2026-0003"), false, Some(7.5), None, now));
        assert!(!worth_listing(find("CVE-2026-0002"), false, Some(9.8), None, now));
        // … sauf exploitée.
        assert!(worth_listing(find("CVE-2026-0002"), true, None, None, now));
    }

    #[test]
    fn une_faille_corrigee_n_est_montree_que_recente() {
        let now = parse_utc("2026-10-22 12:00:00").unwrap();
        let candidate = |cve: &str| Candidate {
            target_id: 1,
            component: "Proxmox VE".into(),
            version: "8.2.10".into(),
            cve: cve.into(),
            verdict: Verdict {
                state: State::Patched,
                reason: Reason::FixedVersionInstalled,
                fixed_version: Some("8.2.5".into()),
            },
            source: "nvd",
            nodsa: false,
        };
        assert!(worth_listing(
            &candidate("CVE-2026-1"),
            false,
            None,
            Some("2026-09-01 00:00:00"),
            now
        ));
        assert!(!worth_listing(
            &candidate("CVE-2024-1"),
            false,
            None,
            Some("2024-09-01 00:00:00"),
            now
        ));
        assert!(worth_listing(&candidate("CVE-2025-1"), false, None, None, now), "année récente");
        assert!(!worth_listing(&candidate("CVE-2019-1"), false, None, None, now));
    }

    #[test]
    fn seules_les_failles_vulnerables_non_ignorees_alertent() {
        let finding = |target_id, cve: &str, state: &str| store::FindingRow {
            target_id,
            component: "openssl".into(),
            cve_id: cve.into(),
            installed_version: "1".into(),
            fixed_version: None,
            state: state.into(),
            reason: "below_fixed_version".into(),
            source: "debian".into(),
            first_seen: String::new(),
        };
        let findings = vec![
            finding(1, "CVE-1", "vulnerable"),
            finding(1, "CVE-2", "patched"),
            finding(1, "CVE-3", "vulnerable"),
            finding(2, "CVE-1", "vulnerable"),
        ];
        let ignored = HashSet::from([(1, "CVE-3".to_string())]);
        let kev = HashMap::from([("CVE-1".to_string(), store::Kev::default())]);
        let mut infos = store::InfoMap::new();
        infos.insert("CVE-1".into(), (String::new(), Some(9.8), None, true));
        let samples = alert_samples(&findings, &ignored, &kev, &infos, 0);
        let names: Vec<(&str, &str)> =
            samples.iter().map(|s| (s.metric.as_str(), s.labels["target"].as_str())).collect();
        assert_eq!(
            names,
            [
                ("cve_vulnerable", "1"),
                ("cve_exploited_vulnerable", "1"),
                ("cve_vulnerable", "2"),
                ("cve_exploited_vulnerable", "2"),
            ]
        );
        assert_eq!(samples[0].value, 9.8);
        assert_eq!(samples[0].labels["cve"], "CVE-1");
    }

    #[test]
    fn la_requete_d_inventaire_lit_toutes_les_series_de_version() {
        let query = inventory_query(&["a_info", "b_version_info"]);
        assert_eq!(
            query,
            r#"last_over_time({__name__=~"dumbmonit_(a_info|b_version_info)"}[1d]) keep_metric_names"#
        );
        let series: Vec<InstantSeries> = serde_json::from_value(serde_json::json!([
            { "metric": { "__name__": "dumbmonit_pbs_version_info", "target": "4", "version": "3.2.8" }, "value": [1.0, "1"] },
            { "metric": { "__name__": "dumbmonit_pbs_version_info", "version": "3.2.8" }, "value": [1.0, "1"] }
        ]))
        .unwrap();
        let grouped = group_series(series);
        assert_eq!(grouped.len(), 1);
        assert_eq!(grouped[&4][0].name, "pbs_version_info");
        assert_eq!(grouped[&4][0].labels.get("version").map(String::as_str), Some("3.2.8"));
        assert!(!grouped[&4][0].labels.contains_key("target"));
    }
}
