//! Frise de disponibilité à la maille fine : les périodes courtes (24 h, 7 jours)
//! et les historiques plus courts que la période choisie.
//!
//! La frise par jour (`build_public`) ne sait pas faire moins d'une journée. Ici,
//! la période est découpée en tranches (5 min à 6 h) choisies pour rester
//! lisibles sur un téléphone (au plus 36 barres), et calculées côté serveur à la
//! même source que les barres par jour : `probe_success` pour les sondes, la
//! présence (`up`) pour les autres équipements. Une tranche d'avant la première
//! mesure n'a pas de valeur : elle ne compte ni comme panne ni comme 100 %.
//!
//! Le détail d'un jour (`/day/{date}`) rend, heure par heure, les pannes d'un
//! jour et les annonces qui le touchent — avec les seuls noms publics.

use std::collections::{HashMap, HashSet};

use axum::Json;
use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, NaiveDate, TimeDelta, Utc};
use dumbmonit_proto::{Target, TargetId};
use serde::Serialize;

use super::{
    PROBE_KINDS, device_covered, first_up_query, format_timestamp, instant_map, last_up_query,
    parse_stored, presence_query, probe_failed_query, probe_uptime_query, public_not_found,
    rescale, round, selector, target_of, unique_key,
};
use crate::api::ApiResult;
use crate::db;
use crate::db::targets::TargetStatus;
use crate::state::AppState;
use crate::tsdb::{Victoria, log_query_failure};

/// Au-delà, la frise reste par jour.
const MAX_SLOT_WINDOW_SECS: i64 = 7 * 86_400;
const MIN_WINDOW_SECS: i64 = 3_600;

/// Découpage de la frise en tranches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Plan {
    /// Durée réellement affichée.
    pub window_secs: i64,
    /// Durée d'une tranche.
    pub step_secs: i64,
}

impl Plan {
    pub fn slots(&self) -> i64 {
        (self.window_secs + self.step_secs - 1) / self.step_secs
    }
}

/// Durée d'une tranche pour une période donnée : 5 min jusqu'à 3 h, 15 min
/// jusqu'à 6 h, 1 h jusqu'à 36 h, 6 h au-delà.
pub(crate) fn step_for(window_secs: i64) -> i64 {
    if window_secs <= 10_800 {
        300
    } else if window_secs <= 21_600 {
        900
    } else if window_secs <= 129_600 {
        3_600
    } else {
        21_600
    }
}

/// Choisit la frise : `None` pour la frise par jour (période de plus de 7 jours).
///
/// `setting_days` est la période choisie pour la page ; `age_secs` l'âge des
/// données (le plus ancien service montré). La période affichée est la plus
/// courte des deux, une heure au moins : une page toute neuve ne montre pas des
/// semaines de barres grises.
pub(crate) fn plan(setting_days: i64, age_secs: i64) -> Option<Plan> {
    let wanted = setting_days.clamp(1, 90) * 86_400;
    let window = wanted.min(age_secs.max(MIN_WINDOW_SECS));
    (window <= MAX_SLOT_WINDOW_SECS)
        .then(|| Plan { window_secs: window, step_secs: step_for(window) })
}

/// Fin (exclue) de chaque tranche, la dernière contenant « maintenant ».
pub(crate) fn slot_starts(plan: Plan, now_secs: i64) -> Vec<i64> {
    let step = plan.step_secs;
    let end = (now_secs.div_euclid(step) + 1) * step;
    let slots = plan.slots();
    (0..slots).map(|i| end - (slots - i) * step).collect()
}

/// Valeur d'une tranche.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Slot {
    pub uptime_pct: f64,
    pub down_minutes: f64,
}

/// Équipements qui ne sondent plus : désactivés ou jamais sondés.
pub(crate) fn inactive_targets(
    targets: &[&Target],
    statuses: &HashMap<TargetId, TargetStatus>,
) -> HashSet<TargetId> {
    targets
        .iter()
        .filter(|t| {
            !t.enabled
                || statuses
                    .get(&t.id)
                    .and_then(|s| s.last_probe_at.as_deref())
                    .and_then(parse_stored)
                    .is_none()
        })
        .map(|t| t.id)
        .collect()
}

/// Tranches de chaque équipement, indexées par début de tranche (secondes).
pub(crate) async fn collect(
    victoria: &Victoria,
    targets: &[&Target],
    starts: &[i64],
    step: i64,
    now_secs: i64,
    inactive: &HashSet<TargetId>,
) -> HashMap<TargetId, HashMap<i64, Slot>> {
    let mut out: HashMap<TargetId, HashMap<i64, Slot>> = HashMap::new();
    let (Some(&first), Some(&last)) = (starts.first(), starts.last()) else { return out };
    let start_ms = (first + step) * 1_000;
    let end_ms = (last + step) * 1_000;
    let window = format!("{step}s");
    let probes: Vec<TargetId> =
        targets.iter().filter(|t| PROBE_KINDS.contains(&t.kind.as_str())).map(|t| t.id).collect();
    let devices: Vec<TargetId> =
        targets.iter().filter(|t| !PROBE_KINDS.contains(&t.kind.as_str())).map(|t| t.id).collect();
    let step_u = step as u64;

    if !probes.is_empty() {
        let sel = selector(&probes);
        let q_up = probe_uptime_query(&sel, &window, "");
        let q_failed = probe_failed_query(&sel, &window);
        match tokio::try_join!(
            victoria.query_range(&q_up, start_ms, end_ms, step_u),
            victoria.query_range(&q_failed, start_ms, end_ms, step_u),
        ) {
            Ok((up, failed)) => {
                let intervals: HashMap<TargetId, f64> =
                    targets.iter().map(|t| (t.id, t.interval.as_secs_f64().max(1.0))).collect();
                let mut down: HashMap<(TargetId, i64), f64> = HashMap::new();
                for series in failed {
                    let Some(id) = target_of(&series.metric) else { continue };
                    let interval = intervals.get(&id).copied().unwrap_or(60.0);
                    for (ts, raw) in series.values {
                        if let Ok(count) = raw.parse::<f64>()
                            && count.is_finite()
                        {
                            let minutes =
                                (count.max(0.0) * interval / 60.0).min(step as f64 / 60.0);
                            down.insert((id, ts as i64 - step), minutes);
                        }
                    }
                }
                for series in up {
                    let Some(id) = target_of(&series.metric) else { continue };
                    for (ts, raw) in series.values {
                        if let Ok(value) = raw.parse::<f64>()
                            && value.is_finite()
                        {
                            let start = ts as i64 - step;
                            let pct = value.clamp(0.0, 100.0);
                            let minutes = down.get(&(id, start)).copied().unwrap_or(0.0);
                            out.entry(id)
                                .or_default()
                                .insert(start, Slot { uptime_pct: pct, down_minutes: minutes });
                        }
                    }
                }
            }
            Err(error) => log_query_failure(&error, "status page: slot metrics unavailable"),
        }
    }

    if !devices.is_empty() {
        let sel = selector(&devices);
        let q_first = first_up_query(&sel, "90d");
        let q_last = last_up_query(&sel, "90d");
        let q_slots = presence_query(&sel, &window);
        match tokio::try_join!(
            victoria.query(&q_first),
            victoria.query(&q_last),
            victoria.query_range(&q_slots, start_ms, end_ms, step_u),
        ) {
            Ok((first_up, last_up, slots)) => {
                let first_up = instant_map(first_up);
                let last_up = instant_map(last_up);
                for series in slots {
                    let Some(id) = target_of(&series.metric) else { continue };
                    let Some(first_ts) = first_up.get(&id).copied() else { continue };
                    for (ts, raw) in series.values {
                        let Ok(measured) = raw.parse::<f64>() else { continue };
                        let start = ts as i64 - step;
                        let covered = device_covered(
                            start as f64,
                            ts,
                            first_ts,
                            inactive.contains(&id).then(|| last_up.get(&id).copied()),
                            now_secs as f64,
                        );
                        if let Some(value) = rescale(measured, step as f64, covered) {
                            let minutes = (100.0 - value).max(0.0) / 100.0 * covered / 60.0;
                            out.entry(id).or_default().insert(
                                start,
                                Slot {
                                    uptime_pct: value,
                                    down_minutes: minutes.min(step as f64 / 60.0),
                                },
                            );
                        }
                    }
                }
            }
            Err(error) => log_query_failure(&error, "status page: slot device metrics unavailable"),
        }
    }
    out
}

/// Disponibilité sur la période : moyenne des tranches qui ont des mesures ;
/// `None` s'il n'y en a aucune.
pub(crate) fn period_uptime(slots: &HashMap<i64, Slot>) -> Option<f64> {
    if slots.is_empty() {
        return None;
    }
    let sum: f64 = slots.values().map(|slot| slot.uptime_pct).sum();
    Some(sum / slots.len() as f64)
}

// --------------------------------------------------------------------------
// Détail d'un jour
// --------------------------------------------------------------------------

/// Tranche du détail d'un jour.
const DAY_STEP: i64 = 300;

#[derive(Debug, Serialize)]
struct Outage {
    from: String,
    to: String,
    minutes: u32,
}

#[derive(Debug, Serialize)]
struct DayService {
    key: String,
    label: String,
    /// Disponibilité de chacune des 24 heures (UTC) ; `null` sans mesure.
    hours: Vec<Option<f64>>,
    outages: Vec<Outage>,
}

#[derive(Debug, Serialize)]
struct DayIncident {
    title: String,
    kind: String,
    status: String,
    starts_at: String,
    ends_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct DayDetail {
    date: String,
    services: Vec<DayService>,
    incidents: Vec<DayIncident>,
}

/// Plages consécutives de tranches en panne (disponibilité sous 100 %).
fn outages(starts: &[i64], slots: &HashMap<i64, Slot>) -> Vec<Outage> {
    let mut out = Vec::new();
    let mut current: Option<(i64, i64, f64)> = None;
    for &start in starts {
        let down = slots.get(&start).filter(|slot| slot.uptime_pct < 99.95);
        match (down, current) {
            (Some(slot), Some((from, _, minutes))) => {
                current = Some((from, start + DAY_STEP, minutes + slot.down_minutes));
            }
            (Some(slot), None) => current = Some((start, start + DAY_STEP, slot.down_minutes)),
            (None, Some(run)) => {
                out.push(run);
                current = None;
            }
            (None, None) => {}
        }
        if start == *starts.last().unwrap_or(&start)
            && let Some(run) = current.take()
        {
            out.push(run);
        }
    }
    let stamp = |secs: i64| {
        DateTime::<Utc>::from_timestamp(secs, 0).map(format_timestamp).unwrap_or_default()
    };
    out.into_iter()
        .map(|(from, to, minutes)| Outage {
            from: stamp(from),
            to: stamp(to),
            minutes: minutes.round().max(1.0) as u32,
        })
        .collect()
}

/// `GET /api/public/status/{slug}/day/{date}`.
pub async fn public_day(
    State(state): State<AppState>,
    Path((slug, date)): Path<(String, String)>,
) -> ApiResult<Response> {
    let page = db::status_pages::get_page_by_slug(&state.pool, &slug).await?;
    let page = match page {
        Some(page) if page.published => page,
        _ => return Err(public_not_found()),
    };
    let bad = || {
        crate::api::ApiError::BadRequest(
            "Use a date like 2026-10-21 within the last 90 days.".into(),
        )
    };
    let day = NaiveDate::parse_from_str(&date, "%Y-%m-%d").map_err(|_| bad())?;
    let day_start = day.and_hms_opt(0, 0, 0).ok_or_else(bad)?.and_utc().timestamp();
    let now = Utc::now();
    if day_start > now.timestamp() || day_start < now.timestamp() - 91 * 86_400 {
        return Err(bad());
    }

    let items = db::status_pages::effective_items(&state.pool, page.id).await?;
    let targets: HashMap<TargetId, Target> = db::targets::list(&state.pool, &state.cipher)
        .await?
        .into_iter()
        .map(|target| (target.id, target))
        .collect();
    let statuses = db::targets::statuses(&state.pool).await?;
    let shown: Vec<&Target> =
        items.iter().filter_map(|item| targets.get(&item.target_id)).collect();
    let inactive = inactive_targets(&shown, &statuses);
    let starts: Vec<i64> = (0..86_400 / DAY_STEP).map(|i| day_start + i * DAY_STEP).collect();
    let slots =
        collect(&state.victoria, &shown, &starts, DAY_STEP, now.timestamp(), &inactive).await;

    let empty = HashMap::new();
    let mut keys = HashSet::new();
    let mut services = Vec::new();
    for item in &items {
        if !targets.contains_key(&item.target_id) {
            continue;
        }
        let key = unique_key(&item.label, &mut keys);
        let own = slots.get(&item.target_id).unwrap_or(&empty);
        let hours = (0..24)
            .map(|hour| {
                let values: Vec<f64> = (0..3_600 / DAY_STEP)
                    .filter_map(|i| own.get(&(day_start + hour * 3_600 + i * DAY_STEP)))
                    .map(|slot| slot.uptime_pct)
                    .collect();
                (!values.is_empty())
                    .then(|| round(values.iter().sum::<f64>() / values.len() as f64))
            })
            .collect();
        services.push(DayService {
            key,
            label: item.label.clone(),
            hours,
            outages: outages(&starts, own),
        });
    }

    let from = format_timestamp(DateTime::<Utc>::from_timestamp(day_start, 0).unwrap_or(now));
    let to = format_timestamp(
        DateTime::<Utc>::from_timestamp(day_start, 0).unwrap_or(now) + TimeDelta::days(1),
    );
    let incidents = db::status_pages::incidents_for_page(&state.pool, page.id, &from)
        .await?
        .into_iter()
        .filter(|incident| {
            incident.starts_at < to
                && incident.ends_at.as_deref().is_none_or(|end| end >= from.as_str())
        })
        .map(|incident| DayIncident {
            title: incident.title,
            kind: incident.kind,
            status: incident.status,
            starts_at: incident.starts_at,
            ends_at: incident.ends_at,
        })
        .collect();

    Ok((
        [(header::CACHE_CONTROL, "public, max-age=60"), (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")],
        Json(DayDetail { date, services, incidents }),
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_step_follows_the_window_and_keeps_bars_readable() {
        assert_eq!(step_for(7_200), 300, "2 h: five-minute bars");
        assert_eq!(step_for(12 * 3_600), 3_600, "12 h: hourly");
        assert_eq!(step_for(86_400), 3_600, "24 h: hourly");
        assert_eq!(step_for(3 * 86_400), 21_600, "3 days: six-hour bars");
        assert_eq!(step_for(7 * 86_400), 21_600, "7 days: six-hour bars");
        for window in [3_600, 7_200, 10_800, 21_600, 86_400, 3 * 86_400, 7 * 86_400] {
            let p = Plan { window_secs: window, step_secs: step_for(window) };
            assert!(p.slots() <= 36, "{window}s -> {} bars", p.slots());
        }
    }

    #[test]
    fn a_short_history_shrinks_the_window_instead_of_showing_grey_bars() {
        // 30 jours demandés, 2 h de données : on montre 2 h.
        let p = plan(30, 7_200).expect("slots");
        assert_eq!((p.window_secs, p.step_secs), (7_200, 300));
        // Moins d'une heure : une heure au moins.
        assert_eq!(plan(30, 600).unwrap().window_secs, 3_600);
        // 3 jours de données sur 90 demandés.
        assert_eq!(plan(90, 3 * 86_400).unwrap().step_secs, 21_600);
        // Assez de données : 24 h et 7 jours sont montrés tels quels.
        assert_eq!(plan(1, 40 * 86_400).unwrap().window_secs, 86_400);
        assert_eq!(plan(7, 40 * 86_400).unwrap().window_secs, 7 * 86_400);
        // Plus de sept jours : la frise par jour.
        assert!(plan(30, 40 * 86_400).is_none());
        assert!(plan(90, 8 * 86_400).is_none());
    }

    #[test]
    fn slots_end_with_the_one_holding_now() {
        let p = Plan { window_secs: 86_400, step_secs: 3_600 };
        let starts = slot_starts(p, 1_000_000);
        assert_eq!(starts.len(), 24);
        let last = *starts.last().unwrap();
        assert!(last <= 1_000_000 && 1_000_000 < last + 3_600);
        assert!(starts.windows(2).all(|w| w[1] - w[0] == 3_600));
    }

    #[test]
    fn the_period_uptime_ignores_slots_without_data() {
        let mut slots = HashMap::new();
        assert_eq!(period_uptime(&slots), None);
        slots.insert(0, Slot { uptime_pct: 100.0, down_minutes: 0.0 });
        slots.insert(3_600, Slot { uptime_pct: 50.0, down_minutes: 30.0 });
        assert_eq!(period_uptime(&slots), Some(75.0));
    }

    #[test]
    fn outages_merge_consecutive_down_slots() {
        let starts: Vec<i64> = (0..6).map(|i| i * DAY_STEP).collect();
        let mut slots = HashMap::new();
        for (i, pct) in [100.0, 0.0, 0.0, 100.0, 40.0, 40.0].into_iter().enumerate() {
            let start = i as i64 * DAY_STEP;
            slots.insert(start, Slot { uptime_pct: pct, down_minutes: (100.0 - pct) / 20.0 });
        }
        let found = outages(&starts, &slots);
        assert_eq!(found.len(), 2, "the run that ends the day is closed too");
        assert_eq!(found[0].minutes, 10);
    }
}
