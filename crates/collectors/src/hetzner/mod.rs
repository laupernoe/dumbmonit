//! Hetzner Cloud, par l'API publique (`https://api.hetzner.cloud/v1`).
//!
//! Un jeton d'API de projet en **lecture seule** suffit : Hetzner n'a pas de
//! droits plus fins que « lecture » et « lecture et écriture », par projet.
//! Ce que le module lit :
//!
//! * **les serveurs** (`GET /servers`, indispensable) : l'état (`running`,
//!   `off`, `migrating`…), le type, le site, et si les sauvegardes
//!   automatiques sont activées (`backup_window`) ;
//! * **l'activité des serveurs en marche** (`GET /servers/{id}/metrics`,
//!   facultatif) : processeur, disque et réseau, moyennés sur dix minutes,
//!   pour les `metrics_servers` premiers serveurs seulement — l'API limite à
//!   3 600 requêtes par heure et par projet ;
//! * **les volumes** (`GET /volumes`, facultatif) : rattachés à un serveur ou
//!   non ;
//! * **les sauvegardes** (`GET /images?type=backup`, facultatif, seulement si
//!   un serveur a les sauvegardes activées) : l'âge de la plus récente.
//!
//! Un sous-appel qui échoue n'empêche pas la sonde : la série
//! `hetzner_<partie>_readable` passe à 0 et les séries de cette partie sont
//! omises.
//!
//! C'est une API sur Internet : l'adresse de la cible est celle de l'API, pas
//! d'un appareil, et le garde-fou des adresses privées ne la concerne pas.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `metrics_servers` | `10` | Serveurs en marche dont on lit l'activité (0 à 100, 0 = aucun). |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use std::collections::BTreeMap;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use reqwest::StatusCode;
use serde_json::{Map, Value};

use crate::cloud::{figure, item};
use crate::rest::{MAX_NAMED, RestClient, age_seconds, flag, gauge, now_ms, number, text};
use crate::selfhosted::options::tag;

pub const DEFAULT_PORT: u16 = 443;
pub const DEFAULT_ADDRESS: &str = "api.hetzner.cloud";

const API: &str = "/v1";
/// Pages de 50 éléments lues au plus par liste.
const MAX_PAGES: usize = 20;
/// Les sauvegardes arrivent de la plus récente à la plus ancienne : peu de pages suffisent.
const BACKUP_PAGES: usize = 5;
const DEFAULT_METRICS_SERVERS: usize = 10;
const MAX_METRICS_SERVERS: usize = 100;
/// Fenêtre sur laquelle l'activité est moyennée, et pas entre deux points.
const METRICS_WINDOW_SECS: i64 = 600;
const METRICS_STEP_SECS: u32 = 60;
/// Âge au-delà duquel la page de l'équipement juge une sauvegarde trop vieille.
const BACKUP_MAX_AGE_SECS: f64 = 3.0 * 86_400.0;

#[derive(Default)]
pub struct HetznerCollector;

impl HetznerCollector {
    pub fn new() -> Self {
        Self
    }
}

fn metrics_servers(target: &Target) -> Result<usize, ProbeError> {
    let Some(raw) = tag(target, "metrics_servers") else { return Ok(DEFAULT_METRICS_SERVERS) };
    raw.parse::<usize>().ok().filter(|n| *n <= MAX_METRICS_SERVERS).ok_or_else(|| {
        ProbeError::Config(format!(
            "Invalid number of servers with metrics \"{raw}\": from 0 to {MAX_METRICS_SERVERS}"
        ))
    })
}

fn build_client(target: &Target) -> Result<RestClient, ProbeError> {
    let mut client = RestClient::for_target(target, "https", DEFAULT_PORT, "Hetzner Cloud")?;
    match &target.credential {
        Credential::ApiToken { token } if !token.trim().is_empty() => {
            client.set_header("Authorization", format!("Bearer {}", token.trim()));
            Ok(client)
        }
        other => Err(ProbeError::Config(format!(
            "Hetzner Cloud expects an API token, configured: {other}"
        ))),
    }
}

/// Traduit la réponse : le corps JSON si 2xx, sinon une erreur typée.
fn envelope(
    client: &RestClient,
    status: StatusCode,
    body: &str,
    path: &str,
) -> Result<Value, ProbeError> {
    if status.is_success() {
        return client.decode(body, path);
    }
    let reply: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let detail = reply.pointer("/error/message").and_then(Value::as_str).unwrap_or_default();
    let suffix = if detail.is_empty() { String::new() } else { format!(" ({detail})") };
    match status.as_u16() {
        401 => Err(ProbeError::Auth(format!(
            "Hetzner Cloud refused the API token (401): it is wrong, deleted, or belongs to \
             another project.{suffix}"
        ))),
        403 => Err(ProbeError::Auth(format!(
            "Hetzner Cloud refused {path} (403): the API token cannot read it.{suffix}"
        ))),
        404 => Err(ProbeError::Config(format!("Hetzner Cloud has no resource at {path}.{suffix}"))),
        _ => Err(client.status_error(status, body, path)),
    }
}

async fn call(client: &RestClient, path: &str) -> Result<Value, ProbeError> {
    let (status, body) = client.get(path).await?;
    envelope(client, status, &body, path)
}

/// Un appel dont la réponse reste associée au nom qui l'a demandé.
async fn call_named<'a>(
    client: &RestClient,
    name: &'a str,
    path: String,
) -> (&'a str, Result<Value, ProbeError>) {
    (name, call(client, &path).await)
}

/// Lit une liste paginée : les éléments de `key`, jusqu'à `next_page` nul.
async fn list(
    client: &RestClient,
    path: &str,
    key: &str,
    max_pages: usize,
) -> Result<Vec<Value>, ProbeError> {
    let separator = if path.contains('?') { '&' } else { '?' };
    let mut out = Vec::new();
    for page in 1..=max_pages {
        let reply = call(client, &format!("{path}{separator}per_page=50&page={page}")).await?;
        if let Some(items) = reply.get(key).and_then(Value::as_array) {
            out.extend_from_slice(items);
        }
        if reply.pointer("/meta/pagination/next_page").and_then(Value::as_u64).is_none() {
            break;
        }
    }
    Ok(out)
}

fn capital(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn readable(part: &str, ok: bool, ts_ms: i64) -> Sample {
    flag(&format!("hetzner_{part}_readable"), ok, ts_ms)
}

fn backups_enabled(server: &Value) -> bool {
    server.get("backup_window").is_some_and(|window| !window.is_null())
}

pub fn server_samples(servers: &[Value], ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let (mut running, mut not_running) = (0usize, 0usize);
    for (index, server) in servers.iter().enumerate() {
        let status = text(server, "status").unwrap_or("unknown");
        let is_running = status == "running";
        running += usize::from(is_running);
        not_running += usize::from(!is_running);
        if index >= MAX_NAMED {
            continue;
        }
        let Some(name) = text(server, "name") else { continue };
        let kind = server.pointer("/server_type/name").and_then(Value::as_str).unwrap_or_default();
        let location =
            server.pointer("/datacenter/location/name").and_then(Value::as_str).unwrap_or_default();
        let named = |sample: Sample| sample.with_label("server", name);
        out.push(named(flag("hetzner_server_running", is_running, ts_ms)));
        out.push(named(flag("hetzner_server_backups_enabled", backups_enabled(server), ts_ms)));
        out.push(
            named(gauge("hetzner_server_info", 1.0, ts_ms))
                .with_label("status", status)
                .with_label("type", kind)
                .with_label("location", location),
        );
        out.push(item("hetzner", "Servers", name, is_running, &capital(status), ts_ms));
    }
    out.push(gauge("hetzner_servers", servers.len() as f64, ts_ms));
    out.push(gauge("hetzner_servers_running", running as f64, ts_ms));
    out.push(gauge("hetzner_servers_not_running", not_running as f64, ts_ms));
    out.push(figure("hetzner", "Servers", "", servers.len() as f64, ts_ms));
    out.push(figure("hetzner", "Servers running", "", running as f64, ts_ms));
    out
}

/// Moyenne des valeurs `[horodatage, "valeur"]` d'une série de l'API.
fn average(series: &Value) -> Option<f64> {
    let values: Vec<f64> = series
        .get("values")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|point| number(point.get(1)))
        .collect();
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

/// Somme des moyennes des séries dont le nom commence et finit ainsi
/// (`disk.0.bandwidth.read`, `disk.1.bandwidth.read`…).
fn summed(series: &Map<String, Value>, prefix: &str, suffix: &str) -> Option<f64> {
    series
        .iter()
        .filter(|(name, _)| name.starts_with(prefix) && name.ends_with(suffix))
        .filter_map(|(_, value)| average(value))
        .fold(None, |total, value| Some(total.unwrap_or(0.0) + value))
}

pub fn activity_samples(server: &str, reply: &Value, ts_ms: i64) -> Vec<Sample> {
    let Some(series) = reply.pointer("/metrics/time_series").and_then(Value::as_object) else {
        return Vec::new();
    };
    let named = |name: &str, value: f64| gauge(name, value, ts_ms).with_label("server", server);
    let mut out = Vec::new();
    if let Some(cpu) = series.get("cpu").and_then(average) {
        out.push(named("hetzner_server_cpu_percent", cpu));
    }
    let wanted = [
        ("hetzner_server_disk_read_bytes_per_second", "disk.", ".bandwidth.read"),
        ("hetzner_server_disk_write_bytes_per_second", "disk.", ".bandwidth.write"),
        ("hetzner_server_network_in_bytes_per_second", "network.", ".bandwidth.in"),
        ("hetzner_server_network_out_bytes_per_second", "network.", ".bandwidth.out"),
    ];
    for (metric, prefix, suffix) in wanted {
        if let Some(value) = summed(series, prefix, suffix) {
            out.push(named(metric, value));
        }
    }
    out
}

pub fn volume_samples(volumes: &[Value], ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let mut unattached = 0usize;
    for (index, volume) in volumes.iter().enumerate() {
        let attached = volume.get("server").is_some_and(|server| !server.is_null());
        unattached += usize::from(!attached);
        if index >= MAX_NAMED {
            continue;
        }
        let Some(name) = text(volume, "name") else { continue };
        let named = |sample: Sample| sample.with_label("volume", name);
        out.push(named(flag("hetzner_volume_attached", attached, ts_ms)));
        if let Some(size) = number(volume.get("size")) {
            out.push(named(gauge("hetzner_volume_size_gigabytes", size, ts_ms)));
        }
        let detail = if attached { "Attached" } else { "Not attached" };
        out.push(item("hetzner", "Volumes", name, attached, detail, ts_ms));
    }
    out.push(gauge("hetzner_volumes", volumes.len() as f64, ts_ms));
    out.push(gauge("hetzner_volumes_unattached", unattached as f64, ts_ms));
    out.push(figure("hetzner", "Volumes", "", volumes.len() as f64, ts_ms));
    out
}

/// Âge de la sauvegarde la plus récente des serveurs qui en ont activé.
pub fn backup_samples(servers: &[Value], images: &[Value], ts_ms: i64) -> Vec<Sample> {
    // Image de sauvegarde → serveur : `bound_to`, sinon le serveur d'origine.
    let mut youngest: BTreeMap<u64, f64> = BTreeMap::new();
    for image in images {
        let owner = image
            .get("bound_to")
            .and_then(Value::as_u64)
            .or_else(|| image.pointer("/created_from/id").and_then(Value::as_u64));
        let age = text(image, "created").and_then(|created| age_seconds(created, ts_ms));
        let (Some(owner), Some(age)) = (owner, age) else { continue };
        youngest.entry(owner).and_modify(|known| *known = known.min(age)).or_insert(age);
    }
    let mut out = Vec::new();
    for server in servers.iter().filter(|server| backups_enabled(server)).take(MAX_NAMED) {
        let (Some(name), Some(id)) =
            (text(server, "name"), server.get("id").and_then(Value::as_u64))
        else {
            continue;
        };
        // Sauvegardes tout juste activées : aucune image, donc rien à juger encore.
        let Some(age) = youngest.get(&id).copied() else { continue };
        out.push(gauge("hetzner_server_backup_age_seconds", age, ts_ms).with_label("server", name));
        let (ok, detail) = if age <= BACKUP_MAX_AGE_SECS {
            (true, "Recent")
        } else {
            (false, "Older than 3 days")
        };
        out.push(item("hetzner", "Backups", name, ok, detail, ts_ms));
    }
    out
}

/// Début de la fenêtre d'activité et fin (maintenant), en RFC 3339.
fn metrics_window(ts_ms: i64) -> (String, String) {
    let iso = |at: i64| {
        chrono::DateTime::from_timestamp(at, 0)
            .unwrap_or_default()
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    };
    let secs = ts_ms / 1000;
    (iso(secs - METRICS_WINDOW_SECS), iso(secs))
}

async fn gather(
    client: &RestClient,
    servers: &[Value],
    metrics_limit: usize,
    ts_ms: i64,
) -> Vec<Sample> {
    let mut out = server_samples(servers, ts_ms);

    // Activité : serveurs en marche seulement, les premiers.
    let running: Vec<(u64, &str)> = servers
        .iter()
        .filter(|server| text(server, "status") == Some("running"))
        .filter_map(|server| Some((server.get("id")?.as_u64()?, text(server, "name")?)))
        .take(metrics_limit)
        .collect();
    if !running.is_empty() {
        let (start, end) = metrics_window(ts_ms);
        let (start, end) = (start.as_str(), end.as_str());
        let answers = futures::future::join_all(running.into_iter().map(|(id, name)| {
            let path = format!(
                "{API}/servers/{id}/metrics?type=cpu,disk,network&start={start}&end={end}\
                 &step={METRICS_STEP_SECS}"
            );
            call_named(client, name, path)
        }))
        .await;
        let mut metrics_ok = true;
        for (name, answer) in &answers {
            match answer {
                Ok(reply) => out.extend(activity_samples(name, reply, ts_ms)),
                Err(_) => metrics_ok = false,
            }
        }
        out.push(readable("metrics", metrics_ok, ts_ms));
    }

    // Volumes.
    match list(client, &format!("{API}/volumes"), "volumes", MAX_PAGES).await {
        Ok(volumes) => {
            out.extend(volume_samples(&volumes, ts_ms));
            out.push(readable("volumes", true, ts_ms));
        }
        Err(_) => out.push(readable("volumes", false, ts_ms)),
    }

    // Sauvegardes : seulement si un serveur en a activé.
    if servers.iter().any(backups_enabled) {
        let path = format!("{API}/images?type=backup&sort=created:desc");
        match list(client, &path, "images", BACKUP_PAGES).await {
            Ok(images) => {
                out.extend(backup_samples(servers, &images, ts_ms));
                out.push(readable("backups", true, ts_ms));
            }
            Err(_) => out.push(readable("backups", false, ts_ms)),
        }
    }
    out
}

#[async_trait]
impl Collector for HetznerCollector {
    fn kind(&self) -> &'static str {
        "hetzner"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let limit = metrics_servers(target)?;
        let client = build_client(target)?;
        let servers = list(&client, &format!("{API}/servers"), "servers", MAX_PAGES).await?;
        Ok(gather(&client, &servers, limit, now_ms()).await)
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let client = build_client(target)?;
        list(&client, &format!("{API}/servers"), "servers", 1).await?;
        Ok(Some("hetzner".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::get;

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    const SERVERS: &str = include_str!("testdata/servers.json");
    const VOLUMES: &str = include_str!("testdata/volumes.json");
    const BACKUPS: &str = include_str!("testdata/backups.json");
    const METRICS: &str = include_str!("testdata/metrics.json");

    const REFUSED: &str = r#"{"error":{"code":"forbidden","message":"insufficient permissions"}}"#;

    fn now() -> i64 {
        chrono::DateTime::parse_from_rfc3339("2026-09-30T09:00:00Z").unwrap().timestamp_millis()
    }

    fn items(raw: &str, key: &str) -> Vec<Value> {
        let reply: Value = serde_json::from_str(raw).unwrap();
        reply[key].as_array().unwrap().clone()
    }

    #[test]
    fn etat_des_serveurs() {
        let s = server_samples(&items(SERVERS, "servers"), now());
        assert_eq!(value(&s, "hetzner_servers", &[]), 3.0);
        assert_eq!(value(&s, "hetzner_servers_running", &[]), 2.0);
        assert_eq!(value(&s, "hetzner_servers_not_running", &[]), 1.0);
        assert_eq!(value(&s, "hetzner_server_running", &[("server", "web-1")]), 1.0);
        assert_eq!(value(&s, "hetzner_server_running", &[("server", "db-1")]), 0.0);
        assert_eq!(value(&s, "hetzner_server_backups_enabled", &[("server", "web-1")]), 1.0);
        assert_eq!(value(&s, "hetzner_server_backups_enabled", &[("server", "db-1")]), 0.0);
        let info = find(&s, "hetzner_server_info", &[("server", "web-1")]).unwrap();
        assert_eq!(info.labels["type"], "cx22");
        assert_eq!(info.labels["location"], "fsn1");
        let off = find(&s, "hetzner_item_ok", &[("item", "db-1")]).unwrap();
        assert_eq!(off.value, 0.0);
        assert_eq!(off.labels["detail"], "Off");
    }

    #[test]
    fn activite_moyennee_sur_la_fenetre() {
        let reply: Value = serde_json::from_str(METRICS).unwrap();
        let s = activity_samples("web-1", &reply, now());
        assert_eq!(value(&s, "hetzner_server_cpu_percent", &[("server", "web-1")]), 90.5);
        assert_eq!(value(&s, "hetzner_server_disk_read_bytes_per_second", &[]), 2000.0);
        assert_eq!(value(&s, "hetzner_server_disk_write_bytes_per_second", &[]), 500.0);
        assert_eq!(value(&s, "hetzner_server_network_in_bytes_per_second", &[]), 2_000_000.0);
        assert_eq!(value(&s, "hetzner_server_network_out_bytes_per_second", &[]), 200.0);
        assert!(activity_samples("x", &Value::Null, now()).is_empty());
    }

    #[test]
    fn volumes_rattaches_ou_non() {
        let s = volume_samples(&items(VOLUMES, "volumes"), now());
        assert_eq!(value(&s, "hetzner_volumes", &[]), 2.0);
        assert_eq!(value(&s, "hetzner_volumes_unattached", &[]), 1.0);
        assert_eq!(value(&s, "hetzner_volume_attached", &[("volume", "data")]), 1.0);
        assert_eq!(value(&s, "hetzner_volume_attached", &[("volume", "scratch")]), 0.0);
        assert_eq!(value(&s, "hetzner_volume_size_gigabytes", &[("volume", "data")]), 100.0);
    }

    #[test]
    fn age_de_la_sauvegarde_la_plus_recente() {
        let servers = items(SERVERS, "servers");
        let s = backup_samples(&servers, &items(BACKUPS, "images"), now());
        assert_eq!(
            value(&s, "hetzner_server_backup_age_seconds", &[("server", "web-1")]),
            3.0 * 86_400.0 + 37_800.0
        );
        assert!(
            find(&s, "hetzner_server_backup_age_seconds", &[("server", "db-1")]).is_none(),
            "backups are not enabled on db-1"
        );
        let item = find(&s, "hetzner_item_ok", &[("item", "web-1")]).unwrap();
        assert_eq!(item.value, 0.0);
        assert_eq!(item.labels["detail"], "Older than 3 days");
        // Sauvegardes activées mais aucune image : rien à juger.
        assert!(backup_samples(&servers, &[], now()).is_empty());
    }

    fn authorized(headers: &HeaderMap) -> bool {
        headers.get("authorization").and_then(|v| v.to_str().ok()) == Some("Bearer good")
    }

    /// Faux API : `refuse` rend 403 sur tout sauf la liste des serveurs.
    fn app(refuse: bool) -> Router {
        let answer = move |headers: &HeaderMap, body: &str| {
            if !authorized(headers) {
                (StatusCode::UNAUTHORIZED, REFUSED.to_string())
            } else if refuse {
                (StatusCode::FORBIDDEN, REFUSED.to_string())
            } else {
                (StatusCode::OK, body.to_string())
            }
        };
        Router::new()
            .route(
                "/v1/servers",
                get(|headers: HeaderMap| async move {
                    if authorized(&headers) {
                        (StatusCode::OK, SERVERS.to_string())
                    } else {
                        (
                            StatusCode::UNAUTHORIZED,
                            r#"{"error":{"code":"unauthorized","message":"bad token"}}"#
                                .to_string(),
                        )
                    }
                }),
            )
            .route(
                "/v1/servers/{id}/metrics",
                get(move |headers: HeaderMap| async move { answer(&headers, METRICS) }),
            )
            .route(
                "/v1/volumes",
                get(move |headers: HeaderMap| async move { answer(&headers, VOLUMES) }),
            )
            .route(
                "/v1/images",
                get(move |headers: HeaderMap| async move { answer(&headers, BACKUPS) }),
            )
    }

    fn target(address: &str, token: &str, tags: &[(&str, &str)]) -> Target {
        let mut target = cible("hetzner", &format!("http://{address}"), tags);
        target.credential = Credential::ApiToken { token: token.into() };
        target
    }

    #[tokio::test]
    async fn sonde_complete() {
        let address = serve(app(false)).await;
        let s = HetznerCollector::new().probe(&target(&address, "good", &[])).await.unwrap();
        assert_eq!(value(&s, "hetzner_servers", &[]), 3.0);
        assert_eq!(value(&s, "hetzner_server_cpu_percent", &[("server", "web-1")]), 90.5);
        assert!(find(&s, "hetzner_server_cpu_percent", &[("server", "worker-1")]).is_some());
        assert!(
            find(&s, "hetzner_server_cpu_percent", &[("server", "db-1")]).is_none(),
            "a server that is off has no activity"
        );
        assert_eq!(value(&s, "hetzner_metrics_readable", &[]), 1.0);
        assert_eq!(value(&s, "hetzner_volumes_unattached", &[]), 1.0);
        assert_eq!(value(&s, "hetzner_volumes_readable", &[]), 1.0);
        assert!(find(&s, "hetzner_server_backup_age_seconds", &[("server", "web-1")]).is_some());
        assert_eq!(value(&s, "hetzner_backups_readable", &[]), 1.0);
        assert!(
            s.iter().all(|sample| !sample.metric.starts_with("dumbmonit_")),
            "the server adds the dumbmonit_ prefix"
        );
    }

    #[tokio::test]
    async fn activite_desactivee_a_zero_serveur() {
        let address = serve(app(false)).await;
        let tags = [("metrics_servers", "0")];
        let s = HetznerCollector::new().probe(&target(&address, "good", &tags)).await.unwrap();
        assert!(find(&s, "hetzner_server_cpu_percent", &[]).is_none());
        assert!(find(&s, "hetzner_metrics_readable", &[]).is_none());
        let tags = [("metrics_servers", "many")];
        let error =
            HetznerCollector::new().probe(&target(&address, "good", &tags)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }

    #[tokio::test]
    async fn sous_appels_refuses_sans_echec_de_la_sonde() {
        let address = serve(app(true)).await;
        let s = HetznerCollector::new().probe(&target(&address, "good", &[])).await.unwrap();
        assert_eq!(value(&s, "hetzner_servers", &[]), 3.0);
        for part in ["metrics", "volumes", "backups"] {
            assert_eq!(value(&s, &format!("hetzner_{part}_readable"), &[]), 0.0, "{part}");
        }
        assert!(find(&s, "hetzner_server_cpu_percent", &[]).is_none());
        assert!(find(&s, "hetzner_volume_attached", &[]).is_none());
        assert!(find(&s, "hetzner_server_backup_age_seconds", &[]).is_none());
    }

    #[tokio::test]
    async fn jeton_refuse_donne_une_erreur_d_authentification() {
        let address = serve(app(false)).await;
        let error =
            HetznerCollector::new().probe(&target(&address, "wrong", &[])).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
        let forbidden = Router::new()
            .route("/v1/servers", get(|| async { (StatusCode::FORBIDDEN, REFUSED.to_string()) }));
        let address = serve(forbidden).await;
        let error =
            HetznerCollector::new().probe(&target(&address, "good", &[])).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
    }

    #[tokio::test]
    async fn panne_du_serveur_veut_dire_hors_service() {
        let broken = Router::new().route(
            "/v1/servers",
            get(|| async { (StatusCode::SERVICE_UNAVAILABLE, "unavailable".to_string()) }),
        );
        let address = serve(broken).await;
        let error =
            HetznerCollector::new().probe(&target(&address, "good", &[])).await.unwrap_err();
        assert!(error.means_down(), "{error}");
    }

    #[tokio::test]
    async fn identifiants_d_un_autre_type_refuses() {
        let mut other = cible("hetzner", "unreachable.invalid:1", &[]);
        other.credential =
            Credential::UsernamePassword { username: "u".into(), password: "p".into() };
        let error = HetznerCollector::new().probe(&other).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }
}
