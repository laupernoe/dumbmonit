//! Règles livrées d'Immich au-delà des quatre premières (travaux bloqués, file
//! en pause, stockage, mise à jour), qui restent dans `rules.rs`.

use std::time::Duration;

use super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

/// Métriques visées par ces règles et absentes de `PRODUITES`.
#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_immich_queue_failed",
    "dumbmonit_immich_maintenance_mode",
    "dumbmonit_immich_user_quota_used_percent",
    "dumbmonit_immich_db_backup_last_age_seconds",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        // Les travaux échoués restent comptés jusqu'à ce qu'on les relance ou
        // les efface : le seuil laisse passer les quelques fichiers corrompus
        // d'un gros import et ne retient que les vagues d'échecs.
        Rule {
            description: "An Immich job queue holds 25 or more failed jobs: files could not be \
                          processed. Open Administration, then Jobs, to see the errors and retry \
                          or clear them."
                .to_string(),
            operator: Operator::Ge,
            threshold: 25.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "immich_jobs_failed",
                "Immich jobs failing",
                RuleKind::Threshold,
                "dumbmonit_immich_queue_failed{queue!~\"smartSearch|faceDetection|\
                 facialRecognition|duplicateDetection|ocr\"}",
            )
        },
        // Les files qui appellent le service d'apprentissage automatique : si
        // son conteneur est arrêté ou injoignable, elles échouent toutes.
        Rule {
            description: "The Immich queues that call machine learning (smart search, faces, \
                          duplicates, text recognition) hold 10 or more failed jobs: the machine \
                          learning container is probably stopped, unreachable or out of memory."
                .to_string(),
            operator: Operator::Ge,
            threshold: 10.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "immich_ml_failing",
                "Immich machine learning failing",
                RuleKind::Threshold,
                "dumbmonit_immich_queue_failed{queue=~\"smartSearch|faceDetection|\
                 facialRecognition|duplicateDetection|ocr\"}",
            )
        },
        Rule {
            description: "Immich has been in maintenance mode for 30 minutes: the library is \
                          not served until the maintenance ends (restore or repair)."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "immich_maintenance_mode",
                "Immich in maintenance mode",
                RuleKind::Threshold,
                "dumbmonit_immich_maintenance_mode",
            )
        },
        // Seuls les comptes qui ont un quota produisent la mesure.
        Rule {
            description: "An Immich user has used more than 90% of their storage quota: new \
                          uploads will be refused once it is full."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            clear_threshold: Some(85.0),
            for_duration: Duration::from_secs(60 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "immich_user_quota_almost_full",
                "Immich user quota almost full",
                RuleKind::Threshold,
                "dumbmonit_immich_user_quota_used_percent",
            )
        },
        // Immich sauvegarde sa base chaque nuit quand l'option est active ; trois
        // jours sans nouveau fichier, c'est la tâche qui échoue ou une option
        // coupée après coup. Sans aucune sauvegarde, la mesure n'existe pas.
        Rule {
            description: "The newest Immich database backup is more than three days old: the \
                          nightly backup job is failing or was turned off."
                .to_string(),
            operator: Operator::Gt,
            threshold: 3.0 * 24.0 * 3600.0,
            for_duration: Duration::from_secs(60 * 60),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "immich_db_backup_stale",
                "Immich database backup stale",
                RuleKind::Threshold,
                "dumbmonit_immich_db_backup_last_age_seconds",
            )
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_regles_immich_visent_des_metriques_declarees() {
        let rules = rules();
        let uids: Vec<&str> = rules.iter().map(|r| r.uid.as_str()).collect();
        assert_eq!(
            uids,
            [
                "immich_jobs_failed",
                "immich_ml_failing",
                "immich_maintenance_mode",
                "immich_user_quota_almost_full",
                "immich_db_backup_stale",
            ]
        );
        for rule in &rules {
            assert!(rule.builtin && rule.enabled);
            for mot in rule.query.split(|c: char| !c.is_alphanumeric() && c != '_') {
                if mot.starts_with("dumbmonit_") {
                    assert!(PRODUCED.contains(&mot), "{}: {mot}", rule.uid);
                }
            }
        }
    }

    #[test]
    fn les_deux_regles_d_echec_se_partagent_les_files_sans_recouvrement() {
        let rules = rules();
        let generic = &rules[0].query;
        let ml = &rules[1].query;
        assert!(generic.contains("queue!~") && ml.contains("queue=~"));
        // Même liste de files des deux côtés.
        let list = |q: &str| q.split('"').nth(1).unwrap().to_string();
        assert_eq!(list(generic), list(ml));
    }
}
