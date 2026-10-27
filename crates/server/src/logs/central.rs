//! Journaux centralisés : agents, syslog et traps SNMP aboutissent dans le
//! VictoriaLogs embarqué, par une file bornée et un écrivain unique.
//!
//! Chaque entrée porte les champs `service` (`agent`, `syslog` ou `trap`),
//! `host`, `level`, `origin` (journald, fichier, canal, adresse de l'émetteur)
//! et, quand l'émetteur est rattaché à une cible, `target` (son identifiant) et
//! `target_name`. Si le VictoriaLogs embarqué est éteint, les entrées sont
//! comptées puis perdues : un récepteur ne doit jamais bloquer un émetteur.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde_json::{Map, Value};
use tokio::sync::mpsc;

use crate::state::AppState;

/// Entrées en attente d'écriture, au plus.
const QUEUE: usize = 20_000;
/// Entrées par requête d'écriture, au plus.
const BATCH: usize = 500;

/// Une entrée à écrire.
#[derive(Debug, Clone)]
pub struct Entry {
    pub ts_ms: i64,
    /// `agent`, `syslog` ou `trap`.
    pub service: &'static str,
    /// `error`, `warn`, `info` ou `debug`.
    pub level: &'static str,
    pub host: String,
    pub origin: String,
    pub message: String,
    pub target: Option<(i64, String)>,
    pub extra: BTreeMap<String, String>,
}

impl Entry {
    /// La ligne JSON attendue par `/insert/jsonline`.
    pub fn to_json(&self) -> Value {
        let mut object = Map::new();
        let time = chrono::DateTime::from_timestamp_millis(self.ts_ms)
            .unwrap_or_else(chrono::Utc::now)
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        object.insert("_time".into(), time.into());
        object.insert("_msg".into(), self.message.clone().into());
        object.insert("level".into(), self.level.into());
        object.insert("service".into(), self.service.into());
        object.insert("host".into(), self.host.clone().into());
        object.insert("origin".into(), self.origin.clone().into());
        if let Some((id, name)) = &self.target {
            object.insert("target".into(), id.to_string().into());
            object.insert("target_name".into(), name.clone().into());
        }
        for (key, value) in &self.extra {
            let key = if ["_msg", "_time", "level", "service", "host", "origin", "target"]
                .contains(&key.as_str())
            {
                format!("field.{key}")
            } else {
                key.clone()
            };
            object.entry(key).or_insert_with(|| Value::String(value.clone()));
        }
        Value::Object(object)
    }
}

/// Compteurs d'un récepteur (syslog ou traps), lus par `/api/logs/inputs`.
pub struct Input {
    pub received: AtomicU64,
    pub rejected: AtomicU64,
    pub unmatched: AtomicU64,
    pub last_ms: AtomicI64,
    listening: Mutex<Vec<String>>,
    error: Mutex<Option<String>>,
}

impl Input {
    const fn new() -> Self {
        Self {
            received: AtomicU64::new(0),
            rejected: AtomicU64::new(0),
            unmatched: AtomicU64::new(0),
            last_ms: AtomicI64::new(0),
            listening: Mutex::new(Vec::new()),
            error: Mutex::new(None),
        }
    }

    pub fn note_received(&self) {
        self.received.fetch_add(1, Ordering::Relaxed);
        self.last_ms.store(chrono::Utc::now().timestamp_millis(), Ordering::Relaxed);
    }

    pub fn note_rejected(&self) {
        self.rejected.fetch_add(1, Ordering::Relaxed);
    }

    pub fn note_unmatched(&self) {
        self.unmatched.fetch_add(1, Ordering::Relaxed);
    }

    pub fn set_listening(&self, addresses: Vec<String>) {
        if let Ok(mut held) = self.listening.lock() {
            *held = addresses;
        }
    }

    pub fn set_error(&self, error: Option<String>) {
        if let Ok(mut held) = self.error.lock() {
            *held = error;
        }
    }

    pub fn listening(&self) -> Vec<String> {
        self.listening.lock().map(|held| held.clone()).unwrap_or_default()
    }

    pub fn error(&self) -> Option<String> {
        self.error.lock().ok().and_then(|held| held.clone())
    }
}

static SYSLOG: Input = Input::new();
static TRAPS: Input = Input::new();

pub fn syslog_input() -> &'static Input {
    &SYSLOG
}

pub fn traps_input() -> &'static Input {
    &TRAPS
}

/// Ce que le concentrateur a fait des entrées reçues.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counters {
    pub written: u64,
    pub dropped_queue_full: u64,
    pub dropped_store_off: u64,
    pub failed: u64,
}

pub struct Central {
    tx: mpsc::Sender<Entry>,
    rx: Mutex<Option<mpsc::Receiver<Entry>>>,
    written: AtomicU64,
    dropped_full: AtomicU64,
    dropped_off: AtomicU64,
    failed: AtomicU64,
}

static CENTRAL: OnceLock<Central> = OnceLock::new();

pub fn central() -> &'static Central {
    CENTRAL.get_or_init(|| {
        let (tx, rx) = mpsc::channel(QUEUE);
        Central {
            tx,
            rx: Mutex::new(Some(rx)),
            written: AtomicU64::new(0),
            dropped_full: AtomicU64::new(0),
            dropped_off: AtomicU64::new(0),
            failed: AtomicU64::new(0),
        }
    })
}

impl Central {
    /// Remet une entrée à l'écrivain, sans attendre.
    pub fn push(&self, entry: Entry) {
        if self.tx.try_send(entry).is_err() {
            self.dropped_full.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn counters(&self) -> Counters {
        Counters {
            written: self.written.load(Ordering::Relaxed),
            dropped_queue_full: self.dropped_full.load(Ordering::Relaxed),
            dropped_store_off: self.dropped_off.load(Ordering::Relaxed),
            failed: self.failed.load(Ordering::Relaxed),
        }
    }
}

/// Corps d'une requête d'écriture : une ligne JSON par entrée.
pub fn body(entries: &[Entry]) -> Vec<u8> {
    let mut out = Vec::new();
    for entry in entries {
        if let Ok(line) = serde_json::to_vec(&entry.to_json()) {
            out.extend_from_slice(&line);
            out.push(b'\n');
        }
    }
    out
}

/// Lance l'écrivain. Sans effet une seconde fois.
pub fn spawn(state: AppState) {
    let central = central();
    let Some(mut rx) = central.rx.lock().ok().and_then(|mut held| held.take()) else { return };
    tokio::spawn(async move {
        let client = match reqwest::Client::builder().timeout(Duration::from_secs(15)).build() {
            Ok(client) => client,
            Err(error) => {
                tracing::error!(%error, "central log writer not started");
                return;
            }
        };
        let mut batch: Vec<Entry> = Vec::new();
        let mut warned = false;
        loop {
            batch.clear();
            if rx.recv_many(&mut batch, BATCH).await == 0 {
                return;
            }
            let count = batch.len() as u64;
            let Some(base) = crate::storage::logs_url(&state).await else {
                central.dropped_off.fetch_add(count, Ordering::Relaxed);
                tokio::time::sleep(Duration::from_millis(500)).await;
                continue;
            };
            let sent = match super::sinks::victorialogs::endpoint(&base, "service,host") {
                Ok(url) => client.post(url).body(body(&batch)).send().await,
                Err(error) => {
                    tracing::warn!(error, "central log store address invalid");
                    central.failed.fetch_add(count, Ordering::Relaxed);
                    continue;
                }
            };
            match sent {
                Ok(response) if response.status().is_success() => {
                    central.written.fetch_add(count, Ordering::Relaxed);
                    warned = false;
                }
                other => {
                    central.failed.fetch_add(count, Ordering::Relaxed);
                    if !warned {
                        let why = match other {
                            Ok(response) => format!("HTTP {}", response.status()),
                            Err(error) => error.to_string(),
                        };
                        tracing::warn!(why, "central log store refused or unreachable");
                        warned = true;
                    }
                }
            }
            // Laisse le temps aux entrées voisines de se regrouper en un lot.
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    });
}

/// Vrai si la requête contient un `|` hors des guillemets : un traitement
/// (`| stats …`) changerait la forme des réponses, la page n'accepte que des
/// filtres.
pub fn has_pipe(query: &str) -> bool {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for c in query.chars() {
        match quote {
            Some(q) => {
                if escaped {
                    escaped = false;
                } else if c == '\\' && q != '`' {
                    escaped = true;
                } else if c == q {
                    quote = None;
                }
            }
            None => match c {
                '"' | '\'' | '`' => quote = Some(c),
                '|' => return true,
                _ => {}
            },
        }
    }
    false
}

/// Niveau d'une gravité syslog (RFC 5424, §6.2.1).
pub fn level_of_severity(severity: u8) -> &'static str {
    match severity {
        0..=3 => "error",
        4 => "warn",
        5 | 6 => "info",
        _ => "debug",
    }
}

/// Niveau reconnu (`error`, `warn`, `info`, `debug`) ; `info` à défaut.
pub fn normalise_level(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "error" | "err" | "critical" | "crit" | "fatal" | "alert" | "emerg" => "error",
        "warn" | "warning" => "warn",
        "debug" | "trace" => "debug",
        _ => "info",
    }
}

/// Réglage booléen d'environnement (`1`, `true`, `yes`, `on`).
pub fn env_flag(name: &str, default: bool) -> bool {
    match std::env::var(name).map(|v| v.trim().to_ascii_lowercase()) {
        Ok(v) if ["1", "true", "yes", "on"].contains(&v.as_str()) => true,
        Ok(v) if ["0", "false", "no", "off"].contains(&v.as_str()) => false,
        _ => default,
    }
}

/// Port d'écoute d'environnement.
pub fn env_port(name: &str, default: u16) -> u16 {
    std::env::var(name).ok().and_then(|v| v.trim().parse::<u16>().ok()).unwrap_or(default)
}

/// Adresse d'écoute d'environnement.
pub fn env_bind(name: &str) -> std::net::IpAddr {
    std::env::var(name)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> Entry {
        Entry {
            ts_ms: 1_700_000_000_000,
            service: "syslog",
            level: "warn",
            host: "nas".into(),
            origin: "udp:10.0.0.5".into(),
            message: "disk failing".into(),
            target: Some((12, "NAS".into())),
            extra: BTreeMap::from([
                ("app".to_string(), "smartd".to_string()),
                ("level".to_string(), "spoof".to_string()),
            ]),
        }
    }

    #[test]
    fn an_entry_is_written_with_its_target_and_cannot_be_spoofed() {
        let json = entry().to_json();
        assert_eq!(json["_msg"], "disk failing");
        assert_eq!(json["service"], "syslog");
        assert_eq!(json["target"], "12");
        assert_eq!(json["target_name"], "NAS");
        assert_eq!(json["level"], "warn");
        assert_eq!(json["field.level"], "spoof");
        assert_eq!(json["app"], "smartd");
        assert!(json["_time"].as_str().unwrap().starts_with("2023-11-14T"));
    }

    #[test]
    fn the_body_has_one_line_per_entry() {
        let text = String::from_utf8(body(&[entry(), entry()])).unwrap();
        assert_eq!(text.lines().count(), 2);
    }

    #[test]
    fn severities_map_to_the_four_levels() {
        assert_eq!(level_of_severity(0), "error");
        assert_eq!(level_of_severity(4), "warn");
        assert_eq!(level_of_severity(6), "info");
        assert_eq!(level_of_severity(7), "debug");
        assert_eq!(normalise_level("WARNING"), "warn");
        assert_eq!(normalise_level("whatever"), "info");
    }
}
