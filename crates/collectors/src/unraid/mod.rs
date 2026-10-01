//! Unraid 7.x, par son API GraphQL (`/graphql`, clé d'API en `x-api-key`).
//!
//! L'API est intégrée à Unraid depuis 7.2 ; avant, elle vient avec le greffon
//! Unraid Connect. Une clé au rôle `VIEWER` lit tout et ne peut rien changer.
//!
//! Une requête obligatoire (version, état de la grappe, disques de parité, de
//! données et de cache), puis quatre facultatives, chacune seule pour qu'un
//! champ absent d'une version plus ancienne de l'API, ou des VM désactivées,
//! n'emportent pas le reste : contrôle de parité, conteneurs Docker, VM,
//! notifications non lues.
//!
//! Les pannes que ce module existe pour voir :
//!
//! * **un disque désactivé** (`DISK_DSBL`) : la grappe tourne en mode dégradé,
//!   la parité reconstitue ses données à la volée, un second disque perdu et
//!   les données des deux sont perdues ;
//! * **des erreurs de lecture** sur un disque, avant qu'il ne soit désactivé ;
//! * **un contrôle de parité** qui trouve des erreurs, ou qui n'a plus tourné
//!   depuis des semaines ;
//! * **un cache plein** : les écritures vont sur la grappe, lente, ou échouent ;
//! * **un conteneur démarré automatiquement** qui s'est arrêté.
//!
//! Tailles en kibioctets dans l'API (`BigInt` sérialisé en texte ou en
//! nombre, selon la version) : converties en octets.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `http` | Protocole de l'interface web. |
//! | `port` | `80` | Port de l'interface web. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use serde_json::{Value, json};

use crate::rest::{MAX_NAMED, RestClient, age_seconds, flag, gauge, now_ms, number, text};

pub const DEFAULT_PORT: u16 = 80;
const PATH: &str = "/graphql";

/// Version, grappe et disques : sans elle, rien à dire.
const CORE_QUERY: &str = "query DumbMonitArray { \
  info { versions { core { unraid api } } } \
  array { state \
    capacity { kilobytes { free used total } } \
    parities { name device status temp numErrors size type isSpinning } \
    disks { name device status temp numErrors size fsSize fsFree fsUsed type isSpinning } \
    caches { name device status temp numErrors size fsSize fsFree fsUsed type isSpinning } } }";

const PARITY_QUERY: &str = "query DumbMonitParity { array { parityCheckStatus { \
  status progress errors date duration running paused correcting } } }";
const DOCKER_QUERY: &str =
    "query DumbMonitDocker { docker { containers { names state status autoStart } } }";
const VMS_QUERY: &str = "query DumbMonitVms { vms { domains { name state } } }";
const NOTIFICATIONS_QUERY: &str =
    "query DumbMonitNotifications { notifications { overview { unread { info warning alert } } } }";

#[derive(Default)]
pub struct UnraidCollector;

impl UnraidCollector {
    pub fn new() -> Self {
        Self
    }
}

fn client(target: &Target) -> Result<RestClient, ProbeError> {
    let key = match &target.credential {
        Credential::ApiToken { token } if !token.trim().is_empty() => token.trim().to_string(),
        other => {
            return Err(ProbeError::Config(format!(
                "Unraid expects an API key (x-api-key), configured: {other}"
            )));
        }
    };
    Ok(RestClient::for_target(target, "http", DEFAULT_PORT, "Unraid")?
        .with_header("x-api-key", key))
}

/// Une requête GraphQL : `data`, ou l'erreur que l'API a rendue.
async fn query(client: &RestClient, text: &str) -> Result<Value, ProbeError> {
    let (status, body) = client.post_json(PATH, &json!({ "query": text })).await?;
    let reply: Option<Value> = serde_json::from_str(&body).ok();
    let first_error = reply
        .as_ref()
        .and_then(|r| r.get("errors"))
        .and_then(Value::as_array)
        .and_then(|e| e.first())
        .cloned()
        .unwrap_or(Value::Null);
    let code = first_error.pointer("/extensions/code").and_then(Value::as_str).unwrap_or_default();
    if matches!(code, "UNAUTHENTICATED") || status.as_u16() == 401 {
        return Err(ProbeError::Auth(
            "Unraid refused the API key (401). Check the key, or create one with the Viewer role \
             under Settings > Management Access > API Keys."
                .to_string(),
        ));
    }
    if matches!(code, "FORBIDDEN") || status.as_u16() == 403 {
        return Err(ProbeError::Auth(
            "Unraid accepted the API key but refused to answer (403): give the key the Viewer \
             role."
                .to_string(),
        ));
    }
    if !status.is_success() {
        return Err(client.status_error(status, &body, PATH));
    }
    let Some(reply) = reply else {
        return Err(ProbeError::Protocol(format!(
            "{} did not answer in JSON: is this the Unraid web interface?",
            client.url(PATH)
        )));
    };
    match reply.get("data") {
        Some(data) if !data.is_null() => Ok(data.clone()),
        _ => {
            let message = first_error.get("message").and_then(Value::as_str).unwrap_or("no data");
            Err(ProbeError::Protocol(format!("Unraid GraphQL API: {message}")))
        }
    }
}

#[async_trait]
impl Collector for UnraidCollector {
    fn kind(&self) -> &'static str {
        "unraid"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let client = client(target)?;
        let core = query(&client, CORE_QUERY).await?;
        let (parity, docker, vms, notes) = tokio::join!(
            query(&client, PARITY_QUERY),
            query(&client, DOCKER_QUERY),
            query(&client, VMS_QUERY),
            query(&client, NOTIFICATIONS_QUERY),
        );
        let extras = Extras {
            parity: optional(parity, target, "parityCheckStatus"),
            docker: optional(docker, target, "docker"),
            vms: optional(vms, target, "vms"),
            notifications: optional(notes, target, "notifications"),
        };
        Ok(samples(&core, &extras, now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        query(&client(target)?, CORE_QUERY).await?;
        Ok(Some("unraid".to_string()))
    }
}

/// Une requête facultative : son échec est journalisé, pas propagé.
fn optional(outcome: Result<Value, ProbeError>, target: &Target, part: &str) -> Option<Value> {
    match outcome {
        Ok(data) => Some(data),
        Err(error) => {
            tracing::debug!(target_id = target.id, part, %error, "Unraid : partie facultative indisponible");
            None
        }
    }
}

/// Les réponses des requêtes facultatives, `None` pour celles qui ont échoué.
#[derive(Default)]
pub struct Extras {
    pub parity: Option<Value>,
    pub docker: Option<Value>,
    pub vms: Option<Value>,
    pub notifications: Option<Value>,
}

fn kib(value: Option<&Value>) -> Option<f64> {
    number(value).map(|v| v * 1024.0)
}

pub fn samples(core: &Value, extras: &Extras, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut out = Vec::new();

    if let Some(version) = core.pointer("/info/versions/core/unraid").and_then(Value::as_str) {
        out.push(g("unraid_version_info", 1.0).with_label("version", version));
    }

    let array = core.get("array").cloned().unwrap_or(Value::Null);
    let state = text(&array, "state").unwrap_or("UNKNOWN");
    out.push(flag("unraid_array_started", state == "STARTED", ts_ms));
    out.push(g("unraid_array_state_info", 1.0).with_label("state", state.to_ascii_lowercase()));
    if let Some(capacity) = array.pointer("/capacity/kilobytes") {
        let total = kib(capacity.get("total"));
        let used = kib(capacity.get("used"));
        if let Some(total) = total {
            out.push(g("unraid_array_size_bytes", total));
        }
        if let Some(used) = used {
            out.push(g("unraid_array_used_bytes", used));
        }
        if let Some(free) = kib(capacity.get("free")) {
            out.push(g("unraid_array_free_bytes", free));
        }
        if let (Some(total), Some(used)) = (total, used)
            && total > 0.0
        {
            out.push(g("unraid_array_used_percent", 100.0 * used / total));
        }
    }

    let (mut disks, mut disabled, mut with_errors) = (0, 0, 0);
    for (key, role) in [("parities", "parity"), ("disks", "data"), ("caches", "cache")] {
        for disk in array.get(key).and_then(Value::as_array).into_iter().flatten().take(MAX_NAMED) {
            let status = text(disk, "status").unwrap_or("UNKNOWN");
            // Emplacement vide : rien à surveiller.
            if status == "DISK_NP" {
                continue;
            }
            let Some(name) = text(disk, "name") else { continue };
            disks += 1;
            let labelled =
                |sample: Sample| sample.with_label("disk", name).with_label("role", role);
            let ok = status == "DISK_OK";
            if !ok {
                disabled += 1;
            }
            out.push(labelled(flag("unraid_disk_ok", ok, ts_ms)));
            out.push(labelled(
                g("unraid_disk_status_info", 1.0).with_label("status", status.to_ascii_lowercase()),
            ));
            if let Some(temp) = number(disk.get("temp")).filter(|t| *t > 0.0) {
                out.push(labelled(g("unraid_disk_temperature_celsius", temp)));
            }
            let errors = number(disk.get("numErrors")).unwrap_or(0.0);
            if errors > 0.0 {
                with_errors += 1;
            }
            out.push(labelled(g("unraid_disk_errors", errors)));
            if let Some(spinning) = disk.get("isSpinning").and_then(Value::as_bool) {
                out.push(labelled(flag("unraid_disk_spinning", spinning, ts_ms)));
            }
            if let Some(size) = kib(disk.get("size")).filter(|s| *s > 0.0) {
                out.push(labelled(g("unraid_disk_size_bytes", size)));
            }
            let fs_size = kib(disk.get("fsSize")).filter(|s| *s > 0.0);
            let fs_used = kib(disk.get("fsUsed"));
            if let (Some(size), Some(used)) = (fs_size, fs_used) {
                out.push(labelled(g("unraid_disk_used_percent", 100.0 * used / size)));
                out.push(labelled(g("unraid_disk_fs_size_bytes", size)));
            }
        }
    }
    out.push(g("unraid_disks", f64::from(disks)));
    out.push(g("unraid_disks_problem", f64::from(disabled)));
    out.push(g("unraid_disks_with_errors", f64::from(with_errors)));

    if let Some(check) = extras.parity.as_ref().and_then(|d| d.pointer("/array/parityCheckStatus"))
    {
        parity(check, ts_ms, &mut out);
    }
    if let Some(list) = extras
        .docker
        .as_ref()
        .and_then(|d| d.pointer("/docker/containers"))
        .and_then(Value::as_array)
    {
        containers(list, ts_ms, &mut out);
    }
    if let Some(list) =
        extras.vms.as_ref().and_then(|d| d.pointer("/vms/domains")).and_then(Value::as_array)
    {
        let running = list.iter().filter(|vm| text(vm, "state") == Some("RUNNING")).count();
        out.push(g("unraid_vms", list.len() as f64));
        out.push(g("unraid_vms_running", running as f64));
        for vm in list.iter().take(MAX_NAMED) {
            let (Some(name), state) = (text(vm, "name"), text(vm, "state").unwrap_or("NOSTATE"))
            else {
                continue;
            };
            out.push(
                g("unraid_vm_state_info", 1.0)
                    .with_label("vm", name)
                    .with_label("state", state.to_ascii_lowercase()),
            );
            out.push(flag("unraid_vm_crashed", state == "CRASHED", ts_ms).with_label("vm", name));
        }
    }
    if let Some(unread) =
        extras.notifications.as_ref().and_then(|d| d.pointer("/notifications/overview/unread"))
    {
        for importance in ["alert", "warning", "info"] {
            if let Some(count) = number(unread.get(importance)) {
                out.push(
                    g("unraid_notifications_unread", count).with_label("importance", importance),
                );
            }
        }
    }
    out
}

fn parity(check: &Value, ts_ms: i64, out: &mut Vec<Sample>) {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let status = text(check, "status").unwrap_or("NEVER_RUN");
    out.push(
        g("unraid_parity_check_status_info", 1.0).with_label("status", status.to_ascii_lowercase()),
    );
    let running = status == "RUNNING" || status == "PAUSED";
    out.push(flag("unraid_parity_check_running", running, ts_ms));
    if running && let Some(progress) = number(check.get("progress")) {
        out.push(g("unraid_parity_check_progress_percent", progress));
    }
    if status == "NEVER_RUN" {
        return;
    }
    let errors = number(check.get("errors")).unwrap_or(0.0);
    out.push(g("unraid_parity_check_errors", errors));
    // Seul un contrôle terminé dit quelque chose de la parité : annulé, il ne
    // prouve rien dans un sens ni dans l'autre.
    match status {
        "COMPLETED" => out.push(flag("unraid_parity_check_ok", errors == 0.0, ts_ms)),
        "FAILED" => out.push(flag("unraid_parity_check_ok", false, ts_ms)),
        _ => {}
    }
    if let Some(age) = check.get("date").and_then(Value::as_str).and_then(|d| age_seconds(d, ts_ms))
    {
        out.push(g("unraid_parity_check_age_seconds", age));
    }
    if let Some(duration) = number(check.get("duration")).filter(|d| *d > 0.0) {
        out.push(g("unraid_parity_check_duration_seconds", duration));
    }
}

fn containers(list: &[Value], ts_ms: i64, out: &mut Vec<Sample>) {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut running = 0;
    let mut stopped_autostart = 0;
    for container in list {
        let is_running = text(container, "state") == Some("RUNNING");
        let autostart = container.get("autoStart").and_then(Value::as_bool).unwrap_or(false);
        running += usize::from(is_running);
        stopped_autostart += usize::from(autostart && !is_running);
    }
    out.push(g("unraid_containers", list.len() as f64));
    out.push(g("unraid_containers_running", running as f64));
    out.push(g("unraid_containers_autostart_stopped", stopped_autostart as f64));
    for container in list.iter().take(MAX_NAMED) {
        let name = container
            .get("names")
            .and_then(Value::as_array)
            .and_then(|names| names.first())
            .and_then(Value::as_str)
            .map(|n| n.trim_start_matches('/'))
            .filter(|n| !n.is_empty());
        let Some(name) = name else { continue };
        let is_running = text(container, "state") == Some("RUNNING");
        let autostart = container.get("autoStart").and_then(Value::as_bool).unwrap_or(false);
        out.push(flag("unraid_container_running", is_running, ts_ms).with_label("container", name));
        out.push(
            flag("unraid_container_autostart_stopped", autostart && !is_running, ts_ms)
                .with_label("container", name),
        );
    }
}

#[cfg(test)]
mod tests {
    use axum::Json;
    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::post;

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    /// Réponse de la requête principale, construite d'après le schéma publié
    /// de l'API (`generated-schema.graphql`, Unraid 7.2) : un disque de
    /// parité, un disque sain, un disque désactivé avec des erreurs, un
    /// emplacement vide, un cache presque plein.
    const CORE: &str = include_str!("testdata/unraid_7.2.json");
    /// Réponses des requêtes facultatives, réunies : parité, Docker, VM,
    /// notifications.
    const EXTRAS: &str = include_str!("testdata/unraid_7.2_extras.json");

    fn data(raw: &str) -> Value {
        serde_json::from_str::<Value>(raw).unwrap()["data"].clone()
    }

    fn extras() -> Extras {
        let all = data(EXTRAS);
        Extras {
            parity: Some(all.clone()),
            docker: Some(all.clone()),
            vms: Some(all.clone()),
            notifications: Some(all),
        }
    }

    const NOW: i64 = 1_790_000_000_000; // 2026-09-21

    #[test]
    fn grappe_disques_et_cache() {
        let s = samples(&data(CORE), &Extras::default(), NOW);
        assert_eq!(find(&s, "unraid_version_info", &[]).unwrap().labels["version"], "7.2.1");
        assert_eq!(value(&s, "unraid_array_started", &[]), 1.0);
        assert_eq!(value(&s, "unraid_array_size_bytes", &[]), 13_874_691_340.0 * 1024.0);
        assert!((value(&s, "unraid_array_used_percent", &[]) - 29.72).abs() < 0.01);
        assert_eq!(value(&s, "unraid_disk_ok", &[("disk", "parity"), ("role", "parity")]), 1.0);
        assert_eq!(value(&s, "unraid_disk_ok", &[("disk", "disk2")]), 0.0);
        assert_eq!(value(&s, "unraid_disk_errors", &[("disk", "disk2")]), 1_287.0);
        assert!(find(&s, "unraid_disk_temperature_celsius", &[("disk", "disk2")]).is_none());
        assert_eq!(value(&s, "unraid_disk_temperature_celsius", &[("disk", "cache")]), 41.0);
        assert!(find(&s, "unraid_disk_ok", &[("disk", "disk3")]).is_none(), "empty slot");
        let cache = value(&s, "unraid_disk_used_percent", &[("disk", "cache"), ("role", "cache")]);
        assert!((cache - 94.0).abs() < 0.1, "{cache}");
        assert_eq!(value(&s, "unraid_disks", &[]), 4.0);
        assert_eq!(value(&s, "unraid_disks_problem", &[]), 1.0);
        assert_eq!(value(&s, "unraid_disks_with_errors", &[]), 1.0);
        assert!(find(&s, "unraid_containers", &[]).is_none(), "no optional part, no series");
    }

    #[test]
    fn parite_conteneurs_vm_et_notifications() {
        let s = samples(&data(CORE), &extras(), NOW);
        assert_eq!(value(&s, "unraid_parity_check_ok", &[]), 1.0);
        assert_eq!(value(&s, "unraid_parity_check_running", &[]), 0.0);
        let age = value(&s, "unraid_parity_check_age_seconds", &[]);
        assert!((age - 1_767_680.0).abs() < 1.0, "{age}");
        assert_eq!(value(&s, "unraid_containers", &[]), 3.0);
        assert_eq!(value(&s, "unraid_containers_autostart_stopped", &[]), 1.0);
        assert_eq!(
            value(&s, "unraid_container_autostart_stopped", &[("container", "photo-sync")]),
            1.0
        );
        assert_eq!(
            value(&s, "unraid_container_autostart_stopped", &[("container", "test-build")]),
            0.0,
            "a container that does not start on its own may stay stopped"
        );
        assert_eq!(value(&s, "unraid_vms_running", &[]), 1.0);
        assert_eq!(value(&s, "unraid_notifications_unread", &[("importance", "warning")]), 1.0);
    }

    #[test]
    fn un_controle_de_parite_en_cours_ou_jamais_lance() {
        let mut out = Vec::new();
        parity(&json!({"status": "RUNNING", "progress": 42, "errors": 0}), NOW, &mut out);
        assert_eq!(value(&out, "unraid_parity_check_progress_percent", &[]), 42.0);
        assert!(find(&out, "unraid_parity_check_ok", &[]).is_none());
        let mut out = Vec::new();
        parity(&json!({"status": "NEVER_RUN"}), NOW, &mut out);
        assert!(find(&out, "unraid_parity_check_age_seconds", &[]).is_none());
        let mut out = Vec::new();
        parity(&json!({"status": "COMPLETED", "errors": "12", "date": null}), NOW, &mut out);
        assert_eq!(value(&out, "unraid_parity_check_ok", &[]), 0.0);
    }

    #[tokio::test]
    async fn interrogation_complete() {
        let app = Router::new().route(
            PATH,
            post(|headers: HeaderMap, Json(body): Json<Value>| async move {
                if headers.get("x-api-key").and_then(|v| v.to_str().ok()) != Some("viewer-key") {
                    return (
                        StatusCode::UNAUTHORIZED,
                        r#"{"errors":[{"message":"Unauthorized","extensions":{"code":"UNAUTHENTICATED"}}]}"#.to_string(),
                    );
                }
                let query = body["query"].as_str().unwrap_or_default();
                let reply = if query.contains("DumbMonitArray") {
                    CORE.to_string()
                } else if query.contains("DumbMonitVms") {
                    // VM désactivées : une erreur GraphQL, sans données.
                    r#"{"errors":[{"message":"VMs are not available"}],"data":null}"#.to_string()
                } else {
                    EXTRAS.to_string()
                };
                (StatusCode::OK, reply)
            }),
        );
        let address = serve(app).await;
        let mut target = cible("unraid", &format!("http://{address}"), &[]);
        target.credential = Credential::ApiToken { token: "viewer-key".into() };
        let s = UnraidCollector::new().probe(&target).await.unwrap();
        assert_eq!(value(&s, "unraid_disk_ok", &[("disk", "disk2")]), 0.0);
        assert_eq!(value(&s, "unraid_parity_check_ok", &[]), 1.0);
        assert!(find(&s, "unraid_vms", &[]).is_none(), "VMs disabled: skipped, not an error");

        target.credential = Credential::ApiToken { token: "wrong".into() };
        let error = UnraidCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        target.credential = Credential::None;
        let error = UnraidCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }
}
