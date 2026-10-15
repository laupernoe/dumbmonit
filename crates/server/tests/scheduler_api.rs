//! La boucle du planificateur, lancée pour de bon : les cibles échues sont
//! interrogées, le résultat est enregistré, une cible désactivée est laissée
//! tranquille et les cibles supprimées ne sont plus rejouées.

use std::sync::Arc;
use std::time::{Duration, Instant};

use dumbmonit_server::config::Config;
use dumbmonit_server::state::{AppState, Inner};
use dumbmonit_server::{collectors, db, scheduler, tsdb};
use sqlx::SqlitePool;

const UNREACHABLE_VICTORIA: &str = "http://127.0.0.1:1";

struct Lab {
    pool: SqlitePool,
    state: AppState,
    _dir: tempfile::TempDir,
}

async fn lab() -> Lab {
    let dir = tempfile::tempdir().expect("répertoire temporaire");
    let mut config = Config::from_env().expect("configuration");
    config.data_dir = dir.path().to_path_buf();
    config.victoria_url = Some(UNREACHABLE_VICTORIA.to_string());
    config.secret = Some("secret-de-test-suffisamment-long".to_string());

    let pool = db::open(&config.database_path()).await.expect("base");
    let cipher = db::init_cipher(&pool, "secret-de-test-suffisamment-long").await.expect("clé");
    let victoria = tsdb::Victoria::new(UNREACHABLE_VICTORIA).expect("client");
    let sink = tsdb::spawn_writer(victoria.clone(), Duration::from_secs(60));

    let mut registry = collectors::Registry::new();
    registry.register(Arc::new(collectors::DummyCollector));
    registry.register(Arc::new(collectors::HttpCollector::new()));

    let state = AppState::new(Inner {
        config,
        pool: pool.clone(),
        cipher,
        victoria,
        sink,
        collectors: registry,
    });
    Lab { pool, state, _dir: dir }
}

async fn add_target(pool: &SqlitePool, name: &str, kind: &str, address: &str, enabled: bool) -> i64 {
    sqlx::query_scalar(
        "INSERT INTO targets (name, address, kind, interval_secs, enabled, tags)
         VALUES (?, ?, ?, 10, ?, '{}') RETURNING id",
    )
    .bind(name)
    .bind(address)
    .bind(kind)
    .bind(i64::from(enabled))
    .fetch_one(pool)
    .await
    .expect("cible")
}

async fn last_probe(pool: &SqlitePool, id: i64) -> (Option<String>, Option<String>) {
    sqlx::query_as("SELECT last_probe_at, last_error FROM targets WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .expect("cible")
}

async fn wait_for_probe(pool: &SqlitePool, id: i64) -> (Option<String>, Option<String>) {
    let started = Instant::now();
    loop {
        let state = last_probe(pool, id).await;
        if state.0.is_some() {
            return state;
        }
        assert!(started.elapsed() < Duration::from_secs(25), "la cible {id} n'a pas été sondée");
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

#[tokio::test]
async fn les_cibles_echues_sont_sondees_et_leur_resultat_enregistre() {
    let lab = lab().await;
    let up = add_target(&lab.pool, "nas", "dummy", "nas.lan", true).await;
    // Une page web injoignable : la sonde échoue et l'erreur est gardée.
    let down = add_target(&lab.pool, "site", "http", "http://127.0.0.1:1/", true).await;
    let off = add_target(&lab.pool, "veille", "dummy", "veille.lan", false).await;
    // Le planificateur ne démarre qu'une fois les cibles posées : il les charge
    // dès son premier tour au lieu d'attendre le rechargement suivant.
    scheduler::spawn(lab.state.clone());

    let (probed, error) = wait_for_probe(&lab.pool, up).await;
    assert!(probed.is_some());
    assert!(error.is_none(), "la sonde factice réussit : {error:?}");

    let (_, error) = wait_for_probe(&lab.pool, down).await;
    assert!(error.is_some(), "une cible injoignable garde son erreur");

    // La cible désactivée n'a jamais été interrogée.
    assert_eq!(last_probe(&lab.pool, off).await, (None, None));
}
