//! Journaux centralisés : réception des lignes des agents, réglage par machine,
//! état des récepteurs syslog et traps.
//!
//! - [`agent_routes`] : ce que l'agent appelle (même jeton et même clé
//!   d'identité que le canal de commandes), sur le même port que ses mesures ;
//! - [`ui_routes`] : le réglage d'une machine (`/logs/agents/{id}`) et l'état des
//!   entrées (`/logs/inputs`), réservés à un administrateur connecté : un réglage
//!   de fichiers à suivre ne se confie pas à un jeton d'API.

use axum::extract::{Extension, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use dumbmonit_proto::{
    LogBatch, LogsSetting, MAX_LOG_LINES, MAX_LOG_MESSAGE, TargetId, level_rank,
};
use serde::Serialize;
use sqlx::Row;

use crate::agent_tls::AgentCert;
use crate::api::agent_commands::{self, Rejection};
use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::{CurrentPrincipal, Principal};
use crate::collectors::agent::{self, IngestError, logs};
use crate::logs::central::{self, Entry, central as hub};
use crate::state::AppState;

// ------------------------------------------------------------- agent side

pub fn agent_routes() -> Router<AppState> {
    Router::new().route("/agent/logs", post(receive))
}

/// Un horodatage hors de `[maintenant - 30 j, maintenant + 1 h]` est remplacé
/// par l'heure de réception.
fn clamp_ts(ts_ms: i64, now_ms: i64) -> i64 {
    if ts_ms < now_ms - 30 * 86_400_000 || ts_ms > now_ms + 3_600_000 { now_ms } else { ts_ms }
}

/// Les entrées d'un lot, filtrées par le niveau du réglage.
pub fn entries_of(
    batch: LogBatch,
    setting: &LogsSetting,
    target: (i64, String),
    host: &str,
    now_ms: i64,
) -> Vec<Entry> {
    let wanted = level_rank(&setting.min_level);
    batch
        .lines
        .into_iter()
        .filter(|line| !line.message.trim().is_empty())
        .filter(|line| level_rank(&line.level) <= wanted)
        .map(|line| {
            let mut message = line.message;
            if message.len() > MAX_LOG_MESSAGE {
                let mut end = MAX_LOG_MESSAGE;
                while !message.is_char_boundary(end) {
                    end -= 1;
                }
                message.truncate(end);
            }
            let origin: String = line.source.chars().take(200).collect();
            Entry {
                ts_ms: clamp_ts(line.ts_ms, now_ms),
                service: "agent",
                level: central::normalise_level(&line.level),
                host: host.to_string(),
                origin,
                message,
                target: Some(target.clone()),
                extra: Default::default(),
            }
        })
        .collect()
}

/// `POST /api/agent/logs` : un lot de lignes de journal de la machine.
async fn receive(
    State(state): State<AppState>,
    headers: HeaderMap,
    cert: Option<Extension<AgentCert>>,
    Json(batch): Json<LogBatch>,
) -> Result<StatusCode, Rejection> {
    let (token_id, target_id) =
        agent_commands::authenticate(&state, &headers, "", cert.as_deref()).await?;
    if batch.lines.len() > MAX_LOG_LINES {
        return Err(Rejection::from(IngestError::BadRequest(format!(
            "at most {MAX_LOG_LINES} log lines per batch"
        ))));
    }
    let setting = logs::setting(&state.pool, target_id).await?;
    // Collecte éteinte entre-temps : le lot est acquitté puis jeté.
    if !setting.enabled || batch.lines.is_empty() {
        return Ok(StatusCode::NO_CONTENT);
    }
    let row = sqlx::query(
        "SELECT t.name AS name, COALESCE(h.hostname, t.name) AS hostname FROM targets t \
         LEFT JOIN agent_hosts h ON h.target_id = t.id WHERE t.id = ?",
    )
    .bind(target_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|error| Rejection::from(anyhow::Error::from(error)))?;
    let Some(row) = row else { return Ok(StatusCode::NO_CONTENT) };
    let name: String = row.try_get("name").map_err(anyhow::Error::from)?;
    let hostname: String = row.try_get("hostname").map_err(anyhow::Error::from)?;
    let entries = entries_of(
        batch,
        &setting,
        (target_id, name),
        &hostname,
        chrono::Utc::now().timestamp_millis(),
    );
    let count = entries.len() as i64;
    for entry in entries {
        hub().push(entry);
    }
    if count > 0 {
        logs::note_received(&state.pool, target_id, count).await?;
    }
    agent::touch_token(&state.pool, token_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------- UI side

pub fn ui_routes() -> Router<AppState> {
    Router::new()
        .route("/logs/agents/{id}", get(get_setting).put(put_setting))
        .route("/logs/inputs", get(inputs))
}

fn admin(principal: &Principal) -> ApiResult<()> {
    match principal {
        Principal::User(user) if user.role.is_admin() => Ok(()),
        _ => Err(ApiError::Forbidden(
            "Only an administrator signed in to the web interface can read the log inputs.".into(),
        )),
    }
}

async fn agent_target(state: &AppState, id: TargetId) -> ApiResult<()> {
    let kind: Option<String> = sqlx::query_scalar("SELECT kind FROM targets WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|error| ApiError::Internal(error.into()))?;
    match kind.as_deref() {
        Some("agent") => Ok(()),
        Some(_) => Err(ApiError::NotFound(format!("Monitor {id} has no agent."))),
        None => Err(ApiError::NotFound(format!("Monitor {id} not found."))),
    }
}

/// Le réglage d'une machine, tel que l'interface l'affiche.
#[derive(Debug, Serialize)]
pub struct LogsView {
    #[serde(flatten)]
    pub setting: LogsSetting,
    /// Dernière réception, UTC sans suffixe.
    pub last_received_at: Option<String>,
    pub received_total: i64,
    /// Le magasin de journaux embarqué tourne : sans lui, les lignes sont perdues.
    pub store_running: bool,
}

async fn view(state: &AppState, id: TargetId) -> ApiResult<LogsView> {
    let setting = logs::setting(&state.pool, id).await?;
    let (last_received_at, received_total) = logs::status(&state.pool, id).await?;
    Ok(LogsView {
        setting,
        last_received_at,
        received_total,
        store_running: crate::storage::logs_url(state).await.is_some(),
    })
}

/// `GET /api/logs/agents/{id}`.
async fn get_setting(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<LogsView>> {
    admin(&principal)?;
    agent_target(&state, id).await?;
    Ok(Json(view(&state, id).await?))
}

/// `PUT /api/logs/agents/{id}` : règle la collecte ; l'agent l'applique
/// à chaud à son prochain lot.
async fn put_setting(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    Path(id): Path<TargetId>,
    Json(setting): Json<LogsSetting>,
) -> ApiResult<Json<LogsView>> {
    admin(&principal)?;
    agent_target(&state, id).await?;
    let setting = logs::clean(setting).map_err(ApiError::BadRequest)?;
    logs::save(&state.pool, id, &setting).await?;
    tracing::info!(
        cible = id,
        collecte = setting.enabled,
        par = principal.label(),
        "collecte de journaux de l'agent réglée depuis l'interface"
    );
    Ok(Json(view(&state, id).await?))
}

/// État d'un récepteur (syslog ou traps).
#[derive(Debug, Serialize)]
pub struct InputView {
    pub enabled: bool,
    /// Ports à l'écoute (`udp/5514`, `tcp/5514`).
    pub listening: Vec<String>,
    pub received: u64,
    pub rejected: u64,
    /// Messages reçus d'un émetteur qu'aucune cible ne porte.
    pub unmatched: u64,
    /// Dernier message reçu, UTC sans suffixe.
    pub last_at: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InputsView {
    pub store_running: bool,
    pub syslog: InputView,
    pub traps: InputView,
    /// Entrées écrites dans le magasin depuis le démarrage, et perdues.
    pub written: u64,
    pub dropped: u64,
}

fn input_view(input: &central::Input, enabled: bool) -> InputView {
    use std::sync::atomic::Ordering;
    let last = input.last_ms.load(Ordering::Relaxed);
    InputView {
        enabled,
        listening: input.listening(),
        received: input.received.load(Ordering::Relaxed),
        rejected: input.rejected.load(Ordering::Relaxed),
        unmatched: input.unmatched.load(Ordering::Relaxed),
        last_at: (last > 0)
            .then(|| chrono::DateTime::from_timestamp_millis(last))
            .flatten()
            .map(|at| at.format("%Y-%m-%d %H:%M:%S").to_string()),
        error: input.error(),
    }
}

/// `GET /api/logs/inputs` : les entrées de journaux et leur état.
async fn inputs(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
) -> ApiResult<Json<InputsView>> {
    admin(&principal)?;
    let counters = hub().counters();
    Ok(Json(InputsView {
        store_running: crate::storage::logs_url(&state).await.is_some(),
        syslog: input_view(
            central::syslog_input(),
            central::env_flag("DUMBMONIT_SYSLOG_ENABLED", false),
        ),
        traps: input_view(
            central::traps_input(),
            central::env_flag("DUMBMONIT_TRAPS_ENABLED", false),
        ),
        written: counters.written,
        dropped: counters.dropped_queue_full + counters.dropped_store_off + counters.failed,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dumbmonit_proto::LogLine;

    fn line(level: &str, message: &str, ts_ms: i64) -> LogLine {
        LogLine {
            ts_ms,
            level: level.into(),
            source: "journald:ssh.service".into(),
            message: message.into(),
        }
    }

    #[test]
    fn lines_below_the_minimum_level_or_empty_are_dropped() {
        let setting = LogsSetting { enabled: true, min_level: "warn".into(), ..Default::default() };
        let batch = LogBatch {
            lines: vec![
                line("error", "boom", 1_000),
                line("info", "chatter", 1_000),
                line("warn", " ", 1_000),
            ],
        };
        let entries = entries_of(batch, &setting, (3, "nas".into()), "nas.lan", 1_000);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "boom");
        assert_eq!(entries[0].service, "agent");
        assert_eq!(entries[0].target, Some((3, "nas".to_string())));
        assert_eq!(entries[0].host, "nas.lan");
    }

    #[test]
    fn wild_timestamps_fall_back_to_the_reception_time() {
        let now = 1_700_000_000_000;
        assert_eq!(clamp_ts(now - 1000, now), now - 1000);
        assert_eq!(clamp_ts(0, now), now);
        assert_eq!(clamp_ts(now + 10 * 3_600_000, now), now);
    }
}
