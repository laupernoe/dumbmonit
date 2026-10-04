//! Immich.
//!
//! Tout passe par l'API, avec une clé restreinte à huit permissions de
//! lecture (`server.about`, `server.versionCheck`, `server.storage`,
//! `server.statistics`, `queue.read`, `session.read`, `asset.read`,
//! `user.read`). Immich réserve `server.statistics` et `queue.read` aux
//! comptes qui ouvrent l'administration : la clé doit être créée par l'un
//! d'eux, mais elle ne peut rien faire d'autre que lire ces pages. Une clé
//! d'un compte ordinaire fonctionne aussi ; photos et files de travaux sont
//! alors sautées sans erreur, les appareils restent lus.
//!
//! La panne que ce module existe pour voir : **des travaux qui n'avancent
//! plus**. Une file mise en pause et oubliée, ou un service de travaux arrêté,
//! et les photos téléversées n'ont plus ni miniature, ni métadonnées, ni
//! reconnaissance — sans rien d'autre qu'un compteur dans l'administration.
//!
//! Immich ≥ 2 expose les files sous `/api/queues` ; les versions antérieures
//! sous `/api/jobs` (permission `job.read`), lu en repli.
//!
//! # Appareils (`session.read`, `asset.read`, `user.read`)
//!
//! `GET /api/sessions` renvoie les connexions de **l'unique compte qui a créé
//! la clé** : pas une liste d'utilisateurs, mais la liste de ses téléphones,
//! tablettes et navigateurs — exactement l'usage courant d'un homelab, un
//! compte partagé entre les appareils d'un foyer. `updatedAt` vaut la
//! dernière activité de la session, prise ici comme dernière connexion ;
//! l'appareil n'a ni nom ni identifiant stable au-delà de son `id` de
//! session, d'où l'étiquette composée dans [`session_device`].
//!
//! `last_backup`, en revanche, **n'est pas attribuable à un appareil** :
//! Immich a retiré `deviceId` des filtres de `/api/search/metadata` (vérifié
//! sur la spécification OpenAPI du dépôt `immich-app/immich`, tag `v3.2.4` —
//! le champ n'existe tout simplement plus). La seule information disponible
//! est la date de création du dernier asset de toute la bibliothèque, tous
//! appareils confondus. Lui donner le nom d'un appareil précis ferait croire
//! qu'un téléphone a bien sauvegardé alors que c'est un autre appareil du
//! même compte qui l'a fait — l'inverse du but de cette fonctionnalité. Elle
//! est donc publiée comme un appareil à part, [`LIBRARY_DEVICE`], et la
//! notice d'installation le dit explicitement.

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target, TargetId};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::Value;

use crate::client_devices::{self, Device};

use super::client::{Auth, HttpClient};
use super::{MAX_NAMED, flag, gauge, http_client, newer, now_ms, number, token};

pub const DEFAULT_PORT: u16 = 2283;

/// Nom donné à l'appareil synthétique qui porte la fraîcheur de sauvegarde de
/// toute la bibliothèque, faute de pouvoir l'attribuer à un appareil réel
/// (voir le commentaire de module).
pub const LIBRARY_DEVICE: &str = "library";

#[derive(Debug, Deserialize)]
pub struct About {
    pub version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionCheck {
    #[serde(default)]
    pub release_version: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Statistics {
    #[serde(default)]
    pub photos: f64,
    #[serde(default)]
    pub videos: f64,
    #[serde(default)]
    pub usage: f64,
    #[serde(default)]
    pub usage_by_user: Vec<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Storage {
    #[serde(default)]
    pub disk_available_raw: Option<f64>,
    #[serde(default)]
    pub disk_size_raw: Option<f64>,
    #[serde(default)]
    pub disk_usage_percentage: Option<f64>,
}

/// `GET /api/sessions` : une connexion, c'est-à-dire un appareil du compte
/// qui a créé la clé.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub device_type: String,
    // `camelCase` donnerait « deviceOs » : Immich écrit « deviceOS », sigle
    // entièrement majuscule.
    #[serde(rename = "deviceOS")]
    pub device_os: String,
    /// Date de dernière activité de la session : la meilleure approximation
    /// d'une dernière connexion que l'API offre.
    pub updated_at: String,
}

/// `GET /api/users/me` : juste assez pour nommer le compte propriétaire des
/// appareils ci-dessus.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserMe {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
}

impl UserMe {
    fn display_name(&self) -> &str {
        if !self.name.trim().is_empty() { &self.name } else { &self.email }
    }
}

/// `POST /api/search/metadata`, réponse réduite à ce que la sonde utilise :
/// l'asset le plus récent de la bibliothèque, tous appareils confondus.
#[derive(Debug, Deserialize)]
pub struct SearchMetadata {
    pub assets: AssetPage,
}

#[derive(Debug, Deserialize, Default)]
pub struct AssetPage {
    #[serde(default)]
    pub items: Vec<AssetSummary>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetSummary {
    /// Date d'enregistrement côté serveur — celle du téléversement, à
    /// distinguer de `fileCreatedAt` (date EXIF, fixée par l'appareil photo
    /// et pas forcément proche du moment où Immich a reçu le fichier).
    pub created_at: String,
}

/// Une file de travaux, lue dans l'une ou l'autre forme de l'API.
#[derive(Debug, Clone, PartialEq)]
pub struct Queue {
    pub name: String,
    pub paused: bool,
    pub active: f64,
    /// En attente, différés, ou retenus par la pause.
    pub waiting: f64,
    pub failed: f64,
}

fn counts(name: &str, paused: bool, stats: &Value) -> Queue {
    let get = |key: &str| number(stats.get(key)).unwrap_or(0.0);
    Queue {
        name: name.to_string(),
        paused,
        active: get("active"),
        waiting: get("waiting") + get("delayed") + get("paused"),
        failed: get("failed"),
    }
}

/// `/api/queues` : `[{"name", "isPaused", "statistics": {…}}]`.
pub fn parse_queues(body: &Value) -> Vec<Queue> {
    body.as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(|queue| {
            let name = queue.get("name")?.as_str()?;
            let paused = queue.get("isPaused").and_then(Value::as_bool).unwrap_or(false);
            Some(counts(name, paused, queue.get("statistics")?))
        })
        .collect()
}

/// `/api/jobs` : `{"<nom>": {"queueStatus": {"isPaused"}, "jobCounts": {…}}}`.
pub fn parse_jobs(body: &Value) -> Vec<Queue> {
    body.as_object()
        .map(|map| {
            map.iter()
                .filter_map(|(name, queue)| {
                    let paused = queue
                        .pointer("/queueStatus/isPaused")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    Some(counts(name, paused, queue.get("jobCounts")?))
                })
                .collect()
        })
        .unwrap_or_default()
}

// ------------------------------------------------------------------ sonde

pub async fn probe(
    client: &HttpClient,
    target: &Target,
    ts_ms: i64,
) -> Result<Vec<Sample>, ProbeError> {
    let target_id: TargetId = target.id;
    let stale_days = client_devices::stale_days(target)?;
    // La preuve de vie et d'authentification : le seul appel qui condamne.
    let about: About = client.get_json("/api/server/about").await?;
    let mut errors = 0u32;
    let (check, storage, statistics, queues, config, sessions, user, last_backup) = futures::join!(
        client.get_json::<VersionCheck>("/api/server/version-check"),
        client.get_json::<Storage>("/api/server/storage"),
        optional::<Statistics>(client, "/api/server/statistics"),
        queues(client),
        client.get_json::<Value>("/api/server/config"),
        optional::<Vec<Session>>(client, "/api/sessions"),
        optional::<UserMe>(client, "/api/users/me"),
        last_asset_created_at(client),
    );

    let mut out =
        vec![gauge("immich_version_info", 1.0, ts_ms).with_label("version", &about.version)];
    let mut note = |outcome: Result<(), ProbeError>, path: &str| {
        if let Err(error) = outcome {
            errors += 1;
            tracing::warn!(target_id, path, %error, "appel Immich indisponible");
        }
    };
    note(check.map(|check| out.extend(update_samples(&about, &check, ts_ms))), "version-check");
    note(storage.map(|storage| out.extend(storage_samples(&storage, ts_ms))), "storage");
    note(
        statistics.map(|stats| {
            out.push(flag("immich_statistics_readable", stats.is_some(), ts_ms));
            out.extend(stats.iter().flat_map(|stats| statistics_samples(stats, ts_ms)));
        }),
        "statistics",
    );
    note(
        queues
            .map(|queues| out.extend(queues.map(|q| queue_samples(&q, ts_ms)).unwrap_or_default())),
        "queues",
    );
    note(
        config.map(|config| {
            if let Some(maintenance) = config.get("maintenanceMode").and_then(Value::as_bool) {
                out.push(flag("immich_maintenance_mode", maintenance, ts_ms));
            }
        }),
        "config",
    );

    // Appareils : voir le commentaire de module pour les limites de l'API.
    let mut user_name = String::new();
    note(
        user.map(|user| {
            if let Some(user) = user {
                user_name = user.display_name().to_string();
            }
        }),
        "users/me",
    );
    let mut devices: Vec<Device> = Vec::new();
    note(
        sessions.map(|sessions| {
            if let Some(sessions) = sessions {
                devices.extend(sessions.iter().filter_map(|s| session_device(s, &user_name)));
            }
        }),
        "sessions",
    );
    note(
        last_backup.map(|instant| {
            if let Some(instant) = instant {
                devices.push(Device {
                    name: LIBRARY_DEVICE.to_string(),
                    device_type: "Library".to_string(),
                    os: String::new(),
                    user: user_name.clone(),
                    last_seen: None,
                    last_backup: Some(instant),
                });
            }
        }),
        "search/metadata",
    );
    out.extend(client_devices::samples("immich", &devices, stale_days, ts_ms));

    out.push(gauge("immich_scrape_errors", f64::from(errors), ts_ms));
    Ok(out)
}

/// Un appareil Immich n'a ni nom, ni identifiant stable au-delà de celui de
/// la session : on compose un identifiant lisible, « Mobile-3fa85f64 », en
/// reprenant les huit premiers caractères de l'id de session.
fn session_device(session: &Session, user: &str) -> Option<Device> {
    let last_seen = parse_instant(&session.updated_at)?;
    let short: String = session.id.chars().filter(|c| *c != '-').take(8).collect();
    let device_type = non_empty(&session.device_type);
    Some(Device {
        name: format!("{device_type}-{short}"),
        device_type,
        os: non_empty(&session.device_os),
        user: user.to_string(),
        last_seen: Some(last_seen),
        last_backup: None,
    })
}

fn non_empty(value: &str) -> String {
    if value.trim().is_empty() { "Unknown".to_string() } else { value.to_string() }
}

/// Date ISO 8601 (telle qu'Immich la renvoie) convertie en secondes Unix.
fn parse_instant(value: &str) -> Option<i64> {
    value.parse::<chrono::DateTime<chrono::Utc>>().ok().map(|dt| dt.timestamp())
}

/// Le dernier asset créé sur le serveur, tous appareils confondus : voir le
/// commentaire de module sur l'absence de `deviceId` dans l'API de
/// recherche. `order` est dépréciée depuis Immich v3.2.0 mais toujours
/// servie en v3.2.4 ; c'est le seul tri qui porte sur la date d'arrivée sur
/// le serveur (`createdAt`) plutôt que sur une donnée EXIF que l'appareil
/// contrôle lui-même (`fileCreatedAt`).
async fn last_asset_created_at(client: &HttpClient) -> Result<Option<i64>, ProbeError> {
    let body = serde_json::json!({ "size": 1, "order": "desc" });
    let page: Option<SearchMetadata> = optional_post(client, "/api/search/metadata", &body).await?;
    Ok(page
        .and_then(|page| page.assets.items.into_iter().next())
        .and_then(|asset| parse_instant(&asset.created_at)))
}

/// Une page que la clé peut ne pas avoir le droit de lire : un 403 n'est ni
/// une panne ni une erreur, seulement une clé d'un compte ordinaire.
async fn optional<T: serde::de::DeserializeOwned>(
    client: &HttpClient,
    path: &str,
) -> Result<Option<T>, ProbeError> {
    let (status, body) = client.get(path).await?;
    if status == StatusCode::FORBIDDEN {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(client.status_error(status, &body, path));
    }
    client.decode(&body, path).map(Some)
}

/// Même tolérance que [`optional`], pour un appel `POST` (la recherche
/// d'Immich n'a pas de forme `GET`).
async fn optional_post<T: serde::de::DeserializeOwned>(
    client: &HttpClient,
    path: &str,
    body: &impl serde::Serialize,
) -> Result<Option<T>, ProbeError> {
    let (status, response) = client.post(path, body).await?;
    if status == StatusCode::FORBIDDEN {
        return Ok(None);
    }
    if !status.is_success() {
        return Err(client.status_error(status, &response, path));
    }
    client.decode(&response, path).map(Some)
}

/// `/api/queues`, puis `/api/jobs` pour un Immich qui ne le connaît pas encore.
async fn queues(client: &HttpClient) -> Result<Option<Vec<Queue>>, ProbeError> {
    let (status, body) = client.get("/api/queues").await?;
    if status.is_success() {
        return Ok(Some(parse_queues(&client.decode(&body, "/api/queues")?)));
    }
    if status == StatusCode::FORBIDDEN {
        return Ok(None);
    }
    if status != StatusCode::NOT_FOUND {
        return Err(client.status_error(status, &body, "/api/queues"));
    }
    Ok(optional::<Value>(client, "/api/jobs").await?.map(|jobs| parse_jobs(&jobs)))
}

// ------------------------------------------------------------ traduction

pub fn update_samples(about: &About, check: &VersionCheck, ts_ms: i64) -> Vec<Sample> {
    let Some(latest) = check.release_version.as_deref().filter(|v| !v.is_empty()) else {
        // Vérification des versions coupée dans les réglages : rien à dire.
        return Vec::new();
    };
    match newer(latest, &about.version) {
        Some(available) => vec![
            flag("immich_update_available", available, ts_ms).with_label("latest_version", latest),
        ],
        None => Vec::new(),
    }
}

pub fn storage_samples(storage: &Storage, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    if let Some(percent) = storage.disk_usage_percentage {
        out.push(gauge("immich_storage_used_percent", percent, ts_ms));
    }
    if let Some(available) = storage.disk_available_raw {
        out.push(gauge("immich_storage_available_bytes", available, ts_ms));
    }
    if let Some(size) = storage.disk_size_raw {
        out.push(gauge("immich_storage_size_bytes", size, ts_ms));
    }
    out
}

pub fn statistics_samples(stats: &Statistics, ts_ms: i64) -> Vec<Sample> {
    vec![
        gauge("immich_photos", stats.photos, ts_ms),
        gauge("immich_videos", stats.videos, ts_ms),
        gauge("immich_usage_bytes", stats.usage, ts_ms),
        gauge("immich_users", stats.usage_by_user.len() as f64, ts_ms),
    ]
}

pub fn queue_samples(queues: &[Queue], ts_ms: i64) -> Vec<Sample> {
    let sum = |f: fn(&Queue) -> f64| queues.iter().map(f).sum::<f64>();
    let mut out = vec![
        gauge("immich_jobs_waiting", sum(|q| q.waiting), ts_ms),
        gauge("immich_jobs_active", sum(|q| q.active), ts_ms),
        gauge("immich_jobs_failed", sum(|q| q.failed), ts_ms),
        gauge("immich_queues_paused", queues.iter().filter(|q| q.paused).count() as f64, ts_ms),
    ];
    for queue in queues.iter().take(MAX_NAMED) {
        let name = queue.name.as_str();
        out.push(gauge("immich_queue_waiting", queue.waiting, ts_ms).with_label("queue", name));
        out.push(gauge("immich_queue_active", queue.active, ts_ms).with_label("queue", name));
        out.push(gauge("immich_queue_failed", queue.failed, ts_ms).with_label("queue", name));
        out.push(flag("immich_queue_paused", queue.paused, ts_ms).with_label("queue", name));
    }
    out
}

// ------------------------------------------------------------ collecteur

#[derive(Default)]
pub struct ImmichCollector;

impl ImmichCollector {
    pub fn new() -> Self {
        Self
    }

    fn client(target: &Target) -> Result<HttpClient, ProbeError> {
        let Some(key) = token(&target.credential) else {
            return Err(ProbeError::Config(format!(
                "Immich expects an API key, configured: {}",
                target.credential
            )));
        };
        let mut target = target.clone();
        let trimmed = target.address.trim().trim_end_matches('/');
        target.address = trimmed.strip_suffix("/api").unwrap_or(trimmed).to_string();
        http_client(&target, "http", DEFAULT_PORT, Auth::header("x-api-key", key), "Immich")
    }
}

#[async_trait]
impl Collector for ImmichCollector {
    fn kind(&self) -> &'static str {
        "immich"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        probe(&Self::client(target)?, target, now_ms()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let _: About = Self::client(target)?.get_json("/api/server/about").await?;
        Ok(Some("immich".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use axum::Router;
    use axum::http::HeaderMap;
    use axum::routing::get;
    use dumbmonit_proto::Credential;

    use super::*;

    // Réponses réelles d'un Immich v3.2.4 (apprentissage automatique coupé),
    // lues avec une clé restreinte aux cinq permissions ci-dessus. La file des
    // miniatures a été mise en pause avant de téléverser deux photos : elles
    // attendent. `/api/jobs` est l'ancienne route, lue sur la même instance.
    // Les identifiants d'utilisateur ont été remplacés.
    const ABOUT: &str = include_str!("testdata/immich_3.2.4/about.json");
    const VERSION_CHECK: &str = include_str!("testdata/immich_3.2.4/version_check.json");
    const STORAGE: &str = include_str!("testdata/immich_3.2.4/storage.json");
    const STATISTICS: &str = include_str!("testdata/immich_3.2.4/statistics.json");
    const QUEUES: &str = include_str!("testdata/immich_3.2.4/queues.json");
    const JOBS: &str = include_str!("testdata/immich_3.2.4/jobs.json");
    const CONFIG: &str = include_str!("testdata/immich_3.2.4/config.json");
    const SESSIONS: &str = include_str!("testdata/immich_3.2.4/sessions.json");
    const USERS_ME: &str = include_str!("testdata/immich_3.2.4/users_me.json");
    const SEARCH_METADATA: &str = include_str!("testdata/immich_3.2.4/search_metadata.json");

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn device_value(samples: &[Sample], name: &str, device: &str) -> Option<f64> {
        samples
            .iter()
            .find(|s| {
                s.metric == name && s.labels.get("device").map(String::as_str) == Some(device)
            })
            .map(|s| s.value)
    }

    fn labelled(samples: &[Sample], name: &str, queue: &str) -> Option<f64> {
        samples
            .iter()
            .find(|s| s.metric == name && s.labels.get("queue").map(String::as_str) == Some(queue))
            .map(|s| s.value)
    }

    #[test]
    fn les_deux_formes_des_files_donnent_la_meme_chose() {
        let queues = parse_queues(&serde_json::from_str(QUEUES).unwrap());
        let mut jobs = parse_jobs(&serde_json::from_str(JOBS).unwrap());
        let mut sorted = queues.clone();
        sorted.sort_by(|a, b| a.name.cmp(&b.name));
        jobs.sort_by(|a, b| a.name.cmp(&b.name));
        assert_eq!(sorted, jobs);
        let thumbnails = queues.iter().find(|q| q.name == "thumbnailGeneration").unwrap();
        assert!(thumbnails.paused);
        assert_eq!(thumbnails.waiting, 2.0, "les travaux retenus par la pause attendent");
    }

    #[test]
    fn une_file_en_pause_se_voit() {
        let queues = parse_queues(&serde_json::from_str(QUEUES).unwrap());
        let samples = queue_samples(&queues, 0);
        assert_eq!(value(&samples, "immich_queues_paused"), Some(1.0));
        assert_eq!(value(&samples, "immich_jobs_waiting"), Some(2.0));
        assert_eq!(labelled(&samples, "immich_queue_paused", "thumbnailGeneration"), Some(1.0));
        assert_eq!(labelled(&samples, "immich_queue_waiting", "thumbnailGeneration"), Some(2.0));
        assert_eq!(labelled(&samples, "immich_queue_paused", "metadataExtraction"), Some(0.0));
    }

    #[test]
    fn la_mise_a_jour_se_compare_a_la_version_installee() {
        let about: About = serde_json::from_str(ABOUT).unwrap();
        let check: VersionCheck = serde_json::from_str(VERSION_CHECK).unwrap();
        assert_eq!(value(&update_samples(&about, &check, 0), "immich_update_available"), Some(0.0));
        let newer = VersionCheck { release_version: Some("v3.3.0".into()) };
        let samples = update_samples(&about, &newer, 0);
        assert_eq!(value(&samples, "immich_update_available"), Some(1.0));
        assert_eq!(samples[0].labels["latest_version"], "v3.3.0");
        let off = VersionCheck { release_version: None };
        assert!(update_samples(&about, &off, 0).is_empty());
    }

    #[test]
    fn stockage_et_statistiques() {
        let samples = storage_samples(&serde_json::from_str(STORAGE).unwrap(), 0);
        assert_eq!(value(&samples, "immich_storage_used_percent"), Some(71.19));
        assert!(value(&samples, "immich_storage_available_bytes").unwrap() > 1e9);
        let samples = statistics_samples(&serde_json::from_str(STATISTICS).unwrap(), 0);
        assert_eq!(value(&samples, "immich_photos"), Some(6.0));
        assert_eq!(value(&samples, "immich_videos"), Some(1.0));
        assert_eq!(value(&samples, "immich_users"), Some(2.0));
    }

    #[test]
    fn une_session_devient_un_appareil_nomme_type_et_id() {
        let sessions: Vec<Session> = serde_json::from_str(SESSIONS).unwrap();
        assert_eq!(sessions.len(), 3);
        let device = session_device(&sessions[0], "Noe").unwrap();
        assert_eq!(device.name, "Mobile-3fa85f64");
        assert_eq!(device.device_type, "Mobile");
        assert_eq!(device.os, "iOS 17.5");
        assert_eq!(device.user, "Noe");
        assert!(device.last_seen.is_some());
        assert!(device.last_backup.is_none());
    }

    #[test]
    fn le_nom_du_compte_retombe_sur_l_email_si_vide() {
        let user: UserMe = serde_json::from_str(USERS_ME).unwrap();
        assert_eq!(user.display_name(), "Noe");
        let anonyme = UserMe { name: String::new(), email: "a@b.lan".into() };
        assert_eq!(anonyme.display_name(), "a@b.lan");
    }

    #[test]
    fn la_recherche_de_metadonnees_donne_la_date_du_dernier_asset() {
        let page: SearchMetadata = serde_json::from_str(SEARCH_METADATA).unwrap();
        let instant = parse_instant(&page.assets.items[0].created_at).unwrap();
        assert_eq!(instant, 1790798700); // 2026-09-30T20:05:00Z
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn target(address: String, key: &str) -> Target {
        Target {
            id: 5,
            name: "photos".into(),
            address,
            kind: "immich".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::new(),
            credential: Credential::ApiToken { token: key.into() },
        }
    }

    #[tokio::test]
    async fn une_cle_d_un_compte_ordinaire_lit_ce_qu_elle_peut() {
        use axum::http::StatusCode;
        use axum::routing::post;
        let forbidden = || async { (StatusCode::FORBIDDEN, r#"{"message":"Forbidden"}"#) };
        let app = Router::new()
            .route(
                "/api/server/about",
                get(|headers: HeaderMap| async move {
                    if headers.get("x-api-key").and_then(|v| v.to_str().ok()) != Some("cle") {
                        return (StatusCode::UNAUTHORIZED, r#"{"message":"Invalid API key"}"#);
                    }
                    (StatusCode::OK, ABOUT)
                }),
            )
            .route("/api/server/version-check", get(|| async { VERSION_CHECK }))
            .route("/api/server/storage", get(|| async { STORAGE }))
            .route("/api/server/statistics", get(forbidden))
            .route("/api/queues", get(forbidden))
            .route("/api/server/config", get(|| async { CONFIG }))
            // Compte ordinaire : les appareils, eux, restent lisibles (ce ne
            // sont pas des pages d'administration).
            .route("/api/sessions", get(|| async { SESSIONS }))
            .route("/api/users/me", get(|| async { USERS_ME }))
            .route("/api/search/metadata", post(|| async { SEARCH_METADATA }));
        let base = serve(app).await;
        let samples =
            ImmichCollector::new().probe(&target(format!("{base}/api"), "cle")).await.unwrap();
        assert_eq!(value(&samples, "immich_statistics_readable"), Some(0.0));
        assert_eq!(value(&samples, "immich_scrape_errors"), Some(0.0));
        assert_eq!(value(&samples, "immich_maintenance_mode"), Some(0.0));
        assert!(value(&samples, "immich_jobs_waiting").is_none());
        let info = samples.iter().find(|s| s.metric == "immich_version_info").unwrap();
        assert_eq!(info.labels["version"], "v3.2.4");
        assert_eq!(
            device_value(&samples, "client_device_last_seen_timestamp_seconds", "Web-9b2d3c1a"),
            Some(1_790_798_730.0)
        );
        assert_eq!(
            device_value(&samples, "client_device_last_backup_timestamp_seconds", LIBRARY_DEVICE),
            Some(1_790_798_700.0)
        );

        let error = ImmichCollector::new().probe(&target(base, "fausse")).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
    }

    #[tokio::test]
    async fn un_immich_ancien_donne_ses_files_par_api_jobs() {
        use axum::routing::post;
        let app = Router::new()
            .route("/api/server/about", get(|| async { ABOUT }))
            .route("/api/server/version-check", get(|| async { VERSION_CHECK }))
            .route("/api/server/storage", get(|| async { STORAGE }))
            .route("/api/server/statistics", get(|| async { STATISTICS }))
            .route("/api/jobs", get(|| async { JOBS }))
            .route("/api/server/config", get(|| async { CONFIG }))
            .route("/api/sessions", get(|| async { SESSIONS }))
            .route("/api/users/me", get(|| async { USERS_ME }))
            .route("/api/search/metadata", post(|| async { SEARCH_METADATA }));
        let base = serve(app).await;
        let samples = ImmichCollector::new().probe(&target(base, "cle")).await.unwrap();
        assert_eq!(value(&samples, "immich_queues_paused"), Some(1.0));
        assert_eq!(value(&samples, "immich_photos"), Some(6.0));
        assert_eq!(value(&samples, "immich_scrape_errors"), Some(0.0));
        // Les trois sessions de la fixture donnent trois appareils, plus
        // l'appareil « bibliothèque » qui porte la dernière sauvegarde.
        let devices: std::collections::HashSet<_> = samples
            .iter()
            .filter(|s| s.metric == "client_device_last_seen_timestamp_seconds")
            .filter_map(|s| s.labels.get("device"))
            .collect();
        assert_eq!(devices.len(), 3);
        assert_eq!(
            device_value(&samples, "client_device_last_backup_timestamp_seconds", LIBRARY_DEVICE),
            Some(1_790_798_700.0)
        );
        // L'écart au seuil dépend de l'horloge au moment du test (le seuil
        // par défaut compare à `now()`) : on vérifie seulement qu'il existe
        // pour chaque signal, pas son signe.
        for (device, signal) in [("Web-9b2d3c1a", "connection"), (LIBRARY_DEVICE, "backup")] {
            assert!(
                samples.iter().any(|s| {
                    s.metric == "client_device_stale_seconds"
                        && s.labels.get("device").map(String::as_str) == Some(device)
                        && s.labels.get("signal").map(String::as_str) == Some(signal)
                }),
                "no staleness series for {device}/{signal}"
            );
        }
    }

    #[tokio::test]
    async fn sans_les_permissions_optionnelles_aucun_appareil_n_est_signale() {
        use axum::http::StatusCode;
        use axum::routing::post;
        let forbidden = || async { (StatusCode::FORBIDDEN, r#"{"message":"Forbidden"}"#) };
        let app = Router::new()
            .route("/api/server/about", get(|| async { ABOUT }))
            .route("/api/server/version-check", get(|| async { VERSION_CHECK }))
            .route("/api/server/storage", get(|| async { STORAGE }))
            .route("/api/server/statistics", get(forbidden))
            .route("/api/queues", get(forbidden))
            .route("/api/server/config", get(|| async { CONFIG }))
            .route("/api/sessions", get(forbidden))
            .route("/api/users/me", get(forbidden))
            .route("/api/search/metadata", post(forbidden));
        let base = serve(app).await;
        let samples = ImmichCollector::new().probe(&target(base, "cle")).await.unwrap();
        assert_eq!(
            value(&samples, "immich_scrape_errors"),
            Some(0.0),
            "un 403 n'est pas une panne"
        );
        assert!(samples.iter().all(|s| s.metric != "client_device_last_seen_timestamp_seconds"));
        assert!(samples.iter().all(|s| s.metric != "client_device_last_backup_timestamp_seconds"));
    }
}
