//! qBittorrent (WebUI API v2).
//!
//! L'API ouvre une session par cookie : `POST /api/v2/auth/login` avec le
//! compte de l'interface web, puis le cookie `SID` accompagne chaque requête.
//! Sans identifiants, la sonde essaie quand même : qBittorrent peut sauter
//! l'authentification pour un sous-réseau ou pour `localhost` (option « Bypass
//! authentication for clients on whitelisted IP subnets »).
//!
//! Un seul appel, `GET /api/v2/sync/maindata?rid=0`, rend tout d'un coup : l'état
//! du serveur (vitesses, connectivité, espace libre) et la liste des torrents
//! avec leur état. Compter les états suffit à voir ce qui coince.
//!
//! Les pannes que ce module existe pour voir : **torrents en erreur** (disque
//! plein, fichiers manquants après un déplacement) qui ne reprennent jamais
//! seuls, **connectivité perdue** (port fermé, VPN tombé, « firewalled »), et
//! **espace libre** du dossier de téléchargement.

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use reqwest::StatusCode;
use serde_json::Value;

use super::client::{Auth, HttpClient, cookies_of};
use super::{MAX_NAMED, flag, gauge, http_client, metric, now_ms, number};

pub const DEFAULT_PORT: u16 = 8080;
const PRODUCT: &str = "qBittorrent";
const P: &str = "qbittorrent";

/// Longueur maximale d'un nom de torrent repris dans une étiquette.
const MAX_NAME: usize = 120;

/// Le groupe d'un état de torrent, tel que l'API le nomme (v4 et v5).
pub fn state_group(state: &str) -> &'static str {
    match state {
        "error" | "missingFiles" => "error",
        "downloading" | "forcedDL" | "metaDL" | "forcedMetaDL" | "allocating" => "downloading",
        "stalledDL" => "stalled_download",
        "uploading" | "forcedUP" | "stalledUP" => "seeding",
        "pausedDL" | "pausedUP" | "stoppedDL" | "stoppedUP" => "paused",
        "queuedDL" | "queuedUP" => "queued",
        "checkingDL" | "checkingUP" | "checkingResumeData" | "moving" => "checking",
        _ => "other",
    }
}

const GROUPS: [&str; 8] = [
    "downloading",
    "stalled_download",
    "seeding",
    "paused",
    "queued",
    "checking",
    "error",
    "other",
];

// ------------------------------------------------------------ traduction

/// `GET /api/v2/sync/maindata?rid=0` en métriques.
pub fn samples(version: &str, body: &Value, ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![gauge(&metric(P, "version_info"), 1.0, ts_ms).with_label("version", version)];
    let state = body.get("server_state");
    let field = |key: &str| state.and_then(|s| number(s.get(key)));
    if let Some(status) = state.and_then(|s| s.get("connection_status")).and_then(Value::as_str) {
        out.push(
            flag(&metric(P, "connected"), status == "connected", ts_ms)
                .with_label("status", status),
        );
    }
    for (name, key) in [
        ("download_speed_bytes", "dl_info_speed"),
        ("upload_speed_bytes", "up_info_speed"),
        ("free_space_bytes", "free_space_on_disk"),
        ("dht_nodes", "dht_nodes"),
    ] {
        if let Some(value) = field(key) {
            out.push(gauge(&metric(P, name), value, ts_ms));
        }
    }

    let empty = serde_json::Map::new();
    let torrents = body.get("torrents").and_then(Value::as_object).unwrap_or(&empty);
    let mut counts = [0usize; GROUPS.len()];
    let mut named = 0usize;
    let mut errored: Vec<Sample> = Vec::new();
    for torrent in torrents.values() {
        let state = torrent.get("state").and_then(Value::as_str).unwrap_or("");
        let group = state_group(state);
        if let Some(index) = GROUPS.iter().position(|g| *g == group) {
            counts[index] += 1;
        }
        if group == "error" && named < MAX_NAMED {
            named += 1;
            let name: String = torrent
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unnamed")
                .chars()
                .take(MAX_NAME)
                .collect();
            errored.push(
                gauge(&metric(P, "torrent_error"), 1.0, ts_ms)
                    .with_label("torrent", &name)
                    .with_label("state", state),
            );
        }
    }
    out.push(gauge(&metric(P, "torrents_total"), torrents.len() as f64, ts_ms));
    for (group, count) in GROUPS.iter().zip(counts) {
        out.push(gauge(&metric(P, "torrents"), count as f64, ts_ms).with_label("state", *group));
    }
    out.push(gauge(&metric(P, "torrents_errored"), counts[6] as f64, ts_ms));
    out.extend(errored);
    out
}

// ------------------------------------------------------------------ sonde

/// Ouvre la session si des identifiants sont configurés, et rend le client
/// qui porte le cookie.
async fn session(target: &Target) -> Result<HttpClient, ProbeError> {
    let anonymous = http_client(target, "http", DEFAULT_PORT, Auth::default(), PRODUCT)?;
    let (username, password) = match &target.credential {
        Credential::UsernamePassword { username, password } => (username, password),
        Credential::None => return Ok(anonymous),
        other => {
            return Err(ProbeError::Config(format!(
                "qBittorrent expects a user name and password (or none, with authentication \
                 bypass), configured: {other}"
            )));
        }
    };
    let form = [("username", username.as_str()), ("password", password.as_str())];
    let path = "/api/v2/auth/login";
    let (status, headers, body) = anonymous.login(path, Some(&form[..]), None).await?;
    if status == StatusCode::FORBIDDEN {
        return Err(ProbeError::Auth(
            "qBittorrent banned this address after too many failed logins (403): wait, or \
             clear the ban in the WebUI settings."
                .to_string(),
        ));
    }
    if !status.is_success() {
        return Err(anonymous.status_error(status, &body, path));
    }
    if body.trim().eq_ignore_ascii_case("fails.") {
        return Err(ProbeError::Auth(
            "qBittorrent refused the user name or password.".to_string(),
        ));
    }
    let cookie = cookies_of(&headers);
    if cookie.is_empty() {
        return Ok(anonymous);
    }
    http_client(target, "http", DEFAULT_PORT, Auth::header("Cookie", cookie), PRODUCT)
}

pub async fn probe(client: &HttpClient, ts_ms: i64) -> Result<Vec<Sample>, ProbeError> {
    // La preuve de vie et d'authentification : le seul appel qui condamne.
    let path = "/api/v2/app/version";
    let (status, body) = client.get(path).await?;
    if !status.is_success() {
        return Err(client.status_error(status, &body, path));
    }
    let version = body.trim().trim_matches('"');
    let maindata: Value = client.get_json("/api/v2/sync/maindata?rid=0").await?;
    Ok(samples(version, &maindata, ts_ms))
}

// ------------------------------------------------------------ collecteur

#[derive(Default)]
pub struct QbittorrentCollector;

impl QbittorrentCollector {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Collector for QbittorrentCollector {
    fn kind(&self) -> &str {
        "qbittorrent"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        probe(&session(target).await?, now_ms()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let client = session(target).await?;
        let (status, body) = client.get("/api/v2/app/version").await?;
        if !status.is_success() {
            return Err(client.status_error(status, &body, "/api/v2/app/version"));
        }
        Ok(Some("qbittorrent".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use axum::Router;
    use axum::http::{HeaderMap, header};
    use axum::response::IntoResponse;
    use axum::routing::{get, post};

    use super::*;

    // `sync/maindata` d'un qBittorrent 5.0.4 : sept torrents, dont un en
    // erreur, un aux fichiers manquants et un bloqué sans source.
    const MAINDATA: &str = include_str!("testdata/qbittorrent_5.0.4/maindata.json");
    const FIREWALLED: &str = include_str!("testdata/qbittorrent_5.0.4/maindata_firewalled.json");
    const VERSION: &str = include_str!("testdata/qbittorrent_5.0.4/version.txt");

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn by_state(samples: &[Sample], state: &str) -> Option<f64> {
        samples
            .iter()
            .find(|s| {
                s.metric == "qbittorrent_torrents"
                    && s.labels.get("state").map(String::as_str) == Some(state)
            })
            .map(|s| s.value)
    }

    #[test]
    fn les_etats_des_deux_generations_d_api_se_regroupent() {
        assert_eq!(state_group("pausedUP"), "paused");
        assert_eq!(state_group("stoppedUP"), "paused");
        assert_eq!(state_group("missingFiles"), "error");
        assert_eq!(state_group("forcedMetaDL"), "downloading");
        assert_eq!(state_group("stalledUP"), "seeding");
        assert_eq!(state_group("nouveau"), "other");
    }

    #[test]
    fn les_torrents_se_comptent_par_etat_et_les_erreurs_se_nomment() {
        let samples = samples("v5.0.4", &serde_json::from_str(MAINDATA).unwrap(), 0);
        assert_eq!(value(&samples, "qbittorrent_torrents_total"), Some(7.0));
        assert_eq!(value(&samples, "qbittorrent_torrents_errored"), Some(2.0));
        assert_eq!(by_state(&samples, "downloading"), Some(1.0));
        assert_eq!(by_state(&samples, "stalled_download"), Some(1.0));
        assert_eq!(by_state(&samples, "seeding"), Some(1.0));
        assert_eq!(by_state(&samples, "paused"), Some(1.0));
        assert_eq!(by_state(&samples, "queued"), Some(1.0));
        let named: Vec<&Sample> =
            samples.iter().filter(|s| s.metric == "qbittorrent_torrent_error").collect();
        assert_eq!(named.len(), 2);
        assert!(named.iter().any(|s| s.labels["torrent"] == "Docu.S01E01.720p.mkv"));
        assert!(named.iter().any(|s| s.labels["state"] == "missingFiles"));
    }

    #[test]
    fn vitesses_connectivite_et_espace_libre() {
        let samples = samples("v5.0.4", &serde_json::from_str(MAINDATA).unwrap(), 0);
        assert_eq!(value(&samples, "qbittorrent_connected"), Some(1.0));
        assert_eq!(value(&samples, "qbittorrent_download_speed_bytes"), Some(4_194_304.0));
        assert_eq!(value(&samples, "qbittorrent_upload_speed_bytes"), Some(65_536.0));
        assert_eq!(value(&samples, "qbittorrent_free_space_bytes"), Some(268_435_456_000.0));
        assert_eq!(value(&samples, "qbittorrent_dht_nodes"), Some(312.0));
    }

    #[test]
    fn un_port_ferme_se_voit_comme_une_connectivite_perdue() {
        let samples = samples("v5.0.4", &serde_json::from_str(FIREWALLED).unwrap(), 0);
        assert_eq!(value(&samples, "qbittorrent_connected"), Some(0.0));
        let connected = samples.iter().find(|s| s.metric == "qbittorrent_connected").unwrap();
        assert_eq!(connected.labels["status"], "firewalled");
        assert_eq!(value(&samples, "qbittorrent_torrents_total"), Some(0.0));
        assert_eq!(value(&samples, "qbittorrent_torrents_errored"), Some(0.0));
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn target(address: String, credential: Credential) -> Target {
        Target {
            id: 3,
            name: "torrents".into(),
            address,
            kind: "qbittorrent".into(),
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

    fn login_ok() -> impl IntoResponse {
        ([(header::SET_COOKIE, "SID=abc123; HttpOnly; SameSite=Strict; path=/")], "Ok.")
    }

    fn app() -> Router {
        let authorised = |headers: &HeaderMap| {
            headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) == Some("SID=abc123")
        };
        Router::new()
            .route(
                "/api/v2/auth/login",
                post(|body: String| async move {
                    if body.contains("password=secret") {
                        login_ok().into_response()
                    } else {
                        "Fails.".into_response()
                    }
                }),
            )
            .route(
                "/api/v2/app/version",
                get(move |headers: HeaderMap| async move {
                    if authorised(&headers) {
                        (axum::http::StatusCode::OK, VERSION)
                    } else {
                        (axum::http::StatusCode::FORBIDDEN, "Forbidden")
                    }
                }),
            )
            .route("/api/v2/sync/maindata", get(|| async { MAINDATA }))
    }

    fn login(user: &str, password: &str) -> Credential {
        Credential::UsernamePassword { username: user.into(), password: password.into() }
    }

    #[tokio::test]
    async fn la_session_ouverte_par_cookie_donne_les_mesures() {
        let base = serve(app()).await;
        let samples = QbittorrentCollector::new()
            .probe(&target(base, login("admin", "secret")))
            .await
            .unwrap();
        assert_eq!(value(&samples, "qbittorrent_torrents_errored"), Some(2.0));
        let info = samples.iter().find(|s| s.metric == "qbittorrent_version_info").unwrap();
        assert_eq!(info.labels["version"], "v5.0.4");
    }

    #[tokio::test]
    async fn un_mauvais_mot_de_passe_n_est_pas_une_panne() {
        let base = serve(app()).await;
        let error = QbittorrentCollector::new()
            .probe(&target(base, login("admin", "mauvais")))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn sans_identifiants_et_sans_contournement_l_acces_est_refuse() {
        let base = serve(app()).await;
        let error =
            QbittorrentCollector::new().probe(&target(base, Credential::None)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
    }

    #[tokio::test]
    async fn un_jeton_n_est_pas_un_identifiant_qbittorrent() {
        let credential = Credential::ApiToken { token: "x".into() };
        let error = QbittorrentCollector::new()
            .probe(&target("http://127.0.0.1:1".into(), credential))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
    }
}
