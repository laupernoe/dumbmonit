//! Faux contrôleur TP-Link Omada (API ouverte) : le réseau du bureau du client
//! PME — une passerelle, un commutateur PoE et deux points d'accès, sur un
//! site. Réponses écrites d'après les captures des tests du collecteur
//! (`omada/testdata`), compteurs des ports qui avancent avec l'heure.

use axum::Router;
use axum::http::{StatusCode, Uri};
use axum::response::Response;
use serde_json::{Value, json};

use super::{json, now_s};

/// Identifiant du contrôleur, publié sans authentification sur `/api/info`.
const OMADAC: &str = "5e11a4c0d2e4b5967788990011aabbcc";
const SWITCH_MAC: &str = "3C-52-A1-20-00-02";

pub(super) fn router() -> Router {
    Router::new().fallback(handle)
}

fn ok(result: Value) -> Response {
    json(StatusCode::OK, json!({"errorCode": 0, "msg": "Success.", "result": result}).to_string())
}

fn page(rows: Vec<Value>, total: usize) -> Value {
    json!({"totalRows": total, "currentPage": 1, "currentSize": rows.len(), "data": rows})
}

async fn handle(uri: Uri) -> Response {
    let path = uri.path().trim_end_matches('/');
    if path == "/api/info" {
        return ok(json!({
            "controllerVer": "6.3.0.45", "apiVer": "3", "configured": true, "type": 0,
            "supportApp": true, "omadacId": OMADAC
        }));
    }
    if path.ends_with("/openapi/authorize/token") {
        return ok(json!({
            "accessToken": "AT-demonstration", "tokenType": "bearer", "expiresIn": 7200,
            "refreshToken": "RT-demonstration"
        }));
    }
    let base = format!("/openapi/v1/{OMADAC}/sites");
    let Some(rest) = path.strip_prefix(&base) else {
        return json(StatusCode::NOT_FOUND, "{}".into());
    };
    match rest {
        "" => ok(page(
            vec![json!({
                "siteId": "site-geneva", "name": "Geneva office", "region": "Switzerland",
                "timeZone": "Europe/Zurich", "scenario": "Office", "type": 0
            })],
            1,
        )),
        "/site-geneva/devices" => ok(page(devices(), 4)),
        "/site-geneva/clients" => {
            ok(page(vec![json!({"mac": "8C-85-90-11-22-33", "name": "laptop-claire",
                "wireless": true, "ssid": "Muller", "active": true})], 23))
        }
        p if p == format!("/site-geneva/switches/{SWITCH_MAC}") => ok(switch()),
        _ => json(StatusCode::FORBIDDEN, "{}".into()),
    }
}

fn devices() -> Vec<Value> {
    let uptime = 1_987_200 + now_s().rem_euclid(86_400);
    vec![
        json!({"mac": "3C-52-A1-20-00-01", "name": "gw-geneva", "type": "gateway",
            "model": "ER7206", "modelVersion": "2.0",
            "firmwareVersion": "2.2.5 Build 20250910 Rel.48720", "ip": "10.20.0.1",
            "status": 1, "statusCategory": 1, "cpuUtil": 9, "memUtil": 41,
            "uptimeLong": uptime, "needUpgrade": false, "clientNum": 0}),
        json!({"mac": SWITCH_MAC, "name": "sw-geneva", "type": "switch",
            "model": "SG2428P", "modelVersion": "5.20",
            "firmwareVersion": "5.20.6 Build 20250703 Rel.62218", "ip": "10.20.0.2",
            "status": 1, "statusCategory": 1, "cpuUtil": 14, "memUtil": 52,
            "uptimeLong": uptime, "needUpgrade": true, "clientNum": 0}),
        json!({"mac": "3C-52-A1-20-00-03", "name": "ap-studio", "type": "ap",
            "model": "EAP670", "modelVersion": "1.0",
            "firmwareVersion": "1.1.4 Build 20250822 Rel.39108", "ip": "10.20.0.11",
            "status": 1, "statusCategory": 1, "cpuUtil": 17, "memUtil": 58,
            "uptimeLong": uptime, "needUpgrade": false, "clientNum": 15}),
        json!({"mac": "3C-52-A1-20-00-04", "name": "ap-meeting", "type": "ap",
            "model": "EAP650", "modelVersion": "1.0",
            "firmwareVersion": "1.1.4 Build 20250822 Rel.39108", "ip": "10.20.0.12",
            "status": 1, "statusCategory": 1, "cpuUtil": 6, "memUtil": 44,
            "uptimeLong": uptime, "needUpgrade": false, "clientNum": 8}),
    ]
}

/// Ports du commutateur : (port, nom, lien, vitesse Omada, PoE en watts,
/// voisin LLDP, débit moyen en octets par seconde, liaison montante).
type Port = (u32, &'static str, bool, u32, Option<f64>, Option<&'static str>, f64, bool);

const PORTS: &[Port] = &[
    (1, "Port1", true, 3, None, Some("gw-geneva"), 2.4e6, true),
    (2, "Port2", true, 3, Some(9.8), Some("ap-studio"), 1.1e6, false),
    (3, "Port3", true, 3, Some(7.2), Some("ap-meeting"), 4.0e5, false),
    (4, "Port4", true, 3, None, Some("srv-dc01"), 3.1e6, false),
    (5, "Port5", true, 3, None, Some("vbr01"), 5.2e6, false),
    (6, "Port6", true, 2, Some(3.9), None, 2.0e4, false),
    (7, "Port7", false, 0, Some(0.0), None, 0.0, false),
    (8, "Port8", false, 0, None, None, 0.0, false),
];

fn switch() -> Value {
    // Compteurs cumulés depuis une date fixe : le débit lu par `rate()` reste stable.
    let elapsed = (now_s() - 1_767_225_600).max(0) as f64;
    let ports: Vec<Value> = PORTS
        .iter()
        .map(|(port, name, up, speed, poe, neighbor, rate, uplink)| {
            let mut p = json!({
                "port": port, "name": name, "portEnable": true,
                "status": u8::from(*up), "linkSpeed": speed, "duplex": if *up { 2 } else { 0 },
                "poe": poe.is_some(), "poePower": poe.unwrap_or(0.0),
                "rx": (rate * 0.6 * elapsed).round(), "tx": (rate * 0.4 * elapsed).round(),
                "rxErrors": 0, "txErrors": 0, "uplink": uplink
            });
            if let Some(neighbor) = neighbor {
                p["lldpSystemName"] = json!(neighbor);
                p["lldpPortId"] = json!("eth0");
            }
            p
        })
        .collect();
    json!({"mac": SWITCH_MAC, "name": "sw-geneva", "type": "switch", "ports": ports})
}
