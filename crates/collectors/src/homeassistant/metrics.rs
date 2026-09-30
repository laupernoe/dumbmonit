//! Traduction de `/api/config`, `/api/states` et des réparations en échantillons.
//!
//! Home Assistant ne sait pas dire « quelque chose ne va pas » d'une seule
//! voix : une intégration qui décroche laisse ses entités `unavailable`, une
//! pile qui s'épuise baisse doucement, une mise à jour attend dans une entité
//! `update.*`. Ce module lit l'état de toutes les entités en un appel et en
//! tire des totaux par domaine, plus une série nommée pour ce qui demande une
//! action (pile faible, mise à jour, réparation, entité indisponible).

use std::collections::{BTreeMap, BTreeSet};

use dumbmonit_proto::{MetricKind, Sample};
use serde::Deserialize;
use serde_json::Value;

use super::websocket::Issues;

/// Au plus autant d'entités nommées par famille (indisponibles, piles faibles,
/// mises à jour, réparations) : au-delà, seuls les totaux restent.
pub const MAX_NAMED: usize = 50;

/// Domaines dont l'état normal, au repos, est `unknown` : un bouton jamais
/// pressé, un événement jamais reçu, un service de notification ou de synthèse
/// vocale. Les compter comme « inconnus » noierait les vrais.
pub const RESTING_UNKNOWN: &[&str] = &[
    "button",
    "input_button",
    "event",
    "scene",
    "notify",
    "tts",
    "stt",
    "conversation",
    "wake_word",
    "ai_task",
    "person",
];

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub safe_mode: Option<bool>,
    #[serde(default)]
    pub recovery_mode: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct State {
    pub entity_id: String,
    pub state: String,
    #[serde(default)]
    pub attributes: serde_json::Map<String, Value>,
}

impl State {
    fn domain(&self) -> &str {
        self.entity_id.split_once('.').map_or(self.entity_id.as_str(), |(domain, _)| domain)
    }

    fn attribute(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).and_then(Value::as_str)
    }

    fn name(&self) -> &str {
        self.attribute("friendly_name").unwrap_or(&self.entity_id)
    }
}

/// Ce que l'utilisateur a réglé sur la cible.
#[derive(Debug, Clone)]
pub struct Tuning {
    /// Une pile en dessous de ce pourcentage est faible.
    pub battery_threshold: f64,
    /// Domaines laissés hors des totaux d'entités indisponibles et inconnues.
    pub exclude_domains: Vec<String>,
}

fn gauge(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Gauge, ts_ms)
}

fn flag(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

/// `2026.9.4` : la version, et si le cœur tourne ou démarre encore.
pub fn config_samples(config: &Config, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    if let Some(version) = &config.version {
        out.push(gauge("homeassistant_info", 1.0, ts_ms).with_label("version", version.as_str()));
    }
    if let Some(state) = &config.state {
        out.push(gauge(
            "homeassistant_running",
            flag(state.eq_ignore_ascii_case("RUNNING")),
            ts_ms,
        ));
    }
    // Le mode sans échec a été remplacé par le mode de récupération : les deux
    // disent la même chose, la configuration n'a pas pu être chargée.
    let degraded = config.safe_mode.unwrap_or(false) || config.recovery_mode.unwrap_or(false);
    out.push(gauge("homeassistant_recovery_mode", flag(degraded), ts_ms));
    out
}

pub fn state_samples(states: &[State], tuning: &Tuning, ts_ms: i64) -> Vec<Sample> {
    let mut totals = BTreeMap::<&str, f64>::new();
    let mut unavailable = BTreeMap::<&str, f64>::new();
    let mut unknown = BTreeMap::<&str, f64>::new();
    let mut named_unavailable = Vec::new();
    let mut batteries = 0.0;
    let mut low = Vec::new();
    let mut updates = Vec::new();
    let excluded: BTreeSet<&str> = tuning.exclude_domains.iter().map(String::as_str).collect();

    for state in states {
        let domain = state.domain();
        *totals.entry(domain).or_default() += 1.0;
        let counted = !excluded.contains(domain);
        match state.state.as_str() {
            "unavailable" if counted => {
                *unavailable.entry(domain).or_default() += 1.0;
                named_unavailable.push(state);
            }
            "unknown" if counted && !RESTING_UNKNOWN.contains(&domain) => {
                *unknown.entry(domain).or_default() += 1.0;
            }
            _ => {}
        }
        if state.attribute("device_class") == Some("battery") {
            match domain {
                "sensor" => {
                    batteries += 1.0;
                    if let Ok(level) = state.state.trim().parse::<f64>()
                        && level < tuning.battery_threshold
                    {
                        low.push((state, level));
                    }
                }
                // Un capteur binaire de pile est « on » quand elle est faible.
                "binary_sensor" => {
                    batteries += 1.0;
                    if state.state == "on" {
                        low.push((state, 0.0));
                    }
                }
                _ => {}
            }
        }
        if domain == "update" && state.state == "on" {
            updates.push(state);
        }
    }

    let mut out = Vec::new();
    for (domain, total) in &totals {
        let label = |sample: Sample| sample.with_label("domain", *domain);
        out.push(label(gauge("homeassistant_entities", *total, ts_ms)));
        out.push(label(gauge(
            "homeassistant_entities_unavailable",
            unavailable.get(domain).copied().unwrap_or(0.0),
            ts_ms,
        )));
        out.push(label(gauge(
            "homeassistant_entities_unknown",
            unknown.get(domain).copied().unwrap_or(0.0),
            ts_ms,
        )));
    }
    named_unavailable.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));
    for state in named_unavailable.into_iter().take(MAX_NAMED) {
        out.push(
            gauge("homeassistant_entity_unavailable", 1.0, ts_ms)
                .with_label("entity", state.entity_id.as_str())
                .with_label("name", state.name())
                .with_label("domain", state.domain()),
        );
    }

    out.push(gauge("homeassistant_batteries", batteries, ts_ms));
    out.push(gauge("homeassistant_batteries_low", low.len() as f64, ts_ms));
    low.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.entity_id.cmp(&b.0.entity_id)));
    for (state, level) in low.into_iter().take(MAX_NAMED) {
        out.push(
            gauge("homeassistant_battery_low", level, ts_ms)
                .with_label("entity", state.entity_id.as_str())
                .with_label("name", state.name()),
        );
    }

    out.push(gauge("homeassistant_updates_available", updates.len() as f64, ts_ms));
    updates.sort_by(|a, b| a.entity_id.cmp(&b.entity_id));
    for state in updates.into_iter().take(MAX_NAMED) {
        out.push(
            gauge("homeassistant_update_available", 1.0, ts_ms)
                .with_label("entity", state.entity_id.as_str())
                .with_label("name", state.name())
                .with_label("installed", state.attribute("installed_version").unwrap_or(""))
                .with_label("latest", state.attribute("latest_version").unwrap_or("")),
        );
    }
    out
}

/// Les réparations ouvertes, sans celles que l'utilisateur a ignorées.
pub fn repair_samples(issues: &Issues, ts_ms: i64) -> Vec<Sample> {
    let open: Vec<_> = issues
        .issues
        .iter()
        .filter(|issue| !issue.ignored && issue.dismissed_version.is_none())
        .collect();
    let mut by_severity = BTreeMap::from([("critical", 0.0), ("error", 0.0), ("warning", 0.0)]);
    for issue in &open {
        let severity = issue.severity.as_deref().unwrap_or("warning");
        if let Some(count) = by_severity.get_mut(severity) {
            *count += 1.0;
        }
    }
    let mut out: Vec<Sample> = by_severity
        .into_iter()
        .map(|(severity, count)| {
            gauge("homeassistant_repairs", count, ts_ms).with_label("severity", severity)
        })
        .collect();
    for issue in open.into_iter().take(MAX_NAMED) {
        out.push(
            gauge("homeassistant_repair", 1.0, ts_ms)
                .with_label("issue", issue.issue_id.as_str())
                .with_label("domain", issue.domain.as_str())
                .with_label("severity", issue.severity.as_deref().unwrap_or("warning")),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Réponses réelles de Home Assistant 2026.9.4 (intégration de démonstration
    // et quelques capteurs modèles), lues avec le jeton longue durée d'un
    // compte non administrateur.
    const CONFIG: &str = include_str!("testdata/ha_2026.9.4/api_config.json");
    const STATES: &str = include_str!("testdata/ha_2026.9.4/api_states.json");
    const REPAIRS: &str = include_str!("testdata/ha_2026.9.4/repairs_list_issues.json");

    fn tuning() -> Tuning {
        Tuning { battery_threshold: 20.0, exclude_domains: Vec::new() }
    }

    fn states() -> Vec<State> {
        serde_json::from_str(STATES).unwrap()
    }

    fn find<'a>(samples: &'a [Sample], metric: &str, key: &str, value: &str) -> Option<&'a Sample> {
        samples
            .iter()
            .find(|s| s.metric == metric && s.labels.get(key).map(String::as_str) == Some(value))
    }

    fn single(samples: &[Sample], metric: &str) -> f64 {
        samples.iter().find(|s| s.metric == metric).unwrap().value
    }

    #[test]
    fn la_configuration_donne_la_version_et_l_etat_du_coeur() {
        let config: Config = serde_json::from_str(CONFIG).unwrap();
        let samples = config_samples(&config, 0);
        assert_eq!(find(&samples, "homeassistant_info", "version", "2026.9.4").unwrap().value, 1.0);
        assert_eq!(single(&samples, "homeassistant_running"), 1.0);
        assert_eq!(single(&samples, "homeassistant_recovery_mode"), 0.0);
    }

    #[test]
    fn les_entites_indisponibles_et_inconnues_sont_comptees_par_domaine() {
        let samples = state_samples(&states(), &tuning(), 0);
        let unavailable =
            find(&samples, "homeassistant_entities_unavailable", "domain", "sensor").unwrap();
        assert_eq!(unavailable.value, 1.0);
        let named = find(
            &samples,
            "homeassistant_entity_unavailable",
            "entity",
            "sensor.garage_sensor_battery",
        )
        .unwrap();
        assert_eq!(named.labels["name"], "Garage sensor battery");
        // Un bouton jamais pressé est « inconnu » par nature : pas compté.
        assert_eq!(
            find(&samples, "homeassistant_entities_unknown", "domain", "button").unwrap().value,
            0.0
        );
        assert!(
            find(&samples, "homeassistant_entities_unknown", "domain", "sensor").unwrap().value
                >= 1.0
        );
        let lights = find(&samples, "homeassistant_entities", "domain", "light").unwrap();
        assert_eq!(lights.value, 6.0);
    }

    #[test]
    fn un_domaine_exclu_ne_compte_plus_ses_indisponibles() {
        let tuning = Tuning { battery_threshold: 20.0, exclude_domains: vec!["sensor".into()] };
        let samples = state_samples(&states(), &tuning, 0);
        assert_eq!(
            find(&samples, "homeassistant_entities_unavailable", "domain", "sensor").unwrap().value,
            0.0
        );
        assert!(find(&samples, "homeassistant_entity_unavailable", "domain", "sensor").is_none());
    }

    #[test]
    fn les_piles_faibles_sont_nommees_avec_leur_niveau() {
        let samples = state_samples(&states(), &tuning(), 0);
        // 12 % sous le porche, 12 % pour la démo, et le capteur binaire de la
        // serrure ; la pile du garage est indisponible, pas faible.
        assert_eq!(single(&samples, "homeassistant_batteries_low"), 3.0);
        let porch =
            find(&samples, "homeassistant_battery_low", "entity", "sensor.porch_sensor_battery")
                .unwrap();
        assert_eq!(porch.value, 12.0);
        let lock = find(
            &samples,
            "homeassistant_battery_low",
            "entity",
            "binary_sensor.front_door_lock_battery_low",
        )
        .unwrap();
        assert_eq!(lock.value, 0.0);
        assert!(
            find(&samples, "homeassistant_battery_low", "entity", "sensor.kitchen_sensor_battery")
                .is_none()
        );
        // Le seuil se règle.
        let strict = Tuning { battery_threshold: 90.0, exclude_domains: Vec::new() };
        let samples = state_samples(&states(), &strict, 0);
        assert!(
            find(&samples, "homeassistant_battery_low", "entity", "sensor.kitchen_sensor_battery")
                .is_some()
        );
    }

    #[test]
    fn les_mises_a_jour_disponibles_disent_leurs_versions() {
        let samples = state_samples(&states(), &tuning(), 0);
        assert_eq!(single(&samples, "homeassistant_updates_available"), 5.0);
        let bulb = find(
            &samples,
            "homeassistant_update_available",
            "entity",
            "update.demo_living_room_bulb_update",
        )
        .unwrap();
        assert_eq!(bulb.labels["installed"], "1.93.3");
        assert_eq!(bulb.labels["latest"], "1.94.2");
        assert!(
            find(&samples, "homeassistant_update_available", "entity", "update.demo_no_update")
                .is_none()
        );
    }

    #[test]
    fn les_reparations_sont_comptees_par_gravite() {
        let issues: Issues = serde_json::from_str(REPAIRS).unwrap();
        let samples = repair_samples(&issues, 0);
        assert_eq!(
            find(&samples, "homeassistant_repairs", "severity", "warning").unwrap().value,
            1.0
        );
        assert_eq!(
            find(&samples, "homeassistant_repairs", "severity", "error").unwrap().value,
            1.0
        );
        assert_eq!(
            find(&samples, "homeassistant_repairs", "severity", "critical").unwrap().value,
            0.0
        );
        let named =
            find(&samples, "homeassistant_repair", "issue", "country_not_configured").unwrap();
        assert_eq!(named.labels["domain"], "homeassistant");
    }
}
