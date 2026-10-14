//! XCP-ng / Citrix Hypervisor (XenServer) : un pool, par l'API de gestion
//! XAPI, en JSON-RPC 2.0 (`POST /jsonrpc`, voir `doc/content/xen-api/wire-protocol.md`
//! du dépôt `xapi-project/xen-api`).
//!
//! # XAPI plutôt que Xen Orchestra
//!
//! Deux API lisent un pool XCP-ng : XAPI, embarquée sur chaque hôte, et
//! l'API REST de Xen Orchestra (XO), un service tiers à installer et à
//! tenir à jour séparément. XAPI est retenue : elle ne demande rien de plus
//! que le pool lui-même, et un sujet au rôle intégré `read-only` ne peut rien
//! changer. Le prix est un modèle de données plus brut (des références
//! opaques entre classes, à recomposer ici).
//!
//! # Session
//!
//! `session.login_with_password(uname, pwd, version, originator)` ouvre une
//! session, gardée d'une collecte à l'autre (comme le cookie de
//! `collectors/vsphere`) pour ne pas journaliser une connexion par minute sur
//! le pool. Une session expirée (`SESSION_INVALID`) est rouverte une fois.
//!
//! Un membre du pool qui n'en est pas le coordinateur refuse la connexion avec
//! `HOST_IS_SLAVE` et l'adresse du coordinateur : la collecte la suit une fois
//! (même protocole, même port), et la retient avec la session. Après une
//! bascule du coordinateur, la cible saisie n'a donc pas à être corrigée.
//!
//! # Ce qui est lu
//!
//! `host`, `host_metrics`, `VM`, `VM_metrics`, `SR` et `pool`, chacun par son
//! `get_all_records` : un aller-retour par classe plutôt qu'un par objet. Les
//! modèles, instantanés et domaines de contrôle sont écartés : ce ne sont pas
//! des charges de travail. XenAPI n'a pas d'état « VM plantée » ; une VM que
//! la haute disponibilité d'un pool où elle est active est censée garder en
//! marche (`ha_restart_priority` non vide) et qui est arrêtée est le signal le
//! plus proche d'un arrêt inattendu que l'API expose.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `port` | `443` | Port HTTPS du coordinateur. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable (celui d'XCP-ng par défaut). |
//! | `request_timeout_seconds` | `15` | Délai par appel. |

pub mod metrics;

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use serde_json::{Value, json};

use crate::rest::{RestClient, now_ms};

pub const DEFAULT_PORT: u16 = 443;
const PATH: &str = "/jsonrpc";

/// Version de l'API annoncée à la connexion, et nom de l'appelant que XAPI
/// inscrit dans son journal d'audit.
const API_VERSION: &str = "1.0";
const ORIGINATOR: &str = "DumbMonit";

/// Session ouverte et adresse où elle vaut (celle du coordinateur, qui peut
/// différer de l'adresse saisie après un `HOST_IS_SLAVE`).
#[derive(Clone)]
struct Session {
    base_url: String,
    reference: String,
}

/// Collecteur XCP-ng / XenServer.
#[derive(Default)]
pub struct XcpngCollector {
    /// Session par cible (voir le module).
    sessions: Mutex<HashMap<String, Session>>,
}

impl XcpngCollector {
    pub fn new() -> Self {
        Self::default()
    }

    fn cached(&self, key: &str) -> Option<Session> {
        self.sessions.lock().unwrap_or_else(|poison| poison.into_inner()).get(key).cloned()
    }

    fn remember(&self, key: &str, session: Option<Session>) {
        let mut sessions = self.sessions.lock().unwrap_or_else(|poison| poison.into_inner());
        match session {
            Some(session) => sessions.insert(key.to_string(), session),
            None => sessions.remove(key),
        };
    }
}

/// Clé du cache de sessions : une autre cible, une autre adresse ou un autre
/// compte ouvre une autre session.
fn session_key(target: &Target, username: &str) -> String {
    format!("{}\u{0}{}\u{0}{}", target.id, target.address, username)
}

/// Client vers `base_url` (`None` : l'adresse de la cible), avec les réglages
/// de la cible (certificat, délai).
fn client(target: &Target, base_url: Option<&str>) -> Result<RestClient, ProbeError> {
    match base_url {
        None => RestClient::for_target(target, "https", DEFAULT_PORT, "XCP-ng"),
        Some(url) => {
            let mut moved = target.clone();
            moved.address = url.to_string();
            RestClient::for_target(&moved, "https", DEFAULT_PORT, "XCP-ng")
        }
    }
}

fn credentials(target: &Target) -> Result<(String, String), ProbeError> {
    match &target.credential {
        Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
            Ok((username.trim().to_string(), password.clone()))
        }
        other => Err(ProbeError::Config(format!(
            "XCP-ng expects a user name and password, configured: {other}"
        ))),
    }
}

/// Ce que XAPI a refusé, avant d'être traduit en [`ProbeError`] : la session
/// périmée et le renvoi vers le coordinateur se traitent ici, pas chez
/// l'appelant.
enum Failure {
    Probe(ProbeError),
    SessionInvalid,
    /// Adresse du coordinateur du pool.
    NotCoordinator(String),
}

impl From<ProbeError> for Failure {
    fn from(error: ProbeError) -> Self {
        Self::Probe(error)
    }
}

impl Failure {
    fn into_probe(self) -> ProbeError {
        match self {
            Self::Probe(error) => error,
            Self::SessionInvalid => {
                ProbeError::Protocol("XCP-ng invalidated the session twice in a row".into())
            }
            Self::NotCoordinator(master) => ProbeError::Config(format!(
                "This host is a pool member, not the coordinator: enter the coordinator's \
                 address ({master}) instead."
            )),
        }
    }
}

/// Un appel JSON-RPC : `result`, ou l'erreur XAPI.
async fn call(rest: &RestClient, method: &str, params: Vec<Value>) -> Result<Value, Failure> {
    let body = json!({ "jsonrpc": "2.0", "method": method, "params": params, "id": 1 });
    let (status, text) = rest.post_json(PATH, &body).await?;
    if !status.is_success() {
        return Err(rest.status_error(status, &text, PATH).into());
    }
    let reply: Value = rest.decode(&text, PATH)?;
    if let Some(error) = reply.get("error") {
        return Err(xapi_error(error));
    }
    Ok(reply.get("result").cloned().unwrap_or(Value::Null))
}

/// `{"code": 1, "message": "SESSION_INVALID", "data": ["OpaqueRef:…"]}` : le
/// code XAPI est dans `message`, ses paramètres dans `data`.
fn xapi_error(error: &Value) -> Failure {
    let code = error.get("message").and_then(Value::as_str).unwrap_or("UNKNOWN_ERROR");
    let data: Vec<&str> = error
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let detail =
        if data.is_empty() { code.to_string() } else { format!("{code} ({})", data.join(", ")) };
    match code {
        "SESSION_INVALID" => Failure::SessionInvalid,
        "HOST_IS_SLAVE" => match data.first().map(|address| address.trim()) {
            Some(master) if master.parse::<IpAddr>().is_ok() => {
                Failure::NotCoordinator(master.to_string())
            }
            _ => Failure::Probe(ProbeError::Protocol(format!("XCP-ng answered {detail}"))),
        },
        "SESSION_AUTHENTICATION_FAILED" => Failure::Probe(ProbeError::Auth(format!(
            "XCP-ng refused the user name or password: {detail}."
        ))),
        "RBAC_PERMISSION_DENIED" | "PERMISSION_DENIED" => {
            Failure::Probe(ProbeError::Auth(format!(
                "XCP-ng accepted the account but refused the call: {detail}. Give its subject the \
             read-only role."
            )))
        }
        _ => Failure::Probe(ProbeError::Protocol(format!("XCP-ng answered {detail}"))),
    }
}

/// L'adresse du coordinateur, avec le protocole et le port de `base_url`.
fn coordinator_url(base_url: &str, master: &str) -> Option<String> {
    let mut url = reqwest::Url::parse(base_url).ok()?;
    let host = if master.contains(':') { format!("[{master}]") } else { master.to_string() };
    url.set_host(Some(&host)).ok()?;
    Some(url.as_str().trim_end_matches('/').to_string())
}

async fn login(rest: &RestClient, username: &str, password: &str) -> Result<String, Failure> {
    let params = vec![json!(username), json!(password), json!(API_VERSION), json!(ORIGINATOR)];
    let result = call(rest, "session.login_with_password", params).await?;
    result.as_str().map(str::to_string).ok_or_else(|| {
        Failure::Probe(ProbeError::Protocol("XCP-ng did not return a session reference".into()))
    })
}

/// Connexion à l'adresse de la cible, ou au coordinateur qu'elle désigne.
async fn open(
    target: &Target,
    username: &str,
    password: &str,
) -> Result<(RestClient, Session), ProbeError> {
    let rest = client(target, None)?;
    match login(&rest, username, password).await {
        Ok(reference) => {
            let base_url = rest.base_url().to_string();
            Ok((rest, Session { base_url, reference }))
        }
        Err(Failure::NotCoordinator(master)) => {
            let Some(url) = coordinator_url(rest.base_url(), &master) else {
                return Err(Failure::NotCoordinator(master).into_probe());
            };
            let rest = client(target, Some(&url))?;
            let reference = login(&rest, username, password).await.map_err(Failure::into_probe)?;
            Ok((rest, Session { base_url: url, reference }))
        }
        Err(other) => Err(other.into_probe()),
    }
}

/// Les six classes lues, dans l'ordre où `metrics::*` les attend.
struct Records {
    hosts: Value,
    host_metrics: Value,
    vms: Value,
    vm_metrics: Value,
    srs: Value,
    pools: Value,
}

async fn fetch_all(rest: &RestClient, session: &str) -> Result<Records, Failure> {
    let session = json!(session);
    let read = |class: &'static str| {
        let session = session.clone();
        async move { call(rest, &format!("{class}.get_all_records"), vec![session]).await }
    };
    Ok(Records {
        hosts: read("host").await?,
        host_metrics: read("host_metrics").await?,
        vms: read("VM").await?,
        vm_metrics: read("VM_metrics").await?,
        srs: read("SR").await?,
        pools: read("pool").await?,
    })
}

impl XcpngCollector {
    async fn collect(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let (username, password) = credentials(target)?;
        let key = session_key(target, &username);

        let (rest, session) = match self.cached(&key) {
            Some(session) => (client(target, Some(&session.base_url))?, session),
            None => {
                let (rest, session) = open(target, &username, &password).await?;
                self.remember(&key, Some(session.clone()));
                (rest, session)
            }
        };

        let ts_ms = now_ms();
        let records = match fetch_all(&rest, &session.reference).await {
            Ok(records) => records,
            // Session expirée, révoquée, ou coordinateur changé depuis : une
            // seule nouvelle connexion, depuis l'adresse saisie.
            Err(Failure::SessionInvalid | Failure::NotCoordinator(_)) => {
                self.remember(&key, None);
                let (rest, session) = open(target, &username, &password).await?;
                self.remember(&key, Some(session.clone()));
                fetch_all(&rest, &session.reference).await.map_err(|failure| {
                    self.remember(&key, None);
                    failure.into_probe()
                })?
            }
            Err(Failure::Probe(error)) => {
                self.remember(&key, None);
                return Err(error);
            }
        };

        let ha_enabled = metrics::pool_ha_enabled(&records.pools);
        let mut out = metrics::host_samples(&records.hosts, &records.host_metrics, ts_ms);
        out.extend(metrics::vm_samples(&records.vms, &records.vm_metrics, ha_enabled, ts_ms));
        out.extend(metrics::sr_samples(&records.srs, ts_ms));
        out.extend(metrics::pool_samples(&records.pools, &records.hosts, ts_ms));
        Ok(out)
    }
}

#[async_trait]
impl Collector for XcpngCollector {
    fn kind(&self) -> &str {
        "xcpng"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        self.collect(target).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        self.collect(target).await?;
        Ok(Some("xcpng".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::extract::State;
    use axum::http::StatusCode;
    use axum::routing::post;
    use axum::{Json, Router};

    use super::*;
    use crate::rest::test_support::{find, serve};
    use crate::uptime::tags::test_support::cible;

    const LOGIN: &str = include_str!("testdata/login.json");
    const LOGIN_REFUSED: &str = include_str!("testdata/login_refused.json");
    const SESSION_INVALID: &str = include_str!("testdata/session_invalid.json");
    const HOSTS: &str = include_str!("testdata/hosts.json");
    const HOST_METRICS: &str = include_str!("testdata/host_metrics.json");
    const VMS: &str = include_str!("testdata/vms.json");
    const VM_METRICS: &str = include_str!("testdata/vm_metrics.json");
    const SRS: &str = include_str!("testdata/srs.json");
    const POOLS: &str = include_str!("testdata/pools.json");

    #[derive(Clone, Default)]
    struct Pool {
        logins: Arc<AtomicUsize>,
    }

    async fn handler(State(pool): State<Pool>, Json(body): Json<Value>) -> (StatusCode, String) {
        let method = body["method"].as_str().unwrap_or_default();
        let params = body["params"].as_array().cloned().unwrap_or_default();
        if method == "session.login_with_password" {
            pool.logins.fetch_add(1, Ordering::SeqCst);
            assert_eq!(params.len(), 4, "uname, pwd, version, originator");
            let user = params.first().and_then(Value::as_str);
            let pass = params.get(1).and_then(Value::as_str);
            return if user == Some("dumbmonit") && pass == Some("s3cret") {
                (StatusCode::OK, LOGIN.to_string())
            } else {
                (StatusCode::OK, LOGIN_REFUSED.to_string())
            };
        }
        if params.first().and_then(Value::as_str) == Some("stale-session") {
            return (StatusCode::OK, SESSION_INVALID.to_string());
        }
        let reply = match method {
            "host.get_all_records" => HOSTS,
            "host_metrics.get_all_records" => HOST_METRICS,
            "VM.get_all_records" => VMS,
            "VM_metrics.get_all_records" => VM_METRICS,
            "SR.get_all_records" => SRS,
            "pool.get_all_records" => POOLS,
            other => panic!("unexpected XAPI method: {other}"),
        };
        (StatusCode::OK, reply.to_string())
    }

    fn app(pool: Pool) -> Router {
        Router::new().route(PATH, post(handler)).with_state(pool)
    }

    fn target(address: &str) -> Target {
        let mut target = cible("xcpng", address, &[]);
        target.credential = Credential::UsernamePassword {
            username: "dumbmonit".into(),
            password: "s3cret".into(),
        };
        target
    }

    #[tokio::test]
    async fn a_collection_logs_in_once_then_reuses_the_session() {
        let pool = Pool::default();
        let address = serve(app(pool.clone())).await;
        let collector = XcpngCollector::new();
        let target = target(&format!("http://{address}"));
        let samples = collector.probe(&target).await.unwrap();
        assert!(find(&samples, "xcpng_hosts", &[]).is_some());
        assert!(find(&samples, "xcpng_vms", &[]).is_some());
        assert!(find(&samples, "xcpng_srs", &[]).is_some());
        assert!(find(&samples, "xcpng_pool_ha_enabled", &[("pool", "Production pool")]).is_some());
        collector.probe(&target).await.unwrap();
        assert_eq!(pool.logins.load(Ordering::SeqCst), 1, "session gardée");
    }

    #[tokio::test]
    async fn a_stale_session_is_reopened_exactly_once() {
        let address = serve(app(Pool::default())).await;
        let collector = XcpngCollector::new();
        let target = target(&format!("http://{address}"));
        let base_url = format!("http://{address}");
        collector.remember(
            &session_key(&target, "dumbmonit"),
            Some(Session { base_url, reference: "stale-session".into() }),
        );
        let samples = collector.probe(&target).await.unwrap();
        assert!(find(&samples, "xcpng_hosts", &[]).is_some(), "collecte malgré la session périmée");
    }

    #[test]
    fn the_coordinator_keeps_the_scheme_and_port() {
        assert_eq!(
            coordinator_url("http://127.0.0.1:4430", "10.0.0.11").unwrap(),
            "http://10.0.0.11:4430"
        );
        assert_eq!(coordinator_url("https://xcp02.lan", "10.0.0.11").unwrap(), "https://10.0.0.11");
        assert_eq!(
            coordinator_url("https://xcp02.lan:8443", "fd00::1").unwrap(),
            "https://[fd00::1]:8443"
        );
    }

    #[tokio::test]
    async fn host_is_slave_with_the_coordinator_on_the_same_port_is_followed() {
        // Un seul serveur joue les deux rôles : il refuse en tant que membre la
        // connexion arrivée par `localhost`, et l'accepte arrivée par 127.0.0.1.
        async fn both(
            State(logins): State<Arc<AtomicUsize>>,
            headers: axum::http::HeaderMap,
            Json(body): Json<Value>,
        ) -> (StatusCode, String) {
            let host = headers.get("host").and_then(|v| v.to_str().ok()).unwrap_or_default();
            if body["method"] == "session.login_with_password" {
                logins.fetch_add(1, Ordering::SeqCst);
                if host.starts_with("localhost") {
                    return (
                        StatusCode::OK,
                        r#"{"jsonrpc":"2.0","error":{"code":1,"message":"HOST_IS_SLAVE","data":["127.0.0.1"]},"id":1}"#
                            .to_string(),
                    );
                }
            }
            handler(State(Pool::default()), Json(body)).await
        }
        let logins = Arc::new(AtomicUsize::new(0));
        let app = Router::new().route(PATH, post(both)).with_state(logins.clone());
        let address = serve(app).await;
        let port = address.rsplit(':').next().unwrap();
        let collector = XcpngCollector::new();
        let target = target(&format!("http://localhost:{port}"));
        let samples = collector.probe(&target).await.unwrap();
        assert!(find(&samples, "xcpng_hosts", &[]).is_some());
        assert_eq!(logins.load(Ordering::SeqCst), 2, "membre, puis coordinateur");
        collector.probe(&target).await.unwrap();
        assert_eq!(logins.load(Ordering::SeqCst), 2, "le coordinateur est retenu");
    }

    #[tokio::test]
    async fn wrong_credentials_are_an_auth_error() {
        let address = serve(app(Pool::default())).await;
        let mut target = target(&format!("http://{address}"));
        target.credential =
            Credential::UsernamePassword { username: "dumbmonit".into(), password: "wrong".into() };
        let error = XcpngCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn a_missing_credential_is_a_configuration_error() {
        let mut target = target("http://127.0.0.1:1");
        target.credential = Credential::None;
        let error = XcpngCollector::new().probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }

    #[tokio::test]
    async fn an_unreachable_pool_means_down() {
        let error = XcpngCollector::new().probe(&target("http://127.0.0.1:1")).await.unwrap_err();
        assert!(error.means_down(), "{error}");
    }

    #[test]
    fn xapi_error_codes_are_sorted() {
        let error = xapi_error(&json!({"message": "SESSION_INVALID", "data": ["OpaqueRef:x"]}));
        assert!(matches!(error, Failure::SessionInvalid));
        let error = xapi_error(&json!({"message": "HOST_IS_SLAVE", "data": ["10.0.0.11"]}));
        assert!(matches!(error, Failure::NotCoordinator(ref ip) if ip == "10.0.0.11"));
        // Une adresse qui n'en est pas une n'est jamais suivie.
        let error = xapi_error(&json!({"message": "HOST_IS_SLAVE", "data": ["evil.example/x"]}));
        assert!(matches!(error, Failure::Probe(ProbeError::Protocol(_))));
        let error = xapi_error(&json!({"message": "RBAC_PERMISSION_DENIED", "data": []}));
        assert!(matches!(error, Failure::Probe(ProbeError::Auth(_))));
    }
}
