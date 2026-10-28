//! Weathermap : le trafic des liens de la carte de topologie.
//!
//! Rien de nouveau n'est collecté. Un lien `lldp` ou `l2` connaît le port de
//! son commutateur ; les compteurs `if_octets_*`, `if_errors_*`,
//! `if_discards_*` et la vitesse `if_speed_bps` de ce port sont déjà dans
//! VictoriaMetrics (profil IF-MIB, agent). Le serveur en tire, par lien, un
//! débit entrant et sortant, une utilisation (débit / vitesse) et un taux
//! d'erreurs, que `GET /api/topology` joint à l'arête (`traffic`).
//!
//! Le côté mesuré est celui du commutateur qui a un port connu et des
//! compteurs : `from` d'abord, `to` ensuite. Plusieurs ports d'un même lien
//! (agrégat) s'additionnent. Les noms de ports viennent des tables LLDP : un
//! nom qui ne correspond à aucun `ifname` ne donne simplement pas de trafic.
//!
//! `GET /api/targets/{id}/port-traffic?ports=Gi1,Gi2` donne la courbe de la
//! dernière heure de ces ports (infobulle du lien). Le contrôle d'accès des
//! comptes restreints est celui de `/targets/{id}/…`.

use std::collections::{BTreeMap, HashMap};

use axum::Json;
use axum::extract::{Path, Query, State};
use dumbmonit_proto::TargetId;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::api::topology::EdgeView;
use crate::api::{ApiError, ApiResult};
use crate::state::AppState;
use crate::tsdb::{InstantSeries, METRIC_PREFIX};

/// Fenêtre du taux : plusieurs collectes, pour lisser sans masquer un pic.
const RATE_WINDOW: &str = "5m";
/// La vitesse d'un port change rarement : fenêtre large, comme `WINDOW` de la carte.
const SPEED_WINDOW: &str = "30m";
/// Ports d'un même lien lus au plus (un agrégat en a rarement plus).
const MAX_PORTS: usize = 16;
/// Courbe de l'infobulle : une heure, un point par minute.
const SPARK_SECS: i64 = 3600;
const SPARK_STEP: u64 = 60;

/// Le trafic d'un lien, mesuré sur l'un de ses ports.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EdgeTraffic {
    /// Nœud (`t<id>`) dont les compteurs ont servi : `from` ou `to`.
    pub measured_on: String,
    /// Ports mesurés, tels que l'équipement les nomme (`ifname`).
    pub ports: Vec<String>,
    /// Débit reçu / émis par `measured_on` sur ces ports, en bit/s.
    pub bps_in: f64,
    pub bps_out: f64,
    /// Vitesse de négociation (somme des ports), en bit/s ; absente si un port
    /// ne la donne pas.
    pub speed_bps: Option<f64>,
    /// Débit / vitesse, de 0 à 1 ; absent sans vitesse.
    pub utilization_in: Option<f64>,
    pub utilization_out: Option<f64>,
    /// Erreurs et rejets par seconde, entrée et sortie confondues.
    pub errors_per_sec: f64,
}

/// Ce que les mesures disent d'un port.
#[derive(Debug, Clone, Default, PartialEq)]
struct PortStats {
    /// Nom d'origine (`ifname`), casse comprise.
    name: String,
    bps_in: f64,
    bps_out: f64,
    errors_per_sec: f64,
    speed_bps: Option<f64>,
}

/// Les ports de tous les équipements, par (cible, `ifname` en minuscules).
#[derive(Debug, Default)]
pub struct TrafficData {
    ports: HashMap<(TargetId, String), PortStats>,
}

fn rates_query() -> String {
    format!(
        "rate({{__name__=~\"{METRIC_PREFIX}if_(octets|errors|discards)_(in|out)\"}}[{RATE_WINDOW}]) \
         keep_metric_names"
    )
}

fn speed_query() -> String {
    format!("last_over_time({METRIC_PREFIX}if_speed_bps[{SPEED_WINDOW}])")
}

impl TrafficData {
    /// Lit les taux (octets, erreurs, rejets) et les vitesses ; les séries d'une
    /// cible inconnue, sans `ifname` ou à valeur invalide sont ignorées.
    pub fn parse(rates: &[InstantSeries], speeds: &[InstantSeries], known: &[TargetId]) -> Self {
        let mut data = Self::default();
        for s in rates {
            let Some(value) = value_of(s) else { continue };
            let name = s.metric.get("__name__").map(String::as_str).unwrap_or_default();
            let name = name.strip_prefix(METRIC_PREFIX).unwrap_or(name);
            if !matches!(
                name,
                "if_octets_in"
                    | "if_octets_out"
                    | "if_errors_in"
                    | "if_errors_out"
                    | "if_discards_in"
                    | "if_discards_out"
            ) {
                continue;
            }
            let Some(stats) = data.port(s, known) else { continue };
            match name {
                // Octets par seconde -> bits par seconde.
                "if_octets_in" => stats.bps_in = value * 8.0,
                "if_octets_out" => stats.bps_out = value * 8.0,
                _ => stats.errors_per_sec += value,
            }
        }
        for s in speeds {
            let Some(value) = value_of(s).filter(|v| *v > 0.0) else { continue };
            if let Some(stats) = data.port(s, known) {
                stats.speed_bps = Some(value);
            }
        }
        data
    }

    /// La fiche du port que désigne la série, créée au besoin.
    fn port(&mut self, s: &InstantSeries, known: &[TargetId]) -> Option<&mut PortStats> {
        let target: TargetId = s.metric.get("target")?.parse().ok()?;
        if !known.contains(&target) {
            return None;
        }
        let name = s.metric.get("ifname")?.trim();
        if name.is_empty() {
            return None;
        }
        let stats = self.ports.entry((target, name.to_lowercase())).or_default();
        stats.name = name.to_string();
        Some(stats)
    }

    /// Le trafic d'un lien sur le port `text` de `target` (plusieurs ports
    /// séparés par des virgules : somme), ou `None` si aucun n'a de compteurs.
    fn measure(&self, node: &str, target: TargetId, text: &str) -> Option<EdgeTraffic> {
        let found: Vec<&PortStats> = port_names(text)
            .iter()
            .filter_map(|name| self.ports.get(&(target, name.to_lowercase())))
            .collect();
        if found.is_empty() {
            return None;
        }
        let bps_in: f64 = found.iter().map(|p| p.bps_in).sum();
        let bps_out: f64 = found.iter().map(|p| p.bps_out).sum();
        let speed_bps = found
            .iter()
            .map(|p| p.speed_bps)
            .sum::<Option<f64>>()
            .filter(|speed| *speed > 0.0);
        let ratio = |bps: f64| speed_bps.map(|speed| (bps / speed).clamp(0.0, 1.0));
        Some(EdgeTraffic {
            measured_on: node.to_string(),
            ports: found.iter().map(|p| p.name.clone()).collect(),
            bps_in,
            bps_out,
            speed_bps,
            utilization_in: ratio(bps_in),
            utilization_out: ratio(bps_out),
            errors_per_sec: found.iter().map(|p| p.errors_per_sec).sum(),
        })
    }
}

fn value_of(s: &InstantSeries) -> Option<f64> {
    s.value.1.parse::<f64>().ok().filter(|v| v.is_finite() && *v >= 0.0)
}

/// Les noms de ports d'un texte de lien (`Gi1, Gi2 +3` : le `+3` des ports
/// non listés est écarté).
fn port_names(text: &str) -> Vec<String> {
    text.split(',')
        .map(|part| {
            let part = part.trim();
            let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
            match part.rsplit_once(" +") {
                Some((head, more)) if digits(more) => head.trim(),
                _ => part,
            }
        })
        .filter(|name| !name.is_empty())
        .map(String::from)
        .take(MAX_PORTS)
        .collect()
}

/// Le numéro de cible d'un nœud `t<id>` ; un invité ou un nœud de cluster
/// n'a pas de compteurs d'interface.
fn node_target(node: &str) -> Option<TargetId> {
    node.strip_prefix('t')?.parse().ok()
}

/// Lit les mesures de ports. Une panne de VictoriaMetrics laisse les liens
/// sans trafic : le reste de la carte reste exact.
pub async fn read(state: &AppState, known: &[TargetId]) -> TrafficData {
    let (rates, speeds) =
        tokio::join!(state.victoria.query(&rates_query()), state.victoria.query(&speed_query()));
    match (rates, speeds) {
        (Ok(rates), Ok(speeds)) => TrafficData::parse(&rates, &speeds, known),
        (rates, speeds) => {
            if let Some(error) = rates.as_ref().err().or(speeds.as_ref().err()) {
                warn!(%error, "topologie : trafic des ports illisible, liens sans débit");
            }
            TrafficData::default()
        }
    }
}

/// Pose le trafic sur chaque lien dont un port connu a des compteurs : le côté
/// `from` d'abord, sinon le côté `to`.
pub fn apply(edges: &mut [EdgeView], data: &TrafficData) {
    for edge in edges {
        let found = [(&edge.from, &edge.from_port), (&edge.to, &edge.to_port)]
            .into_iter()
            .find_map(|(node, port)| {
                let target = node_target(node)?;
                data.measure(node, target, port.as_deref()?)
            });
        edge.traffic = found;
    }
}

#[derive(Debug, Deserialize)]
pub struct PortTrafficQuery {
    /// Ports séparés par des virgules.
    pub ports: String,
}

/// Un point de la courbe : `t` en secondes Unix, débits en bit/s.
#[derive(Debug, Serialize, PartialEq)]
pub struct TrafficPoint {
    pub t: i64,
    pub bps_in: f64,
    pub bps_out: f64,
}

#[derive(Debug, Serialize)]
pub struct PortTraffic {
    pub step_secs: u64,
    pub points: Vec<TrafficPoint>,
}

/// Échappe un texte pour un littéral de requête MetricsQL entre guillemets.
fn quoted(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Somme des débits (bit/s) de ces ports sur une cible, pour la métrique d'octets.
fn spark_query(metric: &str, target: TargetId, ports: &[String]) -> String {
    let regex = ports.iter().map(|p| regex::escape(p)).collect::<Vec<_>>().join("|");
    format!(
        "sum(rate({METRIC_PREFIX}{metric}{{target=\"{target}\",ifname=~\"{}\"}}[{RATE_WINDOW}])) * 8",
        quoted(&regex)
    )
}

/// `GET /api/targets/{id}/port-traffic?ports=Gi1,Gi2` : débit entrant et
/// sortant de ces ports sur la dernière heure.
pub async fn port_traffic(
    State(state): State<AppState>,
    Path(id): Path<TargetId>,
    Query(query): Query<PortTrafficQuery>,
) -> ApiResult<Json<PortTraffic>> {
    let ports = port_names(&query.ports);
    if ports.is_empty() {
        return Err(ApiError::BadRequest("Give at least one port in `ports`.".into()));
    }
    let end_ms = chrono::Utc::now().timestamp_millis();
    let start_ms = end_ms - SPARK_SECS * 1000;
    let (inbound, outbound) = tokio::join!(
        state.victoria.query_range(
            &spark_query("if_octets_in", id, &ports),
            start_ms,
            end_ms,
            SPARK_STEP
        ),
        state.victoria.query_range(
            &spark_query("if_octets_out", id, &ports),
            start_ms,
            end_ms,
            SPARK_STEP
        ),
    );
    let unavailable = |error: anyhow::Error| {
        warn!(%error, "trafic des ports illisible");
        ApiError::BadGateway("The metrics store did not answer; try again shortly.".into())
    };
    let (inbound, outbound) = (inbound.map_err(unavailable)?, outbound.map_err(unavailable)?);
    Ok(Json(PortTraffic { step_secs: SPARK_STEP, points: merge_points(&inbound, &outbound) }))
}

/// Joint les deux séries par horodatage.
fn merge_points(
    inbound: &[crate::tsdb::RangeSeries],
    outbound: &[crate::tsdb::RangeSeries],
) -> Vec<TrafficPoint> {
    let mut by_time: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
    let mut read = |series: &[crate::tsdb::RangeSeries], inbound: bool| {
        for (ts, value) in series.iter().flat_map(|s| s.values.iter()) {
            let Some(value) = value.parse::<f64>().ok().filter(|v| v.is_finite()) else { continue };
            let slot = by_time.entry(*ts as i64).or_default();
            if inbound {
                slot.0 = value;
            } else {
                slot.1 = value;
            }
        }
    };
    read(inbound, true);
    read(outbound, false);
    by_time.into_iter().map(|(t, (bps_in, bps_out))| TrafficPoint { t, bps_in, bps_out }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(name: &str, target: TargetId, ifname: &str, value: f64) -> InstantSeries {
        InstantSeries {
            metric: [
                ("__name__".to_string(), format!("{METRIC_PREFIX}{name}")),
                ("target".to_string(), target.to_string()),
                ("ifname".to_string(), ifname.to_string()),
            ]
            .into_iter()
            .collect(),
            value: (0.0, value.to_string()),
        }
    }

    fn edge(from: &str, to: &str, from_port: Option<&str>, to_port: Option<&str>) -> EdgeView {
        let mut edge = EdgeView::deduced(from.into(), to.into(), "l2");
        edge.from_port = from_port.map(String::from);
        edge.to_port = to_port.map(String::from);
        edge
    }

    fn data() -> TrafficData {
        let rates = [
            // 12,5 Mo/s reçus = 100 Mb/s ; 1,25 Mo/s émis = 10 Mb/s.
            series("if_octets_in", 1, "Gi24", 12_500_000.0),
            series("if_octets_out", 1, "Gi24", 1_250_000.0),
            series("if_errors_in", 1, "Gi24", 0.5),
            series("if_discards_out", 1, "Gi24", 0.25),
            series("if_octets_in", 1, "Gi23", 125_000_000.0),
            series("if_octets_in", 2, "Gi1", 1_000.0),
            // Cible inconnue et valeur illisible : ignorées.
            series("if_octets_in", 99, "Gi1", 1.0),
            series("if_octets_out", 2, "Gi1", f64::NAN),
        ];
        let speeds = [
            series("if_speed_bps", 1, "Gi24", 1e9),
            series("if_speed_bps", 1, "Gi23", 1e9),
        ];
        TrafficData::parse(&rates, &speeds, &[1, 2])
    }

    #[test]
    fn a_port_gives_rates_in_bits_and_utilization() {
        let mut edges = vec![edge("t1", "t5", Some("Gi24"), None)];
        apply(&mut edges, &data());
        let traffic = edges[0].traffic.as_ref().expect("traffic");
        assert_eq!(traffic.measured_on, "t1");
        assert_eq!(traffic.ports, vec!["Gi24".to_string()]);
        assert_eq!(traffic.bps_in, 100_000_000.0);
        assert_eq!(traffic.bps_out, 10_000_000.0);
        assert_eq!(traffic.speed_bps, Some(1e9));
        assert_eq!(traffic.utilization_in, Some(0.1));
        assert_eq!(traffic.utilization_out, Some(0.01));
        assert_eq!(traffic.errors_per_sec, 0.75);
    }

    #[test]
    fn a_lag_adds_its_ports_and_clamps_the_ratio() {
        let mut edges = vec![edge("t1", "t5", Some("Gi24, Gi23 +2"), None)];
        apply(&mut edges, &data());
        let traffic = edges[0].traffic.as_ref().expect("traffic");
        assert_eq!(traffic.ports.len(), 2);
        assert_eq!(traffic.bps_in, 1_100_000_000.0);
        assert_eq!(traffic.speed_bps, Some(2e9));
        assert_eq!(traffic.utilization_in, Some(0.55));
    }

    #[test]
    fn the_other_side_measures_when_the_first_has_nothing() {
        let mut edges = vec![edge("t5", "t1", Some("eth0"), Some("gi24"))];
        apply(&mut edges, &data());
        let traffic = edges[0].traffic.as_ref().expect("traffic");
        assert_eq!(traffic.measured_on, "t1");
        assert_eq!(traffic.ports, vec!["Gi24".to_string()]);
    }

    #[test]
    fn a_port_without_speed_has_no_utilization() {
        let mut edges = vec![edge("t2", "t1", Some("Gi1"), None)];
        apply(&mut edges, &data());
        let traffic = edges[0].traffic.as_ref().expect("traffic");
        assert_eq!(traffic.speed_bps, None);
        assert_eq!(traffic.utilization_in, None);
        assert_eq!(traffic.bps_in, 8_000.0);
    }

    #[test]
    fn no_port_or_no_match_leaves_the_link_without_traffic() {
        let mut edges = vec![
            edge("t1", "t2", None, None),
            edge("t1", "t2", Some("Gi99"), Some("Gi98")),
            // Un invité (`g…`) n'a pas de compteurs d'interface.
            edge("g1:vm", "t1", Some("Gi24"), None),
        ];
        apply(&mut edges, &data());
        assert!(edges.iter().all(|e| e.traffic.is_none()));
    }

    #[test]
    fn port_names_drop_the_overflow_marker() {
        assert_eq!(port_names("Gi1, Gi2 +3"), vec!["Gi1", "Gi2"]);
        assert_eq!(port_names("Port 1"), vec!["Port 1"]);
        assert_eq!(port_names(" , "), Vec::<String>::new());
    }

    #[test]
    fn the_curve_query_escapes_port_names() {
        let ports = vec!["Gi1/0/1".to_string(), "a.b\"c".to_string()];
        let query = spark_query("if_octets_in", 7, &ports);
        assert_eq!(
            query,
            "sum(rate(dumbmonit_if_octets_in{target=\"7\",ifname=~\"Gi1/0/1|a\\\\.b\\\"c\"}[5m])) * 8"
        );
    }

    #[test]
    fn the_curve_joins_both_directions_by_time() {
        let range = |values: &[(f64, &str)]| crate::tsdb::RangeSeries {
            metric: BTreeMap::new(),
            values: values.iter().map(|(t, v)| (*t, v.to_string())).collect(),
        };
        let points = merge_points(
            &[range(&[(60.0, "1"), (120.0, "2")])],
            &[range(&[(120.0, "5"), (180.0, "NaN")])],
        );
        assert_eq!(
            points,
            vec![
                TrafficPoint { t: 60, bps_in: 1.0, bps_out: 0.0 },
                TrafficPoint { t: 120, bps_in: 2.0, bps_out: 5.0 },
            ]
        );
    }
}
