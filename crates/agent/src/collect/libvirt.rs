//! Domaines KVM et pools de stockage libvirt, par `virsh`.
//!
//! Même logique que `collect/zfs.rs` : aucune supposition que libvirt est
//! installé, aucune écriture, et un silence complet (pas une série à zéro)
//! quand `virsh` est absent ou que le démon `libvirtd` ne répond pas. Un
//! homelab qui fait tourner des VM KVM à la main, sans aucune surcouche,
//! retrouve ainsi ses domaines sans rien configurer sur le serveur DumbMonit :
//! c'est l'agent installé sur l'hyperviseur qui lit `virsh` localement.
//!
//! Trois commandes, toutes en lecture, toutes avec `-c <uri>` (par défaut
//! `qemu:///system`) : sans lui, un compte non root du groupe `libvirt`
//! lirait sa session privée `qemu:///session`, vide, au lieu des VM du
//! système.
//!
//! - `virsh list --all --name` : le nom de chaque domaine, actif ou non.
//! - `virsh dominfo --domain <nom>` : son état, son nombre de vCPU et sa
//!   mémoire (en kio). Une commande par domaine : un homelab en compte
//!   quelques unités, pas des centaines. `--domain` plutôt qu'un argument
//!   positionnel : un nom qui commence par `-` ne devient pas une option.
//! - `virsh pool-list --all --name` puis `virsh pool-info --pool <nom>` : l'état et
//!   la capacité de chaque pool de stockage. `pool-info` affiche des tailles
//!   arrondies à deux décimales dans une unité binaire (« 50.00 GiB ») plutôt
//!   qu'en octets exacts ; le pourcentage d'utilisation qui en découle est
//!   donc approximatif, ce qui suffit à une alerte de remplissage.
//!
//! Les commandes tournent avec `LC_ALL=C` : la sortie de `virsh` dépend
//! de la locale pour certains libellés.

use std::time::Duration;

use dumbmonit_proto::{MetricKind, Sample};
use tokio::task::JoinHandle;
use tracing::debug;

/// Période par défaut entre deux lectures. Un domaine ne change pas d'état
/// entre deux battements de cœur.
pub const DEFAULT_INTERVAL_SECS: u64 = 60;

/// En dessous, l'agent passerait son temps dans `virsh`.
pub const MIN_INTERVAL_SECS: u64 = 15;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(20);

/// Attente au premier cycle, pour qu'un `--once` montre quelque chose.
const FIRST_CYCLE_WAIT: Duration = Duration::from_secs(5);

/// Plafond de domaines et de pools détaillés. Un homelab en a quelques
/// dizaines au plus ; au-delà, c'est une baie qui mérite son propre outil.
pub const MAX_DOMAINS: usize = 200;
pub const MAX_POOLS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibvirtConfig {
    /// Faux : ni détection ni série, quoi qu'il y ait sur la machine.
    pub enabled: bool,
    /// Binaire, nom nu (cherché dans `PATH`) ou chemin complet.
    pub bin: String,
    /// URI de connexion libvirt passée à `virsh -c`.
    pub uri: String,
    pub interval: Duration,
}

/// Les VM du système, quel que soit le compte qui lance l'agent.
pub const DEFAULT_URI: &str = "qemu:///system";

impl Default for LibvirtConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bin: "virsh".to_string(),
            uri: DEFAULT_URI.to_string(),
            interval: Duration::from_secs(DEFAULT_INTERVAL_SECS),
        }
    }
}

/// État d'un domaine, dans l'ordre croissant de gravité.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainState {
    /// `running` ou `idle` : la VM tourne.
    Running,
    /// `paused` : suspendue en mémoire, volontairement le plus souvent.
    Paused,
    /// `pmsuspended` : suspension invitée (ACPI S3/S4).
    Suspended,
    /// `shut off` : arrêtée proprement ou jamais démarrée.
    ShutOff,
    /// `crashed` : l'hyperviseur a dû la tuer.
    Crashed,
    /// Tout autre libellé (`in shutdown`, `blocked`, …).
    Other,
}

impl DomainState {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "running" | "idle" => Self::Running,
            "paused" => Self::Paused,
            "pmsuspended" => Self::Suspended,
            "shut off" | "shutoff" => Self::ShutOff,
            "crashed" => Self::Crashed,
            _ => Self::Other,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Suspended => "suspended",
            Self::ShutOff => "shut off",
            Self::Crashed => "crashed",
            Self::Other => "other",
        }
    }

    fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }

    fn is_crashed(self) -> bool {
        matches!(self, Self::Crashed)
    }
}

/// État d'un pool de stockage, lu de `pool-info`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolState {
    Running,
    Inactive,
    Other,
}

impl PoolState {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "running" | "active" => Self::Running,
            "inactive" => Self::Inactive,
            _ => Self::Other,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Inactive => "inactive",
            Self::Other => "other",
        }
    }

    fn is_active(self) -> bool {
        matches!(self, Self::Running)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainStat {
    pub name: String,
    pub state: DomainState,
    /// `None` si `dominfo` ne le dit pas (domaine arrêté, le plus souvent).
    pub vcpus: Option<u32>,
    pub max_memory_bytes: Option<u64>,
    pub used_memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolStat {
    pub name: String,
    pub state: PoolState,
    pub capacity_bytes: Option<u64>,
    pub allocation_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LibvirtReport {
    pub domains: Vec<DomainStat>,
    pub pools: Vec<PoolStat>,
}

/// Traduit le relevé en échantillons.
pub fn samples(report: &LibvirtReport, now_ms: i64) -> Vec<Sample> {
    let mut samples = Vec::with_capacity(report.domains.len() * 6 + report.pools.len() * 5 + 2);
    let gauge = |metric: &str, value: f64| Sample::new(metric, value, MetricKind::Gauge, now_ms);

    let running = report.domains.iter().filter(|domain| domain.state.is_running()).count();
    samples.push(gauge("agent_libvirt_domains", report.domains.len() as f64));
    samples.push(gauge("agent_libvirt_domains_running", running as f64));
    for domain in report.domains.iter().take(MAX_DOMAINS) {
        let labelled = |sample: Sample| sample.with_label("domain", &domain.name);
        samples.push(labelled(
            gauge("agent_libvirt_domain_state_info", 1.0).with_label("state", domain.state.label()),
        ));
        samples.push(labelled(gauge(
            "agent_libvirt_domain_running",
            f64::from(u8::from(domain.state.is_running())),
        )));
        samples.push(labelled(gauge(
            "agent_libvirt_domain_crashed",
            f64::from(u8::from(domain.state.is_crashed())),
        )));
        if let Some(vcpus) = domain.vcpus {
            samples.push(labelled(gauge("agent_libvirt_domain_vcpus", f64::from(vcpus))));
        }
        if let Some(max) = domain.max_memory_bytes {
            samples.push(labelled(gauge("agent_libvirt_domain_memory_max_bytes", max as f64)));
        }
        if let Some(used) = domain.used_memory_bytes {
            samples.push(labelled(gauge("agent_libvirt_domain_memory_used_bytes", used as f64)));
        }
    }

    samples.push(gauge("agent_libvirt_pools", report.pools.len() as f64));
    for pool in report.pools.iter().take(MAX_POOLS) {
        let labelled = |sample: Sample| sample.with_label("pool", &pool.name);
        samples.push(labelled(
            gauge("agent_libvirt_pool_state_info", 1.0).with_label("state", pool.state.label()),
        ));
        samples.push(labelled(gauge(
            "agent_libvirt_pool_active",
            f64::from(u8::from(pool.state.is_active())),
        )));
        if let Some(capacity) = pool.capacity_bytes {
            samples.push(labelled(gauge("agent_libvirt_pool_capacity_bytes", capacity as f64)));
        }
        if let Some(allocation) = pool.allocation_bytes {
            samples.push(labelled(gauge("agent_libvirt_pool_allocation_bytes", allocation as f64)));
        }
        if let (Some(capacity), Some(allocation)) = (pool.capacity_bytes, pool.allocation_bytes)
            && capacity > 0
        {
            let percent = 100.0 * allocation as f64 / capacity as f64;
            samples.push(labelled(gauge("agent_libvirt_pool_used_percent", percent)));
        }
    }
    samples
}

// ----------------------------------------------------------------- analyse

/// `virsh list --all --name` : un nom de domaine par ligne, une ligne vide en
/// fin de sortie.
pub fn parse_domain_names(stdout: &str) -> Vec<String> {
    stdout.lines().map(str::trim).filter(|name| !name.is_empty()).map(str::to_string).collect()
}

/// `virsh dominfo <nom>` : des paires `Clé:   Valeur` alignées sur deux
/// points, mémoire en kio.
pub fn parse_dominfo(name: &str, stdout: &str) -> DomainStat {
    let mut state = DomainState::Other;
    let mut vcpus = None;
    let mut max_memory_bytes = None;
    let mut used_memory_bytes = None;
    for line in stdout.lines() {
        let Some((key, value)) = line.split_once(':') else { continue };
        let value = value.trim();
        match key.trim() {
            "State" => state = DomainState::parse(value),
            "CPU(s)" => vcpus = value.parse().ok(),
            "Max memory" => max_memory_bytes = parse_kib_field(value),
            "Used memory" => used_memory_bytes = parse_kib_field(value),
            _ => {}
        }
    }
    DomainStat { name: name.to_string(), state, vcpus, max_memory_bytes, used_memory_bytes }
}

/// `"2097152 KiB"` → octets. Les autres unités (`dominfo` n'en affiche pas
/// d'autre) sont acceptées par prudence.
fn parse_kib_field(value: &str) -> Option<u64> {
    let mut parts = value.split_whitespace();
    let number: f64 = parts.next()?.parse().ok()?;
    let bytes = match parts.next().unwrap_or("KiB").to_ascii_uppercase().as_str() {
        "B" | "BYTES" => number,
        "MIB" => number * 1024.0 * 1024.0,
        "GIB" => number * 1024.0 * 1024.0 * 1024.0,
        _ => number * 1024.0,
    };
    Some(bytes as u64)
}

/// `virsh pool-info <nom>` : mêmes paires `Clé:   Valeur`, tailles affichées
/// en unité binaire arrondie (« 50.00 GiB »).
pub fn parse_pool_info(name: &str, stdout: &str) -> PoolStat {
    let mut state = PoolState::Other;
    let mut capacity_bytes = None;
    let mut allocation_bytes = None;
    let mut available_bytes = None;
    for line in stdout.lines() {
        let Some((key, value)) = line.split_once(':') else { continue };
        let value = value.trim();
        match key.trim() {
            "State" => state = PoolState::parse(value),
            "Capacity" => capacity_bytes = parse_pretty_capacity(value),
            "Allocation" => allocation_bytes = parse_pretty_capacity(value),
            "Available" => available_bytes = parse_pretty_capacity(value),
            _ => {}
        }
    }
    PoolStat { name: name.to_string(), state, capacity_bytes, allocation_bytes, available_bytes }
}

/// `"50.00 GiB"`, `"1.50 TiB"`, `"512 bytes"` : la mise en forme binaire de
/// `virsh` (`vshPrettyCapacity`), à deux décimales.
fn parse_pretty_capacity(value: &str) -> Option<u64> {
    let mut parts = value.split_whitespace();
    let number: f64 = parts.next()?.parse().ok()?;
    let unit = parts.next()?.to_ascii_uppercase();
    let multiplier = match unit.as_str() {
        "B" | "BYTE" | "BYTES" => 1.0,
        "KIB" => 1024.0,
        "MIB" => 1024.0_f64.powi(2),
        "GIB" => 1024.0_f64.powi(3),
        "TIB" => 1024.0_f64.powi(4),
        "PIB" => 1024.0_f64.powi(5),
        _ => return None,
    };
    Some((number * multiplier) as u64)
}

/// Recolle les trois lectures en un relevé.
pub fn merge(domains: Vec<DomainStat>, pools: Vec<PoolStat>) -> LibvirtReport {
    let mut domains = domains;
    domains.sort_by(|a, b| a.name.cmp(&b.name));
    let mut pools = pools;
    pools.sort_by(|a, b| a.name.cmp(&b.name));
    LibvirtReport { domains, pools }
}

// ----------------------------------------------------------------- lecture

pub struct LibvirtProbe {
    config: LibvirtConfig,
    last: Option<LibvirtReport>,
    refreshed_at: Option<tokio::time::Instant>,
    task: Option<JoinHandle<Option<LibvirtReport>>>,
    announced_absent: bool,
}

impl LibvirtProbe {
    pub fn new(config: &LibvirtConfig) -> Self {
        if !config.enabled {
            debug!("libvirt reporting disabled by the configuration");
        }
        Self {
            config: config.clone(),
            last: None,
            refreshed_at: None,
            task: None,
            announced_absent: false,
        }
    }

    /// Un cycle. `None` : pas de libvirt sur cette machine (pas de `virsh`,
    /// ou `libvirtd` ne répond pas), ou première lecture pas encore aboutie.
    pub async fn read(&mut self) -> Option<LibvirtReport> {
        if !self.config.enabled {
            return None;
        }
        if self.task.is_none()
            && self.refreshed_at.is_none_or(|at| at.elapsed() >= self.config.interval)
        {
            self.task = Some(tokio::spawn(read_all(self.config.clone())));
        }

        if let Some(task) = self.task.as_mut() {
            let first_cycle = self.refreshed_at.is_none();
            if (task.is_finished() || first_cycle)
                && let Ok(outcome) = tokio::time::timeout(FIRST_CYCLE_WAIT, task).await
            {
                self.task = None;
                self.refreshed_at = Some(tokio::time::Instant::now());
                match outcome {
                    Ok(report) => self.announce(report),
                    Err(error) => debug!(%error, "libvirt reading aborted"),
                }
            }
        }
        self.last.clone()
    }

    fn announce(&mut self, report: Option<LibvirtReport>) {
        match report {
            Some(report) => {
                if self.last.is_none() {
                    debug!(domains = report.domains.len(), "libvirt domains found");
                }
                self.announced_absent = false;
                self.last = Some(report);
            }
            None => {
                if !self.announced_absent {
                    debug!(
                        bin = self.config.bin,
                        "no libvirt domain readable on this machine, nothing reported"
                    );
                    self.announced_absent = true;
                }
                self.last = None;
            }
        }
    }
}

async fn read_all(config: LibvirtConfig) -> Option<LibvirtReport> {
    let names = parse_domain_names(&run(&config, &["list", "--all", "--name"]).await?);
    if names.is_empty() {
        // `virsh` répond : libvirt tourne, mais aucun domaine défini. On le
        // distingue d'un `virsh` absent en rendant un relevé vide plutôt que
        // `None`, pour que les compteurs à zéro remplacent d'anciennes séries.
        let pools = read_pools(&config).await;
        return Some(LibvirtReport { domains: Vec::new(), pools });
    }
    let mut domains = Vec::with_capacity(names.len());
    for name in names.iter().take(MAX_DOMAINS) {
        if let Some(out) = run(&config, &["dominfo", "--domain", name.as_str()]).await {
            domains.push(parse_dominfo(name, &out));
        }
    }
    let pools = read_pools(&config).await;
    Some(merge(domains, pools))
}

async fn read_pools(config: &LibvirtConfig) -> Vec<PoolStat> {
    let Some(out) = run(config, &["pool-list", "--all", "--name"]).await else {
        return Vec::new();
    };
    let names = parse_domain_names(&out);
    let mut pools = Vec::with_capacity(names.len());
    for name in names.iter().take(MAX_POOLS) {
        if let Some(info) = run(config, &["pool-info", "--pool", name.as_str()]).await {
            pools.push(parse_pool_info(name, &info));
        }
    }
    pools
}

/// Lance `virsh` en anglais et rend sa sortie standard. `None` : binaire
/// absent, `libvirtd` injoignable, ou commande refusée (domaine ou pool
/// disparu entre la liste et l'interrogation).
async fn run(config: &LibvirtConfig, args: &[&str]) -> Option<String> {
    let bin = config.bin.as_str();
    let output = tokio::time::timeout(
        COMMAND_TIMEOUT,
        tokio::process::Command::new(bin)
            .arg("-c")
            .arg(&config.uri)
            .args(args)
            .env("LC_ALL", "C")
            .env("LANG", "C")
            .kill_on_drop(true)
            .output(),
    )
    .await;
    match output {
        Ok(Ok(output)) if output.status.success() => {
            Some(String::from_utf8_lossy(&output.stdout).to_string())
        }
        Ok(Ok(_)) => None,
        Ok(Err(error)) => {
            debug!(bin, %error, "virsh could not be run");
            None
        }
        Err(_) => {
            debug!(bin, "virsh timed out");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOMINFO_RUNNING: &str = "Id:             3\n\
        Name:           ubuntu-vm\n\
        UUID:           12345678-1234-1234-1234-123456789abc\n\
        OS Type:        hvm\n\
        State:          running\n\
        CPU(s):         2\n\
        CPU time:       123.4s\n\
        Max memory:     2097152 KiB\n\
        Used memory:    2097152 KiB\n\
        Persistent:     yes\n\
        Autostart:      disable\n\
        Managed save:   no\n\
        Security model: none\n\
        Security DOI:   0\n";

    const DOMINFO_SHUTOFF: &str = "Id:             -\n\
        Name:           debian-template\n\
        UUID:           87654321-4321-4321-4321-cba987654321\n\
        OS Type:        hvm\n\
        State:          shut off\n\
        CPU(s):         1\n\
        Max memory:     1048576 KiB\n\
        Persistent:     yes\n\
        Autostart:      disable\n\
        Managed save:   no\n\
        Security model: none\n\
        Security DOI:   0\n";

    const DOMINFO_CRASHED: &str = "Id:             -\n\
        Name:           flaky-vm\n\
        State:          crashed\n\
        CPU(s):         4\n\
        Max memory:     4194304 KiB\n";

    const POOL_INFO_DEFAULT: &str = "Name:           default\n\
        UUID:           11111111-1111-1111-1111-111111111111\n\
        State:          running\n\
        Persistent:     yes\n\
        Autostart:      yes\n\
        Capacity:       50.00 GiB\n\
        Allocation:     45.50 GiB\n\
        Available:      4.50 GiB\n";

    const POOL_INFO_INACTIVE: &str = "Name:           archive\n\
        State:          inactive\n\
        Persistent:     yes\n\
        Autostart:      no\n\
        Capacity:       0.00 B\n\
        Allocation:     0.00 B\n\
        Available:      0.00 B\n";

    #[test]
    fn domain_names_are_split_one_per_line() {
        assert_eq!(
            parse_domain_names("ubuntu-vm\ndebian-template\n\n"),
            vec!["ubuntu-vm".to_string(), "debian-template".to_string()]
        );
        assert!(parse_domain_names("\n").is_empty());
        assert!(parse_domain_names("").is_empty());
    }

    #[test]
    fn a_running_domain_carries_its_cpu_and_memory() {
        let domain = parse_dominfo("ubuntu-vm", DOMINFO_RUNNING);
        assert_eq!(domain.name, "ubuntu-vm");
        assert_eq!(domain.state, DomainState::Running);
        assert_eq!(domain.vcpus, Some(2));
        assert_eq!(domain.max_memory_bytes, Some(2_097_152 * 1024));
        assert_eq!(domain.used_memory_bytes, Some(2_097_152 * 1024));
    }

    #[test]
    fn a_shut_off_domain_has_no_used_memory() {
        let domain = parse_dominfo("debian-template", DOMINFO_SHUTOFF);
        assert_eq!(domain.state, DomainState::ShutOff);
        assert_eq!(domain.vcpus, Some(1));
        assert_eq!(domain.max_memory_bytes, Some(1_048_576 * 1024));
        assert_eq!(domain.used_memory_bytes, None);
    }

    #[test]
    fn a_crashed_domain_is_reported_as_such() {
        let domain = parse_dominfo("flaky-vm", DOMINFO_CRASHED);
        assert_eq!(domain.state, DomainState::Crashed);
        assert!(domain.state.is_crashed());
        assert!(!domain.state.is_running());
    }

    #[test]
    fn pool_info_reads_binary_rounded_sizes() {
        let pool = parse_pool_info("default", POOL_INFO_DEFAULT);
        assert_eq!(pool.state, PoolState::Running);
        assert_eq!(pool.capacity_bytes, Some((50.00_f64 * 1024.0_f64.powi(3)) as u64));
        assert_eq!(pool.allocation_bytes, Some((45.50_f64 * 1024.0_f64.powi(3)) as u64));
    }

    #[test]
    fn an_inactive_pool_has_zero_capacity_not_unknown() {
        let pool = parse_pool_info("archive", POOL_INFO_INACTIVE);
        assert_eq!(pool.state, PoolState::Inactive);
        assert_eq!(pool.capacity_bytes, Some(0));
    }

    #[test]
    fn pretty_capacity_understands_every_unit_virsh_prints() {
        assert_eq!(parse_pretty_capacity("512 bytes"), Some(512));
        assert_eq!(parse_pretty_capacity("1.00 KiB"), Some(1024));
        assert_eq!(parse_pretty_capacity("2.50 MiB"), Some((2.5 * 1024.0 * 1024.0) as u64));
        assert_eq!(parse_pretty_capacity("1.50 TiB"), Some((1.5 * 1024.0_f64.powi(4)) as u64));
        assert_eq!(parse_pretty_capacity("not a size"), None);
    }

    #[test]
    fn samples_cover_domains_and_pools() {
        let report = merge(
            vec![
                parse_dominfo("ubuntu-vm", DOMINFO_RUNNING),
                parse_dominfo("flaky-vm", DOMINFO_CRASHED),
            ],
            vec![parse_pool_info("default", POOL_INFO_DEFAULT)],
        );
        let samples = samples(&report, 1_760_000_000_000);
        let value = |metric: &str, labels: &[(&str, &str)]| -> f64 {
            samples
                .iter()
                .find(|s| {
                    s.metric == metric
                        && labels
                            .iter()
                            .all(|(k, v)| s.labels.get(*k).map(String::as_str) == Some(v))
                })
                .map(|s| s.value)
                .unwrap_or_else(|| panic!("no sample {metric} {labels:?}"))
        };
        assert_eq!(value("agent_libvirt_domains", &[]), 2.0);
        assert_eq!(value("agent_libvirt_domains_running", &[]), 1.0);
        assert_eq!(value("agent_libvirt_domain_running", &[("domain", "ubuntu-vm")]), 1.0);
        assert_eq!(value("agent_libvirt_domain_crashed", &[("domain", "flaky-vm")]), 1.0);
        assert_eq!(value("agent_libvirt_domain_vcpus", &[("domain", "ubuntu-vm")]), 2.0);
        assert_eq!(value("agent_libvirt_pool_active", &[("pool", "default")]), 1.0);
        let used_percent = value("agent_libvirt_pool_used_percent", &[("pool", "default")]);
        assert!((used_percent - 91.0).abs() < 1.0, "{used_percent}");
    }

    #[test]
    fn no_domain_still_reports_pools_and_zero_counts() {
        let report = LibvirtReport { domains: Vec::new(), pools: Vec::new() };
        let samples = samples(&report, 0);
        assert!(samples.iter().any(|s| s.metric == "agent_libvirt_domains" && s.value == 0.0));
    }

    #[tokio::test]
    async fn a_machine_without_virsh_reports_nothing() {
        let config =
            LibvirtConfig { bin: "/inexistant/virsh".to_string(), ..LibvirtConfig::default() };
        let mut probe = LibvirtProbe::new(&config);
        assert!(probe.read().await.is_none());

        let mut off =
            LibvirtProbe::new(&LibvirtConfig { enabled: false, ..LibvirtConfig::default() });
        assert!(off.read().await.is_none());
    }
}
