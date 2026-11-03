//! Mesures de profondeur de l'API classique : ce que `stat/device` et `stat/sta`
//! disent en plus de l'état des équipements (satisfaction, température, budget
//! PoE, radios Wi-Fi, test de débit, qualité du signal des clients).
//!
//! Noms de champs de l'API classique non documentée par Ubiquiti, tels que les
//! relèvent les clients communautaires (aiounifi, unpoller) : chacun reste
//! facultatif, un champ absent ne produit simplement pas de série.
//!
//! | Métrique | Valeurs |
//! |---|---|
//! | `device_satisfaction_percent` | score d'expérience de l'équipement (0-100), absent si le contrôleur n'en calcule pas (`-1`) |
//! | `device_overheating` | 1 si l'équipement se déclare en surchauffe |
//! | `device_poe_budget_watts` / `device_poe_used_watts` | budget d'un commutateur PoE et somme de ses ports |
//! | `radio_channel_utilization_percent` | occupation du canal d'une radio (`ng`, `na`, `6e`) |
//! | `wifi_clients_weak_signal` | clients Wi-Fi dont le signal est sous -75 dBm |

use dumbmonit_proto::{MetricKind, Sample};
use serde_json::Value;

use super::model::{DeviceState, classic_device};

/// Signal en dessous duquel un client Wi-Fi décroche et ralentit (dBm).
pub const WEAK_SIGNAL_DBM: f64 = -75.0;

fn gauge(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Gauge, ts_ms)
}

fn num(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Mesures supplémentaires des équipements en ligne, lues dans `stat/device`.
pub fn device_samples(raw: &[Value], ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    for entry in raw.iter().take(super::metrics::MAX_DEVICES) {
        let Some(device) = classic_device(entry) else { continue };
        // Les chiffres d'un équipement hors ligne sont les derniers connus.
        if device.state != DeviceState::Online {
            continue;
        }
        let label = |sample: Sample| {
            sample
                .with_label("device", device.name.as_str())
                .with_label("mac", device.mac.as_str())
                .with_label("model", device.model.as_str())
                .with_label("type", device.kind.word())
        };
        if let Some(score) = num(entry.get("satisfaction")).filter(|s| *s >= 0.0) {
            out.push(label(gauge("unifi_device_satisfaction_percent", score, ts_ms)));
        }
        if let Some(celsius) = num(entry.get("general_temperature")).filter(|t| *t > 0.0) {
            out.push(label(gauge("unifi_device_temperature_celsius", celsius, ts_ms)));
        }
        if let Some(hot) = entry.get("overheating").and_then(Value::as_bool) {
            out.push(label(gauge("unifi_device_overheating", f64::from(u8::from(hot)), ts_ms)));
        }
        if let Some(budget) = num(entry.get("total_max_power")).filter(|b| *b > 0.0) {
            let used: f64 = entry
                .get("port_table")
                .and_then(Value::as_array)
                .map(|ports| {
                    ports
                        .iter()
                        .filter(|p| p.get("poe_enable").and_then(Value::as_bool) == Some(true))
                        .filter_map(|p| num(p.get("poe_power")))
                        .sum()
                })
                .unwrap_or(0.0);
            out.push(label(gauge("unifi_device_poe_budget_watts", budget, ts_ms)));
            out.push(label(gauge("unifi_device_poe_used_watts", used, ts_ms)));
        }
        let radios = entry.get("radio_table_stats").and_then(Value::as_array);
        for radio in radios.into_iter().flatten() {
            let Some(band) = radio.get("radio").and_then(Value::as_str) else { continue };
            let with_band = |sample: Sample| label(sample).with_label("radio", band);
            if let Some(busy) = num(radio.get("cu_total")) {
                out.push(with_band(gauge("unifi_radio_channel_utilization_percent", busy, ts_ms)));
            }
            if let Some(clients) = num(radio.get("num_sta")) {
                out.push(with_band(gauge("unifi_radio_clients", clients, ts_ms)));
            }
        }
        // Dernier test de débit de la passerelle (Mbit/s et millisecondes).
        if let Some(test) = entry.get("speedtest-status") {
            if let Some(down) = num(test.get("xput_download")).filter(|v| *v > 0.0) {
                out.push(label(gauge("unifi_speedtest_download_bps", down * 1e6, ts_ms)));
            }
            if let Some(up) = num(test.get("xput_upload")).filter(|v| *v > 0.0) {
                out.push(label(gauge("unifi_speedtest_upload_bps", up * 1e6, ts_ms)));
            }
            if let Some(ping) = num(test.get("latency")).filter(|v| *v > 0.0) {
                out.push(label(gauge("unifi_speedtest_latency_seconds", ping / 1000.0, ts_ms)));
            }
        }
    }
    out
}

/// Qualité du signal des clients Wi-Fi connectés, lue dans `stat/sta` : des
/// agrégats, pas une série par client (un réseau en compte des centaines).
pub fn wifi_client_samples(stations: &[Value], ts_ms: i64) -> Vec<Sample> {
    let mut total = 0.0;
    let mut with_signal = 0.0;
    let mut weak = 0.0;
    let mut sum = 0.0;
    for station in stations {
        if station.get("is_wired").and_then(Value::as_bool).unwrap_or(false) {
            continue;
        }
        total += 1.0;
        // `signal` est en dBm (négatif) ; 0 veut dire « non mesuré ».
        if let Some(dbm) = num(station.get("signal")).filter(|s| *s < 0.0) {
            with_signal += 1.0;
            sum += dbm;
            if dbm < WEAK_SIGNAL_DBM {
                weak += 1.0;
            }
        }
    }
    let mut out = vec![
        gauge("unifi_wifi_clients", total, ts_ms),
        gauge("unifi_wifi_clients_weak_signal", weak, ts_ms),
    ];
    if with_signal > 0.0 {
        out.push(gauge("unifi_wifi_signal_dbm_avg", sum / with_signal, ts_ms));
    }
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn find<'a>(samples: &'a [Sample], metric: &str) -> Option<&'a Sample> {
        samples.iter().find(|s| s.metric == metric)
    }

    #[test]
    fn un_commutateur_poe_dit_son_budget_et_sa_consommation() {
        let raw = vec![json!({
            "mac": "74:83:c2:00:00:10", "name": "Switch PoE", "model": "USW-24-POE",
            "type": "usw", "state": 1, "adopted": true,
            "satisfaction": 100, "general_temperature": 52, "overheating": false,
            "total_max_power": 95,
            "port_table": [
                {"port_idx": 1, "poe_enable": true, "poe_power": "6.5"},
                {"port_idx": 2, "poe_enable": true, "poe_power": "12.0"},
                {"port_idx": 3, "poe_enable": false, "poe_power": "0.00"}
            ]
        })];
        let samples = device_samples(&raw, 0);
        assert_eq!(find(&samples, "unifi_device_poe_budget_watts").unwrap().value, 95.0);
        assert_eq!(find(&samples, "unifi_device_poe_used_watts").unwrap().value, 18.5);
        assert_eq!(find(&samples, "unifi_device_satisfaction_percent").unwrap().value, 100.0);
        assert_eq!(find(&samples, "unifi_device_temperature_celsius").unwrap().value, 52.0);
        assert_eq!(find(&samples, "unifi_device_overheating").unwrap().value, 0.0);
        let used = find(&samples, "unifi_device_poe_used_watts").unwrap();
        assert_eq!(used.labels["type"], "switch");
    }

    #[test]
    fn les_radios_et_le_test_de_debit_sont_lus() {
        let raw = vec![json!({
            "mac": "74:83:c2:00:00:11", "name": "Living room AP", "model": "U6-Pro",
            "type": "uap", "state": 1, "adopted": true, "satisfaction": -1,
            "radio_table_stats": [
                {"name": "wifi0", "radio": "ng", "cu_total": 31, "num_sta": 7},
                {"name": "wifi1", "radio": "na", "cu_total": 84, "num_sta": 12}
            ],
            "speedtest-status": {"xput_download": 480.5, "xput_upload": 40.0, "latency": 12}
        })];
        let samples = device_samples(&raw, 0);
        // Satisfaction -1 : le contrôleur n'en calcule pas.
        assert!(find(&samples, "unifi_device_satisfaction_percent").is_none());
        let busy: Vec<_> = samples
            .iter()
            .filter(|s| s.metric == "unifi_radio_channel_utilization_percent")
            .collect();
        assert_eq!(busy.len(), 2);
        assert_eq!(busy.iter().find(|s| s.labels["radio"] == "na").unwrap().value, 84.0);
        assert_eq!(find(&samples, "unifi_speedtest_download_bps").unwrap().value, 480.5e6);
        let ping = find(&samples, "unifi_speedtest_latency_seconds").unwrap().value;
        assert!((ping - 0.012).abs() < 1e-9);
    }

    #[test]
    fn un_equipement_hors_ligne_ne_donne_rien() {
        let raw = vec![json!({
            "mac": "74:83:c2:00:00:12", "name": "Garden AP", "type": "uap", "state": 0,
            "adopted": true, "satisfaction": 80
        })];
        assert!(device_samples(&raw, 0).is_empty());
    }

    #[test]
    fn les_clients_wifi_au_signal_faible_sont_comptes() {
        let stations = vec![
            json!({"mac": "a", "is_wired": false, "signal": -52}),
            json!({"mac": "b", "is_wired": false, "signal": -80}),
            json!({"mac": "c", "is_wired": false, "signal": -78}),
            json!({"mac": "d", "is_wired": true}),
            json!({"mac": "e", "is_wired": false}),
        ];
        let samples = wifi_client_samples(&stations, 0);
        assert_eq!(find(&samples, "unifi_wifi_clients").unwrap().value, 4.0);
        assert_eq!(find(&samples, "unifi_wifi_clients_weak_signal").unwrap().value, 2.0);
        let avg = find(&samples, "unifi_wifi_signal_dbm_avg").unwrap().value;
        assert!((avg - (-70.0)).abs() < 1e-9);
    }
}
