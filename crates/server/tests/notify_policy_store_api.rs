//! Persistance de la politique de notification : politique globale et par canal,
//! surcharges par équipement, file d'attente, registre des envois et entretien.
//! Les décisions elles-mêmes sont testées dans `alerting::notify_policy` ; ici
//! on vérifie que ce qui est décidé s'écrit et se relit à l'identique.

use std::collections::HashMap;

use chrono::{DateTime, TimeDelta, Utc};
use dumbmonit_server::alerting::group::{GroupItem, NotifyReason};
use dumbmonit_server::alerting::machine::EffectivePhase;
use dumbmonit_server::alerting::model::Severity;
use dumbmonit_server::alerting::notify_policy::{
    ChannelPolicy, GlobalPolicy, Hold, LogEntry, Plan, QueuedItem,
};
use dumbmonit_server::alerting::overrides::RuleOverride;
use dumbmonit_server::db;
use dumbmonit_server::notify::policy_store as store;
use sqlx::SqlitePool;

async fn pool() -> (SqlitePool, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let pool = db::open(&dir.path().join("dumbmonit.db")).await.expect("base");
    (pool, dir)
}

async fn channel(pool: &SqlitePool, name: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO notification_channels (name, kind, enabled, settings) \
         VALUES (?, 'discord', 1, '{}') RETURNING id",
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("canal")
}

async fn target(pool: &SqlitePool, address: &str) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO targets (name, address, kind, interval_secs, enabled, tags) \
         VALUES (?, ?, 'dummy', 60, 1, '{}') RETURNING id",
    )
    .bind(address)
    .bind(address)
    .fetch_one(pool)
    .await
    .expect("cible")
}

fn item(fingerprint: &str, reason: NotifyReason) -> GroupItem {
    GroupItem {
        fingerprint: fingerprint.to_string(),
        rule_uid: "cpu_high".to_string(),
        rule_name: "CPU high".to_string(),
        severity: Severity::Warning,
        reason,
        value: Some(93.0),
        score: None,
        unit: "%".to_string(),
        operator: ">".to_string(),
        threshold: 90.0,
        series_key: "cpu".to_string(),
        since: Some(Utc::now()),
        phase: EffectivePhase::Firing,
        note: None,
        detail: None,
    }
}

fn queued(channel_id: i64, fingerprint: &str, at: DateTime<Utc>) -> QueuedItem {
    QueuedItem {
        id: None,
        channel_id,
        target_id: None,
        target_name: "nas".to_string(),
        hold: Hold::Batch,
        item: item(fingerprint, NotifyReason::Firing),
        resolved_meanwhile: false,
        queued_at: at,
    }
}

#[tokio::test]
async fn la_politique_globale_et_celle_des_canaux_se_relisent() {
    let (pool, _dir) = pool().await;

    // Sans réglage, la politique par défaut s'applique.
    assert_eq!(store::load_global(&pool).await.unwrap(), GlobalPolicy::default());
    let global = GlobalPolicy { batch_window_secs: 90, max_per_hour: 5, ..Default::default() };
    store::save_global(&pool, &global).await.unwrap();
    assert_eq!(store::load_global(&pool).await.unwrap(), global);

    let calm = channel(&pool, "Calme").await;
    let noisy = channel(&pool, "Bavard").await;
    let policy = ChannelPolicy {
        min_severity: Severity::Critical,
        notify_resolved: false,
        min_interval_secs: 600,
        ..Default::default()
    };
    store::save_channel_policy(&pool, calm, &policy).await.unwrap();
    // Un JSON illisible retombe sur la politique par défaut : mieux vaut un canal
    // bavard qu'un canal muet.
    sqlx::query("UPDATE notification_channels SET policy = 'pas du json' WHERE id = ?")
        .bind(noisy)
        .execute(&pool)
        .await
        .unwrap();
    let policies = store::load_channel_policies(&pool).await.unwrap();
    assert_eq!(policies[&calm], policy);
    assert_eq!(policies[&noisy], ChannelPolicy::default());
}

#[tokio::test]
async fn les_surcharges_se_creent_se_listent_se_filtrent_et_s_effacent() {
    let (pool, _dir) = pool().await;
    let a = target(&pool, "a.lan").await;
    let b = target(&pool, "b.lan").await;

    let over =
        |rule: &str, target_id: i64, threshold: Option<f64>, enabled: Option<bool>| RuleOverride {
            rule_uid: rule.to_string(),
            target_id,
            threshold,
            clear_threshold: None,
            enabled,
        };
    store::upsert_override(&pool, &over("cpu", a, Some(95.0), None)).await.unwrap();
    store::upsert_override(&pool, &over("cpu", b, None, Some(false))).await.unwrap();
    store::upsert_override(&pool, &over("disk", a, Some(80.0), Some(true))).await.unwrap();

    assert_eq!(store::list_overrides(&pool, None, None).await.unwrap().len(), 3);
    let cpu = store::list_overrides(&pool, Some("cpu"), None).await.unwrap();
    assert_eq!(cpu.len(), 2);
    assert_eq!(cpu[0].threshold, Some(95.0));
    assert_eq!(cpu[1].enabled, Some(false));
    assert_eq!(store::list_overrides(&pool, None, Some(a)).await.unwrap().len(), 2);
    assert_eq!(store::list_overrides(&pool, Some("disk"), Some(b)).await.unwrap().len(), 0);

    // Remplacement sur la même clé.
    store::upsert_override(&pool, &over("cpu", a, Some(99.0), None)).await.unwrap();
    let cpu = store::list_overrides(&pool, Some("cpu"), Some(a)).await.unwrap();
    assert_eq!(cpu.len(), 1);
    assert_eq!(cpu[0].threshold, Some(99.0));

    // Une surcharge vide est effacée plutôt que stockée.
    store::upsert_override(&pool, &over("cpu", a, None, None)).await.unwrap();
    assert!(store::list_overrides(&pool, Some("cpu"), Some(a)).await.unwrap().is_empty());
    assert!(!store::delete_override(&pool, "cpu", a).await.unwrap());
    assert!(store::delete_override(&pool, "cpu", b).await.unwrap());
}

#[tokio::test]
async fn un_plan_s_applique_puis_se_recharge_comme_un_registre() {
    let (pool, _dir) = pool().await;
    let chan = channel(&pool, "Ops").await;
    let now = Utc::now();

    let plan = Plan {
        enqueue: vec![queued(chan, "fp-1", now - TimeDelta::minutes(5)), {
            let mut quiet = queued(chan, "fp-2", now - TimeDelta::minutes(3));
            quiet.hold = Hold::Quiet;
            quiet.resolved_meanwhile = true;
            quiet
        }],
        flap_holds: vec![
            ("fp-flap".to_string(), now + TimeDelta::minutes(30)),
            ("fp-old".to_string(), now - TimeDelta::minutes(1)),
        ],
        intake: vec![LogEntry {
            channel_id: 0,
            fingerprint: "fp-1".to_string(),
            reason: NotifyReason::Firing,
            at: now - TimeDelta::minutes(1),
        }],
        ..Default::default()
    };
    store::apply_plan(&pool, &plan).await.unwrap();

    let ledger = store::load_ledger(&pool, now, TimeDelta::hours(2)).await.unwrap();
    assert_eq!(ledger.queue.len(), 2);
    assert_eq!(ledger.queue[0].item.fingerprint, "fp-1");
    assert_eq!(ledger.queue[0].hold, Hold::Batch);
    assert_eq!(ledger.queue[1].hold, Hold::Quiet);
    assert!(ledger.queue[1].resolved_meanwhile);
    assert!(ledger.queue[0].id.is_some());
    // Seule une retenue encore valable est rechargée.
    assert_eq!(ledger.flap_holds.keys().collect::<Vec<_>>(), ["fp-flap"]);
    assert_eq!(ledger.log.len(), 1);
    assert_eq!(ledger.log[0].reason, NotifyReason::Firing);

    // Remettre la même empreinte en file la remplace au lieu de la doubler.
    let mut again = queued(chan, "fp-1", now);
    again.item.value = Some(99.0);
    store::apply_plan(&pool, &Plan { enqueue: vec![again], ..Default::default() }).await.unwrap();
    let ledger = store::load_ledger(&pool, now, TimeDelta::hours(2)).await.unwrap();
    assert_eq!(ledger.queue.len(), 2);
    assert_eq!(
        ledger.queue.iter().find(|q| q.item.fingerprint == "fp-1").unwrap().item.value,
        Some(99.0)
    );

    // Une ligne d'un format inconnu est abandonnée sans bloquer la file.
    sqlx::query(
        "INSERT INTO notify_queue (channel_id, fingerprint, target_name, hold, item, queued_at) \
         VALUES (?, 'fp-ancien', 'nas', 'batch', '{\"ancien\":true}', ?)",
    )
    .bind(chan)
    .bind(now.to_rfc3339())
    .execute(&pool)
    .await
    .unwrap();
    let ledger = store::load_ledger(&pool, now, TimeDelta::hours(2)).await.unwrap();
    assert_eq!(ledger.queue.len(), 2);

    // Dequeue : ce qui part sans envoi quitte la file.
    let ids: Vec<i64> = ledger.queue.iter().filter_map(|q| q.id).collect();
    store::apply_plan(&pool, &Plan { dequeue: ids, ..Default::default() }).await.unwrap();
    let ledger = store::load_ledger(&pool, now, TimeDelta::hours(2)).await.unwrap();
    assert!(ledger.queue.is_empty());
}

#[tokio::test]
async fn un_envoi_reussi_vide_la_file_et_se_consigne_un_echec_la_remet() {
    let (pool, _dir) = pool().await;
    let chan = channel(&pool, "Ops").await;
    let now = Utc::now();

    store::apply_plan(
        &pool,
        &Plan {
            enqueue: vec![queued(chan, "fp-a", now - TimeDelta::minutes(10))],
            ..Default::default()
        },
    )
    .await
    .unwrap();
    let ledger = store::load_ledger(&pool, now, TimeDelta::hours(1)).await.unwrap();
    let ids: Vec<i64> = ledger.queue.iter().filter_map(|q| q.id).collect();

    store::record_sent(
        &pool,
        chan,
        &ids,
        &[("fp-a".to_string(), NotifyReason::Firing), ("fp-b".to_string(), NotifyReason::Reminder)],
        now,
    )
    .await
    .unwrap();
    let ledger = store::load_ledger(&pool, now, TimeDelta::hours(1)).await.unwrap();
    assert!(ledger.queue.is_empty());
    assert_eq!(ledger.messages.len(), 1);
    assert_eq!(ledger.messages[0].0, chan);
    assert_eq!(ledger.log.len(), 2);

    // Envoi raté : les lignes reviennent, datées assez loin pour repartir tout de suite.
    let window = TimeDelta::minutes(2);
    store::requeue(&pool, &[queued(chan, "fp-a", now)], window).await.unwrap();
    let ledger = store::load_ledger(&pool, now, TimeDelta::hours(1)).await.unwrap();
    assert_eq!(ledger.queue.len(), 1);
    assert!(ledger.queue[0].queued_at <= now - window + TimeDelta::seconds(5));
}

#[tokio::test]
async fn l_entretien_purge_le_registre_perime_et_les_retenues_echues() {
    let (pool, _dir) = pool().await;
    let chan = channel(&pool, "Ops").await;
    let now = Utc::now();
    let old = now - TimeDelta::hours(5);

    store::record_sent(&pool, chan, &[], &[("vieux".to_string(), NotifyReason::Firing)], old)
        .await
        .unwrap();
    store::record_sent(&pool, chan, &[], &[("neuf".to_string(), NotifyReason::Firing)], now)
        .await
        .unwrap();
    store::apply_plan(
        &pool,
        &Plan {
            flap_holds: vec![("echue".to_string(), now - TimeDelta::minutes(5))],
            ..Default::default()
        },
    )
    .await
    .unwrap();

    // Un journal, un message et une retenue périmés.
    let removed = store::purge(&pool, now, TimeDelta::hours(1)).await.unwrap();
    assert_eq!(removed, 3);
    assert_eq!(store::purge(&pool, now, TimeDelta::hours(1)).await.unwrap(), 0);
    let ledger = store::load_ledger(&pool, now, TimeDelta::hours(6)).await.unwrap();
    assert_eq!(ledger.log.len(), 1);
    assert_eq!(ledger.messages.len(), 1);
}

#[test]
fn la_fenetre_du_registre_est_la_plus_longue_des_fenetres_utiles() {
    let global = GlobalPolicy::default();
    let none = HashMap::new();
    // Par défaut : la fenêtre de battement (30 min) est plus courte que l'heure.
    assert_eq!(store::lookback(&global, &none), TimeDelta::seconds(3600));

    let slow = ChannelPolicy { min_interval_secs: 7200, ..Default::default() };
    let channels = HashMap::from([(1, slow)]);
    assert_eq!(store::lookback(&global, &channels), TimeDelta::seconds(7200));

    // Le délai d'escalade garde une marge d'une heure de plus.
    let escalating = GlobalPolicy { escalate_after_secs: 4 * 3600, ..Default::default() };
    assert_eq!(store::lookback(&escalating, &none), TimeDelta::seconds(5 * 3600));
}
