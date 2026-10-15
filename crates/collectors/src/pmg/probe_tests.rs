//! Une interrogation complète contre une fausse passerelle PMG montée en mémoire :
//! ticket, jeton, nœuds multiples, appels refusés ou absents, et les erreurs
//! que le serveur doit savoir lire.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use axum::Router;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, Uri};
use axum::response::IntoResponse;
use axum::routing::{any, post};
use serde_json::{Value, json};

use super::*;

#[derive(Default)]
struct Seen {
    tickets: AtomicUsize,
    calls: std::sync::Mutex<Vec<String>>,
}

type Shared = std::sync::Arc<Seen>;

/// Corps `{"data": …}` du chemin demandé, `None` pour une 404.
fn answer(path: &str, query: &HashMap<String, String>, now: i64) -> Option<Value> {
    let tail = path.strip_prefix("/api2/json").unwrap_or(path);
    let node_part = tail.strip_prefix("/nodes/").map(|rest| rest.split_once('/'));
    let data = match (tail, node_part) {
        ("/version", _) => json!({ "version": "9.1.2", "release": "1", "repoid": "abc" }),
        ("/nodes", _) => json!([{ "node": "pmg2" }, { "node": "pmg1" }]),
        ("/statistics/mail", _) => json!({
            "count_in": "1240", "count_out": 310, "bytes_in": 4.2e7, "spamcount_in": 180,
            "viruscount_in": 3, "bounces_in": 12, "junk_in": 248, "glcount": 50,
            "rbl_rejects": 18, "avptime": "0.68"
        }),
        ("/statistics/recent", _) => json!([
            { "time": now - 3600, "timespan": 1800, "count_in": 120, "count_out": 30,
              "spam_in": 18, "virus_in": 0 },
            { "time": now - 1800, "timespan": 1800, "count_in": 100, "count_out": 20,
              "spam_in": 9, "virus_in": 1 }
        ]),
        ("/statistics/spamscores", _) => json!([
            { "level": "high", "count": 40, "ratio": 0.2 },
            { "level": "low", "count": 160, "ratio": 0.8 }
        ]),
        ("/statistics/virus", _) => json!([{ "name": "Eicar", "count": 3 }]),
        ("/quarantine/spamstatus", _) => json!({ "count": 12, "mbytes": 3.5, "avgspam": 9.2 }),
        ("/quarantine/virusstatus", _) => json!({ "count": 1, "mbytes": 0.2 }),
        ("/quarantine/attachment", _) => json!([{ "id": "a" }, { "id": "b" }]),
        ("/config/cluster/status", _) => json!([
            { "name": "pmg1", "ip": "10.0.0.1", "type": "master", "insync": 1 },
            { "name": "pmg2", "ip": "10.0.0.2", "type": "node", "insync": 0,
              "conn_error": "timeout" }
        ]),
        (_, Some(Some((_, "status")))) => json!({
            "uptime": 86400, "cpu": 0.12, "wait": 0.01, "loadavg": ["0.82", "0.7", "0.5"],
            "cpuinfo": { "cpus": 4 }, "memory": { "total": 8e9, "used": 2e9 },
            "swap": { "total": 1e9, "used": 1e8 }, "rootfs": { "total": 1e10, "used": 5e9 },
            "kversion": "Linux 6.8.12", "insync": 1, "time": now,
            "pmgversion": "pmg-api/9.1.2/42245585286a"
        }),
        (_, Some(Some((_, "services")))) => json!([
            { "service": "postfix", "state": "running", "active-state": "active",
              "unit-state": "enabled" },
            { "service": "clamav-daemon", "state": "dead", "active-state": "failed",
              "unit-state": "enabled" },
            { "service": "pmg-hide", "unit-state": "not-found" }
        ]),
        (_, Some(Some((_, "clamav/database")))) => json!([
            { "name": "daily", "type": "cvd", "build_time": "16 Dec 2025 23-18 +0000",
              "version": "27500", "nsigs": "2000000" }
        ]),
        (_, Some(Some((_, "spamassassin/rules")))) => json!([
            { "channel": "updates.spamassassin.org", "last_updated": now - 86400,
              "update_avail": 0, "version": "1234" }
        ]),
        (_, Some(Some((_, "certificates/info")))) => json!([
            { "filename": "pmg-api.pem", "subject": "CN=pmg", "issuer": "CN=pmg",
              "notafter": now + 40 * 86400, "san": ["pmg.lan"] }
        ]),
        (_, Some(Some((_, "subscription")))) => {
            json!({ "status": "active", "level": "c", "nextduedate": "2027-01-01" })
        }
        (_, Some(Some((_, "apt/update")))) => json!([
            { "Origin": "Debian" }, { "Origin": "Debian-Security" }
        ]),
        (_, Some(Some((_, "postfix/qshape")))) => {
            let queue = query.get("queue").map(String::as_str).unwrap_or("");
            if queue == "hold" {
                // Refus de lecture des files : rien à publier, pas une erreur.
                return None;
            }
            json!([
                { "domain": "TOTAL", "total": "42", "5m": "40", "1280m+": "2" },
                { "domain": "example.net", "total": "30", "5m": "30" },
                { "domain": "home.arpa", "total": "12", "5m": "10", "1280m+": "2" }
            ])
        }
        _ => return None,
    };
    Some(json!({ "data": data }))
}

async fn serve(
    State(seen): State<Shared>,
    uri: Uri,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> axum::response::Response {
    seen.calls.lock().unwrap().push(format!(
        "{} auth={} cookie={}",
        uri.path(),
        headers.contains_key("authorization"),
        headers.contains_key("cookie")
    ));
    let now = chrono::Utc::now().timestamp();
    match answer(uri.path(), &query, now) {
        Some(body) => axum::Json(body).into_response(),
        // 403 pour une file refusée, comme le fait PMG sans le droit d'audit.
        None if uri.path().ends_with("/qshape") => StatusCode::FORBIDDEN.into_response(),
        None => (StatusCode::NOT_FOUND, "no such path").into_response(),
    }
}

async fn ticket(State(seen): State<Shared>) -> axum::Json<Value> {
    seen.tickets.fetch_add(1, Ordering::SeqCst);
    axum::Json(json!({ "data": { "ticket": "PMG:monitoring@pmg:ABCDEF", "username": "m" } }))
}

async fn fake_pmg() -> (String, Shared) {
    let seen = Shared::default();
    let router = Router::new()
        .route("/api2/json/access/ticket", post(ticket))
        .fallback(any(serve))
        .with_state(seen.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (url, seen)
}

struct Capture(std::sync::Mutex<Vec<ProbeView>>);

#[async_trait]
impl ProbeObserver for Capture {
    async fn observe(&self, _target: &Target, view: &ProbeView) {
        self.0.lock().unwrap().push(view.clone());
    }
}

fn target_at(url: &str, credential: Credential, tags: &[(&str, &str)]) -> Target {
    let mut target = Target {
        id: 9,
        name: "pmg".into(),
        address: url.into(),
        kind: "pmg".into(),
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

fn password() -> Credential {
    Credential::UsernamePassword { username: "monitoring@pmg".into(), password: "secret".into() }
}

fn has(samples: &[Sample], metric: &str) -> bool {
    samples.iter().any(|s| s.metric == metric)
}

#[tokio::test]
async fn une_interrogation_complete_publie_tout_ce_que_la_passerelle_sait() {
    let (url, seen) = fake_pmg().await;
    let capture = Arc::new(Capture(Default::default()));
    let collector = PmgCollector::new().with_observer(capture.clone());
    let target = target_at(&url, password(), &[("attachment_quarantine", "true")]);

    let samples = collector.probe(&target).await.expect("interrogation");
    for metric in ["pmg_up", "pmg_scrape_errors", "pmg_scrape_duration_seconds"] {
        assert!(has(&samples, metric), "{metric} absent");
    }
    let errors = samples.iter().find(|s| s.metric == "pmg_scrape_errors").unwrap();
    assert_eq!(errors.value, 0.0, "aucune erreur de collecte attendue");

    let views = capture.0.lock().unwrap();
    let view = &views[0];
    assert_eq!(view.version.as_deref(), Some("9.1.2"));
    // Deux nœuds, interrogés dans l'ordre alphabétique.
    assert_eq!(view.nodes.iter().map(|n| n.name.as_str()).collect::<Vec<_>>(), ["pmg1", "pmg2"]);
    let node = &view.nodes[0];
    assert_eq!(node.cpu_count, Some(4.0));
    assert_eq!(node.services.len(), 2, "le service absent n'est pas listé");
    assert!(node.subscription.is_some());
    assert_eq!(node.updates_pending, Some(2.0));
    assert_eq!(node.updates_security_pending, Some(1.0));
    assert!(!node.certificates.is_empty() && !node.virus_databases.is_empty());
    assert!(!node.spam_rules.is_empty());
    // Files agrégées sur la grappe ; la file `hold` refusée n'apparaît pas.
    let queue_names: Vec<&str> = view.queues.iter().map(|q| q.queue.as_str()).collect();
    assert!(queue_names.contains(&"deferred") && !queue_names.contains(&"hold"), "{queue_names:?}");
    let deferred = view.queues.iter().find(|q| q.queue == "deferred").unwrap();
    assert_eq!(deferred.messages, 84.0, "42 messages par nœud, deux nœuds");
    assert_eq!(view.cluster.len(), 2);
    let quarantine = view.quarantine.as_ref().expect("quarantaine");
    assert_eq!(quarantine.attachment_count, Some(2.0));
    assert!(view.mail.is_some() && !view.recent.is_empty());
    assert!(!view.spam_scores.is_empty() && !view.viruses.is_empty());
    drop(views);

    // Un seul ticket pour toute l'interrogation, présenté en cookie.
    assert_eq!(seen.tickets.load(Ordering::SeqCst), 1);
    assert!(
        seen.calls
            .lock()
            .unwrap()
            .iter()
            .all(|c| c.contains("cookie=true") || c.contains("access/ticket"))
    );

    // Deuxième interrogation : le ticket est en cache.
    collector.probe(&target).await.expect("seconde interrogation");
    assert_eq!(seen.tickets.load(Ordering::SeqCst), 1, "ticket réutilisé");
}

#[tokio::test]
async fn un_jeton_d_api_et_des_options_coupees_allegent_l_interrogation() {
    let (url, seen) = fake_pmg().await;
    let collector = PmgCollector::new();
    let token = Credential::ApiToken { token: "monitoring@pmg!dumbmonit=0000-1111".into() };
    let tags = [
        ("node", "pmg1"),
        ("queues", "false"),
        ("quarantine", "false"),
        ("signatures", "false"),
        ("services", "false"),
        ("certificates", "false"),
        ("updates", "false"),
        ("subscription", "false"),
    ];
    let samples = collector.probe(&target_at(&url, token, &tags)).await.expect("interrogation");
    assert!(has(&samples, "pmg_up"));
    assert_eq!(seen.tickets.load(Ordering::SeqCst), 0, "un jeton n'ouvre pas de session");
    let calls = seen.calls.lock().unwrap().clone();
    assert!(calls.iter().all(|c| c.contains("auth=true")), "{calls:?}");
    assert!(calls.iter().any(|c| c.contains("/nodes/pmg1/status")));
    assert!(calls.iter().all(|c| !c.contains("pmg2")), "{calls:?}");
    assert!(calls.iter().all(|c| !c.contains("qshape") && !c.contains("quarantine")), "{calls:?}");
}

#[tokio::test]
async fn sans_liste_de_noeuds_la_sonde_se_replie_sur_localhost() {
    // Une passerelle dont `/nodes` répond 404 : l'état système reste lisible.
    let router = Router::new()
        .route(
            "/api2/json/version",
            axum::routing::get(|| async { axum::Json(json!({ "data": { "version": "8.0" } })) }),
        )
        .route(
            "/api2/json/nodes/localhost/status",
            axum::routing::get(|| async { axum::Json(json!({ "data": { "uptime": 10 } })) }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let capture = Arc::new(Capture(Default::default()));
    let collector = PmgCollector::new().with_observer(capture.clone());
    let token = Credential::ApiToken { token: "m@pmg!x=y".into() };
    let samples = collector.probe(&target_at(&url, token, &[("queues", "false")])).await.unwrap();
    // Les appels facultatifs absents (404) ne sont pas des erreurs de collecte.
    let errors = samples.iter().find(|s| s.metric == "pmg_scrape_errors").unwrap().value;
    assert_eq!(errors, 0.0);
    assert_eq!(capture.0.lock().unwrap()[0].nodes[0].name, "localhost");
}

#[tokio::test]
async fn les_refus_et_les_pannes_sont_classes_comme_il_faut() {
    // Mauvais identifiants : le ticket est refusé.
    let router = Router::new()
        .route("/api2/json/access/ticket", post(|| async { StatusCode::UNAUTHORIZED }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let error = PmgCollector::new().probe(&target_at(&url, password(), &[])).await.unwrap_err();
    assert!(matches!(error, ProbeError::Auth(_)), "{error}");
    assert!(!error.means_down());

    // Ticket accordé mais 401 ensuite : un seul renouvellement, puis l'échec.
    let tickets = Arc::new(AtomicUsize::new(0));
    let counter = tickets.clone();
    let router = Router::new()
        .route(
            "/api2/json/access/ticket",
            post(move || {
                let counter = counter.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    axum::Json(json!({ "data": { "ticket": "t" } }))
                }
            }),
        )
        .fallback(any(|| async { StatusCode::UNAUTHORIZED }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let error = PmgCollector::new().probe(&target_at(&url, password(), &[])).await.unwrap_err();
    assert!(matches!(error, ProbeError::Auth(_)), "{error}");
    assert_eq!(tickets.load(Ordering::SeqCst), 2, "ticket renouvelé une fois");

    // Réponse illisible, puis serveur en panne, puis serveur absent.
    let router = Router::new().fallback(any(|| async { "pas du JSON" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let token = Credential::ApiToken { token: "m@pmg!x=y".into() };
    let error = PmgCollector::new().probe(&target_at(&url, token.clone(), &[])).await.unwrap_err();
    assert!(matches!(error, ProbeError::Protocol(_)), "{error}");

    let router = Router::new().fallback(any(|| async { StatusCode::BAD_GATEWAY }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let error = PmgCollector::new().probe(&target_at(&url, token.clone(), &[])).await.unwrap_err();
    assert!(error.means_down(), "{error}");

    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let dead = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    let error = PmgCollector::new().probe(&target_at(&dead, token, &[])).await.unwrap_err();
    assert!(error.means_down(), "{error}");
}

#[tokio::test]
async fn la_decouverte_reconnait_une_passerelle() {
    let (url, _) = fake_pmg().await;
    let profile = PmgCollector::new().discover(&target_at(&url, password(), &[])).await.unwrap();
    assert_eq!(profile.as_deref(), Some("proxmox-mail-gateway"));
    let broken = target_at("http://127.0.0.1:1", password(), &[]);
    assert!(PmgCollector::new().discover(&broken).await.is_err());
}
