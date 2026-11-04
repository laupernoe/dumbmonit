//! Règles livrées de Home Assistant, en plus de celles de `builtin_rules`
//! (mode de récupération, démarrage bloqué, entités indisponibles, piles,
//! mises à jour, réparations) : la sécurité de la maison et les automatisations
//! cassées.

use std::time::Duration;

use super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

/// Les métriques (préfixe `dumbmonit_` compris) que ces règles visent.
#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_homeassistant_safety_alert",
    "dumbmonit_homeassistant_alarm_triggered",
    "dumbmonit_homeassistant_problem",
    "dumbmonit_homeassistant_entities_unavailable",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        // Une série par détecteur déclenché (fumée, gaz, monoxyde, fuite d'eau,
        // classe « sécurité »), qui disparaît dès qu'il repasse à `off`. Une
        // minute écarte un faux contact ; le rappel est court, c'est urgent.
        Rule {
            description: "A smoke, gas, carbon monoxide, water leak or safety sensor of Home \
                          Assistant is triggered."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(30 * 60)),
            ..base(
                "homeassistant_safety_alert",
                "Home Assistant safety sensor triggered",
                RuleKind::Threshold,
                "dumbmonit_homeassistant_safety_alert",
            )
        },
        Rule {
            description: "A Home Assistant alarm panel is in the triggered state.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(30 * 60)),
            ..base(
                "homeassistant_alarm_triggered",
                "Home Assistant alarm triggered",
                RuleKind::Threshold,
                "dumbmonit_homeassistant_alarm_triggered",
            )
        },
        // Un capteur « problème » (onduleur, imprimante, routeur) ou « sabotage »
        // qui reste allumé : une demi-heure laisse passer un cycle normal.
        Rule {
            description: "A problem or tamper binary sensor of Home Assistant has been on for \
                          thirty minutes."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "homeassistant_problem_sensor",
                "Home Assistant problem sensor on",
                RuleKind::Threshold,
                "dumbmonit_homeassistant_problem",
            )
        },
        // Une automatisation devient `unavailable` quand sa configuration ne se
        // charge plus (entité supprimée, modèle d'automatisation modifié) : elle ne
        // s'exécute plus, sans autre signe.
        Rule {
            description: "A Home Assistant automation is unavailable: its configuration no \
                          longer loads, so it never runs."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "homeassistant_automation_unavailable",
                "Home Assistant automation unavailable",
                RuleKind::Threshold,
                "dumbmonit_homeassistant_entities_unavailable{domain=\"automation\"}",
            )
        },
    ]
}
