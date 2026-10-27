//! Les sources de lignes : journal systemd, fichiers suivis, Journal
//! d'événements Windows. Chacune pousse dans une file bornée ; quand elle est
//! pleine, la ligne est perdue plutôt que de ralentir la machine surveillée.

use std::collections::HashMap;
use std::io::SeekFrom;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use dumbmonit_proto::{LogLine, MAX_LOG_MESSAGE, level_rank};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::Sender;
use tracing::{debug, warn};

use super::glob;

pub type Out = Sender<LogLine>;

/// Lecture maximale par fichier et par tour de scrutation.
const MAX_READ: u64 = 1024 * 1024;
/// Scrutation des fichiers.
const FILE_POLL: Duration = Duration::from_secs(2);
/// Scrutation du Journal d'événements Windows.
const EVENTLOG_POLL: Duration = Duration::from_secs(15);

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Message borné à [`MAX_LOG_MESSAGE`] octets, coupé sur une frontière de caractère.
pub fn clip(mut message: String) -> String {
    if message.len() > MAX_LOG_MESSAGE {
        let mut end = MAX_LOG_MESSAGE;
        while !message.is_char_boundary(end) {
            end -= 1;
        }
        message.truncate(end);
    }
    message
}

fn emit(out: &Out, min_rank: u8, line: LogLine) {
    if level_rank(&line.level) > min_rank || line.message.trim().is_empty() {
        return;
    }
    // File pleine : la ligne est perdue, jamais la machine ralentie.
    let _ = out.try_send(line);
}

// --------------------------------------------------------------------------
// journald
// --------------------------------------------------------------------------

/// Une entrée `journalctl -o json`.
pub fn parse_journal_line(raw: &str) -> Option<LogLine> {
    let value: Value = serde_json::from_str(raw).ok()?;
    let message = match value.get("MESSAGE")? {
        Value::String(text) => text.clone(),
        // Message binaire : tableau d'octets.
        Value::Array(bytes) => {
            let bytes: Vec<u8> = bytes.iter().filter_map(|b| b.as_u64()).map(|b| b as u8).collect();
            String::from_utf8_lossy(&bytes).into_owned()
        }
        _ => return None,
    };
    let priority = value
        .get("PRIORITY")
        .and_then(Value::as_str)
        .and_then(|p| p.parse::<u8>().ok())
        .unwrap_or(6);
    let level = match priority {
        0..=3 => "error",
        4 => "warn",
        5 | 6 => "info",
        _ => "debug",
    };
    let micros: i64 = value
        .get("__REALTIME_TIMESTAMP")
        .and_then(Value::as_str)
        .and_then(|t| t.parse().ok())
        .unwrap_or(0);
    let unit = value
        .get("_SYSTEMD_UNIT")
        .or_else(|| value.get("SYSLOG_IDENTIFIER"))
        .and_then(Value::as_str)
        .unwrap_or("system");
    Some(LogLine {
        ts_ms: if micros > 0 { micros / 1000 } else { now_ms() },
        level: level.to_string(),
        source: format!("journald:{unit}"),
        message: clip(message),
    })
}

fn journal_priority(min_level: &str) -> &'static str {
    match level_rank(min_level) {
        0 => "err",
        1 => "warning",
        2 => "info",
        _ => "debug",
    }
}

pub async fn run_journald(out: Out, min_level: String) {
    let min_rank = level_rank(&min_level);
    let mut warned = false;
    loop {
        let spawned = Command::new("journalctl")
            .args(["-f", "-o", "json", "--no-pager", "-n", "0", "-p", journal_priority(&min_level)])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn();
        match spawned {
            Ok(mut child) => {
                if let Some(stdout) = child.stdout.take() {
                    let mut lines = BufReader::new(stdout).lines();
                    while let Ok(Some(raw)) = lines.next_line().await {
                        if let Some(line) = parse_journal_line(&raw) {
                            emit(&out, min_rank, line);
                        }
                    }
                }
                let _ = child.wait().await;
                debug!("journalctl ended, restarting");
            }
            Err(error) => {
                if !warned {
                    warn!(%error, "journald log collection unavailable (journalctl not found?)");
                    warned = true;
                }
                tokio::time::sleep(Duration::from_secs(300)).await;
            }
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

// --------------------------------------------------------------------------
// Fichiers
// --------------------------------------------------------------------------

/// Niveau deviné d'une ligne de fichier, d'après ses mots.
pub fn guess_level(line: &str) -> &'static str {
    let lower = line.to_ascii_lowercase();
    if lower.contains("error")
        || lower.contains("fatal")
        || lower.contains("critical")
        || lower.contains("panic")
    {
        "error"
    } else if lower.contains("warn") {
        "warn"
    } else if lower.contains("debug") {
        "debug"
    } else {
        "info"
    }
}

struct Tail {
    offset: u64,
    ident: u64,
    carry: Vec<u8>,
}

#[cfg(unix)]
fn identity_of(meta: &std::fs::Metadata) -> u64 {
    std::os::unix::fs::MetadataExt::ino(meta)
}

#[cfg(not(unix))]
fn identity_of(_meta: &std::fs::Metadata) -> u64 {
    0
}

fn push_lines(tail: &mut Tail, path: &str, out: &Out, min_rank: u8) {
    while let Some(position) = tail.carry.iter().position(|b| *b == b'\n') {
        let raw: Vec<u8> = tail.carry.drain(..=position).collect();
        let text = String::from_utf8_lossy(&raw).trim_end().to_string();
        if text.is_empty() {
            continue;
        }
        let level = guess_level(&text).to_string();
        emit(
            out,
            min_rank,
            LogLine { ts_ms: now_ms(), level, source: format!("file:{path}"), message: clip(text) },
        );
    }
    // Une « ligne » sans fin de ligne qui dépasse tout bon sens : expédiée telle quelle.
    if tail.carry.len() > MAX_LOG_MESSAGE * 2 {
        let text = String::from_utf8_lossy(&tail.carry).to_string();
        tail.carry.clear();
        let level = guess_level(&text).to_string();
        emit(
            out,
            min_rank,
            LogLine { ts_ms: now_ms(), level, source: format!("file:{path}"), message: clip(text) },
        );
    }
}

pub async fn run_files(out: Out, patterns: Vec<String>, min_level: String) {
    let min_rank = level_rank(&min_level);
    let mut tails: HashMap<PathBuf, Tail> = HashMap::new();
    let mut first_scan = true;
    loop {
        let wanted = patterns.clone();
        let mut found: Vec<PathBuf> = tokio::task::spawn_blocking(move || {
            wanted.iter().flat_map(|pattern| glob::expand(pattern)).collect::<Vec<PathBuf>>()
        })
        .await
        .unwrap_or_default();
        found.sort();
        found.dedup();
        tails.retain(|path, _| found.contains(path));
        for path in &found {
            let Ok(meta) = tokio::fs::metadata(path).await else { continue };
            let length = meta.len();
            let ident = identity_of(&meta);
            let tail = tails.entry(path.clone()).or_insert_with(|| Tail {
                // Au démarrage, l'historique des fichiers existants n'est pas
                // rejoué ; un fichier apparu ensuite est lu depuis son début.
                offset: if first_scan { length } else { 0 },
                ident,
                carry: Vec::new(),
            });
            if tail.ident != ident || length < tail.offset {
                tail.offset = 0;
                tail.ident = ident;
                tail.carry.clear();
            }
            if length == tail.offset {
                continue;
            }
            let Ok(mut file) = tokio::fs::File::open(path).await else { continue };
            if file.seek(SeekFrom::Start(tail.offset)).await.is_err() {
                continue;
            }
            let mut buffer = Vec::new();
            let wanted_bytes = (length - tail.offset).min(MAX_READ);
            if file.take(wanted_bytes).read_to_end(&mut buffer).await.is_err() {
                continue;
            }
            tail.offset += buffer.len() as u64;
            tail.carry.extend_from_slice(&buffer);
            push_lines(tail, &path.display().to_string(), &out, min_rank);
        }
        first_scan = false;
        tokio::time::sleep(FILE_POLL).await;
    }
}

// --------------------------------------------------------------------------
// Journal d'événements Windows
// --------------------------------------------------------------------------

/// Un nom de canal ne porte que des caractères sûrs : il est inséré dans un
/// script PowerShell.
pub fn valid_channel(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name.chars().all(|c| c.is_ascii_alphanumeric() || " _-./".contains(c))
}

fn eventlog_script(channel: &str, lookback_secs: u64) -> String {
    format!(
        "$ErrorActionPreference='SilentlyContinue'; \
         Get-WinEvent -FilterHashtable @{{LogName='{channel}'; \
         StartTime=(Get-Date).AddSeconds(-{lookback_secs})}} -MaxEvents 300 | \
         ForEach-Object {{ [pscustomobject]@{{ id=$_.RecordId; \
         t=([DateTimeOffset]$_.TimeCreated).ToUnixTimeMilliseconds(); l=$_.Level; \
         p=$_.ProviderName; m=$_.Message }} }} | ConvertTo-Json -Compress"
    )
}

/// Évènements d'un canal : `(identifiant d'enregistrement, ligne)`.
pub fn parse_eventlog(raw: &str, channel: &str) -> Vec<(i64, LogLine)> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Vec::new();
    }
    let Ok(value) = serde_json::from_str::<Value>(raw) else { return Vec::new() };
    let items: Vec<Value> = match value {
        Value::Array(items) => items,
        other => vec![other],
    };
    items
        .iter()
        .filter_map(|item| {
            let id = item.get("id").and_then(Value::as_i64)?;
            let level = match item.get("l").and_then(Value::as_i64) {
                Some(1) | Some(2) => "error",
                Some(3) => "warn",
                Some(0) | Some(5) => "debug",
                _ => "info",
            };
            let provider = item.get("p").and_then(Value::as_str).unwrap_or("");
            let text = item.get("m").and_then(Value::as_str).unwrap_or("");
            let message =
                if provider.is_empty() { text.to_string() } else { format!("{provider}: {text}") };
            Some((
                id,
                LogLine {
                    ts_ms: item.get("t").and_then(Value::as_i64).unwrap_or_else(now_ms),
                    level: level.to_string(),
                    source: format!("eventlog:{channel}"),
                    message: clip(message),
                },
            ))
        })
        .collect()
}

pub async fn run_eventlog(out: Out, channel: String, min_level: String) {
    let min_rank = level_rank(&min_level);
    if !valid_channel(&channel) {
        warn!(channel, "event log channel ignored: unexpected characters");
        return;
    }
    let mut last_id: i64 = 0;
    let mut last_poll: Option<Instant> = None;
    let mut warned = false;
    loop {
        let lookback = last_poll.map_or(30, |at| at.elapsed().as_secs() + 5);
        last_poll = Some(Instant::now());
        let script = eventlog_script(&channel, lookback);
        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script.as_str()])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .output()
            .await;
        match output {
            Ok(output) => {
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                let mut events = parse_eventlog(&text, &channel);
                events.sort_by_key(|(id, _)| *id);
                for (id, line) in events {
                    if id > last_id {
                        last_id = id;
                        emit(&out, min_rank, line);
                    }
                }
            }
            Err(error) => {
                if !warned {
                    warn!(%error, channel, "event log collection unavailable (PowerShell not found?)");
                    warned = true;
                }
            }
        }
        tokio::time::sleep(EVENTLOG_POLL).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_journal_entry_becomes_a_line() {
        let raw = r#"{"MESSAGE":"Failed password for root","PRIORITY":"4","__REALTIME_TIMESTAMP":"1700000000123456","_SYSTEMD_UNIT":"ssh.service"}"#;
        let line = parse_journal_line(raw).unwrap();
        assert_eq!(line.level, "warn");
        assert_eq!(line.source, "journald:ssh.service");
        assert_eq!(line.ts_ms, 1_700_000_000_123);
        assert_eq!(line.message, "Failed password for root");
    }

    #[test]
    fn a_binary_journal_message_is_decoded_lossily() {
        let raw = r#"{"MESSAGE":[104,105],"PRIORITY":"3","SYSLOG_IDENTIFIER":"app"}"#;
        let line = parse_journal_line(raw).unwrap();
        assert_eq!(line.message, "hi");
        assert_eq!(line.level, "error");
        assert_eq!(line.source, "journald:app");
        assert!(parse_journal_line("not json").is_none());
    }

    #[test]
    fn levels_are_guessed_from_the_words() {
        assert_eq!(guess_level("2026/10/26 [error] connect() failed"), "error");
        assert_eq!(guess_level("WARNING: disk"), "warn");
        assert_eq!(guess_level("GET / 200"), "info");
        assert_eq!(guess_level("debug: x"), "debug");
    }

    #[test]
    fn long_messages_are_clipped_on_a_char_boundary() {
        let long = "é".repeat(MAX_LOG_MESSAGE);
        let clipped = clip(long);
        assert!(clipped.len() <= MAX_LOG_MESSAGE);
        assert!(clipped.chars().all(|c| c == 'é'));
    }

    #[test]
    fn channel_names_are_checked_before_reaching_a_script() {
        assert!(valid_channel("System"));
        assert!(valid_channel("Microsoft-Windows-Sysmon/Operational"));
        assert!(!valid_channel("System'; Remove-Item x; '"));
        assert!(!valid_channel(""));
    }

    #[test]
    fn event_log_output_is_read_as_one_object_or_an_array() {
        let one = r#"{"id":5,"t":1700000000000,"l":2,"p":"Disk","m":"bad block"}"#;
        let events = parse_eventlog(one, "System");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].1.level, "error");
        assert_eq!(events[0].1.message, "Disk: bad block");
        assert_eq!(events[0].1.source, "eventlog:System");
        let many = r#"[{"id":1,"t":1,"l":4,"p":"","m":"a"},{"id":2,"t":2,"l":3,"p":"","m":"b"}]"#;
        assert_eq!(parse_eventlog(many, "Application").len(), 2);
        assert!(parse_eventlog("", "System").is_empty());
    }

    #[tokio::test]
    async fn a_followed_file_ships_new_lines_and_survives_truncation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.log");
        std::fs::write(&path, "old line\n").unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);
        let pattern = format!("{}/*.log", dir.path().display());
        let task = tokio::spawn(run_files(tx, vec![pattern], "debug".to_string()));
        tokio::time::sleep(Duration::from_millis(300)).await;
        std::fs::write(&path, "old line\nnew error here\n").unwrap();
        let line = tokio::time::timeout(Duration::from_secs(6), rx.recv()).await.unwrap().unwrap();
        assert_eq!(line.message, "new error here");
        assert_eq!(line.level, "error");
        std::fs::write(&path, "again\n").unwrap();
        let line = tokio::time::timeout(Duration::from_secs(6), rx.recv()).await.unwrap().unwrap();
        assert_eq!(line.message, "again");
        task.abort();
    }
}
