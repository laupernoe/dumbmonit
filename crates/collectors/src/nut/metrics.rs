//! Variables NUT → échantillons DumbMonit.
//!
//! Les noms de variables sont ceux de la nomenclature NUT (« NUT command and
//! variable naming scheme ») : un pilote ne publie que ce que l'onduleur sait
//! dire, et une variable absente ne produit simplement pas de série.
//!
//! Toutes les séries portent l'étiquette `ups` (le nom de l'onduleur dans
//! `ups.conf`). Aucun numéro de série n'est repris dans une étiquette.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use dumbmonit_proto::{MetricKind, Sample};

/// Les drapeaux de `ups.status`, chacun rendu en une jauge 0/1.
///
/// Tous sont émis dès que `ups.status` existe, à 0 quand le drapeau est
/// absent : une règle `… > 0` retombe ainsi d'elle-même au retour du secteur,
/// au lieu d'attendre la disparition d'une série.
pub const STATUS_FLAGS: &[(&str, &str)] = &[
    ("OL", "nut_ups_on_line"),
    ("OB", "nut_ups_on_battery"),
    ("LB", "nut_ups_low_battery"),
    ("HB", "nut_ups_high_battery"),
    ("RB", "nut_ups_replace_battery"),
    ("CHRG", "nut_ups_charging"),
    ("DISCHRG", "nut_ups_discharging"),
    ("BYPASS", "nut_ups_bypass"),
    ("CAL", "nut_ups_calibrating"),
    ("OFF", "nut_ups_off"),
    ("OVER", "nut_ups_overload"),
    ("TRIM", "nut_ups_trim"),
    ("BOOST", "nut_ups_boost"),
    ("FSD", "nut_ups_forced_shutdown"),
    ("ALARM", "nut_ups_alarm"),
];

/// Variables numériques reprises telles quelles : (variable NUT, métrique).
const NUMERIC: &[(&str, &str)] = &[
    ("battery.charge", "nut_battery_charge_percent"),
    ("battery.charge.low", "nut_battery_charge_low_percent"),
    ("battery.runtime", "nut_battery_runtime_seconds"),
    ("battery.runtime.low", "nut_battery_runtime_low_seconds"),
    ("battery.voltage", "nut_battery_voltage_volts"),
    ("battery.temperature", "nut_battery_temperature_celsius"),
    ("input.voltage", "nut_input_voltage_volts"),
    ("input.voltage.nominal", "nut_input_voltage_nominal_volts"),
    ("input.frequency", "nut_input_frequency_hertz"),
    ("output.voltage", "nut_output_voltage_volts"),
    ("output.frequency", "nut_output_frequency_hertz"),
    ("ups.load", "nut_ups_load_percent"),
    ("ups.realpower", "nut_ups_realpower_watts"),
    ("ups.realpower.nominal", "nut_ups_realpower_nominal_watts"),
    ("ups.power", "nut_ups_power_va"),
    ("ups.power.nominal", "nut_ups_power_nominal_va"),
    ("ups.temperature", "nut_ups_temperature_celsius"),
];

/// Longueur maximale d'une valeur reprise en étiquette.
const MAX_LABEL: usize = 64;

fn gauge(name: &str, value: f64, ts_ms: i64, ups: &str) -> Sample {
    Sample::new(name, value, MetricKind::Gauge, ts_ms).with_label("ups", ups)
}

fn label_value(raw: &str) -> String {
    raw.trim().chars().take(MAX_LABEL).collect()
}

/// Un nombre NUT : `231.0`, `100`, parfois suivi d'une unité chez de vieux
/// pilotes (`27.3 V`).
fn number(raw: &str) -> Option<f64> {
    raw.split_whitespace().next()?.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Date de batterie : `2021/03/15`, `2021-03-15`, ou la forme américaine des
/// pilotes APC, `07/15/19` ou `07/15/2019`.
pub fn parse_battery_date(raw: &str) -> Option<NaiveDate> {
    let raw = raw.trim();
    let parts: Vec<&str> = raw.split(['/', '-', '.']).collect();
    if parts.len() != 3
        || parts.iter().any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    let n = |i: usize| parts[i].parse::<u32>().ok();
    if parts[0].len() == 4 {
        return NaiveDate::from_ymd_opt(n(0)? as i32, n(1)?, n(2)?);
    }
    let year = match parts[2].len() {
        4 => n(2)? as i32,
        2 => 2000 + n(2)? as i32,
        _ => return None,
    };
    NaiveDate::from_ymd_opt(year, n(0)?, n(1)?)
}

/// `ups.test.result` : vrai si le dernier autotest signale un problème.
/// `None` pour « aucun test », « en cours » ou une phrase inconnue.
pub fn self_test_failed(raw: &str) -> Option<bool> {
    let text = raw.to_ascii_lowercase();
    if text.contains("error") || text.contains("fail") || text.contains("warning") {
        Some(true)
    } else if text.contains("passed") {
        Some(false)
    } else {
        None
    }
}

/// Les échantillons d'un onduleur dont `upsd` a rendu les variables.
pub fn ups_samples(
    ups: &str,
    description: &str,
    vars: &[(String, String)],
    now: DateTime<Utc>,
) -> Vec<Sample> {
    let ts_ms = now.timestamp_millis();
    let map: HashMap<&str, &str> = vars.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let get = |key: &str| map.get(key).copied().map(str::trim).filter(|v| !v.is_empty());
    let mut out = vec![
        gauge("nut_ups_driver_connected", 1.0, ts_ms, ups),
        gauge("nut_ups_data_stale", 0.0, ts_ms, ups),
    ];

    let mut info = gauge("nut_ups_info", 1.0, ts_ms, ups);
    for (label, keys) in [
        ("manufacturer", &["device.mfr", "ups.mfr"][..]),
        ("model", &["device.model", "ups.model"][..]),
        ("firmware", &["ups.firmware"][..]),
        ("driver", &["driver.name"][..]),
        ("battery_type", &["battery.type"][..]),
    ] {
        if let Some(value) = keys.iter().find_map(|k| get(k)) {
            info = info.with_label(label, label_value(value));
        }
    }
    if !description.trim().is_empty() {
        info = info.with_label("description", label_value(description));
    }
    out.push(info);

    if let Some(status) = get("ups.status") {
        let flags: Vec<&str> = status.split_whitespace().collect();
        for (flag, metric) in STATUS_FLAGS {
            let set = flags.contains(flag);
            out.push(gauge(metric, if set { 1.0 } else { 0.0 }, ts_ms, ups));
        }
        out.push(
            gauge("nut_ups_status_info", 1.0, ts_ms, ups).with_label("status", label_value(status)),
        );
    }

    for (key, metric) in NUMERIC {
        if let Some(value) = get(key).and_then(number) {
            out.push(gauge(metric, value, ts_ms, ups));
        }
    }

    // La date de pose ou de remplacement dit l'âge du jeu de batteries ; à
    // défaut, sa date de fabrication.
    let dated =
        get("battery.date").and_then(parse_battery_date).map(|d| (d, "installed")).or_else(|| {
            get("battery.mfr.date").and_then(parse_battery_date).map(|d| (d, "manufactured"))
        });
    if let Some((date, source)) = dated {
        let seconds = (now.date_naive() - date).num_seconds();
        if seconds >= 0 {
            out.push(
                gauge("nut_battery_age_seconds", seconds as f64, ts_ms, ups)
                    .with_label("source", source),
            );
        }
    }

    if let Some(result) = get("ups.test.result")
        && let Some(failed) = self_test_failed(result)
    {
        out.push(
            gauge("nut_ups_self_test_failed", if failed { 1.0 } else { 0.0 }, ts_ms, ups)
                .with_label("result", label_value(result)),
        );
    }
    out
}

/// Un onduleur que `upsd` connaît mais dont il n'a pas de données fraîches :
/// pilote arrêté (`DRIVER-NOT-CONNECTED`) ou onduleur muet (`DATA-STALE`).
pub fn stale_samples(
    ups: &str,
    description: &str,
    driver_connected: bool,
    ts_ms: i64,
) -> Vec<Sample> {
    let mut info = gauge("nut_ups_info", 1.0, ts_ms, ups);
    if !description.trim().is_empty() {
        info = info.with_label("description", label_value(description));
    }
    vec![
        gauge("nut_ups_driver_connected", if driver_connected { 1.0 } else { 0.0 }, ts_ms, ups),
        gauge("nut_ups_data_stale", 1.0, ts_ms, ups),
        info,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nut::protocol::tokenize;

    /// Lit une transcription `LIST VAR` capturée sur `upsd` 2.8.2.
    fn vars(transcript: &str) -> Vec<(String, String)> {
        transcript
            .lines()
            .filter_map(|line| {
                let mut words = tokenize(line).unwrap();
                (words.len() == 4 && words[0] == "VAR")
                    .then(|| (std::mem::take(&mut words[2]), std::mem::take(&mut words[3])))
            })
            .collect()
    }

    fn now() -> DateTime<Utc> {
        "2026-09-30T12:00:00Z".parse().unwrap()
    }

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    #[test]
    fn un_onduleur_sur_secteur() {
        let s = ups_samples(
            "rack",
            "Rack UPS",
            &vars(include_str!("testdata/var_rack_online.txt")),
            now(),
        );
        assert_eq!(value(&s, "nut_ups_on_line"), Some(1.0));
        assert_eq!(value(&s, "nut_ups_on_battery"), Some(0.0));
        assert_eq!(value(&s, "nut_ups_low_battery"), Some(0.0));
        assert_eq!(value(&s, "nut_battery_charge_percent"), Some(100.0));
        assert_eq!(value(&s, "nut_battery_runtime_seconds"), Some(2400.0));
        assert_eq!(value(&s, "nut_battery_runtime_low_seconds"), Some(120.0));
        assert_eq!(value(&s, "nut_input_voltage_volts"), Some(231.0));
        assert_eq!(value(&s, "nut_ups_load_percent"), Some(32.0));
        assert_eq!(value(&s, "nut_ups_realpower_watts"), Some(350.0));
        assert_eq!(value(&s, "nut_ups_realpower_nominal_watts"), Some(1100.0));
        assert_eq!(value(&s, "nut_ups_self_test_failed"), Some(0.0));
        assert_eq!(value(&s, "nut_ups_data_stale"), Some(0.0));
        // Posée le 15 mars 2021 : cinq ans et demi plus tard.
        let age = s.iter().find(|s| s.metric == "nut_battery_age_seconds").unwrap();
        assert_eq!(age.labels["source"], "installed");
        assert!((age.value / 86_400.0 - 2025.0).abs() < 1.0, "{} jours", age.value / 86_400.0);
        let info = s.iter().find(|s| s.metric == "nut_ups_info").unwrap();
        assert_eq!(info.labels["manufacturer"], "Eaton");
        assert_eq!(info.labels["model"], "5P 1550");
        assert_eq!(info.labels["description"], "Rack UPS");
        // Aucun numéro de série en étiquette.
        for sample in &s {
            assert!(sample.labels.values().all(|v| !v.contains("G123A45678")), "{}", sample.metric);
            assert_eq!(sample.labels["ups"], "rack");
        }
    }

    #[test]
    fn une_coupure_puis_une_batterie_basse() {
        let s =
            ups_samples("rack", "", &vars(include_str!("testdata/var_rack_on_battery.txt")), now());
        assert_eq!(value(&s, "nut_ups_on_battery"), Some(1.0));
        assert_eq!(value(&s, "nut_ups_discharging"), Some(1.0));
        assert_eq!(value(&s, "nut_ups_on_line"), Some(0.0));
        assert_eq!(value(&s, "nut_ups_low_battery"), Some(0.0));
        assert_eq!(value(&s, "nut_input_voltage_volts"), Some(0.0));

        let s = ups_samples(
            "rack",
            "",
            &vars(include_str!("testdata/var_rack_low_battery.txt")),
            now(),
        );
        assert_eq!(value(&s, "nut_ups_on_battery"), Some(1.0));
        assert_eq!(value(&s, "nut_ups_low_battery"), Some(1.0));
        assert_eq!(value(&s, "nut_battery_charge_percent"), Some(9.0));
        assert_eq!(value(&s, "nut_battery_runtime_seconds"), Some(95.0));
        let status = s.iter().find(|s| s.metric == "nut_ups_status_info").unwrap();
        assert_eq!(status.labels["status"], "OB DISCHRG LB");
    }

    #[test]
    fn batterie_a_remplacer_et_derivation() {
        let s = ups_samples(
            "office",
            "",
            &vars(include_str!("testdata/var_office_replace_bypass.txt")),
            now(),
        );
        assert_eq!(value(&s, "nut_ups_replace_battery"), Some(1.0));
        assert_eq!(value(&s, "nut_ups_bypass"), Some(1.0));
        assert_eq!(value(&s, "nut_ups_on_line"), Some(1.0));
        // Ni date de batterie ni autotest : pas de série, pas de zéro inventé.
        assert!(value(&s, "nut_battery_age_seconds").is_none());
        assert!(value(&s, "nut_ups_self_test_failed").is_none());
        let info = s.iter().find(|s| s.metric == "nut_ups_info").unwrap();
        assert_eq!(info.labels["manufacturer"], "CyberPower");
    }

    #[test]
    fn les_dates_de_batterie_usuelles() {
        let d = |y, m, d| NaiveDate::from_ymd_opt(y, m, d);
        assert_eq!(parse_battery_date("2021/03/15"), d(2021, 3, 15));
        assert_eq!(parse_battery_date("2021-03-15"), d(2021, 3, 15));
        assert_eq!(parse_battery_date("07/15/19"), d(2019, 7, 15));
        assert_eq!(parse_battery_date("07/15/2019"), d(2019, 7, 15));
        assert_eq!(parse_battery_date("not set"), None);
        assert_eq!(parse_battery_date("13/45/2019"), None);
    }

    #[test]
    fn les_autotests() {
        assert_eq!(self_test_failed("Done and passed"), Some(false));
        assert_eq!(self_test_failed("Done and warning"), Some(true));
        assert_eq!(self_test_failed("Done and error"), Some(true));
        assert_eq!(self_test_failed("No test initiated"), None);
        assert_eq!(self_test_failed("In progress"), None);
    }

    #[test]
    fn un_onduleur_muet_reste_visible() {
        let s = stale_samples("office", "Office UPS", false, 0);
        assert_eq!(value(&s, "nut_ups_data_stale"), Some(1.0));
        assert_eq!(value(&s, "nut_ups_driver_connected"), Some(0.0));
        assert!(value(&s, "nut_ups_on_battery").is_none());
    }
}
