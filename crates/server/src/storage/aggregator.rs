//! Agrégateur de l'historique long terme, et relevé quotidien de l'espace
//! occupé (pour l'estimation de croissance).
//!
//! Toutes les minutes, il agrège les fenêtres de cinq minutes terminées depuis
//! son dernier passage (curseur `storage_rollup_cursor`), puis remonte le
//! passé, un jour par passage au plus, jusqu'à l'horizon de la rétention brute
//! (curseur `storage_rollup_backfill`) : à la première mise en route, tout ce
//! que le brut contient encore est recopié en agrégats.
//!
//! Chaque morceau est une poignée de `query_range` au pas de cinq minutes sur
//! l'instance brute (voir [`rollup::aggregate_queries`]), écrits d'un bloc
//! dans l'instance long terme. Un curseur n'avance qu'une fois l'écriture
//! acceptée : une panne de l'une ou l'autre instance se rattrape au passage
//! suivant, et un agrégat réécrit remplace l'ancien (déduplication sur la
//! minute).

use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use super::settings;
use crate::state::AppState;
use crate::tsdb::rollup::{self, STEP_SECS};
use crate::tsdb::{LongTerm, Victoria};

const CURSOR_KEY: &str = "storage_rollup_cursor";
const BACKFILL_KEY: &str = "storage_rollup_backfill";
const SAMPLES_KEY: &str = "storage_disk_samples";

/// Période du passage.
const TICK: Duration = Duration::from_secs(60);
/// Marge laissée aux écritures en retard avant d'agréger une fenêtre.
const SETTLE_SECS: i64 = 120;
/// Taille d'un morceau : douze fenêtres par série et par requête.
const CHUNK_SECS: i64 = 3_600;
/// Morceaux de rattrapage du passé par passage (un jour).
const BACKFILL_CHUNKS_PER_TICK: usize = 24;
/// Taille d'un lot écrit dans l'instance long terme.
const WRITE_BATCH: usize = 20_000;
/// Relevés d'espace disque gardés (un par jour).
const MAX_SAMPLES: usize = 30;

pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(TICK);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            if let Err(error) = sample_disk(&state).await {
                tracing::debug!(error = %format!("{error:#}"), "disk usage sample failed");
            }
            let Some(long) = state.victoria.long_term().filter(|long| long.enabled()) else {
                continue;
            };
            match pass(&state, &long).await {
                Ok(()) => state.storage().update_status(|s| s.rollup_error = None),
                Err(error) => {
                    crate::tsdb::log_query_failure(&error, "long-term aggregation pass failed");
                    state.storage().update_status(|s| s.rollup_error = Some(format!("{error:#}")));
                }
            }
        }
    });
}

async fn read_i64(pool: &SqlitePool, key: &str) -> Option<i64> {
    settings::read(pool, key).await.ok().flatten().and_then(|v| v.trim().parse().ok())
}

/// Plus ancienne fenêtre déjà agrégée, d'après la base.
pub async fn stored_backfill(pool: &SqlitePool) -> Option<i64> {
    read_i64(pool, BACKFILL_KEY).await
}

/// Agrège les fenêtres `]first_end - 5m, last_end]` (fins incluses) et les
/// écrit. Rend le nombre de séries écrites.
async fn aggregate(raw: &Victoria, long: &Victoria, first_end: i64, last_end: i64) -> Result<u64> {
    if last_end < first_end {
        return Ok(0);
    }
    let mut samples = Vec::new();
    let mut series = 0u64;
    for query in rollup::aggregate_queries() {
        let answer = raw
            .query_range_raw(&query.query, first_end * 1000, last_end * 1000, STEP_SECS as u64)
            .await?;
        series += answer.len() as u64;
        samples.extend(rollup::to_samples(&answer, query.suffix));
    }
    for batch in samples.chunks(WRITE_BATCH) {
        long.write_now(batch).await?;
    }
    Ok(series)
}

/// Prochain morceau du présent : fins de fenêtres `(premier, dernier)` après
/// `cursor` (dernière fin agrégée), jusqu'à `target` compris.
pub fn forward_chunk(cursor: i64, target: i64) -> Option<(i64, i64)> {
    (cursor < target).then(|| (cursor + STEP_SECS, (cursor + CHUNK_SECS).min(target)))
}

/// Prochain morceau du passé : les fins de fenêtres juste avant `oldest`
/// (plus ancienne fin agrégée), sans descendre sous `floor`.
pub fn backfill_chunk(oldest: i64, floor: i64) -> Option<(i64, i64)> {
    let last = oldest - STEP_SECS;
    (last >= floor).then(|| ((last - CHUNK_SECS + STEP_SECS).max(floor), last))
}

/// Un passage : le présent d'abord, puis un morceau du passé.
async fn pass(state: &AppState, long: &LongTerm) -> Result<()> {
    let pool = &state.pool;
    let now = Utc::now().timestamp();
    let target = rollup::last_complete_window(now, SETTLE_SECS);
    let current = settings::load(pool, &state.config).await?;
    let floor = rollup::align_down(now - i64::from(current.metrics.days) * 86_400)
        + STEP_SECS
        + rollup::HORIZON_MARGIN_SECS;

    // Le présent : de la dernière fenêtre agrégée à la dernière terminée. Un
    // serveur resté arrêté plus longtemps que la rétention brute reprend à
    // l'horizon, rien n'existant plus avant.
    let mut cursor = match read_i64(pool, CURSOR_KEY).await {
        Some(cursor) => cursor.max(floor - STEP_SECS),
        None => {
            // Première mise en route : le passé sera rattrapé à partir d'ici.
            let start = target - CHUNK_SECS;
            settings::write(pool, BACKFILL_KEY, &(start + STEP_SECS).to_string()).await?;
            start
        }
    };
    while let Some((first, last)) = forward_chunk(cursor, target) {
        let series = aggregate(&state.victoria, &long.victoria, first, last).await?;
        cursor = last;
        settings::write(pool, CURSOR_KEY, &cursor.to_string()).await?;
        state.storage().update_status(|s| {
            s.last_window = Some(cursor);
            s.series = Some(series);
        });
    }
    state.storage().update_status(|s| s.last_window = Some(cursor));

    // Le passé, en remontant.
    let mut oldest = read_i64(pool, BACKFILL_KEY).await.unwrap_or(cursor + STEP_SECS);
    for _ in 0..BACKFILL_CHUNKS_PER_TICK {
        let Some((first, last)) = backfill_chunk(oldest, floor) else { break };
        aggregate(&state.victoria, &long.victoria, first, last).await?;
        oldest = first;
        settings::write(pool, BACKFILL_KEY, &oldest.to_string()).await?;
        long.extend_coverage(oldest);
    }
    long.extend_coverage(oldest);
    let done = oldest - STEP_SECS < floor;
    state.storage().update_status(|s| {
        s.backfilled_until = Some(oldest);
        s.backfill_done = done;
    });
    Ok(())
}

// --------------------------------------------------------------------------
// Croissance
// --------------------------------------------------------------------------

/// Un relevé quotidien : jour (secondes depuis l'époque / 86 400) et octets.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DiskSample {
    pub day: i64,
    pub bytes: u64,
}

/// Ajoute (ou remplace) le relevé du jour, et garde les [`MAX_SAMPLES`]
/// derniers.
pub fn record(samples: &mut Vec<DiskSample>, sample: DiskSample) {
    samples.retain(|s| s.day != sample.day);
    samples.push(sample);
    samples.sort_by_key(|s| s.day);
    let excess = samples.len().saturating_sub(MAX_SAMPLES);
    samples.drain(..excess);
}

/// Croissance par jour sur les relevés, et jours restants avant de remplir
/// `free` octets. Deux relevés au moins, sur des jours différents.
pub fn estimate(samples: &[DiskSample], free: Option<u64>) -> super::GrowthView {
    let sampled_days = samples.len() as u32;
    let (Some(first), Some(last)) = (samples.first(), samples.last()) else {
        return super::GrowthView { bytes_per_day: None, days_until_full: None, sampled_days };
    };
    if last.day <= first.day {
        return super::GrowthView { bytes_per_day: None, days_until_full: None, sampled_days };
    }
    let per_day = (last.bytes as f64 - first.bytes as f64) / (last.day - first.day) as f64;
    let until_full = match free {
        Some(free) if per_day > 0.0 => Some(free as f64 / per_day),
        _ => None,
    };
    super::GrowthView { bytes_per_day: Some(per_day), days_until_full: until_full, sampled_days }
}

async fn load_samples(pool: &SqlitePool) -> Vec<DiskSample> {
    settings::read(pool, SAMPLES_KEY)
        .await
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub async fn growth(pool: &SqlitePool, free: Option<u64>) -> super::GrowthView {
    estimate(&load_samples(pool).await, free)
}

/// Relevé du jour, une fois par heure au plus (le dernier de la journée gagne).
async fn sample_disk(state: &AppState) -> Result<()> {
    static LAST: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
    let now = Utc::now().timestamp();
    let last = LAST.load(std::sync::atomic::Ordering::Relaxed);
    if now - last < 3_600 {
        return Ok(());
    }
    LAST.store(now, std::sync::atomic::Ordering::Relaxed);
    let disk = super::disk_view(state).await;
    let bytes: u64 = disk
        .usage
        .iter()
        // Les sauvegardes peuvent vivre ailleurs : elles ne comptent pas dans
        // la croissance des données vivantes.
        .filter(|usage| usage.key != "backups")
        .filter_map(|usage| usage.bytes)
        .sum();
    let mut samples = load_samples(&state.pool).await;
    record(&mut samples, DiskSample { day: now.div_euclid(86_400), bytes });
    settings::write(&state.pool, SAMPLES_KEY, &serde_json::to_string(&samples)?).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les morceaux se suivent sans trou ni recouvrement : chaque fin de
    /// fenêtre est agrégée une fois, dans un sens comme dans l'autre.
    #[test]
    fn windows_are_aggregated_once_each() {
        let mut ends = Vec::new();
        let mut cursor = 0;
        while let Some((first, last)) = forward_chunk(cursor, 7_500) {
            assert_eq!(first % STEP_SECS, 0);
            ends.extend((first..=last).step_by(STEP_SECS as usize));
            cursor = last;
        }
        assert_eq!(ends, (1..=25).map(|i| i * STEP_SECS).collect::<Vec<_>>());
        assert_eq!(forward_chunk(7_500, 7_500), None);

        let mut back = Vec::new();
        let mut oldest = 30_000;
        while let Some((first, last)) = backfill_chunk(oldest, 21_000) {
            back.extend((first..=last).step_by(STEP_SECS as usize));
            oldest = first;
        }
        back.sort();
        assert_eq!(back, (70..100).map(|i| i * STEP_SECS).collect::<Vec<_>>());
        assert_eq!(oldest, 21_000);
    }

    #[test]
    fn one_sample_per_day_and_thirty_days_at_most() {
        let mut samples = Vec::new();
        for day in 0..40 {
            record(&mut samples, DiskSample { day, bytes: 1_000 * day as u64 });
        }
        record(&mut samples, DiskSample { day: 39, bytes: 5 });
        assert_eq!(samples.len(), MAX_SAMPLES);
        assert_eq!(samples.first().unwrap().day, 10);
        assert_eq!(samples.last().unwrap(), &DiskSample { day: 39, bytes: 5 });
    }

    #[test]
    fn growth_needs_two_days_and_projects_the_free_space() {
        let none = estimate(&[DiskSample { day: 1, bytes: 10 }], Some(100));
        assert_eq!(none.bytes_per_day, None);
        assert_eq!(none.sampled_days, 1);

        let samples = [DiskSample { day: 1, bytes: 1_000 }, DiskSample { day: 11, bytes: 2_000 }];
        let growth = estimate(&samples, Some(10_000));
        assert_eq!(growth.bytes_per_day, Some(100.0));
        assert_eq!(growth.days_until_full, Some(100.0));

        // Des données qui diminuent ne remplissent jamais le disque.
        let shrinking = [DiskSample { day: 1, bytes: 2_000 }, DiskSample { day: 2, bytes: 1_000 }];
        assert_eq!(estimate(&shrinking, Some(10)).days_until_full, None);
    }
}
