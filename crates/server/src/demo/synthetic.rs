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
use std::sync::{Arc, OnceLock};

use async_trait::async_trait;
use dumbmonit_proto::{
    Collector, Fingerprint, KindDescription, Measures, MetricKind, ProbeError, Sample, Target,
};

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
pub fn noise(seed: u64, bucket: i64) -> f64 {
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

/// Heure pleine du démarrage du processus : l'origine des tendances relatives
/// (remplissage des disques, échéances qui se rapprochent). Fixée une fois, elle
/// rend les mesures en direct et l'historique rétroactif cohérents entre eux, et
/// deux démarrages racontent la même histoire à quelques heures près.
pub fn anchor_ms() -> i64 {
    static ANCHOR: OnceLock<i64> = OnceLock::new();
    *ANCHOR.get_or_init(|| chrono::Utc::now().timestamp_millis().div_euclid(3_600_000) * 3_600_000)
}

/// Jours écoulés depuis [`anchor_ms`] (négatif dans le passé).
pub fn days_since_anchor(ts_ms: i64) -> f64 {
    (ts_ms - anchor_ms()) as f64 / 86_400_000.0
}

/// Valeur d'un compteur cumulatif dont le débit moyen est `rate` (unités par
/// seconde) : l'**intégrale exacte** d'un débit qui ondule — un cycle quotidien et
/// un cycle de 45 minutes, toujours positif. `rate()` retrouve donc le débit
/// voulu à chaque instant, sans compteur qui recule ni pic parasite, et
/// l'historique et le direct se raccordent sans état conservé.
pub fn cumulative(seed: u64, rate: f64, ts_ms: i64) -> f64 {
    let t = ((ts_ms - COUNTER_EPOCH_MS).max(0) / 1000) as f64;
    let (w1, w2) = (TAU / 86_400.0, TAU / 2_700.0);
    let phi1 = (seed % 628) as f64 / 100.0;
    let phi2 = ((seed >> 12) % 628) as f64 / 100.0;
    let daily = 0.3 * (phi1.cos() - (w1 * t + phi1).cos()) / w1;
    let quick = 0.15 * (phi2.cos() - (w2 * t + phi2).cos()) / w2;
    rate * (0.6 * t + daily + quick)
}

/// Temps de réponse plausible d'un équipement interrogé par son API : quelques
/// dizaines de millisecondes qui suivent l'activité de la journée.
pub fn device_latency(name: &str, ts_ms: i64) -> f64 {
    let seed = seed(name);
    0.025 + (seed % 9) as f64 * 0.006 + 0.09 * activity(seed, ts_ms).powi(2)
}

pub fn gauge(metric: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(metric, value, MetricKind::Gauge, ts_ms)
}

pub fn counter(metric: &str, value: f64, ts_ms: i64) -> Sample {
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
        let now = chrono::Utc::now().timestamp_millis();
        match super::extras::samples(self.kind, &target.name, now) {
            Some(samples) => Ok(samples),
            None => probe_samples(self.kind, &target.name, now),
        }
    }
}

/// Un vrai collecteur (équipement simulé) auquel la démonstration ajoute un temps
/// de réponse : sans lui, les colonnes « Latency » et « p95 » de la console
/// resteraient vides pour tout ce qui n'est pas une sonde de disponibilité.
pub struct Timed {
    inner: Arc<dyn Collector>,
}

impl Timed {
    pub fn new(inner: Arc<dyn Collector>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl Collector for Timed {
    fn kind(&self) -> &str {
        self.inner.kind()
    }

    fn description(&self) -> Option<KindDescription> {
        self.inner.description()
    }

    fn measures(&self, kind: &str) -> Option<Measures> {
        self.inner.measures(kind)
    }

    fn fingerprints(&self) -> Vec<Fingerprint> {
        self.inner.fingerprints()
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let mut samples = self.inner.probe(target).await?;
        if !samples.iter().any(|s| s.metric == "probe_duration_seconds") {
            let now = chrono::Utc::now().timestamp_millis();
            samples.push(gauge("probe_duration_seconds", device_latency(&target.name, now), now));
        }
        Ok(samples)
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        self.inner.discover(target).await
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
                for (i, header) in [
                    "strict-transport-security",
                    "x-content-type-options",
                    "content-security-policy",
                    "x-frame-options",
                    "frame-ancestors",
                ]
                .into_iter()
                .enumerate()
                {
                    let ok = i < 2 || !(seed >> i).is_multiple_of(3);
                    labelled(
                        gauge("probe_http_security_header", f64::from(u8::from(ok)), ts_ms)
                            .with_label("header", header),
                    );
                }
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

/// Remplissage du volume de données au démarrage, croissance par jour (fraction
/// de la capacité) et durée d'un cycle avant nettoyage, en jours.
const DATA_BASE: f64 = 0.865;
const DATA_GROWTH_PER_DAY: f64 = 0.0115;
const DATA_CYCLE_DAYS: f64 = 8.0;

/// Part occupée du volume de données à l'instant `ts_ms`. Dans le passé, la
/// droite se prolonge sans rupture (30 jours d'historique) ; à partir du
/// démarrage, un nettoyage ramène le volume à son niveau de départ tous les
/// [`DATA_CYCLE_DAYS`] jours.
pub fn data_fraction(ts_ms: i64) -> f64 {
    let days = days_since_anchor(ts_ms);
    let in_cycle = if days < 0.0 { days } else { days.rem_euclid(DATA_CYCLE_DAYS) };
    // Une ondulation d'une journée (les sauvegardes de la nuit) sur la tendance.
    let wave = 0.0012 * (TAU * days).sin();
    (DATA_BASE + DATA_GROWTH_PER_DAY * in_cycle + wave).clamp(0.0, 0.995)
}

const MEMORY_TOTAL: f64 = 32.0 * 1024.0 * 1024.0 * 1024.0;
const DATA_TOTAL: f64 = 1_800.0 * 1_000_000_000.0;
const ROOT_TOTAL: f64 = 120.0 * 1_000_000_000.0;
/// Point de départ des compteurs cumulatifs : une date fixe, pour qu'ils ne
/// repartent jamais de zéro d'un redémarrage à l'autre.
const COUNTER_EPOCH_MS: i64 = 1_767_225_600_000; // 2026-01-01T00:00:00Z

/// Poids du trafic de chaque conteneur, dans l'ordre de [`CONTAINERS`] : le
/// proxy et le serveur de médias en transportent le plus.
const CONTAINER_WEIGHTS: [f64; 6] = [4.0, 2.5, 1.5, 0.4, 6.0, 0.2];

/// Ce que l'agent de [`AGENT_HOST`] enverrait à l'instant `ts_ms`.
pub fn agent_samples(ts_ms: i64) -> Vec<Sample> {
    let host_seed = seed(AGENT_HOST);
    let load = activity(host_seed, ts_ms);
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
        let core_load = activity(host_seed.wrapping_add(core), ts_ms);
        samples.push(
            gauge("cpu_core_usage_percent", 4.0 + 62.0 * core_load, ts_ms)
                .with_label("core", core.to_string()),
        );
    }
    // Le volume de données se remplit d'environ 1 point par jour (≈ 20 Go) et se
    // fait nettoyer tous les huit jours : de quoi nourrir la prévision de
    // remplissage (« plein dans ≈ 12 jours » au démarrage) sans jamais le saturer.
    let data_used = DATA_TOTAL * data_fraction(ts_ms);
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
    // Compteurs : intégrale exacte d'un débit qui ondule (voir [`cumulative`]).
    for (ifname, rx, tx) in [("eth0", 3.2e6, 1.1e6), ("docker0", 4.0e5, 6.0e5)] {
        for (metric, rate) in [("if_octets_in", rx), ("if_octets_out", tx)] {
            let seed = seed(&format!("{ifname}{metric}"));
            samples.push(
                counter(metric, cumulative(seed, rate, ts_ms), ts_ms).with_label("ifname", ifname),
            );
        }
        for (metric, rate) in [("if_packets_in", rx / 900.0), ("if_packets_out", tx / 900.0)] {
            let seed = seed(&format!("{ifname}{metric}"));
            samples.push(
                counter(metric, cumulative(seed, rate, ts_ms), ts_ms).with_label("ifname", ifname),
            );
        }
        for metric in ["if_errors_in", "if_errors_out"] {
            samples.push(counter(metric, 0.0, ts_ms).with_label("ifname", ifname));
        }
    }
    for (metric, rate) in [("disk_read_bytes", 2.1e6), ("disk_written_bytes", 3.4e6)] {
        let seed = seed(&format!("nvme0n1{metric}"));
        samples.push(
            counter(metric, cumulative(seed, rate, ts_ms), ts_ms).with_label("device", "nvme0n1"),
        );
    }
    // Espace pris par Docker : les images grossissent avec les mises à jour.
    let growth = 1.0 + 0.02 * days_since_anchor(ts_ms).rem_euclid(8.0);
    for (metric, gib) in [
        ("container_df_images_bytes", 18.4 * growth),
        ("container_df_containers_bytes", 2.1),
        ("container_df_volumes_bytes", 96.0 * growth),
        ("container_df_build_cache_bytes", 4.2),
        ("container_df_reclaimable_bytes", 11.5 * growth),
    ] {
        samples.push(gauge(metric, gib * 1_073_741_824.0, ts_ms));
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
            // Consommation : une onde lente par conteneur, mémoire sous une limite de 1 Gio.
            let wave = ((ts_ms / 60_000) as f64 / (3.0 + index as f64)).sin().abs();
            let memory = (60.0 + 25.0 * index as f64 + 120.0 * wave) * 1_048_576.0;
            samples.push(labelled(gauge("container_cpu_percent", 2.0 + 18.0 * wave, ts_ms)));
            samples.push(labelled(gauge("container_memory_bytes", memory, ts_ms)));
            samples.push(labelled(gauge("container_memory_limit_bytes", 1_073_741_824.0, ts_ms)));
            samples.push(labelled(gauge(
                "container_memory_percent",
                memory / 1_073_741_824.0 * 100.0,
                ts_ms,
            )));
            let pids = 8.0 + 14.0 * index as f64 + 6.0 * wave;
            samples.push(labelled(gauge("container_pids", pids, ts_ms)));
            // Compteurs par conteneur : réseau et disque, chacun son rythme.
            let weight = CONTAINER_WEIGHTS[index % CONTAINER_WEIGHTS.len()];
            for (metric, rate) in [
                ("container_net_rx_bytes", 420_000.0 * weight),
                ("container_net_tx_bytes", 260_000.0 * weight),
                ("container_block_read_bytes", 90_000.0 * weight),
                ("container_block_write_bytes", 150_000.0 * weight),
            ] {
                let seed = seed(&format!("{name}{metric}"));
                samples.push(labelled(counter(metric, cumulative(seed, rate, ts_ms), ts_ms)));
            }
        }
        // 1 = une mise à jour de l'image existe, 0 = à jour, -1 = inconnu (arrêté).
        let update = match (*running, *name) {
            (false, _) => -1.0,
            (true, "nextcloud" | "jellyfin") => 1.0,
            (true, _) => 0.0,
        };
        samples.push(labelled(gauge("container_update_available", update, ts_ms)));
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

    #[test]
    fn un_compteur_cumulatif_ne_recule_jamais_et_rend_son_debit() {
        let start = 1_790_000_000_000;
        let rate = 2.0e6;
        let mut last = 0.0;
        for step in 0..600 {
            let value = cumulative(42, rate, start + step * 300_000);
            assert!(value >= last, "recul à l'étape {step}");
            last = value;
        }
        // Sur une journée entière, le débit moyen est 0,6 fois le débit nominal.
        let day = cumulative(42, rate, start + 86_400_000) - cumulative(42, rate, start);
        let mean = day / 86_400.0 / rate;
        assert!((mean - 0.6).abs() < 0.02, "{mean}");
        // Et sur cinq minutes, il reste dans la plage de l'onde (0,15 à 1,05).
        for step in 0..300 {
            let at = start + step * 300_000;
            let gained = cumulative(7, rate, at + 300_000) - cumulative(7, rate, at);
            let local = gained / 300.0 / rate;
            assert!((0.14..=1.06).contains(&local), "{local}");
        }
    }

    #[test]
    fn le_volume_de_donnees_se_remplit_et_annonce_son_echeance() {
        let day = 86_400_000;
        let now = anchor_ms();
        // Dans le passé, une droite : 1,15 point par jour (à l'ondulation près).
        let before = data_fraction(now - 7 * day);
        let after = data_fraction(now);
        let per_day = (after - before) / 7.0;
        assert!((per_day - DATA_GROWTH_PER_DAY).abs() < 0.002, "{per_day}");
        // Au démarrage il reste une dizaine de jours : sous le seuil des 30 jours
        // de la règle livrée, et sans être déjà plein.
        let days_to_full = (1.0 - after) / per_day;
        assert!((8.0..16.0).contains(&days_to_full), "{days_to_full}");
        // Un nettoyage ramène le volume au niveau de départ, sans le dépasser.
        for step in 0..400 {
            let fraction = data_fraction(now + step * 3_600_000);
            assert!((0.5..0.995).contains(&fraction), "{fraction}");
        }
        assert!(data_fraction(now + 9 * day) < data_fraction(now + 7 * day));
    }

    #[test]
    fn l_agent_publie_des_compteurs_par_conteneur() {
        let samples = agent_samples(anchor_ms());
        for metric in [
            "container_net_rx_bytes",
            "container_net_tx_bytes",
            "container_block_read_bytes",
            "container_block_write_bytes",
        ] {
            let running = CONTAINERS.iter().filter(|(_, _, up)| *up).count();
            let found: Vec<_> = samples.iter().filter(|s| s.metric == metric).collect();
            assert_eq!(found.len(), running, "{metric}");
            assert!(found.iter().all(|s| s.kind == MetricKind::Counter && s.value > 0.0));
        }
        assert!(samples.iter().any(|s| s.metric == "container_df_volumes_bytes"));
        // Le volume de données est celui de la prévision.
        let used = samples
            .iter()
            .find(|s| {
                s.metric == "filesystem_used_percent"
                    && s.labels.get("mountpoint").is_some_and(|m| m == "/srv/data")
            })
            .unwrap();
        assert!((used.value - 100.0 * data_fraction(anchor_ms())).abs() < 1e-6);
    }

    #[test]
    fn un_equipement_simule_a_un_temps_de_reponse() {
        for step in 0..200 {
            let latency = device_latency("pve.home.arpa", 1_790_000_000_000 + step * 300_000);
            assert!((0.02..0.3).contains(&latency), "{latency}");
        }
    }
}
