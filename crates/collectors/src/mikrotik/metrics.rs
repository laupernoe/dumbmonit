//! Réponses de RouterOS rendues en échantillons.
//!
//! L'API REST rend tout en chaînes : `"cpu-load":"4"`, `"running":"true"`,
//! `"uptime":"1w2d3h4m5s"`. Les enregistrements sont donc lus comme des
//! dictionnaires, et chaque valeur convertie ici, avec tolérance : un champ
//! absent ou illisible ne produit pas de série, il ne fait pas échouer la
//! collecte.

use std::collections::BTreeMap;

use dumbmonit_proto::{MetricKind, Sample};
use serde_json::Value;

pub type Record = BTreeMap<String, Value>;

fn gauge(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Gauge, ts_ms)
}

fn counter(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Counter, ts_ms)
}

fn flag(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

/// Le texte d'un champ, espaces retirés ; `None` s'il est absent ou vide.
pub fn text<'a>(record: &'a Record, key: &str) -> Option<&'a str> {
    record.get(key).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty())
}

/// Un nombre, écrit en chaîne ou en nombre JSON.
pub fn number(record: &Record, key: &str) -> Option<f64> {
    match record.get(key)? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
    .filter(|v| v.is_finite())
}

/// Un booléen : `"true"` / `"false"`, ou `"yes"` / `"no"` selon les menus.
pub fn boolean(record: &Record, key: &str) -> Option<bool> {
    match record.get(key)? {
        Value::Bool(b) => Some(*b),
        Value::String(s) => match s.trim() {
            "true" | "yes" => Some(true),
            "false" | "no" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// Une durée RouterOS : `1w2d3h4m5s`, `4m2s`, `850ms`, ou l'ancienne forme
/// `2d03:04:05`.
pub fn parse_duration(raw: &str) -> Option<f64> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    if raw.contains(':') {
        let (days, clock) = match raw.rfind(['d', 'w']) {
            Some(i) => (parse_duration(&raw[..=i])?, &raw[i + 1..]),
            None => (0.0, raw),
        };
        let mut seconds = 0.0;
        for part in clock.split(':') {
            seconds = seconds * 60.0 + part.parse::<f64>().ok()?;
        }
        return Some(days + seconds);
    }
    let mut total = 0.0;
    let mut digits = String::new();
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() || c == '.' {
            digits.push(c);
            continue;
        }
        let value: f64 = digits.parse().ok()?;
        digits.clear();
        let unit = match c {
            'w' => 604_800.0,
            'd' => 86_400.0,
            'h' => 3_600.0,
            'm' if chars.peek() == Some(&'s') => {
                chars.next();
                0.001
            }
            'm' => 60.0,
            's' => 1.0,
            'u' if chars.peek() == Some(&'s') => {
                chars.next();
                0.000_001
            }
            _ => return None,
        };
        total += value * unit;
    }
    if !digits.is_empty() {
        // Un nombre nu : des secondes.
        total += digits.parse::<f64>().ok()?;
    }
    Some(total)
}

/// `7.23.7 (long-term)` → (`7.23.7`, `long-term`).
pub fn split_version(raw: &str) -> (String, Option<String>) {
    let raw = raw.trim();
    match raw.split_once(" (") {
        Some((version, rest)) => {
            (version.trim().to_string(), Some(rest.trim_end_matches(')').trim().to_string()))
        }
        None => (raw.to_string(), None),
    }
}

fn percent_used(total: Option<f64>, free: Option<f64>) -> Option<f64> {
    match (total, free) {
        (Some(total), Some(free)) if total > 0.0 => Some(100.0 * (total - free) / total),
        _ => None,
    }
}

// ------------------------------------------------------------ système

/// `/system/resource`, et le nom donné au routeur (`/system/identity`).
pub fn resource_samples(resource: &Record, identity: Option<&str>, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let mut info = gauge("mikrotik_info", 1.0, ts_ms);
    if let Some(raw) = text(resource, "version") {
        let (version, release) = split_version(raw);
        info = info.with_label("version", version);
        if let Some(release) = release {
            info = info.with_label("release", release);
        }
    }
    for (key, label) in [("board-name", "board"), ("architecture-name", "architecture")] {
        if let Some(value) = text(resource, key) {
            info = info.with_label(label, value);
        }
    }
    if let Some(identity) = identity.map(str::trim).filter(|i| !i.is_empty()) {
        info = info.with_label("identity", identity);
    }
    out.push(info);

    let mut push = |name: &str, value: Option<f64>| {
        if let Some(value) = value {
            out.push(gauge(name, value, ts_ms));
        }
    };
    push("mikrotik_uptime_seconds", text(resource, "uptime").and_then(parse_duration));
    push("mikrotik_cpu_load_percent", number(resource, "cpu-load"));
    push("mikrotik_cpu_count", number(resource, "cpu-count"));
    let (mem_total, mem_free) = (number(resource, "total-memory"), number(resource, "free-memory"));
    push("mikrotik_memory_total_bytes", mem_total);
    push("mikrotik_memory_free_bytes", mem_free);
    push("mikrotik_memory_used_percent", percent_used(mem_total, mem_free));
    let (hdd_total, hdd_free) =
        (number(resource, "total-hdd-space"), number(resource, "free-hdd-space"));
    push("mikrotik_storage_total_bytes", hdd_total);
    push("mikrotik_storage_free_bytes", hdd_free);
    push("mikrotik_storage_used_percent", percent_used(hdd_total, hdd_free));
    out
}

// ------------------------------------------------------------ mises à jour

/// `/system/package/update`. `latest-version` n'y figure qu'après une
/// vérification faite par le routeur lui-même (`check-for-updates`, qu'un
/// compte en lecture ne peut pas lancer) : sans elle, seule la version
/// installée est connue, et `mikrotik_update_checked` vaut 0.
pub fn update_samples(update: &Record, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let installed = text(update, "installed-version");
    let latest = text(update, "latest-version");
    let status = text(update, "status").unwrap_or_default();
    let failed = status.to_ascii_uppercase().starts_with("ERROR");
    out.push(gauge("mikrotik_update_checked", flag(latest.is_some() && !failed), ts_ms));
    if let (Some(installed), Some(latest)) = (installed, latest)
        && !failed
    {
        let mut sample = gauge("mikrotik_update_available", flag(installed != latest), ts_ms)
            .with_label("installed_version", installed)
            .with_label("latest_version", latest);
        if let Some(channel) = text(update, "channel") {
            sample = sample.with_label("channel", channel);
        }
        out.push(sample);
    }
    out
}

// ------------------------------------------------------------ RouterBOARD

/// `/system/routerboard`, absent d'un CHR ou d'une machine x86.
pub fn routerboard_samples(board: Option<&Record>, ts_ms: i64) -> Vec<Sample> {
    let is_board = board.and_then(|b| boolean(b, "routerboard")).unwrap_or(false);
    let mut out = vec![gauge("mikrotik_routerboard", flag(is_board), ts_ms)];
    let Some(board) = board.filter(|_| is_board) else { return out };
    if let (Some(current), Some(upgrade)) =
        (text(board, "current-firmware"), text(board, "upgrade-firmware"))
    {
        let mut sample =
            gauge("mikrotik_firmware_upgrade_pending", flag(current != upgrade), ts_ms)
                .with_label("current_firmware", current)
                .with_label("upgrade_firmware", upgrade);
        if let Some(model) = text(board, "model") {
            sample = sample.with_label("model", model);
        }
        out.push(sample);
    }
    out
}

// ------------------------------------------------------------ capteurs

/// Ce qu'un capteur mesure, d'après son unité (RouterOS 7) ou son nom
/// (l'ancienne forme en un seul objet).
fn sensor_metric(name: &str, unit: Option<&str>) -> Option<&'static str> {
    let unit = unit.map(str::trim).unwrap_or_default();
    match unit {
        "C" | "°C" => return Some("mikrotik_temperature_celsius"),
        "V" => return Some("mikrotik_voltage_volts"),
        "RPM" => return Some("mikrotik_fan_rpm"),
        "W" => return Some("mikrotik_power_watts"),
        "A" => return Some("mikrotik_current_amperes"),
        _ => {}
    }
    if name.ends_with("-state") {
        return Some("mikrotik_health_ok");
    }
    if !unit.is_empty() {
        return None;
    }
    if name.contains("temperature") {
        Some("mikrotik_temperature_celsius")
    } else if name.contains("voltage") {
        Some("mikrotik_voltage_volts")
    } else if name.contains("fan") && name.ends_with("speed") {
        Some("mikrotik_fan_rpm")
    } else if name.contains("power-consumption") {
        Some("mikrotik_power_watts")
    } else {
        None
    }
}

fn sensor_sample(name: &str, value: &str, unit: Option<&str>, ts_ms: i64) -> Option<Sample> {
    let metric = sensor_metric(name, unit)?;
    let value = if metric == "mikrotik_health_ok" {
        // `ok`, ou `fail` / `error` : tout ce qui n'est pas « ok » est une panne.
        flag(value.trim().eq_ignore_ascii_case("ok"))
    } else {
        value.trim().parse::<f64>().ok().filter(|v| v.is_finite())?
    };
    Some(gauge(metric, value, ts_ms).with_label("sensor", name))
}

/// `/system/health`. RouterOS 7 rend une liste `{name, value, type}` ; un CHR
/// rend un objet `{"state":"disabled"}` sans aucun capteur ; les versions
/// anciennes un objet dont chaque clé est un capteur.
pub fn health_samples(health: &Value, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    match health {
        Value::Array(rows) => {
            for row in rows {
                let (Some(name), Some(value)) = (
                    row.get("name").and_then(Value::as_str),
                    row.get("value").and_then(Value::as_str),
                ) else {
                    continue;
                };
                let unit = row.get("type").and_then(Value::as_str);
                out.extend(sensor_sample(name, value, unit, ts_ms));
            }
        }
        Value::Object(map) => {
            for (name, value) in map {
                if matches!(name.as_str(), "state" | "state-after-reboot") {
                    continue;
                }
                if let Some(value) = value.as_str() {
                    out.extend(sensor_sample(name, value, None, ts_ms));
                }
            }
        }
        _ => {}
    }
    out
}

// ------------------------------------------------------------ interfaces

/// Champs demandés à `/interface` (`.proplist`) : le reste — adresses MAC,
/// compteurs du chemin rapide, MTU — n'est pas lu.
pub const INTERFACE_FIELDS: &str = "name,type,running,disabled,dynamic,rx-byte,tx-byte,rx-packet,\
     tx-packet,rx-error,tx-error,rx-drop,tx-drop,link-downs";

/// Compteurs d'interface : champ RouterOS → métrique.
const COUNTERS: [(&str, &str); 9] = [
    ("rx-byte", "mikrotik_interface_rx_bytes_total"),
    ("tx-byte", "mikrotik_interface_tx_bytes_total"),
    ("rx-packet", "mikrotik_interface_rx_packets_total"),
    ("tx-packet", "mikrotik_interface_tx_packets_total"),
    ("rx-error", "mikrotik_interface_rx_errors_total"),
    ("tx-error", "mikrotik_interface_tx_errors_total"),
    ("rx-drop", "mikrotik_interface_rx_drops_total"),
    ("tx-drop", "mikrotik_interface_tx_drops_total"),
    ("link-downs", "mikrotik_interface_link_downs_total"),
];

/// Une interface qui mérite une série : ni désactivée (l'administrateur l'a
/// voulue ainsi), ni dynamique (un client PPPoE ou L2TP qui va et vient, et
/// dont le nom change à chaque connexion), ni la boucle locale.
fn watched(interface: &Record) -> bool {
    !boolean(interface, "disabled").unwrap_or(false)
        && !boolean(interface, "dynamic").unwrap_or(false)
        && text(interface, "type") != Some("loopback")
        && text(interface, "name").is_some()
}

/// `/interface` : l'état et les compteurs, au plus `max` interfaces.
pub fn interface_samples(interfaces: &[Record], max: usize, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let candidates: Vec<&Record> = interfaces.iter().filter(|i| watched(i)).collect();
    let skipped = candidates.len().saturating_sub(max);
    for interface in candidates.into_iter().take(max) {
        let name = text(interface, "name").unwrap_or_default();
        let kind = text(interface, "type").unwrap_or("unknown");
        let label = |sample: Sample| sample.with_label("interface", name).with_label("type", kind);
        if let Some(running) = boolean(interface, "running") {
            out.push(label(gauge("mikrotik_interface_running", flag(running), ts_ms)));
        }
        for (field, metric) in COUNTERS {
            if let Some(value) = number(interface, field) {
                out.push(label(counter(metric, value, ts_ms)));
            }
        }
    }
    out.push(gauge("mikrotik_interfaces_skipped", skipped as f64, ts_ms));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const RESOURCE: &str = include_str!("testdata/chr_7.23.7/system_resource.json");
    const IDENTITY: &str = include_str!("testdata/chr_7.23.7/system_identity.json");
    const UPDATE_AVAILABLE: &str = include_str!("testdata/chr_7.23.7/update_available.json");
    const UPDATE_CURRENT: &str = include_str!("testdata/chr_7.23.7/update_current.json");
    const HEALTH_CHR: &str = include_str!("testdata/chr_7.23.7/system_health.json");
    const INTERFACES: &str = include_str!("testdata/chr_7.23.7/interface.json");
    /// Pas une capture : un routeur physique reconstitué d'après le format
    /// documenté par MikroTik (`/system/health` en liste `{name, value, type}`
    /// de RouterOS 7, `/system/routerboard` d'un CCR), le CHR n'ayant ni
    /// capteur ni RouterBOARD.
    const HEALTH_BOARD: &str = include_str!("testdata/documented/system_health.json");
    const ROUTERBOARD: &str = include_str!("testdata/documented/system_routerboard.json");

    fn record(json: &str) -> Record {
        serde_json::from_str(json).unwrap()
    }

    fn find<'a>(samples: &'a [Sample], name: &str) -> Option<&'a Sample> {
        samples.iter().find(|s| s.metric == name)
    }

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        find(samples, name).map(|s| s.value)
    }

    fn labelled(samples: &[Sample], name: &str, key: &str, wanted: &str) -> Option<f64> {
        samples
            .iter()
            .find(|s| s.metric == name && s.labels.get(key).map(String::as_str) == Some(wanted))
            .map(|s| s.value)
    }

    #[test]
    fn les_durees_de_routeros() {
        assert_eq!(parse_duration("2m9s"), Some(129.0));
        assert_eq!(parse_duration("1w2d3h4m5s"), Some(604_800.0 + 2.0 * 86_400.0 + 11_045.0));
        assert_eq!(parse_duration("850ms"), Some(0.85));
        assert_eq!(parse_duration("2d03:04:05"), Some(2.0 * 86_400.0 + 11_045.0));
        assert_eq!(parse_duration("00:00:30"), Some(30.0));
        assert_eq!(parse_duration("42"), Some(42.0));
        assert_eq!(parse_duration("bientôt"), None);
        assert_eq!(parse_duration(""), None);
    }

    #[test]
    fn la_version_et_sa_branche() {
        assert_eq!(
            split_version("7.23.7 (long-term)"),
            ("7.23.7".into(), Some("long-term".into()))
        );
        assert_eq!(split_version("7.24.4"), ("7.24.4".into(), None));
    }

    #[test]
    fn systeme_d_un_chr_reel() {
        let identity = record(IDENTITY);
        let samples = resource_samples(&record(RESOURCE), text(&identity, "name"), 0);
        let info = find(&samples, "mikrotik_info").unwrap();
        assert_eq!(info.labels["version"], "7.23.7");
        assert_eq!(info.labels["release"], "long-term");
        assert_eq!(info.labels["architecture"], "x86_64");
        assert_eq!(info.labels["identity"], "edge-router");
        assert!(info.labels["board"].starts_with("CHR"));
        assert_eq!(value(&samples, "mikrotik_cpu_count"), Some(4.0));
        assert_eq!(value(&samples, "mikrotik_memory_total_bytes"), Some(536_870_912.0));
        let memory = value(&samples, "mikrotik_memory_used_percent").unwrap();
        assert!((0.0..100.0).contains(&memory), "{memory}");
        let storage = value(&samples, "mikrotik_storage_used_percent").unwrap();
        assert!((0.0..100.0).contains(&storage), "{storage}");
        assert!(value(&samples, "mikrotik_uptime_seconds").unwrap() > 0.0);
        assert!(value(&samples, "mikrotik_cpu_load_percent").is_some());
    }

    #[test]
    fn mise_a_jour_disponible_sur_la_branche_stable() {
        let samples = update_samples(&record(UPDATE_AVAILABLE), 0);
        assert_eq!(value(&samples, "mikrotik_update_checked"), Some(1.0));
        let update = find(&samples, "mikrotik_update_available").unwrap();
        assert_eq!(update.value, 1.0);
        assert_eq!(update.labels["installed_version"], "7.23.7");
        assert_eq!(update.labels["latest_version"], "7.24.4");
        assert_eq!(update.labels["channel"], "stable");
    }

    #[test]
    fn a_jour_sur_la_branche_long_terme() {
        let samples = update_samples(&record(UPDATE_CURRENT), 0);
        assert_eq!(value(&samples, "mikrotik_update_available"), Some(0.0));
    }

    #[test]
    fn jamais_verifie_ou_verification_en_echec() {
        // Tel que rendu avant toute vérification, puis sans accès à Internet.
        let never = record(
            r#"{"channel":"stable","check-certificate":"yes","installed-version":"7.23.7","ip-version":"auto","mode":"https"}"#,
        );
        let samples = update_samples(&never, 0);
        assert_eq!(value(&samples, "mikrotik_update_checked"), Some(0.0));
        assert!(find(&samples, "mikrotik_update_available").is_none());
        let failed = record(
            r#"{"channel":"stable","installed-version":"7.23.7","latest-version":"7.23.7","status":"ERROR: IPv4: no internet connection\n       IPv6: no internet connection"}"#,
        );
        let samples = update_samples(&failed, 0);
        assert_eq!(value(&samples, "mikrotik_update_checked"), Some(0.0));
        assert!(find(&samples, "mikrotik_update_available").is_none());
    }

    #[test]
    fn un_chr_n_a_ni_capteur_ni_routerboard() {
        let health: Value = serde_json::from_str(HEALTH_CHR).unwrap();
        assert!(health_samples(&health, 0).is_empty());
        let samples = routerboard_samples(None, 0);
        assert_eq!(value(&samples, "mikrotik_routerboard"), Some(0.0));
        assert!(find(&samples, "mikrotik_firmware_upgrade_pending").is_none());
    }

    #[test]
    fn capteurs_d_un_routeur_physique_au_format_documente() {
        let health: Value = serde_json::from_str(HEALTH_BOARD).unwrap();
        let samples = health_samples(&health, 0);
        let t = "mikrotik_temperature_celsius";
        assert_eq!(labelled(&samples, t, "sensor", "cpu-temperature"), Some(58.0));
        assert_eq!(labelled(&samples, t, "sensor", "board-temperature1"), Some(41.0));
        assert_eq!(labelled(&samples, "mikrotik_voltage_volts", "sensor", "voltage"), Some(24.2));
        assert_eq!(labelled(&samples, "mikrotik_fan_rpm", "sensor", "fan1-speed"), Some(5_550.0));
        assert_eq!(labelled(&samples, "mikrotik_health_ok", "sensor", "psu1-state"), Some(1.0));
        assert_eq!(labelled(&samples, "mikrotik_health_ok", "sensor", "psu2-state"), Some(0.0));
        assert_eq!(
            labelled(&samples, "mikrotik_power_watts", "sensor", "power-consumption"),
            Some(38.5)
        );
    }

    #[test]
    fn capteurs_a_l_ancienne_en_un_seul_objet() {
        let health: Value = serde_json::from_str(
            r#"{"voltage":"23.9","temperature":"37","cpu-temperature":"51","fan1-speed":"4200","psu1-state":"fail","state":"enabled"}"#,
        )
        .unwrap();
        let samples = health_samples(&health, 0);
        assert_eq!(samples.len(), 5);
        assert_eq!(labelled(&samples, "mikrotik_health_ok", "sensor", "psu1-state"), Some(0.0));
        assert_eq!(labelled(&samples, "mikrotik_voltage_volts", "sensor", "voltage"), Some(23.9));
    }

    #[test]
    fn firmware_en_retard_sur_la_version_installee() {
        let board = record(ROUTERBOARD);
        let samples = routerboard_samples(Some(&board), 0);
        assert_eq!(value(&samples, "mikrotik_routerboard"), Some(1.0));
        let pending = find(&samples, "mikrotik_firmware_upgrade_pending").unwrap();
        assert_eq!(pending.value, 1.0);
        assert_eq!(pending.labels["current_firmware"], "7.19.4");
        assert_eq!(pending.labels["upgrade_firmware"], "7.23.7");
        assert!(!pending.labels.contains_key("serial-number"));
    }

    #[test]
    fn interfaces_reelles_sans_la_boucle_ni_les_desactivees() {
        let interfaces: Vec<Record> = serde_json::from_str(INTERFACES).unwrap();
        let samples = interface_samples(&interfaces, 64, 0);
        let names: std::collections::BTreeSet<&str> =
            samples.iter().filter_map(|s| s.labels.get("interface").map(String::as_str)).collect();
        assert_eq!(names.into_iter().collect::<Vec<_>>(), ["bridge-lan", "ether1", "ether2"]);
        let rx = samples
            .iter()
            .find(|s| {
                s.metric == "mikrotik_interface_rx_bytes_total" && s.labels["interface"] == "ether1"
            })
            .unwrap();
        assert!(rx.value > 0.0);
        assert_eq!(rx.kind, MetricKind::Counter);
        assert_eq!(rx.labels["type"], "ether");
        assert_eq!(
            labelled(&samples, "mikrotik_interface_running", "interface", "ether2"),
            Some(1.0)
        );
        assert_eq!(value(&samples, "mikrotik_interfaces_skipped"), Some(0.0));
        for s in samples.iter().filter(|s| s.kind == MetricKind::Counter) {
            assert!(s.metric.ends_with("_total"), "{}", s.metric);
        }
    }

    #[test]
    fn au_dela_de_la_limite_les_interfaces_sont_comptees() {
        let interfaces: Vec<Record> = serde_json::from_str(INTERFACES).unwrap();
        let samples = interface_samples(&interfaces, 1, 0);
        assert_eq!(value(&samples, "mikrotik_interfaces_skipped"), Some(2.0));
        let dynamic: Vec<Record> = serde_json::from_str(
            r#"[{"name":"<pppoe-client7>","type":"pppoe-in","dynamic":"true","running":"true","disabled":"false"}]"#,
        )
        .unwrap();
        assert_eq!(interface_samples(&dynamic, 64, 0).len(), 1, "seul le compteur d'ignorées");
    }
}
