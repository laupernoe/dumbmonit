//! Canaux de tickets (Jira, GLPI, Zammad, GitHub, GitLab) face à un serveur HTTP
//! simulé : création, dédoublonnage, commentaires, fermeture, réouverture et
//! erreurs. Rien ne sort de la machine : l'adresse du canal est celle du simulacre.

// Le journal du simulacre est un verrou synchrone, lu entre deux appels : aucune tâche ne
// l'attend pendant un `await`.
#![allow(clippy::await_holding_lock)]

use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, Method, StatusCode, Uri};
use chrono::Utc;
use dumbmonit_server::alerting::group::{AlertGroup, GroupItem, NotifyReason};
use dumbmonit_server::alerting::machine::EffectivePhase;
use dumbmonit_server::alerting::model::Severity;
use dumbmonit_server::alerting::notify_policy::Digest;
use dumbmonit_server::db;
use dumbmonit_server::notify::tickets;
use dumbmonit_server::notify::{self, ChannelConfig, NotifyError};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tokio::net::TcpListener;

#[derive(Debug, Clone)]
struct Seen {
    method: String,
    path: String,
    headers: HeaderMap,
    body: Value,
}

impl Seen {
    fn line(&self) -> String {
        format!("{} {}", self.method, self.path)
    }
    fn header(&self, name: &str) -> String {
        self.headers.get(name).and_then(|v| v.to_str().ok()).unwrap_or_default().to_string()
    }
}

type Log = Arc<Mutex<Vec<Seen>>>;
type Responder = fn(&str, &str) -> (u16, String);

async fn handle(
    State((log, respond)): State<(Log, Responder)>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, [(&'static str, &'static str); 1], String) {
    let json = serde_json::from_slice(&body).unwrap_or(Value::Null);
    log.lock().unwrap().push(Seen {
        method: method.to_string(),
        path: uri.path().to_string(),
        headers,
        body: json,
    });
    let (status, text) = respond(method.as_str(), uri.path());
    (StatusCode::from_u16(status).unwrap(), [("content-type", "application/json")], text)
}

/// Serveur simulé : renvoie son adresse et le journal des requêtes reçues.
async fn mock(respond: Responder) -> (String, Log) {
    let log: Log = Arc::default();
    let app = Router::new().fallback(handle).with_state((log.clone(), respond));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{address}"), log)
}

struct Fixture {
    pool: SqlitePool,
    _dir: tempfile::TempDir,
}

async fn fixture(kind: &str) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let pool = db::open(&dir.path().join("dumbmonit.db")).await.unwrap();
    sqlx::query("INSERT INTO notification_channels (id, name, kind) VALUES (1, 'tickets', ?)")
        .bind(kind)
        .execute(&pool)
        .await
        .unwrap();
    Fixture { pool, _dir: dir }
}

fn config(kind: &str, settings: Value, secrets: Value) -> ChannelConfig {
    ChannelConfig {
        id: 1,
        name: "tickets".to_string(),
        kind: kind.to_string(),
        enabled: true,
        settings,
        secrets,
    }
}

fn item(reason: NotifyReason) -> GroupItem {
    GroupItem {
        fingerprint: "fp-disk".to_string(),
        rule_uid: "disk_full".to_string(),
        rule_name: "Disk full".to_string(),
        severity: Severity::Critical,
        reason,
        value: Some(95.0),
        score: None,
        unit: "%".to_string(),
        operator: ">".to_string(),
        threshold: 90.0,
        series_key: "disk".to_string(),
        since: None,
        phase: if reason == NotifyReason::Resolved {
            EffectivePhase::Resolved
        } else {
            EffectivePhase::Firing
        },
        note: None,
        detail: None,
    }
}

fn digest(reason: NotifyReason) -> Digest {
    let at = Utc::now();
    let item = item(reason);
    Digest {
        groups: vec![AlertGroup {
            target_id: Some(1),
            target_name: "nas".to_string(),
            target_kind: String::new(),
            target_tags: Default::default(),
            severity: item.severity,
            items: vec![item],
            channels: vec![],
            at,
        }],
        resolved_meanwhile: vec![],
        hold: None,
        at,
    }
}

const PUBLIC: &str = "https://monit.example.org";

/// Joue le cycle de vie complet et rend, pour chaque étape, les requêtes reçues.
async fn lifecycle(config: &ChannelConfig, fixture: &Fixture, log: &Log) -> Vec<Vec<String>> {
    let http = notify::http_client();
    let mut steps = Vec::new();
    let mut seen = 0;
    for reason in [
        NotifyReason::Firing,
        NotifyReason::Reminder,
        NotifyReason::Escalation,
        NotifyReason::Resolved,
        NotifyReason::Firing,
    ] {
        let report = tickets::deliver(
            &fixture.pool,
            &http,
            config,
            &digest(reason),
            Some(PUBLIC),
            Utc::now(),
        )
        .await;
        assert!(report.is_success(), "{reason:?}: {:?}", report.error);
        let log = log.lock().unwrap();
        steps.push(log[seen..].iter().map(Seen::line).collect());
        seen = log.len();
    }
    steps
}

fn lines(steps: &[&[&str]]) -> Vec<Vec<String>> {
    steps.iter().map(|step| step.iter().map(ToString::to_string).collect()).collect()
}

// --------------------------------------------------------------------------
// Jira
// --------------------------------------------------------------------------

fn jira_responder(method: &str, path: &str) -> (u16, String) {
    match (method, path) {
        ("POST", "/rest/api/2/issue") => (201, r#"{"id":"10001","key":"OPS-1"}"#.into()),
        ("GET", "/rest/api/2/issue/OPS-1/transitions") => (
            200,
            json!({"transitions": [
                {"id": "31", "name": "Finish", "to": {"name": "Done"}},
                {"id": "11", "name": "Reopen", "to": {"name": "To Do"}}
            ]})
            .to_string(),
        ),
        _ => (204, String::new()),
    }
}

#[tokio::test]
async fn jira_suit_le_cycle_de_vie_d_une_alerte() {
    let (url, log) = mock(jira_responder).await;
    let fixture = fixture("jira").await;
    let config = config(
        "jira",
        json!({"server_url": url, "email": "ops@example.org", "project": "OPS",
               "labels": ["dumbmonit"], "reopen_hours": 24}),
        json!({"api_token": "jeton-jira-secret"}),
    );
    let steps = lifecycle(&config, &fixture, &log).await;
    assert_eq!(
        steps,
        lines(&[
            &["POST /rest/api/2/issue"],
            &[],
            &["POST /rest/api/2/issue/OPS-1/comment"],
            &[
                "POST /rest/api/2/issue/OPS-1/comment",
                "GET /rest/api/2/issue/OPS-1/transitions",
                "POST /rest/api/2/issue/OPS-1/transitions",
            ],
            &[
                "GET /rest/api/2/issue/OPS-1/transitions",
                "POST /rest/api/2/issue/OPS-1/transitions",
                "POST /rest/api/2/issue/OPS-1/comment",
            ],
        ])
    );

    let log = log.lock().unwrap();
    let create = &log[0];
    assert!(create.header("authorization").starts_with("Basic "), "Cloud uses basic auth");
    let fields = &create.body["fields"];
    assert_eq!(fields["project"]["key"], "OPS");
    assert_eq!(fields["issuetype"]["name"], "Task");
    assert_eq!(fields["priority"]["name"], "Highest");
    assert_eq!(fields["labels"], json!(["dumbmonit"]));
    assert_eq!(fields["summary"], "nas — Disk full");
    assert!(
        fields["description"].as_str().unwrap().contains("https://monit.example.org/targets/1")
    );
    // La transition de fermeture est choisie par le nom du statut cible.
    assert_eq!(log[4].body, json!({"transition": {"id": "31"}}));
}

#[tokio::test]
async fn jira_data_center_s_authentifie_par_jeton_porteur() {
    let (url, log) = mock(jira_responder).await;
    let fixture = fixture("jira").await;
    let config = config(
        "jira",
        json!({"server_url": url, "project": "OPS", "priority": "none"}),
        json!({"api_token": "pat-jira"}),
    );
    let report = tickets::deliver(
        &fixture.pool,
        &notify::http_client(),
        &config,
        &digest(NotifyReason::Firing),
        None,
        Utc::now(),
    )
    .await;
    assert!(report.is_success(), "{:?}", report.error);
    let log = log.lock().unwrap();
    assert_eq!(log[0].header("authorization"), "Bearer pat-jira");
    assert!(log[0].body["fields"].get("priority").is_none());
}

#[tokio::test]
async fn jira_dit_quelles_transitions_existent() {
    let (url, _log) = mock(jira_responder).await;
    let fixture = fixture("jira").await;
    let config = config(
        "jira",
        json!({"server_url": url, "email": "a@b.org", "project": "OPS", "close_transition": "Nope"}),
        json!({"api_token": "t"}),
    );
    let http = notify::http_client();
    tickets::deliver(
        &fixture.pool,
        &http,
        &config,
        &digest(NotifyReason::Firing),
        None,
        Utc::now(),
    )
    .await;
    let report = tickets::deliver(
        &fixture.pool,
        &http,
        &config,
        &digest(NotifyReason::Resolved),
        None,
        Utc::now(),
    )
    .await;
    let error = report.error.expect("closing must fail");
    assert!(error.contains("Nope") && error.contains("Finish"), "{error}");
    assert!(!report.transient);
}

// --------------------------------------------------------------------------
// GLPI
// --------------------------------------------------------------------------

fn glpi_responder(method: &str, path: &str) -> (u16, String) {
    match (method, path) {
        ("GET", "/apirest.php/initSession") => (200, r#"{"session_token":"sess-glpi"}"#.into()),
        ("POST", "/apirest.php/Ticket") => {
            (201, r#"{"id":7,"message":"Item successfully added"}"#.into())
        }
        _ => (200, "{}".into()),
    }
}

#[tokio::test]
async fn glpi_ouvre_un_incident_le_suit_et_le_resout() {
    let (url, log) = mock(glpi_responder).await;
    let fixture = fixture("glpi").await;
    let config = config(
        "glpi",
        json!({"server_url": format!("{url}/apirest.php"), "entity_id": 2, "category_id": "12",
               "reopen_hours": 24}),
        json!({"user_token": "utok", "app_token": "atok"}),
    );
    let steps = lifecycle(&config, &fixture, &log).await;
    let session = ["GET /apirest.php/initSession"];
    let end = ["GET /apirest.php/killSession"];
    let wrap = |middle: &[&str]| -> Vec<String> {
        session.iter().chain(middle).chain(end.iter()).map(ToString::to_string).collect()
    };
    assert_eq!(
        steps,
        vec![
            wrap(&["POST /apirest.php/Ticket"]),
            vec![],
            wrap(&["POST /apirest.php/ITILFollowup"]),
            [wrap(&["POST /apirest.php/ITILFollowup"]), wrap(&["PUT /apirest.php/Ticket/7"]),]
                .concat(),
            [wrap(&["PUT /apirest.php/Ticket/7"]), wrap(&["POST /apirest.php/ITILFollowup"]),]
                .concat(),
        ]
    );

    let log = log.lock().unwrap();
    assert_eq!(log[0].header("authorization"), "user_token utok");
    assert_eq!(log[0].header("app-token"), "atok");
    assert_eq!(log[1].header("session-token"), "sess-glpi");
    let input = &log[1].body["input"];
    assert_eq!(input["name"], "nas — Disk full");
    assert_eq!(input["priority"], 5);
    assert_eq!(input["entities_id"], 2);
    assert_eq!(input["itilcategories_id"], 12);
    // Résolution : statut « résolu » (5) ; réouverture : « nouveau » (1).
    let puts: Vec<&Seen> = log.iter().filter(|seen| seen.method == "PUT").collect();
    assert_eq!(puts[0].body, json!({"input": {"status": 5}}));
    assert_eq!(puts[1].body, json!({"input": {"status": 1}}));
}

// --------------------------------------------------------------------------
// Zammad
// --------------------------------------------------------------------------

fn zammad_responder(method: &str, path: &str) -> (u16, String) {
    match (method, path) {
        ("POST", "/api/v1/tickets") => (201, r#"{"id":5,"number":"20005"}"#.into()),
        _ => (200, "{}".into()),
    }
}

#[tokio::test]
async fn zammad_cree_commente_ferme_et_rouvre() {
    let (url, log) = mock(zammad_responder).await;
    let fixture = fixture("zammad").await;
    let config = config(
        "zammad",
        json!({"server_url": url, "group": "Users", "customer": "ops@example.org",
               "reopen_hours": 24}),
        json!({"token": "zt"}),
    );
    let steps = lifecycle(&config, &fixture, &log).await;
    assert_eq!(
        steps,
        lines(&[
            &["POST /api/v1/tickets"],
            &[],
            &["POST /api/v1/ticket_articles"],
            &["POST /api/v1/ticket_articles", "PUT /api/v1/tickets/5"],
            &["PUT /api/v1/tickets/5", "POST /api/v1/ticket_articles"],
        ])
    );
    let log = log.lock().unwrap();
    assert_eq!(log[0].header("authorization"), "Token token=zt");
    assert_eq!(log[0].body["group"], "Users");
    assert_eq!(log[0].body["customer"], "ops@example.org");
    assert_eq!(log[0].body["priority_id"], 3);
    assert_eq!(log[1].body["ticket_id"], 5);
    assert_eq!(log[3].body, json!({"state": "closed"}));
    assert_eq!(log[4].body, json!({"state": "open"}));
}

// --------------------------------------------------------------------------
// GitHub
// --------------------------------------------------------------------------

fn github_responder(method: &str, path: &str) -> (u16, String) {
    match (method, path) {
        ("POST", "/repos/my-org/homelab/issues") => {
            (201, r#"{"number":3,"html_url":"https://github.com/my-org/homelab/issues/3"}"#.into())
        }
        _ => (200, "{}".into()),
    }
}

#[tokio::test]
async fn github_ouvre_commente_et_ferme_une_issue() {
    let (url, log) = mock(github_responder).await;
    let fixture = fixture("github_issues").await;
    let config = config(
        "github_issues",
        json!({"repository": "my-org/homelab", "api_url": url, "labels": ["monitoring"],
               "reopen_hours": 24}),
        json!({"token": "ghp-secret"}),
    );
    let steps = lifecycle(&config, &fixture, &log).await;
    assert_eq!(
        steps,
        lines(&[
            &["POST /repos/my-org/homelab/issues"],
            &[],
            &["POST /repos/my-org/homelab/issues/3/comments"],
            &[
                "POST /repos/my-org/homelab/issues/3/comments",
                "PATCH /repos/my-org/homelab/issues/3",
            ],
            &[
                "PATCH /repos/my-org/homelab/issues/3",
                "POST /repos/my-org/homelab/issues/3/comments",
            ],
        ])
    );
    let log = log.lock().unwrap();
    assert_eq!(log[0].header("authorization"), "Bearer ghp-secret");
    assert_eq!(log[0].header("x-github-api-version"), "2022-11-28");
    assert_eq!(log[0].body["labels"], json!(["monitoring"]));
    assert_eq!(log[0].body["title"], "nas — Disk full");
    assert_eq!(log[3].body, json!({"state": "closed", "state_reason": "completed"}));
    assert_eq!(log[4].body, json!({"state": "open"}));

    // Le lien du ticket est visible sur l'alerte.
    let links = tickets::links_by_fingerprint(&fixture.pool).await.unwrap();
    let link = &links["fp-disk"][0];
    assert_eq!(link.label, "#3");
    assert_eq!(link.url, "https://github.com/my-org/homelab/issues/3");
    assert_eq!(link.state, "open");
}

// --------------------------------------------------------------------------
// GitLab
// --------------------------------------------------------------------------

fn gitlab_responder(method: &str, path: &str) -> (u16, String) {
    match (method, path) {
        ("POST", "/api/v4/projects/ops%2Fhomelab/issues") => (
            201,
            r#"{"iid":4,"web_url":"https://gitlab.example.org/ops/homelab/-/issues/4"}"#.into(),
        ),
        _ => (200, "{}".into()),
    }
}

#[tokio::test]
async fn gitlab_encode_le_chemin_du_projet_et_ferme_par_evenement() {
    let (url, log) = mock(gitlab_responder).await;
    let fixture = fixture("gitlab_issues").await;
    let config = config(
        "gitlab_issues",
        json!({"server_url": url, "project": "ops/homelab", "labels": ["a", "b"]}),
        json!({"token": "glpat-secret"}),
    );
    let http = notify::http_client();
    for reason in [NotifyReason::Firing, NotifyReason::Resolved] {
        let report =
            tickets::deliver(&fixture.pool, &http, &config, &digest(reason), None, Utc::now())
                .await;
        assert!(report.is_success(), "{:?}", report.error);
    }
    let log = log.lock().unwrap();
    let seen: Vec<String> = log.iter().map(Seen::line).collect();
    assert_eq!(
        seen,
        vec![
            "POST /api/v4/projects/ops%2Fhomelab/issues",
            "POST /api/v4/projects/ops%2Fhomelab/issues/4/notes",
            "PUT /api/v4/projects/ops%2Fhomelab/issues/4",
        ]
    );
    assert_eq!(log[0].header("private-token"), "glpat-secret");
    assert_eq!(log[0].body["labels"], "a,b");
    assert_eq!(log[2].body, json!({"state_event": "close"}));

    // Fermé, sans délai de réouverture : le lien reste mais l'alerte suivante ouvrira un nouveau ticket.
    let links = tickets::links_by_fingerprint(&fixture.pool).await.unwrap();
    assert_eq!(links["fp-disk"][0].state, "closed");
}

// --------------------------------------------------------------------------
// Dédoublonnage, réouverture, erreurs, test
// --------------------------------------------------------------------------

#[tokio::test]
async fn sans_delai_de_reouverture_une_alerte_qui_revient_ouvre_un_nouveau_ticket() {
    let (url, log) = mock(github_responder).await;
    let fixture = fixture("github_issues").await;
    let config = config(
        "github_issues",
        json!({"repository": "my-org/homelab", "api_url": url}),
        json!({"token": "t"}),
    );
    let http = notify::http_client();
    for reason in [NotifyReason::Firing, NotifyReason::Resolved, NotifyReason::Firing] {
        tickets::deliver(&fixture.pool, &http, &config, &digest(reason), None, Utc::now()).await;
    }
    let creations = log
        .lock()
        .unwrap()
        .iter()
        .filter(|seen| seen.line() == "POST /repos/my-org/homelab/issues")
        .count();
    assert_eq!(creations, 2);
}

#[tokio::test]
async fn une_resolution_sans_ticket_ne_fait_aucun_appel() {
    let (url, log) = mock(github_responder).await;
    let fixture = fixture("github_issues").await;
    let config = config(
        "github_issues",
        json!({"repository": "my-org/homelab", "api_url": url}),
        json!({"token": "t"}),
    );
    let report = tickets::deliver(
        &fixture.pool,
        &notify::http_client(),
        &config,
        &digest(NotifyReason::Resolved),
        None,
        Utc::now(),
    )
    .await;
    assert!(report.is_success());
    assert!(log.lock().unwrap().is_empty());
}

fn refusing(_: &str, _: &str) -> (u16, String) {
    (401, r#"{"message":"Bad credentials ghp-secret"}"#.into())
}

fn failing(_: &str, _: &str) -> (u16, String) {
    (503, "{}".into())
}

#[tokio::test]
async fn les_erreurs_sont_rapportees_sans_secret_et_sans_lien_enregistre() {
    let fixture = fixture("github_issues").await;
    let http = notify::http_client();

    let (url, _log) = mock(refusing).await;
    let config = config(
        "github_issues",
        json!({"repository": "my-org/homelab", "api_url": url}),
        json!({"token": "ghp-secret"}),
    );
    let report = tickets::deliver(
        &fixture.pool,
        &http,
        &config,
        &digest(NotifyReason::Firing),
        None,
        Utc::now(),
    )
    .await;
    let error = report.error.expect("a refusal is reported");
    assert!(error.contains("401") && !error.contains("ghp-secret"), "{error}");
    assert!(!report.transient, "a bad token is not retried");

    let (url, _log) = mock(failing).await;
    let config = config_with_url(&config, &url);
    let report = tickets::deliver(
        &fixture.pool,
        &http,
        &config,
        &digest(NotifyReason::Firing),
        None,
        Utc::now(),
    )
    .await;
    assert!(report.error.is_some());
    assert!(report.transient, "an outage is retried by the engine");

    let links = tickets::links_by_fingerprint(&fixture.pool).await.unwrap();
    assert!(links.is_empty(), "no ticket, no link");
}

fn config_with_url(config: &ChannelConfig, url: &str) -> ChannelConfig {
    let mut copy = config.clone();
    copy.settings["api_url"] = json!(url);
    copy
}

#[tokio::test]
async fn le_test_cree_puis_ferme_un_ticket() {
    let (url, log) = mock(github_responder).await;
    let config = config(
        "github_issues",
        json!({"repository": "my-org/homelab", "api_url": url}),
        json!({"token": "t"}),
    );
    notify::test_channel(&notify::http_client(), &config, Some(PUBLIC)).await.unwrap();
    let seen: Vec<String> = log.lock().unwrap().iter().map(Seen::line).collect();
    assert_eq!(
        seen,
        vec![
            "POST /repos/my-org/homelab/issues",
            "POST /repos/my-org/homelab/issues/3/comments",
            "PATCH /repos/my-org/homelab/issues/3",
        ]
    );
}

fn github_cannot_close(method: &str, path: &str) -> (u16, String) {
    if method == "PATCH" { (403, "{}".into()) } else { github_responder(method, path) }
}

#[tokio::test]
async fn un_test_qui_ne_peut_pas_fermer_nomme_le_ticket_a_fermer() {
    let (url, _log) = mock(github_cannot_close).await;
    let config = config(
        "github_issues",
        json!({"repository": "my-org/homelab", "api_url": url}),
        json!({"token": "t"}),
    );
    let error = notify::test_channel(&notify::http_client(), &config, None).await.unwrap_err();
    assert!(matches!(error, NotifyError::Config(_)));
    assert!(error.to_string().contains("#3"), "{error}");
}

#[tokio::test]
async fn l_acquittement_est_ajoute_aux_tickets_ouverts() {
    let (url, log) = mock(github_responder).await;
    let fixture = fixture("github_issues").await;
    let config = config(
        "github_issues",
        json!({"repository": "my-org/homelab", "api_url": url}),
        json!({"token": "t"}),
    );
    // Le chiffrement du canal passe par la vraie base : on l'y enregistre.
    let cipher = db::init_cipher(&fixture.pool, "a-test-secret-that-is-long-enough").await.unwrap();
    let encrypted = cipher.encrypt(config.secrets.to_string().as_bytes()).unwrap();
    sqlx::query("UPDATE notification_channels SET settings = ?, secret_enc = ? WHERE id = 1")
        .bind(config.settings.to_string())
        .bind(encrypted)
        .execute(&fixture.pool)
        .await
        .unwrap();
    tickets::deliver(
        &fixture.pool,
        &notify::http_client(),
        &config,
        &digest(NotifyReason::Firing),
        None,
        Utc::now(),
    )
    .await;
    tickets::on_acknowledged(&fixture.pool, &cipher, "fp-disk", "Acknowledged by noe.").await;
    let log = log.lock().unwrap();
    let last = log.last().unwrap();
    assert_eq!(last.line(), "POST /repos/my-org/homelab/issues/3/comments");
    assert_eq!(last.body["body"], "Acknowledged by noe.");
}
