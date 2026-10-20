//! Graylog, par GELF 1.1 (UDP, TCP ou HTTP).
//!
//! Format (go2docs.graylog.org, « GELF ») : un objet JSON avec `version`,
//! `host`, `short_message`, `timestamp` (secondes Unix, décimales permises),
//! `level` (gravité syslog), et des champs additionnels préfixés `_` (nom en
//! `[\w.-]`, `_id` interdit).
//!
//! * UDP : un message par datagramme ; au-delà de 8192 octets, découpé en
//!   morceaux (`0x1e 0x0f`, identifiant de 8 octets, rang, total ; 128 morceaux
//!   au plus).
//! * TCP : messages terminés par un octet nul, sans compression.
//! * HTTP : un message par requête `POST` (`/gelf` sur l'entrée GELF HTTP).

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Map, Value};

use super::{HttpAuth, Sink, StreamSink, host_port, parse_url, send_datagrams, truncate};
use crate::logs::catalog::{self, DestinationConfig, KindInfo};
use crate::logs::{Record, hostname};

const DEFAULT_PORT: u16 = 12201;
/// Taille maximale d'un datagramme avant découpage.
const CHUNK_THRESHOLD: usize = 8192;
/// Charge utile d'un morceau : le datagramme moins l'en-tête de 12 octets.
const CHUNK_PAYLOAD: usize = CHUNK_THRESHOLD - 12;
const MAX_CHUNKS: usize = 128;
/// Longueur maximale d'un message : assez pour une pile d'appels, assez peu
/// pour tenir en 128 morceaux.
const MAX_MESSAGE: usize = 32 * 1024;

pub fn describe() -> KindInfo {
    KindInfo {
        kind: "gelf",
        label: "Graylog (GELF)",
        summary: "Graylog GELF input over UDP, TCP or HTTP.",
        doc_url: catalog::doc("graylog-gelf"),
        settings: vec![
            catalog::destination(catalog::select(
                "transport",
                "Transport",
                "Must match the GELF input created in Graylog.",
                &["udp", "tcp", "http"],
            )),
            catalog::address(
                "UDP and TCP: host:port (port 12201 if omitted). HTTP: the input URL, for example http://graylog:12201/gelf.",
                "graylog.lan:12201",
            ),
            catalog::destination(catalog::boolean(
                "tls",
                "TLS",
                "TCP only: the GELF TCP input has TLS enabled.",
            )),
            catalog::insecure_tls(),
            catalog::username(),
        ],
        secrets: vec![catalog::password(
            "password",
            "Password",
            "HTTP only, for an input behind an authenticating proxy.",
        )],
    }
}

enum Transport {
    Udp { host: String, port: u16 },
    Tcp(StreamSink),
    Http { client: reqwest::Client, url: reqwest::Url, auth: HttpAuth },
}

pub struct Gelf {
    transport: Transport,
}

pub fn build(config: &DestinationConfig) -> Result<Arc<dyn Sink>, String> {
    let address = config.text("address").ok_or("the address is required")?;
    let transport = match config.text_or("transport", "udp").as_str() {
        "udp" => {
            let (host, port) = host_port(&address, DEFAULT_PORT)?;
            Transport::Udp { host, port }
        }
        "tcp" => {
            let (host, port) = host_port(&address, DEFAULT_PORT)?;
            let tls = if config.flag("tls") {
                Some(super::tls_config(None, config.flag("insecure_tls"))?)
            } else {
                None
            };
            Transport::Tcp(StreamSink::new(host, port, tls))
        }
        "http" => Transport::Http {
            client: super::http_client(config.flag("insecure_tls"))?,
            url: parse_url(&address)?,
            auth: HttpAuth {
                username: config.text("username"),
                password: config.secret("password").map(str::to_string),
                authorization: None,
            },
        },
        other => return Err(format!("unknown GELF transport `{other}`")),
    };
    Ok(Arc::new(Gelf { transport }))
}

/// Nom de champ additionnel admis par GELF.
fn field_name(key: &str) -> String {
    let clean: String = key
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') { c } else { '_' })
        .collect();
    if clean == "id" { "_field_id".into() } else { format!("_{clean}") }
}

/// Le message GELF d'un enregistrement.
pub fn message(record: &Record) -> Value {
    let text = truncate(&record.message, MAX_MESSAGE);
    let (short, full) = match text.split_once('\n') {
        Some((first, _)) => (first, Some(text)),
        None => (text, None),
    };
    let mut object = Map::new();
    object.insert("version".into(), "1.1".into());
    object.insert("host".into(), hostname().into());
    object.insert(
        "short_message".into(),
        if short.trim().is_empty() { "(empty)".into() } else { short.into() },
    );
    if let Some(full) = full {
        object.insert("full_message".into(), full.into());
    }
    let millis = record.ts.timestamp_millis();
    object.insert("timestamp".into(), Value::from(millis as f64 / 1000.0));
    object.insert("level".into(), record.level.syslog_severity().into());
    object.insert("_application".into(), "dumbmonit".into());
    object.insert("_logger".into(), record.target.clone().into());
    object.insert("_kind".into(), record.kind().into());
    for (key, value) in &record.fields {
        // GELF n'admet que chaînes et nombres pour les champs additionnels.
        let value = match value {
            Value::Number(_) | Value::String(_) => value.clone(),
            other => Value::String(other.to_string()),
        };
        object.entry(field_name(key)).or_insert(value);
    }
    Value::Object(object)
}

/// Datagrammes d'un message : lui-même, ou ses morceaux.
pub fn datagrams(payload: Vec<u8>) -> Result<Vec<Vec<u8>>, String> {
    if payload.len() <= CHUNK_THRESHOLD {
        return Ok(vec![payload]);
    }
    let count = payload.len().div_ceil(CHUNK_PAYLOAD);
    if count > MAX_CHUNKS {
        return Err(format!("GELF message too large for UDP ({} bytes)", payload.len()));
    }
    let id = rand::random::<u64>().to_be_bytes();
    Ok(payload
        .chunks(CHUNK_PAYLOAD)
        .enumerate()
        .map(|(index, chunk)| {
            let mut datagram = Vec::with_capacity(12 + chunk.len());
            datagram.extend_from_slice(&[0x1e, 0x0f]);
            datagram.extend_from_slice(&id);
            datagram.push(index as u8);
            datagram.push(count as u8);
            datagram.extend_from_slice(chunk);
            datagram
        })
        .collect())
}

#[async_trait]
impl Sink for Gelf {
    async fn send(&self, batch: &[Arc<Record>]) -> Result<(), String> {
        match &self.transport {
            Transport::Udp { host, port } => {
                let mut all = Vec::new();
                for record in batch {
                    all.extend(datagrams(message(record).to_string().into_bytes())?);
                }
                send_datagrams(host, *port, &all).await
            }
            Transport::Tcp(stream) => {
                let frames = batch
                    .iter()
                    .map(|record| {
                        let mut frame = message(record).to_string().into_bytes();
                        frame.push(0);
                        frame
                    })
                    .collect();
                stream.write_frames(frames).await
            }
            Transport::Http { client, url, auth } => {
                for record in batch {
                    let body = message(record).to_string().into_bytes();
                    super::post(client, url, "application/json", body, auth, &[]).await?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::Level;

    #[test]
    fn message_conforme() {
        let mut record = Record::new(Level::Warn, "probe failed\nstack");
        record.fields.insert("id".into(), Value::from(3));
        record.fields.insert("target id".into(), Value::from("x"));
        record.fields.insert("ok".into(), Value::from(false));
        let gelf = message(&record);
        assert_eq!(gelf["version"], "1.1");
        assert_eq!(gelf["short_message"], "probe failed");
        assert_eq!(gelf["full_message"], "probe failed\nstack");
        assert_eq!(gelf["level"], 4);
        assert!(gelf["timestamp"].is_f64());
        assert_eq!(gelf["_field_id"], 3);
        assert_eq!(gelf["_target_id"], "x");
        assert_eq!(gelf["_ok"], "false");
        assert!(gelf.get("_id").is_none());
    }

    #[test]
    fn decoupage_udp() {
        let payload = vec![b'a'; CHUNK_PAYLOAD * 2 + 10];
        let parts = datagrams(payload).unwrap();
        assert_eq!(parts.len(), 3);
        for (index, part) in parts.iter().enumerate() {
            assert_eq!(&part[..2], &[0x1e, 0x0f]);
            assert_eq!(part[10] as usize, index);
            assert_eq!(part[11], 3);
            assert!(part.len() <= CHUNK_THRESHOLD);
        }
        assert_eq!(&parts[0][2..10], &parts[2][2..10]);
        assert_eq!(datagrams(vec![b'a'; 100]).unwrap().len(), 1);
    }
}
