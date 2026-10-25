//! Imports depuis UptimeRobot, Gatus et Healthchecks.io.
//!
//! Même principe que l'import Uptime Kuma (`kuma_import.rs`) : aperçu
//! (`?dry_run=true`), puis import des lignes cochées (`?rows=1,3`), avec le même
//! rapport et la même validation que pour une création à la main.
//!
//! - `POST /api/targets/import/uptimerobot` : `{"api_key"}` (lecture seule suffit)
//!   ou `{"csv"}` (export du tableau de bord) ;
//! - `POST /api/targets/import/gatus` : `{"yaml"}`, le fichier de configuration
//!   (`endpoints:` et `external-endpoints:`) ;
//! - `POST /api/targets/import/healthchecks` : `{"api_key", "base_url"?}` (API v3).
//!   Chaque check devient un moniteur en poussée ; son ancienne URL de ping est
//!   rappelée dans la note pour la rediriger (voir la documentation).
//!
//! Les clés d'API ne sont ni conservées ni journalisées. Les canaux de
//! notification, les identifiants, les en-têtes et les corps de requête ne sont
//! jamais importés.

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use reqwest::Url;
use serde::Deserialize;
use serde_json::Value;

use crate::api::kuma_import::{Converted, ImportQuery, ImportReport, host_port, run, tag_key};
use crate::api::{ApiError, ApiResult};
use crate::notify::guard;
use crate::state::AppState;

const MAX_MONITORS: usize = 2000;
const FETCH_TIMEOUT: Duration = Duration::from_secs(20);
/// Taille maximale du texte envoyé (CSV, YAML).
pub const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
const UPTIMEROBOT_API: &str = "https://api.uptimerobot.com/v2/getMonitors";
const HEALTHCHECKS_DEFAULT: &str = "https://healthchecks.io";

type Reply = ApiResult<(StatusCode, Json<ImportReport>)>;

fn blank(name: &str, source_type: &str, fallback: &str) -> Converted {
    let name = name.trim();
    Converted {
        name: if name.is_empty() { fallback.to_string() } else { name.to_string() },
        source_type: source_type.to_string(),
        kind: None,
        address: String::new(),
        folder: String::new(),
        interval_secs: 60,
        enabled: true,
        tags: BTreeMap::new(),
        user_tags: BTreeMap::new(),
        ignored: None,
        notes: Vec::new(),
    }
}

/// Même tri que l'import de `kuma.db` : l'aperçu et l'import numérotent pareil.
fn sorted(mut items: Vec<Converted>) -> Vec<Converted> {
    items.sort_by_cached_key(|c| {
        (c.kind.is_none(), c.kind, c.folder.to_lowercase(), c.name.to_lowercase())
    });
    items
}

/// Applique la période minimale du produit.
fn clamp_interval(item: &mut Converted) {
    if item.interval_secs < 10 {
        item.interval_secs = 10;
        item.notes.push("Interval raised to the 10 s minimum.".into());
    }
}

fn truncate_ids(items: &[Converted]) -> Result<(), ApiError> {
    if items.len() > MAX_MONITORS {
        return Err(ApiError::BadRequest(format!(
            "At most {MAX_MONITORS} monitors can be imported at once ({} found).",
            items.len()
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------- HTTP sortant

/// Requête vers un service tiers, jugée par le garde des notifications (jamais
/// vers le serveur lui-même) et bornée dans le temps.
async fn fetch(builder: reqwest::RequestBuilder, url: &str) -> Result<Value, ApiError> {
    let parsed = Url::parse(url).map_err(|_| ApiError::BadRequest("Invalid address.".into()))?;
    guard::vet_url(&parsed).await.map_err(|e| ApiError::BadRequest(e.to_string()))?;
    let port = parsed.port_or_known_default();
    let response = guard::scoped(port, builder.timeout(FETCH_TIMEOUT).send())
        .await
        .map_err(|e| ApiError::BadGateway(format!("The service is unreachable: {}", e.without_url())))?;
    let status = response.status();
    if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        return Err(ApiError::BadRequest("The service refused this API key.".into()));
    }
    if !status.is_success() {
        return Err(ApiError::BadGateway(format!("The service answered {status}.")));
    }
    response
        .json::<Value>()
        .await
        .map_err(|_| ApiError::BadGateway("The service answer is not valid JSON.".into()))
}

fn number(value: &Value, key: &str) -> Option<u64> {
    let v = value.get(key)?;
    v.as_u64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).map(str::trim).unwrap_or_default()
}

// ---------------------------------------------------------------- UptimeRobot

#[derive(Deserialize)]
pub struct UptimeRobotBody {
    #[serde(default)]
    api_key: Option<String>,
    #[serde(default)]
    csv: Option<String>,
}

pub async fn uptimerobot(
    State(state): State<AppState>,
    Query(query): Query<ImportQuery>,
    Json(body): Json<UptimeRobotBody>,
) -> Reply {
    let items = match (body.api_key.as_deref().map(str::trim), body.csv.as_deref()) {
        (Some(key), _) if !key.is_empty() => uptimerobot_api(key).await?,
        (_, Some(csv)) if !csv.trim().is_empty() => {
            uptimerobot_csv(csv).map_err(ApiError::BadRequest)?
        }
        _ => {
            return Err(ApiError::BadRequest(
                "Send an UptimeRobot API key (\"api_key\") or the exported CSV (\"csv\").".into(),
            ));
        }
    };
    truncate_ids(&items)?;
    run(&state, sorted(items), 0, &query).await
}

async fn uptimerobot_api(api_key: &str) -> Result<Vec<Converted>, ApiError> {
    let client = crate::notify::http_client();
    let mut monitors: Vec<Value> = Vec::new();
    let mut offset = 0usize;
    loop {
        let form = [
            ("api_key", api_key.to_string()),
            ("format", "json".into()),
            ("limit", "50".into()),
            ("offset", offset.to_string()),
        ];
        let answer = fetch(client.post(UPTIMEROBOT_API).form(&form), UPTIMEROBOT_API).await?;
        if answer.get("stat").and_then(Value::as_str) != Some("ok") {
            let why = answer
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("the request was refused");
            return Err(ApiError::BadRequest(format!("UptimeRobot: {why}.")));
        }
        let page = answer.get("monitors").and_then(Value::as_array).cloned().unwrap_or_default();
        let total = number(answer.get("pagination").unwrap_or(&Value::Null), "total")
            .unwrap_or(page.len() as u64) as usize;
        let got = page.len();
        monitors.extend(page);
        offset += got;
        if got == 0 || offset >= total || monitors.len() > MAX_MONITORS {
            break;
        }
    }
    Ok(monitors.iter().map(convert_uptimerobot).collect())
}

/// Un moniteur de l'API v2 (`type` : 1 HTTP(s), 2 mot-clé, 3 ping, 4 port, 5 heartbeat).
fn convert_uptimerobot(m: &Value) -> Converted {
    let kind = number(m, "type").unwrap_or(0);
    let mut out = blank(text(m, "friendly_name"), &format!("type {kind}"), "UptimeRobot monitor");
    out.interval_secs = number(m, "interval").unwrap_or(300);
    // 0 : en pause.
    out.enabled = number(m, "status") != Some(0);
    let url = text(m, "url");
    let mut options: BTreeMap<String, String> = BTreeMap::new();
    match kind {
        1 | 2 => {
            if url.is_empty() {
                out.ignored = Some("The monitor has no URL.".into());
            } else {
                out.kind = Some("http");
                out.address = url.to_string();
                out.source_type = if kind == 1 { "http".into() } else { "keyword".into() };
                match number(m, "http_method") {
                    Some(1) => options.insert("method".into(), "HEAD".into()),
                    Some(3) => options.insert("method".into(), "POST".into()),
                    Some(4) => options.insert("method".into(), "PUT".into()),
                    Some(5) => options.insert("method".into(), "PATCH".into()),
                    Some(6) => options.insert("method".into(), "DELETE".into()),
                    Some(7) => options.insert("method".into(), "OPTIONS".into()),
                    _ => None,
                };
                if kind == 2 {
                    let keyword = text(m, "keyword_value");
                    if keyword.is_empty() {
                        out.kind = None;
                        out.ignored = Some("The keyword monitor has no keyword.".into());
                    } else {
                        options.insert("keyword".into(), keyword.to_string());
                        options.insert("keyword_case_sensitive".into(), "true".into());
                        // 1 : le mot doit exister ; 2 : il ne doit pas exister.
                        if number(m, "keyword_type") == Some(2) {
                            options.insert("keyword_absent".into(), "true".into());
                        }
                    }
                }
                if ["http_username", "http_password", "custom_http_headers", "post_value"]
                    .iter()
                    .any(|k| m.get(*k).is_some_and(|v| !v.is_null() && v.as_str() != Some("")))
                {
                    out.notes.push(
                        "Credentials, headers and request body are not imported: add them on the \
                         device if the site needs them."
                            .into(),
                    );
                }
            }
        }
        3 => {
            if url.is_empty() {
                out.ignored = Some("The monitor has no host.".into());
            } else {
                out.kind = Some("ping");
                out.address = url.to_string();
            }
        }
        4 => match number(m, "port").filter(|p| (1..=65535).contains(p)) {
            Some(port) if !url.is_empty() => {
                out.kind = Some("tcp");
                out.address = host_port(url, port);
            }
            _ => out.ignored = Some("The monitor has no host or port.".into()),
        },
        5 => {
            out.kind = Some(crate::collectors::push::KIND);
            out.address = match tag_key(&out.name).as_str() {
                "" => "heartbeat".to_string(),
                label => label.to_string(),
            };
            options.insert("expected_interval".into(), format!("{}s", out.interval_secs));
            out.notes.push("Gets a new push URL: update the job that calls it.".into());
        }
        other => {
            out.ignored = Some(format!("No equivalent for the UptimeRobot monitor type {other}."));
        }
    }
    finish(&mut out, options);
    out
}

/// Fin commune : période minimale, abandon propre d'une ligne ignorée.
fn finish(out: &mut Converted, options: BTreeMap<String, String>) {
    if out.kind.is_some() {
        clamp_interval(out);
    }
    if out.ignored.is_some() {
        out.kind = None;
        out.notes.clear();
    }
    out.tags.extend(options);
}

/// Export CSV du tableau de bord : colonnes reconnues par leur titre, sans
/// tenir compte de la casse ni de l'ordre.
fn uptimerobot_csv(csv: &str) -> Result<Vec<Converted>, String> {
    let rows = crate::api::targets_import::parse_csv(csv)?;
    let Some((header, body)) = rows.split_first() else {
        return Err("The CSV is empty.".into());
    };
    let columns: HashMap<String, usize> = header
        .iter()
        .enumerate()
        .map(|(i, h)| (h.trim().to_lowercase().replace([' ', '_', '-', '/'], ""), i))
        .collect();
    let column = |names: &[&str]| names.iter().find_map(|n| columns.get(*n).copied());
    let (Some(name_col), Some(url_col)) =
        (column(&["friendlyname", "name", "monitor"]), column(&["url", "urlip", "hostname", "host"]))
    else {
        return Err(
            "The CSV needs a \"Friendly Name\" column and a \"URL / IP\" column (export it from \
             the UptimeRobot dashboard)."
                .into(),
        );
    };
    let type_col = column(&["type", "monitortype"]);
    let port_col = column(&["port"]);
    let interval_col = column(&["interval", "monitoringinterval", "checkinterval"]);
    let keyword_col = column(&["keyword", "keywordvalue"]);
    let cell = |row: &Vec<String>, col: Option<usize>| -> String {
        col.and_then(|c| row.get(c)).map(|v| v.trim().to_string()).unwrap_or_default()
    };

    let mut out = Vec::new();
    for row in body {
        let name = cell(row, Some(name_col));
        let target = cell(row, Some(url_col));
        let kind_text = cell(row, type_col).to_lowercase();
        let mut item = blank(&name, &kind_text, "UptimeRobot monitor");
        // « 5 minutes », « 300 », « 5 min ».
        let interval = cell(row, interval_col);
        let digits: String = interval.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(n) = digits.parse::<u64>() {
            let lower = interval.to_lowercase();
            item.interval_secs = if lower.contains("min") || lower.ends_with('m') {
                n * 60
            } else if lower.contains("hour") || lower.ends_with('h') {
                n * 3600
            } else if lower.contains("sec") || lower.ends_with('s') || n >= 30 {
                n
            } else {
                n * 60
            };
        } else {
            item.interval_secs = 300;
        }
        let mut options = BTreeMap::new();
        let port = cell(row, port_col).parse::<u64>().ok().filter(|p| (1..=65535).contains(p));
        let keyword = cell(row, keyword_col);
        if target.is_empty() {
            item.ignored = Some("The monitor has no URL or host.".into());
        } else if kind_text.contains("heartbeat") {
            item.kind = Some(crate::collectors::push::KIND);
            item.address = match tag_key(&item.name).as_str() {
                "" => "heartbeat".to_string(),
                label => label.to_string(),
            };
            options.insert("expected_interval".into(), format!("{}s", item.interval_secs));
            item.notes.push("Gets a new push URL: update the job that calls it.".into());
        } else if kind_text.contains("ping") {
            item.kind = Some("ping");
            item.address = target;
        } else if kind_text.contains("port") {
            match port {
                Some(port) => {
                    item.kind = Some("tcp");
                    item.address = host_port(&target, port);
                }
                None => item.ignored = Some("The monitor has no port.".into()),
            }
        } else if kind_text.contains("keyword") || !keyword.is_empty() {
            if keyword.is_empty() {
                item.ignored = Some("The keyword monitor has no keyword.".into());
            } else {
                item.kind = Some("http");
                item.address = target;
                options.insert("keyword".into(), keyword);
                options.insert("keyword_case_sensitive".into(), "true".into());
            }
        } else if target.starts_with("http://") || target.starts_with("https://") {
            item.kind = Some("http");
            item.address = target;
        } else {
            item.ignored = Some(format!("No equivalent for the type \"{kind_text}\"."));
        }
        finish(&mut item, options);
        out.push(item);
    }
    Ok(out)
}

// ---------------------------------------------------------------- Gatus

#[derive(Deserialize)]
pub struct GatusBody {
    yaml: String,
}

pub async fn gatus(
    State(state): State<AppState>,
    Query(query): Query<ImportQuery>,
    Json(body): Json<GatusBody>,
) -> Reply {
    let items = convert_gatus(&body.yaml).map_err(ApiError::BadRequest)?;
    truncate_ids(&items)?;
    run(&state, sorted(items), 0, &query).await
}

/// Durée Gatus (`30s`, `5m`, `1h30m`) en secondes.
fn gatus_duration(raw: &str) -> Option<u64> {
    crate::collectors::push::parse_duration(raw).map(|d| d.as_secs()).filter(|s| *s > 0)
}

/// Lit le fichier de configuration Gatus : `endpoints:` (sondes) et
/// `external-endpoints:` (en poussée).
pub fn convert_gatus(yaml: &str) -> Result<Vec<Converted>, String> {
    let root: Value = dumbmonit_collectors::yaml::from_str(yaml)
        .map_err(|e| format!("This is not valid YAML: {e}"))?;
    let endpoints = root.get("endpoints").and_then(Value::as_array);
    let external = root.get("external-endpoints").and_then(Value::as_array);
    if endpoints.is_none() && external.is_none() {
        return Err("This is not a Gatus configuration: \"endpoints\" is missing.".into());
    }
    let default_interval = root
        .pointer("/default-endpoint/interval")
        .and_then(Value::as_str)
        .and_then(gatus_duration)
        .unwrap_or(60);
    let mut out = Vec::new();
    for endpoint in endpoints.into_iter().flatten() {
        out.push(convert_gatus_endpoint(endpoint, default_interval));
    }
    for endpoint in external.into_iter().flatten() {
        out.push(convert_gatus_external(endpoint));
    }
    Ok(out)
}

fn gatus_folder(endpoint: &Value) -> String {
    text(endpoint, "group").replace('/', "-")
}

fn convert_gatus_external(endpoint: &Value) -> Converted {
    let mut out = blank(text(endpoint, "name"), "external-endpoint", "Gatus endpoint");
    out.folder = gatus_folder(endpoint);
    out.kind = Some(crate::collectors::push::KIND);
    out.address = match tag_key(&out.name).as_str() {
        "" => "heartbeat".to_string(),
        label => label.to_string(),
    };
    out.enabled = endpoint.get("enabled").and_then(Value::as_bool).unwrap_or(true);
    let period = endpoint
        .pointer("/heartbeat/interval")
        .and_then(Value::as_str)
        .and_then(gatus_duration)
        .unwrap_or(86_400);
    out.interval_secs = 60;
    out.tags.insert("expected_interval".into(), format!("{period}s"));
    out.notes.push("Gets a new push URL: update the job that calls it.".into());
    out
}

fn convert_gatus_endpoint(endpoint: &Value, default_interval: u64) -> Converted {
    let mut out = blank(text(endpoint, "name"), "endpoint", "Gatus endpoint");
    out.folder = gatus_folder(endpoint);
    out.enabled = endpoint.get("enabled").and_then(Value::as_bool).unwrap_or(true);
    out.interval_secs =
        endpoint.get("interval").and_then(Value::as_str).and_then(gatus_duration).unwrap_or(default_interval);
    let url = text(endpoint, "url");
    let mut options: BTreeMap<String, String> = BTreeMap::new();

    if endpoint.get("dns").is_some() {
        let query_name = endpoint.pointer("/dns/query-name").and_then(Value::as_str).unwrap_or("");
        if query_name.is_empty() {
            out.ignored = Some("The DNS endpoint has no query-name.".into());
        } else {
            out.kind = Some("dns");
            out.address = query_name.trim_end_matches('.').to_string();
            let record = endpoint
                .pointer("/dns/query-type")
                .and_then(Value::as_str)
                .unwrap_or("A")
                .to_uppercase();
            if record != "A" {
                options.insert("record_type".into(), record);
            }
            if url.parse::<std::net::IpAddr>().is_ok() {
                options.insert("resolver".into(), url.to_string());
            }
        }
    } else if let Some(rest) = url.strip_prefix("tcp://") {
        match rest.rsplit_once(':') {
            Some((host, port)) if port.parse::<u16>().is_ok() && !host.is_empty() => {
                out.kind = Some("tcp");
                out.address = rest.to_string();
            }
            _ => out.ignored = Some("The TCP endpoint has no host:port.".into()),
        }
    } else if let Some(rest) = url.strip_prefix("icmp://") {
        out.kind = Some("ping");
        out.address = rest.trim_end_matches('/').to_string();
    } else if url.starts_with("http://") || url.starts_with("https://") {
        out.kind = Some("http");
        out.address = url.to_string();
        let method = text(endpoint, "method").to_uppercase();
        if !method.is_empty() && method != "GET" {
            options.insert("method".into(), method);
        }
        if endpoint.pointer("/client/insecure").and_then(Value::as_bool) == Some(true) {
            options.insert("insecure_tls".into(), "true".into());
        }
        if let Some(timeout) =
            endpoint.pointer("/client/timeout").and_then(Value::as_str).and_then(gatus_duration)
        {
            options.insert("timeout_seconds".into(), timeout.clamp(1, 60).min(out.interval_secs).to_string());
        }
        let skipped = gatus_conditions(endpoint, &mut options);
        if !skipped.is_empty() {
            out.notes.push(format!("Conditions not imported: {}.", skipped.join("; ")));
        }
        if endpoint.get("headers").is_some() || endpoint.get("body").is_some() {
            out.notes.push(
                "Headers and request body are not imported: add them on the device if needed."
                    .into(),
            );
        }
    } else if url.is_empty() {
        out.ignored = Some("The endpoint has no URL.".into());
    } else {
        let scheme = url.split("://").next().unwrap_or("this");
        out.ignored = Some(format!("No equivalent for a \"{scheme}://\" endpoint."));
    }
    if endpoint.get("alerts").is_some() {
        out.notes.push("Gatus alerts are not imported: set up notification channels here.".into());
    }
    finish(&mut out, options);
    out
}

/// Traduit les conditions qu'un moniteur HTTP sait exprimer ; rend les autres.
fn gatus_conditions(endpoint: &Value, options: &mut BTreeMap<String, String>) -> Vec<String> {
    let mut skipped = Vec::new();
    for condition in endpoint.get("conditions").and_then(Value::as_array).into_iter().flatten() {
        let Some(condition) = condition.as_str().map(str::trim) else { continue };
        if let Some(code) = condition.strip_prefix("[STATUS] ==").map(str::trim)
            && code.parse::<u16>().is_ok()
        {
            options.insert("accepted_status".into(), code.to_string());
        } else if let Some(rest) = condition.strip_prefix("[BODY].")
            && let Some((path, expected)) = rest.split_once("==")
            && !options.contains_key("json_path")
        {
            options.insert("json_path".into(), format!("$.{}", path.trim()));
            options.insert("json_expect".into(), expected.trim().trim_matches('"').to_string());
        } else if condition.starts_with("[STATUS] <") || condition.starts_with("[STATUS] <=") {
            // Le comportement par défaut (2xx et 3xx suivis) couvre « < 300/400 ».
        } else {
            skipped.push(condition.to_string());
        }
    }
    skipped
}

// ---------------------------------------------------------------- Healthchecks.io

#[derive(Deserialize)]
pub struct HealthchecksBody {
    api_key: String,
    /// Instance auto-hébergée ; absent : healthchecks.io.
    #[serde(default)]
    base_url: Option<String>,
}

pub async fn healthchecks(
    State(state): State<AppState>,
    Query(query): Query<ImportQuery>,
    Json(body): Json<HealthchecksBody>,
) -> Reply {
    let key = body.api_key.trim();
    if key.is_empty() {
        return Err(ApiError::BadRequest("The Healthchecks API key is required.".into()));
    }
    let base = body.base_url.as_deref().map(str::trim).filter(|b| !b.is_empty());
    let base = base.unwrap_or(HEALTHCHECKS_DEFAULT).trim_end_matches('/').to_string();
    if !(base.starts_with("https://") || base.starts_with("http://")) {
        return Err(ApiError::BadRequest("The server address must start with https://.".into()));
    }
    let url = format!("{base}/api/v3/checks/");
    let client = crate::notify::http_client();
    let answer = fetch(client.get(&url).header("X-Api-Key", key), &url).await?;
    let checks = answer
        .get("checks")
        .and_then(Value::as_array)
        .ok_or_else(|| ApiError::BadGateway("Unexpected Healthchecks answer.".into()))?;
    let items: Vec<Converted> = checks.iter().map(convert_healthchecks).collect();
    truncate_ids(&items)?;
    run(&state, sorted(items), 0, &query).await
}

/// Période approchée d'un horaire cron, en secondes.
///
/// Reconnaît les formes courantes (`*/N * * * *`, `M * * * *`, `M H * * *`,
/// `M H * * D`, `M H D * *`) ; le reste vaut un jour et reçoit une note.
pub fn cron_period(expr: &str) -> Option<u64> {
    let f: Vec<&str> = expr.split_whitespace().collect();
    if f.len() != 5 {
        return None;
    }
    let (min, hour, dom, month, dow) = (f[0], f[1], f[2], f[3], f[4]);
    if let Some(step) = min.strip_prefix("*/").and_then(|n| n.parse::<u64>().ok())
        && [hour, dom, month, dow] == ["*", "*", "*", "*"]
        && step > 0
    {
        return Some(step * 60);
    }
    let fixed = |s: &str| s.parse::<u64>().is_ok();
    if min == "*" && [hour, dom, month, dow] == ["*", "*", "*", "*"] {
        return Some(60);
    }
    if fixed(min) && hour == "*" && [dom, month, dow] == ["*", "*", "*"] {
        return Some(3600);
    }
    if fixed(min) && fixed(hour) && month == "*" {
        if dom == "*" && dow == "*" {
            return Some(86_400);
        }
        if dom == "*" && fixed(dow) {
            return Some(7 * 86_400);
        }
        if fixed(dom) && dow == "*" {
            return Some(31 * 86_400);
        }
    }
    None
}

fn convert_healthchecks(check: &Value) -> Converted {
    let mut out = blank(text(check, "name"), "check", "Healthchecks check");
    let label = match text(check, "slug") {
        "" => tag_key(&out.name),
        slug => slug.to_string(),
    };
    out.kind = Some(crate::collectors::push::KIND);
    out.address = if label.is_empty() { "heartbeat".into() } else { label };
    out.interval_secs = 60;
    // Un check en pause chez eux le reste ici.
    out.enabled = text(check, "status") != "paused";

    // Les étiquettes : « prod nightly » → prod=true, nightly=true.
    for tag in text(check, "tags").split_whitespace() {
        let key = tag_key(tag);
        if !key.is_empty() {
            out.user_tags.insert(key.clone(), "true".into());
            out.tags.insert(key, "true".into());
        }
    }

    let (period, note) = match (number(check, "timeout"), text(check, "schedule")) {
        (_, schedule) if !schedule.is_empty() => match cron_period(schedule) {
            Some(secs) => (secs, None),
            None => (
                86_400,
                Some(format!(
                    "Cron schedule \"{schedule}\" approximated to one day: set the expected \
                     interval by hand."
                )),
            ),
        },
        (Some(timeout), _) if timeout > 0 => (timeout, None),
        _ => (86_400, None),
    };
    out.tags.insert("expected_interval".into(), format!("{period}s"));
    if let Some(grace) = number(check, "grace").filter(|g| *g > 0) {
        out.tags.insert("grace".into(), format!("{grace}s"));
    }
    out.notes.extend(note);
    out.notes.push(match text(check, "ping_url") {
        "" => "Gets a new push URL. Use a read-write API key to also list the old ping URLs."
            .to_string(),
        old => format!(
            "Gets a new push URL: update the job that calls {old}, or redirect that address to it."
        ),
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn by_name<'a>(items: &'a [Converted], name: &str) -> &'a Converted {
        items.iter().find(|c| c.name == name).unwrap_or_else(|| panic!("moniteur {name}"))
    }

    #[test]
    fn uptimerobot_api_convertit_les_types_usuels() {
        let http = convert_uptimerobot(&json!({
            "friendly_name": "Blog", "url": "https://blog.example.com", "type": 1,
            "interval": 300, "status": 2, "http_method": 3, "http_username": "bob"
        }));
        assert_eq!(http.kind, Some("http"));
        assert_eq!(http.interval_secs, 300);
        assert_eq!(http.tags["method"], "POST");
        assert!(http.enabled);
        assert!(http.notes.iter().any(|n| n.contains("not imported")));

        let keyword = convert_uptimerobot(&json!({
            "friendly_name": "Shop", "url": "https://shop.example.com", "type": 2,
            "keyword_type": 2, "keyword_value": "Maintenance", "interval": 60
        }));
        assert_eq!(keyword.tags["keyword"], "Maintenance");
        assert_eq!(keyword.tags["keyword_absent"], "true");

        let port = convert_uptimerobot(&json!({
            "friendly_name": "SSH", "url": "nas.lan", "type": 4, "port": 22, "interval": 120
        }));
        assert_eq!((port.kind, port.address.as_str()), (Some("tcp"), "nas.lan:22"));

        let beat = convert_uptimerobot(&json!({
            "friendly_name": "Nightly backup", "type": 5, "interval": 86400, "status": 0
        }));
        assert_eq!(beat.kind, Some("push"));
        assert_eq!(beat.address, "nightly-backup");
        assert_eq!(beat.tags["expected_interval"], "86400s");
        assert!(!beat.enabled, "paused monitors stay paused");

        let odd = convert_uptimerobot(&json!({"friendly_name": "x", "type": 9}));
        assert_eq!(odd.kind, None);
        assert!(odd.ignored.is_some());
    }

    #[test]
    fn uptimerobot_csv_est_lu_sans_tenir_compte_de_l_ordre() {
        let csv = "Friendly Name,URL / IP,Type,Port,Interval,Keyword\n\
                   Blog,https://blog.example.com,HTTP(s),,5 minutes,\n\
                   NAS,nas.lan,Port,22,1 minute,\n\
                   Router,192.168.1.1,Ping,,300,\n\
                   Status,https://status.example.com,Keyword,,5 minutes,All good\n";
        let items = uptimerobot_csv(csv).unwrap();
        assert_eq!(items.len(), 4);
        assert_eq!(by_name(&items, "Blog").interval_secs, 300);
        assert_eq!(by_name(&items, "NAS").address, "nas.lan:22");
        assert_eq!(by_name(&items, "NAS").interval_secs, 60);
        assert_eq!(by_name(&items, "Router").kind, Some("ping"));
        assert_eq!(by_name(&items, "Status").tags["keyword"], "All good");
        assert!(uptimerobot_csv("a,b\n1,2\n").is_err());
    }

    const GATUS: &str = r#"
default-endpoint:
  interval: 2m
endpoints:
  - name: Website
    group: core
    url: "https://example.org/health"
    interval: 30s
    client:
      insecure: true
    conditions:
      - "[STATUS] == 204"
      - "[BODY].status == UP"
      - "[RESPONSE_TIME] < 300"
    headers:
      Authorization: Bearer hunter2
    alerts:
      - type: slack
  - name: Database
    group: core
    url: "tcp://db.lan:5432"
  - name: Router
    url: "icmp://192.168.1.1"
    enabled: false
  - name: Resolver
    url: 1.1.1.1
    dns:
      query-name: example.org.
      query-type: MX
  - name: Mail
    url: "starttls://mail.example.org:587"
external-endpoints:
  - name: Backup
    group: jobs
    token: secret-token
    heartbeat:
      interval: 24h
"#;

    #[test]
    fn gatus_convertit_les_sondes_et_ignore_le_reste() {
        let items = convert_gatus(GATUS).unwrap();
        assert_eq!(items.len(), 6);
        let site = by_name(&items, "Website");
        assert_eq!(site.kind, Some("http"));
        assert_eq!(site.folder, "core");
        assert_eq!(site.interval_secs, 30);
        assert_eq!(site.tags["accepted_status"], "204");
        assert_eq!(site.tags["json_path"], "$.status");
        assert_eq!(site.tags["json_expect"], "UP");
        assert_eq!(site.tags["insecure_tls"], "true");
        assert!(site.notes.iter().any(|n| n.contains("RESPONSE_TIME")));
        assert!(site.notes.iter().any(|n| n.contains("alerts")));
        assert!(!format!("{site:?}").contains("hunter2"), "headers are never copied");

        let db = by_name(&items, "Database");
        assert_eq!((db.kind, db.address.as_str()), (Some("tcp"), "db.lan:5432"));
        assert_eq!(db.interval_secs, 120, "default-endpoint interval");

        let router = by_name(&items, "Router");
        assert_eq!(router.kind, Some("ping"));
        assert!(!router.enabled);

        let dns = by_name(&items, "Resolver");
        assert_eq!(dns.kind, Some("dns"));
        assert_eq!(dns.address, "example.org");
        assert_eq!(dns.tags["record_type"], "MX");
        assert_eq!(dns.tags["resolver"], "1.1.1.1");

        assert_eq!(by_name(&items, "Mail").kind, None);

        let beat = by_name(&items, "Backup");
        assert_eq!(beat.kind, Some("push"));
        assert_eq!(beat.tags["expected_interval"], "86400s");
        assert!(!format!("{beat:?}").contains("secret-token"));
    }

    #[test]
    fn gatus_refuse_un_fichier_qui_n_en_est_pas_un() {
        assert!(convert_gatus("hello: world\n").is_err());
        assert!(convert_gatus("a: [\n").is_err());
        assert!(convert_gatus("endpoints: []\n").unwrap().is_empty());
    }

    #[test]
    fn healthchecks_devient_un_moniteur_en_poussee_avec_l_ancienne_url() {
        let simple = convert_healthchecks(&json!({
            "name": "Nightly backup", "slug": "nightly-backup", "tags": "prod Nightly",
            "timeout": 86400, "grace": 3600, "status": "up",
            "ping_url": "https://hc-ping.com/0f1e2d3c"
        }));
        assert_eq!(simple.kind, Some("push"));
        assert_eq!(simple.address, "nightly-backup");
        assert_eq!(simple.tags["expected_interval"], "86400s");
        assert_eq!(simple.tags["grace"], "3600s");
        assert_eq!(simple.tags["prod"], "true");
        assert_eq!(simple.tags["nightly"], "true");
        assert!(simple.notes.iter().any(|n| n.contains("https://hc-ping.com/0f1e2d3c")));

        let cron = convert_healthchecks(&json!({
            "name": "Weekly", "schedule": "30 2 * * 1", "status": "paused"
        }));
        assert_eq!(cron.tags["expected_interval"], "604800s");
        assert!(!cron.enabled);
        assert!(cron.notes.iter().any(|n| n.contains("read-write")));

        let odd = convert_healthchecks(&json!({"name": "Odd", "schedule": "0 9 1,15 * *"}));
        assert_eq!(odd.tags["expected_interval"], "86400s");
        assert!(odd.notes.iter().any(|n| n.contains("approximated")));
    }

    #[test]
    fn periodes_cron() {
        assert_eq!(cron_period("*/5 * * * *"), Some(300));
        assert_eq!(cron_period("15 * * * *"), Some(3600));
        assert_eq!(cron_period("0 3 * * *"), Some(86_400));
        assert_eq!(cron_period("0 3 * * 0"), Some(604_800));
        assert_eq!(cron_period("0 3 1 * *"), Some(31 * 86_400));
        assert_eq!(cron_period("0 9 1,15 * *"), None);
        assert_eq!(cron_period("nope"), None);
    }
}
