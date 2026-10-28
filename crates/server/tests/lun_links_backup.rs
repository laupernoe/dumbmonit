//! Les liens manuels LUN → équipement consommateur voyagent avec la sauvegarde :
//! désignés par type + adresse, ils retrouvent leurs équipements sur une
//! instance vierge, et relire le même lot ne change rien.

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
async fn les_liens_de_lun_sont_sauvegardes_et_restaures() {
    let source_dir = tempfile::tempdir().unwrap();
    let (source, cipher) = instance(&source_dir).await;
    sqlx::query(
        "INSERT INTO targets (id, name, address, kind, interval_secs, enabled, tags)
         VALUES (50, 'nas', '10.0.6.10', 'dummy', 60, 1, '{}'),
                (51, 'esxi', '10.0.6.20', 'dummy', 60, 1, '{}')",
    )
    .execute(&source)
    .await
    .unwrap();
    db::lun_links::upsert(&source, 50, "uuid-1", 51, "vsphere_datastore", "datastore1", "admin")
        .await
        .unwrap();

    let lot = collect(&source, &cipher, ExportOptions::default()).await.expect("export");
    assert_eq!(lot.lun_links.len(), 1);
    assert_eq!(lot.lun_links[0].family, "vsphere_datastore");
    assert_eq!(lot.summary().get("lun_links"), Some(&1));

    let target_dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance(&target_dir).await;
    let report = restore(&pool, &cipher, &lot, true).await.expect("restauration");
    assert_eq!(counts(&report, "lun_links"), (1, 0, 0), "{report:#?}");

    let id = |address: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, i64>("SELECT id FROM targets WHERE address = ?")
                .bind(address)
                .fetch_one(&pool)
                .await
                .unwrap()
        }
    };
    let (nas, esxi) = (id("10.0.6.10").await, id("10.0.6.20").await);
    let links = db::lun_links::list_for(&pool, nas).await.unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].lun, "uuid-1");
    assert_eq!(links[0].consumer_target_id, esxi, "le consommateur suit son nouvel identifiant");
    assert_eq!(links[0].entity, "datastore1");

    let again = restore(&pool, &cipher, &lot, true).await.expect("seconde restauration");
    assert_eq!(counts(&again, "lun_links"), (0, 0, 1), "{again:#?}");

    // Supprimer le consommateur emporte le lien.
    sqlx::query("DELETE FROM targets WHERE id = ?").bind(esxi).execute(&pool).await.unwrap();
    assert!(db::lun_links::list_for(&pool, nas).await.unwrap().is_empty());
}
