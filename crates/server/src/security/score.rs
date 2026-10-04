//! Note pondérée et lettre.
//!
//! note = 100 × Σ poids des contrôles réussis / Σ poids des contrôles évalués
//!
//! Les contrôles `unknown` ne comptent ni au numérateur ni au dénominateur.
//! Lettre : A ≥ 90, B ≥ 75, C ≥ 60, D ≥ 40, F en dessous. Un seul contrôle
//! **critique** en échec plafonne la lettre à C, quelle que soit la note : un
//! pare-feu désactivé ne se rattrape pas par dix bons réglages mineurs (même
//! logique que PingCastle, où la pire règle fixe le niveau).

use serde::Serialize;

use super::{Check, Outcome, Severity};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Grade {
    A,
    B,
    C,
    D,
    F,
}

pub fn grade_for(score: u32) -> Grade {
    match score {
        90.. => Grade::A,
        75..=89 => Grade::B,
        60..=74 => Grade::C,
        40..=59 => Grade::D,
        _ => Grade::F,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreOutcome {
    pub score: Option<u32>,
    pub grade: Option<Grade>,
    pub capped: bool,
}

pub fn score_checks(checks: &[Check]) -> ScoreOutcome {
    let weight = |outcome: Outcome| -> u32 {
        checks.iter().filter(|c| c.result == outcome).map(|c| c.weight).sum()
    };
    let passed = weight(Outcome::Pass);
    let evaluated = passed + weight(Outcome::Fail);
    if evaluated == 0 {
        return ScoreOutcome { score: None, grade: None, capped: false };
    }
    let score = ((f64::from(passed) * 100.0) / f64::from(evaluated)).round() as u32;
    let natural = grade_for(score);
    let critical_failed =
        checks.iter().any(|c| c.result == Outcome::Fail && c.severity == Severity::Critical);
    let grade = if critical_failed { natural.max(Grade::C) } else { natural };
    ScoreOutcome { score: Some(score), grade: Some(grade), capped: grade != natural }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{Category, CheckDef};

    const fn def(id: &'static str, severity: Severity) -> CheckDef {
        CheckDef {
            id,
            title: "t",
            category: Category::Configuration,
            severity,
            reference: "https://example.org",
            remediation: "r",
        }
    }
    const LOW: CheckDef = def("low", Severity::Low);
    const MEDIUM: CheckDef = def("medium", Severity::Medium);
    const HIGH: CheckDef = def("high", Severity::High);
    const CRITICAL: CheckDef = def("critical", Severity::Critical);

    #[test]
    fn les_seuils_des_lettres() {
        assert_eq!(grade_for(100), Grade::A);
        assert_eq!(grade_for(90), Grade::A);
        assert_eq!(grade_for(89), Grade::B);
        assert_eq!(grade_for(75), Grade::B);
        assert_eq!(grade_for(74), Grade::C);
        assert_eq!(grade_for(60), Grade::C);
        assert_eq!(grade_for(59), Grade::D);
        assert_eq!(grade_for(40), Grade::D);
        assert_eq!(grade_for(39), Grade::F);
        assert_eq!(grade_for(0), Grade::F);
    }

    #[test]
    fn la_note_est_ponderee_et_ignore_les_inconnus() {
        // 6 réussis sur 6 + 3 évalués = 67 ; l'inconnu critique ne compte pas.
        let checks = [HIGH.pass("ok"), MEDIUM.fail("ko"), CRITICAL.unknown("?"), LOW.unknown("?")];
        let outcome = score_checks(&checks);
        assert_eq!(outcome.score, Some(67));
        assert_eq!(outcome.grade, Some(Grade::C));
        assert!(!outcome.capped);
    }

    #[test]
    fn sans_controle_evalue_rien_n_est_note() {
        let outcome = score_checks(&[HIGH.unknown("?")]);
        assert_eq!(outcome, ScoreOutcome { score: None, grade: None, capped: false });
        assert_eq!(score_checks(&[]).score, None);
    }

    #[test]
    fn tout_reussi_donne_cent_et_a() {
        let outcome = score_checks(&[LOW.pass(""), HIGH.pass("")]);
        assert_eq!(outcome.score, Some(100));
        assert_eq!(outcome.grade, Some(Grade::A));
    }

    #[test]
    fn un_echec_critique_plafonne_a_c() {
        // 10 critiques échoués sur 10 + 6×10 = 70 réussis : 86 → B, plafonné à C.
        let mut checks = vec![CRITICAL.fail("pare-feu coupé")];
        checks.extend((0..10).map(|_| HIGH.pass("")));
        let outcome = score_checks(&checks);
        assert_eq!(outcome.score, Some(86));
        assert_eq!(outcome.grade, Some(Grade::C));
        assert!(outcome.capped);
    }

    #[test]
    fn le_plafond_ne_releve_jamais_une_mauvaise_note() {
        let outcome = score_checks(&[CRITICAL.fail(""), LOW.pass("")]);
        assert_eq!(outcome.grade, Some(Grade::F));
        assert!(!outcome.capped);
    }
}
