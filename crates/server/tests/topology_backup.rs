//! Les corrections et les règles de la carte voyagent avec la sauvegarde :
//! désignées par type + adresse, elles retrouvent leurs équipements sur une
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
async fn les_corrections_et_les_regles_de_la_carte_sont_restaurees() {
    let source_dir = tempfile::tempdir().unwrap();
    let (source, cipher) = instance(&source_dir).await;
    // Des identifiants qui ne seront pas ceux de l'instance restaurée.
    sqlx::query(
        "INSERT INTO targets (id, name, address, kind, interval_secs, enabled, tags)
         VALUES (40, 'sw', '10.0.5.2', 'dummy', 60, 1, '{}'),
                (41, 'nas', '10.0.5.10', 'dummy', 60, 1, '{}'),
                (42, 'pve', '10.0.5.20', 'dummy', 60, 1, '{}')",
    )
    .execute(&source)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO topology_corrections (kind, from_node, to_node, from_target, to_target,
             source, value, use_for_alerts, author)
         VALUES ('parent', 't40', 't41', 40, 41, NULL, NULL, 0, 'admin'),
                ('site', NULL, 'g42:web', NULL, 42, NULL, 'Annexe', 0, 'admin')",
    )
    .execute(&source)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO topology_rules (key, kind, status, subnet, parent_node, parent_target,
             evidence, author)
         VALUES ('parent|10.0.5.0/24||||t40', 'parent', 'active', '10.0.5.0/24', 't40', 40, 2,
                 'admin')",
    )
    .execute(&source)
    .await
    .unwrap();

    let lot = collect(&source, &cipher, ExportOptions::default()).await.expect("export");
    assert_eq!(lot.topology_corrections.len(), 2);
    assert_eq!(lot.topology_rules.len(), 1);
    assert_eq!(lot.topology_corrections[1].to.child.as_deref(), Some("g:web"));

    let target_dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance(&target_dir).await;
    let report = restore(&pool, &cipher, &lot, true).await.expect("restauration");
    assert_eq!(counts(&report, "topology_corrections"), (2, 0, 0), "{report:#?}");
    assert_eq!(counts(&report, "topology_rules"), (1, 0, 0), "{report:#?}");

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
    let (sw, nas, pve) = (id("10.0.5.2").await, id("10.0.5.10").await, id("10.0.5.20").await);
    let rows: Vec<(String, Option<String>, String)> =
        sqlx::query_as("SELECT kind, from_node, to_node FROM topology_corrections ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        rows,
        vec![
            ("parent".to_string(), Some(format!("t{sw}")), format!("t{nas}")),
            ("site".to_string(), None, format!("g{pve}:web")),
        ]
    );
    let (key, parent): (String, String) =
        sqlx::query_as("SELECT key, parent_node FROM topology_rules")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(parent, format!("t{sw}"));
    assert_eq!(key, format!("parent|10.0.5.0/24||||t{sw}"), "la clé suit le nouvel identifiant");

    let again = restore(&pool, &cipher, &lot, true).await.expect("seconde restauration");
    assert_eq!(counts(&again, "topology_corrections"), (0, 0, 2), "{again:#?}");
    assert_eq!(counts(&again, "topology_rules"), (0, 0, 1), "{again:#?}");

    // Supprimer l'équipement emporte ses corrections.
    sqlx::query("DELETE FROM targets WHERE id = ?").bind(nas).execute(&pool).await.unwrap();
    let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM topology_corrections")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(left, 1);
}
