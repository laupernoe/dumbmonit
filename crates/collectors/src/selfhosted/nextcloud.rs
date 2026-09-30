//! Nextcloud (Server, Hub, AIO).
//!
//! Deux sources :
//!
//! * `status.php`, public, qui dit si l'instance est installée, en
//!   maintenance, ou attend une mise à niveau de sa base — l'état dans lequel
//!   une mise à jour ratée la laisse, et que rien d'autre ne signale ;
//! * l'API de l'application `serverinfo` (livrée avec Nextcloud), lue avec le
//!   jeton de supervision `NC-Token`. Ce jeton n'ouvre que cette API : il ne
//!   voit ni fichier, ni utilisateur, ni partage. C'est la seule façon de la
//!   lire sans compte d'administration complet : la délégation
//!   d'administration ne l'ouvre pas.
//!
//! En maintenance, l'API répond 503 : la sonde s'arrête alors à `status.php`,
//! qui suffit à le dire.
//!
//! Ce que ni l'une ni l'autre ne disent : la date du dernier passage de la
//! tâche de fond (cron). Elle n'est lisible qu'avec un compte
//! d'administration complet, que DumbMonit ne demande pas ; la documentation
//! propose un battement de cœur à la place.

use async_trait::async_trait;
use dumbmonit_proto::{Collector, ProbeError, Sample, Target};
use serde::Deserialize;
use serde_json::Value;

use super::client::{Auth, HttpClient};
use super::{MAX_NAMED, flag, gauge, http_client, now_ms, number, token};

pub const DEFAULT_PORT: u16 = 443;

const SERVERINFO: &str =
    "/ocs/v2.php/apps/serverinfo/api/v1/info?format=json&skipApps=false&skipUpdate=false";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    #[serde(default)]
    pub installed: bool,
    #[serde(default)]
    pub maintenance: bool,
    #[serde(default)]
    pub needs_db_upgrade: bool,
    #[serde(default)]
    pub versionstring: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub productname: Option<String>,
}

// ------------------------------------------------------------------ sonde

pub async fn probe(client: &HttpClient, ts_ms: i64) -> Result<Vec<Sample>, ProbeError> {
    // Public, et la preuve de vie : un Nextcloud qui ne répond pas ici est tombé.
    let status: Status = client.get_json("/status.php").await?;
    let mut out = status_samples(&status, ts_ms);
    if !status.installed {
        return Err(ProbeError::Protocol(
            "Nextcloud answers but is not installed yet: finish the installation first."
                .to_string(),
        ));
    }
    // En maintenance ou en attente de mise à niveau, l'API répond 503 : ce
    // n'est pas une panne, `status.php` vient de le dire.
    if status.maintenance || status.needs_db_upgrade {
        return Ok(out);
    }
    let info: Value = client.get_json(SERVERINFO).await?;
    let data = info.pointer("/ocs/data").ok_or_else(|| {
        ProbeError::Protocol("serverinfo answered without its data block".to_string())
    })?;
    out.extend(serverinfo_samples(data, ts_ms));
    Ok(out)
}

// ------------------------------------------------------------ traduction

pub fn status_samples(status: &Status, ts_ms: i64) -> Vec<Sample> {
    let mut out = vec![
        flag("nextcloud_maintenance", status.maintenance, ts_ms),
        flag("nextcloud_needs_db_upgrade", status.needs_db_upgrade, ts_ms),
    ];
    if let Some(version) = status.versionstring.as_deref().or(status.version.as_deref()) {
        out.push(gauge("nextcloud_version_info", 1.0, ts_ms).with_label("version", version));
    }
    out
}

/// Le bloc `ocs.data` de `serverinfo`. Lu en `Value` : ses champs varient d'une
/// version à l'autre (`opcache` vaut `[]` quand OPcache est coupé, `fpm`
/// `false` hors PHP-FPM, `size` du texte sur d'anciennes versions).
pub fn serverinfo_samples(data: &Value, ts_ms: i64) -> Vec<Sample> {
    let mut out = Vec::new();
    let mut put = |name: &str, pointer: &str| {
        if let Some(value) = number(data.pointer(pointer)) {
            out.push(gauge(name, value, ts_ms));
        }
    };
    put("nextcloud_free_space_bytes", "/nextcloud/system/freespace");
    put("nextcloud_users", "/nextcloud/storage/num_users");
    put("nextcloud_files", "/nextcloud/storage/num_files");
    put("nextcloud_shares", "/nextcloud/shares/num_shares");
    put("nextcloud_apps_installed", "/nextcloud/system/apps/num_installed");
    put("nextcloud_app_updates_available", "/nextcloud/system/apps/num_updates_available");
    put("nextcloud_database_size_bytes", "/server/database/size");
    put("nextcloud_php_memory_limit_bytes", "/server/php/memory_limit");

    for (window, key) in [("5m", "last5minutes"), ("1h", "last1hour"), ("24h", "last24hours")] {
        if let Some(value) = number(data.pointer(&format!("/activeUsers/{key}"))) {
            out.push(gauge("nextcloud_active_users", value, ts_ms).with_label("window", window));
        }
    }

    if let Some(update) = data.pointer("/nextcloud/system/update") {
        let available = update.get("available").and_then(Value::as_bool).unwrap_or(false);
        let mut sample = flag("nextcloud_update_available", available, ts_ms);
        if let Some(version) = update.get("available_version").and_then(Value::as_str)
            && available
        {
            sample = sample.with_label("available_version", version);
        }
        out.push(sample);
    }
    if let Some(Value::Object(apps)) = data.pointer("/nextcloud/system/apps/app_updates") {
        for (app, version) in apps.iter().take(MAX_NAMED) {
            out.push(
                gauge("nextcloud_app_update_available", 1.0, ts_ms)
                    .with_label("app", app)
                    .with_label("available_version", version.as_str().unwrap_or("")),
            );
        }
    }
    if let Some(kind) = data.pointer("/server/database/type").and_then(Value::as_str) {
        let version = data.pointer("/server/database/version").and_then(Value::as_str);
        let php = data.pointer("/server/php/version").and_then(Value::as_str);
        out.push(
            gauge("nextcloud_platform_info", 1.0, ts_ms)
                .with_label("database", kind)
                .with_label("database_version", version.unwrap_or(""))
                .with_label("php_version", php.unwrap_or("")),
        );
    }
    out.extend(opcache_samples(data.pointer("/server/php/opcache"), ts_ms));
    out.extend(fpm_samples(data.pointer("/server/fpm"), ts_ms));
    out
}

/// OPcache plein, c'est PHP qui recompile chaque script à chaque requête :
/// Nextcloud devient lent sans rien dire. `[]` ou absent : OPcache coupé.
fn opcache_samples(opcache: Option<&Value>, ts_ms: i64) -> Vec<Sample> {
    let Some(opcache) = opcache.filter(|value| value.is_object()) else {
        return vec![flag("nextcloud_opcache_enabled", false, ts_ms)];
    };
    let enabled = opcache.get("opcache_enabled").and_then(Value::as_bool).unwrap_or(false);
    let mut out = vec![flag("nextcloud_opcache_enabled", enabled, ts_ms)];
    if !enabled {
        return out;
    }
    if let Some(full) = opcache.get("cache_full").and_then(Value::as_bool) {
        out.push(flag("nextcloud_opcache_full", full, ts_ms));
    }
    let used = number(opcache.pointer("/memory_usage/used_memory"));
    let free = number(opcache.pointer("/memory_usage/free_memory"));
    let wasted = number(opcache.pointer("/memory_usage/wasted_memory")).unwrap_or(0.0);
    if let (Some(used), Some(free)) = (used, free)
        && used + free + wasted > 0.0
    {
        out.push(gauge(
            "nextcloud_opcache_memory_used_percent",
            (used + wasted) / (used + free + wasted) * 100.0,
            ts_ms,
        ));
    }
    let strings_used = number(opcache.pointer("/interned_strings_usage/used_memory"));
    let strings_size = number(opcache.pointer("/interned_strings_usage/buffer_size"));
    if let (Some(used), Some(size)) = (strings_used, strings_size)
        && size > 0.0
    {
        out.push(gauge(
            "nextcloud_opcache_interned_strings_used_percent",
            used / size * 100.0,
            ts_ms,
        ));
    }
    if let Some(rate) = number(opcache.pointer("/opcache_statistics/opcache_hit_rate")) {
        out.push(gauge("nextcloud_opcache_hit_rate_percent", rate, ts_ms));
    }
    if let Some(restarts) = number(opcache.pointer("/opcache_statistics/oom_restarts")) {
        out.push(gauge("nextcloud_opcache_oom_restarts", restarts, ts_ms));
    }
    out
}

/// PHP-FPM, quand Nextcloud tourne derrière lui (image `-fpm`, AIO). `false`
/// avec Apache et mod_php : rien à dire.
fn fpm_samples(fpm: Option<&Value>, ts_ms: i64) -> Vec<Sample> {
    let Some(fpm) = fpm.filter(|value| value.is_object()) else { return Vec::new() };
    let mut out = Vec::new();
    for (name, key) in [
        ("nextcloud_fpm_active_processes", "active-processes"),
        ("nextcloud_fpm_total_processes", "total-processes"),
        ("nextcloud_fpm_listen_queue", "listen-queue"),
        ("nextcloud_fpm_max_children_reached", "max-children-reached"),
        ("nextcloud_fpm_slow_requests", "slow-requests"),
    ] {
        if let Some(value) = number(fpm.get(key)) {
            out.push(gauge(name, value, ts_ms));
        }
    }
    out
}

// ------------------------------------------------------------ collecteur

#[derive(Default)]
pub struct NextcloudCollector;

impl NextcloudCollector {
    pub fn new() -> Self {
        Self
    }

    fn client(target: &Target) -> Result<HttpClient, ProbeError> {
        let Some(token) = token(&target.credential) else {
            return Err(ProbeError::Config(format!(
                "Nextcloud expects the serverinfo monitoring token (NC-Token), configured: {}",
                target.credential
            )));
        };
        // `OCS-APIRequest` : sans lui, l'API OCS renvoie vers la page de connexion.
        let auth = Auth::header("NC-Token", token).with_header("OCS-APIRequest", "true");
        // Une adresse copiée depuis le navigateur finit souvent par `/index.php`
        // ou `/login` : la racine suffit.
        let mut target = target.clone();
        let trimmed = target.address.trim().trim_end_matches('/');
        let trimmed = trimmed.strip_suffix("/login").unwrap_or(trimmed);
        target.address = trimmed.strip_suffix("/index.php").unwrap_or(trimmed).to_string();
        http_client(&target, "https", DEFAULT_PORT, auth, "Nextcloud")
    }
}

#[async_trait]
impl Collector for NextcloudCollector {
    fn kind(&self) -> &'static str {
        "nextcloud"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        probe(&Self::client(target)?, now_ms()).await
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        probe(&Self::client(target)?, now_ms()).await?;
        Ok(Some("nextcloud".to_string()))
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

    // Réponses réelles d'un Nextcloud 35.0.1 (image officielle, SQLite), lues
    // avec le jeton de supervision. L'application `calendar` a été installée
    // dans une version antérieure pour qu'une mise à jour d'application soit
    // proposée ; la mise à jour de Nextcloud lui-même a été posée dans la
    // configuration (`core.lastupdateResult`) au format du serveur de mises à
    // jour, faute d'une version plus récente publiée ce jour-là.
    const STATUS: &str = include_str!("testdata/nextcloud_35.0.1/status.json");
    const STATUS_MAINTENANCE: &str =
        include_str!("testdata/nextcloud_35.0.1/status_maintenance.json");
    const SERVERINFO: &str = include_str!("testdata/nextcloud_35.0.1/serverinfo.json");
    const SERVERINFO_MAINTENANCE: &str =
        include_str!("testdata/nextcloud_35.0.1/serverinfo_maintenance.json");
    const SERVERINFO_UNAUTHORIZED: &str =
        include_str!("testdata/nextcloud_35.0.1/serverinfo_unauthorized.json");

    fn value(samples: &[Sample], name: &str) -> Option<f64> {
        samples.iter().find(|s| s.metric == name).map(|s| s.value)
    }

    fn data() -> Value {
        let info: Value = serde_json::from_str(SERVERINFO).unwrap();
        info.pointer("/ocs/data").unwrap().clone()
    }

    #[test]
    fn status_php_dit_version_et_maintenance() {
        let samples = status_samples(&serde_json::from_str(STATUS).unwrap(), 0);
        assert_eq!(value(&samples, "nextcloud_maintenance"), Some(0.0));
        assert_eq!(value(&samples, "nextcloud_needs_db_upgrade"), Some(0.0));
        let version = samples.iter().find(|s| s.metric == "nextcloud_version_info").unwrap();
        assert_eq!(version.labels["version"], "35.0.1");
        let samples = status_samples(&serde_json::from_str(STATUS_MAINTENANCE).unwrap(), 0);
        assert_eq!(value(&samples, "nextcloud_maintenance"), Some(1.0));
    }

    #[test]
    fn serverinfo_donne_mises_a_jour_utilisateurs_et_stockage() {
        let samples = serverinfo_samples(&data(), 0);
        let update = samples.iter().find(|s| s.metric == "nextcloud_update_available").unwrap();
        assert_eq!(update.value, 1.0);
        assert_eq!(update.labels["available_version"], "35.0.2.1");
        assert_eq!(value(&samples, "nextcloud_app_updates_available"), Some(1.0));
        let app = samples.iter().find(|s| s.metric == "nextcloud_app_update_available").unwrap();
        assert_eq!(app.labels["app"], "calendar");
        assert_eq!(app.labels["available_version"], "6.6.1");
        assert_eq!(value(&samples, "nextcloud_apps_installed"), Some(52.0));
        let active: Vec<_> =
            samples.iter().filter(|s| s.metric == "nextcloud_active_users").collect();
        assert_eq!(active.len(), 3);
        assert!(value(&samples, "nextcloud_free_space_bytes").unwrap() > 1e9);
        assert!(value(&samples, "nextcloud_database_size_bytes").unwrap() > 0.0);
        assert_eq!(value(&samples, "nextcloud_php_memory_limit_bytes"), Some(536_870_912.0));
        let platform = samples.iter().find(|s| s.metric == "nextcloud_platform_info").unwrap();
        assert_eq!(platform.labels["database"], "sqlite3");
    }

    #[test]
    fn opcache_et_fpm_selon_ce_que_php_expose() {
        let samples = serverinfo_samples(&data(), 0);
        assert_eq!(value(&samples, "nextcloud_opcache_enabled"), Some(1.0));
        assert_eq!(value(&samples, "nextcloud_opcache_full"), Some(0.0));
        let used = value(&samples, "nextcloud_opcache_memory_used_percent").unwrap();
        assert!(used > 0.0 && used < 100.0, "{used}");
        assert!(value(&samples, "nextcloud_opcache_hit_rate_percent").is_some());
        // Apache et mod_php : pas de PHP-FPM, pas de série.
        assert!(value(&samples, "nextcloud_fpm_active_processes").is_none());

        // OPcache coupé : PHP renvoie un tableau vide.
        let disabled = opcache_samples(Some(&serde_json::json!([])), 0);
        assert_eq!(value(&disabled, "nextcloud_opcache_enabled"), Some(0.0));
        let fpm = fpm_samples(
            Some(&serde_json::json!({"active-processes": 3, "max-children-reached": 12})),
            0,
        );
        assert_eq!(value(&fpm, "nextcloud_fpm_max_children_reached"), Some(12.0));
    }

    fn target(address: String, token: &str) -> Target {
        Target {
            id: 3,
            name: "cloud".into(),
            address,
            kind: "nextcloud".into(),
            profile_id: None,
            parent_id: None,
            interval: Duration::from_secs(60),
            enabled: true,
            tags: BTreeMap::new(),
            credential: Credential::ApiToken { token: token.into() },
        }
    }

    async fn serve(app: Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn fake(maintenance: bool) -> Router {
        Router::new()
            .route(
                "/status.php",
                get(move || async move { if maintenance { STATUS_MAINTENANCE } else { STATUS } }),
            )
            .route(
                "/ocs/v2.php/apps/serverinfo/api/v1/info",
                get(move |headers: HeaderMap| async move {
                    if maintenance {
                        return (StatusCode::SERVICE_UNAVAILABLE, SERVERINFO_MAINTENANCE);
                    }
                    let token = headers.get("nc-token").and_then(|v| v.to_str().ok());
                    let ocs = headers.get("ocs-apirequest").and_then(|v| v.to_str().ok());
                    if token != Some("jeton-de-supervision") || ocs != Some("true") {
                        return (StatusCode::UNAUTHORIZED, SERVERINFO_UNAUTHORIZED);
                    }
                    (StatusCode::OK, SERVERINFO)
                }),
            )
    }

    #[tokio::test]
    async fn la_sonde_complete_lit_status_puis_serverinfo() {
        let base = serve(fake(false)).await;
        let samples = NextcloudCollector::new()
            .probe(&target(format!("{base}/index.php/"), " jeton-de-supervision\n"))
            .await
            .unwrap();
        assert_eq!(value(&samples, "nextcloud_maintenance"), Some(0.0));
        assert_eq!(value(&samples, "nextcloud_update_available"), Some(1.0));

        let error = NextcloudCollector::new().probe(&target(base, "mauvais")).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
    }

    #[tokio::test]
    async fn en_maintenance_la_sonde_reussit_et_le_dit() {
        let base = serve(fake(true)).await;
        let samples =
            NextcloudCollector::new().probe(&target(base, "jeton-de-supervision")).await.unwrap();
        assert_eq!(value(&samples, "nextcloud_maintenance"), Some(1.0));
        assert!(value(&samples, "nextcloud_users").is_none());
    }

    #[test]
    fn sans_jeton_c_est_une_erreur_de_configuration() {
        let mut t = target("cloud.lan".into(), "x");
        t.credential = Credential::None;
        assert!(matches!(NextcloudCollector::client(&t), Err(ProbeError::Config(_))));
    }
}
