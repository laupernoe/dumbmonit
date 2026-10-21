//! Les réglages SEO des pages de statut, les dossiers cochés et les noms
//! publics de leurs membres voyagent avec la sauvegarde ; une ancienne
//! sauvegarde (sans ces champs) se restaure toujours.

use dumbmonit_server::backup::{Bundle, ExportOptions, RestoreReport, collect, restore};
use dumbmonit_server::crypto::Cipher;
use dumbmonit_server::db;
use serde_json::json;
use sqlx::SqlitePool;

async fn instance(dir: &tempfile::TempDir) -> (SqlitePool, Cipher) {
    let pool = db::open(&dir.path().join("dumbmonit.db")).await.expect("base");
    let cipher = db::init_cipher(&pool, "secret-de-test-suffisamment-long").await.expect("clé");
    (pool, cipher)
}

fn counts(report: &RestoreReport, name: &str) -> (usize, usize, usize) {
    let section = report.sections.iter().find(|s| s.section == name).expect("section");
    (section.created, section.updated, section.skipped)
}

#[tokio::test]
async fn seo_groupes_et_noms_publics_sont_sauvegardes_et_restaures() {
    let source_dir = tempfile::tempdir().unwrap();
    let (source, cipher) = instance(&source_dir).await;
    sqlx::query(
        "INSERT INTO targets (name, address, kind, interval_secs, enabled, tags, group_name)
         VALUES ('nas', '10.0.0.2', 'dummy', 60, 1, '{}', 'Rack')",
    )
    .execute(&source)
    .await
    .unwrap();
    let seo = json!({ "index": true, "title": "Statut Acme", "description": "Etat des services",
        "keywords": "acme,statut", "canonical": "https://status.acme.test/",
        "image": "", "lang": "fr", "site_name": "Acme" });
    sqlx::query(
        "INSERT INTO status_pages (slug, title, published, seo) VALUES ('acme', 'Acme', 1, ?)",
    )
    .bind(seo.to_string())
    .execute(&source)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO status_page_groups (page_id, group_name, position)
         VALUES (1, 'Rack', 1), (1, 'Bureau', 0)",
    )
    .execute(&source)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO status_page_group_labels (page_id, target_id, label) VALUES (1, 1, 'Stockage')",
    )
    .execute(&source)
    .await
    .unwrap();

    let lot = collect(&source, &cipher, ExportOptions::default()).await.expect("export");
    assert_eq!(lot.status_pages[0].groups, vec!["Bureau", "Rack"]);
    assert_eq!(lot.status_pages[0].group_labels.len(), 1);
    // Passage par le JSON, comme un vrai fichier.
    let lot: Bundle = serde_json::from_str(&serde_json::to_string(&lot).unwrap()).unwrap();

    let target_dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance(&target_dir).await;
    let report = restore(&pool, &cipher, &lot, true).await.expect("restauration");
    assert_eq!(counts(&report, "status_pages"), (1, 0, 0), "{report:#?}");
    let stored: String = sqlx::query_scalar("SELECT seo FROM status_pages WHERE slug = 'acme'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let stored: serde_json::Value = serde_json::from_str(&stored).unwrap();
    assert_eq!(stored, seo);
    let groups: Vec<String> =
        sqlx::query_scalar("SELECT group_name FROM status_page_groups ORDER BY position")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(groups, vec!["Bureau", "Rack"]);
    let label: String = sqlx::query_scalar(
        "SELECT l.label FROM status_page_group_labels l JOIN targets t ON t.id = l.target_id
         WHERE t.address = '10.0.0.2'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(label, "Stockage");

    let again = restore(&pool, &cipher, &lot, true).await.expect("seconde restauration");
    assert_eq!(counts(&again, "status_pages"), (0, 0, 1), "rien ne bouge : {again:#?}");
}

#[tokio::test]
async fn une_ancienne_sauvegarde_sans_ces_champs_se_restaure() {
    let old: Bundle = serde_json::from_value(json!({
        "version": 1,
        "created_at": "2026-09-01 10:00:00",
        "source_version": "0.13.0",
        "status_pages": [
            { "slug": "vieille", "title": "Vieille", "published": true, "theme": "auto",
              "show_uptime_days": 30 }
        ]
    }))
    .expect("ancien lot lisible");

    let dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance(&dir).await;
    let report = restore(&pool, &cipher, &old, true).await.expect("restauration");
    assert_eq!(counts(&report, "status_pages"), (1, 0, 0), "{report:#?}");
    let stored: String = sqlx::query_scalar("SELECT seo FROM status_pages WHERE slug = 'vieille'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(stored, "{}", "valeurs par défaut");
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM status_page_groups")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 0);

    // Une page qui a déjà ses réglages les garde si le fichier n'en dit rien.
    sqlx::query("UPDATE status_pages SET seo = '{\"index\":true,\"title\":\"Garde\"}'")
        .execute(&pool)
        .await
        .unwrap();
    let report = restore(&pool, &cipher, &old, true).await.expect("seconde restauration");
    assert_eq!(counts(&report, "status_pages"), (0, 0, 1), "{report:#?}");
    let kept: String = sqlx::query_scalar("SELECT seo FROM status_pages WHERE slug = 'vieille'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(kept.contains("Garde"));
}
