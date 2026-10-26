//! Exécution d'une action, manuelle ou automatique, et sa trace au journal.
//!
//! Les actions d'agent passent par la file de commandes existante
//! (`collectors/agent/commands.rs`) : le serveur ne joint jamais la machine, il
//! dépose une demande que l'agent vient chercher sur son canal authentifié.
//! Chaque demande porte un nonce et une échéance que l'agent vérifie.

use std::collections::BTreeMap;
use std::net::IpAddr;

use anyhow::Context;
use dumbmonit_proto::remediation::{
    ACTION_TTL_SECS, ARG_EXPIRES_AT_MS, ARG_NONCE, CMD_SCRIPT_RUN, CMD_SERVICE_RESTART,
};
use dumbmonit_proto::{CMD_CONTAINER_RESTART, RemediationInfo};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};

use super::ActionSpec;
use super::store::{self, ActionRecord, NewRun, RunRecord};
use crate::auth::audit;
use crate::collectors::agent::commands::{self, CommandError};
use crate::collectors::agent::modules;
use crate::crypto::Cipher;
use crate::notify::http::{Request, send};
use crate::notify::secret::SecretString;

/// Nombre maximal d'exécutions par minute, toutes origines confondues.
pub const MAX_RUNS_PER_MINUTE: i64 = 30;

/// D'où vient la demande.
#[derive(Debug, Clone)]
pub struct Trigger {
    /// `manual` ou `auto`.
    pub kind: &'static str,
    /// Compte, `token:nom`, ou `policy:<id>` pour l'automate.
    pub requested_by: String,
    pub fingerprint: Option<String>,
    pub rule_uid: Option<String>,
    pub policy_id: Option<i64>,
    /// Équipement visé quand l'action n'en fixe pas.
    pub target_id: Option<i64>,
    pub ip: Option<IpAddr>,
}

#[derive(Debug)]
pub enum ExecError {
    BadRequest(String),
    Conflict(String),
    TooMany(String),
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for ExecError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}

/// Exécute une action et renvoie sa ligne de journal.
///
/// Une demande refusée par les vérifications d'agent (actions désactivées,
/// service non autorisé, script absent) est journalisée comme un échec : c'est
/// une tentative, et l'automate doit pouvoir la compter.
pub async fn run(
    pool: &SqlitePool,
    cipher: &Cipher,
    http: &reqwest::Client,
    action: &ActionRecord,
    trigger: Trigger,
) -> Result<RunRecord, ExecError> {
    if store::runs_last_minute(pool).await? >= MAX_RUNS_PER_MINUTE {
        return Err(ExecError::TooMany(format!(
            "More than {MAX_RUNS_PER_MINUTE} actions in the last minute: try again shortly."
        )));
    }
    let spec = ActionSpec::parse(&action.kind, &action.params).map_err(ExecError::BadRequest)?;
    let target_id = action.target_id.or(trigger.target_id);
    if spec.needs_agent() && target_id.is_none() {
        return Err(ExecError::BadRequest("This action needs a device.".to_string()));
    }
    let target_name = match target_id {
        Some(id) => Some(
            lookup_target_name(pool, id)
                .await?
                .ok_or_else(|| ExecError::BadRequest(format!("Monitor {id} not found.")))?,
        ),
        None => None,
    };
    let mut entry = NewRun {
        action_id: Some(action.id),
        action_name: action.name.clone(),
        kind: action.kind.clone(),
        params: spec.log_params(),
        target_id,
        target_name: target_name.clone(),
        fingerprint: trigger.fingerprint.clone(),
        rule_uid: trigger.rule_uid.clone(),
        policy_id: trigger.policy_id,
        trigger: trigger.kind,
        requested_by: trigger.requested_by.clone(),
        status: "queued",
        output: None,
        agent_command_id: None,
    };

    let id = match &spec {
        ActionSpec::Webhook { url } => {
            entry.status = "running";
            let id = store::insert_run(pool, &entry).await?;
            let secrets = store::action_secrets(pool, cipher, action.id).await?;
            let body = webhook_body(action, &trigger, target_id, target_name.as_deref());
            let (status, output) = match call_webhook(http, url, &secrets, body).await {
                Ok(()) => ("done", "The webhook accepted the request.".to_string()),
                Err(error) => ("failed", error),
            };
            store::finish_run(pool, id, status, &output).await?;
            id
        }
        _ => {
            let target = target_id.unwrap_or_default();
            match agent_request(pool, target, &spec).await? {
                Err(reason) => {
                    entry.status = "failed";
                    entry.output = Some(format!("Refused before sending: {reason}"));
                    store::insert_run(pool, &entry).await?
                }
                Ok((host, kind, args)) => {
                    let queued =
                        commands::enqueue(pool, host, kind, &args, &trigger.requested_by).await;
                    let command = match queued {
                        Ok(command) => command,
                        Err(CommandError::Conflict(why)) => return Err(ExecError::Conflict(why)),
                        Err(CommandError::NotFound) => {
                            return Err(ExecError::BadRequest("Device not found.".to_string()));
                        }
                        Err(CommandError::Internal(error)) => return Err(error.into()),
                    };
                    entry.agent_command_id = Some(command.id);
                    store::insert_run(pool, &entry).await?
                }
            }
        }
    };

    let subject = format!("{} on {}", action.name, target_name.as_deref().unwrap_or("-"));
    audit::record(pool, Some(&trigger.requested_by), "remediation.run", Some(&subject), trigger.ip)
        .await;
    store::get_run(pool, id)
        .await?
        .ok_or_else(|| ExecError::Internal(anyhow::anyhow!("run {id} vanished")))
}

async fn lookup_target_name(pool: &SqlitePool, id: i64) -> anyhow::Result<Option<String>> {
    let name: Option<String> = sqlx::query_scalar("SELECT name FROM targets WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("reading the target")?;
    Ok(name)
}

/// La machine dont l'agent recevra la demande : l'équipement lui-même s'il est
/// un agent, sinon la machine qui porte son module (appareil Docker…).
pub async fn agent_host(pool: &SqlitePool, target_id: i64) -> anyhow::Result<Option<i64>> {
    let kind: Option<String> = sqlx::query_scalar("SELECT kind FROM targets WHERE id = ?")
        .bind(target_id)
        .fetch_optional(pool)
        .await
        .context("reading the target kind")?;
    match kind.as_deref() {
        Some("agent") => Ok(Some(target_id)),
        Some(_) => Ok(modules::owner(pool, target_id).await?.map(|(host, _)| host)),
        None => Ok(None),
    }
}

/// Ce que l'agent de la machine déclare : commandes acceptées, et actions de
/// remédiation (`None` pour un agent antérieur).
pub async fn agent_capabilities(
    pool: &SqlitePool,
    host: i64,
) -> anyhow::Result<Option<(bool, Option<RemediationInfo>)>> {
    let row =
        sqlx::query("SELECT commands_enabled, remediation FROM agent_hosts WHERE target_id = ?")
            .bind(host)
            .fetch_optional(pool)
            .await
            .context("reading the agent capabilities")?;
    let Some(row) = row else { return Ok(None) };
    let commands = row.try_get::<Option<i64>, _>("commands_enabled")? == Some(1);
    let remediation: Option<String> = row.try_get("remediation")?;
    let info = remediation.and_then(|text| serde_json::from_str(&text).ok());
    Ok(Some((commands, info)))
}

/// Prépare la commande d'agent, ou dit pourquoi elle serait refusée. Le serveur
/// vérifie ce que l'agent a déclaré ; l'agent revérifiera tout à l'exécution.
async fn agent_request(
    pool: &SqlitePool,
    target_id: i64,
    spec: &ActionSpec,
) -> anyhow::Result<Result<(i64, &'static str, Value), String>> {
    let Some(host) = agent_host(pool, target_id).await? else {
        return Ok(Err("this device has no agent".to_string()));
    };
    let Some((commands_enabled, info)) = agent_capabilities(pool, host).await? else {
        return Ok(Err("the agent of this device never reported".to_string()));
    };
    if !commands_enabled {
        return Ok(Err("the agent does not accept commands (commands: false)".to_string()));
    }
    let nonce = hex::encode(rand::random::<[u8; 16]>());
    let expires = chrono::Utc::now().timestamp_millis() + ACTION_TTL_SECS * 1000;
    let info = info.unwrap_or_default();
    let remediation_on = || {
        if info.enabled {
            Ok(())
        } else {
            Err("remediation actions are disabled on this machine (actions: false)".to_string())
        }
    };
    let request = match spec {
        ActionSpec::Container { container } => Ok((
            CMD_CONTAINER_RESTART,
            json!({ "name": container, ARG_NONCE: nonce, ARG_EXPIRES_AT_MS: expires }),
        )),
        ActionSpec::Service { service } => remediation_on().and_then(|()| {
            if info.allows_service(service) {
                Ok((
                    CMD_SERVICE_RESTART,
                    json!({ "name": service, ARG_NONCE: nonce, ARG_EXPIRES_AT_MS: expires }),
                ))
            } else {
                Err(format!("service '{service}' is not in the agent's action_services"))
            }
        }),
        ActionSpec::Script { script, params } => remediation_on().and_then(|()| {
            let declared = info
                .script(script)
                .ok_or_else(|| format!("script '{script}' is not declared on this machine"))?;
            declared.check_params(params)?;
            Ok((
                CMD_SCRIPT_RUN,
                json!({
                    "name": script,
                    "params": params,
                    ARG_NONCE: nonce,
                    ARG_EXPIRES_AT_MS: expires,
                }),
            ))
        }),
        ActionSpec::Webhook { .. } => Err("not an agent action".to_string()),
    };
    Ok(request.map(|(kind, args)| (host, kind, args)))
}

fn webhook_body(
    action: &ActionRecord,
    trigger: &Trigger,
    target_id: Option<i64>,
    target_name: Option<&str>,
) -> Value {
    json!({
        "source": "dumbmonit",
        "action": action.name,
        "trigger": trigger.kind,
        "requested_by": trigger.requested_by,
        "target": target_id.map(|id| json!({ "id": id, "name": target_name })),
        "alert": trigger.fingerprint.as_ref().map(|fingerprint| json!({
            "fingerprint": fingerprint,
            "rule": trigger.rule_uid,
        })),
        "at": chrono::Utc::now().to_rfc3339(),
    })
}

/// POST JSON par le client des notifications : même garde anti-SSRF (refus
/// des adresses du serveur lui-même, à la résolution et à chaque
/// redirection), même délai, même expurgation des secrets dans les erreurs.
async fn call_webhook(
    http: &reqwest::Client,
    url: &str,
    secrets: &BTreeMap<String, String>,
    body: Value,
) -> Result<(), String> {
    let mut request = Request::post(url).json(body).secret_text(url.to_string());
    if let Some(token) = secrets.get("token").filter(|token| !token.is_empty()) {
        request = request.bearer(&SecretString::new(token.clone()));
    }
    send(http, request).await.map_err(|error| error.to_string())
}
