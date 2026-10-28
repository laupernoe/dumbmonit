//! TP-Link Omada, par l'API ouverte du contrôleur (`/openapi/v1/{omadacId}/…`).
//!
//! Ports des commutateurs (via `crate::switch_ports`), au schéma d'UniFi :
//! `omada_port_enabled`, `omada_port_up`, `omada_port_speed_bps`, `omada_port_full_duplex`, `omada_port_uplink`, `omada_port_poe_enabled`, `omada_port_poe_watts`, `omada_port_rx_bytes_total`, `omada_port_tx_bytes_total`, `omada_port_rx_errors_total`, `omada_port_tx_errors_total`, `omada_port_neighbor_info`.
//!
//! Un client OpenAPI créé dans le contrôleur (Paramètres de la plateforme →
//! Intégrations → Open API, mode « Client ») avec le rôle Observateur : son
//! identifiant et son secret s'échangent (`grant_type=client_credentials`)
//! contre un jeton d'accès de deux heures, gardé jusqu'à un peu avant son
//! expiration puis redemandé de la même façon (le jeton de renouvellement n'est
//! pas utilisé : refaire la demande donne le même résultat sans rien stocker de
//! plus). Le jeton part en `Authorization: AccessToken=…`, jamais dans l'URL.
//!
//! L'identifiant du contrôleur (`omadacId`) est celui que l'interface affiche ;
//! laissé vide, il est lu sur `/api/info`, que le contrôleur publie sans
//! authentification.
//!
//! Chaque réponse porte un `errorCode` (0 = succès) même quand le statut HTTP
//! est 200 : un jeton refusé se reconnaît à lui, et déclenche une seule
//! reconnexion par interrogation.
//!
//! Lectures : la liste des sites, puis pour chacun les appareils (points
//! d'accès, commutateurs, passerelles : état, processeur, mémoire, version,
//! mise à jour proposée) — obligatoires — et le nombre de clients — facultatif.
//!
//! Les pannes que ce module existe pour voir :
//!
//! * **un appareil déconnecté** du contrôleur (ou dont le battement de cœur
//!   manque) ; un appareil « en attente » d'adoption est compté à part ;
//! * **un appareil saturé** en processeur ou en mémoire ;
//! * **une mise à jour du micrologiciel** proposée par le contrôleur.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `8043` | Port HTTPS du contrôleur (443 pour un contrôleur matériel OC200/OC300). |
//! | `omadac_id` | vide | Identifiant du contrôleur ; vide = lu sur `/api/info`. |
//! | `site` | vide | Nom ou identifiant d'un site ; vide = tous. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable (celui d'origine est auto-signé). |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use reqwest::StatusCode;
use serde_json::{Value, json};

use crate::rest::{MAX_NAMED, RestClient, TokenCache, flag, gauge, now_ms, number, text};
use crate::selfhosted::options::tag;
use crate::switch_ports::{
    self, DevicePorts, MAX_PORTED_DEVICES, Port, pick, pick_bool, pick_number, pick_text,
};

pub const DEFAULT_PORT: u16 = 8043;
const TOKEN_PATH: &str = "/openapi/authorize/token?grant_type=client_credentials";
const INFO_PATH: &str = "/api/info";
/// Codes d'erreur d'un jeton absent, expiré ou invalide.
const TOKEN_CODES: [i64; 3] = [-44111, -44112, -44113];
/// Pages lues au plus pour une liste.
const MAX_PAGES: usize = 5;
/// Sites lus au plus à chaque passage.
const MAX_SITES: usize = 20;

#[derive(Default)]
pub struct OmadaCollector {
    tokens: TokenCache,
}

impl OmadaCollector {
    pub fn new() -> Self {
        Self::default()
    }

    async fn session(&self, target: &Target) -> Result<Session<'_>, ProbeError> {
        let (client_id, secret) = match &target.credential {
            Credential::UsernamePassword { username, password }
                if !username.trim().is_empty() && !password.trim().is_empty() =>
            {
                (username.trim().to_string(), password.trim().to_string())
            }
            other => {
                return Err(ProbeError::Config(format!(
                    "Omada expects an OpenAPI client (client ID and secret), configured: {other}"
                )));
            }
        };
        let client = RestClient::for_target(target, "https", DEFAULT_PORT, "Omada")?;
        let omadac = match tag(target, "omadac_id") {
            Some(id) => id.to_string(),
            None => controller_id(&client).await?,
        };
        if !omadac.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(ProbeError::Config(format!("Invalid controller ID \"{omadac}\"")));
        }
        let key = format!("{}:{client_id}:{omadac}", target.id);
        let mut session = Session { collector: self, client, omadac, client_id, secret, key };
        session.login(false).await?;
        Ok(session)
    }
}

/// L'identifiant du contrôleur, lu sur `/api/info` (sans authentification).
async fn controller_id(client: &RestClient) -> Result<String, ProbeError> {
    let info = info(client).await?;
    text(&info, "omadacId").map(str::to_string).ok_or_else(|| {
        ProbeError::Config(
            "Omada did not publish its controller ID: enter it in the Controller ID option \
             (Settings → Platform Integration → Open API shows it)."
                .to_string(),
        )
    })
}

/// Le `result` de `/api/info` : version du contrôleur, de l'API, identifiant.
async fn info(client: &RestClient) -> Result<Value, ProbeError> {
    let reply: Value = client.get_json(INFO_PATH).await?;
    reply.get("result").cloned().filter(Value::is_object).ok_or_else(|| {
        ProbeError::Protocol(format!(
            "{} did not answer like an Omada controller on {INFO_PATH}: check the address and \
             the port (8043 for the software controller, 443 for a hardware one).",
            client.base_url()
        ))
    })
}

struct Session<'a> {
    collector: &'a OmadaCollector,
    client: RestClient,
    omadac: String,
    client_id: String,
    secret: String,
    key: String,
}

/// Le code d'erreur d'une réponse de l'API ouverte.
fn error_code(reply: &Value) -> Option<i64> {
    number(reply.get("errorCode")).map(|code| code as i64)
}

impl Session<'_> {
    /// Connexion, ou jeton gardé.
    async fn login(&mut self, fresh: bool) -> Result<(), ProbeError> {
        if !fresh && let Some(token) = self.collector.tokens.get(&self.key) {
            self.client.set_header("Authorization", format!("AccessToken={token}"));
            return Ok(());
        }
        let body = json!({
            "omadacId": self.omadac,
            "client_id": self.client_id,
            "client_secret": self.secret,
        });
        let (status, raw) = self.client.post_json(TOKEN_PATH, &body).await?;
        if matches!(status.as_u16(), 400 | 401 | 403) {
            return Err(ProbeError::Auth(format!(
                "Omada refused the OpenAPI client ({status}). Check the client ID and secret, \
                 and that the client uses the Client mode."
            )));
        }
        if !status.is_success() {
            return Err(self.client.status_error(status, &raw, TOKEN_PATH));
        }
        let reply: Value = self.client.decode(&raw, TOKEN_PATH)?;
        match error_code(&reply) {
            Some(0) => {}
            Some(code) => {
                return Err(ProbeError::Auth(format!(
                    "Omada refused the OpenAPI client (error {code}): {}. Check the client ID, \
                     the secret and the controller ID.",
                    text(&reply, "msg").unwrap_or("no message")
                )));
            }
            None => {
                return Err(ProbeError::Protocol(format!(
                    "{} did not answer like the Omada OpenAPI on {TOKEN_PATH}.",
                    self.client.base_url()
                )));
            }
        }
        let result = reply.get("result").unwrap_or(&Value::Null);
        let token = text(result, "accessToken").ok_or_else(|| {
            ProbeError::Protocol("Omada returned no accessToken for the OpenAPI client".to_string())
        })?;
        let lifetime = number(result.get("expiresIn")).map(|s| s as u64).unwrap_or(7200);
        self.collector.tokens.put(&self.key, token, Duration::from_secs(lifetime));
        self.client.set_header("Authorization", format!("AccessToken={token}"));
        Ok(())
    }

    /// Le `result` d'une lecture ; un jeton refusé provoque une seule reconnexion.
    async fn get(&mut self, path: &str) -> Result<Value, ProbeError> {
        let mut retried = false;
        loop {
            let (status, raw) = self.client.get(path).await?;
            if status.is_success() {
                let reply: Value = self.client.decode(&raw, path)?;
                let Some(code) = error_code(&reply) else {
                    return Err(ProbeError::Protocol(format!(
                        "{} did not answer like the Omada OpenAPI on {path}.",
                        self.client.base_url()
                    )));
                };
                if code == 0 {
                    return Ok(reply.get("result").cloned().unwrap_or(Value::Null));
                }
                if retried || !TOKEN_CODES.contains(&code) {
                    return Err(api_error(code, &reply, path));
                }
            } else if status != StatusCode::UNAUTHORIZED || retried {
                return Err(self.client.status_error(status, &raw, path));
            }
            // Jeton gardé révoqué ou expiré : une seule reconnexion.
            retried = true;
            self.collector.tokens.forget(&self.key);
            self.login(true).await?;
        }
    }

    /// Les lignes d'une liste paginée (`result.data`), au plus `MAX_PAGES` pages.
    async fn rows(&mut self, path: &str, size: usize) -> Result<Vec<Value>, ProbeError> {
        let mut out: Vec<Value> = Vec::new();
        for page in 1..=MAX_PAGES {
            let separator = if path.contains('?') { '&' } else { '?' };
            let result = self.get(&format!("{path}{separator}page={page}&pageSize={size}")).await?;
            let data = result.get("data").and_then(Value::as_array).cloned().unwrap_or_default();
            let total = number(result.get("totalRows")).unwrap_or(0.0) as usize;
            let got = data.len();
            out.extend(data);
            if got == 0 || out.len() >= total {
                break;
            }
        }
        Ok(out)
    }

    /// Les ports des commutateurs en ligne, un par un : lecture facultative
    /// (un rôle ou une version de contrôleur sans cette route ne fait rien échouer).
    async fn switch_ports(
        &mut self,
        sites_path: &str,
        site_id: &str,
        devices: &[Value],
    ) -> Result<Vec<DevicePorts>, ProbeError> {
        let mut out = Vec::new();
        let switches = devices
            .iter()
            .filter(|d| is_switch(d) && state_of(d) == Some(State::Online))
            .take(MAX_PORTED_DEVICES);
        for device in switches {
            let Some(mac) = text(device, "mac") else { continue };
            if !mac.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == ':') {
                continue;
            }
            // À vérifier contre la référence de l'API ouverte du contrôleur :
            // le détail d'un commutateur porte la liste `ports`.
            match self.get(&format!("{sites_path}/{site_id}/switches/{mac}")).await {
                Ok(detail) => out.push(DevicePorts {
                    device: text(device, "name").unwrap_or(mac).to_string(),
                    unit: mac.to_string(),
                    ports: parse_ports(&detail),
                }),
                Err(error) if error.means_down() => return Err(error),
                Err(error) => {
                    tracing::debug!(%error, mac, "Omada : ports du commutateur indisponibles");
                    break;
                }
            }
        }
        Ok(out)
    }

    async fn collect(&mut self, wanted_site: Option<&str>) -> Result<Vec<Site>, ProbeError> {
        let sites_path = format!("/openapi/v1/{}/sites", self.omadac);
        let sites = self.rows(&sites_path, 100).await?;
        let chosen: Vec<&Value> = sites
            .iter()
            .filter(|site| match wanted_site {
                None => true,
                Some(wanted) => {
                    text(site, "siteId") == Some(wanted)
                        || text(site, "name").is_some_and(|n| n.eq_ignore_ascii_case(wanted))
                }
            })
            .take(MAX_SITES)
            .collect();
        if chosen.is_empty() {
            return Err(match wanted_site {
                Some(wanted) => ProbeError::Config(format!(
                    "The Omada controller has no site \"{wanted}\" (name or ID): leave the option \
                     empty to read every site."
                )),
                None => ProbeError::Config(
                    "The OpenAPI client sees no Omada site: give its role access to the sites."
                        .to_string(),
                ),
            });
        }
        let mut out = Vec::new();
        for site in chosen {
            let Some(id) = text(site, "siteId") else { continue };
            if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                continue;
            }
            let name = text(site, "name").unwrap_or(id).to_string();
            let devices = self.rows(&format!("{sites_path}/{id}/devices"), 1000).await?;
            // Facultatif : un rôle plus restreint peut refuser la liste des clients.
            let clients_path = format!("{sites_path}/{id}/clients?page=1&pageSize=1");
            let clients = match self.get(&clients_path).await {
                Ok(result) => number(result.get("totalRows")),
                Err(error) if error.means_down() => return Err(error),
                Err(error) => {
                    tracing::debug!(%error, "Omada : nombre de clients indisponible");
                    None
                }
            };
            let ports = self.switch_ports(&sites_path, id, &devices).await?;
            out.push(Site { name, devices, clients, ports });
        }
        Ok(out)
    }
}

fn api_error(code: i64, reply: &Value, path: &str) -> ProbeError {
    let message = text(reply, "msg").unwrap_or("no message");
    let lowered = message.to_ascii_lowercase();
    if TOKEN_CODES.contains(&code) {
        ProbeError::Auth(format!("Omada refused the access token on {path} (error {code})."))
    } else if lowered.contains("permission") || lowered.contains("privilege") {
        ProbeError::Auth(format!(
            "Omada refused {path} (error {code}): {message}. Give the OpenAPI client the Viewer \
             role on the sites."
        ))
    } else {
        ProbeError::Protocol(format!("Omada refused {path} (error {code}): {message}"))
    }
}

#[async_trait]
impl Collector for OmadaCollector {
    fn kind(&self) -> &'static str {
        "omada"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let mut session = self.session(target).await?;
        // Version du contrôleur : facultative, `/api/info` est un détail.
        let info = info(&session.client).await.ok();
        let sites = session.collect(tag(target, "site")).await?;
        Ok(samples(&Replies { info, sites }, now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let mut session = self.session(target).await?;
        session.get(&format!("/openapi/v1/{}/sites?page=1&pageSize=1", session.omadac)).await?;
        Ok(Some("omada".to_string()))
    }
}

pub struct Site {
    pub name: String,
    pub devices: Vec<Value>,
    pub clients: Option<f64>,
    /// Ports des commutateurs en ligne (facultatif).
    pub ports: Vec<DevicePorts>,
}

pub struct Replies {
    pub info: Option<Value>,
    pub sites: Vec<Site>,
}

fn is_switch(device: &Value) -> bool {
    text(device, "type").or_else(|| text(device, "deviceType")).is_some_and(|t| t == "switch")
}

/// Vitesse d'un port : l'énumération de l'API (1 = 10M, 2 = 100M, 3 = 1G,
/// 4 = 2,5G, 5 = 10G, 6 = 25G), ou directement des Mbit/s. À vérifier.
fn link_speed_bps(raw: f64) -> Option<f64> {
    let mbps = match raw as i64 {
        1 => 10.0,
        2 => 100.0,
        3 => 1000.0,
        4 => 2500.0,
        5 => 10_000.0,
        6 => 25_000.0,
        n if n >= 10 => n as f64,
        _ => return None,
    };
    Some(mbps * 1e6)
}

/// Les ports du détail d'un commutateur (`ports`). Les noms de champs sont
/// ceux de la documentation de l'API ouverte, avec quelques variantes : à
/// vérifier sur un contrôleur réel.
pub fn parse_ports(detail: &Value) -> Vec<Port> {
    detail
        .get("ports")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|port| {
            let index = match pick(port, &["port", "portId", "id"])? {
                Value::String(s) => s.clone(),
                other => (number(Some(other))? as i64).to_string(),
            };
            let up = pick_bool(port, &["status", "linkStatus"]);
            let poe_enabled = pick_bool(port, &["poe", "poeEnable", "poeStatus"]);
            Some(Port {
                index,
                name: pick_text(port, &["name", "portName"]).map(str::to_string),
                enabled: pick_bool(port, &["portEnable", "enable", "enabled"]),
                up,
                speed_bps: pick_number(port, &["linkSpeed", "speed"]).and_then(link_speed_bps),
                // `duplex`: 1 demi, 2 plein (0 = automatique, inconnu). À vérifier.
                full_duplex: pick_number(port, &["duplex", "linkDuplex"]).and_then(|d| {
                    match d as i64 {
                        1 => Some(false),
                        2 => Some(true),
                        _ => None,
                    }
                }),
                uplink: pick_bool(port, &["uplink", "isUplink"]).unwrap_or(false),
                poe_enabled,
                poe_watts: pick_number(port, &["poePower", "power"]),
                rx_bytes: pick_number(port, &["rx", "rxBytes"]),
                tx_bytes: pick_number(port, &["tx", "txBytes"]),
                rx_errors: pick_number(port, &["rxErrors", "rxErr"]),
                tx_errors: pick_number(port, &["txErrors", "txErr"]),
                neighbor: pick_text(port, &["lldpSystemName", "neighborName", "lldpName"]).map(
                    |name| {
                        (
                            name.to_string(),
                            pick_text(port, &["lldpPortId", "neighborPort"]).map(str::to_string),
                        )
                    },
                ),
            })
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Online,
    Offline,
    Pending,
}

/// L'état d'un appareil : `statusCategory` (ou `status`) vaut 0 déconnecté,
/// 1 connecté, 2 en attente d'adoption, 3 battement de cœur manquant,
/// 4 isolé ; certaines versions l'écrivent en toutes lettres.
fn state_of(device: &Value) -> Option<State> {
    let raw = device.get("statusCategory").or_else(|| device.get("status"))?;
    match raw {
        Value::String(word) => {
            let word = word.to_ascii_lowercase();
            if word.contains("pending") {
                Some(State::Pending)
            } else if word.contains("disconnect")
                || word.contains("heartbeat")
                || word.contains("isolated")
            {
                Some(State::Offline)
            } else if word.contains("connect") {
                Some(State::Online)
            } else {
                word.parse::<i64>().ok().and_then(state_from_code)
            }
        }
        other => state_from_code(number(Some(other))? as i64),
    }
}

fn state_from_code(code: i64) -> Option<State> {
    match code {
        1 => Some(State::Online),
        2 => Some(State::Pending),
        0 | 3 | 4 => Some(State::Offline),
        _ => None,
    }
}

pub fn samples(r: &Replies, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut out = Vec::new();

    if let Some(info) = r.info.as_ref() {
        out.push(
            g("omada_controller_info", 1.0)
                .with_label("version", text(info, "controllerVer").unwrap_or(""))
                .with_label("api_version", text(info, "apiVer").unwrap_or(""))
                .with_label("omadac_id", text(info, "omadacId").unwrap_or("")),
        );
    }
    out.push(g("omada_sites", r.sites.len() as f64));

    let (mut online, mut offline, mut pending) = (0usize, 0usize, 0usize);
    let mut named = 0usize;
    let mut clients_total = 0.0;
    let mut clients_known = false;
    for site in &r.sites {
        if let Some(clients) = site.clients {
            clients_known = true;
            clients_total += clients;
            out.push(g("omada_site_clients", clients).with_label("site", site.name.as_str()));
        }
        for device in &site.devices {
            let Some(state) = state_of(device) else { continue };
            match state {
                State::Online => online += 1,
                State::Offline => offline += 1,
                State::Pending => pending += 1,
            }
            named += 1;
            if named > MAX_NAMED {
                continue;
            }
            let mac = text(device, "mac").unwrap_or("");
            let name = text(device, "name").unwrap_or(mac);
            let kind = text(device, "type").or_else(|| text(device, "deviceType")).unwrap_or("");
            let labelled = |sample: Sample| {
                sample
                    .with_label("device", name)
                    .with_label("mac", mac)
                    .with_label("model", text(device, "model").unwrap_or(""))
                    .with_label("device_type", kind)
                    .with_label("site", site.name.as_str())
            };
            out.push(labelled(flag("omada_device_online", state == State::Online, ts_ms)));
            out.push(labelled(flag("omada_device_offline", state == State::Offline, ts_ms)));
            out.push(labelled(flag("omada_device_pending", state == State::Pending, ts_ms)));
            out.push(
                labelled(g("omada_device_info", 1.0))
                    .with_label("firmware", text(device, "firmwareVersion").unwrap_or(""))
                    .with_label("ip", text(device, "ip").unwrap_or("")),
            );
            // Un appareil hors ligne rapporte des zéros : ce ne sont pas des mesures.
            if state == State::Online {
                for (field, metric) in [
                    ("cpuUtil", "omada_device_cpu_usage_percent"),
                    ("memUtil", "omada_device_memory_used_percent"),
                    ("uptimeLong", "omada_device_uptime_seconds"),
                    ("clientNum", "omada_device_clients"),
                ] {
                    if let Some(value) = number(device.get(field)) {
                        out.push(labelled(g(metric, value)));
                    }
                }
            }
            if let Some(upgrade) = device.get("needUpgrade").and_then(Value::as_bool) {
                out.push(labelled(flag("omada_device_upgrade_available", upgrade, ts_ms)));
            }
        }
    }
    out.extend(switch_ports::samples(
        "omada",
        &r.sites.iter().flat_map(|site| site.ports.iter().cloned()).collect::<Vec<_>>(),
        ts_ms,
    ));
    for (status, count) in [("online", online), ("offline", offline), ("pending", pending)] {
        out.push(g("omada_devices", count as f64).with_label("status", status));
    }
    if clients_known {
        out.push(g("omada_clients", clients_total));
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::Router;
    use axum::http::{HeaderMap, StatusCode, Uri};
    use axum::response::IntoResponse;
    use axum::routing::{get, post};

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    // Réponses construites d'après la documentation de l'API ouverte d'Omada
    // (contrôleur 5.14) : un site avec un point d'accès sain, un commutateur
    // saturé qui attend une mise à jour, une passerelle déconnectée et un point
    // d'accès en attente d'adoption ; un second site sans histoire.
    const INFO: &str = include_str!("testdata/api_info.json");
    const TOKEN: &str = include_str!("testdata/token.json");
    const TOKEN_REFUSED: &str = include_str!("testdata/token_refused.json");
    const TOKEN_EXPIRED: &str = include_str!("testdata/token_expired.json");
    const SITES: &str = include_str!("testdata/sites.json");
    const DEVICES_DEFAULT: &str = include_str!("testdata/devices_default.json");
    const DEVICES_BRANCH: &str = include_str!("testdata/devices_branch.json");
    const CLIENTS: &str = include_str!("testdata/clients.json");
    // Détail d'un commutateur : noms de champs d'après la documentation, à
    // confirmer sur un contrôleur réel.
    const SWITCH_DETAIL: &str = include_str!("testdata/switch_detail.json");

    const NOW: i64 = 1_790_000_000_000;
    const OMADAC: &str = "a3f1c0d2e4b5967788990011aabbccdd";

    fn result(raw: &str) -> Value {
        serde_json::from_str::<Value>(raw).unwrap()["result"].clone()
    }

    fn data(raw: &str) -> Vec<Value> {
        result(raw)["data"].as_array().unwrap().clone()
    }

    fn replies() -> Replies {
        Replies {
            info: Some(result(INFO)),
            sites: vec![
                Site {
                    name: "Default".into(),
                    devices: data(DEVICES_DEFAULT),
                    clients: Some(42.0),
                    ports: vec![DevicePorts {
                        device: "SW-Core".into(),
                        unit: "AA-BB-CC-00-00-02".into(),
                        ports: parse_ports(&result(SWITCH_DETAIL)),
                    }],
                },
                Site {
                    name: "Branch".into(),
                    devices: data(DEVICES_BRANCH),
                    clients: Some(42.0),
                    ports: vec![],
                },
            ],
        }
    }

    #[test]
    fn appareils_etats_mesures_et_mises_a_jour() {
        let s = samples(&replies(), NOW);
        let info = find(&s, "omada_controller_info", &[]).unwrap();
        assert_eq!(info.labels["version"], "5.14.26.1");
        assert_eq!(value(&s, "omada_sites", &[]), 2.0);
        assert_eq!(value(&s, "omada_devices", &[("status", "online")]), 3.0);
        assert_eq!(value(&s, "omada_devices", &[("status", "offline")]), 1.0);
        assert_eq!(value(&s, "omada_devices", &[("status", "pending")]), 1.0);
        assert_eq!(value(&s, "omada_device_offline", &[("device", "ER605")]), 1.0);
        assert_eq!(value(&s, "omada_device_online", &[("device", "ER605")]), 0.0);
        assert_eq!(value(&s, "omada_device_online", &[("device", "AP-Hall")]), 1.0);
        // Sans nom, l'appareil en attente se présente par son adresse MAC.
        assert_eq!(value(&s, "omada_device_pending", &[("device", "AA-BB-CC-00-00-04")]), 1.0);
        assert_eq!(value(&s, "omada_device_cpu_usage_percent", &[("device", "SW-Core")]), 95.0);
        assert_eq!(value(&s, "omada_device_memory_used_percent", &[("device", "SW-Core")]), 91.0);
        assert!(find(&s, "omada_device_cpu_usage_percent", &[("device", "ER605")]).is_none());
        assert_eq!(value(&s, "omada_device_upgrade_available", &[("device", "SW-Core")]), 1.0);
        assert_eq!(value(&s, "omada_device_upgrade_available", &[("device", "AP-Hall")]), 0.0);
        assert_eq!(
            find(&s, "omada_device_online", &[("device", "AP-Branch")]).unwrap().labels["site"],
            "Branch"
        );
        assert_eq!(value(&s, "omada_site_clients", &[("site", "Default")]), 42.0);
        assert_eq!(value(&s, "omada_clients", &[]), 84.0);
    }

    #[test]
    fn ports_du_commutateur() {
        let s = samples(&replies(), NOW);
        let key = [("device", "SW-Core"), ("port", "1")];
        assert_eq!(value(&s, "omada_port_up", &key), 1.0);
        assert_eq!(value(&s, "omada_port_speed_bps", &key), 1e9);
        assert_eq!(value(&s, "omada_port_full_duplex", &key), 1.0);
        assert_eq!(value(&s, "omada_port_poe_watts", &key), 6.5);
        assert_eq!(value(&s, "omada_port_rx_bytes_total", &key), 123456.0);
        let n = find(&s, "omada_port_neighbor_info", &key).unwrap();
        assert_eq!(n.labels["remote_chassis"], "ap-hall");
        assert_eq!(n.labels["remote_port"], "eth0");
        let two = [("device", "SW-Core"), ("port", "2")];
        assert_eq!(value(&s, "omada_port_up", &two), 0.0);
        assert!(find(&s, "omada_port_speed_bps", &two).is_none());
        assert_eq!(value(&s, "omada_port_full_duplex", &[("port", "3")]), 0.0);
        let off = [("device", "SW-Core"), ("port", "4")];
        assert_eq!(value(&s, "omada_port_enabled", &off), 0.0);
        assert!(find(&s, "omada_port_up", &off).is_none());
        assert_eq!(value(&s, "omada_port_speed_bps", &[("port", "5")]), 10e9);
    }

    #[test]
    fn etats_en_chiffres_et_en_lettres() {
        let state = |raw: Value| state_of(&json!({ "status": raw }));
        assert_eq!(state(json!(1)), Some(State::Online));
        assert_eq!(state(json!(0)), Some(State::Offline));
        assert_eq!(state(json!(3)), Some(State::Offline));
        assert_eq!(state(json!(2)), Some(State::Pending));
        assert_eq!(state(json!("Connected")), Some(State::Online));
        assert_eq!(state(json!("Disconnected")), Some(State::Offline));
        assert_eq!(state(json!("Pending")), Some(State::Pending));
        assert_eq!(state(json!(99)), None);
        assert_eq!(state_of(&json!({})), None);
    }

    fn authorized(headers: &HeaderMap, token: &str) -> bool {
        headers.get("authorization").and_then(|v| v.to_str().ok())
            == Some(format!("AccessToken={token}").as_str())
    }

    #[tokio::test]
    async fn interrogation_complete_avec_reconnexion() {
        let logins = Arc::new(AtomicUsize::new(0));
        let app = Router::new()
            .route(
                "/openapi/authorize/token",
                post({
                    let logins = logins.clone();
                    move |uri: Uri, body: String| {
                        let logins = logins.clone();
                        async move {
                            let sent: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
                            let good = uri.query() == Some("grant_type=client_credentials")
                                && sent["client_id"] == "cid"
                                && sent["client_secret"] == "sec"
                                && sent["omadacId"] == OMADAC;
                            if !good {
                                return (StatusCode::OK, TOKEN_REFUSED.to_string()).into_response();
                            }
                            // Chaque connexion donne un jeton différent.
                            let n = logins.fetch_add(1, Ordering::SeqCst) + 1;
                            let body = TOKEN.replace("AT-TOKEN", &format!("AT-{n}"));
                            (StatusCode::OK, body).into_response()
                        }
                    }
                }),
            )
            .fallback(get(move |headers: HeaderMap, uri: Uri| async move {
                if uri.path() == "/api/info" {
                    return (StatusCode::OK, INFO).into_response();
                }
                // Le premier jeton (AT-1) est tenu pour expiré par le contrôleur.
                if !authorized(&headers, "AT-2") {
                    return (StatusCode::OK, TOKEN_EXPIRED).into_response();
                }
                let body = match uri.path() {
                    p if p == format!("/openapi/v1/{OMADAC}/sites") => SITES,
                    p if p == format!("/openapi/v1/{OMADAC}/sites/site-default/devices") => {
                        DEVICES_DEFAULT
                    }
                    p if p == format!("/openapi/v1/{OMADAC}/sites/site-branch/devices") => {
                        DEVICES_BRANCH
                    }
                    p if p == format!("/openapi/v1/{OMADAC}/sites/site-default/clients") => CLIENTS,
                    p if p
                        == format!(
                            "/openapi/v1/{OMADAC}/sites/site-default/switches/AA-BB-CC-00-00-02"
                        ) =>
                    {
                        SWITCH_DETAIL
                    }
                    // Un rôle sans droit sur les clients du second site.
                    _ => return (StatusCode::FORBIDDEN, "").into_response(),
                };
                (StatusCode::OK, body).into_response()
            }));
        let address = serve(app).await;

        // Sans identifiant de contrôleur : lu sur /api/info.
        let mut target = cible("omada", &format!("http://{address}"), &[]);
        target.credential =
            Credential::UsernamePassword { username: "cid".into(), password: "sec".into() };
        let collector = OmadaCollector::new();
        let s = collector.probe(&target).await.unwrap();
        assert_eq!(logins.load(Ordering::SeqCst), 2, "one reconnection after the stale token");
        assert_eq!(value(&s, "omada_devices", &[("status", "offline")]), 1.0);
        assert_eq!(value(&s, "omada_site_clients", &[("site", "Default")]), 42.0);
        assert!(find(&s, "omada_site_clients", &[("site", "Branch")]).is_none());
        assert!(find(&s, "omada_controller_info", &[]).is_some());
        assert_eq!(value(&s, "omada_port_up", &[("device", "SW-Core"), ("port", "1")]), 1.0);

        // Le jeton est gardé : pas de nouvelle connexion au passage suivant.
        collector.probe(&target).await.unwrap();
        assert_eq!(logins.load(Ordering::SeqCst), 2);

        // Un seul site, par son nom.
        let mut one = cible("omada", &format!("http://{address}"), &[("site", "default")]);
        one.credential = target.credential.clone();
        let s = collector.probe(&one).await.unwrap();
        assert_eq!(value(&s, "omada_sites", &[]), 1.0);
        let mut missing = cible("omada", &format!("http://{address}"), &[("site", "nope")]);
        missing.credential = target.credential.clone();
        let error = collector.probe(&missing).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");

        // Mauvais secret : erreur de configuration, jamais une panne.
        target.credential =
            Credential::UsernamePassword { username: "cid".into(), password: "wrong".into() };
        let error = OmadaCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn sans_client_openapi_c_est_une_erreur_de_configuration() {
        let target = cible("omada", "controller.lan", &[]);
        let error = OmadaCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }
}
