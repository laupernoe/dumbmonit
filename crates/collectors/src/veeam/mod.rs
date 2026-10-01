//! Veeam Backup & Replication, par son API REST (port 9419, `/api/v1`).
//!
//! Connexion OAuth2 « mot de passe » (`/api/oauth2/token`) avec un compte qui
//! n'a que le rôle Veeam Backup Viewer ; le jeton (quinze minutes) est gardé
//! jusqu'à son expiration. Chaque appel porte l'en-tête `x-api-version`
//! obligatoire, `1.1-rev0` par défaut : la révision publiée avec la v12.0, que
//! toutes les versions suivantes acceptent encore.
//!
//! L'état des travaux est la seule lecture obligatoire. Les autres —
//! version, sessions des dernières 24 heures, dépôts, licence — sont lues si
//! le rôle le permet : Veeam réserve certaines routes, la licence en tête, au
//! rôle Backup Administrator, et un refus n'y est pas une erreur
//! (`veeam_section_readable{section="license"} 0`).
//!
//! Les pannes que ce module existe pour voir :
//!
//! * **un travail dont la dernière exécution a échoué** ou s'est terminée en
//!   avertissement ;
//! * **un dépôt qui se remplit** : les prochaines sauvegardes échoueront ;
//! * **une licence qui expire** : les travaux s'arrêtent à la fin de la
//!   période de grâce.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `https` | Protocole de l'API. |
//! | `port` | `9419` | Port de l'API. |
//! | `api_version` | `1.1-rev0` | Valeur de l'en-tête `x-api-version`. |
//! | `insecure_tls` | `false` | Accepte un certificat non vérifiable (celui d'origine est auto-signé). |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use reqwest::StatusCode;
use serde_json::Value;

use crate::rest::{
    MAX_NAMED, RestClient, TokenCache, age_seconds, flag, gauge, now_ms, number, seconds_until,
    text,
};
use crate::selfhosted::options::tag;

pub const DEFAULT_PORT: u16 = 9419;
pub const DEFAULT_API_VERSION: &str = "1.1-rev0";
const TOKEN_PATH: &str = "/api/oauth2/token";
const JOBS: &str = "/api/v1/jobs/states?limit=1000";
const SERVER_INFO: &str = "/api/v1/serverInfo";
const REPOSITORIES: &str = "/api/v1/backupInfrastructure/repositories/states?limit=500";
const LICENSE: &str = "/api/v1/license";
const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

#[derive(Default)]
pub struct VeeamCollector {
    tokens: TokenCache,
}

impl VeeamCollector {
    pub fn new() -> Self {
        Self::default()
    }
}

struct Session<'a> {
    collector: &'a VeeamCollector,
    client: RestClient,
    username: String,
    password: String,
    key: String,
}

impl Session<'_> {
    /// Connexion, ou jeton gardé.
    async fn login(&mut self, fresh: bool) -> Result<(), ProbeError> {
        if !fresh && let Some(token) = self.collector.tokens.get(&self.key) {
            self.client.set_header("Authorization", format!("Bearer {token}"));
            return Ok(());
        }
        let form = [
            ("grant_type", "password"),
            ("username", self.username.as_str()),
            ("password", self.password.as_str()),
        ];
        let (status, body) = self.client.post_form(TOKEN_PATH, &form).await?;
        if matches!(status.as_u16(), 400 | 401 | 403) {
            return Err(ProbeError::Auth(format!(
                "Veeam refused the login of {} ({status}). Check the user name (DOMAIN\\user or \
                 HOST\\user) and the password, and that the account has a Veeam role.",
                self.username
            )));
        }
        if !status.is_success() {
            return Err(self.client.status_error(status, &body, TOKEN_PATH));
        }
        let reply: Value = self.client.decode(&body, TOKEN_PATH)?;
        let token = text(&reply, "access_token")
            .ok_or_else(|| ProbeError::Protocol("Veeam returned no access_token".to_string()))?;
        let lifetime = reply.get("expires_in").and_then(Value::as_u64).unwrap_or(900);
        self.collector.tokens.put(&self.key, token, Duration::from_secs(lifetime));
        self.client.set_header("Authorization", format!("Bearer {token}"));
        Ok(())
    }

    /// Un GET : `Ok(None)` si le rôle n'y donne pas accès (403).
    async fn get(&mut self, path: &str) -> Result<Option<Value>, ProbeError> {
        let (mut status, mut body) = self.client.get(path).await?;
        if status == StatusCode::UNAUTHORIZED {
            // Jeton gardé révoqué (redémarrage du service) : une seule reconnexion.
            self.collector.tokens.forget(&self.key);
            self.login(true).await?;
            (status, body) = self.client.get(path).await?;
        }
        match status.as_u16() {
            200..=299 => self.client.decode(&body, path).map(Some),
            403 => Ok(None),
            400 if body.contains("x-api-version") || body.contains("version") => {
                Err(ProbeError::Config(format!(
                    "Veeam refused the API version on {path}: set the API version option to one \
                     this server supports (1.1-rev0 for v12.0 and later, 1.3-rev1 for v13)."
                )))
            }
            _ => Err(self.client.status_error(status, &body, path)),
        }
    }
}

#[async_trait]
impl Collector for VeeamCollector {
    fn kind(&self) -> &'static str {
        "veeam"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let mut session = self.session(target)?;
        session.login(false).await?;
        let jobs = session.get(JOBS).await?.ok_or_else(|| {
            ProbeError::Auth(
                "Veeam refused to list the jobs (403): give the account the Veeam Backup Viewer \
                 role under Users and Roles."
                    .to_string(),
            )
        })?;
        let now = now_ms();
        let since = chrono::DateTime::from_timestamp_millis(now - 24 * 3600 * 1000)
            .unwrap_or_default()
            .format("%Y-%m-%dT%H:%M:%SZ");
        let sessions_path = format!("/api/v1/sessions?createdAfterFilter={since}&limit=2000");
        let replies = Replies {
            server: session.get(SERVER_INFO).await.ok().flatten(),
            jobs,
            sessions: section(session.get(&sessions_path).await)?,
            repositories: section(session.get(REPOSITORIES).await)?,
            license: section(session.get(LICENSE).await)?,
        };
        Ok(samples(&replies, now))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let mut session = self.session(target)?;
        session.login(false).await?;
        session.get(JOBS).await?;
        Ok(Some("veeam".to_string()))
    }
}

/// Une lecture facultative : refusée, elle est `Refused` ; une panne réelle
/// (injoignable, délai) fait échouer toute l'interrogation.
fn section(outcome: Result<Option<Value>, ProbeError>) -> Result<Section, ProbeError> {
    match outcome {
        Ok(Some(value)) => Ok(Section::Read(value)),
        Ok(None) => Ok(Section::Refused),
        Err(error) if error.means_down() => Err(error),
        Err(error) => {
            tracing::debug!(%error, "Veeam : lecture facultative indisponible");
            Ok(Section::Refused)
        }
    }
}

impl VeeamCollector {
    fn session(&self, target: &Target) -> Result<Session<'_>, ProbeError> {
        let (username, password) = match &target.credential {
            Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
                (username.trim().to_string(), password.clone())
            }
            other => {
                return Err(ProbeError::Config(format!(
                    "Veeam expects a user name and password, configured: {other}"
                )));
            }
        };
        let api_version = tag(target, "api_version").unwrap_or(DEFAULT_API_VERSION);
        if !api_version.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') {
            return Err(ProbeError::Config(format!("Invalid API version \"{api_version}\"")));
        }
        let client = RestClient::for_target(target, "https", DEFAULT_PORT, "Veeam")?
            .with_header("x-api-version", api_version);
        let key = format!("{}:{username}", target.id);
        Ok(Session { collector: self, client, username, password, key })
    }
}

pub enum Section {
    Read(Value),
    Refused,
}

pub struct Replies {
    pub server: Option<Value>,
    pub jobs: Value,
    pub sessions: Section,
    pub repositories: Section,
    pub license: Section,
}

fn data(value: &Value) -> impl Iterator<Item = &Value> {
    value.get("data").and_then(Value::as_array).into_iter().flatten()
}

fn is(value: Option<&str>, expected: &str) -> bool {
    value.is_some_and(|v| v.eq_ignore_ascii_case(expected))
}

pub fn samples(r: &Replies, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut out = Vec::new();
    if let Some(version) = r.server.as_ref().and_then(|s| text(s, "buildVersion")) {
        out.push(g("veeam_version_info", 1.0).with_label("version", version));
    }

    let (mut jobs, mut failed, mut warning, mut running, mut disabled) = (0, 0, 0, 0, 0);
    for job in data(&r.jobs) {
        let Some(name) = text(job, "name") else { continue };
        jobs += 1;
        let status = text(job, "status");
        let result = text(job, "lastResult").unwrap_or("None");
        let enabled = !is(status, "Disabled");
        let is_running = is(status, "Running") || is(status, "Starting") || is(status, "Stopping");
        // Un travail désactivé garde son dernier résultat, qui ne dit plus rien.
        let is_failed = enabled && result.eq_ignore_ascii_case("Failed");
        let is_warning = enabled && result.eq_ignore_ascii_case("Warning");
        failed += usize::from(is_failed);
        warning += usize::from(is_warning);
        running += usize::from(is_running);
        disabled += usize::from(!enabled);
        if jobs > MAX_NAMED {
            continue;
        }
        let kind = text(job, "type").unwrap_or("Unknown");
        let named = |sample: Sample| sample.with_label("job", name).with_label("type", kind);
        out.push(named(flag("veeam_job_enabled", enabled, ts_ms)));
        out.push(named(flag("veeam_job_running", is_running, ts_ms)));
        out.push(named(flag("veeam_job_failed", is_failed, ts_ms)));
        out.push(named(flag("veeam_job_warning", is_warning, ts_ms)));
        out.push(named(
            g("veeam_job_result_info", 1.0).with_label("result", result.to_ascii_lowercase()),
        ));
        if let Some(age) =
            job.get("lastRun").and_then(Value::as_str).and_then(|t| age_seconds(t, ts_ms))
        {
            out.push(named(g("veeam_job_last_run_age_seconds", age)));
        }
        if is_running && let Some(progress) = number(job.get("progressPercent")) {
            out.push(named(g("veeam_job_progress_percent", progress)));
        }
    }
    out.push(g("veeam_jobs", jobs as f64));
    out.push(g("veeam_jobs_failed", failed as f64));
    out.push(g("veeam_jobs_warning", warning as f64));
    out.push(g("veeam_jobs_running", running as f64));
    out.push(g("veeam_jobs_disabled", disabled as f64));

    let readable = |section: &Section, name: &str| {
        flag("veeam_section_readable", matches!(section, Section::Read(_)), ts_ms)
            .with_label("section", name)
    };
    out.push(readable(&r.sessions, "sessions"));
    out.push(readable(&r.repositories, "repositories"));
    out.push(readable(&r.license, "license"));

    if let Section::Read(sessions) = &r.sessions {
        let (mut total, mut failed, mut warning) = (0, 0, 0);
        for session in data(sessions) {
            total += 1;
            let result = session.pointer("/result/result").and_then(Value::as_str);
            failed += usize::from(is(result, "Failed"));
            warning += usize::from(is(result, "Warning"));
        }
        out.push(g("veeam_sessions_24h", total as f64));
        out.push(g("veeam_sessions_failed_24h", failed as f64));
        out.push(g("veeam_sessions_warning_24h", warning as f64));
    }

    if let Section::Read(repositories) = &r.repositories {
        for repository in data(repositories).take(MAX_NAMED) {
            let Some(name) = text(repository, "name") else { continue };
            let kind = text(repository, "type").unwrap_or("Unknown");
            let named =
                |sample: Sample| sample.with_label("repository", name).with_label("type", kind);
            let capacity = number(repository.get("capacityGB")).map(|v| v * GIB);
            let free = number(repository.get("freeGB")).map(|v| v * GIB);
            if let Some(used) = number(repository.get("usedSpaceGB")) {
                out.push(named(g("veeam_repository_used_bytes", used * GIB)));
            }
            // Un dépôt objet (S3, Azure) n'a pas de capacité : 0 n'est pas « plein ».
            if let (Some(capacity), Some(free)) = (capacity, free)
                && capacity > 0.0
            {
                out.push(named(g("veeam_repository_capacity_bytes", capacity)));
                out.push(named(g("veeam_repository_free_bytes", free)));
                out.push(named(g(
                    "veeam_repository_used_percent",
                    100.0 * (capacity - free) / capacity,
                )));
            }
        }
    }

    if let Section::Read(license) = &r.license {
        let status = text(license, "status").unwrap_or("Unknown");
        out.push(flag("veeam_license_valid", status.eq_ignore_ascii_case("Valid"), ts_ms));
        out.push(
            g("veeam_license_info", 1.0)
                .with_label("status", status)
                .with_label("type", text(license, "type").unwrap_or(""))
                .with_label("edition", text(license, "edition").unwrap_or("")),
        );
        // Une licence perpétuelle n'expire pas : pas de série plutôt qu'une date nulle.
        if let Some(left) = license
            .get("expirationDate")
            .and_then(Value::as_str)
            .and_then(|t| seconds_until(t, ts_ms))
        {
            out.push(g("veeam_license_expiry_seconds", left));
        }
        if let Some(left) = license
            .get("supportExpirationDate")
            .and_then(Value::as_str)
            .and_then(|t| seconds_until(t, ts_ms))
        {
            out.push(g("veeam_support_expiry_seconds", left));
        }
        if let Some(summary) = license.get("instanceLicenseSummary") {
            if let Some(licensed) = number(summary.get("licensedInstancesNumber")) {
                out.push(g("veeam_license_instances_licensed", licensed));
            }
            if let Some(used) = number(summary.get("usedInstancesNumber")) {
                out.push(g("veeam_license_instances_used", used));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::Router;
    use axum::extract::Form;
    use axum::http::{HeaderMap, StatusCode, Uri};
    use axum::routing::{get, post};

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    // Réponses construites d'après la référence de l'API REST de VBR 12
    // (modèles `JobStatesResult`, `SessionsResult`, `RepositoryStatesResult`,
    // `LicenseModel`, `ServerInfoModel`) : un travail réussi, un en échec, une
    // copie en cours terminée la veille en avertissement, un travail désactivé
    // en échec, une stratégie d'agent jamais lancée ; un dépôt NAS plein à
    // 96 %, un dépôt S3 sans capacité ; une licence qui expire dans 20 jours.
    const TOKEN: &str = include_str!("testdata/token.json");
    const SERVER_JSON: &str = include_str!("testdata/server_info.json");
    const JOBS_JSON: &str = include_str!("testdata/jobs_states.json");
    const SESSIONS_JSON: &str = include_str!("testdata/sessions.json");
    const REPOS_JSON: &str = include_str!("testdata/repositories_states.json");
    const LICENSE_JSON: &str = include_str!("testdata/license.json");
    const FORBIDDEN: &str = include_str!("testdata/error_403.json");
    const BAD_LOGIN: &str = include_str!("testdata/error_401.json");

    fn json(raw: &str) -> Value {
        serde_json::from_str(raw).unwrap()
    }

    fn now() -> i64 {
        chrono::DateTime::parse_from_rfc3339("2026-09-30T09:00:00+02:00")
            .unwrap()
            .timestamp_millis()
    }

    #[test]
    fn travaux_sessions_depots_licence() {
        let replies = Replies {
            server: Some(json(SERVER_JSON)),
            jobs: json(JOBS_JSON),
            sessions: Section::Read(json(SESSIONS_JSON)),
            repositories: Section::Read(json(REPOS_JSON)),
            license: Section::Read(json(LICENSE_JSON)),
        };
        let s = samples(&replies, now());
        assert_eq!(find(&s, "veeam_version_info", &[]).unwrap().labels["version"], "12.3.1.1139");
        assert_eq!(value(&s, "veeam_jobs", &[]), 5.0);
        assert_eq!(value(&s, "veeam_jobs_failed", &[]), 1.0, "the disabled job does not count");
        assert_eq!(value(&s, "veeam_job_failed", &[("job", "File server")]), 1.0);
        assert_eq!(value(&s, "veeam_job_failed", &[("job", "Old SQL job")]), 0.0);
        assert_eq!(value(&s, "veeam_job_warning", &[("job", "Offsite copy")]), 1.0);
        assert_eq!(value(&s, "veeam_job_running", &[("job", "Offsite copy")]), 1.0);
        assert_eq!(value(&s, "veeam_job_progress_percent", &[("job", "Offsite copy")]), 41.0);
        let age = value(&s, "veeam_job_last_run_age_seconds", &[("job", "Daily VMs")]);
        assert!((age - (8.0 * 3600.0 - 12.31)).abs() < 0.01, "{age}");
        assert!(
            find(&s, "veeam_job_last_run_age_seconds", &[("job", "New laptop policy")]).is_none()
        );
        assert_eq!(value(&s, "veeam_sessions_failed_24h", &[]), 1.0);
        assert_eq!(value(&s, "veeam_sessions_warning_24h", &[]), 1.0);
        let nas = value(&s, "veeam_repository_used_percent", &[("repository", "NAS repository")]);
        assert!((nas - 96.0).abs() < 0.01, "{nas}");
        assert!(
            find(&s, "veeam_repository_used_percent", &[("repository", "Offsite S3")]).is_none()
        );
        assert_eq!(value(&s, "veeam_license_valid", &[]), 1.0);
        assert_eq!(value(&s, "veeam_license_expiry_seconds", &[]), 20.0 * 86_400.0 - 9.0 * 3600.0);
        assert_eq!(value(&s, "veeam_section_readable", &[("section", "license")]), 1.0);
    }

    #[test]
    fn une_licence_refusee_au_role_viewer_n_est_pas_une_panne() {
        let replies = Replies {
            server: None,
            jobs: json(JOBS_JSON),
            sessions: Section::Refused,
            repositories: Section::Read(json(REPOS_JSON)),
            license: Section::Refused,
        };
        let s = samples(&replies, now());
        assert_eq!(value(&s, "veeam_section_readable", &[("section", "license")]), 0.0);
        assert!(find(&s, "veeam_license_valid", &[]).is_none());
        assert!(find(&s, "veeam_sessions_24h", &[]).is_none());
    }

    #[tokio::test]
    async fn interrogation_complete() {
        let logins = Arc::new(AtomicUsize::new(0));
        let counted = logins.clone();
        let app = Router::new()
            .route(
                TOKEN_PATH,
                post(move |headers: HeaderMap, Form(form): Form<HashMap<String, String>>| {
                    let counted = counted.clone();
                    async move {
                        assert_eq!(headers.get("x-api-version").unwrap(), "1.1-rev0");
                        if form.get("grant_type").map(String::as_str) != Some("password")
                            || form.get("password").map(String::as_str) != Some("s3cret")
                        {
                            return (StatusCode::BAD_REQUEST, BAD_LOGIN);
                        }
                        counted.fetch_add(1, Ordering::SeqCst);
                        (StatusCode::OK, TOKEN)
                    }
                }),
            )
            .fallback(get(|headers: HeaderMap, uri: Uri| async move {
                if headers.get("authorization").and_then(|v| v.to_str().ok())
                    != Some("Bearer eyJhbGciOiJIUzI1NiJ9.dummy.signature")
                {
                    return (StatusCode::UNAUTHORIZED, "");
                }
                match uri.path() {
                    "/api/v1/jobs/states" => (StatusCode::OK, JOBS_JSON),
                    "/api/v1/sessions" => {
                        assert!(uri.query().unwrap().contains("createdAfterFilter=2026-"));
                        (StatusCode::OK, SESSIONS_JSON)
                    }
                    "/api/v1/serverInfo" => (StatusCode::OK, SERVER_JSON),
                    "/api/v1/backupInfrastructure/repositories/states" => {
                        (StatusCode::OK, REPOS_JSON)
                    }
                    // Rôle Viewer : la licence est réservée aux administrateurs.
                    _ => (StatusCode::FORBIDDEN, FORBIDDEN),
                }
            }));
        let address = serve(app).await;
        let collector = VeeamCollector::new();
        let mut target = cible("veeam", &format!("http://{address}"), &[]);
        target.credential = Credential::UsernamePassword {
            username: r"VBR01\dumbmonit".into(),
            password: "s3cret".into(),
        };
        let s = collector.probe(&target).await.unwrap();
        assert_eq!(value(&s, "veeam_jobs_failed", &[]), 1.0);
        assert_eq!(value(&s, "veeam_section_readable", &[("section", "license")]), 0.0);
        assert_eq!(value(&s, "veeam_sessions_24h", &[]), 4.0);
        collector.probe(&target).await.unwrap();
        assert_eq!(logins.load(Ordering::SeqCst), 1, "the token is kept");

        target.credential =
            Credential::UsernamePassword { username: "other".into(), password: "nope".into() };
        let error = collector.probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
    }
}
