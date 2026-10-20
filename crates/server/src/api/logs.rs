//! Routes HTTP des journaux du serveur : stockage local, export, destinations.
//!
//! Toutes réservées à un administrateur connecté à l'interface, lecture
//! comprise : un journal dit qui s'est connecté, d'où, et ce que surveille
//! l'instance. Un jeton d'API, même en écriture, n'y a pas accès — même règle
//! que la sauvegarde. Comme pour les canaux de notification, les secrets d'une
//! destination n'en ressortent jamais (`has_secret` seulement).

use std::collections::HashMap;
use std::path::PathBuf;

use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Extension, Json, Router};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sqlx::Row;

use crate::api::{ApiError, ApiResult};
use crate::auth::audit;
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::{CurrentPrincipal, Principal};
use crate::logs::catalog::{self, DestinationConfig};
use crate::logs::export::{self, Entry};
use crate::logs::{self, Level, files, ship, store};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/logs", get(overview))
        .route("/logs/settings", put(put_settings))
        .route("/logs/export", get(download))
        .route("/logs/export/estimate", get(estimate))
        .route("/logs/kinds", get(kinds))
        .route("/logs/destinations", get(list).post(create))
        .route("/logs/destinations/{id}", put(update).delete(remove))
        .route("/logs/destinations/{id}/test", post(test))
}

fn admin(principal: &Principal) -> ApiResult<String> {
    match principal {
        Principal::User(user) if user.role.is_admin() => Ok(user.username.clone()),
        Principal::User(_) => {
            Err(ApiError::Forbidden("Only an administrator can read or export server logs.".into()))
        }
        Principal::Token(_) => Err(ApiError::Forbidden(
            "Server logs are only handed to an administrator signed in to the web interface, \
             never to an API token. Sign in and use Settings → Logs."
                .into(),
        )),
    }
}

fn logs_dir(state: &AppState) -> PathBuf {
    logs::dir(&state.config.data_dir)
}

fn today() -> NaiveDate {
    Utc::now().date_naive()
}

// --------------------------------------------------------------------------
// GET /api/logs — stockage local
// --------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct FileView {
    pub day: String,
    pub bytes: u64,
    pub compressed: bool,
}

#[derive(Debug, Serialize)]
pub struct WriterView {
    pub active: bool,
    pub dropped: u64,
}

#[derive(Debug, Serialize)]
pub struct Overview {
    pub directory: String,
    pub retention_days: u32,
    pub max_total_mb: u64,
    pub total_bytes: u64,
    pub oldest_day: Option<String>,
    pub days_available: usize,
    pub files: Vec<FileView>,
    pub writer: WriterView,
}

async fn overview_of(state: &AppState) -> ApiResult<Overview> {
    let (retention_days, max_total_mb) = store::limits(&state.pool).await?;
    let dir = logs_dir(state);
    let listed = {
        let dir = dir.clone();
        tokio::task::spawn_blocking(move || files::list(&dir)).await??
    };
    let mut days: Vec<NaiveDate> = listed.iter().map(|file| file.day).collect();
    days.dedup();
    let hub = logs::hub();
    Ok(Overview {
        directory: dir.display().to_string(),
        retention_days,
        max_total_mb,
        total_bytes: listed.iter().map(|file| file.bytes).sum(),
        oldest_day: listed.last().map(|file| file.day.to_string()),
        days_available: days.len(),
        files: listed
            .iter()
            .map(|file| FileView {
                day: file.day.to_string(),
                bytes: file.bytes,
                compressed: file.compressed,
            })
            .collect(),
        writer: WriterView {
            active: hub.writer_active.load(std::sync::atomic::Ordering::Relaxed),
            dropped: hub.file_dropped.load(std::sync::atomic::Ordering::Relaxed),
        },
    })
}

pub async fn overview(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
) -> ApiResult<Json<Overview>> {
    admin(&principal)?;
    Ok(Json(overview_of(&state).await?))
}

#[derive(Debug, Deserialize)]
pub struct SettingsPayload {
    pub retention_days: Option<i64>,
    pub max_total_mb: Option<i64>,
}

pub async fn put_settings(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    ClientIp(ip): ClientIp,
    Json(payload): Json<SettingsPayload>,
) -> ApiResult<Json<Overview>> {
    let me = admin(&principal)?;
    let (mut retention, mut max_total) = store::limits(&state.pool).await?;
    if let Some(days) = payload.retention_days {
        let range = i64::from(logs::MIN_RETENTION_DAYS)..=i64::from(logs::MAX_RETENTION_DAYS);
        if !range.contains(&days) {
            return Err(ApiError::BadRequest(format!(
                "Retention must be between {} and {} days.",
                logs::MIN_RETENTION_DAYS,
                logs::MAX_RETENTION_DAYS
            )));
        }
        retention = days as u32;
    }
    if let Some(mb) = payload.max_total_mb {
        let range = logs::MIN_TOTAL_MB as i64..=logs::MAX_TOTAL_MB as i64;
        if !range.contains(&mb) {
            return Err(ApiError::BadRequest(format!(
                "The size cap must be between {} and {} MB.",
                logs::MIN_TOTAL_MB,
                logs::MAX_TOTAL_MB
            )));
        }
        max_total = mb as u64;
    }
    store::save_limits(&state.pool, retention, max_total).await?;
    logs::hub().limits.set(retention, max_total);
    audit::record(
        &state.pool,
        Some(&me),
        "logs.settings",
        Some(&format!("{retention} days, {max_total} MB")),
        ip,
    )
    .await;
    Ok(Json(overview_of(&state).await?))
}

// --------------------------------------------------------------------------
// Export
// --------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct DaysQuery {
    pub days: Option<i64>,
}

async fn check_days(state: &AppState, days: Option<i64>) -> ApiResult<u32> {
    let (retention, _) = store::limits(&state.pool).await?;
    let days = days.unwrap_or(1);
    if days < 1 || days > i64::from(retention) {
        return Err(ApiError::BadRequest(format!(
            "The number of days must be between 1 and the retention ({retention} days)."
        )));
    }
    Ok(days as u32)
}

/// Extrait du journal d'audit sur la période, une ligne JSON par entrée.
async fn audit_extract(state: &AppState, from: NaiveDate) -> ApiResult<Option<Vec<u8>>> {
    let rows = sqlx::query(
        "SELECT at, actor, action, subject, ip FROM audit_log WHERE at >= ? ORDER BY id",
    )
    .bind(from.format("%Y-%m-%d").to_string())
    .fetch_all(&state.pool)
    .await?;
    if rows.is_empty() {
        return Ok(None);
    }
    let mut out = Vec::new();
    for row in rows {
        let line = serde_json::json!({
            "ts": row.try_get::<String, _>("at")?,
            "kind": "audit",
            "action": row.try_get::<String, _>("action")?,
            "actor": row.try_get::<Option<String>, _>("actor")?,
            "subject": row.try_get::<Option<String>, _>("subject")?,
            "ip": row.try_get::<Option<String>, _>("ip")?,
        });
        out.extend_from_slice(line.to_string().as_bytes());
        out.push(b'\n');
    }
    Ok(Some(out))
}

/// Les pièces de l'archive : fichiers du disque, puis l'extrait d'audit.
async fn entries(state: &AppState, days: u32) -> ApiResult<(Vec<Entry>, usize)> {
    let dir = logs_dir(state);
    let today = today();
    let selected = tokio::task::spawn_blocking(move || export::select(&dir, today, days)).await??;
    let count = selected.len();
    let mut entries: Vec<Entry> = selected.iter().map(Entry::from_log).collect();
    let (from, _) = export::period(today, days);
    if let Some(data) = audit_extract(state, from).await? {
        entries.push(Entry::Memory { name: "audit.jsonl".into(), data });
    }
    Ok((entries, count))
}

#[derive(Debug, Serialize)]
pub struct Estimate {
    pub days: u32,
    pub from: String,
    pub to: String,
    pub files: usize,
    pub bytes: u64,
}

pub async fn estimate(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    Query(query): Query<DaysQuery>,
) -> ApiResult<Json<Estimate>> {
    admin(&principal)?;
    let days = check_days(&state, query.days).await?;
    let (entries, files) = entries(&state, days).await?;
    let bytes = tokio::task::spawn_blocking(move || export::estimate(&entries)).await??;
    let (from, to) = export::period(today(), days);
    Ok(Json(Estimate { days, from: from.to_string(), to: to.to_string(), files, bytes }))
}

pub async fn download(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    ClientIp(ip): ClientIp,
    Query(query): Query<DaysQuery>,
) -> ApiResult<Response> {
    let me = admin(&principal)?;
    let days = check_days(&state, query.days).await?;
    let (entries, _) = entries(&state, days).await?;
    let name = format!("dumbmonit-logs-{}-{days}d.tar.gz", today().format("%Y%m%d"));
    audit::record(&state.pool, Some(&me), "logs.exported", Some(&name), ip).await;

    let (tx, rx) = tokio::sync::mpsc::channel::<std::io::Result<axum::body::Bytes>>(4);
    tokio::task::spawn_blocking(move || {
        if let Err(error) = export::write_archive(&entries, export::ChannelWriter::new(tx.clone()))
        {
            // Client parti : rien à signaler. Autre erreur : le flux est coupé,
            // et le navigateur voit un téléchargement échoué.
            if error.kind() != std::io::ErrorKind::BrokenPipe {
                tracing::warn!(%error, "log export interrupted");
                let _ = tx.blocking_send(Err(error));
            }
        }
    });
    let stream =
        futures::stream::unfold(rx, |mut rx| async move { rx.recv().await.map(|item| (item, rx)) });
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/gzip".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{name}\"")),
            (header::CACHE_CONTROL, "no-store".to_string()),
        ],
        Body::from_stream(stream),
    )
        .into_response())
}

// --------------------------------------------------------------------------
// Destinations
// --------------------------------------------------------------------------

pub async fn kinds(
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
) -> ApiResult<Json<Vec<catalog::KindInfo>>> {
    admin(&principal)?;
    Ok(Json(catalog::all()))
}

#[derive(Debug, Serialize)]
pub struct DestinationView {
    #[serde(flatten)]
    pub destination: store::Destination,
    pub stats: ship::StatsView,
}

fn view(destination: store::Destination) -> DestinationView {
    let stats = logs::hub().routes.view(destination.id);
    DestinationView { destination, stats }
}

pub async fn list(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
) -> ApiResult<Json<Vec<DestinationView>>> {
    admin(&principal)?;
    Ok(Json(store::list(&state.pool).await?.into_iter().map(view).collect()))
}

#[derive(Debug, Deserialize)]
pub struct DestinationPayload {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub min_level: Option<String>,
    #[serde(default)]
    pub settings: Option<Value>,
    /// Absent ou `null` à la modification : les secrets enregistrés restent.
    /// `{}` les efface.
    #[serde(default)]
    pub secrets: Option<Value>,
}

/// Ce qu'un formulaire validé enregistre.
struct Checked {
    name: String,
    enabled: bool,
    min_level: String,
    settings: Map<String, Value>,
    secrets: Option<HashMap<String, String>>,
}

fn check(payload: &DestinationPayload) -> ApiResult<Checked> {
    let name = payload.name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(ApiError::BadRequest(
            "Give the destination a name (100 characters at most).".into(),
        ));
    }
    let spec = catalog::spec(&payload.kind).ok_or_else(|| {
        ApiError::BadRequest(format!("Unknown log destination type `{}`.", payload.kind))
    })?;
    let info = (spec.describe)();
    let min_level = payload.min_level.as_deref().unwrap_or("info").trim().to_ascii_lowercase();
    if !Level::CHOICES.contains(&min_level.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Minimum level must be one of: {}.",
            Level::CHOICES.join(", ")
        )));
    }

    let mut settings = Map::new();
    match &payload.settings {
        None | Some(Value::Null) => {}
        Some(Value::Object(given)) => {
            for (key, value) in given {
                let Some(field) = info.settings.iter().find(|field| field.key == key.as_str())
                else {
                    let hint = if info.secrets.iter().any(|field| field.key == key.as_str()) {
                        " It is a secret: send it in `secrets`."
                    } else {
                        ""
                    };
                    return Err(ApiError::BadRequest(format!("Unknown setting `{key}`.{hint}")));
                };
                let value = match value {
                    Value::Null => continue,
                    Value::String(text) if text.trim().is_empty() => continue,
                    Value::String(text) => Value::String(text.trim().to_string()),
                    Value::Bool(_) | Value::Number(_) => value.clone(),
                    _ => {
                        return Err(ApiError::BadRequest(format!(
                            "Setting `{key}` must be a text, a number or a boolean."
                        )));
                    }
                };
                if field.input == "select"
                    && !value.as_str().is_some_and(|text| field.options.contains(&text))
                {
                    return Err(ApiError::BadRequest(format!(
                        "`{key}` must be one of: {}.",
                        field.options.join(", ")
                    )));
                }
                settings.insert(key.clone(), value);
            }
        }
        Some(_) => return Err(ApiError::BadRequest("`settings` must be an object.".into())),
    }
    if let Some(field) =
        info.settings.iter().find(|field| field.required && !settings.contains_key(field.key))
    {
        return Err(ApiError::BadRequest(format!("{} is required.", field.label)));
    }

    let secrets = match &payload.secrets {
        None | Some(Value::Null) => None,
        Some(Value::Object(given)) => {
            let mut secrets = HashMap::new();
            for (key, value) in given {
                if !info.secrets.iter().any(|field| field.key == key.as_str()) {
                    return Err(ApiError::BadRequest(format!("Unknown secret `{key}`.")));
                }
                match value {
                    Value::String(text) if !text.is_empty() => {
                        secrets.insert(key.clone(), text.clone());
                    }
                    Value::String(_) | Value::Null => {}
                    _ => {
                        return Err(ApiError::BadRequest(format!(
                            "Secret `{key}` must be a text."
                        )));
                    }
                }
            }
            Some(secrets)
        }
        Some(_) => return Err(ApiError::BadRequest("`secrets` must be an object.".into())),
    };

    Ok(Checked {
        name: name.to_string(),
        enabled: payload.enabled.unwrap_or(true),
        min_level,
        settings,
        secrets,
    })
}

/// Construit l'expéditeur pour valider la configuration complète.
fn try_build(
    kind: &str,
    settings: &Map<String, Value>,
    secrets: &HashMap<String, String>,
) -> ApiResult<()> {
    let config = DestinationConfig { settings: settings.clone(), secrets: secrets.clone() };
    catalog::build(kind, &config).map(|_| ()).map_err(ApiError::BadRequest)
}

async fn reload(state: &AppState) {
    if let Err(error) = ship::reload(&state.pool, &state.cipher).await {
        tracing::warn!(error = %format!("{error:#}"), "log destinations could not be reloaded");
    }
}

async fn one(state: &AppState, id: i64) -> ApiResult<store::Destination> {
    store::get(&state.pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Log destination {id} not found.")))
}

pub async fn create(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    ClientIp(ip): ClientIp,
    Json(payload): Json<DestinationPayload>,
) -> ApiResult<(StatusCode, Json<DestinationView>)> {
    let me = admin(&principal)?;
    let checked = check(&payload)?;
    let secrets = checked.secrets.clone().unwrap_or_default();
    try_build(&payload.kind, &checked.settings, &secrets)?;
    let input = store::Input {
        name: &checked.name,
        kind: &payload.kind,
        enabled: checked.enabled,
        min_level: &checked.min_level,
        settings: &checked.settings,
    };
    let id = store::insert(&state.pool, &state.cipher, &input, &secrets).await?;
    audit::record(&state.pool, Some(&me), "logs.destination.created", Some(&checked.name), ip)
        .await;
    reload(&state).await;
    Ok((StatusCode::CREATED, Json(view(one(&state, id).await?))))
}

pub async fn update(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
    Json(payload): Json<DestinationPayload>,
) -> ApiResult<Json<DestinationView>> {
    let me = admin(&principal)?;
    let existing = one(&state, id).await?;
    let checked = check(&payload)?;

    // Les secrets ne suivent pas une destination qui change d'adresse : ils
    // seraient envoyés à un serveur qu'ils n'ont jamais connu.
    if checked.secrets.is_none() && existing.has_secret {
        let moved = existing.kind != payload.kind
            || catalog::destination_keys(&payload.kind)
                .iter()
                .any(|key| existing.settings.get(*key) != checked.settings.get(*key));
        if moved {
            return Err(ApiError::BadRequest(
                "The destination address changed: enter the password or token again.".into(),
            ));
        }
    }
    let secrets = match &checked.secrets {
        Some(secrets) => secrets.clone(),
        None => store::secrets(&state.pool, &state.cipher, id).await?,
    };
    try_build(&payload.kind, &checked.settings, &secrets)?;
    let input = store::Input {
        name: &checked.name,
        kind: &payload.kind,
        enabled: checked.enabled,
        min_level: &checked.min_level,
        settings: &checked.settings,
    };
    store::update(&state.pool, &state.cipher, id, &input, checked.secrets.as_ref()).await?;
    audit::record(&state.pool, Some(&me), "logs.destination.updated", Some(&checked.name), ip)
        .await;
    reload(&state).await;
    Ok(Json(view(one(&state, id).await?)))
}

pub async fn remove(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    let me = admin(&principal)?;
    let existing = one(&state, id).await?;
    store::delete(&state.pool, id).await?;
    audit::record(&state.pool, Some(&me), "logs.destination.deleted", Some(&existing.name), ip)
        .await;
    reload(&state).await;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct TestReport {
    pub ok: bool,
    pub message: String,
}

pub async fn test(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    Path(id): Path<i64>,
) -> ApiResult<Json<TestReport>> {
    admin(&principal)?;
    let destination = one(&state, id).await?;
    let config = store::config(&state.pool, &state.cipher, &destination).await?;
    let report = match catalog::build(&destination.kind, &config) {
        Err(error) => TestReport { ok: false, message: error },
        Ok(sink) => match ship::test(sink).await {
            Ok(()) => TestReport { ok: true, message: "Test message sent.".into() },
            Err(error) => TestReport { ok: false, message: logs::scrub::text(&error) },
        },
    };
    Ok(Json(report))
}
