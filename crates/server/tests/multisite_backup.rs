//! Le réglage multi-emplacements voyage avec la sauvegarde : serveur, agents
//! relais (par référence) et quorum, restaurés sur une instance vierge, et
//! relire le même lot ne change rien.

use dumbmonit_server::backup::{ExportOptions, RestoreReport, collect, restore};
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

#[tokio::test]
async fn le_reglage_multi_emplacements_est_restaure() {
    let source_dir = tempfile::tempdir().unwrap();
    let (source, cipher) = instance(&source_dir).await;
    sqlx::query(
        "INSERT INTO targets (id, name, address, kind, interval_secs, enabled, tags) VALUES
         (1, 'site', 'https://example.org', 'http', 60, 1, '{}'),
         (2, 'relais-a', 'relais-a', 'agent', 60, 1, '{}'),
         (3, 'relais-b', 'relais-b', 'agent', 60, 1, '{}')",
    )
    .execute(&source)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO target_multisite (target_id, include_server, quorum) VALUES (1, 0, 2)",
    )
    .execute(&source)
    .await
    .unwrap();
    sqlx::query("INSERT INTO target_multisite_agents (target_id, agent_id) VALUES (1, 2), (1, 3)")
        .execute(&source)
        .await
        .unwrap();

    let lot = collect(&source, &cipher, ExportOptions::default()).await.expect("export");
    let site = lot.targets.iter().find(|t| t.name == "site").expect("site");
    let multisite = site.multisite.as_ref().expect("réglage exporté");
    assert!(!multisite.server);
    assert_eq!(multisite.agents.len(), 2);
    assert_eq!(multisite.quorum, Some(2));
    assert!(lot.targets.iter().filter(|t| t.name != "site").all(|t| t.multisite.is_none()));

    let target_dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance(&target_dir).await;
    let report = restore(&pool, &cipher, &lot, true).await.expect("restauration");
    let id: i64 = sqlx::query_scalar("SELECT id FROM targets WHERE name = 'site'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let config = db::multisite::get(&pool, id).await.unwrap().expect("réglage restauré");
    assert!(!config.server, "{report:#?}");
    assert_eq!(config.quorum, Some(2));
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT t.name FROM target_multisite_agents m JOIN targets t ON t.id = m.agent_id
         WHERE m.target_id = ? ORDER BY t.name",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(names, ["relais-a", "relais-b"]);

    let again = restore(&pool, &cipher, &lot, true).await.expect("seconde restauration");
    assert_eq!(counts(&again, "targets"), (0, 0, 3), "rien ne bouge : {again:#?}");
}
