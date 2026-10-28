//! Corrections de la carte de topologie et règles apprises.
//!
//! La carte se déduit seule (`topology.rs`), et se trompe parfois : un
//! équipement « passe par le pare-feu » alors qu'il est derrière un
//! commutateur, un moniteur HTTP ne sait pas sur quelle VM il tourne. Un
//! administrateur corrige la carte sur place :
//!
//! - `forbid` : ce lien est faux, ne plus jamais le déduire ;
//! - `link` : un lien forcé, d'un type choisi ;
//! - `parent` : un nouveau parent pour un nœud (glisser un nœud sur un autre).
//!   Case « utiliser aussi pour les alertes » cochée, le rattachement devient
//!   aussi le parent déclaré (`targets.parent_id`), celui que suit la
//!   suppression des alertes en cascade ; décochée, il ne change que la carte ;
//! - `site` et `group` : le site et le regroupement d'un nœud sur la carte.
//!
//! Les corrections s'appliquent après la déduction, dans cet ordre : liens
//! interdits (à la main, puis par règle), liens forcés, rattachements manuels,
//! puis règles de rattachement apprises — une règle ne touche jamais un nœud
//! qu'une correction place déjà, ni un nœud qu'un lien sûr place (parent
//! déclaré, service d'agent, hyperviseur, invité). Aucune ne crée de nœud.
//!
//! **Le système comprend** : des heuristiques déterministes, sans
//! apprentissage statistique, tirent des corrections des règles générales,
//! proposées (jamais appliquées sans accord) puis appliquées aux équipements
//! présents et futurs une fois acceptées :
//!
//! - un lien d'un type interdit vers un équipement d'un sous-réseau qui en
//!   compte au moins deux propose d'interdire ce type de lien pour tout le
//!   sous-réseau ;
//! - deux rattachements au moins sous le même parent, d'équipements à la même
//!   adresse, proposent « ce qui est à cette adresse dépend de ce parent »
//!   (limité au type d'équipement quand ils ont tous le même) ;
//! - deux rattachements au moins sous le même parent, d'équipements du même
//!   sous-réseau, proposent « ce sous-réseau dépend de ce parent ».
//!
//! Une suggestion qu'une autre correction contredit (un équipement du même
//! périmètre rattaché ailleurs) n'est pas proposée. Une règle refusée est
//! gardée (`dismissed`) pour ne plus revenir.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::net::Ipv4Addr;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get};
use axum::{Json, Router};
use dumbmonit_proto::TargetId;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, Sqlite, SqlitePool, Transaction};

use super::topology::{Cidr, EdgeView, FIREWALL_KINDS, NodeView, TargetRow, host_key, ipv4_of};
use crate::api::{ApiError, ApiResult};
use crate::auth::middleware::AdminIdentity;
use crate::collectors::agent::modules::SERVICE_KIND;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/topology/corrections", get(list_corrections).post(create_correction))
        .route("/topology/corrections/{id}", delete(delete_correction))
        .route("/topology/rules", get(list_rules).post(save_rule))
        .route("/topology/rules/{id}", delete(delete_rule))
        .route("/topology/suggestions", get(list_suggestions))
}

/// Tous les types de lien de la carte.
pub const SOURCES: &[&str] = &[
    "parent",
    "service",
    "relay",
    "hypervisor",
    "hosted",
    "lldp",
    "l2",
    "samehost",
    "backup",
    "firewall",
    "manual",
];

/// Types qu'un lien forcé peut prendre. `manual` : « dépend de », sur la carte
/// seule ; un parent d'alerte se pose par un rattachement (`parent`).
pub const LINK_SOURCES: &[&str] =
    &["manual", "hypervisor", "hosted", "relay", "lldp", "l2", "samehost", "backup", "firewall"];

/// Liens déduits qu'une règle peut interdire pour tout un périmètre.
pub const GENERAL_FORBID: &[&str] =
    &["hypervisor", "hosted", "lldp", "l2", "samehost", "backup", "firewall"];

/// Liens qui placent un nœud avec certitude : une règle apprise n'y touche pas.
const STRONG: &[&str] = &["parent", "service", "hypervisor", "hosted", "manual"];

const MAX_NODE_CHARS: usize = 300;
const MAX_VALUE_CHARS: usize = 80;
const MAX_KIND_CHARS: usize = 64;
/// Au-delà, la carte se corrige mieux à la source (parents déclarés, sites).
const MAX_CORRECTIONS: i64 = 5000;
const MAX_RULES: i64 = 500;

fn static_source(text: &str) -> Option<&'static str> {
    SOURCES.iter().copied().find(|s| *s == text)
}

// ------------------------------------------------------------------- nœuds

/// Ce qu'un identifiant de nœud désigne : l'équipement lui-même (`t12`), ou
/// un invité (`g12:web`) ou un nœud de cluster (`n12:pve1`) de l'hyperviseur 12.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRef<'a> {
    Target(TargetId),
    Guest(TargetId, &'a str),
    PveNode(TargetId, &'a str),
}

impl NodeRef<'_> {
    /// L'équipement visé, ou l'hyperviseur qui porte l'invité ou le nœud.
    pub fn host(&self) -> TargetId {
        match self {
            Self::Target(id) | Self::Guest(id, _) | Self::PveNode(id, _) => *id,
        }
    }
}

pub fn parse_node(id: &str) -> Option<NodeRef<'_>> {
    if id.len() > MAX_NODE_CHARS {
        return None;
    }
    let (prefix, rest) = id.split_at_checked(1)?;
    if prefix == "t" {
        let target: TargetId = rest.parse().ok()?;
        return (target > 0).then_some(NodeRef::Target(target));
    }
    let (host, name) = rest.split_once(':')?;
    let host: TargetId = host.parse().ok()?;
    if host <= 0 || name.is_empty() {
        return None;
    }
    match prefix {
        "g" => Some(NodeRef::Guest(host, name)),
        "n" => Some(NodeRef::PveNode(host, name)),
        _ => None,
    }
}

/// L'équipement d'un nœud `t<id>` ; `None` pour un invité ou un nœud de cluster.
pub fn target_node(id: &str) -> Option<TargetId> {
    match parse_node(id)? {
        NodeRef::Target(target) => Some(target),
        _ => None,
    }
}

/// Un nœud en deux morceaux portables : l'équipement (ou l'hyperviseur), et
/// `g:<nom>` ou `n:<nœud>` pour ce qui vit dessous. Sert à la sauvegarde.
pub fn node_parts(id: &str) -> Option<(TargetId, Option<String>)> {
    Some(match parse_node(id)? {
        NodeRef::Target(target) => (target, None),
        NodeRef::Guest(host, name) => (host, Some(format!("g:{name}"))),
        NodeRef::PveNode(host, name) => (host, Some(format!("n:{name}"))),
    })
}

/// L'inverse de [`node_parts`], avec l'identifiant d'équipement d'ici.
pub fn node_from_parts(host: TargetId, child: Option<&str>) -> Option<String> {
    let id = match child {
        None => format!("t{host}"),
        Some(child) => match child.split_once(':')? {
            ("g", name) if !name.is_empty() => format!("g{host}:{name}"),
            ("n", name) if !name.is_empty() => format!("n{host}:{name}"),
            _ => return None,
        },
    };
    parse_node(&id).is_some().then_some(id)
}

// -------------------------------------------------------------- corrections

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionKind {
    Forbid,
    Link,
    Parent,
    Site,
    Group,
}

impl CorrectionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Forbid => "forbid",
            Self::Link => "link",
            Self::Parent => "parent",
            Self::Site => "site",
            Self::Group => "group",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "forbid" => Some(Self::Forbid),
            "link" => Some(Self::Link),
            "parent" => Some(Self::Parent),
            "site" => Some(Self::Site),
            "group" => Some(Self::Group),
            _ => None,
        }
    }
}

/// Une correction de la carte.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Correction {
    pub id: i64,
    pub kind: CorrectionKind,
    /// Côté amont : le nouveau parent, ou l'origine du lien interdit ou forcé.
    pub from: Option<String>,
    pub to: String,
    /// Type du lien interdit ou forcé.
    pub source: Option<String>,
    /// Site (vide : site principal) ou regroupement.
    pub value: Option<String>,
    /// Rattachement : aussi le parent d'alerte de l'équipement.
    pub use_for_alerts: bool,
    pub author: String,
    /// UTC, sans suffixe.
    pub created_at: String,
    /// Parent d'alerte d'avant la correction, rendu quand on la retire.
    #[serde(skip)]
    pub previous_parent: Option<TargetId>,
}

/// Colonnes d'une correction : un littéral, que `sqlx` accepte sans audit.
macro_rules! correction_columns {
    () => {
        "SELECT id, kind, from_node, to_node, source, value, use_for_alerts, previous_parent,
             author, created_at FROM topology_corrections"
    };
}
const CORRECTIONS_ALL: &str = concat!(correction_columns!(), " ORDER BY id");
const CORRECTION_ONE: &str = concat!(correction_columns!(), " WHERE id = ?");

fn correction_row(row: &SqliteRow) -> Result<Correction, sqlx::Error> {
    let kind: String = row.try_get("kind")?;
    Ok(Correction {
        id: row.try_get("id")?,
        kind: CorrectionKind::parse(&kind)
            .ok_or_else(|| sqlx::Error::Decode(format!("correction inconnue : {kind}").into()))?,
        from: row.try_get("from_node")?,
        to: row.try_get("to_node")?,
        source: row.try_get("source")?,
        value: row.try_get("value")?,
        use_for_alerts: row.try_get::<i64, _>("use_for_alerts")? != 0,
        previous_parent: row.try_get("previous_parent")?,
        author: row.try_get("author")?,
        created_at: row.try_get("created_at")?,
    })
}

/// Toutes les corrections, les plus anciennes d'abord.
pub async fn load_corrections(pool: &SqlitePool) -> anyhow::Result<Vec<Correction>> {
    let rows = sqlx::query(CORRECTIONS_ALL).fetch_all(pool).await?;
    Ok(rows.iter().map(correction_row).collect::<Result<Vec<_>, _>>()?)
}

// ------------------------------------------------------------------- règles

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleKind {
    Forbid,
    Parent,
}

impl RuleKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Forbid => "forbid",
            Self::Parent => "parent",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "forbid" => Some(Self::Forbid),
            "parent" => Some(Self::Parent),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleStatus {
    Active,
    Dismissed,
}

impl RuleStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Dismissed => "dismissed",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "active" => Some(Self::Active),
            "dismissed" => Some(Self::Dismissed),
            _ => None,
        }
    }
}

/// Ce que dit une règle : son périmètre (sous-réseau ou adresse, type
/// d'équipement) et son effet (lien interdit, ou parent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleSpec {
    pub kind: RuleKind,
    #[serde(default)]
    pub subnet: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub target_kind: Option<String>,
    /// `forbid` : le type de lien interdit.
    #[serde(default)]
    pub source: Option<String>,
    /// `parent` : le nœud dont le périmètre dépend.
    #[serde(default)]
    pub parent: Option<String>,
}

impl RuleSpec {
    /// Résumé stable : deux suggestions identiques ont la même clé.
    pub fn key(&self) -> String {
        let part = |value: &Option<String>| value.clone().unwrap_or_default();
        format!(
            "{}|{}|{}|{}|{}|{}",
            self.kind.as_str(),
            part(&self.subnet),
            part(&self.address),
            part(&self.target_kind),
            part(&self.source),
            part(&self.parent)
        )
    }

    /// Vrai quand l'équipement décrit par `info` tombe dans le périmètre.
    fn covers(&self, info: &Info) -> bool {
        if info.target.is_none() || (self.subnet.is_none() && self.address.is_none()) {
            return false;
        }
        if let Some(kind) = &self.target_kind
            && *kind != info.kind
        {
            return false;
        }
        if let Some(address) = &self.address
            && info.address.as_deref() != Some(address.as_str())
        {
            return false;
        }
        if let Some(subnet) = &self.subnet {
            let Some(cidr) = Cidr::parse(subnet) else { return false };
            let inside = info.subnet == Some(cidr) || info.ip.is_some_and(|ip| cidr.contains(ip));
            if !inside {
                return false;
            }
        }
        true
    }
}

/// Une règle acceptée (`active`) ou refusée (`dismissed`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rule {
    pub id: i64,
    pub status: RuleStatus,
    #[serde(flatten)]
    pub spec: RuleSpec,
    /// Corrections qui l'ont fait naître.
    pub evidence: i64,
    pub author: String,
    pub created_at: String,
    /// Liens qu'elle pose ou retire sur la carte actuelle.
    pub matches: usize,
}

/// Une règle que les corrections laissent deviner, à accepter ou refuser.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Suggestion {
    pub key: String,
    #[serde(flatten)]
    pub spec: RuleSpec,
    /// Nombre de corrections qui la fondent, et lesquelles.
    pub evidence: usize,
    pub corrections: Vec<i64>,
    /// Autres liens qu'elle changerait aujourd'hui sur la carte.
    pub matches: usize,
}

fn rule_row(row: &SqliteRow) -> Result<Rule, sqlx::Error> {
    let kind: String = row.try_get("kind")?;
    let status: String = row.try_get("status")?;
    let unreadable = |what: &str| sqlx::Error::Decode(format!("règle illisible : {what}").into());
    Ok(Rule {
        id: row.try_get("id")?,
        status: RuleStatus::parse(&status).ok_or_else(|| unreadable(&status))?,
        spec: RuleSpec {
            kind: RuleKind::parse(&kind).ok_or_else(|| unreadable(&kind))?,
            subnet: row.try_get("subnet")?,
            address: row.try_get("address")?,
            target_kind: row.try_get("target_kind")?,
            source: row.try_get("source")?,
            parent: row.try_get("parent_node")?,
        },
        evidence: row.try_get("evidence")?,
        author: row.try_get("author")?,
        created_at: row.try_get("created_at")?,
        matches: 0,
    })
}

macro_rules! rule_columns {
    () => {
        "SELECT id, kind, status, subnet, address, target_kind, source, parent_node, evidence,
             author, created_at FROM topology_rules"
    };
}
const RULES_ALL: &str = concat!(rule_columns!(), " ORDER BY id");
const RULE_ONE: &str = concat!(rule_columns!(), " WHERE id = ?");

/// Toutes les règles, les plus anciennes d'abord.
pub async fn load_rules(pool: &SqlitePool) -> anyhow::Result<Vec<Rule>> {
    let rows = sqlx::query(RULES_ALL).fetch_all(pool).await?;
    Ok(rows.iter().map(rule_row).collect::<Result<Vec<_>, _>>()?)
}

// --------------------------------------------------------------- application

/// Ce que l'application des règles doit savoir d'un nœud.
#[derive(Debug, Clone)]
struct Info {
    subnet: Option<Cidr>,
    ip: Option<Ipv4Addr>,
    /// Clé d'hôte de l'adresse de l'équipement (voir [`host_key`]).
    address: Option<String>,
    kind: String,
    target: Option<TargetId>,
    parent_id: Option<TargetId>,
}

fn infos(targets: &[TargetRow], nodes: &[NodeView]) -> HashMap<String, Info> {
    let by_id: HashMap<TargetId, &TargetRow> = targets.iter().map(|t| (t.id, t)).collect();
    nodes
        .iter()
        .map(|n| {
            let row = n.target_id.and_then(|id| by_id.get(&id).copied());
            let info = Info {
                subnet: n.subnet.as_deref().and_then(Cidr::parse),
                ip: row.and_then(|t| ipv4_of(&t.address)),
                address: row.and_then(|t| host_key(&t.address)),
                kind: n.kind.clone(),
                target: n.target_id,
                parent_id: row.and_then(|t| t.parent_id),
            };
            (n.id.clone(), info)
        })
        .collect()
}

fn same_pair(edge: &EdgeView, a: &str, b: &str) -> bool {
    (edge.from == a && edge.to == b) || (edge.from == b && edge.to == a)
}

/// Vrai quand `target` se trouve en aval de `from`, en suivant les liens.
fn reaches(edges: &[EdgeView], from: &str, target: &str) -> bool {
    let mut seen: HashSet<&str> = HashSet::from([from]);
    let mut queue = vec![from];
    while let Some(at) = queue.pop() {
        for edge in edges.iter().filter(|e| e.from == at) {
            if edge.to == target {
                return true;
            }
            if seen.insert(edge.to.as_str()) {
                queue.push(edge.to.as_str());
            }
        }
    }
    false
}

/// Un nœud qu'une règle de rattachement peut placer.
fn placeable(rule: &RuleSpec, id: &str, info: &Info, parent: &str, edges: &[EdgeView]) -> bool {
    id != parent
        && !FIREWALL_KINDS.contains(&info.kind.as_str())
        && info.kind != SERVICE_KIND
        && rule.covers(info)
        && !edges.iter().any(|e| e.to == id && STRONG.contains(&e.source))
        && !reaches(edges, id, parent)
}

/// Applique corrections et règles acceptées à la carte déduite. Renvoie, par
/// règle, le nombre de liens qu'elle a posés ou retirés.
pub fn apply(
    targets: &[TargetRow],
    nodes: &mut [NodeView],
    edges: &mut Vec<EdgeView>,
    corrections: &[Correction],
    rules: &[Rule],
) -> HashMap<i64, usize> {
    let info = infos(targets, nodes);
    let known = |id: &str| info.contains_key(id);
    let mut matches: HashMap<i64, usize> = HashMap::new();

    // 1. Liens interdits à la main : plus jamais déduits.
    for c in corrections.iter().filter(|c| c.kind == CorrectionKind::Forbid) {
        let Some(from) = c.from.as_deref() else { continue };
        edges.retain(|e| {
            !(same_pair(e, from, &c.to) && c.source.as_deref().is_none_or(|s| s == e.source))
        });
    }

    // 2. Liens interdits par une règle acceptée, pour tout son périmètre.
    let active = |kind: RuleKind| {
        rules.iter().filter(move |r| r.status == RuleStatus::Active && r.spec.kind == kind)
    };
    for rule in active(RuleKind::Forbid) {
        let Some(source) = rule.spec.source.as_deref() else { continue };
        let before = edges.len();
        edges.retain(|e| {
            !(e.source == source && info.get(&e.to).is_some_and(|i| rule.spec.covers(i)))
        });
        let removed = before - edges.len();
        if removed > 0 {
            *matches.entry(rule.id).or_default() += removed;
        }
    }

    // 3. Liens forcés : la paire n'est plus reliée que par eux.
    for c in corrections.iter().filter(|c| c.kind == CorrectionKind::Link) {
        let (Some(from), Some(source)) =
            (c.from.as_deref(), c.source.as_deref().and_then(static_source))
        else {
            continue;
        };
        if from == c.to || !known(from) || !known(&c.to) {
            continue;
        }
        edges.retain(|e| !same_pair(e, from, &c.to));
        edges.push(EdgeView {
            from: from.to_string(),
            to: c.to.clone(),
            source,
            origin: "manual",
            correction_id: Some(c.id),
            rule_id: None,
            from_port: None,
            to_port: None,
            traffic: None,
        });
    }

    // 4. Rattachements à la main : le plus récent par nœud. Le lien « par le
    //    pare-feu », simple placement par défaut, s'efface ; les autres
    //    restent, dessinés en maillage.
    let mut placed: BTreeMap<&str, &Correction> = BTreeMap::new();
    for c in corrections.iter().filter(|c| c.kind == CorrectionKind::Parent) {
        if let Some(from) = c.from.as_deref()
            && from != c.to
            && known(from)
            && known(&c.to)
        {
            placed.insert(c.to.as_str(), c);
        }
    }
    for (&child, &c) in &placed {
        let parent = c.from.as_deref().unwrap_or_default();
        edges.retain(|e| {
            !(same_pair(e, parent, child) || (e.to == child && e.source == "firewall"))
        });
        // Le parent d'alerte n'est dit que s'il est bien en place.
        let parent_target = info.get(parent).and_then(|i| i.target);
        let alerts = c.use_for_alerts
            && parent_target.is_some()
            && info.get(child).and_then(|i| i.parent_id) == parent_target;
        edges.push(EdgeView {
            from: parent.to_string(),
            to: child.to_string(),
            source: if alerts { "parent" } else { "manual" },
            origin: "manual",
            correction_id: Some(c.id),
            rule_id: None,
            from_port: None,
            to_port: None,
            traffic: None,
        });
    }

    // 5. Règles de rattachement acceptées, pour ce que rien ne place encore.
    let mut ordered: Vec<&String> = info.keys().collect();
    ordered.sort();
    let mut ruled: HashSet<&str> = HashSet::new();
    for rule in active(RuleKind::Parent) {
        let Some(parent) = rule.spec.parent.as_deref().filter(|p| known(p)) else { continue };
        for id in &ordered {
            let id = id.as_str();
            if placed.contains_key(id) || ruled.contains(id) {
                continue;
            }
            let Some(node) = info.get(id) else { continue };
            if !placeable(&rule.spec, id, node, parent, edges) {
                continue;
            }
            edges.retain(|e| !(same_pair(e, parent, id) || (e.to == id && e.source == "firewall")));
            edges.push(EdgeView {
                from: parent.to_string(),
                to: id.to_string(),
                source: "manual",
                origin: "rule",
                correction_id: None,
                rule_id: Some(rule.id),
                from_port: None,
                to_port: None,
                traffic: None,
            });
            ruled.insert(id);
            *matches.entry(rule.id).or_default() += 1;
        }
    }

    // 6. Site et regroupement corrigés.
    for c in corrections {
        let Some(node) = nodes.iter_mut().find(|n| n.id == c.to) else { continue };
        match c.kind {
            CorrectionKind::Site => {
                node.site =
                    c.value.as_deref().map(str::trim).filter(|v| !v.is_empty()).map(String::from);
                node.site_source = Some("manual");
            }
            CorrectionKind::Group => {
                node.group_name = c.value.as_deref().map(str::trim).unwrap_or_default().to_string();
                node.group_source = Some("manual");
            }
            _ => {}
        }
    }
    matches
}

// --------------------------------------------------------------- suggestions

/// Les règles que les corrections laissent deviner, sauf celles déjà
/// acceptées ou refusées. Triées par clé : le même état donne la même liste.
pub fn suggest(
    targets: &[TargetRow],
    nodes: &[NodeView],
    edges: &[EdgeView],
    corrections: &[Correction],
    rules: &[Rule],
) -> Vec<Suggestion> {
    let info = infos(targets, nodes);
    let taken: HashSet<String> = rules.iter().map(|r| r.spec.key()).collect();
    let mut out: BTreeMap<String, Suggestion> = BTreeMap::new();
    let mut offer = |spec: RuleSpec, ids: Vec<i64>, matches: usize| {
        let key = spec.key();
        if !taken.contains(&key) {
            out.insert(
                key.clone(),
                Suggestion { key, spec, evidence: ids.len(), corrections: ids, matches },
            );
        }
    };

    // Un lien interdit vers un équipement d'un sous-réseau : le même type de
    // lien, pour tout le sous-réseau.
    let mut forbidden: BTreeMap<(String, String), Vec<i64>> = BTreeMap::new();
    for c in corrections.iter().filter(|c| c.kind == CorrectionKind::Forbid) {
        let Some(source) = c.source.as_deref().filter(|s| GENERAL_FORBID.contains(s)) else {
            continue;
        };
        let Some(subnet) = info.get(&c.to).and_then(|i| i.subnet) else { continue };
        forbidden.entry((source.to_string(), subnet.to_string())).or_default().push(c.id);
    }
    for ((source, subnet), ids) in forbidden {
        let spec = RuleSpec {
            kind: RuleKind::Forbid,
            subnet: Some(subnet),
            address: None,
            target_kind: None,
            source: Some(source),
            parent: None,
        };
        let members = info.values().filter(|i| spec.covers(i)).count();
        if members < 2 {
            continue;
        }
        let matches = edges
            .iter()
            .filter(|e| {
                spec.source.as_deref() == Some(e.source)
                    && info.get(&e.to).is_some_and(|i| spec.covers(i))
            })
            .count();
        offer(spec, ids, matches);
    }

    // Rattachements : le plus récent par nœud, regroupés par parent et par
    // adresse, puis par parent et par sous-réseau.
    let mut latest: BTreeMap<&str, &Correction> = BTreeMap::new();
    for c in corrections.iter().filter(|c| c.kind == CorrectionKind::Parent && c.from.is_some()) {
        latest.insert(c.to.as_str(), c);
    }
    let mut by_address: BTreeMap<(String, String), Vec<&Correction>> = BTreeMap::new();
    let mut by_subnet: BTreeMap<(String, String), Vec<&Correction>> = BTreeMap::new();
    for (&child, &c) in &latest {
        let Some(node) = info.get(child).filter(|i| i.target.is_some()) else { continue };
        let parent = c.from.clone().unwrap_or_default();
        if let Some(address) = &node.address {
            by_address.entry((parent.clone(), address.clone())).or_default().push(c);
        }
        if let Some(subnet) = node.subnet {
            by_subnet.entry((parent, subnet.to_string())).or_default().push(c);
        }
    }
    // Un équipement du même périmètre rattaché ailleurs : rien à généraliser.
    let contradicted = |spec: &RuleSpec, parent: &str| {
        latest.iter().any(|(&child, c)| {
            c.from.as_deref() != Some(parent) && info.get(child).is_some_and(|i| spec.covers(i))
        })
    };
    // Ce que la règle placerait en plus de ce qui l'a fait naître.
    let reach = |spec: &RuleSpec, parent: &str| {
        info.iter()
            .filter(|(id, i)| {
                !latest.contains_key(id.as_str()) && placeable(spec, id, i, parent, edges)
            })
            .count()
    };
    let ids = |list: &[&Correction]| list.iter().map(|c| c.id).collect::<Vec<_>>();

    for ((parent, address), list) in &by_address {
        if list.len() < 2 {
            continue;
        }
        let kinds: BTreeSet<&str> =
            list.iter().filter_map(|c| info.get(&c.to)).map(|i| i.kind.as_str()).collect();
        let target_kind =
            if kinds.len() == 1 { kinds.first().map(|k| k.to_string()) } else { None };
        let spec = RuleSpec {
            kind: RuleKind::Parent,
            subnet: None,
            address: Some(address.clone()),
            target_kind,
            source: None,
            parent: Some(parent.clone()),
        };
        if contradicted(&spec, parent.as_str()) {
            continue;
        }
        let matches = reach(&spec, parent.as_str());
        offer(spec, ids(list.as_slice()), matches);
    }
    for ((parent, subnet), list) in &by_subnet {
        if list.len() < 2 {
            continue;
        }
        // Tous à la même adresse : la règle d'adresse, plus étroite, suffit.
        let addresses: BTreeSet<Option<&str>> =
            list.iter().map(|c| info.get(&c.to).and_then(|i| i.address.as_deref())).collect();
        if addresses.len() == 1 {
            continue;
        }
        let spec = RuleSpec {
            kind: RuleKind::Parent,
            subnet: Some(subnet.clone()),
            address: None,
            target_kind: None,
            source: None,
            parent: Some(parent.clone()),
        };
        if contradicted(&spec, parent.as_str()) {
            continue;
        }
        let matches = reach(&spec, parent.as_str());
        offer(spec, ids(list.as_slice()), matches);
    }
    out.into_values().collect()
}

// -------------------------------------------------------------------- API

async fn list_corrections(State(state): State<AppState>) -> ApiResult<Json<Vec<Correction>>> {
    Ok(Json(load_corrections(&state.pool).await?))
}

/// `GET /api/topology/rules` : avec ce que chaque règle touche aujourd'hui.
async fn list_rules(State(state): State<AppState>) -> ApiResult<Json<Vec<Rule>>> {
    Ok(Json(super::topology::compute(&state).await?.rules))
}

async fn list_suggestions(State(state): State<AppState>) -> ApiResult<Json<Vec<Suggestion>>> {
    Ok(Json(super::topology::compute(&state).await?.suggestions))
}

#[derive(Debug, Deserialize)]
pub struct CorrectionInput {
    pub kind: CorrectionKind,
    #[serde(default)]
    pub from: Option<String>,
    pub to: String,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    /// Rattachement : en faire aussi le parent d'alerte. Vrai par défaut.
    #[serde(default = "default_alerts")]
    pub use_for_alerts: bool,
}

fn default_alerts() -> bool {
    true
}

/// Réponse d'une création : la correction, et celles qu'elle remplace (même
/// nœud, même paire) pour qu'un « annuler » puisse les rétablir.
#[derive(Debug, Serialize)]
pub struct CreatedCorrection {
    pub correction: Correction,
    pub replaced: Vec<Correction>,
}

fn bad(why: impl Into<String>) -> ApiError {
    ApiError::BadRequest(why.into())
}

/// Vérifie qu'un nœud se lit et que son équipement existe ; renvoie celui-ci.
async fn check_node(tx: &mut Transaction<'_, Sqlite>, id: &str) -> ApiResult<TargetId> {
    let node = parse_node(id).ok_or_else(|| bad(format!("\"{id}\" is not a node of the map.")))?;
    let found: Option<TargetId> = sqlx::query_scalar("SELECT id FROM targets WHERE id = ?")
        .bind(node.host())
        .fetch_optional(&mut **tx)
        .await?;
    found.ok_or_else(|| bad(format!("No device behind the map node \"{id}\".")))
}

async fn parent_of(
    tx: &mut Transaction<'_, Sqlite>,
    id: TargetId,
) -> ApiResult<Option<Option<TargetId>>> {
    let parent: Option<Option<TargetId>> =
        sqlx::query_scalar("SELECT parent_id FROM targets WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?;
    Ok(parent)
}

async fn set_parent(
    tx: &mut Transaction<'_, Sqlite>,
    id: TargetId,
    parent: Option<TargetId>,
) -> ApiResult<()> {
    sqlx::query("UPDATE targets SET parent_id = ? WHERE id = ?")
        .bind(parent)
        .bind(id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Vrai quand `parent` dépend déjà de `child` (parents d'alerte) : le
/// rattachement fermerait une boucle.
async fn would_loop(
    tx: &mut Transaction<'_, Sqlite>,
    child: TargetId,
    parent: TargetId,
) -> ApiResult<bool> {
    let mut at = Some(parent);
    let mut seen = HashSet::new();
    while let Some(id) = at {
        if id == child {
            return Ok(true);
        }
        if !seen.insert(id) {
            return Ok(false);
        }
        at = parent_of(tx, id).await?.flatten();
    }
    Ok(false)
}

async fn fetch_correction(
    tx: &mut Transaction<'_, Sqlite>,
    id: i64,
) -> ApiResult<Option<Correction>> {
    let row = sqlx::query(CORRECTION_ONE).bind(id).fetch_optional(&mut **tx).await?;
    Ok(row.as_ref().map(correction_row).transpose()?)
}

/// Les corrections qu'une nouvelle remplace : même nœud pour un rattachement,
/// un site ou un regroupement ; même paire pour un lien interdit ou forcé.
async fn replaced_by(
    tx: &mut Transaction<'_, Sqlite>,
    kind: CorrectionKind,
    from: Option<&str>,
    to: &str,
    source: Option<&str>,
) -> ApiResult<Vec<Correction>> {
    let rows = sqlx::query(CORRECTIONS_ALL).fetch_all(&mut **tx).await?;
    let all = rows.iter().map(correction_row).collect::<Result<Vec<_>, _>>()?;
    let pair = |c: &Correction| match (c.from.as_deref(), from) {
        (Some(a), Some(b)) => (a == b && c.to == to) || (a == to && c.to == b),
        _ => false,
    };
    Ok(all
        .into_iter()
        .filter(|c| match kind {
            CorrectionKind::Parent | CorrectionKind::Site | CorrectionKind::Group => {
                c.kind == kind && c.to == to
            }
            CorrectionKind::Forbid => {
                pair(c)
                    && (c.kind == CorrectionKind::Link
                        || (c.kind == CorrectionKind::Forbid && c.source.as_deref() == source))
            }
            CorrectionKind::Link => {
                pair(c) && matches!(c.kind, CorrectionKind::Link | CorrectionKind::Forbid)
            }
        })
        .collect())
}

fn trimmed(value: Option<&str>) -> Option<String> {
    value.map(str::trim).filter(|v| !v.is_empty()).map(String::from)
}

/// `POST /api/topology/corrections`
async fn create_correction(
    State(state): State<AppState>,
    AdminIdentity(principal): AdminIdentity,
    Json(input): Json<CorrectionInput>,
) -> ApiResult<(StatusCode, Json<CreatedCorrection>)> {
    let author = principal.label();
    let mut tx = state.pool.begin().await?;
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM topology_corrections").fetch_one(&mut *tx).await?;
    if count >= MAX_CORRECTIONS {
        return Err(bad(format!("At most {MAX_CORRECTIONS} map corrections.")));
    }
    let to_target = check_node(&mut tx, &input.to).await?;
    let from = trimmed(input.from.as_deref());
    let from_target = match from.as_deref() {
        Some(node) => Some(check_node(&mut tx, node).await?),
        None => None,
    };
    let needs_both = matches!(
        input.kind,
        CorrectionKind::Forbid | CorrectionKind::Link | CorrectionKind::Parent
    );
    if needs_both && from.is_none() {
        return Err(bad("This correction needs both ends of the link."));
    }
    if needs_both && from.as_deref() == Some(input.to.as_str()) {
        return Err(bad("A node cannot be linked to itself."));
    }

    // Le type du lien, le site ou le regroupement, selon la correction.
    let source: Option<&'static str> = match input.kind {
        CorrectionKind::Forbid => {
            let kind = input.source.as_deref().and_then(static_source).ok_or_else(|| {
                bad("Say which link is wrong: its type (`source`) is missing or unknown.")
            })?;
            if kind == "service" || kind == "manual" {
                return Err(bad(
                    "This link is not deduced: remove the service, or the correction that drew it.",
                ));
            }
            Some(kind)
        }
        CorrectionKind::Link => Some(
            input
                .source
                .as_deref()
                .and_then(static_source)
                .filter(|s| LINK_SOURCES.contains(s))
                .ok_or_else(|| bad("Pick the type of link to draw."))?,
        ),
        _ => None,
    };
    let value = match input.kind {
        CorrectionKind::Site | CorrectionKind::Group => {
            let text = trimmed(input.value.as_deref()).unwrap_or_default();
            if text.chars().count() > MAX_VALUE_CHARS {
                return Err(bad(format!("At most {MAX_VALUE_CHARS} characters.")));
            }
            Some(text)
        }
        _ => None,
    };
    let replaced = replaced_by(&mut tx, input.kind, from.as_deref(), &input.to, source).await?;

    let mut use_for_alerts = false;
    let mut previous_parent = None;
    let child = target_node(&input.to);
    let up = from.as_deref().and_then(target_node);
    if input.kind == CorrectionKind::Forbid
        && source == Some("parent")
        && let (Some(child), Some(up)) = (child, up)
        && parent_of(&mut tx, child).await?.flatten() == Some(up)
    {
        // Un parent déclaré marqué faux n'est plus le parent d'alerte non
        // plus : la carte et les alertes disent la même chose.
        set_parent(&mut tx, child, None).await?;
        previous_parent = Some(up);
    }
    if input.kind == CorrectionKind::Parent {
        let service: Option<String> = match child {
            Some(id) => {
                sqlx::query_scalar("SELECT kind FROM targets WHERE id = ?")
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await?
            }
            None => None,
        };
        let before = replaced.iter().find(|c| c.use_for_alerts);
        if let (Some(child), Some(up)) = (child, up)
            && input.use_for_alerts
            && service.as_deref() != Some(SERVICE_KIND)
        {
            if would_loop(&mut tx, child, up).await? {
                return Err(bad(
                    "That device already depends on this one: attaching it would make a loop.",
                ));
            }
            let current = parent_of(&mut tx, child).await?.flatten();
            // Une correction remplacée avait déjà changé le parent : celui
            // d'avant elle est le vrai point de départ.
            previous_parent = before.map_or(current, |c| c.previous_parent);
            set_parent(&mut tx, child, Some(up)).await?;
            use_for_alerts = true;
        } else if let (Some(child), Some(old)) = (child, before) {
            // Le rattachement remplacé valait pour les alertes, pas celui-ci :
            // le parent d'avant revient.
            let old_up = old.from.as_deref().and_then(target_node);
            if old_up.is_some() && parent_of(&mut tx, child).await?.flatten() == old_up {
                let back = existing(&mut tx, old.previous_parent).await?;
                set_parent(&mut tx, child, back).await?;
            }
        }
    }

    for old in &replaced {
        sqlx::query("DELETE FROM topology_corrections WHERE id = ?")
            .bind(old.id)
            .execute(&mut *tx)
            .await?;
    }
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO topology_corrections (kind, from_node, to_node, from_target, to_target,
             source, value, use_for_alerts, previous_parent, author)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(input.kind.as_str())
    .bind(&from)
    .bind(&input.to)
    .bind(from_target)
    .bind(to_target)
    .bind(source)
    .bind(&value)
    .bind(use_for_alerts)
    .bind(previous_parent)
    .bind(&author)
    .fetch_one(&mut *tx)
    .await?;
    let correction = fetch_correction(&mut tx, id)
        .await?
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("correction {id} introuvable")))?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(CreatedCorrection { correction, replaced })))
}

/// `previous` s'il existe encore.
async fn existing(
    tx: &mut Transaction<'_, Sqlite>,
    previous: Option<TargetId>,
) -> ApiResult<Option<TargetId>> {
    let Some(id) = previous else { return Ok(None) };
    Ok(parent_of(tx, id).await?.map(|_| id))
}

/// `DELETE /api/topology/corrections/{id}` — un parent d'alerte que la
/// correction avait changé revient à ce qu'il était, s'il n'a pas bougé depuis.
async fn delete_correction(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    let mut tx = state.pool.begin().await?;
    let correction = fetch_correction(&mut tx, id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Map correction {id} not found.")))?;
    let child = target_node(&correction.to);
    let up = correction.from.as_deref().and_then(target_node);
    if let (Some(child), Some(up)) = (child, up) {
        let current = parent_of(&mut tx, child).await?.flatten();
        match correction.kind {
            CorrectionKind::Parent if correction.use_for_alerts && current == Some(up) => {
                let back = existing(&mut tx, correction.previous_parent).await?;
                set_parent(&mut tx, child, back).await?;
            }
            CorrectionKind::Forbid
                if correction.previous_parent == Some(up) && current.is_none() =>
            {
                let back = existing(&mut tx, Some(up)).await?;
                set_parent(&mut tx, child, back).await?;
            }
            _ => {}
        }
    }
    sqlx::query("DELETE FROM topology_corrections WHERE id = ?").bind(id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct RuleInput {
    pub status: RuleStatus,
    #[serde(flatten)]
    pub spec: RuleSpec,
    #[serde(default)]
    pub evidence: i64,
}

/// Normalise et vérifie une règle ; renvoie aussi l'équipement de son parent.
async fn check_rule(
    tx: &mut Transaction<'_, Sqlite>,
    spec: RuleSpec,
) -> ApiResult<(RuleSpec, Option<TargetId>)> {
    let subnet = match trimmed(spec.subnet.as_deref()) {
        Some(text) => Some(
            Cidr::parse(&text)
                .ok_or_else(|| bad(format!("\"{text}\" is not an IPv4 subnet (a.b.c.d/nn).")))?
                .to_string(),
        ),
        None => None,
    };
    let address = trimmed(spec.address.as_deref()).and_then(|a| host_key(&a));
    if subnet.is_none() && address.is_none() {
        return Err(bad("A rule applies to a subnet or to an address."));
    }
    let target_kind = trimmed(spec.target_kind.as_deref());
    if target_kind.as_deref().is_some_and(|k| k.len() > MAX_KIND_CHARS) {
        return Err(bad("Unknown device type."));
    }
    let (source, parent, parent_target) = match spec.kind {
        RuleKind::Forbid => {
            let source = spec
                .source
                .as_deref()
                .filter(|s| GENERAL_FORBID.contains(s))
                .ok_or_else(|| bad("Pick the type of link the rule forbids."))?;
            (Some(source.to_string()), None, None)
        }
        RuleKind::Parent => {
            let parent = trimmed(spec.parent.as_deref())
                .ok_or_else(|| bad("Pick the node the devices depend on."))?;
            let target = check_node(tx, &parent).await?;
            (None, Some(parent), Some(target))
        }
    };
    Ok((RuleSpec { kind: spec.kind, subnet, address, target_kind, source, parent }, parent_target))
}

/// `POST /api/topology/rules` — accepte (`active`) ou refuse (`dismissed`)
/// une règle ; la même règle (même clé) change seulement d'état.
async fn save_rule(
    State(state): State<AppState>,
    AdminIdentity(principal): AdminIdentity,
    Json(input): Json<RuleInput>,
) -> ApiResult<(StatusCode, Json<Rule>)> {
    let mut tx = state.pool.begin().await?;
    let (spec, parent_target) = check_rule(&mut tx, input.spec).await?;
    let key = spec.key();
    let known: Option<i64> = sqlx::query_scalar("SELECT id FROM topology_rules WHERE key = ?")
        .bind(&key)
        .fetch_optional(&mut *tx)
        .await?;
    if known.is_none() {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM topology_rules").fetch_one(&mut *tx).await?;
        if count >= MAX_RULES {
            return Err(bad(format!("At most {MAX_RULES} map rules.")));
        }
    }
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO topology_rules (key, kind, status, subnet, address, target_kind, source,
             parent_node, parent_target, evidence, author)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET status = excluded.status, evidence = excluded.evidence,
             author = excluded.author, created_at = datetime('now')
         RETURNING id",
    )
    .bind(&key)
    .bind(spec.kind.as_str())
    .bind(input.status.as_str())
    .bind(&spec.subnet)
    .bind(&spec.address)
    .bind(&spec.target_kind)
    .bind(&spec.source)
    .bind(&spec.parent)
    .bind(parent_target)
    .bind(input.evidence.max(0))
    .bind(principal.label())
    .fetch_one(&mut *tx)
    .await?;
    let row = sqlx::query(RULE_ONE).bind(id).fetch_one(&mut *tx).await?;
    let rule = rule_row(&row)?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(rule)))
}

/// `DELETE /api/topology/rules/{id}` — la règle cesse de s'appliquer ; une
/// règle refusée retirée peut de nouveau être proposée.
async fn delete_rule(State(state): State<AppState>, Path(id): Path<i64>) -> ApiResult<StatusCode> {
    let done = sqlx::query("DELETE FROM topology_rules WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if done.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!("Map rule {id} not found.")));
    }
    Ok(StatusCode::NO_CONTENT)
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

    fn node(t: &TargetRow, subnet: Option<&str>) -> NodeView {
        NodeView {
            id: format!("t{}", t.id),
            target_id: Some(t.id),
            name: t.name.clone(),
            kind: t.kind.clone(),
            group_name: String::new(),
            group_source: None,
            site: None,
            site_source: None,
            host_id: None,
            subnet: subnet.map(String::from),
            vlan: None,
            vlan_name: None,
            running: None,
            ips: Vec::new(),
        }
    }

    fn edge(from: &str, to: &str, source: &'static str) -> EdgeView {
        EdgeView::deduced(from.into(), to.into(), source)
    }

    fn correction(id: i64, kind: CorrectionKind, from: Option<&str>, to: &str) -> Correction {
        Correction {
            id,
            kind,
            from: from.map(String::from),
            to: to.into(),
            source: None,
            value: None,
            use_for_alerts: false,
            author: "admin".into(),
            created_at: "2026-10-25 10:00:00".into(),
            previous_parent: None,
        }
    }

    fn forbid(id: i64, from: &str, to: &str, source: &str) -> Correction {
        Correction {
            source: Some(source.into()),
            ..correction(id, CorrectionKind::Forbid, Some(from), to)
        }
    }

    fn reparent(id: i64, parent: &str, child: &str) -> Correction {
        correction(id, CorrectionKind::Parent, Some(parent), child)
    }

    fn rule(id: i64, spec: RuleSpec) -> Rule {
        Rule {
            id,
            status: RuleStatus::Active,
            spec,
            evidence: 2,
            author: "admin".into(),
            created_at: "2026-10-25 10:00:00".into(),
            matches: 0,
        }
    }

    fn triples(edges: &[EdgeView]) -> Vec<(&str, &str, &str, &str)> {
        let mut out: Vec<_> =
            edges.iter().map(|e| (e.from.as_str(), e.to.as_str(), e.source, e.origin)).collect();
        out.sort();
        out
    }

    /// Un pare-feu, un commutateur et trois équipements du même /24, tous
    /// « par le pare-feu » d'après la déduction.
    fn lan() -> (Vec<TargetRow>, Vec<NodeView>, Vec<EdgeView>) {
        let targets = vec![
            row(1, "fw", "10.0.5.1", "opnsense"),
            row(2, "switch-core", "10.0.5.2", "snmp"),
            row(3, "nas", "10.0.5.10", "synology"),
            row(4, "printer", "10.0.5.11", "snmp"),
            row(5, "cam", "10.0.5.12", "ping"),
        ];
        let nodes = targets.iter().map(|t| node(t, Some("10.0.5.0/24"))).collect();
        let edges = (2..=5).map(|i| edge("t1", &format!("t{i}"), "firewall")).collect();
        (targets, nodes, edges)
    }

    #[test]
    fn un_identifiant_de_noeud_se_lit_et_voyage() {
        assert_eq!(parse_node("t12"), Some(NodeRef::Target(12)));
        assert_eq!(parse_node("g3:web:01"), Some(NodeRef::Guest(3, "web:01")));
        assert_eq!(parse_node("n3:pve1"), Some(NodeRef::PveNode(3, "pve1")));
        for wrong in ["", "t", "t0", "tx", "g3", "g3:", "x3:a", "gx:a"] {
            assert_eq!(parse_node(wrong), None, "{wrong}");
        }
        assert_eq!(node_parts("g3:web"), Some((3, Some("g:web".into()))));
        assert_eq!(node_from_parts(9, Some("g:web")).as_deref(), Some("g9:web"));
        assert_eq!(node_from_parts(9, None).as_deref(), Some("t9"));
        assert_eq!(node_from_parts(9, Some("z:web")), None);
    }

    #[test]
    fn un_lien_interdit_disparait_dans_les_deux_sens() {
        let (targets, mut nodes, mut edges) = lan();
        edges.push(edge("t3", "t2", "lldp"));
        let corrections = [forbid(1, "t1", "t3", "firewall"), forbid(2, "t2", "t3", "lldp")];
        apply(&targets, &mut nodes, &mut edges, &corrections, &[]);
        assert_eq!(
            triples(&edges),
            vec![
                ("t1", "t2", "firewall", "deduced"),
                ("t1", "t4", "firewall", "deduced"),
                ("t1", "t5", "firewall", "deduced"),
            ]
        );
    }

    #[test]
    fn un_rattachement_remplace_le_pare_feu_sans_doublon() {
        let (mut targets, mut nodes, mut edges) = lan();
        // La déduction avait aussi vu le NAS en LLDP derrière le commutateur.
        edges.push(edge("t2", "t3", "lldp"));
        targets[2].parent_id = Some(2);
        let mut alerts = reparent(7, "t2", "t3");
        alerts.use_for_alerts = true;
        let corrections = [reparent(6, "t2", "t4"), alerts];
        apply(&targets, &mut nodes, &mut edges, &corrections, &[]);
        assert_eq!(
            triples(&edges),
            vec![
                ("t1", "t2", "firewall", "deduced"),
                ("t1", "t5", "firewall", "deduced"),
                ("t2", "t3", "parent", "manual"),
                ("t2", "t4", "manual", "manual"),
            ]
        );
        let pairs: HashSet<(String, String)> =
            edges.iter().map(|e| (e.from.clone(), e.to.clone())).collect();
        assert_eq!(pairs.len(), edges.len(), "une paire, un lien");
        assert_eq!(nodes.len(), 5, "aucun nœud créé");
    }

    #[test]
    fn les_corrections_passent_avant_les_regles_et_les_regles_avant_la_deduction() {
        let (targets, mut nodes, mut edges) = lan();
        let switch = rule(
            1,
            RuleSpec {
                kind: RuleKind::Parent,
                subnet: Some("10.0.5.0/24".into()),
                address: None,
                target_kind: None,
                source: None,
                parent: Some("t2".into()),
            },
        );
        // Le NAS est rattaché ailleurs à la main : la règle ne le touche pas.
        let corrections = [reparent(1, "t5", "t3")];
        let applied = apply(&targets, &mut nodes, &mut edges, &corrections, &[switch]);
        assert_eq!(
            triples(&edges),
            vec![
                ("t1", "t2", "firewall", "deduced"),
                ("t2", "t4", "manual", "rule"),
                ("t2", "t5", "manual", "rule"),
                ("t5", "t3", "manual", "manual"),
            ],
            "le commutateur et le pare-feu restent hors de la règle, le NAS suit sa correction"
        );
        assert_eq!(applied.get(&1), Some(&2));
    }

    #[test]
    fn une_regle_ne_ferme_pas_de_boucle_ni_ne_deplace_un_invite() {
        let mut targets = lan().0;
        targets.push(row(6, "pve", "10.0.5.20", "proxmox"));
        targets.push(row(7, "vm-web", "10.0.5.30", "agent"));
        let mut nodes: Vec<NodeView> =
            targets.iter().map(|t| node(t, Some("10.0.5.0/24"))).collect();
        let mut edges = vec![
            edge("t1", "t2", "firewall"),
            edge("t1", "t6", "firewall"),
            edge("t6", "t7", "hypervisor"),
            // Le commutateur pend du NAS (lien forcé) : le NAS ne peut pas
            // passer sous le commutateur.
            edge("t3", "t2", "lldp"),
        ];
        let switch = rule(
            1,
            RuleSpec {
                kind: RuleKind::Parent,
                subnet: Some("10.0.5.0/24".into()),
                address: None,
                target_kind: None,
                source: None,
                parent: Some("t2".into()),
            },
        );
        apply(&targets, &mut nodes, &mut edges, &[], &[switch]);
        let placed: Vec<&str> =
            edges.iter().filter(|e| e.origin == "rule").map(|e| e.to.as_str()).collect();
        assert_eq!(placed, vec!["t4", "t5", "t6"], "ni le NAS (boucle) ni la VM (invitée)");
    }

    #[test]
    fn une_regle_dinterdiction_vaut_pour_tout_le_sous_reseau() {
        let (mut targets, mut nodes, mut edges) = lan();
        targets.push(row(9, "web", "192.168.1.9", "http"));
        nodes.push(node(&targets[5], Some("192.168.1.0/24")));
        edges.push(edge("t1", "t9", "firewall"));
        let no_fw = rule(
            4,
            RuleSpec {
                kind: RuleKind::Forbid,
                subnet: Some("10.0.5.0/24".into()),
                address: None,
                target_kind: None,
                source: Some("firewall".into()),
                parent: None,
            },
        );
        let applied = apply(&targets, &mut nodes, &mut edges, &[], &[no_fw]);
        assert_eq!(triples(&edges), vec![("t1", "t9", "firewall", "deduced")]);
        assert_eq!(applied.get(&4), Some(&4));
    }

    #[test]
    fn le_site_et_le_regroupement_se_corrigent() {
        let (targets, mut nodes, mut edges) = lan();
        let mut site = correction(1, CorrectionKind::Site, None, "t3");
        site.value = Some(" Annexe ".into());
        let mut main = correction(2, CorrectionKind::Site, None, "t4");
        main.value = Some(String::new());
        let mut group = correction(3, CorrectionKind::Group, None, "t5");
        group.value = Some("Caméras".into());
        nodes[3].site = Some("Ailleurs".into());
        apply(&targets, &mut nodes, &mut edges, &[site, main, group], &[]);
        assert_eq!(nodes[2].site.as_deref(), Some("Annexe"));
        assert_eq!(nodes[2].site_source, Some("manual"));
        assert_eq!(nodes[3].site, None, "vide : le site principal");
        assert_eq!(nodes[4].group_name, "Caméras");
        assert_eq!(nodes[4].group_source, Some("manual"));
    }

    #[test]
    fn deux_rattachements_dun_sous_reseau_proposent_une_regle() {
        let (targets, mut nodes, mut edges) = lan();
        let corrections = [reparent(1, "t2", "t3"), reparent(2, "t2", "t4")];
        apply(&targets, &mut nodes, &mut edges, &corrections, &[]);
        let found = suggest(&targets, &nodes, &edges, &corrections, &[]);
        assert_eq!(found.len(), 1, "{found:#?}");
        let s = &found[0];
        assert_eq!(s.spec.kind, RuleKind::Parent);
        assert_eq!(s.spec.subnet.as_deref(), Some("10.0.5.0/24"));
        assert_eq!(s.spec.parent.as_deref(), Some("t2"));
        assert_eq!(s.corrections, vec![1, 2]);
        assert_eq!(s.matches, 1, "la caméra suivrait ; ni le pare-feu ni le commutateur");

        // Acceptée, elle ne revient pas ; refusée non plus.
        let mut taken = rule(1, s.spec.clone());
        assert!(suggest(&targets, &nodes, &edges, &corrections, &[taken.clone()]).is_empty());
        taken.status = RuleStatus::Dismissed;
        assert!(suggest(&targets, &nodes, &edges, &corrections, &[taken]).is_empty());
    }

    #[test]
    fn une_seule_correction_ou_une_contradiction_ne_proposent_rien() {
        let (targets, nodes, edges) = lan();
        let one = [reparent(1, "t2", "t3")];
        assert!(suggest(&targets, &nodes, &edges, &one, &[]).is_empty());
        let split = [reparent(1, "t2", "t3"), reparent(2, "t2", "t4"), reparent(3, "t5", "t1")];
        assert!(
            suggest(&targets, &nodes, &edges, &split, &[]).is_empty(),
            "le pare-feu du même réseau est rattaché ailleurs"
        );
    }

    #[test]
    fn des_moniteurs_a_la_meme_adresse_proposent_une_regle_dadresse() {
        let mut targets = vec![
            row(1, "vm-web", "10.0.0.30", "agent"),
            row(2, "site", "http://10.0.0.40/", "http"),
            row(3, "api", "https://10.0.0.40:8443/health", "http"),
            row(4, "new", "http://10.0.0.40:9000/", "http"),
            row(5, "ping", "10.0.0.40", "ping"),
        ];
        targets[0].group_name = "VMs".into();
        let nodes: Vec<NodeView> = targets.iter().map(|t| node(t, Some("10.0.0.0/24"))).collect();
        let edges = Vec::new();
        let corrections = [reparent(1, "t1", "t2"), reparent(2, "t1", "t3")];
        let found = suggest(&targets, &nodes, &edges, &corrections, &[]);
        assert_eq!(found.len(), 1, "l'adresse suffit, pas de règle de sous-réseau : {found:#?}");
        assert_eq!(found[0].spec.address.as_deref(), Some("10.0.0.40"));
        assert_eq!(found[0].spec.target_kind.as_deref(), Some("http"));
        assert_eq!(found[0].matches, 1, "le nouveau moniteur HTTP, pas le ping");
    }

    #[test]
    fn un_lien_interdit_propose_dinterdire_le_sous_reseau() {
        let (targets, mut nodes, mut edges) = lan();
        let corrections = [forbid(1, "t1", "t3", "firewall")];
        apply(&targets, &mut nodes, &mut edges, &corrections, &[]);
        let found = suggest(&targets, &nodes, &edges, &corrections, &[]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].spec.kind, RuleKind::Forbid);
        assert_eq!(found[0].spec.source.as_deref(), Some("firewall"));
        assert_eq!(found[0].spec.subnet.as_deref(), Some("10.0.5.0/24"));
        assert_eq!(found[0].matches, 3);
        // Un parent déclaré faux ne se généralise pas.
        let declared = [forbid(1, "t2", "t3", "parent")];
        assert!(suggest(&targets, &nodes, &edges, &declared, &[]).is_empty());
    }

    #[test]
    fn la_meme_entree_donne_la_meme_carte() {
        let (targets, nodes, edges) = lan();
        let corrections = [reparent(1, "t2", "t3"), reparent(2, "t2", "t4")];
        let run = || {
            let (mut n, mut e) = (nodes.clone(), edges.clone());
            apply(&targets, &mut n, &mut e, &corrections, &[]);
            (triples(&e).iter().map(|t| format!("{t:?}")).collect::<Vec<_>>(), n)
        };
        assert_eq!(run(), run());
    }
}
