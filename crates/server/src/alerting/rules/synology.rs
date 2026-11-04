//! Règles livrées de Synology DSM en plus de celles de `rules.rs` (disques,
//! volumes, groupes, température, mémoire, Active Backup) — collecteur
//! `collectors/synology/`.
//!
//! Ces règles couvrent des séries que le collecteur publiait sans qu'aucune règle
//! ne les regarde (système écroulé, processeur, Hyper Backup, mise à jour de DSM)
//! et les séries ajoutées avec elles (volume en lecture seule ou verrouillé,
//! certificats, vérification des données).

use std::time::Duration;

use super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

/// Les métriques (préfixe `dumbmonit_` compris) que ces règles visent.
#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_synology_system_crashed",
    "dumbmonit_synology_system_need_repair",
    "dumbmonit_synology_cpu_usage_percent",
    "dumbmonit_synology_swap_usage_percent",
    "dumbmonit_synology_update_available",
    "dumbmonit_synology_backup_last_result",
    "dumbmonit_synology_backup_last_success_age_seconds",
    "dumbmonit_synology_volume_read_only",
    "dumbmonit_synology_volume_locked",
    "dumbmonit_synology_volume_inode_full",
    "dumbmonit_synology_pool_scrub_age_seconds",
    "dumbmonit_synology_certificate_expires_in_seconds",
    "dumbmonit_synology_certificate_broken",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        Rule {
            description: "DSM reports its system partition as crashed: the NAS may not survive \
                          a reboot. Check Storage Manager."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "synology_system_crashed",
                "Synology system partition crashed",
                RuleKind::Threshold,
                "dumbmonit_synology_system_crashed",
            )
        },
        Rule {
            description: "DSM says its system partition needs repair (Storage Manager offers \
                          to repair it)."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "synology_system_need_repair",
                "Synology system partition needs repair",
                RuleKind::Threshold,
                "dumbmonit_synology_system_need_repair",
            )
        },
        // L'occupation du processeur d'un Synology ne passe pas par la règle
        // générique (`cpu_high`) : elle n'a pas la même métrique.
        Rule {
            description: "Synology CPU above 90% for 15 minutes (a data scrub or indexing run \
                          usually ends well before)."
                .to_string(),
            operator: Operator::Gt,
            threshold: 90.0,
            clear_threshold: Some(85.0),
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "synology_cpu_high",
                "Synology CPU sustained high",
                RuleKind::Threshold,
                "dumbmonit_synology_cpu_usage_percent",
            )
        },
        Rule {
            description: "More than 60% of the Synology's swap is in use for an hour: the NAS \
                          is short of memory."
                .to_string(),
            operator: Operator::Gt,
            threshold: 60.0,
            clear_threshold: Some(50.0),
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "synology_swap_high",
                "Synology swap heavily used",
                RuleKind::Threshold,
                "dumbmonit_synology_swap_usage_percent",
            )
        },
        Rule {
            description: "A DSM update is available.".to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "synology_dsm_update_available",
                "Synology DSM update available",
                RuleKind::Threshold,
                "dumbmonit_synology_update_available",
            )
        },
        // `backup_last_result` suit l'échelle des états de DSM : 0 réussi ou en cours,
        // 1 jamais exécuté, annulé ou suspendu, 2 échec (destination absente,
        // somme de contrôle fausse…). Seul l'échec réveille.
        Rule {
            description: "The last run of this Hyper Backup task failed (unreachable \
                          destination, checksum error…)."
                .to_string(),
            operator: Operator::Ge,
            threshold: 2.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            escalate_after: Some(Duration::from_secs(48 * 3600)),
            ..base(
                "synology_backup_failed",
                "Synology Hyper Backup failed",
                RuleKind::Threshold,
                "dumbmonit_synology_backup_last_result",
            )
        },
        // Huit jours couvrent une sauvegarde hebdomadaire. Une tâche mensuelle doit
        // allonger le seuil ou couper la règle pour elle.
        Rule {
            description: "No successful run of this Hyper Backup task for more than 8 days: it \
                          is disabled, suspended or never gets to run. Monthly tasks need a \
                          longer limit."
                .to_string(),
            operator: Operator::Gt,
            threshold: 8.0 * 86400.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "synology_backup_stale",
                "Synology Hyper Backup success too old",
                RuleKind::Threshold,
                "dumbmonit_synology_backup_last_success_age_seconds",
            )
        },
        Rule {
            description: "A Synology volume is read-only: nothing can write to it (file system \
                          errors or a degraded volume). Check Storage Manager."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Critical,
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "synology_volume_read_only",
                "Synology volume read-only",
                RuleKind::Threshold,
                "dumbmonit_synology_volume_read_only",
            )
        },
        Rule {
            description: "An encrypted Synology volume is locked: its shared folders are \
                          unavailable until the key is entered or the key vault is mounted."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "synology_volume_locked",
                "Synology volume locked",
                RuleKind::Threshold,
                "dumbmonit_synology_volume_locked",
            )
        },
        Rule {
            description: "A Synology volume is out of inodes: no file can be created, even \
                          with free space."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "synology_volume_inodes_full",
                "Synology volume out of inodes",
                RuleKind::Threshold,
                "dumbmonit_synology_volume_inode_full",
            )
        },
        // Sans vérification des données, des erreurs silencieuses s'accumulent sur
        // les disques jusqu'à la reconstruction qui les révèle. Elle n'a de série que
        // si une vérification a déjà eu lieu.
        Rule {
            description: "The last data scrubbing of this storage pool finished more than 200 \
                          days ago. Schedule a regular scrub in Storage Manager."
                .to_string(),
            operator: Operator::Gt,
            threshold: 200.0 * 86400.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(30 * 24 * 3600)),
            ..base(
                "synology_scrub_overdue",
                "Synology data scrubbing overdue",
                RuleKind::Threshold,
                "dumbmonit_synology_pool_scrub_age_seconds",
            )
        },
        // Certificats de DSM : un Let's Encrypt se renouvelle à trente jours de
        // l'échéance, quatorze jours restants veulent dire que le renouvellement
        // échoue. La valeur devient négative une fois le certificat périmé.
        Rule {
            description: "A DSM certificate expires in less than 14 days: automatic renewal \
                          is probably failing."
                .to_string(),
            operator: Operator::Lt,
            threshold: 14.0 * 86400.0,
            clear_threshold: Some(15.0 * 86400.0),
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "synology_certificate_expiring",
                "Synology certificate expiring",
                RuleKind::Threshold,
                "dumbmonit_synology_certificate_expires_in_seconds",
            )
        },
        Rule {
            description: "A DSM certificate expires in less than 3 days, or already has."
                .to_string(),
            operator: Operator::Lt,
            threshold: 3.0 * 86400.0,
            clear_threshold: Some(4.0 * 86400.0),
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "synology_certificate_expired",
                "Synology certificate about to expire",
                RuleKind::Threshold,
                "dumbmonit_synology_certificate_expires_in_seconds",
            )
        },
        Rule {
            description: "DSM marks a certificate as broken (its key or chain cannot be read)."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "synology_certificate_broken",
                "Synology certificate broken",
                RuleKind::Threshold,
                "dumbmonit_synology_certificate_broken",
            )
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_identifiants_sont_uniques_et_prefixes() {
        let rules = rules();
        let mut uids: Vec<&str> = rules.iter().map(|r| r.uid.as_str()).collect();
        assert!(uids.iter().all(|uid| uid.starts_with("synology_")), "{uids:?}");
        uids.sort_unstable();
        let total = uids.len();
        uids.dedup();
        assert_eq!(uids.len(), total, "two Synology rules share a uid");
    }

    #[test]
    fn chaque_metrique_visee_est_declaree_produite() {
        for rule in rules() {
            for mot in rule.query.split(|c: char| !c.is_alphanumeric() && c != '_') {
                if mot.starts_with("dumbmonit_") {
                    assert!(PRODUCED.contains(&mot), "{} cites {mot}", rule.uid);
                }
            }
        }
    }

    #[test]
    fn seul_l_echec_de_hyper_backup_reveille() {
        let rule = rules().into_iter().find(|r| r.uid == "synology_backup_failed").unwrap();
        // 0 réussi ou en cours, 1 jamais exécutée ou annulée, 2 échec.
        assert_eq!(rule.operator, Operator::Ge);
        assert_eq!(rule.threshold, 2.0);
    }

    #[test]
    fn les_deux_seuils_de_certificat_s_emboitent() {
        let all = rules();
        let soon = all.iter().find(|r| r.uid == "synology_certificate_expiring").unwrap();
        let now = all.iter().find(|r| r.uid == "synology_certificate_expired").unwrap();
        assert_eq!(soon.query, now.query);
        assert!(now.threshold < soon.threshold);
        assert!(now.severity > soon.severity);
        assert!(soon.clear_threshold.unwrap() > soon.threshold);
    }

    #[test]
    fn aucune_regle_n_est_sans_duree() {
        assert!(rules().iter().all(|r| r.for_duration > Duration::ZERO));
    }
}
