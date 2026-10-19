//! Les emplacements voyagent avec la sauvegarde : exportés avec l'affectation
//! de chaque équipement, restaurés sur une instance vierge, et relire le même
//! lot ne change rien.

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
async fn les_emplacements_et_les_affectations_sont_restaures() {
    let source_dir = tempfile::tempdir().unwrap();
    let (source, cipher) = instance(&source_dir).await;
    sqlx::query(
        "INSERT INTO locations (name, address, latitude, longitude, site, main_site)
         VALUES ('Siège', 'Lausanne', 46.52, 6.63, '', 1), ('Annexe', '', 48.85, 2.35, 'annexe', 0)",
    )
    .execute(&source)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO targets (name, address, kind, interval_secs, enabled, tags, location_id)
         VALUES ('nas', '10.0.0.2', 'dummy', 60, 1, '{}', 2)",
    )
    .execute(&source)
    .await
    .unwrap();

    let lot = collect(&source, &cipher, ExportOptions::default()).await.expect("export");
    assert_eq!(lot.locations.len(), 2);
    assert_eq!(lot.targets[0].location.as_deref(), Some("Annexe"));

    let target_dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance(&target_dir).await;
    let report = restore(&pool, &cipher, &lot, true).await.expect("restauration");
    assert_eq!(counts(&report, "locations"), (2, 0, 0), "{report:#?}");
    let placed: Option<String> = sqlx::query_scalar(
        "SELECT l.name FROM targets t JOIN locations l ON l.id = t.location_id
         WHERE t.address = '10.0.0.2'",
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert_eq!(placed.as_deref(), Some("Annexe"));
    let main: String = sqlx::query_scalar("SELECT name FROM locations WHERE main_site = 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(main, "Siège");

    let again = restore(&pool, &cipher, &lot, true).await.expect("seconde restauration");
    assert_eq!(counts(&again, "locations"), (0, 0, 2), "rien ne bouge : {again:#?}");
}
