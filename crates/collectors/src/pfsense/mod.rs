//! pfSense, par le paquet communautaire `pfSense-pkg-RESTAPI` (API REST v2).
//!
//! Clé d'API en `X-API-Key`. Une clé porte exactement les privilèges de
//! l'utilisateur qui l'a créée : un compte `dumbmonit` qui n'a que les
//! privilèges « REST API - … GET » des pages lues ici ne peut rien modifier.
//!
//! Cinq lectures, en parallèle : état du système (processeur, mémoire, disque,
//! température), passerelles (perte, latence, état), interfaces, services, et
//! version. Une sixième, facultative, compare le paquet REST API à sa
//! dernière version publiée. pfSense lui-même ne dit pas par cette API si une
//! mise à jour du système existe : la seule route qui s'en approche la lance.
//!
//! Les pannes que ce module existe pour voir :
//!
//! * **une passerelle tombée** ou qui perd des paquets : la connexion à
//!   Internet, ou la ligne de secours, ne sert plus ;
//! * **une interface activée sans lien** : câble, commutateur ou carte ;
//! * **un service activé qui ne tourne plus** : résolveur DNS, OpenVPN, NTP…
//! * **un disque plein** : journaux et paquets ne s'écrivent plus.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `https` | Protocole de l'interface web. |
//! | `port` | `443` | Port de l'interface web. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable (celui d'origine est auto-signé). |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use serde_json::Value;

use crate::rest::{MAX_NAMED, RestClient, counter, flag, gauge, now_ms, number, text};

pub const DEFAULT_PORT: u16 = 443;

const SYSTEM: &str = "/api/v2/status/system";
const GATEWAYS: &str = "/api/v2/status/gateways";
const INTERFACES: &str = "/api/v2/status/interfaces";
const SERVICES: &str = "/api/v2/status/services";
const VERSION: &str = "/api/v2/system/version";
const RESTAPI_VERSION: &str = "/api/v2/system/restapi/version";

#[derive(Default)]
pub struct PfsenseCollector;

impl PfsenseCollector {
    pub fn new() -> Self {
        Self
    }
}

fn client(target: &Target) -> Result<RestClient, ProbeError> {
    let key = match &target.credential {
        Credential::ApiToken { token } if !token.trim().is_empty() => token.trim().to_string(),
        other => {
            return Err(ProbeError::Config(format!(
                "pfSense expects a REST API key (X-API-Key), configured: {other}"
            )));
        }
    };
    Ok(RestClient::for_target(target, "https", DEFAULT_PORT, "pfSense")?
        .with_header("X-API-Key", key))
}

/// Une route de l'API : le champ `data` de l'enveloppe.
async fn read(client: &RestClient, path: &str) -> Result<Value, ProbeError> {
    let (status, body) = client.get(path).await?;
    let reply: Option<Value> = serde_json::from_str(&body).ok();
    let response_id = reply
        .as_ref()
        .and_then(|r| r.get("response_id"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    match status.as_u16() {
        401 => {
            return Err(ProbeError::Auth(
                "pfSense refused the API key (401). Check the key, and that \"Key\" is ticked \
                 under System > REST API > Settings > Authentication methods."
                    .to_string(),
            ));
        }
        403 => {
            return Err(ProbeError::Auth(format!(
                "pfSense accepted the API key but refused {path} (403, {response_id}): give the \
                 dumbmonit user the \"REST API - {path} GET\" privilege."
            )));
        }
        // Le paquet absent, pfSense répond par sa page de connexion.
        404 => {
            return Err(ProbeError::Protocol(format!(
                "{} not found: is the REST API package (pfSense-pkg-RESTAPI v2) installed?",
                client.url(path)
            )));
        }
        _ => {}
    }
    if !status.is_success() {
        return Err(client.status_error(status, &body, path));
    }
    match reply.and_then(|mut r| r.get_mut("data").map(Value::take)) {
        Some(data) => Ok(data),
        None => Err(ProbeError::Protocol(format!(
            "{} did not answer like the pfSense REST API v2 (no \"data\"): is the package \
             installed, and is it version 2?",
            client.url(path)
        ))),
    }
}

#[async_trait]
impl Collector for PfsenseCollector {
    fn kind(&self) -> &'static str {
        "pfsense"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let client = client(target)?;
        let (system, gateways, interfaces, services, version, restapi) = tokio::join!(
            read(&client, SYSTEM),
            read(&client, GATEWAYS),
            read(&client, INTERFACES),
            read(&client, SERVICES),
            read(&client, VERSION),
            read(&client, RESTAPI_VERSION),
        );
        let replies = Replies {
            system: system?,
            gateways: gateways?,
            interfaces: interfaces?,
            services: services?,
            version: version.ok(),
            // Facultative : elle interroge GitHub depuis le pare-feu, et son
            // privilège peut ne pas avoir été donné.
            restapi: restapi.ok(),
        };
        Ok(samples(&replies, now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        read(&client(target)?, SYSTEM).await?;
        Ok(Some("pfsense".to_string()))
    }
}

pub struct Replies {
    pub system: Value,
    pub gateways: Value,
    pub interfaces: Value,
    pub services: Value,
    pub version: Option<Value>,
    pub restapi: Option<Value>,
}

fn list(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten()
}

pub fn samples(r: &Replies, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut out = Vec::new();

    let system = &r.system;
    for (field, metric) in [
        ("cpu_usage", "pfsense_cpu_usage_percent"),
        ("mem_usage", "pfsense_memory_used_percent"),
        ("swap_usage", "pfsense_swap_used_percent"),
        ("disk_usage", "pfsense_disk_used_percent"),
        ("mbuf_usage", "pfsense_mbuf_used_percent"),
        ("temp_c", "pfsense_temperature_celsius"),
        ("cpu_count", "pfsense_cpu_count"),
    ] {
        if let Some(value) = number(system.get(field)) {
            out.push(g(metric, value));
        }
    }
    for (value, period) in
        list(system.get("cpu_load_avg").unwrap_or(&Value::Null)).zip(["1m", "5m", "15m"])
    {
        if let Some(load) = number(Some(value)) {
            out.push(g("pfsense_load_average", load).with_label("period", period));
        }
    }
    if let Some(version) = r.version.as_ref().and_then(|v| text(v, "version")) {
        out.push(g("pfsense_version_info", 1.0).with_label("version", version));
    }

    let (mut gateways, mut gateways_down) = (0, 0);
    for gateway in list(&r.gateways).take(MAX_NAMED) {
        let Some(name) = text(gateway, "name") else { continue };
        let status = text(gateway, "status").unwrap_or("none");
        let up = status != "down";
        gateways += 1;
        gateways_down += usize::from(!up);
        let named = |sample: Sample| sample.with_label("gateway", name);
        out.push(named(flag("pfsense_gateway_up", up, ts_ms)));
        out.push(named(g("pfsense_gateway_status_info", 1.0).with_label("status", status)));
        if let Some(loss) = number(gateway.get("loss")) {
            out.push(named(g("pfsense_gateway_loss_percent", loss)));
        }
        // Une passerelle tombée n'a pas de latence : 0 ferait croire à un lien parfait.
        if up && let Some(delay) = number(gateway.get("delay")) {
            out.push(named(g("pfsense_gateway_delay_milliseconds", delay)));
        }
        if up && let Some(stddev) = number(gateway.get("stddev")) {
            out.push(named(g("pfsense_gateway_stddev_milliseconds", stddev)));
        }
    }
    out.push(g("pfsense_gateways", gateways as f64));
    out.push(g("pfsense_gateways_down", gateways_down as f64));

    let mut interfaces_down = 0;
    for interface in list(&r.interfaces).take(MAX_NAMED) {
        let Some(name) = text(interface, "name") else { continue };
        let descr = text(interface, "descr").unwrap_or(name);
        let enabled = interface.get("enable").and_then(Value::as_bool).unwrap_or(true);
        let status = text(interface, "status").unwrap_or("unknown");
        let link_up = status == "up" || status == "associated";
        // Une interface désactivée n'a pas à avoir de lien.
        let down = enabled && !link_up;
        interfaces_down += usize::from(down);
        let named =
            |sample: Sample| sample.with_label("interface", name).with_label("descr", descr);
        out.push(named(flag("pfsense_interface_enabled", enabled, ts_ms)));
        out.push(named(flag("pfsense_interface_up", link_up, ts_ms)));
        out.push(named(flag("pfsense_interface_down", down, ts_ms)));
        for (field, metric) in [
            ("inerrs", "pfsense_interface_errors_in_total"),
            ("outerrs", "pfsense_interface_errors_out_total"),
            ("inbytes", "pfsense_interface_bytes_in_total"),
            ("outbytes", "pfsense_interface_bytes_out_total"),
        ] {
            if let Some(value) = number(interface.get(field)) {
                out.push(named(counter(metric, value, ts_ms)));
            }
        }
    }
    out.push(g("pfsense_interfaces_down", interfaces_down as f64));

    let mut stopped = 0;
    for service in list(&r.services).take(MAX_NAMED) {
        let Some(name) = text(service, "name") else { continue };
        let description = text(service, "description").unwrap_or(name);
        let enabled = service.get("enabled").and_then(Value::as_bool).unwrap_or(false);
        let running = service.get("status").and_then(Value::as_bool).unwrap_or(false);
        let is_stopped = enabled && !running;
        stopped += usize::from(is_stopped);
        let named = |sample: Sample| {
            sample.with_label("service", name).with_label("description", description)
        };
        out.push(named(flag("pfsense_service_running", running, ts_ms)));
        out.push(named(flag("pfsense_service_enabled", enabled, ts_ms)));
        out.push(named(flag("pfsense_service_stopped", is_stopped, ts_ms)));
    }
    out.push(g("pfsense_services_stopped", stopped as f64));

    if let Some(restapi) = &r.restapi {
        if let Some(available) = restapi.get("update_available").and_then(Value::as_bool) {
            out.push(flag("pfsense_restapi_update_available", available, ts_ms));
        }
        if let Some(current) = text(restapi, "current_version") {
            let latest = text(restapi, "latest_version").unwrap_or(current);
            out.push(
                g("pfsense_restapi_version_info", 1.0)
                    .with_label("version", current.trim_start_matches('v'))
                    .with_label("latest", latest.trim_start_matches('v')),
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::http::{HeaderMap, StatusCode, Uri};
    use axum::routing::get;

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    // Réponses construites d'après les modèles du paquet `pfSense-pkg-RESTAPI`
    // v2 (`Models/SystemStatus.inc`, `RoutingGatewayStatus.inc`,
    // `InterfaceStats.inc`, `Service.inc`, `SystemVersion.inc`,
    // `RESTAPIVersion.inc`) et de son enveloppe (`Core/Response.inc`).
    const SYSTEM_JSON: &str = include_str!("testdata/status_system.json");
    const GATEWAYS_JSON: &str = include_str!("testdata/status_gateways.json");
    const INTERFACES_JSON: &str = include_str!("testdata/status_interfaces.json");
    const SERVICES_JSON: &str = include_str!("testdata/status_services.json");
    const VERSION_JSON: &str = include_str!("testdata/system_version.json");
    const RESTAPI_JSON: &str = include_str!("testdata/restapi_version.json");
    const UNAUTHORIZED: &str = include_str!("testdata/unauthorized.json");
    const FORBIDDEN: &str = include_str!("testdata/forbidden.json");

    fn data(raw: &str) -> Value {
        serde_json::from_str::<Value>(raw).unwrap()["data"].clone()
    }

    fn replies() -> Replies {
        Replies {
            system: data(SYSTEM_JSON),
            gateways: data(GATEWAYS_JSON),
            interfaces: data(INTERFACES_JSON),
            services: data(SERVICES_JSON),
            version: Some(data(VERSION_JSON)),
            restapi: Some(data(RESTAPI_JSON)),
        }
    }

    #[test]
    fn systeme_passerelles_interfaces_services() {
        let s = samples(&replies(), 0);
        assert_eq!(value(&s, "pfsense_cpu_usage_percent", &[]), 8.0);
        assert_eq!(value(&s, "pfsense_disk_used_percent", &[]), 17.9);
        assert_eq!(value(&s, "pfsense_temperature_celsius", &[]), 42.5);
        assert_eq!(value(&s, "pfsense_load_average", &[("period", "15m")]), 0.05);
        assert_eq!(
            find(&s, "pfsense_version_info", &[]).unwrap().labels["version"],
            "2.7.2-RELEASE"
        );

        assert_eq!(value(&s, "pfsense_gateway_up", &[("gateway", "WAN_DHCP")]), 1.0);
        assert_eq!(value(&s, "pfsense_gateway_up", &[("gateway", "WAN2_PPPOE")]), 0.0);
        assert!(
            find(&s, "pfsense_gateway_delay_milliseconds", &[("gateway", "WAN2_PPPOE")]).is_none()
        );
        assert_eq!(value(&s, "pfsense_gateway_loss_percent", &[("gateway", "LTE_BACKUP")]), 12.0);
        assert_eq!(
            value(&s, "pfsense_gateway_delay_milliseconds", &[("gateway", "LTE_BACKUP")]),
            312.8
        );
        assert_eq!(value(&s, "pfsense_gateways_down", &[]), 1.0);

        assert_eq!(value(&s, "pfsense_interface_down", &[("interface", "opt1")]), 1.0);
        assert_eq!(
            value(&s, "pfsense_interface_down", &[("interface", "opt2")]),
            0.0,
            "a disabled interface needs no link"
        );
        assert_eq!(value(&s, "pfsense_interfaces_down", &[]), 1.0);
        let errors = find(&s, "pfsense_interface_errors_in_total", &[("descr", "LAN")]).unwrap();
        assert_eq!((errors.value, errors.kind), (3.0, dumbmonit_proto::MetricKind::Counter));

        assert_eq!(value(&s, "pfsense_service_stopped", &[("service", "unbound")]), 1.0);
        assert_eq!(value(&s, "pfsense_service_stopped", &[("service", "openvpn")]), 0.0);
        assert_eq!(value(&s, "pfsense_services_stopped", &[]), 1.0);
        assert_eq!(value(&s, "pfsense_restapi_update_available", &[]), 1.0);
        assert_eq!(
            find(&s, "pfsense_restapi_version_info", &[]).unwrap().labels["latest"],
            "2.5.0"
        );
    }

    #[tokio::test]
    async fn interrogation_complete() {
        let app = Router::new().fallback(get(|headers: HeaderMap, uri: Uri| async move {
            match headers.get("x-api-key").and_then(|v| v.to_str().ok()) {
                Some("read-only-key") => {}
                Some("no-privilege") => return (StatusCode::FORBIDDEN, FORBIDDEN),
                _ => return (StatusCode::UNAUTHORIZED, UNAUTHORIZED),
            }
            let body = match uri.path() {
                SYSTEM => SYSTEM_JSON,
                GATEWAYS => GATEWAYS_JSON,
                INTERFACES => INTERFACES_JSON,
                SERVICES => SERVICES_JSON,
                VERSION => VERSION_JSON,
                // Privilège non donné : la partie facultative est sautée.
                _ => return (StatusCode::FORBIDDEN, FORBIDDEN),
            };
            (StatusCode::OK, body)
        }));
        let address = serve(app).await;
        let mut target = cible("pfsense", &format!("http://{address}"), &[]);
        target.credential = Credential::ApiToken { token: "read-only-key".into() };
        let s = PfsenseCollector::new().probe(&target).await.unwrap();
        assert_eq!(value(&s, "pfsense_gateways_down", &[]), 1.0);
        assert!(find(&s, "pfsense_restapi_update_available", &[]).is_none());

        target.credential = Credential::ApiToken { token: "wrong".into() };
        let error = PfsenseCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
        target.credential = Credential::ApiToken { token: "no-privilege".into() };
        let error = PfsenseCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(ref m) if m.contains("privilege")), "{error}");
    }

    #[tokio::test]
    async fn sans_le_paquet_ce_n_est_pas_l_api() {
        let app = Router::new()
            .fallback(get(|| async { (StatusCode::OK, "<html>pfSense login</html>") }));
        let address = serve(app).await;
        let mut target = cible("pfsense", &format!("http://{address}"), &[]);
        target.credential = Credential::ApiToken { token: "k".into() };
        let error = PfsenseCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Protocol(_)), "{error}");
    }
}
