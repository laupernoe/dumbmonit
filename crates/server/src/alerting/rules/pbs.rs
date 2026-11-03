//! Règles livrées de Proxmox Backup Server qui complètent celles de `rules.rs`
//! (datastores pleins, sauvegardes trop anciennes, GC, vérifications, travaux,
//! disques) : datastore hors service, maintenance oubliée, système de fichiers
//! racine, travaux planifiés silencieux, abonnement.

use std::time::Duration;

use super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_pbs_datastore_available",
    "dumbmonit_pbs_datastore_removable_unmounted",
    "dumbmonit_pbs_datastore_maintenance",
    "dumbmonit_pbs_node_rootfs_percent",
    "dumbmonit_pbs_job_last_run_age_seconds",
    "dumbmonit_pbs_job_enabled",
    "dumbmonit_pbs_job_next_run_seconds",
    "dumbmonit_pbs_node_subscription_info",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        // Un datastore en erreur (disque débranché, chemin non monté, système de
        // fichiers en lecture seule) refuse toute sauvegarde. Le démontage d'un
        // disque amovible a sa propre règle et n'est pas compté deux fois.
        Rule {
            description: "A datastore is unavailable: backups to it and restores from it fail."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "pbs_datastore_unavailable",
                "PBS datastore unavailable",
                RuleKind::Threshold,
                "dumbmonit_pbs_datastore_available \
                 unless on(target, datastore) (dumbmonit_pbs_datastore_removable_unmounted == 1)",
            )
        },
        // Le mode maintenance se lève à la main : un `read-only` oublié après une
        // migration refuse les sauvegardes des nuits suivantes, un `offline` aussi.
        Rule {
            description: "A datastore has been in maintenance mode (read-only, offline) for more \
                          than 24 hours. Check that it was not left on by mistake."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(24 * 3600),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pbs_datastore_maintenance",
                "PBS datastore left in maintenance mode",
                RuleKind::Threshold,
                "dumbmonit_pbs_datastore_maintenance",
            )
        },
        // PBS écrit son journal, ses tâches et ses index sur le système de
        // fichiers racine : plein, plus aucune tâche ne démarre, même si les
        // datastores ont de la place.
        Rule {
            description: "The root file system of the backup server is more than 90% full. Tasks \
                          stop starting when it fills up, whatever the datastores have left."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            clear_threshold: Some(85.0),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "pbs_rootfs_almost_full",
                "PBS root file system almost full",
                RuleKind::Threshold,
                "dumbmonit_pbs_node_rootfs_percent",
            )
        },
        // Un travail planifié qui ne tourne plus ne tombe pas en erreur : il se
        // tait. `job_last_ok` ne le voit pas, puisqu'il garde son dernier verdict.
        // Seuls les travaux de synchronisation et d'élagage, actifs et pourvus
        // d'une planification, sont visés ; huit jours laissent passer un
        // travail hebdomadaire. Les vérifications, souvent mensuelles, sont exclues.
        Rule {
            description: "A scheduled sync or prune job is enabled but has not run for more than \
                          eight days."
                .to_string(),
            operator: Operator::Gt,
            threshold: 8.0 * 86_400.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pbs_job_stale",
                "PBS scheduled job not running",
                RuleKind::Threshold,
                "dumbmonit_pbs_job_last_run_age_seconds{kind=~\"sync|prune\"} \
                 and on(target, job, kind) (dumbmonit_pbs_job_enabled == 1) \
                 and on(target, job, kind) dumbmonit_pbs_job_next_run_seconds",
            )
        },
        // `notfound` est l'état ordinaire d'un serveur sans abonnement : seuls les
        // états qui disent « il y en avait un » sonnent.
        Rule {
            description: "The subscription of this backup server is expired, invalid or \
                          suspended: the enterprise repository stops serving updates."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pbs_subscription_expired",
                "PBS subscription expired",
                RuleKind::Threshold,
                "dumbmonit_pbs_node_subscription_info{status=~\"expired|invalid|suspended\"}",
            )
        },
    ]
}
