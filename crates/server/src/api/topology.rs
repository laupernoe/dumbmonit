//! Carte de topologie : `GET /api/topology`.
//!
//! Rien n'y est saisi à la main : chaque lien vient de ce que le serveur sait
//! déjà, et porte sa source pour que l'interface puisse le dire.
//!
//! - `parent` : la dépendance déclarée (`parent_id`), celle qui sert à la
//!   suppression des alertes ;
//! - `relay` : l'agent relais qui interroge l'équipement (`via_agent`) ;
//! - `hypervisor` : un hyperviseur et ses invités, lus dans les dernières
//!   mesures (Proxmox, XCP-ng, Nutanix, vSphere, libvirt de l'agent) ;
//! - `lldp` : un voisin LLDP qu'un commutateur SNMP a vu sur un port.
//!
//! Un service trouvé par un agent (`agent_service`) est relié à sa machine par
//! `parent` et prend son site ; il ne sert jamais de cible à un invité ou à un
//! voisin, puisqu'il partage l'adresse de sa machine.
//!
//! Un invité ou un voisin n'est relié à un équipement surveillé que si son nom
//! désigne une seule cible : un nom ambigu ne crée aucun lien plutôt qu'un lien
//! faux. Un invité qui ne correspond à aucune cible devient un nœud à part,
//! rattaché à son hyperviseur ; un voisin LLDP inconnu, lui, n'apparaît pas —
//! la carte montre l'infrastructure surveillée, pas tout ce qui répond sur un
//! port.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::IpAddr;

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
    /// `t<id>` pour une cible, `g<hôte>:<nom>` pour un invité non surveillé.
    pub id: String,
    pub target_id: Option<TargetId>,
    pub name: String,
    /// Type de la cible ; `vm` ou `container` pour un invité.
    pub kind: String,
    /// Dossier de la cible ; pour un invité, celui de son hyperviseur.
    pub group_name: String,
    /// Site déclaré par l'agent relais (le sien, celui qui relaie la cible, ou
    /// celui de la machine d'un service d'agent).
    pub site: Option<String>,
    /// Invité seulement : la cible hyperviseur qui l'héberge.
    pub host_id: Option<TargetId>,
    /// Invité seulement : en marche à la dernière mesure.
    pub running: Option<bool>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EdgeView {
    /// Côté amont : parent, relais, hyperviseur, commutateur.
    pub from: String,
    pub to: String,
    /// `parent`, `relay`, `hypervisor` ou `lldp`.
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
}

/// Un invité lu dans les mesures d'un hyperviseur.
#[derive(Debug, Clone, PartialEq)]
pub struct Guest {
    pub host: TargetId,
    pub name: String,
    pub container: bool,
    pub running: bool,
}

/// Un voisin LLDP vu par un commutateur.
#[derive(Debug, Clone, PartialEq)]
pub struct Neighbor {
    pub switch: TargetId,
    pub remote: String,
}

/// `GET /api/topology` : lecture seule, ouverte à tous les rôles comme la
/// liste des équipements. Les adresses n'y figurent pas.
pub async fn topology(State(state): State<AppState>) -> ApiResult<Json<TopologyView>> {
    let rows = sqlx::query(
        "SELECT id, name, address, kind, parent_id, via_agent, group_name FROM targets ORDER BY id",
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
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(anyhow::Error::from)?;

    let sites: HashMap<TargetId, String> = agent::list_hosts(&state.pool)
        .await?
        .into_iter()
        .filter_map(|(id, _, info)| info.site.filter(|s| !s.trim().is_empty()).map(|s| (id, s)))
        .collect();

    let ids: Vec<TargetId> = targets.iter().map(|t| t.id).collect();
    let (guests, neighbors, metrics_available) = if targets.is_empty() {
        (Vec::new(), Vec::new(), true)
    } else {
        let guests = state.victoria.query(&guests_query()).await;
        let neighbors = state.victoria.query(&lldp_query()).await;
        let available = guests.is_ok() && neighbors.is_ok();
        if let Err(error) = guests.as_ref().and(neighbors.as_ref()) {
            warn!(%error, "topologie : mesures illisibles, liens hyperviseur et LLDP omis");
        }
        (
            guests.map(|s| parse_guests(&s, &ids)).unwrap_or_default(),
            neighbors.map(|s| parse_neighbors(&s, &ids)).unwrap_or_default(),
            available,
        )
    };

    let (nodes, edges) = build(&targets, &sites, &guests, &neighbors);
    Ok(Json(TopologyView { nodes, edges, metrics_available }))
}

fn guests_query() -> String {
    let names: Vec<&str> = GUEST_SERIES.iter().map(|(name, _, _)| *name).collect();
    format!(
        "last_over_time({{__name__=~\"dumbmonit_({})\"}}[{WINDOW}]) keep_metric_names",
        names.join("|")
    )
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
        out.push(Guest {
            host,
            name: name.to_string(),
            container: s.metric.get("type").is_some_and(|t| t == "lxc"),
            running: value == *running_value,
        });
    }
    out
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
        for t in targets.iter().filter(|t| t.kind != agent::modules::SERVICE_KIND) {
            index.exact.entry(t.name.trim().to_lowercase()).or_default().push(t.id);
            if let Some(key) = host_key(&t.name) {
                index.name_key.entry(key).or_default().push(t.id);
            }
            if let Some(key) = host_key(&t.address) {
                index.address_key.entry(key).or_default().push(t.id);
            }
        }
        index
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
    let by_id: HashMap<TargetId, &TargetRow> = targets.iter().map(|t| (t.id, t)).collect();
    let site_of = |t: &TargetRow| -> Option<String> {
        let own = |t: &TargetRow| {
            sites.get(&t.id).or_else(|| t.via_agent.and_then(|relay| sites.get(&relay))).cloned()
        };
        own(t).or_else(|| {
            (t.kind == agent::modules::SERVICE_KIND)
                .then(|| t.parent_id.and_then(|p| by_id.get(&p)).and_then(|h| own(h)))
                .flatten()
        })
    };
    let mut nodes: Vec<NodeView> = targets
        .iter()
        .map(|t| NodeView {
            id: node_id(t.id),
            target_id: Some(t.id),
            name: t.name.clone(),
            kind: t.kind.clone(),
            group_name: t.group_name.clone(),
            site: site_of(t),
            host_id: None,
            running: None,
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
            link(node_id(parent), node_id(t.id), "parent", &mut edges);
        }
    }
    for t in targets {
        if let Some(relay) = t.via_agent.filter(|r| by_id.contains_key(r)) {
            link(node_id(relay), node_id(t.id), "relay", &mut edges);
        }
    }

    let index = NameIndex::new(targets);
    let mut per_host: BTreeMap<TargetId, usize> = BTreeMap::new();
    for guest in guests {
        let Some(host) = by_id.get(&guest.host) else { continue };
        if let Some(target) = index.find(&guest.name, guest.host) {
            link(node_id(guest.host), node_id(target), "hypervisor", &mut edges);
            continue;
        }
        let count = per_host.entry(guest.host).or_default();
        if *count >= MAX_GUESTS_PER_HOST {
            continue;
        }
        *count += 1;
        let id = format!("g{}:{}", guest.host, guest.name);
        nodes.push(NodeView {
            id: id.clone(),
            target_id: None,
            name: guest.name.clone(),
            kind: if guest.container { "container" } else { "vm" }.to_string(),
            group_name: host.group_name.clone(),
            site: site_of(host),
            host_id: Some(host.id),
            running: Some(guest.running),
        });
        link(node_id(guest.host), id, "hypervisor", &mut edges);
    }

    for neighbor in neighbors {
        if !by_id.contains_key(&neighbor.switch) {
            continue;
        }
        if let Some(target) = index.find(&neighbor.remote, neighbor.switch) {
            link(node_id(neighbor.switch), node_id(target), "lldp", &mut edges);
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
            Guest { host: 1, name: "web01.lan".into(), container: false, running: true },
            Guest { host: 4, name: "vm-test".into(), container: false, running: true },
        ];
        let sites = HashMap::from([(2, "Annexe".to_string())]);
        let (nodes, edges) = build(&[pve, host, docker, libvirt], &sites, &guests, &[]);
        assert_eq!(
            sources(&edges),
            vec![
                ("t2", "t3", "parent"),
                ("t2", "t4", "parent"),
                ("t1", "t2", "hypervisor"),
                ("t4", "g4:vm-test", "hypervisor"),
            ]
        );
        let site = |id: &str| nodes.iter().find(|n| n.id == id).and_then(|n| n.site.clone());
        assert_eq!(site("t3").as_deref(), Some("Annexe"), "le service prend le site de sa machine");
        assert_eq!(site("g4:vm-test").as_deref(), Some("Annexe"));
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
            Guest { host: 1, name: "WEB01".into(), container: false, running: true },
            Guest { host: 1, name: "nextcloud".into(), container: true, running: false },
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
        let guests = vec![Guest { host: 1, name: "web01".into(), container: false, running: true }];
        let (nodes, edges) = build(&[pve, a, b], &HashMap::new(), &guests, &[]);
        assert_eq!(sources(&edges), vec![("t1", "g1:web01", "hypervisor")]);
        assert_eq!(nodes.len(), 4);
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
                Guest { host: 1, name: "ct1".into(), container: true, running: true },
                Guest { host: 2, name: "a".into(), container: false, running: true },
                Guest { host: 2, name: "b".into(), container: false, running: false },
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
}
