//! Les actions de bout en bout : un faux démon Docker sur socket Unix et un faux
//! serveur DumbMonit sur TCP. Ce qui est vérifié est l'ordre des appels au démon
//! et ce que l'agent rapporte au serveur, y compris le retour arrière.

use super::*;
use crate::testsupport::{Calls, Req, Resp, fake_docker, fake_http};

const OLD: &str = "0ld0ld0ld0ld0ld0ld0ld0ld0ld0ld0ld0ld0ld0ld0ld0ld0ld0ld0ld0ld0ld0ld0ld0ld0ld0ld0";
const NEW: &str = "new1new1new1new1new1new1new1new1new1new1new1new1new1new1new1new1new1new1new1new";

fn container(id: &str, name: &str, health: Option<&str>) -> Value {
    let mut state = json!({ "Running": true, "Status": "running" });
    if let Some(health) = health {
        state["Health"] = json!({ "Status": health });
    }
    json!({
        "Id": id, "Name": format!("/{name}"), "Image": "sha256:oldimg", "State": state,
        "Config": { "Image": "nginx:1.27", "Env": ["A=1"] },
        "HostConfig": { "Binds": [] },
        "Mounts": [],
        "NetworkSettings": { "Networks": {} }
    })
}

type Rule = (String, u16, String);

fn rule(prefix: &str, status: u16, body: impl ToString) -> Rule {
    (prefix.to_string(), status, body.to_string())
}

/// Scénario : la première règle dont le préfixe `METHODE chemin` convient.
fn script(rules: Vec<Rule>) -> impl Fn(&Req) -> Resp + Send + Sync + 'static {
    move |req| {
        let line = format!("{} {}", req.method, req.path);
        rules
            .iter()
            .find(|(prefix, _, _)| line.starts_with(prefix.as_str()))
            .map(|(_, status, body)| (*status, body.clone()))
            .unwrap_or((404, json!({ "message": format!("no route for {line}") }).to_string()))
    }
}

/// Règles communes à toutes les mises à jour réussies.
fn update_rules() -> Vec<Rule> {
    vec![
        rule("GET /containers/web.previous/json", 404, r#"{"message":"No such container"}"#),
        rule("GET /containers/web/json", 200, container(OLD, "web", Some("healthy"))),
        rule(
            "POST /images/create?fromImage=nginx%3A1.27",
            200,
            "{\"status\":\"Pulling\"}\n{\"status\":\"Downloaded newer image\"}",
        ),
        rule("GET /images/nginx:1.27/json", 200, r#"{"Id":"sha256:newimg"}"#),
        rule("GET /images/sha256:oldimg/json", 200, r#"{"Id":"sha256:oldimg","Config":{}}"#),
        rule(&format!("POST /containers/{OLD}/rename?name=web.previous"), 204, ""),
        rule("POST /containers/create?name=web", 201, json!({ "Id": NEW })),
        rule(&format!("POST /containers/{OLD}/stop"), 204, ""),
        rule(&format!("POST /containers/{NEW}/start"), 204, ""),
        rule(&format!("GET /containers/{NEW}/json"), 200, container(NEW, "web", Some("healthy"))),
        rule(&format!("DELETE /containers/{OLD}"), 204, ""),
        rule("DELETE /images/sha256:oldimg", 200, "[]"),
    ]
}

fn lines(calls: &Calls) -> Vec<String> {
    calls.lines()
}

fn position(lines: &[String], needle: &str) -> usize {
    lines
        .iter()
        .position(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("appel absent : {needle}\n{lines:#?}"))
}

#[tokio::test]
async fn a_restart_waits_for_the_container_to_be_healthy() {
    let dir = tempfile::tempdir().unwrap();
    let (socket, calls) = fake_docker(
        &dir,
        script(vec![
            rule("GET /containers/web/json", 200, container(OLD, "web", Some("healthy"))),
            rule("POST /containers/web/restart?t=10", 204, ""),
        ]),
    );
    let mut log = Log::default();
    restart(&DockerClient::new(&socket), "web", &mut log).await.expect("redémarrage");
    let text = log.excerpt();
    assert!(text.contains("restarting web (nginx:1.27)") && text.ends_with("healthy"), "{text}");
    assert_eq!(calls.count("POST", "/containers/web/restart"), 1);
}

#[tokio::test]
async fn a_restart_fails_when_the_container_does_not_come_back() {
    let dir = tempfile::tempdir().unwrap();
    let mut stopped = container(OLD, "web", None);
    stopped["State"] = json!({ "Running": false, "Status": "exited", "ExitCode": 137 });
    let (socket, _) = fake_docker(
        &dir,
        script(vec![
            rule("GET /containers/web/json", 200, stopped),
            rule("POST /containers/web/restart", 204, ""),
        ]),
    );
    let error = restart(&DockerClient::new(&socket), "web", &mut Log::default())
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("not running") && error.contains("137"), "{error}");

    // Le démon qui refuse le redémarrage est cité tel quel.
    let dir = tempfile::tempdir().unwrap();
    let (socket, _) = fake_docker(
        &dir,
        script(vec![
            rule("GET /containers/web/json", 200, container(OLD, "web", Some("healthy"))),
            rule("POST /containers/web/restart", 500, r#"{"message":"cgroup busy"}"#),
        ]),
    );
    let error = restart(&DockerClient::new(&socket), "web", &mut Log::default())
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("refused the restart (500): cgroup busy"), "{error}");
}

#[tokio::test]
async fn an_unhealthy_container_after_restart_is_a_failure() {
    let dir = tempfile::tempdir().unwrap();
    let (socket, _) = fake_docker(
        &dir,
        script(vec![
            rule("GET /containers/web/json", 200, container(OLD, "web", Some("unhealthy"))),
            rule("POST /containers/web/restart", 204, ""),
        ]),
    );
    let error = restart(&DockerClient::new(&socket), "web", &mut Log::default())
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("unhealthy"), "{error}");
}

#[tokio::test]
async fn inspection_refuses_unknown_and_monitoring_containers() {
    let dir = tempfile::tempdir().unwrap();
    let (socket, _) = fake_docker(
        &dir,
        script(vec![
            rule("GET /containers/ghost/json", 404, r#"{"message":"No such container"}"#),
            // Désigné par son identifiant, mais c'est bien la supervision.
            rule("GET /containers/9f4c1b2e3d5a/json", 200, container(OLD, "dumbmonit-1", None)),
        ]),
    );
    let docker = DockerClient::new(&socket);
    let error = inspect(&docker, "ghost").await.unwrap_err().to_string();
    assert!(error.contains("no container named ghost"), "{error}");
    let error = inspect(&docker, "9f4c1b2e3d5a").await.unwrap_err().to_string();
    assert!(error.contains("monitoring container dumbmonit-1"), "{error}");
}

#[tokio::test]
async fn an_update_swaps_the_container_then_removes_the_old_one_and_its_image() {
    let dir = tempfile::tempdir().unwrap();
    let (socket, calls) = fake_docker(&dir, script(update_rules()));
    let mut log = Log::default();
    update(&DockerClient::new(&socket), "web", true, &mut log).await.expect("mise à jour");

    let calls = lines(&calls);
    // L'ancien conteneur est renommé avant la création, arrêté avant le
    // démarrage du nouveau, et supprimé seulement une fois celui-ci sain.
    let rename = position(&calls, "rename?name=web.previous");
    let create = position(&calls, "POST /containers/create?name=web");
    let stop = position(&calls, &format!("POST /containers/{OLD}/stop"));
    let start = position(&calls, &format!("POST /containers/{NEW}/start"));
    let remove = position(&calls, &format!("DELETE /containers/{OLD}"));
    let prune = position(&calls, "DELETE /images/sha256:oldimg");
    assert!(rename < create && create < stop && stop < start && start < remove && remove < prune);
    let text = log.excerpt();
    assert!(text.contains("updated to nginx:1.27"), "{text}");
    assert!(text.contains("removed the old image"), "{text}");
}

#[tokio::test]
async fn an_update_keeps_the_old_image_when_another_container_uses_it() {
    let dir = tempfile::tempdir().unwrap();
    let mut rules = vec![rule("DELETE /images/sha256:oldimg", 409, r#"{"message":"in use"}"#)];
    rules.extend(update_rules());
    let (socket, _) = fake_docker(&dir, script(rules));
    let mut log = Log::default();
    update(&DockerClient::new(&socket), "web", true, &mut log).await.expect("mise à jour");
    assert!(log.excerpt().contains("still in use"), "{}", log.excerpt());
}

#[tokio::test]
async fn an_update_without_prune_leaves_the_image_alone() {
    let dir = tempfile::tempdir().unwrap();
    let (socket, calls) = fake_docker(&dir, script(update_rules()));
    update(&DockerClient::new(&socket), "web", false, &mut Log::default()).await.unwrap();
    assert_eq!(calls.count("DELETE", "/images/"), 0);
}

#[tokio::test]
async fn an_update_does_nothing_when_the_image_is_already_the_latest() {
    let dir = tempfile::tempdir().unwrap();
    let mut rules = vec![rule("GET /images/nginx:1.27/json", 200, r#"{"Id":"sha256:oldimg"}"#)];
    rules.extend(update_rules());
    let (socket, calls) = fake_docker(&dir, script(rules));
    let mut log = Log::default();
    update(&DockerClient::new(&socket), "web", true, &mut log).await.unwrap();
    assert!(log.excerpt().contains("nothing to do"));
    assert_eq!(calls.count("POST", "/containers/create"), 0);
}

#[tokio::test]
async fn a_failed_start_rolls_the_old_container_back_into_service() {
    let dir = tempfile::tempdir().unwrap();
    let mut rules = vec![rule(
        &format!("POST /containers/{NEW}/start"),
        500,
        r#"{"message":"port already allocated"}"#,
    )];
    rules.extend(update_rules());
    rules.push(rule(&format!("POST /containers/{OLD}/rename?name=web"), 204, ""));
    rules.push(rule(&format!("POST /containers/{OLD}/start"), 204, ""));
    // Le premier `rename` (vers .previous) est déjà dans `update_rules`, qui
    // passe devant : on le laisse répondre pour l'aller comme pour le retour.
    let (socket, calls) = fake_docker(&dir, script(rules));
    let mut log = Log::default();
    let error = format!(
        "{:#}",
        update(&DockerClient::new(&socket), "web", true, &mut log).await.unwrap_err()
    );
    assert!(error.contains("port already allocated"), "{error}");

    let text = log.excerpt();
    assert!(text.contains("rolling back"), "{text}");
    assert!(text.contains("restored the name web"), "{text}");
    assert!(text.contains("previous container back in service"), "{text}");
    let calls = lines(&calls);
    // Le remplaçant raté est supprimé, l'ancien reprend son nom puis repart.
    assert!(calls.iter().any(|l| l == &format!("DELETE /containers/{NEW}?force=1&v=0")));
    assert!(calls.iter().any(|l| l.contains(&format!("{OLD}/rename?name=web"))));
    assert!(calls.iter().any(|l| l == &format!("POST /containers/{OLD}/start")));
    assert_eq!(
        calls.iter().filter(|l| l.starts_with("DELETE /images/")).count(),
        0,
        "rien ne doit être nettoyé après un échec"
    );
}

#[tokio::test]
async fn the_rollback_reports_each_step_that_fails() {
    let dir = tempfile::tempdir().unwrap();
    let (socket, _) = fake_docker(
        &dir,
        script(vec![
            rule("DELETE /containers/new", 500, r#"{"message":"busy"}"#),
            rule("POST /containers/old/rename", 409, r#"{"message":"name taken"}"#),
            rule("POST /containers/old/start", 500, r#"{"message":"no way"}"#),
        ]),
    );
    let mut log = Log::default();
    rollback(&DockerClient::new(&socket), "web", "old", true, Some("new".into()), &mut log).await;
    let text = log.excerpt();
    assert!(text.contains("cannot remove the new container: busy"), "{text}");
    assert!(text.contains("cannot restore the name web: name taken"), "{text}");
    assert!(text.contains("cannot restart the previous container: no way"), "{text}");

    // Démon injoignable : chaque étape est tentée et consignée.
    let mut log = Log::default();
    let absent = DockerClient::new(std::path::Path::new("/inexistant/docker.sock"));
    rollback(&absent, "web", "old", true, Some("new".into()), &mut log).await;
    assert!(log.excerpt().contains("cannot restore the name web"), "{}", log.excerpt());
}

#[tokio::test]
async fn an_update_refuses_a_container_built_from_an_image_id() {
    let dir = tempfile::tempdir().unwrap();
    let mut by_id = container(OLD, "web", Some("healthy"));
    by_id["Config"]["Image"] = json!("sha256:0123456789abcdef");
    let (socket, _) = fake_docker(&dir, script(vec![rule("GET /containers/web/json", 200, by_id)]));
    let error = update(&DockerClient::new(&socket), "web", true, &mut Log::default())
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("image id"), "{error}");
}

#[tokio::test]
async fn a_pull_error_hidden_in_the_stream_fails_the_update() {
    let dir = tempfile::tempdir().unwrap();
    let mut rules = vec![rule(
        "POST /images/create",
        200,
        r#"{"errorDetail":{"message":"manifest unknown"},"error":"manifest unknown"}"#,
    )];
    rules.extend(update_rules());
    let (socket, _) = fake_docker(&dir, script(rules));
    let error = update(&DockerClient::new(&socket), "web", true, &mut Log::default())
        .await
        .unwrap_err()
        .to_string();
    assert!(error.contains("manifest unknown"), "{error}");
}

#[tokio::test]
async fn a_stale_previous_container_is_removed_only_when_it_is_ours() {
    let current = container(OLD, "web", Some("healthy"));

    // Reste d'une mise à jour interrompue : même image, mêmes étiquettes.
    let dir = tempfile::tempdir().unwrap();
    let (socket, calls) = fake_docker(
        &dir,
        script(vec![
            rule(
                "GET /containers/web.previous/json",
                200,
                container("stale", "web.previous", None),
            ),
            rule("DELETE /containers/stale", 204, ""),
        ]),
    );
    let mut log = Log::default();
    remove_stale_previous(&DockerClient::new(&socket), "web.previous", &current, &mut log)
        .await
        .expect("nettoyage");
    assert_eq!(calls.count("DELETE", "/containers/stale"), 1);

    // Un homonyme venu d'ailleurs bloque la mise à jour au lieu d'être écrasé.
    let mut foreign = container("other", "web.previous", None);
    foreign["Config"]["Image"] = json!("postgres:16");
    let dir = tempfile::tempdir().unwrap();
    let (socket, calls) =
        fake_docker(&dir, script(vec![rule("GET /containers/web.previous/json", 200, foreign)]));
    let error = remove_stale_previous(
        &DockerClient::new(&socket),
        "web.previous",
        &current,
        &mut Log::default(),
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(error.contains("is not ours"), "{error}");
    assert_eq!(calls.count("DELETE", "/containers"), 0);
}

#[tokio::test]
async fn stopping_tolerates_an_already_stopped_container() {
    let dir = tempfile::tempdir().unwrap();
    let (socket, _) = fake_docker(
        &dir,
        script(vec![
            rule("POST /containers/a/stop", 304, ""),
            rule("POST /containers/b/stop", 500, r#"{"message":"stuck"}"#),
        ]),
    );
    let docker = DockerClient::new(&socket);
    let mut log = Log::default();
    stop_container(&docker, "a", &mut log, "previous container").await.expect("déjà arrêté");
    let error =
        stop_container(&docker, "b", &mut log, "previous container").await.unwrap_err().to_string();
    assert!(error.contains("cannot stop the previous container: stuck"), "{error}");
}

#[tokio::test]
async fn extra_networks_are_connected_after_creation() {
    let dir = tempfile::tempdir().unwrap();
    let (socket, calls) = fake_docker(
        &dir,
        script(vec![
            rule("POST /containers/create?name=web", 201, json!({ "Id": NEW })),
            rule("POST /networks/backend/connect", 200, ""),
        ]),
    );
    let spec = CreateSpec {
        body: json!({ "Image": "nginx:1.27" }),
        extra_networks: vec![("backend".into(), json!({ "Aliases": ["web"] }))],
    };
    let mut new_id = None;
    // Conteneur d'origine arrêté : le remplaçant est créé et laissé arrêté.
    replace(
        &DockerClient::new(&socket),
        "web",
        "old",
        false,
        spec,
        &mut Log::default(),
        &mut new_id,
    )
    .await
    .expect("remplacement");
    assert_eq!(new_id.as_deref(), Some(NEW));
    let bodies = calls.bodies("POST", "/networks/backend/connect");
    assert!(bodies[0].contains(NEW) && bodies[0].contains("Aliases"), "{bodies:?}");
    assert_eq!(calls.count("POST", &format!("/containers/{NEW}/start")), 0);
}

// ---------------------------------------------------------------- serveur

fn pending(id: i64, kind: &str, name: &str, created_at_ms: i64) -> Value {
    json!({ "id": id, "kind": kind, "args": { "name": name }, "created_at_ms": created_at_ms })
}

fn reported_statuses(server: &Calls, id: i64) -> Vec<String> {
    server
        .bodies("POST", &format!("/api/agent/commands/{id}"))
        .iter()
        .map(|body| serde_json::from_str::<Value>(body).unwrap()["status"].as_str().unwrap().into())
        .collect()
}

#[tokio::test]
async fn the_runner_reports_expired_commands_and_runs_the_next_one() {
    let dir = tempfile::tempdir().unwrap();
    let (socket, docker_calls) = fake_docker(
        &dir,
        script(vec![
            rule("GET /containers/web/json", 200, container(OLD, "web", Some("healthy"))),
            rule("POST /containers/web/restart", 204, ""),
        ]),
    );
    let now = chrono::Utc::now().timestamp_millis();
    let queue = json!([
        pending(1, CMD_CONTAINER_RESTART, "web", 0),
        pending(2, CMD_CONTAINER_RESTART, "web", now),
    ]);
    let (url, server) = fake_http(move |req| {
        if req.method == "GET" { (200, queue.to_string()) } else { (204, String::new()) }
    })
    .await;
    let client = PushClient::new(&url, "dmon_t", Some("dmab_s".into()), Duration::from_secs(5))
        .expect("client");
    let mut runner = CommandRunner::new(true, client, "nas".into(), &socket);

    runner.poll().await;
    runner.finish(Duration::from_secs(10)).await;

    assert_eq!(reported_statuses(&server, 1), ["failed"], "périmée avant exécution");
    assert_eq!(reported_statuses(&server, 2), ["running", "done"]);
    assert_eq!(docker_calls.count("POST", "/containers/web/restart"), 1);
    // Identité et secret de liaison accompagnent chaque appel.
    let first = &server.all()[0];
    assert!(first.path.ends_with("?key=nas"), "{}", first.path);
    assert_eq!(first.header("authorization"), Some("Bearer dmon_t"));
    assert_eq!(first.header("x-dumbmonit-agent-secret"), Some("dmab_s"));
    assert!(!runner.is_busy());
}

#[tokio::test]
async fn the_runner_stays_silent_when_actions_are_disabled_or_the_server_fails() {
    let (url, server) = fake_http(|_| (500, r#"{"error":"down"}"#.to_string())).await;
    let client = PushClient::new(&url, "dmon_t", None, Duration::from_secs(5)).unwrap();
    let socket = std::path::Path::new("/inexistant/docker.sock");

    CommandRunner::new(false, client.clone(), "nas".into(), socket).poll().await;
    assert!(server.all().is_empty(), "actions désactivées : l'agent ne demande même pas");

    let mut runner = CommandRunner::new(true, client, "nas".into(), socket);
    runner.poll().await;
    assert_eq!(server.all().len(), 1);
    assert!(!runner.is_busy());
    // Rien à attendre.
    runner.finish(Duration::from_millis(10)).await;
}

#[tokio::test]
async fn a_refused_command_is_reported_as_failed_with_the_reason() {
    let (url, server) = fake_http(|_| (204, String::new())).await;
    let client = PushClient::new(&url, "dmon_t", None, Duration::from_secs(5)).unwrap();
    let executor = Executor {
        client,
        key: "nas".into(),
        docker: DockerClient::new(std::path::Path::new("/inexistant/docker.sock")),
    };
    let command: AgentCommand =
        serde_json::from_value(pending(9, CMD_CONTAINER_RESTART, "dumbmonit-1", 0)).unwrap();
    executor.execute(command).await;

    let bodies = server.bodies("POST", "/api/agent/commands/9");
    assert_eq!(bodies.len(), 1);
    let report: Value = serde_json::from_str(&bodies[0]).unwrap();
    assert_eq!(report["status"], "failed");
    assert!(report["result"].as_str().unwrap().contains("monitoring stack"), "{report}");
}

#[tokio::test]
async fn an_unreachable_docker_daemon_fails_the_command_with_its_log() {
    let (url, server) = fake_http(|_| (204, String::new())).await;
    let client = PushClient::new(&url, "dmon_t", None, Duration::from_secs(5)).unwrap();
    let executor = Executor {
        client,
        key: "nas".into(),
        docker: DockerClient::new(std::path::Path::new("/inexistant/docker.sock")),
    };
    let command: AgentCommand =
        serde_json::from_value(pending(4, CMD_CONTAINER_UPDATE, "web", 0)).unwrap();
    executor.execute(command).await;
    assert_eq!(reported_statuses(&server, 4), ["running", "failed"]);
    let last = server.bodies("POST", "/api/agent/commands/4").pop().unwrap();
    assert!(last.contains("error:"), "{last}");
}
