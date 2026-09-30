//! Traduction du modèle UniFi en échantillons.
//!
//! | Métrique | Valeurs |
//! |---|---|
//! | `device_state` | 0 en ligne, 1 hors ligne, 2 en attente d'adoption, 3 mise à jour, 4 adoption en cours, 5 adoption échouée, 6 isolé, 7 autre |
//! | `device_up` | 1 en ligne ou en mise à jour, 0 hors ligne, isolé ou adoption échouée ; absent pour un équipement pas encore adopté |
//! | `subsystem_status` | 0 ok, 1 avertissement, 2 erreur, 3 inconnu |

use std::collections::BTreeMap;

use dumbmonit_proto::{MetricKind, Sample};

use super::model::{Device, DeviceState, Health};

/// Au-delà, les équipements ne portent plus de série propre.
pub const MAX_DEVICES: usize = 500;

fn gauge(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Gauge, ts_ms)
}

fn flag(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

pub fn version_sample(version: &str, api: &str, ts_ms: i64) -> Sample {
    gauge("unifi_info", 1.0, ts_ms).with_label("version", version).with_label("api", api)
}

pub fn device_samples(devices: &[Device], ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let mut counts: BTreeMap<&str, f64> =
        DeviceState::ALL.iter().map(|s| (s.word(), 0.0)).collect();
    let mut upgradable = 0.0;
    for device in devices {
        *counts.entry(device.state.word()).or_default() += 1.0;
        if device.upgradable == Some(true) && device.state != DeviceState::Pending {
            upgradable += 1.0;
        }
    }
    for (state, count) in counts {
        out.push(gauge("unifi_devices", count, ts_ms).with_label("state", state));
    }
    out.push(gauge("unifi_devices_upgradable", upgradable, ts_ms));

    for device in devices.iter().take(MAX_DEVICES) {
        let label = |sample: Sample| {
            sample
                .with_label("device", device.name.as_str())
                .with_label("mac", device.mac.as_str())
                .with_label("model", device.model.as_str())
                .with_label("type", device.kind.word())
        };
        out.push(label(gauge("unifi_device_state", device.state.code(), ts_ms)));
        // Un équipement en attente n'est pas encore à nous : ni « en ligne » ni
        // « en panne ».
        if !matches!(
            device.state,
            DeviceState::Pending | DeviceState::Adopting | DeviceState::Other
        ) {
            out.push(label(gauge("unifi_device_up", flag(!device.state.is_down()), ts_ms)));
        }
        if let Some(upgradable) = device.upgradable
            && device.state != DeviceState::Pending
        {
            let mut sample = label(gauge("unifi_device_upgradable", flag(upgradable), ts_ms));
            if let Some(firmware) = &device.firmware {
                sample = sample.with_label("firmware", firmware.as_str());
            }
            out.push(sample);
        }
        // Les statistiques d'un équipement hors ligne sont les dernières connues.
        if device.state == DeviceState::Online {
            for (name, value) in [
                ("unifi_device_uptime_seconds", device.uptime_seconds),
                ("unifi_device_cpu_percent", device.cpu_percent),
                ("unifi_device_memory_percent", device.memory_percent),
                ("unifi_device_clients", device.clients),
            ] {
                if let Some(value) = value {
                    out.push(label(gauge(name, value, ts_ms)));
                }
            }
            for (wan, up) in &device.wan_links {
                out.push(
                    label(gauge("unifi_wan_link_up", flag(*up), ts_ms))
                        .with_label("wan", wan.as_str()),
                );
            }
        }
    }
    out
}

fn status_code(status: &str) -> f64 {
    match status {
        "ok" => 0.0,
        "warning" | "warn" => 1.0,
        "error" => 2.0,
        _ => 3.0,
    }
}

pub fn health_samples(health: &Health, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    for (subsystem, status) in &health.subsystems {
        out.push(
            gauge("unifi_subsystem_status", status_code(status), ts_ms)
                .with_label("subsystem", subsystem.as_str()),
        );
    }
    if let Some(ok) = health.wan_ok {
        out.push(gauge("unifi_wan_up", flag(ok), ts_ms));
    }
    if let Some(ok) = health.internet_ok {
        out.push(gauge("unifi_internet_up", flag(ok), ts_ms));
    }
    if let Some(latency) = health.latency_ms {
        out.push(gauge("unifi_internet_latency_seconds", latency / 1000.0, ts_ms));
    }
    for (wan, availability) in &health.wan_availability {
        out.push(
            gauge("unifi_wan_availability_percent", *availability, ts_ms)
                .with_label("wan", wan.to_ascii_lowercase()),
        );
    }
    let types = [
        ("wireless", health.clients_wireless),
        ("wired", health.clients_wired),
        ("guest", health.clients_guest),
        ("vpn", health.clients_vpn),
    ];
    for (kind, value) in types {
        if let Some(value) = value {
            out.push(gauge("unifi_clients_by_type", value, ts_ms).with_label("type", kind));
        }
    }
    out
}

pub fn clients_sample(total: f64, ts_ms: i64) -> Sample {
    gauge("unifi_clients", total, ts_ms)
}

pub fn alarms_sample(count: f64, ts_ms: i64) -> Sample {
    gauge("unifi_alarms", count, ts_ms)
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::super::model::{Classic, classic_device, health};
    use super::*;

    fn devices() -> Vec<Device> {
        let classic: Classic<Value> =
            serde_json::from_str(include_str!("testdata/documented/classic_stat_device.json"))
                .unwrap();
        classic.data.iter().filter_map(classic_device).collect()
    }

    fn find<'a>(samples: &'a [Sample], metric: &str, key: &str, value: &str) -> Option<&'a Sample> {
        samples
            .iter()
            .find(|s| s.metric == metric && s.labels.get(key).map(String::as_str) == Some(value))
    }

    #[test]
    fn les_equipements_sont_comptes_et_nommes() {
        let samples = device_samples(&devices(), 0);
        assert_eq!(find(&samples, "unifi_devices", "state", "online").unwrap().value, 2.0);
        assert_eq!(find(&samples, "unifi_devices", "state", "offline").unwrap().value, 1.0);
        assert_eq!(find(&samples, "unifi_devices", "state", "pending").unwrap().value, 1.0);
        assert_eq!(find(&samples, "unifi_devices", "state", "isolated").unwrap().value, 1.0);
        assert_eq!(
            samples.iter().find(|s| s.metric == "unifi_devices_upgradable").unwrap().value,
            1.0
        );
        let ap = find(&samples, "unifi_device_up", "device", "Living room AP").unwrap();
        assert_eq!(ap.value, 0.0);
        assert_eq!(ap.labels["type"], "access_point");
        assert_eq!(find(&samples, "unifi_device_up", "device", "Garden AP").unwrap().value, 0.0);
        // Pas de « en panne » pour un équipement qui attend d'être adopté.
        assert!(find(&samples, "unifi_device_up", "device", "74:83:c2:00:00:04").is_none());
        assert_eq!(
            find(&samples, "unifi_device_state", "device", "74:83:c2:00:00:04").unwrap().value,
            2.0
        );
    }

    #[test]
    fn la_charge_et_les_liens_wan_ne_valent_que_pour_un_equipement_en_ligne() {
        let samples = device_samples(&devices(), 0);
        let cpu = find(&samples, "unifi_device_cpu_percent", "device", "Gateway").unwrap();
        assert_eq!(cpu.value, 12.4);
        assert_eq!(cpu.labels["type"], "gateway");
        assert!(find(&samples, "unifi_device_cpu_percent", "device", "Garden AP").is_none());
        assert_eq!(find(&samples, "unifi_wan_link_up", "wan", "wan2").unwrap().value, 0.0);
        let upgrade = find(&samples, "unifi_device_upgradable", "device", "Gateway").unwrap();
        assert_eq!(upgrade.labels["firmware"], "4.1.13.9913");
    }

    #[test]
    fn la_sante_devient_des_etats_et_des_clients() {
        let classic: Classic<Value> =
            serde_json::from_str(include_str!("testdata/documented/classic_stat_health.json"))
                .unwrap();
        let samples = health_samples(&health(&classic.data), 0);
        assert_eq!(
            find(&samples, "unifi_subsystem_status", "subsystem", "wlan").unwrap().value,
            2.0
        );
        assert_eq!(samples.iter().find(|s| s.metric == "unifi_wan_up").unwrap().value, 1.0);
        assert_eq!(samples.iter().find(|s| s.metric == "unifi_internet_up").unwrap().value, 1.0);
        let latency =
            samples.iter().find(|s| s.metric == "unifi_internet_latency_seconds").unwrap();
        assert!((latency.value - 0.013).abs() < 1e-9);
        assert_eq!(
            find(&samples, "unifi_clients_by_type", "type", "wireless").unwrap().value,
            27.0
        );
        assert_eq!(
            find(&samples, "unifi_wan_availability_percent", "wan", "wan").unwrap().value,
            99.87
        );
    }
}
