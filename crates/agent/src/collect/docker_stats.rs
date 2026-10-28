//! Consommation des conteneurs (`GET /containers/{id}/stats`) et espace disque
//! de Docker (`GET /system/df`).
//!
//! Les statistiques sont lues en un coup (`stream=false&one-shot=true`) : le
//! démon répond tout de suite au lieu d'attendre deux secondes pour remplir
//! `precpu_stats`. Le pourcentage de CPU se calcule donc ici, par différence
//! entre deux cycles, ce qui coûte une requête par conteneur en marche et
//! aucune attente. Le premier cycle d'un conteneur n'a pas de CPU : mieux
//! vaut aucune valeur qu'une moyenne depuis le démarrage.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::Deserialize;

use super::docker::DockerClient;

/// Requêtes de statistiques simultanées : assez pour tenir la cadence avec
/// deux cents conteneurs, trop peu pour réveiller le démon en rafale.
pub const STATS_PARALLELISM: usize = 8;

/// Budget total d'un cycle de statistiques. Au-delà, les conteneurs restants
/// n'ont pas de valeur ce cycle-ci : la collecte système ne doit pas attendre.
pub const STATS_BUDGET: Duration = Duration::from_secs(10);

/// `system/df` parcourt les couches et les volumes : cher sur un gros hôte, et
/// ce qu'il mesure bouge lentement.
pub const DF_REFRESH: Duration = Duration::from_secs(600);

/// Délai propre à `system/df`, bien plus long que les lectures ordinaires.
pub const DF_TIMEOUT: Duration = Duration::from_secs(20);

/// Consommation instantanée d'un conteneur.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContainerUsage {
    /// CPU en centièmes de pourcent d'un cœur (10 000 = un cœur plein) ;
    /// `None` au premier relevé, ou après une remise à zéro du compteur.
    pub cpu_centipercent: Option<u64>,
    /// Mémoire utilisée, cache de pages inactif exclu (comme `docker stats`).
    pub memory_bytes: u64,
    pub net_rx_bytes: u64,
    pub net_tx_bytes: u64,
    pub block_read_bytes: u64,
    pub block_write_bytes: u64,
    pub pids: Option<u64>,
}

/// Relevé brut, avant calcul du CPU.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RawStats {
    /// Temps CPU cumulé, en nanosecondes.
    pub cpu_total_ns: u64,
    pub usage: ContainerUsage,
}

#[derive(Debug, Deserialize, Default)]
struct StatsBody {
    #[serde(default)]
    cpu_stats: CpuStats,
    #[serde(default)]
    memory_stats: MemoryStats,
    #[serde(default)]
    networks: HashMap<String, NetStats>,
    #[serde(default)]
    blkio_stats: BlkioStats,
    #[serde(default)]
    pids_stats: PidsStats,
}

#[derive(Debug, Deserialize, Default)]
struct CpuStats {
    #[serde(default)]
    cpu_usage: CpuUsage,
}

#[derive(Debug, Deserialize, Default)]
struct CpuUsage {
    #[serde(default)]
    total_usage: u64,
}

#[derive(Debug, Deserialize, Default)]
struct MemoryStats {
    #[serde(default)]
    usage: u64,
    #[serde(default)]
    stats: HashMap<String, u64>,
}

#[derive(Debug, Deserialize, Default)]
struct NetStats {
    #[serde(default)]
    rx_bytes: u64,
    #[serde(default)]
    tx_bytes: u64,
}

#[derive(Debug, Deserialize, Default)]
struct BlkioStats {
    /// `null` sur certains noyaux, d'où l'`Option`.
    #[serde(default)]
    io_service_bytes_recursive: Option<Vec<BlkioEntry>>,
}

#[derive(Debug, Deserialize)]
struct BlkioEntry {
    #[serde(default)]
    op: String,
    #[serde(default)]
    value: u64,
}

#[derive(Debug, Deserialize, Default)]
struct PidsStats {
    current: Option<u64>,
}

/// Lit le corps de `/containers/{id}/stats`.
pub fn parse_stats(body: &[u8]) -> Result<RawStats> {
    let raw: StatsBody = serde_json::from_slice(body).context("unreadable container stats")?;
    // Même règle que `docker stats` : on retire le cache de pages récupérable
    // (`inactive_file` en cgroup v2, `total_inactive_file` ou `cache` en v1).
    let cache = ["inactive_file", "total_inactive_file", "cache"]
        .iter()
        .find_map(|key| raw.memory_stats.stats.get(*key).copied())
        .unwrap_or(0);
    let (mut read, mut write) = (0u64, 0u64);
    for entry in raw.blkio_stats.io_service_bytes_recursive.unwrap_or_default() {
        match entry.op.to_ascii_lowercase().as_str() {
            "read" => read = read.saturating_add(entry.value),
            "write" => write = write.saturating_add(entry.value),
            _ => {}
        }
    }
    Ok(RawStats {
        cpu_total_ns: raw.cpu_stats.cpu_usage.total_usage,
        usage: ContainerUsage {
            cpu_centipercent: None,
            memory_bytes: raw.memory_stats.usage.saturating_sub(cache),
            net_rx_bytes: raw.networks.values().map(|n| n.rx_bytes).sum(),
            net_tx_bytes: raw.networks.values().map(|n| n.tx_bytes).sum(),
            block_read_bytes: read,
            block_write_bytes: write,
            pids: raw.pids_stats.current,
        },
    })
}

/// Mémorise le dernier temps CPU de chaque conteneur pour en tirer un taux.
#[derive(Debug, Default)]
pub struct CpuTracker {
    last: HashMap<String, (u64, Instant)>,
}

impl CpuTracker {
    /// Centièmes de pourcent d'un cœur depuis le relevé précédent. `None` au
    /// premier relevé, si le compteur a reculé (conteneur recréé) ou si les
    /// deux relevés sont trop proches pour que la division ait un sens.
    pub fn rate(&mut self, id: &str, cpu_total_ns: u64, now: Instant) -> Option<u64> {
        let previous = self.last.insert(id.to_string(), (cpu_total_ns, now));
        let (before_ns, before_at) = previous?;
        let wall_ns = now.checked_duration_since(before_at)?.as_nanos();
        if cpu_total_ns < before_ns || wall_ns < 100_000_000 {
            return None;
        }
        let cpu_ns = u128::from(cpu_total_ns - before_ns);
        Some((cpu_ns * 10_000 / wall_ns) as u64)
    }

    /// Oublie les conteneurs qui ne sont plus en marche.
    pub fn retain(&mut self, alive: &std::collections::HashSet<&str>) {
        self.last.retain(|id, _| alive.contains(id.as_str()));
    }
}

/// Lit les statistiques de plusieurs conteneurs, `STATS_PARALLELISM` à la fois
/// et dans la limite de `STATS_BUDGET`. Un conteneur qui échoue est simplement
/// absent du résultat.
pub async fn read_many(client: &DockerClient, ids: &[String]) -> HashMap<String, RawStats> {
    let started = Instant::now();
    let mut found = HashMap::new();
    for chunk in ids.chunks(STATS_PARALLELISM) {
        if started.elapsed() >= STATS_BUDGET {
            tracing::debug!(read = found.len(), total = ids.len(), "container stats budget spent");
            break;
        }
        let mut set = tokio::task::JoinSet::new();
        for id in chunk {
            let client = client.clone();
            let id = id.clone();
            set.spawn(async move {
                let path = format!("/containers/{id}/stats?stream=false&one-shot=true");
                let stats = match client.get(&path).await {
                    Ok(response) if response.is_success() => parse_stats(&response.body).ok(),
                    _ => None,
                };
                (id, stats)
            });
        }
        while let Some(joined) = set.join_next().await {
            if let Ok((id, Some(stats))) = joined {
                found.insert(id, stats);
            }
        }
    }
    found
}

/// Espace disque de Docker, en octets.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiskUsage {
    pub images_bytes: u64,
    pub containers_bytes: u64,
    pub volumes_bytes: u64,
    pub build_cache_bytes: u64,
    /// Ce qu'un `prune` rendrait : images sans conteneur, conteneurs arrêtés,
    /// volumes inutilisés, cache de build hors d'usage.
    pub reclaimable_bytes: u64,
}

#[derive(Debug, Deserialize, Default)]
struct DfBody {
    #[serde(rename = "LayersSize", default)]
    layers_size: Option<i64>,
    #[serde(rename = "Images", default)]
    images: Option<Vec<DfImage>>,
    #[serde(rename = "Containers", default)]
    containers: Option<Vec<DfContainer>>,
    #[serde(rename = "Volumes", default)]
    volumes: Option<Vec<DfVolume>>,
    #[serde(rename = "BuildCache", default)]
    build_cache: Option<Vec<DfCache>>,
}

#[derive(Debug, Deserialize)]
struct DfImage {
    #[serde(rename = "Size", default)]
    size: i64,
    #[serde(rename = "SharedSize", default)]
    shared_size: i64,
    /// Nombre de conteneurs qui l'utilisent ; `-1` : inconnu.
    #[serde(rename = "Containers", default)]
    containers: i64,
}

#[derive(Debug, Deserialize)]
struct DfContainer {
    #[serde(rename = "SizeRw", default)]
    size_rw: i64,
    #[serde(rename = "State", default)]
    state: String,
}

#[derive(Debug, Deserialize)]
struct DfVolume {
    #[serde(rename = "UsageData")]
    usage: Option<DfVolumeUsage>,
}

#[derive(Debug, Deserialize)]
struct DfVolumeUsage {
    #[serde(rename = "Size", default)]
    size: i64,
    #[serde(rename = "RefCount", default)]
    ref_count: i64,
}

#[derive(Debug, Deserialize)]
struct DfCache {
    #[serde(rename = "Size", default)]
    size: i64,
    #[serde(rename = "InUse", default)]
    in_use: bool,
}

/// Les tailles inconnues valent `-1` chez Docker : on les compte pour zéro.
fn size(value: i64) -> u64 {
    value.max(0) as u64
}

/// Lit le corps de `/system/df`.
pub fn parse_disk_usage(body: &[u8]) -> Result<DiskUsage> {
    let raw: DfBody = serde_json::from_slice(body).context("unreadable Docker disk usage")?;
    let images = raw.images.unwrap_or_default();
    let containers = raw.containers.unwrap_or_default();
    let volumes = raw.volumes.unwrap_or_default();
    let cache = raw.build_cache.unwrap_or_default();

    // `LayersSize` compte les couches partagées une seule fois ; sans lui, la
    // somme des tailles d'images les compterait autant de fois qu'elles servent.
    let images_bytes = match raw.layers_size {
        Some(layers) if layers >= 0 => layers as u64,
        _ => images.iter().map(|i| size(i.size)).sum(),
    };
    let containers_bytes = containers.iter().map(|c| size(c.size_rw)).sum();
    let volumes_bytes: u64 =
        volumes.iter().filter_map(|v| v.usage.as_ref()).map(|u| size(u.size)).sum();
    let build_cache_bytes = cache.iter().map(|c| size(c.size)).sum();

    let reclaimable_images: u64 = images
        .iter()
        .filter(|i| i.containers == 0)
        .map(|i| size(i.size).saturating_sub(size(i.shared_size)))
        .sum();
    let reclaimable_containers: u64 =
        containers.iter().filter(|c| c.state != "running").map(|c| size(c.size_rw)).sum();
    let reclaimable_volumes: u64 = volumes
        .iter()
        .filter_map(|v| v.usage.as_ref())
        .filter(|u| u.ref_count == 0)
        .map(|u| size(u.size))
        .sum();
    let reclaimable_cache: u64 = cache.iter().filter(|c| !c.in_use).map(|c| size(c.size)).sum();

    Ok(DiskUsage {
        images_bytes,
        containers_bytes,
        volumes_bytes,
        build_cache_bytes,
        reclaimable_bytes: reclaimable_images
            + reclaimable_containers
            + reclaimable_volumes
            + reclaimable_cache,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATS_V2: &str = r#"{
        "cpu_stats": {"cpu_usage": {"total_usage": 2000000000}},
        "memory_stats": {"usage": 300000000, "limit": 8000000000,
                         "stats": {"inactive_file": 100000000, "anon": 150000000}},
        "networks": {"eth0": {"rx_bytes": 1000, "tx_bytes": 500},
                     "eth1": {"rx_bytes": 24, "tx_bytes": 12}},
        "blkio_stats": {"io_service_bytes_recursive": [
            {"major": 8, "minor": 0, "op": "read", "value": 4096},
            {"major": 8, "minor": 0, "op": "write", "value": 8192},
            {"major": 8, "minor": 16, "op": "Write", "value": 1}]},
        "pids_stats": {"current": 7}
    }"#;

    #[test]
    fn stats_give_memory_without_cache_and_sum_the_interfaces() {
        let raw = parse_stats(STATS_V2.as_bytes()).unwrap();
        assert_eq!(raw.cpu_total_ns, 2_000_000_000);
        let usage = raw.usage;
        assert_eq!(usage.memory_bytes, 200_000_000, "inactive_file is not used memory");
        assert_eq!((usage.net_rx_bytes, usage.net_tx_bytes), (1024, 512));
        assert_eq!((usage.block_read_bytes, usage.block_write_bytes), (4096, 8193));
        assert_eq!(usage.pids, Some(7));
        assert_eq!(usage.cpu_centipercent, None, "the rate needs a previous reading");
    }

    #[test]
    fn cgroup_v1_stats_use_the_cache_and_tolerate_missing_sections() {
        let body = r#"{"memory_stats":{"usage":500,"stats":{"cache":200}},
                       "blkio_stats":{"io_service_bytes_recursive":null}}"#;
        let usage = parse_stats(body.as_bytes()).unwrap().usage;
        assert_eq!(usage.memory_bytes, 300);
        assert_eq!((usage.net_rx_bytes, usage.block_read_bytes, usage.pids), (0, 0, None));
        assert!(parse_stats(b"pas du json").is_err());
    }

    #[test]
    fn the_cpu_rate_is_a_difference_between_two_readings() {
        let mut tracker = CpuTracker::default();
        let t0 = Instant::now();
        assert_eq!(tracker.rate("a", 1_000_000_000, t0), None, "first reading");
        // 1,5 s de CPU en 3 s d'horloge : la moitié d'un cœur.
        let t1 = t0 + Duration::from_secs(3);
        assert_eq!(tracker.rate("a", 2_500_000_000, t1), Some(5_000));
        // Deux cœurs pleins.
        let t2 = t1 + Duration::from_secs(1);
        assert_eq!(tracker.rate("a", 4_500_000_000, t2), Some(20_000));
        // Compteur remis à zéro (conteneur recréé) : pas de valeur négative.
        let t3 = t2 + Duration::from_secs(1);
        assert_eq!(tracker.rate("a", 10, t3), None);
        // Deux relevés quasi simultanés : pas de division par presque zéro.
        assert_eq!(tracker.rate("a", 20, t3 + Duration::from_millis(5)), None);
        // Un conteneur oublié repart de zéro.
        tracker.retain(&std::collections::HashSet::new());
        assert_eq!(tracker.rate("a", 30, t3 + Duration::from_secs(5)), None);
    }

    #[test]
    fn disk_usage_counts_what_a_prune_would_free() {
        let body = r#"{
            "LayersSize": 1000,
            "Images": [{"Size": 600, "SharedSize": 100, "Containers": 0},
                       {"Size": 700, "SharedSize": 100, "Containers": 2},
                       {"Size": 50, "SharedSize": -1, "Containers": -1}],
            "Containers": [{"SizeRw": 10, "State": "running"},
                           {"SizeRw": 30, "State": "exited"}],
            "Volumes": [{"UsageData": {"Size": 400, "RefCount": 0}},
                        {"UsageData": {"Size": 200, "RefCount": 1}},
                        {"UsageData": {"Size": -1, "RefCount": -1}},
                        {"UsageData": null}],
            "BuildCache": [{"Size": 80, "InUse": false}, {"Size": 20, "InUse": true}]
        }"#;
        let usage = parse_disk_usage(body.as_bytes()).unwrap();
        assert_eq!(
            usage,
            DiskUsage {
                images_bytes: 1000,
                containers_bytes: 40,
                volumes_bytes: 600,
                build_cache_bytes: 100,
                // images 500 + conteneurs 30 + volumes 400 + cache 80
                reclaimable_bytes: 1010,
            }
        );
    }

    #[test]
    fn an_empty_or_null_disk_report_is_all_zero() {
        let body = r#"{"Images":null,"Containers":null,"Volumes":null,"BuildCache":null}"#;
        assert_eq!(parse_disk_usage(body.as_bytes()).unwrap(), DiskUsage::default());
        assert!(parse_disk_usage(b"[").is_err());
    }
}
