//! Règles livrées de Nextcloud en plus de celles de `rules.rs` (maintenance,
//! mise à niveau en attente, mise à jour disponible, OPcache plein, espace libre)
//! — collecteur `collectors/selfhosted/nextcloud.rs`.
//!
//! Le jeton de supervision ne lit ni les tâches de fond ni le contrôle d'intégrité
//! (réservés à un compte d'administration complet) : la règle d'ancienneté de la
//! vérification des mises à jour, tâche de fond quotidienne, est le seul témoin
//! de cron que cette API laisse voir.

use std::time::Duration;

use super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

/// Les métriques (préfixe `dumbmonit_` compris) que ces règles visent.
#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_nextcloud_debug_mode",
    "dumbmonit_nextcloud_opcache_cached_keys_used_percent",
    "dumbmonit_nextcloud_opcache_oom_restarts",
    "dumbmonit_nextcloud_update_check_age_seconds",
    "dumbmonit_nextcloud_php_memory_limit_bytes",
    "dumbmonit_nextcloud_fpm_max_children_reached",
    "dumbmonit_nextcloud_fpm_listen_queue",
    "dumbmonit_nextcloud_extended_support",
    "dumbmonit_nextcloud_app_updates_available",
];

pub(super) fn rules() -> Vec<Rule> {
    vec![
        Rule {
            description: "Nextcloud runs in debug mode: it is slower, writes far more to its \
                          log and may show stack traces to users. Set 'debug' => false in \
                          config.php."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "nextcloud_debug_mode",
                "Nextcloud in debug mode",
                RuleKind::Threshold,
                "dumbmonit_nextcloud_debug_mode",
            )
        },
        // Les emplacements de clés se remplissent avant la mémoire : `cache_full`
        // reste faux pendant que PHP redémarre le cache à chaque dépassement.
        Rule {
            description: "95% of OPcache's script slots are used: PHP will start flushing the \
                          cache. Raise opcache.max_accelerated_files."
                .to_string(),
            operator: Operator::Ge,
            threshold: 95.0,
            clear_threshold: Some(90.0),
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "nextcloud_opcache_keys_full",
                "Nextcloud OPcache script slots almost full",
                RuleKind::Threshold,
                "dumbmonit_nextcloud_opcache_cached_keys_used_percent",
            )
        },
        Rule {
            description: "PHP restarted OPcache because it ran out of memory in the last hour: \
                          pages are slow while it refills. Raise opcache.memory_consumption."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "nextcloud_opcache_restarts",
                "Nextcloud OPcache restarting",
                RuleKind::Threshold,
                "increase_prometheus(dumbmonit_nextcloud_opcache_oom_restarts[1h])",
            )
        },
        // La vérification des mises à jour est une tâche de fond quotidienne. Quatorze
        // jours sans elle : le cron ne tourne pas (ou le vérificateur a été coupé
        // après avoir tourné, auquel cas on peut éteindre la règle).
        Rule {
            description: "Nextcloud's daily update check has not run for 14 days: its background \
                          jobs (cron) are most likely not running, so previews, notifications, \
                          file scans and cleanups are not happening."
                .to_string(),
            operator: Operator::Gt,
            threshold: 14.0 * 86400.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "nextcloud_background_jobs_stale",
                "Nextcloud background jobs seem stopped",
                RuleKind::Threshold,
                "dumbmonit_nextcloud_update_check_age_seconds",
            )
        },
        Rule {
            description: "Nextcloud's PHP memory_limit is below the 512 MiB it recommends \
                          (unlimited is not flagged)."
                .to_string(),
            operator: Operator::Lt,
            threshold: 536_870_912.0,
            for_duration: Duration::from_secs(24 * 3600),
            severity: Severity::Info,
            unit: "B".to_string(),
            repeat_interval: Some(Duration::from_secs(14 * 24 * 3600)),
            ..base(
                "nextcloud_php_memory_low",
                "Nextcloud PHP memory limit low",
                RuleKind::Threshold,
                "dumbmonit_nextcloud_php_memory_limit_bytes > 0",
            )
        },
        Rule {
            description: "PHP-FPM ran out of worker processes (pm.max_children reached) in the \
                          last 30 minutes: requests queue up and Nextcloud feels slow."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "nextcloud_fpm_max_children",
                "Nextcloud PHP-FPM out of workers",
                RuleKind::Threshold,
                "increase_prometheus(dumbmonit_nextcloud_fpm_max_children_reached[30m])",
            )
        },
        Rule {
            description: "More than 10 requests have waited for a free PHP-FPM worker for 10 \
                          minutes."
                .to_string(),
            operator: Operator::Gt,
            threshold: 10.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "nextcloud_fpm_queue",
                "Nextcloud PHP-FPM queue building up",
                RuleKind::Threshold,
                "dumbmonit_nextcloud_fpm_listen_queue",
            )
        },
        Rule {
            description: "Apps have updates waiting for three days; they often carry security \
                          fixes."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3 * 24 * 3600),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "nextcloud_app_updates_waiting",
                "Nextcloud app updates waiting",
                RuleKind::Threshold,
                "dumbmonit_nextcloud_app_updates_available",
            )
        },
        Rule {
            description: "This Nextcloud version is out of standard support: only Enterprise \
                          subscribers still get fixes. Upgrade."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "nextcloud_extended_support",
                "Nextcloud version out of standard support",
                RuleKind::Threshold,
                "dumbmonit_nextcloud_extended_support",
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
        assert!(uids.iter().all(|uid| uid.starts_with("nextcloud_")), "{uids:?}");
        uids.sort_unstable();
        let total = uids.len();
        uids.dedup();
        assert_eq!(uids.len(), total, "two Nextcloud rules share a uid");
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
    fn la_limite_de_memoire_illimitee_ne_sonne_pas() {
        let rule = rules().into_iter().find(|r| r.uid == "nextcloud_php_memory_low").unwrap();
        // PHP note l'absence de limite -1 : la requête l'écarte avant la comparaison.
        assert!(rule.query.ends_with("> 0"), "{}", rule.query);
        assert_eq!(rule.operator, Operator::Lt);
    }

    #[test]
    fn le_temoin_de_cron_attend_deux_semaines() {
        let rule =
            rules().into_iter().find(|r| r.uid == "nextcloud_background_jobs_stale").unwrap();
        assert_eq!(rule.threshold, 14.0 * 86400.0);
    }

    #[test]
    fn aucune_regle_n_est_sans_duree() {
        assert!(rules().iter().all(|r| r.for_duration > Duration::ZERO));
    }
}
