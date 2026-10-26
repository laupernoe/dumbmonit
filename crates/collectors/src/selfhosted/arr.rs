//! Sonarr, Radarr, Lidarr et Prowlarr.
//!
//! Les quatre applications de la famille *arr parlent la même API (Servarr) :
//! un en-tête `X-Api-Key`, `/system/status`, `/health`, `/diskspace`,
//! `/update`, et, pour les trois gestionnaires de médias, `/queue/status`.
//! Seuls le port, la version de l'API (`v3` pour Sonarr et Radarr, `v1` pour
//! Lidarr et Prowlarr) et quelques routes propres changent : un seul module,
//! quatre [`Flavor`].
//!
//! Ce que ces applications font de pire est de se taire. Un client de
//! téléchargement qui ne répond plus, un indexeur désactivé après des échecs,
//! un disque de bibliothèque plein : l'application tourne, la page s'ouvre,
//! plus rien n'est récupéré. Le message d'alerte de la page « System → Status »
//! est exactement ce que `/health` renvoie ; on le lit tel quel.
//!
//! Une clé d'API Servarr n'a qu'un niveau de droit (tout), mais DumbMonit
//! n'envoie que des `GET`.

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};
use serde_json::Value;

use super::client::{Auth, HttpClient};
use super::{
    MAX_NAMED, flag, gauge, http_client, instant, metric, now_ms, number, optional_json, settle,
    token,
};

/// Ce qui distingue les quatre applications.
#[derive(Debug, Clone, Copy)]
pub struct Flavor {
    pub kind: &'static str,
    pub product: &'static str,
    pub port: u16,
    /// Version de l'API dans les chemins : `v3` ou `v1`.
    pub api: &'static str,
    /// Possède une file de téléchargements (`/queue/status`).
    pub queue: bool,
    /// Possède des indexeurs (`/indexerstatus`) : Prowlarr.
    pub indexers: bool,
}

pub const SONARR: Flavor = Flavor {
    kind: "sonarr",
    product: "Sonarr",
    port: 8989,
    api: "v3",
    queue: true,
    indexers: false,
};
pub const RADARR: Flavor = Flavor {
    kind: "radarr",
    product: "Radarr",
    port: 7878,
    api: "v3",
    queue: true,
    indexers: false,
};
pub const LIDARR: Flavor = Flavor {
    kind: "lidarr",
    product: "Lidarr",
    port: 8686,
    api: "v1",
    queue: true,
    indexers: false,
};
pub const PROWLARR: Flavor = Flavor {
    kind: "prowlarr",
    product: "Prowlarr",
    port: 9696,
    api: "v1",
    queue: false,
    indexers: true,
};

// ------------------------------------------------------------ traduction

/// `GET /health` : `[{"source", "type": "ok|notice|warning|error", "message"}]`.
pub fn health_samples(flavor: &Flavor, body: &Value, ts_ms: i64) -> Vec<Sample> {
    let items = body.as_array().map(Vec::as_slice).unwrap_or(&[]);
    let level = |item: &Value| item.get("type").and_then(Value::as_str).unwrap_or("").to_string();
    let errors = items.iter().filter(|i| level(i) == "error").count();
    let warnings = items.iter().filter(|i| level(i) == "warning").count();
    let p = flavor.kind;
    let mut out = vec![
        gauge(&metric(p, "health_errors"), errors as f64, ts_ms),
        gauge(&metric(p, "health_warnings"), warnings as f64, ts_ms),
    ];
    for item in items.iter().filter(|i| matches!(level(i).as_str(), "error" | "warning")) {
        if out.len() >= 2 + MAX_NAMED {
            break;
        }
        let source = item.get("source").and_then(Value::as_str).unwrap_or("unknown");
        out.push(
            gauge(&metric(p, "health_issue"), 1.0, ts_ms)
                .with_label("source", source)
                .with_label("level", level(item)),
        );
    }
    out
}

/// `GET /diskspace` : `[{"path", "freeSpace", "totalSpace"}]`.
pub fn disk_samples(flavor: &Flavor, body: &Value, ts_ms: i64) -> Vec<Sample> {
    let p = flavor.kind;
    let mut out = Vec::new();
    for disk in body.as_array().map(Vec::as_slice).unwrap_or(&[]).iter().take(MAX_NAMED) {
        let (Some(path), Some(free), Some(total)) = (
            disk.get("path").and_then(Value::as_str),
            number(disk.get("freeSpace")),
            number(disk.get("totalSpace")),
        ) else {
            continue;
        };
        if total <= 0.0 {
            continue;
        }
        let used = (100.0 * (1.0 - free / total)).clamp(0.0, 100.0);
        out.push(gauge(&metric(p, "disk_used_percent"), used, ts_ms).with_label("path", path));
        out.push(gauge(&metric(p, "disk_free_bytes"), free, ts_ms).with_label("path", path));
    }
    out
}

/// `GET /update` : les versions publiées, dont celle qui est installée
/// (`installed`) et la plus récente (`latest`). Une mise à jour existe quand la
/// plus récente n'est pas celle qui tourne.
pub fn update_samples(flavor: &Flavor, body: &Value, ts_ms: i64) -> Vec<Sample> {
    let Some(items) = body.as_array() else { return Vec::new() };
    let flag_of = |item: &Value, key: &str| item.get(key).and_then(Value::as_bool).unwrap_or(false);
    let newer = items.iter().find(|i| flag_of(i, "latest") && !flag_of(i, "installed"));
    let name = metric(flavor.kind, "update_available");
    match newer {
        Some(item) => {
            let version = item.get("version").and_then(Value::as_str).unwrap_or("");
            vec![flag(&name, true, ts_ms).with_label("latest_version", version)]
        }
        None => vec![flag(&name, false, ts_ms)],
    }
}

/// `GET /queue/status` : les compteurs de la file, et deux drapeaux qui disent
/// qu'au moins un élément a une erreur ou un avertissement (téléchargement
/// échoué, import bloqué).
pub fn queue_samples(flavor: &Flavor, body: &Value, ts_ms: i64) -> Vec<Sample> {
    let p = flavor.kind;
    let flag_of = |key: &str| body.get(key).and_then(Value::as_bool).unwrap_or(false);
    let mut out = Vec::new();
    if let Some(total) = number(body.get("totalCount")) {
        out.push(gauge(&metric(p, "queue_total"), total, ts_ms));
    }
    if let Some(unknown) = number(body.get("unknownCount")) {
        out.push(gauge(&metric(p, "queue_unknown"), unknown, ts_ms));
    }
    out.push(flag(
        &metric(p, "queue_has_errors"),
        flag_of("errors") || flag_of("unknownErrors"),
        ts_ms,
    ));
    out.push(flag(
        &metric(p, "queue_has_warnings"),
        flag_of("warnings") || flag_of("unknownWarnings"),
        ts_ms,
    ));
    out
}

/// `GET /indexerstatus` (Prowlarr) : un enregistrement par indexeur en échec,
/// désactivé jusqu'à `disabledTill`. Un indexeur dont la pénalité est passée
/// est de nouveau interrogé : il ne compte plus.
pub fn indexer_samples(flavor: &Flavor, body: &Value, now_s: i64, ts_ms: i64) -> Vec<Sample> {
    let disabled = body
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter(|status| match status.get("disabledTill").and_then(Value::as_str) {
            Some(till) => instant(till).is_none_or(|till| till > now_s),
            None => false,
        })
        .count();
    vec![gauge(&metric(flavor.kind, "indexers_disabled"), disabled as f64, ts_ms)]
}

// ------------------------------------------------------------------ sonde

pub async fn probe(
    flavor: &Flavor,
    client: &HttpClient,
    target: &Target,
    ts_ms: i64,
) -> Result<Vec<Sample>, ProbeError> {
    let api = flavor.api;
    // La preuve de vie et d'authentification : le seul appel qui condamne.
    let status: Value = client.get_json(&format!("/api/{api}/system/status")).await?;
    let (health_path, disks_path, update_path) = (
        format!("/api/{api}/health"),
        format!("/api/{api}/diskspace"),
        format!("/api/{api}/update"),
    );
    let (queue_path, indexers_path) =
        (format!("/api/{api}/queue/status"), format!("/api/{api}/indexerstatus"));
    let (health, disks, updates, queue, indexers) = futures::join!(
        client.get_json::<Value>(&health_path),
        client.get_json::<Value>(&disks_path),
        optional_json(client, &update_path),
        async { if flavor.queue { optional_json(client, &queue_path).await } else { Ok(None) } },
        async {
            if flavor.indexers { optional_json(client, &indexers_path).await } else { Ok(None) }
        },
    );

    let mut errors = 0u32;
    let mut out = Vec::new();
    let version = status.get("version").and_then(Value::as_str).unwrap_or("unknown");
    out.push(
        gauge(&metric(flavor.kind, "version_info"), 1.0, ts_ms).with_label("version", version),
    );
    if let Some(body) = settle(health, &mut errors, target.id, "health") {
        out.extend(health_samples(flavor, &body, ts_ms));
    }
    if let Some(body) = settle(disks, &mut errors, target.id, "diskspace") {
        out.extend(disk_samples(flavor, &body, ts_ms));
    }
    if let Some(Some(body)) = settle(updates, &mut errors, target.id, "update") {
        out.extend(update_samples(flavor, &body, ts_ms));
    }
    if let Some(Some(body)) = settle(queue, &mut errors, target.id, "queue/status") {
        out.extend(queue_samples(flavor, &body, ts_ms));
    }
    if let Some(Some(body)) = settle(indexers, &mut errors, target.id, "indexerstatus") {
        out.extend(indexer_samples(flavor, &body, ts_ms / 1000, ts_ms));
    }
    out.push(gauge(&metric(flavor.kind, "scrape_errors"), f64::from(errors), ts_ms));
    Ok(out)
}

// ------------------------------------------------------------ collecteur

pub struct ArrCollector {
    flavor: Flavor,
}

impl ArrCollector {
    pub fn new(flavor: Flavor) -> Self {
        Self { flavor }
    }

    pub fn sonarr() -> Self {
        Self::new(SONARR)
    }

    pub fn radarr() -> Self {
        Self::new(RADARR)
    }

    pub fn lidarr() -> Self {
        Self::new(LIDARR)
    }

    pub fn prowlarr() -> Self {
        Self::new(PROWLARR)
    }

    fn client(&self, target: &Target) -> Result<HttpClient, ProbeError> {
        let Some(key) = token(&target.credential) else {
            return Err(ProbeError::Config(format!(
                "{} expects an API key, configured: {}",
                self.flavor.product, target.credential
            )));
        };
        http_client(
            target,
            "http",
            self.flavor.port,
            Auth::header("X-Api-Key", key),
            self.flavor.product,
        )
    }
}

#[async_trait]
impl Collector for ArrCollector {
    fn kind(&self) -> &str {
        self.flavor.kind
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        probe(&self.flavor, &self.client(target)?, target, now_ms()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let path = format!("/api/{}/system/status", self.flavor.api);
        let _: Value = self.client(target)?.get_json(&path).await?;
        Ok(Some(self.flavor.kind.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use axum::Router;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::get;
    use dumbmonit_proto::Credential;

    use super::*;

    // Réponses d'un Sonarr 4.0.15 (Docker) dont un client de téléchargement est
    // tombé et dont un indexeur est en échec, d'un Radarr 5.21.1 sain, d'un
    // Lidarr 2.9.6 dont la file porte un avertissement et d'un Prowlarr 1.31.2
    // dont deux indexeurs sont désactivés.
    const SONARR_STATUS: &str = include_str!("testdata/sonarr_4.0.15/system_status.json");
    const SONARR_HEALTH: &str = include_str!("testdata/sonarr_4.0.15/health.json");
    const SONARR_DISKS: &str = include_str!("testdata/sonarr_4.0.15/diskspace.json");
    const SONARR_UPDATE: &str = include_str!("testdata/sonarr_4.0.15/update.json");
    const SONARR_QUEUE: &str = include_str!("testdata/sonarr_4.0.15/queue_status.json");
    const RADARR_STATUS: &str = include_str!("testdata/radarr_5.21.1/system_status.json");
    const RADARR_HEALTH: &str = include_str!("testdata/radarr_5.21.1/health.json");
    const RADARR_UPDATE: &str = include_str!("testdata/radarr_5.21.1/update.json");
    const RADARR_QUEUE: &str = include_str!("testdata/radarr_5.21.1/queue_status.json");
    const LIDARR_QUEUE: &str = include_str!("testdata/lidarr_2.9.6/queue_status.json");
    const LIDARR_DISKS: &str = include_str!("testdata/lidarr_2.9.6/diskspace.json");
    const PROWLARR_STATUS: &str = include_str!("testdata/prowlarr_1.31.2/system_status.json");
    const PROWLARR_HEALTH: &str = include_str!("testdata/prowlarr_1.31.2/health.json");
    const PROWLARR_INDEXERS: &str = include_str!("testdata/prowlarr_1.31.2/indexerstatus.json");
    const PROWLARR_UPDATE: &str = include_str!("testdata/prowlarr_1.31.2/update.json");
    const PROWLARR_DISKS: &str = include_str!("testdata/prowlarr_1.31.2/diskspace.json");

    fn json(text: &str) -> Value {
        serde_json::from_str(text).unwrap()
    }

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn labelled(samples: &[Sample], name: &str, key: &str, wanted: &str) -> Option<f64> {
        samples
            .iter()
            .find(|s| s.metric == name && s.labels.get(key).map(String::as_str) == Some(wanted))
            .map(|s| s.value)
    }

    #[test]
    fn la_sante_compte_erreurs_et_avertissements_et_nomme_les_problemes() {
        let samples = health_samples(&SONARR, &json(SONARR_HEALTH), 0);
        assert_eq!(value(&samples, "sonarr_health_errors"), Some(2.0));
        assert_eq!(value(&samples, "sonarr_health_warnings"), Some(1.0));
        assert_eq!(
            labelled(&samples, "sonarr_health_issue", "source", "DownloadClientCheck"),
            Some(1.0)
        );
        // La notice (mise à jour disponible) n'est pas un problème.
        assert!(labelled(&samples, "sonarr_health_issue", "source", "UpdateCheck").is_none());
        let samples = health_samples(&RADARR, &json(RADARR_HEALTH), 0);
        assert_eq!(value(&samples, "radarr_health_errors"), Some(0.0));
        assert_eq!(samples.len(), 2);
    }

    #[test]
    fn les_disques_donnent_un_pourcentage_et_sautent_les_volumes_vides() {
        let samples = disk_samples(&SONARR, &json(SONARR_DISKS), 0);
        let used = |samples: &[Sample], metric: &str, path: &str| {
            labelled(samples, metric, "path", path).unwrap()
        };
        assert!((used(&samples, "sonarr_disk_used_percent", "/") - 50.0).abs() < 1e-6);
        assert!((used(&samples, "sonarr_disk_used_percent", "/tv") - 95.0).abs() < 1e-6);
        assert!(labelled(&samples, "sonarr_disk_used_percent", "path", "/downloads").is_none());
        let samples = disk_samples(&LIDARR, &json(LIDARR_DISKS), 0);
        assert!((used(&samples, "lidarr_disk_used_percent", "/music") - 10.0).abs() < 1e-6);
    }

    #[test]
    fn une_mise_a_jour_n_est_annoncee_que_si_la_derniere_n_est_pas_installee() {
        let samples = update_samples(&SONARR, &json(SONARR_UPDATE), 0);
        assert_eq!(value(&samples, "sonarr_update_available"), Some(1.0));
        assert_eq!(samples[0].labels["latest_version"], "4.0.16.2944");
        let samples = update_samples(&RADARR, &json(RADARR_UPDATE), 0);
        assert_eq!(value(&samples, "radarr_update_available"), Some(0.0));
        assert!(update_samples(&RADARR, &json("{}"), 0).is_empty());
    }

    #[test]
    fn la_file_dit_si_un_element_est_en_erreur() {
        let samples = queue_samples(&SONARR, &json(SONARR_QUEUE), 0);
        assert_eq!(value(&samples, "sonarr_queue_total"), Some(3.0));
        assert_eq!(value(&samples, "sonarr_queue_unknown"), Some(1.0));
        assert_eq!(value(&samples, "sonarr_queue_has_errors"), Some(1.0));
        assert_eq!(value(&samples, "sonarr_queue_has_warnings"), Some(0.0));
        let samples = queue_samples(&LIDARR, &json(LIDARR_QUEUE), 0);
        assert_eq!(value(&samples, "lidarr_queue_has_errors"), Some(0.0));
        assert_eq!(value(&samples, "lidarr_queue_has_warnings"), Some(1.0));
        let samples = queue_samples(&RADARR, &json(RADARR_QUEUE), 0);
        assert_eq!(value(&samples, "radarr_queue_total"), Some(0.0));
    }

    #[test]
    fn prowlarr_compte_les_indexeurs_encore_desactives() {
        let now = 1_790_000_000;
        let samples = indexer_samples(&PROWLARR, &json(PROWLARR_INDEXERS), now, 0);
        assert_eq!(value(&samples, "prowlarr_indexers_disabled"), Some(2.0));
        let samples = indexer_samples(&PROWLARR, &json("[]"), now, 0);
        assert_eq!(value(&samples, "prowlarr_indexers_disabled"), Some(0.0));
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn target(kind: &str, address: String, key: &str) -> Target {
        Target {
            id: 9,
            name: "arr".into(),
            address,
            kind: kind.into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::new(),
            credential: Credential::ApiToken { token: key.into() },
            group_name: String::new(),
            position: 0,
        }
    }

    fn guarded(
        body: &'static str,
    ) -> impl Fn(HeaderMap) -> std::future::Ready<(StatusCode, &'static str)> + Clone {
        move |headers: HeaderMap| {
            let ok = headers.get("x-api-key").and_then(|v| v.to_str().ok()) == Some("cle");
            std::future::ready(if ok {
                (StatusCode::OK, body)
            } else {
                (StatusCode::UNAUTHORIZED, r#"{"error":"Unauthorized"}"#)
            })
        }
    }

    #[tokio::test]
    async fn sonarr_complet_et_cle_refusee() {
        let app = Router::new()
            .route("/api/v3/system/status", get(guarded(SONARR_STATUS)))
            .route("/api/v3/health", get(|| async { SONARR_HEALTH }))
            .route("/api/v3/diskspace", get(|| async { SONARR_DISKS }))
            .route("/api/v3/update", get(|| async { SONARR_UPDATE }))
            .route("/api/v3/queue/status", get(|| async { SONARR_QUEUE }));
        let base = serve(app).await;
        let samples =
            ArrCollector::sonarr().probe(&target("sonarr", base.clone(), "cle")).await.unwrap();
        assert_eq!(value(&samples, "sonarr_scrape_errors"), Some(0.0));
        assert_eq!(value(&samples, "sonarr_health_errors"), Some(2.0));
        assert_eq!(value(&samples, "sonarr_queue_has_errors"), Some(1.0));
        let info = samples.iter().find(|s| s.metric == "sonarr_version_info").unwrap();
        assert_eq!(info.labels["version"], "4.0.15.2941");

        let error =
            ArrCollector::sonarr().probe(&target("sonarr", base, "fausse")).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)));
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn radarr_derriere_un_prefixe_et_sans_mise_a_jour_connue() {
        let app = Router::new().nest(
            "/radarr",
            Router::new()
                .route("/api/v3/system/status", get(|| async { RADARR_STATUS }))
                .route("/api/v3/health", get(|| async { RADARR_HEALTH }))
                .route("/api/v3/diskspace", get(|| async { "[]" }))
                .route("/api/v3/queue/status", get(|| async { RADARR_QUEUE })),
        );
        let base = serve(app).await;
        let samples = ArrCollector::radarr()
            .probe(&target("radarr", format!("{base}/radarr"), "cle"))
            .await
            .unwrap();
        // `/update` absent (404) : sauté sans erreur.
        assert_eq!(value(&samples, "radarr_scrape_errors"), Some(0.0));
        assert!(value(&samples, "radarr_update_available").is_none());
        assert_eq!(value(&samples, "radarr_queue_total"), Some(0.0));
    }

    #[tokio::test]
    async fn prowlarr_n_a_pas_de_file_mais_des_indexeurs() {
        let app = Router::new()
            .route("/api/v1/system/status", get(|| async { PROWLARR_STATUS }))
            .route("/api/v1/health", get(|| async { PROWLARR_HEALTH }))
            .route("/api/v1/diskspace", get(|| async { PROWLARR_DISKS }))
            .route("/api/v1/update", get(|| async { PROWLARR_UPDATE }))
            .route("/api/v1/indexerstatus", get(|| async { PROWLARR_INDEXERS }));
        let base = serve(app).await;
        let samples =
            ArrCollector::prowlarr().probe(&target("prowlarr", base, "cle")).await.unwrap();
        assert_eq!(value(&samples, "prowlarr_indexers_disabled"), Some(2.0));
        assert_eq!(value(&samples, "prowlarr_health_warnings"), Some(1.0));
        assert!(value(&samples, "prowlarr_queue_total").is_none());
        assert_eq!(value(&samples, "prowlarr_update_available"), Some(0.0));
    }

    #[tokio::test]
    async fn une_cible_sans_cle_est_une_erreur_de_configuration() {
        let mut no_key = target("sonarr", "http://127.0.0.1:1".into(), "x");
        no_key.credential = Credential::None;
        let error = ArrCollector::sonarr().probe(&no_key).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)));
        assert!(!error.means_down());
        // Injoignable : une panne, pas une erreur de configuration.
        let error = ArrCollector::sonarr()
            .probe(&target("sonarr", "http://127.0.0.1:1".into(), "cle"))
            .await
            .unwrap_err();
        assert!(error.means_down());
    }
}
