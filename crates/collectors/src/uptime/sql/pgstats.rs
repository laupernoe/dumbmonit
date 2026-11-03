//! Santé interne d'un serveur PostgreSQL, lue sur la connexion de la sonde.
//!
//! La sonde de disponibilité dit « la base répond ». Ce module dit *comment elle
//! va* : connexions contre `max_connections`, transactions qui traînent, verrous
//! en attente, réplicas en retard, slots qui retiennent du WAL, âge des
//! identifiants de transaction (le jour où la base refuse d'écrire), tables
//! gonflées, archivage WAL.
//!
//! # Règles de conduite
//!
//! * **Aucun échec n'est une panne.** Chaque lecture est indépendante : un compte
//!   sans `pg_monitor` voit moins de lignes de `pg_stat_activity`, un serveur
//!   ancien n'a pas telle colonne ; la lecture est alors omise, jamais comptée
//!   contre la sonde (`probe_success` ne dépend pas d'ici).
//! * **Même connexion, même budget.** Aucune connexion de plus n'est ouverte, et
//!   le temps passé ici est borné par ce que la sonde n'a pas consommé : voir
//!   [`collect`]. Le temps passé est retranché de `probe_duration_seconds`.
//! * **Lecture seule.** Que des `SELECT` sur les vues de statistiques.
//! * **Cardinalité bornée** : vingt bases, vingt réplicas, vingt slots, cinq
//!   tables.
//!
//! Les noms sont ceux de `postgres_*`, sans le préfixe `probe_` : ce sont des
//! mesures du serveur, pas de la sonde.

use std::time::Duration;

use dumbmonit_proto::{MetricKind, Sample};
use sqlx::Row;
use sqlx::postgres::{PgConnection, PgRow};
use tracing::debug;

use crate::uptime::session::Deadline;

/// Bornes de cardinalité.
const MAX_DATABASES: usize = 20;
const MAX_REPLICAS: usize = 20;
const MAX_SLOTS: usize = 20;
const MAX_TABLES: usize = 5;

/// Les identifiants de transaction tiennent sur 32 bits : au bout de 2^31
/// transactions non « gelées », la base passe en lecture seule pour se protéger.
const XID_LIMIT: f64 = 2_147_483_648.0;

const SERVER: &str = "SELECT \
    EXTRACT(EPOCH FROM now() - pg_postmaster_start_time())::float8, \
    pg_is_in_recovery(), \
    current_setting('server_version'), \
    current_setting('max_connections')::float8, \
    current_setting('superuser_reserved_connections')::float8, \
    current_setting('archive_mode')";

// `backend_type IS NULL` : un compte sans `pg_monitor` ne voit pas le type des
// sessions des autres, mais elles comptent quand même contre `max_connections`.
const ACTIVITY: &str = "SELECT \
    count(*) FILTER (WHERE backend_type IS NULL OR backend_type = 'client backend')::float8, \
    count(*) FILTER (WHERE state LIKE 'idle in transaction%')::float8, \
    COALESCE(max(EXTRACT(EPOCH FROM now() - xact_start)) \
        FILTER (WHERE xact_start IS NOT NULL AND state <> 'idle' \
            AND backend_type = 'client backend'), 0)::float8, \
    COALESCE(max(EXTRACT(EPOCH FROM now() - state_change)) \
        FILTER (WHERE state LIKE 'idle in transaction%'), 0)::float8, \
    COALESCE(max(EXTRACT(EPOCH FROM now() - query_start)) \
        FILTER (WHERE state = 'active' AND backend_type = 'client backend'), 0)::float8 \
    FROM pg_stat_activity";

const LOCKS: &str = "SELECT count(*) FILTER (WHERE NOT granted)::float8 FROM pg_locks";

// `has_database_privilege` : `pg_database_size` échoue sur une base dont on n'a pas
// le droit de se connecter, ce qui ferait perdre toute la requête.
const DATABASES: &str = "SELECT d.datname::text, \
    CASE WHEN has_database_privilege(d.datname, 'CONNECT') \
        THEN pg_database_size(d.datname)::float8 END, \
    COALESCE(s.blks_hit, 0)::float8, COALESCE(s.blks_read, 0)::float8, \
    COALESCE(s.deadlocks, 0)::float8, COALESCE(s.xact_commit, 0)::float8, \
    COALESCE(s.xact_rollback, 0)::float8, COALESCE(s.temp_bytes, 0)::float8, \
    age(d.datfrozenxid)::float8 \
    FROM pg_database d LEFT JOIN pg_stat_database s ON s.datid = d.oid \
    WHERE d.datallowconn AND NOT d.datistemplate \
    ORDER BY d.datname LIMIT 20";

const REPLICAS: &str = "SELECT COALESCE(application_name, '')::text, \
    COALESCE(client_addr::text, ''), COALESCE(state, ''), \
    COALESCE(EXTRACT(EPOCH FROM replay_lag), 0)::float8, \
    CASE WHEN replay_lsn IS NULL THEN NULL \
        ELSE pg_wal_lsn_diff(pg_current_wal_lsn(), replay_lsn)::float8 END \
    FROM pg_stat_replication ORDER BY application_name, client_addr LIMIT 20";

const SLOTS: &str = "SELECT slot_name::text, active, \
    CASE WHEN restart_lsn IS NULL THEN NULL \
        ELSE pg_wal_lsn_diff(pg_current_wal_lsn(), restart_lsn)::float8 END \
    FROM pg_replication_slots ORDER BY slot_name LIMIT 20";

// Un primaire au repos n'écrit rien : le dernier rejeu est alors ancien sans que le
// réplica soit en retard. Récepteur et rejeu à la même position : aucun retard.
const STANDBY: &str = "SELECT \
    CASE WHEN pg_last_wal_receive_lsn() = pg_last_wal_replay_lsn() THEN 0::float8 \
        ELSE COALESCE(EXTRACT(EPOCH FROM now() - pg_last_xact_replay_timestamp()), 0)::float8 END, \
    (SELECT count(*) FROM pg_stat_wal_receiver)::float8";

// Tables de la base ouverte seulement : les statistiques de tables sont par base.
const TABLE_TOTALS: &str = "SELECT COALESCE(sum(n_dead_tup), 0)::float8, \
    COALESCE(sum(n_live_tup), 0)::float8 FROM pg_stat_user_tables";

const TABLES: &str = "SELECT schemaname::text, relname::text, \
    (100.0 * n_dead_tup / NULLIF(n_live_tup + n_dead_tup, 0))::float8 \
    FROM pg_stat_user_tables \
    WHERE n_live_tup + n_dead_tup >= 10000 AND n_dead_tup >= 1000 \
    ORDER BY 3 DESC LIMIT 5";

const ARCHIVER: &str = "SELECT failed_count::float8, archived_count::float8, \
    COALESCE(EXTRACT(EPOCH FROM now() - last_archived_time), -1)::float8, \
    (last_failed_time IS NOT NULL \
        AND (last_archived_time IS NULL OR last_failed_time > last_archived_time)) \
    FROM pg_stat_archiver";

const PREPARED: &str = "SELECT count(*)::float8, \
    COALESCE(max(EXTRACT(EPOCH FROM now() - prepared)), 0)::float8 FROM pg_prepared_xacts";

// ------------------------------------------------------------------ données

#[derive(Debug, Clone, PartialEq)]
pub struct Server {
    pub uptime_seconds: f64,
    pub in_recovery: bool,
    pub version: String,
    pub max_connections: f64,
    pub reserved_connections: f64,
    pub archive_mode: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Activity {
    pub connections: f64,
    pub idle_in_transaction: f64,
    pub longest_transaction: f64,
    pub longest_idle_in_transaction: f64,
    pub longest_query: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Database {
    pub name: String,
    pub size_bytes: Option<f64>,
    pub blocks_hit: f64,
    pub blocks_read: f64,
    pub deadlocks: f64,
    pub commits: f64,
    pub rollbacks: f64,
    pub temp_bytes: f64,
    pub xid_age: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Replica {
    pub application: String,
    pub address: String,
    pub state: String,
    pub replay_lag_seconds: f64,
    pub lag_bytes: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    pub name: String,
    pub active: bool,
    pub retained_bytes: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Archiver {
    pub failed: f64,
    pub archived: f64,
    pub last_archived_age: f64,
    pub failing: bool,
}

// ------------------------------------------------------------- traduction

fn gauge(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Gauge, ts_ms)
}

fn counter(name: &str, value: f64, ts_ms: i64) -> Sample {
    Sample::new(name, value, MetricKind::Counter, ts_ms)
}

fn flag(name: &str, value: bool, ts_ms: i64) -> Sample {
    gauge(name, if value { 1.0 } else { 0.0 }, ts_ms)
}

pub fn server_samples(server: &Server, ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![
        gauge("postgres_uptime_seconds", server.uptime_seconds, ts_ms),
        flag("postgres_in_recovery", server.in_recovery, ts_ms),
        gauge("postgres_max_connections", server.max_connections, ts_ms),
    ];
    let version = server.version.split_whitespace().next().unwrap_or("");
    if !version.is_empty() {
        out.push(gauge("postgres_version_info", 1.0, ts_ms).with_label("version", version));
    }
    out
}

/// Part des connexions utilisées parmi celles que les comptes ordinaires peuvent
/// ouvrir : les connexions réservées aux superutilisateurs ne comptent pas, c'est
/// leur épuisement que l'on veut voir venir.
pub fn activity_samples(activity: &Activity, server: &Server, ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![
        gauge("postgres_connections", activity.connections, ts_ms),
        gauge("postgres_connections_idle_in_transaction", activity.idle_in_transaction, ts_ms),
        gauge("postgres_longest_transaction_seconds", activity.longest_transaction, ts_ms),
        gauge(
            "postgres_longest_idle_in_transaction_seconds",
            activity.longest_idle_in_transaction,
            ts_ms,
        ),
        gauge("postgres_longest_query_seconds", activity.longest_query, ts_ms),
    ];
    let usable = server.max_connections - server.reserved_connections;
    if usable > 0.0 {
        out.push(gauge(
            "postgres_connections_used_percent",
            (activity.connections / usable * 100.0).min(100.0),
            ts_ms,
        ));
    }
    out
}

pub fn database_samples(databases: &[Database], ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let (mut hit, mut read) = (0.0, 0.0);
    for db in databases.iter().take(MAX_DATABASES) {
        let label = |sample: Sample| sample.with_label("database", db.name.as_str());
        if let Some(size) = db.size_bytes {
            out.push(label(gauge("postgres_database_size_bytes", size, ts_ms)));
        }
        out.push(label(gauge(
            "postgres_database_wraparound_percent",
            (db.xid_age / XID_LIMIT * 100.0).clamp(0.0, 100.0),
            ts_ms,
        )));
        out.push(label(counter("postgres_database_blocks_hit_total", db.blocks_hit, ts_ms)));
        out.push(label(counter("postgres_database_blocks_read_total", db.blocks_read, ts_ms)));
        out.push(label(counter("postgres_database_deadlocks_total", db.deadlocks, ts_ms)));
        out.push(label(counter("postgres_database_commits_total", db.commits, ts_ms)));
        out.push(label(counter("postgres_database_rollbacks_total", db.rollbacks, ts_ms)));
        out.push(label(counter("postgres_database_temp_bytes_total", db.temp_bytes, ts_ms)));
        hit += db.blocks_hit;
        read += db.blocks_read;
    }
    // Cumulé depuis la dernière remise à zéro des statistiques : bon pour un
    // chiffre à l'écran, pas pour une alerte (la règle regarde la dernière heure).
    if hit + read > 0.0 {
        out.push(gauge("postgres_cache_hit_percent", hit / (hit + read) * 100.0, ts_ms));
    }
    out
}

/// Le nom d'un réplica : `application_name` seul est le même (`walreceiver`)
/// pour tous les réplicas par défaut, l'adresse les distingue.
fn replica_name(replica: &Replica) -> String {
    match (replica.application.is_empty(), replica.address.is_empty()) {
        (false, false) => format!("{}@{}", replica.application, replica.address),
        (false, true) => replica.application.clone(),
        (true, false) => replica.address.clone(),
        (true, true) => "unknown".to_string(),
    }
}

pub fn replication_samples(replicas: &[Replica], slots: &[Slot], ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![gauge("postgres_replicas", replicas.len() as f64, ts_ms)];
    for replica in replicas.iter().take(MAX_REPLICAS) {
        let name = replica_name(replica);
        let label = |sample: Sample| sample.with_label("replica", name.as_str());
        out.push(label(flag("postgres_replica_streaming", replica.state == "streaming", ts_ms)));
        out.push(label(gauge("postgres_replica_lag_seconds", replica.replay_lag_seconds, ts_ms)));
        if let Some(bytes) = replica.lag_bytes {
            out.push(label(gauge("postgres_replica_lag_bytes", bytes, ts_ms)));
        }
    }
    for slot in slots.iter().take(MAX_SLOTS) {
        let label = |sample: Sample| sample.with_label("slot", slot.name.as_str());
        out.push(label(flag("postgres_replication_slot_active", slot.active, ts_ms)));
        if let Some(bytes) = slot.retained_bytes {
            out.push(label(gauge("postgres_replication_slot_retained_bytes", bytes, ts_ms)));
        }
    }
    out
}

pub fn standby_samples(lag_seconds: f64, receivers: f64, ts_ms: i64) -> Vec<Sample> {
    vec![
        gauge("postgres_standby_replay_lag_seconds", lag_seconds.max(0.0), ts_ms),
        flag("postgres_standby_receiving", receivers > 0.0, ts_ms),
    ]
}

pub fn table_samples(
    dead: f64,
    live: f64,
    worst: &[(String, String, f64)],
    ts_ms: i64,
) -> Vec<Sample> {
    let mut out = Vec::new();
    if dead + live > 0.0 {
        out.push(gauge("postgres_dead_tuples_percent", dead / (dead + live) * 100.0, ts_ms));
    }
    for (schema, table, percent) in worst.iter().take(MAX_TABLES) {
        out.push(
            gauge("postgres_table_dead_tuple_percent", *percent, ts_ms)
                .with_label("table", format!("{schema}.{table}")),
        );
    }
    out
}

pub fn archiver_samples(archiver: &Archiver, ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![
        counter("postgres_archiver_failed_total", archiver.failed, ts_ms),
        counter("postgres_archiver_archived_total", archiver.archived, ts_ms),
        flag("postgres_archiver_failing", archiver.failing, ts_ms),
    ];
    if archiver.last_archived_age >= 0.0 {
        out.push(gauge(
            "postgres_archiver_last_archived_age_seconds",
            archiver.last_archived_age,
            ts_ms,
        ));
    }
    out
}

pub fn prepared_samples(count: f64, oldest_seconds: f64, ts_ms: i64) -> Vec<Sample> {
    vec![
        gauge("postgres_prepared_transactions", count, ts_ms),
        gauge("postgres_prepared_transactions_oldest_seconds", oldest_seconds, ts_ms),
    ]
}

// ------------------------------------------------------------- lecture

fn num(row: &PgRow, index: usize) -> f64 {
    row.try_get::<f64, _>(index).unwrap_or(0.0)
}

fn opt(row: &PgRow, index: usize) -> Option<f64> {
    row.try_get::<Option<f64>, _>(index).ok().flatten()
}

fn text(row: &PgRow, index: usize) -> String {
    row.try_get::<String, _>(index).unwrap_or_default()
}

/// Une lecture, bornée par l'échéance. `None` : refusée, absente ou trop lente —
/// l'appelant passe à la suite (après une lecture trop lente l'échéance est
/// dépassée, les suivantes sont sautées).
async fn rows(
    conn: &mut PgConnection,
    sql: &'static str,
    deadline: Deadline,
) -> Option<Vec<PgRow>> {
    if deadline.expired() {
        return None;
    }
    match deadline.wait(sqlx::query(sql).fetch_all(&mut *conn)).await {
        Ok(Ok(rows)) => Some(rows),
        Ok(Err(error)) => {
            debug!(%error, "lecture des statistiques PostgreSQL refusée ou indisponible");
            None
        }
        Err(_) => None,
    }
}

/// La première ligne d'une lecture.
async fn first(conn: &mut PgConnection, sql: &'static str, deadline: Deadline) -> Option<PgRow> {
    rows(conn, sql, deadline).await?.into_iter().next()
}

/// Lit les statistiques du serveur sur `conn`, dans la limite de `budget`.
///
/// Toujours un `Vec`, jamais une erreur : voir la doc du module.
pub async fn collect(conn: &mut PgConnection, ts_ms: i64, budget: Duration) -> Vec<Sample> {
    let deadline = Deadline::starting_now(budget);
    let mut out = Vec::new();

    let Some(row) = first(conn, SERVER, deadline).await else { return out };
    let server = Server {
        uptime_seconds: num(&row, 0),
        in_recovery: row.try_get::<bool, _>(1).unwrap_or(false),
        version: text(&row, 2),
        max_connections: num(&row, 3),
        reserved_connections: num(&row, 4),
        archive_mode: text(&row, 5),
    };
    out.extend(server_samples(&server, ts_ms));

    if let Some(row) = first(conn, ACTIVITY, deadline).await {
        let activity = Activity {
            connections: num(&row, 0),
            idle_in_transaction: num(&row, 1),
            longest_transaction: num(&row, 2),
            longest_idle_in_transaction: num(&row, 3),
            longest_query: num(&row, 4),
        };
        out.extend(activity_samples(&activity, &server, ts_ms));
    }
    if let Some(row) = first(conn, LOCKS, deadline).await {
        out.push(gauge("postgres_locks_waiting", num(&row, 0), ts_ms));
    }
    if let Some(found) = rows(conn, DATABASES, deadline).await {
        let databases: Vec<Database> = found
            .iter()
            .map(|row| Database {
                name: text(row, 0),
                size_bytes: opt(row, 1),
                blocks_hit: num(row, 2),
                blocks_read: num(row, 3),
                deadlocks: num(row, 4),
                commits: num(row, 5),
                rollbacks: num(row, 6),
                temp_bytes: num(row, 7),
                xid_age: num(row, 8),
            })
            .collect();
        out.extend(database_samples(&databases, ts_ms));
    }

    if server.in_recovery {
        if let Some(row) = first(conn, STANDBY, deadline).await {
            out.extend(standby_samples(num(&row, 0), num(&row, 1), ts_ms));
        }
    } else {
        // Les fonctions de position WAL du primaire échouent sur un réplica.
        let replicas: Vec<Replica> = rows(conn, REPLICAS, deadline)
            .await
            .unwrap_or_default()
            .iter()
            .map(|row| Replica {
                application: text(row, 0),
                address: text(row, 1),
                state: text(row, 2),
                replay_lag_seconds: num(row, 3),
                lag_bytes: opt(row, 4),
            })
            .collect();
        let slots: Vec<Slot> = rows(conn, SLOTS, deadline)
            .await
            .unwrap_or_default()
            .iter()
            .map(|row| Slot {
                name: text(row, 0),
                active: row.try_get::<bool, _>(1).unwrap_or(false),
                retained_bytes: opt(row, 2),
            })
            .collect();
        out.extend(replication_samples(&replicas, &slots, ts_ms));
    }

    if let Some(row) = first(conn, TABLE_TOTALS, deadline).await {
        let worst: Vec<(String, String, f64)> = rows(conn, TABLES, deadline)
            .await
            .unwrap_or_default()
            .iter()
            .map(|row| (text(row, 0), text(row, 1), num(row, 2)))
            .collect();
        out.extend(table_samples(num(&row, 0), num(&row, 1), &worst, ts_ms));
    }

    if !matches!(server.archive_mode.as_str(), "off" | "")
        && let Some(row) = first(conn, ARCHIVER, deadline).await
    {
        let archiver = Archiver {
            failed: num(&row, 0),
            archived: num(&row, 1),
            last_archived_age: row.try_get::<f64, _>(2).unwrap_or(-1.0),
            failing: row.try_get::<bool, _>(3).unwrap_or(false),
        };
        out.extend(archiver_samples(&archiver, ts_ms));
    }
    if let Some(row) = first(conn, PREPARED, deadline).await {
        out.extend(prepared_samples(num(&row, 0), num(&row, 1), ts_ms));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn server() -> Server {
        Server {
            uptime_seconds: 864_000.0,
            in_recovery: false,
            version: "16.4 (Debian 16.4-1.pgdg120+2)".into(),
            max_connections: 100.0,
            reserved_connections: 3.0,
            archive_mode: "on".into(),
        }
    }

    #[test]
    fn la_version_ne_garde_que_le_numero() {
        let samples = server_samples(&server(), 0);
        let info = samples.iter().find(|s| s.metric == "postgres_version_info").unwrap();
        assert_eq!(info.labels["version"], "16.4");
        assert_eq!(value(&samples, "postgres_in_recovery"), Some(0.0));
    }

    #[test]
    fn les_connexions_reservees_ne_comptent_pas_dans_le_pourcentage() {
        let activity = Activity {
            connections: 97.0,
            idle_in_transaction: 4.0,
            longest_transaction: 12.5,
            longest_idle_in_transaction: 300.0,
            longest_query: 2.0,
        };
        let samples = activity_samples(&activity, &server(), 0);
        assert_eq!(value(&samples, "postgres_connections_used_percent"), Some(100.0));
        assert_eq!(value(&samples, "postgres_longest_idle_in_transaction_seconds"), Some(300.0));
        let calm = Activity { connections: 20.0, ..activity };
        let samples = activity_samples(&calm, &server(), 0);
        let used = value(&samples, "postgres_connections_used_percent").unwrap();
        assert!((used - 20.618).abs() < 0.01, "{used}");
    }

    #[test]
    fn un_max_connections_aberrant_ne_divise_pas_par_zero() {
        let mut odd = server();
        odd.reserved_connections = odd.max_connections;
        let activity = Activity {
            connections: 1.0,
            idle_in_transaction: 0.0,
            longest_transaction: 0.0,
            longest_idle_in_transaction: 0.0,
            longest_query: 0.0,
        };
        let samples = activity_samples(&activity, &odd, 0);
        assert!(value(&samples, "postgres_connections_used_percent").is_none());
    }

    #[test]
    fn les_bases_donnent_taille_age_des_xid_et_cache() {
        let databases = vec![
            Database {
                name: "nextcloud".into(),
                size_bytes: Some(2_147_483_648.0),
                blocks_hit: 990.0,
                blocks_read: 10.0,
                deadlocks: 0.0,
                commits: 5000.0,
                rollbacks: 12.0,
                temp_bytes: 0.0,
                xid_age: 1_073_741_824.0,
            },
            // Base dont on ne peut pas lire la taille (pas le droit CONNECT).
            Database {
                name: "private".into(),
                size_bytes: None,
                blocks_hit: 0.0,
                blocks_read: 0.0,
                deadlocks: 1.0,
                commits: 0.0,
                rollbacks: 0.0,
                temp_bytes: 0.0,
                xid_age: 200_000_000.0,
            },
        ];
        let samples = database_samples(&databases, 0);
        let wrap: Vec<_> = samples
            .iter()
            .filter(|s| s.metric == "postgres_database_wraparound_percent")
            .collect();
        assert_eq!(wrap.len(), 2);
        assert_eq!(wrap[0].labels["database"], "nextcloud");
        assert!((wrap[0].value - 50.0).abs() < 1e-9);
        let sizes = samples.iter().filter(|s| s.metric == "postgres_database_size_bytes").count();
        assert_eq!(sizes, 1);
        let cache = value(&samples, "postgres_cache_hit_percent").unwrap();
        assert!((cache - 99.0).abs() < 1e-9, "{cache}");
        let deadlocks = samples
            .iter()
            .find(|s| s.metric == "postgres_database_deadlocks_total" && s.value == 1.0)
            .unwrap();
        assert_eq!(deadlocks.kind, MetricKind::Counter);
    }

    #[test]
    fn deux_replicas_au_nom_par_defaut_restent_distincts() {
        let replica = |address: &str, lag: f64| Replica {
            application: "walreceiver".into(),
            address: address.into(),
            state: "streaming".into(),
            replay_lag_seconds: lag,
            lag_bytes: Some(16_384.0),
        };
        let samples = replication_samples(
            &[replica("10.0.0.2", 0.4), replica("10.0.0.3", 95.0)],
            &[
                Slot { name: "standby_a".into(), active: true, retained_bytes: Some(0.0) },
                Slot { name: "old_consumer".into(), active: false, retained_bytes: None },
            ],
            0,
        );
        assert_eq!(value(&samples, "postgres_replicas"), Some(2.0));
        let lags: Vec<_> =
            samples.iter().filter(|s| s.metric == "postgres_replica_lag_seconds").collect();
        assert_eq!(lags[0].labels["replica"], "walreceiver@10.0.0.2");
        assert_eq!(lags[1].labels["replica"], "walreceiver@10.0.0.3");
        let slot = samples
            .iter()
            .find(|s| s.metric == "postgres_replication_slot_active" && s.value == 0.0)
            .unwrap();
        assert_eq!(slot.labels["slot"], "old_consumer");
        // Slot sans `restart_lsn` : pas de volume retenu à annoncer.
        let retained =
            samples.iter().filter(|s| s.metric == "postgres_replication_slot_retained_bytes");
        assert_eq!(retained.count(), 1);
    }

    #[test]
    fn un_primaire_sans_replica_dit_zero_replica() {
        let samples = replication_samples(&[], &[], 0);
        assert_eq!(value(&samples, "postgres_replicas"), Some(0.0));
        assert_eq!(samples.len(), 1);
    }

    #[test]
    fn un_replica_en_catchup_nest_pas_streaming() {
        let replica = Replica {
            application: "".into(),
            address: "10.0.0.9".into(),
            state: "catchup".into(),
            replay_lag_seconds: 0.0,
            lag_bytes: None,
        };
        let samples = replication_samples(&[replica], &[], 0);
        assert_eq!(value(&samples, "postgres_replica_streaming"), Some(0.0));
        assert!(value(&samples, "postgres_replica_lag_bytes").is_none());
    }

    #[test]
    fn le_gonflement_cite_les_pires_tables() {
        let worst = vec![("public".to_string(), "oc_filecache".to_string(), 38.5)];
        let samples = table_samples(40_000.0, 160_000.0, &worst, 0);
        let dead = value(&samples, "postgres_dead_tuples_percent").unwrap();
        assert!((dead - 20.0).abs() < 1e-9, "{dead}");
        let table = samples.iter().find(|s| s.metric == "postgres_table_dead_tuple_percent");
        assert_eq!(table.unwrap().labels["table"], "public.oc_filecache");
        assert!(table_samples(0.0, 0.0, &[], 0).is_empty(), "base vide : rien à dire");
    }

    #[test]
    fn larchivage_en_echec_est_signale_et_lage_est_optionnel() {
        let archiver = Archiver {
            failed: 3.0,
            archived: 1200.0,
            last_archived_age: -1.0,
            failing: true,
        };
        let samples = archiver_samples(&archiver, 0);
        assert_eq!(value(&samples, "postgres_archiver_failing"), Some(1.0));
        assert!(value(&samples, "postgres_archiver_last_archived_age_seconds").is_none());
    }

    #[test]
    fn un_replica_au_repos_na_pas_de_retard_negatif() {
        let samples = standby_samples(-0.2, 1.0, 0);
        assert_eq!(value(&samples, "postgres_standby_replay_lag_seconds"), Some(0.0));
        assert_eq!(value(&samples, "postgres_standby_receiving"), Some(1.0));
    }
}
