//! Le mode sentinelle voyage avec la sauvegarde : adresses, seuil et canal de
//! notification (secret compris, dans la charge utile chiffrée) retrouvent leur
//! agent sur une instance vierge, et relire le même lot ne change rien.

use dumbmonit_proto::SentinelChannel;
use dumbmonit_server::backup::{ExportOptions, RestoreReport, collect, restore};
use dumbmonit_server::collectors::agent::sentinel::{self, Input};
use dumbmonit_server::db;
use sqlx::SqlitePool;

async fn instance(dir: &tempfile::TempDir) -> (SqlitePool, dumbmonit_server::crypto::Cipher) {
    let pool = db::open(&dir.path().join("dumbmonit.db")).await.expect("base");
    let cipher = db::init_cipher(&pool, "secret-de-test-suffisamment-long").await.expect("clé");
    (pool, cipher)
}

fn counts(report: &RestoreReport, name: &str) -> (usize, usize, usize) {
    let section = report.sections.iter().find(|s| s.section == name).expect("section");
    (section.created, section.updated, section.skipped)
}

const HOOK: &str = "https://hooks.example.org/secret-path";

#[tokio::test]
async fn la_sentinelle_est_sauvegardee_et_restauree() {
    let source_dir = tempfile::tempdir().unwrap();
    let (source, cipher) = instance(&source_dir).await;
    sqlx::query(
        "INSERT INTO targets (id, name, address, kind, interval_secs, enabled, tags)
         VALUES (60, 'sentinelle', 'nas-cave', 'agent', 60, 1, '{}')",
    )
    .execute(&source)
    .await
    .unwrap();
    let stored = sentinel::merge(
        None,
        Input {
            enabled: true,
            urls: vec!["https://dumbmonit.example.org/api/health".into()],
            pins: std::collections::BTreeMap::from([(
                "https://dumbmonit.example.org/api/health".to_string(),
                "AB".repeat(32),
            )]),
            interval_secs: 30,
            threshold: 4,
            channel: Some(SentinelChannel::Webhook { url: HOOK.into() }),
            clear_channel: false,
        },
    )
    .unwrap();
    sentinel::save(&source, &cipher, 60, &stored).await.unwrap();

    // Le canal est chiffré au repos : l'adresse du webhook n'est pas en clair en base.
    let raw: Vec<u8> =
        sqlx::query_scalar("SELECT channel_enc FROM agent_sentinel WHERE target_id = 60")
            .fetch_one(&source)
            .await
            .unwrap();
    assert!(!String::from_utf8_lossy(&raw).contains("secret-path"));

    let lot = collect(&source, &cipher, ExportOptions::default()).await.expect("export");
    assert_eq!(lot.agent_sentinels.len(), 1);
    assert_eq!(lot.summary().get("agent_sentinels"), Some(&1));

    let target_dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance(&target_dir).await;
    let report = restore(&pool, &cipher, &lot, true).await.expect("restauration");
    assert_eq!(counts(&report, "agent_sentinels"), (1, 0, 0), "{report:#?}");

    let id: i64 = sqlx::query_scalar("SELECT id FROM targets WHERE address = 'nas-cave'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let restored = sentinel::load(&pool, &cipher, id).await.unwrap().expect("réglage restauré");
    assert!(restored.enabled);
    assert_eq!(restored.threshold, 4);
    assert_eq!(restored.interval_secs, 30);
    assert_eq!(restored.urls, vec!["https://dumbmonit.example.org/api/health".to_string()]);
    assert_eq!(restored.pins.values().next(), Some(&"ab".repeat(32)));
    assert_eq!(restored.channel, Some(SentinelChannel::Webhook { url: HOOK.into() }));

    let again = restore(&pool, &cipher, &lot, true).await.expect("seconde restauration");
    assert_eq!(counts(&again, "agent_sentinels"), (0, 0, 1), "{again:#?}");
}
