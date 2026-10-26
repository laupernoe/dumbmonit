//! Oracle Database, par l'API REST d'Oracle REST Data Services (ORDS).
//!
//! # Pourquoi ORDS
//!
//! Le protocole natif d'Oracle (TNS/Net8) est propriétaire et sans
//! implémentation libre en Rust pur ; les pilotes courants passent par OCI, une
//! bibliothèque C propriétaire (Instant Client) que l'image statique `scratch`
//! ne peut ni embarquer ni charger, et dont la licence interdit la
//! redistribution. Aucune crate sous licence OSI ne parle TNS de bout en bout.
//!
//! ORDS, lui, est un composant officiel et gratuit d'Oracle qui expose la base en
//! HTTP. Avec son « REST-enabled SQL », une requête `SELECT` part en JSON sur
//! `POST /ords/<schéma>/_/sql` : c'est ce que ce module emploie, avec le client
//! HTTP commun et la même authentification (compte de la base, en basic). La
//! limite est assumée : il faut qu'ORDS soit déployé devant la base, avec
//! `restEnabledSql.active` à `true` et le schéma du compte activé pour REST.
//! Pour une base sans ORDS, un script exécuté par l'agent (sqlplus) est la voie
//! de repli ; elle n'est pas fournie ici.
//!
//! Le compte de supervision n'a que `CREATE SESSION` et `SELECT_CATALOG_ROLE`
//! (lecture des vues `V$` et `DBA_`), aucun droit sur les données.
//!
//! # Ce que ce module existe pour voir
//!
//! * **l'instance qui n'est plus ouverte** (`MOUNTED`, `STARTED`) ;
//! * **un tablespace presque plein**, avant l'erreur `ORA-01653` ;
//! * **les sessions et processus qui approchent leur limite** (`ORA-00018`,
//!   `ORA-00020`) ;
//! * **un Data Guard en retard** : décalage de transport et d'application ;
//! * **la zone de récupération rapide pleine** (archivage bloqué, base figée) ;
//! * **les travaux du planificateur en échec** sur 24 heures.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `http` | `https` si ORDS est servi en TLS. |
//! | `port` | `8080` | Port d'ORDS. |
//! | `ords_path` | `/ords` | Préfixe d'ORDS (contexte d'application). |
//! | `schema` | nom du compte, en minuscules | Alias REST du schéma activé. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable. |
//! | `request_timeout_seconds` | `10` | Délai par requête. |

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, MetricKind, ProbeError, Sample, Target};
use serde_json::{Value, json};

use crate::observability::client::{Auth, HttpClient};
use crate::observability::options::Options;
use crate::uptime::tags;

pub const DEFAULT_PORT: u16 = 8080;
pub const DEFAULT_ORDS_PATH: &str = "/ords";

const INSTANCE: &str = "select status, database_status, version, \
    round((sysdate - startup_time) * 86400) as uptime from v$instance";
const DATABASE: &str = "select open_mode, database_role, log_mode from v$database";
const TABLESPACES: &str = "select tablespace_name, used_percent from dba_tablespace_usage_metrics";
const LIMITS: &str = "select resource_name, current_utilization, limit_value \
    from v$resource_limit where resource_name in ('sessions', 'processes')";
const DATAGUARD: &str = "select name, value from v$dataguard_stats \
    where name in ('transport lag', 'apply lag')";
const RECOVERY_AREA: &str = "select round(space_used / nullif(space_limit, 0) * 100, 2) \
    as used_percent from v$recovery_file_dest";
const FAILED_JOBS: &str = "select count(*) as failed from dba_scheduler_job_run_details \
    where status = 'FAILED' and log_date > systimestamp - 1";

#[derive(Default)]
pub struct OracleCollector;

impl OracleCollector {
    pub fn new() -> Self {
        Self
    }
}

struct Connection {
    client: HttpClient,
    sql_path: String,
}

fn connection(target: &Target) -> Result<Connection, ProbeError> {
    let options = Options::from_target(target, DEFAULT_PORT)?;
    let (username, password) = match &target.credential {
        Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
            (username.trim().to_string(), password.clone())
        }
        other => {
            return Err(ProbeError::Config(format!(
                "Oracle (ORDS) expects a database user name and password, configured: {other}"
            )));
        }
    };
    let prefix = tags::tag(target, "ords_path").unwrap_or(DEFAULT_ORDS_PATH);
    let prefix = format!("/{}", prefix.trim_matches('/'));
    let prefix = if prefix == "/" { String::new() } else { prefix };
    let schema = tags::tag(target, "schema").map(str::to_string).unwrap_or_else(|| {
        // L'alias REST d'un schéma est, par défaut, le nom du compte en minuscules.
        username.to_lowercase()
    });
    if schema.is_empty() || schema.contains(['/', '?', '#', ' ']) {
        return Err(ProbeError::Config(format!("Invalid schema alias: \"{schema}\"")));
    }
    let auth = Auth::Basic { username, password };
    Ok(Connection {
        client: HttpClient::new(
            crate::http::client(options.insecure_tls)?,
            options.base_url,
            auth,
            options.request_timeout,
            "Oracle ORDS",
        ),
        sql_path: format!("{prefix}/{schema}/_/sql"),
    })
}

/// Le résultat d'une requête : des lignes, clés en minuscules, ou une erreur
/// d'Oracle (`ORA-…`).
#[derive(Debug)]
pub enum Outcome {
    Rows(Vec<Value>),
    Failed(String),
}

/// Lit la réponse du REST-enabled SQL pour une seule instruction.
pub fn parse_outcome(body: &str) -> Result<Outcome, ProbeError> {
    let json: Value = serde_json::from_str(body).map_err(|error| {
        ProbeError::Protocol(format!(
            "Unexpected response from ORDS: {error}. Check the ORDS path and that REST-enabled \
             SQL is on."
        ))
    })?;
    let item = json.pointer("/items/0").ok_or_else(|| {
        ProbeError::Protocol("ORDS answered without a statement result.".to_string())
    })?;
    // En cas d'erreur, ORDS pose `error` (ou `errorDetails`) sur l'instruction ;
    // le code d'état de l'instruction reste 200 côté HTTP.
    if let Some(error) = item.get("errorDetails").or_else(|| item.get("error")) {
        let text = error
            .as_str()
            .map(str::to_string)
            .or_else(|| error.get("message").and_then(Value::as_str).map(str::to_string))
            .unwrap_or_else(|| error.to_string());
        return Ok(Outcome::Failed(text));
    }
    let rows =
        item.pointer("/resultSet/items").and_then(Value::as_array).cloned().unwrap_or_default();
    Ok(Outcome::Rows(rows))
}

async fn query(connection: &Connection, statement: &str) -> Result<Outcome, ProbeError> {
    let reply = connection
        .client
        .post_raw(&connection.sql_path, &json!({ "statementText": statement }))
        .await?;
    match reply.status.as_u16() {
        200 => parse_outcome(&reply.body),
        _ => Err(connection.client.status_error(reply.status, &reply.body, &connection.sql_path)),
    }
}

/// Une requête facultative : une vue absente ou refusée (`ORA-00942`) laisse
/// simplement ses métriques de côté.
async fn optional(connection: &Connection, statement: &str) -> Result<Vec<Value>, ProbeError> {
    Ok(match query(connection, statement).await? {
        Outcome::Rows(rows) => rows,
        Outcome::Failed(_) => Vec::new(),
    })
}

/// Ce que les requêtes ont rapporté.
#[derive(Default)]
pub struct Replies {
    pub instance: Vec<Value>,
    pub database: Vec<Value>,
    pub tablespaces: Vec<Value>,
    pub limits: Vec<Value>,
    pub dataguard: Vec<Value>,
    pub recovery_area: Vec<Value>,
    pub failed_jobs: Vec<Value>,
}

/// `ORA-01017` : identifiants refusés par la base (ORDS les lui transmet).
fn oracle_error(text: &str) -> ProbeError {
    if text.contains("ORA-01017") || text.contains("ORA-28000") {
        ProbeError::Auth(format!("Oracle refused the account: {text}"))
    } else {
        ProbeError::Config(format!(
            "Oracle refused the query: {text}. The account needs CREATE SESSION and \
             SELECT_CATALOG_ROLE."
        ))
    }
}

pub async fn read(target: &Target) -> Result<Replies, ProbeError> {
    let connection = connection(target)?;
    // L'instance est la requête qui prouve que tout fonctionne : son échec est
    // une erreur de configuration, pas une métrique manquante.
    let instance = match query(&connection, INSTANCE).await? {
        Outcome::Rows(rows) => rows,
        Outcome::Failed(text) => return Err(oracle_error(&text)),
    };
    Ok(Replies {
        instance,
        database: optional(&connection, DATABASE).await?,
        tablespaces: optional(&connection, TABLESPACES).await?,
        limits: optional(&connection, LIMITS).await?,
        dataguard: optional(&connection, DATAGUARD).await?,
        recovery_area: optional(&connection, RECOVERY_AREA).await?,
        failed_jobs: optional(&connection, FAILED_JOBS).await?,
    })
}

#[async_trait]
impl Collector for OracleCollector {
    fn kind(&self) -> &'static str {
        "oracle"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let replies = read(target).await?;
        Ok(samples(&replies, chrono::Utc::now().timestamp_millis()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let connection = connection(target)?;
        match query(&connection, INSTANCE).await? {
            Outcome::Rows(_) => Ok(Some("oracle".to_string())),
            Outcome::Failed(text) => Err(oracle_error(&text)),
        }
    }
}

/// Un nombre, qu'ORDS l'écrive en JSON ou en texte (`"300"`, `"UNLIMITED"`).
fn value_f64(row: &Value, key: &str) -> Option<f64> {
    let value = row.get(key)?;
    match value {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
    .filter(|v| v.is_finite())
}

fn value_str<'a>(row: &'a Value, key: &str) -> &'a str {
    row.get(key).and_then(Value::as_str).unwrap_or("")
}

/// `+00 00:01:30` ou `+00 00:01:30.5` (intervalle jour-seconde de Data Guard) en secondes.
pub fn parse_interval(text: &str) -> Option<f64> {
    let text = text.trim().trim_start_matches(['+', '-']);
    let (days, time) = text.split_once(' ')?;
    let mut parts = time.split(':');
    let hours: f64 = parts.next()?.parse().ok()?;
    let minutes: f64 = parts.next()?.parse().ok()?;
    let seconds: f64 = parts.next()?.parse().ok()?;
    let days: f64 = days.parse().ok()?;
    Some(days * 86_400.0 + hours * 3600.0 + minutes * 60.0 + seconds)
}

pub fn samples(replies: &Replies, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let gauge = |name: &str, value: f64| Sample::new(name, value, MetricKind::Gauge, ts_ms);

    if let Some(instance) = replies.instance.first() {
        let status = value_str(instance, "status");
        out.push(gauge("oracle_instance_open", if status == "OPEN" { 1.0 } else { 0.0 }));
        out.push(
            gauge("oracle_version_info", 1.0)
                .with_label("version", value_str(instance, "version"))
                .with_label("status", status)
                .with_label("database_status", value_str(instance, "database_status")),
        );
        if let Some(uptime) = value_f64(instance, "uptime") {
            out.push(gauge("oracle_uptime_seconds", uptime));
        }
    }
    if let Some(database) = replies.database.first() {
        out.push(
            gauge("oracle_database_info", 1.0)
                .with_label("open_mode", value_str(database, "open_mode"))
                .with_label("role", value_str(database, "database_role"))
                .with_label("log_mode", value_str(database, "log_mode")),
        );
    }
    for row in &replies.tablespaces {
        if let Some(used) = value_f64(row, "used_percent") {
            out.push(
                gauge("oracle_tablespace_used_percent", used)
                    .with_label("tablespace", value_str(row, "tablespace_name")),
            );
        }
    }
    for row in &replies.limits {
        let name = value_str(row, "resource_name");
        let Some(current) = value_f64(row, "current_utilization") else { continue };
        out.push(gauge(&format!("oracle_{name}"), current));
        // `UNLIMITED` n'est pas un nombre : pas de pourcentage.
        if let Some(limit) = value_f64(row, "limit_value")
            && limit > 0.0
        {
            out.push(gauge(&format!("oracle_{name}_used_percent"), current / limit * 100.0));
        }
    }
    for row in &replies.dataguard {
        let metric = match value_str(row, "name") {
            "transport lag" => "oracle_dataguard_transport_lag_seconds",
            "apply lag" => "oracle_dataguard_apply_lag_seconds",
            _ => continue,
        };
        if let Some(seconds) = parse_interval(value_str(row, "value")) {
            out.push(gauge(metric, seconds));
        }
    }
    if let Some(used) = replies.recovery_area.first().and_then(|r| value_f64(r, "used_percent")) {
        out.push(gauge("oracle_recovery_area_used_percent", used));
    }
    if let Some(failed) = replies.failed_jobs.first().and_then(|r| value_f64(r, "failed")) {
        out.push(gauge("oracle_scheduler_jobs_failed_24h", failed));
    }
    out
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::post;

    use super::*;
    use crate::uptime::tags::test_support::cible;

    fn rows(items: Value) -> Vec<Value> {
        items.as_array().unwrap().clone()
    }

    fn find<'a>(samples: &'a [Sample], name: &str, labels: &[(&str, &str)]) -> Option<&'a Sample> {
        samples.iter().find(|s| {
            s.metric == name
                && labels.iter().all(|(k, v)| s.labels.get(*k).map(String::as_str) == Some(v))
        })
    }

    #[test]
    fn la_reponse_rest_est_lue() {
        let body = r#"{"env":{},"items":[{"statementId":1,"statementType":"query",
            "resultSet":{"items":[{"status":"OPEN","version":"19.0.0.0.0"}],"hasMore":false}}]}"#;
        match parse_outcome(body).unwrap() {
            Outcome::Rows(rows) => assert_eq!(rows[0]["status"], "OPEN"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn une_erreur_oracle_est_reconnue() {
        let body = r#"{"items":[{"statementId":1,"statementType":"query","response":[],
            "error":{"line":1,"column":22,"message":"ORA-00942: table or view does not exist"}}]}"#;
        match parse_outcome(body).unwrap() {
            Outcome::Failed(text) => assert!(text.contains("ORA-00942"), "{text}"),
            other => panic!("{other:?}"),
        }
        assert!(parse_outcome("<html>").is_err());
        assert!(matches!(oracle_error("ORA-01017: invalid password"), ProbeError::Auth(_)));
        assert!(matches!(oracle_error("ORA-00942"), ProbeError::Config(_)));
    }

    #[test]
    fn les_intervalles_data_guard() {
        assert_eq!(parse_interval("+00 00:01:30"), Some(90.0));
        assert_eq!(parse_interval("+01 02:00:00.5"), Some(93_600.5));
        assert_eq!(parse_interval(""), None);
        assert_eq!(parse_interval("n/a"), None);
    }

    #[test]
    fn metriques_d_une_base_chargee() {
        let replies = Replies {
            instance: rows(json!([{"status": "OPEN", "database_status": "ACTIVE",
                "version": "19.0.0.0.0", "uptime": 7200}])),
            database: rows(json!([{"open_mode": "READ WRITE", "database_role": "PRIMARY",
                "log_mode": "ARCHIVELOG"}])),
            tablespaces: rows(json!([
                {"tablespace_name": "USERS", "used_percent": 93.5},
                {"tablespace_name": "SYSTEM", "used_percent": 40}
            ])),
            limits: rows(json!([
                {"resource_name": "sessions", "current_utilization": 90, "limit_value": " 300"},
                {"resource_name": "processes", "current_utilization": 50,
                    "limit_value": "UNLIMITED"}
            ])),
            dataguard: rows(json!([
                {"name": "transport lag", "value": "+00 00:00:02"},
                {"name": "apply lag", "value": "+00 00:10:00"}
            ])),
            recovery_area: rows(json!([{"used_percent": 71.25}])),
            failed_jobs: rows(json!([{"failed": 2}])),
        };
        let samples = samples(&replies, 0);
        assert_eq!(find(&samples, "oracle_instance_open", &[]).unwrap().value, 1.0);
        assert_eq!(find(&samples, "oracle_uptime_seconds", &[]).unwrap().value, 7200.0);
        let users = find(&samples, "oracle_tablespace_used_percent", &[("tablespace", "USERS")]);
        assert_eq!(users.unwrap().value, 93.5);
        assert_eq!(find(&samples, "oracle_sessions_used_percent", &[]).unwrap().value, 30.0);
        assert!(find(&samples, "oracle_processes_used_percent", &[]).is_none());
        assert_eq!(find(&samples, "oracle_processes", &[]).unwrap().value, 50.0);
        assert_eq!(find(&samples, "oracle_dataguard_apply_lag_seconds", &[]).unwrap().value, 600.0);
        assert_eq!(find(&samples, "oracle_recovery_area_used_percent", &[]).unwrap().value, 71.25);
        assert_eq!(find(&samples, "oracle_scheduler_jobs_failed_24h", &[]).unwrap().value, 2.0);
        let info = find(&samples, "oracle_database_info", &[]).unwrap();
        assert_eq!(info.labels["role"], "PRIMARY");
    }

    #[test]
    fn une_instance_montee_n_est_pas_ouverte() {
        let replies = Replies {
            instance: rows(json!([{"status": "MOUNTED", "version": "19.0.0.0.0"}])),
            ..Replies::default()
        };
        let samples = samples(&replies, 0);
        assert_eq!(find(&samples, "oracle_instance_open", &[]).unwrap().value, 0.0);
        assert!(find(&samples, "oracle_uptime_seconds", &[]).is_none());
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    #[tokio::test]
    async fn interrogation_par_ords() {
        let app = Router::new().route(
            "/ords/dumbmonit/_/sql",
            post(|headers: HeaderMap, body: String| async move {
                // dumbmonit:secret
                if headers.get("authorization").and_then(|v| v.to_str().ok())
                    != Some("Basic ZHVtYm1vbml0OnNlY3JldA==")
                {
                    return (StatusCode::UNAUTHORIZED, String::new());
                }
                let item = if body.contains("v$instance") {
                    r#"{"resultSet":{"items":[{"status":"OPEN","version":"23.4.0.0.0","uptime":60}]}}"#
                } else {
                    r#"{"error":{"message":"ORA-00942: table or view does not exist"}}"#
                };
                (StatusCode::OK, format!(r#"{{"items":[{item}]}}"#))
            }),
        );
        let base = serve(app).await;
        let mut target = cible("oracle", &base, &[]);
        target.credential = Credential::UsernamePassword {
            username: "DUMBMONIT".into(),
            password: "secret".into(),
        };
        let samples = OracleCollector::new().probe(&target).await.unwrap();
        assert_eq!(find(&samples, "oracle_instance_open", &[]).unwrap().value, 1.0);
        // Les vues refusées laissent leurs métriques de côté.
        assert!(find(&samples, "oracle_tablespace_used_percent", &[]).is_none());

        target.credential =
            Credential::UsernamePassword { username: "DUMBMONIT".into(), password: "faux".into() };
        let error = OracleCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)) && !error.means_down(), "{error}");

        target.credential = Credential::None;
        let error = OracleCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }

    #[tokio::test]
    async fn un_port_ferme_est_une_panne() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let mut target = cible("oracle", &format!("127.0.0.1:{port}"), &[]);
        target.credential =
            Credential::UsernamePassword { username: "dumbmonit".into(), password: "x".into() };
        let error = OracleCollector::new().probe(&target).await.unwrap_err();
        assert!(error.means_down(), "{error}");
    }

    #[test]
    fn le_chemin_ords_se_compose_des_etiquettes() {
        let mut target =
            cible("oracle", "db.lan", &[("ords_path", "/prod/ords/"), ("schema", "mon")]);
        target.credential =
            Credential::UsernamePassword { username: "DUMBMONIT".into(), password: "x".into() };
        assert_eq!(connection(&target).unwrap().sql_path, "/prod/ords/mon/_/sql");
        let mut target = cible("oracle", "db.lan", &[("schema", "a/b")]);
        target.credential =
            Credential::UsernamePassword { username: "DUMBMONIT".into(), password: "x".into() };
        assert!(matches!(connection(&target), Err(ProbeError::Config(_))));
    }
}
