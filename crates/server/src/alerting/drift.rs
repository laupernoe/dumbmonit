//! Dérive de baseline : « ce service répond quatre fois plus lentement que
//! d'habitude à cette heure-ci ».
//!
//! Un seuil fixe sur une latence ne convient à personne : 300 ms est normal
//! pour un site à l'autre bout du monde et catastrophique pour le NAS du
//! salon. On compare donc la mesure courante (moyenne sur une courte fenêtre)
//! à sa **normale** :
//!
//! * de préférence la médiane des mêmes fenêtres, à la même heure et le même
//!   jour, sur les semaines précédentes (la saisonnalité : la sauvegarde de
//!   3 h du matin ralentit tout, chaque nuit) ;
//! * à défaut (série trop jeune), la médiane mobile des sept derniers jours.
//!
//! L'échelle est la déviation absolue médiane (MAD) sur sept jours, convertie en
//! écart-type, avec les planchers de [`crate::alerting::baseline`] pour qu'une
//! série plate ne divise pas par zéro. Le score dit « combien d'écarts », le
//! rapport dit « combien de fois la normale » : le premier écarte le bruit, le
//! second est ce que l'utilisateur règle et lit.
//!
//! Contre le bruit, la règle exige en plus un volume minimal (assez de points
//! dans la fenêtre), une valeur minimale (une latence qui passe de 2 à 6 ms ne
//! compte pas), un écart absolu minimal et, comme toute règle, une durée `for`.

use crate::alerting::baseline::{MAD_TO_SIGMA, median};
use crate::alerting::model::{AnomalyParams, DriftDirection};

/// Points d'historique (à cinq minutes) exigés pour se rabattre sur la médiane
/// mobile : une journée.
pub const MIN_HISTORY_POINTS: f64 = 288.0;
/// Semaines de référence exigées pour la normale saisonnière.
pub const MIN_SEASONAL_REFS: usize = 2;
/// Plafond du rapport, pour une normale à zéro.
const MAX_RATIO: f64 = 1e6;

/// Ce que VictoriaMetrics a répondu pour une série.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DriftInput {
    /// Moyenne de la fenêtre courante.
    pub current: f64,
    /// Nombre de points dans la fenêtre courante.
    pub samples: f64,
    /// Moyenne de la même fenêtre il y a 1, 2, … semaines (absentes omises).
    pub seasonal: Vec<f64>,
    /// Médiane mobile sur sept jours.
    pub rolling_median: Option<f64>,
    /// Déviation absolue médiane sur sept jours.
    pub rolling_mad: Option<f64>,
    /// Nombre de points de l'historique de sept jours.
    pub history: f64,
}

/// Verdict d'une série.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriftVerdict {
    /// La normale retenue.
    pub baseline: f64,
    /// Écart à la normale en nombre d'écarts robustes, dans le sens surveillé
    /// (négatif quand la mesure s'écarte dans l'autre sens).
    pub score: f64,
    /// Combien de fois la normale, dans le sens surveillé (≥ 1 quand la mesure
    /// s'en écarte dans ce sens).
    pub ratio: f64,
    /// Vrai si le score atteint `k` et que l'écart absolu atteint
    /// `drift_min_delta` : la condition a le droit de s'ouvrir.
    pub armed: bool,
    /// Normale saisonnière (vrai) ou médiane mobile (faux).
    pub seasonal: bool,
}

/// Juge une série. `None` : pas assez de volume ni d'historique pour juger, ou
/// mesure sous la valeur minimale.
pub fn evaluate(input: &DriftInput, params: &AnomalyParams) -> Option<DriftVerdict> {
    if !input.current.is_finite() || input.samples < f64::from(params.drift_min_samples) {
        return None;
    }
    let seasonal: Vec<f64> = input.seasonal.iter().copied().filter(|v| v.is_finite()).collect();
    let (baseline, is_seasonal) = if seasonal.len() >= MIN_SEASONAL_REFS {
        (median(&seasonal)?, true)
    } else if input.history >= MIN_HISTORY_POINTS {
        (input.rolling_median.filter(|v| v.is_finite())?, false)
    } else {
        return None;
    };

    let watched = match params.drift_direction {
        DriftDirection::Up => input.current,
        DriftDirection::Down => baseline,
        DriftDirection::Both => input.current.max(baseline),
    };
    if watched < params.drift_min_value {
        return None;
    }

    // L'échelle : la dispersion de la semaine, et celle des semaines de
    // référence quand elles sont plus agitées que la semaine écoulée.
    let rolling = input.rolling_mad.filter(|v| v.is_finite()).unwrap_or(0.0);
    let spread_refs = if is_seasonal {
        let deviations: Vec<f64> = seasonal.iter().map(|v| (v - baseline).abs()).collect();
        median(&deviations).unwrap_or(0.0)
    } else {
        0.0
    };
    let floor = (params.mad_floor_rel * baseline.abs()).max(params.mad_floor_abs);
    let scale = (MAD_TO_SIGMA * rolling.max(spread_refs)).max(floor);

    // Le rapport se mesure à partir de la valeur minimale qui compte : un taux
    // de 5xx habituellement nul qui passe à 4 % vaut « 4 fois le minimum », pas
    // « un million de fois la normale ».
    let deviation = input.current - baseline;
    let level_floor = params.drift_min_value.max(params.mad_floor_abs);
    let up_ratio = input.current / baseline.max(level_floor);
    let down_ratio = baseline / input.current.max(level_floor);
    let (score, ratio) = match params.drift_direction {
        DriftDirection::Up => (deviation / scale, up_ratio),
        DriftDirection::Down => (-deviation / scale, down_ratio),
        DriftDirection::Both => (deviation.abs() / scale, up_ratio.max(down_ratio)),
    };
    let ratio = ratio.clamp(0.0, MAX_RATIO);
    let armed = score >= params.k && deviation.abs() >= params.drift_min_delta;
    Some(DriftVerdict { baseline, score, ratio, armed, seasonal: is_seasonal })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> AnomalyParams {
        AnomalyParams::default()
    }

    fn input(current: f64, seasonal: &[f64]) -> DriftInput {
        DriftInput {
            current,
            samples: 15.0,
            seasonal: seasonal.to_vec(),
            rolling_median: Some(0.12),
            rolling_mad: Some(0.01),
            history: 2016.0,
        }
    }

    #[test]
    fn une_latence_quadruplee_est_une_derive() {
        let verdict = evaluate(&input(0.48, &[0.11, 0.12, 0.13, 0.12]), &params()).expect("judged");
        assert!(verdict.seasonal);
        assert!((verdict.baseline - 0.12).abs() < 1e-9);
        assert!((verdict.ratio - 4.0).abs() < 1e-9, "ratio = {}", verdict.ratio);
        assert!(verdict.armed, "score = {}", verdict.score);
    }

    #[test]
    fn une_variation_ordinaire_ne_s_arme_pas() {
        let verdict = evaluate(&input(0.13, &[0.11, 0.12, 0.13, 0.12]), &params()).expect("judged");
        assert!(!verdict.armed, "score = {}", verdict.score);
    }

    #[test]
    fn la_saisonnalite_prime_sur_la_semaine() {
        // La nuit, la sauvegarde ralentit tout : 0,9 s est la normale de 3 h,
        // alors que la médiane de la semaine est 0,12 s.
        let verdict = evaluate(&input(0.95, &[0.85, 0.9, 0.92, 0.88]), &params()).expect("judged");
        assert!(verdict.ratio < 1.2);
        assert!(!verdict.armed);
    }

    #[test]
    fn sans_semaines_de_reference_la_mediane_mobile_prend_le_relais() {
        let verdict = evaluate(&input(0.6, &[0.11]), &params()).expect("judged");
        assert!(!verdict.seasonal);
        assert!((verdict.baseline - 0.12).abs() < 1e-9);
        assert!(verdict.armed);
    }

    #[test]
    fn une_serie_trop_jeune_ou_trop_maigre_n_est_pas_jugee() {
        let mut jeune = input(0.6, &[]);
        jeune.history = 100.0;
        assert_eq!(evaluate(&jeune, &params()), None);

        let mut maigre = input(0.6, &[0.1, 0.1]);
        maigre.samples = 2.0;
        assert_eq!(evaluate(&maigre, &params()), None, "volume below drift_min_samples");
    }

    #[test]
    fn la_valeur_minimale_ecarte_les_petites_latences() {
        let params = AnomalyParams { drift_min_value: 0.05, ..AnomalyParams::default() };
        // 2 ms → 6 ms : triple, mais sans importance.
        assert_eq!(evaluate(&input(0.006, &[0.002, 0.002, 0.002]), &params), None);
    }

    #[test]
    fn l_ecart_absolu_minimal_desarme() {
        let params = AnomalyParams { drift_min_delta: 0.5, ..AnomalyParams::default() };
        let verdict = evaluate(&input(0.48, &[0.12, 0.12, 0.12]), &params).expect("judged");
        assert!(!verdict.armed, "0.36 s of drift is below the 0.5 s floor");
    }

    #[test]
    fn le_sens_bas_surveille_les_chutes() {
        let params =
            AnomalyParams { drift_direction: DriftDirection::Down, ..AnomalyParams::default() };
        let mut traffic = input(10.0, &[100.0, 105.0, 95.0]);
        traffic.rolling_median = Some(100.0);
        traffic.rolling_mad = Some(5.0);
        let verdict = evaluate(&traffic, &params).expect("judged");
        assert!((verdict.ratio - 10.0).abs() < 1e-9);
        assert!(verdict.armed);

        // Une hausse n'intéresse pas une règle qui surveille les chutes.
        let mut pic = traffic.clone();
        pic.current = 500.0;
        assert!(!evaluate(&pic, &params).expect("judged").armed);
    }

    #[test]
    fn une_normale_nulle_se_mesure_depuis_la_valeur_minimale() {
        let params = AnomalyParams { drift_min_value: 1.0, ..AnomalyParams::default() };
        let mut erreurs = input(4.0, &[0.0, 0.0, 0.0]);
        erreurs.rolling_mad = Some(0.0);
        let verdict = evaluate(&erreurs, &params).expect("judged");
        assert!((verdict.ratio - 4.0).abs() < 1e-9, "ratio = {}", verdict.ratio);
    }

    #[test]
    fn une_normale_a_zero_garde_un_rapport_fini() {
        let mut erreurs = input(3.0, &[0.0, 0.0, 0.0]);
        erreurs.rolling_mad = Some(0.0);
        let verdict = evaluate(&erreurs, &params()).expect("judged");
        assert!(verdict.ratio.is_finite() && verdict.score.is_finite());
        assert!(verdict.armed);
    }
}
