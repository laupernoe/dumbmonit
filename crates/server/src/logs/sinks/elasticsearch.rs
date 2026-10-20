//! Elasticsearch et OpenSearch, par l'API `_bulk`.
//!
//! `POST /_bulk`, corps NDJSON (`application/x-ndjson`) : une ligne d'action
//! puis une ligne de document, et un saut de ligne final obligatoire. L'action
//! est `create`, la seule qu'accepte un data stream, et qui convient aussi à un
//! index ordinaire. La réponse est 200 même quand des documents sont refusés :
//! c'est `errors: true` qui le dit.
//!
//! Les champs suivent les noms ECS courants (`@timestamp`, `message`,
//! `log.level`, `host.name`, `service.name`) ; les champs propres à
//! l'évènement sont rangés en chaînes sous `dumbmonit.fields`, pour qu'un même
//! nom n'impose jamais deux types au mappage dynamique.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Map, Value, json};

use super::{HttpAuth, Sink, parse_url};
use crate::logs::catalog::{self, DestinationConfig, KindInfo};
use crate::logs::{Record, hostname};

const DEFAULT_INDEX: &str = "dumbmonit-logs";

pub fn describe() -> KindInfo {
    KindInfo {
        kind: "elasticsearch",
        label: "Elasticsearch / OpenSearch",
        summary: "Bulk API (/_bulk) of Elasticsearch or OpenSearch.",
        doc_url: catalog::doc("elasticsearch-opensearch"),
        settings: vec![
            catalog::placeholder(
                catalog::url("url", "URL", "Base address of the cluster; /_bulk is added."),
                "https://elasticsearch:9200",
            ),
            catalog::with_default(
                catalog::placeholder(
                    catalog::text(
                        "index",
                        "Index or data stream",
                        "Documents are sent with the create action, which data streams require.",
                    ),
                    DEFAULT_INDEX,
                ),
                DEFAULT_INDEX,
            ),
            catalog::insecure_tls(),
            catalog::username(),
        ],
        secrets: vec![
            catalog::password("password", "Password", "HTTP Basic password, with the username."),
            catalog::password(
                "api_key",
                "API key",
                "Elasticsearch API key (the encoded value), sent as Authorization: ApiKey.",
            ),
        ],
    }
}

pub struct Elasticsearch {
    client: reqwest::Client,
    url: reqwest::Url,
    index: String,
    auth: HttpAuth,
}

pub fn endpoint(base: &str) -> Result<reqwest::Url, String> {
    let mut url = parse_url(base)?;
    if !url.path().ends_with("/_bulk") {
        let path = format!("{}/_bulk", url.path().trim_end_matches('/'));
        url.set_path(&path);
    }
    Ok(url)
}

pub fn build(config: &DestinationConfig) -> Result<Arc<dyn Sink>, String> {
    let base = config.text("url").ok_or("the URL is required")?;
    let index = config.text_or("index", DEFAULT_INDEX);
    if index.starts_with(['_', '-', '+'])
        || index.contains([' ', '"', '*', '\\', '/', ',', '#', '?', '<', '>', '|'])
        || index != index.to_lowercase()
    {
        return Err(format!("`{index}` is not a valid index name"));
    }
    Ok(Arc::new(Elasticsearch {
        client: super::http_client(config.flag("insecure_tls"))?,
        url: endpoint(&base)?,
        index,
        auth: HttpAuth {
            username: config.text("username"),
            password: config.secret("password").map(str::to_string),
            authorization: config.secret("api_key").map(|key| format!("ApiKey {key}")),
        },
    }))
}

pub fn document(record: &Record) -> Value {
    let fields: Map<String, Value> =
        record.flat_fields().map(|(key, value)| (key.to_string(), Value::String(value))).collect();
    json!({
        "@timestamp": record.ts_rfc3339(),
        "message": record.message,
        "log": { "level": record.level.as_str(), "logger": record.target },
        "host": { "name": hostname() },
        "service": { "name": "dumbmonit" },
        "dumbmonit": { "kind": record.kind(), "fields": fields },
    })
}

pub fn body(index: &str, batch: &[Arc<Record>]) -> Vec<u8> {
    let action = json!({ "create": { "_index": index } }).to_string();
    let mut out = Vec::new();
    for record in batch {
        out.extend_from_slice(action.as_bytes());
        out.push(b'\n');
        out.extend_from_slice(document(record).to_string().as_bytes());
        out.push(b'\n');
    }
    out
}

/// Premier refus d'une réponse `_bulk`, s'il y en a un.
pub fn first_error(response: &str) -> Option<String> {
    let value: Value = serde_json::from_str(response).ok()?;
    if value["errors"] != Value::Bool(true) {
        return None;
    }
    let items = value["items"].as_array()?;
    let refused = items
        .iter()
        .filter_map(|item| item.as_object()?.values().next())
        .find(|op| op.get("error").is_some())?;
    let error = &refused["error"];
    Some(format!(
        "document refused: {} ({})",
        error["reason"].as_str().unwrap_or("no reason given"),
        error["type"].as_str().unwrap_or("error"),
    ))
}

#[async_trait]
impl Sink for Elasticsearch {
    async fn send(&self, batch: &[Arc<Record>]) -> Result<(), String> {
        let response = super::post(
            &self.client,
            &self.url,
            "application/x-ndjson",
            body(&self.index, batch),
            &self.auth,
            &[],
        )
        .await?;
        match first_error(&response) {
            Some(error) => Err(crate::logs::scrub::text(&error)),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logs::Level;

    #[test]
    fn corps_ndjson() {
        let batch = vec![Arc::new(Record::new(Level::Error, "boom"))];
        let text = String::from_utf8(body("logs", &batch)).unwrap();
        let lines: Vec<&str> = text.split_terminator('\n').collect();
        assert_eq!(lines.len(), 2);
        assert!(text.ends_with('\n'));
        let action: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(action["create"]["_index"], "logs");
        let doc: Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(doc["log"]["level"], "error");
        assert_eq!(doc["message"], "boom");
    }

    #[test]
    fn refus_partiel() {
        let response = r#"{"errors":true,"items":[{"create":{"status":201}},{"create":{"status":400,"error":{"type":"mapper_parsing_exception","reason":"bad"}}}]}"#;
        assert_eq!(
            first_error(response).unwrap(),
            "document refused: bad (mapper_parsing_exception)"
        );
        assert!(first_error(r#"{"errors":false,"items":[]}"#).is_none());
    }
}
