//! UniFi Protect (caméras et enregistreur vidéo).
//!
//! Protect n'a pas d'API de supervision publique à jeton qui rende le
//! stockage du NVR : tout ce qu'il sait de lui passe par l'« amorce » que
//! l'application web charge à l'ouverture, `GET /proxy/protect/api/bootstrap`,
//! derrière une session UniFi OS (`POST /api/auth/login`, cookie `TOKEN`).
//! DumbMonit s'y connecte avec un **compte local** limité à la consultation de
//! Protect, jamais avec un compte Ubiquiti en nuage. Le cookie est gardé entre
//! deux sondes ; un 401 déclenche une nouvelle ouverture de session.
//!
//! Les pannes que ce module existe pour voir : **une caméra hors ligne** (câble,
//! PoE, firmware), **une caméra connectée qui n'enregistre pas** (le disque
//! est plein ou l'enregistrement est coupé) et **un disque du NVR en défaut**.
//!
//! Distinct du type `unifi` (UniFi Network : bornes, commutateurs, passerelle).

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target, TargetId};
use reqwest::StatusCode;
use serde_json::Value;

use super::client::{Auth, HttpClient, cookies_of};
use super::{MAX_NAMED, flag, gauge, http_client, metric, now_ms, number};

pub const DEFAULT_PORT: u16 = 443;
const PRODUCT: &str = "UniFi Protect";
const P: &str = "unifiprotect";

// ------------------------------------------------------------ traduction

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// `GET /proxy/protect/api/bootstrap` en métriques.
pub fn samples(bootstrap: &Value, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let nvr = bootstrap.get("nvr");
    let version = nvr.and_then(|n| text(n, "version")).unwrap_or("unknown");
    out.push(gauge(&metric(P, "version_info"), 1.0, ts_ms).with_label("version", version));

    let cameras: Vec<&Value> = bootstrap
        .get("cameras")
        .and_then(Value::as_array)
        .map(|list| {
            // Les caméras découvertes mais pas adoptées n'appartiennent pas
            // encore au système : elles ne comptent pas comme des pannes.
            list.iter().filter(|c| c.get("isAdopted").and_then(Value::as_bool) != Some(false)).collect()
        })
        .unwrap_or_default();
    let (mut offline, mut idle) = (0usize, 0usize);
    for (index, camera) in cameras.iter().enumerate() {
        let online = text(camera, "state") == Some("CONNECTED")
            || camera.get("isConnected").and_then(Value::as_bool) == Some(true);
        let recording = camera.get("isRecording").and_then(Value::as_bool).unwrap_or(false);
        let never = camera.pointer("/recordingSettings/mode").and_then(Value::as_str) == Some("never");
        if !online {
            offline += 1;
        } else if !recording && !never {
            idle += 1;
        }
        if index >= MAX_NAMED {
            continue;
        }
        let name = text(camera, "name").unwrap_or("camera");
        out.push(flag(&metric(P, "camera_online"), online, ts_ms).with_label("camera", name));
        out.push(
            flag(&metric(P, "camera_recording"), online && recording, ts_ms)
                .with_label("camera", name),
        );
    }
    out.push(gauge(&metric(P, "cameras_total"), cameras.len() as f64, ts_ms));
    out.push(gauge(&metric(P, "cameras_offline"), offline as f64, ts_ms));
    out.push(gauge(&metric(P, "cameras_not_recording"), idle as f64, ts_ms));

    if let Some(nvr) = nvr {
        if let Some(disabled) = nvr.get("isRecordingDisabled").and_then(Value::as_bool) {
            out.push(flag(&metric(P, "recording_disabled"), disabled, ts_ms));
        }
        // La capacité vient de `systemInfo.storage` (Protect ≥ 5), à défaut de
        // l'espace d'enregistrement des statistiques.
        let storage = nvr.pointer("/systemInfo/storage");
        let space = nvr.pointer("/storageStats/recordingSpace");
        let figure = |from: Option<&Value>, key: &str| from.and_then(|v| number(v.get(key)));
        let size = figure(storage, "size").or_else(|| figure(space, "total"));
        let used = figure(storage, "used").or_else(|| figure(space, "used"));
        if let (Some(size), Some(used)) = (size, used)
            && size > 0.0
        {
            out.push(gauge(
                &metric(P, "storage_used_percent"),
                (100.0 * used / size).clamp(0.0, 100.0),
                ts_ms,
            ));
            out.push(gauge(&metric(P, "storage_size_bytes"), size, ts_ms));
        }
        if let Some(devices) =
            nvr.pointer("/systemInfo/storage/devices").and_then(Value::as_array)
        {
            let unhealthy = devices
                .iter()
                .filter(|d| d.get("healthy").and_then(Value::as_bool) == Some(false))
                .count();
            out.push(gauge(&metric(P, "disks_total"), devices.len() as f64, ts_ms));
            out.push(gauge(&metric(P, "disks_unhealthy"), unhealthy as f64, ts_ms));
        }
    }
    out
}

// ------------------------------------------------------------------ sonde

fn credentials(target: &Target) -> Result<(&str, &str), ProbeError> {
    match &target.credential {
        Credential::UsernamePassword { username, password } => Ok((username, password)),
        other => Err(ProbeError::Config(format!(
            "UniFi Protect expects the user name and password of a local account, \
             configured: {other}"
        ))),
    }
}

fn client(target: &Target, cookie: Option<&str>) -> Result<HttpClient, ProbeError> {
    let auth = match cookie {
        Some(cookie) => Auth::header("Cookie", cookie),
        None => Auth::default(),
    };
    http_client(target, "https", DEFAULT_PORT, auth, PRODUCT)
}

/// Ouvre une session UniFi OS et rend le cookie à renvoyer.
async fn login(target: &Target) -> Result<String, ProbeError> {
    let (username, password) = credentials(target)?;
    let path = "/api/auth/login";
    let body = serde_json::json!({ "username": username, "password": password, "rememberMe": false });
    let (status, headers, text) = client(target, None)?.login(path, None, Some(&body)).await?;
    if matches!(
        status,
        StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN
    ) {
        return Err(ProbeError::Auth(
            "UniFi OS refused the user name or password. Use a local account with access to \
             Protect, not an Ubiquiti cloud account."
                .to_string(),
        ));
    }
    if !status.is_success() {
        return Err(client(target, None)?.status_error(status, &text, path));
    }
    let cookie = cookies_of(&headers);
    if cookie.is_empty() {
        return Err(ProbeError::Protocol("UniFi OS opened no session (no cookie).".to_string()));
    }
    Ok(cookie)
}

/// L'amorce de Protect, ou `None` si la session a expiré.
async fn bootstrap(target: &Target, cookie: &str) -> Result<Option<Value>, ProbeError> {
    let path = "/proxy/protect/api/bootstrap";
    let client = client(target, Some(cookie))?;
    let (status, body) = client.get(path).await?;
    if status == StatusCode::UNAUTHORIZED {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(client.status_error(status, &body, path));
    }
    client.decode(&body, path).map(Some)
}

// ------------------------------------------------------------ collecteur

/// Garde le cookie de session de chaque cible entre deux sondes : UniFi OS
/// limite les ouvertures de session, et en créer une par minute les remplirait
/// de sessions inutiles.
#[derive(Default)]
pub struct ProtectCollector {
    sessions: Mutex<HashMap<TargetId, String>>,
}

impl ProtectCollector {
    pub fn new() -> Self {
        Self::default()
    }

    fn cached(&self, id: TargetId) -> Option<String> {
        self.sessions.lock().ok().and_then(|sessions| sessions.get(&id).cloned())
    }

    fn remember(&self, id: TargetId, cookie: &str) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.insert(id, cookie.to_string());
        }
    }

    async fn fetch(&self, target: &Target) -> Result<Value, ProbeError> {
        credentials(target)?;
        if let Some(cookie) = self.cached(target.id)
            && let Some(value) = bootstrap(target, &cookie).await?
        {
            return Ok(value);
        }
        let cookie = login(target).await?;
        self.remember(target.id, &cookie);
        bootstrap(target, &cookie).await?.ok_or_else(|| {
            ProbeError::Auth(
                "UniFi OS accepted the login but Protect refused the session (401): the \
                 account has no access to Protect."
                    .to_string(),
            )
        })
    }
}

#[async_trait]
impl Collector for ProtectCollector {
    fn kind(&self) -> &str {
        "unifiprotect"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        Ok(samples(&self.fetch(target).await?, now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        self.fetch(target).await?;
        Ok(Some("unifiprotect".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use axum::Router;
    use axum::http::{HeaderMap, StatusCode, header};
    use axum::response::IntoResponse;
    use axum::routing::{get, post};

    use super::*;

    // Amorce d'un UNVR sous Protect 6.0.34 : cinq caméras dont une hors ligne
    // (« Garage »), une connectée qui n'enregistre pas (« Jardin »), une dont
    // l'enregistrement est volontairement coupé (« Atelier ») et une non
    // adoptée ; deux disques sains, 84 % d'occupation. L'autre amorce est celle
    // d'un NVR 5.3.2 au disque en défaut et à l'enregistrement coupé.
    const BOOTSTRAP: &str = include_str!("testdata/protect_6.0.34/bootstrap.json");
    const DISK_FAILED: &str = include_str!("testdata/protect_6.0.34/bootstrap_disk_failed.json");
    const LOGIN_OK: &str = include_str!("testdata/protect_6.0.34/login_ok.json");
    const LOGIN_REFUSED: &str = include_str!("testdata/protect_6.0.34/login_refused.json");

    fn json(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn camera(samples: &[Sample], name: &str, camera: &str) -> Option<f64> {
        samples
            .iter()
            .find(|s| {
                s.metric == name && s.labels.get("camera").map(String::as_str) == Some(camera)
            })
            .map(|s| s.value)
    }

    #[test]
    fn les_cameras_hors_ligne_et_muettes_se_distinguent() {
        let samples = samples(&json(BOOTSTRAP), 0);
        // La caméra non adoptée ne compte pas.
        assert_eq!(value(&samples, "unifiprotect_cameras_total"), Some(4.0));
        assert_eq!(value(&samples, "unifiprotect_cameras_offline"), Some(1.0));
        assert_eq!(camera(&samples, "unifiprotect_camera_online", "Garage"), Some(0.0));
        assert_eq!(camera(&samples, "unifiprotect_camera_online", "Entree"), Some(1.0));
        assert_eq!(camera(&samples, "unifiprotect_camera_recording", "Entree"), Some(1.0));
        // « Jardin » est connectée sans enregistrer ; « Atelier » ne doit pas enregistrer.
        assert_eq!(value(&samples, "unifiprotect_cameras_not_recording"), Some(1.0));
        assert!(camera(&samples, "unifiprotect_camera_online", "Nouvelle").is_none());
    }

    #[test]
    fn le_stockage_et_les_disques_du_nvr() {
        let samples = samples(&json(BOOTSTRAP), 0);
        let used = value(&samples, "unifiprotect_storage_used_percent").unwrap();
        assert!((used - 84.2).abs() < 1e-6, "{used}");
        assert_eq!(value(&samples, "unifiprotect_storage_size_bytes"), Some(8e12));
        assert_eq!(value(&samples, "unifiprotect_disks_total"), Some(2.0));
        assert_eq!(value(&samples, "unifiprotect_disks_unhealthy"), Some(0.0));
        assert_eq!(value(&samples, "unifiprotect_recording_disabled"), Some(0.0));
        let info = samples.iter().find(|s| s.metric == "unifiprotect_version_info").unwrap();
        assert_eq!(info.labels["version"], "6.0.34");
    }

    #[test]
    fn un_disque_en_defaut_et_l_enregistrement_coupe_se_voient() {
        let samples = samples(&json(DISK_FAILED), 0);
        assert_eq!(value(&samples, "unifiprotect_disks_unhealthy"), Some(1.0));
        assert_eq!(value(&samples, "unifiprotect_recording_disabled"), Some(1.0));
        let used = value(&samples, "unifiprotect_storage_used_percent").unwrap();
        assert!((used - 99.0).abs() < 1e-6, "{used}");
        assert_eq!(value(&samples, "unifiprotect_cameras_not_recording"), Some(1.0));
    }

    #[test]
    fn une_amorce_vide_ne_donne_que_la_version() {
        let samples = samples(&json("{}"), 0);
        assert_eq!(value(&samples, "unifiprotect_cameras_total"), Some(0.0));
        assert!(value(&samples, "unifiprotect_storage_used_percent").is_none());
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn target(address: String, password: &str) -> Target {
        Target {
            id: 12,
            name: "nvr".into(),
            address,
            kind: "unifiprotect".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::new(),
            credential: Credential::UsernamePassword {
                username: "dumbmonit".into(),
                password: password.into(),
            },
            group_name: String::new(),
            position: 0,
        }
    }

    fn app(logins: Arc<AtomicUsize>) -> Router {
        Router::new()
            .route(
                "/api/auth/login",
                post(move |axum::Json(body): axum::Json<Value>| {
                    let logins = logins.clone();
                    async move {
                        if body["password"] != "secret" {
                            return (StatusCode::BAD_REQUEST, LOGIN_REFUSED).into_response();
                        }
                        logins.fetch_add(1, Ordering::SeqCst);
                        ([(header::SET_COOKIE, "TOKEN=jwt123; path=/; samesite=none; secure; httponly")], LOGIN_OK)
                            .into_response()
                    }
                }),
            )
            .route(
                "/proxy/protect/api/bootstrap",
                get(|headers: HeaderMap| async move {
                    let ok = headers.get(header::COOKIE).and_then(|v| v.to_str().ok())
                        == Some("TOKEN=jwt123");
                    if ok {
                        (StatusCode::OK, BOOTSTRAP)
                    } else {
                        (StatusCode::UNAUTHORIZED, r#"{"error":"Unauthorized"}"#)
                    }
                }),
            )
    }

    #[tokio::test]
    async fn la_session_est_reutilisee_d_une_sonde_a_l_autre() {
        let logins = Arc::new(AtomicUsize::new(0));
        let base = serve(app(logins.clone())).await;
        let collector = ProtectCollector::new();
        for _ in 0..3 {
            let samples = collector.probe(&target(base.clone(), "secret")).await.unwrap();
            assert_eq!(value(&samples, "unifiprotect_cameras_offline"), Some(1.0));
        }
        assert_eq!(logins.load(Ordering::SeqCst), 1, "une seule ouverture de session");
    }

    #[tokio::test]
    async fn un_mauvais_mot_de_passe_est_une_erreur_d_authentification() {
        let base = serve(app(Arc::new(AtomicUsize::new(0)))).await;
        let error = ProtectCollector::new().probe(&target(base, "mauvais")).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn un_jeton_est_refuse_et_un_nvr_eteint_est_une_panne() {
        let mut token = target("http://127.0.0.1:1".into(), "x");
        token.credential = Credential::ApiToken { token: "x".into() };
        let error = ProtectCollector::new().probe(&token).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
        let error = ProtectCollector::new()
            .probe(&target("http://127.0.0.1:1".into(), "secret"))
            .await
            .unwrap_err();
        assert!(error.means_down());
    }
}
