//! Tunnels WireGuard : pairs, âge de la dernière poignée de main, volumes.
//!
//! Une seule commande, `wg show all dump`, dont la sortie est faite pour les
//! machines : une ligne par interface, une ligne par pair, champs séparés par
//! des tabulations. Elle contient aussi la **clé privée** de chaque interface et
//! la clé pré-partagée de chaque pair : l'analyseur les saute sans jamais les
//! copier, elles ne peuvent donc finir ni dans une série ni dans un journal.
//!
//! La commande est bon marché (un appel netlink) : elle tourne à chaque cycle,
//! avec un délai court. WireGuard n'est jamais supposé : sans `wg`, ou sans
//! interface, aucune série n'est émise. Sans les droits (`CAP_NET_ADMIN`), l'agent
//! le dit une fois et se tait.
//!
//! Un pair qui n'a encore jamais fait de poignée de main n'a pas d'âge : on
//! compte alors le silence depuis que l'agent l'a vu pour la première fois. Un
//! tunnel qui n'est jamais monté est ainsi traité comme un tunnel silencieux,
//! ce qu'il est — au lieu de rester invisible pour la règle.

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use dumbmonit_proto::{MetricKind, Sample};
use tracing::{debug, warn};

/// Délai de `wg show`. Un appel netlink répond en millisecondes ; au-delà,
/// quelque chose ne va pas et le cycle ne doit pas attendre.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

/// Plafond de pairs. Un serveur d'accès distant en a quelques dizaines ; au-delà,
/// chaque pair coûterait cinq séries pour un tableau que personne ne lirait.
pub const MAX_PEERS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireguardConfig {
    /// Faux : ni détection ni série.
    pub enabled: bool,
    /// Binaire `wg`, nom nu (cherché dans `PATH`) ou chemin complet.
    pub bin: String,
    /// Noms lisibles des pairs, par clé publique. WireGuard n'en connaît aucun.
    pub peer_names: BTreeMap<String, String>,
}

impl Default for WireguardConfig {
    fn default() -> Self {
        Self { enabled: true, bin: "wg".to_string(), peer_names: BTreeMap::new() }
    }
}

/// Un pair, tel que `wg show all dump` le décrit — sans sa clé pré-partagée.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peer {
    pub interface: String,
    pub public_key: String,
    /// Plages autorisées, dans l'ordre de `wg` ; vide pour `(none)`.
    pub allowed_ips: Vec<String>,
    /// Dernière poignée de main, en secondes Unix. `None` : jamais.
    pub latest_handshake: Option<i64>,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    /// Maintien de connexion, en secondes. `None` : désactivé.
    pub keepalive_secs: Option<u32>,
}

/// Une interface et le nombre de ses pairs. Sa clé privée n'est pas lue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    pub name: String,
    pub listen_port: Option<u16>,
    pub peers: usize,
}

/// Lecture brute de `wg show all dump`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Dump {
    pub interfaces: Vec<Interface>,
    pub peers: Vec<Peer>,
}

/// Un pair prêt à devenir des séries : le silence est daté, le nom résolu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerStat {
    pub peer: Peer,
    /// Depuis quand le pair se tait, en secondes Unix : sa dernière poignée de
    /// main, ou le moment où l'agent l'a vu la première fois sans aucune.
    pub silent_since: i64,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WireguardReport {
    pub interfaces: Vec<Interface>,
    pub peers: Vec<PeerStat>,
}

/// Lit la sortie de `wg show all dump`. Les lignes illisibles sont ignorées ;
/// les clés privées et pré-partagées ne sont jamais copiées.
pub fn parse_dump(text: &str) -> Dump {
    let mut dump = Dump::default();
    for line in text.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        match fields.as_slice() {
            // interface, clé privée, clé publique, port d'écoute, fwmark.
            [name, _private, _public, port, _fwmark] => dump.interfaces.push(Interface {
                name: (*name).to_string(),
                listen_port: port.parse().ok().filter(|p| *p != 0),
                peers: 0,
            }),
            // interface, clé publique, clé pré-partagée, point d'accès, plages,
            // poignée de main, reçus, émis, maintien.
            [
                interface,
                public_key,
                _preshared,
                _endpoint,
                allowed,
                handshake,
                rx,
                tx,
                keepalive,
            ] => {
                let Ok(rx_bytes) = rx.parse() else { continue };
                let Ok(tx_bytes) = tx.parse() else { continue };
                dump.peers.push(Peer {
                    interface: (*interface).to_string(),
                    public_key: (*public_key).to_string(),
                    allowed_ips: if *allowed == "(none)" {
                        Vec::new()
                    } else {
                        allowed.split(',').map(str::trim).map(String::from).collect()
                    },
                    latest_handshake: handshake.parse::<i64>().ok().filter(|t| *t > 0),
                    rx_bytes,
                    tx_bytes,
                    keepalive_secs: keepalive.parse().ok().filter(|k| *k > 0),
                });
            }
            _ => {}
        }
    }
    for interface in &mut dump.interfaces {
        interface.peers = dump.peers.iter().filter(|p| p.interface == interface.name).count();
    }
    dump
}

/// Traduit le rapport en échantillons.
pub fn samples(report: &WireguardReport, now_ms: i64) -> Vec<Sample> {
    let now = now_ms / 1000;
    let mut samples = Vec::with_capacity(report.interfaces.len() + report.peers.len() * 5);
    for interface in &report.interfaces {
        samples.push(
            Sample::new(
                "agent_wireguard_interface_peers",
                interface.peers as f64,
                MetricKind::Gauge,
                now_ms,
            )
            .with_label("interface", &interface.name),
        );
    }
    for stat in report.peers.iter().take(MAX_PEERS) {
        let peer = &stat.peer;
        let allowed = peer.allowed_ips.join(",");
        let labelled = |sample: Sample| {
            let sample = sample
                .with_label("interface", &peer.interface)
                .with_label("peer", &peer.public_key)
                .with_label("allowed_ips", &allowed);
            match &stat.name {
                Some(name) => sample.with_label("name", name),
                None => sample,
            }
        };
        let gauge = |metric: &str, value: f64| {
            labelled(Sample::new(metric, value, MetricKind::Gauge, now_ms))
        };
        let age = now.saturating_sub(stat.silent_since).max(0);
        samples.push(gauge("agent_wireguard_peer_handshake_age_seconds", age as f64));
        samples.push(gauge(
            "agent_wireguard_peer_has_handshake",
            f64::from(u8::from(peer.latest_handshake.is_some())),
        ));
        samples.push(gauge(
            "agent_wireguard_peer_keepalive_seconds",
            f64::from(peer.keepalive_secs.unwrap_or(0)),
        ));
        samples.push(labelled(Sample::new(
            "agent_wireguard_peer_rx_bytes",
            peer.rx_bytes as f64,
            MetricKind::Counter,
            now_ms,
        )));
        samples.push(labelled(Sample::new(
            "agent_wireguard_peer_tx_bytes",
            peer.tx_bytes as f64,
            MetricKind::Counter,
            now_ms,
        )));
    }
    samples
}

/// Lecteur des tunnels, conservé d'un cycle à l'autre : il retient depuis quand
/// chaque pair sans poignée de main se tait.
pub struct WireguardProbe {
    config: WireguardConfig,
    /// Première fois qu'un pair a été vu sans aucune poignée de main, en
    /// secondes Unix, par (interface, clé publique).
    first_silent: HashMap<(String, String), i64>,
    /// Ce qui a déjà été dit, pour ne le dire qu'une fois.
    announced: Option<Announced>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Announced {
    Absent,
    Denied,
    Found,
}

impl WireguardProbe {
    pub fn new(config: &WireguardConfig) -> Self {
        if !config.enabled {
            debug!("WireGuard reporting disabled by the configuration");
        }
        Self { config: config.clone(), first_silent: HashMap::new(), announced: None }
    }

    /// Un cycle. `None` : WireGuard absent, sans interface, ou illisible.
    pub async fn read(&mut self) -> Option<WireguardReport> {
        if !self.config.enabled {
            return None;
        }
        let dump = match run(&self.config.bin).await {
            Ok(text) => parse_dump(&text),
            Err(RunError::Missing) => {
                self.announce(Announced::Absent, "");
                return None;
            }
            Err(RunError::Failed(why)) => {
                self.announce(Announced::Denied, &why);
                return None;
            }
        };
        if dump.interfaces.is_empty() {
            self.announce(Announced::Absent, "");
            return None;
        }
        self.announce(Announced::Found, "");
        Some(self.report(dump, chrono::Utc::now().timestamp()))
    }

    /// Date le silence de chaque pair et oublie ceux qui ont disparu.
    fn report(&mut self, dump: Dump, now: i64) -> WireguardReport {
        let mut seen = HashMap::with_capacity(dump.peers.len());
        let peers = dump
            .peers
            .into_iter()
            .map(|peer| {
                let key = (peer.interface.clone(), peer.public_key.clone());
                let silent_since = match peer.latest_handshake {
                    Some(at) => at,
                    None => *self.first_silent.get(&key).unwrap_or(&now),
                };
                if peer.latest_handshake.is_none() {
                    seen.insert(key, silent_since);
                }
                let name = self.config.peer_names.get(&peer.public_key).cloned();
                PeerStat { peer, silent_since, name }
            })
            .collect();
        self.first_silent = seen;
        WireguardReport { interfaces: dump.interfaces, peers }
    }

    fn announce(&mut self, state: Announced, why: &str) {
        if self.announced == Some(state) {
            return;
        }
        match state {
            Announced::Absent => {
                debug!(bin = self.config.bin, "no WireGuard interface here, nothing reported")
            }
            Announced::Denied => warn!(
                bin = self.config.bin,
                why, "cannot read the WireGuard interfaces: the agent needs CAP_NET_ADMIN (root)"
            ),
            Announced::Found => debug!("WireGuard interfaces found"),
        }
        self.announced = Some(state);
    }
}

#[derive(Debug)]
enum RunError {
    Missing,
    Failed(String),
}

async fn run(bin: &str) -> Result<String, RunError> {
    let output = tokio::time::timeout(
        COMMAND_TIMEOUT,
        tokio::process::Command::new(bin)
            .args(["show", "all", "dump"])
            .env("LC_ALL", "C")
            .stdin(std::process::Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await;
    match output {
        Ok(Ok(output)) if output.status.success() => {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        }
        Ok(Ok(output)) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(RunError::Failed(stderr.lines().next().unwrap_or("").trim().to_string()))
        }
        Ok(Err(error)) if error.kind() == std::io::ErrorKind::NotFound => Err(RunError::Missing),
        Ok(Err(error)) => Err(RunError::Failed(error.to_string())),
        Err(_) => Err(RunError::Failed("wg show timed out".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sortie réelle de `wg show all dump` (wireguard-tools 1.0.20210914) côté
    /// serveur : un pair actif, un pair avec maintien qui n'a jamais répondu, un
    /// pair inactif, et une seconde interface sans pair. Clés privées remplacées.
    const SERVER: &str = include_str!("testdata/wg_show_all_dump.txt");
    /// La même liaison vue du client, qui maintient la connexion.
    const CLIENT: &str = include_str!("testdata/wg_show_all_dump_client.txt");

    const ACTIVE: &str = "zonmO992nrIL4FgJ3B/wccmu/SBkoX2kDhSYr8KpgRU=";
    const DEAD: &str = "UsKpqhrXNlYy/+rh2DLAOOEqesn8L8okEwNNorZ09lQ=";

    #[test]
    fn the_dump_gives_interfaces_and_peers() {
        let dump = parse_dump(SERVER);
        assert_eq!(dump.interfaces.len(), 2);
        assert_eq!(dump.interfaces[0].name, "wg0");
        assert_eq!(dump.interfaces[0].listen_port, Some(51820));
        assert_eq!(dump.interfaces[0].peers, 3);
        assert_eq!(dump.interfaces[1].name, "wg1");
        assert_eq!(dump.interfaces[1].peers, 0);

        let active = dump.peers.iter().find(|p| p.public_key == ACTIVE).expect("active peer");
        assert_eq!(active.allowed_ips, vec!["10.8.0.2/32"]);
        assert_eq!(active.latest_handshake, Some(1_790_808_303));
        assert_eq!((active.rx_bytes, active.tx_bytes), (564, 476));
        assert_eq!(active.keepalive_secs, None);

        let dead = dump.peers.iter().find(|p| p.public_key == DEAD).expect("dead peer");
        assert_eq!(dead.allowed_ips, vec!["10.8.0.3/32", "fd00::3/128"]);
        assert_eq!(dead.latest_handshake, None, "0 means never");
        assert_eq!(dead.keepalive_secs, Some(25));
        assert_eq!(dead.tx_bytes, 148, "handshake initiations count as sent bytes");

        let idle = dump.peers.iter().find(|p| p.allowed_ips == ["10.8.0.4/32"]).expect("idle");
        assert_eq!((idle.latest_handshake, idle.rx_bytes, idle.keepalive_secs), (None, 0, None));
    }

    #[test]
    fn the_client_side_reads_the_same() {
        let dump = parse_dump(CLIENT);
        assert_eq!(dump.interfaces[0].listen_port, Some(42166));
        assert_eq!(dump.peers.len(), 1);
        assert_eq!(dump.peers[0].allowed_ips, vec!["10.8.0.0/24"]);
        assert_eq!(dump.peers[0].keepalive_secs, Some(25));
    }

    #[test]
    fn private_and_preshared_keys_never_reach_a_series() {
        let with_psk = "wg0\tPRIVATEKEYPRIVATEKEYPRIVATEKEYPRIVATEKEY00=\tpub\t51820\toff\n\
                        wg0\tpeer\tPRESHAREDPRESHAREDPRESHAREDPRESHARED0000=\t192.0.2.2:1\t10.0.0.2/32\t1\t2\t3\toff\n";
        let mut probe = WireguardProbe::new(&WireguardConfig::default());
        let report = probe.report(parse_dump(with_psk), 100);
        let rendered = format!("{:?}{:?}", report, samples(&report, 100_000));
        assert!(!rendered.contains("PRIVATEKEY"));
        assert!(!rendered.contains("PRESHARED"));
    }

    #[test]
    fn silence_is_counted_from_the_handshake_or_from_the_first_sighting() {
        let mut probe = WireguardProbe::new(&WireguardConfig::default());
        let now = 1_790_808_403;
        let first = probe.report(parse_dump(SERVER), now);
        let series = samples(&first, now * 1000);
        let age = |key: &str| {
            series
                .iter()
                .find(|s| {
                    s.metric == "agent_wireguard_peer_handshake_age_seconds"
                        && s.labels.get("peer").map(String::as_str) == Some(key)
                })
                .map(|s| s.value)
        };
        assert_eq!(age(ACTIVE), Some(100.0));
        assert_eq!(age(DEAD), Some(0.0), "never seen before: silent from now on");

        // Dix minutes plus tard, toujours rien : le silence a dix minutes.
        let later = probe.report(parse_dump(SERVER), now + 600);
        let dead = later.peers.iter().find(|p| p.peer.public_key == DEAD).expect("dead");
        assert_eq!(dead.silent_since, now);
    }

    #[test]
    fn a_vanished_peer_is_forgotten() {
        let mut probe = WireguardProbe::new(&WireguardConfig::default());
        probe.report(parse_dump(SERVER), 1000);
        assert_eq!(probe.first_silent.len(), 2);
        probe.report(parse_dump(CLIENT), 2000);
        assert!(probe.first_silent.is_empty());
    }

    #[test]
    fn each_peer_becomes_labelled_series_with_counters() {
        let names = BTreeMap::from([(ACTIVE.to_string(), "laptop".to_string())]);
        let mut probe = WireguardProbe::new(&WireguardConfig {
            peer_names: names,
            ..WireguardConfig::default()
        });
        let report = probe.report(parse_dump(SERVER), 1_790_808_403);
        let series = samples(&report, 1_790_808_403_000);
        assert_eq!(series.len(), 2 + 3 * 5);
        let rx = series
            .iter()
            .find(|s| {
                s.metric == "agent_wireguard_peer_rx_bytes"
                    && s.labels.get("peer").map(String::as_str) == Some(ACTIVE)
            })
            .expect("rx");
        assert_eq!(rx.kind, MetricKind::Counter);
        assert_eq!(rx.value, 564.0);
        assert_eq!(rx.labels.get("name").map(String::as_str), Some("laptop"));
        assert_eq!(rx.labels.get("allowed_ips").map(String::as_str), Some("10.8.0.2/32"));
        let peers = series
            .iter()
            .find(|s| {
                s.metric == "agent_wireguard_interface_peers"
                    && s.labels.get("interface").map(String::as_str) == Some("wg1")
            })
            .expect("wg1");
        assert_eq!(peers.value, 0.0);
    }

    #[test]
    fn peers_are_capped() {
        let peers = (0..MAX_PEERS + 10)
            .map(|i| PeerStat {
                peer: Peer {
                    interface: "wg0".into(),
                    public_key: format!("k{i}"),
                    allowed_ips: Vec::new(),
                    latest_handshake: None,
                    rx_bytes: 0,
                    tx_bytes: 0,
                    keepalive_secs: None,
                },
                silent_since: 0,
                name: None,
            })
            .collect();
        let report = WireguardReport { interfaces: Vec::new(), peers };
        assert_eq!(samples(&report, 0).len(), MAX_PEERS * 5);
    }

    #[tokio::test]
    async fn a_machine_without_wg_reports_nothing() {
        let mut probe = WireguardProbe::new(&WireguardConfig {
            bin: "/nonexistent/wg".into(),
            ..WireguardConfig::default()
        });
        assert!(probe.read().await.is_none());
        let mut off =
            WireguardProbe::new(&WireguardConfig { enabled: false, ..WireguardConfig::default() });
        assert!(off.read().await.is_none());
    }
}
