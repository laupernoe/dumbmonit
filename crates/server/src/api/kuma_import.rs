//! Import depuis Uptime Kuma (`POST /api/targets/import/kuma`).
//!
//! Le corps est le fichier de sauvegarde que produit Uptime Kuma 1.x
//! (Réglages → Sauvegarde → Exporter) : `{ "version", "notificationList",
//! "monitorList": [...] }`. Uptime Kuma 2.x n'a plus cet export. Seule la liste
//! des moniteurs est lue : les notifications ne sont pas importées, pas plus que
//! les identifiants, en-têtes et corps de requête (jamais copiés, jamais journalisés).
//!
//! Correspondances : `http`, `keyword`, `json-query` → `http` ; `ping` → `ping` ;
//! `port` → `tcp` ; `dns` → `dns` ; `push` → `push` (heartbeat, nouvelle URL) ;
//! `group` → dossier. Le reste est ignoré avec sa raison.
//!
//! `?dry_run=true` rend le même rapport sans rien créer.

use std::collections::{BTreeMap, HashMap, HashSet};

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::api::targets::TargetPayload;
use crate::api::{ApiError, ApiResult};
use crate::db;
use crate::state::AppState;

/// Taille maximale du fichier. Une sauvegarde ne contient pas les mesures : quelques
/// centaines de moniteurs tiennent dans quelques centaines de Kio.
pub const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
const MAX_MONITORS: usize = 2000;
const MAX_FOLDER_DEPTH: usize = 8;
const NOT_A_BACKUP: &str = "This is not an Uptime Kuma backup: \"monitorList\" is missing. \
Export it from Settings, Backup in Uptime Kuma 1.x (version 2 has no backup export).";

#[derive(Deserialize)]
pub struct ImportQuery {
    #[serde(default)]
    dry_run: bool,
}

#[derive(Serialize)]
pub struct RowReport {
    /// Rang du moniteur dans le fichier, à partir de 1.
    row: usize,
    name: String,
    /// Type Uptime Kuma d'origine.
    source_type: String,
    /// Type DumbMonit créé ; absent quand le moniteur est ignoré.
    kind: Option<String>,
    address: String,
    folder: String,
    interval_secs: Option<u64>,
    tags: Vec<String>,
    /// `ok` (aperçu), `created`, `skipped` (ignoré ou déjà suivi) ou `error`.
    status: &'static str,
    message: Option<String>,
    id: Option<i64>,
}

#[derive(Serialize)]
pub struct ImportReport {
    dry_run: bool,
    total: usize,
    created: usize,
    skipped: usize,
    errors: usize,
    /// Notifications du fichier, jamais importées.
    notifications_ignored: usize,
    rows: Vec<RowReport>,
}

pub async fn import(
    State(state): State<AppState>,
    Query(query): Query<ImportQuery>,
    Json(root): Json<Value>,
) -> ApiResult<(StatusCode, Json<ImportReport>)> {
    let converted = convert(&root).map_err(ApiError::BadRequest)?;
    let notifications_ignored =
        root.get("notificationList").and_then(Value::as_array).map_or(0, Vec::len);

    let mut seen: HashSet<String> = db::targets::list(&state.pool, &state.cipher)
        .await?
        .into_iter()
        .map(|t| dedup_key(&t.kind, &t.address))
        .collect();

    let mut rows = Vec::with_capacity(converted.len());
    let (mut created, mut skipped, mut errors) = (0, 0, 0);
    for (index, item) in converted.into_iter().enumerate() {
        let mut report = RowReport {
            row: index + 1,
            name: item.name.clone(),
            source_type: item.source_type.clone(),
            kind: item.kind.map(str::to_string),
            address: item.address.clone(),
            folder: item.folder.clone(),
            interval_secs: item.kind.map(|_| item.interval_secs),
            tags: item.user_tags.iter().map(|(k, v)| format!("{k}={v}")).collect(),
            status: "skipped",
            message: None,
            id: None,
        };
        let Some(kind) = item.kind else {
            report.message = item.ignored;
            skipped += 1;
            rows.push(report);
            continue;
        };
        let key = dedup_key(kind, &item.address);
        if seen.contains(&key) {
            report.message = Some("Already monitored.".into());
            skipped += 1;
            rows.push(report);
            continue;
        }
        let payload = TargetPayload {
            name: item.name,
            address: item.address,
            kind: kind.into(),
            profile_id: None,
            parent_id: None,
            via_agent: None,
            interval_secs: Some(item.interval_secs),
            enabled: Some(item.enabled),
            tags: item.tags,
            group_name: Some(Some(item.folder)),
            credential: None,
        };
        let input = match payload.validate(&state, None) {
            Ok(input) => input,
            Err(error) => {
                report.status = "error";
                report.message = Some(error.into_parts().1);
                errors += 1;
                rows.push(report);
                continue;
            }
        };
        seen.insert(key);
        report.message = (!item.notes.is_empty()).then(|| item.notes.join(" "));
        if query.dry_run {
            report.status = "ok";
        } else {
            let id = db::targets::create(&state.pool, &state.cipher, &input)
                .await
                .map_err(ApiError::Internal)?;
            report.status = "created";
            report.id = Some(id);
        }
        created += 1;
        rows.push(report);
    }

    let status = if query.dry_run { StatusCode::OK } else { StatusCode::CREATED };
    Ok((
        status,
        Json(ImportReport {
            dry_run: query.dry_run,
            total: rows.len(),
            created,
            skipped,
            errors,
            notifications_ignored,
            rows,
        }),
    ))
}

fn dedup_key(kind: &str, address: &str) -> String {
    format!("{kind}|{}", address.trim().trim_end_matches('/').to_lowercase())
}

// ---------------------------------------------------------------- conversion

/// Un moniteur Uptime Kuma tel que DumbMonit le créerait (ou l'ignore).
#[derive(Debug)]
pub struct Converted {
    pub name: String,
    pub source_type: String,
    /// `None` : moniteur ignoré, la raison est dans `ignored`.
    pub kind: Option<&'static str>,
    pub address: String,
    pub folder: String,
    pub interval_secs: u64,
    pub enabled: bool,
    /// Options du type et étiquettes de l'utilisateur, ensemble (c'est la même table).
    pub tags: BTreeMap<String, String>,
    /// Étiquettes Uptime Kuma seules, pour l'aperçu.
    pub user_tags: BTreeMap<String, String>,
    pub ignored: Option<String>,
    pub notes: Vec<String>,
}

type Groups = HashMap<u64, (String, Option<u64>)>;

fn text<'a>(monitor: &'a Value, key: &str) -> &'a str {
    monitor.get(key).and_then(Value::as_str).map(str::trim).unwrap_or_default()
}

fn number(monitor: &Value, key: &str) -> Option<u64> {
    let value = monitor.get(key)?;
    value
        .as_u64()
        .or_else(|| value.as_f64().filter(|f| *f >= 0.0).map(|f| f as u64))
        .or_else(|| value.as_str().and_then(|s| s.trim().parse().ok()))
}

fn flag(monitor: &Value, key: &str) -> bool {
    match monitor.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_i64().is_some_and(|n| n != 0),
        _ => false,
    }
}

/// Lit `monitorList` (tableau, ou objet indexé par identifiant) et convertit chaque moniteur.
pub fn convert(root: &Value) -> Result<Vec<Converted>, String> {
    let list = match root.get("monitorList") {
        Some(Value::Array(items)) => items.iter().collect::<Vec<_>>(),
        Some(Value::Object(items)) => items.values().collect(),
        _ => {
            return Err(NOT_A_BACKUP.into());
        }
    };
    if list.len() > MAX_MONITORS {
        return Err(format!(
            "At most {MAX_MONITORS} monitors can be imported at once ({} found).",
            list.len()
        ));
    }
    let mut groups = Groups::new();
    for monitor in &list {
        if text(monitor, "type") == "group"
            && let Some(id) = number(monitor, "id")
        {
            groups.insert(id, (folder_part(text(monitor, "name")), number(monitor, "parent")));
        }
    }
    Ok(list.into_iter().map(|monitor| convert_one(monitor, &groups)).collect())
}

fn folder_part(name: &str) -> String {
    name.replace('/', "-").trim().to_string()
}

fn folder_of(monitor: &Value, groups: &Groups) -> String {
    let mut parts = Vec::new();
    let mut next = number(monitor, "parent");
    while let Some(id) = next {
        let Some((name, parent)) = groups.get(&id) else { break };
        if parts.len() >= MAX_FOLDER_DEPTH {
            break;
        }
        parts.push(name.clone());
        next = *parent;
    }
    parts.reverse();
    parts.retain(|p| !p.is_empty());
    parts.join("/")
}

/// Nom d'étiquette DumbMonit : minuscules, chiffres, `_` et `-`.
fn tag_key(name: &str) -> String {
    let key: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '-' })
        .collect();
    key.trim_matches('-').to_string()
}

fn user_tags(monitor: &Value) -> BTreeMap<String, String> {
    let mut tags = BTreeMap::new();
    for tag in monitor.get("tags").and_then(Value::as_array).into_iter().flatten() {
        let key = tag_key(text(tag, "name"));
        if key.is_empty() {
            continue;
        }
        let value = text(tag, "value");
        tags.insert(key, if value.is_empty() { "true".into() } else { value.to_string() });
    }
    tags
}

fn host_port(host: &str, port: u64) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

fn convert_one(monitor: &Value, groups: &Groups) -> Converted {
    let source_type = text(monitor, "type").to_string();
    let name = match text(monitor, "name") {
        "" => "Uptime Kuma monitor".to_string(),
        n => n.to_string(),
    };
    let user_tags = user_tags(monitor);
    let mut out = Converted {
        name,
        source_type: source_type.clone(),
        kind: None,
        address: String::new(),
        folder: folder_of(monitor, groups),
        interval_secs: number(monitor, "interval").unwrap_or(60),
        enabled: monitor.get("active").is_none_or(|_| flag(monitor, "active")),
        tags: user_tags.clone(),
        user_tags,
        ignored: None,
        notes: Vec::new(),
    };
    if out.interval_secs < 10 {
        out.interval_secs = 10;
        out.notes.push("Interval raised to the 10 s minimum.".into());
    }

    let mut options: BTreeMap<String, String> = BTreeMap::new();
    let hostname = text(monitor, "hostname");
    let ignore = |out: &mut Converted, why: &str| out.ignored = Some(why.to_string());

    match source_type.as_str() {
        "http" | "keyword" | "json-query" => {
            let url = text(monitor, "url");
            if url.is_empty() {
                ignore(&mut out, "The monitor has no URL.");
            } else {
                out.kind = Some("http");
                out.address = url.to_string();
                let method = text(monitor, "method").to_uppercase();
                if !method.is_empty() && method != "GET" {
                    options.insert("method".into(), method);
                }
                let codes: Vec<&str> = monitor
                    .get("accepted_statuscodes")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect();
                if !codes.is_empty() && codes != ["200-299"] {
                    options.insert("accepted_status".into(), codes.join(","));
                }
                if flag(monitor, "ignoreTls") {
                    options.insert("insecure_tls".into(), "true".into());
                }
                match number(monitor, "maxredirects") {
                    Some(0) => {
                        options.insert("follow_redirects".into(), "false".into());
                    }
                    Some(max) if max != 10 => {
                        options.insert("max_redirects".into(), max.min(20).to_string());
                    }
                    _ => {}
                }
                if let Some(timeout) = number(monitor, "timeout").filter(|t| *t != 0) {
                    let secs = timeout.clamp(1, 60).min(out.interval_secs);
                    options.insert("timeout_seconds".into(), secs.to_string());
                }
                if source_type == "keyword" {
                    let keyword = text(monitor, "keyword");
                    if keyword.is_empty() {
                        ignore(&mut out, "The keyword monitor has no keyword.");
                    } else {
                        options.insert("keyword".into(), keyword.to_string());
                        options.insert("keyword_case_sensitive".into(), "true".into());
                        if flag(monitor, "invertKeyword") {
                            options.insert("keyword_absent".into(), "true".into());
                        }
                    }
                } else if source_type == "json-query" {
                    let path = text(monitor, "jsonPath");
                    if path.is_empty() {
                        ignore(&mut out, "The JSON query monitor has no query.");
                    } else {
                        options.insert("json_path".into(), path.to_string());
                        options.insert(
                            "json_expect".into(),
                            text(monitor, "expectedValue").to_string(),
                        );
                    }
                }
                if ["basic_auth_user", "headers", "body", "oauth_client_id"]
                    .iter()
                    .any(|k| !text(monitor, k).is_empty())
                {
                    out.notes.push(
                        "Credentials, headers and request body are not imported: add them on the \
                         device if the site needs them."
                            .into(),
                    );
                }
            }
        }
        "ping" => {
            if hostname.is_empty() {
                ignore(&mut out, "The monitor has no host name.");
            } else {
                out.kind = Some("ping");
                out.address = hostname.to_string();
            }
        }
        "port" => match number(monitor, "port").filter(|p| (1..=65535).contains(p)) {
            Some(port) if !hostname.is_empty() => {
                out.kind = Some("tcp");
                out.address = host_port(hostname, port);
            }
            _ => ignore(&mut out, "The monitor has no host name or port."),
        },
        "dns" => {
            if hostname.is_empty() {
                ignore(&mut out, "The monitor has no host name.");
            } else {
                out.kind = Some("dns");
                out.address = hostname.to_string();
                let record = text(monitor, "dns_resolve_type").to_uppercase();
                if !record.is_empty() && record != "A" {
                    options.insert("record_type".into(), record);
                }
                let resolver = text(monitor, "dns_resolve_server");
                if resolver.parse::<std::net::IpAddr>().is_ok() {
                    options.insert("resolver".into(), resolver.to_string());
                }
            }
        }
        "push" => {
            out.kind = Some(crate::collectors::push::KIND);
            out.address = match tag_key(&out.name).as_str() {
                "" => "heartbeat".to_string(),
                label => label.to_string(),
            };
            options.insert("expected_interval".into(), format!("{}s", out.interval_secs));
            out.notes.push("Gets a new push URL: update the job that calls it.".into());
        }
        "group" => ignore(&mut out, "Groups become folders."),
        "docker" => ignore(
            &mut out,
            "Containers are watched by the agent, which lists them as monitors of their own.",
        ),
        other => ignore(&mut out, &format!("No equivalent for the \"{other}\" monitor type.")),
    }

    if out.kind.is_some() && flag(monitor, "upsideDown") {
        ignore(&mut out, "Upside-down mode (down means up) is not supported.");
    }
    if out.ignored.is_some() {
        out.kind = None;
        out.notes.clear();
    }
    // Les options du type l'emportent sur une étiquette de même nom.
    out.tags.extend(options);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const BACKUP: &str = include_str!("../../tests/fixtures/uptime-kuma-backup.json");

    fn rows() -> Vec<Converted> {
        convert(&serde_json::from_str(BACKUP).unwrap()).unwrap()
    }

    fn row<'a>(rows: &'a [Converted], name: &str) -> &'a Converted {
        rows.iter().find(|r| r.name == name).unwrap_or_else(|| panic!("moniteur {name}"))
    }

    #[test]
    fn les_types_usuels_sont_convertis() {
        let rows = rows();
        assert_eq!(rows.len(), 13);

        let site = row(&rows, "Blog");
        assert_eq!(site.kind, Some("http"));
        assert_eq!(site.address, "https://blog.example.com/");
        assert_eq!(site.interval_secs, 60);
        assert_eq!(site.folder, "Clients/Acme");
        assert_eq!(site.tags.get("env").map(String::as_str), Some("prod"));
        assert_eq!(site.tags.get("critical").map(String::as_str), Some("true"));
        assert_eq!(site.tags.get("accepted_status").map(String::as_str), Some("200-299,301"));
        assert!(site.notes.iter().any(|n| n.contains("not imported")));

        let mot = row(&rows, "Status page");
        assert_eq!(mot.tags["keyword"], "All systems operational");
        assert_eq!(mot.tags["keyword_absent"], "true");
        assert_eq!(mot.tags["insecure_tls"], "true");

        let json = row(&rows, "API health");
        assert_eq!(json.tags["json_path"], "$.status");
        assert_eq!(json.tags["json_expect"], "ok");
        assert_eq!(json.tags["method"], "POST");

        assert_eq!(row(&rows, "Router").kind, Some("ping"));
        let ssh = row(&rows, "NAS SSH");
        assert_eq!((ssh.kind, ssh.address.as_str()), (Some("tcp"), "nas.lan:22"));
        assert_eq!(row(&rows, "IPv6 web").address, "[2001:db8::1]:443");

        let dns = row(&rows, "MX example");
        assert_eq!(dns.kind, Some("dns"));
        assert_eq!(dns.tags["record_type"], "MX");
        assert_eq!(dns.tags["resolver"], "1.1.1.1");

        let cron = row(&rows, "Nightly backup");
        assert_eq!(cron.kind, Some("push"));
        assert_eq!(cron.address, "nightly-backup");
        assert_eq!(cron.tags["expected_interval"], "86400s");
        assert!(!cron.enabled, "moniteur en pause dans Uptime Kuma");
    }

    #[test]
    fn le_reste_est_ignore_avec_sa_raison() {
        let rows = rows();
        for (name, part) in [
            ("Acme", "folders"),
            ("Containers", "agent"),
            ("Inverted", "Upside-down"),
            ("Postgres", "No equivalent"),
        ] {
            let item = row(&rows, name);
            assert_eq!(item.kind, None, "{name}");
            assert!(item.ignored.as_deref().unwrap_or_default().contains(part), "{name}");
        }
    }

    #[test]
    fn aucun_identifiant_ne_ressort() {
        for item in rows() {
            let dump = format!("{item:?}");
            for secret in ["hunter2", "s3cr3t-token", "Bearer abc123", "pushtok-0123"] {
                assert!(!dump.contains(secret), "{} fuit {secret}", item.name);
            }
        }
    }

    #[test]
    fn un_fichier_qui_n_en_est_pas_un_est_refuse() {
        assert!(convert(&serde_json::json!({ "hello": 1 })).is_err());
        assert!(convert(&serde_json::json!({ "monitorList": [] })).unwrap().is_empty());
        // Objet indexé par identifiant, comme la liste côté navigateur d'Uptime Kuma.
        let indexed = serde_json::json!({
            "monitorList": { "1": { "type": "ping", "name": "a", "hostname": "h" } }
        });
        assert_eq!(convert(&indexed).unwrap()[0].kind, Some("ping"));
    }

    #[test]
    fn intervalle_court_releve() {
        let one = serde_json::json!({
            "monitorList": [{ "type": "ping", "name": "a", "hostname": "h", "interval": 5 }]
        });
        let item = &convert(&one).unwrap()[0];
        assert_eq!(item.interval_secs, 10);
        assert!(!item.notes.is_empty());
    }
}
