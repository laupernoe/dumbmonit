//! Scans de découverte planifiés et inventaire du réseau.
//!
//! Un scan planifié balaie un sous-réseau à intervalle fixe, avec le moteur de
//! découverte existant ([`crate::api::discovery::start_scan`] : serveur ou agent
//! relais, mêmes garde-fous de taille). Le résultat est rapproché de
//! l'inventaire ([`diff`]) et rangé ([`store`]) :
//!
//! - le premier balayage qui trouve quelque chose initialise l'inventaire sans
//!   rien signaler ;
//! - ensuite, un appareil ou un port absent y entre « inconnu » ;
//! - tant qu'un élément est inconnu, une mesure par élément est publiée
//!   (`dumbmonit_netscan_unknown_device`, `dumbmonit_netscan_unknown_port`) : ce
//!   sont elles que lisent les règles d'alerte livrées, désactivées par défaut.
//!   Marquer l'élément connu (ou l'ignorer) arrête la mesure, et l'alerte se
//!   résout ;
//! - un appareil nouveau qui prend l'IP d'un appareil connu sous une autre MAC
//!   publie en plus `dumbmonit_netscan_mac_change` (usurpation ARP ou matériel
//!   remplacé), jusqu'à ce qu'il soit marqué connu ou ignoré.
//!
//! Pas de chevauchement : un sous-réseau n'a jamais deux balayages en même
//! temps, et le registre de découverte n'en tient qu'un par exécutant.

pub mod diff;
pub mod store;

use std::collections::HashMap;
use std::time::Duration;

use dumbmonit_proto::{MetricKind, Sample};
use tokio::time::{MissedTickBehavior, interval};
use tracing::{debug, info, warn};

use crate::api::ApiError;
use crate::collectors::discovery::JobStatus;
use crate::state::AppState;
use store::{Schedule, Unknown};

/// Granularité de la boucle : lancement des balayages échus, rangement des
/// balayages finis, republication des mesures.
const TICK: Duration = Duration::from_secs(15);

/// Intervalle minimal entre deux balayages d'un même sous-réseau.
pub const MIN_INTERVAL_SECS: i64 = 300;
/// Intervalle maximal : trente jours.
pub const MAX_INTERVAL_SECS: i64 = 30 * 24 * 3600;
/// Nombre maximal de scans planifiés.
pub const MAX_SCHEDULES: i64 = 50;

/// Mesure d'un appareil inconnu.
pub const METRIC_UNKNOWN_DEVICE: &str = "netscan_unknown_device";
/// Mesure d'un port inconnu sur un appareil connu.
pub const METRIC_UNKNOWN_PORT: &str = "netscan_unknown_port";
/// Mesure d'un changement de MAC pour une IP (usurpation ou remplacement).
pub const METRIC_MAC_CHANGE: &str = "netscan_mac_change";

const TIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

/// Lance la boucle des scans planifiés.
pub fn spawn(state: AppState) {
    tokio::spawn(run(state));
}

async fn run(state: AppState) {
    // Un balayage en cours à l'arrêt du serveur n'existe plus : il ne doit pas
    // rester « en cours » pour toujours.
    if let Err(error) = store::reset_interrupted(&state.pool).await {
        warn!(?error, "cannot reset interrupted network scans");
    }
    let mut runs: HashMap<i64, String> = HashMap::new();
    let mut ticker = interval(TICK);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        if let Err(error) = tick(&state, &mut runs).await {
            warn!(?error, "scheduled network scans: cycle failed");
        }
    }
}

/// Un tour : range les balayages finis, lance les échus, publie les mesures.
async fn tick(state: &AppState, runs: &mut HashMap<i64, String>) -> anyhow::Result<()> {
    settle(state, runs).await;

    let now = chrono::Utc::now().naive_utc();
    for schedule in store::list_schedules(&state.pool).await? {
        if !schedule.enabled || runs.contains_key(&schedule.id) || !is_due(&schedule, now) {
            continue;
        }
        let started = crate::api::discovery::start_scan(
            state,
            &schedule.cidr,
            schedule.agent_id,
            "public".to_string(),
            schedule.depth,
        )
        .await;
        match started {
            Ok(job_id) => {
                store::record_run_start(&state.pool, schedule.id).await?;
                info!(schedule = schedule.id, cidr = %schedule.cidr, "scheduled network scan started");
                runs.insert(schedule.id, job_id);
            }
            // L'exécutant est occupé (balayage manuel, autre scan planifié) :
            // on réessaie au prochain tour.
            Err(ApiError::Conflict(_)) => {
                debug!(schedule = schedule.id, "scheduled network scan postponed: target busy")
            }
            Err(error) => {
                let (_, message) = error.into_parts();
                store::record_run_start(&state.pool, schedule.id).await?;
                store::record_run_end(&state.pool, schedule.id, "failed", Some(&message)).await?;
            }
        }
    }

    let items = store::unknown_items(&state.pool).await?;
    if !items.is_empty() {
        state.sink.send(unknown_samples(&items, chrono::Utc::now().timestamp_millis())).await;
    }
    Ok(())
}

/// Vrai si le scan doit repartir : jamais lancé, ou intervalle écoulé.
fn is_due(schedule: &Schedule, now: chrono::NaiveDateTime) -> bool {
    let Some(last) = schedule
        .last_run_at
        .as_deref()
        .and_then(|raw| chrono::NaiveDateTime::parse_from_str(raw, TIME_FORMAT).ok())
    else {
        return true;
    };
    now.signed_duration_since(last).num_seconds() >= schedule.interval_secs
}

/// Range le résultat des balayages terminés dans l'inventaire.
async fn settle(state: &AppState, runs: &mut HashMap<i64, String>) {
    let pending: Vec<(i64, String)> = runs.iter().map(|(id, job)| (*id, job.clone())).collect();
    for (schedule_id, job_id) in pending {
        let (status, error) = match state.discovery().get(&job_id) {
            None => ("failed", Some("the scan result is no longer available".to_string())),
            Some(job) => match job.summary.status {
                JobStatus::Running => continue,
                JobStatus::Done => {
                    match store::apply_scan(&state.pool, schedule_id, &job.hosts).await {
                        Ok(applied) => {
                            info!(
                                schedule = schedule_id,
                                hosts = job.hosts.len(),
                                initialized = applied.initialized,
                                new_devices = applied.new_devices,
                                new_ports = applied.new_ports,
                                mac_changes = applied.mac_changes,
                                "scheduled network scan stored"
                            );
                            ("done", None)
                        }
                        Err(error) => {
                            warn!(schedule = schedule_id, ?error, "cannot store the scan result");
                            ("failed", Some("cannot store the scan result".to_string()))
                        }
                    }
                }
                JobStatus::Failed => ("failed", job.error),
                JobStatus::Cancelled => ("cancelled", None),
            },
        };
        runs.remove(&schedule_id);
        if let Err(error) =
            store::record_run_end(&state.pool, schedule_id, status, error.as_deref()).await
        {
            warn!(schedule = schedule_id, ?error, "cannot record the end of the scan");
        }
    }
}

/// Une mesure par élément inconnu, valant 1, avec des étiquettes stables : l'identité
/// d'une alerte en dépend, une IP ou un nom qui change ne doit pas la renouveler.
pub fn unknown_samples(items: &[Unknown], ts_ms: i64) -> Vec<Sample> {
    items
        .iter()
        .map(|item| match item {
            Unknown::Device { schedule_id, cidr, ident } => {
                Sample::new(METRIC_UNKNOWN_DEVICE, 1.0, MetricKind::Gauge, ts_ms)
                    .with_label("scan", schedule_id.to_string())
                    .with_label("subnet", cidr.as_str())
                    .with_label("device", ident.as_str())
            }
            Unknown::Port { schedule_id, cidr, ident, port, protocol } => {
                Sample::new(METRIC_UNKNOWN_PORT, 1.0, MetricKind::Gauge, ts_ms)
                    .with_label("scan", schedule_id.to_string())
                    .with_label("subnet", cidr.as_str())
                    .with_label("device", ident.as_str())
                    .with_label("port", port.to_string())
                    .with_label("protocol", protocol.as_str())
            }
            Unknown::MacChange { schedule_id, cidr, ident, previous_mac } => {
                Sample::new(METRIC_MAC_CHANGE, 1.0, MetricKind::Gauge, ts_ms)
                    .with_label("scan", schedule_id.to_string())
                    .with_label("subnet", cidr.as_str())
                    .with_label("device", ident.as_str())
                    .with_label("previous_mac", previous_mac.as_str())
            }
        })
        .collect()
}

/// Vérifie un sous-réseau et le rend sous sa forme canonique (adresse de
/// réseau), ou le message d'erreur à renvoyer.
pub fn canonical_cidr(raw: &str) -> Result<String, ApiError> {
    crate::api::discovery::check_network(raw)?;
    let network: ipnet::IpNet = raw
        .trim()
        .parse()
        .map_err(|_| ApiError::BadRequest(format!("Invalid network \"{raw}\".")))?;
    Ok(network.trunc().to_string())
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDateTime;

    use super::*;

    fn schedule(last_run_at: Option<&str>, interval_secs: i64) -> Schedule {
        Schedule {
            id: 1,
            name: String::new(),
            cidr: "10.0.0.0/24".to_string(),
            agent_id: None,
            interval_secs,
            depth: dumbmonit_proto::ScanDepth::Standard,
            enabled: true,
            initialized: false,
            last_run_at: last_run_at.map(str::to_string),
            last_status: None,
            last_error: None,
        }
    }

    fn at(raw: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(raw, TIME_FORMAT).unwrap()
    }

    #[test]
    fn un_scan_jamais_lance_est_du() {
        assert!(is_due(&schedule(None, 3600), at("2026-01-01T10:00:00")));
    }

    #[test]
    fn un_scan_est_du_une_fois_l_intervalle_ecoule() {
        let schedule = schedule(Some("2026-01-01T10:00:00"), 3600);
        assert!(!is_due(&schedule, at("2026-01-01T10:59:59")));
        assert!(is_due(&schedule, at("2026-01-01T11:00:00")));
    }

    #[test]
    fn les_mesures_ont_des_etiquettes_stables_par_element() {
        let items = [
            Unknown::Device {
                schedule_id: 4,
                cidr: "10.0.0.0/24".into(),
                ident: "mac:aa:bb".into(),
            },
            Unknown::Port {
                schedule_id: 4,
                cidr: "10.0.0.0/24".into(),
                ident: "mac:aa:bb".into(),
                port: 3389,
                protocol: "tcp".into(),
            },
        ];
        let samples = unknown_samples(&items, 1_000);
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].metric, METRIC_UNKNOWN_DEVICE);
        assert_eq!(samples[0].labels.get("device").map(String::as_str), Some("mac:aa:bb"));
        assert_eq!(samples[1].metric, METRIC_UNKNOWN_PORT);
        assert_eq!(samples[1].labels.get("port").map(String::as_str), Some("3389"));
        // Ni IP ni nom d'hôte : ils changent sans que l'élément soit nouveau.
        assert!(samples.iter().all(|s| !s.labels.contains_key("ip")));
    }

    #[test]
    fn un_changement_de_mac_publie_sa_mesure() {
        let items = [Unknown::MacChange {
            schedule_id: 4,
            cidr: "10.0.0.0/24".into(),
            ident: "mac:bb".into(),
            previous_mac: "aa".into(),
        }];
        let samples = unknown_samples(&items, 1_000);
        assert_eq!(samples[0].metric, METRIC_MAC_CHANGE);
        assert_eq!(samples[0].labels.get("previous_mac").map(String::as_str), Some("aa"));
        assert!(!samples[0].labels.contains_key("ip"));
    }

    #[test]
    fn le_sous_reseau_est_ramene_a_son_adresse_de_reseau() {
        assert_eq!(canonical_cidr(" 192.168.1.77/24 ").ok().as_deref(), Some("192.168.1.0/24"));
        assert!(canonical_cidr("nope").is_err());
        assert!(canonical_cidr("10.0.0.0/8").is_err());
    }
}
