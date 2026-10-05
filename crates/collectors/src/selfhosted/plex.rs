//! Plex Media Server.
//!
//! Lu par l'API locale du serveur (port 32400), jamais par plex.tv. Un serveur
//! revendiqué (lié à un compte Plex) exige un jeton `X-Plex-Token` ; Plex ne
//! sait pas en émettre de restreint, celui du propriétaire ouvre tout. Un
//! serveur non revendiqué, ou dont le réseau de DumbMonit figure dans « List
//! of IP addresses and networks that are allowed without auth », répond sans
//! jeton.
//!
//! Tout est demandé en JSON (`Accept: application/json`), que Plex sert à
//! côté de son XML historique.
//!
//! Ce que Plex ne publie pas : l'issue des tâches de maintenance (le
//! « butler » ne donne que leur programmation) ni l'état d'une sauvegarde de
//! base. Le module s'en tient aux lectures en cours, au transcodeur, aux
//! bibliothèques et aux mises à jour.

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target, TargetId};
use serde_json::Value;

use super::client::{Auth, HttpClient};
use super::{flag, gauge, http_client, newer, now_ms, number, settle, token};

pub const DEFAULT_PORT: u16 = 32400;

fn container(body: &Value) -> &Value {
    body.get("MediaContainer").unwrap_or(&Value::Null)
}

fn list<'a>(body: &'a Value, key: &str) -> &'a [Value] {
    container(body).get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

// ------------------------------------------------------------------ sonde

pub async fn probe(
    client: &HttpClient,
    target_id: TargetId,
    ts_ms: i64,
) -> Result<Vec<Sample>, ProbeError> {
    // La racine exige le jeton sur un serveur revendiqué (`/identity`, non) :
    // c'est la preuve de vie et d'authentification.
    let root: Value = client.get_json("/").await?;
    let mut out = root_samples(&root, ts_ms);
    let mut errors = 0u32;
    let (identity, sessions, sections, updater, activities) = futures::join!(
        client.get_json::<Value>("/identity"),
        client.get_json::<Value>("/status/sessions"),
        client.get_json::<Value>("/library/sections"),
        client.get_json::<Value>("/updater/status"),
        client.get_json::<Value>("/activities"),
    );
    if let Some(identity) = settle(identity, &mut errors, target_id, "/identity")
        && let Some(claimed) = container(&identity).get("claimed").and_then(Value::as_bool)
    {
        out.push(flag("plex_claimed", claimed, ts_ms));
    }
    if let Some(sessions) = settle(sessions, &mut errors, target_id, "/status/sessions") {
        out.extend(session_samples(&sessions, ts_ms));
    }
    if let Some(sections) = settle(sections, &mut errors, target_id, "/library/sections") {
        out.extend(library_samples(&sections, ts_ms));
    }
    if let Some(updater) = settle(updater, &mut errors, target_id, "/updater/status") {
        let installed = container(&root).get("version").and_then(Value::as_str).unwrap_or("");
        out.extend(update_samples(&updater, installed, ts_ms));
    }
    if let Some(activities) = settle(activities, &mut errors, target_id, "/activities") {
        out.push(gauge("plex_activities", list(&activities, "Activity").len() as f64, ts_ms));
    }
    out.push(gauge("plex_scrape_errors", f64::from(errors), ts_ms));
    Ok(out)
}

// ------------------------------------------------------------ traduction

pub fn root_samples(root: &Value, ts_ms: i64) -> Vec<Sample> {
    let root = container(root);
    let mut out = Vec::new();
    if let Some(version) = root.get("version").and_then(Value::as_str) {
        let short = version.split('-').next().unwrap_or(version);
        out.push(gauge("plex_version_info", 1.0, ts_ms).with_label("version", short));
    }
    if let Some(active) = number(root.get("transcoderActiveVideoSessions")) {
        out.push(gauge("plex_transcoder_active_video_sessions", active, ts_ms));
    }
    out
}

/// Les lectures en cours. Une lecture transcode quand la vidéo ou le son est
/// réencodé (`videoDecision` / `audioDecision` à `transcode`) ; « copy » est
/// un simple remballage, léger.
pub fn session_samples(sessions: &Value, ts_ms: i64) -> Vec<Sample> {
    let items = list(sessions, "Metadata");
    let decision = |item: &Value, key: &str| {
        item.pointer(&format!("/TranscodeSession/{key}")).and_then(Value::as_str)
            == Some("transcode")
    };
    let video = items.iter().filter(|item| decision(item, "videoDecision")).count();
    let any = items
        .iter()
        .filter(|item| decision(item, "videoDecision") || decision(item, "audioDecision"))
        .count();
    let hardware = items
        .iter()
        .filter(|item| {
            let hw = |key: &str| {
                item.pointer(&format!("/TranscodeSession/{key}")).and_then(Value::as_bool)
                    == Some(true)
            };
            hw("transcodeHwFullPipeline") || hw("transcodeHwEncoding") || hw("transcodeHwDecoding")
        })
        .count();
    let remote = items
        .iter()
        .filter(|item| item.pointer("/Player/local").and_then(Value::as_bool) == Some(false))
        .count();
    let paused = items
        .iter()
        .filter(|item| item.pointer("/Player/state").and_then(Value::as_str) == Some("paused"))
        .count();
    let bandwidth: f64 =
        items.iter().filter_map(|item| number(item.pointer("/Session/bandwidth"))).sum();
    vec![
        gauge("plex_streams", items.len() as f64, ts_ms),
        gauge("plex_streams_paused", paused as f64, ts_ms),
        gauge("plex_streams_remote", remote as f64, ts_ms),
        gauge("plex_transcodes", any as f64, ts_ms),
        gauge("plex_transcodes_video", video as f64, ts_ms),
        gauge("plex_transcodes_hardware", hardware as f64, ts_ms),
        // Plex donne la bande passante réservée par lecture en kbit/s.
        gauge("plex_stream_bandwidth_bits_per_second", bandwidth * 1000.0, ts_ms),
    ]
}

pub fn library_samples(sections: &Value, ts_ms: i64) -> Vec<Sample> {
    let directories = list(sections, "Directory");
    let refreshing = directories
        .iter()
        .filter(|d| d.get("refreshing").and_then(Value::as_bool) == Some(true))
        .count();
    vec![
        gauge("plex_libraries", directories.len() as f64, ts_ms),
        gauge("plex_libraries_scanning", refreshing as f64, ts_ms),
    ]
}

/// `/updater/status` : `checkedAt` vaut -1 tant que Plex n'a jamais cherché
/// de mise à jour (serveur non revendiqué, mises à jour coupées) — rien à
/// dire alors. Une mise à jour trouvée arrive dans `Release`.
pub fn update_samples(updater: &Value, installed: &str, ts_ms: i64) -> Vec<Sample> {
    let status = container(updater);
    if number(status.get("checkedAt")).is_none_or(|checked| checked <= 0.0) {
        return Vec::new();
    }
    let latest = list(updater, "Release")
        .iter()
        .filter_map(|release| release.get("version").and_then(Value::as_str))
        .find(|version| newer(version, installed) == Some(true));
    let mut sample = flag("plex_update_available", latest.is_some(), ts_ms);
    if let Some(latest) = latest {
        sample = sample.with_label("latest_version", latest.split('-').next().unwrap_or(latest));
    }
    vec![sample]
}

// ------------------------------------------------------------ collecteur

#[derive(Default)]
pub struct PlexCollector;

impl PlexCollector {
    pub fn new() -> Self {
        Self
    }

    fn client(target: &Target) -> Result<HttpClient, ProbeError> {
        let mut auth = Auth::header("X-Plex-Product", "DumbMonit")
            .with_header("X-Plex-Client-Identifier", "dumbmonit")
            .with_header("X-Plex-Version", env!("CARGO_PKG_VERSION"));
        match (&target.credential, token(&target.credential)) {
            (_, Some(token)) => auth = auth.with_header("X-Plex-Token", token),
            (Credential::None, _) => {}
            (other, _) => {
                return Err(ProbeError::Config(format!(
                    "Plex expects an X-Plex-Token, or no credential for a server that allows this \
                     network without authentication; configured: {other}"
                )));
            }
        }
        let mut target = target.clone();
        let trimmed = target.address.trim().trim_end_matches('/');
        target.address = trimmed.strip_suffix("/web").unwrap_or(trimmed).to_string();
        http_client(&target, "http", DEFAULT_PORT, auth, "Plex Media Server")
    }
}

#[async_trait]
impl Collector for PlexCollector {
    fn kind(&self) -> &'static str {
        "plex"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        probe(&Self::client(target)?, target.id, now_ms()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let _: Value = Self::client(target)?.get_json("/").await?;
        Ok(Some("plex".to_string()))
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

    // Réponses réelles d'un Plex Media Server 1.43.4 (image officielle, non
    // revendiqué), en JSON. Un film de test y était lu en transcodage HLS
    // depuis un client web simulé. Les métadonnées que l'agent Plex a
    // associées au fichier, l'identifiant de machine, le nom du serveur et
    // les adresses ont été remplacés.
    const ROOT: &str = include_str!("testdata/plex_1.43.4/root.json");
    const IDENTITY: &str = include_str!("testdata/plex_1.43.4/identity.json");
    const SESSIONS: &str = include_str!("testdata/plex_1.43.4/sessions_transcode.json");
    const SECTIONS: &str = include_str!("testdata/plex_1.43.4/library_sections.json");
    const UPDATER: &str = include_str!("testdata/plex_1.43.4/updater_status.json");
    const UPDATER_CHECKED: &str = include_str!("testdata/plex_1.43.4/updater_status_checked.json");
    const ACTIVITIES: &str = include_str!("testdata/plex_1.43.4/activities.json");

    fn json(body: &str) -> Value {
        serde_json::from_str(body).unwrap()
    }

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    #[test]
    fn la_racine_donne_version_et_transcodeur() {
        let samples = root_samples(&json(ROOT), 0);
        let version = samples.iter().find(|s| s.metric == "plex_version_info").unwrap();
        assert_eq!(version.labels["version"], "1.43.4.10903");
        assert_eq!(value(&samples, "plex_transcoder_active_video_sessions"), Some(1.0));
    }

    #[test]
    fn une_lecture_transcodee_est_comptee() {
        let samples = session_samples(&json(SESSIONS), 0);
        assert_eq!(value(&samples, "plex_streams"), Some(1.0));
        assert_eq!(value(&samples, "plex_transcodes"), Some(1.0));
        assert_eq!(value(&samples, "plex_transcodes_video"), Some(1.0));
        assert_eq!(value(&samples, "plex_transcodes_hardware"), Some(0.0));
        assert_eq!(value(&samples, "plex_streams_remote"), Some(0.0));
        assert_eq!(value(&samples, "plex_stream_bandwidth_bits_per_second"), Some(414_000.0));
        let empty = session_samples(&json(r#"{"MediaContainer":{"size":0}}"#), 0);
        assert_eq!(value(&empty, "plex_streams"), Some(0.0));
    }

    #[test]
    fn les_bibliotheques() {
        let samples = library_samples(&json(SECTIONS), 0);
        assert_eq!(value(&samples, "plex_libraries"), Some(1.0));
        assert_eq!(value(&samples, "plex_libraries_scanning"), Some(0.0));
    }

    #[test]
    fn la_mise_a_jour_n_est_dite_qu_apres_une_verification() {
        let installed = "1.43.4.10903-e5521bd8c";
        assert!(update_samples(&json(UPDATER), installed, 0).is_empty(), "jamais vérifié");
        let samples = update_samples(&json(UPDATER_CHECKED), installed, 0);
        assert_eq!(value(&samples, "plex_update_available"), Some(0.0));
        // Forme documentée d'une mise à jour trouvée (élément `Release`).
        let found = json(
            r#"{"MediaContainer":{"size":1,"canInstall":true,"checkedAt":1790752652,
                "downloadURL":"https://plex.tv/downloads/latest/5","status":0,
                "Release":[{"key":"https://plex.tv/updater/releases/1","version":"1.43.5.10950-0a1b2c3d4",
                "added":"New things","fixed":"Old things","downloadURL":"https://plex.tv/x","state":"notify"}]}}"#,
        );
        let samples = update_samples(&found, installed, 0);
        assert_eq!(value(&samples, "plex_update_available"), Some(1.0));
        assert_eq!(samples[0].labels["latest_version"], "1.43.5.10950");
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn target(address: String, credential: Credential) -> Target {
        Target {
            id: 13,
            name: "plex".into(),
            address,
            kind: "plex".into(),
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
    async fn un_serveur_revendique_exige_le_jeton() {
        let app = Router::new()
            .route(
                "/",
                get(|headers: HeaderMap| async move {
                    assert_eq!(
                        headers.get("accept").and_then(|v| v.to_str().ok()),
                        Some("application/json")
                    );
                    if headers.get("x-plex-token").and_then(|v| v.to_str().ok()) != Some("tok") {
                        return (
                            StatusCode::UNAUTHORIZED,
                            "<html><title>Unauthorized</title></html>",
                        );
                    }
                    (StatusCode::OK, ROOT)
                }),
            )
            .route("/identity", get(|| async { IDENTITY }))
            .route("/status/sessions", get(|| async { SESSIONS }))
            .route("/library/sections", get(|| async { SECTIONS }))
            .route("/updater/status", get(|| async { UPDATER_CHECKED }))
            .route("/activities", get(|| async { ACTIVITIES }));
        let base = serve(app).await;
        let samples = PlexCollector::new()
            .probe(&target(base.clone(), Credential::ApiToken { token: "tok".into() }))
            .await
            .unwrap();
        assert_eq!(value(&samples, "plex_transcodes"), Some(1.0));
        assert_eq!(value(&samples, "plex_claimed"), Some(0.0));
        assert_eq!(value(&samples, "plex_update_available"), Some(0.0));
        assert_eq!(value(&samples, "plex_activities"), Some(0.0));
        assert_eq!(value(&samples, "plex_scrape_errors"), Some(0.0));

        let error = PlexCollector::new().probe(&target(base, Credential::None)).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
    }

    #[test]
    fn un_identifiant_inattendu_est_une_erreur_de_configuration() {
        let login = Credential::UsernamePassword { username: "u".into(), password: "p".into() };
        assert!(matches!(
            PlexCollector::client(&target("plex.lan".into(), login)),
            Err(ProbeError::Config(_))
        ));
    }
}
