//! Import en masse de sites web (`POST /api/targets/import`).
//!
//! Le corps est un CSV (`text/csv`) ou `{"csv": "..."}`. Colonnes : `url`
//! (obligatoire), `name`, `folder`, `tags` (`k=v;k=v`), `interval`,
//! `expect_text`. Chaque ligne est validée comme une création ordinaire ; les
//! doublons sont ignorés et signalés. Sans `folder`, le dossier est
//! `Websites/<plateforme>`, la plateforme étant lue par un GET au moment de
//! l'import (4 à la fois, 10 s au plus chacun).
//!
//! `?dry_run=true` rend le même rapport sans rien créer.

use std::collections::{BTreeMap, HashSet};
use std::time::{Duration, Instant};

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use futures::StreamExt;
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::api::targets::TargetPayload;
use crate::api::{ApiError, ApiResult};
use crate::db;
use crate::state::AppState;

/// Type de cible créé par l'import.
const KIND: &str = "website";
const MAX_ROWS: usize = 1000;
const FETCH_TIMEOUT: Duration = Duration::from_secs(10);
const FETCH_CONCURRENCY: usize = 4;
/// Au-delà, les lignes restantes ne sont plus visitées : elles vont dans `Other`.
const DETECT_BUDGET: Duration = Duration::from_secs(90);
const MAX_HTML_BYTES: usize = 512 * 1024;
const DEFAULT_FOLDER: &str = "Websites";
const OTHER_PLATFORM: &str = "Other";

#[derive(Deserialize)]
pub struct ImportQuery {
    #[serde(default)]
    dry_run: bool,
}

#[derive(Serialize)]
pub struct RowReport {
    /// Numéro de ligne dans le fichier (l'en-tête est la ligne 1).
    row: usize,
    url: String,
    name: String,
    folder: String,
    platform: Option<String>,
    /// `ok` (aperçu), `created`, `skipped` ou `error`.
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
    rows: Vec<RowReport>,
}

pub async fn import(
    State(state): State<AppState>,
    Query(query): Query<ImportQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<(StatusCode, Json<ImportReport>)> {
    let csv = csv_text(&headers, &body)?;
    let table = parse_csv(&csv).map_err(ApiError::BadRequest)?;
    let (columns, records) = table.split_first().ok_or_else(|| {
        ApiError::BadRequest(
            "The CSV is empty: a header row with a \"url\" column is required.".into(),
        )
    })?;
    let columns: Vec<String> = columns.iter().map(|c| c.trim().to_lowercase()).collect();
    let index = |name: &str| columns.iter().position(|c| c == name);
    let Some(url_col) = index("url") else {
        return Err(ApiError::BadRequest("The header row needs a \"url\" column.".into()));
    };
    let records: Vec<(usize, &Vec<String>)> = records
        .iter()
        .enumerate()
        .map(|(i, record)| (i + 2, record))
        .filter(|(_, record)| record.iter().any(|cell| !cell.trim().is_empty()))
        .collect();
    if records.len() > MAX_ROWS {
        return Err(ApiError::BadRequest(format!(
            "At most {MAX_ROWS} rows can be imported at once ({} found).",
            records.len()
        )));
    }
    let cell = |record: &Vec<String>, name: &str| -> String {
        index(name).and_then(|i| record.get(i)).map(|v| v.trim().to_string()).unwrap_or_default()
    };

    // Premier passage : analyse locale, sans réseau.
    let mut rows: Vec<Row> = Vec::new();
    for (line, record) in &records {
        let raw_url = record.get(url_col).map(|v| v.trim().to_string()).unwrap_or_default();
        let parsed = parse_row(&raw_url, record, &cell);
        rows.push(Row { line: *line, raw_url, parsed });
    }

    // Deuxième passage : plateforme et URL finale des lignes sans dossier.
    let existing = existing_addresses(&state).await?;
    let client = detection_client();
    let started = Instant::now();
    let jobs: Vec<Option<String>> = rows
        .iter()
        .map(|row| match &row.parsed {
            Ok(p) if p.folder.is_none() => Some(p.url.clone()),
            _ => None,
        })
        .collect();
    let detections: Vec<Option<Detection>> = futures::stream::iter(jobs.into_iter().map(|url| {
        let client = client.clone();
        async move {
            match (url, client) {
                (Some(url), Some(client)) if started.elapsed() < DETECT_BUDGET => {
                    Some(detect(&client, &url).await)
                }
                _ => None,
            }
        }
    }))
    .buffered(FETCH_CONCURRENCY)
    .collect()
    .await;

    // Troisième passage : doublons, validation commune, création.
    let mut seen: HashSet<String> = existing;
    let mut reports = Vec::with_capacity(rows.len());
    let (mut created, mut skipped, mut errors) = (0, 0, 0);
    for (row, detection) in rows.into_iter().zip(detections) {
        let mut report = RowReport {
            row: row.line,
            url: row.raw_url.clone(),
            name: String::new(),
            folder: String::new(),
            platform: None,
            status: "error",
            message: None,
            id: None,
        };
        let parsed = match row.parsed {
            Ok(parsed) => parsed,
            Err(message) => {
                report.message = Some(message);
                errors += 1;
                reports.push(report);
                continue;
            }
        };
        report.url = parsed.url.clone();
        report.name = parsed.name.clone();
        let (platform, final_url) = match detection {
            Some(d) => (Some(d.platform), d.final_url),
            None => (None, None),
        };
        let folder = match (&parsed.folder, &platform) {
            (Some(folder), _) => folder.clone(),
            (None, Some(platform)) => format!("{DEFAULT_FOLDER}/{platform}"),
            (None, None) => format!("{DEFAULT_FOLDER}/{OTHER_PLATFORM}"),
        };
        report.folder = folder.clone();
        report.platform = platform;

        let key = dedup_key(&parsed.url);
        let final_key = final_url.as_deref().map(dedup_key);
        let duplicate = if seen.contains(&key) {
            Some(parsed.url.clone())
        } else {
            final_key.clone().filter(|k| seen.contains(k)).and(final_url.clone())
        };
        if let Some(which) = duplicate {
            report.status = "skipped";
            report.message = Some(format!("Already monitored ({which})."));
            skipped += 1;
            reports.push(report);
            continue;
        }

        let payload = TargetPayload {
            name: parsed.name,
            address: parsed.url,
            kind: KIND.into(),
            profile_id: None,
            parent_id: None,
            via_agent: None,
            interval_secs: parsed.interval_secs,
            enabled: None,
            tags: parsed.tags,
            group_name: Some(Some(folder)),
            credential: None,
        };
        let input = match payload.validate(&state, None) {
            Ok(input) => input,
            Err(error) => {
                report.message = Some(error.into_parts().1);
                errors += 1;
                reports.push(report);
                continue;
            }
        };
        seen.insert(key);
        if let Some(final_key) = final_key {
            seen.insert(final_key);
        }
        if query.dry_run {
            report.status = "ok";
            created += 1;
        } else {
            match db::targets::create(&state.pool, &state.cipher, &input).await {
                Ok(id) => {
                    report.status = "created";
                    report.id = Some(id);
                    created += 1;
                }
                Err(error) if format!("{error:#}").contains("UNIQUE constraint failed") => {
                    report.status = "skipped";
                    report.message = Some("Already monitored.".into());
                    skipped += 1;
                }
                Err(error) => return Err(ApiError::Internal(error)),
            }
        }
        reports.push(report);
    }

    let status = if query.dry_run { StatusCode::OK } else { StatusCode::CREATED };
    Ok((
        status,
        Json(ImportReport {
            dry_run: query.dry_run,
            total: reports.len(),
            created,
            skipped,
            errors,
            rows: reports,
        }),
    ))
}

struct Row {
    line: usize,
    raw_url: String,
    parsed: Result<Parsed, String>,
}

struct Parsed {
    url: String,
    name: String,
    folder: Option<String>,
    tags: BTreeMap<String, String>,
    interval_secs: Option<u64>,
}

fn csv_text(headers: &HeaderMap, body: &Bytes) -> ApiResult<String> {
    let is_json = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.to_ascii_lowercase().contains("json"));
    if is_json {
        #[derive(Deserialize)]
        struct Body {
            csv: String,
        }
        let parsed: Body = serde_json::from_slice(body)
            .map_err(|_| ApiError::BadRequest("Expected a JSON body {\"csv\": \"...\"}.".into()))?;
        Ok(parsed.csv)
    } else {
        String::from_utf8(body.to_vec())
            .map_err(|_| ApiError::BadRequest("The CSV must be UTF-8 text.".into()))
    }
}

fn parse_row(
    raw_url: &str,
    record: &Vec<String>,
    cell: &dyn Fn(&Vec<String>, &str) -> String,
) -> Result<Parsed, String> {
    if raw_url.is_empty() {
        return Err("The url is empty.".into());
    }
    let with_scheme =
        if raw_url.contains("://") { raw_url.to_string() } else { format!("https://{raw_url}") };
    let url = Url::parse(&with_scheme).map_err(|_| format!("\"{raw_url}\" is not a valid URL."))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("Only http:// and https:// URLs can be monitored.".into());
    }
    let Some(host) = url.host_str().filter(|h| !h.is_empty()) else {
        return Err(format!("\"{raw_url}\" has no host name."));
    };
    let name = Some(cell(record, "name")).filter(|n| !n.is_empty()).unwrap_or_else(|| host.into());
    let folder = Some(cell(record, "folder")).filter(|f| !f.is_empty());
    let mut tags = parse_tags(&cell(record, "tags"))?;
    let expect_text = cell(record, "expect_text");
    if !expect_text.is_empty() {
        tags.insert("expect_text".into(), expect_text);
    }
    let interval = cell(record, "interval");
    let interval_secs = if interval.is_empty() { None } else { Some(parse_interval(&interval)?) };
    Ok(Parsed { url: url.to_string(), name, folder, tags, interval_secs })
}

fn parse_tags(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut tags = BTreeMap::new();
    for part in text.split(';').map(str::trim).filter(|p| !p.is_empty()) {
        let Some((key, value)) = part.split_once('=') else {
            return Err(format!("Tag \"{part}\" must look like key=value."));
        };
        let key = key.trim();
        if key.is_empty() {
            return Err(format!("Tag \"{part}\" has no key."));
        }
        tags.insert(key.to_string(), value.trim().to_string());
    }
    Ok(tags)
}

/// `300`, `300s`, `5m`, `1h`.
fn parse_interval(text: &str) -> Result<u64, String> {
    let text = text.trim().to_lowercase();
    let (digits, factor) = match text.chars().last() {
        Some('s') => (&text[..text.len() - 1], 1),
        Some('m') => (&text[..text.len() - 1], 60),
        Some('h') => (&text[..text.len() - 1], 3600),
        _ => (text.as_str(), 1),
    };
    digits
        .trim()
        .parse::<u64>()
        .ok()
        .and_then(|n| n.checked_mul(factor))
        .ok_or_else(|| format!("\"{text}\" is not a valid interval (use 300, 5m or 1h)."))
}

/// Clé de comparaison : sans schéma, sans « www. », sans fragment ni barre finale.
fn dedup_key(url: &str) -> String {
    let Ok(mut parsed) = Url::parse(url) else { return url.to_lowercase() };
    parsed.set_fragment(None);
    let host = parsed.host_str().unwrap_or_default().to_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host).to_string();
    let path = parsed.path().trim_end_matches('/').to_string();
    let query = parsed.query().map(|q| format!("?{q}")).unwrap_or_default();
    format!("{host}{path}{query}")
}

async fn existing_addresses(state: &AppState) -> ApiResult<HashSet<String>> {
    let targets = db::targets::list(&state.pool, &state.cipher).await?;
    Ok(targets
        .into_iter()
        .filter(|t| t.kind == KIND || t.kind == "webchange")
        .map(|t| dedup_key(&t.address))
        .collect())
}

// ---------------------------------------------------------------- CSV

/// CSV RFC 4180 : guillemets, `""`, sauts de ligne entre guillemets, CRLF, BOM.
fn parse_csv(text: &str) -> Result<Vec<Vec<String>>, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut was_quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' => quoted = false,
                _ => field.push(c),
            }
            continue;
        }
        match c {
            '"' if field.is_empty() && !was_quoted => {
                quoted = true;
                was_quoted = true;
            }
            ',' => {
                row.push(std::mem::take(&mut field));
                was_quoted = false;
            }
            '\r' => {}
            '\n' => {
                row.push(std::mem::take(&mut field));
                was_quoted = false;
                rows.push(std::mem::take(&mut row));
            }
            _ => field.push(c),
        }
    }
    if quoted {
        return Err("The CSV has an unterminated quoted field.".into());
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    // Lignes totalement vides entre les enregistrements : ignorées.
    rows.retain(|r| !(r.len() == 1 && r[0].trim().is_empty()));
    Ok(rows)
}

// ---------------------------------------------------------------- plateforme

struct Detection {
    platform: String,
    final_url: Option<String>,
}

fn detection_client() -> Option<reqwest::Client> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .connect_timeout(Duration::from_secs(5))
        .timeout(FETCH_TIMEOUT)
        .user_agent(concat!("DumbMonit/", env!("CARGO_PKG_VERSION"), " (import)"))
        .build()
        .ok()
}

async fn detect(client: &reqwest::Client, url: &str) -> Detection {
    let platform = platform_for(client, url).await;
    Detection { platform: platform.0, final_url: platform.1 }
}

/// Une requête GET (redirections suivies) puis reconnaissance de la plateforme.
/// Renvoie le nom du dossier et l'URL finale quand la page a répondu.
async fn platform_for(client: &reqwest::Client, url: &str) -> (String, Option<String>) {
    let Ok(mut response) = client.get(url).send().await else {
        return (OTHER_PLATFORM.into(), None);
    };
    let final_url = response.url().clone();
    let headers = response.headers().clone();
    let mut html = Vec::new();
    while let Ok(Some(chunk)) = response.chunk().await {
        html.extend_from_slice(&chunk);
        if html.len() >= MAX_HTML_BYTES {
            break;
        }
    }
    let html = String::from_utf8_lossy(&html);
    let platform = crate::website::platform::detect_platform(&final_url, &headers, &html);
    (platform.label().to_string(), Some(final_url.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_handles_quotes_and_newlines() {
        let rows =
            parse_csv("\u{feff}url,name\r\n\"a.com\",\"My \"\"site\"\"\nline\"\r\n\r\nb.com,\n")
                .unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1][1], "My \"site\"\nline");
        assert_eq!(rows[2], vec!["b.com".to_string(), String::new()]);
        assert!(parse_csv("url\n\"oops").is_err());
    }

    #[test]
    fn interval_and_tags() {
        assert_eq!(parse_interval("5m"), Ok(300));
        assert_eq!(parse_interval("90"), Ok(90));
        assert!(parse_interval("soon").is_err());
        assert_eq!(parse_tags("a=1; b=2").unwrap().len(), 2);
        assert!(parse_tags("oops").is_err());
    }

    #[test]
    fn dedup_ignores_scheme_www_and_slash() {
        assert_eq!(dedup_key("https://www.Example.com/"), dedup_key("http://example.com"));
    }
}
