//! VictoriaLogs, par son API d'ingestion JSON lines.
//!
//! `POST /insert/jsonline` (docs.victoriametrics.com/victorialogs/data-ingestion/) :
//! un objet JSON par ligne. Les paramètres de requête disent quel champ porte
//! le message (`_msg_field`), l'horodatage (`_time_field`, RFC 3339) et quels
//! champs forment le flux (`_stream_fields`).

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Map, Value};

use super::{HttpAuth, Sink, parse_url};
use crate::logs::catalog::{self, DestinationConfig, KindInfo};
use crate::logs::{Record, hostname};

const DEFAULT_STREAM_FIELDS: &str = "service,host";
/// Champs posés par l'expéditeur ; un champ d'évènement du même nom est
/// rangé sous `field.<nom>`.
const RESERVED: [&str; 6] = ["_msg", "_time", "level", "service", "host", "kind"];

pub fn describe() -> KindInfo {
    KindInfo {
        kind: "victorialogs",
        label: "VictoriaLogs",
        summary: "VictoriaLogs JSON lines ingestion (/insert/jsonline).",
        doc_url: catalog::doc("victorialogs"),
        settings: vec![
            catalog::placeholder(
                catalog::url(
                    "url",
                    "URL",
                    "Base address of VictoriaLogs; /insert/jsonline is added.",
                ),
                "http://victorialogs:9428",
            ),
            catalog::with_default(
                catalog::placeholder(
                    catalog::text(
                        "stream_fields",
                        "Stream fields",
                        "Comma-separated fields that identify a log stream.",
                    ),
                    DEFAULT_STREAM_FIELDS,
                ),
                DEFAULT_STREAM_FIELDS,
            ),
            catalog::insecure_tls(),
            catalog::username(),
        ],
        secrets: vec![
            catalog::password("password", "Password", "HTTP Basic password, with the username."),
            catalog::password(
                "token",
                "Bearer token",
                "Sent as Authorization: Bearer, for a proxy in front of VictoriaLogs.",
            ),
        ],
    }
}

pub struct VictoriaLogs {
    client: reqwest::Client,
    url: reqwest::Url,
    auth: HttpAuth,
}

/// Adresse d'ingestion : la base complétée, sauf si l'utilisateur a déjà
/// donné le chemin.
pub fn endpoint(base: &str, stream_fields: &str) -> Result<reqwest::Url, String> {
    let mut url = parse_url(base)?;
    if !url.path().contains("/insert/") {
        let path = format!("{}/insert/jsonline", url.path().trim_end_matches('/'));
        url.set_path(&path);
    }
    url.query_pairs_mut()
        .append_pair("_msg_field", "_msg")
        .append_pair("_time_field", "_time")
        .append_pair("_stream_fields", stream_fields);
    Ok(url)
}

pub fn build(config: &DestinationConfig) -> Result<Arc<dyn Sink>, String> {
    let base = config.text("url").ok_or("the URL is required")?;
    let stream_fields = config.text_or("stream_fields", DEFAULT_STREAM_FIELDS);
    Ok(Arc::new(VictoriaLogs {
        client: super::http_client(config.flag("insecure_tls"))?,
        url: endpoint(&base, &stream_fields)?,
        auth: HttpAuth {
            username: config.text("username"),
            password: config.secret("password").map(str::to_string),
            authorization: config.secret("token").map(|token| format!("Bearer {token}")),
        },
    }))
}

/// La ligne d'un enregistrement.
pub fn line(record: &Record) -> Value {
    let mut object = Map::new();
    object.insert("_time".into(), record.ts_rfc3339().into());
    object.insert("_msg".into(), record.message.clone().into());
    object.insert("level".into(), record.level.as_str().into());
    object.insert("service".into(), "dumbmonit".into());
    object.insert("host".into(), hostname().into());
    object.insert("kind".into(), record.kind().into());
    object.insert("logger".into(), record.target.clone().into());
    for (key, value) in record.flat_fields() {
        let key = if RESERVED.contains(&key) || key == "logger" {
            format!("field.{key}")
        } else {
            key.to_string()
        };
        object.entry(key).or_insert(Value::String(value));
    }
    Value::Object(object)
}

pub fn body(batch: &[Arc<Record>]) -> Vec<u8> {
    let mut out = Vec::new();
    for record in batch {
        out.extend_from_slice(line(record).to_string().as_bytes());
        out.push(b'\n');
    }
    out
}

#[async_trait]
impl Sink for VictoriaLogs {
    async fn send(&self, batch: &[Arc<Record>]) -> Result<(), String> {
        super::post(
            &self.client,
            &self.url,
            "application/stream+json",
            body(batch),
            &self.auth,
            &[],
        )
        .await
        .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adresse_d_ingestion() {
        let url = endpoint("http://vl:9428/", "service,host").unwrap();
        assert_eq!(url.path(), "/insert/jsonline");
        assert!(url.query().unwrap().contains("_stream_fields=service%2Chost"));
        let given = endpoint("http://proxy/vl/insert/jsonline", "service").unwrap();
        assert_eq!(given.path(), "/vl/insert/jsonline");
    }
}
