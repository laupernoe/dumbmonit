//! Faux DSM 7.4 : les réponses d'un vrai DS918+ (`synology/testdata/dsm74`),
//! plus trois tâches Active Backup dont l'historique suit l'heure courante.

use std::collections::HashMap;

use axum::Router;
use axum::extract::RawQuery;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::get;
use serde_json::{Value, json};

use super::{json, now_s};

pub(super) const USERNAME: &str = "monitoring";
pub(super) const PASSWORD: &str = "demonstration";
const SID: &str = "demonstration-sid";
const TOKEN: &str = "demonstration-token";

const SYSTEM_INFO: &str =
    include_str!("../../../../collectors/src/synology/testdata/dsm74/system-info.json");
/// Numéro de série pseudonymisé de la capture, remplacé par un faux lisible.
const SERIAL: (&str, &str) = ("SERIAL-1", "2150PDN123456");
const UTILIZATION: &str =
    include_str!("../../../../collectors/src/synology/testdata/dsm74/utilization.json");
const STORAGE: &str =
    include_str!("../../../../collectors/src/synology/testdata/dsm74/storage-load-info.json");

/// `task_id → (nom, type de source, appareils)` des tâches Active Backup.
const TASKS: &[(i64, &str, i64, &[&str])] = &[
    (5, "Laptops", 2, &["laptop-alex", "laptop-sam"]),
    (6, "Home VMs", 1, &["nextcloud", "home-assistant", "immich"]),
    (7, "Family photos", 4, &["photos-share"]),
];
/// La tâche dont la dernière nuit a échoué : un portable resté éteint.
const FAILED_TASK: i64 = 5;

pub(super) fn router() -> Router {
    Router::new().route("/webapi/entry.cgi", get(entry).post(entry))
}

fn ok(data: Value) -> Response {
    json(StatusCode::OK, json!({"data": data, "success": true}).to_string())
}

fn fail(code: i64) -> Response {
    json(StatusCode::OK, json!({"error": {"code": code}, "success": false}).to_string())
}

/// Le `data` d'une réponse capturée.
fn capture(body: &str) -> Value {
    serde_json::from_str::<Value>(body).map(|doc| doc["data"].clone()).unwrap_or(Value::Null)
}

/// Paramètres de la requête, chaîne et corps de formulaire confondus, décodés.
fn params(query: Option<String>, body: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for raw in [query.unwrap_or_default(), body.to_string()] {
        if raw.is_empty() {
            continue;
        }
        if let Ok(url) = reqwest::Url::parse(&format!("http://dsm/?{raw}")) {
            out.extend(url.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())));
        }
    }
    out
}

async fn entry(RawQuery(query): RawQuery, body: String) -> Response {
    let params = params(query, &body);
    let api = params.get("api").map(String::as_str).unwrap_or_default();
    let method = params.get("method").map(String::as_str).unwrap_or_default();

    match (api, method) {
        ("SYNO.API.Info", "query") => ok(catalog()),
        ("SYNO.API.Auth", "login") => ok(json!({"sid": SID, "synotoken": TOKEN})),
        ("SYNO.API.Auth", "logout") => ok(json!({})),
        ("SYNO.Core.System", "info") => ok(capture(&SYSTEM_INFO.replace(SERIAL.0, SERIAL.1))),
        ("SYNO.Core.System.Utilization", "get") => ok(capture(UTILIZATION)),
        ("SYNO.Storage.CGI.Storage", "load_info") => ok(capture(STORAGE)),
        ("SYNO.ActiveBackup.Task", "list") => ok(json!({
            "tasks": TASKS.iter().map(|t| task(t.0)).collect::<Vec<_>>(),
            "total": TASKS.len()
        })),
        ("SYNO.ActiveBackup.Log", "list_result") => {
            let task_id: i64 = params.get("task_id").and_then(|id| id.parse().ok()).unwrap_or(0);
            let filter: Value = params
                .get("filter")
                .and_then(|raw| serde_json::from_str(raw).ok())
                .unwrap_or_else(|| json!({}));
            let limit: usize = params.get("limit").and_then(|l| l.parse().ok()).unwrap_or(50);
            let selected: Vec<Value> = results(task_id)
                .into_iter()
                .filter(|r| filter.get("status").is_none_or(|s| s == &r["status"]))
                .filter(|r| filter.get("job_action").is_none_or(|a| a == &r["job_action"]))
                .take(limit)
                .collect();
            ok(json!({"count": selected.len(), "results": selected}))
        }
        _ => fail(102),
    }
}

fn catalog() -> Value {
    let mut apis = json!({
        "SYNO.API.Auth": {"path": "entry.cgi", "minVersion": 1, "maxVersion": 7},
        "SYNO.Core.System": {"path": "entry.cgi", "minVersion": 1, "maxVersion": 3},
        "SYNO.Core.System.Utilization": {"path": "entry.cgi", "minVersion": 1, "maxVersion": 1},
        "SYNO.Storage.CGI.Storage": {"path": "entry.cgi", "minVersion": 1, "maxVersion": 1},
    });
    for api in ["SYNO.ActiveBackup.Task", "SYNO.ActiveBackup.Log"] {
        apis[api] = json!({"path": "entry.cgi", "minVersion": 1, "maxVersion": 2});
    }
    apis
}

/// La dernière exécution nocturne : 3 h du matin (UTC) la plus récente.
fn last_run() -> i64 {
    let now = now_s();
    let today = now - now.rem_euclid(86_400) + 3 * 3600;
    if today <= now { today } else { today - 86_400 }
}

fn result(task_id: i64, start: i64, status: i64) -> Value {
    let (_, name, source, devices) =
        TASKS.iter().find(|t| t.0 == task_id).copied().unwrap_or(TASKS[0]);
    json!({
        "backup_type": source, "error_count": if status == 4 { devices.len() } else { 0 },
        "job_action": 1, "result_id": 600 + task_id, "status": status,
        "task_id": task_id, "task_name": name, "time_end": start + 9 * 60, "time_start": start,
        "transfered_bytes": 2_122_801_152_i64, "warning_count": 0
    })
}

/// Historique d'une tâche, la plus récente en premier : la tâche en échec
/// n'a raté que la dernière nuit.
fn results(task_id: i64) -> Vec<Value> {
    (0..14)
        .map(|nights_ago| {
            let start = last_run() - nights_ago * 86_400;
            let status = if task_id == FAILED_TASK && nights_ago == 0 { 4 } else { 2 };
            result(task_id, start, status)
        })
        .collect()
}

fn task(task_id: i64) -> Value {
    let (_, name, source, devices) =
        TASKS.iter().find(|t| t.0 == task_id).copied().unwrap_or(TASKS[0]);
    json!({
        "task_id": task_id, "task_name": name, "source_type": source, "backup_type": source,
        "device_count": devices.len(),
        "devices": devices.iter().enumerate().map(|(i, host)| json!({
            "device_id": task_id * 10 + i as i64, "host_name": host, "agent_status": "online"
        })).collect::<Vec<_>>(),
        "last_result": results(task_id)[0],
        "next_trigger_time": last_run() + 86_400, "target_status": "online",
        "sched_content": {"is_continuous_paused": false, "repeat_type": "Daily", "run_hour": 3},
        "versions": []
    })
}
