//! Routes HTTP du stockage : rétentions, historique long terme, VictoriaLogs
//! embarqué, espace disque (`/api/system/storage`), et recherche dans les
//! journaux gardés par VictoriaLogs (`/api/logs/search`).
//!
//! Réservées à un administrateur connecté : un changement de rétention efface
//! des données, et les journaux disent qui s'est connecté et d'où.

use std::collections::BTreeMap;

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::auth::audit;
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::{CurrentPrincipal, Principal};
use crate::logs::Level;
use crate::logs::central::has_pipe;
use crate::state::AppState;
use crate::storage::{self, StorageView, Update, UpdateError};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/system/storage", get(get_storage).put(put_storage))
        .route("/logs/search", get(search))
}

fn admin(principal: &Principal) -> ApiResult<String> {
    match principal {
        Principal::User(user) if user.role.is_admin() => Ok(user.username.clone()),
        Principal::User(_) => Err(ApiError::Forbidden(
            "Only an administrator can read or change storage and search the logs.".into(),
        )),
        Principal::Token(_) => Err(ApiError::Forbidden(
            "Storage settings and log search are only available to an administrator signed in \
             to the web interface, never to an API token."
                .into(),
        )),
    }
}

pub async fn get_storage(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
) -> ApiResult<Json<StorageView>> {
    admin(&principal)?;
    Ok(Json(storage::view(&state).await?))
}

pub async fn put_storage(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    ClientIp(ip): ClientIp,
    Json(update): Json<Update>,
) -> ApiResult<Json<StorageView>> {
    let me = admin(&principal)?;
    let restarted = match storage::apply(&state, &update).await {
        Ok(restarted) => restarted,
        Err(UpdateError::Invalid(why)) => return Err(ApiError::BadRequest(why)),
        Err(UpdateError::Failed(error)) => {
            tracing::warn!(error = %format!("{error:#}"), "storage change failed");
            return Err(ApiError::BadGateway(format!("{error:#}")));
        }
    };
    audit::record(&state.pool, Some(&me), "storage.settings", Some(&describe(&update)), ip).await;
    let mut view = storage::view(&state).await?;
    view.restarted = restarted;
    Ok(Json(view))
}

/// Résumé lisible d'une modification, pour le journal d'audit.
fn describe(update: &Update) -> String {
    let mut parts = Vec::new();
    if let Some(days) = update.metrics_retention_days {
        parts.push(format!("raw {days} d"));
    }
    if let Some(enabled) = update.rollup_enabled {
        parts.push(format!("long-term {}", if enabled { "on" } else { "off" }));
    }
    if let Some(days) = update.rollup_retention_days {
        parts.push(format!("long-term {days} d"));
    }
    if let Some(enabled) = update.logs_store_enabled {
        parts.push(format!("log store {}", if enabled { "on" } else { "off" }));
    }
    if let Some(days) = update.logs_store_retention_days {
        parts.push(format!("log store {days} d"));
    }
    parts.join(", ")
}

// --------------------------------------------------------------------------
// GET /api/logs/search
// --------------------------------------------------------------------------

/// Plafond d'entrées rendues par une recherche.
const MAX_LIMIT: usize = 1_000;
const DEFAULT_LIMIT: usize = 200;
/// Plafond de la réponse de VictoriaLogs lue.
const MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub query: Option<String>,
    pub start: Option<i64>,
    pub end: Option<i64>,
    pub level: Option<String>,
    pub limit: Option<usize>,
    /// `server` (défaut), `agent`, `syslog`, `trap` ou `all`.
    pub source: Option<String>,
    /// Identifiant d'une cible : ses lignes d'agent, syslog et traps seulement.
    pub target: Option<i64>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct LogSearchEntry {
    pub time: String,
    pub level: String,
    pub message: String,
    pub source: String,
    pub host: Option<String>,
    /// Origine de la ligne : unité journald, fichier, canal, adresse de l'émetteur.
    pub origin: Option<String>,
    /// Nom de la cible rattachée à l'émetteur, quand il y en a une.
    pub target_name: Option<String>,
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
pub struct LogSearchResult {
    pub entries: Vec<LogSearchEntry>,
    pub truncated: bool,
}

/// Niveaux affichés à partir d'un niveau minimal.
fn levels_from(min: Level) -> Vec<&'static str> {
    Level::CHOICES
        .iter()
        .filter(|name| Level::parse(name).is_some_and(|level| level.passes(min)))
        .copied()
        .collect()
}

fn rfc3339(ms: i64) -> Option<String> {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|at| at.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}

/// Le filtre de flux d'une source : le serveur lui-même, ou ce que la collecte
/// centrale y écrit (agents, syslog, traps) ; `all` : tous. `None` : inconnue.
pub fn stream_filter(source: &str) -> Option<String> {
    match source {
        "server" => Some("{service=\"dumbmonit\"}".to_string()),
        "agent" | "agents" => Some("{service=\"agent\"}".to_string()),
        "syslog" => Some("{service=\"syslog\"}".to_string()),
        "trap" | "traps" => Some("{service=\"trap\"}".to_string()),
        "all" => Some("{service=~\"dumbmonit|agent|syslog|trap\"}".to_string()),
        _ => None,
    }
}

/// La requête LogsQL envoyée : période, flux, cible, niveaux, puis le filtre de
/// l'utilisateur entre parenthèses, triée du plus récent au plus ancien.
pub fn build_logsql(
    user: &str,
    start_ms: i64,
    end_ms: i64,
    min: Level,
    limit: usize,
    source: &str,
    target: Option<i64>,
) -> Result<String, String> {
    let user = user.trim();
    let user = if user.is_empty() { "*" } else { user };
    if has_pipe(user) {
        return Err("Only filters are accepted here: remove the `|` pipes from the query.".into());
    }
    let (Some(start), Some(end)) = (rfc3339(start_ms), rfc3339(end_ms)) else {
        return Err("The period is not valid.".into());
    };
    let Some(stream) = stream_filter(source) else {
        return Err(format!("Unknown log source `{source}`."));
    };
    let levels: Vec<String> = levels_from(min).iter().map(|level| format!("\"{level}\"")).collect();
    let target = target.map(|id| format!(" target:=\"{id}\"")).unwrap_or_default();
    Ok(format!(
        "_time:[{start}, {end}] {stream}{target} level:in({}) ({user}) \
         | sort by (_time desc) | limit {limit}",
        levels.join(", ")
    ))
}

/// Champs rendus à part (ou sans intérêt), pas dans `fields`.
const SHOWN_APART: [&str; 9] =
    ["_time", "_msg", "_stream", "_stream_id", "level", "service", "host", "origin", "target_name"];

/// Une ligne de réponse de VictoriaLogs (un objet JSON aux valeurs texte).
pub fn parse_line(line: &str) -> Option<LogSearchEntry> {
    let object: serde_json::Map<String, serde_json::Value> = serde_json::from_str(line).ok()?;
    let text = |key: &str| object.get(key).and_then(|v| v.as_str()).map(str::to_string);
    let time = text("_time")
        .and_then(|raw| chrono::DateTime::parse_from_rfc3339(&raw).ok())
        .map(|at| at.with_timezone(&chrono::Utc).format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default();
    let mut fields = BTreeMap::new();
    for (key, value) in &object {
        if SHOWN_APART.contains(&key.as_str()) {
            continue;
        }
        let value = match value {
            serde_json::Value::String(text) => text.clone(),
            other => other.to_string(),
        };
        fields.insert(key.clone(), value);
    }
    Some(LogSearchEntry {
        time,
        level: text("level").unwrap_or_else(|| "info".into()),
        message: text("_msg").unwrap_or_default(),
        source: text("service").unwrap_or_default(),
        host: text("host"),
        origin: text("origin"),
        target_name: text("target_name"),
        fields,
    })
}

pub async fn search(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    crate::auth::teams::TeamAccess(access): crate::auth::teams::TeamAccess,
    Query(params): Query<SearchQuery>,
) -> ApiResult<Json<LogSearchResult>> {
    admin(&principal)?;
    // Mode MSP, vue limitée à un client : les lignes de ses équipements
    // seulement (agents, syslog, traps). Les journaux de DumbMonit lui-même
    // n'ont pas d'équipement et n'en font donc pas partie.
    let restricted = access.visible_targets(&state.pool).await?;
    if let Some(visible) = &restricted
        && (visible.is_empty() || params.target.is_some_and(|target| !visible.contains(&target)))
    {
        return Ok(Json(LogSearchResult { entries: Vec::new(), truncated: false }));
    }
    let source = params.source.as_deref().unwrap_or("server");
    if stream_filter(source).is_none() {
        return Err(ApiError::BadRequest(format!("Unknown log source `{source}`.")));
    }
    let Some(base) = storage::logs_url(&state).await else {
        return Err(ApiError::Conflict(
            "The log store is off: turn it on in Settings → System.".into(),
        ));
    };
    let now = chrono::Utc::now().timestamp_millis();
    let end = params.end.unwrap_or(now);
    let start = params.start.unwrap_or(end - 3_600_000);
    if end <= start {
        return Err(ApiError::BadRequest("The end of the period must be after its start.".into()));
    }
    let min = match params.level.as_deref() {
        None | Some("") => Level::Info,
        Some(raw) => Level::parse(raw)
            .ok_or_else(|| ApiError::BadRequest(format!("Unknown level `{raw}`.")))?,
    };
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let user = params.query.as_deref().unwrap_or("*").trim();
    let user = if user.is_empty() { "*" } else { user };
    let user = match &restricted {
        Some(visible) => {
            let mut ids: Vec<i64> = visible.iter().copied().collect();
            ids.sort_unstable();
            let list: Vec<String> = ids.iter().map(|id| format!("\"{id}\"")).collect();
            format!("target:in({}) ({user})", list.join(","))
        }
        None => user.to_string(),
    };
    let logsql = build_logsql(
        &user,
        start,
        end,
        min,
        limit + 1,
        source,
        params.target,
    )
    .map_err(ApiError::BadRequest)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|error| ApiError::Internal(error.into()))?;
    let response = client
        .post(format!("{base}/select/logsql/query"))
        .form(&[("query", logsql.as_str())])
        .send()
        .await
        .map_err(|_| ApiError::BadGateway("The log store is not answering.".into()))?;
    let status = response.status();
    let body = response
        .bytes()
        .await
        .map_err(|_| ApiError::BadGateway("The log store answer could not be read.".into()))?;
    if body.len() > MAX_RESPONSE_BYTES {
        return Err(ApiError::BadRequest(
            "Too many matching logs: narrow the period or the query.".into(),
        ));
    }
    let text = String::from_utf8_lossy(&body);
    if status.is_client_error() {
        return Err(ApiError::BadRequest(format!(
            "The log store refused the query: {}",
            text.trim()
        )));
    }
    if !status.is_success() {
        return Err(ApiError::BadGateway(format!("The log store answered {status}.")));
    }
    let mut entries: Vec<LogSearchEntry> = text.lines().filter_map(parse_line).collect();
    let truncated = entries.len() > limit;
    entries.truncate(limit);
    Ok(Json(LogSearchResult { entries, truncated }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipes_are_refused_outside_quotes_only() {
        assert!(has_pipe("error | stats count()"));
        assert!(!has_pipe("_msg:\"a|b\" level:error"));
        assert!(!has_pipe("'x|y'"));
        assert!(!has_pipe("\"escaped \\\" | still inside\""));
    }

    #[test]
    fn the_query_is_scoped_to_the_server_period_and_levels() {
        let q = build_logsql("login failed", 0, 60_000, Level::Warn, 201, "server", None).unwrap();
        assert_eq!(
            q,
            "_time:[1970-01-01T00:00:00.000Z, 1970-01-01T00:01:00.000Z] {service=\"dumbmonit\"} \
             level:in(\"error\", \"warn\") (login failed) | sort by (_time desc) | limit 201"
        );
        assert!(build_logsql("", 0, 1, Level::Debug, 1, "server", None).unwrap().contains("(*)"));
        assert!(build_logsql("a | b", 0, 1, Level::Info, 1, "server", None).is_err());
        assert!(build_logsql("a", 0, 1, Level::Info, 1, "nope", None).is_err());
    }

    #[test]
    fn the_central_sources_and_the_target_narrow_the_query() {
        let q = build_logsql("*", 0, 60_000, Level::Info, 10, "syslog", Some(12)).unwrap();
        assert!(q.contains("{service=\"syslog\"} target:=\"12\" level:in("), "{q}");
        let all = build_logsql("*", 0, 60_000, Level::Info, 10, "all", None).unwrap();
        assert!(all.contains("{service=~\"dumbmonit|agent|syslog|trap\"}"), "{all}");
        assert_eq!(stream_filter("agents"), stream_filter("agent"));
    }

    #[test]
    fn answer_lines_become_entries() {
        let line = r#"{"_time":"2026-10-25T08:00:01.5Z","_stream":"{service=\"dumbmonit\"}","_msg":"probe failed","level":"warn","service":"dumbmonit","host":"nuc","logger":"dumbmonit_server::scheduler","target":"12"}"#;
        let entry = parse_line(line).unwrap();
        assert_eq!(entry.time, "2026-10-25 08:00:01");
        assert_eq!(entry.level, "warn");
        assert_eq!(entry.message, "probe failed");
        assert_eq!(entry.source, "dumbmonit");
        assert_eq!(entry.host.as_deref(), Some("nuc"));
        assert_eq!(entry.fields.get("target").map(String::as_str), Some("12"));
        assert!(!entry.fields.contains_key("_stream"));
        assert_eq!(parse_line("pas du json"), None);
    }
}
