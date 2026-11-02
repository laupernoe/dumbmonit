//! Ports de commutateur, au même schéma que `unifi_port_*`, pour les API
//! infonuagiques (Omada, Aruba Central) : le panneau de commutateur de
//! l'interface lit `<genre>_port_*` sans connaître le fabricant.
//!
//! Étiquettes : `device`, `mac` (clé de l'unité : l'adresse MAC, ou le numéro
//! de série quand l'API n'en donne pas), `port`, `port_name`.

use dumbmonit_proto::Sample;
use serde_json::Value;

use crate::rest::{counter, flag, gauge, number, text};

/// Équipements dont les ports sont publiés : au-delà, la façade d'un parc
/// entier coûterait des dizaines de milliers de séries.
pub const MAX_PORTED_DEVICES: usize = 32;
/// Ports publiés par équipement.
pub const MAX_PORTS_PER_DEVICE: usize = 64;

#[derive(Debug, Default, Clone)]
pub struct Port {
    pub index: String,
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub up: Option<bool>,
    pub speed_bps: Option<f64>,
    pub full_duplex: Option<bool>,
    pub uplink: bool,
    pub poe_enabled: Option<bool>,
    pub poe_watts: Option<f64>,
    pub rx_bytes: Option<f64>,
    pub tx_bytes: Option<f64>,
    pub rx_errors: Option<f64>,
    pub tx_errors: Option<f64>,
    /// Voisin LLDP : (nom ou châssis, port distant).
    pub neighbor: Option<(String, Option<String>)>,
}

/// Les ports d'un équipement, déjà lus dans la réponse de l'API.
#[derive(Debug, Clone)]
pub struct DevicePorts {
    pub device: String,
    pub unit: String,
    pub ports: Vec<Port>,
}

/// Premier champ présent parmi plusieurs noms (les API varient d'une version
/// à l'autre).
pub fn pick<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|key| value.get(*key).filter(|v| !v.is_null()))
}

pub fn pick_text<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| text(value, key))
}

pub fn pick_number(value: &Value, keys: &[&str]) -> Option<f64> {
    number(pick(value, keys))
}

/// Un booléen écrit en booléen, en 0/1 ou en texte.
pub fn pick_bool(value: &Value, keys: &[&str]) -> Option<bool> {
    match pick(value, keys)? {
        Value::Bool(b) => Some(*b),
        Value::String(s) => match s.to_ascii_lowercase().as_str() {
            "true" | "on" | "up" | "enabled" | "yes" => Some(true),
            "false" | "off" | "down" | "disabled" | "no" => Some(false),
            _ => None,
        },
        other => number(Some(other)).map(|n| n > 0.0),
    }
}

pub fn samples(prefix: &str, devices: &[DevicePorts], ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    for device in devices.iter().filter(|d| !d.ports.is_empty()).take(MAX_PORTED_DEVICES) {
        for port in device.ports.iter().take(MAX_PORTS_PER_DEVICE) {
            let label = |sample: Sample| {
                let sample = sample
                    .with_label("device", device.device.as_str())
                    .with_label("mac", device.unit.as_str())
                    .with_label("port", port.index.as_str());
                match &port.name {
                    Some(name) => sample.with_label("port_name", name.as_str()),
                    None => sample,
                }
            };
            let m = |suffix: &str| format!("{prefix}_port_{suffix}");
            let enabled = port.enabled.unwrap_or(true);
            out.push(label(flag(&m("enabled"), enabled, ts_ms)));
            if !enabled {
                continue;
            }
            if let Some(up) = port.up {
                out.push(label(flag(&m("up"), up, ts_ms)));
            }
            if port.up == Some(true) {
                if let Some(speed) = port.speed_bps.filter(|s| *s > 0.0) {
                    out.push(label(gauge(&m("speed_bps"), speed, ts_ms)));
                }
                if let Some(full) = port.full_duplex {
                    out.push(label(flag(&m("full_duplex"), full, ts_ms)));
                }
            }
            if port.uplink {
                out.push(label(gauge(&m("uplink"), 1.0, ts_ms)));
            }
            if let Some(poe) = port.poe_enabled {
                out.push(label(flag(&m("poe_enabled"), poe, ts_ms)));
                if poe && let Some(watts) = port.poe_watts {
                    out.push(label(gauge(&m("poe_watts"), watts, ts_ms)));
                }
            }
            for (suffix, value) in [
                ("rx_bytes_total", port.rx_bytes),
                ("tx_bytes_total", port.tx_bytes),
                ("rx_errors_total", port.rx_errors),
                ("tx_errors_total", port.tx_errors),
            ] {
                if let Some(value) = value {
                    out.push(label(counter(&m(suffix), value, ts_ms)));
                }
            }
            if let Some((chassis, remote)) = &port.neighbor {
                let mut sample = label(gauge(&m("neighbor_info"), 1.0, ts_ms))
                    .with_label("remote_chassis", chassis.as_str());
                if let Some(remote) = remote {
                    sample = sample.with_label("remote_port", remote.as_str());
                }
                out.push(sample);
            }
        }
    }
    out
}
