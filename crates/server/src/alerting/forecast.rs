//! Prévision de remplissage : « à ce rythme, plein dans N jours ».
//!
//! Le calcul est volontairement fait ici plutôt que par `predict_linear()` :
//! la régression des moindres carrés de VictoriaMetrics se laisse emporter par
//! un seul point aberrant et, surtout, par les paliers. Un disque qu'on vient
//! de nettoyer, un pool qu'on vient d'agrandir ou un quota de trafic remis à
//! zéro en début de mois donnent une pente négative, ou une pente moyenne qui ne
//! décrit plus rien. On procède donc en deux temps, sur une série échantillonnée
//! (une valeur par heure environ, trous compris) :
//!
//! 1. on repère la dernière **rupture à la baisse** — une chute nettement hors
//!    de la variation habituelle — et on ne garde que ce qui la suit ;
//! 2. on ajuste une droite par l'estimateur de **Theil–Sen** (médiane des pentes
//!    de toutes les paires de points), insensible aux aberrations et qui suit
//!    correctement les croissances en escalier (une sauvegarde par nuit).
//!
//! Les ruptures à la hausse ne sont pas des paliers : c'est de la consommation
//! réelle (une copie de 500 Go), et Theil–Sen la moyenne avec le reste.
//!
//! Aucun état n'est conservé : tout part de l'historique que VictoriaMetrics
//! garde déjà, ce qui rend le calcul identique au premier cycle comme au
//! millième, et reproductible pour l'API.

use crate::alerting::baseline::{MAD_TO_SIGMA, median};
use crate::alerting::model::{AnomalyParams, Severity};

/// Nombre minimal de points après la dernière rupture pour oser prévoir.
pub const MIN_POINTS: usize = 6;
/// Durée minimale couverte par ces points, en secondes : douze heures.
pub const MIN_SPAN_SECS: f64 = 12.0 * 3600.0;
/// Au-delà de dix ans, « plein un jour » ne veut plus rien dire : stable.
pub const MAX_DAYS: f64 = 3650.0;
/// Une série dont le dernier point a plus de deux jours n'est plus écrite.
pub const STALE_AFTER_SECS: f64 = 2.0 * 86_400.0;
/// Nombre maximal de points régressés : au-delà, la série est sous-échantillonnée
/// (Theil–Sen est quadratique).
const MAX_POINTS: usize = 240;
/// Une chute est une rupture au-delà de six écarts robustes…
const BREAK_SIGMAS: f64 = 6.0;
/// … et d'au moins 2 % de la limite : la rotation quotidienne d'un journal ne
/// doit pas effacer la tendance de la semaine.
const BREAK_FLOOR_REL: f64 = 0.02;
const SECS_PER_DAY: f64 = 86_400.0;

/// Où en est la prévision d'une série.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// La série croît : `days_to_full` est renseigné.
    Growing,
    /// Pas de croissance mesurable (ou plein dans plus de dix ans).
    Stable,
    /// Déjà à la limite.
    Full,
    /// Trop peu d'historique depuis la dernière rupture pour prévoir.
    InsufficientData,
}

/// Prévision d'une série.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Forecast {
    pub status: Status,
    /// Dernière valeur observée.
    pub current: f64,
    /// Croissance par jour, dans l'unité de la série (points de pourcentage pour
    /// les familles livrées). `None` sans assez de données.
    pub growth_per_day: Option<f64>,
    /// Jours avant d'atteindre la limite, depuis `now`. `Some(0.0)` si plein.
    pub days_to_full: Option<f64>,
    /// Confiance de 0 à 1 : historique suffisant, droite qui décrit bien les
    /// points, pas de rupture récente.
    pub confidence: f64,
}

/// Prévoit quand `points` (horodatage en secondes, valeur) atteindra `limit`.
///
/// `window_secs` est la profondeur demandée : la confiance n'est pleine que si
/// la tendance retenue en couvre au moins la moitié.
pub fn forecast(points: &[(f64, f64)], limit: f64, now_s: f64, window_secs: f64) -> Forecast {
    let mut clean: Vec<(f64, f64)> =
        points.iter().copied().filter(|(t, v)| t.is_finite() && v.is_finite()).collect();
    clean.sort_by(|a, b| a.0.total_cmp(&b.0));
    clean.dedup_by(|a, b| a.0 == b.0);

    let Some(&(last_t, current)) = clean.last() else {
        return insufficient(f64::NAN);
    };
    if current >= limit {
        return Forecast {
            status: Status::Full,
            current,
            growth_per_day: None,
            days_to_full: Some(0.0),
            confidence: 1.0,
        };
    }
    if now_s - last_t > STALE_AFTER_SECS {
        return insufficient(current);
    }

    let segment = after_last_drop(&clean, limit);
    let segment = thin(segment);
    let span = segment.last().map_or(0.0, |l| l.0) - segment.first().map_or(0.0, |f| f.0);
    if segment.len() < MIN_POINTS || span < MIN_SPAN_SECS {
        return insufficient(current);
    }
    let Some((slope, intercept, t0)) = theil_sen(&segment) else {
        return insufficient(current);
    };

    let growth_per_day = slope * SECS_PER_DAY;
    let confidence = confidence(&segment, slope, intercept, t0, span, window_secs);
    // Le niveau « actuel » est celui de la droite, pas le dernier point : un pic
    // passager ne doit pas avancer la date de remplissage.
    let level_now = intercept + slope * (now_s - t0);
    let negligible = growth_per_day <= limit.abs().max(1e-9) * 1e-5;
    if negligible {
        return Forecast {
            status: Status::Stable,
            current,
            growth_per_day: Some(growth_per_day),
            days_to_full: None,
            confidence,
        };
    }
    let days = ((limit - level_now) / slope / SECS_PER_DAY).max(0.0);
    if days > MAX_DAYS {
        return Forecast {
            status: Status::Stable,
            current,
            growth_per_day: Some(growth_per_day),
            days_to_full: None,
            confidence,
        };
    }
    Forecast {
        status: Status::Growing,
        current,
        growth_per_day: Some(growth_per_day),
        days_to_full: Some(days),
        confidence,
    }
}

fn insufficient(current: f64) -> Forecast {
    Forecast {
        status: Status::InsufficientData,
        current,
        growth_per_day: None,
        days_to_full: None,
        confidence: 0.0,
    }
}

/// Ce qui suit la dernière chute franche de la série.
///
/// La variation attendue entre deux points est le rythme médian multiplié par
/// l'écart de temps : un trou de deux jours suivi d'une hausse cohérente avec
/// la tendance n'est donc pas une rupture. Seules les chutes comptent (voir
/// l'en-tête du module).
pub fn after_last_drop(points: &[(f64, f64)], limit: f64) -> &[(f64, f64)] {
    if points.len() < 3 {
        return points;
    }
    let rates: Vec<f64> = points
        .windows(2)
        .filter(|w| w[1].0 > w[0].0)
        .map(|w| (w[1].1 - w[0].1) / (w[1].0 - w[0].0))
        .collect();
    let Some(rate) = median(&rates) else { return points };
    let residuals: Vec<f64> =
        points.windows(2).map(|w| (w[1].1 - w[0].1) - rate * (w[1].0 - w[0].0)).collect();
    let spread: Vec<f64> = residuals.iter().map(|r| r.abs()).collect();
    let scale = MAD_TO_SIGMA * median(&spread).unwrap_or(0.0);
    let floor = BREAK_FLOOR_REL * reference(points, limit);
    let threshold = (BREAK_SIGMAS * scale).max(floor);

    // Une chute n'est un palier que si le niveau reste en bas : un relevé
    // aberrant isolé, aussitôt rattrapé, n'efface pas l'historique (Theil–Sen
    // l'absorbe). Il faut au moins deux points après la chute pour en juger.
    // Les niveaux avant et après sont des médianes, ramenées à l'instant de la
    // chute par le rythme médian : ni un pic juste avant, ni un creux juste
    // après ne suffisent à faire un palier.
    let persistent = |i: usize| {
        let pivot = points[i].0;
        let level = |slice: &[(f64, f64)]| {
            let adjusted: Vec<f64> = slice.iter().map(|(t, v)| v - rate * (t - pivot)).collect();
            median(&adjusted)
        };
        let before = level(&points[(i + 1).saturating_sub(PERSIST_POINTS)..=i]);
        let after_slice = &points[i + 1..points.len().min(i + 1 + PERSIST_POINTS)];
        match (before, level(after_slice)) {
            (Some(before), Some(after)) if after_slice.len() >= 2 => after - before < -threshold,
            _ => false,
        }
    };
    let start = (0..residuals.len())
        .rev()
        .find(|&i| residuals[i] < -threshold && persistent(i))
        .map_or(0, |i| i + 1);
    &points[start..]
}

/// Nombre de points examinés après une chute pour juger qu'elle tient.
const PERSIST_POINTS: usize = 3;

/// Grandeur de référence pour le plancher de rupture : la limite quand elle a
/// un sens (100 %), sinon le niveau typique de la série.
fn reference(points: &[(f64, f64)], limit: f64) -> f64 {
    if limit.abs() > 0.0 {
        return limit.abs();
    }
    let levels: Vec<f64> = points.iter().map(|p| p.1.abs()).collect();
    median(&levels).unwrap_or(0.0).max(1e-9)
}

/// Sous-échantillonne régulièrement au-delà de [`MAX_POINTS`], en gardant le
/// dernier point.
fn thin(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if points.len() <= MAX_POINTS {
        return points.to_vec();
    }
    let stride = points.len().div_ceil(MAX_POINTS);
    let mut kept: Vec<(f64, f64)> = points.iter().step_by(stride).copied().collect();
    if let Some(last) = points.last()
        && kept.last() != Some(last)
    {
        kept.push(*last);
    }
    kept
}

/// Droite de Theil–Sen : `(pente par seconde, ordonnée en t0, t0)`.
///
/// Le temps est recentré sur le premier point pour garder des nombres petits :
/// des horodatages Unix au carré feraient perdre des chiffres significatifs.
pub fn theil_sen(points: &[(f64, f64)]) -> Option<(f64, f64, f64)> {
    let t0 = points.first()?.0;
    let mut slopes = Vec::with_capacity(points.len() * points.len() / 2);
    for (i, a) in points.iter().enumerate() {
        for b in &points[i + 1..] {
            let dt = b.0 - a.0;
            if dt > 0.0 {
                slopes.push((b.1 - a.1) / dt);
            }
        }
    }
    let slope = median(&slopes)?;
    let offsets: Vec<f64> = points.iter().map(|(t, v)| v - slope * (t - t0)).collect();
    let intercept = median(&offsets)?;
    Some((slope, intercept, t0))
}

/// Confiance de 0 à 1, produit de trois facteurs :
///
/// * assez de points (une journée à l'heure) ;
/// * une tendance qui couvre au moins la moitié de la fenêtre demandée — une
///   rupture récente fait donc baisser la confiance, le temps que la nouvelle
///   pente se confirme ;
/// * une droite qui explique les points : la croissance sur la période doit
///   dominer le bruit autour de la droite.
fn confidence(
    points: &[(f64, f64)],
    slope: f64,
    intercept: f64,
    t0: f64,
    span: f64,
    window_secs: f64,
) -> f64 {
    let count = (points.len() as f64 / 24.0).min(1.0);
    let coverage = if window_secs > 0.0 { (span / (window_secs * 0.5)).min(1.0) } else { 1.0 };
    let residuals: Vec<f64> =
        points.iter().map(|(t, v)| (v - (intercept + slope * (t - t0))).abs()).collect();
    let noise = MAD_TO_SIGMA * median(&residuals).unwrap_or(0.0);
    let trend = slope.abs() * span;
    let fit = if trend + noise > 0.0 { trend / (trend + 2.0 * noise) } else { 0.0 };
    let value = (count * coverage * fit).clamp(0.0, 1.0);
    (value * 100.0).round() / 100.0
}

/// Sévérité d'une alerte de prévision selon les jours restants : critique sous
/// `forecast_critical_days`, au moins avertissement sous
/// `forecast_warning_days`, sinon la sévérité déclarée par la règle.
pub fn tier(base: Severity, days: Option<f64>, params: &AnomalyParams) -> Severity {
    match days.filter(|d| d.is_finite()) {
        Some(days) if days <= params.forecast_critical_days => Severity::Critical,
        Some(days) if days <= params.forecast_warning_days => base.max(Severity::Warning),
        _ => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: f64 = 3600.0;
    const WEEK: f64 = 7.0 * 86_400.0;
    const NOW: f64 = 1_800_000_000.0;

    /// Une semaine de points horaires se terminant à `NOW`.
    fn week(value: impl Fn(f64) -> f64) -> Vec<(f64, f64)> {
        (0..=168).map(|h| NOW - WEEK + f64::from(h) * HOUR).map(|t| (t, value(t))).collect()
    }

    fn days_since_start(t: f64) -> f64 {
        (t - (NOW - WEEK)) / 86_400.0
    }

    #[test]
    fn une_croissance_reguliere_donne_la_bonne_echeance() {
        // 50 % il y a une semaine, +1 point par jour : 57 % aujourd'hui, plein
        // dans 43 jours.
        let points = week(|t| 50.0 + days_since_start(t));
        let f = forecast(&points, 100.0, NOW, WEEK);
        assert_eq!(f.status, Status::Growing);
        let days = f.days_to_full.expect("growing");
        assert!((days - 43.0).abs() < 0.01, "days = {days}");
        assert!((f.growth_per_day.unwrap() - 1.0).abs() < 1e-6);
        assert!(f.confidence > 0.95, "confidence = {}", f.confidence);
    }

    #[test]
    fn une_serie_plate_est_stable() {
        let f = forecast(&week(|_| 42.0), 100.0, NOW, WEEK);
        assert_eq!(f.status, Status::Stable);
        assert_eq!(f.days_to_full, None);
    }

    #[test]
    fn une_serie_qui_decroit_n_est_jamais_pleine() {
        let f = forecast(&week(|t| 80.0 - days_since_start(t)), 100.0, NOW, WEEK);
        assert_eq!(f.status, Status::Stable);
        assert_eq!(f.days_to_full, None);
    }

    #[test]
    fn une_serie_deja_pleine_vaut_zero_jour() {
        let f = forecast(&week(|_| 100.0), 100.0, NOW, WEEK);
        assert_eq!(f.status, Status::Full);
        assert_eq!(f.days_to_full, Some(0.0));
    }

    #[test]
    fn un_nettoyage_recent_relance_la_tendance_sans_la_fausser() {
        // +2 points par jour, puis 30 points libérés il y a trois jours : une
        // régression naïve verrait une pente quasi nulle.
        let cleanup = NOW - 3.0 * 86_400.0;
        let points = week(|t| {
            let base = 40.0 + 2.0 * days_since_start(t);
            if t >= cleanup { base - 30.0 } else { base }
        });
        let segment = after_last_drop(&points, 100.0);
        assert!(segment[0].0 >= cleanup, "the segment starts after the cleanup");
        let f = forecast(&points, 100.0, NOW, WEEK);
        assert_eq!(f.status, Status::Growing);
        assert!((f.growth_per_day.unwrap() - 2.0).abs() < 1e-6, "{:?}", f.growth_per_day);
        // 40 + 14 − 30 = 24 % aujourd'hui, 76 points à 2 par jour.
        assert!((f.days_to_full.unwrap() - 38.0).abs() < 0.05, "{:?}", f.days_to_full);
        // Trois jours sur une fenêtre de sept : la confiance est entamée.
        assert!(f.confidence < 0.9, "confidence = {}", f.confidence);
    }

    #[test]
    fn une_croissance_en_escalier_n_est_pas_une_rupture() {
        // Un datastore de sauvegarde : +3 points chaque nuit, plat le reste du jour.
        let points = week(|t| 30.0 + 3.0 * days_since_start(t).floor());
        assert_eq!(after_last_drop(&points, 100.0).len(), points.len());
        let f = forecast(&points, 100.0, NOW, WEEK);
        assert_eq!(f.status, Status::Growing);
        let growth = f.growth_per_day.unwrap();
        assert!((growth - 3.0).abs() < 0.5, "growth = {growth}");
    }

    #[test]
    fn les_trous_ne_genent_pas_la_regression() {
        // Deux jours sans mesure au milieu de la semaine.
        let points: Vec<(f64, f64)> = week(|t| 20.0 + 0.5 * days_since_start(t))
            .into_iter()
            .filter(|(t, _)| !(2.0..4.0).contains(&days_since_start(*t)))
            .collect();
        let f = forecast(&points, 100.0, NOW, WEEK);
        assert_eq!(f.status, Status::Growing);
        assert!((f.growth_per_day.unwrap() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn un_point_aberrant_ne_deplace_pas_la_droite() {
        let mut points = week(|t| 50.0 + days_since_start(t));
        points[80].1 = 99.0;
        points[81].1 = 0.0;
        assert_eq!(after_last_drop(&points, 100.0).len(), points.len(), "an isolated dip");
        let f = forecast(&points, 100.0, NOW, WEEK);
        assert!((f.growth_per_day.unwrap() - 1.0).abs() < 0.05, "{:?}", f.growth_per_day);
    }

    #[test]
    fn trop_peu_d_historique_ne_prevoit_rien() {
        let points: Vec<(f64, f64)> =
            (0..5).map(|h| (NOW - f64::from(h) * HOUR, 50.0 + f64::from(h))).collect();
        let f = forecast(&points, 100.0, NOW, WEEK);
        assert_eq!(f.status, Status::InsufficientData);
        assert_eq!(f.confidence, 0.0);
        assert_eq!(forecast(&[], 100.0, NOW, WEEK).status, Status::InsufficientData);
    }

    #[test]
    fn une_serie_qui_n_est_plus_ecrite_ne_prevoit_rien() {
        let old: Vec<(f64, f64)> =
            week(|t| 50.0 + days_since_start(t)).into_iter().map(|(t, v)| (t - WEEK, v)).collect();
        assert_eq!(forecast(&old, 100.0, NOW, WEEK).status, Status::InsufficientData);
    }

    #[test]
    fn le_bruit_fait_baisser_la_confiance() {
        let calme = forecast(&week(|t| 50.0 + 0.2 * days_since_start(t)), 100.0, NOW, WEEK);
        let agite = forecast(
            &week(|t| {
                let jitter = if (t / HOUR) as i64 % 2 == 0 { 3.0 } else { -3.0 };
                50.0 + 0.2 * days_since_start(t) + jitter
            }),
            100.0,
            NOW,
            WEEK,
        );
        assert!(
            agite.confidence < calme.confidence,
            "{} vs {}",
            agite.confidence,
            calme.confidence
        );
        assert!(agite.confidence < 0.5, "noise dominates the trend: {}", agite.confidence);
    }

    #[test]
    fn les_paliers_de_severite_suivent_les_jours_restants() {
        let params = AnomalyParams::default();
        assert_eq!(tier(Severity::Info, Some(25.0), &params), Severity::Info);
        assert_eq!(tier(Severity::Info, Some(14.0), &params), Severity::Warning);
        assert_eq!(tier(Severity::Info, Some(6.5), &params), Severity::Critical);
        assert_eq!(tier(Severity::Critical, Some(20.0), &params), Severity::Critical);
        assert_eq!(tier(Severity::Warning, None, &params), Severity::Warning);
    }
}
