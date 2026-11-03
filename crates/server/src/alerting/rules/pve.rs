//! Règles livrées de Proxmox VE qui complètent celles de `rules.rs` (nœuds,
//! quorum, HA, stockages pleins, sauvegardes, Ceph, ZFS, SMART) : stockage
//! inactif, redémarrage en attente, abonnement, réplication en retard, travaux
//! de sauvegarde silencieux, invités hors sauvegarde, tâches en échec.

use std::time::Duration;

use super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_proxmox_storage_active",
    "dumbmonit_proxmox_storage_enabled",
    "dumbmonit_proxmox_node_reboot_required",
    "dumbmonit_proxmox_node_subscription_info",
    "dumbmonit_proxmox_replication_job_next_sync_seconds",
    "dumbmonit_proxmox_replication_job_enabled",
    "dumbmonit_proxmox_backup_job_last_run_age_seconds",
    "dumbmonit_proxmox_backup_job_enabled",
    "dumbmonit_proxmox_backup_covered",
    "dumbmonit_proxmox_node_tasks_failed",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        // Un stockage NFS, CIFS ou PBS injoignable passe `active` à 0 sans que
        // rien d'autre ne bouge : les machines tournent, les sauvegardes qui le
        // visent échouent. Un stockage désactivé exprès (`enabled` à 0) n'est pas
        // visé.
        Rule {
            description: "An enabled storage is not active on a node (unreachable share, \
                          unmounted disk, backup server down). Backups and disks on it fail."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "pve_storage_inactive",
                "Proxmox storage inactive",
                RuleKind::Threshold,
                "dumbmonit_proxmox_storage_active \
                 and on(target, node, storage) (dumbmonit_proxmox_storage_enabled == 1)",
            )
        },
        // Un noyau installé n'est actif qu'après un redémarrage. Trois jours
        // laissent le temps de choisir un moment, sans l'oublier pour des mois.
        Rule {
            description: "A newer kernel is installed on a node but not running: the node needs \
                          a reboot to apply it."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3 * 24 * 3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pve_reboot_required",
                "Proxmox node needs a reboot",
                RuleKind::Threshold,
                "dumbmonit_proxmox_node_reboot_required",
            )
        },
        // `notfound` est l'état ordinaire d'un homelab sans abonnement : seuls les
        // états qui disent « il y en avait un » sonnent.
        Rule {
            description: "The subscription of a node is expired, invalid or suspended: the \
                          enterprise repository stops serving updates."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pve_subscription_expired",
                "Proxmox subscription expired",
                RuleKind::Threshold,
                "dumbmonit_proxmox_node_subscription_info{status=~\"expired|invalid|suspended\"}",
            )
        },
        // `next_sync` est l'heure du prochain passage prévu : passée de plusieurs
        // heures sur un travail actif, la réplication ne tourne plus (ou reste
        // bloquée), sans pour autant être en erreur. Six heures laissent passer
        // une première synchronisation complète.
        Rule {
            description: "A storage replication job is more than six hours late on its \
                          schedule: the copy on the other node is getting older."
                .to_string(),
            operator: Operator::Gt,
            threshold: 6.0 * 3600.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "pve_replication_overdue",
                "Proxmox replication overdue",
                RuleKind::Threshold,
                "-dumbmonit_proxmox_replication_job_next_sync_seconds \
                 and on(target, job, vmid, node, to_node) \
                 (dumbmonit_proxmox_replication_job_enabled == 1)",
            )
        },
        // Un travail de sauvegarde qui ne démarre plus ne tombe pas en échec : il
        // se tait, et `job_last_ok` garde son dernier verdict. La série n'existe
        // que tant que la dernière exécution est dans la fenêtre d'examen. Huit
        // jours laissent passer un travail hebdomadaire.
        Rule {
            description: "An enabled backup job has not run for more than eight days.".to_string(),
            operator: Operator::Gt,
            threshold: 8.0 * 86_400.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pve_backup_job_stale",
                "Proxmox backup job not running",
                RuleKind::Threshold,
                "dumbmonit_proxmox_backup_job_last_run_age_seconds \
                 and on(target, job) (dumbmonit_proxmox_backup_job_enabled == 1)",
            )
        },
        // PVE liste lui-même les invités qu'aucun travail ne couvre. C'est souvent
        // voulu (machine de test), d'où le niveau d'information et le jour d'attente.
        Rule {
            description: "A guest is not included in any backup job.".to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(24 * 3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "pve_guest_not_backed_up",
                "Proxmox guest not in any backup job",
                RuleKind::Threshold,
                "dumbmonit_proxmox_backup_covered",
            )
        },
        // Un démarrage refusé, une migration avortée, une restauration en erreur
        // ne laissent de trace que dans le journal des tâches. Les sauvegardes, les
        // sessions de console et les arrêts demandés à la main sont écartés par
        // le collecteur ; la fenêtre est de six heures, l'alerte retombe seule.
        Rule {
            description: "Tasks failed on a node in the last six hours (guest start, migration, \
                          restore...). Backups have their own rule."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "pve_tasks_failed",
                "Proxmox tasks failed",
                RuleKind::Threshold,
                "dumbmonit_proxmox_node_tasks_failed",
            )
        },
    ]
}
