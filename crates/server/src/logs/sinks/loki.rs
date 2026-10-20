//! Grafana Loki, par son API de poussée.
//!
//! `POST /loki/api/v1/push` (grafana.com/docs/loki/latest/reference/loki-http-api/) :
//! `{"streams":[{"stream":{étiquettes},"values":[["<ns Unix en chaîne>","<ligne>"]]}]}`.
//! Les étiquettes restent peu nombreuses (service, hôte, niveau, nature) : Loki
//! indexe chaque combinaison. La ligne est l'enregistrement JSON complet, que
//! `| json` relit dans LogQL.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use super::{HttpAuth, Sink, parse_url};
use crate::logs::catalog::{self, DestinationConfig, KindInfo};
use crate::logs::{Record, hostname};

const PUSH_PATH: &str = "/loki/api/v1/push";

pub fn describe() -> KindInfo {
    KindInfo {
        kind: "loki",
        label: "Grafana Loki",
        summary: "Loki push API (/loki/api/v1/push), Grafana Cloud included.",
        doc_url: catalog::doc("grafana-loki"),
        settings: vec![
            catalog::placeholder(
                catalog::url("url", "URL", "Base address of Loki; /loki/api/v1/push is added."),
                "http://loki:3100",
            ),
            catalog::destination(catalog::text(
                "tenant",
                "Tenant (X-Scope-OrgID)",
                "Only for a multi-tenant Loki.",
            )),
            catalog::insecure_tls(),
            catalog::username(),
        ],
        secrets: vec![
            catalog::password(
                "password",
                "Password",
                "HTTP Basic password (on Grafana Cloud: an access policy token).",
            ),
            catalog::password("token", "Bearer token", "Sent as Authorization: Bearer."),
        ],
    }
}

pub struct Loki {
    client: reqwest::Client,
    url: reqwest::Url,
    auth: HttpAuth,
    tenant: Option<String>,
}

pub fn endpoint(base: &str) -> Result<reqwest::Url, String> {
    let mut url = parse_url(base)?;
    if !url.path().ends_with(PUSH_PATH) {
        let path = format!("{}{PUSH_PATH}", url.path().trim_end_matches('/'));
        url.set_path(&path);
    }
    Ok(url)
}

pub fn build(config: &DestinationConfig) -> Result<Arc<dyn Sink>, String> {
    let base = config.text("url").ok_or("the URL is required")?;
    Ok(Arc::new(Loki {
        client: super::http_client(config.flag("insecure_tls"))?,
        url: endpoint(&base)?,
        auth: HttpAuth {
            username: config.text("username"),
            password: config.secret("password").map(str::to_string),
            authorization: config.secret("token").map(|token| format!("Bearer {token}")),
        },
        tenant: config.text("tenant"),
    }))
}

/// Le corps d'une poussée : un flux par combinaison d'étiquettes.
pub fn body(batch: &[Arc<Record>]) -> Value {
    let mut streams: BTreeMap<(&'static str, &'static str), Vec<Value>> = BTreeMap::new();
    for record in batch {
        let nanos = record.ts.timestamp_nanos_opt().unwrap_or_default();
        streams
            .entry((record.level.as_str(), record.kind()))
            .or_default()
            .push(json!([nanos.to_string(), record.to_line()]));
    }
    let streams: Vec<Value> = streams
        .into_iter()
        .map(|((level, kind), values)| {
            json!({
                "stream": {
                    "service_name": "dumbmonit",
                    "host": hostname(),
                    "level": level,
                    "kind": kind,
                },
                "values": values,
            })
        })
        .collect();
    json!({ "streams": streams })
}

#[async_trait]
impl Sink for Loki {
    async fn send(&self, batch: &[Arc<Record>]) -> Result<(), String> {
        let headers: Vec<(&str, String)> =
            self.tenant.iter().map(|tenant| ("X-Scope-OrgID", tenant.clone())).collect();
        let body = body(batch).to_string().into_bytes();
        super::post(&self.client, &self.url, "application/json", body, &self.auth, &headers)
            .await
            .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::Level;

    #[test]
    fn flux_par_niveau() {
        let batch = vec![
            Arc::new(Record::new(Level::Info, "a")),
            Arc::new(Record::new(Level::Warn, "b")),
            Arc::new(Record::new(Level::Info, "c")),
        ];
        let body = body(&batch);
        let streams = body["streams"].as_array().unwrap();
        assert_eq!(streams.len(), 2);
        let info = streams.iter().find(|s| s["stream"]["level"] == "info").unwrap();
        assert_eq!(info["values"].as_array().unwrap().len(), 2);
        assert!(info["values"][0][0].as_str().unwrap().len() >= 19);
        assert_eq!(endpoint("http://loki:3100").unwrap().path(), PUSH_PATH);
    }
}
