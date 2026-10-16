//! Une interrogation complète contre une fausse console PDM montée en mémoire :
//! instances fédérées (dont une injoignable), versions, tâches, état de l'hôte
//! et les refus que la console oppose aux jetons trop modestes.

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex as StdMutex;
use std::time::Duration;

use axum::Router;
use axum::extract::Query;
use axum::http::{StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::any;
use serde_json::{Value, json};

use super::*;

fn answer(path: &str, now: i64) -> Option<Value> {
    let tail = path.strip_prefix("/api2/json").unwrap_or(path);
    let data = match tail {
        "/version" => json!({ "version": "1.0.2", "release": "1", "repoid": "abc" }),
        "/remotes/remote" => json!([
            { "id": "pve-main", "type": "pve", "nodes": ["10.0.0.5"] },
            { "id": "pve-old", "type": "pve", "nodes": ["10.0.0.6"] },
            { "id": "pbs-1", "type": "pbs", "nodes": ["10.0.0.7"] }
        ]),
        "/resources/status" => json!({
            "remotes": 2, "failed_remotes": 1,
            "remote-list": [
                { "name": "pve-main", "status": "Ok", "ty": "pve" },
                { "name": "pve-old", "status": "Error", "ty": "pve",
                  "messages": ["connection refused"] }
            ],
            "qemu": { "running": 5, "stopped": 2 }, "lxc": { "running": 3, "stopped": 1 },
            "pve_nodes": { "online": 2, "offline": 0 },
            "pve_cpu_stats": { "used": 3.5, "max": 16 },
            "pve_memory_stats": { "used": 8e9, "total": 6.4e10 },
            "pve_storage_stats": { "used": 1e11, "total": 1e12 },
            "pbs_datastores": { "online": 2 }
        }),
        "/resources/list" => json!([
            { "remote": "pve-main", "resources": [
                { "type": "pve-qemu", "id": "qemu/100", "name": "web", "status": "running",
                  "cpu": 0.1, "maxcpu": 4, "mem": 1e9, "maxmem": 4e9 },
                { "type": "pve-qemu", "id": "qemu/101", "status": "stopped", "template": true },
                { "type": "pve-node", "id": "node/pve1", "name": "pve1", "status": "online",
                  "cpu": 0.2, "maxcpu": 8, "mem": 4e9, "maxmem": 3.2e10 },
                { "type": "pve-storage", "id": "storage/pve1/local", "storage": "local",
                  "status": "available", "disk": 1e10, "maxdisk": 1e11 }
            ] },
            { "remote": "pve-old", "error": "connection refused" }
        ]),
        "/resources/subscription" => json!([
            { "remote": "pve-main", "state": "active" }, { "remote": "pve-old", "state": "none" }
        ]),
        "/remotes/metric-collection/status" => json!([
            { "remote": "pve-main", "last-collection": now - 30 },
            { "remote": "pve-old", "last-collection": now - 7200, "error": "timeout" }
        ]),
        "/remotes/tasks/list" => json!([
            { "upid": "pve-main!UPID:pve1:0001:0002:0003:vzdump:100:root@pam:",
              "worker_type": "vzdump", "worker_id": "100", "node": "pve1",
              "starttime": now - 3600, "endtime": now - 3500, "status": "OK" },
            { "upid": "pve-main!UPID:pve1:0004:0005:0006:qmstart:101:root@pam:",
              "worker_type": "qmstart", "node": "pve1", "starttime": now - 600,
              "endtime": now - 590, "status": "command failed" },
            { "upid": "pve-main!UPID:pve1:0007:0008:0009:qmstop:102:root@pam:",
              "worker_type": "qmstop", "node": "pve1", "starttime": now - 20 }
        ]),
        "/remotes/updates/summary" => json!({
            "remotes": { "pve-main": { "available-updates": 7 }, "pbs-1": { "count": 2 } }
        }),
        "/remotes/remote/pve-main/version" => json!({ "version": "8.4.1" }),
        "/remotes/remote/pbs-1/version" => json!({ "version": "3.4.0" }),
        "/nodes/localhost/status" => json!({
            "uptime": 3600, "cpu": 0.05, "wait": 0.0, "loadavg": ["0.1", "0.2", "0.3"],
            "cpuinfo": { "cpus": 2, "model": "Xeon" },
            "memory": { "total": 4e9, "used": 1e9 }, "swap": { "total": 0, "used": 0 },
            "root": { "total": 3e10, "used": 1e10 }, "kversion": "Linux 6.8"
        }),
        "/nodes/localhost/certificates/info" => json!([
            { "filename": "proxy.pem", "subject": "CN=pdm", "issuer": "CN=pdm",
              "notafter": now + 20 * 86400 }
        ]),
        "/nodes/localhost/apt/update" => json!([{}, {}, {}]),
        "/nodes/localhost/subscription" => json!({
            "status": "active", "statistics": { "active-subscriptions": 2, "total-nodes": 3 }
        }),
        _ => return None,
    };
    Some(json!({ "data": data }))
}

async fn fake_pdm(
    deny: &'static [&'static str],
) -> (String, std::sync::Arc<StdMutex<Vec<String>>>) {
    let seen = std::sync::Arc::new(StdMutex::new(Vec::<String>::new()));
    let log = seen.clone();
    let router = Router::new().fallback(any(
        move |uri: Uri,
              headers: axum::http::HeaderMap,
              Query(_): Query<HashMap<String, String>>| {
            let log = log.clone();
            async move {
                log.lock().unwrap().push(format!(
                    "{} {}",
                    uri.path(),
                    headers.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("-")
                ));
                let tail = uri.path().strip_prefix("/api2/json").unwrap_or(uri.path());
                if deny.contains(&tail) {
                    return StatusCode::FORBIDDEN.into_response();
                }
                match answer(uri.path(), chrono::Utc::now().timestamp()) {
                    Some(body) => axum::Json(body).into_response(),
                    None => (StatusCode::NOT_FOUND, "no such path").into_response(),
                }
            }
        },
    ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (url, seen)
}

struct Capture(StdMutex<Vec<ProbeView>>);

#[async_trait]
impl ProbeObserver for Capture {
    async fn observe(&self, _target: &Target, view: &ProbeView) {
        self.0.lock().unwrap().push(view.clone());
    }
}

fn target_at(url: &str, credential: Credential, tags: &[(&str, &str)]) -> Target {
    let mut target = Target {
        id: 3,
        name: "pdm".into(),
        address: url.into(),
        kind: "pdm".into(),
        profile_id: None,
        parent_id: None,
        interval: Duration::from_secs(60),
        enabled: true,
        tags: BTreeMap::new(),
        credential,
        group_name: String::new(),
        position: 0,
    };
    for (key, value) in tags {
        target.tags.insert((*key).into(), (*value).into());
    }
    target
}

fn token() -> Credential {
    Credential::ApiToken { token: "monitoring@pdm!dumbmonit=0000-1111".into() }
}

fn value_of(samples: &[Sample], metric: &str) -> Option<f64> {
    samples.iter().find(|s| s.metric == metric).map(|s| s.value)
}

#[tokio::test]
async fn une_interrogation_complete_decrit_le_parc_federe() {
    let (url, seen) = fake_pdm(&[]).await;
    let capture = Arc::new(Capture(Default::default()));
    let collector = PdmCollector::new().with_observer(capture.clone());

    let tags = [("remote_updates", "true")];
    let samples = collector.probe(&target_at(&url, token(), &tags)).await.expect("interrogation");
    assert_eq!(value_of(&samples, "pdm_up"), Some(1.0));
    assert_eq!(value_of(&samples, "pdm_scrape_errors"), Some(0.0));

    let views = capture.0.lock().unwrap();
    let view = &views[0];
    assert_eq!(view.version.as_deref(), Some("1.0.2"));
    let ids: Vec<&str> = view.remotes.iter().map(|r| r.id.as_str()).collect();
    assert!(
        ids.contains(&"pve-main") && ids.contains(&"pve-old") && ids.contains(&"pbs-1"),
        "{ids:?}"
    );
    let main = view.remotes.iter().find(|r| r.id == "pve-main").unwrap();
    assert!(main.reachable);
    assert_eq!(main.version.as_deref(), Some("8.4.1"));
    assert_eq!(main.updates_pending, Some(7.0));
    let old = view.remotes.iter().find(|r| r.id == "pve-old").unwrap();
    assert!(!old.reachable, "la console n'atteint pas cette instance");
    assert!(old.error.is_some());
    assert_eq!(view.tasks.len(), 3);
    let node = view.node.as_ref().expect("hôte de la console");
    assert_eq!(node.updates_pending, Some(3.0));
    assert!(node.subscription.is_some());
    assert!(!node.certificates.is_empty());

    // Le jeton voyage dans l'en-tête propre à PDM, jamais dans l'URL.
    let calls = seen.lock().unwrap().clone();
    assert!(
        calls.iter().all(|c| c.contains("PDMAPIToken=monitoring@pdm!dumbmonit:0000-1111")),
        "{calls:?}"
    );
}

#[tokio::test]
async fn les_options_filtrent_les_instances_et_coupent_les_appels() {
    let (url, seen) = fake_pdm(&[]).await;
    let tags = [
        ("remotes", "pve-main"),
        ("versions", "false"),
        ("tasks", "false"),
        ("node_status", "false"),
        ("updates", "false"),
        ("remote_updates", "false"),
    ];
    let capture = Arc::new(Capture(Default::default()));
    let collector = PdmCollector::new().with_observer(capture.clone());
    collector.probe(&target_at(&url, token(), &tags)).await.expect("interrogation");

    let views = capture.0.lock().unwrap();
    assert_eq!(views[0].remotes.len(), 1);
    assert_eq!(views[0].remotes[0].id, "pve-main");
    assert!(views[0].remotes[0].version.is_none());
    assert!(views[0].tasks.is_empty());
    let calls = seen.lock().unwrap().clone();
    assert!(calls.iter().all(|c| !c.contains("/tasks/") && !c.contains("/nodes/")), "{calls:?}");
    assert!(calls.iter().all(|c| !c.contains("/remote/pve-main/version")), "{calls:?}");
}

#[tokio::test]
async fn un_jeton_sans_droits_facultatifs_ne_fait_pas_echouer_la_sonde() {
    let (url, _) = fake_pdm(&[
        "/remotes/tasks/list",
        "/resources/subscription",
        "/nodes/localhost/certificates/info",
        "/nodes/localhost/subscription",
    ])
    .await;
    let samples =
        PdmCollector::new().probe(&target_at(&url, token(), &[])).await.expect("interrogation");
    assert_eq!(value_of(&samples, "pdm_scrape_errors"), Some(0.0));
}

#[tokio::test]
async fn un_inventaire_obligatoire_en_panne_compte_des_erreurs_de_collecte() {
    // `/resources/list` et `/remotes/remote` sont absents (404 hors refus) : la
    // sonde répond tout de même, avec des erreurs comptées.
    let router = Router::new().fallback(any(|uri: Uri| async move {
        if uri.path() == "/api2/json/version" {
            axum::Json(json!({ "data": { "version": "1.0" } })).into_response()
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR, "boom").into_response()
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let samples =
        PdmCollector::new().probe(&target_at(&url, token(), &[])).await.expect("interrogation");
    assert!(value_of(&samples, "pdm_scrape_errors").unwrap() >= 3.0);
}

#[tokio::test]
async fn les_identifiants_et_les_pannes_sont_classes() {
    // Seul un jeton convient à PDM.
    let (url, _) = fake_pdm(&[]).await;
    let password =
        Credential::UsernamePassword { username: "root@pam".into(), password: "x".into() };
    let error = PdmCollector::new().probe(&target_at(&url, password, &[])).await.unwrap_err();
    assert!(matches!(error, ProbeError::Config(_)), "{error}");
    assert!(!error.to_string().contains("\"x\""));

    let router = Router::new().fallback(any(|| async { StatusCode::UNAUTHORIZED }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let error = PdmCollector::new().probe(&target_at(&url, token(), &[])).await.unwrap_err();
    assert!(matches!(error, ProbeError::Auth(_)), "{error}");

    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let dead = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    let error = PdmCollector::new().probe(&target_at(&dead, token(), &[])).await.unwrap_err();
    assert!(error.means_down(), "{error}");

    let router = Router::new().fallback(any(|| async { "pas du JSON" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let error = PdmCollector::new().probe(&target_at(&url, token(), &[])).await.unwrap_err();
    assert!(matches!(error, ProbeError::Protocol(_)), "{error}");
}

#[tokio::test]
async fn la_decouverte_reconnait_une_console_pdm() {
    let (url, _) = fake_pdm(&[]).await;
    let profile = PdmCollector::new().discover(&target_at(&url, token(), &[])).await.unwrap();
    assert_eq!(profile.as_deref(), Some("proxmox-datacenter-manager"));
}

#[test]
fn les_noms_d_instances_sont_encodes_dans_les_chemins() {
    assert_eq!(percent_encode("pve-main"), "pve-main");
    assert_eq!(percent_encode("a b/c"), "a%20b%2Fc");
}
