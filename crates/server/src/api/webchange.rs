//! Changements d'un site web : pages suivies, changements constatés, diff
//! « avant / après » et captures d'écran (`crate::webchange`).
//!
//! Tout est en lecture, sauf `POST …/check` qui lance une vérification sans
//! attendre la prochaine échéance. Une cible qui n'est pas de type
//! `webchange` répond 404, comme une cible inexistante.

use axum::Json;
use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use dumbmonit_proto::{Target, TargetId};
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::db;
use crate::state::AppState;
use crate::webchange::{self, Context, cdp, diff, store};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/targets/{id}/webchange/pages", get(pages))
        .route("/targets/{id}/webchange/changes", get(changes))
        .route("/targets/{id}/webchange/changes/{change_id}", get(change))
        .route("/targets/{id}/webchange/snapshots/{snapshot_id}/screenshot", get(screenshot))
        .route("/targets/{id}/webchange/check", post(check))
}

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 200;

#[derive(Serialize)]
struct PagesView {
    /// Un navigateur est configuré et répond : les nouveaux instantanés
    /// auront une capture.
    screenshots_available: bool,
    pages: Vec<PageView>,
}

#[derive(Serialize)]
struct PageView {
    url: String,
    title: Option<String>,
    status: Option<i64>,
    error: Option<String>,
    last_checked: Option<String>,
    last_changed: Option<String>,
    /// Changements gardés pour cette page.
    changes: i64,
    latest_snapshot: Option<i64>,
}

#[derive(Serialize)]
struct ChangeView {
    id: i64,
    url: String,
    kind: String,
    detected_at: String,
    added: i64,
    removed: i64,
    before: Option<i64>,
    after: Option<i64>,
}

impl From<store::ChangeRow> for ChangeView {
    fn from(row: store::ChangeRow) -> Self {
        Self {
            id: row.id,
            url: row.url,
            kind: row.kind,
            detected_at: row.detected_at,
            added: row.added,
            removed: row.removed,
            before: row.before_snapshot,
            after: row.after_snapshot,
        }
    }
}

#[derive(Serialize)]
struct SnapshotView {
    id: i64,
    fetched_at: String,
    title: Option<String>,
    has_screenshot: bool,
}

impl From<store::SnapshotRow> for SnapshotView {
    fn from(row: store::SnapshotRow) -> Self {
        Self {
            id: row.id,
            fetched_at: row.fetched_at,
            title: row.title,
            has_screenshot: row.has_screenshot,
        }
    }
}

#[derive(Serialize)]
struct ChangeDetail {
    #[serde(flatten)]
    change: ChangeView,
    before_snapshot: Option<SnapshotView>,
    after_snapshot: Option<SnapshotView>,
    diff: Vec<diff::DiffLine>,
}

#[derive(Deserialize)]
struct ChangesQuery {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
}

/// `GET /api/targets/{id}/webchange/pages`
async fn pages(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<PagesView>> {
    load(&state, id).await?;
    let pages = store::pages(&state.pool, id)
        .await?
        .into_iter()
        .map(|page| PageView {
            url: page.url,
            title: page.title,
            status: page.status,
            error: page.error,
            last_checked: page.last_checked,
            last_changed: page.last_changed,
            changes: page.changes,
            latest_snapshot: page.latest_snapshot,
        })
        .collect();
    let screenshots_available = Context::from_state(&state).screenshots_available().await;
    Ok(Json(PagesView { screenshots_available, pages }))
}

/// `GET /api/targets/{id}/webchange/changes?url=…&limit=…`
async fn changes(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
    Query(query): Query<ChangesQuery>,
) -> ApiResult<Json<Vec<ChangeView>>> {
    load(&state, id).await?;
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let url = query.url.as_deref().map(str::trim).filter(|url| !url.is_empty());
    let rows = store::changes(&state.pool, id, url, limit).await?;
    Ok(Json(rows.into_iter().map(ChangeView::from).collect()))
}

/// `GET /api/targets/{id}/webchange/changes/{change_id}`
async fn change(
    State(state): State<AppState>,
    Path((id, change_id)): Path<(TargetId, i64)>,
) -> ApiResult<Json<ChangeDetail>> {
    load(&state, id).await?;
    let row = store::change(&state.pool, id, change_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Change {change_id} not found.")))?;

    let mut before_text = String::new();
    let mut after_text = String::new();
    let mut before_snapshot = None;
    let mut after_snapshot = None;
    if let Some(snapshot) = row.before_snapshot {
        before_snapshot = store::snapshot(&state.pool, id, snapshot).await?;
        before_text = store::snapshot_content(&state.pool, snapshot).await?.unwrap_or_default();
    }
    if let Some(snapshot) = row.after_snapshot {
        after_snapshot = store::snapshot(&state.pool, id, snapshot).await?;
        after_text = store::snapshot_content(&state.pool, snapshot).await?.unwrap_or_default();
    }
    // Le diff se calcule à la demande : garder le texte des deux versions
    // suffit, et un diff stocké ne servirait qu'à cet écran.
    let before_lines: Vec<&str> = before_text.lines().collect();
    let after_lines: Vec<&str> = after_text.lines().collect();
    let edits = diff::diff(&before_lines, &after_lines);
    Ok(Json(ChangeDetail {
        change: row.into(),
        before_snapshot: before_snapshot.map(SnapshotView::from),
        after_snapshot: after_snapshot.map(SnapshotView::from),
        diff: diff::render(&edits, diff::CONTEXT),
    }))
}

/// `GET /api/targets/{id}/webchange/snapshots/{snapshot_id}/screenshot`
async fn screenshot(
    State(state): State<AppState>,
    Path((id, snapshot_id)): Path<(TargetId, i64)>,
) -> ApiResult<Response> {
    load(&state, id).await?;
    let missing = || ApiError::NotFound(format!("Snapshot {snapshot_id} has no screenshot."));
    let snapshot = store::snapshot(&state.pool, id, snapshot_id).await?.ok_or_else(missing)?;
    if !snapshot.has_screenshot {
        return Err(missing());
    }
    let path = store::screenshot_path(&state.config.data_dir, id, snapshot_id);
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Err(missing()),
        Err(error) => return Err(ApiError::Internal(error.into())),
    };
    let mut response = bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(cdp::CONTENT_TYPE));
    // Un instantané ne change jamais : son numéro n'est jamais réutilisé. Mais
    // l'image n'est servie qu'à une session : aucun cache partagé ne la garde.
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=31536000, immutable"),
    );
    Ok(response)
}

#[derive(Serialize)]
struct CheckStarted {
    /// Faux si une vérification était déjà en cours : elle tient lieu de
    /// celle-ci.
    started: bool,
}

/// `POST /api/targets/{id}/webchange/check` : vérifier maintenant.
async fn check(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<(StatusCode, Json<CheckStarted>)> {
    let target = load(&state, id).await?;
    let started = webchange::start_check(Context::from_state(&state), target).await?;
    Ok((StatusCode::ACCEPTED, Json(CheckStarted { started })))
}

async fn load(state: &AppState, id: TargetId) -> ApiResult<Target> {
    let not_found = || ApiError::NotFound(format!("Website watch {id} not found."));
    let target = db::targets::get(&state.pool, &state.cipher, id).await?.ok_or_else(not_found)?;
    if target.kind != webchange::KIND {
        return Err(not_found());
    }
    Ok(target)
}
