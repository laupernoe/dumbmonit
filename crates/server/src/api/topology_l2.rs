//! Liens de niveau 2 de la carte (source `l2`), lus dans les tables SNMP des
//! commutateurs : voisins LLDP et CDP, table d'adresses du pont (FDB), table
//! ARP et agrégats 802.3ad.
//!
//! - **Commutateur ↔ commutateur** : un voisin LLDP ou CDP est rapproché d'une
//!   cible par son nom système, à défaut par l'adresse MAC de son châssis (vue
//!   dans une table ARP). Les ports des deux côtés sont portés par le lien ; les
//!   liens parallèles (agrégat, ports redondants) entre la même paire n'en font
//!   qu'un.
//! - **Hôte ↔ port d'accès** : une adresse MAC apprise sur un port (FDB) est
//!   rapprochée, par l'ARP, d'une cible à cette adresse IP. Les ports qui
//!   portent un voisin LLDP/CDP, les membres d'un agrégat et les ports qui
//!   apprennent plus de [`MAX_ACCESS_MACS`] adresses sont des liaisons montantes
//!   (trunks) : aucun hôte n'y est rattaché. Une adresse vue sur plusieurs
//!   commutateurs n'est rattachée qu'à celui où elle est seule sur son port.
//!
//! Un lien déjà déduit autrement (parent, LLDP par nom…) reçoit seulement les
//! noms de ports ; un lien neuf prend la source `l2`. Rien ici n'est imposé : les
//! corrections manuelles, appliquées ensuite, gardent la priorité.
//!
//! Un lien disparaît quand ses mesures sortent de la fenêtre de lecture (30
//! minutes), soit quelques cycles de collecte.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::net::IpAddr;

use dumbmonit_proto::TargetId;

use super::topology::{EdgeView, NameIndex, TargetRow, host_key, node_id};
use crate::tsdb::InstantSeries;

/// Au-delà, un port qui apprend autant d'adresses est une liaison montante.
pub const MAX_ACCESS_MACS: usize = 24;

/// Séries lues pour le niveau 2.
pub const L2_SERIES: &str = "dumbmonit_(lldp_neighbor_info|lldp_local_port_info|cdp_neighbor_info\
|bridge_port_info|fdb_entry|fdb_entry_q|arp_entry|lag_member_info|if_oper_status)";

/// Un voisin annoncé par un commutateur (LLDP ou CDP).
#[derive(Debug, Clone, PartialEq)]
pub struct Adjacency {
    pub switch: TargetId,
    pub local_port: Option<u32>,
    pub remote_name: String,
    pub remote_port: String,
    pub remote_chassis: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FdbRow {
    pub switch: TargetId,
    pub mac: String,
    pub bridge_port: u32,
}

/// Tout ce que les commutateurs disent du niveau 2.
#[derive(Debug, Default, Clone)]
pub struct L2Data {
    pub adjacencies: Vec<Adjacency>,
    pub local_ports: HashMap<(TargetId, u32), String>,
    pub if_names: HashMap<(TargetId, u32), String>,
    pub bridge_ports: HashMap<(TargetId, u32), u32>,
    pub fdb: Vec<FdbRow>,
    pub arp: Vec<(IpAddr, String)>,
    pub lag_members: HashSet<(TargetId, u32)>,
}

/// Un lien trouvé : `a` est un commutateur, `b` un commutateur ou un hôte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub a: TargetId,
    pub a_port: Option<String>,
    pub b: TargetId,
    pub b_port: Option<String>,
}

/// `aa:bb:cc:dd:ee:ff` en minuscules, depuis un texte à 12 chiffres
/// hexadécimaux (séparateurs `:` `-` `.` admis).
pub fn parse_mac(text: &str) -> Option<String> {
    let digits: String = text.chars().filter(|c| !matches!(c, ':' | '-' | '.')).collect();
    if digits.len() != 12 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let digits = digits.to_lowercase();
    let pairs: Vec<&str> = (0..6).map(|i| &digits[i * 2..i * 2 + 2]).collect();
    Some(pairs.join(":"))
}

/// L'adresse MAC portée par les 6 derniers arcs d'un index (`….a.b.c.d.e.f`).
fn mac_from_index(index: &str) -> Option<String> {
    let arcs: Vec<u32> = index.split('.').map(|a| a.parse().ok()).collect::<Option<_>>()?;
    if arcs.len() < 6 {
        return None;
    }
    let tail = &arcs[arcs.len() - 6..];
    if tail.iter().any(|b| *b > 255) {
        return None;
    }
    Some(tail.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(":"))
}

fn arc(index: &str, n: usize) -> Option<u32> {
    index.split('.').nth(n)?.parse().ok()
}

/// L'adresse IPv4 portée par les 4 derniers arcs d'un index ARP.
fn ip_from_index(index: &str) -> Option<IpAddr> {
    let arcs: Vec<u8> = index.split('.').map(|a| a.parse().ok()).collect::<Option<_>>()?;
    if arcs.len() < 4 {
        return None;
    }
    let t = &arcs[arcs.len() - 4..];
    Some(IpAddr::from([t[0], t[1], t[2], t[3]]))
}

/// Un identifiant CDP `sw2.lan(FOC1234)` : seul le nom compte.
fn cdp_name(text: &str) -> &str {
    text.split('(').next().unwrap_or(text).trim()
}

impl L2Data {
    /// Lit les séries de [`L2_SERIES`] ; celles d'une cible inconnue ou sans
    /// l'étiquette attendue sont ignorées.
    pub fn parse(series: &[InstantSeries], known: &[TargetId]) -> Self {
        let mut data = Self::default();
        for s in series {
            let Some(name) = s.metric.get("__name__") else { continue };
            let Some(target) = s.metric.get("target").and_then(|t| t.parse::<TargetId>().ok())
            else {
                continue;
            };
            if !known.contains(&target) {
                continue;
            }
            let Some(index) = s.metric.get("index") else { continue };
            let label = |key: &str| s.metric.get(key).map(|v| v.trim()).filter(|v| !v.is_empty());
            let value = s.value.1.parse::<f64>().ok();
            match name.as_str() {
                "dumbmonit_lldp_neighbor_info" => {
                    let remote = label("remote_sysname").unwrap_or_default();
                    let chassis = label("remote_chassis").and_then(parse_mac);
                    if remote.is_empty() && chassis.is_none() {
                        continue;
                    }
                    data.adjacencies.push(Adjacency {
                        switch: target,
                        local_port: arc(index, 1),
                        remote_name: remote.to_string(),
                        remote_port: label("remote_port")
                            .or_else(|| label("remote_port_desc"))
                            .unwrap_or_default()
                            .to_string(),
                        remote_chassis: chassis,
                    });
                }
                "dumbmonit_cdp_neighbor_info" => {
                    let remote = cdp_name(label("remote_sysname").unwrap_or_default());
                    if remote.is_empty() {
                        continue;
                    }
                    data.adjacencies.push(Adjacency {
                        switch: target,
                        local_port: arc(index, 0),
                        remote_name: remote.to_string(),
                        remote_port: label("remote_port").unwrap_or_default().to_string(),
                        remote_chassis: None,
                    });
                }
                "dumbmonit_lldp_local_port_info" => {
                    if let (Some(port), Some(text)) =
                        (index.parse::<u32>().ok(), label("port_id").or_else(|| label("port_desc")))
                    {
                        data.local_ports.insert((target, port), text.to_string());
                    }
                }
                "dumbmonit_if_oper_status" => {
                    if let (Some(port), Some(text)) =
                        (index.parse::<u32>().ok(), label("ifname"))
                    {
                        data.if_names.insert((target, port), text.to_string());
                    }
                }
                "dumbmonit_bridge_port_info" => {
                    if let (Some(base), Some(v)) = (index.parse::<u32>().ok(), value) {
                        data.bridge_ports.insert((target, base), v as u32);
                    }
                }
                "dumbmonit_fdb_entry" | "dumbmonit_fdb_entry_q" => {
                    // 3 = apprise ; 5 = statique (gestion) : on garde l'apprise.
                    if label("status").is_some_and(|st| st != "3") {
                        continue;
                    }
                    if let (Some(mac), Some(v)) = (mac_from_index(index), value.filter(|v| *v >= 1.0))
                    {
                        data.fdb.push(FdbRow { switch: target, mac, bridge_port: v as u32 });
                    }
                }
                "dumbmonit_arp_entry" => {
                    if let (Some(ip), Some(mac)) =
                        (ip_from_index(index), label("mac").and_then(parse_mac))
                    {
                        data.arp.push((ip, mac));
                    }
                }
                "dumbmonit_lag_member_info" => {
                    if let (Some(port), Some(_)) =
                        (index.parse::<u32>().ok(), value.filter(|v| *v >= 1.0))
                    {
                        data.lag_members.insert((target, port));
                    }
                }
                _ => {}
            }
        }
        data
    }

    /// Nom lisible d'un port : ifName, à défaut le nom LLDP, à défaut `#n`.
    fn port_name(&self, switch: TargetId, if_index: u32) -> String {
        self.if_names
            .get(&(switch, if_index))
            .or_else(|| self.local_ports.get(&(switch, if_index)))
            .cloned()
            .unwrap_or_else(|| format!("#{if_index}"))
    }

    /// ifIndex d'un port du pont (le numéro lui-même sans table de ports).
    fn if_index_of(&self, switch: TargetId, bridge_port: u32) -> u32 {
        self.bridge_ports.get(&(switch, bridge_port)).copied().unwrap_or(bridge_port)
    }
}

/// Jusqu'à 4 noms de ports, triés, séparés par des virgules.
fn join_ports(ports: &BTreeSet<String>) -> Option<String> {
    if ports.is_empty() {
        return None;
    }
    let shown: Vec<&str> = ports.iter().take(4).map(String::as_str).collect();
    let more = ports.len().saturating_sub(4);
    let mut text = shown.join(", ");
    if more > 0 {
        text.push_str(&format!(" +{more}"));
    }
    Some(text)
}

/// Les cibles joignables par adresse MAC : MAC -> cible, d'après l'ARP et les
/// adresses IP des cibles. Une adresse ambiguë n'est attribuée à personne.
fn mac_owners(targets: &[TargetRow], data: &L2Data) -> HashMap<String, TargetId> {
    let mut by_ip: HashMap<IpAddr, Vec<&TargetRow>> = HashMap::new();
    for t in targets.iter().filter(|t| t.kind != crate::collectors::agent::modules::SERVICE_KIND)
    {
        let ip = host_key(&t.address).and_then(|k| k.parse::<IpAddr>().ok());
        if let Some(ip) = ip.filter(|ip| !ip.is_loopback()) {
            by_ip.entry(ip).or_default().push(t);
        }
    }
    let mut owners: HashMap<String, BTreeSet<TargetId>> = HashMap::new();
    for (ip, mac) in &data.arp {
        let Some(list) = by_ip.get(ip) else { continue };
        let pick = match list.as_slice() {
            [one] => Some(one.id),
            many => {
                let machines: Vec<TargetId> = many
                    .iter()
                    .filter(|t| matches!(t.kind.as_str(), "agent" | "snmp"))
                    .map(|t| t.id)
                    .collect();
                (machines.len() == 1).then(|| machines[0])
            }
        };
        if let Some(id) = pick {
            owners.entry(mac.clone()).or_default().insert(id);
        }
    }
    owners
        .into_iter()
        .filter_map(|(mac, ids)| (ids.len() == 1).then(|| (mac, *ids.iter().next().unwrap())))
        .collect()
}

/// Les liens de niveau 2 que disent les tables.
pub fn derive(targets: &[TargetRow], data: &L2Data) -> Vec<Link> {
    let known: HashSet<TargetId> = targets.iter().map(|t| t.id).collect();
    let index = NameIndex::new(targets);
    let owners = mac_owners(targets, data);

    // --- commutateur <-> commutateur
    // (paire ordonnée) -> noms de ports de chaque côté : propres, puis annoncés.
    #[derive(Default)]
    struct Sides {
        own: BTreeMap<TargetId, BTreeSet<String>>,
        reported: BTreeMap<TargetId, BTreeSet<String>>,
    }
    let mut pairs: BTreeMap<(TargetId, TargetId), Sides> = BTreeMap::new();
    let mut uplinks: HashSet<(TargetId, u32)> = data.lag_members.clone();
    for adj in data.adjacencies.iter().filter(|a| known.contains(&a.switch)) {
        if let Some(port) = adj.local_port {
            uplinks.insert((adj.switch, port));
        }
        let by_name = (!adj.remote_name.is_empty())
            .then(|| index.find(&adj.remote_name, adj.switch))
            .flatten();
        let remote = by_name
            .or_else(|| adj.remote_chassis.as_ref().and_then(|m| owners.get(m)).copied())
            .filter(|id| *id != adj.switch);
        let Some(remote) = remote else { continue };
        let key = (adj.switch.min(remote), adj.switch.max(remote));
        let sides = pairs.entry(key).or_default();
        if let Some(port) = adj.local_port {
            sides.own.entry(adj.switch).or_default().insert(data.port_name(adj.switch, port));
        }
        if !adj.remote_port.is_empty() {
            sides.reported.entry(remote).or_default().insert(adj.remote_port.clone());
        }
    }
    let mut links = Vec::new();
    for ((a, b), sides) in &pairs {
        let ports = |id: &TargetId| {
            let own = sides.own.get(id).filter(|s| !s.is_empty());
            join_ports(own.or_else(|| sides.reported.get(id)).unwrap_or(&BTreeSet::new()))
        };
        links.push(Link { a: *a, a_port: ports(a), b: *b, b_port: ports(b) });
    }

    // --- hôte <-> port d'accès
    // Adresses apprises par (commutateur, ifIndex).
    let mut per_port: BTreeMap<(TargetId, u32), BTreeSet<&str>> = BTreeMap::new();
    for row in data.fdb.iter().filter(|r| known.contains(&r.switch)) {
        let port = data.if_index_of(row.switch, row.bridge_port);
        per_port.entry((row.switch, port)).or_default().insert(row.mac.as_str());
    }
    // Pour chaque adresse, les ports d'accès où elle est vue : (nb d'adresses du
    // port, commutateur, port).
    let mut seen: BTreeMap<&str, Vec<(usize, TargetId, u32)>> = BTreeMap::new();
    for ((switch, port), macs) in &per_port {
        if uplinks.contains(&(*switch, *port)) || macs.len() > MAX_ACCESS_MACS {
            continue;
        }
        for mac in macs {
            seen.entry(*mac).or_default().push((macs.len(), *switch, *port));
        }
    }
    let mut hosts: BTreeMap<(TargetId, TargetId), BTreeSet<String>> = BTreeMap::new();
    for (mac, spots) in &seen {
        let Some(&host) = owners.get(*mac) else { continue };
        let best = spots.iter().map(|s| s.0).min().unwrap_or(0);
        let closest: Vec<(usize, TargetId, u32)> =
            spots.iter().filter(|s| s.0 == best).copied().collect();
        let [(_, switch, port)] = closest.as_slice() else { continue };
        // Un commutateur n'est pas un hôte d'accès, et un lien déjà tracé suffit.
        let key = (std::cmp::min(*switch, host), std::cmp::max(*switch, host));
        if host == *switch || pairs.contains_key(&key) || !known.contains(&host) {
            continue;
        }
        hosts.entry((*switch, host)).or_default().insert(data.port_name(*switch, *port));
    }
    for ((switch, host), ports) in &hosts {
        links.push(Link { a: *switch, a_port: join_ports(ports), b: *host, b_port: None });
    }
    links
}

/// Pose les liens sur la carte : les noms de ports vont sur un lien déjà
/// déduit entre la même paire, sinon un lien `l2` est ajouté.
pub fn apply(edges: &mut Vec<EdgeView>, links: &[Link]) {
    for link in links {
        let (a, b) = (node_id(link.a), node_id(link.b));
        if let Some(edge) = edges
            .iter_mut()
            .find(|e| (e.from == a && e.to == b) || (e.from == b && e.to == a))
        {
            if edge.from == a {
                edge.from_port = link.a_port.clone();
                edge.to_port = link.b_port.clone();
            } else {
                edge.from_port = link.b_port.clone();
                edge.to_port = link.a_port.clone();
            }
        } else {
            let mut edge = EdgeView::deduced(a, b, "l2");
            edge.from_port = link.a_port.clone();
            edge.to_port = link.b_port.clone();
            edges.push(edge);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(id: TargetId, name: &str, address: &str, kind: &str) -> TargetRow {
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

    fn serie(name: &str, target: TargetId, value: &str, labels: &[(&str, &str)]) -> InstantSeries {
        let mut metric = BTreeMap::new();
        metric.insert("__name__".to_string(), format!("dumbmonit_{name}"));
        metric.insert("target".to_string(), target.to_string());
        for (k, v) in labels {
            metric.insert((*k).to_string(), (*v).to_string());
        }
        InstantSeries { metric, value: (1.0, value.to_string()) }
    }

    #[test]
    fn mac_depuis_texte_et_index() {
        assert_eq!(parse_mac("AA-BB-CC-DD-EE-FF").as_deref(), Some("aa:bb:cc:dd:ee:ff"));
        assert_eq!(parse_mac("sw1.lan"), None);
        assert_eq!(mac_from_index("1.0.26.43.60.77.255").as_deref(), Some("00:1a:2b:3c:4d:ff"));
        assert_eq!(mac_from_index("0.26.43.60.77.256"), None);
    }

    #[test]
    fn lldp_donne_un_lien_commutateur_avec_ports_des_deux_cotes() {
        let targets = [
            target(1, "sw1", "10.0.0.1", "snmp"),
            target(2, "sw2", "10.0.0.2", "snmp"),
        ];
        let series = [
            serie("lldp_neighbor_info", 1, "4", &[
                ("index", "0.7.1"), ("remote_sysname", "sw2.lan"), ("remote_port", "Gi1/0/24"),
            ]),
            serie("lldp_neighbor_info", 2, "4", &[
                ("index", "0.24.1"), ("remote_sysname", "SW1"), ("remote_port", "Gi1/0/7"),
            ]),
            serie("lldp_local_port_info", 1, "5", &[("index", "7"), ("port_id", "Gi1/0/7")]),
            serie("lldp_local_port_info", 2, "5", &[("index", "24"), ("port_id", "Gi1/0/24")]),
        ];
        let data = L2Data::parse(&series, &[1, 2]);
        let links = derive(&targets, &data);
        assert_eq!(
            links,
            vec![Link {
                a: 1,
                a_port: Some("Gi1/0/7".into()),
                b: 2,
                b_port: Some("Gi1/0/24".into())
            }]
        );
    }

    #[test]
    fn cdp_et_liens_paralleles_ne_font_qu_un_lien() {
        let targets = [
            target(1, "core", "10.0.0.1", "snmp"),
            target(2, "access", "10.0.0.2", "snmp"),
        ];
        let series = [
            serie("cdp_neighbor_info", 1, "1", &[
                ("index", "3.1"), ("remote_sysname", "access.lan(FOC123)"),
                ("remote_port", "Te1/1"),
            ]),
            serie("cdp_neighbor_info", 1, "1", &[
                ("index", "4.1"), ("remote_sysname", "access.lan(FOC123)"),
                ("remote_port", "Te1/2"),
            ]),
            serie("if_oper_status", 1, "1", &[("index", "3"), ("ifname", "Te1/0/3")]),
            serie("if_oper_status", 1, "1", &[("index", "4"), ("ifname", "Te1/0/4")]),
        ];
        let links = derive(&targets, &L2Data::parse(&series, &[1, 2]));
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].a_port.as_deref(), Some("Te1/0/3, Te1/0/4"));
        assert_eq!(links[0].b_port.as_deref(), Some("Te1/1, Te1/2"));
    }

    /// Commutateur 1 : port 5 vers un hôte, port 24 trunk vers sw2 (LLDP) qui
    /// voit lui aussi l'hôte (derrière le trunk), port 25 membre d'agrégat.
    #[test]
    fn la_fdb_rattache_l_hote_au_port_d_acces_et_pas_au_trunk() {
        let targets = [
            target(1, "sw1", "10.0.0.1", "snmp"),
            target(2, "sw2", "10.0.0.2", "snmp"),
            target(3, "nas", "10.0.0.30", "agent"),
            target(4, "cam", "10.0.0.40", "ping"),
        ];
        let host = "0.17.34.51.68.85"; // 00:11:22:33:44:55
        let cam = "0.17.34.51.68.102"; // 00:11:22:33:44:66
        let series = [
            serie("lldp_neighbor_info", 1, "4", &[
                ("index", "0.24.1"), ("remote_sysname", "sw2"), ("remote_port", "Gi24"),
            ]),
            serie("fdb_entry", 1, "5", &[("index", host), ("status", "3")]),
            serie("fdb_entry", 1, "24", &[("index", cam), ("status", "3")]),
            // sw2 voit l'hôte via le trunk (port 24) et la caméra sur son port 3.
            serie("fdb_entry", 2, "24", &[("index", host), ("status", "3")]),
            serie("fdb_entry", 2, "3", &[("index", cam), ("status", "3")]),
            serie("lldp_neighbor_info", 2, "4", &[
                ("index", "0.24.1"), ("remote_sysname", "sw1"), ("remote_port", "Gi24"),
            ]),
            serie("bridge_port_info", 1, "105", &[("index", "5")]),
            serie("if_oper_status", 1, "1", &[("index", "105"), ("ifname", "Gi1/0/5")]),
            serie("if_oper_status", 2, "1", &[("index", "3"), ("ifname", "Gi1/0/3")]),
            serie("arp_entry", 1, "3", &[("index", "1.10.0.0.30"), ("mac", "00:11:22:33:44:55")]),
            serie("arp_entry", 1, "3", &[("index", "1.10.0.0.40"), ("mac", "00:11:22:33:44:66")]),
            // port 7 de sw1 : membre d'agrégat, ses adresses ne comptent pas.
            serie("lag_member_info", 1, "99", &[("index", "7"), ("ifname", "Gi1/0/7")]),
            serie("fdb_entry", 1, "7", &[("index", "0.17.34.51.68.119"), ("status", "3")]),
        ];
        let mut data = L2Data::parse(&series, &[1, 2, 3, 4]);
        // Le port 5 du pont de sw1 est l'ifIndex 105 ; ses autres ports sont eux-mêmes.
        assert_eq!(data.if_index_of(1, 5), 105);
        data.lag_members.insert((1, 7));
        let links = derive(&targets, &data);
        assert!(links.iter().any(|l| l.a == 1 && l.b == 2));
        assert!(links.contains(&Link {
            a: 1, a_port: Some("Gi1/0/5".into()), b: 3, b_port: None
        }));
        assert!(links.contains(&Link {
            a: 2, a_port: Some("Gi1/0/3".into()), b: 4, b_port: None
        }));
        // L'hôte n'est rattaché qu'une fois, et pas au trunk de sw2.
        assert_eq!(links.iter().filter(|l| l.b == 3).count(), 1);
        assert!(!links.iter().any(|l| l.a == 1 && l.b == 4));
    }

    #[test]
    fn un_port_qui_apprend_trop_d_adresses_est_un_trunk() {
        let targets = [target(1, "sw", "10.0.0.1", "snmp"), target(3, "nas", "10.0.0.30", "agent")];
        let mut series = vec![serie("arp_entry", 1, "3", &[
            ("index", "1.10.0.0.30"), ("mac", "00:11:22:33:44:55"),
        ])];
        series.push(serie("fdb_entry", 1, "9", &[("index", "0.17.34.51.68.85"), ("status", "3")]));
        for i in 0..=MAX_ACCESS_MACS {
            series.push(serie("fdb_entry", 1, "9", &[
                ("index", format!("2.0.0.0.0.{i}").as_str()), ("status", "3"),
            ]));
        }
        let links = derive(&targets, &L2Data::parse(&series, &[1, 3]));
        assert!(links.is_empty(), "{links:?}");
    }

    #[test]
    fn les_noms_de_ports_vont_sur_le_lien_existant_sinon_l2() {
        let mut edges = vec![EdgeView::deduced("t2".into(), "t1".into(), "lldp")];
        let links = [
            Link { a: 1, a_port: Some("Gi1".into()), b: 2, b_port: Some("Gi24".into()) },
            Link { a: 1, a_port: Some("Gi5".into()), b: 3, b_port: None },
        ];
        apply(&mut edges, &links);
        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0].source, "lldp");
        assert_eq!(edges[0].from_port.as_deref(), Some("Gi24"));
        assert_eq!(edges[0].to_port.as_deref(), Some("Gi1"));
        assert_eq!(edges[1].source, "l2");
        assert_eq!((edges[1].from.as_str(), edges[1].to.as_str()), ("t1", "t3"));
    }
}
