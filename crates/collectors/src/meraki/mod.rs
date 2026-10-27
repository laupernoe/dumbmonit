//! Cisco Meraki, par l'API Dashboard v1 (`https://api.meraki.com/api/v1`).
//!
//! Une clé d'API d'un administrateur de l'organisation en lecture seule, envoyée
//! en `Authorization: Bearer`, jamais dans l'URL. C'est une API sur Internet :
//! l'adresse de la cible est celle de l'API, pas d'un équipement.
//!
//! Deux particularités de l'API que ce module absorbe :
//!
//! * **la redirection 307 vers le serveur de l'organisation** (`n123.meraki.com`) :
//!   le client partagé ne suit les redirections que vers le même hôte, la
//!   redirection est donc suivie ici, à la main, et seulement vers un hôte du
//!   même domaine que l'API (la clé ne part jamais ailleurs) ; le serveur
//!   trouvé est gardé pour le reste de l'interrogation ;
//! * **la limite de 10 requêtes par seconde et par organisation** : les
//!   lectures sont faites l'une après l'autre, et un 429 est retenté après
//!   `Retry-After` (au plus trois fois, cinq secondes d'attente au maximum).
//!
//! Lectures obligatoires : la liste des organisations et l'état des appareils
//! de l'organisation choisie. Facultatives, car une clé plus restreinte ou un
//! modèle de licence différent peut les refuser sans faire échouer le reste :
//! les réseaux (noms), l'état des liaisons Internet des routeurs, la licence
//! et le nombre de clients par réseau.
//!
//! Les pannes que ce module existe pour voir :
//!
//! * **un appareil hors ligne** (`offline`) ; `alerting` (en ligne mais avec un
//!   problème) est compté à part et `dormant` (absent depuis longtemps) n'est
//!   jamais une panne ;
//! * **une liaison Internet en échec** (`failed`) d'un routeur MX, et le
//!   basculement sur l'autre liaison qui en découle ;
//! * **une licence hors conformité ou qui expire**.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `organization` | vide | Identifiant ou nom de l'organisation ; vide = la première que la clé voit. |
//! | `networks` | vide | Identifiants ou noms des réseaux à surveiller, séparés par des virgules ; vide = tous. |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use reqwest::StatusCode;
use serde_json::Value;

use crate::rest::{
    MAX_NAMED, Options, RestClient, age_seconds, flag, gauge, now_ms, seconds_until, text,
};
use crate::selfhosted::options::tag;

pub const DEFAULT_PORT: u16 = 443;
pub const DEFAULT_ADDRESS: &str = "api.meraki.com";

const API: &str = "/api/v1";
/// Nombre de nouvelles tentatives après un 429.
const MAX_RETRIES: u8 = 3;
/// Attente par défaut avant une nouvelle tentative, sans `Retry-After`.
const RETRY_DELAY: Duration =
    if cfg!(test) { Duration::from_millis(10) } else { Duration::from_secs(1) };
/// Attente maximale acceptée d'un `Retry-After`.
const MAX_RETRY_AFTER: u64 = 5;
/// Redirections vers un serveur d'organisation suivies pour une même lecture.
const MAX_REDIRECTS: u8 = 3;
/// Au plus autant de réseaux dont on compte les clients à chaque passage.
const MAX_CLIENT_NETWORKS: usize = 25;
/// Fenêtre (secondes) des clients comptés : ceux vus dans la dernière heure.
const CLIENT_TIMESPAN: u64 = 3600;

#[derive(Default)]
pub struct MerakiCollector;

impl MerakiCollector {
    pub fn new() -> Self {
        Self
    }
}

/// Une réponse HTTP, avec les deux en-têtes qui comptent ici.
struct Reply {
    status: StatusCode,
    location: Option<String>,
    retry_after: Option<u64>,
    body: String,
}

struct Api {
    http: reqwest::Client,
    /// Pour décoder et traduire les statuts HTTP, comme les autres produits.
    client: RestClient,
    /// Racine courante : l'API, puis le serveur de l'organisation une fois la
    /// redirection rencontrée.
    base: String,
    key: String,
    timeout: Duration,
}

impl Api {
    fn new(target: &Target) -> Result<Self, ProbeError> {
        let key = match &target.credential {
            Credential::ApiToken { token } if !token.trim().is_empty() => token.trim().to_string(),
            other => {
                return Err(ProbeError::Config(format!(
                    "Meraki expects the API key of a read-only organization administrator, \
                     configured: {other}"
                )));
            }
        };
        let options = Options::from_target(target, "https", DEFAULT_PORT)?;
        let client = RestClient::for_target(target, "https", DEFAULT_PORT, "Meraki")?;
        Ok(Self {
            http: crate::http::client(options.insecure_tls)?,
            client,
            base: options.base_url,
            key,
            timeout: options.request_timeout,
        })
    }

    async fn send(&self, url: &str, path: &str) -> Result<Reply, ProbeError> {
        let response = self
            .http
            .get(url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", self.key))
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|error| self.transport(&error, path))?;
        let status = response.status();
        let header = |name: reqwest::header::HeaderName| {
            response.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_string)
        };
        let location = header(reqwest::header::LOCATION);
        let retry_after =
            header(reqwest::header::RETRY_AFTER).and_then(|v| v.trim().parse::<u64>().ok());
        let body = response.text().await.map_err(|error| self.transport(&error, path))?;
        Ok(Reply { status, location, retry_after, body })
    }

    fn transport(&self, error: &reqwest::Error, path: &str) -> ProbeError {
        if error.is_timeout() {
            return ProbeError::Timeout(self.timeout);
        }
        if error.is_builder() {
            return ProbeError::Config(format!("Malformed request to {path}: {error}"));
        }
        ProbeError::Unreachable(format!("{path}: {error}"))
    }

    /// L'adresse d'une redirection, si elle mène à un serveur Meraki : le même
    /// hôte, ou un nom du même domaine (`api.meraki.com` → `n123.meraki.com`),
    /// sans jamais quitter HTTPS pour HTTP.
    fn shard(&self, current: &str, location: &str) -> Option<reqwest::Url> {
        let next = reqwest::Url::parse(current).ok()?.join(location).ok()?;
        let home = reqwest::Url::parse(&self.base).ok()?;
        if next.scheme() != home.scheme() && next.scheme() != "https" {
            return None;
        }
        let host = next.host_str()?.to_ascii_lowercase();
        let home_host = home.host_str()?.to_ascii_lowercase();
        let same_domain = next.domain().is_some() && home.domain().is_some() && {
            let tail = |h: &str| h.rsplit('.').take(2).collect::<Vec<_>>().join(".");
            tail(&host) == tail(&home_host)
        };
        (host == home_host || same_domain).then_some(next)
    }

    /// GET d'un document JSON : redirection vers le serveur de l'organisation
    /// suivie, 429 retenté, tout autre statut non 2xx devient une erreur typée.
    async fn get(&mut self, path: &str) -> Result<Value, ProbeError> {
        let mut url = format!("{}{path}", self.base);
        let (mut redirects, mut waits) = (0u8, 0u8);
        loop {
            let reply = self.send(&url, path).await?;
            match reply.status.as_u16() {
                200..=299 => return self.client.decode(&reply.body, path),
                307 | 308 if redirects < MAX_REDIRECTS => {
                    let next =
                        reply.location.as_deref().and_then(|l| self.shard(&url, l)).ok_or_else(
                            || {
                                ProbeError::Protocol(format!(
                                    "Meraki redirected {path} to an address outside its own \
                                     domain: not followed."
                                ))
                            },
                        )?;
                    self.base = next.origin().ascii_serialization();
                    url = next.to_string();
                    redirects += 1;
                }
                429 if waits < MAX_RETRIES => {
                    let wait = reply
                        .retry_after
                        .map(|seconds| Duration::from_secs(seconds.min(MAX_RETRY_AFTER)))
                        .unwrap_or(RETRY_DELAY);
                    tokio::time::sleep(wait).await;
                    waits += 1;
                }
                401 => {
                    return Err(ProbeError::Auth(
                        "Meraki refused the API key (401): check the key, and that it was not \
                         revoked."
                            .to_string(),
                    ));
                }
                403 => {
                    return Err(ProbeError::Auth(format!(
                        "Meraki refused {path} (403): enable API access in Organization → \
                         Settings, and give the key's administrator access to the organization."
                    )));
                }
                _ => return Err(self.client.status_error(reply.status, &reply.body, path)),
            }
        }
    }

    /// Une lecture facultative : un refus, une route absente ou un modèle de
    /// licence qui ne la connaît pas ne fait rien échouer, une panne réelle si.
    async fn optional(&mut self, path: &str) -> Result<Option<Value>, ProbeError> {
        match self.get(path).await {
            Ok(reply) => Ok(Some(reply)),
            Err(error) if error.means_down() => Err(error),
            Err(error) => {
                tracing::debug!(path, %error, "Meraki : lecture facultative indisponible");
                Ok(None)
            }
        }
    }

    /// Le nombre de clients vus récemment, réseau par réseau.
    async fn clients(
        &mut self,
        networks: Option<&Value>,
        filter: &[String],
    ) -> Result<Vec<(String, usize)>, ProbeError> {
        let scope = selected_networks(networks, filter);
        let ids: Vec<String> = array(networks)
            .into_iter()
            .filter_map(|network| id_of(network, "id"))
            .filter(|id| in_scope(&scope, id))
            .filter(|id| id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'))
            .take(MAX_CLIENT_NETWORKS)
            .collect();
        let mut out = Vec::new();
        for id in ids {
            let path =
                format!("{API}/networks/{id}/clients?timespan={CLIENT_TIMESPAN}&perPage=1000");
            if let Some(Value::Array(clients)) = self.optional(&path).await? {
                out.push((id, clients.len()));
            }
        }
        Ok(out)
    }
}

#[async_trait]
impl Collector for MerakiCollector {
    fn kind(&self) -> &'static str {
        "meraki"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let mut api = Api::new(target)?;
        let filter = list(tag(target, "networks"));
        let organizations = api.get(&format!("{API}/organizations")).await?;
        let organization = pick_organization(&organizations, tag(target, "organization"))?.clone();
        let id = id_of(&organization, "id").unwrap_or_default();
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(ProbeError::Protocol("Meraki returned an organization without id".into()));
        }
        let base = format!("{API}/organizations/{id}");
        let statuses = api.get(&format!("{base}/devices/statuses?perPage=1000")).await?;
        if !statuses.is_array() {
            return Err(ProbeError::Protocol(
                "Meraki sent an unexpected answer on devices/statuses: a list was expected."
                    .to_string(),
            ));
        }
        let networks = api.optional(&format!("{base}/networks?perPage=1000")).await?;
        let uplinks = api.optional(&format!("{base}/uplinks/statuses?perPage=1000")).await?;
        let license_overview = api.optional(&format!("{base}/licenses/overview")).await?;
        // Les organisations à licence par appareil refusent la vue d'ensemble.
        let licenses = if license_overview.is_none() {
            api.optional(&format!("{base}/licenses?perPage=1000")).await?
        } else {
            None
        };
        let clients = api.clients(networks.as_ref(), &filter).await?;
        let replies = Replies {
            organization,
            networks,
            statuses,
            uplinks,
            license_overview,
            licenses,
            clients,
            filter,
        };
        Ok(samples(&replies, now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let mut api = Api::new(target)?;
        let organizations = api.get(&format!("{API}/organizations")).await?;
        pick_organization(&organizations, tag(target, "organization"))?;
        Ok(Some("meraki".to_string()))
    }
}

pub struct Replies {
    pub organization: Value,
    pub networks: Option<Value>,
    pub statuses: Value,
    pub uplinks: Option<Value>,
    pub license_overview: Option<Value>,
    pub licenses: Option<Value>,
    /// (identifiant du réseau, clients vus récemment).
    pub clients: Vec<(String, usize)>,
    /// Réseaux surveillés (identifiants ou noms) ; vide = tous.
    pub filter: Vec<String>,
}

/// Une liste séparée par des virgules.
fn list(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn array(value: Option<&Value>) -> Vec<&Value> {
    value.and_then(Value::as_array).into_iter().flatten().collect()
}

/// Un identifiant, qu'il arrive en texte ou en nombre.
fn id_of(value: &Value, key: &str) -> Option<String> {
    match value.get(key)? {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn lower(value: &Value, key: &str) -> String {
    text(value, key).map(str::to_ascii_lowercase).unwrap_or_default()
}

/// L'organisation demandée (identifiant ou nom), sinon la première.
fn pick_organization<'a>(
    organizations: &'a Value,
    wanted: Option<&str>,
) -> Result<&'a Value, ProbeError> {
    let all = array(Some(organizations));
    let Some(first) = all.first() else {
        return Err(ProbeError::Config(
            "The API key sees no Meraki organization: check the key and its administrator access."
                .to_string(),
        ));
    };
    let Some(wanted) = wanted else { return Ok(*first) };
    all.iter()
        .find(|org| {
            id_of(org, "id").is_some_and(|id| id == wanted)
                || text(org, "name").is_some_and(|name| name.eq_ignore_ascii_case(wanted))
        })
        .copied()
        .ok_or_else(|| {
            ProbeError::Config(format!(
                "The API key does not see an organization \"{wanted}\" (ID or name): leave the \
                 option empty for the first one."
            ))
        })
}

/// Les réseaux surveillés, par identifiant ; `None` = tous. Les noms sont
/// traduits en identifiants avec la liste des réseaux quand on l'a.
fn selected_networks(networks: Option<&Value>, filter: &[String]) -> Option<Vec<String>> {
    if filter.is_empty() {
        return None;
    }
    let mut ids: Vec<String> = filter.to_vec();
    for network in array(networks) {
        let (Some(id), Some(name)) = (id_of(network, "id"), text(network, "name")) else {
            continue;
        };
        if filter.iter().any(|item| item.eq_ignore_ascii_case(name)) {
            ids.push(id);
        }
    }
    Some(ids)
}

fn in_scope(scope: &Option<Vec<String>>, network_id: &str) -> bool {
    scope.as_ref().is_none_or(|ids| ids.iter().any(|id| id.eq_ignore_ascii_case(network_id)))
}

/// Secondes jusqu'à l'expiration : date RFC 3339 (licences par appareil) ou
/// « Feb 8, 2020 UTC » (vue d'ensemble de la licence unique).
fn expiry_seconds(raw: &str, now_ms: i64) -> Option<f64> {
    if let Some(seconds) = seconds_until(raw, now_ms) {
        return Some(seconds);
    }
    let day = raw.trim().trim_end_matches("UTC").trim();
    let date = chrono::NaiveDate::parse_from_str(day, "%b %d, %Y").ok()?;
    let at = date.and_hms_opt(0, 0, 0)?.and_utc().timestamp_millis();
    Some((at - now_ms) as f64 / 1000.0)
}

pub fn samples(r: &Replies, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut out = Vec::new();

    out.push(
        g("meraki_organization_info", 1.0)
            .with_label("organization", text(&r.organization, "name").unwrap_or(""))
            .with_label(
                "organization_id",
                id_of(&r.organization, "id").unwrap_or_default().as_str(),
            ),
    );

    let scope = selected_networks(r.networks.as_ref(), &r.filter);
    let network_names: HashMap<String, String> = array(r.networks.as_ref())
        .into_iter()
        .filter_map(|network| Some((id_of(network, "id")?, text(network, "name")?.to_string())))
        .collect();
    let network_name = |id: &str| network_names.get(id).cloned().unwrap_or_else(|| id.to_string());

    // Appareils : les comptes sont exacts, le détail est borné par MAX_NAMED.
    let mut counts: [(&str, usize); 4] =
        [("online", 0), ("alerting", 0), ("offline", 0), ("dormant", 0)];
    let mut device_names: HashMap<String, String> = HashMap::new();
    let mut named = 0usize;
    for device in array(Some(&r.statuses)) {
        let network_id = text(device, "networkId").unwrap_or("");
        if !in_scope(&scope, network_id) {
            continue;
        }
        let serial = text(device, "serial").unwrap_or("");
        let name = text(device, "name").unwrap_or(serial);
        device_names.insert(serial.to_string(), name.to_string());
        let status = lower(device, "status");
        let Some(slot) = counts.iter_mut().find(|slot| slot.0 == status) else { continue };
        slot.1 += 1;
        if status == "dormant" {
            continue;
        }
        named += 1;
        if named > MAX_NAMED {
            continue;
        }
        let network = network_name(network_id);
        let labelled = |sample: Sample| {
            sample
                .with_label("device", name)
                .with_label("serial", serial)
                .with_label("model", text(device, "model").unwrap_or(""))
                .with_label("product_type", text(device, "productType").unwrap_or(""))
                .with_label("network", network.as_str())
        };
        out.push(labelled(flag("meraki_device_online", status != "offline", ts_ms)));
        out.push(labelled(flag("meraki_device_offline", status == "offline", ts_ms)));
        out.push(labelled(flag("meraki_device_alerting", status == "alerting", ts_ms)));
        if let Some(age) = text(device, "lastReportedAt").and_then(|raw| age_seconds(raw, ts_ms)) {
            out.push(labelled(g("meraki_device_last_report_age_seconds", age)));
        }
    }
    for (label, count) in counts {
        out.push(g("meraki_devices", count as f64).with_label("status", label));
    }

    uplinks(r, &scope, &device_names, &network_name, ts_ms, &mut out);
    licenses(r, ts_ms, &mut out);

    let mut total = 0usize;
    for (id, count) in &r.clients {
        total += count;
        out.push(
            g("meraki_network_clients", *count as f64)
                .with_label("network", network_name(id).as_str())
                .with_label("network_id", id.as_str()),
        );
    }
    if !r.clients.is_empty() {
        out.push(g("meraki_clients", total as f64));
    }
    out
}

fn uplinks(
    r: &Replies,
    scope: &Option<Vec<String>>,
    device_names: &HashMap<String, String>,
    network_name: &dyn Fn(&str) -> String,
    ts_ms: i64,
    out: &mut Vec<Sample>,
) {
    if r.uplinks.is_none() {
        return;
    }
    let mut failed_total = 0usize;
    for entry in array(r.uplinks.as_ref()).into_iter().take(MAX_NAMED) {
        let network_id = text(entry, "networkId").unwrap_or("");
        if !in_scope(scope, network_id) {
            continue;
        }
        let serial = text(entry, "serial").unwrap_or("");
        let name = device_names.get(serial).map(String::as_str).unwrap_or(serial);
        let network = network_name(network_id);
        let uplinks = array(entry.get("uplinks"));
        for uplink in uplinks.iter().take(MAX_NAMED) {
            let Some(interface) = text(uplink, "interface") else { continue };
            let status = lower(uplink, "status");
            // « failed » : une liaison configurée qui a perdu Internet. « not
            // connected » est une prise sans câble (le second WAN d'un MX,
            // la carte cellulaire vide) : ce n'est pas une panne.
            let failed = status == "failed";
            failed_total += usize::from(failed);
            let labelled = |sample: Sample| {
                sample
                    .with_label("device", name)
                    .with_label("serial", serial)
                    .with_label("interface", interface)
                    .with_label("network", network.as_str())
            };
            out.push(labelled(flag(
                "meraki_uplink_up",
                matches!(status.as_str(), "active" | "ready"),
                ts_ms,
            )));
            out.push(labelled(flag("meraki_uplink_failed", failed, ts_ms)));
        }
        // Basculement : la liaison principale est tombée et une autre porte le trafic.
        let primary_failed = uplinks
            .iter()
            .any(|u| text(u, "interface") == Some("wan1") && lower(u, "status") == "failed");
        let other_active = uplinks
            .iter()
            .any(|u| text(u, "interface") != Some("wan1") && lower(u, "status") == "active");
        out.push(
            flag("meraki_uplink_failover", primary_failed && other_active, ts_ms)
                .with_label("device", name)
                .with_label("serial", serial)
                .with_label("network", network.as_str()),
        );
    }
    out.push(gauge("meraki_uplinks_failed", failed_total as f64, ts_ms));
}

fn licenses(r: &Replies, ts_ms: i64, out: &mut Vec<Sample>) {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    if let Some(overview) = r.license_overview.as_ref() {
        if let Some(status) = text(overview, "status") {
            out.push(flag("meraki_license_ok", status.eq_ignore_ascii_case("ok"), ts_ms));
            out.push(g("meraki_license_status_info", 1.0).with_label("status", status));
        }
        if let Some(seconds) =
            text(overview, "expirationDate").and_then(|raw| expiry_seconds(raw, ts_ms))
        {
            out.push(g("meraki_license_expiry_seconds", seconds));
        }
    } else if r.licenses.is_some() {
        // Licences par appareil : la plus proche échéance des licences en cours.
        let soonest = array(r.licenses.as_ref())
            .into_iter()
            .filter(|license| matches!(lower(license, "state").as_str(), "active" | "expiring"))
            .filter_map(|license| expiry_seconds(text(license, "expirationDate")?, ts_ms))
            .min_by(|a, b| a.total_cmp(b));
        if let Some(seconds) = soonest {
            out.push(g("meraki_license_expiry_seconds", seconds));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::Router;
    use axum::http::{HeaderMap, StatusCode, Uri};
    use axum::response::{IntoResponse, Response};
    use axum::routing::get;

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    // Réponses construites d'après la documentation de l'API Dashboard v1
    // (developer.cisco.com/meraki/api-v1) : deux organisations, deux réseaux,
    // un routeur dont le WAN 1 est en échec (basculé sur le WAN 2), un
    // commutateur en alerte, un point d'accès hors ligne, un autre dormant.
    const ORGANIZATIONS: &str = include_str!("testdata/organizations.json");
    const NETWORKS: &str = include_str!("testdata/networks.json");
    const STATUSES: &str = include_str!("testdata/devices_statuses.json");
    const UPLINKS: &str = include_str!("testdata/uplinks_statuses.json");
    const OVERVIEW: &str = include_str!("testdata/licenses_overview.json");
    const OVERVIEW_PROBLEM: &str = include_str!("testdata/licenses_overview_problem.json");
    const LICENSES: &str = include_str!("testdata/licenses_list.json");
    const CLIENTS: &str = include_str!("testdata/clients.json");

    const NOW: i64 = 1_790_000_000_000;

    fn json(raw: &str) -> Value {
        serde_json::from_str(raw).unwrap()
    }

    fn replies() -> Replies {
        Replies {
            organization: json(ORGANIZATIONS)[0].clone(),
            networks: Some(json(NETWORKS)),
            statuses: json(STATUSES),
            uplinks: Some(json(UPLINKS)),
            license_overview: Some(json(OVERVIEW)),
            licenses: None,
            clients: vec![("N_24329156".into(), 3), ("N_24329157".into(), 1)],
            filter: Vec::new(),
        }
    }

    #[test]
    fn appareils_liaisons_licence_clients() {
        let s = samples(&replies(), NOW);
        assert_eq!(value(&s, "meraki_devices", &[("status", "online")]), 2.0);
        assert_eq!(value(&s, "meraki_devices", &[("status", "alerting")]), 1.0);
        assert_eq!(value(&s, "meraki_devices", &[("status", "offline")]), 1.0);
        assert_eq!(value(&s, "meraki_devices", &[("status", "dormant")]), 1.0);
        assert_eq!(value(&s, "meraki_device_offline", &[("device", "AP-lobby")]), 1.0);
        assert_eq!(value(&s, "meraki_device_online", &[("device", "AP-lobby")]), 0.0);
        assert_eq!(value(&s, "meraki_device_offline", &[("device", "MS-core")]), 0.0);
        assert_eq!(value(&s, "meraki_device_alerting", &[("device", "MS-core")]), 1.0);
        assert_eq!(value(&s, "meraki_device_online", &[("device", "MS-core")]), 1.0);
        assert_eq!(
            value(&s, "meraki_device_last_report_age_seconds", &[("device", "MX-main")]),
            80.0
        );
        // Un appareil dormant n'a aucun échantillon propre : ni en ligne ni hors ligne.
        assert!(find(&s, "meraki_device_offline", &[("serial", "Q2MD-DDDD-0004")]).is_none());
        assert_eq!(
            find(&s, "meraki_device_offline", &[("device", "AP-lobby")]).unwrap().labels["network"],
            "Main office"
        );

        assert_eq!(
            value(&s, "meraki_uplink_failed", &[("device", "MX-main"), ("interface", "wan1")]),
            1.0
        );
        assert_eq!(
            value(&s, "meraki_uplink_up", &[("device", "MX-main"), ("interface", "wan2")]),
            1.0
        );
        assert_eq!(
            value(&s, "meraki_uplink_failed", &[("device", "MX-main"), ("interface", "cellular")]),
            0.0,
            "not connected is an empty socket"
        );
        assert_eq!(
            value(&s, "meraki_uplink_failed", &[("device", "MX-warehouse"), ("interface", "wan2")]),
            0.0
        );
        assert_eq!(value(&s, "meraki_uplink_failover", &[("device", "MX-main")]), 1.0);
        assert_eq!(value(&s, "meraki_uplink_failover", &[("device", "MX-warehouse")]), 0.0);
        assert_eq!(value(&s, "meraki_uplinks_failed", &[]), 1.0);

        assert_eq!(value(&s, "meraki_license_ok", &[]), 1.0);
        assert_eq!(value(&s, "meraki_license_expiry_seconds", &[]), 2_627_200.0);
        assert_eq!(value(&s, "meraki_network_clients", &[("network", "Main office")]), 3.0);
        assert_eq!(value(&s, "meraki_clients", &[]), 4.0);
        assert_eq!(
            find(&s, "meraki_organization_info", &[]).unwrap().labels["organization"],
            "Acme Corp"
        );
    }

    #[test]
    fn licence_hors_conformite_et_licences_par_appareil() {
        let mut r = replies();
        r.license_overview = Some(json(OVERVIEW_PROBLEM));
        let s = samples(&r, NOW);
        assert_eq!(value(&s, "meraki_license_ok", &[]), 0.0);

        r.license_overview = None;
        r.licenses = Some(json(LICENSES));
        let s = samples(&r, NOW);
        assert!(find(&s, "meraki_license_ok", &[]).is_none());
        // 2026-10-31 : la licence expirée de 2023 est ignorée.
        assert_eq!(value(&s, "meraki_license_expiry_seconds", &[]), 3_404_800.0);
    }

    #[test]
    fn un_reseau_choisi_par_son_nom_restreint_tout() {
        let mut r = replies();
        r.filter = vec!["warehouse".to_string()];
        let s = samples(&r, NOW);
        assert_eq!(value(&s, "meraki_devices", &[("status", "online")]), 1.0);
        assert_eq!(value(&s, "meraki_devices", &[("status", "offline")]), 0.0);
        assert!(find(&s, "meraki_uplink_up", &[("device", "MX-main")]).is_none());
        assert!(find(&s, "meraki_uplink_up", &[("device", "MX-warehouse")]).is_some());
    }

    #[test]
    fn dates_de_licence() {
        assert_eq!(expiry_seconds("Oct 22, 2026 UTC", NOW), Some(2_627_200.0));
        assert_eq!(expiry_seconds("2026-10-22T00:00:00Z", NOW), Some(2_627_200.0));
        assert_eq!(expiry_seconds("bientôt", NOW), None);
    }

    #[test]
    fn choix_de_l_organisation() {
        let orgs = json(ORGANIZATIONS);
        assert_eq!(text(pick_organization(&orgs, None).unwrap(), "name"), Some("Acme Corp"));
        assert_eq!(text(pick_organization(&orgs, Some("3318807")).unwrap(), "name"), Some("Lab"));
        assert_eq!(text(pick_organization(&orgs, Some("lab")).unwrap(), "name"), Some("Lab"));
        assert!(matches!(pick_organization(&orgs, Some("nope")), Err(ProbeError::Config(_))));
        assert!(matches!(pick_organization(&json("[]"), None), Err(ProbeError::Config(_))));
    }

    fn authorized(headers: &HeaderMap) -> bool {
        headers.get("authorization").and_then(|v| v.to_str().ok()) == Some("Bearer meraki-key")
    }

    #[tokio::test]
    async fn interrogation_complete_avec_redirection_et_limite_de_debit() {
        // Le « serveur de l'organisation », vers lequel l'API redirige.
        let throttled = Arc::new(AtomicUsize::new(0));
        let shard = Router::new().fallback(get({
            let throttled = throttled.clone();
            move |headers: HeaderMap, uri: Uri| {
                let throttled = throttled.clone();
                async move {
                    if !authorized(&headers) {
                        return StatusCode::UNAUTHORIZED.into_response();
                    }
                    let body = match uri.path() {
                        "/api/v1/organizations" => ORGANIZATIONS,
                        "/api/v1/organizations/2930418/devices/statuses" => STATUSES,
                        "/api/v1/organizations/2930418/networks" => {
                            // Première lecture limitée en débit, la suivante passe.
                            if throttled.fetch_add(1, Ordering::SeqCst) == 0 {
                                return (StatusCode::TOO_MANY_REQUESTS, "").into_response();
                            }
                            NETWORKS
                        }
                        "/api/v1/organizations/2930418/uplinks/statuses" => UPLINKS,
                        // Licence par appareil : la vue d'ensemble est refusée.
                        "/api/v1/organizations/2930418/licenses" => LICENSES,
                        "/api/v1/networks/N_24329156/clients" => CLIENTS,
                        _ => {
                            return (StatusCode::BAD_REQUEST, "{\"errors\":[\"no\"]}")
                                .into_response();
                        }
                    };
                    (StatusCode::OK, body).into_response()
                }
            }
        }));
        let shard_address = serve(shard).await;

        let api = Router::new().fallback(get({
            let shard_address = shard_address.clone();
            move |headers: HeaderMap, uri: Uri| {
                let shard_address = shard_address.clone();
                async move {
                    if !authorized(&headers) {
                        return StatusCode::UNAUTHORIZED.into_response();
                    }
                    if uri.path() == "/api/v1/organizations" {
                        return (StatusCode::OK, ORGANIZATIONS).into_response();
                    }
                    let location = format!(
                        "http://{shard_address}{}",
                        uri.path_and_query().map(|p| p.as_str()).unwrap_or("/")
                    );
                    Response::builder()
                        .status(StatusCode::TEMPORARY_REDIRECT)
                        .header("location", location)
                        .body(axum::body::Body::empty())
                        .unwrap()
                }
            }
        }));
        let address = serve(api).await;

        let mut target =
            cible("meraki", &format!("http://{address}"), &[("organization", "Acme Corp")]);
        target.credential = Credential::ApiToken { token: "meraki-key".into() };
        let s = MerakiCollector::new().probe(&target).await.unwrap();
        assert_eq!(value(&s, "meraki_devices", &[("status", "offline")]), 1.0);
        assert_eq!(value(&s, "meraki_uplinks_failed", &[]), 1.0);
        assert_eq!(value(&s, "meraki_network_clients", &[("network", "Main office")]), 3.0);
        assert!(find(&s, "meraki_network_clients", &[("network", "Warehouse")]).is_none());
        assert!(find(&s, "meraki_license_ok", &[]).is_none(), "per-device licensing");
        assert!(find(&s, "meraki_license_expiry_seconds", &[]).is_some());
        assert_eq!(throttled.load(Ordering::SeqCst), 2, "retried once after the 429");

        target.credential = Credential::ApiToken { token: "revoked".into() };
        let error = MerakiCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn une_redirection_hors_du_domaine_n_est_pas_suivie() {
        let api = Router::new().fallback(get(|uri: Uri| async move {
            if uri.path() == "/api/v1/organizations" {
                return Response::builder()
                    .status(StatusCode::TEMPORARY_REDIRECT)
                    .header("location", "https://evil.example.net/steal")
                    .body(axum::body::Body::empty())
                    .unwrap();
            }
            (StatusCode::NOT_FOUND, "").into_response()
        }));
        let address = serve(api).await;
        let mut target = cible("meraki", &format!("http://{address}"), &[]);
        target.credential = Credential::ApiToken { token: "meraki-key".into() };
        let error = MerakiCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Protocol(_)), "{error}");
    }

    #[tokio::test]
    async fn sans_cle_c_est_une_erreur_de_configuration() {
        let target = cible("meraki", "api.meraki.com", &[]);
        let error = MerakiCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
        assert!(!error.means_down());
    }
}
