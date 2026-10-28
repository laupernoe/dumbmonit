//! Trente jours d'historique rétroactif pour la démonstration.
//!
//! Sans lui, une instance qui vient de démarrer n'a que des courbes de quelques
//! minutes : ni « Last 7 days », ni prévision, ni disponibilité à montrer. La
//! dernière semaine est écrite au pas fin, les trois semaines d'avant à un pas
//! plus lâche : de quoi tracer 30 jours de disponibilité et de tendance sans
//! multiplier les points.
//!
//! Deux sources :
//!
//! - les sondes synthétiques, l'agent simulé et les équipements de
//!   [`super::extras`] (commutateur, menaces, échéances) sont recalculés à chaque
//!   point de la grille par les mêmes fonctions qu'en direct ;
//! - les équipements simulés ([`super::estate`]) n'ont qu'une valeur, celle de
//!   leurs réponses enregistrées. Une fois leur première collecte écrite, chaque
//!   série est relue et prolongée dans le passé : plate pour un état, ondulée
//!   pour une charge, en pente douce pour un remplissage — et raccordée
//!   exactement à la valeur actuelle.
//!
//! Les points tombent sur une grille calée sur l'époque Unix et leurs valeurs ne
//! dépendent que de l'horodatage : un redémarrage réécrit les mêmes points, que
//! VictoriaMetrics dédoublonne. Rien ne dérive.

use std::collections::BTreeMap;
use std::time::Duration;

use dumbmonit_proto::{MetricKind, Sample, TargetId};
use tracing::{info, warn};

use super::extras;
use super::synthetic::{self, AGENT_HOST};
use crate::state::AppState;
use crate::tsdb::METRIC_PREFIX;

const DAY_MS: i64 = 86_400_000;
/// Profondeur de l'historique.
const HISTORY_MS: i64 = 30 * DAY_MS;
/// Durée écrite au pas fin ; avant, le pas est plus lâche.
const RECENT_MS: i64 = 7 * DAY_MS;
/// Pas des séries synthétiques : cinq minutes, comme une vraie collecte espacée.
const SYNTHETIC_STEP_MS: i64 = 300_000;
/// … et un quart d'heure au-delà de la dernière semaine.
const SYNTHETIC_OLD_STEP_MS: i64 = 900_000;
/// Pas des séries d'équipements, plus nombreuses : un quart d'heure.
const DEVICE_STEP_MS: i64 = 900_000;
/// … et deux heures au-delà de la dernière semaine.
const DEVICE_OLD_STEP_MS: i64 = 7_200_000;
/// Une machine en panne cesse d'écrire des échecs peu avant le présent : une
/// alerte « sonde en échec » ne doit pas voir un dernier zéro tout frais.
const QUIET_MS: i64 = 30 * 60_000;
/// Les points récents sont laissés à la collecte en direct.
const LIVE_MARGIN_MS: i64 = 300_000;
/// Taille des envois vers VictoriaMetrics.
const CHUNK: usize = 40_000;
/// Depuis combien de temps les machines en panne ne répondent plus.
const DOWN_FOR_MS: i64 = 26 * 3_600_000;
/// Garde-fou : au-delà, les séries d'un équipement ne sont pas prolongées.
const MAX_DEVICE_SERIES: usize = 6_000;

/// Lance le remplissage en tâche de fond.
pub fn spawn(state: AppState, targets: Vec<(TargetId, String, String)>) {
    tokio::spawn(async move {
        if let Err(error) = synthetic_history(&state, &targets).await {
            warn!(%error, "demo: synthetic history not written");
        }
        // Laisse aux collecteurs le temps d'une première passe complète.
        tokio::time::sleep(Duration::from_secs(90)).await;
        if let Err(error) = device_history(&state, &targets).await {
            warn!(%error, "demo: device history not written");
        }
    });
}

/// Les instants de `from_ms` (exclu) à `to_ms` (inclus), alignés sur `step_ms`.
fn grid(from_ms: i64, to_ms: i64, step_ms: i64) -> impl Iterator<Item = i64> {
    let first = from_ms.div_euclid(step_ms) * step_ms + step_ms;
    (0..).map(move |i| first + i * step_ms).take_while(move |ts| *ts <= to_ms)
}

/// Les instants de l'historique, du plus ancien au plus récent : pas lâche
/// (`old_step_ms`) jusqu'à il y a une semaine, pas fin (`step_ms`) ensuite.
/// `old_step_ms` doit être un multiple de `step_ms`.
fn timeline(step_ms: i64, old_step_ms: i64) -> Vec<i64> {
    let now = chrono::Utc::now().timestamp_millis();
    let recent_from = now - RECENT_MS;
    let old = grid(now - HISTORY_MS, recent_from, old_step_ms);
    old.chain(grid(recent_from, now - LIVE_MARGIN_MS, step_ms)).collect()
}

fn base_labels(id: TargetId, name: &str) -> BTreeMap<String, String> {
    BTreeMap::from([("target".to_string(), id.to_string()), ("host".to_string(), name.to_string())])
}

/// Étiquettes des appareils de service d'une machine à agent, par module.
async fn service_labels(
    state: &AppState,
    host: TargetId,
) -> anyhow::Result<BTreeMap<String, BTreeMap<String, String>>> {
    let mut labels = BTreeMap::new();
    for (module, found) in crate::collectors::agent::modules::states(&state.pool, host).await? {
        let Some(child) = found.target_id else { continue };
        if let Some(target) = crate::db::targets::get(&state.pool, &state.cipher, child).await? {
            labels.insert(module, base_labels(child, &target.name));
        }
    }
    Ok(labels)
}

async fn write(state: &AppState, samples: &mut Vec<Sample>, force: bool) {
    if samples.len() < CHUNK && !force {
        return;
    }
    if let Err(error) = state.victoria.write(samples).await {
        warn!(%error, "demo: history chunk refused");
    }
    samples.clear();
}

async fn synthetic_history(
    state: &AppState,
    targets: &[(TargetId, String, String)],
) -> anyhow::Result<()> {
    let mut buffer = Vec::with_capacity(CHUNK);
    let mut written = 0usize;
    // Les machines de `DOWN_HOSTS` se sont tues il y a un peu plus d'un jour.
    let now = chrono::Utc::now().timestamp_millis();
    let down_since = now - DOWN_FOR_MS;
    let quiet_after = now - QUIET_MS;
    let timeline = timeline(SYNTHETIC_STEP_MS, SYNTHETIC_OLD_STEP_MS);
    for (id, name, kind) in targets {
        let labels = base_labels(*id, name);
        // Les services de l'agent (Docker…) ont leur propre appareil, créé par
        // le premier lot : leur historique va sous lui, comme les mesures vives.
        let services = service_labels(state, *id).await?;
        for ts in timeline.iter().copied() {
            let (samples, healthy) = match kind.as_str() {
                "agent" if name == AGENT_HOST => (synthetic::agent_samples(ts), true),
                "http" | "tls" | "ping" => match synthetic::probe_samples(kind, name, ts) {
                    Ok(samples) => (samples, true),
                    // Une machine en panne a répondu jusqu'à la veille : sans série
                    // `up` interrompue, l'alerte « injoignable » n'aurait rien à
                    // mesurer. Ses mesures d'avant sont celles d'une voisine saine.
                    Err(_) if ts < down_since => (
                        synthetic::probe_samples(kind, "printer.home.arpa", ts).unwrap_or_default(),
                        true,
                    ),
                    // Depuis, chaque passage a échoué : des zéros, pour que la
                    // disponibilité sur 30 jours ne reste pas à 100 %. Pas de `up` :
                    // la série reste interrompue.
                    Err(_) if ts <= quiet_after => (
                        vec![
                            Sample::new("probe_success", 0.0, MetricKind::Gauge, ts)
                                .with_label("probe", kind.as_str()),
                        ],
                        false,
                    ),
                    Err(_) => continue,
                },
                other if extras::handles(other) => {
                    (extras::samples(other, name, ts).unwrap_or_default(), true)
                }
                _ => break,
            };
            let up = healthy.then(|| Sample::new("up", 1.0, MetricKind::Gauge, ts));
            for mut sample in samples.into_iter().chain(up) {
                let own = crate::collectors::agent::modules::module_of(&sample.metric)
                    .and_then(|module| services.get(module.id));
                sample.labels.extend(own.unwrap_or(&labels).clone());
                buffer.push(sample);
            }
            written += 1;
            write(state, &mut buffer, false).await;
        }
    }
    write(state, &mut buffer, true).await;
    info!(points = written, "demo: synthetic history written");
    Ok(())
}

async fn device_history(
    state: &AppState,
    targets: &[(TargetId, String, String)],
) -> anyhow::Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    let timeline = timeline(DEVICE_STEP_MS, DEVICE_OLD_STEP_MS);
    let mut buffer = Vec::with_capacity(CHUNK);
    for (id, _, kind) in targets {
        // Les types simulés par [`extras`] ont déjà leur historique complet.
        if matches!(kind.as_str(), "agent" | "http" | "tls" | "ping") || extras::handles(kind) {
            continue;
        }
        let series = state.victoria.query(&format!("{{target=\"{id}\"}}")).await?;
        if series.len() > MAX_DEVICE_SERIES {
            warn!(target = id, series = series.len(), "demo: too many series, history skipped");
            continue;
        }
        for series in series {
            let mut labels = series.metric;
            let Some(name) = labels.remove("__name__") else { continue };
            let Some(metric) = name.strip_prefix(METRIC_PREFIX).map(str::to_string) else {
                continue;
            };
            let Ok(current) = series.value.1.parse::<f64>() else { continue };
            let shape = Shape::of(&metric);
            let seed = synthetic::seed(&format!("{metric}{labels:?}"));
            for ts in timeline.iter().copied() {
                let value = shape.value(current, seed, ts, now);
                let mut sample = Sample::new(metric.clone(), value, MetricKind::Gauge, ts);
                sample.labels = labels.clone();
                buffer.push(sample);
                write(state, &mut buffer, false).await;
            }
        }
    }
    write(state, &mut buffer, true).await;
    info!("demo: device history written");
    Ok(())
}

/// Comment prolonger une série dans le passé, d'après son seul nom.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    /// Un état, une capacité, une version : la même valeur toute la semaine.
    Flat,
    /// Une charge : ondule au rythme des journées.
    Load,
    /// Un remplissage : 4 % plus bas une semaine plus tôt, et ainsi de suite.
    Filling,
    /// Un temps de fonctionnement : décroît d'autant qu'on remonte le temps.
    Uptime,
}

impl Shape {
    fn of(metric: &str) -> Self {
        let has = |part: &str| metric.contains(part);
        if has("uptime") {
            Self::Uptime
        } else if has("total")
            || has("count")
            || has("info")
            || has("status")
            || has("state")
            || has("_age")
            || has("last_")
        {
            Self::Flat
        } else if has("cpu")
            || has("load")
            || has("temperature")
            || has("celsius")
            || has("mem")
            || has("iowait")
            || has("rate")
            || has("latency")
            || has("seconds")
            || has("states")
        {
            Self::Load
        } else if has("used") || has("usage") || has("alloc") {
            Self::Filling
        } else {
            Self::Flat
        }
    }

    fn value(self, current: f64, seed: u64, ts: i64, now: i64) -> f64 {
        let age_ms = (now - ts).max(0) as f64;
        match self {
            Self::Flat => current,
            Self::Load => {
                let factor = |at: i64| 0.7 + 0.6 * synthetic::activity(seed, at);
                let value = current * factor(ts) / factor(now);
                if current <= 100.0 { value.clamp(0.0, 100.0) } else { value.max(0.0) }
            }
            // 4 % de moins par semaine remontée : assez pour une pente lisible.
            Self::Filling => current * (1.0 - 0.04 * age_ms / RECENT_MS as f64),
            Self::Uptime => {
                let back = age_ms / 1000.0;
                if current > back { current - back } else { current }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaque_forme_rejoint_la_valeur_actuelle() {
        let now = 1_790_000_000_000;
        for shape in [Shape::Flat, Shape::Load, Shape::Filling, Shape::Uptime] {
            assert!((shape.value(42.0, 7, now, now) - 42.0).abs() < 1e-9, "{shape:?}");
        }
    }

    #[test]
    fn les_noms_choisissent_la_forme() {
        assert_eq!(Shape::of("proxmox_node_cpu_percent"), Shape::Load);
        assert_eq!(Shape::of("proxmox_storage_used_percent"), Shape::Filling);
        assert_eq!(Shape::of("up"), Shape::Flat);
        assert_eq!(Shape::of("proxmox_node_uptime_seconds"), Shape::Uptime);
    }

    #[test]
    fn la_grille_ne_touche_pas_au_direct() {
        let now = chrono::Utc::now().timestamp_millis();
        for (step, old_step) in
            [(DEVICE_STEP_MS, DEVICE_OLD_STEP_MS), (SYNTHETIC_STEP_MS, SYNTHETIC_OLD_STEP_MS)]
        {
            assert_eq!(old_step % step, 0, "le pas lâche est un multiple du pas fin");
            let points = timeline(step, old_step);
            assert!(points.iter().all(|ts| ts % step == 0));
            assert!(points.windows(2).all(|w| w[0] < w[1]), "croissante, sans doublon");
            assert!(points.last().is_some_and(|ts| *ts <= now - LIVE_MARGIN_MS));
            assert!(points.first().is_some_and(|ts| *ts > now - HISTORY_MS));
            let recent = (RECENT_MS / step) as usize;
            let old = ((HISTORY_MS - RECENT_MS) / old_step) as usize;
            assert!(points.len() >= recent + old - 4, "{} points", points.len());
        }
    }

    #[test]
    fn l_historique_couvre_trente_jours() {
        let now = chrono::Utc::now().timestamp_millis();
        let points = timeline(SYNTHETIC_STEP_MS, SYNTHETIC_OLD_STEP_MS);
        let span = points.last().unwrap() - points.first().unwrap();
        assert!(span > 29 * DAY_MS, "{span} ms");
        // Le pas fin ne couvre que la dernière semaine.
        let fine = points.iter().filter(|ts| **ts > now - RECENT_MS).count();
        assert!(fine > 2000 && fine < 2020, "{fine}");
    }

    #[test]
    fn un_remplissage_remonte_de_quatre_pour_cent_par_semaine() {
        let now = 1_790_000_000_000;
        let week = Shape::Filling.value(100.0, 1, now - RECENT_MS, now);
        assert!((week - 96.0).abs() < 1e-6, "{week}");
        let month = Shape::Filling.value(100.0, 1, now - HISTORY_MS, now);
        assert!(month > 80.0 && month < week, "{month}");
    }
}
