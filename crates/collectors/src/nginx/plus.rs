//! Lecture optionnelle de l'API NGINX Plus : santé des upstreams et compteurs
//! par zone de serveur.
//!
//! Réservée à la version commerciale : son absence (404, ou l'option
//! `plus_api` éteinte) n'est jamais une erreur, `stub_status` suffit à noter
//! l'équipement « up » ou non.
//!
//! <https://nginx.org/en/docs/http/ngx_http_api_module.html>

use std::collections::BTreeMap;

use dumbmonit_proto::{MetricKind, ProbeError, Sample};
use serde::Deserialize;

use crate::observability::client::HttpClient;

#[derive(Debug, Clone, Deserialize)]
pub struct UpstreamPeer {
    pub server: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub active: u64,
    #[serde(default)]
    pub requests: u64,
    #[serde(default)]
    pub fails: u64,
    #[serde(default)]
    pub responses: Responses,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Upstream {
    #[serde(default)]
    pub peers: Vec<UpstreamPeer>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Responses {
    #[serde(rename = "5xx", default)]
    pub five_xx: u64,
    #[serde(default)]
    pub total: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerZone {
    #[serde(default)]
    pub requests: u64,
    #[serde(default)]
    pub responses: Responses,
    #[serde(default)]
    pub received: u64,
    #[serde(default)]
    pub sent: u64,
}

/// Lit `/http/upstreams` et `/http/server_zones`. Un 404 sur l'un ou l'autre
/// (API présente mais sans upstream déclaré, ou sans zone nommée) n'écarte que
/// la famille concernée.
pub async fn read(
    client: &HttpClient,
    version: u32,
    ts_ms: i64,
) -> Result<Vec<Sample>, ProbeError> {
    let mut samples = Vec::new();
    if let Some(upstreams) = get(client, &format!("/api/{version}/http/upstreams")).await? {
        samples.extend(upstream_samples(&upstreams, ts_ms));
    }
    if let Some(zones) = get(client, &format!("/api/{version}/http/server_zones")).await? {
        samples.extend(zone_samples(&zones, ts_ms));
    }
    Ok(samples)
}

/// Un `GET` JSON tolérant à l'absence du point d'accès (404 : pas de NGINX
/// Plus, ou famille non configurée) mais pas au reste — un 401 doit être une
/// vraie erreur d'authentification.
async fn get<T: serde::de::DeserializeOwned>(
    client: &HttpClient,
    path: &str,
) -> Result<Option<T>, ProbeError> {
    let reply = client.get_json_raw(path).await?;
    if reply.status == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !reply.status.is_success() {
        return Err(client.status_error(reply.status, &reply.body, path));
    }
    serde_json::from_str(&reply.body)
        .map(Some)
        .map_err(|error| ProbeError::Protocol(format!("Unexpected response from {path}: {error}")))
}

fn gauge(metric: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(metric, value, MetricKind::Gauge, ts_ms)
}

fn counter(metric: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(metric, value, MetricKind::Counter, ts_ms)
}

fn upstream_samples(upstreams: &BTreeMap<String, Upstream>, ts_ms: i64) -> Vec<Sample> {
    let mut samples = Vec::new();
    for (name, upstream) in upstreams {
        for peer in &upstream.peers {
            let labelled = |sample: Sample| {
                sample.with_label("upstream", name).with_label("server", &peer.server)
            };
            samples.push(labelled(gauge(
                "nginx_plus_upstream_server_up",
                f64::from(u8::from(peer.state == "up")),
                ts_ms,
            )));
            samples.push(labelled(gauge(
                "nginx_plus_upstream_server_active",
                peer.active as f64,
                ts_ms,
            )));
            samples.push(labelled(counter(
                "nginx_plus_upstream_server_requests_total",
                peer.requests as f64,
                ts_ms,
            )));
            samples.push(labelled(counter(
                "nginx_plus_upstream_server_fails_total",
                peer.fails as f64,
                ts_ms,
            )));
            samples.push(labelled(counter(
                "nginx_plus_upstream_server_5xx_total",
                peer.responses.five_xx as f64,
                ts_ms,
            )));
        }
    }
    samples
}

fn zone_samples(zones: &BTreeMap<String, ServerZone>, ts_ms: i64) -> Vec<Sample> {
    let mut samples = Vec::new();
    for (name, zone) in zones {
        let labelled = |sample: Sample| sample.with_label("zone", name);
        samples.push(labelled(counter(
            "nginx_plus_zone_requests_total",
            zone.requests as f64,
            ts_ms,
        )));
        samples.push(labelled(counter(
            "nginx_plus_zone_5xx_total",
            zone.responses.five_xx as f64,
            ts_ms,
        )));
        samples.push(labelled(counter(
            "nginx_plus_zone_received_bytes_total",
            zone.received as f64,
            ts_ms,
        )));
        samples.push(labelled(counter(
            "nginx_plus_zone_sent_bytes_total",
            zone.sent as f64,
            ts_ms,
        )));
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_upstream_nginx_plus_se_decode() {
        let json = r#"{
            "backend": {
                "peers": [
                    {"server": "10.0.0.1:80", "state": "up", "active": 2, "requests": 100, "fails": 1, "responses": {"5xx": 3, "total": 100}},
                    {"server": "10.0.0.2:80", "state": "down", "active": 0, "requests": 0, "fails": 5, "responses": {"5xx": 0, "total": 0}}
                ]
            }
        }"#;
        let upstreams: BTreeMap<String, Upstream> = serde_json::from_str(json).unwrap();
        let samples = upstream_samples(&upstreams, 0);
        let up = samples
            .iter()
            .find(|s| {
                s.metric == "nginx_plus_upstream_server_up"
                    && s.labels.get("server").map(String::as_str) == Some("10.0.0.1:80")
            })
            .unwrap();
        assert_eq!(up.value, 1.0);
        let down = samples
            .iter()
            .find(|s| {
                s.metric == "nginx_plus_upstream_server_up"
                    && s.labels.get("server").map(String::as_str) == Some("10.0.0.2:80")
            })
            .unwrap();
        assert_eq!(down.value, 0.0);
    }

    #[test]
    fn une_zone_se_decode() {
        let json = r#"{"site": {"requests": 500, "responses": {"5xx": 2, "total": 500}, "received": 1000, "sent": 2000}}"#;
        let zones: BTreeMap<String, ServerZone> = serde_json::from_str(json).unwrap();
        let samples = zone_samples(&zones, 0);
        let requests =
            samples.iter().find(|s| s.metric == "nginx_plus_zone_requests_total").unwrap();
        assert_eq!(requests.value, 500.0);
        assert_eq!(requests.labels.get("zone").map(String::as_str), Some("site"));
    }
}
