//! Règles livrées de PostgreSQL (`collectors/uptime/sql/pgstats.rs`).
//!
//! La disponibilité (`probe_success`) reste couverte par « Monitor unreachable » et
//! les règles de disponibilité ; ici, la santé interne du serveur.
//!
//! Principe : aucune règle ne doit sonner pour un comportement normal d'une base
//! saine. Les sauvegardes `pg_dump` tiennent une transaction ouverte des heures
//! (seuil de la transaction longue en conséquence), un primaire au repos n'a pas de
//! retard de réplication (le collecteur le dit déjà à zéro), et une base froide a
//! un taux de cache bas sans que rien ne soit cassé (règle d'information, avec un
//! plancher de volume).

use std::time::Duration;

use super::base;
use crate::alerting::model::{Operator, Rule, RuleKind, Severity};

/// Les métriques (préfixe `dumbmonit_` compris) que ces règles visent.
#[cfg(test)]
pub(super) const PRODUCED: &[&str] = &[
    "dumbmonit_postgres_connections_used_percent",
    "dumbmonit_postgres_longest_transaction_seconds",
    "dumbmonit_postgres_longest_idle_in_transaction_seconds",
    "dumbmonit_postgres_locks_waiting",
    "dumbmonit_postgres_database_wraparound_percent",
    "dumbmonit_postgres_replica_streaming",
    "dumbmonit_postgres_replica_lag_seconds",
    "dumbmonit_postgres_replication_slot_retained_bytes",
    "dumbmonit_postgres_replication_slot_active",
    "dumbmonit_postgres_standby_replay_lag_seconds",
    "dumbmonit_postgres_archiver_failing",
    "dumbmonit_postgres_prepared_transactions_oldest_seconds",
    "dumbmonit_postgres_database_blocks_hit_total",
    "dumbmonit_postgres_database_blocks_read_total",
    "dumbmonit_postgres_table_dead_tuple_percent",
    "dumbmonit_postgres_database_deadlocks_total",
    "dumbmonit_postgres_uptime_seconds",
];

/// Taux de réussite du cache sur la dernière heure, par base, avec un plancher de
/// cent mille blocs lus ou servis : sans lui, une base presque inactive passerait
/// pour mal cachée. Le taux cumulé depuis le dernier `pg_stat_reset` ne servirait à
/// rien ici, d'où les compteurs.
const CACHE_HIT_QUERY: &str = "(100 * sum by (target, host, database) \
     (increase_prometheus(dumbmonit_postgres_database_blocks_hit_total[1h])) \
     / (sum by (target, host, database) \
        (increase_prometheus(dumbmonit_postgres_database_blocks_hit_total[1h])) \
      + sum by (target, host, database) \
        (increase_prometheus(dumbmonit_postgres_database_blocks_read_total[1h])))) \
     and (sum by (target, host, database) \
        (increase_prometheus(dumbmonit_postgres_database_blocks_hit_total[1h])) \
      + sum by (target, host, database) \
        (increase_prometheus(dumbmonit_postgres_database_blocks_read_total[1h])) >= 100000)";

/// Volume de WAL gardé par un slot inactif, en Gio : un slot actif qui retient du
/// WAL est un réplica un peu en retard, pas un risque pour le disque.
const IDLE_SLOT_QUERY: &str = "dumbmonit_postgres_replication_slot_retained_bytes / 1073741824 \
     and on (target, slot) dumbmonit_postgres_replication_slot_active == 0";

pub(super) fn rules() -> Vec<Rule> {
    vec![
        Rule {
            description: "More than 85% of the connections ordinary roles may open are in use: \
                          new clients will soon be refused."
                .to_string(),
            operator: Operator::Gt,
            threshold: 85.0,
            clear_threshold: Some(80.0),
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "%".to_string(),
            escalate_after: Some(Duration::from_secs(3600)),
            ..base(
                "postgres_connections_high",
                "PostgreSQL connections near the limit",
                RuleKind::Threshold,
                "dumbmonit_postgres_connections_used_percent",
            )
        },
        Rule {
            description: "A transaction has been open for more than 6 hours. It keeps vacuum \
                          from cleaning up and tables grow; backups (pg_dump) are normally \
                          shorter."
                .to_string(),
            operator: Operator::Gt,
            threshold: 6.0 * 3600.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "postgres_long_transaction",
                "PostgreSQL long transaction",
                RuleKind::Threshold,
                "dumbmonit_postgres_longest_transaction_seconds",
            )
        },
        Rule {
            description: "A session has been idle inside an open transaction for more than 15 \
                          minutes: an application forgot to commit, and it holds locks and \
                          blocks vacuum."
                .to_string(),
            operator: Operator::Gt,
            threshold: 900.0,
            for_duration: Duration::from_secs(5 * 60),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            ..base(
                "postgres_idle_in_transaction",
                "PostgreSQL session idle in transaction",
                RuleKind::Threshold,
                "dumbmonit_postgres_longest_idle_in_transaction_seconds",
            )
        },
        Rule {
            description: "Queries have been waiting for a lock for 10 minutes: something holds \
                          a lock and does not let go."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "postgres_lock_waits",
                "PostgreSQL lock waits",
                RuleKind::Threshold,
                "dumbmonit_postgres_locks_waiting",
            )
        },
        Rule {
            description: "The oldest transaction ID of a database is over 40% of the way to \
                          wraparound: autovacuum is not freezing old rows (it normally keeps \
                          this under 10%)."
                .to_string(),
            operator: Operator::Ge,
            threshold: 40.0,
            clear_threshold: Some(35.0),
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "postgres_wraparound_warning",
                "PostgreSQL transaction ID age high",
                RuleKind::Threshold,
                "dumbmonit_postgres_database_wraparound_percent",
            )
        },
        Rule {
            description: "The oldest transaction ID of a database is over 75% of the way to \
                          wraparound: PostgreSQL will stop accepting writes to protect itself. \
                          Run VACUUM FREEZE and look for what blocks autovacuum."
                .to_string(),
            operator: Operator::Ge,
            threshold: 75.0,
            clear_threshold: Some(70.0),
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Critical,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(3600)),
            ..base(
                "postgres_wraparound_critical",
                "PostgreSQL transaction ID wraparound imminent",
                RuleKind::Threshold,
                "dumbmonit_postgres_database_wraparound_percent",
            )
        },
        Rule {
            description: "A replica is connected but not streaming (catching up, starting, or \
                          stuck) for 10 minutes."
                .to_string(),
            operator: Operator::Lt,
            threshold: 1.0,
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "postgres_replica_not_streaming",
                "PostgreSQL replica not streaming",
                RuleKind::Threshold,
                "dumbmonit_postgres_replica_streaming",
            )
        },
        Rule {
            description: "A replica replays more than 5 minutes behind its primary.".to_string(),
            operator: Operator::Gt,
            threshold: 300.0,
            clear_threshold: Some(120.0),
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "postgres_replica_lagging",
                "PostgreSQL replica lagging",
                RuleKind::Threshold,
                "dumbmonit_postgres_replica_lag_seconds",
            )
        },
        Rule {
            description: "This standby replays more than 5 minutes behind (0 when it has \
                          replayed everything it received)."
                .to_string(),
            operator: Operator::Gt,
            threshold: 300.0,
            clear_threshold: Some(120.0),
            for_duration: Duration::from_secs(10 * 60),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "postgres_standby_lagging",
                "PostgreSQL standby lagging",
                RuleKind::Threshold,
                "dumbmonit_postgres_standby_replay_lag_seconds",
            )
        },
        Rule {
            description: "An inactive replication slot keeps more than 5 GiB of WAL on the \
                          primary's disk. Drop the slot if its consumer is gone for good."
                .to_string(),
            operator: Operator::Gt,
            threshold: 5.0,
            clear_threshold: Some(4.0),
            for_duration: Duration::from_secs(3600),
            severity: Severity::Warning,
            unit: "GiB".to_string(),
            repeat_interval: Some(Duration::from_secs(12 * 3600)),
            escalate_after: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "postgres_slot_holding_wal",
                "PostgreSQL replication slot holding WAL",
                RuleKind::Threshold,
                IDLE_SLOT_QUERY,
            )
        },
        Rule {
            description: "The last WAL archive attempt failed and none has succeeded since for \
                          15 minutes: WAL piles up on disk and point-in-time recovery has a \
                          gap."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(15 * 60),
            severity: Severity::Warning,
            repeat_interval: Some(Duration::from_secs(6 * 3600)),
            escalate_after: Some(Duration::from_secs(6 * 3600)),
            ..base(
                "postgres_archiving_failing",
                "PostgreSQL WAL archiving failing",
                RuleKind::Threshold,
                "dumbmonit_postgres_archiver_failing",
            )
        },
        Rule {
            description: "A prepared (two-phase) transaction has been left open for more than \
                          an hour. It is usually orphaned, and it blocks vacuum until \
                          committed or rolled back."
                .to_string(),
            operator: Operator::Gt,
            threshold: 3600.0,
            for_duration: Duration::from_secs(30 * 60),
            severity: Severity::Warning,
            unit: "s".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "postgres_prepared_transaction_old",
                "PostgreSQL prepared transaction left open",
                RuleKind::Threshold,
                "dumbmonit_postgres_prepared_transactions_oldest_seconds",
            )
        },
        Rule {
            description: "A table has more than 30% dead rows for 6 hours: autovacuum is not \
                          keeping up or is blocked by an old transaction."
                .to_string(),
            operator: Operator::Gt,
            threshold: 30.0,
            clear_threshold: Some(20.0),
            for_duration: Duration::from_secs(6 * 3600),
            severity: Severity::Warning,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "postgres_dead_rows",
                "PostgreSQL table full of dead rows",
                RuleKind::Threshold,
                "dumbmonit_postgres_table_dead_tuple_percent",
            )
        },
        Rule {
            description: "Under 90% of a database's block reads over the last hour were served \
                          from memory: the working set no longer fits in shared_buffers and \
                          the OS cache."
                .to_string(),
            operator: Operator::Lt,
            threshold: 90.0,
            clear_threshold: Some(93.0),
            for_duration: Duration::from_secs(3600),
            severity: Severity::Info,
            unit: "%".to_string(),
            repeat_interval: Some(Duration::from_secs(7 * 24 * 3600)),
            ..base(
                "postgres_cache_hit_low",
                "PostgreSQL cache hit ratio low",
                RuleKind::Threshold,
                CACHE_HIT_QUERY,
            )
        },
        Rule {
            description: "PostgreSQL detected a deadlock in the last 15 minutes and rolled one \
                          of the transactions back: the application takes locks in an \
                          inconsistent order."
                .to_string(),
            operator: Operator::Gt,
            threshold: 0.0,
            for_duration: Duration::from_secs(60),
            severity: Severity::Info,
            repeat_interval: Some(Duration::from_secs(24 * 3600)),
            ..base(
                "postgres_deadlocks",
                "PostgreSQL deadlock",
                RuleKind::Threshold,
                "increase_prometheus(dumbmonit_postgres_database_deadlocks_total[15m])",
            )
        },
        Rule {
            description: "PostgreSQL started less than 5 minutes ago: it restarted, perhaps \
                          after a crash."
                .to_string(),
            operator: Operator::Lt,
            threshold: 300.0,
            for_duration: Duration::from_secs(60),
            severity: Severity::Info,
            unit: "s".to_string(),
            repeat_interval: None,
            ..base(
                "postgres_restarted",
                "PostgreSQL restarted",
                RuleKind::Threshold,
                "dumbmonit_postgres_uptime_seconds",
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
        assert!(uids.iter().all(|uid| uid.starts_with("postgres_")), "{uids:?}");
        uids.sort_unstable();
        let total = uids.len();
        uids.dedup();
        assert_eq!(uids.len(), total, "two PostgreSQL rules share a uid");
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
    fn la_transaction_longue_tolere_une_sauvegarde_pg_dump() {
        let rule = rules().into_iter().find(|r| r.uid == "postgres_long_transaction").unwrap();
        assert!(rule.threshold >= 6.0 * 3600.0);
        assert!(rule.for_duration >= Duration::from_secs(600));
    }

    #[test]
    fn le_slot_inactif_ne_sonne_pas_pour_un_slot_actif() {
        let rule = rules().into_iter().find(|r| r.uid == "postgres_slot_holding_wal").unwrap();
        assert!(rule.query.contains("replication_slot_active == 0"), "{}", rule.query);
        assert_eq!(rule.operator, Operator::Gt);
    }

    #[test]
    fn le_taux_de_cache_a_un_plancher_de_volume() {
        let rule = rules().into_iter().find(|r| r.uid == "postgres_cache_hit_low").unwrap();
        assert!(rule.query.contains(">= 100000"), "{}", rule.query);
        assert_eq!(rule.severity, Severity::Info);
    }

    #[test]
    fn les_regles_sont_livrees_actives_et_sans_canal_impose() {
        for rule in rules() {
            assert!(rule.enabled && rule.builtin && rule.channels.is_empty(), "{}", rule.uid);
        }
    }
}
