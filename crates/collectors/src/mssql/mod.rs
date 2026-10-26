//! Microsoft SQL Server, par le protocole TDS (crate `tiberius`, Rust pur,
//! MIT OR Apache-2.0, TLS par rustls : ni OpenSSL ni bibliothèque C de plus).
//!
//! Le compte recommandé est une connexion SQL (authentification SQL Server) à
//! qui l'on accorde `VIEW SERVER STATE`, `VIEW ANY DEFINITION` et, dans `msdb`,
//! le rôle `SQLAgentReaderRole` : il lit l'état du moteur et des travaux, aucune
//! donnée d'aucune base. L'authentification Windows (NTLM, Kerberos) n'est pas
//! prise en charge.
//!
//! Ce que ce module existe pour voir :
//!
//! * **une base qui n'est plus en ligne** (`SUSPECT`, `RECOVERY_PENDING`,
//!   `OFFLINE`, `RESTORING` qui n'en finit pas) ;
//! * **des requêtes bloquées** : une transaction oubliée ouverte fige les autres
//!   sessions, sans erreur côté moteur ;
//! * **un volume presque plein**, avant que les fichiers ne puissent grossir ;
//! * **un travail de l'Agent SQL Server dont la dernière exécution a échoué**
//!   (sauvegardes, maintenance) ;
//! * **un groupe de disponibilité AlwaysOn dont une réplique n'est plus saine**.
//!
//! Les requêtes sont toutes des `SELECT` sur des vues de gestion. Celles qui
//! dépendent d'une fonctionnalité optionnelle (Agent absent d'une édition
//! Express, AlwaysOn non activé) sont facultatives : leur échec laisse leurs
//! métriques de côté sans faire échouer la collecte.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `1433` | Port TCP (pas de résolution d'instance nommée). |
//! | `database` | `master` | Base de connexion. |
//! | `tls` | `true` | Chiffre la session ; sinon seul le login l'est. |
//! | `insecure_tls` | `false` | Accepte le certificat auto-signé d'une installation par défaut. |
//! | `request_timeout_seconds` | `10` | Délai de toute l'interrogation. |

use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, MetricKind, ProbeError, Sample, Target};
use tiberius::{AuthMethod, Client, Config, EncryptionLevel, Row};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use crate::uptime::tags;

pub const DEFAULT_PORT: u16 = 1433;
pub const DEFAULT_DATABASE: &str = "master";
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Code d'erreur du serveur pour « Login failed for user ».
const LOGIN_FAILED: u32 = 18456;

const SERVER: &str = "SELECT CAST(SERVERPROPERTY('ProductVersion') AS nvarchar(64)), \
    CAST(SERVERPROPERTY('Edition') AS nvarchar(128)), \
    CAST(ISNULL(SERVERPROPERTY('IsHadrEnabled'), 0) AS int), \
    DATEDIFF(SECOND, sqlserver_start_time, SYSDATETIME()) FROM sys.dm_os_sys_info";
const SESSIONS: &str = "SELECT COUNT(*), \
    (SELECT CAST(value_in_use AS int) FROM sys.configurations WHERE name = 'user connections') \
    FROM sys.dm_exec_sessions WHERE is_user_process = 1";
const BLOCKING: &str = "SELECT COUNT(*), ISNULL(MAX(wait_time), 0) FROM sys.dm_exec_requests \
    WHERE blocking_session_id <> 0";
const DATABASES: &str = "SELECT name, state_desc, CAST(is_read_only AS int) FROM sys.databases";
const SIZES: &str = "SELECT DB_NAME(database_id), CAST(SUM(CAST(size AS bigint)) * 8192 AS bigint) \
    FROM sys.master_files WHERE type = 0 GROUP BY database_id";
const VOLUMES: &str = "SELECT DISTINCT vs.volume_mount_point, \
    CAST(vs.total_bytes AS bigint), CAST(vs.available_bytes AS bigint) \
    FROM sys.master_files f CROSS APPLY sys.dm_os_volume_stats(f.database_id, f.file_id) vs";
const AGENT_JOBS: &str = "SELECT j.name, CAST(h.run_status AS int) FROM msdb.dbo.sysjobs j \
    CROSS APPLY (SELECT TOP 1 run_status FROM msdb.dbo.sysjobhistory \
    WHERE job_id = j.job_id AND step_id = 0 ORDER BY instance_id DESC) h WHERE j.enabled = 1";
const ALWAYS_ON: &str = "SELECT ar.replica_server_name, DB_NAME(drs.database_id), \
    CAST(drs.synchronization_health AS int), CAST(drs.is_suspended AS int) \
    FROM sys.dm_hadr_database_replica_states drs \
    JOIN sys.availability_replicas ar ON drs.replica_id = ar.replica_id";

#[derive(Default)]
pub struct MssqlCollector;

impl MssqlCollector {
    pub fn new() -> Self {
        Self
    }
}

/// Réglages de connexion, lus sur la cible.
struct Settings {
    host: String,
    port: u16,
    database: String,
    username: String,
    password: String,
    tls: bool,
    insecure_tls: bool,
    timeout: Duration,
}

fn settings(target: &Target) -> Result<Settings, ProbeError> {
    let (username, password) = match &target.credential {
        Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
            (username.trim().to_string(), password.clone())
        }
        other => {
            return Err(ProbeError::Config(format!(
                "SQL Server expects a SQL login (user name and password), configured: {other}"
            )));
        }
    };
    let port = tags::parse_u32(target, "port", u32::from(DEFAULT_PORT), 1..=65_535)? as u16;
    let address = target.address.trim();
    let address = address.split_once("://").map_or(address, |(_, rest)| rest);
    let (host, port) = tags::split_host_port(address.split(['/', '?']).next().unwrap_or(address), port)?;
    Ok(Settings {
        host,
        port,
        database: tags::tag(target, "database").unwrap_or(DEFAULT_DATABASE).to_string(),
        username,
        password,
        tls: tags::parse_bool(target, "tls", true)?,
        insecure_tls: tags::parse_bool(target, "insecure_tls", false)?,
        timeout: Duration::from_secs(u64::from(tags::parse_u32(
            target,
            "request_timeout_seconds",
            DEFAULT_TIMEOUT.as_secs() as u32,
            1..=120,
        )?)),
    })
}

/// Classe une erreur du pilote selon la règle commune : le réseau est une
/// panne, un login refusé une affaire de compte, le reste une réponse anormale.
fn classify(error: tiberius::error::Error, settings: &Settings) -> ProbeError {
    use tiberius::error::Error;
    let endpoint = format!("{}:{}", settings.host, settings.port);
    match error {
        Error::Io { message, .. } => {
            ProbeError::Unreachable(format!("SQL Server at {endpoint}: {message}"))
        }
        Error::Server(ref token) if token.code() == LOGIN_FAILED => ProbeError::Auth(format!(
            "SQL Server refused the login: {}. Check the user name and password, and that SQL \
             Server authentication is enabled.",
            token.message()
        )),
        Error::Server(token) => ProbeError::Config(format!(
            "SQL Server refused the request ({}): {}",
            token.code(),
            token.message()
        )),
        Error::Tls(message) => ProbeError::Config(format!(
            "TLS with SQL Server at {endpoint} failed: {message}. Enable \"Accept an unverifiable \
             certificate\" for a self-signed certificate, or untick TLS."
        )),
        other => ProbeError::Protocol(format!("SQL Server at {endpoint}: {other}")),
    }
}

type Session = Client<Compat<TcpStream>>;

async fn connect(settings: &Settings) -> Result<Session, ProbeError> {
    let mut config = Config::new();
    config.host(&settings.host);
    config.port(settings.port);
    config.database(&settings.database);
    config.application_name("DumbMonit");
    config.authentication(AuthMethod::sql_server(&settings.username, &settings.password));
    config.encryption(if settings.tls { EncryptionLevel::Required } else { EncryptionLevel::Off });
    if settings.insecure_tls {
        config.trust_cert();
    }
    let tcp = TcpStream::connect(config.get_addr()).await.map_err(|error| {
        ProbeError::Unreachable(format!(
            "SQL Server at {}:{}: {error}",
            settings.host, settings.port
        ))
    })?;
    let _ = tcp.set_nodelay(true);
    Client::connect(config, tcp.compat_write()).await.map_err(|error| classify(error, settings))
}

async fn rows(
    session: &mut Session,
    settings: &Settings,
    statement: &str,
) -> Result<Vec<Row>, ProbeError> {
    session
        .simple_query(statement)
        .await
        .map_err(|error| classify(error, settings))?
        .into_first_result()
        .await
        .map_err(|error| classify(error, settings))
}

/// Une requête facultative : une vue absente ou un droit manquant (Agent d'une
/// édition Express, AlwaysOn non activé) laissent la liste vide. Une erreur du
/// serveur est avalée ; une erreur de réseau remonte, la connexion est perdue.
async fn optional(session: &mut Session, settings: &Settings, statement: &str) -> Vec<Row> {
    rows(session, settings, statement).await.unwrap_or_default()
}

/// Une ligne, lue en valeurs simples : le pilote typé reste dans `collect`, les
/// calculs de `samples` se testent sans serveur.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Replies {
    pub version: String,
    pub edition: String,
    pub hadr_enabled: bool,
    pub uptime_seconds: Option<f64>,
    pub sessions: Option<f64>,
    pub max_connections: Option<f64>,
    pub blocked_requests: Option<f64>,
    pub longest_block_ms: Option<f64>,
    /// Nom, état, lecture seule.
    pub databases: Vec<(String, String, bool)>,
    /// Nom, taille en octets.
    pub sizes: Vec<(String, f64)>,
    /// Point de montage, octets au total, octets libres.
    pub volumes: Vec<(String, f64, f64)>,
    /// Travail, `run_status` de sa dernière exécution.
    pub jobs: Vec<(String, i32)>,
    /// Réplique, base, santé de synchronisation (0 à 2), suspendue.
    pub replicas: Vec<(String, String, i32, bool)>,
}

fn text(row: &Row, index: usize) -> String {
    row.get::<&str, _>(index).unwrap_or("").to_string()
}

fn int(row: &Row, index: usize) -> Option<i32> {
    row.get::<i32, _>(index)
}

fn big(row: &Row, index: usize) -> Option<i64> {
    row.get::<i64, _>(index)
}

async fn collect(settings: &Settings) -> Result<Replies, ProbeError> {
    let mut session = connect(settings).await?;
    let mut replies = Replies::default();

    // Le serveur est la requête qui prouve que tout fonctionne : son échec compte.
    if let Some(row) = rows(&mut session, settings, SERVER).await?.first() {
        replies.version = text(row, 0);
        replies.edition = text(row, 1);
        replies.hadr_enabled = int(row, 2).unwrap_or(0) == 1;
        replies.uptime_seconds = int(row, 3).map(f64::from);
    }
    if let Some(row) = rows(&mut session, settings, SESSIONS).await?.first() {
        replies.sessions = int(row, 0).map(f64::from);
        // 0 = illimité (32 767 en pratique) : pas de plafond à comparer.
        replies.max_connections = int(row, 1).filter(|max| *max > 0).map(f64::from);
    }
    if let Some(row) = rows(&mut session, settings, BLOCKING).await?.first() {
        replies.blocked_requests = int(row, 0).map(f64::from);
        replies.longest_block_ms = int(row, 1).map(f64::from);
    }
    for row in rows(&mut session, settings, DATABASES).await? {
        replies.databases.push((text(&row, 0), text(&row, 1), int(&row, 2).unwrap_or(0) == 1));
    }
    for row in optional(&mut session, settings, SIZES).await {
        if let (Some(name), Some(bytes)) = (row.get::<&str, _>(0), big(&row, 1)) {
            replies.sizes.push((name.to_string(), bytes as f64));
        }
    }
    for row in optional(&mut session, settings, VOLUMES).await {
        if let (Some(total), Some(free)) = (big(&row, 1), big(&row, 2)) {
            replies.volumes.push((text(&row, 0), total as f64, free as f64));
        }
    }
    for row in optional(&mut session, settings, AGENT_JOBS).await {
        if let Some(status) = int(&row, 1) {
            replies.jobs.push((text(&row, 0), status));
        }
    }
    if replies.hadr_enabled {
        for row in optional(&mut session, settings, ALWAYS_ON).await {
            if let Some(health) = int(&row, 2) {
                replies.replicas.push((
                    text(&row, 0),
                    text(&row, 1),
                    health,
                    int(&row, 3).unwrap_or(0) == 1,
                ));
            }
        }
    }
    Ok(replies)
}

pub async fn read(target: &Target) -> Result<Replies, ProbeError> {
    let settings = settings(target)?;
    match tokio::time::timeout(settings.timeout, collect(&settings)).await {
        Ok(result) => result,
        Err(_) => Err(ProbeError::Timeout(settings.timeout)),
    }
}

#[async_trait]
impl Collector for MssqlCollector {
    fn kind(&self) -> &'static str {
        "mssql"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let replies = read(target).await?;
        Ok(samples(&replies, chrono::Utc::now().timestamp_millis()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let settings = settings(target)?;
        let attempt = async {
            let mut session = connect(&settings).await?;
            rows(&mut session, &settings, SERVER).await
        };
        match tokio::time::timeout(settings.timeout, attempt).await {
            Ok(Ok(_)) => Ok(Some("mssql".to_string())),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(ProbeError::Timeout(settings.timeout)),
        }
    }
}

/// Une base est en service sauf dans les états qui la rendent inutilisable.
/// `RESTORING` n'en fait pas partie : c'est l'état normal du secondaire d'un
/// log shipping, et une alerte permanente n'apprendrait rien. `RECOVERING` est
/// transitoire (reprise après un arrêt) et ne compte pas non plus.
fn database_online(state: &str) -> bool {
    !matches!(state, "SUSPECT" | "RECOVERY_PENDING" | "OFFLINE" | "EMERGENCY")
}

pub fn samples(replies: &Replies, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);

    out.push(
        gauge("mssql_version_info", 1.0)
            .with_label("version", replies.version.clone())
            .with_label("edition", replies.edition.clone()),
    );
    let mut push = |name: &str, value: Option<f64>| {
        if let Some(value) = value {
            out.push(gauge(name, value));
        }
    };
    push("mssql_uptime_seconds", replies.uptime_seconds);
    push("mssql_sessions", replies.sessions);
    push("mssql_blocked_requests", replies.blocked_requests);
    push("mssql_longest_block_seconds", replies.longest_block_ms.map(|ms| ms / 1000.0));
    if let (Some(sessions), Some(max)) = (replies.sessions, replies.max_connections) {
        push("mssql_sessions_used_percent", Some(sessions / max * 100.0));
    }

    let offline = replies.databases.iter().filter(|(_, state, _)| !database_online(state)).count();
    out.push(gauge("mssql_databases", replies.databases.len() as f64));
    out.push(gauge("mssql_databases_offline", offline as f64));
    for (name, state, read_only) in &replies.databases {
        out.push(
            gauge("mssql_database_online", if database_online(state) { 1.0 } else { 0.0 })
                .with_label("database", name.clone())
                .with_label("state", state.clone()),
        );
        out.push(
            gauge("mssql_database_read_only", if *read_only { 1.0 } else { 0.0 })
                .with_label("database", name.clone()),
        );
    }
    for (name, bytes) in &replies.sizes {
        out.push(gauge("mssql_database_size_bytes", *bytes).with_label("database", name.clone()));
    }
    for (mount, total, free) in &replies.volumes {
        if *total <= 0.0 {
            continue;
        }
        let with = |sample: Sample| sample.with_label("volume", mount.clone());
        out.push(with(gauge("mssql_volume_free_bytes", *free)));
        out.push(with(gauge(
            "mssql_volume_used_percent",
            ((total - free) / total * 100.0).clamp(0.0, 100.0),
        )));
    }

    // Travaux de l'Agent : 0 échec, 1 réussite, 2 nouvelle tentative, 3
    // annulation, 4 en cours. Seul l'échec compte comme tel.
    let mut failed = 0.0;
    for (job, status) in &replies.jobs {
        let ok = *status != 0;
        if !ok {
            failed += 1.0;
        }
        out.push(
            gauge("mssql_agent_job_last_ok", if ok { 1.0 } else { 0.0 })
                .with_label("job", job.clone()),
        );
    }
    if !replies.jobs.is_empty() {
        out.push(gauge("mssql_agent_jobs_failed", failed));
    }

    // AlwaysOn : santé de synchronisation 0 (mauvaise), 1 (partielle), 2 (saine).
    let mut unhealthy = 0.0;
    for (replica, database, health, suspended) in &replies.replicas {
        if *health < 2 {
            unhealthy += 1.0;
        }
        let with = |sample: Sample| {
            sample.with_label("replica", replica.clone()).with_label("database", database.clone())
        };
        out.push(with(gauge("mssql_alwayson_sync_health", f64::from(*health))));
        out.push(with(gauge("mssql_alwayson_suspended", if *suspended { 1.0 } else { 0.0 })));
    }
    if !replies.replicas.is_empty() {
        out.push(gauge("mssql_alwayson_unhealthy", unhealthy));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uptime::tags::test_support::cible;

    fn find<'a>(samples: &'a [Sample], name: &str, labels: &[(&str, &str)]) -> Option<&'a Sample> {
        samples.iter().find(|s| {
            s.metric == name
                && labels.iter().all(|(k, v)| s.labels.get(*k).map(String::as_str) == Some(v))
        })
    }

    fn login() -> Credential {
        Credential::UsernamePassword { username: "dumbmonit".into(), password: "secret".into() }
    }

    fn healthy() -> Replies {
        Replies {
            version: "16.0.4135.4".into(),
            edition: "Standard Edition (64-bit)".into(),
            hadr_enabled: true,
            uptime_seconds: Some(86_400.0),
            sessions: Some(150.0),
            max_connections: Some(300.0),
            blocked_requests: Some(0.0),
            longest_block_ms: Some(0.0),
            databases: vec![
                ("master".into(), "ONLINE".into(), false),
                ("shop".into(), "ONLINE".into(), false),
            ],
            sizes: vec![("shop".into(), 1_073_741_824.0)],
            volumes: vec![("D:\\".into(), 1000.0, 250.0)],
            jobs: vec![("Backup".into(), 1), ("Reindex".into(), 4)],
            replicas: vec![("SQL1".into(), "shop".into(), 2, false)],
        }
    }

    #[test]
    fn serveur_sain() {
        let samples = samples(&healthy(), 0);
        assert_eq!(find(&samples, "mssql_databases_offline", &[]).unwrap().value, 0.0);
        assert_eq!(find(&samples, "mssql_sessions_used_percent", &[]).unwrap().value, 50.0);
        assert_eq!(find(&samples, "mssql_volume_used_percent", &[]).unwrap().value, 75.0);
        assert_eq!(find(&samples, "mssql_agent_jobs_failed", &[]).unwrap().value, 0.0);
        assert_eq!(find(&samples, "mssql_alwayson_unhealthy", &[]).unwrap().value, 0.0);
        let version = find(&samples, "mssql_version_info", &[]).unwrap();
        assert_eq!(version.labels["version"], "16.0.4135.4");
    }

    #[test]
    fn base_suspecte_et_travail_en_echec() {
        let mut replies = healthy();
        replies.databases.push(("old".into(), "SUSPECT".into(), false));
        replies.jobs.push(("Nightly".into(), 0));
        let samples = samples(&replies, 0);
        assert_eq!(find(&samples, "mssql_databases_offline", &[]).unwrap().value, 1.0);
        let online = find(&samples, "mssql_database_online", &[("database", "old")]).unwrap();
        assert_eq!((online.value, online.labels["state"].as_str()), (0.0, "SUSPECT"));
        assert_eq!(find(&samples, "mssql_agent_job_last_ok", &[("job", "Nightly")]).unwrap().value, 0.0);
        assert_eq!(find(&samples, "mssql_agent_jobs_failed", &[]).unwrap().value, 1.0);
        // Un travail en cours (4) n'est pas un échec.
        assert_eq!(find(&samples, "mssql_agent_job_last_ok", &[("job", "Reindex")]).unwrap().value, 1.0);
    }

    #[test]
    fn blocages_et_replique_degradee() {
        let mut replies = healthy();
        replies.blocked_requests = Some(4.0);
        replies.longest_block_ms = Some(125_000.0);
        replies.replicas.push(("SQL2".into(), "shop".into(), 1, true));
        let samples = samples(&replies, 0);
        assert_eq!(find(&samples, "mssql_blocked_requests", &[]).unwrap().value, 4.0);
        assert_eq!(find(&samples, "mssql_longest_block_seconds", &[]).unwrap().value, 125.0);
        assert_eq!(find(&samples, "mssql_alwayson_unhealthy", &[]).unwrap().value, 1.0);
        let suspended = find(&samples, "mssql_alwayson_suspended", &[("replica", "SQL2")]);
        assert_eq!(suspended.unwrap().value, 1.0);
    }

    #[test]
    fn sans_agent_ni_alwayson_pas_de_serie_inventee() {
        let replies = Replies {
            version: "15.0".into(),
            databases: vec![("master".into(), "ONLINE".into(), false)],
            ..Replies::default()
        };
        let samples = samples(&replies, 0);
        assert!(find(&samples, "mssql_agent_jobs_failed", &[]).is_none());
        assert!(find(&samples, "mssql_alwayson_unhealthy", &[]).is_none());
        assert!(find(&samples, "mssql_sessions_used_percent", &[]).is_none());
    }

    #[test]
    fn les_reglages_se_lisent_sur_la_cible() {
        let mut target = cible("mssql", "mssql://sql.lan:1444", &[("database", "app"), ("tls", "no")]);
        target.credential = login();
        let settings = settings(&target).unwrap();
        assert_eq!((settings.host.as_str(), settings.port), ("sql.lan", 1444));
        assert_eq!(settings.database, "app");
        assert!(!settings.tls && !settings.insecure_tls);
        assert_eq!(settings.timeout, DEFAULT_TIMEOUT);

        target.credential = Credential::None;
        assert!(matches!(settings_err(&target), ProbeError::Config(_)));
        let mut target = cible("mssql", "sql.lan", &[("port", "0")]);
        target.credential = login();
        assert!(matches!(settings_err(&target), ProbeError::Config(_)));
    }

    fn settings_err(target: &Target) -> ProbeError {
        settings(target).err().unwrap()
    }

    #[tokio::test]
    async fn un_port_ferme_est_une_panne() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let mut target = cible("mssql", &format!("127.0.0.1:{port}"), &[]);
        target.credential = login();
        let error = MssqlCollector::new().probe(&target).await.unwrap_err();
        assert!(error.means_down(), "{error}");
    }

    /// Un service qui n'est pas SQL Server (il se tait) est une erreur de
    /// protocole ou un délai, jamais un succès.
    #[tokio::test]
    async fn un_service_muet_n_est_pas_un_succes() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (_socket, _) = listener.accept().await.unwrap();
            tokio::time::sleep(Duration::from_secs(30)).await;
        });
        let mut target =
            cible("mssql", &format!("127.0.0.1:{port}"), &[("request_timeout_seconds", "1")]);
        target.credential = login();
        let error = MssqlCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Timeout(_)), "{error}");
    }

    /// Contre une vraie instance : `DUMBMONIT_TEST_MSSQL=hôte` et
    /// `DUMBMONIT_TEST_MSSQL_PASSWORD` (connexion `dumbmonit`).
    #[tokio::test]
    #[ignore = "demande un SQL Server joignable"]
    async fn mssql_reel_joignable() {
        let address = std::env::var("DUMBMONIT_TEST_MSSQL").unwrap();
        let password = std::env::var("DUMBMONIT_TEST_MSSQL_PASSWORD").unwrap();
        let mut target = cible("mssql", &address, &[("insecure_tls", "true")]);
        target.credential = Credential::UsernamePassword { username: "dumbmonit".into(), password };
        for sample in MssqlCollector::new().probe(&target).await.unwrap() {
            println!("{} {:?} {}", sample.metric, sample.labels, sample.value);
        }
    }
}
