//! Keycloak.
//!
//! Deux niveaux, le second facultatif.
//!
//! 1. **La santé** : `/health/ready` et `/health/live`, que Keycloak sert quand
//!    il est lancé avec `health-enabled=true`. Depuis Keycloak 25 elles
//!    écoutent sur l'**interface de gestion**, port 9000 (option
//!    `health_port`) ; avant, sur le port principal (`health_port` = `same`).
//!    Un `503` avec un corps JSON n'est pas une erreur de transport : c'est
//!    Keycloak qui dit quelle vérification (base de données…) est tombée.
//! 2. **L'administration** : avec un client de service (`client_id` et secret,
//!    flux « client credentials »), la version, les domaines (realms), et pour
//!    chacun le nombre de sessions et d'utilisateurs. Le compte de service ne
//!    reçoit que des rôles de lecture ; un refus (403) saute la page sans
//!    échec.
//!
//! La panne que ce module existe pour voir : **un Keycloak qui répond mais dont
//! la base est tombée** — la page de connexion s'affiche, plus personne ne peut
//! se connecter — et un domaine désactivé par erreur.

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use serde_json::Value;

use super::client::{Auth, HttpClient};
use super::options::tag;
use super::{MAX_NAMED, flag, gauge, http_client, metric, now_ms, number, optional_json, settle};

pub const DEFAULT_PORT: u16 = 8080;
pub const DEFAULT_HEALTH_PORT: u16 = 9000;
pub const DEFAULT_AUTH_REALM: &str = "master";
const PRODUCT: &str = "Keycloak";
const P: &str = "keycloak";

/// Au plus autant de domaines interrogés par sonde (deux appels chacun).
pub const MAX_REALMS: usize = 20;

/// Encode un segment de chemin : tout sauf les caractères non réservés.
fn encode(segment: &str) -> String {
    let mut out = String::new();
    for byte in segment.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// L'hôte d'une adresse saisie, sans protocole, port ni chemin : l'interface
/// de gestion écoute sur la même machine mais sur un autre port.
fn host_only(address: &str) -> String {
    let address = address.trim();
    let address = address
        .strip_prefix("https://")
        .or_else(|| address.strip_prefix("http://"))
        .unwrap_or(address);
    let address = address.split('/').next().unwrap_or(address);
    if address.starts_with('[') {
        return match address.find(']') {
            Some(end) => address[..=end].to_string(),
            None => address.to_string(),
        };
    }
    // Plusieurs `:` : une IPv6 sans crochets, sans port.
    if address.matches(':').count() > 1 {
        return address.to_string();
    }
    address.split(':').next().unwrap_or(address).to_string()
}

// ------------------------------------------------------------ traduction

/// Une réponse de `/health/*` : `{"status": "UP|DOWN", "checks": [...]}`.
pub fn health_samples(name: &str, body: &Value, ts_ms: i64) -> Vec<Sample> {
    let up = |value: &Value| value.get("status").and_then(Value::as_str) == Some("UP");
    let mut out = vec![flag(&metric(P, name), up(body), ts_ms)];
    // Le détail des vérifications n'est publié que pour `ready`, où se trouvent
    // la base de données et le cache.
    if name == "ready" {
        let checks = body.get("checks").and_then(Value::as_array).map(Vec::as_slice);
        for check in checks.unwrap_or(&[]).iter().take(MAX_NAMED) {
            if let Some(label) = check.get("name").and_then(Value::as_str) {
                out.push(
                    flag(&metric(P, "health_check_up"), up(check), ts_ms).with_label("check", label),
                );
            }
        }
    }
    out
}

/// `GET /admin/serverinfo`.
pub fn serverinfo_samples(body: &Value, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    if let Some(version) = body.pointer("/systemInfo/version").and_then(Value::as_str) {
        out.push(gauge(&metric(P, "version_info"), 1.0, ts_ms).with_label("version", version));
    }
    if let Some(millis) = number(body.pointer("/systemInfo/uptimeMillis")) {
        out.push(gauge(&metric(P, "uptime_seconds"), millis / 1000.0, ts_ms));
    }
    let total = number(body.pointer("/memoryInfo/total"));
    let used = number(body.pointer("/memoryInfo/used"));
    if let (Some(total), Some(used)) = (total, used)
        && total > 0.0
    {
        out.push(gauge(&metric(P, "memory_used_percent"), 100.0 * used / total, ts_ms));
    }
    out
}

/// Les noms des domaines à interroger, et les compteurs de domaines.
/// `wanted` restreint la liste (option `realms`) ; vide, tous (jusqu'à
/// [`MAX_REALMS`]).
pub fn realm_samples(body: &Value, wanted: &[String], ts_ms: i64) -> (Vec<Sample>, Vec<String>) {
    let list = body.as_array().map(Vec::as_slice).unwrap_or(&[]);
    let disabled = list
        .iter()
        .filter(|r| r.get("enabled").and_then(Value::as_bool) == Some(false))
        .count();
    let samples = vec![
        gauge(&metric(P, "realms_total"), list.len() as f64, ts_ms),
        gauge(&metric(P, "realms_disabled"), disabled as f64, ts_ms),
    ];
    let names = list
        .iter()
        .filter_map(|r| r.get("realm").and_then(Value::as_str))
        .filter(|name| wanted.is_empty() || wanted.iter().any(|w| w == name))
        .take(MAX_REALMS)
        .map(str::to_string)
        .collect();
    (samples, names)
}

/// `GET /admin/realms/{realm}/client-session-stats` : `[{"clientId", "active",
/// "offline"}]`, les nombres arrivant en texte. Rend (actives, hors ligne).
pub fn session_counts(body: &Value) -> (f64, f64) {
    let list = body.as_array().map(Vec::as_slice).unwrap_or(&[]);
    let sum = |key: &str| list.iter().filter_map(|c| number(c.get(key))).sum::<f64>();
    (sum("active"), sum("offline"))
}

pub fn realm_detail_samples(
    realm: &str,
    sessions: Option<(f64, f64)>,
    users: Option<f64>,
    ts_ms: i64,
) -> Vec<Sample> {
    let mut out = Vec::new();
    if let Some((active, offline)) = sessions {
        out.push(gauge(&metric(P, "realm_sessions"), active, ts_ms).with_label("realm", realm));
        out.push(
            gauge(&metric(P, "realm_offline_sessions"), offline, ts_ms).with_label("realm", realm),
        );
    }
    if let Some(users) = users {
        out.push(gauge(&metric(P, "realm_users"), users, ts_ms).with_label("realm", realm));
    }
    out
}

// ------------------------------------------------------------------ sonde

/// Le client qui interroge l'interface de santé.
fn health_client(target: &Target) -> Result<HttpClient, ProbeError> {
    let port = match tag(target, "health_port") {
        Some("same") => return http_client(target, "http", DEFAULT_PORT, Auth::default(), PRODUCT),
        Some(raw) => raw.parse::<u16>().ok().filter(|p| *p > 0).ok_or_else(|| {
            ProbeError::Config(format!("Invalid health port: \"{raw}\" (a number, or \"same\")"))
        })?,
        None => DEFAULT_HEALTH_PORT,
    };
    let mut management = target.clone();
    management.address = host_only(&target.address);
    management.tags.insert("port".to_string(), port.to_string());
    management
        .tags
        .insert("scheme".to_string(), tag(target, "health_scheme").unwrap_or("http").to_string());
    http_client(&management, "http", DEFAULT_HEALTH_PORT, Auth::default(), PRODUCT)
}

fn wanted_realms(target: &Target) -> Vec<String> {
    tag(target, "realms")
        .map(|list| {
            list.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()
        })
        .unwrap_or_default()
}

/// Un jeton d'accès par le flux « client credentials ».
async fn admin_token(
    client: &HttpClient,
    realm: &str,
    client_id: &str,
    secret: &str,
) -> Result<String, ProbeError> {
    let path = format!("/realms/{}/protocol/openid-connect/token", encode(realm));
    let form = [
        ("grant_type", "client_credentials"),
        ("client_id", client_id),
        ("client_secret", secret),
    ];
    let (status, _, body) = client.login(&path, Some(&form[..]), None).await?;
    if matches!(status.as_u16(), 400 | 401 | 403) {
        return Err(ProbeError::Auth(format!(
            "Keycloak refused the client credentials ({status}): check the client ID and secret, \
             that \"Client authentication\" and \"Service accounts roles\" are on, and the \
             realm \"{realm}\"."
        )));
    }
    if !status.is_success() {
        return Err(client.status_error(status, &body, &path));
    }
    let token: Value = client.decode(&body, &path)?;
    token
        .get("access_token")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| ProbeError::Protocol("Keycloak sent no access token.".to_string()))
}

async fn admin_samples(
    target: &Target,
    client_id: &str,
    secret: &str,
    ts_ms: i64,
    errors: &mut u32,
) -> Result<Vec<Sample>, ProbeError> {
    let anonymous = http_client(target, "http", DEFAULT_PORT, Auth::default(), PRODUCT)?;
    let realm = tag(target, "auth_realm").unwrap_or(DEFAULT_AUTH_REALM);
    let token = admin_token(&anonymous, realm, client_id, secret).await?;
    let admin = http_client(
        target,
        "http",
        DEFAULT_PORT,
        Auth::header("Authorization", format!("Bearer {token}")),
        PRODUCT,
    )?;
    let mut out = vec![flag(&metric(P, "admin_readable"), true, ts_ms)];
    let info = optional_json(&admin, "/admin/serverinfo").await;
    if let Some(Some(info)) = settle(info, errors, target.id, "serverinfo") {
        out.extend(serverinfo_samples(&info, ts_ms));
    }
    let realms = optional_json(&admin, "/admin/realms").await;
    let Some(Some(realms)) = settle(realms, errors, target.id, "realms") else { return Ok(out) };
    let (counts, names) = realm_samples(&realms, &wanted_realms(target), ts_ms);
    out.extend(counts);

    let admin_ref = &admin;
    let details = futures::future::join_all(names.iter().map(|name| async move {
        let encoded = encode(name);
        let stats_path = format!("/admin/realms/{encoded}/client-session-stats");
        let users_path = format!("/admin/realms/{encoded}/users/count");
        let (sessions, users) = futures::join!(
            optional_json(admin_ref, &stats_path),
            optional_json(admin_ref, &users_path),
        );
        (name, sessions, users)
    }))
    .await;
    let mut total_active = 0.0;
    let mut readable = false;
    for (name, sessions, users) in details {
        let sessions = settle(sessions, errors, target.id, "client-session-stats").flatten();
        let users = settle(users, errors, target.id, "users/count").flatten();
        let counts = sessions.as_ref().map(session_counts);
        if let Some((active, _)) = counts {
            total_active += active;
            readable = true;
        }
        out.extend(realm_detail_samples(name, counts, users.as_ref().and_then(|u| number(Some(u))), ts_ms));
    }
    if readable {
        out.push(gauge(&metric(P, "sessions_active"), total_active, ts_ms));
    }
    Ok(out)
}

pub async fn probe(target: &Target, ts_ms: i64) -> Result<Vec<Sample>, ProbeError> {
    let health = health_client(target)?;
    let mut errors = 0u32;
    // La preuve de vie : le seul appel de santé qui condamne.
    let ready: Value = health.get_json_even_on_error("/health/ready").await?;
    let mut out = health_samples("ready", &ready, ts_ms);
    let live = health.get_json_even_on_error::<Value>("/health/live").await;
    if let Some(live) = settle(live, &mut errors, target.id, "health/live") {
        out.extend(health_samples("live", &live, ts_ms));
    }
    match &target.credential {
        Credential::None => {}
        Credential::UsernamePassword { username, password } => {
            out.extend(admin_samples(target, username, password, ts_ms, &mut errors).await?);
        }
        other => {
            return Err(ProbeError::Config(format!(
                "Keycloak expects the client ID and secret of a service account (or nothing, \
                 for the health checks only), configured: {other}"
            )));
        }
    }
    out.push(gauge(&metric(P, "scrape_errors"), f64::from(errors), ts_ms));
    Ok(out)
}

// ------------------------------------------------------------ collecteur

#[derive(Default)]
pub struct KeycloakCollector;

impl KeycloakCollector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Collector for KeycloakCollector {
    fn kind(&self) -> &str {
        "keycloak"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        probe(target, now_ms()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let _: Value = health_client(target)?.get_json_even_on_error("/health/ready").await?;
        Ok(Some("keycloak".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::{get, post};

    use super::*;

    // Réponses d'un Keycloak 26.0.5 : santé de l'interface de gestion (prête,
    // puis base de données tombée), `serverinfo`, trois domaines dont un
    // désactivé, statistiques de sessions du domaine « homelab », et le refus
    // d'un client dont le secret est faux.
    const READY: &str = include_str!("testdata/keycloak_26.0.5/health_ready.json");
    const READY_DOWN: &str = include_str!("testdata/keycloak_26.0.5/health_ready_down.json");
    const LIVE: &str = include_str!("testdata/keycloak_26.0.5/health_live.json");
    const SERVERINFO: &str = include_str!("testdata/keycloak_26.0.5/serverinfo.json");
    const REALMS: &str = include_str!("testdata/keycloak_26.0.5/realms.json");
    const SESSIONS: &str = include_str!("testdata/keycloak_26.0.5/client_session_stats_homelab.json");
    const TOKEN: &str = include_str!("testdata/keycloak_26.0.5/token.json");
    const TOKEN_REFUSED: &str = include_str!("testdata/keycloak_26.0.5/token_refused.json");

    fn json(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn realm(samples: &[Sample], name: &str, realm: &str) -> Option<f64> {
        samples
            .iter()
            .find(|s| {
                s.metric == name && s.labels.get("realm").map(String::as_str) == Some(realm)
            })
            .map(|s| s.value)
    }

    #[test]
    fn la_base_de_donnees_tombee_se_voit_dans_les_verifications() {
        let samples = health_samples("ready", &json(READY_DOWN), 0);
        assert_eq!(value(&samples, "keycloak_ready"), Some(0.0));
        let check = samples.iter().find(|s| s.metric == "keycloak_health_check_up").unwrap();
        assert_eq!(check.value, 0.0);
        assert!(check.labels["check"].contains("database"));
        let samples = health_samples("ready", &json(READY), 0);
        assert_eq!(value(&samples, "keycloak_ready"), Some(1.0));
        let live = health_samples("live", &json(LIVE), 0);
        assert_eq!(live.len(), 1);
        assert_eq!(value(&live, "keycloak_live"), Some(1.0));
    }

    #[test]
    fn serverinfo_donne_version_duree_de_vie_et_memoire() {
        let samples = serverinfo_samples(&json(SERVERINFO), 0);
        let info = samples.iter().find(|s| s.metric == "keycloak_version_info").unwrap();
        assert_eq!(info.labels["version"], "26.0.5");
        assert_eq!(value(&samples, "keycloak_uptime_seconds"), Some(443_528.0));
        assert_eq!(value(&samples, "keycloak_memory_used_percent"), Some(75.0));
    }

    #[test]
    fn les_domaines_se_comptent_et_se_filtrent() {
        let (samples, names) = realm_samples(&json(REALMS), &[], 0);
        assert_eq!(value(&samples, "keycloak_realms_total"), Some(3.0));
        assert_eq!(value(&samples, "keycloak_realms_disabled"), Some(1.0));
        assert_eq!(names, ["master", "homelab", "old test"]);
        let (_, names) = realm_samples(&json(REALMS), &["homelab".to_string()], 0);
        assert_eq!(names, ["homelab"]);
    }

    #[test]
    fn les_sessions_arrivent_en_texte_et_se_somment() {
        let counts = session_counts(&json(SESSIONS));
        assert_eq!(counts, (13.0, 1.0));
        let samples = realm_detail_samples("homelab", Some(counts), Some(42.0), 0);
        assert_eq!(realm(&samples, "keycloak_realm_sessions", "homelab"), Some(13.0));
        assert_eq!(realm(&samples, "keycloak_realm_offline_sessions", "homelab"), Some(1.0));
        assert_eq!(realm(&samples, "keycloak_realm_users", "homelab"), Some(42.0));
        assert!(realm_detail_samples("x", None, None, 0).is_empty());
    }

    #[test]
    fn l_adresse_de_gestion_garde_l_hote_seulement() {
        assert_eq!(host_only("sso.lan"), "sso.lan");
        assert_eq!(host_only("sso.lan:8080"), "sso.lan");
        assert_eq!(host_only("https://sso.example.com/auth"), "sso.example.com");
        assert_eq!(host_only("[fd00::1]:8443"), "[fd00::1]");
        assert_eq!(host_only("fd00::1"), "fd00::1");
        assert_eq!(encode("old test"), "old%20test");
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn target(address: String, credential: Credential, tags: &[(&str, &str)]) -> Target {
        Target {
            id: 6,
            name: "sso".into(),
            address,
            kind: "keycloak".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: tags.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<BTreeMap<_, _>>(),
            credential,
            group_name: String::new(),
            position: 0,
        }
    }

    fn secret(password: &str) -> Credential {
        Credential::UsernamePassword { username: "dumbmonit".into(), password: password.into() }
    }

    fn app() -> Router {
        Router::new()
            .route("/health/ready", get(|| async { READY }))
            .route("/health/live", get(|| async { LIVE }))
            .route(
                "/realms/master/protocol/openid-connect/token",
                post(|body: String| async move {
                    if body.contains("client_secret=bon") {
                        (StatusCode::OK, TOKEN)
                    } else {
                        (StatusCode::UNAUTHORIZED, TOKEN_REFUSED)
                    }
                }),
            )
            .route(
                "/admin/serverinfo",
                get(|headers: HeaderMap| async move {
                    let bearer = headers.get("authorization").and_then(|v| v.to_str().ok());
                    if bearer.is_some_and(|b| b.starts_with("Bearer eyJ")) {
                        (StatusCode::OK, SERVERINFO)
                    } else {
                        (StatusCode::UNAUTHORIZED, "{}")
                    }
                }),
            )
            .route("/admin/realms", get(|| async { REALMS }))
            .route("/admin/realms/homelab/client-session-stats", get(|| async { SESSIONS }))
            .route("/admin/realms/homelab/users/count", get(|| async { "42" }))
            // Les autres domaines : le compte de service n'a pas le droit d'y lire.
            .fallback(|| async { (StatusCode::FORBIDDEN, r#"{"error":"HTTP 403 Forbidden"}"#) })
    }

    #[tokio::test]
    async fn la_sante_seule_se_lit_sans_identifiants() {
        let base = serve(app()).await;
        let tags = [("health_port", "same")];
        let samples = KeycloakCollector::new()
            .probe(&target(base, Credential::None, &tags))
            .await
            .unwrap();
        assert_eq!(value(&samples, "keycloak_ready"), Some(1.0));
        assert_eq!(value(&samples, "keycloak_live"), Some(1.0));
        assert!(value(&samples, "keycloak_admin_readable").is_none());
        assert_eq!(value(&samples, "keycloak_scrape_errors"), Some(0.0));
    }

    #[tokio::test]
    async fn l_administration_donne_domaines_sessions_et_utilisateurs() {
        let base = serve(app()).await;
        let tags = [("health_port", "same"), ("realms", "homelab, master")];
        let samples = KeycloakCollector::new()
            .probe(&target(base, secret("bon"), &tags))
            .await
            .unwrap();
        assert_eq!(value(&samples, "keycloak_admin_readable"), Some(1.0));
        assert_eq!(value(&samples, "keycloak_realms_total"), Some(3.0));
        assert_eq!(realm(&samples, "keycloak_realm_sessions", "homelab"), Some(13.0));
        assert_eq!(realm(&samples, "keycloak_realm_users", "homelab"), Some(42.0));
        // « master » est interdit au compte de service : sauté, jamais une panne.
        assert!(realm(&samples, "keycloak_realm_sessions", "master").is_none());
        assert_eq!(value(&samples, "keycloak_sessions_active"), Some(13.0));
        assert_eq!(value(&samples, "keycloak_uptime_seconds"), Some(443_528.0));
    }

    #[tokio::test]
    async fn un_secret_refuse_est_une_erreur_d_authentification() {
        let base = serve(app()).await;
        let tags = [("health_port", "same")];
        let error = KeycloakCollector::new()
            .probe(&target(base, secret("faux"), &tags))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn la_sante_en_echec_reste_une_mesure_et_l_injoignable_une_panne() {
        let app = Router::new()
            .route("/health/ready", get(|| async { (StatusCode::SERVICE_UNAVAILABLE, READY_DOWN) }))
            .route("/health/live", get(|| async { LIVE }));
        let base = serve(app).await;
        let tags = [("health_port", "same")];
        let samples = KeycloakCollector::new()
            .probe(&target(base, Credential::None, &tags))
            .await
            .unwrap();
        assert_eq!(value(&samples, "keycloak_ready"), Some(0.0));

        let error = KeycloakCollector::new()
            .probe(&target("127.0.0.1".into(), Credential::None, &[("health_port", "1")]))
            .await
            .unwrap_err();
        assert!(error.means_down());
        let error = KeycloakCollector::new()
            .probe(&target("127.0.0.1".into(), Credential::None, &[("health_port", "neuf")]))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
    }
}
