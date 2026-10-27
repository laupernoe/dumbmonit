//! Lecture de l'inventaire contre un faux démon sur un vrai socket Unix.

use super::*;
use crate::testsupport::fake_docker;

const LIST: &str = r#"[
    {"Id":"aaa111","Names":["/web"],"Image":"nginx:1.27","ImageID":"sha256:img1","State":"running"},
    {"Id":"bbb222","Names":["/job"],"Image":"restic:1","ImageID":"sha256:img1","State":"exited"}
]"#;
const RUNNING: &str = r#"{"RestartCount":2,
    "State":{"Status":"running","Running":true,"StartedAt":"2026-09-15T10:08:06Z",
             "Health":{"Status":"unhealthy"}}}"#;
const IMAGE: &str = r#"{"Id":"sha256:img1","RepoDigests":["nginx@sha256:d1"],
    "Created":"2026-08-22T12:20:23Z"}"#;

#[tokio::test]
async fn the_inventory_is_detailed_from_the_daemon_and_images_are_cached() {
    let dir = tempfile::tempdir().unwrap();
    let (socket, calls) = fake_docker(&dir, |req| match req.path.as_str() {
        "/containers/json?all=1" => (200, LIST.to_string()),
        "/containers/aaa111/json" => (200, RUNNING.to_string()),
        // Disparu entre la liste et l'inspection : pas une panne.
        "/containers/bbb222/json" => (404, r#"{"message":"gone"}"#.to_string()),
        "/images/sha256:img1/json" => (200, IMAGE.to_string()),
        _ => (500, String::new()),
    });
    let mut probe = DockerProbe::new(&socket, 10);

    let inventory = probe.read().await.expect("inventaire");
    assert_eq!((inventory.total, inventory.running), (2, 1));
    let web = inventory.detailed.iter().find(|c| c.name == "web").unwrap();
    assert!(web.running);
    assert_eq!(web.health, Health::Unhealthy);
    assert_eq!(web.restart_count, 2);
    assert!(web.uptime_secs > 0);
    assert!(web.image_age_secs.is_some());
    // Le conteneur introuvable garde les valeurs de la liste.
    let job = inventory.detailed.iter().find(|c| c.name == "job").unwrap();
    assert!(!job.running);

    // Une image ne se réinspecte pas à chaque cycle.
    assert_eq!(calls.count("GET", "/images/"), 1);
    probe.read().await.expect("second cycle");
    assert_eq!(calls.count("GET", "/images/"), 1, "le cache aurait dû servir");
    assert_eq!(probe.repo_digests("sha256:img1"), ["nginx@sha256:d1"]);
    assert!(probe.repo_digests("sha256:inconnue").is_empty());
}

#[tokio::test]
async fn a_refusing_or_incoherent_daemon_yields_no_inventory() {
    let dir = tempfile::tempdir().unwrap();
    // Agent hors du groupe `docker` : le démon refuse.
    let (socket, _) = fake_docker(&dir, |_| (403, r#"{"message":"permission denied"}"#.into()));
    assert!(DockerProbe::new(&socket, 10).read().await.is_none());

    let dir = tempfile::tempdir().unwrap();
    let (socket, _) = fake_docker(&dir, |_| (200, "pas du json".into()));
    assert!(DockerProbe::new(&socket, 10).read().await.is_none());

    // Sans socket du tout : cas nominal d'une machine sans Docker.
    let absent = std::path::Path::new("/inexistant/docker.sock");
    assert!(DockerProbe::new(absent, 10).read().await.is_none());
    // Socket présent mais sans démon derrière.
    let dir = tempfile::tempdir().unwrap();
    let dead = dir.path().join("mort.sock");
    drop(std::os::unix::net::UnixListener::bind(&dead).unwrap());
    assert!(DockerProbe::new(&dead, 10).read().await.is_none());
}

#[tokio::test]
async fn the_client_wraps_failures_with_a_readable_message() {
    let dir = tempfile::tempdir().unwrap();
    let (socket, calls) =
        fake_docker(&dir, |_| (409, r#"{"message":"conteneur déjà là"}"#.to_string()));
    let client = DockerClient::new(&socket);
    let response = client.post("/containers/x/start", Some("{}"), Duration::from_secs(5)).await;
    let response = response.expect("réponse");
    assert!(!response.is_success());
    assert_eq!(response.message(), "conteneur déjà là");
    assert!(response.json::<serde_json::Value>("start").is_err());
    let removed = client.delete("/containers/x", Duration::from_secs(5)).await.unwrap();
    assert_eq!(removed.status, 409);
    assert_eq!(calls.count("POST", "/containers/x/start"), 1);
    assert_eq!(calls.bodies("POST", "/containers/x/start"), ["{}"]);

    // Un démon qui ne répond pas dans le délai imparti.
    let dir = tempfile::tempdir().unwrap();
    let slow = dir.path().join("lent.sock");
    let listener = tokio::net::UnixListener::bind(&slow).unwrap();
    tokio::spawn(async move {
        let _held = listener.accept().await;
        tokio::time::sleep(Duration::from_secs(30)).await;
    });
    let error = DockerClient::new(&slow)
        .request("GET", "/x", None, Duration::from_millis(100))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("did not answer"), "{error}");
}

#[tokio::test]
async fn usage_and_disk_are_read_and_the_cpu_needs_two_cycles() {
    use std::sync::atomic::{AtomicU64, Ordering};
    let dir = tempfile::tempdir().unwrap();
    let cpu = std::sync::Arc::new(AtomicU64::new(1_000_000_000));
    let seen = cpu.clone();
    let (socket, calls) = fake_docker(&dir, move |req| match req.path.as_str() {
        "/containers/json?all=1" => (200, LIST.to_string()),
        "/containers/aaa111/json" => (
            200,
            r#"{"State":{"Status":"running","Running":true},"HostConfig":{"Memory":1000}}"#
                .to_string(),
        ),
        "/containers/bbb222/json" => (200, r#"{"State":{"Status":"exited"}}"#.to_string()),
        "/containers/aaa111/stats?stream=false&one-shot=true" => (
            200,
            format!(
                r#"{{"cpu_stats":{{"cpu_usage":{{"total_usage":{}}}}},
                    "memory_stats":{{"usage":400,"stats":{{"inactive_file":100}}}}}}"#,
                seen.fetch_add(500_000_000, Ordering::SeqCst)
            ),
        ),
        "/system/df" => (
            200,
            r#"{"LayersSize":42,"Images":[],"Containers":[],"Volumes":[],"BuildCache":[]}"#
                .to_string(),
        ),
        "/images/sha256:img1/json" => (200, IMAGE.to_string()),
        _ => (500, String::new()),
    });
    let mut probe = DockerProbe::new(&socket, 10);

    let first = probe.read().await.expect("premier cycle");
    let web = first.detailed.iter().find(|c| c.name == "web").unwrap();
    let usage = web.usage.as_ref().expect("statistiques");
    assert_eq!(usage.memory_bytes, 300);
    assert_eq!(web.memory_limit_bytes, Some(1000));
    assert_eq!(usage.cpu_centipercent, None, "pas de CPU au premier relevé");
    assert_eq!(first.disk.as_ref().map(|d| d.images_bytes), Some(42));
    // Un conteneur arrêté n'est pas interrogé.
    assert_eq!(calls.count("GET", "/containers/bbb222/stats"), 0);
    assert!(first.detailed.iter().find(|c| c.name == "job").unwrap().usage.is_none());

    tokio::time::sleep(Duration::from_millis(250)).await;
    let second = probe.read().await.expect("second cycle");
    let web = second.detailed.iter().find(|c| c.name == "web").unwrap();
    let centi = web.usage.as_ref().unwrap().cpu_centipercent.expect("CPU au second relevé");
    // 0,5 s de CPU en ~0,25 s d'horloge : de l'ordre de deux cœurs, la
    // précision du minuteur ne se teste pas ici.
    assert!(centi > 0, "{centi}");
    // `system/df` est mis en cache dix minutes.
    assert_eq!(calls.count("GET", "/system/df"), 1);
}

#[tokio::test]
async fn a_daemon_without_stats_still_yields_the_inventory() {
    let dir = tempfile::tempdir().unwrap();
    let (socket, _) = fake_docker(&dir, |req| match req.path.as_str() {
        "/containers/json?all=1" => (200, LIST.to_string()),
        "/containers/aaa111/json" => (200, RUNNING.to_string()),
        _ => (404, String::new()),
    });
    let inventory = DockerProbe::new(&socket, 10).read().await.expect("inventaire");
    assert!(inventory.detailed.iter().all(|c| c.usage.is_none()));
    assert!(inventory.disk.is_none());
}
