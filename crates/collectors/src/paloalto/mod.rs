//! Palo Alto Networks PAN-OS (pare-feu et VM-Series), par l'API XML
//! (`/api/`, requêtes `type=op`).
//!
//! BÊTA : construit d'après la documentation de l'API XML de PAN-OS, sans
//! pare-feu réel pour l'essayer.
//!
//! Un administrateur au rôle « Role Based » dont l'onglet « XML API » n'ouvre
//! que les requêtes opérationnelles (« Operational Requests »), jamais la
//! configuration. La clé d'API part dans l'en-tête `X-PAN-KEY`, jamais dans
//! l'URL (où l'ancienne forme `?key=` la laisserait dans les journaux). Les
//! requêtes partent en POST (formulaire) : l'API accepte les deux méthodes, et
//! le POST garde aussi le mot de passe hors de l'URL quand la clé est obtenue
//! par `type=keygen`.
//!
//! Une seule lecture obligatoire — `show system info` (modèle, version,
//! durée de fonctionnement) — et six facultatives, qu'un rôle plus restreint
//! ou un pare-feu sans grappe peut refuser sans faire échouer le reste :
//! ressources du plan de gestion, table des sessions, état HA, licences,
//! interfaces, dernière validation (commit).
//!
//! Les pannes que ce module existe pour voir : **un lien coupé** sur une
//! interface rattachée à une zone, **une grappe HA** amputée de son pair ou
//! désynchronisée, **la table des sessions qui se remplit**, **une licence qui
//! expire**, **une validation de configuration en échec**.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `443` | Port de l'interface de gestion (HTTPS seulement). |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable (celui d'origine est auto-signé). |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use roxmltree::Node;

use crate::rest::{MAX_NAMED, RestClient, TokenCache, flag, gauge, now_ms};

pub const DEFAULT_PORT: u16 = 443;
const PATH: &str = "/api/";

/// Durée pendant laquelle une clé obtenue par `keygen` est gardée.
const KEY_LIFETIME: Duration = Duration::from_secs(3600);

const SYSTEM: &str = "<show><system><info></info></system></show>";
const RESOURCES: &str = "<show><system><resources></resources></system></show>";
const SESSIONS: &str = "<show><session><info></info></session></show>";
const HIGH_AVAILABILITY: &str =
    "<show><high-availability><state></state></high-availability></show>";
const LICENSES: &str = "<request><license><info></info></license></request>";
const INTERFACES: &str = "<show><interface>all</interface></show>";
const JOBS: &str = "<show><jobs><all></all></jobs></show>";

#[derive(Default)]
pub struct PaloaltoCollector {
    keys: TokenCache,
}

impl PaloaltoCollector {
    pub fn new() -> Self {
        Self::default()
    }
}

struct Api<'a> {
    client: RestClient,
    keys: &'a TokenCache,
    /// Identifiants de `keygen`, quand la clé n'est pas fournie telle quelle.
    login: Option<(String, String)>,
    cache_key: String,
}

impl<'a> Api<'a> {
    fn new(collector: &'a PaloaltoCollector, target: &Target) -> Result<Self, ProbeError> {
        let mut client = RestClient::for_target(target, "https", DEFAULT_PORT, "PAN-OS")?;
        let mut login = None;
        let mut cache_key = String::new();
        match &target.credential {
            Credential::ApiToken { token } if !token.trim().is_empty() => {
                client.set_header("x-pan-key", token.trim());
            }
            Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
                login = Some((username.trim().to_string(), password.clone()));
                cache_key = format!("{}:{}", target.id, username.trim());
            }
            other => {
                return Err(ProbeError::Config(format!(
                    "PAN-OS expects an API key or the user name and password of an API \
                     administrator, configured: {other}"
                )));
            }
        }
        Ok(Self { client, keys: &collector.keys, login, cache_key })
    }

    /// Pose la clé d'API : celle du jeton, ou celle que `keygen` rend (gardée).
    async fn authenticate(&mut self) -> Result<(), ProbeError> {
        let Some((username, password)) = self.login.clone() else { return Ok(()) };
        if let Some(key) = self.keys.get(&self.cache_key) {
            self.client.set_header("x-pan-key", key);
            return Ok(());
        }
        let (status, body) = self
            .client
            .post_form(
                PATH,
                &[("type", "keygen"), ("user", username.as_str()), ("password", password.as_str())],
            )
            .await?;
        let body = self.answer(status, body)?;
        let document = roxmltree::Document::parse(&body)
            .map_err(|error| ProbeError::Protocol(format!("PAN-OS keygen answer: {error}")))?;
        let key = find(document.root_element(), "key")
            .and_then(|n| n.text())
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .ok_or_else(|| {
                ProbeError::Protocol("PAN-OS keygen answered without a key".to_string())
            })?;
        self.keys.put(&self.cache_key, key, KEY_LIFETIME);
        self.client.set_header("x-pan-key", key);
        Ok(())
    }

    /// Traduit une réponse HTTP en corps XML vérifié, ou en erreur typée.
    fn answer(&self, status: reqwest::StatusCode, body: String) -> Result<String, ProbeError> {
        match status.as_u16() {
            401 | 403 => Err(ProbeError::Auth(
                "PAN-OS refused the credentials (403): check the API key or the account, and \
                 that the administrator's role allows the XML API."
                    .to_string(),
            )),
            200..=299 => {
                check(&body)?;
                Ok(body)
            }
            _ => Err(self.client.status_error(status, &body, PATH)),
        }
    }

    /// Une requête opérationnelle.
    async fn op(&self, cmd: &str) -> Result<String, ProbeError> {
        let (status, body) = self.client.post_form(PATH, &[("type", "op"), ("cmd", cmd)]).await?;
        self.answer(status, body)
    }

    /// Une lecture facultative : un refus ou une commande inconnue ne fait
    /// rien échouer, une panne réelle si.
    async fn optional(&self, cmd: &str) -> Result<Option<String>, ProbeError> {
        match self.op(cmd).await {
            Ok(body) => Ok(Some(body)),
            Err(error) if error.means_down() => Err(error),
            Err(error) => {
                tracing::debug!(cmd, %error, "PAN-OS : lecture facultative indisponible");
                Ok(None)
            }
        }
    }
}

/// Vérifie l'enveloppe `<response status="…">`.
pub fn check(body: &str) -> Result<(), ProbeError> {
    let document = roxmltree::Document::parse(body).map_err(|error| {
        ProbeError::Protocol(format!(
            "PAN-OS did not answer in XML ({error}): check the address and the HTTPS port of the \
             management interface."
        ))
    })?;
    let root = document.root_element();
    if !root.has_tag_name("response") {
        return Err(ProbeError::Protocol(
            "The answer is not a PAN-OS XML API response".to_string(),
        ));
    }
    if root.attribute("status") == Some("success") {
        return Ok(());
    }
    let code = root.attribute("code").unwrap_or_default();
    let message: String = root
        .descendants()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(200)
        .collect();
    Err(match code {
        "403" | "16" => ProbeError::Auth(format!(
            "PAN-OS refused the request ({code}: {message}): check the API key and that the \
             role allows Operational Requests."
        )),
        _ => ProbeError::Protocol(format!("PAN-OS answered an error ({code}): {message}")),
    })
}

#[async_trait]
impl Collector for PaloaltoCollector {
    fn kind(&self) -> &'static str {
        "paloalto"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let mut api = Api::new(self, target)?;
        api.authenticate().await?;
        let system = match api.op(SYSTEM).await {
            Ok(body) => body,
            Err(error) => {
                if matches!(error, ProbeError::Auth(_)) {
                    self.keys.forget(&api.cache_key);
                }
                return Err(error);
            }
        };
        let (resources, sessions, ha, licenses, interfaces, jobs) = tokio::join!(
            api.optional(RESOURCES),
            api.optional(SESSIONS),
            api.optional(HIGH_AVAILABILITY),
            api.optional(LICENSES),
            api.optional(INTERFACES),
            api.optional(JOBS),
        );
        let replies = Replies {
            system,
            resources: resources?,
            sessions: sessions?,
            ha: ha?,
            licenses: licenses?,
            interfaces: interfaces?,
            jobs: jobs?,
        };
        Ok(samples(&replies, now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let mut api = Api::new(self, target)?;
        api.authenticate().await?;
        api.op(SYSTEM).await?;
        Ok(Some("paloalto".to_string()))
    }
}

pub struct Replies {
    pub system: String,
    pub resources: Option<String>,
    pub sessions: Option<String>,
    pub ha: Option<String>,
    pub licenses: Option<String>,
    pub interfaces: Option<String>,
    pub jobs: Option<String>,
}

/// Premier descendant de ce nom (le nœud lui-même compris).
fn find<'a, 'i>(node: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    node.descendants().find(|n| n.has_tag_name(name))
}

/// Texte du premier descendant de ce nom, s'il n'est pas vide.
fn dtext<'a>(node: Node<'a, '_>, name: &str) -> Option<&'a str> {
    find(node, name).and_then(|n| n.text()).map(str::trim).filter(|t| !t.is_empty())
}

fn dnum(node: Node<'_, '_>, name: &str) -> Option<f64> {
    dtext(node, name)?.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// `12 days, 3:04:05` ou `3:04:05` → secondes.
fn parse_uptime(raw: &str) -> Option<f64> {
    let (days, clock) = match raw.split_once("day") {
        Some((days, rest)) => (
            days.trim().parse::<f64>().ok()?,
            rest.trim_start_matches('s').trim_start_matches(',').trim(),
        ),
        None => (0.0, raw.trim()),
    };
    let mut parts = clock.split(':');
    let hours: f64 = parts.next()?.trim().parse().ok()?;
    let minutes: f64 = parts.next()?.trim().parse().ok()?;
    let seconds: f64 = parts.next()?.trim().parse().ok()?;
    Some(days * 86_400.0 + hours * 3600.0 + minutes * 60.0 + seconds)
}

/// Les couples (valeur, étiquette) d'une ligne de `top` :
/// `6.2 us, 90.9 id`, `6.2%us`, `8070268k total`, `2070000 buff/cache`.
fn figures(line: &str) -> Vec<(f64, String)> {
    let body = line.split_once(':').map(|(_, rest)| rest).unwrap_or(line);
    body.split(',')
        .filter_map(|segment| {
            let segment = segment.trim();
            let end = segment
                .find(|c: char| !(c.is_ascii_digit() || c == '.'))
                .unwrap_or(segment.len());
            let value: f64 = segment[..end].parse().ok()?;
            let label = segment[end..]
                .split_whitespace()
                .last()?
                .trim_start_matches('%')
                .to_ascii_lowercase();
            Some((value, label))
        })
        .collect()
}

fn figure(all: &[(f64, String)], label: &str) -> Option<f64> {
    all.iter().find(|(_, l)| l == label).map(|(v, _)| *v)
}

/// Processeur et mémoire du plan de gestion d'après la sortie texte de `top`.
/// Lecture prudente : un format inconnu ne rend rien plutôt qu'un faux chiffre.
pub fn parse_resources(text: &str) -> (Option<f64>, Option<f64>) {
    let cpu = text
        .lines()
        .find(|l| l.contains("Cpu(s)"))
        .and_then(|l| figure(&figures(l), "id"))
        .map(|idle| (100.0 - idle).clamp(0.0, 100.0));
    // Les lignes « Mem » puis « Swap » : on garde la première valeur de chaque
    // étiquette, celle de la mémoire vive.
    let memory_lines: Vec<(f64, String)> = text
        .lines()
        .filter(|l| l.contains("Mem") || l.contains("Swap"))
        .flat_map(figures)
        .collect();
    let total = figure(&memory_lines, "total").filter(|t| *t > 0.0);
    let free = figure(&memory_lines, "free");
    let memory = total.and_then(|total| {
        let used = if let (Some(free), Some(cache)) = (free, figure(&memory_lines, "buff/cache")) {
            total - free - cache
        } else if let (Some(free), Some(buffers), Some(cached)) =
            (free, figure(&memory_lines, "buffers"), figure(&memory_lines, "cached"))
        {
            total - free - buffers - cached
        } else {
            figure(&memory_lines, "used")?
        };
        Some((used / total * 100.0).clamp(0.0, 100.0))
    });
    (cpu, memory)
}

pub fn samples(r: &Replies, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    if let Ok(document) = roxmltree::Document::parse(&r.system) {
        system(document.root_element(), ts_ms, &mut out);
    }
    if let Some(Ok(document)) = r.resources.as_deref().map(roxmltree::Document::parse) {
        let text: String = document
            .root_element()
            .descendants()
            .filter(|n| n.is_text())
            .filter_map(|n| n.text())
            .collect();
        let (cpu, memory) = parse_resources(&text);
        if let Some(cpu) = cpu {
            out.push(gauge("paloalto_cpu_usage_percent", cpu, ts_ms));
        }
        if let Some(memory) = memory {
            out.push(gauge("paloalto_memory_used_percent", memory, ts_ms));
        }
    }
    if let Some(Ok(document)) = r.sessions.as_deref().map(roxmltree::Document::parse) {
        sessions(document.root_element(), ts_ms, &mut out);
    }
    if let Some(Ok(document)) = r.ha.as_deref().map(roxmltree::Document::parse) {
        ha(document.root_element(), ts_ms, &mut out);
    }
    if let Some(Ok(document)) = r.licenses.as_deref().map(roxmltree::Document::parse) {
        licenses(document.root_element(), ts_ms, &mut out);
    }
    if let Some(Ok(document)) = r.interfaces.as_deref().map(roxmltree::Document::parse) {
        interfaces(document.root_element(), ts_ms, &mut out);
    }
    if let Some(Ok(document)) = r.jobs.as_deref().map(roxmltree::Document::parse) {
        commit(document.root_element(), ts_ms, &mut out);
    }
    out
}

fn system(root: Node<'_, '_>, ts_ms: i64, out: &mut Vec<Sample>) {
    let Some(system) = find(root, "system") else { return };
    if let Some(version) = dtext(system, "sw-version") {
        out.push(
            gauge("paloalto_version_info", 1.0, ts_ms)
                .with_label("version", version)
                .with_label("model", dtext(system, "model").unwrap_or(""))
                .with_label("hostname", dtext(system, "hostname").unwrap_or("")),
        );
    }
    if let Some(uptime) = dtext(system, "uptime").and_then(parse_uptime) {
        out.push(gauge("paloalto_uptime_seconds", uptime, ts_ms));
    }
}

fn sessions(root: Node<'_, '_>, ts_ms: i64, out: &mut Vec<Sample>) {
    let active = dnum(root, "num-active");
    let max = dnum(root, "num-max");
    if let Some(active) = active {
        out.push(gauge("paloalto_sessions", active, ts_ms));
    }
    if let Some(max) = max {
        out.push(gauge("paloalto_sessions_max", max, ts_ms));
    }
    if let (Some(active), Some(max)) = (active, max)
        && max > 0.0
    {
        out.push(gauge("paloalto_session_usage_percent", active / max * 100.0, ts_ms));
    }
    for (tag, metric) in [
        ("cps", "paloalto_session_setup_rate"),
        ("pps", "paloalto_packets_per_second"),
        ("kbps", "paloalto_throughput_kbps"),
    ] {
        if let Some(value) = dnum(root, tag) {
            out.push(gauge(metric, value, ts_ms));
        }
    }
}

fn ha(root: Node<'_, '_>, ts_ms: i64, out: &mut Vec<Sample>) {
    let enabled = dtext(root, "enabled").is_some_and(|e| e.eq_ignore_ascii_case("yes"));
    out.push(flag("paloalto_ha_enabled", enabled, ts_ms));
    if !enabled {
        return;
    }
    let local = find(root, "local-info").and_then(|n| dtext(n, "state"));
    let peer_info = find(root, "peer-info");
    let peer = peer_info.and_then(|n| dtext(n, "state"));
    let connection = peer_info.and_then(|n| dtext(n, "conn-status"));
    if let Some(local) = local {
        out.push(
            gauge("paloalto_ha_state_info", 1.0, ts_ms)
                .with_label("state", local)
                .with_label("peer_state", peer.unwrap_or("unknown")),
        );
        let state = local.to_ascii_lowercase();
        out.push(flag(
            "paloalto_ha_local_functional",
            state != "suspended" && state != "non-functional",
            ts_ms,
        ));
    }
    if let Some(peer) = peer {
        let state = peer.to_ascii_lowercase();
        let healthy =
            matches!(state.as_str(), "active" | "passive" | "active-primary" | "active-secondary");
        let linked = connection.is_none_or(|c| c.eq_ignore_ascii_case("up"));
        out.push(flag("paloalto_ha_peer_up", healthy && linked, ts_ms));
    }
    if let Some(sync) = dtext(root, "running-sync") {
        out.push(flag("paloalto_ha_in_sync", sync.eq_ignore_ascii_case("synchronized"), ts_ms));
    }
}

/// `November 18, 2026` → secondes Unix (minuit UTC).
fn license_date(raw: &str) -> Option<f64> {
    let date = chrono::NaiveDate::parse_from_str(raw.trim(), "%B %d, %Y").ok()?;
    Some(date.and_hms_opt(0, 0, 0)?.and_utc().timestamp() as f64)
}

fn licenses(root: Node<'_, '_>, ts_ms: i64, out: &mut Vec<Sample>) {
    let Some(list) = find(root, "licenses") else { return };
    let now = ts_ms as f64 / 1000.0;
    let entries = list.children().filter(|n| n.has_tag_name("entry")).take(MAX_NAMED);
    for entry in entries {
        let Some(feature) = dtext(entry, "feature") else { continue };
        let expires = dtext(entry, "expires").and_then(license_date);
        let expired = match dtext(entry, "expired") {
            Some(flagged) => flagged.eq_ignore_ascii_case("yes"),
            None => expires.is_some_and(|at| at < now),
        };
        out.push(
            gauge("paloalto_license_status_info", 1.0, ts_ms)
                .with_label("license", feature)
                .with_label("status", if expired { "expired" } else { "active" }),
        );
        out.push(flag("paloalto_license_expired", expired, ts_ms).with_label("license", feature));
        // « Never » ne se lit pas comme une date : pas de compte à rebours.
        if let Some(at) = expires {
            out.push(
                gauge("paloalto_license_expiry_seconds", at - now, ts_ms)
                    .with_label("license", feature),
            );
        }
    }
}

fn interfaces(root: Node<'_, '_>, ts_ms: i64, out: &mut Vec<Sample>) {
    // État du lien de chaque port physique, d'après la section `hw`.
    let mut hardware: BTreeMap<&str, (&str, Option<f64>)> = BTreeMap::new();
    if let Some(hw) = find(root, "hw") {
        for entry in hw.children().filter(|n| n.has_tag_name("entry")) {
            let (Some(name), Some(state)) = (dtext(entry, "name"), dtext(entry, "state")) else {
                continue;
            };
            hardware.insert(name, (state, dnum(entry, "speed")));
        }
    }
    let Some(ifnet) = find(root, "ifnet") else { return };
    let (mut count, mut down) = (0, 0);
    for entry in ifnet.children().filter(|n| n.has_tag_name("entry")) {
        let Some(name) = dtext(entry, "name") else { continue };
        let zone = dtext(entry, "zone").unwrap_or("");
        let address = dtext(entry, "ip")
            .filter(|ip| {
                ip.split('/').next().is_some_and(|a| a.parse::<std::net::Ipv4Addr>().is_ok())
            });
        count += 1;
        // Seuls les ports physiques et les agrégats ont un lien à surveiller.
        let physical = name.starts_with("ethernet") || name.starts_with("ae");
        let link = hardware.get(name).filter(|_| physical);
        // Un port rattaché à une zone, sans lien : un câble ou un voisin en
        // moins. Un port libre (sans zone) n'a pas à avoir de lien.
        let is_down = !zone.is_empty()
            && link.is_some_and(|(state, _)| state.eq_ignore_ascii_case("down"));
        down += usize::from(is_down);
        if count > MAX_NAMED {
            continue;
        }
        let named = |sample: Sample| sample.with_label("interface", name).with_label("zone", zone);
        if let Some((state, speed)) = link {
            let up = state.eq_ignore_ascii_case("up");
            out.push(named(flag("paloalto_interface_link_up", up, ts_ms)));
            out.push(named(flag("paloalto_interface_down", is_down, ts_ms)));
            if up && let Some(speed) = speed {
                out.push(named(gauge("paloalto_interface_speed_mbps", *speed, ts_ms)));
            }
        }
        // Le réseau de l'interface, pour rattacher les équipements au pare-feu.
        if let Some(address) = address {
            let vlan = dnum(entry, "tag")
                .map(|v| v as u16)
                .filter(|v| (1..=4094).contains(v))
                .or_else(|| crate::ifaddr::vlan_hint(name));
            out.push(crate::ifaddr::address_sample(
                "paloalto_interface_address_info",
                name,
                zone,
                address,
                vlan,
                ts_ms,
            ));
        }
    }
    out.push(gauge("paloalto_interfaces_down", down as f64, ts_ms));
}

/// Résultat de la dernière validation de configuration terminée.
fn commit(root: Node<'_, '_>, ts_ms: i64, out: &mut Vec<Sample>) {
    let last = root
        .descendants()
        .filter(|n| {
            n.has_tag_name("job")
                && dtext(*n, "type") == Some("Commit")
                && dtext(*n, "status") == Some("FIN")
        })
        .max_by_key(|n| dtext(*n, "id").and_then(|id| id.parse::<u64>().ok()).unwrap_or(0));
    if let Some(job) = last {
        // Une validation avec avertissements (WARN) a bien été appliquée.
        let ok = matches!(dtext(job, "result"), Some("OK" | "WARN"));
        out.push(flag("paloalto_last_commit_ok", ok, ts_ms));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use axum::Router;
    use axum::extract::Form;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::post;

    use super::*;
    use crate::rest::test_support::{find as find_sample, serve, value};
    use crate::uptime::tags::test_support::cible;

    // Réponses construites d'après la documentation de l'API XML de PAN-OS
    // (`show system info`, `show high-availability state`, `show session
    // info`, `request license info`, `show interface all`, `show jobs all`) :
    // un pair HA désynchronisé, un port de DMZ sans lien, un port libre, une
    // licence expirée et une qui expire, une validation en échec.
    const SYSTEM_XML: &str = include_str!("testdata/system_info.xml");
    const HA_XML: &str = include_str!("testdata/ha_state.xml");
    const HA_DISABLED_XML: &str = include_str!("testdata/ha_disabled.xml");
    const SESSIONS_XML: &str = include_str!("testdata/session_info.xml");
    const RESOURCES_XML: &str = include_str!("testdata/system_resources.xml");
    const RESOURCES_LEGACY_XML: &str = include_str!("testdata/system_resources_legacy.xml");
    const LICENSES_XML: &str = include_str!("testdata/license_info.xml");
    const INTERFACES_XML: &str = include_str!("testdata/interfaces.xml");
    const JOBS_XML: &str = include_str!("testdata/jobs.xml");
    const INVALID_CREDENTIAL: &str = include_str!("testdata/error_invalid_credential.xml");
    const UNAUTHORIZED: &str = include_str!("testdata/error_unauthorized.xml");
    const KEYGEN: &str = include_str!("testdata/keygen.xml");

    // 2026-10-19 10:00:00 UTC.
    const NOW: i64 = 1_792_404_000_000;

    fn replies() -> Replies {
        Replies {
            system: SYSTEM_XML.to_string(),
            resources: Some(RESOURCES_XML.to_string()),
            sessions: Some(SESSIONS_XML.to_string()),
            ha: Some(HA_XML.to_string()),
            licenses: Some(LICENSES_XML.to_string()),
            interfaces: Some(INTERFACES_XML.to_string()),
            jobs: Some(JOBS_XML.to_string()),
        }
    }

    #[test]
    fn systeme_sessions_grappe_interfaces_licences() {
        let s = samples(&replies(), NOW);
        let info = find_sample(&s, "paloalto_version_info", &[]).unwrap();
        assert_eq!(
            (info.labels["version"].as_str(), info.labels["model"].as_str()),
            ("11.1.4-h7", "PA-440")
        );
        assert_eq!(
            value(&s, "paloalto_uptime_seconds", &[]),
            12.0 * 86_400.0 + 3.0 * 3600.0 + 4.0 * 60.0 + 5.0
        );
        assert!((value(&s, "paloalto_cpu_usage_percent", &[]) - 9.1).abs() < 0.01);
        assert!((value(&s, "paloalto_memory_used_percent", &[]) - 66.9).abs() < 0.1);

        assert_eq!(value(&s, "paloalto_sessions", &[]), 18_432.0);
        assert_eq!(value(&s, "paloalto_sessions_max", &[]), 262_144.0);
        assert!((value(&s, "paloalto_session_usage_percent", &[]) - 7.03).abs() < 0.01);
        assert_eq!(value(&s, "paloalto_session_setup_rate", &[]), 42.0);

        assert_eq!(value(&s, "paloalto_ha_enabled", &[]), 1.0);
        assert_eq!(value(&s, "paloalto_ha_peer_up", &[]), 1.0);
        assert_eq!(value(&s, "paloalto_ha_local_functional", &[]), 1.0);
        assert_eq!(value(&s, "paloalto_ha_in_sync", &[]), 0.0);
        assert_eq!(
            find_sample(&s, "paloalto_ha_state_info", &[]).unwrap().labels["peer_state"],
            "passive"
        );

        assert_eq!(value(&s, "paloalto_interface_down", &[("interface", "ethernet1/3")]), 1.0);
        assert_eq!(value(&s, "paloalto_interface_down", &[("interface", "ethernet1/1")]), 0.0);
        assert_eq!(
            value(&s, "paloalto_interface_speed_mbps", &[("interface", "ethernet1/1")]),
            1000.0
        );
        assert!(
            find_sample(&s, "paloalto_interface_link_up", &[("interface", "ethernet1/4")]).is_none()
                || value(&s, "paloalto_interface_down", &[("interface", "ethernet1/4")]) == 0.0,
            "free port without zone"
        );
        assert_eq!(value(&s, "paloalto_interfaces_down", &[]), 1.0);
        let address = find_sample(
            &s,
            "paloalto_interface_address_info",
            &[("interface", "ethernet1/2.30")],
        )
        .unwrap();
        assert_eq!(address.labels["vlan"], "30");
        assert!(
            find_sample(&s, "paloalto_interface_address_info", &[("interface", "tunnel.1")])
                .is_none()
        );

        let expired =
            |license: &str| value(&s, "paloalto_license_expired", &[("license", license)]);
        assert_eq!(expired("PAN-DB URL Filtering"), 1.0);
        assert_eq!(expired("Threat Prevention"), 0.0);
        let days = value(&s, "paloalto_license_expiry_seconds", &[("license", "Threat Prevention")])
            / 86_400.0;
        assert!((days - 29.58).abs() < 0.1, "{days}");
        assert!(
            find_sample(&s, "paloalto_license_expiry_seconds", &[("license", "PA-440")]).is_none(),
            "never expires"
        );

        assert_eq!(value(&s, "paloalto_last_commit_ok", &[]), 0.0);
    }

    #[test]
    fn sans_droits_facultatifs_ni_grappe() {
        let r = Replies {
            system: SYSTEM_XML.to_string(),
            resources: None,
            sessions: None,
            ha: Some(HA_DISABLED_XML.to_string()),
            licenses: None,
            interfaces: None,
            jobs: None,
        };
        let s = samples(&r, NOW);
        assert_eq!(value(&s, "paloalto_ha_enabled", &[]), 0.0);
        assert!(find_sample(&s, "paloalto_ha_peer_up", &[]).is_none());
        assert!(find_sample(&s, "paloalto_sessions", &[]).is_none());
        assert!(find_sample(&s, "paloalto_interfaces_down", &[]).is_none());
        assert!(find_sample(&s, "paloalto_cpu_usage_percent", &[]).is_none());
    }

    #[test]
    fn la_sortie_de_top_se_lit_prudemment() {
        let (cpu, memory) = parse_resources("rien d'utile ici");
        assert_eq!((cpu, memory), (None, None));
        let text = RESOURCES_LEGACY_XML;
        let (cpu, memory) = parse_resources(text);
        assert!((cpu.unwrap() - 11.0).abs() < 0.01);
        // 4045912 - 809182 libres - 80144 tampons - 1000000 cache.
        assert!((memory.unwrap() - 53.3).abs() < 0.1, "{memory:?}");
    }

    #[test]
    fn durees() {
        assert_eq!(parse_uptime("1 day, 0:00:10"), Some(86_410.0));
        assert_eq!(parse_uptime("0:05:00"), Some(300.0));
        assert_eq!(parse_uptime("n'importe quoi"), None);
    }

    #[test]
    fn enveloppes_refusees() {
        assert!(check(SYSTEM_XML).is_ok());
        assert!(matches!(check(INVALID_CREDENTIAL), Err(ProbeError::Auth(_))));
        assert!(matches!(check(UNAUTHORIZED), Err(ProbeError::Auth(_))));
        assert!(matches!(check("<html>login</html>"), Err(ProbeError::Protocol(_))));
        assert!(matches!(check("pas du xml"), Err(ProbeError::Protocol(_))));
        assert!(matches!(
            check(r#"<response status="error" code="12"><msg>Invalid object</msg></response>"#),
            Err(ProbeError::Protocol(_))
        ));
    }

    fn app() -> Router {
        Router::new().route(
            PATH,
            post(|headers: HeaderMap, Form(form): Form<HashMap<String, String>>| async move {
                let kind = form.get("type").map(String::as_str).unwrap_or_default();
                if kind == "keygen" {
                    return if form.get("password").map(String::as_str) == Some("s3cret") {
                        (StatusCode::OK, KEYGEN)
                    } else {
                        (StatusCode::FORBIDDEN, INVALID_CREDENTIAL)
                    };
                }
                let key = headers.get("x-pan-key").and_then(|v| v.to_str().ok());
                let minted = "LUFRPT1SR0lyb1dwSDQzRXBkL2lIWXJNcXpEbU1HQ3M9PQ==";
                if key != Some("ro-key") && key != Some(minted) {
                    return (StatusCode::FORBIDDEN, INVALID_CREDENTIAL);
                }
                let cmd = form.get("cmd").map(String::as_str).unwrap_or_default();
                let body = match cmd {
                    SYSTEM => SYSTEM_XML,
                    SESSIONS => SESSIONS_XML,
                    HIGH_AVAILABILITY => HA_DISABLED_XML,
                    INTERFACES => INTERFACES_XML,
                    // Rôle sans droit sur le reste.
                    _ => return (StatusCode::OK, UNAUTHORIZED),
                };
                (StatusCode::OK, body)
            }),
        )
    }

    #[tokio::test]
    async fn interrogation_complete_avec_une_cle() {
        let address = serve(app()).await;
        let mut target = cible("paloalto", &format!("http://{address}"), &[]);
        target.credential = Credential::ApiToken { token: "ro-key".into() };
        let s = PaloaltoCollector::new().probe(&target).await.unwrap();
        assert_eq!(value(&s, "paloalto_sessions", &[]), 18_432.0);
        assert_eq!(value(&s, "paloalto_interfaces_down", &[]), 1.0);
        assert_eq!(value(&s, "paloalto_ha_enabled", &[]), 0.0);
        assert!(find_sample(&s, "paloalto_license_status_info", &[]).is_none());

        target.credential = Credential::ApiToken { token: "revoked".into() };
        let error = PaloaltoCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn interrogation_avec_identifiants_et_cle_gardee() {
        let address = serve(app()).await;
        let mut target = cible("paloalto", &format!("http://{address}"), &[]);
        target.credential = Credential::UsernamePassword {
            username: "dumbmonit".into(),
            password: "s3cret".into(),
        };
        let collector = PaloaltoCollector::new();
        let s = collector.probe(&target).await.unwrap();
        assert_eq!(value(&s, "paloalto_sessions", &[]), 18_432.0);
        assert!(collector.keys.get("1:dumbmonit").is_some(), "key kept");

        target.credential = Credential::UsernamePassword {
            username: "dumbmonit".into(),
            password: "nope".into(),
        };
        let error = PaloaltoCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
    }

    #[tokio::test]
    async fn sans_identifiants_c_est_une_erreur_de_reglage() {
        let target = cible("paloalto", "http://127.0.0.1:1", &[]);
        let error = PaloaltoCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }
}
