//! Configuration en fichiers : plan, application, export (voir [`crate::gitops`]).
//!
//! Réservé aux administrateurs, session ou jeton d'API `write` : c'est ce que
//! l'outil en ligne de commande et le fournisseur Terraform présentent. Rien
//! ici ne rend un secret en clair — l'export ne contient que des références, et
//! le plan ne nomme que des champs — ce qui permet, contrairement à la
//! sauvegarde, d'ouvrir ces routes aux jetons.
//!
//! Le corps d'un plan ou d'une application porte soit `yaml` (le texte du
//! fichier, tel qu'on le colle dans l'interface), soit `config` (le même
//! document en JSON, déjà lu et résolu par l'outil en ligne de commande).

use axum::extract::{Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::{ApiError, ApiResult};
use crate::auth::audit;
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::{CurrentPrincipal, Principal};
use crate::gitops::apply::{self, ApplyError};
use crate::gitops::plan::{self, Plan};
use crate::gitops::{ConfigDoc, MANAGED_BY_FILE, SCHEMA_VERSION, refs, watch};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/config/plan", post(plan_config))
        .route("/config/apply", post(apply_config))
        .route("/config/export", get(export_config))
        .route("/config/status", get(status))
        .route("/config/release", post(release))
}

fn admin(principal: &Principal) -> ApiResult<()> {
    if principal.is_admin() {
        Ok(())
    } else {
        Err(ApiError::Forbidden(
            "Only an administrator (or an API token with write scope) can read or apply the \
             configuration files."
                .into(),
        ))
    }
}

fn refused(error: ApplyError) -> ApiError {
    match error {
        ApplyError::Invalid(message) => ApiError::BadRequest(message),
        ApplyError::Internal(error) => ApiError::Internal(error),
    }
}

#[derive(Debug, Deserialize)]
pub struct ConfigRequest {
    /// Le fichier tel quel (YAML).
    #[serde(default)]
    pub yaml: Option<String>,
    /// Ou le document déjà lu, en JSON.
    #[serde(default)]
    pub config: Option<Value>,
    /// Supprimer ce qui est géré par fichier et n'est plus déclaré.
    #[serde(default)]
    pub prune: bool,
}

impl ConfigRequest {
    fn document(self) -> ApiResult<(ConfigDoc, bool)> {
        let doc = match (self.yaml, self.config) {
            (Some(text), None) => ConfigDoc::parse_yaml(&text),
            (None, Some(value)) => ConfigDoc::from_json(value),
            _ => Err("Send either \"yaml\" (the file) or \"config\" (the same document as JSON)."
                .to_string()),
        };
        Ok((doc.map_err(ApiError::BadRequest)?, self.prune))
    }
}

#[derive(Debug, Serialize)]
pub struct PlanReply {
    #[serde(flatten)]
    pub plan: Plan,
    /// Vrai : le plan a été appliqué.
    pub applied: bool,
    /// Objets dont une référence de secret n'a pas été résolue (plan seulement :
    /// le secret y est supposé inchangé).
    pub unresolved: Vec<String>,
}

/// `POST /api/config/plan` : ce que le fichier changerait, sans rien écrire.
pub async fn plan_config(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    Json(request): Json<ConfigRequest>,
) -> ApiResult<Json<PlanReply>> {
    admin(&principal)?;
    let (mut doc, prune) = request.document()?;
    let unresolved = refs::unresolved(&mut doc);
    let plan = apply::plan(&state, doc, prune).await.map_err(refused)?;
    Ok(Json(PlanReply { plan, applied: false, unresolved }))
}

/// `POST /api/config/apply` : applique le fichier, en une transaction.
pub async fn apply_config(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    ClientIp(ip): ClientIp,
    Json(request): Json<ConfigRequest>,
) -> ApiResult<Json<PlanReply>> {
    admin(&principal)?;
    let (doc, prune) = request.document()?;
    let plan = apply::apply(&state, doc, prune, &principal.label(), ip).await.map_err(refused)?;
    Ok(Json(PlanReply { plan, applied: true, unresolved: Vec::new() }))
}

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    /// `yaml` (défaut) ou `json`.
    #[serde(default)]
    pub format: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExportReply {
    pub version: u32,
    pub yaml: String,
    pub config: ConfigDoc,
    /// Variables à définir avant d'appliquer ce fichier.
    pub variables: Vec<String>,
}

/// `GET /api/config/export` : l'instance sous forme de fichier, secrets remplacés
/// par des références.
pub async fn export_config(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    Query(query): Query<ExportQuery>,
) -> ApiResult<Response> {
    admin(&principal)?;
    let current = plan::load_current(&state.pool, &state.cipher).await?;
    let mut doc = current.doc;
    let variables = refs::redact(&mut doc);
    let body = doc.to_yaml().map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;

    let mut yaml = format!(
        "# DumbMonit configuration (schema {SCHEMA_VERSION}), exported {} UTC.\n# Secrets are \
         references, never values.\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M")
    );
    if !variables.is_empty() {
        yaml.push_str(
            "# Define these variables (or replace them with ${file:/path}) before apply:\n",
        );
        for name in &variables {
            yaml.push_str(&format!("#   {name}\n"));
        }
    }
    yaml.push_str(&body);

    if query.format.as_deref() == Some("json") {
        let reply = ExportReply { version: SCHEMA_VERSION, yaml, config: doc, variables };
        return Ok(Json(reply).into_response());
    }
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/yaml; charset=utf-8".to_string()),
            (header::CACHE_CONTROL, "no-store".to_string()),
        ],
        yaml,
    )
        .into_response())
}

#[derive(Debug, Serialize)]
pub struct StatusReply {
    pub schema_version: u32,
    /// Dossier surveillé (`DUMBMONIT_CONFIG_DIR`), s'il y en a un.
    pub watch: watch::WatchStatus,
    /// Objets gérés par fichier, par nature (`target`, `rule`, `channel`…).
    pub managed: std::collections::BTreeMap<String, i64>,
    /// Clés de ces objets (uid d'une règle, nom d'un canal, slug d'une page…),
    /// pour le badge « géré par fichier » des listes.
    pub keys: std::collections::BTreeMap<String, Vec<String>>,
}

/// `GET /api/config/status` : dossier surveillé et objets gérés par fichier.
pub async fn status(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
) -> ApiResult<Json<StatusReply>> {
    admin(&principal)?;
    Ok(Json(StatusReply {
        schema_version: SCHEMA_VERSION,
        watch: watch::status(),
        managed: apply::managed_counts(&state).await?,
        keys: apply::managed_keys(&state).await?,
    }))
}

#[derive(Debug, Serialize)]
pub struct ReleaseReply {
    pub released: u64,
}

/// `POST /api/config/release` : rend tout à la gestion manuelle, sans rien supprimer.
pub async fn release(
    State(state): State<AppState>,
    Extension(CurrentPrincipal(principal)): Extension<CurrentPrincipal>,
    ClientIp(ip): ClientIp,
) -> ApiResult<Json<ReleaseReply>> {
    admin(&principal)?;
    let released = apply::release_all(&state).await?;
    audit::record(
        &state.pool,
        Some(&principal.label()),
        "config.released",
        Some(MANAGED_BY_FILE),
        ip,
    )
    .await;
    Ok(Json(ReleaseReply { released }))
}
