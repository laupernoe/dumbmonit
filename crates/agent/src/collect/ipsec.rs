//! Tunnels IPsec (strongSwan), lus en lecture seule par `swanctl`.
//!
//! Deux commandes, chacune bon marché (un aller-retour sur le socket VICI) :
//! `swanctl --list-conns` donne les tunnels **configurés** — une CHILD_SA par
//! nom, même jamais montée — et `swanctl --list-sas` donne les tunnels
//! **actifs**. L'un sans l'autre ne suffirait pas : sans `--list-conns`, un
//! tunnel qui n'a jamais été monté serait invisible plutôt que signalé comme
//! éteint ; sans `--list-sas`, aucun tunnel ne pourrait jamais être dit « up ».
//!
//! Pas de bibliothèque VICI : les deux sorties sont celles que `swanctl`
//! affiche à l'écran, faites pour un humain mais stables d'une version à
//! l'autre — c'est ce que toutes les notices de strongSwan montrent. Aucune
//! commande d'écriture n'est jamais envoyée (`swanctl --initiate`,
//! `--terminate`, …) : l'agent regarde, il ne pilote rien.
//!
//! strongSwan n'est jamais **supposé** : sans `swanctl`, ou sans connexion
//! configurée, l'agent n'émet aucune série `agent_ipsec_*` et le dit une fois,
//! en `debug`. `enabled: false` coupe la détection elle-même.

use std::collections::BTreeMap;
use std::time::Duration;

use dumbmonit_proto::{MetricKind, Sample};
use tracing::{debug, warn};

/// Délai de chaque commande `swanctl`. Un aller-retour sur le socket VICI
/// local répond en quelques millisecondes ; au-delà, quelque chose ne va pas.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

/// Au-delà, les tunnels suivants sont ignorés (et signalés dans le journal).
pub const MAX_TUNNELS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpsecConfig {
    /// Faux : ni détection ni série.
    pub enabled: bool,
    /// Binaire `swanctl`, nom nu (cherché dans `PATH`) ou chemin complet.
    pub bin: String,
}

impl Default for IpsecConfig {
    fn default() -> Self {
        Self { enabled: true, bin: "swanctl".to_string() }
    }
}

/// Une CHILD_SA configurée (`swanctl --list-conns`), qu'elle soit montée ou non.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfiguredTunnel {
    /// Nom de la connexion IKE qui la porte.
    pub connection: String,
    /// Nom de la CHILD_SA, tel que déclaré dans `swanctl.conf`.
    pub name: String,
}

/// Une CHILD_SA active (`swanctl --list-sas`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveTunnel {
    pub connection: String,
    pub name: String,
    /// `INSTALLED`, `REKEYED`, … tel que `swanctl` l'affiche.
    pub state: String,
    pub bytes_in: u64,
    pub bytes_out: u64,
    /// Temps avant le prochain renouvellement de clé, en secondes. Absent sur
    /// une CHILD_SA qui n'en a pas (mode `PASS`/`DROP`).
    pub rekey_in_seconds: Option<u64>,
}

/// Un tunnel prêt à devenir des séries : son réglage et, s'il est monté,
/// son état.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelStat {
    pub connection: String,
    pub name: String,
    pub up: bool,
    pub bytes_in: Option<u64>,
    pub bytes_out: Option<u64>,
    pub rekey_in_seconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IpsecReport {
    pub tunnels: Vec<TunnelStat>,
}

/// Un tunnel est « monté » dès lors qu'il a une CHILD_SA installée ; les états
/// transitoires (`REKEYING`, `REKEYED`, `DELETING`) comptent encore debout —
/// seule son absence totale vaut tunnel éteint.
fn is_up(state: &str) -> bool {
    !matches!(state, "DESTROYING")
}

/// Lit `swanctl --list-conns` : une CHILD_SA configurée par ligne indentée de
/// deux espaces et portant « , rekeying every » — ce qui la distingue des
/// lignes d'attributs (`local:`, `remote:`, l'authentification) qui n'ont pas
/// ce motif.
pub fn parse_conns(text: &str) -> Vec<ConfiguredTunnel> {
    let mut tunnels = Vec::new();
    let mut connection = String::new();
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if indent == 0 {
            connection = trimmed.split(':').next().unwrap_or(trimmed).trim().to_string();
            continue;
        }
        if indent == 2 && trimmed.contains(", rekeying every ") {
            let Some((name, _)) = trimmed.split_once(':') else { continue };
            let name = name.trim();
            if !name.is_empty() && !connection.is_empty() {
                tunnels
                    .push(ConfiguredTunnel { connection: connection.clone(), name: name.into() });
            }
        }
    }
    tunnels
}

/// Lit `swanctl --list-sas` : une CHILD_SA active par ligne indentée de deux
/// espaces et portant « , reqid » ; ses propres attributs (`in`, `out`,
/// `installed/rekeying`) sont indentés de quatre espaces.
pub fn parse_sas(text: &str) -> Vec<ActiveTunnel> {
    let mut tunnels = Vec::new();
    let mut connection = String::new();
    let mut current: Option<ActiveTunnel> = None;
    let flush = |current: &mut Option<ActiveTunnel>, tunnels: &mut Vec<ActiveTunnel>| {
        if let Some(tunnel) = current.take() {
            tunnels.push(tunnel);
        }
    };
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if indent == 0 {
            flush(&mut current, &mut tunnels);
            connection = trimmed.split(':').next().unwrap_or(trimmed).trim().to_string();
            continue;
        }
        if indent == 2 && trimmed.contains(", reqid ") {
            flush(&mut current, &mut tunnels);
            let Some((name, rest)) = trimmed.split_once(':') else { continue };
            let state = rest.split(',').nth(2).map(str::trim).unwrap_or("").to_string();
            current = Some(ActiveTunnel {
                connection: connection.clone(),
                name: name.trim().to_string(),
                state,
                bytes_in: 0,
                bytes_out: 0,
                rekey_in_seconds: None,
            });
            continue;
        }
        if indent == 4 {
            let Some(tunnel) = current.as_mut() else { continue };
            if let Some(rest) = trimmed.strip_prefix("in ") {
                tunnel.bytes_in = parse_bytes_field(rest).unwrap_or(0);
            } else if let Some(rest) = trimmed.strip_prefix("out ") {
                tunnel.bytes_out = parse_bytes_field(rest).unwrap_or(0);
            } else if trimmed.contains("rekeying in ") {
                tunnel.rekey_in_seconds = parse_rekeying_in(trimmed);
            }
        }
    }
    flush(&mut current, &mut tunnels);
    tunnels
}

/// `c1234567,    147852 bytes,     987 packets,    12s ago` → 147852.
fn parse_bytes_field(rest: &str) -> Option<u64> {
    let bytes_part = rest.split(',').nth(1)?.trim();
    bytes_part.strip_suffix("bytes")?.trim().parse().ok()
}

/// `installed 3618s ago, rekeying in 2782s, expires in 3582s` → 2782.
fn parse_rekeying_in(line: &str) -> Option<u64> {
    let after = line.split("rekeying in ").nth(1)?;
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Apparie la configuration (ce qui doit exister) à l'actif (ce qui existe) :
/// une CHILD_SA configurée sans CHILD_SA active installée vaut tunnel éteint.
pub fn merge(configured: &[ConfiguredTunnel], active: &[ActiveTunnel]) -> IpsecReport {
    let mut by_key: BTreeMap<(&str, &str), &ActiveTunnel> = BTreeMap::new();
    for tunnel in active {
        // Une CHILD_SA peut être relue plusieurs fois (renouvellement de clé) ;
        // la plus récemment listée par `swanctl` est gardée, ce qui est aussi la
        // seule plausiblement installée.
        by_key.insert((tunnel.connection.as_str(), tunnel.name.as_str()), tunnel);
    }
    let tunnels = configured
        .iter()
        .take(MAX_TUNNELS)
        .map(|wanted| match by_key.get(&(wanted.connection.as_str(), wanted.name.as_str())) {
            Some(tunnel) if is_up(&tunnel.state) => TunnelStat {
                connection: wanted.connection.clone(),
                name: wanted.name.clone(),
                up: true,
                bytes_in: Some(tunnel.bytes_in),
                bytes_out: Some(tunnel.bytes_out),
                rekey_in_seconds: tunnel.rekey_in_seconds,
            },
            _ => TunnelStat {
                connection: wanted.connection.clone(),
                name: wanted.name.clone(),
                up: false,
                bytes_in: None,
                bytes_out: None,
                rekey_in_seconds: None,
            },
        })
        .collect();
    IpsecReport { tunnels }
}

/// Traduit le rapport en échantillons.
pub fn samples(report: &IpsecReport, now_ms: i64) -> Vec<Sample> {
    let mut samples = Vec::with_capacity(report.tunnels.len() * 4);
    for tunnel in &report.tunnels {
        let labelled = |sample: Sample| {
            sample.with_label("connection", &tunnel.connection).with_label("tunnel", &tunnel.name)
        };
        samples.push(labelled(Sample::new(
            "agent_ipsec_tunnel_up",
            f64::from(u8::from(tunnel.up)),
            MetricKind::Gauge,
            now_ms,
        )));
        if let Some(bytes_in) = tunnel.bytes_in {
            samples.push(labelled(Sample::new(
                "agent_ipsec_tunnel_bytes_in",
                bytes_in as f64,
                MetricKind::Counter,
                now_ms,
            )));
        }
        if let Some(bytes_out) = tunnel.bytes_out {
            samples.push(labelled(Sample::new(
                "agent_ipsec_tunnel_bytes_out",
                bytes_out as f64,
                MetricKind::Counter,
                now_ms,
            )));
        }
        if let Some(rekey) = tunnel.rekey_in_seconds {
            samples.push(labelled(Sample::new(
                "agent_ipsec_tunnel_rekey_seconds",
                rekey as f64,
                MetricKind::Gauge,
                now_ms,
            )));
        }
    }
    samples
}

/// Lecteur des tunnels IPsec, conservé d'un cycle à l'autre pour ne dire
/// qu'une fois ce qu'il a trouvé.
pub struct IpsecProbe {
    config: IpsecConfig,
    announced: Option<Announced>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Announced {
    Absent,
    Denied,
    Found,
}

impl IpsecProbe {
    pub fn new(config: &IpsecConfig) -> Self {
        if !config.enabled {
            debug!("IPsec reporting disabled by the configuration");
        }
        Self { config: config.clone(), announced: None }
    }

    /// Un cycle. `None` : `swanctl` absent, sans connexion configurée, ou
    /// illisible.
    pub async fn read(&mut self) -> Option<IpsecReport> {
        if !self.config.enabled {
            return None;
        }
        let conns_text = match run(&self.config.bin, &["--list-conns"]).await {
            Ok(text) => text,
            Err(RunError::Missing) => {
                self.announce(Announced::Absent, "");
                return None;
            }
            Err(RunError::Failed(why)) => {
                self.announce(Announced::Denied, &why);
                return None;
            }
        };
        let configured = parse_conns(&conns_text);
        if configured.is_empty() {
            self.announce(Announced::Absent, "");
            return None;
        }
        let active = match run(&self.config.bin, &["--list-sas"]).await {
            Ok(text) => parse_sas(&text),
            Err(RunError::Missing) => {
                self.announce(Announced::Absent, "");
                return None;
            }
            Err(RunError::Failed(why)) => {
                self.announce(Announced::Denied, &why);
                return None;
            }
        };
        self.announce(Announced::Found, "");
        if configured.len() > MAX_TUNNELS {
            warn!(count = configured.len(), max = MAX_TUNNELS, "too many IPsec tunnels, truncated");
        }
        Some(merge(&configured, &active))
    }

    fn announce(&mut self, state: Announced, why: &str) {
        if self.announced == Some(state) {
            return;
        }
        match state {
            Announced::Absent => {
                debug!(bin = self.config.bin, "no IPsec connection here, nothing reported")
            }
            Announced::Denied => warn!(
                bin = self.config.bin,
                why, "cannot read the IPsec connections: is the strongSwan daemon running?"
            ),
            Announced::Found => debug!("IPsec connections found"),
        }
        self.announced = Some(state);
    }
}

#[derive(Debug)]
enum RunError {
    Missing,
    Failed(String),
}

async fn run(bin: &str, args: &[&str]) -> Result<String, RunError> {
    let output = tokio::time::timeout(
        COMMAND_TIMEOUT,
        tokio::process::Command::new(bin)
            .args(args)
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
        Err(_) => Err(RunError::Failed(format!("swanctl {} timed out", args.join(" ")))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONNS: &str = include_str!("testdata/swanctl_list_conns.txt");
    const SAS: &str = include_str!("testdata/swanctl_list_sas.txt");

    #[test]
    fn configured_tunnels_are_read_from_list_conns() {
        let tunnels = parse_conns(CONNS);
        assert_eq!(tunnels.len(), 3);
        assert_eq!(
            tunnels[0],
            ConfiguredTunnel { connection: "site-a".into(), name: "net-net".into() }
        );
        assert_eq!(
            tunnels[1],
            ConfiguredTunnel { connection: "site-b".into(), name: "office-net".into() }
        );
        assert_eq!(
            tunnels[2],
            ConfiguredTunnel { connection: "backup-link".into(), name: "backup-net".into() }
        );
    }

    #[test]
    fn active_tunnels_are_read_from_list_sas() {
        let tunnels = parse_sas(SAS);
        assert_eq!(tunnels.len(), 1, "site-b is CONNECTING with no child SA yet");
        let net = &tunnels[0];
        assert_eq!(net.connection, "site-a");
        assert_eq!(net.name, "net-net");
        assert_eq!(net.state, "INSTALLED");
        assert_eq!(net.bytes_in, 147_852);
        assert_eq!(net.bytes_out, 98_765);
        assert_eq!(net.rekey_in_seconds, Some(2782));
    }

    #[test]
    fn a_configured_tunnel_without_an_installed_child_sa_is_down() {
        let report = merge(&parse_conns(CONNS), &parse_sas(SAS));
        assert_eq!(report.tunnels.len(), 3);

        let up = report.tunnels.iter().find(|t| t.name == "net-net").expect("net-net");
        assert!(up.up);
        assert_eq!(up.bytes_in, Some(147_852));

        let never_installed =
            report.tunnels.iter().find(|t| t.name == "office-net").expect("office-net");
        assert!(!never_installed.up, "site-b never completed its IKE_SA");
        assert_eq!(never_installed.bytes_in, None);

        let unknown_to_sas =
            report.tunnels.iter().find(|t| t.name == "backup-net").expect("backup-net");
        assert!(!unknown_to_sas.up, "backup-link has no IKE_SA at all in list-sas");
    }

    #[test]
    fn samples_carry_bytes_only_for_tunnels_that_are_up() {
        let report = merge(&parse_conns(CONNS), &parse_sas(SAS));
        let series = samples(&report, 1_000);
        assert_eq!(series.iter().filter(|s| s.metric == "agent_ipsec_tunnel_up").count(), 3);
        let up_values: Vec<f64> = series
            .iter()
            .filter(|s| s.metric == "agent_ipsec_tunnel_up")
            .map(|s| s.value)
            .collect();
        assert_eq!(up_values.iter().filter(|v| **v == 1.0).count(), 1);
        assert_eq!(up_values.iter().filter(|v| **v == 0.0).count(), 2);
        assert_eq!(series.iter().filter(|s| s.metric == "agent_ipsec_tunnel_bytes_in").count(), 1);
        assert!(series.iter().all(|s| s.ts_ms == 1_000));
    }

    #[test]
    fn without_any_configured_connection_nothing_is_read() {
        assert!(parse_conns("").is_empty());
        assert!(merge(&[], &parse_sas(SAS)).tunnels.is_empty());
    }

    #[tokio::test]
    async fn a_machine_without_swanctl_reports_nothing() {
        let mut probe =
            IpsecProbe::new(&IpsecConfig { bin: "/nonexistent/swanctl".into(), enabled: true });
        assert!(probe.read().await.is_none());
        let mut off = IpsecProbe::new(&IpsecConfig { enabled: false, ..IpsecConfig::default() });
        assert!(off.read().await.is_none());
    }

    #[test]
    fn tunnels_are_capped() {
        let configured: Vec<ConfiguredTunnel> = (0..MAX_TUNNELS + 10)
            .map(|i| ConfiguredTunnel { connection: "c".into(), name: format!("t{i}") })
            .collect();
        let report = merge(&configured, &[]);
        assert_eq!(report.tunnels.len(), MAX_TUNNELS);
    }
}
