//! Faux pare-feu OPNsense 26.1 : les réponses d'un vrai pare-feu
//! (`opnsense/testdata/opnsense261`), servies en `GET` comme en `POST`.

use axum::Router;
use axum::http::{StatusCode, Uri};
use axum::response::Response;
use serde_json::Value;

use super::{json, now_s, shift_times};

/// Instant approximatif de la capture : seuls les baux DHCP et l'état des VPN
/// portent des instants, recalés d'autant.
const CAPTURE_S: i64 = 1_790_263_400;

macro_rules! capture {
    ($file:literal) => {
        include_str!(concat!("../../../../collectors/src/opnsense/testdata/opnsense261/", $file))
    };
}

/// Chemins servis (les deux orthographes que le collecteur essaie) → réponse.
const ROUTES: &[(&[&str], &str)] = &[
    (
        &["system/system_information", "system/systemInformation"],
        capture!("system_information.json"),
    ),
    (&["system/system_time", "system/systemTime"], capture!("system_time.json")),
    (&["system/system_resources", "system/systemResources"], capture!("system_resources.json")),
    (&["system/system_disk", "system/systemDisk"], capture!("system_disk.json")),
    (&["system/system_swap", "system/systemSwap"], capture!("system_swap.json")),
    (&["system/system_mbuf", "system/systemMbuf"], capture!("system_mbuf.json")),
    (
        &["system/system_temperature", "system/systemTemperature"],
        capture!("system_temperature.json"),
    ),
    (&["cpu_usage/get_c_p_u_type", "cpu_usage/getCPUType"], capture!("cpu_type.json")),
    (
        &["interface/get_interface_statistics", "interface/getInterfaceStatistics"],
        capture!("interface_statistics.json"),
    ),
    (&["firewall/pf_states", "firewall/pfStates"], capture!("pf_states.json")),
    (&["interface/get_vip_status", "interface/getVipStatus"], capture!("vip_status.json")),
    (&["routes/gateway/status"], capture!("gateway_status.json")),
    (&["interfaces/overview/export"], capture!("interfaces_overview_export.json")),
    (&["core/service/search"], capture!("service_search.json")),
    (&["kea/leases4/search"], capture!("kea_leases4_search.json")),
    (&["wireguard/service/show"], capture!("wireguard_show.json")),
    (&["openvpn/service/search_sessions"], capture!("openvpn_search_sessions.json")),
    (&["ipsec/sessions/search_phase1"], capture!("ipsec_search_phase1.json")),
    (&["unbound/service/status"], capture!("unbound_status.json")),
    (&["unbound/diagnostics/stats"], capture!("unbound_stats.json")),
    (&["core/firmware/status"], capture!("firmware_status.json")),
];

/// Ce que le journal d'un pare-feu exposé voit passer : des balayages de ports
/// venus de partout, bloqués, et quelques connexions autorisées vers le
/// mandataire inverse. Des adresses publiques connues pour leurs balayages, de
/// quoi donner des pays à la carte en direct.
const SCANNERS: &[(&str, u16, &str)] = &[
    ("185.220.101.34", 22, "tcp"),
    ("45.155.205.13", 5060, "udp"),
    ("61.177.172.140", 22, "tcp"),
    ("218.92.0.112", 23, "tcp"),
    ("89.248.165.52", 3389, "tcp"),
    ("5.188.206.14", 445, "tcp"),
    ("177.54.144.21", 8080, "tcp"),
    ("103.145.13.6", 1433, "tcp"),
    ("162.142.125.9", 443, "tcp"),
    ("193.163.125.40", 6379, "tcp"),
    ("80.82.77.139", 9200, "tcp"),
    ("2001:470:1:332::8", 22, "tcp"),
];
const VISITORS: &[(&str, u16)] = &[
    ("2a02:1210:5c00::1d", 443),
    ("83.76.12.40", 443),
    ("92.104.33.7", 51820),
    ("86.247.110.21", 443),
];

/// Quelques lignes neuves à chaque lecture, la plus récente d'abord, au
/// format de `read_log.py`.
fn firewall_log(now: i64) -> String {
    let count = 2 + rand::random::<u32>() % 7;
    let lines: Vec<Value> = (0..count)
        .map(|i| {
            let at = chrono::DateTime::from_timestamp(now - i64::from(i), 0).unwrap_or_default();
            let digest = hex::encode(rand::random::<[u8; 16]>());
            let roll = rand::random::<u32>() % 10;
            let (src, port, proto, action, label) = if roll < 7 {
                let (ip, port, proto) = SCANNERS[rand::random::<u32>() as usize % SCANNERS.len()];
                (ip, port, proto, "block", "Default deny / state violation rule")
            } else {
                let (ip, port) = VISITORS[rand::random::<u32>() as usize % VISITORS.len()];
                let label =
                    if port == 443 { "Allow HTTPS to reverse proxy" } else { "Allow WireGuard" };
                (ip, port, if port == 443 { "tcp" } else { "udp" }, "pass", label)
            };
            serde_json::json!({
                "action": action, "dir": "in", "interface": "igb0", "reason": "match",
                "ipversion": if src.contains(':') { "6" } else { "4" },
                "protoname": proto, "src": src, "dst": "203.0.113.7",
                "srcport": (40000 + rand::random::<u16>() % 20000).to_string(),
                "dstport": port.to_string(), "label": label,
                "rid": hex::encode(rand::random::<[u8; 16]>()),
                "__timestamp__": at.to_rfc3339(), "__host__": "fw.home.arpa", "__digest__": digest,
            })
        })
        .collect();
    Value::Array(lines).to_string()
}

pub(super) fn router() -> Router {
    Router::new().fallback(handle)
}

async fn handle(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches("/api/").trim_start_matches("diagnostics/");
    let path = path.trim_end_matches('/');
    if path == "firewall/log" {
        return json(StatusCode::OK, firewall_log(now_s()));
    }
    let Some((_, body)) = ROUTES.iter().find(|(paths, _)| paths.contains(&path)) else {
        return json(StatusCode::NOT_FOUND, "{\"errorMessage\": \"Endpoint not found\"}".into());
    };
    let body = body.replace("OPNsense.internal", "fw.home.arpa");
    match serde_json::from_str::<Value>(&body) {
        Ok(mut doc) => {
            shift_times(&mut doc, now_s() - CAPTURE_S);
            json(StatusCode::OK, doc.to_string())
        }
        Err(_) => json(StatusCode::OK, body),
    }
}
