//! Automate des politiques de remédiation.
//!
//! Toutes les trente secondes, pour chaque politique armée et non arrêtée, il
//! cherche les alertes de sa règle qui sonnent depuis assez longtemps, et ne
//! déclenche l'action que si **toutes** les gardes le permettent :
//!
//! - l'alerte sonne (`firing`) depuis au moins `after_minutes` ;
//! - elle n'est ni en maintenance/silence, ni supprimée parce qu'un parent est
//!   en panne, ni écartée (« j'ai compris ») ;
//! - aucune exécution de la politique n'attend ou ne tourne encore ;
//! - moins de `max_attempts` exécutions sur cet équipement dans la fenêtre ;
//! - la borne globale d'exécutions par minute n'est pas atteinte.
//!
//! Deux échecs de suite arrêtent la politique : un redémarrage qui ne répare
//! rien ne doit pas tourner en boucle sur une machine malade.

use std::time::Duration;

use anyhow::Result;
use chrono::{TimeDelta, Utc};
use tokio::time::{MissedTickBehavior, interval};
use tracing::{debug, info, warn};

use super::exec::{self, ExecError, Trigger};
use super::store::{self, PolicyRecord};
use crate::alerting::cycle::StoredAlert;
use crate::alerting::machine::Phase;
use crate::db;
use crate::state::AppState;

/// Période de l'automate.
const TICK: Duration = Duration::from_secs(30);

/// Rétention du journal de remédiation, en jours.
const LOG_RETENTION_DAYS: i64 = 180;

pub async fn run(state: AppState) {
    let http = crate::notify::http_client();
    let mut ticker = interval(TICK);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut last_purge = Utc::now() - TimeDelta::days(1);
    loop {
        ticker.tick().await;
        if let Err(error) = tick(&state, &http).await {
            warn!(?error, "remediation cycle failed");
        }
        if Utc::now() - last_purge >= TimeDelta::hours(6) {
            last_purge = Utc::now();
            match store::purge_runs(&state.pool, LOG_RETENTION_DAYS).await {
                Ok(0) => {}
                Ok(count) => debug!(count, "old remediation runs purged"),
                Err(error) => warn!(?error, "remediation log purge failed"),
            }
        }
    }
}

/// Un passage de l'automate. Public pour les tests.
pub async fn tick(state: &AppState, http: &reqwest::Client) -> Result<usize> {
    let policies = store::active_policies(&state.pool).await?;
    if policies.is_empty() {
        return Ok(0);
    }
    let alerts = db::alerts::list_active(&state.pool).await?;
    let mut fired = 0;
    for policy in &policies {
        fired += run_policy(state, http, policy, &alerts).await?;
    }
    Ok(fired)
}

/// Vrai si l'alerte réunit les conditions d'un déclenchement automatique.
pub fn eligible(policy: &PolicyRecord, alert: &StoredAlert, now: chrono::DateTime<Utc>) -> bool {
    if alert.rule_uid != policy.rule_uid {
        return false;
    }
    if policy.target_id.is_some() && alert.target_id != policy.target_id {
        return false;
    }
    let state = &alert.state;
    if state.phase != Phase::Firing || state.silenced || state.suppressed || state.is_dismissed() {
        return false;
    }
    let Some(since) = state.firing_since else { return false };
    now - since >= TimeDelta::minutes(policy.after_minutes)
}

async fn run_policy(
    state: &AppState,
    http: &reqwest::Client,
    policy: &PolicyRecord,
    alerts: &[StoredAlert],
) -> Result<usize> {
    // Deux échecs de suite : arrêt, avant toute nouvelle tentative.
    let outcomes =
        store::policy_last_outcomes(&state.pool, policy.id, policy.armed_at.as_deref()).await?;
    if outcomes.len() == 2 && outcomes.iter().all(|status| status != "done") {
        let reason = "Stopped after two failed runs in a row.";
        store::halt_policy(&state.pool, policy.id, reason).await?;
        warn!(policy = policy.id, "remediation policy halted after two failures");
        return Ok(0);
    }
    if store::policy_has_open_run(&state.pool, policy.id).await? {
        return Ok(0);
    }
    let now = Utc::now();
    let Some(action) = store::get_action(&state.pool, policy.action_id).await? else {
        return Ok(0);
    };
    // Mode MSP : l'action d'un client ne se déclenche que sur ses équipements.
    let owner = crate::auth::clients::owner_of(
        &state.pool,
        crate::auth::clients::Kind::RemediationAction,
        action.id,
    )
    .await?
    .flatten();
    for alert in alerts.iter().filter(|alert| eligible(policy, alert, now)) {
        if let Some(owner) = owner {
            let client = match alert.target_id {
                Some(target) => crate::auth::clients::target_client(&state.pool, target).await?,
                None => None,
            };
            if client != Some(owner) {
                continue;
            }
        }
        let recent = store::policy_runs_since(
            &state.pool,
            policy.id,
            alert.target_id,
            policy.window_minutes,
        )
        .await?;
        if recent >= policy.max_attempts {
            continue;
        }
        let trigger = Trigger {
            kind: "auto",
            requested_by: format!("policy:{}", policy.id),
            fingerprint: Some(alert.fingerprint.clone()),
            rule_uid: Some(alert.rule_uid.clone()),
            policy_id: Some(policy.id),
            target_id: alert.target_id,
            ip: None,
        };
        match exec::run(&state.pool, &state.cipher, http, &action, trigger).await {
            Ok(run) => {
                info!(policy = policy.id, run = run.id, status = %run.status, "remediation fired");
                // Une exécution à la fois par politique : la suivante attendra
                // l'issue de celle-ci au prochain passage.
                return Ok(1);
            }
            Err(ExecError::Conflict(why)) | Err(ExecError::TooMany(why)) => {
                debug!(policy = policy.id, %why, "remediation postponed");
                return Ok(0);
            }
            Err(ExecError::BadRequest(why)) => {
                store::halt_policy(&state.pool, policy.id, &why).await?;
                warn!(policy = policy.id, %why, "remediation policy halted");
                return Ok(0);
            }
            Err(ExecError::Internal(error)) => return Err(error),
        }
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::alerting::machine::AlertState;

    fn policy() -> PolicyRecord {
        PolicyRecord {
            id: 1,
            action_id: 1,
            action_name: "restart".into(),
            action_kind: "service.restart".into(),
            rule_uid: "host_down".into(),
            target_id: None,
            after_minutes: 5,
            max_attempts: 1,
            window_minutes: 60,
            enabled: true,
            armed_by: Some("admin".into()),
            armed_at: None,
            halted_at: None,
            halt_reason: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn alert(minutes: i64, tweak: impl FnOnce(&mut AlertState)) -> StoredAlert {
        let mut state = AlertState {
            phase: Phase::Firing,
            firing_since: Some(Utc::now() - TimeDelta::minutes(minutes)),
            ..AlertState::default()
        };
        tweak(&mut state);
        StoredAlert {
            fingerprint: "host_down@3".into(),
            rule_uid: "host_down".into(),
            target_id: Some(3),
            series_key: String::new(),
            labels: BTreeMap::new(),
            state,
        }
    }

    #[test]
    fn only_long_enough_unsilenced_unsuppressed_alerts_trigger() {
        let now = Utc::now();
        let p = policy();
        assert!(eligible(&p, &alert(6, |_| {}), now));
        assert!(!eligible(&p, &alert(2, |_| {}), now), "not down long enough");
        assert!(!eligible(&p, &alert(6, |s| s.silenced = true), now), "maintenance");
        assert!(!eligible(&p, &alert(6, |s| s.suppressed = true), now), "parent down");
        assert!(!eligible(&p, &alert(6, |s| s.phase = Phase::Pending), now));
        assert!(!eligible(&p, &alert(6, |s| s.dismissed_at = Some(now)), now));
        let other = PolicyRecord { target_id: Some(4), ..policy() };
        assert!(!eligible(&other, &alert(6, |_| {}), now), "another device");
        let rule = PolicyRecord { rule_uid: "cpu_high".into(), ..policy() };
        assert!(!eligible(&rule, &alert(6, |_| {}), now), "another rule");
    }
}
