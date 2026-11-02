//! Les changements de configuration fermés voyagent avec la sauvegarde : une
//! instance restaurée ne rappelle pas ce que l'administrateur a déjà traité, et
//! relire le même lot ne change rien (la date de fermeture d'origine reste).

use dumbmonit_server::backup::{Bundle, ExportOptions, RestoreReport, collect, restore};
use dumbmonit_server::db;
use dumbmonit_server::upgrade_notes::{self, CATALOG};
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
async fn les_changements_fermes_sont_sauvegardes_et_restaures() {
    let source_dir = tempfile::tempdir().unwrap();
    let (source, cipher) = instance(&source_dir).await;
    let note = CATALOG.iter().find(|n| n.id == "agent-channel-port").expect("la note du port");
    assert!(upgrade_notes::close_note(&source, note, "admin").await.unwrap());
    sqlx::query("UPDATE upgrade_notes_closed SET closed_at = '2026-10-31 08:30:00'")
        .execute(&source)
        .await
        .unwrap();

    let lot = collect(&source, &cipher, ExportOptions::default()).await.expect("export");
    assert_eq!(lot.upgrade_notes_closed.len(), 1);
    assert_eq!(lot.upgrade_notes_closed[0].note_id, "agent-channel-port");
    assert_eq!(lot.upgrade_notes_closed[0].version, note.version);
    assert_eq!(lot.summary().get("upgrade_notes_closed"), Some(&1));

    // Une simulation ne change rien.
    let target_dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance(&target_dir).await;
    let dry = restore(&pool, &cipher, &lot, false).await.expect("simulation");
    assert_eq!(counts(&dry, "upgrade_notes_closed"), (1, 0, 0), "la simulation compte la note");
    assert!(upgrade_notes::closed_rows(&pool).await.unwrap().is_empty());

    let report = restore(&pool, &cipher, &lot, true).await.expect("restauration");
    assert_eq!(counts(&report, "upgrade_notes_closed"), (1, 0, 0), "la note est restaurée");
    let rows = upgrade_notes::closed_rows(&pool).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].note_id, "agent-channel-port");
    assert_eq!(rows[0].closed_by, "admin");
    assert_eq!(rows[0].closed_at, "2026-10-31 08:30:00", "la date d'origine est gardée");

    let again = restore(&pool, &cipher, &lot, true).await.expect("seconde restauration");
    assert_eq!(
        counts(&again, "upgrade_notes_closed"),
        (0, 0, 1),
        "la seconde fois, la note existe déjà"
    );
    assert_eq!(upgrade_notes::closed_rows(&pool).await.unwrap().len(), 1);
}

#[tokio::test]
async fn un_lot_ancien_sans_cette_section_se_restaure_sans_rien_fermer() {
    let lot: Bundle =
        serde_json::from_str(r#"{ "version": 1, "created_at": "2026-10-01T00:00:00Z" }"#)
            .expect("un lot d'avant cette section");
    assert!(lot.upgrade_notes_closed.is_empty());

    let dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance(&dir).await;
    let report = restore(&pool, &cipher, &lot, true).await.expect("restauration");
    assert_eq!(counts(&report, "upgrade_notes_closed"), (0, 0, 0), "rien à restaurer");
    assert!(upgrade_notes::closed_rows(&pool).await.unwrap().is_empty());
}
