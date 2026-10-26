//! Les tableaux de bord voyagent avec la sauvegarde : partagés et personnels
//! (par nom de compte), restaurés sur une instance vierge ; relire le même lot
//! ne change rien, et un tableau personnel sans son compte est ignoré.

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

async fn user(pool: &SqlitePool, name: &str) {
    sqlx::query("INSERT INTO users (username, role) VALUES (?, 'viewer')")
        .bind(name)
        .execute(pool)
        .await
        .unwrap();
}

const GRID: &str = r#"[{"id":"w1","type":"stat","title":"CPU","x":0,"y":0,"w":3,"h":3,
    "config":{"query":"avg(dumbmonit_cpu_usage_percent)","unit":"%","warn":80,"crit":95,"direction":"above"}}]"#;

#[tokio::test]
async fn les_tableaux_de_bord_sont_restaures() {
    let source_dir = tempfile::tempdir().unwrap();
    let (source, cipher) = instance(&source_dir).await;
    user(&source, "alice").await;
    sqlx::query(
        "INSERT INTO dashboards (name, shared, owner_id, range_secs, refresh_secs, widgets)
         VALUES ('Parc', 1, NULL, 21600, 60, ?),
                ('Perso', 0, (SELECT id FROM users WHERE username = 'alice'), 3600, 0, '[]')",
    )
    .bind(GRID)
    .execute(&source)
    .await
    .unwrap();

    let lot = collect(&source, &cipher, ExportOptions::default()).await.expect("export");
    assert_eq!(lot.dashboards.len(), 2);

    // Sans le compte, le tableau personnel est ignoré, le partagé passe.
    let bare_dir = tempfile::tempdir().unwrap();
    let (bare, cipher) = instance(&bare_dir).await;
    let mut sans_comptes = lot.clone();
    sans_comptes.users.clear();
    let report = restore(&bare, &cipher, &sans_comptes, true).await.expect("restauration");
    assert_eq!(counts(&report, "dashboards"), (1, 0, 1), "{report:#?}");

    // Avec le compte, les deux arrivent ; une seconde lecture ne change rien.
    let target_dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance(&target_dir).await;
    user(&pool, "alice").await;
    let report = restore(&pool, &cipher, &lot, true).await.expect("restauration");
    assert_eq!(counts(&report, "dashboards"), (2, 0, 0), "{report:#?}");
    let (shared, range): (i64, i64) =
        sqlx::query_as("SELECT shared, range_secs FROM dashboards WHERE name = 'Parc'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((shared, range), (1, 21600));
    let owner: String = sqlx::query_scalar(
        "SELECT u.username FROM dashboards d JOIN users u ON u.id = d.owner_id WHERE d.name = 'Perso'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(owner, "alice");

    let again = restore(&pool, &cipher, &lot, true).await.expect("seconde restauration");
    assert_eq!(counts(&again, "dashboards"), (0, 0, 2), "rien ne bouge : {again:#?}");
}

#[tokio::test]
async fn supprimer_un_compte_emporte_ses_tableaux_personnels_seulement() {
    let dir = tempfile::tempdir().unwrap();
    let (pool, _cipher) = instance(&dir).await;
    user(&pool, "bob").await;
    sqlx::query(
        "INSERT INTO dashboards (name, shared, owner_id) VALUES
            ('Commun', 1, (SELECT id FROM users WHERE username = 'bob')),
            ('Privé', 0, (SELECT id FROM users WHERE username = 'bob'))",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("DELETE FROM users WHERE username = 'bob'").execute(&pool).await.unwrap();
    let left: Vec<String> =
        sqlx::query_scalar("SELECT name FROM dashboards").fetch_all(&pool).await.unwrap();
    assert_eq!(left, vec!["Commun".to_string()]);
}
