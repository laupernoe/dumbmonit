//! Vaultwarden.
//!
//! Deux niveaux, le second facultatif.
//!
//! 1. **Public** : `/alive` (la preuve de vie, que Vaultwarden sert sans
//!    identifiant) et `/api/config` (la version). Aucun secret, aucun jeton :
//!    c'est tout ce qu'il faut pour savoir que le coffre répond.
//! 2. **Administration** : avec le jeton d'administration (`ADMIN_TOKEN`), le
//!    nombre d'utilisateurs, de comptes désactivés, de comptes sans second
//!    facteur et d'éléments. Vaultwarden n'a pas d'API d'administration à
//!    jeton : le jeton ouvre une session sur la page `/admin` (cookie
//!    `VW_ADMIN`, vingt minutes). Cette ouverture est limitée (trois essais,
//!    puis un toutes les cinq minutes) : le cookie est donc gardé et réutilisé
//!    entre deux sondes, jamais renégocié à chaque minute.
//!
//! Aucun courriel, nom, mot de passe ou élément de coffre n'est lu ou publié :
//! seuls des compteurs sortent de la page des utilisateurs.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target, TargetId};
use reqwest::StatusCode;
use serde_json::Value;

use super::client::{Auth, HttpClient, cookies_of};
use super::options::Options;
use super::{flag, gauge, http_client, metric, now_ms, number, settle, token};

pub const DEFAULT_PORT: u16 = 443;
const PRODUCT: &str = "Vaultwarden";
const P: &str = "vaultwarden";

/// Durée pendant laquelle un cookie d'administration est réutilisé. La
/// session de Vaultwarden dure vingt minutes par défaut.
const SESSION_REUSE: Duration = Duration::from_secs(15 * 60);

// ------------------------------------------------------------ traduction

/// `GET /api/config`.
pub fn config_samples(body: &Value, ts_ms: i64) -> Vec<Sample> {
    let version = body.get("version").and_then(Value::as_str).unwrap_or("unknown");
    let mut out = vec![gauge(&metric(P, "version_info"), 1.0, ts_ms).with_label("version", version)];
    if let Some(closed) = body.pointer("/settings/disableUserRegistration").and_then(Value::as_bool)
    {
        out.push(flag(&metric(P, "signups_open"), !closed, ts_ms));
    }
    out
}

/// `GET /admin/users` : uniquement des compteurs.
pub fn user_samples(body: &Value, ts_ms: i64) -> Vec<Sample> {
    let users = body.as_array().map(Vec::as_slice).unwrap_or(&[]);
    let enabled = |user: &Value| user.get("user_enabled").and_then(Value::as_bool).unwrap_or(true);
    let two_factor =
        |user: &Value| user.get("TwoFactorEnabled").and_then(Value::as_bool).unwrap_or(false);
    let disabled = users.iter().filter(|u| !enabled(u)).count();
    // Un compte désactivé n'a pas à avoir de second facteur : on ne compte que
    // les comptes actifs qui s'en passent.
    let without_2fa = users.iter().filter(|u| enabled(u) && !two_factor(u)).count();
    let ciphers: f64 = users.iter().filter_map(|u| number(u.get("cipher_count"))).sum();
    vec![
        flag(&metric(P, "admin_readable"), true, ts_ms),
        gauge(&metric(P, "users"), users.len() as f64, ts_ms),
        gauge(&metric(P, "users_disabled"), disabled as f64, ts_ms),
        gauge(&metric(P, "users_without_2fa"), without_2fa as f64, ts_ms),
        gauge(&metric(P, "items"), ciphers, ts_ms),
    ]
}

// ------------------------------------------------------------------ sonde

fn net(error: &reqwest::Error, timeout: Duration, path: &str) -> ProbeError {
    if error.is_timeout() {
        return ProbeError::Timeout(timeout);
    }
    ProbeError::Unreachable(format!("{path}: {error}"))
}

/// Un client qui ne suit aucune redirection : la connexion à `/admin` répond
/// par un `303` dont c'est précisément le cookie qu'on veut lire.
fn admin_http(options: &Options) -> Result<reqwest::Client, ProbeError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .danger_accept_invalid_certs(options.insecure_tls)
        .connect_timeout(Duration::from_secs(10))
        .user_agent(concat!("DumbMonit/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| ProbeError::Config(format!("HTTP client unavailable: {error}")))
}

async fn admin_login(
    http: &reqwest::Client,
    options: &Options,
    admin_token: &str,
) -> Result<String, ProbeError> {
    let path = "/admin";
    let response = http
        .post(format!("{}{path}", options.base_url))
        .form(&[("token", admin_token)])
        .timeout(options.request_timeout)
        .send()
        .await
        .map_err(|error| net(&error, options.request_timeout, path))?;
    let status = response.status();
    let cookie = cookies_of(response.headers());
    match status {
        StatusCode::NOT_FOUND => Err(ProbeError::Protocol(
            "Vaultwarden's admin page is disabled (no ADMIN_TOKEN set on the server).".to_string(),
        )),
        StatusCode::TOO_MANY_REQUESTS => Err(ProbeError::Protocol(
            "Vaultwarden rate-limited the admin login: probe less often, or raise \
             ADMIN_RATELIMIT_MAX_BURST."
                .to_string(),
        )),
        _ if cookie.is_empty() => Err(ProbeError::Auth(
            "Vaultwarden refused the admin token. Use the plain token typed on the /admin \
             page, not its hash."
                .to_string(),
        )),
        _ => Ok(cookie),
    }
}

/// La liste des utilisateurs, ou `None` si la session n'est plus valable.
async fn fetch_users(
    http: &reqwest::Client,
    options: &Options,
    cookie: &str,
) -> Result<Option<Value>, ProbeError> {
    let path = "/admin/users";
    let response = http
        .get(format!("{}{path}", options.base_url))
        .header(reqwest::header::COOKIE, cookie)
        .header(reqwest::header::ACCEPT, "application/json")
        .timeout(options.request_timeout)
        .send()
        .await
        .map_err(|error| net(&error, options.request_timeout, path))?;
    let status = response.status();
    if status.is_redirection()
        || matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN)
    {
        return Ok(None);
    }
    let body = response.text().await.map_err(|error| net(&error, options.request_timeout, path))?;
    if !status.is_success() {
        let excerpt: String = body.chars().take(200).collect();
        return Err(if status.is_server_error() {
            ProbeError::Unreachable(format!("{PRODUCT} answered {status} on {path}: {excerpt}"))
        } else {
            ProbeError::Protocol(format!("{PRODUCT} answered {status} on {path}: {excerpt}"))
        });
    }
    serde_json::from_str(&body).map(Some).map_err(|error| {
        ProbeError::Protocol(format!("{PRODUCT} sent an unexpected answer on {path}: {error}"))
    })
}

// ------------------------------------------------------------ collecteur

/// Garde le cookie d'administration de chaque cible, avec l'instant où il a
/// été obtenu.
#[derive(Default)]
pub struct VaultwardenCollector {
    sessions: Mutex<HashMap<TargetId, (String, Instant)>>,
}

impl VaultwardenCollector {
    pub fn new() -> Self {
        Self::default()
    }

    fn cached(&self, id: TargetId) -> Option<String> {
        let sessions = self.sessions.lock().ok()?;
        let (cookie, since) = sessions.get(&id)?;
        (since.elapsed() < SESSION_REUSE).then(|| cookie.clone())
    }

    fn remember(&self, id: TargetId, cookie: &str) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.insert(id, (cookie.to_string(), Instant::now()));
        }
    }

    async fn users(&self, target: &Target, admin_token: &str) -> Result<Value, ProbeError> {
        let options = Options::from_target(target, "https", DEFAULT_PORT)?;
        let http = admin_http(&options)?;
        if let Some(cookie) = self.cached(target.id)
            && let Some(users) = fetch_users(&http, &options, &cookie).await?
        {
            return Ok(users);
        }
        let cookie = admin_login(&http, &options, admin_token).await?;
        self.remember(target.id, &cookie);
        fetch_users(&http, &options, &cookie).await?.ok_or_else(|| {
            ProbeError::Auth("Vaultwarden refused the admin session it just opened.".to_string())
        })
    }

    fn client(target: &Target) -> Result<HttpClient, ProbeError> {
        http_client(target, "https", DEFAULT_PORT, Auth::default(), PRODUCT)
    }
}

#[async_trait]
impl Collector for VaultwardenCollector {
    fn kind(&self) -> &str {
        "vaultwarden"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let ts_ms = now_ms();
        let client = Self::client(target)?;
        // La preuve de vie : le seul appel qui condamne.
        let (status, body) = client.get("/alive").await?;
        if !status.is_success() {
            return Err(client.status_error(status, &body, "/alive"));
        }
        let mut errors = 0u32;
        let mut out = vec![flag(&metric(P, "up"), true, ts_ms)];
        let config = client.get_json::<Value>("/api/config").await;
        if let Some(config) = settle(config, &mut errors, target.id, "api/config") {
            out.extend(config_samples(&config, ts_ms));
        }
        if let Some(admin_token) = token(&target.credential) {
            out.extend(user_samples(&self.users(target, admin_token).await?, ts_ms));
        }
        out.push(gauge(&metric(P, "scrape_errors"), f64::from(errors), ts_ms));
        Ok(out)
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let client = Self::client(target)?;
        let (status, body) = client.get("/alive").await?;
        if !status.is_success() {
            return Err(client.status_error(status, &body, "/alive"));
        }
        Ok(Some("vaultwarden".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::Router;
    use axum::extract::Request;
    use axum::http::{HeaderMap, header};
    use axum::response::{IntoResponse, Redirect};
    use axum::routing::{get, post};
    use dumbmonit_proto::Credential;

    use super::*;

    // Réponses d'un Vaultwarden 1.34.3 : `/alive`, `/api/config` et la liste
    // des utilisateurs de l'administration (trois comptes : un sans second
    // facteur et un désactivé). Les adresses et identifiants sont fictifs.
    const ALIVE: &str = include_str!("testdata/vaultwarden_1.34.3/alive.json");
    const CONFIG: &str = include_str!("testdata/vaultwarden_1.34.3/config.json");
    const USERS: &str = include_str!("testdata/vaultwarden_1.34.3/admin_users.json");

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    #[test]
    fn la_configuration_donne_la_version_et_l_etat_des_inscriptions() {
        let samples = config_samples(&serde_json::from_str(CONFIG).unwrap(), 0);
        let info = samples.iter().find(|s| s.metric == "vaultwarden_version_info").unwrap();
        assert_eq!(info.labels["version"], "2025.5.0");
        assert_eq!(value(&samples, "vaultwarden_signups_open"), Some(0.0));
        let open = serde_json::json!({"version": "1", "settings": {"disableUserRegistration": false}});
        assert_eq!(value(&config_samples(&open, 0), "vaultwarden_signups_open"), Some(1.0));
    }

    #[test]
    fn les_utilisateurs_ne_sortent_que_en_compteurs() {
        let samples = user_samples(&serde_json::from_str(USERS).unwrap(), 0);
        assert_eq!(value(&samples, "vaultwarden_users"), Some(3.0));
        assert_eq!(value(&samples, "vaultwarden_users_disabled"), Some(1.0));
        // Bob seul : Eve est désactivée, Alice a un second facteur.
        assert_eq!(value(&samples, "vaultwarden_users_without_2fa"), Some(1.0));
        assert_eq!(value(&samples, "vaultwarden_items"), Some(272.0));
        for sample in &samples {
            assert!(sample.labels.is_empty(), "{} porte une étiquette", sample.metric);
        }
    }

    #[test]
    fn une_liste_vide_donne_des_zeros() {
        let samples = user_samples(&serde_json::json!([]), 0);
        assert_eq!(value(&samples, "vaultwarden_users"), Some(0.0));
        assert_eq!(value(&samples, "vaultwarden_items"), Some(0.0));
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn target(address: String, credential: Credential) -> Target {
        Target {
            id: 21,
            name: "vault".into(),
            address,
            kind: "vaultwarden".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::new(),
            credential,
            group_name: String::new(),
            position: 0,
        }
    }

    fn app(logins: Arc<AtomicUsize>) -> Router {
        Router::new()
            .route("/alive", get(|| async { ALIVE }))
            .route("/api/config", get(|| async { CONFIG }))
            .route(
                "/admin",
                post(move |body: String| {
                    let logins = logins.clone();
                    async move {
                        if !body.contains("token=bon") {
                            return (
                                axum::http::StatusCode::UNAUTHORIZED,
                                "Invalid admin token, please try again.",
                            )
                                .into_response();
                        }
                        logins.fetch_add(1, Ordering::SeqCst);
                        let mut response = Redirect::to("/admin").into_response();
                        response.headers_mut().insert(
                            header::SET_COOKIE,
                            "VW_ADMIN=jwt; HttpOnly; SameSite=Strict; Path=/admin".parse().unwrap(),
                        );
                        response
                    }
                }),
            )
            .route(
                "/admin/users",
                get(|request: Request| async move {
                    let headers: &HeaderMap = request.headers();
                    let cookie = headers.get(header::COOKIE).and_then(|v| v.to_str().ok());
                    if cookie == Some("VW_ADMIN=jwt") {
                        USERS.into_response()
                    } else {
                        Redirect::to("/admin").into_response()
                    }
                }),
            )
    }

    fn admin(token: &str) -> Credential {
        Credential::ApiToken { token: token.into() }
    }

    #[tokio::test]
    async fn sans_jeton_seuls_le_public_est_lu() {
        let base = serve(app(Arc::new(AtomicUsize::new(0)))).await;
        let samples =
            VaultwardenCollector::new().probe(&target(base, Credential::None)).await.unwrap();
        assert_eq!(value(&samples, "vaultwarden_up"), Some(1.0));
        assert!(value(&samples, "vaultwarden_users").is_none());
        assert!(value(&samples, "vaultwarden_admin_readable").is_none());
        assert_eq!(value(&samples, "vaultwarden_scrape_errors"), Some(0.0));
    }

    #[tokio::test]
    async fn le_cookie_d_administration_est_reutilise() {
        let logins = Arc::new(AtomicUsize::new(0));
        let base = serve(app(logins.clone())).await;
        let collector = VaultwardenCollector::new();
        for _ in 0..3 {
            let samples = collector.probe(&target(base.clone(), admin("bon"))).await.unwrap();
            assert_eq!(value(&samples, "vaultwarden_users"), Some(3.0));
        }
        assert_eq!(logins.load(Ordering::SeqCst), 1, "une seule ouverture de session admin");
    }

    #[tokio::test]
    async fn un_mauvais_jeton_est_une_erreur_d_authentification() {
        let base = serve(app(Arc::new(AtomicUsize::new(0)))).await;
        let error = VaultwardenCollector::new()
            .probe(&target(base, admin("mauvais")))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn un_coffre_eteint_est_une_panne() {
        let error = VaultwardenCollector::new()
            .probe(&target("http://127.0.0.1:1".into(), Credential::None))
            .await
            .unwrap_err();
        assert!(error.means_down());
    }
}
