//! Procédures (runbooks) et remédiation : actions, politiques, journal.
//!
//! Droits :
//! - lecture : tout compte connecté (le journal ne contient aucun secret) ;
//! - exécution manuelle : opérateur ou administrateur, depuis une alerte en
//!   cours (un opérateur ne choisit pas la machine : c'est celle de l'alerte,
//!   ou celle que l'administrateur a fixée sur l'action) ;
//! - configuration (procédures, actions, politiques) : administrateur ;
//! - armement d'une politique : administrateur, en personne (session, jamais un
//!   jeton d'API), en deux étapes.
//!
//! Les jetons d'API n'ont pas accès à `/remediation` (`TOKEN_DENIED`) : agir sur
//! une machine reste un geste humain, fait dans l'interface.

use std::collections::BTreeMap;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiResult};
use crate::auth::audit;
use crate::auth::client_ip::ClientIp;
use crate::auth::middleware::{AdminIdentity, AdminUser, Identity, OperatorIdentity, Principal};
use crate::db;
use crate::remediation::exec::{self, ExecError, Trigger};
use crate::remediation::runbooks::{self, MatchContext, Runbook, RunbookDraft};
use crate::remediation::store::{
    self, ActionDraft, ActionRecord, Confirmation, PolicyDraft, PolicyRecord, RunFilter, RunRecord,
};
use crate::remediation::{self, ActionSpec, KindInfo};
use crate::state::AppState;

/// Longueur maximale du nom d'une action.
const MAX_ACTION_NAME: usize = 100;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/runbooks", get(list_runbooks).post(create_runbook))
        .route("/runbooks/match", get(match_runbooks))
        .route("/runbooks/{id}", get(get_runbook).put(update_runbook).delete(delete_runbook))
        .route("/remediation/kinds", get(kinds))
        .route("/remediation/actions", get(list_actions).post(create_action))
        .route("/remediation/actions/{id}", put(update_action).delete(delete_action))
        .route("/remediation/actions/{id}/run", post(run_action))
        .route("/remediation/policies", get(list_policies).post(create_policy))
        .route("/remediation/policies/{id}", put(update_policy).delete(delete_policy))
        .route("/remediation/policies/{id}/arm", post(arm_policy))
        .route("/remediation/policies/{id}/confirm", post(confirm_policy))
        .route("/remediation/policies/{id}/disable", post(disable_policy))
        .route("/remediation/runs", get(list_runs))
        .route("/remediation/alert/{fingerprint}", get(alert_context))
        .route("/remediation/targets/{id}", get(target_context))
}

// ---------------------------------------------------------------- runbooks

async fn list_runbooks(
    State(state): State<AppState>,
    Identity(_): Identity,
) -> ApiResult<Json<Vec<Runbook>>> {
    Ok(Json(runbooks::list(&state.pool).await?))
}

async fn get_runbook(
    State(state): State<AppState>,
    Identity(_): Identity,
    Path(id): Path<i64>,
) -> ApiResult<Json<Runbook>> {
    runbooks::get(&state.pool, id)
        .await?
        .map(Json)
        .ok_or_else(|| ApiError::NotFound(format!("Runbook {id} not found.")))
}

#[derive(Debug, Deserialize)]
struct MatchQuery {
    rule_uid: Option<String>,
    target_id: Option<i64>,
}

async fn match_runbooks(
    State(state): State<AppState>,
    Identity(_): Identity,
    Query(query): Query<MatchQuery>,
) -> ApiResult<Json<Vec<Runbook>>> {
    let mut context = MatchContext::for_target(&state.pool, query.target_id).await?;
    context.rule_uid = query.rule_uid.filter(|uid| !uid.is_empty());
    Ok(Json(runbooks::matching(&state.pool, &context).await?))
}

async fn create_runbook(
    State(state): State<AppState>,
    AdminIdentity(principal): AdminIdentity,
    ClientIp(ip): ClientIp,
    Json(draft): Json<RunbookDraft>,
) -> ApiResult<(StatusCode, Json<Runbook>)> {
    let draft = draft.validate().map_err(ApiError::BadRequest)?;
    let actor = principal.label();
    let id = runbooks::create(&state.pool, &draft, &actor).await?;
    audit::record(&state.pool, Some(&actor), "runbook.created", Some(&draft.title), ip).await;
    let runbook = runbooks::get(&state.pool, id).await?.ok_or_else(|| not_found("Runbook", id))?;
    Ok((StatusCode::CREATED, Json(runbook)))
}

async fn update_runbook(
    State(state): State<AppState>,
    AdminIdentity(principal): AdminIdentity,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
    Json(draft): Json<RunbookDraft>,
) -> ApiResult<Json<Runbook>> {
    let draft = draft.validate().map_err(ApiError::BadRequest)?;
    let actor = principal.label();
    if !runbooks::update(&state.pool, id, &draft, &actor).await? {
        return Err(not_found("Runbook", id));
    }
    audit::record(&state.pool, Some(&actor), "runbook.updated", Some(&draft.title), ip).await;
    let runbook = runbooks::get(&state.pool, id).await?.ok_or_else(|| not_found("Runbook", id))?;
    Ok(Json(runbook))
}

async fn delete_runbook(
    State(state): State<AppState>,
    AdminIdentity(principal): AdminIdentity,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if !runbooks::delete(&state.pool, id).await? {
        return Err(not_found("Runbook", id));
    }
    let subject = format!("runbook {id}");
    audit::record(&state.pool, Some(&principal.label()), "runbook.deleted", Some(&subject), ip)
        .await;
    Ok(StatusCode::NO_CONTENT)
}

fn not_found(what: &str, id: i64) -> ApiError {
    ApiError::NotFound(format!("{what} {id} not found."))
}

// ---------------------------------------------------------------- actions

async fn kinds(Identity(_): Identity) -> Json<Vec<KindInfo>> {
    Json(remediation::catalog())
}

async fn list_actions(
    State(state): State<AppState>,
    Identity(principal): Identity,
) -> ApiResult<Json<Vec<ActionRecord>>> {
    let actions = store::list_actions(&state.pool).await?;
    Ok(Json(shown_to(&principal, actions)))
}

/// Seul un administrateur voit les paramètres d'une action (l'URL complète
/// d'un webhook, par exemple) : les autres n'en ont besoin que du nom.
fn shown_to(principal: &Principal, actions: Vec<ActionRecord>) -> Vec<ActionRecord> {
    if principal.is_admin() {
        return actions;
    }
    actions
        .into_iter()
        .map(|action| ActionRecord { params: serde_json::json!({}), ..action })
        .collect()
}

/// Corps de `POST`/`PUT` d'une action. `secrets` omis : inchangés ; `{}` :
/// effacés.
#[derive(Debug, Deserialize)]
struct ActionPayload {
    name: String,
    kind: String,
    #[serde(default)]
    params: serde_json::Value,
    #[serde(default)]
    target_id: Option<i64>,
    #[serde(default)]
    secrets: Option<BTreeMap<String, String>>,
}

async fn action_draft(state: &AppState, payload: ActionPayload) -> ApiResult<ActionDraft> {
    let name = payload.name.trim().to_string();
    if name.is_empty()
        || name.chars().count() > MAX_ACTION_NAME
        || name.chars().any(char::is_control)
    {
        return Err(ApiError::BadRequest(format!(
            "The name is required ({MAX_ACTION_NAME} characters max)."
        )));
    }
    let spec = ActionSpec::parse(&payload.kind, &payload.params).map_err(ApiError::BadRequest)?;
    if let Some(secrets) = &payload.secrets {
        remediation::validate_secrets(spec.kind(), secrets).map_err(ApiError::BadRequest)?;
    }
    if let Some(id) = payload.target_id {
        ensure_target(state, id).await?;
    }
    Ok(ActionDraft {
        name,
        kind: spec.kind().to_string(),
        params: spec.to_params(),
        target_id: payload.target_id,
        secrets: payload.secrets,
    })
}

async fn ensure_target(state: &AppState, id: i64) -> ApiResult<()> {
    let exists: Option<i64> = sqlx::query_scalar("SELECT id FROM targets WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .map_err(anyhow::Error::from)?;
    exists.map(|_| ()).ok_or_else(|| ApiError::BadRequest(format!("Monitor {id} not found.")))
}

fn unique_violation(error: &anyhow::Error) -> bool {
    format!("{error:#}").contains("UNIQUE")
}

async fn create_action(
    State(state): State<AppState>,
    AdminIdentity(principal): AdminIdentity,
    ClientIp(ip): ClientIp,
    Json(payload): Json<ActionPayload>,
) -> ApiResult<(StatusCode, Json<ActionRecord>)> {
    let draft = action_draft(&state, payload).await?;
    let id = match store::create_action(&state.pool, &state.cipher, &draft).await {
        Ok(id) => id,
        Err(error) if unique_violation(&error) => {
            return Err(ApiError::Conflict(format!("An action named '{}' exists.", draft.name)));
        }
        Err(error) => return Err(error.into()),
    };
    audit::record(&state.pool, Some(&principal.label()), "action.created", Some(&draft.name), ip)
        .await;
    let action =
        store::get_action(&state.pool, id).await?.ok_or_else(|| not_found("Action", id))?;
    Ok((StatusCode::CREATED, Json(action)))
}

async fn update_action(
    State(state): State<AppState>,
    AdminIdentity(principal): AdminIdentity,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
    Json(payload): Json<ActionPayload>,
) -> ApiResult<Json<ActionRecord>> {
    let draft = action_draft(&state, payload).await?;
    let current =
        store::get_action(&state.pool, id).await?.ok_or_else(|| not_found("Action", id))?;
    // Un secret ne survit pas à un changement de type : il ne voudrait plus
    // rien dire, et ne doit pas partir vers une autre destination.
    let draft = if current.kind != draft.kind && draft.secrets.is_none() {
        ActionDraft { secrets: Some(BTreeMap::new()), ..draft }
    } else {
        draft
    };
    match store::update_action(&state.pool, &state.cipher, id, &draft).await {
        Ok(true) => {}
        Ok(false) => return Err(not_found("Action", id)),
        Err(error) if unique_violation(&error) => {
            return Err(ApiError::Conflict(format!("An action named '{}' exists.", draft.name)));
        }
        Err(error) => return Err(error.into()),
    }
    audit::record(&state.pool, Some(&principal.label()), "action.updated", Some(&draft.name), ip)
        .await;
    let action =
        store::get_action(&state.pool, id).await?.ok_or_else(|| not_found("Action", id))?;
    Ok(Json(action))
}

async fn delete_action(
    State(state): State<AppState>,
    AdminIdentity(principal): AdminIdentity,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    let action =
        store::get_action(&state.pool, id).await?.ok_or_else(|| not_found("Action", id))?;
    store::delete_action(&state.pool, id).await?;
    audit::record(&state.pool, Some(&principal.label()), "action.deleted", Some(&action.name), ip)
        .await;
    Ok(StatusCode::NO_CONTENT)
}

/// Corps de `POST /remediation/actions/{id}/run`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RunPayload {
    /// Alerte en cours qui motive le geste : elle désigne la machine.
    fingerprint: Option<String>,
    /// Machine visée, sans alerte — réservé aux administrateurs.
    target_id: Option<i64>,
}

async fn run_action(
    State(state): State<AppState>,
    OperatorIdentity(principal): OperatorIdentity,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
    payload: Option<Json<RunPayload>>,
) -> ApiResult<(StatusCode, Json<RunRecord>)> {
    let payload = payload.map(|Json(payload)| payload).unwrap_or_default();
    let action =
        store::get_action(&state.pool, id).await?.ok_or_else(|| not_found("Action", id))?;
    let (target_id, fingerprint, rule_uid) = match payload.fingerprint {
        Some(fingerprint) => {
            let alert =
                db::alerts::get_active(&state.pool, &fingerprint).await?.ok_or_else(|| {
                    ApiError::NotFound(format!(
                        "No active alert with fingerprint \"{fingerprint}\"."
                    ))
                })?;
            // La machine est celle de l'alerte : une machine choisie dans le
            // corps n'est pas écoutée ici. Un opérateur ne lance pas, depuis
            // cette alerte, une action fixée sur une autre machine.
            if let Some(fixed) = action.target_id
                && alert.target_id != Some(fixed)
                && !principal.is_admin()
            {
                return Err(ApiError::Forbidden(
                    "This action is bound to another device than the alert's.".to_string(),
                ));
            }
            (alert.target_id, Some(fingerprint), Some(alert.rule_uid))
        }
        None if principal.is_admin() => (payload.target_id, None, None),
        None if action.target_id.is_some() => (None, None, None),
        None => {
            return Err(ApiError::Forbidden(
                "Operators run actions from an active alert, on the alert's device.".to_string(),
            ));
        }
    };
    let trigger = Trigger {
        kind: "manual",
        requested_by: principal.label(),
        fingerprint,
        rule_uid,
        policy_id: None,
        target_id,
        ip,
    };
    let http = crate::notify::http_client();
    match exec::run(&state.pool, &state.cipher, &http, &action, trigger).await {
        Ok(run) => Ok((StatusCode::CREATED, Json(run))),
        Err(ExecError::BadRequest(why)) => Err(ApiError::BadRequest(why)),
        Err(ExecError::Conflict(why)) => Err(ApiError::Conflict(why)),
        Err(ExecError::TooMany(why)) => Err(ApiError::TooManyRequests(why)),
        Err(ExecError::Internal(error)) => Err(ApiError::Internal(error)),
    }
}

// ---------------------------------------------------------------- policies

async fn list_policies(
    State(state): State<AppState>,
    Identity(_): Identity,
) -> ApiResult<Json<Vec<PolicyRecord>>> {
    Ok(Json(store::list_policies(&state.pool).await?))
}

#[derive(Debug, Deserialize)]
struct PolicyPayload {
    action_id: i64,
    rule_uid: String,
    #[serde(default)]
    target_id: Option<i64>,
    #[serde(default = "default_after")]
    after_minutes: i64,
    #[serde(default = "default_attempts")]
    max_attempts: i64,
    #[serde(default = "default_window")]
    window_minutes: i64,
}

fn default_after() -> i64 {
    5
}
fn default_attempts() -> i64 {
    1
}
fn default_window() -> i64 {
    60
}

async fn policy_draft(state: &AppState, payload: PolicyPayload) -> ApiResult<PolicyDraft> {
    if !(1..=1440).contains(&payload.after_minutes) {
        return Err(ApiError::BadRequest("after_minutes must be between 1 and 1440.".into()));
    }
    if !(1..=10).contains(&payload.max_attempts) {
        return Err(ApiError::BadRequest("max_attempts must be between 1 and 10.".into()));
    }
    if !(5..=10080).contains(&payload.window_minutes) {
        return Err(ApiError::BadRequest("window_minutes must be between 5 and 10080.".into()));
    }
    store::get_action(&state.pool, payload.action_id)
        .await?
        .ok_or_else(|| ApiError::BadRequest(format!("Action {} not found.", payload.action_id)))?;
    let rule_uid = payload.rule_uid.trim().to_string();
    let rule: Option<i64> = sqlx::query_scalar("SELECT id FROM alert_rules WHERE uid = ?")
        .bind(&rule_uid)
        .fetch_optional(&state.pool)
        .await
        .map_err(anyhow::Error::from)?;
    if rule.is_none() {
        return Err(ApiError::BadRequest(format!("Alert rule '{rule_uid}' not found.")));
    }
    if let Some(id) = payload.target_id {
        ensure_target(state, id).await?;
    }
    Ok(PolicyDraft {
        action_id: payload.action_id,
        rule_uid,
        target_id: payload.target_id,
        after_minutes: payload.after_minutes,
        max_attempts: payload.max_attempts,
        window_minutes: payload.window_minutes,
    })
}

async fn create_policy(
    State(state): State<AppState>,
    AdminIdentity(principal): AdminIdentity,
    ClientIp(ip): ClientIp,
    Json(payload): Json<PolicyPayload>,
) -> ApiResult<(StatusCode, Json<PolicyRecord>)> {
    let draft = policy_draft(&state, payload).await?;
    let id = store::create_policy(&state.pool, &draft).await?;
    let subject = format!("policy {id} ({})", draft.rule_uid);
    audit::record(&state.pool, Some(&principal.label()), "policy.created", Some(&subject), ip)
        .await;
    let policy =
        store::get_policy(&state.pool, id).await?.ok_or_else(|| not_found("Policy", id))?;
    Ok((StatusCode::CREATED, Json(policy)))
}

async fn update_policy(
    State(state): State<AppState>,
    AdminIdentity(principal): AdminIdentity,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
    Json(payload): Json<PolicyPayload>,
) -> ApiResult<Json<PolicyRecord>> {
    let draft = policy_draft(&state, payload).await?;
    if !store::update_policy(&state.pool, id, &draft).await? {
        return Err(not_found("Policy", id));
    }
    let subject = format!("policy {id}");
    audit::record(&state.pool, Some(&principal.label()), "policy.updated", Some(&subject), ip)
        .await;
    let policy =
        store::get_policy(&state.pool, id).await?.ok_or_else(|| not_found("Policy", id))?;
    Ok(Json(policy))
}

async fn delete_policy(
    State(state): State<AppState>,
    AdminIdentity(principal): AdminIdentity,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    if !store::delete_policy(&state.pool, id).await? {
        return Err(not_found("Policy", id));
    }
    let subject = format!("policy {id}");
    audit::record(&state.pool, Some(&principal.label()), "policy.deleted", Some(&subject), ip)
        .await;
    Ok(StatusCode::NO_CONTENT)
}

/// Réponse de la première étape de l'armement.
#[derive(Debug, Serialize)]
struct ArmChallenge {
    /// À renvoyer tel quel à `confirm`, par le même compte.
    token: String,
    expires_in_secs: i64,
    /// Ce qui sera fait, en clair, pour que la confirmation porte sur du concret.
    summary: String,
    policy: PolicyRecord,
}

/// Première étape : seul un administrateur connecté en personne (pas un jeton)
/// peut armer une politique.
async fn arm_policy(
    State(state): State<AppState>,
    AdminUser(user): AdminUser,
    Path(id): Path<i64>,
) -> ApiResult<Json<ArmChallenge>> {
    let policy =
        store::get_policy(&state.pool, id).await?.ok_or_else(|| not_found("Policy", id))?;
    let token = store::begin_arming(&state.pool, id, &user.username)
        .await?
        .ok_or_else(|| not_found("Policy", id))?;
    let scope = match policy.target_id {
        Some(target) => format!("device {target}"),
        None => "any device".to_string(),
    };
    let summary = format!(
        "Run \"{}\" ({}) automatically when rule \"{}\" has been firing for {} min on {}, at \
         most {} time(s) per {} min.",
        policy.action_name,
        policy.action_kind,
        policy.rule_uid,
        policy.after_minutes,
        scope,
        policy.max_attempts,
        policy.window_minutes
    );
    Ok(Json(ArmChallenge { token, expires_in_secs: store::CONFIRM_TTL_SECS, summary, policy }))
}

#[derive(Debug, Deserialize)]
struct ConfirmPayload {
    token: String,
}

async fn confirm_policy(
    State(state): State<AppState>,
    AdminUser(user): AdminUser,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
    Json(payload): Json<ConfirmPayload>,
) -> ApiResult<Json<PolicyRecord>> {
    match store::confirm_arming(&state.pool, id, &user.username, payload.token.trim()).await? {
        Confirmation::Armed => {}
        Confirmation::NotFound => return Err(not_found("Policy", id)),
        Confirmation::Refused => {
            return Err(ApiError::Forbidden(
                "Confirmation refused: start again (the code is single use, expires after two \
                 minutes and belongs to the account that asked for it)."
                    .to_string(),
            ));
        }
    }
    let subject = format!("policy {id}");
    audit::record(&state.pool, Some(&user.username), "policy.armed", Some(&subject), ip).await;
    let policy =
        store::get_policy(&state.pool, id).await?.ok_or_else(|| not_found("Policy", id))?;
    Ok(Json(policy))
}

async fn disable_policy(
    State(state): State<AppState>,
    AdminIdentity(principal): AdminIdentity,
    ClientIp(ip): ClientIp,
    Path(id): Path<i64>,
) -> ApiResult<Json<PolicyRecord>> {
    if !store::disable_policy(&state.pool, id).await? {
        return Err(not_found("Policy", id));
    }
    let subject = format!("policy {id}");
    audit::record(&state.pool, Some(&principal.label()), "policy.disabled", Some(&subject), ip)
        .await;
    let policy =
        store::get_policy(&state.pool, id).await?.ok_or_else(|| not_found("Policy", id))?;
    Ok(Json(policy))
}

// ---------------------------------------------------------------- journal

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RunsQuery {
    target_id: Option<i64>,
    fingerprint: Option<String>,
    action_id: Option<i64>,
    policy_id: Option<i64>,
    before: Option<i64>,
    limit: Option<i64>,
}

async fn list_runs(
    State(state): State<AppState>,
    Identity(_): Identity,
    Query(query): Query<RunsQuery>,
) -> ApiResult<Json<Vec<RunRecord>>> {
    let filter = RunFilter {
        target_id: query.target_id,
        fingerprint: query.fingerprint.filter(|f| !f.is_empty()),
        action_id: query.action_id,
        policy_id: query.policy_id,
        before_id: query.before,
        limit: query.limit.unwrap_or(50),
    };
    Ok(Json(store::list_runs(&state.pool, &filter).await?))
}

/// Ce que l'interface montre sous une alerte.
#[derive(Debug, Serialize)]
struct AlertContext {
    runbooks: Vec<Runbook>,
    /// Actions applicables à la machine de l'alerte.
    actions: Vec<ActionRecord>,
    policies: Vec<PolicyRecord>,
    runs: Vec<RunRecord>,
}

async fn alert_context(
    State(state): State<AppState>,
    Identity(principal): Identity,
    Path(fingerprint): Path<String>,
) -> ApiResult<Json<AlertContext>> {
    let alert = db::alerts::get_active(&state.pool, &fingerprint).await?.ok_or_else(|| {
        ApiError::NotFound(format!("No active alert with fingerprint \"{fingerprint}\"."))
    })?;
    let mut context = MatchContext::for_target(&state.pool, alert.target_id).await?;
    context.rule_uid = Some(alert.rule_uid.clone());
    let runbooks = runbooks::matching(&state.pool, &context).await?;
    let actions = store::list_actions(&state.pool)
        .await?
        .into_iter()
        .filter(|action| match action.target_id {
            Some(fixed) => alert.target_id == Some(fixed),
            None => {
                alert.target_id.is_some()
                    || ActionSpec::parse(&action.kind, &action.params)
                        .is_ok_and(|spec| !spec.needs_agent())
            }
        })
        .collect();
    let actions = shown_to(&principal, actions);
    let policies = store::list_policies(&state.pool)
        .await?
        .into_iter()
        .filter(|policy| {
            policy.rule_uid == alert.rule_uid
                && (policy.target_id.is_none() || policy.target_id == alert.target_id)
        })
        .collect();
    let runs = store::list_runs(
        &state.pool,
        &RunFilter { fingerprint: Some(fingerprint), limit: 20, ..RunFilter::default() },
    )
    .await?;
    Ok(Json(AlertContext { runbooks, actions, policies, runs }))
}

/// Ce que l'interface montre sur la fiche d'un équipement.
#[derive(Debug, Serialize)]
struct TargetContext {
    runbooks: Vec<Runbook>,
    /// Ce que l'agent de la machine déclare, quand il y en a un.
    agent: Option<AgentActions>,
    runs: Vec<RunRecord>,
}

#[derive(Debug, Serialize)]
struct AgentActions {
    host_id: i64,
    commands_enabled: bool,
    remediation: Option<dumbmonit_proto::RemediationInfo>,
}

async fn target_context(
    State(state): State<AppState>,
    Identity(_): Identity,
    Path(id): Path<i64>,
) -> ApiResult<Json<TargetContext>> {
    ensure_target(&state, id).await.map_err(|_| not_found("Monitor", id))?;
    let context = MatchContext::for_target(&state.pool, Some(id)).await?;
    let runbooks = runbooks::matching(&state.pool, &context).await?;
    let agent = match exec::agent_host(&state.pool, id).await? {
        Some(host) => exec::agent_capabilities(&state.pool, host).await?.map(
            |(commands_enabled, remediation)| AgentActions {
                host_id: host,
                commands_enabled,
                remediation,
            },
        ),
        None => None,
    };
    let runs = store::list_runs(
        &state.pool,
        &RunFilter { target_id: Some(id), limit: 20, ..RunFilter::default() },
    )
    .await?;
    Ok(Json(TargetContext { runbooks, agent, runs }))
}
