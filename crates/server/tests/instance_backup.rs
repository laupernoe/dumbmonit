//! Les réglages d'instance voyagent avec la sauvegarde : mode produit, nom,
//! langue et fuseau sont restaurés sur une instance vierge, relire le même lot
//! ne change rien, et un assistant déjà terminé ici ne revient pas.

use dumbmonit_server::backup::{ExportOptions, RestoreReport, collect, restore};
use dumbmonit_server::db;
use dumbmonit_server::instance::{self, InstanceSettings, ProductMode};
use sqlx::SqlitePool;

async fn instance_db(dir: &tempfile::TempDir) -> (SqlitePool, dumbmonit_server::crypto::Cipher) {
    let pool = db::open(&dir.path().join("dumbmonit.db")).await.expect("base");
    let cipher = db::init_cipher(&pool, "secret-de-test-suffisamment-long").await.expect("clé");
    (pool, cipher)
}

fn counts(report: &RestoreReport, name: &str) -> (usize, usize, usize) {
    let section = report.sections.iter().find(|s| s.section == name).expect("section");
    (section.created, section.updated, section.skipped)
}

#[tokio::test]
async fn les_reglages_d_instance_sont_restaures() {
    let source_dir = tempfile::tempdir().unwrap();
    let (source, cipher) = instance_db(&source_dir).await;
    let chosen = InstanceSettings {
        product_mode: ProductMode::Enterprise,
        setup_completed: true,
        setup_completed_at: Some("2026-10-27 09:00:00".into()),
        name: "Acme NOC".into(),
        language: "fr".into(),
        timezone: "Europe/Zurich".into(),
    };
    instance::save(&source, &chosen).await.unwrap();

    let lot = collect(&source, &cipher, ExportOptions::default()).await.expect("export");
    assert_eq!(lot.instance.as_ref(), Some(&chosen));

    let target_dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance_db(&target_dir).await;
    // Une base neuve n'a encore rien écrit : l'assistant l'attend.
    assert!(!instance::load(&pool).await.setup_completed);

    let report = restore(&pool, &cipher, &lot, true).await.expect("restauration");
    assert_eq!(counts(&report, "instance"), (1, 0, 0), "{report:#?}");
    assert_eq!(instance::load(&pool).await, chosen);

    let again = restore(&pool, &cipher, &lot, true).await.expect("seconde restauration");
    assert_eq!(counts(&again, "instance"), (0, 0, 1), "rien ne bouge : {again:#?}");
}

#[tokio::test]
async fn un_assistant_termine_ici_ne_revient_pas_et_un_lot_ancien_ne_touche_rien() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance_db(&dir).await;
    let here = InstanceSettings {
        setup_completed: true,
        setup_completed_at: Some("2026-10-01 08:00:00".into()),
        ..InstanceSettings::default()
    };
    instance::save(&pool, &here).await.unwrap();

    // Lot d'une instance dont l'assistant n'était pas terminé.
    let mut lot = collect(&pool, &cipher, ExportOptions::default()).await.expect("export");
    lot.instance = Some(InstanceSettings {
        product_mode: ProductMode::Enterprise,
        ..InstanceSettings::default()
    });
    let report = restore(&pool, &cipher, &lot, true).await.expect("restauration");
    assert_eq!(counts(&report, "instance"), (0, 1, 0), "{report:#?}");
    let now = instance::load(&pool).await;
    assert_eq!(now.product_mode, ProductMode::Enterprise);
    assert!(now.setup_completed);
    assert_eq!(now.setup_completed_at, here.setup_completed_at);

    // Un lot écrit avant ce réglage n'a pas de section : rien n'est touché.
    lot.instance = None;
    let report = restore(&pool, &cipher, &lot, true).await.expect("lot ancien");
    assert_eq!(counts(&report, "instance"), (0, 0, 0));
    assert_eq!(instance::load(&pool).await.product_mode, ProductMode::Enterprise);
}
