//! Le client contre un vrai serveur HTTP local : ce qui part (jeton, secret de
//! liaison, corps) et la façon dont chaque statut de réponse est qualifié.

use super::*;
use crate::testsupport::fake_http;
use dumbmonit_proto::{AgentIdentity, CommandStatus};

fn batch() -> PushBatch {
    let identity = AgentIdentity {
        hostname: "nas".into(),
        os: "linux".into(),
        os_version: None,
        kernel_version: None,
        arch: None,
        agent_version: "0.0.0".into(),
        commands_enabled: None,
        relay: false,
        site: None,
        machine_id: None,
        tags: Default::default(),
        binding_supported: true,
    };
    PushBatch::new(identity, Vec::new(), 1_000)
}

fn client(url: &str, secret: Option<&str>) -> PushClient {
    PushClient::new(url, "dmon_jeton", secret.map(str::to_string), Duration::from_secs(5))
        .expect("client")
}

const ACK: &str = r#"{"target_id":12,"accepted":3,"interval_secs":45,"registered":true,
    "agent_secret":"dmab_nouveau","bound":true}"#;

#[tokio::test]
async fn an_accepted_batch_returns_the_ack_and_carries_the_credentials() {
    let (url, calls) = fake_http(|_| (200, ACK.to_string())).await;
    let ack = client(&url, Some("dmab_actuel")).send(&batch()).await.expect("accusé");
    assert_eq!((ack.target_id, ack.accepted, ack.interval_secs), (12, 3, 45));
    assert_eq!(ack.agent_secret.as_deref(), Some("dmab_nouveau"));
    assert!(ack.bound);

    let request = &calls.all()[0];
    assert_eq!((request.method.as_str(), request.path.as_str()), ("POST", "/api/ingest"));
    assert_eq!(request.header("authorization"), Some("Bearer dmon_jeton"));
    assert_eq!(request.header("x-dumbmonit-agent-secret"), Some("dmab_actuel"));
    assert!(request.body.contains("\"hostname\":\"nas\""), "{}", request.body);

    // Pas encore lié : l'en-tête n'est pas envoyé du tout.
    client(&url, None).send(&batch()).await.expect("accusé");
    assert!(calls.all()[1].header("x-dumbmonit-agent-secret").is_none());
}

#[tokio::test]
async fn each_status_is_qualified_for_the_retry_decision() {
    for (status, body, retryable) in [
        (401, "", true),
        (403, r#"{"error":"Allow re-enrolment"}"#, true),
        (429, r#"{"error":"slow down"}"#, true),
        (503, "maintenance", true),
        (400, r#"{"error":"empty batch"}"#, false),
        (422, "", false),
        // Sans l'enveloppe d'erreur du serveur, un 400 ou un 404 vient d'un
        // mandataire (chemin mal routé, maintenance) : le lot n'y est pour rien.
        (400, "<html>400 Bad Request</html>", true),
        (404, "<html>404 Not Found</html>", true),
        (404, r#"{"error":"Unknown command."}"#, false),
        // Trop gros pour un intermédiaire : à couper, pas à jeter.
        (413, "<html>413 Request Entity Too Large</html>", true),
    ] {
        let reply = body.to_string();
        let (url, _) = fake_http(move |_| (status, reply.clone())).await;
        let error = client(&url, None).send(&batch()).await.expect_err("refus");
        assert_eq!(error.is_retryable(), retryable, "{status}: {error}");
        match (status, &error) {
            (401, PushError::Unauthorized) => {}
            (403, PushError::Forbidden(message)) => assert_eq!(message, "Allow re-enrolment"),
            (400 | 404 | 429 | 503, PushError::Transport(message)) => {
                assert!(message.starts_with(&format!("HTTP {status}")), "{message}")
            }
            (413, PushError::TooLarge(_)) => {}
            (400 | 404 | 422, PushError::Rejected { status: got, .. }) => assert_eq!(*got, status),
            other => panic!("qualification inattendue : {other:?}"),
        }
    }
}

#[tokio::test]
async fn an_unreadable_ack_or_a_dead_server_is_a_transport_error() {
    let (url, _) = fake_http(|_| (200, "ça n'est pas du JSON".to_string())).await;
    let error = client(&url, None).send(&batch()).await.unwrap_err();
    assert!(matches!(&error, PushError::Transport(m) if m.contains("unreadable response")));

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let dead = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let error = client(&dead, None).send(&batch()).await.unwrap_err();
    assert!(matches!(error, PushError::Transport(_)), "{error}");
    assert!(!error.to_string().contains("dmon_jeton"));
    assert!(client(&dead, None).fetch_commands("k").await.is_err());
    assert!(client(&dead, None).report_probe("k", 1, &outcome()).await.is_err());
    let report = CommandReport { status: CommandStatus::Done, result: String::new() };
    assert!(client(&dead, None).report_command("k", 1, &report).await.is_err());
    let wait = Duration::from_secs(1);
    assert!(client(&dead, None).fetch_probes("k", wait, wait).await.is_err());
}

fn outcome() -> ProbeOutcome {
    ProbeOutcome {
        duration_ms: 12,
        error: Some("timeout".into()),
        samples: Vec::new(),
        profile_id: None,
    }
}

#[tokio::test]
async fn commands_are_fetched_and_reported_with_the_machine_key() {
    let (url, calls) = fake_http(|req| match req.method.as_str() {
        "GET" => (
            200,
            r#"[{"id":5,"kind":"container.restart","args":{"name":"web"},"created_at_ms":1}]"#
                .into(),
        ),
        _ => (204, String::new()),
    })
    .await;
    let client = client(&url, Some("dmab_s"));

    let commands = client.fetch_commands("nas salon").await.expect("commandes");
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].arg_str("name"), Some("web"));
    assert_eq!(calls.all()[0].path, "/api/agent/commands?key=nas%20salon");

    let report = CommandReport { status: CommandStatus::Done, result: "ok".into() };
    client.report_command("nas salon", 5, &report).await.expect("compte rendu");
    let sent = &calls.all()[1];
    assert_eq!(sent.path, "/api/agent/commands/5?key=nas%20salon");
    assert!(sent.body.contains("\"done\""), "{}", sent.body);
}

#[tokio::test]
async fn relay_probes_are_polled_with_a_wait_and_reported() {
    let (url, calls) = fake_http(|req| match req.method.as_str() {
        "GET" => (200, "[]".to_string()),
        _ => (403, r#"{"error":"not your device"}"#.to_string()),
    })
    .await;
    let client = client(&url, None);
    let jobs = client.fetch_probes("site-a", Duration::from_secs(25), Duration::from_secs(5)).await;
    assert!(jobs.expect("sondes").is_empty());
    assert_eq!(calls.all()[0].path, "/api/agent/relay?key=site-a&wait=25");

    let error = client.report_probe("site-a", 3, &outcome()).await.unwrap_err();
    assert!(matches!(&error, PushError::Forbidden(m) if m == "not your device"), "{error}");
    assert_eq!(calls.all()[1].path, "/api/agent/relay/3?key=site-a");
    assert!(calls.all()[1].body.contains("timeout"));
}
