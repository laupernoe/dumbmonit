//! Mesures synthétiques du mode démonstration.
//!
//! Les sondes de disponibilité (HTTP, TLS, ping) et la machine équipée de
//! l'agent n'ont pas d'équipement simulé derrière elles : leurs valeurs sont
//! calculées ici, **en fonction du seul horodatage** et du nom de la cible. Le
//! même instant donne toujours la même valeur — c'est ce qui permet à
//! l'historique rétroactif ([`super::backfill`]) et aux mesures en direct de se
//! raccorder sans couture, et à un redémarrage de ne rien faire dériver.
//!
//! Aucune de ces sondes ne touche le réseau : dans une démonstration publique,
//! pas une seule connexion ne part vers une adresse, même fictive.

use std::f64::consts::TAU;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, MetricKind, ProbeError, Sample, Target};

/// Machines déclarées injoignables en permanence : un commutateur et le point
/// d'accès qui en dépend, pour montrer la suppression des alertes en cascade.
pub const DOWN_HOSTS: &[&str] = &["switch-attic.home.arpa", "ap-attic.home.arpa"];

/// Service qui tombe une heure de temps en temps (toutes les 97 heures, calé sur
/// l'horloge absolue) : l'historique des alertes et les pourcentages de
/// disponibilité ont ainsi quelque chose à raconter.
pub const FLAKY_HOST: &str = "jellyfin.home.arpa";

/// Certificat qui arrive à échéance : l'alerte « Certificate expiring soon ».
pub const EXPIRING_CERT_HOST: &str = "vault.home.arpa";

/// Empreinte FNV-1a d'un nom : la graine de ses courbes.
pub fn seed(name: &str) -> u64 {
    name.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Bruit déterministe dans [0, 1) pour une graine et un créneau.
fn noise(seed: u64, bucket: i64) -> f64 {
    // splitmix64 : court, et suffisant pour du bruit visuel.
    let mut z = seed.wrapping_add((bucket as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

/// Niveau d'activité dans [0, 1] : un cycle quotidien (creux la nuit, pic en
/// soirée), un léger effet week-end et un bruit par tranche de cinq minutes.
pub fn activity(seed: u64, ts_ms: i64) -> f64 {
    let ts_s = ts_ms.div_euclid(1000);
    let day = (ts_s.rem_euclid(86_400)) as f64 / 86_400.0;
    let phase = (seed % 360) as f64 / 360.0 * 0.15;
    // Pic vers 20 h UTC.
    let daily = 0.5 - 0.5 * (TAU * (day - 0.33 + phase)).cos();
    let weekday = (ts_s.div_euclid(86_400) + 4).rem_euclid(7); // 0 = lundi
    let weekend = if weekday >= 5 { 0.12 } else { 0.0 };
    let jitter = noise(seed, ts_s.div_euclid(300)) - 0.5;
    (0.15 + 0.6 * daily + weekend + 0.2 * jitter).clamp(0.0, 1.0)
}

fn gauge(metric: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(metric, value, MetricKind::Gauge, ts_ms)
}

fn counter(metric: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(metric, value, MetricKind::Counter, ts_ms)
}

/// Sonde synthétique : même type que la vraie, aucune connexion.
pub struct SyntheticProbe {
    kind: &'static str,
}

impl SyntheticProbe {
    pub fn new(kind: &'static str) -> Self {
        Self { kind }
    }
}

#[async_trait]
impl Collector for SyntheticProbe {
    fn kind(&self) -> &'static str {
        self.kind
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        probe_samples(self.kind, &target.name, chrono::Utc::now().timestamp_millis())
    }
}

/// Ce que la sonde `kind` rapporterait pour `name` à l'instant `ts_ms`.
pub fn probe_samples(kind: &str, name: &str, ts_ms: i64) -> Result<Vec<Sample>, ProbeError> {
    if DOWN_HOSTS.contains(&name) {
        return Err(ProbeError::Unreachable("no reply to 3 ICMP echo requests".to_string()));
    }
    let seed = seed(name);
    let load = activity(seed, ts_ms);
    let mut samples = Vec::new();
    let mut success = true;
    let mut labelled = |sample: Sample| samples.push(sample.with_label("probe", kind));

    match kind {
        "http" => {
            let outage = name == FLAKY_HOST && (ts_ms / 3_600_000) % 97 == 0;
            let base = 0.04 + (seed % 7) as f64 * 0.012;
            let first_byte = base + 0.18 * load * load;
            if outage {
                success = false;
                labelled(gauge("probe_http_status_code", 502.0, ts_ms));
                labelled(gauge("probe_failure_info", 1.0, ts_ms).with_label("reason", "status"));
            } else {
                labelled(gauge("probe_http_status_code", 200.0, ts_ms));
                labelled(gauge("probe_http_first_byte_seconds", first_byte, ts_ms));
                labelled(gauge("probe_http_content_bytes", 18_000.0 + (seed % 9000) as f64, ts_ms));
            }
            labelled(gauge("probe_duration_seconds", first_byte + 0.01, ts_ms));
        }
        "tls" => {
            // Un certificat renouvelé en continu : l'échéance reste dans la même
            // fourchette quel que soit le jour où l'on regarde.
            let within_day = (ts_ms.rem_euclid(86_400_000)) as f64 / 86_400_000.0;
            let days = if name == EXPIRING_CERT_HOST { 9.0 } else { 54.0 + (seed % 30) as f64 };
            labelled(gauge("probe_ssl_cert_expiry_days", days + 1.0 - within_day, ts_ms));
            labelled(gauge("probe_ssl_cert_valid", 1.0, ts_ms));
            labelled(
                gauge("probe_ssl_cert_issuer_info", 1.0, ts_ms)
                    .with_label("issuer", "CN=Home CA Intermediate, O=home.arpa"),
            );
            labelled(gauge("probe_tls_version_info", 1.0, ts_ms).with_label("version", "TLS 1.3"));
            labelled(gauge("probe_connect_seconds", 0.002 + 0.004 * load, ts_ms));
            labelled(gauge("probe_tls_handshake_seconds", 0.011 + 0.02 * load, ts_ms));
            labelled(gauge("probe_duration_seconds", 0.015 + 0.025 * load, ts_ms));
        }
        _ => {
            let rtt = 0.0004 + 0.0015 * load + (seed % 5) as f64 * 0.0002;
            labelled(gauge("probe_icmp_packets_sent", 3.0, ts_ms));
            labelled(gauge("probe_icmp_packets_received", 3.0, ts_ms));
            labelled(gauge("probe_icmp_packet_loss_ratio", 0.0, ts_ms));
            labelled(gauge("probe_icmp_rtt_seconds", rtt, ts_ms));
            labelled(gauge("probe_icmp_rtt_min_seconds", rtt * 0.8, ts_ms));
            labelled(gauge("probe_icmp_rtt_max_seconds", rtt * 1.3, ts_ms));
            labelled(gauge("probe_duration_seconds", rtt * 3.0, ts_ms));
        }
    }
    labelled(gauge("probe_success", if success { 1.0 } else { 0.0 }, ts_ms));
    Ok(samples)
}

/// Nom d'hôte de la machine équipée de l'agent.
pub const AGENT_HOST: &str = "docker01.home.arpa";

/// Conteneurs de la machine : (nom, image, en marche).
const CONTAINERS: &[(&str, &str, bool)] = &[
    ("traefik", "traefik:v3.5", true),
    ("nextcloud", "nextcloud:31-apache", true),
    ("postgres", "postgres:17-alpine", true),
    ("home-assistant", "ghcr.io/home-assistant/home-assistant:2026.9", true),
    ("jellyfin", "jellyfin/jellyfin:10.11", true),
    ("paperless-worker", "ghcr.io/paperless-ngx/paperless-ngx:2.18", false),
];

const MEMORY_TOTAL: f64 = 32.0 * 1024.0 * 1024.0 * 1024.0;
const DATA_TOTAL: f64 = 1_800.0 * 1_000_000_000.0;
const ROOT_TOTAL: f64 = 120.0 * 1_000_000_000.0;
/// Point de départ des compteurs cumulatifs : une date fixe, pour qu'ils ne
/// repartent jamais de zéro d'un redémarrage à l'autre.
const COUNTER_EPOCH_MS: i64 = 1_767_225_600_000; // 2026-01-01T00:00:00Z

/// Ce que l'agent de [`AGENT_HOST`] enverrait à l'instant `ts_ms`.
pub fn agent_samples(ts_ms: i64) -> Vec<Sample> {
    let seed = seed(AGENT_HOST);
    let load = activity(seed, ts_ms);
    let elapsed_s = ((ts_ms - COUNTER_EPOCH_MS).max(0) / 1000) as f64;
    let mut samples = vec![
        gauge("cpu_usage_percent", 6.0 + 58.0 * load, ts_ms),
        gauge("cpu_count", 8.0, ts_ms),
        gauge("load_average_1", 0.3 + 4.2 * load, ts_ms),
        gauge("load_average_5", 0.3 + 3.8 * load, ts_ms),
        gauge("load_average_15", 0.3 + 3.2 * load, ts_ms),
        gauge("memory_total_bytes", MEMORY_TOTAL, ts_ms),
        gauge("memory_used_bytes", MEMORY_TOTAL * (0.46 + 0.2 * load), ts_ms),
        gauge("memory_available_bytes", MEMORY_TOTAL * (0.54 - 0.2 * load), ts_ms),
        gauge("memory_used_percent", 100.0 * (0.46 + 0.2 * load), ts_ms),
        gauge("swap_total_bytes", 4.0 * 1024.0 * 1024.0 * 1024.0, ts_ms),
        gauge("swap_used_bytes", 180.0 * 1024.0 * 1024.0, ts_ms),
        gauge("swap_used_percent", 4.4, ts_ms),
        gauge("uptime_seconds", (ts_ms.rem_euclid(41 * 86_400_000) / 1000) as f64, ts_ms),
        gauge("process_count", 310.0 + 60.0 * load, ts_ms),
        gauge("container_count", CONTAINERS.len() as f64, ts_ms),
        gauge(
            "container_running_count",
            CONTAINERS.iter().filter(|(_, _, running)| *running).count() as f64,
            ts_ms,
        ),
        gauge("container_series_skipped", 0.0, ts_ms),
    ];
    for core in 0..8 {
        let core_load = activity(seed.wrapping_add(core), ts_ms);
        samples.push(
            gauge("cpu_core_usage_percent", 4.0 + 62.0 * core_load, ts_ms)
                .with_label("core", core.to_string()),
        );
    }
    // Le volume de données grossit lentement et régulièrement (≈ 2 Go par jour) :
    // de quoi nourrir la prévision de remplissage sans jamais la faire hurler.
    let data_used = DATA_TOTAL * 0.71 + 2.0e9 * (elapsed_s / 86_400.0 % 90.0);
    for (mount, device, fstype, total, used) in [
        ("/", "/dev/nvme0n1p2", "ext4", ROOT_TOTAL, ROOT_TOTAL * 0.38),
        ("/srv/data", "/dev/md0", "xfs", DATA_TOTAL, data_used),
    ] {
        for (metric, value) in [
            ("filesystem_total_bytes", total),
            ("filesystem_used_bytes", used),
            ("filesystem_free_bytes", total - used),
            ("filesystem_used_percent", 100.0 * used / total),
        ] {
            samples.push(
                gauge(metric, value, ts_ms)
                    .with_label("mountpoint", mount)
                    .with_label("device", device)
                    .with_label("fstype", fstype),
            );
        }
    }
    // Compteurs : intégrale d'un débit moyen, pour que `rate()` retombe sur une
    // courbe plausible sans avoir à conserver d'état.
    for (ifname, rx, tx) in [("eth0", 3.2e6, 1.1e6), ("docker0", 4.0e5, 6.0e5)] {
        for (metric, rate) in [("if_octets_in", rx), ("if_octets_out", tx)] {
            samples.push(
                counter(metric, rate * elapsed_s * (0.6 + 0.4 * load), ts_ms)
                    .with_label("ifname", ifname),
            );
        }
        for (metric, value) in [
            ("if_packets_in", rx / 900.0 * elapsed_s),
            ("if_packets_out", tx / 900.0 * elapsed_s),
            ("if_errors_in", 0.0),
            ("if_errors_out", 0.0),
        ] {
            samples.push(counter(metric, value, ts_ms).with_label("ifname", ifname));
        }
    }
    for (metric, rate) in [("disk_read_bytes", 2.1e6), ("disk_written_bytes", 3.4e6)] {
        samples.push(counter(metric, rate * elapsed_s, ts_ms).with_label("device", "nvme0n1"));
    }
    for (index, (name, image, running)) in CONTAINERS.iter().enumerate() {
        let labelled =
            |sample: Sample| sample.with_label("container", *name).with_label("image", *image);
        samples.push(labelled(gauge("container_up", f64::from(u8::from(*running)), ts_ms)));
        // 1 = sain, comme le code de l'agent (`ContainerHealth::as_value`).
        samples.push(labelled(gauge("container_health", 1.0, ts_ms)));
        samples.push(labelled(gauge("container_restart_count", index as f64 % 2.0, ts_ms)));
        if *running {
            samples.push(labelled(gauge(
                "container_started_seconds",
                (ts_ms.rem_euclid((9 + index as i64) * 86_400_000) / 1000) as f64,
                ts_ms,
            )));
        }
        samples.push(labelled(gauge(
            "container_image_age_seconds",
            (12 + 5 * index as i64) as f64 * 86_400.0,
            ts_ms,
        )));
    }
    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_meme_instant_donne_les_memes_valeurs() {
        let ts = 1_790_000_000_000;
        let first = agent_samples(ts);
        let second = agent_samples(ts);
        assert_eq!(first.len(), second.len());
        assert!(first.iter().zip(&second).all(|(a, b)| a.value == b.value));
        let probe = probe_samples("http", "nextcloud.home.arpa", ts).unwrap();
        assert_eq!(probe, probe_samples("http", "nextcloud.home.arpa", ts).unwrap());
    }

    #[test]
    fn l_activite_reste_bornee() {
        for step in 0..2000 {
            let value = activity(42, 1_790_000_000_000 + step * 300_000);
            assert!((0.0..=1.0).contains(&value));
        }
    }

    #[test]
    fn les_hotes_en_panne_le_restent() {
        assert!(probe_samples("ping", "switch-attic.home.arpa", 0).is_err());
        assert!(probe_samples("ping", "nas.home.arpa", 0).is_ok());
        let tls = probe_samples("tls", EXPIRING_CERT_HOST, 1_790_000_000_000).unwrap();
        let days = tls.iter().find(|s| s.metric == "probe_ssl_cert_expiry_days").unwrap();
        assert!(days.value < 14.0, "l'alerte d'échéance doit se déclencher");
    }
}
