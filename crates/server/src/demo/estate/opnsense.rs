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

pub(super) fn router() -> Router {
    Router::new().fallback(handle)
}

async fn handle(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches("/api/").trim_start_matches("diagnostics/");
    let path = path.trim_end_matches('/');
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
