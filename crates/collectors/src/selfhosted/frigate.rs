//! Frigate NVR.
//!
//! Frigate publie tout ce qu'il sait de lui dans `GET /api/stats` : le débit
//! de chaque caméra (`camera_fps` est ce que ffmpeg reçoit, `process_fps` ce
//! que Frigate traite, `skipped_fps` ce qu'il a dû jeter faute de temps), la
//! vitesse de chaque détecteur (Coral, GPU, CPU), le stockage des
//! enregistrements, la version et la dernière version connue.
//!
//! La panne que ce module existe pour voir : **une caméra qui ne livre plus
//! d'images**. Frigate ne la signale nulle part ailleurs que dans son
//! interface ; l'enregistrement s'arrête sans bruit. `camera_fps` à zéro est
//! le signe le plus fiable. Un détecteur bloqué (`detection_start` ancien)
//! ou un disque d'enregistrement plein sont les deux autres.
//!
//! Le port 5000 (API interne, sans authentification) est celui à utiliser sur
//! un réseau de confiance. Le port 8971 exige une session : derrière un proxy
//! qui attend un jeton porteur, choisir « Bearer token ».

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use serde_json::Value;

use super::client::{Auth, HttpClient};
use super::{MAX_NAMED, flag, gauge, http_client, metric, newer, now_ms, number};

pub const DEFAULT_PORT: u16 = 5000;
const PRODUCT: &str = "Frigate";
const P: &str = "frigate";

// ------------------------------------------------------------ traduction

/// `GET /api/stats` en métriques. `now_s` sert à dater un détecteur resté
/// bloqué sur la même inférence.
pub fn samples(stats: &Value, now_s: i64, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let service = stats.get("service");
    let version = service.and_then(|s| s.get("version")).and_then(Value::as_str);
    out.push(
        gauge(&metric(P, "version_info"), 1.0, ts_ms).with_label("version", version.unwrap_or("unknown")),
    );
    if let (Some(current), Some(latest)) =
        (version, service.and_then(|s| s.get("latest_version")).and_then(Value::as_str))
        && let Some(available) = newer(latest, current)
    {
        out.push(
            flag(&metric(P, "update_available"), available, ts_ms)
                .with_label("latest_version", latest),
        );
    }
    if let Some(uptime) = service.and_then(|s| number(s.get("uptime"))) {
        out.push(gauge(&metric(P, "uptime_seconds"), uptime, ts_ms));
    }
    if let Some(fps) = number(stats.get("detection_fps")) {
        out.push(gauge(&metric(P, "detection_fps"), fps, ts_ms));
    }

    let empty = serde_json::Map::new();
    let cameras = stats.get("cameras").and_then(Value::as_object).unwrap_or(&empty);
    let mut offline = 0usize;
    for (index, (name, camera)) in cameras.iter().enumerate() {
        let fps = number(camera.get("camera_fps")).unwrap_or(0.0);
        if fps <= 0.0 {
            offline += 1;
        }
        if index >= MAX_NAMED {
            continue;
        }
        out.push(flag(&metric(P, "camera_online"), fps > 0.0, ts_ms).with_label("camera", name));
        out.push(gauge(&metric(P, "camera_fps"), fps, ts_ms).with_label("camera", name));
        for (metric_name, key) in [
            ("camera_process_fps", "process_fps"),
            ("camera_skipped_fps", "skipped_fps"),
            ("camera_detection_fps", "detection_fps"),
        ] {
            if let Some(value) = number(camera.get(key)) {
                out.push(gauge(&metric(P, metric_name), value, ts_ms).with_label("camera", name));
            }
        }
    }
    out.push(gauge(&metric(P, "cameras_total"), cameras.len() as f64, ts_ms));
    out.push(gauge(&metric(P, "cameras_offline"), offline as f64, ts_ms));

    let detectors = stats.get("detectors").and_then(Value::as_object).unwrap_or(&empty);
    for (name, detector) in detectors.iter().take(MAX_NAMED) {
        if let Some(speed) = number(detector.get("inference_speed")) {
            out.push(
                gauge(&metric(P, "detector_inference_ms"), speed, ts_ms)
                    .with_label("detector", name),
            );
        }
        // `detection_start` vaut 0 entre deux inférences, l'heure de début
        // pendant l'une d'elles : une valeur ancienne est un détecteur figé.
        let stuck = match number(detector.get("detection_start")) {
            Some(start) if start > 0.0 => (now_s as f64 - start).max(0.0),
            _ => 0.0,
        };
        out.push(gauge(&metric(P, "detector_busy_seconds"), stuck, ts_ms).with_label("detector", name));
    }

    // Les systèmes de fichiers en mémoire (`/dev/shm`, `/tmp/cache`) sont pleins
    // par construction : seuls les disques comptent.
    let storage = service.and_then(|s| s.get("storage")).and_then(Value::as_object);
    for (path, disk) in storage.unwrap_or(&empty).iter().take(MAX_NAMED) {
        if disk.get("mount_type").and_then(Value::as_str) == Some("tmpfs") {
            continue;
        }
        let (Some(total), Some(free)) = (number(disk.get("total")), number(disk.get("free")))
        else {
            continue;
        };
        if total <= 0.0 {
            continue;
        }
        let used = (100.0 * (1.0 - free / total)).clamp(0.0, 100.0);
        out.push(gauge(&metric(P, "storage_used_percent"), used, ts_ms).with_label("path", path));
        out.push(gauge(&metric(P, "storage_free_bytes"), free, ts_ms).with_label("path", path));
    }
    out
}

// ------------------------------------------------------------------ sonde

pub async fn probe(client: &HttpClient, ts_ms: i64) -> Result<Vec<Sample>, ProbeError> {
    // Un seul appel : s'il répond, tout est là.
    let stats: Value = client.get_json("/api/stats").await?;
    Ok(samples(&stats, ts_ms / 1000, ts_ms))
}

// ------------------------------------------------------------ collecteur

#[derive(Default)]
pub struct FrigateCollector;

impl FrigateCollector {
    pub fn new() -> Self {
        Self
    }

    fn client(target: &Target) -> Result<HttpClient, ProbeError> {
        let auth = match &target.credential {
            Credential::None => Auth::default(),
            Credential::ApiToken { token } if !token.trim().is_empty() => {
                Auth::header("Authorization", format!("Bearer {}", token.trim()))
            }
            other => {
                return Err(ProbeError::Config(format!(
                    "Frigate expects no credentials (internal port) or a bearer token, \
                     configured: {other}"
                )));
            }
        };
        http_client(target, "http", DEFAULT_PORT, auth, PRODUCT)
    }
}

#[async_trait]
impl Collector for FrigateCollector {
    fn kind(&self) -> &str {
        "frigate"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        probe(&Self::client(target)?, now_ms()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let client = Self::client(target)?;
        let (status, body) = client.get("/api/version").await?;
        if !status.is_success() {
            return Err(client.status_error(status, &body, "/api/version"));
        }
        Ok(Some("frigate".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::get;

    use super::*;

    // `/api/stats` d'un Frigate 0.15.1 à trois caméras dont une (« jardin »)
    // ne livre plus d'images, un Coral sain, un disque d'enregistrement à
    // 78 % et un montage en mémoire ; puis le même serveur avec un détecteur
    // figé sur une inférence et un disque plein.
    const STATS: &str = include_str!("testdata/frigate_0.15.1/stats.json");
    const STALLED: &str = include_str!("testdata/frigate_0.15.1/stats_stalled_detector.json");

    fn json(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn with(samples: &[Sample], name: &str, key: &str, wanted: &str) -> Option<f64> {
        samples
            .iter()
            .find(|s| s.metric == name && s.labels.get(key).map(String::as_str) == Some(wanted))
            .map(|s| s.value)
    }

    #[test]
    fn une_camera_sans_images_est_hors_ligne() {
        let samples = samples(&json(STATS), 1_790_800_000, 0);
        assert_eq!(value(&samples, "frigate_cameras_total"), Some(3.0));
        assert_eq!(value(&samples, "frigate_cameras_offline"), Some(1.0));
        assert_eq!(with(&samples, "frigate_camera_online", "camera", "jardin"), Some(0.0));
        assert_eq!(with(&samples, "frigate_camera_online", "camera", "garage"), Some(1.0));
        assert_eq!(with(&samples, "frigate_camera_fps", "camera", "entree"), Some(10.1));
        assert_eq!(with(&samples, "frigate_camera_skipped_fps", "camera", "entree"), Some(0.3));
    }

    #[test]
    fn le_detecteur_sain_n_est_pas_occupe_et_le_fige_l_est() {
        let healthy = samples(&json(STATS), 1_790_800_000, 0);
        assert_eq!(with(&healthy, "frigate_detector_inference_ms", "detector", "coral"), Some(8.12));
        assert_eq!(with(&healthy, "frigate_detector_busy_seconds", "detector", "coral"), Some(0.0));
        let stalled = samples(&json(STALLED), 1_790_799_100, 0);
        assert_eq!(
            with(&stalled, "frigate_detector_busy_seconds", "detector", "coral"),
            Some(99.5)
        );
        assert_eq!(with(&stalled, "frigate_detector_inference_ms", "detector", "coral"), Some(850.3));
    }

    #[test]
    fn le_stockage_ignore_les_montages_en_memoire() {
        let samples = samples(&json(STATS), 0, 0);
        let used = with(&samples, "frigate_storage_used_percent", "path", "/media/frigate/recordings")
            .unwrap();
        assert!((used - 78.5).abs() < 1e-6, "{used}");
        assert!(with(&samples, "frigate_storage_used_percent", "path", "/tmp/cache").is_none());
        let full = super::samples(&json(STALLED), 0, 0);
        assert_eq!(
            with(&full, "frigate_storage_used_percent", "path", "/media/frigate/recordings"),
            Some(100.0)
        );
    }

    #[test]
    fn la_mise_a_jour_se_compare_a_la_version_installee() {
        let samples = samples(&json(STATS), 0, 0);
        assert_eq!(value(&samples, "frigate_update_available"), Some(1.0));
        let info = samples.iter().find(|s| s.metric == "frigate_version_info").unwrap();
        assert_eq!(info.labels["version"], "0.15.1-e59d2e1");
        let current = super::samples(&json(STALLED), 0, 0);
        assert_eq!(value(&current, "frigate_update_available"), Some(0.0));
        assert_eq!(value(&current, "frigate_uptime_seconds"), Some(100.0));
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn target(address: String, credential: Credential) -> Target {
        Target {
            id: 4,
            name: "nvr".into(),
            address,
            kind: "frigate".into(),
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

    #[tokio::test]
    async fn le_jeton_porteur_est_envoye_et_un_refus_est_une_erreur_d_authentification() {
        let app = Router::new().route(
            "/api/stats",
            get(|headers: HeaderMap| async move {
                let ok = headers.get("authorization").and_then(|v| v.to_str().ok())
                    == Some("Bearer jeton");
                if ok {
                    (StatusCode::OK, STATS)
                } else {
                    (StatusCode::UNAUTHORIZED, r#"{"message":"Unauthorized"}"#)
                }
            }),
        );
        let base = serve(app).await;
        let good = Credential::ApiToken { token: "jeton".into() };
        let samples = FrigateCollector::new().probe(&target(base.clone(), good)).await.unwrap();
        assert_eq!(value(&samples, "frigate_cameras_offline"), Some(1.0));
        let error =
            FrigateCollector::new().probe(&target(base, Credential::None)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
    }

    #[tokio::test]
    async fn sans_authentification_et_injoignable() {
        let app = Router::new().route("/api/stats", get(|| async { STATS }));
        let base = serve(app).await;
        let samples =
            FrigateCollector::new().probe(&target(base, Credential::None)).await.unwrap();
        assert_eq!(value(&samples, "frigate_cameras_total"), Some(3.0));
        let error = FrigateCollector::new()
            .probe(&target("http://127.0.0.1:1".into(), Credential::None))
            .await
            .unwrap_err();
        assert!(error.means_down());
        let bad = Credential::UsernamePassword { username: "a".into(), password: "b".into() };
        let error = FrigateCollector::new()
            .probe(&target("http://127.0.0.1:1".into(), bad))
            .await
            .unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
    }
}
