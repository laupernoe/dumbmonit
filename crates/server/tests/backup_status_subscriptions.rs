//! Les canaux d'abonnement d'une page de statut (SMS, webhook, Slack) voyagent
//! avec la sauvegarde ; les abonnés, données personnelles, restent chez eux.

use dumbmonit_server::backup::{Bundle, ExportOptions, collect, restore};
use dumbmonit_server::crypto::Cipher;
use dumbmonit_server::db;
use sqlx::SqlitePool;

async fn instance(dir: &tempfile::TempDir) -> (SqlitePool, Cipher) {
    let pool = db::open(&dir.path().join("dumbmonit.db")).await.expect("base");
    let cipher = db::init_cipher(&pool, "secret-de-test-suffisamment-long").await.expect("clé");
    (pool, cipher)
}

async fn texto(pool: &SqlitePool) {
    sqlx::query(
        "INSERT INTO notification_channels (name, kind, enabled, settings, secret_enc)
         VALUES ('Texto', 'vonage', 1, '{\"api_key\":\"k\",\"from\":\"D\",\"to\":[\"+33600000000\"]}', NULL)",
    )
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn subscription_channels_are_backed_up_and_restored() {
    let source_dir = tempfile::tempdir().unwrap();
    let (source, cipher) = instance(&source_dir).await;
    texto(&source).await;
    sqlx::query(
        "INSERT INTO status_pages (slug, title, published, subscribe_sms_channel_id,
             subscribe_webhook, subscribe_slack)
         VALUES ('acme', 'Acme', 1, (SELECT id FROM notification_channels), 1, 1)",
    )
    .execute(&source)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO status_subscribers (page_id, email, token, kind, confirmed_at)
         VALUES (1, '+33612345678', 'tok', 'sms', '2026-01-01T00:00:00Z')",
    )
    .execute(&source)
    .await
    .unwrap();

    let lot = collect(&source, &cipher, ExportOptions::default()).await.expect("export");
    let subscriptions = lot.status_pages[0].subscriptions.clone().expect("subscriptions");
    assert_eq!(subscriptions.sms_channel.as_deref(), Some("Texto"));
    assert!(subscriptions.webhook && subscriptions.slack);
    let json = serde_json::to_string(&lot).unwrap();
    assert!(!json.contains("+33612345678"), "subscribers never travel");
    let lot: Bundle = serde_json::from_str(&json).unwrap();

    let target_dir = tempfile::tempdir().unwrap();
    let (pool, cipher) = instance(&target_dir).await;
    restore(&pool, &cipher, &lot, true).await.expect("restauration");
    let (sms, webhook, slack, name): (Option<i64>, i64, i64, Option<String>) = sqlx::query_as(
        "SELECT p.subscribe_sms_channel_id, p.subscribe_webhook, p.subscribe_slack, c.name
         FROM status_pages p LEFT JOIN notification_channels c ON c.id = p.subscribe_sms_channel_id
         WHERE p.slug = 'acme'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(sms.is_some());
    assert_eq!((webhook, slack, name.as_deref()), (1, 1, Some("Texto")));
    let subscribers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM status_subscribers")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(subscribers, 0);
}
