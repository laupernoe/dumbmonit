//! FortiGate, par l'API REST de FortiOS (`/api/v2/monitor/…`).
//!
//! Un administrateur « REST API » dont le profil n'a que des droits de
//! lecture, limité à l'adresse de DumbMonit (hôte de confiance) ; son jeton
//! part en `Authorization: Bearer`, jamais dans l'URL (FortiOS 7.4.5 et 7.6.1
//! refusent d'ailleurs le jeton en paramètre).
//!
//! Trois lectures obligatoires — état du système (modèle, version), charge
//! (processeur, mémoire, disque, sessions), interfaces — et six facultatives,
//! qu'un profil plus restreint ou un boîtier sans grappe peut refuser sans
//! faire échouer le reste : état administratif des interfaces (cmdb),
//! statistiques et sommes de contrôle de la grappe HA, tunnels IPsec,
//! licences et abonnements FortiGuard, micrologiciels disponibles.
//!
//! Les pannes que ce module existe pour voir :
//!
//! * **un tunnel IPsec tombé** : un site, un centre de données coupé ;
//! * **un lien coupé** sur une interface active qui a une adresse ;
//! * **une grappe HA désynchronisée** ou amputée d'un membre ;
//! * **la mémoire qui approche du mode « conserve »** (88 % par défaut), où le
//!   boîtier cesse d'inspecter le trafic ;
//! * **un abonnement FortiGuard ou FortiCare qui expire**.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `443` | Port de l'interface d'administration (HTTPS seulement). |
//! | `vdom` | vide | Domaine virtuel lu ; vide = celui de l'administrateur. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable (celui d'origine est auto-signé). |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use serde_json::{Map, Value};

use crate::rest::{MAX_NAMED, RestClient, counter, flag, gauge, now_ms, number, text};
use crate::selfhosted::options::tag;

pub const DEFAULT_PORT: u16 = 443;

const STATUS: &str = "/api/v2/monitor/system/status";
const USAGE: &str = "/api/v2/monitor/system/resource/usage?interval=1-min";
const INTERFACES: &str =
    "/api/v2/monitor/system/interface/select?include_vlan=true&include_aggregate=true";
const INTERFACE_CONFIG: &str = "/api/v2/cmdb/system/interface?format=name|status|type";
const HA_STATISTICS: &str = "/api/v2/monitor/system/ha-statistics";
const HA_CHECKSUMS: &str = "/api/v2/monitor/system/ha-checksums";
const IPSEC: &str = "/api/v2/monitor/vpn/ipsec";
const LICENSE: &str = "/api/v2/monitor/license/status";
const FIRMWARE: &str = "/api/v2/monitor/system/firmware";

#[derive(Default)]
pub struct FortigateCollector;

impl FortigateCollector {
    pub fn new() -> Self {
        Self
    }
}

struct Api {
    client: RestClient,
    vdom: Option<String>,
}

impl Api {
    fn new(target: &Target) -> Result<Self, ProbeError> {
        let token = match &target.credential {
            Credential::ApiToken { token } if !token.trim().is_empty() => token.trim().to_string(),
            other => {
                return Err(ProbeError::Config(format!(
                    "FortiGate expects the token of a REST API administrator, configured: {other}"
                )));
            }
        };
        let vdom = tag(target, "vdom").map(str::to_string);
        if let Some(vdom) = &vdom
            && !vdom.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ProbeError::Config(format!("Invalid VDOM name \"{vdom}\"")));
        }
        let client = RestClient::for_target(target, "https", DEFAULT_PORT, "FortiGate")?
            .with_header("Authorization", format!("Bearer {token}"));
        Ok(Self { client, vdom })
    }

    fn path(&self, path: &str) -> String {
        match &self.vdom {
            Some(vdom) if path.contains('?') => format!("{path}&vdom={vdom}"),
            Some(vdom) => format!("{path}?vdom={vdom}"),
            None => path.to_string(),
        }
    }

    /// L'enveloppe entière de la réponse.
    async fn get(&self, path: &str) -> Result<Value, ProbeError> {
        let path = self.path(path);
        let (status, body) = self.client.get(&path).await?;
        match status.as_u16() {
            401 => Err(ProbeError::Auth(
                "FortiGate refused the API token (401): check the token, regenerate it with \
                 execute api-user generate-key dumbmonit if needed."
                    .to_string(),
            )),
            403 => Err(ProbeError::Auth(format!(
                "FortiGate refused {path} (403): the DumbMonit address is not among the trusted \
                 hosts of the REST API administrator, or its profile cannot read this."
            ))),
            200..=299 => {
                let reply: Value = self.client.decode(&body, &path)?;
                if reply.get("results").is_none() {
                    return Err(ProbeError::Protocol(format!(
                        "{} did not answer like FortiOS (no \"results\"): check the address and \
                         the port of the administration interface.",
                        self.client.url(&path)
                    )));
                }
                Ok(reply)
            }
            _ => Err(self.client.status_error(status, &body, &path)),
        }
    }

    /// Une lecture facultative : un refus ou une route absente ne fait rien
    /// échouer, une panne réelle si.
    async fn optional(&self, path: &str) -> Result<Option<Value>, ProbeError> {
        match self.get(path).await {
            Ok(reply) => Ok(Some(reply)),
            Err(error) if error.means_down() => Err(error),
            Err(error) => {
                tracing::debug!(path, %error, "FortiGate : lecture facultative indisponible");
                Ok(None)
            }
        }
    }
}

#[async_trait]
impl Collector for FortigateCollector {
    fn kind(&self) -> &'static str {
        "fortigate"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let api = Api::new(target)?;
        let (status, usage, interfaces) =
            tokio::join!(api.get(STATUS), api.get(USAGE), api.get(INTERFACES));
        let (config, ha_stats, ha_sums, ipsec, license, firmware) = tokio::join!(
            api.optional(INTERFACE_CONFIG),
            api.optional(HA_STATISTICS),
            api.optional(HA_CHECKSUMS),
            api.optional(IPSEC),
            api.optional(LICENSE),
            api.optional(FIRMWARE),
        );
        let replies = Replies {
            status: status?,
            usage: usage?,
            interfaces: interfaces?,
            interface_config: config?,
            ha_statistics: ha_stats?,
            ha_checksums: ha_sums?,
            ipsec: ipsec?,
            license: license?,
            firmware: firmware?,
        };
        Ok(samples(&replies, now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        Api::new(target)?.get(STATUS).await?;
        Ok(Some("fortigate".to_string()))
    }
}

pub struct Replies {
    pub status: Value,
    pub usage: Value,
    pub interfaces: Value,
    pub interface_config: Option<Value>,
    pub ha_statistics: Option<Value>,
    pub ha_checksums: Option<Value>,
    pub ipsec: Option<Value>,
    pub license: Option<Value>,
    pub firmware: Option<Value>,
}

fn results(reply: &Value) -> &Value {
    reply.get("results").unwrap_or(&Value::Null)
}

fn results_list(reply: Option<&Value>) -> Vec<&Value> {
    reply.map(results).and_then(Value::as_array).into_iter().flatten().collect()
}

/// `v7.4.4` → (7, 4, 4).
fn version_parts(raw: &str) -> Option<(u64, u64, u64)> {
    let mut parts = raw.trim().trim_start_matches(['v', 'V']).split('.');
    let mut next = || parts.next()?.split(|c: char| !c.is_ascii_digit()).next()?.parse().ok();
    Some((next()?, next()?, next()?))
}

fn firmware_version(item: &Value) -> Option<(u64, u64, u64)> {
    match (number(item.get("major")), number(item.get("minor")), number(item.get("patch"))) {
        (Some(major), Some(minor), Some(patch)) => Some((major as u64, minor as u64, patch as u64)),
        _ => text(item, "version").and_then(version_parts),
    }
}

pub fn samples(r: &Replies, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut out = Vec::new();

    let status = results(&r.status);
    if let Some(version) = r.status.get("version").and_then(Value::as_str) {
        let build = number(r.status.get("build")).map(|b| format!("{b}")).unwrap_or_default();
        out.push(
            g("fortigate_version_info", 1.0)
                .with_label("version", version.trim_start_matches('v'))
                .with_label("build", build)
                .with_label("model", text(status, "model").unwrap_or(""))
                .with_label("hostname", text(status, "hostname").unwrap_or("")),
        );
    }
    if let Some(disk) = text(status, "log_disk_status") {
        out.push(flag("fortigate_log_disk_ok", disk != "need_format", ts_ms));
    }

    let usage = results(&r.usage);
    for (field, metric) in [
        ("cpu", "fortigate_cpu_usage_percent"),
        ("mem", "fortigate_memory_used_percent"),
        ("disk", "fortigate_disk_used_percent"),
        ("session", "fortigate_sessions"),
        ("setuprate", "fortigate_session_setup_rate"),
    ] {
        let current = usage
            .get(field)
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(|item| number(item.get("current")));
        if let Some(value) = current {
            out.push(g(metric, value));
        }
    }

    interfaces(r, ts_ms, &mut out);
    ha(r, ts_ms, &mut out);
    ipsec(r, ts_ms, &mut out);
    licenses(r, ts_ms, &mut out);

    if let Some(firmware) = r.firmware.as_ref().map(results) {
        let current = firmware.get("current").and_then(firmware_version);
        let available: Vec<((u64, u64, u64), &str)> = firmware
            .get("available")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|item| Some((firmware_version(item)?, text(item, "version")?)))
            .collect();
        if let Some(current) = current {
            // Une version corrective de la même branche : celle qu'on applique
            // sans réfléchir. Une nouvelle branche est un projet, pas une alerte.
            let patch = available
                .iter()
                .filter(|(v, _)| v.0 == current.0 && v.1 == current.1 && v.2 > current.2)
                .max_by_key(|(v, _)| *v);
            out.push(flag("fortigate_firmware_update_available", patch.is_some(), ts_ms));
            if let Some((_, version)) = patch {
                out.push(
                    g("fortigate_firmware_latest_info", 1.0)
                        .with_label("version", version.trim_start_matches('v')),
                );
            }
        }
    }
    out
}

fn interfaces(r: &Replies, ts_ms: i64, out: &mut Vec<Sample>) {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    // État administratif : sans le droit de lire la configuration, toutes
    // les interfaces sont tenues pour actives.
    let admin: Map<String, Value> = results_list(r.interface_config.as_ref())
        .into_iter()
        .filter_map(|item| Some((text(item, "name")?.to_string(), item.clone())))
        .collect();
    let mut down = 0;
    let mut items: Vec<(&String, &Value)> =
        results(&r.interfaces).as_object().into_iter().flatten().collect();
    items.sort_by(|a, b| a.0.cmp(b.0));
    for (key, interface) in items.into_iter().take(MAX_NAMED) {
        let name = text(interface, "name").unwrap_or(key);
        let alias = text(interface, "alias").unwrap_or("");
        let link = interface.get("link").and_then(Value::as_bool).unwrap_or(false);
        let enabled =
            admin.get(name).and_then(|item| text(item, "status")).is_none_or(|s| s == "up");
        let addressed = text(interface, "ip").is_some_and(|ip| ip != "0.0.0.0");
        // Une interface active, avec une adresse, sans lien : un câble ou un
        // équipement en face. Les ports libres (sans adresse) ne comptent pas.
        let is_down = enabled && addressed && !link;
        down += usize::from(is_down);
        let named =
            |sample: Sample| sample.with_label("interface", name).with_label("alias", alias);
        out.push(named(flag("fortigate_interface_link_up", link, ts_ms)));
        out.push(named(flag("fortigate_interface_enabled", enabled, ts_ms)));
        out.push(named(flag("fortigate_interface_down", is_down, ts_ms)));
        if link && let Some(speed) = number(interface.get("speed")) {
            out.push(named(g("fortigate_interface_speed_mbps", speed)));
        }
        for (field, metric) in [
            ("rx_bytes", "fortigate_interface_bytes_in_total"),
            ("tx_bytes", "fortigate_interface_bytes_out_total"),
            ("rx_errors", "fortigate_interface_errors_in_total"),
            ("tx_errors", "fortigate_interface_errors_out_total"),
        ] {
            if let Some(value) = number(interface.get(field)) {
                out.push(named(counter(metric, value, ts_ms)));
            }
        }
    }
    out.push(g("fortigate_interfaces_down", down as f64));
}

fn ha(r: &Replies, ts_ms: i64, out: &mut Vec<Sample>) {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    if r.ha_statistics.is_some() {
        let members = results_list(r.ha_statistics.as_ref());
        // Un boîtier seul rend une liste vide : aucun membre, aucune grappe.
        out.push(g("fortigate_ha_members", members.len() as f64));
        for member in members.into_iter().take(MAX_NAMED) {
            let Some(host) = text(member, "hostname") else { continue };
            if let Some(cpu) = number(member.get("cpu_usage")) {
                out.push(
                    g("fortigate_ha_member_cpu_usage_percent", cpu).with_label("member", host),
                );
            }
            if let Some(mem) = number(member.get("mem_usage")) {
                out.push(
                    g("fortigate_ha_member_memory_used_percent", mem).with_label("member", host),
                );
            }
        }
    }
    let sums: Vec<&str> = results_list(r.ha_checksums.as_ref())
        .into_iter()
        .filter_map(|member| member.pointer("/checksum/all").and_then(Value::as_str))
        .collect();
    if sums.len() >= 2 {
        let in_sync = sums.iter().all(|sum| *sum == sums[0]);
        out.push(flag("fortigate_ha_in_sync", in_sync, ts_ms));
    }
}

fn ipsec(r: &Replies, ts_ms: i64, out: &mut Vec<Sample>) {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    if r.ipsec.is_none() {
        return;
    }
    let (mut tunnels, mut down) = (0, 0);
    for tunnel in results_list(r.ipsec.as_ref()) {
        // Les accès nomades (dialup) vont et viennent : ce ne sont pas des liaisons.
        if text(tunnel, "type") == Some("dialup") {
            continue;
        }
        let Some(name) = text(tunnel, "name") else { continue };
        tunnels += 1;
        let selectors: Vec<&Value> =
            tunnel.get("proxyid").and_then(Value::as_array).into_iter().flatten().collect();
        let up = selectors.iter().any(|p| text(p, "status") == Some("up"));
        down += usize::from(!up);
        if tunnels > MAX_NAMED {
            continue;
        }
        let named = |sample: Sample| sample.with_label("tunnel", name);
        out.push(named(flag("fortigate_ipsec_tunnel_up", up, ts_ms)));
        if let Some(gateway) = text(tunnel, "rgwy") {
            out.push(
                named(g("fortigate_ipsec_tunnel_info", 1.0)).with_label("remote_gateway", gateway),
            );
        }
        for (field, metric) in [
            ("incoming_bytes", "fortigate_ipsec_tunnel_bytes_in_total"),
            ("outgoing_bytes", "fortigate_ipsec_tunnel_bytes_out_total"),
        ] {
            if let Some(value) = number(tunnel.get(field)) {
                out.push(named(counter(metric, value, ts_ms)));
            }
        }
        for selector in selectors.iter().take(MAX_NAMED) {
            let Some(phase2) = text(selector, "p2name") else { continue };
            out.push(
                named(flag(
                    "fortigate_ipsec_phase2_up",
                    text(selector, "status") == Some("up"),
                    ts_ms,
                ))
                .with_label("phase2", phase2),
            );
        }
    }
    out.push(g("fortigate_ipsec_tunnels", tunnels as f64));
    out.push(g("fortigate_ipsec_tunnels_down", down as f64));
}

fn licenses(r: &Replies, ts_ms: i64, out: &mut Vec<Sample>) {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let Some(items) = r.license.as_ref().map(results).and_then(Value::as_object) else { return };
    let now = ts_ms as f64 / 1000.0;
    let mut entries: Vec<(String, &Value)> = Vec::new();
    for (key, item) in items {
        entries.push((key.clone(), item));
        // FortiCare range ses contrats de support un niveau plus bas.
        if let Some(support) = item.get("support").and_then(Value::as_object) {
            for (kind, contract) in support {
                entries.push((format!("{key}_{kind}"), contract));
            }
        }
    }
    for (license, item) in entries.into_iter().take(MAX_NAMED) {
        let Some(status) = text(item, "status") else { continue };
        if status == "no_license" {
            continue;
        }
        out.push(
            g("fortigate_license_status_info", 1.0)
                .with_label("license", license.as_str())
                .with_label("status", status),
        );
        out.push(
            flag("fortigate_license_expired", status == "expired", ts_ms)
                .with_label("license", license.as_str()),
        );
        if let Some(expires) = number(item.get("expires")).filter(|e| *e > 0.0) {
            out.push(
                g("fortigate_license_expiry_seconds", expires - now)
                    .with_label("license", license.as_str()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::http::{HeaderMap, StatusCode, Uri};
    use axum::routing::get;

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    // Réponses construites d'après les jeux d'essai du `fortigate_exporter`
    // (prometheus-community) et la documentation de FortiOS 7.4 : une grappe
    // de deux membres désynchronisée, un second accès Internet sans lien, un
    // port libre sans adresse, un port désactivé, un tunnel tombé, un accès
    // nomade, un filtrage web expiré, une version corrective disponible.
    const STATUS_JSON: &str = include_str!("testdata/system_status.json");
    const USAGE_JSON: &str = include_str!("testdata/resource_usage.json");
    const INTERFACES_JSON: &str = include_str!("testdata/interface_select.json");
    const CMDB_JSON: &str = include_str!("testdata/cmdb_interface.json");
    const HA_STATS_JSON: &str = include_str!("testdata/ha_statistics.json");
    const HA_STANDALONE_JSON: &str = include_str!("testdata/ha_statistics_standalone.json");
    const HA_SUMS_JSON: &str = include_str!("testdata/ha_checksums.json");
    const IPSEC_JSON: &str = include_str!("testdata/vpn_ipsec.json");
    const LICENSE_JSON: &str = include_str!("testdata/license_status.json");
    const FIRMWARE_JSON: &str = include_str!("testdata/system_firmware.json");

    fn json(raw: &str) -> Value {
        serde_json::from_str(raw).unwrap()
    }

    const NOW: i64 = 1_790_000_000_000;

    fn replies() -> Replies {
        Replies {
            status: json(STATUS_JSON),
            usage: json(USAGE_JSON),
            interfaces: json(INTERFACES_JSON),
            interface_config: Some(json(CMDB_JSON)),
            ha_statistics: Some(json(HA_STATS_JSON)),
            ha_checksums: Some(json(HA_SUMS_JSON)),
            ipsec: Some(json(IPSEC_JSON)),
            license: Some(json(LICENSE_JSON)),
            firmware: Some(json(FIRMWARE_JSON)),
        }
    }

    #[test]
    fn systeme_interfaces_grappe_tunnels_licences() {
        let s = samples(&replies(), NOW);
        let info = find(&s, "fortigate_version_info", &[]).unwrap();
        assert_eq!(
            (info.labels["version"].as_str(), info.labels["model"].as_str()),
            ("7.4.4", "FGT60F")
        );
        assert_eq!(value(&s, "fortigate_cpu_usage_percent", &[]), 12.0);
        assert_eq!(value(&s, "fortigate_memory_used_percent", &[]), 58.0);
        assert_eq!(value(&s, "fortigate_sessions", &[]), 1532.0);

        assert_eq!(value(&s, "fortigate_interface_down", &[("interface", "wan2")]), 1.0);
        assert_eq!(
            value(&s, "fortigate_interface_down", &[("interface", "internal1")]),
            0.0,
            "no address"
        );
        assert_eq!(value(&s, "fortigate_interface_down", &[("interface", "dmz")]), 0.0, "disabled");
        assert_eq!(value(&s, "fortigate_interfaces_down", &[]), 1.0);
        assert_eq!(value(&s, "fortigate_interface_speed_mbps", &[("interface", "wan1")]), 1000.0);

        assert_eq!(value(&s, "fortigate_ha_members", &[]), 2.0);
        assert_eq!(value(&s, "fortigate_ha_in_sync", &[]), 0.0);

        assert_eq!(value(&s, "fortigate_ipsec_tunnels", &[]), 2.0, "dialup left out");
        assert_eq!(value(&s, "fortigate_ipsec_tunnel_up", &[("tunnel", "to-hq")]), 1.0);
        assert_eq!(value(&s, "fortigate_ipsec_tunnel_up", &[("tunnel", "to-dc2")]), 0.0);
        assert_eq!(value(&s, "fortigate_ipsec_tunnels_down", &[]), 1.0);

        assert_eq!(value(&s, "fortigate_license_expired", &[("license", "web_filtering")]), 1.0);
        assert_eq!(
            value(&s, "fortigate_license_expiry_seconds", &[("license", "antivirus")]),
            600_000.0
        );
        assert!(find(&s, "fortigate_license_status_info", &[("license", "sms")]).is_none());
        assert!(
            find(&s, "fortigate_license_expiry_seconds", &[("license", "forticare_hardware")])
                .is_some()
        );

        assert_eq!(value(&s, "fortigate_firmware_update_available", &[]), 1.0);
        assert_eq!(
            find(&s, "fortigate_firmware_latest_info", &[]).unwrap().labels["version"],
            "7.4.5",
            "the patch release, not the new 7.6 branch"
        );
    }

    #[test]
    fn un_boitier_seul_sans_droits_facultatifs() {
        let mut r = replies();
        r.ha_statistics = Some(json(HA_STANDALONE_JSON));
        r.ha_checksums = None;
        r.interface_config = None;
        r.ipsec = None;
        r.license = None;
        r.firmware = None;
        let s = samples(&r, NOW);
        assert_eq!(value(&s, "fortigate_ha_members", &[]), 0.0);
        assert!(find(&s, "fortigate_ha_in_sync", &[]).is_none());
        // Sans la configuration, l'interface désactivée compte comme active.
        assert_eq!(value(&s, "fortigate_interface_down", &[("interface", "dmz")]), 1.0);
        assert!(find(&s, "fortigate_ipsec_tunnels", &[]).is_none());
        assert!(find(&s, "fortigate_firmware_update_available", &[]).is_none());
    }

    #[test]
    fn versions() {
        assert_eq!(version_parts("v7.4.4"), Some((7, 4, 4)));
        assert_eq!(version_parts("7.6.0-build3401"), Some((7, 6, 0)));
        assert_eq!(version_parts("v7"), None);
    }

    #[tokio::test]
    async fn interrogation_complete() {
        let app = Router::new().fallback(get(|headers: HeaderMap, uri: Uri| async move {
            if headers.get("authorization").and_then(|v| v.to_str().ok()) != Some("Bearer ro-token")
            {
                return (StatusCode::UNAUTHORIZED, "");
            }
            assert!(uri.query().unwrap_or_default().contains("vdom=root"));
            let body = match uri.path() {
                "/api/v2/monitor/system/status" => STATUS_JSON,
                "/api/v2/monitor/system/resource/usage" => USAGE_JSON,
                "/api/v2/monitor/system/interface/select" => INTERFACES_JSON,
                "/api/v2/monitor/system/ha-statistics" => HA_STANDALONE_JSON,
                "/api/v2/monitor/vpn/ipsec" => IPSEC_JSON,
                "/api/v2/monitor/license/status" => LICENSE_JSON,
                // Profil sans droit sur la configuration ni le micrologiciel.
                _ => return (StatusCode::FORBIDDEN, ""),
            };
            (StatusCode::OK, body)
        }));
        let address = serve(app).await;
        let mut target = cible("fortigate", &format!("http://{address}"), &[("vdom", "root")]);
        target.credential = Credential::ApiToken { token: "ro-token".into() };
        let s = FortigateCollector::new().probe(&target).await.unwrap();
        assert_eq!(value(&s, "fortigate_ipsec_tunnels_down", &[]), 1.0);
        assert_eq!(value(&s, "fortigate_ha_members", &[]), 0.0);
        assert!(find(&s, "fortigate_firmware_update_available", &[]).is_none());

        target.credential = Credential::ApiToken { token: "revoked".into() };
        let error = FortigateCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
    }
}
