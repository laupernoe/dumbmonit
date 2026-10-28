//! Faux UniFi Network sous UniFi OS (Cloud Key), interrogé avec une clé d'API :
//! l'API d'intégration sous `/proxy/network/integration/v1` et la santé de
//! l'API classique. Le Wi-Fi de la maison : trois points d'accès et un petit
//! commutateur PoE, d'après les réponses documentées des tests du collecteur
//! (`unifi/testdata/documented`).

use axum::Router;
use axum::http::{StatusCode, Uri};
use axum::response::Response;
use serde_json::{Value, json};

use super::synthetic_load;
use super::{json, now_s};

pub(super) const KEY: &str = "demonstration-unifi-key";
const PREFIX: &str = "/proxy/network";
const SITE: &str = "88f7af54-98f8-306a-a1c7-c9349722b1f6";

/// (identifiant, MAC, IP, nom, modèle, fonction, micrologiciel, à mettre à jour).
type Device = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    bool,
);

const DEVICES: &[Device] = &[
    (
        "1f0e0000-0000-4000-8000-0000000000a1",
        "f0:9f:c2:91:50:77",
        "192.168.1.50",
        "ap-livingroom",
        "U7 Pro",
        "accessPoint",
        "8.1.7",
        false,
    ),
    (
        "1f0e0000-0000-4000-8000-0000000000a2",
        "f0:9f:c2:91:50:78",
        "192.168.1.51",
        "ap-office",
        "U6 Lite",
        "accessPoint",
        "6.7.31",
        true,
    ),
    (
        "1f0e0000-0000-4000-8000-0000000000a3",
        "f0:9f:c2:91:50:79",
        "192.168.1.52",
        "ap-garden",
        "U6 Mesh",
        "accessPoint",
        "6.7.31",
        true,
    ),
    (
        "1f0e0000-0000-4000-8000-0000000000a4",
        "f0:9f:c2:91:50:7a",
        "192.168.1.4",
        "switch-office",
        "USW Lite 8 PoE",
        "switching",
        "7.1.26",
        false,
    ),
];

pub(super) fn router() -> Router {
    Router::new().fallback(handle)
}

fn page(data: Vec<Value>, total: usize) -> Response {
    let body = json!({"offset": 0, "limit": 200, "count": data.len(), "totalCount": total,
        "data": data});
    json(StatusCode::OK, body.to_string())
}

async fn handle(uri: Uri) -> Response {
    let Some(path) = uri.path().strip_prefix(PREFIX) else {
        return json(StatusCode::NOT_FOUND, "{}".into());
    };
    let path = path.trim_end_matches('/');
    let devices = format!("/integration/v1/sites/{SITE}/devices");
    if path == "/integration/v1/info" {
        return json(StatusCode::OK, json!({"applicationVersion": "10.0.160"}).to_string());
    }
    if path == "/integration/v1/sites" {
        return page(vec![json!({"id": SITE, "internalReference": "default", "name": "Home"})], 1);
    }
    if path == devices {
        let rows = DEVICES
            .iter()
            .map(|(id, mac, ip, name, model, feature, firmware, upgradable)| {
                let interfaces = if *feature == "accessPoint" { "radios" } else { "ports" };
                json!({"id": id, "macAddress": mac, "ipAddress": ip, "name": name,
                    "model": model, "state": "ONLINE", "supported": true,
                    "firmwareVersion": firmware, "firmwareUpdatable": upgradable,
                    "features": [feature], "interfaces": [interfaces]})
            })
            .collect();
        return page(rows, DEVICES.len());
    }
    if let Some(rest) = path.strip_prefix(&format!("{devices}/"))
        && let Some(id) = rest.strip_suffix("/statistics/latest")
    {
        return statistics(id);
    }
    if path == format!("/integration/v1/sites/{SITE}/clients") {
        return page(vec![json!({"type": "WIRELESS"})], 31);
    }
    match path {
        "/api/s/default/stat/health" => json(StatusCode::OK, health().to_string()),
        "/api/s/default/rest/user" => {
            json(StatusCode::OK, json!({"meta": {"rc": "ok"}, "data": []}).to_string())
        }
        _ => json(StatusCode::NOT_FOUND, "{}".into()),
    }
}

/// Charge d'un équipement, qui suit la journée comme le reste du parc.
fn statistics(id: &str) -> Response {
    let load = synthetic_load(id);
    let now = now_s();
    let heartbeat = chrono::DateTime::from_timestamp(now - 12, 0).unwrap_or_default();
    let body = json!({
        "uptimeSec": 2_419_200 + now.rem_euclid(86_400),
        "lastHeartbeatAt": heartbeat.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "nextHeartbeatAt": (heartbeat + chrono::Duration::seconds(30))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "loadAverage1Min": 0.2 + 0.8 * load, "loadAverage5Min": 0.2 + 0.7 * load,
        "loadAverage15Min": 0.2 + 0.6 * load,
        "cpuUtilizationPct": ((4.0 + 30.0 * load) * 10.0).round() / 10.0,
        "memoryUtilizationPct": (520.0 + 120.0 * load).round() / 10.0,
        "uplink": {"txRateBps": (1.2e6 * (0.3 + load)).round() as u64,
            "rxRateBps": (9.5e6 * (0.3 + load)).round() as u64}
    });
    json(StatusCode::OK, body.to_string())
}

fn health() -> Value {
    json!({"meta": {"rc": "ok"}, "data": [
        {"subsystem": "wlan", "status": "ok", "num_ap": 3, "num_adopted": 3,
            "num_disabled": 0, "num_disconnected": 0, "num_pending": 0, "num_user": 24,
            "num_guest": 2, "num_iot": 5},
        {"subsystem": "lan", "status": "ok", "num_sw": 1, "num_adopted": 1,
            "num_disconnected": 0, "num_pending": 0, "num_user": 9, "num_guest": 0,
            "num_iot": 0}
    ]})
}
