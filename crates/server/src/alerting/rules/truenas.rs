//! Règles livrées de TrueNAS pour les mesures de profondeur du collecteur
//! (`collectors/truenas/extra.rs`) : certificats, pool de démarrage, tâches de
//! sauvegarde distantes, applications, machines virtuelles. Les règles de base
//! (pools, disques, instantanés, réplications) restent dans `rules.rs`.

use std::time::Duration;

use super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_truenas_certificate_expiry_days",
    "dumbmonit_truenas_boot_pool_healthy",
    "dumbmonit_truenas_boot_pool_used_percent",
    "dumbmonit_truenas_backup_task_failed",
    "dumbmonit_truenas_backup_task_last_run_age_seconds",
    "dumbmonit_truenas_app_running",
    "dumbmonit_truenas_app_crashed",
    "dumbmonit_truenas_app_upgrade_available",
    "dumbmonit_truenas_vm_running",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        // Seul le certificat qui sert l'interface web compte (`in_use="1"`) : un
        // certificat de rechange périmé ne gêne personne. Quand `system/general`
        // est illisible, aucun certificat n'est marqué et la règle se tait.
        Rule {
            description: "The certificate serving the TrueNAS web interface expires in less \
                          than 14 days. Browsers, apps and replication peers will refuse it."
                .to_string(),
            operator: Operator::Lt,
            threshold: 14.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "d".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "truenas_certificate_expiring",
                "NAS certificate expiring",
                RuleKind::Threshold,
                "dumbmonit_truenas_certificate_expiry_days{in_use=\"1\"}",
            )
        },
        Rule {
            description: "The certificate serving the TrueNAS web interface expires in less \
                          than 2 days, or has already expired."
                .to_string(),
            operator: Operator::Lt,
            threshold: 2.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Critical,
            unit: "d".to_string(),
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "truenas_certificate_expired",
                "NAS certificate expired",
                RuleKind::Threshold,
                "dumbmonit_truenas_certificate_expiry_days{in_use=\"1\"}",
            )
        },
        // Un miroir de démarrage dégradé démarre encore : c'est le disque suivant
        // qui laisse le NAS sans système.
        Rule {
            description: "The boot pool is no longer healthy. The NAS still boots from the \
                          remaining device, until that one fails too."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "truenas_boot_pool_degraded",
                "NAS boot pool degraded",
                RuleKind::Threshold,
                "dumbmonit_truenas_boot_pool_healthy",
            )
        },
        // Les anciens environnements de démarrage s'accumulent à chaque mise à
        // jour ; un pool de démarrage plein empêche la suivante.
        Rule {
            description: "The boot pool is more than 85% full. Old boot environments pile up \
                          with every update and block the next one."
                .to_string(),
            operator: Operator::Gt,
            threshold: 85.0,
            clear_threshold: Some(80.0),
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "truenas_boot_pool_almost_full",
                "NAS boot pool almost full",
                RuleKind::Threshold,
                "dumbmonit_truenas_boot_pool_used_percent",
            )
        },
        // Cloud sync, rsync et sauvegarde cloud : la série vaut 1 quand le dernier
        // travail a échoué, 0 quand il a réussi, et n'existe ni pour une tâche
        // désactivée ni pour une tâche jamais lancée ou en cours.
        Rule {
            description: "The last run of a cloud sync, rsync or cloud backup task failed: the \
                          remote copy is getting older."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "truenas_backup_task_failed",
                "NAS remote backup task failed",
                RuleKind::Threshold,
                "dumbmonit_truenas_backup_task_failed",
            )
        },
        Rule {
            description: "An app is in the crashed state.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "truenas_app_crashed",
                "NAS app crashed",
                RuleKind::Threshold,
                "dumbmonit_truenas_app_crashed",
            )
        },
        // Seules les machines qui démarrent avec le NAS sont mesurées : une machine
        // éteinte exprès n'a pas de série.
        Rule {
            description: "A virtual machine set to start with the NAS is not running.".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "truenas_vm_down",
                "NAS virtual machine stopped",
                RuleKind::Threshold,
                "dumbmonit_truenas_vm_running",
            )
        },
    ]
}
