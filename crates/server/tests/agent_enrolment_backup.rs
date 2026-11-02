//! La garde de l'enrôlement voyage avec la sauvegarde : une instance restaurée
//! garde la même politique de validation, le même débit et le même délai de
//! mise en veille. Un lot d'avant la garde ne fait pas attendre tout un parc.

use dumbmonit_server::backup::{Bundle, ExportOptions, RestoreReport, collect, restore};
use dumbmonit_server::collectors::agent::enrolment::{
    self, ApprovalMode, EnrolmentSettings, SETTINGS_KEY,
};
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

async fn stored(pool: &SqlitePool) -> bool {
    sqlx::query("SELECT 1 FROM settings WHERE key = ?")
        .bind(SETTINGS_KEY)
        .fetch_optional(pool)
        .await
        .expect("lecture")
        .is_some()
}

#[tokio::test]
async fn les_reglages_de_la_garde_sont_sauvegardes_et_restaures() {
    let source_dir = tempfile::tempdir().unwrap();
    let (source, cipher) = instance(&source_dir).await;
    let wanted = EnrolmentSettings {
        fleet_approval: ApprovalMode::AdDomain,
        single_use_approval: ApprovalMode::Manual,
        rate_per_hour: 12,
        dormant_days: 0,
    };
    enrolment::save_settings(&source, &wanted).await.unwrap();

    let lot = collect(&source, &cipher, ExportOptions::default()).await.expect("export");
    assert!(lot.agent_enrolment.is_some(), "la section est toujours écrite");

    let target_dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance(&target_dir).await;
    let dry = restore(&pool, &cipher, &lot, false).await.expect("simulation");
    assert_eq!(counts(&dry, "agent_enrolment"), (1, 0, 0));
    assert!(!stored(&pool).await, "une simulation n'écrit rien");

    let report = restore(&pool, &cipher, &lot, true).await.expect("restauration");
    assert_eq!(counts(&report, "agent_enrolment"), (1, 0, 0));
    assert_eq!(enrolment::load_settings(&pool).await.unwrap(), wanted);

    let again = restore(&pool, &cipher, &lot, true).await.expect("seconde restauration");
    assert_eq!(counts(&again, "agent_enrolment"), (0, 0, 1), "rien ne change");
}

#[tokio::test]
async fn une_installation_neuve_exporte_ses_valeurs_par_defaut() {
    let dir = tempfile::tempdir().unwrap();
    let (source, cipher) = instance(&dir).await;
    let lot = collect(&source, &cipher, ExportOptions::default()).await.expect("export");
    let exported: EnrolmentSettings =
        serde_json::from_value(lot.agent_enrolment.expect("section")).unwrap();
    assert_eq!(exported, EnrolmentSettings::default());
    assert_eq!(exported.fleet_approval, ApprovalMode::Manual);
}

#[tokio::test]
async fn un_lot_d_avant_la_garde_garde_l_acceptation_automatique_de_son_parc() {
    // Des jetons, pas de section : l'instance d'origine acceptait d'office.
    let lot: Bundle = serde_json::from_value(serde_json::json!({
        "version": 1,
        "created_at": "2026-10-01T00:00:00Z",
        "agent_tokens": [{ "name": "parc", "token_hash": "abc123", "prefix": "dmon_abc" }]
    }))
    .expect("un lot d'avant cette section");
    assert!(lot.agent_enrolment.is_none());

    let dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance(&dir).await;
    let report = restore(&pool, &cipher, &lot, true).await.expect("restauration");
    assert_eq!(counts(&report, "agent_enrolment"), (1, 0, 0));
    assert_eq!(enrolment::load_settings(&pool).await.unwrap(), EnrolmentSettings::legacy());

    // Sans jeton, rien à préserver : l'instance garde ses valeurs.
    let empty: Bundle =
        serde_json::from_str(r#"{ "version": 1, "created_at": "2026-10-01T00:00:00Z" }"#).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance(&dir).await;
    restore(&pool, &cipher, &empty, true).await.expect("restauration");
    assert!(!stored(&pool).await);
}

#[tokio::test]
async fn un_reglage_illisible_est_signale_et_ecarte() {
    let lot: Bundle = serde_json::from_value(serde_json::json!({
        "version": 1,
        "created_at": "2026-10-01T00:00:00Z",
        "agent_enrolment": { "fleet_approval": "maybe" }
    }))
    .expect("lot");
    let dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance(&dir).await;
    let report = restore(&pool, &cipher, &lot, true).await.expect("restauration");
    assert_eq!(counts(&report, "agent_enrolment"), (0, 0, 1));
    assert!(!stored(&pool).await);
}
