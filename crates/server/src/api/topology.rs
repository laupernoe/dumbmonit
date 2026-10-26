//! Carte de topologie : `GET /api/topology`.
//!
//! Rien n'y est saisi à la main : chaque lien vient de ce que le serveur sait
//! déjà, et porte sa source pour que l'interface puisse le dire.
//!
//! - `parent` : la dépendance déclarée (`parent_id`), celle qui sert à la
//!   suppression des alertes ;
//! - `service` : un service trouvé par un agent (`agent_service`), rattaché à
//!   sa machine par `parent_id` lui aussi, mais qui n'est pas une dépendance
//!   d'infrastructure : la carte le dessine autrement ;
//! - `relay` : l'agent relais qui interroge l'équipement (`via_agent`) ;
//! - `hypervisor` : un hyperviseur et ses invités, lus dans les dernières
//!   mesures (Proxmox, XCP-ng, Nutanix, vSphere, libvirt de l'agent) ;
//! - `lldp` : un voisin LLDP qu'un commutateur SNMP a vu sur un port ;
//! - `samehost` : une surveillance PBS et une surveillance Proxmox VE qui
//!   visent la même machine (même hôte, ou même nom) ;
//! - `backup` : un Proxmox VE qui sauvegarde vers un PBS surveillé, lu dans
//!   ses stockages de type `pbs` (serveur et datastore).
//!
//! - `hosted` : ce qui tourne sur un invité — une cible dont l'adresse est
//!   l'une de celles que l'hyperviseur rapporte pour la VM ou le conteneur
//!   (agent QEMU, interfaces LXC, configuration statique, VMware Tools). Une
//!   adresse que plusieurs invités annoncent ne désigne personne, et un parent
//!   déclaré passe avant. Quand une seule cible « machine » (agent, ping, SNMP,
//!   pare-feu, PBS…) répond à l'adresse d'un invité inconnu par son nom, c'est
//!   elle qui représente l'invité : pas de nœud en double.
//!
//! - `firewall` : un équipement dont l'adresse tombe dans un réseau qu'un
//!   pare-feu surveillé du même site dessert (interfaces lues dans ses mesures,
//!   à défaut le /24 de son adresse). Un parent déclaré, un relais, un
//!   hyperviseur ou une sauvegarde passent avant : ce lien-là ne remplace rien.
//!
//! Chaque équipement porte aussi son sous-réseau (CIDR) et, si un pare-feu le
//! sait, le VLAN et le nom de ce réseau : l'interface s'en sert pour les zones.
//!
//! Un cluster Proxmox (au moins deux nœuds connus) se dessine cluster, nœuds,
//! invités : un nœud par nœud Proxmox, avec son état, et chaque invité sous le
//! nœud qui l'héberge. Un Proxmox à nœud unique garde ses invités directement
//! sous la surveillance : le nœud s'y confond.
//!
//! Un service trouvé par un agent prend le site de sa machine ; il ne sert
//! jamais de cible à un invité ou à un voisin, puisqu'il partage l'adresse de
//! sa machine.
//!
//! Le site d'un nœud se trouve sans configuration, du plus explicite au plus
//! déduit : l'étiquette `site` de la cible (sauf UniFi, où elle désigne le site
//! du contrôleur), le site que déclare son agent ou son agent relais, puis le
//! nom de l'agent relais qui l'interroge — un relais sert justement à joindre
//! un réseau distant. Sans aucun de ces indices, le nœud est au site principal.
//! L'emplacement affecté à la main (`targets.location_id`, « Place at ») passe
//! avant tout le reste : sur un emplacement distant, la cible est dessinée dans
//! le cadre de ce site ; sur l'emplacement du site principal, au site principal.
//!
//! Un invité ou un voisin n'est relié à un équipement surveillé que si son nom
//! désigne une seule cible : un nom ambigu ne crée aucun lien plutôt qu'un lien
//! faux. Un invité qui ne correspond à aucune cible devient un nœud à part,
//! rattaché à son hyperviseur ; un voisin LLDP inconnu, lui, n'apparaît pas —
//! la carte montre l'infrastructure surveillée, pas tout ce qui répond sur un
//! port.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr};

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use dumbmonit_proto::TargetId;
use serde::Serialize;
use sqlx::Row;
use tracing::warn;

use crate::api::ApiResult;
use crate::collectors::agent;
use crate::state::AppState;
use crate::tsdb::InstantSeries;

pub fn routes() -> Router<AppState> {
    Router::new().route("/topology", get(topology))
}

/// Invités gardés par hyperviseur : au-delà, la carte n'est plus lisible et
/// la page de l'hyperviseur reste le bon endroit pour les parcourir.
const MAX_GUESTS_PER_HOST: usize = 200;

/// Fenêtre de lecture des mesures : assez large pour un intervalle de collecte
/// lent, assez courte pour qu'une VM supprimée disparaisse dans l'heure.
const WINDOW: &str = "30m";

/// Séries d'état des invités, par collecteur : nom de métrique (sans préfixe),
/// étiquette qui porte le nom de l'invité, et valeur qui veut dire « en marche ».
const GUEST_SERIES: &[(&str, &str, f64)] = &[
    ("proxmox_guest_running", "name", 1.0),
    ("xcpng_vm_running", "vm", 1.0),
    ("nutanix_vm_running", "vm", 1.0),
    // vSphere code l'alimentation : 0 = poweredOn, 1 = poweredOff, 2 = suspended.
    ("vsphere_vm_power_state", "vm", 0.0),
    ("agent_libvirt_domain_running", "domain", 1.0),
];

#[derive(Debug, Serialize)]
pub struct TopologyView {
    pub nodes: Vec<NodeView>,
    pub edges: Vec<EdgeView>,
    /// Faux quand VictoriaMetrics n'a pas répondu : les liens `hypervisor` et
    /// `lldp` manquent alors, les autres restent exacts.
    pub metrics_available: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct NodeView {
    /// `t<id>` pour une cible, `g<hôte>:<nom>` pour un invité non surveillé,
    /// `n<hôte>:<nœud>` pour un nœud d'un cluster Proxmox.
    pub id: String,
    pub target_id: Option<TargetId>,
    pub name: String,
    /// Type de la cible ; `vm` ou `container` pour un invité, `pve_node` pour
    /// un nœud de cluster Proxmox.
    pub kind: String,
    /// Dossier de la cible ; pour un invité, celui de son hyperviseur.
    pub group_name: String,
    /// Site distant du nœud ; `None` pour le site principal.
    pub site: Option<String>,
    /// D'où vient `site` : `tag`, `agent` ou `relay`.
    pub site_source: Option<&'static str>,
    /// Invité ou nœud : la cible hyperviseur qui l'héberge.
    pub host_id: Option<TargetId>,
    /// Sous-réseau de l'équipement (`192.168.10.0/24`), déduit de son adresse
    /// IP et des réseaux que les pare-feux du site connaissent. Vide pour un
    /// invité, un nœud de cluster, ou un équipement sans adresse IP privée.
    pub subnet: Option<String>,
    /// VLAN de ce réseau, quand un pare-feu le donne.
    pub vlan: Option<u16>,
    /// Nom de ce réseau (description de l'interface du pare-feu).
    pub vlan_name: Option<String>,
    /// Invité ou nœud Proxmox : en marche (invité) ou en ligne (nœud) à la
    /// dernière mesure.
    pub running: Option<bool>,
    /// Invité (nœud propre ou cible qui le représente) : les adresses que son
    /// hyperviseur rapporte.
    pub ips: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EdgeView {
    /// Côté amont : parent, relais, hyperviseur, commutateur.
    pub from: String,
    pub to: String,
    /// `parent`, `service`, `relay`, `hypervisor`, `hosted`, `lldp`, `samehost`,
    /// `backup` ou `firewall`.
    pub source: &'static str,
}

/// Ce que la carte retient d'une cible.
#[derive(Debug, Clone)]
pub struct TargetRow {
    pub id: TargetId,
    pub name: String,
    pub address: String,
    pub kind: String,
    pub parent_id: Option<TargetId>,
    pub via_agent: Option<TargetId>,
    pub group_name: String,
    /// Étiquette `site` de la cible, si elle en a une non vide.
    pub site_tag: Option<String>,
    /// Nom d'hôte que déclare l'agent de la machine, s'il y en a un.
    pub hostname: Option<String>,
}

/// Un invité lu dans les mesures d'un hyperviseur.
#[derive(Debug, Clone, PartialEq)]
pub struct Guest {
    pub host: TargetId,
    /// Nœud Proxmox qui l'héberge, quand la mesure le dit.
    pub node: Option<String>,
    pub name: String,
    pub container: bool,
    pub running: bool,
}

/// Un nœud Proxmox lu dans `proxmox_node_up`.
#[derive(Debug, Clone, PartialEq)]
pub struct PveNode {
    pub host: TargetId,
    pub name: String,
    pub online: bool,
}

/// Un stockage `pbs` d'un Proxmox VE : le serveur de sauvegarde qu'il vise.
#[derive(Debug, Clone, PartialEq)]
pub struct PbsStorage {
    pub host: TargetId,
    pub server: String,
    pub datastore: String,
}

/// Ce que les mesures disent de Proxmox VE et de PBS, hors invités.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProxmoxFacts {
    pub nodes: Vec<PveNode>,
    pub storages: Vec<PbsStorage>,
    /// Datastores vus par chaque cible PBS : départagent deux PBS de même hôte.
    pub datastores: Vec<(TargetId, String)>,
    /// Réseaux des interfaces des pare-feux.
    pub networks: Vec<Network>,
    /// Adresses des invités, rapportées par leur hyperviseur.
    pub guest_ips: Vec<GuestIp>,
}

/// Une adresse d'invité lue dans `proxmox_guest_ip_info` ou `vsphere_vm_ip_info`.
#[derive(Debug, Clone, PartialEq)]
pub struct GuestIp {
    pub host: TargetId,
    /// Nom de l'invité, tel que les séries d'état le portent.
    pub name: String,
    pub ip: IpAddr,
}

/// Une adresse d'invité qui peut désigner la machine : ni boucle locale, ni
/// lien local, ni adresse non spécifiée ou multicast. Accepte un préfixe
/// (`/24`) ou une zone (`%eth0`), qui sont retirés.
pub fn guest_address(text: &str) -> Option<IpAddr> {
    let text = text.trim().split('/').next()?.split('%').next()?;
    let ip: IpAddr = text.parse().ok()?;
    let link_local = match ip {
        IpAddr::V4(v4) => v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_unicast_link_local(),
    };
    (!ip.is_loopback() && !ip.is_unspecified() && !ip.is_multicast() && !link_local).then_some(ip)
}

/// Un réseau IPv4 porté par une interface de pare-feu.
#[derive(Debug, Clone, PartialEq)]
pub struct Network {
    pub firewall: TargetId,
    pub cidr: Cidr,
    pub name: String,
    pub vlan: Option<u16>,
}

/// Un réseau IPv4 : adresse (masquée) et longueur de préfixe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cidr {
    network: u32,
    prefix: u8,
}

impl Cidr {
    /// `a.b.c.d/nn` : une adresse sans préfixe n'est pas un réseau.
    pub fn parse(text: &str) -> Option<Self> {
        let (ip, prefix) = text.trim().split_once('/')?;
        Self::new(ip.trim().parse().ok()?, prefix.trim().parse().ok()?)
    }

    pub fn new(ip: Ipv4Addr, prefix: u8) -> Option<Self> {
        (prefix <= 32).then(|| {
            let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
            Self { network: u32::from(ip) & mask, prefix }
        })
    }

    pub fn contains(&self, ip: Ipv4Addr) -> bool {
        let mask = if self.prefix == 0 { 0 } else { u32::MAX << (32 - self.prefix) };
        u32::from(ip) & mask == self.network
    }
}

impl std::fmt::Display for Cidr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", Ipv4Addr::from(self.network), self.prefix)
    }
}

/// Adresse privée (RFC 1918, lien local, CGNAT) : les adresses publiques ne
/// forment pas de zone, un contrôle HTTP d'un site tiers n'est pas « dans »
/// le réseau du pare-feu.
fn is_lan(ip: Ipv4Addr) -> bool {
    let [a, b, ..] = ip.octets();
    ip.is_private() || ip.is_link_local() || (a == 100 && (64..128).contains(&b))
}

/// L'adresse IPv4 d'une cible, quand son adresse en est une (un nom d'hôte
/// n'est pas résolu : la carte ne fait aucun accès réseau).
fn ipv4_of(address: &str) -> Option<Ipv4Addr> {
    host_key(address)?.parse().ok()
}

/// Un voisin LLDP vu par un commutateur.
#[derive(Debug, Clone, PartialEq)]
pub struct Neighbor {
    pub switch: TargetId,
    pub remote: String,
}

/// `GET /api/topology` : lecture seule, ouverte à tous les rôles comme la
/// liste des équipements. Les adresses des cibles n'y figurent pas ; seules y
/// sont celles que l'hyperviseur rapporte pour ses invités (`ips`).
pub async fn topology(State(state): State<AppState>) -> ApiResult<Json<TopologyView>> {
    let targets = load_targets(&state).await?;
    let sites = agent_sites(&state).await?;
    let placed = placed_sites(&state, &targets, &sites).await?;

    let ids: Vec<TargetId> = targets.iter().map(|t| t.id).collect();
    let (guests, neighbors, facts, metrics_available) = if targets.is_empty() {
        (Vec::new(), Vec::new(), ProxmoxFacts::default(), true)
    } else {
        let guests = state.victoria.query(&guests_query()).await;
        let neighbors = state.victoria.query(&lldp_query()).await;
        let facts = state.victoria.query(&facts_query()).await;
        let available = guests.is_ok() && neighbors.is_ok() && facts.is_ok();
        let failed = guests.as_ref().err().or(neighbors.as_ref().err()).or(facts.as_ref().err());
        if let Some(error) = failed {
            warn!(%error, "topologie : mesures illisibles, liens hyperviseur et LLDP omis");
        }
        (
            guests.map(|s| parse_guests(&s, &ids)).unwrap_or_default(),
            neighbors.map(|s| parse_neighbors(&s, &ids)).unwrap_or_default(),
            facts.map(|s| parse_facts(&s, &ids)).unwrap_or_default(),
            available,
        )
    };

    let (nodes, edges) = build_full(&targets, &sites, &placed, &guests, &neighbors, &facts);
    Ok(Json(TopologyView { nodes, edges, metrics_available }))
}

/// Les cibles telles que la carte les lit.
async fn load_targets(state: &AppState) -> ApiResult<Vec<TargetRow>> {
    let rows = sqlx::query(
        "SELECT t.id AS id, t.name AS name, t.address AS address, t.kind AS kind,
                t.parent_id AS parent_id, t.via_agent AS via_agent, t.group_name AS group_name,
                t.tags AS tags, h.hostname AS hostname
         FROM targets t LEFT JOIN agent_hosts h ON h.target_id = t.id
         ORDER BY t.id",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(anyhow::Error::from)?;
    let targets = rows
        .iter()
        .map(|row| {
            Ok(TargetRow {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                address: row.try_get("address")?,
                kind: row.try_get("kind")?,
                parent_id: row.try_get("parent_id")?,
                via_agent: row.try_get("via_agent")?,
                group_name: row.try_get("group_name")?,
                site_tag: site_tag(&row.try_get::<String, _>("tags")?),
                hostname: row
                    .try_get::<Option<String>, _>("hostname")?
                    .map(|h| h.trim().to_string())
                    .filter(|h| !h.is_empty()),
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(anyhow::Error::from)?;
    Ok(targets)
}

/// Le site que déclare chaque machine à agent.
async fn agent_sites(state: &AppState) -> ApiResult<HashMap<TargetId, String>> {
    Ok(agent::list_hosts(&state.pool)
        .await?
        .into_iter()
        .filter_map(|(id, _, info)| info.site.filter(|s| !s.trim().is_empty()).map(|s| (id, s)))
        .collect())
}

/// Le site distant de chaque cible (`None` : site principal), déduit comme sur
/// la carte de topologie, sans lire les mesures : les invités et les voisins
/// n'y changent rien. Sert aux emplacements de la carte du monde.
pub async fn target_sites(state: &AppState) -> ApiResult<HashMap<TargetId, Option<String>>> {
    let targets = load_targets(state).await?;
    let sites = agent_sites(state).await?;
    let (nodes, _) = build(&targets, &sites, &[], &[]);
    Ok(nodes.into_iter().filter_map(|n| n.target_id.map(|id| (id, n.site))).collect())
}

/// Site imposé par l'emplacement affecté à une cible : `Some(None)` pour
/// l'emplacement du site principal, `Some(Some((nom, "location")))` pour un
/// emplacement distant. Une cible sans affectation n'y figure pas.
pub type Placed = HashMap<TargetId, Option<(String, &'static str)>>;

/// Lit les affectations d'emplacement et en tire le site de chaque cible placée.
async fn placed_sites(
    state: &AppState,
    targets: &[TargetRow],
    sites: &HashMap<TargetId, String>,
) -> ApiResult<Placed> {
    let assigned = super::locations::assignments(&state.pool).await?;
    if assigned.is_empty() {
        return Ok(Placed::new());
    }
    let locations = super::locations::load(&state.pool).await?;
    let (base, _) = build(targets, sites, &[], &[]);
    let known: Vec<String> = base.into_iter().filter_map(|n| n.site).collect();
    Ok(placed_from(&assigned, &locations, &known))
}

/// Le site de chaque cible affectée à un emplacement connu. Un emplacement
/// distant s'appelle comme le site qu'il représente, à défaut comme lui-même ;
/// l'orthographe d'un site déjà connu (`known`) est reprise telle quelle pour
/// que « crissier » et « Crissier » ne fassent pas deux cadres.
pub fn placed_from(
    assigned: &HashMap<TargetId, super::locations::LocationId>,
    locations: &[super::locations::LocationRow],
    known: &[String],
) -> Placed {
    let by_id: HashMap<_, _> = locations.iter().map(|l| (l.id, l)).collect();
    assigned
        .iter()
        .filter_map(|(target, location)| {
            let location = by_id.get(location)?;
            if location.main_site {
                return Some((*target, None));
            }
            let name = location_name(location);
            if name.is_empty() {
                return Some((*target, None));
            }
            let spelled = known.iter().find(|k| k.trim().eq_ignore_ascii_case(&name));
            Some((*target, Some((spelled.cloned().unwrap_or(name), "location"))))
        })
        .collect()
}

fn location_name(location: &super::locations::LocationRow) -> String {
    let site = location.site.trim();
    if site.is_empty() { location.name.trim().to_string() } else { site.to_string() }
}

/// L'étiquette `site` d'un objet JSON d'étiquettes, si elle est une chaîne non vide.
pub fn site_tag(tags: &str) -> Option<String> {
    let tags: serde_json::Value = serde_json::from_str(tags).ok()?;
    let site = tags.get("site")?.as_str()?.trim();
    (!site.is_empty()).then(|| site.to_string())
}

fn guests_query() -> String {
    let names: Vec<&str> = GUEST_SERIES.iter().map(|(name, _, _)| *name).collect();
    format!(
        "last_over_time({{__name__=~\"dumbmonit_({})\"}}[{WINDOW}]) keep_metric_names",
        names.join("|")
    )
}

/// Séries qui décrivent Proxmox VE et PBS : état des nœuds, stockages `pbs`
/// (serveur et datastore visés) et datastores vus par chaque PBS ; réseaux des
/// pare-feux ; adresses des invités (Proxmox VE, vSphere).
const FACT_SERIES: &str = "dumbmonit_(proxmox_node_up|proxmox_storage_pbs_info|pbs_datastore_available\
    |opnsense_interface_address_info|pfsense_interface_address_info\
    |fortigate_interface_address_info|paloalto_interface_address_info\
    |proxmox_guest_ip_info|vsphere_vm_ip_info)";

fn facts_query() -> String {
    format!("last_over_time({{__name__=~\"{FACT_SERIES}\"}}[{WINDOW}]) keep_metric_names")
}

fn lldp_query() -> String {
    format!("last_over_time(dumbmonit_lldp_neighbor_info[{WINDOW}])")
}

fn target_label(series: &InstantSeries, known: &[TargetId]) -> Option<TargetId> {
    let id = series.metric.get("target")?.parse().ok()?;
    known.contains(&id).then_some(id)
}

/// Invités lus dans les séries d'état, dédoublonnés par (hôte, nom).
pub fn parse_guests(series: &[InstantSeries], known: &[TargetId]) -> Vec<Guest> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for s in series {
        let Some(host) = target_label(s, known) else { continue };
        let Some(metric) = s.metric.get("__name__") else { continue };
        let metric = metric.strip_prefix(crate::tsdb::METRIC_PREFIX).unwrap_or(metric);
        let Some((_, label, running_value)) = GUEST_SERIES.iter().find(|(m, _, _)| *m == metric)
        else {
            continue;
        };
        let Some(name) = s.metric.get(*label).map(|n| n.trim()).filter(|n| !n.is_empty()) else {
            continue;
        };
        let Ok(value) = s.value.1.parse::<f64>() else { continue };
        if !value.is_finite() || !seen.insert((host, name.to_lowercase())) {
            continue;
        }
        let node = (metric == "proxmox_guest_running")
            .then(|| s.metric.get("node").map(|n| n.trim()).filter(|n| !n.is_empty()))
            .flatten()
            .map(str::to_string);
        out.push(Guest {
            host,
            node,
            name: name.to_string(),
            container: s.metric.get("type").is_some_and(|t| t == "lxc"),
            running: value == *running_value,
        });
    }
    out
}

/// Nœuds Proxmox, stockages `pbs` et datastores PBS lus dans les mesures.
pub fn parse_facts(series: &[InstantSeries], known: &[TargetId]) -> ProxmoxFacts {
    let mut facts = ProxmoxFacts::default();
    let mut seen = HashSet::new();
    for s in series {
        let Some(host) = target_label(s, known) else { continue };
        let Some(metric) = s.metric.get("__name__") else { continue };
        let metric = metric.strip_prefix(crate::tsdb::METRIC_PREFIX).unwrap_or(metric);
        let label = |key: &str| s.metric.get(key).map(|v| v.trim()).filter(|v| !v.is_empty());
        let Ok(value) = s.value.1.parse::<f64>() else { continue };
        if !value.is_finite() {
            continue;
        }
        match metric {
            "proxmox_node_up" => {
                let Some(name) = label("node") else { continue };
                if seen.insert(("n", host, name.to_lowercase(), String::new())) {
                    facts.nodes.push(PveNode {
                        host,
                        name: name.to_string(),
                        online: value >= 1.0,
                    });
                }
            }
            "proxmox_storage_pbs_info" => {
                let Some(server) = label("server") else { continue };
                let datastore = label("datastore").unwrap_or_default();
                if seen.insert(("s", host, server.to_lowercase(), datastore.to_lowercase())) {
                    facts.storages.push(PbsStorage {
                        host,
                        server: server.to_string(),
                        datastore: datastore.to_string(),
                    });
                }
            }
            "pbs_datastore_available" => {
                let Some(name) = label("datastore") else { continue };
                if seen.insert(("d", host, name.to_lowercase(), String::new())) {
                    facts.datastores.push((host, name.to_string()));
                }
            }
            "proxmox_guest_ip_info" | "vsphere_vm_ip_info" => {
                let name = if metric == "vsphere_vm_ip_info" { "vm" } else { "name" };
                let Some(name) = label(name) else { continue };
                let Some(ip) = label("ip").and_then(guest_address) else { continue };
                if seen.insert(("a", host, name.to_lowercase(), ip.to_string())) {
                    facts.guest_ips.push(GuestIp { host, name: name.to_string(), ip });
                }
            }
            m if m.ends_with("_interface_address_info") => {
                let Some(cidr) = label("address").and_then(Cidr::parse) else { continue };
                // Une liaison point à point ou un réseau public ne dessine pas de zone.
                if cidr.prefix > 30 || !is_lan(Ipv4Addr::from(cidr.network)) {
                    continue;
                }
                let interface = label("interface").unwrap_or_default();
                let name = label("description").unwrap_or(interface);
                if seen.insert(("i", host, cidr.to_string(), interface.to_lowercase())) {
                    facts.networks.push(Network {
                        firewall: host,
                        cidr,
                        name: name.to_string(),
                        vlan: label("vlan").and_then(|v| v.parse().ok()),
                    });
                }
            }
            _ => {}
        }
    }
    facts
}

/// Voisins LLDP, par (commutateur, nom système distant).
pub fn parse_neighbors(series: &[InstantSeries], known: &[TargetId]) -> Vec<Neighbor> {
    let mut seen = HashSet::new();
    series
        .iter()
        .filter_map(|s| {
            let switch = target_label(s, known)?;
            let remote = s.metric.get("remote_sysname")?.trim();
            (!remote.is_empty() && seen.insert((switch, remote.to_lowercase())))
                .then(|| Neighbor { switch, remote: remote.to_string() })
        })
        .collect()
}

/// Clé de comparaison d'un nom d'hôte : en minuscules, sans schéma, port ni
/// chemin, et réduit au premier label DNS (`web01.lan` et `web01` désignent la
/// même machine). Une adresse IP reste entière.
pub fn host_key(text: &str) -> Option<String> {
    let mut s = text.trim().to_lowercase();
    if let Some((_, rest)) = s.split_once("://") {
        s = rest.to_string();
    }
    if let Some((_, rest)) = s.rsplit_once('@') {
        s = rest.to_string();
    }
    let s = s.split(['/', '?', '#']).next().unwrap_or_default();
    let s = s.trim_start_matches('[');
    if let Ok(ip) = s.split(']').next().unwrap_or_default().parse::<IpAddr>() {
        return Some(ip.to_string());
    }
    // Un seul « : » : un port à retirer ; plusieurs : une IPv6 déjà traitée.
    let host = match s.matches(':').count() {
        1 => s.split(':').next().unwrap_or_default(),
        _ => s,
    };
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Some(ip.to_string());
    }
    let first = host.split('.').next().unwrap_or_default().trim();
    (!first.is_empty()).then(|| first.to_string())
}

/// Index des cibles par nom, pour relier un invité ou un voisin à la cible qui
/// le surveille.
struct NameIndex {
    exact: HashMap<String, Vec<TargetId>>,
    name_key: HashMap<String, Vec<TargetId>>,
    address_key: HashMap<String, Vec<TargetId>>,
}

impl NameIndex {
    fn new(targets: &[TargetRow]) -> Self {
        let mut index =
            Self { exact: HashMap::new(), name_key: HashMap::new(), address_key: HashMap::new() };
        // Un service d'agent porte l'adresse de sa machine (`hôte#module`) :
        // l'indexer rendrait la machine ambiguë pour chaque invité ou voisin.
        // Le nom d'hôte que déclare un agent compte comme un second nom.
        let add = |list: &mut Vec<TargetId>, id: TargetId| {
            if !list.contains(&id) {
                list.push(id);
            }
        };
        for t in targets.iter().filter(|t| t.kind != agent::modules::SERVICE_KIND) {
            for name in std::iter::once(t.name.as_str()).chain(t.hostname.as_deref()) {
                if name.trim().is_empty() {
                    continue;
                }
                add(index.exact.entry(name.trim().to_lowercase()).or_default(), t.id);
                if let Some(key) = host_key(name) {
                    add(index.name_key.entry(key).or_default(), t.id);
                }
            }
            if let Some(key) = host_key(&t.address) {
                index.address_key.entry(key).or_default().push(t.id);
            }
        }
        index
    }

    /// Vrai quand `name` désigne au moins une cible autre que `except`, même
    /// de façon ambiguë.
    fn is_known(&self, name: &str, except: TargetId) -> bool {
        let key = host_key(name);
        [
            self.exact.get(&name.trim().to_lowercase()),
            key.as_ref().and_then(|k| self.name_key.get(k)),
            key.as_ref().and_then(|k| self.address_key.get(k)),
        ]
        .into_iter()
        .flatten()
        .any(|tier| tier.iter().any(|id| *id != except))
    }

    /// La cible désignée par `name`, autre que `except` : du plus sûr (nom
    /// identique) au plus large (même hôte dans l'adresse). Plusieurs candidats
    /// au premier palier qui en trouve : ambigu, aucun lien.
    fn find(&self, name: &str, except: TargetId) -> Option<TargetId> {
        let key = host_key(name);
        let tiers = [
            self.exact.get(&name.trim().to_lowercase()),
            key.as_ref().and_then(|k| self.name_key.get(k)),
            key.as_ref().and_then(|k| self.address_key.get(k)),
        ];
        for tier in tiers.into_iter().flatten() {
            let mut candidates: Vec<TargetId> =
                tier.iter().copied().filter(|id| *id != except).collect();
            candidates.dedup();
            match candidates.as_slice() {
                [] => continue,
                [one] => return Some(*one),
                _ => return None,
            }
        }
        None
    }
}

/// Types dont la surveillance désigne la machine elle-même plutôt qu'un service
/// qui tourne dessus : à l'adresse d'un invité, une telle cible *est* l'invité.
const MACHINE_KINDS: &[&str] = &[
    "agent",
    "ping",
    "snmp",
    "opnsense",
    "pfsense",
    "fortigate",
    "sophos",
    "paloalto",
    "sonicwall",
    "mikrotik",
    "pbs",
    "pmg",
    "truenas",
    "unraid",
    "qnap",
];

/// Types dont la surveillance désigne un pare-feu ou une passerelle de site.
const FIREWALL_KINDS: &[&str] =
    &["opnsense", "pfsense", "fortigate", "sophos", "paloalto", "sonicwall", "mikrotik"];

fn is_loopback(key: Option<&str>) -> bool {
    key.is_some_and(|k| k == "localhost" || k.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback()))
}

fn node_id(id: TargetId) -> String {
    format!("t{id}")
}

/// Assemble nœuds et liens. Une paire de nœuds n'est reliée qu'une fois, par
/// la source la plus sûre (déclarée avant déduite), et quel que soit le sens.
pub fn build(
    targets: &[TargetRow],
    sites: &HashMap<TargetId, String>,
    guests: &[Guest],
    neighbors: &[Neighbor],
) -> (Vec<NodeView>, Vec<EdgeView>) {
    build_placed(targets, sites, &Placed::new(), guests, neighbors)
}

/// Comme [`build`], avec les sites imposés par les emplacements affectés.
pub fn build_placed(
    targets: &[TargetRow],
    sites: &HashMap<TargetId, String>,
    placed: &Placed,
    guests: &[Guest],
    neighbors: &[Neighbor],
) -> (Vec<NodeView>, Vec<EdgeView>) {
    build_full(targets, sites, placed, guests, neighbors, &ProxmoxFacts::default())
}

/// Comme [`build_placed`], avec ce que les mesures disent de Proxmox VE et de
/// PBS : nœuds de cluster, sauvegardes, machines partagées.
pub fn build_full(
    targets: &[TargetRow],
    sites: &HashMap<TargetId, String>,
    placed: &Placed,
    guests: &[Guest],
    neighbors: &[Neighbor],
    facts: &ProxmoxFacts,
) -> (Vec<NodeView>, Vec<EdgeView>) {
    let by_id: HashMap<TargetId, &TargetRow> = targets.iter().map(|t| (t.id, t)).collect();
    // Agents qui relaient au moins une cible connue.
    let relays: HashSet<TargetId> =
        targets.iter().filter_map(|t| t.via_agent).filter(|r| by_id.contains_key(r)).collect();
    // UniFi lit `site` comme le site du contrôleur, pas comme un lieu.
    let tagged = |t: &TargetRow| t.site_tag.clone().filter(|_| t.kind != "unifi");
    // Ce qu'une cible dit d'elle-même : étiquette, puis site déclaré par son agent.
    let declared = |t: &TargetRow| -> Option<(String, &'static str)> {
        tagged(t).map(|s| (s, "tag")).or_else(|| sites.get(&t.id).map(|s| (s.clone(), "agent")))
    };
    let own = |t: &TargetRow| -> Option<(String, &'static str)> {
        declared(t)
            .or_else(|| {
                let relay = by_id.get(&t.via_agent?)?;
                declared(relay).or_else(|| Some((relay.name.clone(), "relay")))
            })
            .or_else(|| relays.contains(&t.id).then(|| (t.name.clone(), "relay")))
    };
    // L'emplacement affecté l'emporte sur tout ce qui se déduit.
    let own_placed = |t: &TargetRow| -> Option<(String, &'static str)> {
        match placed.get(&t.id) {
            Some(site) => site.clone(),
            None => own(t),
        }
    };
    let site_of = |t: &TargetRow| -> Option<(String, &'static str)> {
        own_placed(t).or_else(|| {
            (t.kind == agent::modules::SERVICE_KIND)
                .then(|| t.parent_id.and_then(|p| by_id.get(&p)).and_then(|h| own_placed(h)))
                .flatten()
        })
    };
    let mut nodes: Vec<NodeView> = targets
        .iter()
        .map(|t| {
            let site = site_of(t);
            NodeView {
                id: node_id(t.id),
                target_id: Some(t.id),
                name: t.name.clone(),
                kind: t.kind.clone(),
                group_name: t.group_name.clone(),
                site_source: site.as_ref().map(|(_, source)| *source),
                site: site.map(|(name, _)| name),
                host_id: None,
                subnet: None,
                vlan: None,
                vlan_name: None,
                running: None,
                ips: Vec::new(),
            }
        })
        .collect();

    let mut edges = Vec::new();
    let mut linked: HashSet<(String, String)> = HashSet::new();
    let mut link = |from: String, to: String, source: &'static str, edges: &mut Vec<EdgeView>| {
        if from == to || linked.contains(&(to.clone(), from.clone())) {
            return;
        }
        if linked.insert((from.clone(), to.clone())) {
            edges.push(EdgeView { from, to, source });
        }
    };

    for t in targets {
        if let Some(parent) = t.parent_id.filter(|p| by_id.contains_key(p)) {
            let source = if t.kind == agent::modules::SERVICE_KIND { "service" } else { "parent" };
            link(node_id(parent), node_id(t.id), source, &mut edges);
        }
    }
    for t in targets {
        if let Some(relay) = t.via_agent.filter(|r| by_id.contains_key(r)) {
            link(node_id(relay), node_id(t.id), "relay", &mut edges);
        }
    }

    let index = NameIndex::new(targets);

    // Nœuds Proxmox par hôte : ceux que `proxmox_node_up` annonce, plus ceux
    // que les invités nomment (repli quand l'état des nœuds est illisible).
    // Deux nœuds ou plus font un cluster ; un seul se confond avec la cible.
    let mut pve_nodes: BTreeMap<TargetId, BTreeMap<String, (String, Option<bool>)>> =
        BTreeMap::new();
    for node in facts.nodes.iter().filter(|n| by_id.contains_key(&n.host)) {
        pve_nodes
            .entry(node.host)
            .or_default()
            .insert(node.name.to_lowercase(), (node.name.clone(), Some(node.online)));
    }
    for guest in guests.iter().filter(|g| by_id.contains_key(&g.host)) {
        if let Some(name) = &guest.node {
            pve_nodes
                .entry(guest.host)
                .or_default()
                .entry(name.to_lowercase())
                .or_insert_with(|| (name.clone(), None));
        }
    }
    pve_nodes.retain(|_, list| list.len() >= 2);
    let node_view_id = |host: TargetId, name: &str| format!("n{host}:{}", name.to_lowercase());
    for (host_id, list) in &pve_nodes {
        let host = by_id[host_id];
        let site = site_of(host);
        for (name, online) in list.values() {
            let id = node_view_id(*host_id, name);
            nodes.push(NodeView {
                id: id.clone(),
                target_id: None,
                name: name.clone(),
                kind: "pve_node".to_string(),
                group_name: host.group_name.clone(),
                site_source: site.as_ref().map(|(_, source)| *source),
                site: site.as_ref().map(|(name, _)| name.clone()),
                host_id: Some(*host_id),
                subnet: None,
                vlan: None,
                vlan_name: None,
                running: *online,
                ips: Vec::new(),
            });
            link(node_id(*host_id), id, "hypervisor", &mut edges);
        }
    }
    // Le parent d'un invité : son nœud dans un cluster, sinon la cible.
    let parent_of = |guest: &Guest| -> String {
        guest
            .node
            .as_deref()
            .filter(|n| {
                pve_nodes.get(&guest.host).is_some_and(|list| list.contains_key(&n.to_lowercase()))
            })
            .map(|n| node_view_id(guest.host, n))
            .unwrap_or_else(|| node_id(guest.host))
    };

    // Adresses de chaque invité, d'après son hyperviseur. Une adresse que
    // plusieurs invités annoncent (adresse virtuelle partagée, réseau interne
    // recopié) ne désigne personne.
    let mut guest_ips: HashMap<(TargetId, String), Vec<IpAddr>> = HashMap::new();
    let mut claims: HashMap<IpAddr, HashSet<(TargetId, String)>> = HashMap::new();
    for address in facts.guest_ips.iter().filter(|a| by_id.contains_key(&a.host)) {
        let key = (address.host, address.name.to_lowercase());
        let list = guest_ips.entry(key.clone()).or_default();
        if !list.contains(&address.ip) {
            list.push(address.ip);
        }
        claims.entry(address.ip).or_default().insert(key);
    }
    // Cibles par adresse IP (un nom d'hôte n'est pas résolu : aucun accès réseau).
    let mut by_ip: HashMap<IpAddr, Vec<TargetId>> = HashMap::new();
    for t in targets.iter().filter(|t| t.kind != agent::modules::SERVICE_KIND) {
        let Some(ip) = host_key(&t.address).and_then(|k| k.parse::<IpAddr>().ok()) else {
            continue;
        };
        if !ip.is_loopback() {
            by_ip.entry(ip).or_default().push(t.id);
        }
    }
    // La cible qui représente un invité inconnu par son nom : la seule cible
    // « machine » à son adresse, ou à défaut le seul agent.
    let machine_at = |candidates: &[TargetId], taken: &HashSet<TargetId>| -> Option<TargetId> {
        let free: Vec<TargetId> = candidates
            .iter()
            .copied()
            .filter(|id| !taken.contains(id) && MACHINE_KINDS.contains(&by_id[id].kind.as_str()))
            .collect();
        let agents: Vec<TargetId> =
            free.iter().copied().filter(|id| by_id[id].kind == "agent").collect();
        match (free.as_slice(), agents.as_slice()) {
            ([one], _) | (_, [one]) => Some(*one),
            _ => None,
        }
    };
    // Cibles qui sont elles-mêmes un invité, reliées à leur hyperviseur.
    let mut is_guest: HashSet<TargetId> = HashSet::new();
    // Nœud de chaque invité et cibles vues à ses adresses.
    let mut hosted: Vec<(String, Vec<TargetId>)> = Vec::new();
    let mut vm_ips: HashMap<String, Vec<IpAddr>> = HashMap::new();

    let mut per_host: BTreeMap<TargetId, usize> = BTreeMap::new();
    for guest in guests {
        let Some(host) = by_id.get(&guest.host) else { continue };
        let ips = guest_ips.get(&(guest.host, guest.name.to_lowercase())).cloned();
        let ips = ips.unwrap_or_default();
        let mut at_ip: Vec<TargetId> = ips
            .iter()
            .filter(|ip| claims.get(*ip).is_some_and(|c| c.len() == 1))
            .flat_map(|ip| by_ip.get(ip).into_iter().flatten().copied())
            .filter(|id| *id != guest.host)
            .collect();
        at_ip.sort_unstable();
        at_ip.dedup();

        let vm = if let Some(target) = index.find(&guest.name, guest.host) {
            link(parent_of(guest), node_id(target), "hypervisor", &mut edges);
            is_guest.insert(target);
            node_id(target)
        } else if index.is_known(&guest.name, guest.host) {
            // Un nom qui désigne plusieurs cibles est une machine surveillée,
            // pas un invité inconnu : pas de lien (ambigu), pas de double non plus.
            continue;
        } else if let Some(target) = machine_at(&at_ip, &is_guest) {
            link(parent_of(guest), node_id(target), "hypervisor", &mut edges);
            is_guest.insert(target);
            node_id(target)
        } else {
            let count = per_host.entry(guest.host).or_default();
            if *count >= MAX_GUESTS_PER_HOST {
                continue;
            }
            *count += 1;
            let id = format!("g{}:{}", guest.host, guest.name);
            let site = site_of(host);
            nodes.push(NodeView {
                id: id.clone(),
                target_id: None,
                name: guest.name.clone(),
                kind: if guest.container { "container" } else { "vm" }.to_string(),
                group_name: host.group_name.clone(),
                site_source: site.as_ref().map(|(_, source)| *source),
                site: site.map(|(name, _)| name),
                host_id: Some(host.id),
                subnet: None,
                vlan: None,
                vlan_name: None,
                running: Some(guest.running),
                ips: Vec::new(),
            });
            link(parent_of(guest), id.clone(), "hypervisor", &mut edges);
            id
        };
        if !ips.is_empty() {
            vm_ips.entry(vm.clone()).or_insert(ips);
        }
        if !at_ip.is_empty() {
            hosted.push((vm, at_ip));
        }
    }

    // Ce qui tourne sur un invité : une cible dont l'adresse est l'une des
    // siennes. Un parent déclaré passe avant, et une cible qui est elle-même
    // un invité n'est pas « sur » un autre.
    for (vm, list) in hosted {
        for id in list {
            let declared = by_id[&id].parent_id.is_some_and(|p| by_id.contains_key(&p));
            if node_id(id) == vm || declared || is_guest.contains(&id) {
                continue;
            }
            link(vm.clone(), node_id(id), "hosted", &mut edges);
        }
    }

    // Une surveillance PBS et une surveillance Proxmox VE sur la même machine :
    // même hôte dans l'adresse, ou même nom (deux surveillances d'un même
    // serveur portent presque toujours le nom du serveur).
    let pbs_targets: Vec<&TargetRow> = targets.iter().filter(|t| t.kind == "pbs").collect();
    let pve_targets: Vec<&TargetRow> = targets.iter().filter(|t| t.kind == "proxmox").collect();
    for pbs in &pbs_targets {
        let same: Vec<&&TargetRow> = pve_targets
            .iter()
            .filter(|pve| {
                let (a, b) = (host_key(&pbs.address), host_key(&pve.address));
                let by_address = a.is_some() && a == b && !is_loopback(a.as_deref());
                let (a, b) = (host_key(&pbs.name), host_key(&pve.name));
                by_address || (a.is_some() && a == b)
            })
            .collect();
        if let [pve] = same.as_slice() {
            link(node_id(pve.id), node_id(pbs.id), "samehost", &mut edges);
        }
    }

    // Un stockage `pbs` de Proxmox VE désigne le serveur de sauvegarde par son
    // adresse : on le retrouve par adresse ou par nom, et le datastore départage
    // deux PBS qui répondraient au même nom.
    for storage in &facts.storages {
        if !by_id.contains_key(&storage.host) {
            continue;
        }
        let Some(key) = host_key(&storage.server) else { continue };
        let mut candidates: Vec<&&TargetRow> = pbs_targets
            .iter()
            .filter(|pbs| {
                host_key(&pbs.address).as_deref() == Some(key.as_str())
                    || host_key(&pbs.name).as_deref() == Some(key.as_str())
            })
            .collect();
        if candidates.len() > 1 {
            candidates.retain(|pbs| {
                facts.datastores.iter().any(|(t, d)| *t == pbs.id && *d == storage.datastore)
            });
        }
        if let [pbs] = candidates.as_slice() {
            link(node_id(storage.host), node_id(pbs.id), "backup", &mut edges);
        }
    }

    // Sous-réseau de chaque équipement : le réseau le plus précis qu'un pare-feu
    // du même site connaît, à défaut le /24 de son adresse.
    let firewalls: Vec<&TargetRow> =
        targets.iter().filter(|t| FIREWALL_KINDS.contains(&t.kind.as_str())).collect();
    let site_name =
        |id: TargetId| nodes.iter().find(|n| n.target_id == Some(id)).map(|n| n.site.clone());
    let mut zones: HashMap<TargetId, (Cidr, Option<&Network>)> = HashMap::new();
    for t in targets {
        let Some(ip) = ipv4_of(&t.address).filter(|ip| is_lan(*ip)) else { continue };
        let site = site_name(t.id);
        let known = facts
            .networks
            .iter()
            .filter(|n| n.cidr.contains(ip) && site_name(n.firewall) == site)
            .max_by_key(|n| n.cidr.prefix);
        match known {
            Some(network) => zones.insert(t.id, (network.cidr, Some(network))),
            None => Cidr::new(ip, 24).and_then(|c| zones.insert(t.id, (c, None))),
        };
    }

    // Un équipement que rien ne rattache encore passe « par » le pare-feu de
    // son site qui dessert son réseau ; le plus précis l'emporte, puis le plus
    // ancien. Deux pare-feux en haute disponibilité restent côte à côte.
    for t in targets {
        if FIREWALL_KINDS.contains(&t.kind.as_str())
            || t.parent_id.is_some_and(|p| by_id.contains_key(&p))
            || edges.iter().any(|e| e.to == node_id(t.id))
        {
            continue;
        }
        let Some((_, ip)) = zones.get(&t.id).zip(ipv4_of(&t.address)) else { continue };
        let site = site_name(t.id);
        let best = firewalls
            .iter()
            .filter(|f| site_name(f.id) == site)
            .filter_map(|f| {
                let prefix = facts
                    .networks
                    .iter()
                    .filter(|n| n.firewall == f.id && n.cidr.contains(ip))
                    .map(|n| n.cidr.prefix)
                    .max();
                // Sans interface connue, le /24 de l'adresse du pare-feu fait foi.
                let prefix = prefix.or_else(|| {
                    let own = ipv4_of(&f.address)?;
                    let known = facts.networks.iter().any(|n| n.firewall == f.id);
                    (!known && Cidr::new(own, 24)?.contains(ip)).then_some(0)
                })?;
                Some((prefix, std::cmp::Reverse(f.id), f.id))
            })
            .max();
        if let Some((_, _, firewall)) = best {
            link(node_id(firewall), node_id(t.id), "firewall", &mut edges);
        }
    }
    for node in nodes.iter_mut() {
        let Some((cidr, network)) = node.target_id.and_then(|id| zones.get(&id)) else { continue };
        node.subnet = Some(cidr.to_string());
        node.vlan = network.and_then(|n| n.vlan);
        node.vlan_name = network.map(|n| n.name.clone()).filter(|n| !n.is_empty());
    }

    for neighbor in neighbors {
        if !by_id.contains_key(&neighbor.switch) {
            continue;
        }
        if let Some(target) = index.find(&neighbor.remote, neighbor.switch) {
            link(node_id(neighbor.switch), node_id(target), "lldp", &mut edges);
        }
    }

    for node in nodes.iter_mut() {
        if let Some(ips) = vm_ips.get(&node.id) {
            node.ips = ips.iter().map(ToString::to_string).collect();
        }
    }

    (nodes, edges)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: TargetId, name: &str, address: &str, kind: &str) -> TargetRow {
        TargetRow {
            id,
            name: name.into(),
            address: address.into(),
            kind: kind.into(),
            parent_id: None,
            via_agent: None,
            group_name: String::new(),
            site_tag: None,
            hostname: None,
        }
    }

    fn serie(labels: &[(&str, &str)], value: &str) -> InstantSeries {
        InstantSeries {
            metric: labels.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            value: (0.0, value.to_string()),
        }
    }

    fn sources(edges: &[EdgeView]) -> Vec<(&str, &str, &str)> {
        edges.iter().map(|e| (e.from.as_str(), e.to.as_str(), e.source)).collect()
    }

    #[test]
    fn la_cle_dhote_ignore_schema_port_et_domaine() {
        assert_eq!(host_key("https://pve1.lan:8006/api").as_deref(), Some("pve1"));
        assert_eq!(host_key("Web01").as_deref(), Some("web01"));
        assert_eq!(host_key("192.168.1.10:161").as_deref(), Some("192.168.1.10"));
        assert_eq!(host_key("[fe80::1]:443").as_deref(), Some("fe80::1"));
        assert_eq!(host_key("user:pw@nas.home").as_deref(), Some("nas"));
        assert_eq!(host_key("  "), None);
    }

    #[test]
    fn parent_et_relais_donnent_un_lien_chacun_sans_doublon() {
        let mut a = row(1, "routeur", "10.0.0.1", "snmp");
        a.group_name = "Bureau".into();
        let relay = row(2, "relais", "relais", "agent");
        let mut c = row(3, "nas", "10.0.0.3", "synology");
        c.parent_id = Some(1);
        c.via_agent = Some(2);
        let mut d = row(4, "imprimante", "10.0.0.4", "snmp");
        d.parent_id = Some(2);
        d.via_agent = Some(2);
        let sites = HashMap::from([(2, "Annexe".to_string())]);
        let (nodes, edges) = build(&[a, relay, c, d], &sites, &[], &[]);
        assert_eq!(
            sources(&edges),
            vec![("t1", "t3", "parent"), ("t2", "t4", "parent"), ("t2", "t3", "relay")]
        );
        assert_eq!(nodes[2].site.as_deref(), Some("Annexe"), "le site du relais suit la cible");
        assert_eq!(nodes[2].site_source, Some("agent"));
        assert_eq!(nodes[0].site, None, "sans indice, le site principal");
        assert_eq!(nodes[0].group_name, "Bureau");
    }

    #[test]
    fn un_service_dagent_suit_sa_machine_sans_la_rendre_ambigue() {
        let pve = row(1, "pve1", "pve1", "proxmox");
        let host = row(2, "web01", "web01", "agent");
        let mut docker = row(3, "web01 · Docker", "web01#docker", "agent_service");
        docker.parent_id = Some(2);
        let mut libvirt = row(4, "web01 · libvirt", "web01#libvirt", "agent_service");
        libvirt.parent_id = Some(2);
        let guests = vec![
            Guest {
                host: 1,
                node: None,
                name: "web01.lan".into(),
                container: false,
                running: true,
            },
            Guest { host: 4, node: None, name: "vm-test".into(), container: false, running: true },
        ];
        let sites = HashMap::from([(2, "Annexe".to_string())]);
        let (nodes, edges) = build(&[pve, host, docker, libvirt], &sites, &guests, &[]);
        assert_eq!(
            sources(&edges),
            vec![
                ("t2", "t3", "service"),
                ("t2", "t4", "service"),
                ("t1", "t2", "hypervisor"),
                ("t4", "g4:vm-test", "hypervisor"),
            ]
        );
        let site = |id: &str| nodes.iter().find(|n| n.id == id).and_then(|n| n.site.clone());
        assert_eq!(site("t3").as_deref(), Some("Annexe"), "le service prend le site de sa machine");
        assert_eq!(site("g4:vm-test").as_deref(), Some("Annexe"));
    }

    #[test]
    fn le_site_se_deduit_du_relais_ou_de_letiquette() {
        let relay = row(1, "relais-cave", "relais-cave", "agent");
        let mut nas = row(2, "nas", "10.1.0.2", "synology");
        nas.via_agent = Some(1);
        let mut tagged = row(3, "camera", "10.2.0.3", "http");
        tagged.site_tag = Some("Grenier".into());
        let mut unifi = row(4, "unifi", "https://unifi", "unifi");
        unifi.site_tag = Some("default".into());
        let local = row(5, "routeur", "10.0.0.1", "snmp");
        let (nodes, _) = build(&[relay, nas, tagged, unifi, local], &HashMap::new(), &[], &[]);
        let site = |id: &str| {
            let n = nodes.iter().find(|n| n.id == id).expect("nœud");
            (n.site.as_deref(), n.site_source)
        };
        assert_eq!(site("t1"), (Some("relais-cave"), Some("relay")), "un relais est un site");
        assert_eq!(site("t2"), (Some("relais-cave"), Some("relay")));
        assert_eq!(site("t3"), (Some("Grenier"), Some("tag")));
        assert_eq!(site("t4"), (None, None), "le site UniFi n'est pas un lieu");
        assert_eq!(site("t5"), (None, None));
    }

    #[test]
    fn letiquette_site_est_lue_dans_le_json() {
        assert_eq!(site_tag(r#"{"site":" Cave ","port":"81"}"#).as_deref(), Some("Cave"));
        assert_eq!(site_tag(r#"{"site":""}"#), None);
        assert_eq!(site_tag("{}"), None);
        assert_eq!(site_tag("pas du json"), None);
    }

    #[test]
    fn un_parent_disparu_ne_cree_aucun_lien() {
        let mut a = row(1, "a", "a", "snmp");
        a.parent_id = Some(99);
        let (_, edges) = build(&[a], &HashMap::new(), &[], &[]);
        assert!(edges.is_empty());
    }

    #[test]
    fn un_invite_surveille_est_relie_et_les_autres_deviennent_des_noeuds() {
        let pve = row(1, "pve1", "https://pve1.lan:8006", "proxmox");
        let web = row(2, "web01", "web01.lan", "agent");
        let guests = vec![
            Guest { host: 1, node: None, name: "WEB01".into(), container: false, running: true },
            Guest {
                host: 1,
                node: None,
                name: "nextcloud".into(),
                container: true,
                running: false,
            },
        ];
        let (nodes, edges) = build(&[pve, web], &HashMap::new(), &guests, &[]);
        assert_eq!(
            sources(&edges),
            vec![("t1", "t2", "hypervisor"), ("t1", "g1:nextcloud", "hypervisor")]
        );
        let guest = nodes.iter().find(|n| n.id == "g1:nextcloud").expect("nœud invité");
        assert_eq!(guest.kind, "container");
        assert_eq!(guest.running, Some(false));
        assert_eq!(guest.host_id, Some(1));
        assert_eq!(guest.target_id, None);
    }

    #[test]
    fn un_nom_ambigu_ne_relie_rien() {
        let pve = row(1, "pve1", "pve1", "proxmox");
        let a = row(2, "web01", "10.0.0.2", "agent");
        let b = row(3, "web01", "https://web01.lan", "http");
        let guests = vec![Guest {
            host: 1,
            node: None,
            name: "web01".into(),
            container: false,
            running: true,
        }];
        let (nodes, edges) = build(&[pve, a, b], &HashMap::new(), &guests, &[]);
        assert!(edges.is_empty(), "ambigu : aucun lien");
        assert_eq!(nodes.len(), 3, "et aucun double de la machine surveillée");
    }

    #[test]
    fn un_emplacement_affecte_place_la_cible_dans_le_cadre_du_site() {
        use super::super::locations::LocationRow;
        let loc = |id, name: &str, site: &str, main_site| LocationRow {
            id,
            name: name.into(),
            address: String::new(),
            latitude: 0.0,
            longitude: 0.0,
            site: site.into(),
            main_site,
        };
        let locations = [loc(1, "Siège", "", true), loc(2, "Crissier", "crissier", false)];
        let relay = row(1, "dm01", "dm01", "agent");
        let nas = row(2, "nas60", "10.0.0.2", "synology");
        let fw = row(3, "fw03", "10.0.0.3", "opnsense");
        let mut svc = row(4, "nas60 · Docker", "nas60#docker", "agent_service");
        svc.parent_id = Some(2);
        let sites = HashMap::from([(1, "Crissier".to_string())]);
        let assigned = HashMap::from([(2, 2), (3, 1)]);
        let placed = placed_from(&assigned, &locations, &["Crissier".to_string()]);
        let pve = row(5, "pve1", "pve1", "proxmox");
        let guests =
            vec![Guest { host: 2, node: None, name: "v9".into(), container: false, running: true }];
        let (nodes, _) = build_placed(&[relay, nas, fw, svc, pve], &sites, &placed, &guests, &[]);
        let site = |id: &str| {
            let n = nodes.iter().find(|n| n.id == id).expect("nœud");
            (n.site.clone(), n.site_source)
        };
        assert_eq!(site("t1"), (Some("Crissier".into()), Some("agent")));
        assert_eq!(
            site("t2"),
            (Some("Crissier".into()), Some("location")),
            "orthographe du site connu"
        );
        assert_eq!(site("t3"), (None, None), "l'emplacement principal ramène au site principal");
        assert_eq!(site("t4"), (Some("Crissier".into()), Some("location")), "le service suit");
        assert_eq!(site("g2:v9").0.as_deref(), Some("Crissier"), "l'invité aussi");
        assert_eq!(site("t5"), (None, None));
    }

    fn node_guest(host: TargetId, node: &str, name: &str, running: bool) -> Guest {
        Guest { host, node: Some(node.into()), name: name.into(), container: false, running }
    }

    fn ids(nodes: &[NodeView]) -> Vec<&str> {
        nodes.iter().map(|n| n.id.as_str()).collect()
    }

    #[test]
    fn un_cluster_se_dessine_en_noeuds_puis_en_invites_allumes_ou_eteints() {
        let cluster = row(1, "cluster", "https://node1.lan:8006", "proxmox");
        let nas = row(2, "nas60", "10.0.0.2", "synology");
        let guests = vec![
            node_guest(1, "node1", "web", true),
            node_guest(1, "node2", "db", false),
            node_guest(1, "node2", "NAS60", true),
        ];
        let facts = ProxmoxFacts {
            nodes: vec![
                PveNode { host: 1, name: "node1".into(), online: true },
                PveNode { host: 1, name: "node2".into(), online: true },
                PveNode { host: 1, name: "node3".into(), online: false },
            ],
            ..Default::default()
        };
        let (nodes, edges) =
            build_full(&[cluster, nas], &HashMap::new(), &Placed::new(), &guests, &[], &facts);
        assert_eq!(
            ids(&nodes),
            ["t1", "t2", "n1:node1", "n1:node2", "n1:node3", "g1:web", "g1:db"]
        );
        let node3 = nodes.iter().find(|n| n.id == "n1:node3").expect("nœud éteint");
        assert_eq!((node3.kind.as_str(), node3.running), ("pve_node", Some(false)));
        assert_eq!(node3.host_id, Some(1));
        let db = nodes.iter().find(|n| n.id == "g1:db").expect("invité éteint");
        assert_eq!(db.running, Some(false), "un invité éteint reste dessiné");
        assert_eq!(
            sources(&edges),
            vec![
                ("t1", "n1:node1", "hypervisor"),
                ("t1", "n1:node2", "hypervisor"),
                ("t1", "n1:node3", "hypervisor"),
                ("n1:node1", "g1:web", "hypervisor"),
                ("n1:node2", "g1:db", "hypervisor"),
                ("n1:node2", "t2", "hypervisor"),
            ],
            "la surveillance qui correspond à un invité est reliée au nœud, sans double"
        );
    }

    #[test]
    fn sans_etat_des_noeuds_ils_se_deduisent_des_invites() {
        let cluster = row(1, "cluster", "node1", "proxmox");
        let guests = vec![node_guest(1, "node1", "a", true), node_guest(1, "node2", "b", true)];
        let (nodes, _) = build(&[cluster], &HashMap::new(), &guests, &[]);
        assert_eq!(ids(&nodes), ["t1", "n1:node1", "n1:node2", "g1:a", "g1:b"]);
        assert_eq!(nodes[1].running, None, "état inconnu plutôt qu'inventé");
    }

    #[test]
    fn un_proxmox_a_noeud_unique_garde_ses_invites_sous_la_cible() {
        let pve = row(1, "pve002", "pve002", "proxmox");
        let facts = ProxmoxFacts {
            nodes: vec![PveNode { host: 1, name: "pve002".into(), online: true }],
            ..Default::default()
        };
        let guests = vec![node_guest(1, "pve002", "adr03", true)];
        let (nodes, edges) =
            build_full(&[pve], &HashMap::new(), &Placed::new(), &guests, &[], &facts);
        assert_eq!(ids(&nodes), ["t1", "g1:adr03"], "pas de niveau nœud inutile");
        assert_eq!(sources(&edges), vec![("t1", "g1:adr03", "hypervisor")]);
    }

    #[test]
    fn un_stockage_pbs_relie_le_proxmox_au_pbs_qui_le_sauvegarde() {
        let cluster = row(1, "cluster", "node1", "proxmox");
        let pbs = row(2, "pbs01", "https://backup.lan:8007", "pbs");
        let autre = row(3, "pbs02", "10.0.0.9", "pbs");
        let facts = ProxmoxFacts {
            storages: vec![PbsStorage {
                host: 1,
                server: "BACKUP.lan".into(),
                datastore: "t".into(),
            }],
            ..Default::default()
        };
        let (_, edges) =
            build_full(&[cluster, pbs, autre], &HashMap::new(), &Placed::new(), &[], &[], &facts);
        assert_eq!(sources(&edges), vec![("t1", "t2", "backup")]);
    }

    #[test]
    fn deux_pbs_de_meme_nom_sont_departages_par_le_datastore() {
        let pve = row(1, "pve", "pve", "proxmox");
        let a = row(2, "backup", "https://backup:8007", "pbs");
        let b = row(3, "backup", "https://backup.autre:8007", "pbs");
        let storage = |datastore: &str| PbsStorage {
            host: 1,
            server: "backup".into(),
            datastore: datastore.into(),
        };
        let targets = [pve, a, b];
        let (_, none) = build_full(
            &targets,
            &HashMap::new(),
            &Placed::new(),
            &[],
            &[],
            &ProxmoxFacts { storages: vec![storage("x")], ..Default::default() },
        );
        assert!(none.is_empty(), "ambigu : aucun lien plutôt qu'un lien faux");
        let facts = ProxmoxFacts {
            storages: vec![storage("tank")],
            datastores: vec![(3, "tank".into())],
            ..Default::default()
        };
        let (_, edges) = build_full(&targets, &HashMap::new(), &Placed::new(), &[], &[], &facts);
        assert_eq!(sources(&edges), vec![("t1", "t3", "backup")]);
    }

    #[test]
    fn un_pbs_et_un_pve_de_meme_hote_ou_de_meme_nom_sont_relies_une_fois() {
        let pve = row(1, "pve002", "https://10.2.0.5:8006", "proxmox");
        let pbs = row(2, "pve002", "https://10.2.0.5:8007", "pbs");
        let ailleurs = row(3, "pve9", "10.9.0.9", "proxmox");
        let facts = ProxmoxFacts {
            storages: vec![PbsStorage {
                host: 1,
                server: "10.2.0.5".into(),
                datastore: "d".into(),
            }],
            ..Default::default()
        };
        let (nodes, edges) =
            build_full(&[pve, pbs, ailleurs], &HashMap::new(), &Placed::new(), &[], &[], &facts);
        assert_eq!(nodes.len(), 3, "chaque cible n'est dessinée qu'une fois");
        assert_eq!(
            sources(&edges),
            vec![("t1", "t2", "samehost")],
            "le lien de sauvegarde se fond dedans"
        );

        let pve = row(1, "hote", "10.0.0.1", "proxmox");
        let pbs = row(2, "hote", "10.0.0.2", "pbs");
        let (_, edges) = build(&[pve, pbs], &HashMap::new(), &[], &[]);
        assert_eq!(
            sources(&edges),
            vec![("t1", "t2", "samehost")],
            "même nom, adresses distinctes"
        );
    }

    #[test]
    fn les_faits_proxmox_sont_lus_dans_les_series() {
        let series = vec![
            serie(
                &[("__name__", "dumbmonit_proxmox_node_up"), ("target", "1"), ("node", "n1")],
                "1",
            ),
            serie(
                &[("__name__", "dumbmonit_proxmox_node_up"), ("target", "1"), ("node", "n2")],
                "0",
            ),
            serie(
                &[("__name__", "dumbmonit_proxmox_node_up"), ("target", "9"), ("node", "x")],
                "1",
            ),
            serie(
                &[
                    ("__name__", "dumbmonit_proxmox_storage_pbs_info"),
                    ("target", "1"),
                    ("server", "pbs.lan"),
                    ("datastore", "tank"),
                ],
                "1",
            ),
            serie(
                &[
                    ("__name__", "dumbmonit_pbs_datastore_available"),
                    ("target", "2"),
                    ("datastore", "tank"),
                ],
                "1",
            ),
        ];
        let facts = parse_facts(&series, &[1, 2]);
        assert_eq!(
            facts.nodes,
            vec![
                PveNode { host: 1, name: "n1".into(), online: true },
                PveNode { host: 1, name: "n2".into(), online: false },
            ]
        );
        assert_eq!(facts.storages.len(), 1);
        assert_eq!(facts.datastores, vec![(2, "tank".to_string())]);
    }

    fn network(firewall: TargetId, cidr: &str, name: &str, vlan: Option<u16>) -> Network {
        Network { firewall, cidr: Cidr::parse(cidr).expect("cidr"), name: name.into(), vlan }
    }

    #[test]
    fn un_cidr_se_normalise_et_dit_ce_quil_contient() {
        let cidr = Cidr::parse("192.168.10.77/24").expect("cidr");
        assert_eq!(cidr.to_string(), "192.168.10.0/24");
        assert!(cidr.contains("192.168.10.200".parse().unwrap()));
        assert!(!cidr.contains("192.168.11.1".parse().unwrap()));
        assert_eq!(Cidr::parse("192.168.10.1"), None, "sans préfixe : pas un réseau");
        assert!(is_lan("100.64.0.1".parse().unwrap()) && !is_lan("8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn le_sous_reseau_vient_des_reseaux_du_pare_feu_sinon_du_slash_24() {
        let fw = row(1, "fw01", "192.168.1.1", "opnsense");
        let srv = row(2, "srv", "https://192.168.10.5:443", "http");
        let autre = row(3, "cam", "192.168.20.9", "http");
        let public = row(4, "site", "8.8.8.8", "http");
        let nom = row(5, "nom", "nas.lan", "http");
        let facts = ProxmoxFacts {
            networks: vec![
                network(1, "192.168.1.1/24", "LAN", None),
                network(1, "192.168.10.1/24", "Servers", Some(10)),
            ],
            ..Default::default()
        };
        let (nodes, _) = build_full(
            &[fw, srv, autre, public, nom],
            &HashMap::new(),
            &Placed::new(),
            &[],
            &[],
            &facts,
        );
        let net = |id: &str| {
            let n = nodes.iter().find(|n| n.id == id).expect("nœud");
            (n.subnet.clone(), n.vlan, n.vlan_name.clone())
        };
        assert_eq!(net("t1"), (Some("192.168.1.0/24".into()), None, Some("LAN".into())));
        assert_eq!(net("t2"), (Some("192.168.10.0/24".into()), Some(10), Some("Servers".into())));
        assert_eq!(net("t3"), (Some("192.168.20.0/24".into()), None, None), "repli : /24");
        assert_eq!(net("t4"), (None, None, None), "une adresse publique ne forme pas de zone");
        assert_eq!(net("t5"), (None, None, None), "un nom d'hôte n'est pas résolu");
    }

    #[test]
    fn un_equipement_est_rattache_au_pare_feu_de_son_reseau() {
        let fw1 = row(1, "fw01", "192.168.1.1", "opnsense");
        let fw2 = row(2, "fw02", "192.168.1.2", "opnsense");
        let nas = row(3, "nas", "192.168.1.30", "synology");
        let loin = row(4, "loin", "172.16.0.4", "http");
        let facts = ProxmoxFacts {
            networks: vec![network(1, "192.168.1.1/24", "LAN", None)],
            ..Default::default()
        };
        let (_, edges) =
            build_full(&[fw1, fw2, nas, loin], &HashMap::new(), &Placed::new(), &[], &[], &facts);
        assert_eq!(
            sources(&edges),
            vec![("t1", "t3", "firewall")],
            "le pare-feu qui connaît le réseau l'emporte ; hors réseau, aucun lien"
        );
    }

    #[test]
    fn sans_reseau_connu_le_slash_24_du_pare_feu_sert_de_repli() {
        let fw = row(1, "fw", "10.0.0.1", "pfsense");
        let a = row(2, "a", "10.0.0.7", "http");
        let b = row(3, "b", "10.0.1.7", "http");
        let (_, edges) = build(&[fw, a, b], &HashMap::new(), &[], &[]);
        assert_eq!(sources(&edges), vec![("t1", "t2", "firewall")]);
    }

    #[test]
    fn le_parent_declare_et_les_autres_liens_passent_avant_le_pare_feu() {
        let fw = row(1, "fw", "10.0.0.1", "opnsense");
        let sw = row(2, "sw", "10.0.0.2", "snmp");
        let mut host = row(3, "host", "10.0.0.3", "agent");
        host.parent_id = Some(2);
        let pve = row(4, "pve", "10.0.0.4", "proxmox");
        let vm = row(5, "vm1", "10.0.0.5", "http");
        let guests = vec![node_guest(4, "pve", "vm1", true)];
        let (_, edges) = build(&[fw, sw, host, pve, vm], &HashMap::new(), &guests, &[]);
        assert_eq!(
            sources(&edges),
            vec![
                ("t2", "t3", "parent"),
                ("t4", "t5", "hypervisor"),
                ("t1", "t2", "firewall"),
                ("t1", "t4", "firewall"),
            ]
        );
    }

    #[test]
    fn les_reseaux_des_pare_feux_sont_lus_dans_les_series() {
        let series = vec![
            serie(
                &[
                    ("__name__", "dumbmonit_fortigate_interface_address_info"),
                    ("target", "1"),
                    ("interface", "vlan10"),
                    ("description", "Servers"),
                    ("address", "192.168.10.1/24"),
                    ("vlan", "10"),
                ],
                "1",
            ),
            serie(
                &[
                    ("__name__", "dumbmonit_opnsense_interface_address_info"),
                    ("target", "1"),
                    ("address", "203.0.113.5/24"),
                ],
                "1",
            ),
            serie(
                &[
                    ("__name__", "dumbmonit_opnsense_interface_address_info"),
                    ("target", "1"),
                    ("address", "10.0.0.1"),
                ],
                "1",
            ),
        ];
        let facts = parse_facts(&series, &[1]);
        assert_eq!(facts.networks, vec![network(1, "192.168.10.1/24", "Servers", Some(10))]);
    }

    #[test]
    fn lldp_relie_seulement_des_cibles_connues_et_une_fois_par_paire() {
        let sw1 = row(1, "sw1", "10.0.0.1", "snmp");
        let sw2 = row(2, "sw2", "10.0.0.2", "snmp");
        let neighbors = vec![
            Neighbor { switch: 1, remote: "sw2.lan".into() },
            Neighbor { switch: 2, remote: "SW1".into() },
            Neighbor { switch: 1, remote: "phone-42".into() },
        ];
        let (nodes, edges) = build(&[sw1, sw2], &HashMap::new(), &[], &neighbors);
        assert_eq!(sources(&edges), vec![("t1", "t2", "lldp")]);
        assert_eq!(nodes.len(), 2, "un voisin inconnu n'est pas inventé");
    }

    #[test]
    fn un_lien_declare_passe_avant_un_lien_deduit() {
        let sw = row(1, "sw1", "10.0.0.1", "snmp");
        let mut host = row(2, "host", "10.0.0.2", "agent");
        host.parent_id = Some(1);
        let neighbors = vec![Neighbor { switch: 1, remote: "host".into() }];
        let (_, edges) = build(&[sw, host], &HashMap::new(), &[], &neighbors);
        assert_eq!(sources(&edges), vec![("t1", "t2", "parent")]);
    }

    #[test]
    fn les_series_dinvites_sont_lues_selon_leur_collecteur() {
        let known = [1, 2];
        let series = vec![
            serie(
                &[
                    ("__name__", "dumbmonit_proxmox_guest_running"),
                    ("target", "1"),
                    ("name", "ct1"),
                    ("type", "lxc"),
                ],
                "1",
            ),
            serie(
                &[("__name__", "dumbmonit_vsphere_vm_power_state"), ("target", "2"), ("vm", "a")],
                "0",
            ),
            serie(
                &[("__name__", "dumbmonit_vsphere_vm_power_state"), ("target", "2"), ("vm", "b")],
                "1",
            ),
            serie(&[("__name__", "dumbmonit_xcpng_vm_running"), ("target", "9"), ("vm", "x")], "1"),
            serie(&[("__name__", "dumbmonit_xcpng_vm_running"), ("target", "1")], "1"),
        ];
        let guests = parse_guests(&series, &known);
        assert_eq!(
            guests,
            vec![
                Guest { host: 1, node: None, name: "ct1".into(), container: true, running: true },
                Guest { host: 2, node: None, name: "a".into(), container: false, running: true },
                Guest { host: 2, node: None, name: "b".into(), container: false, running: false },
            ]
        );
    }

    #[test]
    fn les_voisins_lldp_sont_dedoublonnes() {
        let series = vec![
            serie(&[("target", "1"), ("remote_sysname", "sw2"), ("index", "0.1.1")], "4"),
            serie(&[("target", "1"), ("remote_sysname", "SW2"), ("index", "0.2.1")], "4"),
            serie(&[("target", "1"), ("remote_sysname", "")], "4"),
        ];
        assert_eq!(
            parse_neighbors(&series, &[1]),
            vec![Neighbor { switch: 1, remote: "sw2".into() }]
        );
    }

    fn guest_ip(host: TargetId, name: &str, ip: &str) -> GuestIp {
        GuestIp { host, name: name.into(), ip: ip.parse().unwrap() }
    }

    fn guest(host: TargetId, name: &str) -> Guest {
        Guest { host, node: None, name: name.into(), container: false, running: true }
    }

    #[test]
    fn une_adresse_dinvite_ignore_boucle_locale_et_lien_local() {
        assert_eq!(guest_address("192.168.10.50"), Some("192.168.10.50".parse().unwrap()));
        assert_eq!(guest_address(" 10.0.0.5/24 "), Some("10.0.0.5".parse().unwrap()));
        assert_eq!(guest_address("2001:db8::5"), Some("2001:db8::5".parse().unwrap()));
        assert_eq!(guest_address("fd00::5%eth0"), Some("fd00::5".parse().unwrap()));
        let ignored = ["127.0.0.1", "::1", "169.254.3.4", "fe80::1", "0.0.0.0", "224.0.0.1", "x"];
        for ignored in ignored {
            assert_eq!(guest_address(ignored), None, "{ignored}");
        }
    }

    #[test]
    fn les_adresses_des_invites_sont_lues_dans_les_mesures() {
        let line = |metric: &str, who: (&str, &str), address: &str| {
            serie(&[("__name__", metric), ("target", "1"), who, ("ip", address)], "1")
        };
        let pve = "dumbmonit_proxmox_guest_ip_info";
        let series = vec![
            line(pve, ("name", "web"), "10.0.0.5"),
            line(pve, ("name", "WEB"), "10.0.0.5"),
            line(pve, ("name", "web"), "fe80::1"),
            line("dumbmonit_vsphere_vm_ip_info", ("vm", "db"), "10.0.0.6"),
            line(pve, ("name", "web"), "pas une adresse"),
        ];
        assert_eq!(
            parse_facts(&series, &[1]).guest_ips,
            vec![guest_ip(1, "web", "10.0.0.5"), guest_ip(1, "db", "10.0.0.6")]
        );
    }

    #[test]
    fn un_service_a_ladresse_dun_invite_tourne_dessus() {
        let pve = row(1, "pve1", "10.0.0.1", "proxmox");
        let site = row(2, "Blog", "https://10.0.0.5/", "http");
        let base = row(3, "Postgres", "[2001:db8::5]:5432", "tcp");
        let ailleurs = row(4, "Ailleurs", "https://10.0.0.99", "http");
        let facts = ProxmoxFacts {
            guest_ips: vec![guest_ip(1, "web", "10.0.0.5"), guest_ip(1, "web", "2001:db8::5")],
            ..Default::default()
        };
        let (nodes, edges) = build_full(
            &[pve, site, base, ailleurs],
            &HashMap::new(),
            &Placed::new(),
            &[guest(1, "web")],
            &[],
            &facts,
        );
        assert_eq!(
            sources(&edges),
            vec![
                ("t1", "g1:web", "hypervisor"),
                ("g1:web", "t2", "hosted"),
                ("g1:web", "t3", "hosted"),
            ]
        );
        let vm = nodes.iter().find(|n| n.id == "g1:web").expect("nœud invité");
        assert_eq!(vm.ips, ["10.0.0.5", "2001:db8::5"]);
        assert!(nodes.iter().filter(|n| n.id != "g1:web").all(|n| n.ips.is_empty()));
    }

    #[test]
    fn lagent_a_ladresse_dun_invite_le_represente_sans_double() {
        let pve = row(1, "pve1", "10.0.0.1", "proxmox");
        let agent = row(2, "serveur-web", "10.0.0.5", "agent");
        let site = row(3, "Blog", "http://10.0.0.5:8080", "http");
        let facts = ProxmoxFacts {
            guest_ips: vec![guest_ip(1, "vm105", "10.0.0.5")],
            ..Default::default()
        };
        let (nodes, edges) = build_full(
            &[pve, agent, site],
            &HashMap::new(),
            &Placed::new(),
            &[guest(1, "vm105")],
            &[],
            &facts,
        );
        assert_eq!(ids(&nodes), ["t1", "t2", "t3"], "pas de nœud invité en double");
        assert_eq!(sources(&edges), vec![("t1", "t2", "hypervisor"), ("t2", "t3", "hosted")]);
        assert_eq!(nodes[1].ips, ["10.0.0.5"], "l'agent porte les adresses de la VM");
    }

    #[test]
    fn une_adresse_partagee_ou_un_parent_declare_ne_creent_pas_de_lien() {
        let pve = row(1, "pve1", "10.0.0.1", "proxmox");
        let vip = row(2, "VIP", "https://10.0.0.50", "http");
        let switch = row(3, "switch", "10.0.0.2", "snmp");
        let mut declare = row(4, "Boutique", "https://10.0.0.6", "http");
        declare.parent_id = Some(3);
        let facts = ProxmoxFacts {
            guest_ips: vec![
                guest_ip(1, "lb1", "10.0.0.50"),
                guest_ip(1, "lb2", "10.0.0.50"),
                guest_ip(1, "app", "10.0.0.6"),
            ],
            ..Default::default()
        };
        let (_, edges) = build_full(
            &[pve, vip, switch, declare],
            &HashMap::new(),
            &Placed::new(),
            &[guest(1, "lb1"), guest(1, "lb2"), guest(1, "app")],
            &[],
            &facts,
        );
        assert_eq!(
            sources(&edges),
            vec![
                ("t3", "t4", "parent"),
                ("t1", "g1:lb1", "hypervisor"),
                ("t1", "g1:lb2", "hypervisor"),
                ("t1", "g1:app", "hypervisor"),
            ],
            "adresse revendiquée deux fois : personne ; parent déclaré : gardé seul"
        );
    }

    #[test]
    fn le_nom_dhote_de_lagent_relie_linvite() {
        let pve = row(1, "pve1", "10.0.0.1", "proxmox");
        let mut agent = row(2, "Serveur de fichiers", "id-abc", "agent");
        agent.hostname = Some("files01.lan".into());
        let (nodes, edges) = build(&[pve, agent], &HashMap::new(), &[guest(1, "files01")], &[]);
        assert_eq!(ids(&nodes), ["t1", "t2"]);
        assert_eq!(sources(&edges), vec![("t1", "t2", "hypervisor")]);
    }
}
