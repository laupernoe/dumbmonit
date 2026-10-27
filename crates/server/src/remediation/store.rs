//! Actions, politiques et journal de la remédiation, en base.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};

use crate::crypto::Cipher;

/// Durée de validité du jeton de confirmation d'une politique, en secondes.
pub const CONFIRM_TTL_SECS: i64 = 120;

/// Nombre maximal de lignes renvoyées par une lecture du journal.
pub const MAX_RUNS_PAGE: i64 = 200;

/// Une action, telle que l'API la renvoie : jamais ses secrets, seulement leur
/// présence.
#[derive(Debug, Clone, Serialize)]
pub struct ActionRecord {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub params: serde_json::Value,
    pub has_secret: bool,
    pub target_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

/// Ce qu'on enregistre d'une action. `secrets` : `None` garde ceux en place,
/// `Some({})` les efface — la règle de toute l'API.
#[derive(Debug, Clone)]
pub struct ActionDraft {
    pub name: String,
    pub kind: String,
    pub params: serde_json::Value,
    pub target_id: Option<i64>,
    pub secrets: Option<BTreeMap<String, String>>,
}

const ACTION_COLUMNS: &str = "id, name, kind, params, secrets IS NOT NULL AS has_secret, \
                              target_id, created_at, updated_at";

fn row_to_action(row: &sqlx::sqlite::SqliteRow) -> Result<ActionRecord> {
    let params: String = row.try_get("params")?;
    Ok(ActionRecord {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        kind: row.try_get("kind")?,
        params: serde_json::from_str(&params).unwrap_or(serde_json::Value::Null),
        has_secret: row.try_get::<i64, _>("has_secret")? != 0,
        target_id: row.try_get("target_id")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

pub async fn list_actions(pool: &SqlitePool) -> Result<Vec<ActionRecord>> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ACTION_COLUMNS} FROM remediation_actions ORDER BY name"
    )))
    .fetch_all(pool)
    .await
    .context("listing actions")?;
    rows.iter().map(row_to_action).collect()
}

pub async fn get_action(pool: &SqlitePool, id: i64) -> Result<Option<ActionRecord>> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!(
        "SELECT {ACTION_COLUMNS} FROM remediation_actions WHERE id = ?"
    )))
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("reading the action")?;
    row.as_ref().map(row_to_action).transpose()
}

/// Secrets d'une action, déchiffrés pour l'exécution. Jamais renvoyés à l'API.
pub async fn action_secrets(
    pool: &SqlitePool,
    cipher: &Cipher,
    id: i64,
) -> Result<BTreeMap<String, String>> {
    let blob: Option<Vec<u8>> = sqlx::query_scalar::<_, Option<Vec<u8>>>(
        "SELECT secrets FROM remediation_actions WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("reading the action secrets")?
    .flatten();
    let Some(blob) = blob else { return Ok(BTreeMap::new()) };
    let plain = cipher.decrypt(&blob).context("decrypting the action secrets")?;
    Ok(serde_json::from_slice(&plain).unwrap_or_default())
}

fn encrypt_secrets(cipher: &Cipher, secrets: &BTreeMap<String, String>) -> Result<Option<Vec<u8>>> {
    let kept: BTreeMap<&String, &String> =
        secrets.iter().filter(|(_, value)| !value.is_empty()).collect();
    if kept.is_empty() {
        return Ok(None);
    }
    Ok(Some(cipher.encrypt(&serde_json::to_vec(&kept)?)?))
}

pub async fn create_action(pool: &SqlitePool, cipher: &Cipher, draft: &ActionDraft) -> Result<i64> {
    let secrets = match &draft.secrets {
        Some(secrets) => encrypt_secrets(cipher, secrets)?,
        None => None,
    };
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO remediation_actions (name, kind, params, secrets, target_id)
         VALUES (?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(&draft.name)
    .bind(&draft.kind)
    .bind(draft.params.to_string())
    .bind(secrets)
    .bind(draft.target_id)
    .fetch_one(pool)
    .await
    .context("creating the action")?;
    Ok(id)
}

/// Modifie une action et **désarme** toutes les politiques qui l'emploient :
/// une politique confirmée pour « redémarrer nginx » ne doit pas se mettre à
/// lancer autre chose sans nouvelle confirmation.
pub async fn update_action(
    pool: &SqlitePool,
    cipher: &Cipher,
    id: i64,
    draft: &ActionDraft,
) -> Result<bool> {
    let mut tx = pool.begin().await?;
    let result = match &draft.secrets {
        Some(secrets) => sqlx::query(
            "UPDATE remediation_actions SET name = ?, kind = ?, params = ?, target_id = ?,
                 secrets = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%S', 'now')
             WHERE id = ?",
        )
        .bind(&draft.name)
        .bind(&draft.kind)
        .bind(draft.params.to_string())
        .bind(draft.target_id)
        .bind(encrypt_secrets(cipher, secrets)?)
        .bind(id)
        .execute(&mut *tx)
        .await
        .context("updating the action")?,
        None => sqlx::query(
            "UPDATE remediation_actions SET name = ?, kind = ?, params = ?, target_id = ?,
                 updated_at = strftime('%Y-%m-%dT%H:%M:%S', 'now')
             WHERE id = ?",
        )
        .bind(&draft.name)
        .bind(&draft.kind)
        .bind(draft.params.to_string())
        .bind(draft.target_id)
        .bind(id)
        .execute(&mut *tx)
        .await
        .context("updating the action")?,
    };
    if result.rows_affected() == 0 {
        return Ok(false);
    }
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE remediation_policies SET {DISARM} WHERE action_id = ?"
    )))
    .bind(id)
    .execute(&mut *tx)
    .await
    .context("disarming the policies of the action")?;
    tx.commit().await?;
    Ok(true)
}

pub async fn delete_action(pool: &SqlitePool, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM remediation_actions WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("deleting the action")?;
    Ok(result.rows_affected() > 0)
}

// ------------------------------------------------------------ politiques

/// Ce que remet à zéro toute modification : la politique redevient désarmée et
/// doit être confirmée de nouveau.
const DISARM: &str = "enabled = 0, armed_by = NULL, armed_at = NULL, confirm_hash = NULL, \
                      confirm_user = NULL, confirm_expires_at = NULL, \
                      updated_at = strftime('%Y-%m-%dT%H:%M:%S', 'now')";

#[derive(Debug, Clone, Serialize)]
pub struct PolicyRecord {
    pub id: i64,
    pub action_id: i64,
    pub action_name: String,
    pub action_kind: String,
    pub rule_uid: String,
    pub target_id: Option<i64>,
    pub after_minutes: i64,
    pub max_attempts: i64,
    pub window_minutes: i64,
    pub enabled: bool,
    pub armed_by: Option<String>,
    pub armed_at: Option<String>,
    pub halted_at: Option<String>,
    pub halt_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct PolicyDraft {
    pub action_id: i64,
    pub rule_uid: String,
    pub target_id: Option<i64>,
    pub after_minutes: i64,
    pub max_attempts: i64,
    pub window_minutes: i64,
}

const POLICY_SELECT: &str =
    "SELECT p.id, p.action_id, a.name AS action_name, a.kind AS action_kind,
        p.rule_uid, p.target_id, p.after_minutes, p.max_attempts, p.window_minutes, p.enabled,
        p.armed_by, p.armed_at, p.halted_at, p.halt_reason, p.created_at, p.updated_at
     FROM remediation_policies p JOIN remediation_actions a ON a.id = p.action_id";

fn row_to_policy(row: &sqlx::sqlite::SqliteRow) -> Result<PolicyRecord> {
    Ok(PolicyRecord {
        id: row.try_get("id")?,
        action_id: row.try_get("action_id")?,
        action_name: row.try_get("action_name")?,
        action_kind: row.try_get("action_kind")?,
        rule_uid: row.try_get("rule_uid")?,
        target_id: row.try_get("target_id")?,
        after_minutes: row.try_get("after_minutes")?,
        max_attempts: row.try_get("max_attempts")?,
        window_minutes: row.try_get("window_minutes")?,
        enabled: row.try_get::<i64, _>("enabled")? != 0,
        armed_by: row.try_get("armed_by")?,
        armed_at: row.try_get("armed_at")?,
        halted_at: row.try_get("halted_at")?,
        halt_reason: row.try_get("halt_reason")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

pub async fn list_policies(pool: &SqlitePool) -> Result<Vec<PolicyRecord>> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!("{POLICY_SELECT} ORDER BY p.id")))
        .fetch_all(pool)
        .await
        .context("listing policies")?;
    rows.iter().map(row_to_policy).collect()
}

/// Politiques armées et non arrêtées : ce que l'automate doit suivre.
pub async fn active_policies(pool: &SqlitePool) -> Result<Vec<PolicyRecord>> {
    let rows = sqlx::query(sqlx::AssertSqlSafe(format!(
        "{POLICY_SELECT} WHERE p.enabled = 1 AND p.halted_at IS NULL"
    )))
    .fetch_all(pool)
    .await
    .context("listing active policies")?;
    rows.iter().map(row_to_policy).collect()
}

pub async fn get_policy(pool: &SqlitePool, id: i64) -> Result<Option<PolicyRecord>> {
    let row = sqlx::query(sqlx::AssertSqlSafe(format!("{POLICY_SELECT} WHERE p.id = ?")))
        .bind(id)
        .fetch_optional(pool)
        .await
        .context("reading the policy")?;
    row.as_ref().map(row_to_policy).transpose()
}

/// Crée une politique, toujours désarmée.
pub async fn create_policy(pool: &SqlitePool, draft: &PolicyDraft) -> Result<i64> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO remediation_policies
             (action_id, rule_uid, target_id, after_minutes, max_attempts, window_minutes, enabled)
         VALUES (?, ?, ?, ?, ?, ?, 0) RETURNING id",
    )
    .bind(draft.action_id)
    .bind(&draft.rule_uid)
    .bind(draft.target_id)
    .bind(draft.after_minutes)
    .bind(draft.max_attempts)
    .bind(draft.window_minutes)
    .fetch_one(pool)
    .await
    .context("creating the policy")?;
    Ok(id)
}

/// Modifie une politique et la désarme.
pub async fn update_policy(pool: &SqlitePool, id: i64, draft: &PolicyDraft) -> Result<bool> {
    let result = sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE remediation_policies SET action_id = ?, rule_uid = ?, target_id = ?,
             after_minutes = ?, max_attempts = ?, window_minutes = ?, halted_at = NULL,
             halt_reason = NULL, {DISARM}
         WHERE id = ?"
    )))
    .bind(draft.action_id)
    .bind(&draft.rule_uid)
    .bind(draft.target_id)
    .bind(draft.after_minutes)
    .bind(draft.max_attempts)
    .bind(draft.window_minutes)
    .bind(id)
    .execute(pool)
    .await
    .context("updating the policy")?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_policy(pool: &SqlitePool, id: i64) -> Result<bool> {
    let result = sqlx::query("DELETE FROM remediation_policies WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .context("deleting the policy")?;
    Ok(result.rows_affected() > 0)
}

pub async fn disable_policy(pool: &SqlitePool, id: i64) -> Result<bool> {
    let result = sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE remediation_policies SET {DISARM} WHERE id = ?"
    )))
    .bind(id)
    .execute(pool)
    .await
    .context("disabling the policy")?;
    Ok(result.rows_affected() > 0)
}

fn hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Première étape de l'armement : un jeton à usage unique, valable
/// [`CONFIRM_TTL_SECS`], que seul `user` pourra présenter. Seule son empreinte
/// est gardée en base.
pub async fn begin_arming(pool: &SqlitePool, id: i64, user: &str) -> Result<Option<String>> {
    let token = hex::encode(rand::random::<[u8; 16]>());
    let result = sqlx::query(
        "UPDATE remediation_policies
         SET confirm_hash = ?, confirm_user = ?,
             confirm_expires_at = strftime('%Y-%m-%dT%H:%M:%S', 'now', ?)
         WHERE id = ?",
    )
    .bind(hash(&token))
    .bind(user)
    .bind(format!("+{CONFIRM_TTL_SECS} seconds"))
    .bind(id)
    .execute(pool)
    .await
    .context("starting the arming")?;
    Ok((result.rows_affected() > 0).then_some(token))
}

/// Issue de la seconde étape.
#[derive(Debug, PartialEq, Eq)]
pub enum Confirmation {
    Armed,
    NotFound,
    /// Pas de demande en cours, jeton faux, autre compte ou délai dépassé.
    Refused,
}

/// Seconde étape : le jeton, par le même compte, dans le délai. Le jeton est
/// consommé dans tous les cas où il existait : une erreur ne laisse pas de
/// seconde chance à un jeton deviné.
pub async fn confirm_arming(
    pool: &SqlitePool,
    id: i64,
    user: &str,
    token: &str,
) -> Result<Confirmation> {
    let row = sqlx::query(
        "SELECT confirm_hash, confirm_user,
                confirm_expires_at > strftime('%Y-%m-%dT%H:%M:%S', 'now') AS fresh
         FROM remediation_policies WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .context("reading the arming")?;
    let Some(row) = row else { return Ok(Confirmation::NotFound) };
    let stored: Option<String> = row.try_get("confirm_hash")?;
    let by: Option<String> = row.try_get("confirm_user")?;
    let fresh: Option<i64> = row.try_get("fresh")?;
    let Some(stored) = stored else { return Ok(Confirmation::Refused) };
    // Le jeton est consommé avant d'être comparé.
    sqlx::query(
        "UPDATE remediation_policies
         SET confirm_hash = NULL, confirm_user = NULL, confirm_expires_at = NULL WHERE id = ?",
    )
    .bind(id)
    .execute(pool)
    .await
    .context("consuming the arming token")?;
    let same_token: bool =
        subtle::ConstantTimeEq::ct_eq(stored.as_bytes(), hash(token).as_bytes()).into();
    if !same_token || by.as_deref() != Some(user) || fresh != Some(1) {
        return Ok(Confirmation::Refused);
    }
    sqlx::query(
        "UPDATE remediation_policies
         SET enabled = 1, armed_by = ?, armed_at = strftime('%Y-%m-%dT%H:%M:%S', 'now'),
             halted_at = NULL, halt_reason = NULL,
             updated_at = strftime('%Y-%m-%dT%H:%M:%S', 'now')
         WHERE id = ?",
    )
    .bind(user)
    .bind(id)
    .execute(pool)
    .await
    .context("arming the policy")?;
    Ok(Confirmation::Armed)
}

/// Arrête une politique (deux échecs de suite) : elle ne se déclenche plus
/// tant qu'un administrateur ne l'a pas réarmée.
pub async fn halt_policy(pool: &SqlitePool, id: i64, reason: &str) -> Result<()> {
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "UPDATE remediation_policies
         SET halted_at = strftime('%Y-%m-%dT%H:%M:%S', 'now'), halt_reason = ?, {DISARM}
         WHERE id = ?"
    )))
    .bind(reason)
    .bind(id)
    .execute(pool)
    .await
    .context("halting the policy")?;
    Ok(())
}

// ------------------------------------------------------------ journal

#[derive(Debug, Clone, Serialize)]
pub struct RunRecord {
    pub id: i64,
    pub action_id: Option<i64>,
    pub action_name: String,
    pub kind: String,
    pub params: serde_json::Value,
    pub target_id: Option<i64>,
    pub target_name: Option<String>,
    pub fingerprint: Option<String>,
    pub rule_uid: Option<String>,
    pub policy_id: Option<i64>,
    pub trigger: String,
    pub requested_by: String,
    pub status: String,
    pub output: Option<String>,
    pub created_at: String,
    pub finished_at: Option<String>,
}

/// Une ligne de journal à écrire.
#[derive(Debug, Clone, Default)]
pub struct NewRun {
    pub action_id: Option<i64>,
    pub action_name: String,
    pub kind: String,
    /// Paramètres **sans secrets**.
    pub params: serde_json::Value,
    pub target_id: Option<i64>,
    pub target_name: Option<String>,
    pub fingerprint: Option<String>,
    pub rule_uid: Option<String>,
    pub policy_id: Option<i64>,
    pub trigger: &'static str,
    pub requested_by: String,
    pub status: &'static str,
    pub output: Option<String>,
    pub agent_command_id: Option<i64>,
}

pub async fn insert_run(pool: &SqlitePool, run: &NewRun) -> Result<i64> {
    let output = run.output.as_deref().map(dumbmonit_proto::truncate_result);
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO remediation_runs
             (action_id, action_name, kind, params, target_id, target_name, fingerprint, rule_uid,
              policy_id, origin, requested_by, status, output, agent_command_id, finished_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                 CASE WHEN ? IN ('done', 'failed', 'cancelled', 'expired')
                      THEN strftime('%Y-%m-%dT%H:%M:%S', 'now') END)
         RETURNING id",
    )
    .bind(run.action_id)
    .bind(&run.action_name)
    .bind(&run.kind)
    .bind(run.params.to_string())
    .bind(run.target_id)
    .bind(&run.target_name)
    .bind(&run.fingerprint)
    .bind(&run.rule_uid)
    .bind(run.policy_id)
    .bind(run.trigger)
    .bind(&run.requested_by)
    .bind(run.status)
    .bind(output)
    .bind(run.agent_command_id)
    .bind(run.status)
    .fetch_one(pool)
    .await
    .context("recording the run")?;
    Ok(id)
}

pub async fn finish_run(pool: &SqlitePool, id: i64, status: &str, output: &str) -> Result<()> {
    sqlx::query(
        "UPDATE remediation_runs
         SET status = ?, output = ?, finished_at = strftime('%Y-%m-%dT%H:%M:%S', 'now')
         WHERE id = ?",
    )
    .bind(status)
    .bind(dumbmonit_proto::truncate_result(output))
    .bind(id)
    .execute(pool)
    .await
    .context("closing the run")?;
    Ok(())
}

/// Recopie l'état des commandes d'agent sur les lignes de journal qui les
/// attendent. Appelée à chaque compte rendu, expiration ou annulation.
pub async fn sync_runs(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "UPDATE remediation_runs
         SET status = c.status,
             output = COALESCE(c.result, remediation_runs.output),
             finished_at = CASE WHEN c.status IN ('done', 'failed', 'cancelled', 'expired')
                                THEN COALESCE(remediation_runs.finished_at,
                                              strftime('%Y-%m-%dT%H:%M:%S', 'now'))
                           END
         FROM agent_commands c
         WHERE c.id = remediation_runs.agent_command_id
           AND remediation_runs.status IN ('queued', 'running')
           AND c.status != remediation_runs.status",
    )
    .execute(pool)
    .await
    .context("syncing the remediation log")?;
    Ok(())
}

/// Filtre de lecture du journal.
#[derive(Debug, Clone, Default)]
pub struct RunFilter {
    pub target_id: Option<i64>,
    pub fingerprint: Option<String>,
    pub action_id: Option<i64>,
    pub policy_id: Option<i64>,
    pub before_id: Option<i64>,
    pub limit: i64,
}

pub async fn list_runs(pool: &SqlitePool, filter: &RunFilter) -> Result<Vec<RunRecord>> {
    let limit = filter.limit.clamp(1, MAX_RUNS_PAGE);
    let rows = sqlx::query(
        "SELECT id, action_id, action_name, kind, params, target_id, target_name, fingerprint,
                rule_uid, policy_id, origin, requested_by, status, output, created_at, finished_at
         FROM remediation_runs
         WHERE (? IS NULL OR target_id = ?)
           AND (? IS NULL OR fingerprint = ?)
           AND (? IS NULL OR action_id = ?)
           AND (? IS NULL OR policy_id = ?)
           AND (? IS NULL OR id < ?)
         ORDER BY id DESC LIMIT ?",
    )
    .bind(filter.target_id)
    .bind(filter.target_id)
    .bind(&filter.fingerprint)
    .bind(&filter.fingerprint)
    .bind(filter.action_id)
    .bind(filter.action_id)
    .bind(filter.policy_id)
    .bind(filter.policy_id)
    .bind(filter.before_id)
    .bind(filter.before_id)
    .bind(limit)
    .fetch_all(pool)
    .await
    .context("listing runs")?;
    rows.iter()
        .map(|row| {
            let params: String = row.try_get("params")?;
            Ok(RunRecord {
                id: row.try_get("id")?,
                action_id: row.try_get("action_id")?,
                action_name: row.try_get("action_name")?,
                kind: row.try_get("kind")?,
                params: serde_json::from_str(&params).unwrap_or(serde_json::Value::Null),
                target_id: row.try_get("target_id")?,
                target_name: row.try_get("target_name")?,
                fingerprint: row.try_get("fingerprint")?,
                rule_uid: row.try_get("rule_uid")?,
                policy_id: row.try_get("policy_id")?,
                trigger: row.try_get("origin")?,
                requested_by: row.try_get("requested_by")?,
                status: row.try_get("status")?,
                output: row.try_get("output")?,
                created_at: row.try_get("created_at")?,
                finished_at: row.try_get("finished_at")?,
            })
        })
        .collect()
}

pub async fn get_run(pool: &SqlitePool, id: i64) -> Result<Option<RunRecord>> {
    let runs =
        list_runs(pool, &RunFilter { before_id: Some(id + 1), limit: 1, ..Default::default() })
            .await?;
    Ok(runs.into_iter().find(|run| run.id == id))
}

/// Exécutions d'une politique sur un équipement depuis `minutes` minutes.
pub async fn policy_runs_since(
    pool: &SqlitePool,
    policy_id: i64,
    target_id: Option<i64>,
    minutes: i64,
) -> Result<i64> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM remediation_runs
         WHERE policy_id = ? AND target_id IS ?
           AND created_at >= strftime('%Y-%m-%dT%H:%M:%S', 'now', ?)",
    )
    .bind(policy_id)
    .bind(target_id)
    .bind(format!("-{minutes} minutes"))
    .fetch_one(pool)
    .await
    .context("counting policy runs")?;
    Ok(count)
}

/// Vrai si une exécution de la politique attend ou tourne encore.
pub async fn policy_has_open_run(pool: &SqlitePool, policy_id: i64) -> Result<bool> {
    let open: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM remediation_runs
         WHERE policy_id = ? AND status IN ('queued', 'running')",
    )
    .bind(policy_id)
    .fetch_one(pool)
    .await
    .context("reading open policy runs")?;
    Ok(open > 0)
}

/// États des deux dernières exécutions closes de la politique depuis son
/// armement, la plus récente en tête : réarmer une politique arrêtée efface
/// son ardoise.
pub async fn policy_last_outcomes(
    pool: &SqlitePool,
    policy_id: i64,
    armed_at: Option<&str>,
) -> Result<Vec<String>> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT status FROM remediation_runs
         WHERE policy_id = ? AND status IN ('done', 'failed', 'cancelled', 'expired')
           AND (? IS NULL OR created_at >= ?)
         ORDER BY id DESC LIMIT 2",
    )
    .bind(policy_id)
    .bind(armed_at)
    .bind(armed_at)
    .fetch_all(pool)
    .await
    .context("reading policy outcomes")?;
    Ok(rows)
}

/// Nombre d'exécutions demandées depuis une minute, toutes origines : borne
/// globale contre l'emballement (DoS par l'interface ou par une politique).
pub async fn runs_last_minute(pool: &SqlitePool) -> Result<i64> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM remediation_runs
         WHERE created_at >= strftime('%Y-%m-%dT%H:%M:%S', 'now', '-60 seconds')",
    )
    .fetch_one(pool)
    .await
    .context("counting recent runs")?;
    Ok(count)
}

/// Purge du journal au-delà de `days` jours.
pub async fn purge_runs(pool: &SqlitePool, days: i64) -> Result<u64> {
    let result = sqlx::query(
        "DELETE FROM remediation_runs
         WHERE created_at < strftime('%Y-%m-%dT%H:%M:%S', 'now', ?)",
    )
    .bind(format!("-{days} days"))
    .execute(pool)
    .await
    .context("purging the remediation log")?;
    Ok(result.rows_affected())
}
