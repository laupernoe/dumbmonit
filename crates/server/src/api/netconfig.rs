//! Sauvegarde des configurations réseau : versions, diff, téléchargement et
//! clé d'hôte d'un équipement `netconfig` (`crate::netconfig`).
//!
//! Les droits, plus serrés que pour le reste des lectures :
//!
//! - la liste des versions (dates, tailles, lignes changées) et l'état du
//!   relevé : tous les rôles ;
//! - le contenu d'une version et les diffs, **secrets masqués** : opérateur ou
//!   administrateur (ou jeton `write`) ;
//! - le contenu en clair (`unmasked=true`) et le téléchargement d'une version
//!   brute : administrateur **connecté** seulement, jamais un jeton d'API, et
//!   chaque geste est inscrit au journal d'audit ;
//! - accepter une nouvelle clé d'hôte SSH : administrateur connecté, audité.
//!
//! Une cible qui n'est pas de type `netconfig` répond 404, comme une cible
//! inexistante.

use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use dumbmonit_collectors::netconfig::profile;
use dumbmonit_proto::{Target, TargetId};
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::auth::audit;
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::{CurrentPrincipal, Principal};
use crate::auth::teams::{Level, TeamAccess};
use crate::auth::AuthError;
use crate::db;
use crate::netconfig::{self, store};
use crate::state::AppState;
use crate::webchange::diff;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/targets/{id}/netconfig", get(overview))
        .route("/targets/{id}/netconfig/versions/{version_id}", get(version))
        .route("/targets/{id}/netconfig/versions/{version_id}/download", get(download))
        .route("/targets/{id}/netconfig/diff", get(compare))
        .route("/targets/{id}/netconfig/fetch", post(fetch_now))
        .route("/targets/{id}/netconfig/host-key/accept", post(accept_host_key))
}

/// Versions listées au plus.
const MAX_VERSIONS: i64 = 1000;

#[derive(Serialize)]
struct VersionView {
    id: i64,
    fetched_at: String,
    last_seen_at: String,
    profile: String,
    size_bytes: i64,
    lines: i64,
    added: i64,
    removed: i64,
}

impl From<store::Version> for VersionView {
    fn from(row: store::Version) -> Self {
        Self {
            id: row.id,
            fetched_at: row.fetched_at,
            last_seen_at: row.last_seen_at,
            profile: row.profile,
            size_bytes: row.size_bytes,
            lines: row.lines,
            added: row.added,
            removed: row.removed,
        }
    }
}

#[derive(Serialize)]
struct HostKeyView {
    /// Empreinte mémorisée (`SHA256:…`), absente avant le premier relevé SSH.
    pinned: Option<String>,
    pinned_at: Option<String>,
    /// Empreinte différente présentée par l'équipement et refusée ; présente,
    /// elle attend la décision d'un administrateur.
    changed_to: Option<String>,
    changed_at: Option<String>,
}

#[derive(Serialize)]
struct Overview {
    profile: Option<String>,
    profile_label: Option<String>,
    /// `ssh_shell`, `ssh_exec` ou `http`.
    transport: Option<&'static str>,
    running: bool,
    last_attempt_at: Option<String>,
    last_success_at: Option<String>,
    last_error: Option<String>,
    /// `down`, `auth`, `config` ou `host_key`.
    last_error_kind: Option<String>,
    host_key: HostKeyView,
    /// Le visiteur peut lire le contenu (masqué) des versions.
    can_read: bool,
    /// Le visiteur peut lever le masquage et télécharger.
    can_reveal: bool,
    versions: Vec<VersionView>,
}

#[derive(Deserialize)]
struct RevealQuery {
    #[serde(default)]
    unmasked: bool,
}

#[derive(Deserialize)]
struct CompareQuery {
    /// Version de départ ; par défaut, celle qui précède `to`.
    #[serde(default)]
    from: Option<i64>,
    to: i64,
    #[serde(default)]
    unmasked: bool,
}

#[derive(Serialize)]
struct VersionContent {
    version: VersionView,
    masked: bool,
    /// Valeurs masquées dans ce texte.
    masked_values: usize,
    content: String,
}

#[derive(Serialize)]
struct CompareView {
    from: Option<VersionView>,
    to: VersionView,
    masked: bool,
    added: usize,
    removed: usize,
    diff: Vec<diff::DiffLine>,
}

fn transport_name(transport: profile::Transport) -> &'static str {
    match transport {
        profile::Transport::SshShell => "ssh_shell",
        profile::Transport::SshExec => "ssh_exec",
        profile::Transport::Http => "http",
    }
}

/// Lire le contenu masqué : opérateur, administrateur, jeton `write`.
fn require_reader(principal: &Principal) -> ApiResult<()> {
    if principal.can_operate() {
        Ok(())
    } else {
        Err(ApiError::Forbidden(
            "Reading device configurations needs the operator or admin role.".into(),
        ))
    }
}

/// Le contenu d'une version, même masqué, décrit l'équipement en détail : il
/// demande le niveau « traiter » (operate) sur son dossier, quand la seule liste
/// des versions se contente de la lecture (vérifiée par le garde d'API).
async fn require_operate(state: &AppState, access: &crate::auth::teams::Access, id: TargetId) -> ApiResult<()> {
    match access.require_target(&state.pool, id, Level::Operate).await {
        Ok(()) => Ok(()),
        Err(AuthError::NotFound(message)) => Err(ApiError::NotFound(message)),
        Err(AuthError::Forbidden(message)) => Err(ApiError::Forbidden(message)),
        Err(_) => Err(ApiError::Forbidden("Reading this configuration is not allowed.".into())),
    }
}

/// Lever le masquage, télécharger, accepter une clé : un administrateur
/// connecté, jamais un jeton.
fn require_admin_session(principal: &Principal) -> ApiResult<String> {
    match principal {
        Principal::User(user) if user.role.is_admin() => Ok(user.username.clone()),
        Principal::User(_) => Err(ApiError::Forbidden(
            "Only an administrator can see device secrets or download a configuration.".into(),
        )),
        Principal::Token(_) => Err(ApiError::Forbidden(
            "A configuration in clear holds device secrets: it is only shown to an \
             administrator signed in to the web interface, never to an API token."
                .into(),
        )),
    }
}

async fn load(state: &AppState, id: TargetId) -> ApiResult<Target> {
    let not_found = || ApiError::NotFound(format!("Configuration backup {id} not found."));
    let target = db::targets::get(&state.pool, &state.cipher, id).await?.ok_or_else(not_found)?;
    if target.kind != netconfig::KIND {
        return Err(not_found());
    }
    Ok(target)
}

/// Texte d'une version, masqué ou non.
async fn version_text(
    state: &AppState,
    id: TargetId,
    version: &store::Version,
    unmasked: bool,
) -> ApiResult<(String, usize)> {
    let sealed = store::content(&state.pool, id, version.id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Version {} not found.", version.id)))?;
    let text = netconfig::open(&state.cipher, &sealed)?;
    if unmasked {
        return Ok((text, 0));
    }
    Ok(netconfig::mask(&state.cipher, &version.profile, &text))
}

/// `GET /api/targets/{id}/netconfig`
async fn overview(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<Overview>> {
    let target = load(&state, id).await?;
    let profile = target.tags.get("profile").and_then(|id| profile::find(id.trim()));
    let current = store::state(&state.pool, id).await?.unwrap_or_default();
    let versions = store::versions(&state.pool, id, MAX_VERSIONS).await?;
    Ok(Json(Overview {
        profile: profile.map(|p| p.id.clone()),
        profile_label: profile.map(|p| p.label.clone()),
        transport: profile.map(|p| transport_name(p.transport)),
        running: current.running_since.is_some(),
        last_attempt_at: current.last_attempt_at,
        last_success_at: current.last_success_at,
        last_error: current.last_error,
        last_error_kind: current.last_error_kind,
        host_key: HostKeyView {
            pinned: current.host_key,
            pinned_at: current.host_key_pinned_at,
            changed_to: current.host_key_seen,
            changed_at: current.host_key_seen_at,
        },
        can_read: principal.can_operate(),
        can_reveal: matches!(&principal, Principal::User(user) if user.role.is_admin()),
        versions: versions.into_iter().map(VersionView::from).collect(),
    }))
}

/// `GET /api/targets/{id}/netconfig/versions/{version_id}?unmasked=…`
async fn version(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    TeamAccess(access): TeamAccess,
    ClientIp(ip): ClientIp,
    Path((id, version_id)): Path<(TargetId, i64)>,
    Query(query): Query<RevealQuery>,
) -> ApiResult<Json<VersionContent>> {
    require_reader(&principal)?;
    require_operate(&state, &access, id).await?;
    let actor = if query.unmasked { Some(require_admin_session(&principal)?) } else { None };
    let target = load(&state, id).await?;
    let row = store::version(&state.pool, id, version_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Version {version_id} not found.")))?;
    let (content, masked_values) = version_text(&state, id, &row, query.unmasked).await?;
    if let Some(actor) = actor {
        let subject = format!("{} (version {version_id})", target.name);
        audit::record(&state.pool, Some(&actor), "netconfig.revealed", Some(&subject), ip).await;
    }
    Ok(Json(VersionContent {
        version: row.into(),
        masked: !query.unmasked,
        masked_values,
        content,
    }))
}

/// `GET /api/targets/{id}/netconfig/diff?from=…&to=…&unmasked=…`
async fn compare(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    TeamAccess(access): TeamAccess,
    ClientIp(ip): ClientIp,
    Path(id): Path<TargetId>,
    Query(query): Query<CompareQuery>,
) -> ApiResult<Json<CompareView>> {
    require_reader(&principal)?;
    require_operate(&state, &access, id).await?;
    let actor = if query.unmasked { Some(require_admin_session(&principal)?) } else { None };
    let target = load(&state, id).await?;
    let missing = |version: i64| ApiError::NotFound(format!("Version {version} not found."));
    let to = store::version(&state.pool, id, query.to).await?.ok_or_else(|| missing(query.to))?;
    let from = match query.from {
        Some(from) => {
            Some(store::version(&state.pool, id, from).await?.ok_or_else(|| missing(from))?)
        }
        None => store::previous(&state.pool, id, to.id).await?,
    };
    let after = version_text(&state, id, &to, query.unmasked).await?.0;
    let before = match &from {
        Some(from) => version_text(&state, id, from, query.unmasked).await?.0,
        None => String::new(),
    };
    // Le diff se calcule à la demande, sur les textes déjà masqués : un
    // secret changé apparaît comme une ligne modifiée, sans être révélé.
    let before_lines: Vec<&str> = before.lines().collect();
    let after_lines: Vec<&str> = after.lines().collect();
    let edits = diff::diff(&before_lines, &after_lines);
    let (added, removed) = diff::counts(&edits);
    let rendered = diff::render(&edits, diff::CONTEXT);
    if let Some(actor) = actor {
        let subject = format!("{} (diff to version {})", target.name, to.id);
        audit::record(&state.pool, Some(&actor), "netconfig.revealed", Some(&subject), ip).await;
    }
    Ok(Json(CompareView {
        from: from.map(VersionView::from),
        to: to.into(),
        masked: !query.unmasked,
        added,
        removed,
        diff: rendered,
    }))
}

/// `GET /api/targets/{id}/netconfig/versions/{version_id}/download` : la
/// version brute, en clair, pour un administrateur connecté.
async fn download(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    ClientIp(ip): ClientIp,
    Path((id, version_id)): Path<(TargetId, i64)>,
) -> ApiResult<Response> {
    let actor = require_admin_session(&principal)?;
    let target = load(&state, id).await?;
    let row = store::version(&state.pool, id, version_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Version {version_id} not found.")))?;
    let (content, _) = version_text(&state, id, &row, true).await?;
    let subject = format!("{} (version {version_id})", target.name);
    audit::record(&state.pool, Some(&actor), "netconfig.downloaded", Some(&subject), ip).await;

    // Nom de fichier fait de caractères sûrs seulement : le nom de
    // l'équipement est libre.
    let safe: String = target
        .name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .take(60)
        .collect();
    let stamp: String = row.fetched_at.chars().filter(char::is_ascii_digit).collect();
    let name = format!("{safe}-{stamp}-v{version_id}.txt");
    let mut response = content.into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"));
    if let Ok(value) = HeaderValue::from_str(&format!("attachment; filename=\"{name}\"")) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    // Des secrets en clair n'ont rien à faire dans un cache.
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

#[derive(Serialize)]
struct FetchStarted {
    /// Faux si un relevé était déjà en cours : il tient lieu de celui-ci.
    started: bool,
}

/// `POST /api/targets/{id}/netconfig/fetch` : relever maintenant.
async fn fetch_now(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<(StatusCode, Json<FetchStarted>)> {
    let target = load(&state, id).await?;
    let started = netconfig::start(state.clone(), target).await?;
    Ok((StatusCode::ACCEPTED, Json(FetchStarted { started })))
}

#[derive(Deserialize)]
struct AcceptPayload {
    /// L'empreinte que l'administrateur a lue et accepte : si une autre a été
    /// présentée entre-temps, rien n'est changé.
    fingerprint: String,
}

/// `POST /api/targets/{id}/netconfig/host-key/accept`
async fn accept_host_key(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    ClientIp(ip): ClientIp,
    Path(id): Path<TargetId>,
    Json(payload): Json<AcceptPayload>,
) -> ApiResult<StatusCode> {
    let actor = require_admin_session(&principal)?;
    let target = load(&state, id).await?;
    let fingerprint = payload.fingerprint.trim();
    if !store::accept_host_key(&state.pool, id, fingerprint).await? {
        return Err(ApiError::Conflict(
            "This is no longer the key the device presents: reload the page and check again."
                .into(),
        ));
    }
    let subject = format!("{} ({fingerprint})", target.name);
    audit::record(&state.pool, Some(&actor), "netconfig.host_key_accepted", Some(&subject), ip)
        .await;
    if let Err(error) = netconfig::start(state.clone(), target).await {
        tracing::warn!(target = id, ?error, "configuration backup not restarted");
    }
    Ok(StatusCode::NO_CONTENT)
}
