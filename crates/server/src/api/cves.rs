//! Failles connues (CVE) de l'inventaire logiciel (`crate::vulns`).
//!
//! * `GET` / `PUT /security/cves/settings` — activation (coupée par défaut)
//!   et clé d'API NVD, qui ne revient jamais au client.
//! * `GET /security/cves/summary` — compteurs, état des sources.
//! * `GET /security/cves` — les verdicts, exploitées d'abord.
//! * `GET /security/cves/inventory` — ce qui est inventorié, et d'où.
//! * `POST /security/cves/refresh` — relecture des sources (administrateur).
//! * `POST /security/cves/ignores`, `DELETE /security/cves/ignores/{target}/{cve}`
//!   — accepter un risque sur un équipement, ou y revenir.
//!
//! Tout est lu en base : aucune de ces routes n'appelle une source externe.

use std::collections::{HashMap, HashSet};

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use dumbmonit_proto::TargetId;
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::AdminIdentity;
use crate::auth::teams::TeamAccess;
use crate::db;
use crate::state::AppState;
use crate::vulns::{self, store};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/security/cves", get(list))
        .route("/security/cves/summary", get(summary))
        .route("/security/cves/inventory", get(inventory))
        .route("/security/cves/settings", get(settings).put(set_settings))
        .route("/security/cves/refresh", post(refresh))
        .route("/security/cves/ignores", post(add_ignore))
        .route("/security/cves/ignores/{target_id}/{cve_id}", delete(remove_ignore))
}

fn internal(error: sqlx::Error) -> ApiError {
    ApiError::Internal(error.into())
}

// ----------------------------------------------------------------- réglages

#[derive(Debug, Serialize)]
pub struct CveSettings {
    pub enabled: bool,
    pub has_nvd_key: bool,
}

#[derive(Debug, Deserialize)]
pub struct CveSettingsInput {
    pub enabled: bool,
    /// Absente : inchangée ; vide : effacée.
    #[serde(default)]
    pub nvd_api_key: Option<String>,
}

async fn read_settings(state: &AppState) -> ApiResult<CveSettings> {
    let enabled = vulns::enabled(&state.pool).await.map_err(internal)?;
    let has_nvd_key = vulns::nvd_key(&state.pool, &state.cipher).await.map_err(internal)?.is_some();
    Ok(CveSettings { enabled, has_nvd_key })
}

async fn settings(State(state): State<AppState>) -> ApiResult<Json<CveSettings>> {
    Ok(Json(read_settings(&state).await?))
}

async fn set_settings(
    State(state): State<AppState>,
    _: AdminIdentity,
    Json(input): Json<CveSettingsInput>,
) -> ApiResult<Json<CveSettings>> {
    if let Some(key) = &input.nvd_api_key {
        if key.len() > 128 || key.chars().any(char::is_control) {
            return Err(ApiError::BadRequest("The NVD API key is not valid.".into()));
        }
        vulns::set_nvd_key(&state.pool, &state.cipher, key).await?;
    }
    let was = vulns::enabled(&state.pool).await.map_err(internal)?;
    vulns::set_enabled(&state.pool, input.enabled).await.map_err(internal)?;
    if input.enabled && !was {
        // Premier passage tout de suite, sans attendre le prochain quart d'heure.
        vulns::request_refresh();
    }
    Ok(Json(read_settings(&state).await?))
}

async fn refresh(
    State(state): State<AppState>,
    _: AdminIdentity,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    if !vulns::enabled(&state.pool).await.map_err(internal)? {
        return Err(ApiError::Conflict("The CVE scan is turned off.".into()));
    }
    vulns::request_refresh();
    Ok((StatusCode::ACCEPTED, Json(serde_json::json!({ "refreshing": true }))))
}

// ---------------------------------------------------------------- verdicts

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    None,
}

/// Classes de gravité de la CVSS 3.
pub fn severity_of(cvss: Option<f64>) -> Severity {
    match cvss {
        Some(s) if s >= 9.0 => Severity::Critical,
        Some(s) if s >= 7.0 => Severity::High,
        Some(s) if s >= 4.0 => Severity::Medium,
        Some(s) if s > 0.0 => Severity::Low,
        _ => Severity::None,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CveFinding {
    pub target_id: TargetId,
    pub target_name: String,
    pub target_kind: String,
    pub component: String,
    pub installed_version: String,
    pub fixed_version: Option<String>,
    pub cve_id: String,
    pub summary: String,
    pub cvss: Option<f64>,
    pub severity: Severity,
    pub published: Option<String>,
    pub exploited: bool,
    pub ransomware: bool,
    pub state: String,
    pub reason: String,
    pub source: String,
    pub link: String,
    pub ignored: bool,
    pub ignore_note: Option<String>,
    pub ignored_by: Option<String>,
    pub ignored_at: Option<String>,
}

/// Lien de référence : la fiche NVD d'une CVE, l'avis OSV sinon.
pub fn link_for(id: &str) -> String {
    if id.starts_with("CVE-") {
        format!("https://nvd.nist.gov/vuln/detail/{id}")
    } else {
        format!("https://osv.dev/vulnerability/{id}")
    }
}

fn state_rank(state: &str) -> u8 {
    match state {
        "vulnerable" => 0,
        "unknown" => 1,
        _ => 2,
    }
}

/// Exploitées d'abord, puis vulnérables, à vérifier, corrigées ; à état
/// égal, la note la plus haute, puis la CVE la plus récente.
pub fn sort_findings(findings: &mut [CveFinding]) {
    findings.sort_by(|a, b| {
        b.exploited
            .cmp(&a.exploited)
            .then(state_rank(&a.state).cmp(&state_rank(&b.state)))
            .then(b.cvss.unwrap_or(-1.0).total_cmp(&a.cvss.unwrap_or(-1.0)))
            .then_with(|| b.cve_id.cmp(&a.cve_id))
            .then_with(|| a.target_name.cmp(&b.target_name))
    });
}

async fn all_findings(state: &AppState) -> ApiResult<Vec<CveFinding>> {
    let pool = &state.pool;
    let targets: HashMap<TargetId, (String, String)> = db::targets::list(pool, &state.cipher)
        .await?
        .into_iter()
        .map(|t| (t.id, (t.name, t.kind)))
        .collect();
    let infos = store::infos(pool).await.map_err(internal)?;
    let kev = store::kev_all(pool).await.map_err(internal)?;
    let ignores = store::ignores(pool).await.map_err(internal)?;
    let mut out = Vec::new();
    for row in store::findings(pool).await.map_err(internal)? {
        let Some((target_name, target_kind)) = targets.get(&row.target_id) else { continue };
        let info = infos.get(&row.cve_id);
        let cvss = info.and_then(|i| i.1);
        let ignore = ignores.get(&(row.target_id, row.cve_id.clone()));
        let exploited = kev.get(&row.cve_id);
        out.push(CveFinding {
            target_id: row.target_id,
            target_name: target_name.clone(),
            target_kind: target_kind.clone(),
            component: row.component,
            installed_version: row.installed_version,
            fixed_version: row.fixed_version,
            summary: info.map(|i| i.0.clone()).unwrap_or_default(),
            cvss,
            severity: severity_of(cvss),
            published: info.and_then(|i| i.2.clone()),
            exploited: exploited.is_some(),
            ransomware: exploited.is_some_and(|k| k.ransomware),
            state: row.state,
            reason: row.reason,
            source: row.source,
            link: link_for(&row.cve_id),
            ignored: ignore.is_some(),
            ignore_note: ignore.map(|i| i.note.clone()).filter(|n| !n.is_empty()),
            ignored_by: ignore.map(|i| i.created_by.clone()).filter(|n| !n.is_empty()),
            ignored_at: ignore.map(|i| i.created_at.clone()),
            cve_id: row.cve_id,
        });
    }
    sort_findings(&mut out);
    Ok(out)
}

#[derive(Debug, Default, Deserialize)]
pub struct ListQuery {
    pub target: Option<TargetId>,
    pub severity: Option<Severity>,
    pub state: Option<String>,
    pub exploited: Option<bool>,
    pub ignored: Option<bool>,
}

impl ListQuery {
    pub fn keeps(&self, finding: &CveFinding) -> bool {
        self.target.is_none_or(|t| t == finding.target_id)
            && self.severity.is_none_or(|s| s == finding.severity)
            && self.state.as_deref().is_none_or(|s| s == finding.state)
            && self.exploited.is_none_or(|e| e == finding.exploited)
            && self.ignored.is_none_or(|i| i == finding.ignored)
    }
}

#[derive(Debug, Serialize)]
pub struct CveList {
    pub enabled: bool,
    pub findings: Vec<CveFinding>,
}

async fn list(
    TeamAccess(access): TeamAccess,
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> ApiResult<Json<CveList>> {
    if let Some(wanted) = &query.state
        && vulns::verdict::State::parse(wanted).is_none()
    {
        return Err(ApiError::BadRequest(format!("Unknown state: {wanted}.")));
    }
    let enabled = vulns::enabled(&state.pool).await.map_err(internal)?;
    let visible = access.visible_targets(&state.pool).await?;
    let findings = all_findings(&state)
        .await?
        .into_iter()
        .filter(|f| query.keeps(f))
        .filter(|f| visible.as_ref().is_none_or(|v| v.contains(&f.target_id)))
        .collect();
    Ok(Json(CveList { enabled, findings }))
}

// ------------------------------------------------------------------ résumé

#[derive(Debug, Default, Serialize, PartialEq, Eq)]
pub struct Counts {
    pub vulnerable: usize,
    pub exploited: usize,
    pub unknown: usize,
    pub patched: usize,
    pub ignored: usize,
}

pub fn count(findings: &[CveFinding]) -> Counts {
    let mut counts = Counts::default();
    for finding in findings {
        if finding.ignored {
            counts.ignored += 1;
            continue;
        }
        match finding.state.as_str() {
            "vulnerable" => {
                counts.vulnerable += 1;
                counts.exploited += usize::from(finding.exploited);
            }
            "unknown" => counts.unknown += 1,
            _ => counts.patched += 1,
        }
    }
    counts
}

#[derive(Debug, Serialize)]
pub struct CveSource {
    pub name: &'static str,
    pub last_success: Option<String>,
    pub last_error: Option<String>,
    pub items: i64,
}

#[derive(Debug, Serialize)]
pub struct CveSummary {
    pub enabled: bool,
    pub refreshing: bool,
    pub last_refresh: Option<String>,
    pub counts: Counts,
    pub targets: usize,
    pub components: usize,
    pub sources: Vec<CveSource>,
}

#[derive(Debug, Default, Deserialize)]
pub struct TargetQuery {
    pub target: Option<TargetId>,
}

async fn summary(
    TeamAccess(access): TeamAccess,
    State(state): State<AppState>,
    Query(query): Query<TargetQuery>,
) -> ApiResult<Json<CveSummary>> {
    let pool = &state.pool;
    // Compte restreint par ses équipes : les décomptes ne portent que sur ses
    // équipements.
    let visible = access.visible_targets(pool).await?;
    let keep = |id: TargetId| visible.as_ref().is_none_or(|v| v.contains(&id));
    let enabled = vulns::enabled(pool).await.map_err(internal)?;
    let findings: Vec<CveFinding> = all_findings(&state)
        .await?
        .into_iter()
        .filter(|f| query.target.is_none_or(|t| t == f.target_id))
        .filter(|f| keep(f.target_id))
        .collect();
    let components: Vec<store::ComponentRow> = store::components(pool)
        .await
        .map_err(internal)?
        .into_iter()
        .filter(|c| query.target.is_none_or(|t| t == c.target_id))
        .filter(|c| keep(c.target_id))
        .collect();
    let targets: HashSet<TargetId> = components.iter().map(|c| c.target_id).collect();
    let mut sources = Vec::new();
    for (name, table) in [
        ("nvd", "cve_nvd_product"),
        ("osv", "cve_osv_vuln"),
        ("debian", "cve_debian"),
        ("kev", "cve_kev"),
    ] {
        let source = store::source(pool, name).await.map_err(internal)?;
        sources.push(CveSource {
            name,
            last_success: source.last_success,
            last_error: source.last_error,
            items: store::count(pool, table).await.map_err(internal)?,
        });
    }
    let last_refresh = store::source(pool, "scan").await.map_err(internal)?.last_success;
    Ok(Json(CveSummary {
        enabled,
        refreshing: enabled && vulns::is_refreshing(),
        last_refresh,
        counts: count(&findings),
        targets: targets.len(),
        components: components.len(),
        sources,
    }))
}

// -------------------------------------------------------------- inventaire

#[derive(Debug, Serialize)]
pub struct CveComponent {
    pub target_id: TargetId,
    pub target_name: String,
    pub component: String,
    pub version: String,
    pub origin: String,
    pub findings: usize,
}

#[derive(Debug, Serialize)]
pub struct CveInventory {
    pub components: Vec<CveComponent>,
}

async fn inventory(
    TeamAccess(access): TeamAccess,
    State(state): State<AppState>,
    Query(query): Query<TargetQuery>,
) -> ApiResult<Json<CveInventory>> {
    let pool = &state.pool;
    let visible = access.visible_targets(pool).await?;
    let names: HashMap<TargetId, String> =
        db::targets::list(pool, &state.cipher).await?.into_iter().map(|t| (t.id, t.name)).collect();
    let mut per_component: HashMap<(TargetId, String), usize> = HashMap::new();
    for finding in store::findings(pool).await.map_err(internal)? {
        *per_component.entry((finding.target_id, finding.component)).or_default() += 1;
    }
    let mut components: Vec<CveComponent> = store::components(pool)
        .await
        .map_err(internal)?
        .into_iter()
        .filter(|c| query.target.is_none_or(|t| t == c.target_id))
        .filter(|c| visible.as_ref().is_none_or(|v| v.contains(&c.target_id)))
        .filter_map(|c| {
            let target_name = names.get(&c.target_id)?.clone();
            let findings =
                per_component.get(&(c.target_id, c.component.clone())).copied().unwrap_or(0);
            Some(CveComponent {
                target_id: c.target_id,
                target_name,
                component: c.component,
                version: c.version,
                origin: c.origin,
                findings,
            })
        })
        .collect();
    components.sort_by(|a, b| {
        a.target_name
            .to_lowercase()
            .cmp(&b.target_name.to_lowercase())
            .then(b.findings.cmp(&a.findings))
            .then_with(|| a.component.cmp(&b.component))
    });
    Ok(Json(CveInventory { components }))
}

// --------------------------------------------------------- risques acceptés

#[derive(Debug, Deserialize)]
pub struct IgnoreInput {
    pub target_id: TargetId,
    pub cve_id: String,
    #[serde(default)]
    pub note: Option<String>,
}

/// `CVE-2024-6387`, `UBUNTU-CVE-2024-6387`, `ALSA-2024:4312`.
pub fn valid_cve_id(id: &str) -> bool {
    (3..=64).contains(&id.len())
        && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | ':' | '.' | '_'))
}

async fn add_ignore(
    State(state): State<AppState>,
    AdminIdentity(principal): AdminIdentity,
    Json(input): Json<IgnoreInput>,
) -> ApiResult<StatusCode> {
    if !valid_cve_id(&input.cve_id) {
        return Err(ApiError::BadRequest("The vulnerability identifier is not valid.".into()));
    }
    let note = input.note.unwrap_or_default();
    if note.chars().count() > 500 {
        return Err(ApiError::BadRequest("The note is limited to 500 characters.".into()));
    }
    db::targets::get(&state.pool, &state.cipher, input.target_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Monitor {} not found.", input.target_id)))?;
    store::add_ignore(&state.pool, input.target_id, &input.cve_id, note.trim(), &principal.label())
        .await
        .map_err(internal)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_ignore(
    State(state): State<AppState>,
    _: AdminIdentity,
    Path((target_id, cve_id)): Path<(TargetId, String)>,
) -> ApiResult<StatusCode> {
    if store::remove_ignore(&state.pool, target_id, &cve_id).await.map_err(internal)? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound("This vulnerability is not ignored on this monitor.".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(cve: &str, state: &str, cvss: Option<f64>, exploited: bool) -> CveFinding {
        CveFinding {
            target_id: 1,
            target_name: "pve".into(),
            target_kind: "proxmox".into(),
            component: "Proxmox VE".into(),
            installed_version: "8.2.4".into(),
            fixed_version: None,
            cve_id: cve.into(),
            summary: String::new(),
            cvss,
            severity: severity_of(cvss),
            published: None,
            exploited,
            ransomware: false,
            state: state.into(),
            reason: "below_fixed_version".into(),
            source: "nvd".into(),
            link: link_for(cve),
            ignored: false,
            ignore_note: None,
            ignored_by: None,
            ignored_at: None,
        }
    }

    #[test]
    fn exploitees_d_abord_puis_par_etat_et_par_note() {
        let mut list = vec![
            finding("CVE-1", "patched", Some(9.8), false),
            finding("CVE-2", "vulnerable", Some(5.0), false),
            finding("CVE-3", "vulnerable", Some(9.1), false),
            finding("CVE-4", "unknown", None, false),
            finding("CVE-5", "vulnerable", None, true),
        ];
        sort_findings(&mut list);
        let ids: Vec<&str> = list.iter().map(|f| f.cve_id.as_str()).collect();
        assert_eq!(ids, ["CVE-5", "CVE-3", "CVE-2", "CVE-4", "CVE-1"]);
    }

    #[test]
    fn les_compteurs_et_filtres() {
        let mut ignored = finding("CVE-9", "vulnerable", Some(9.0), true);
        ignored.ignored = true;
        let list = vec![
            finding("CVE-1", "vulnerable", Some(9.8), true),
            finding("CVE-2", "unknown", None, false),
            finding("CVE-3", "patched", None, false),
            ignored,
        ];
        assert_eq!(
            count(&list),
            Counts { vulnerable: 1, exploited: 1, unknown: 1, patched: 1, ignored: 1 }
        );
        let query = ListQuery { severity: Some(Severity::Critical), ..ListQuery::default() };
        assert_eq!(list.iter().filter(|f| query.keeps(f)).count(), 2);
        let query =
            ListQuery { ignored: Some(false), exploited: Some(true), ..ListQuery::default() };
        assert_eq!(list.iter().filter(|f| query.keeps(f)).count(), 1);
    }

    #[test]
    fn gravite_liens_et_identifiants() {
        assert_eq!(severity_of(Some(9.0)), Severity::Critical);
        assert_eq!(severity_of(Some(6.9)), Severity::Medium);
        assert_eq!(severity_of(None), Severity::None);
        assert_eq!(link_for("CVE-2024-6387"), "https://nvd.nist.gov/vuln/detail/CVE-2024-6387");
        assert_eq!(link_for("ALSA-2024:4312"), "https://osv.dev/vulnerability/ALSA-2024:4312");
        assert!(valid_cve_id("ALSA-2024:4312"));
        assert!(!valid_cve_id("CVE-1 OR 1=1"));
        assert!(!valid_cve_id(""));
    }
}
