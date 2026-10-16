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
