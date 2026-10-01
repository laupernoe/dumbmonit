//! Paquets d'intégration : lister, installer, activer, désinstaller.
//!
//! Réservé aux administrateurs (le garde de session l'impose déjà pour les
//! écritures, avec l'en-tête anti-CSRF) et aux jetons d'API — en écriture pour
//! installer. Installer un paquet, c'est ajouter un type d'équipement : les
//! requêtes qu'il décrit partent vers les adresses des équipements qui
//! l'utilisent, jamais ailleurs (voir `dumbmonit-pack`).
//!
//! Le corps de `POST /api/packs` est le YAML du paquet tel quel, ou
//! `{"yaml": "…"}` en JSON pour un client qui n'envoie que du JSON.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::routing::{get, put};
use axum::{Extension, Json, Router};
use dumbmonit_pack::Pack;
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::auth::audit;
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::{CurrentPrincipal, Principal};
use crate::packs::{self, Installed, Saved};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/packs", get(list).post(install))
        .route("/packs/{id}", get(get_one).delete(uninstall))
        .route("/packs/{id}/enable", put(enable))
        .route("/packs/{id}/disable", put(disable))
}

/// Un compte administrateur, ou un jeton d'API (dont la portée est vérifiée par
/// le garde pour les écritures).
fn admin(principal: &Principal) -> ApiResult<String> {
    match principal {
        Principal::User(user) if user.role.is_admin() => Ok(user.username.clone()),
        Principal::User(_) => {
            Err(ApiError::Forbidden("Only an administrator can manage integration packs.".into()))
        }
        Principal::Token(_) => Ok(principal.label()),
    }
}

/// Un paquet installé, tel que l'API le montre.
#[derive(Debug, Serialize)]
pub struct PackView {
    pub id: String,
    pub version: String,
    pub label: String,
    pub summary: String,
    /// Type de cible (`pack.<id>`) ; `null` pour un paquet qui n'apporte que
    /// des profils SNMP.
    pub kind: Option<String>,
    pub enabled: bool,
    pub installed_at: String,
    pub sha256: String,
    /// Noms complets des métriques produites.
    pub metrics: Vec<String>,
    /// `uid` des règles d'alerte livrées.
    pub rules: Vec<String>,
    pub snmp_profiles: Vec<String>,
    /// Équipements qui utilisent ce type.
    pub targets: i64,
    pub warnings: Vec<String>,
    /// Renseigné quand le YAML enregistré ne passe plus la vérification (après
    /// une mise à jour du serveur, par exemple) : le paquet est alors inactif.
    pub error: Option<String>,
}

async fn view(state: &AppState, row: Installed) -> ApiResult<PackView> {
    let targets = packs::targets_using(&state.pool, &row.id).await?;
    Ok(match Pack::parse(&row.yaml) {
        Ok(pack) => PackView {
            label: pack.label().to_string(),
            summary: pack.summary().to_string(),
            kind: pack.has_collector().then(|| pack.kind().to_string()),
            metrics: pack.metric_names().map(str::to_string).collect(),
            rules: pack.rules().iter().map(|rule| rule.uid.clone()).collect(),
            snmp_profiles: pack.snmp_profiles().iter().map(|p| p.id.clone()).collect(),
            warnings: pack.warnings().to_vec(),
            error: None,
            id: row.id,
            version: row.version,
            enabled: row.enabled,
            installed_at: row.installed_at,
            sha256: row.sha256,
            targets,
        },
        Err(error) => PackView {
            label: row.id.clone(),
            summary: String::new(),
            kind: None,
            metrics: Vec::new(),
            rules: Vec::new(),
            snmp_profiles: Vec::new(),
            warnings: Vec::new(),
            error: Some(error.to_string()),
            id: row.id,
            version: row.version,
            enabled: row.enabled,
            installed_at: row.installed_at,
            sha256: row.sha256,
            targets,
        },
    })
}

fn not_found(id: &str) -> ApiError {
    ApiError::NotFound(format!("No integration pack \"{id}\" is installed."))
}

// --------------------------------------------------------------------------
// GET /api/packs, GET /api/packs/{id}
// --------------------------------------------------------------------------

pub async fn list(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
) -> ApiResult<Json<Vec<PackView>>> {
    admin(&principal)?;
    let mut views = Vec::new();
    for row in packs::list(&state.pool).await? {
        views.push(view(&state, row).await?);
    }
    Ok(Json(views))
}

pub async fn get_one(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    Path(id): Path<String>,
) -> ApiResult<Json<PackView>> {
    admin(&principal)?;
    let row = packs::get(&state.pool, &id).await?.ok_or_else(|| not_found(&id))?;
    Ok(Json(view(&state, row).await?))
}

// --------------------------------------------------------------------------
// POST /api/packs — installer ou mettre à jour
// --------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonBody {
    yaml: String,
}

#[derive(Debug, Serialize)]
pub struct InstallReport {
    pub outcome: Saved,
    pub pack: PackView,
    /// Règles ajoutées ; une règle déjà présente n'est jamais réécrite.
    pub rules_added: usize,
    /// Vrai quand le paquet apporte des profils SNMP : le catalogue SNMP est lu
    /// au démarrage, ils ne servent qu'après un redémarrage.
    pub restart_required: bool,
}

fn yaml_of(headers: &HeaderMap, body: &Bytes) -> ApiResult<String> {
    let text = std::str::from_utf8(body)
        .map_err(|_| ApiError::BadRequest("The pack must be UTF-8 text.".into()))?;
    let is_json = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"));
    if !is_json {
        return Ok(text.to_string());
    }
    // Un JSON qui n'est pas l'enveloppe `{"yaml": …}` est lu comme le paquet
    // lui-même : le JSON est du YAML valide.
    Ok(serde_json::from_str::<JsonBody>(text).map(|body| body.yaml).unwrap_or_else(|_| text.into()))
}

pub async fn install(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    ClientIp(ip): ClientIp,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<InstallReport>)> {
    let actor = admin(&principal)?;
    let yaml = yaml_of(&headers, &body)?;
    let pack = Pack::parse(&yaml).map_err(|error| {
        ApiError::BadRequest(format!(
            "This pack cannot be installed:\n- {}",
            error.errors.join("\n- ")
        ))
    })?;
    if state.collectors.get(pack.kind()).is_some()
        && !state.collectors.runtime_kinds().iter().any(|kind| &**kind == pack.kind())
    {
        return Err(ApiError::Conflict(format!("\"{}\" is a built-in device type.", pack.kind())));
    }

    let outcome = packs::save(&state.pool, &pack).await?;
    let rules_added = packs::install_rules(&state.pool, &pack).await?;
    let row = packs::get(&state.pool, pack.id()).await?.ok_or_else(|| not_found(pack.id()))?;
    let pack = std::sync::Arc::new(pack);
    if row.enabled {
        packs::activate(&state.collectors, &pack).map_err(ApiError::Conflict)?;
    }
    let restart_required = outcome != Saved::Unchanged && !pack.snmp_profiles().is_empty();

    if outcome != Saved::Unchanged {
        tracing::info!(actor = %actor, pack = pack.id(), version = pack.version(), ?outcome, "integration pack installed");
        audit::record(
            &state.pool,
            Some(&actor),
            "pack.installed",
            Some(&format!("{} {}", pack.id(), pack.version())),
            ip,
        )
        .await;
    }
    let status = if outcome == Saved::Created { StatusCode::CREATED } else { StatusCode::OK };
    Ok((
        status,
        Json(InstallReport {
            outcome,
            pack: view(&state, row).await?,
            rules_added,
            restart_required,
        }),
    ))
}

// --------------------------------------------------------------------------
// PUT /api/packs/{id}/enable|disable
// --------------------------------------------------------------------------

pub async fn enable(
    state: State<AppState>,
    principal: Extension<CurrentPrincipal>,
    ip: ClientIp,
    Path(id): Path<String>,
) -> ApiResult<Json<PackView>> {
    toggle(state, principal, ip, id, true).await
}

pub async fn disable(
    state: State<AppState>,
    principal: Extension<CurrentPrincipal>,
    ip: ClientIp,
    Path(id): Path<String>,
) -> ApiResult<Json<PackView>> {
    toggle(state, principal, ip, id, false).await
}

async fn toggle(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    ClientIp(ip): ClientIp,
    id: String,
    enabled: bool,
) -> ApiResult<Json<PackView>> {
    let actor = admin(&principal)?;
    let row = packs::get(&state.pool, &id).await?.ok_or_else(|| not_found(&id))?;
    if enabled {
        let pack = Pack::parse(&row.yaml).map_err(|error| {
            ApiError::Conflict(format!("This pack no longer passes validation:\n{error}"))
        })?;
        packs::activate(&state.collectors, &std::sync::Arc::new(pack))
            .map_err(ApiError::Conflict)?;
    } else {
        packs::deactivate(&state.collectors, &id);
    }
    packs::set_enabled(&state.pool, &id, enabled).await?;
    let action = if enabled { "pack.enabled" } else { "pack.disabled" };
    audit::record(&state.pool, Some(&actor), action, Some(&id), ip).await;
    let row = packs::get(&state.pool, &id).await?.ok_or_else(|| not_found(&id))?;
    Ok(Json(view(&state, row).await?))
}

// --------------------------------------------------------------------------
// DELETE /api/packs/{id}
// --------------------------------------------------------------------------

pub async fn uninstall(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    ClientIp(ip): ClientIp,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    let actor = admin(&principal)?;
    if packs::get(&state.pool, &id).await?.is_none() {
        return Err(not_found(&id));
    }
    let used = packs::targets_using(&state.pool, &id).await?;
    if used > 0 {
        return Err(ApiError::Conflict(format!(
            "{used} device(s) still use this pack. Delete them first, or disable the pack \
             (PUT /api/packs/{id}/disable) to keep them."
        )));
    }
    packs::deactivate(&state.collectors, &id);
    packs::remove(&state.pool, &id).await?;
    audit::record(&state.pool, Some(&actor), "pack.uninstalled", Some(&id), ip).await;
    Ok(StatusCode::NO_CONTENT)
}
