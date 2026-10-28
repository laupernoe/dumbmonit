//! Actions sur les conteneurs d'une machine équipée de l'agent.
//!
//! Deux façades sur la même file (`collectors/agent/commands.rs`) :
//!
//! - [`agent_routes`] : ce que l'agent appelle, avec son jeton d'enregistrement
//!   et sa clé d'identité — jamais une session. À merger dans le routeur
//!   public, à côté de `/api/ingest`, pour les mêmes raisons.
//! - [`ui_routes`] : ce que l'interface appelle, sous session ; les écritures
//!   sont réservées aux administrateurs par le garde de session lui-même.

use std::collections::{BTreeMap, BTreeSet};

use axum::extract::{Extension, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use dumbmonit_proto::{
    AGENT_KEY_HEADER, AGENT_SECRET_HEADER, AgentCommand, CMD_CONTAINER_RESTART,
    CMD_CONTAINER_UPDATE, CommandReport, TargetId,
};
use serde::{Deserialize, Serialize};

use crate::agent_tls::AgentCert;
use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::CurrentPrincipal;
use crate::collectors::agent::commands::{self, CommandError, CommandRecord, ContainerPolicy};
use crate::collectors::agent::modules::{self, ModuleState};
use crate::collectors::agent::{self as agent};
use crate::db;
use crate::state::AppState;

/// Nombre de commandes montrées dans « actions récentes ».
const RECENT_COMMANDS: i64 = 20;

/// Les conteneurs du moniteur lui-même : il ne se redémarre ni ne se met à jour
/// par son propre canal — il ne serait plus là pour en rendre compte.
///
/// `ezymonit` reste de la partie : les conteneurs créés avant le renommage du
/// produit portent encore ce nom, et sont tout aussi fatals à toucher. La même
/// liste, mot pour mot, vit dans `crates/agent/src/commands.rs` : le serveur
/// refuse d'émettre, l'agent refuse d'exécuter, et ni l'un ni l'autre ne fait
/// confiance à l'autre pour cela.
const RESERVED_PREFIXES: &[&str] = &["dumbmonit", "ezymonit"];

// ------------------------------------------------------------- agent side

/// Refus côté agent, avec le même corps `{"error": …}` que l'ingestion.
pub struct Rejection {
    status: StatusCode,
    message: String,
}

impl Rejection {
    pub fn not_found(message: &str) -> Self {
        Self { status: StatusCode::NOT_FOUND, message: message.to_string() }
    }

    /// La ressource a existé mais n'est plus : un balayage annulé, par exemple.
    pub fn gone(message: &str) -> Self {
        Self { status: StatusCode::GONE, message: message.to_string() }
    }

    /// Refus opposé à une machine qui se réclame de l'identité d'une autre.
    pub fn not_this_machine() -> Self {
        Self { status: StatusCode::FORBIDDEN, message: agent::BINDING_MISMATCH.to_string() }
    }

    /// Refus opposé à une machine jamais liée à son agent.
    pub fn not_bound() -> Self {
        Self { status: StatusCode::FORBIDDEN, message: agent::NOT_BOUND.to_string() }
    }
}

impl axum::response::IntoResponse for Rejection {
    fn into_response(self) -> axum::response::Response {
        (self.status, Json(serde_json::json!({ "error": self.message }))).into_response()
    }
}

impl From<agent::IngestError> for Rejection {
    fn from(error: agent::IngestError) -> Self {
        match error {
            agent::IngestError::Unauthorized => Self {
                status: StatusCode::UNAUTHORIZED,
                message: "Enrollment token missing, unknown or revoked.".to_string(),
            },
            agent::IngestError::Forbidden(why) => {
                Self { status: StatusCode::FORBIDDEN, message: why }
            }
            agent::IngestError::BadRequest(why) => {
                Self { status: StatusCode::BAD_REQUEST, message: why }
            }
            agent::IngestError::Internal(error) => {
                tracing::error!(?error, "internal error on the command channel");
                Self {
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    message: "Internal server error.".to_string(),
                }
            }
        }
    }
}

impl From<anyhow::Error> for Rejection {
    fn from(error: anyhow::Error) -> Self {
        Self::from(agent::IngestError::Internal(error))
    }
}

#[derive(Debug, Deserialize)]
pub struct KeyQuery {
    #[serde(default)]
    key: String,
}

pub fn agent_routes() -> Router<AppState> {
    Router::new().route("/agent/commands", get(pending)).route("/agent/commands/{id}", post(report))
}

/// Vérifie le jeton, puis que la machine qui parle est bien celle de cette clé.
///
/// La clé d'identité voyage en clair (en-tête, ou URL pour un agent ancien) et
/// n'est un secret pour personne : sans le second contrôle, n'importe quelle machine du parc
/// viendrait chercher — donc consommer — les commandes Docker d'une autre.
///
/// Renvoie l'identifiant du jeton et la cible autorisée.
pub async fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
    key: &str,
    cert: Option<&AgentCert>,
) -> Result<(i64, TargetId), Rejection> {
    let bearer =
        headers.get(axum::http::header::AUTHORIZATION).and_then(|value| value.to_str().ok());
    let token_id = agent::authenticate_token(&state.pool, bearer)
        .await?
        .ok_or_else(|| Rejection::from(agent::IngestError::Unauthorized))?;
    let Some(key) = machine_key(headers, key) else {
        return Err(Rejection::from(agent::IngestError::BadRequest(format!(
            "the machine key is required (header {AGENT_KEY_HEADER})"
        ))));
    };
    let secret = headers.get(AGENT_SECRET_HEADER).and_then(|value| value.to_str().ok());
    match agent::authorise_key(&state.pool, key, agent::extract_secret(secret)).await? {
        // Sur le canal chiffré, le certificat client doit être celui de cette
        // machine-là : le sien ne donne pas accès aux commandes d'une autre.
        agent::KeyAuth::Allowed { target_id }
            if cert.is_some_and(|cert| cert.target_id != target_id) =>
        {
            Err(Rejection {
                status: StatusCode::FORBIDDEN,
                message: crate::api::ingest::CERT_OF_ANOTHER_MACHINE.to_string(),
            })
        }
        agent::KeyAuth::Allowed { target_id } => Ok((token_id, target_id)),
        agent::KeyAuth::Denied => Err(Rejection::not_this_machine()),
        agent::KeyAuth::NotBound => Err(Rejection::not_bound()),
        agent::KeyAuth::Unknown => {
            Err(Rejection::not_found("Unknown machine: push a batch first."))
        }
    }
}

/// Clé d'identité de la machine : l'en-tête [`AGENT_KEY_HEADER`] d'abord, le
/// paramètre `?key=` d'un agent antérieur à défaut. Jamais journalisée.
fn machine_key<'a>(headers: &'a HeaderMap, query: &'a str) -> Option<&'a str> {
    headers
        .get(AGENT_KEY_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .or_else(|| Some(query.trim()).filter(|key| !key.is_empty()))
}

/// `GET /api/agent/commands?key=…` : les commandes en attente pour cette machine.
async fn pending(
    State(state): State<AppState>,
    headers: HeaderMap,
    cert: Option<Extension<AgentCert>>,
    Query(query): Query<KeyQuery>,
) -> Result<Json<Vec<AgentCommand>>, Rejection> {
    let (token_id, target_id) = authenticate(&state, &headers, &query.key, cert.as_deref()).await?;
    let commands = commands::pending_for_target(&state.pool, target_id).await?;
    agent::touch_token(&state.pool, token_id).await?;
    Ok(Json(commands))
}

/// `POST /api/agent/commands/{id}?key=…` : compte rendu de l'agent.
async fn report(
    State(state): State<AppState>,
    headers: HeaderMap,
    cert: Option<Extension<AgentCert>>,
    Path(id): Path<i64>,
    Query(query): Query<KeyQuery>,
    Json(report): Json<CommandReport>,
) -> Result<StatusCode, Rejection> {
    let (_, target_id) = authenticate(&state, &headers, &query.key, cert.as_deref()).await?;
    let accepted = commands::report(&state.pool, target_id, id, &report).await?;
    if !accepted {
        return Err(Rejection::not_found("Command not found for this machine."));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------- UI side

pub fn ui_routes() -> Router<AppState> {
    Router::new()
        .route("/targets/{id}/agent", get(agent_host))
        .route("/targets/{id}/agent/rebind", post(allow_rebind))
        .route("/targets/{id}/agent/relay", put(set_relay))
        .route("/targets/{id}/agent/modules", get(list_modules))
        .route("/targets/{id}/agent/modules/{module}", put(set_module))
        .route("/targets/{id}/containers", get(list_containers))
        .route("/targets/{id}/containers/policy", put(set_policies))
        .route("/targets/{id}/containers/{name}/policy", put(set_policy))
        .route("/targets/{id}/containers/{name}/restart", post(restart))
        .route("/targets/{id}/containers/{name}/update", post(update))
        .route("/targets/{id}/commands", get(list_commands))
        .route("/targets/{id}/commands/{command_id}", delete(cancel_command))
}

/// La machine et son agent, tels que l'interface en a besoin pour savoir si
/// les actions ont une chance d'aboutir.
#[derive(Debug, Serialize)]
pub struct AgentHostView {
    pub hostname: String,
    pub os: String,
    pub os_version: Option<String>,
    pub arch: Option<String>,
    pub agent_version: String,
    /// Vrai seulement si l'agent a déclaré exécuter les commandes. Un agent
    /// antérieur au canal, ou configuré avec `commands: false`, donne faux : les
    /// boutons Restart/Update n'ont alors aucun sens.
    pub commands_supported: bool,
    /// Vrai si l'agent a déclaré relayer des sondes (`relay: true`).
    pub relay: bool,
    /// Site déclaré par l'agent.
    pub site: Option<String>,
    /// Mode relais demandé depuis l'interface (ou par le jeton d'enrôlement),
    /// transmis à l'agent dans chaque accusé de réception. Différent de
    /// `relay` le temps que l'agent l'adopte, ou pour toujours s'il est trop
    /// ancien pour le suivre (`relay_control` faux).
    pub relay_wanted: bool,
    pub relay_wanted_site: Option<String>,
    /// Vrai si l'agent applique ce réglage à chaud. Faux : binaire antérieur,
    /// qu'il faut mettre à jour ou régler par `relay: true` dans agent.yaml.
    pub relay_control: bool,
    /// Vrai si `relay: true` est posé dans la configuration de l'agent : le
    /// relais ne s'éteint pas depuis l'interface.
    pub relay_local: bool,
    /// Nombre d'équipements interrogés à travers cet agent (`via_agent`).
    pub relayed: usize,
    pub last_seen_at: Option<String>,
    /// Où en est la liaison de cette machine à son agent :
    ///
    /// - `bound` : l'agent détient un secret propre à cette machine, personne
    ///   d'autre ne peut pousser en son nom ni prendre ses commandes ;
    /// - `unbound` : machine enrôlée avant la liaison et jamais liée ; ses lots,
    ///   commandes et sondes relayées sont refusés jusqu'à son réenrôlement.
    pub binding: &'static str,
    /// Vrai pour l'état `bound`, pour que l'interface n'ait pas à comparer des
    /// chaînes pour allumer un voyant.
    pub bound: bool,
    pub bound_at: Option<String>,
    /// Faux pour un binaire antérieur à la liaison : il faudra le réinstaller.
    pub binding_supported: bool,
    /// Fin de la fenêtre de reliaison ouverte à la main, si elle court encore.
    pub rebind_until: Option<String>,
    /// Passerelle des agents du site : port demandé depuis l'interface
    /// (`None` : fermée), port sur lequel le relais écoute vraiment, adresse
    /// par laquelle le site le joint, et tunnels ouverts en ce moment.
    pub gateway_wanted: Option<u16>,
    pub gateway_port: Option<u16>,
    pub gateway_host: Option<String>,
    pub gateway_tunnels: usize,
}

impl AgentHostView {
    fn new(info: agent::HostInfo, relayed: usize, tunnels: usize) -> Self {
        Self {
            gateway_wanted: info.relay_gateway_wanted,
            gateway_port: info.relay_gateway_port,
            gateway_host: info.relay_gateway_host.clone(),
            gateway_tunnels: tunnels,
            commands_supported: info.commands_supported(),
            binding: info.binding_state(),
            hostname: info.hostname,
            os: info.os,
            os_version: info.os_version,
            arch: info.arch,
            agent_version: info.agent_version,
            relay: info.relay,
            site: info.site,
            relay_wanted: info.relay_wanted,
            relay_wanted_site: info.relay_wanted_site,
            relay_control: info.relay_control,
            relay_local: info.relay_local,
            relayed,
            bound: info.bound,
            bound_at: info.bound_at,
            binding_supported: info.binding_supported,
            rebind_until: info.rebind_until,
            last_seen_at: info.last_seen_at,
        }
    }
}

/// Une commande telle que l'interface l'affiche.
#[derive(Debug, Serialize)]
pub struct CommandView {
    pub id: i64,
    pub target_id: TargetId,
    pub kind: String,
    pub args: serde_json::Value,
    pub status: &'static str,
    pub requested_by: Option<String>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub result: Option<String>,
}

impl From<CommandRecord> for CommandView {
    fn from(record: CommandRecord) -> Self {
        Self {
            id: record.id,
            target_id: record.target_id,
            kind: record.kind,
            args: record.args,
            status: record.status.as_str(),
            requested_by: record.requested_by,
            created_at: record.created_at,
            started_at: record.started_at,
            finished_at: record.finished_at,
            result: record.result,
        }
    }
}

/// Un conteneur, tel que les dernières mesures le décrivent.
#[derive(Debug, Serialize)]
pub struct ContainerView {
    pub name: String,
    pub image: String,
    pub up: bool,
    pub health: &'static str,
    pub restart_count: u64,
    pub uptime_seconds: Option<u64>,
    pub image_age_seconds: Option<u64>,
    pub update_available: Option<bool>,
    /// CPU en pourcent d'un cœur (100 : un cœur plein). Absent : agent sans
    /// statistiques, conteneur arrêté, ou premier relevé.
    pub cpu_percent: Option<f64>,
    /// Mémoire utilisée, cache exclu.
    pub memory_bytes: Option<u64>,
    /// Limite mémoire du conteneur ; absente s'il n'en a pas.
    pub memory_limit_bytes: Option<u64>,
    /// Redémarrages sur les quinze dernières minutes : trois ou plus, c'est
    /// une boucle de redémarrage.
    pub restarts_15m: Option<u64>,
    pub policy: ContainerPolicy,
    pub last_command: Option<CommandView>,
}

/// Valeurs brutes d'un conteneur, avant mise en forme.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ContainerReading {
    pub image: String,
    pub up: Option<f64>,
    pub health: Option<f64>,
    pub restart_count: Option<f64>,
    pub started_seconds: Option<f64>,
    pub image_age_seconds: Option<f64>,
    pub update_available: Option<f64>,
    pub cpu_percent: Option<f64>,
    pub memory_bytes: Option<f64>,
    pub memory_limit_bytes: Option<f64>,
    pub restarts_15m: Option<f64>,
}

/// Regroupe les séries instantanées par nom de conteneur.
pub fn group_readings<'a>(
    series: impl Iterator<Item = (&'a BTreeMap<String, String>, f64)>,
) -> BTreeMap<String, ContainerReading> {
    let mut readings: BTreeMap<String, ContainerReading> = BTreeMap::new();
    for (labels, value) in series {
        let (Some(name), Some(container)) = (labels.get("__name__"), labels.get("container"))
        else {
            continue;
        };
        let reading = readings.entry(container.clone()).or_default();
        if let Some(image) = labels.get("image") {
            reading.image = image.clone();
        }
        match name.as_str() {
            "dumbmonit_container_up" => reading.up = Some(value),
            "dumbmonit_container_health" => reading.health = Some(value),
            "dumbmonit_container_restart_count" => reading.restart_count = Some(value),
            "dumbmonit_container_started_seconds" => reading.started_seconds = Some(value),
            "dumbmonit_container_image_age_seconds" => reading.image_age_seconds = Some(value),
            "dumbmonit_container_update_available" => reading.update_available = Some(value),
            "dumbmonit_container_cpu_percent" => reading.cpu_percent = Some(value),
            "dumbmonit_container_memory_bytes" => reading.memory_bytes = Some(value),
            "dumbmonit_container_memory_limit_bytes" => reading.memory_limit_bytes = Some(value),
            _ => {}
        }
    }
    // Sans témoin `up`, la série n'est qu'un reste : elle ne décrit rien.
    readings.retain(|_, reading| reading.up.is_some());
    readings
}

fn health_word(code: Option<f64>) -> &'static str {
    match code.map(|c| c as i64) {
        Some(1) => "healthy",
        Some(2) => "unhealthy",
        Some(3) => "starting",
        _ => "none",
    }
}

fn to_view(
    name: String,
    reading: ContainerReading,
    policy: ContainerPolicy,
    last_command: Option<CommandRecord>,
) -> ContainerView {
    let up = reading.up.is_some_and(|v| v != 0.0);
    let seconds = |v: Option<f64>| v.filter(|v| v.is_finite() && *v >= 0.0).map(|v| v as u64);
    ContainerView {
        name,
        image: reading.image,
        up,
        health: health_word(reading.health),
        restart_count: seconds(reading.restart_count).unwrap_or(0),
        uptime_seconds: if up { seconds(reading.started_seconds) } else { None },
        image_age_seconds: seconds(reading.image_age_seconds),
        update_available: reading.update_available.and_then(|v| match v as i64 {
            1 if v == 1.0 => Some(true),
            0 if v == 0.0 => Some(false),
            // -1 : vérification impossible (image locale, dépôt privé…).
            _ => None,
        }),
        cpu_percent: reading.cpu_percent.filter(|v| v.is_finite() && *v >= 0.0),
        memory_bytes: seconds(reading.memory_bytes),
        memory_limit_bytes: seconds(reading.memory_limit_bytes).filter(|limit| *limit > 0),
        restarts_15m: seconds(reading.restarts_15m),
        policy,
        last_command: last_command.map(CommandView::from),
    }
}

/// Charge une cible de type `agent`, ou 404.
async fn agent_target(state: &AppState, id: TargetId) -> ApiResult<dumbmonit_proto::Target> {
    let target = db::targets::get(&state.pool, &state.cipher, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Monitor {id} not found.")))?;
    if target.kind != "agent" {
        return Err(ApiError::NotFound(format!("Monitor {id} has no agent.")));
    }
    Ok(target)
}

/// Les deux cibles d'une demande sur les conteneurs, à partir de la machine ou
/// de son appareil Docker : `host` reçoit les commandes et porte les
/// politiques (c'est lui que l'agent interroge), `metrics` porte les séries des
/// conteneurs. Avant la séparation des modules, les deux sont la machine.
struct DockerTargets {
    host: TargetId,
    metrics: TargetId,
}

async fn docker_targets(state: &AppState, id: TargetId) -> ApiResult<DockerTargets> {
    let target = db::targets::get(&state.pool, &state.cipher, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Monitor {id} not found.")))?;
    match target.kind.as_str() {
        "agent" => {
            let child = modules::states(&state.pool, id)
                .await?
                .remove("docker")
                .and_then(|docker| docker.target_id);
            Ok(DockerTargets { host: id, metrics: child.unwrap_or(id) })
        }
        modules::SERVICE_KIND => match modules::owner(&state.pool, id).await? {
            Some((host, module)) if module == "docker" => Ok(DockerTargets { host, metrics: id }),
            _ => Err(ApiError::NotFound(format!("Monitor {id} is not a Docker service."))),
        },
        _ => Err(ApiError::NotFound(format!("Monitor {id} has no agent."))),
    }
}

/// Un nom de conteneur tel que Docker les accepte, hors ceux du moniteur.
pub fn validate_name(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    let valid_first = chars.next().is_some_and(|c| c.is_ascii_alphanumeric());
    let valid_rest = chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'));
    if name.is_empty() || name.len() > 128 || !valid_first || !valid_rest {
        return Err("Invalid container name.".to_string());
    }
    let lower = name.to_ascii_lowercase();
    if RESERVED_PREFIXES.iter().any(|prefix| lower.starts_with(prefix)) {
        return Err("The monitor never acts on its own containers.".to_string());
    }
    Ok(())
}

fn checked_name(name: &str) -> ApiResult<String> {
    validate_name(name).map_err(ApiError::BadRequest)?;
    Ok(name.to_string())
}

pub(crate) async fn list_containers(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<Vec<ContainerView>>> {
    let DockerTargets { host, metrics } = docker_targets(&state, id).await?;
    let query = format!(
        r#"{{__name__=~"dumbmonit_container_(up|health|restart_count|started_seconds|image_age_seconds|update_available|cpu_percent|memory_bytes|memory_limit_bytes)", target="{metrics}"}}"#
    );
    let series = state.victoria.query(&query).await?;
    let mut readings = group_readings(
        series.iter().map(|s| (&s.metric, s.value.1.parse::<f64>().unwrap_or(f64::NAN))),
    );
    // Les redémarrages récents sont une requête à part : une fonction de
    // fenêtre ne se mélange pas au sélecteur d'instantanés ci-dessus, et même
    // formule que la règle « Container restarting ». Son échec ne casse pas la
    // liste, il retire seulement le badge.
    let recent = state
        .victoria
        .query(&format!(
            r#"increase_prometheus(dumbmonit_container_restart_count{{target="{metrics}"}}[15m])"#
        ))
        .await
        .unwrap_or_default();
    for s in &recent {
        if let (Some(name), Ok(value)) = (s.metric.get("container"), s.value.1.parse::<f64>())
            && let Some(reading) = readings.get_mut(name)
        {
            reading.restarts_15m = Some(value.round());
        }
    }
    let mut policies = commands::list_policies(&state.pool, host).await?;
    let mut latest = commands::last_command_per_container(&state.pool, host).await?;

    Ok(Json(
        readings
            .into_iter()
            .map(|(name, reading)| {
                let policy = policies.remove(&name).unwrap_or_default();
                let last = latest.remove(&name);
                to_view(name, reading, policy, last)
            })
            .collect(),
    ))
}

/// Corps de `PUT …/policy` : chaque champ omis garde sa valeur enregistrée,
/// comme pour les autres `PUT` de l'API. Un interrupteur de l'interface
/// n'envoie ainsi que ce qu'il change.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct PolicyPatch {
    pub auto_restart: Option<bool>,
    pub auto_update: Option<bool>,
    pub prune_old_image: Option<bool>,
    pub only_in_maintenance: Option<bool>,
}

impl PolicyPatch {
    pub fn apply(&self, current: ContainerPolicy) -> ContainerPolicy {
        ContainerPolicy {
            auto_restart: self.auto_restart.unwrap_or(current.auto_restart),
            auto_update: self.auto_update.unwrap_or(current.auto_update),
            prune_old_image: self.prune_old_image.unwrap_or(current.prune_old_image),
            only_in_maintenance: self.only_in_maintenance.unwrap_or(current.only_in_maintenance),
        }
    }
}

async fn set_policy(
    State(state): State<AppState>,
    Path((id, name)): Path<(TargetId, String)>,
    Json(patch): Json<PolicyPatch>,
) -> ApiResult<Json<ContainerPolicy>> {
    let host = docker_targets(&state, id).await?.host;
    let name = checked_name(&name)?;
    let current = commands::get_policy(&state.pool, host, &name).await?;
    let policy = patch.apply(current);
    commands::set_policy(&state.pool, host, &name, policy).await?;
    Ok(Json(policy))
}

/// Plus de conteneurs que cela d'un coup : borne le corps, pas l'usage réel.
const MAX_BULK_CONTAINERS: usize = 500;

/// Corps de `PUT /api/targets/{id}/containers/policy` : les conteneurs visés et
/// les champs à changer (mêmes règles que [`PolicyPatch`] : omis, inchangé).
#[derive(Debug, Default, Deserialize)]
pub struct BulkPolicyBody {
    #[serde(default)]
    pub containers: Vec<String>,
    #[serde(flatten)]
    pub patch: PolicyPatch,
}

/// Résultat pour un conteneur d'une modification groupée.
#[derive(Debug, Serialize)]
pub struct BulkPolicyResult {
    pub name: String,
    pub ok: bool,
    /// Politique complète après changement, quand `ok`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<ContainerPolicy>,
    /// Pourquoi le nom a été refusé, quand `!ok`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Réponse : un résultat par nom demandé, dans l'ordre de la demande.
#[derive(Debug, Serialize)]
pub struct BulkPolicyReply {
    pub results: Vec<BulkPolicyResult>,
}

/// Même droits que la route unitaire. Les noms valides sont écrits en une
/// transaction ; un nom invalide est rapporté sans empêcher les autres.
async fn set_policies(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
    Json(body): Json<BulkPolicyBody>,
) -> ApiResult<Json<BulkPolicyReply>> {
    let host = docker_targets(&state, id).await?.host;
    if body.containers.is_empty() {
        return Err(ApiError::BadRequest("Select at least one container.".into()));
    }
    if body.containers.len() > MAX_BULK_CONTAINERS {
        return Err(ApiError::BadRequest(format!(
            "At most {MAX_BULK_CONTAINERS} containers per request."
        )));
    }
    let mut valid: Vec<String> = Vec::new();
    for name in &body.containers {
        if validate_name(name).is_ok() && !valid.contains(name) {
            valid.push(name.clone());
        }
    }
    let applied: BTreeMap<String, ContainerPolicy> =
        commands::update_policies(&state.pool, host, &valid, |current| body.patch.apply(current))
            .await?
            .into_iter()
            .collect();
    let results = body
        .containers
        .into_iter()
        .map(|name| match validate_name(&name) {
            Err(error) => BulkPolicyResult { name, ok: false, policy: None, error: Some(error) },
            // Un doublon dans la demande répond comme son premier exemplaire.
            Ok(()) => {
                let policy = applied.get(&name).copied();
                BulkPolicyResult { name, ok: true, policy, error: None }
            }
        })
        .collect();
    Ok(Json(BulkPolicyReply { results }))
}

/// Conteneurs connus d'une cible, d'après les dernières mesures de son agent.
async fn known_containers(state: &AppState, id: TargetId) -> ApiResult<BTreeSet<String>> {
    let query = format!(r#"{{__name__="dumbmonit_container_up", target="{id}"}}"#);
    let series = state.victoria.query(&query).await?;
    Ok(series.into_iter().filter_map(|s| s.metric.get("container").cloned()).collect())
}

/// Un nom valide, et présent dans l'inventaire : une commande pour un
/// conteneur que l'agent ne voit pas échouerait de toute façon.
async fn checked_known_name(state: &AppState, id: TargetId, name: &str) -> ApiResult<String> {
    let name = checked_name(name)?;
    if !known_containers(state, id).await?.contains(&name) {
        return Err(ApiError::NotFound(format!(
            "No container named '{name}' on device {id}: it is not in the agent's inventory."
        )));
    }
    Ok(name)
}

/// Refuse une commande que l'agent ne viendra jamais chercher : elle
/// n'expirerait qu'au bout de dix minutes, sans que rien ne se passe.
async fn ensure_commands_supported(state: &AppState, id: TargetId) -> ApiResult<()> {
    let supported = agent::host(&state.pool, id).await?.is_some_and(|h| h.commands_supported());
    if !supported {
        return Err(ApiError::Conflict(
            "This agent cannot run commands: it is too old or has actions disabled. \
             Reinstall it with the current installer."
                .to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdatePayload {
    #[serde(default)]
    prune: Option<bool>,
}

/// Qui demande : le compte de la session ou le jeton d'API (`token:nom`), ou
/// « ui » si le garde ne l'a pas posé.
fn requester(who: Option<&CurrentPrincipal>) -> String {
    who.map(|current| current.0.label()).unwrap_or_else(|| "ui".to_string())
}

async fn enqueue(
    state: &AppState,
    id: TargetId,
    kind: &str,
    args: serde_json::Value,
    requested_by: &str,
) -> ApiResult<(StatusCode, Json<CommandView>)> {
    match commands::enqueue(&state.pool, id, kind, &args, requested_by).await {
        Ok(record) => Ok((StatusCode::CREATED, Json(record.into()))),
        Err(CommandError::Conflict(why)) => Err(ApiError::Conflict(why)),
        Err(CommandError::NotFound) => Err(ApiError::NotFound("Command not found.".into())),
        Err(CommandError::Internal(error)) => Err(ApiError::Internal(error)),
    }
}

pub(crate) async fn restart(
    State(state): State<AppState>,
    Path((id, name)): Path<(TargetId, String)>,
    who: Option<Extension<CurrentPrincipal>>,
) -> ApiResult<(StatusCode, Json<CommandView>)> {
    let DockerTargets { host, metrics } = docker_targets(&state, id).await?;
    ensure_commands_supported(&state, host).await?;
    let name = checked_known_name(&state, metrics, &name).await?;
    let by = requester(who.as_deref());
    enqueue(&state, host, CMD_CONTAINER_RESTART, serde_json::json!({ "name": name }), &by).await
}

async fn update(
    State(state): State<AppState>,
    Path((id, name)): Path<(TargetId, String)>,
    who: Option<Extension<CurrentPrincipal>>,
    payload: Option<Json<UpdatePayload>>,
) -> ApiResult<(StatusCode, Json<CommandView>)> {
    let DockerTargets { host, metrics } = docker_targets(&state, id).await?;
    ensure_commands_supported(&state, host).await?;
    let name = checked_known_name(&state, metrics, &name).await?;
    let policy = commands::get_policy(&state.pool, host, &name).await?;
    let prune = payload.and_then(|Json(p)| p.prune).unwrap_or(policy.prune_old_image);
    let by = requester(who.as_deref());
    let args = serde_json::json!({ "name": name, "prune": prune });
    enqueue(&state, host, CMD_CONTAINER_UPDATE, args, &by).await
}

async fn list_commands(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<Vec<CommandView>>> {
    let host = docker_targets(&state, id).await?.host;
    let records = commands::list_for_target(&state.pool, host, RECENT_COMMANDS).await?;
    Ok(Json(records.into_iter().map(CommandView::from).collect()))
}

/// `DELETE /api/targets/{id}/commands/{command_id}` : retire une commande
/// encore en attente. `409` si l'agent l'a déjà prise ou si elle est close.
async fn cancel_command(
    State(state): State<AppState>,
    Path((id, command_id)): Path<(TargetId, i64)>,
    who: Option<Extension<CurrentPrincipal>>,
) -> ApiResult<StatusCode> {
    let host = docker_targets(&state, id).await?.host;
    let by = requester(who.as_deref());
    match commands::cancel(&state.pool, host, command_id, &by).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(CommandError::Conflict(why)) => Err(ApiError::Conflict(why)),
        Err(CommandError::NotFound) => {
            Err(ApiError::NotFound(format!("Command {command_id} not found on device {id}.")))
        }
        Err(CommandError::Internal(error)) => Err(ApiError::Internal(error)),
    }
}

/// `GET /api/targets/{id}/agent` : la machine telle que son agent la décrit.
/// `404` tant qu'aucun agent ne s'est présenté pour cette cible.
async fn agent_host(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<AgentHostView>> {
    agent_target(&state, id).await?;
    let info = agent::host(&state.pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("No agent has reported for device {id} yet.")))?;
    let relayed = db::targets::relayed_counts(&state.pool).await?.get(&id).copied().unwrap_or(0);
    let tunnels = state.agent_tls().tunnels_of(id);
    Ok(Json(AgentHostView::new(info, relayed, tunnels)))
}

/// Réponse à l'ouverture d'une fenêtre de reliaison.
#[derive(Debug, Serialize)]
pub struct RebindWindow {
    /// Fin de la fenêtre, en UTC sans suffixe comme partout ailleurs.
    pub rebind_until: String,
    /// Durée de la fenêtre, pour que l'interface n'ait pas à la recalculer.
    pub minutes: i64,
}

/// `POST /api/targets/{id}/agent/rebind` : autorise cette machine à se relier.
///
/// La seule issue prévue quand le secret d'une machine est perdu — disque
/// refait, conteneur d'agent recréé sans son volume. Volontairement manuelle et
/// datée : une machine liée doit le rester tant que personne n'a décidé
/// autrement, sans quoi la liaison ne protégerait de rien.
async fn allow_rebind(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
    who: Option<Extension<CurrentPrincipal>>,
) -> ApiResult<Json<RebindWindow>> {
    agent_target(&state, id).await?;
    let until = agent::allow_rebind(&state.pool, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("No agent has reported for device {id} yet.")))?;
    tracing::info!(cible = id, par = requester(who.as_deref()), "fenêtre de reliaison ouverte");
    Ok(Json(RebindWindow { rebind_until: until, minutes: agent::REBIND_WINDOW_MINUTES }))
}

/// Corps de `PUT /api/targets/{id}/agent/relay`.
#[derive(Debug, Deserialize)]
pub struct RelayChoice {
    pub enabled: bool,
    /// Site de ce relais ; vide ou absent : aucun.
    #[serde(default)]
    pub site: Option<String>,
    /// Passerelle des agents du site. Absente : inchangée.
    #[serde(default)]
    pub gateway: Option<GatewayChoice>,
}

/// Passerelle d'un relais, telle que l'interface la règle.
#[derive(Debug, Deserialize)]
pub struct GatewayChoice {
    pub enabled: bool,
    /// Port d'écoute sur le relais. Absent : 8443.
    #[serde(default)]
    pub port: Option<u16>,
    /// Adresse par laquelle les machines du site joignent le relais.
    #[serde(default)]
    pub host: Option<String>,
}

/// `PUT /api/targets/{id}/agent/relay` : allume ou éteint le mode relais de la
/// machine depuis l'interface. L'agent l'applique à chaud à son prochain lot.
async fn set_relay(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
    who: Option<Extension<CurrentPrincipal>>,
    Json(choice): Json<RelayChoice>,
) -> ApiResult<Json<AgentHostView>> {
    agent_target(&state, id).await?;
    let site = agent::clean_site(choice.site.as_deref());
    let max = agent::MAX_SITE_LEN;
    if site.as_ref().is_some_and(|s| s.chars().count() > max) {
        return Err(ApiError::BadRequest(format!("Site name is limited to {max} characters.")));
    }
    let gateway = match &choice.gateway {
        Some(gateway) => {
            let host = agent::clean_gateway_host(gateway.host.as_deref())
                .map_err(|e| ApiError::BadRequest(e.into()))?;
            let port = gateway.port.unwrap_or(dumbmonit_proto::DEFAULT_RELAY_GATEWAY_PORT);
            if port == 0 {
                return Err(ApiError::BadRequest("The gateway port must be 1 to 65535.".into()));
            }
            Some((gateway.enabled.then_some(port), host))
        }
        None => None,
    };
    if !agent::set_relay_wanted(&state.pool, id, choice.enabled, site.as_deref()).await? {
        return Err(ApiError::NotFound(format!("No agent has reported for device {id} yet.")));
    }
    if let Some((port, host)) = gateway {
        agent::set_relay_gateway(&state.pool, id, port, host.as_deref()).await?;
    }
    tracing::info!(
        cible = id,
        relais = choice.enabled,
        site = site.as_deref().unwrap_or(""),
        par = requester(who.as_deref()),
        "mode relais réglé depuis l'interface"
    );
    agent_host(State(state), Path(id)).await
}

/// Un module d'agent tel que le panneau de la machine le liste.
#[derive(Debug, Serialize)]
pub struct AgentModuleView {
    /// Identifiant stable (`docker`, `zfs`…), clé des textes de l'interface.
    pub id: &'static str,
    /// Nom affiché quand l'interface ne connaît pas ce module.
    pub label: &'static str,
    /// Vrai si l'agent en a déjà rapporté des mesures sur cette machine.
    pub detected: bool,
    /// Case cochée : un appareil existe (ou existera) pour ce module.
    pub enabled: bool,
    /// Vrai tant que personne n'a coché ni décoché la case : l'appareil suit
    /// la détection.
    pub automatic: bool,
    /// L'appareil du module, s'il existe.
    pub target_id: Option<TargetId>,
    /// Dernier lot qui en contenait des mesures, UTC sans suffixe.
    pub last_seen_at: Option<String>,
}

impl AgentModuleView {
    fn new(module: &'static modules::Module, state: Option<&ModuleState>) -> Self {
        Self {
            id: module.id,
            label: module.label,
            detected: state.is_some_and(|s| s.last_seen_ms.is_some()),
            enabled: state.is_some_and(ModuleState::wanted),
            automatic: state.is_none_or(|s| s.enabled.is_none()),
            target_id: state.and_then(|s| s.target_id),
            last_seen_at: state.and_then(|s| s.last_seen_ms).and_then(|ms| {
                chrono::DateTime::from_timestamp_millis(ms)
                    .map(|at| at.format("%Y-%m-%dT%H:%M:%S").to_string())
            }),
        }
    }
}

/// `GET /api/targets/{id}/agent/modules` : les services que l'agent sait
/// détecter, trouvés ou non sur cette machine, dans un ordre fixe.
async fn list_modules(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
) -> ApiResult<Json<Vec<AgentModuleView>>> {
    agent_target(&state, id).await?;
    let states = modules::states(&state.pool, id).await?;
    Ok(Json(
        modules::MODULES
            .iter()
            .map(|module| AgentModuleView::new(module, states.get(module.id)))
            .collect(),
    ))
}

#[derive(Debug, Deserialize)]
pub struct ModuleChoice {
    pub enabled: bool,
}

/// `PUT /api/targets/{id}/agent/modules/{module}` : coche ou décoche un module.
///
/// Décocher supprime l'appareil du service et fait écarter ses mesures (l'agent
/// en est prévenu dans l'accusé de son prochain lot) ; cocher crée l'appareil,
/// même avant que l'agent ait trouvé le service.
async fn set_module(
    State(state): State<AppState>,
    Path((id, module)): Path<(TargetId, String)>,
    Json(choice): Json<ModuleChoice>,
) -> ApiResult<Json<AgentModuleView>> {
    let host = agent_target(&state, id).await?;
    let module = modules::find(&module)
        .ok_or_else(|| ApiError::NotFound(format!("Unknown agent module '{module}'.")))?;
    let (saved, removed) =
        modules::set_enabled(&state.pool, &state.cipher, &host, module, choice.enabled).await?;
    if let Some(removed) = removed {
        crate::api::targets::forget(&state, removed).await;
    }
    Ok(Json(AgentModuleView::new(module, Some(&saved))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_machine_key_comes_from_its_header_then_from_the_legacy_query() {
        let mut headers = HeaderMap::new();
        assert_eq!(machine_key(&headers, ""), None);
        assert_eq!(machine_key(&headers, "  "), None);
        // Agent antérieur : la clé n'existe que dans l'URL.
        assert_eq!(machine_key(&headers, " ancienne "), Some("ancienne"));
        headers.insert(AGENT_KEY_HEADER, "9f4c".parse().unwrap());
        assert_eq!(machine_key(&headers, ""), Some("9f4c"));
        assert_eq!(machine_key(&headers, "autre"), Some("9f4c"), "l'en-tête prime");
    }

    fn labels(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn readings_are_joined_by_container_name() {
        let web_up = labels(&[
            ("__name__", "dumbmonit_container_up"),
            ("container", "web"),
            ("image", "nginx:1.25"),
        ]);
        let web_health =
            labels(&[("__name__", "dumbmonit_container_health"), ("container", "web")]);
        let web_update =
            labels(&[("__name__", "dumbmonit_container_update_available"), ("container", "web")]);
        let ghost = labels(&[("__name__", "dumbmonit_container_health"), ("container", "ghost")]);
        let series = [(&web_up, 1.0), (&web_health, 2.0), (&web_update, -1.0), (&ghost, 1.0)];

        let readings = group_readings(series.into_iter());
        assert_eq!(readings.len(), 1, "a container without `up` is a leftover, not a reading");
        let web = &readings["web"];
        assert_eq!(web.image, "nginx:1.25");

        let view = to_view("web".into(), web.clone(), ContainerPolicy::default(), None);
        assert!(view.up);
        assert_eq!(view.health, "unhealthy");
        assert_eq!(view.update_available, None, "-1 means unknown");
        assert_eq!(view.uptime_seconds, None, "no started_seconds series");
    }

    #[test]
    fn usage_readings_reach_the_view() {
        let name = |metric: &str| labels(&[("__name__", metric), ("container", "web")]);
        let (up, cpu, mem, limit) = (
            name("dumbmonit_container_up"),
            name("dumbmonit_container_cpu_percent"),
            name("dumbmonit_container_memory_bytes"),
            name("dumbmonit_container_memory_limit_bytes"),
        );
        let readings =
            group_readings([(&up, 1.0), (&cpu, 42.5), (&mem, 1_000.0), (&limit, 0.0)].into_iter());
        let mut web = readings["web"].clone();
        web.restarts_15m = Some(3.0);
        let view = to_view("web".into(), web, ContainerPolicy::default(), None);
        assert_eq!(view.cpu_percent, Some(42.5));
        assert_eq!(view.memory_bytes, Some(1_000));
        assert_eq!(view.memory_limit_bytes, None, "a zero limit means unlimited");
        assert_eq!(view.restarts_15m, Some(3));

        let bare =
            to_view("db".into(), ContainerReading::default(), ContainerPolicy::default(), None);
        assert_eq!((bare.cpu_percent, bare.memory_bytes, bare.restarts_15m), (None, None, None));
    }

    #[test]
    fn a_stopped_container_has_no_uptime() {
        let reading = ContainerReading {
            image: "x".into(),
            up: Some(0.0),
            started_seconds: Some(42.0),
            update_available: Some(1.0),
            restart_count: Some(3.0),
            ..ContainerReading::default()
        };
        let view = to_view("db".into(), reading, ContainerPolicy::default(), None);
        assert!(!view.up);
        assert_eq!(view.uptime_seconds, None);
        assert_eq!(view.update_available, Some(true));
        assert_eq!(view.restart_count, 3);
        assert_eq!(view.health, "none");
    }

    #[test]
    fn a_partial_policy_keeps_the_omitted_fields() {
        let current = ContainerPolicy {
            auto_restart: true,
            auto_update: false,
            prune_old_image: false,
            only_in_maintenance: true,
        };
        let patch: PolicyPatch = serde_json::from_str(r#"{"auto_update": true}"#).unwrap();
        let next = patch.apply(current);
        assert_eq!(next, ContainerPolicy { auto_update: true, ..current });
        // Un corps vide ne change rien ; un corps complet remplace tout.
        assert_eq!(PolicyPatch::default().apply(current), current);
        let full: PolicyPatch = serde_json::from_str(
            r#"{"auto_restart": false, "auto_update": true, "prune_old_image": true, "only_in_maintenance": false}"#,
        )
        .unwrap();
        assert_eq!(
            full.apply(current),
            ContainerPolicy {
                auto_restart: false,
                auto_update: true,
                prune_old_image: true,
                only_in_maintenance: false
            }
        );
    }

    #[test]
    fn container_names_are_checked_before_reaching_the_agent() {
        assert!(validate_name("nginx-victim").is_ok());
        assert!(validate_name("web_1.0").is_ok());
        assert!(validate_name("").is_err());
        assert!(validate_name("-web").is_err());
        assert!(validate_name("web;rm -rf").is_err());
        assert!(validate_name("../web").is_err());
        assert!(validate_name(&"a".repeat(129)).is_err());
        // Le moniteur ne se touche pas lui-même, sous aucun de ses deux noms.
        assert!(validate_name("dumbmonit-dumbmonit-1").is_err());
        assert!(validate_name("DumbMonit").is_err());
        assert!(validate_name("ezymonit-ezymonit-1").is_err(), "le nom d'avant le renommage");
        assert!(validate_name("EzyMonit-agent").is_err());
        // Le contrôle porte sur un nom : un identifiant de conteneur y échappe
        // par construction, et c'est l'agent qui le rattrape après inspection.
        assert!(validate_name("9f4c1b2e3d5a").is_ok());
    }
}
