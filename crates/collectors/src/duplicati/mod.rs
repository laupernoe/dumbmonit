//! Duplicati, par l'API de son serveur (port 8200, `/api/v1`).
//!
//! # Authentification : Duplicati 2.1 et plus
//!
//! Duplicati n'a pas de compte, seulement le mot de passe de l'interface web.
//! Depuis la 2.1, ce mot de passe s'échange contre un jeton d'accès de courte
//! durée : `POST /api/v1/auth/login` avec `{"Password": "…"}` répond
//! `{"AccessToken": "…"}`, à renvoyer ensuite en `Authorization: Bearer`. Le
//! jeton est gardé une dizaine de minutes (sa durée exacte n'est pas
//! documentée) ; un 401 en cours de route déclenche une seule reconnexion.
//!
//! Les versions 2.0 s'authentifiaient par `/login.cgi` (nonce et sel,
//! `salted-sha256`) et des cookies de session : ce mode n'est pas pris en
//! charge. Un serveur 2.0 répond 404 sur la route de connexion, ce que le
//! collecteur traduit en erreur de réglage qui le dit.
//!
//! # Lectures
//!
//! * `GET /api/v1/backups` — obligatoire : chaque tâche avec sa planification et
//!   ses métadonnées (`LastBackupDate`, `LastBackupFinished`, `LastBackupDuration`,
//!   `LastErrorDate`, `LastErrorMessage`, `SourceFilesSize`, `TargetFilesSize`,
//!   `BackupListCount`…). Les dates y sont au format ISO « de base »
//!   (`20261025T020512Z`), les nombres des textes.
//! * `GET /api/v1/serverstate` — facultative : tâche en cours (`ActiveTask`) et
//!   serveur en pause (`ProgramState`).
//! * `GET /api/v1/progressstate` — facultative : avancement de la tâche en cours.
//! * `GET /api/v1/backup/{id}/log?pagesize=…` — facultative, une requête par
//!   tâche (les 50 premières) : le résultat de la dernière exécution
//!   (`ParsedResult` : `Success`, `Warning`, `Error`, `Fatal`) et ses compteurs
//!   d'avertissements et d'erreurs, que les métadonnées ne portent pas. Sans
//!   elle, l'échec se déduit de `LastErrorDate` et `LastErrorMessage`.
//!
//! `GET /api/v1/backup/{id}` n'est pas lu : la liste porte déjà les mêmes
//! métadonnées pour toutes les tâches en un seul appel.
//!
//! Une lecture facultative refusée ou incompréhensible ne fait pas échouer le
//! reste ; seule une panne réelle (injoignable, délai dépassé) interrompt la
//! collecte.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `http` | Protocole de l'API. |
//! | `port` | `8200` | Port de l'API. |
//! | `insecure_tls` | `false` | Accepte un certificat auto-signé (si le serveur est en HTTPS). |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use reqwest::StatusCode;
use serde_json::{Value, json};

use crate::rest::{MAX_NAMED, RestClient, TokenCache, flag, gauge, now_ms, number, text};

pub const DEFAULT_PORT: u16 = 8200;
const LOGIN_PATH: &str = "/api/v1/auth/login";
const BACKUPS: &str = "/api/v1/backups";
const SERVER_STATE: &str = "/api/v1/serverstate";
const PROGRESS_STATE: &str = "/api/v1/progressstate";
/// Durée pendant laquelle un jeton est gardé (le serveur n'annonce pas la sienne).
const TOKEN_LIFETIME: Duration = Duration::from_secs(10 * 60);
/// Au plus autant de journaux lus par interrogation.
const MAX_LOGS: usize = 50;
/// Longueur maximale du message d'erreur publié en étiquette.
const MAX_MESSAGE: usize = 200;

#[derive(Default)]
pub struct DuplicatiCollector {
    tokens: TokenCache,
}

impl DuplicatiCollector {
    pub fn new() -> Self {
        Self::default()
    }

    fn session(&self, target: &Target) -> Result<Session<'_>, ProbeError> {
        let password = match &target.credential {
            Credential::ApiToken { token } if !token.is_empty() => token.clone(),
            other => {
                return Err(ProbeError::Config(format!(
                    "Duplicati expects the web interface password, configured: {other}"
                )));
            }
        };
        let client = RestClient::for_target(target, "http", DEFAULT_PORT, "Duplicati")?;
        Ok(Session { collector: self, client, password, key: target.id.to_string() })
    }
}

struct Session<'a> {
    collector: &'a DuplicatiCollector,
    client: RestClient,
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
        let body = json!({ "Password": self.password, "RememberMe": false });
        let (status, reply) = self.client.post_json(LOGIN_PATH, &body).await?;
        match status.as_u16() {
            400 | 401 | 403 => {
                return Err(ProbeError::Auth(format!(
                    "Duplicati refused the password ({status}). Check the web interface \
                     password of the server."
                )));
            }
            404 => {
                return Err(ProbeError::Config(format!(
                    "Duplicati has no {LOGIN_PATH}: this collector needs Duplicati 2.1 or later \
                     (2.0 signs in through /login.cgi, which is not supported)."
                )));
            }
            _ => {}
        }
        if !status.is_success() {
            return Err(self.client.status_error(status, &reply, LOGIN_PATH));
        }
        let reply: Value = self.client.decode(&reply, LOGIN_PATH)?;
        let token = text(&reply, "AccessToken")
            .ok_or_else(|| ProbeError::Protocol("Duplicati returned no AccessToken".to_string()))?;
        self.collector.tokens.put(&self.key, token, TOKEN_LIFETIME);
        self.client.set_header("Authorization", format!("Bearer {token}"));
        Ok(())
    }

    /// Un GET : `Ok(None)` si l'accès est refusé (403).
    async fn get(&mut self, path: &str) -> Result<Option<Value>, ProbeError> {
        let (mut status, mut body) = self.client.get(path).await?;
        if status == StatusCode::UNAUTHORIZED {
            // Jeton expiré ou révoqué : une seule reconnexion.
            self.collector.tokens.forget(&self.key);
            self.login(true).await?;
            (status, body) = self.client.get(path).await?;
        }
        match status.as_u16() {
            200..=299 => self.client.decode(&body, path).map(Some),
            403 => Ok(None),
            _ => Err(self.client.status_error(status, &body, path)),
        }
    }

    /// Une lecture facultative : tout échec hors panne réelle donne `None`.
    async fn optional(&mut self, path: &str) -> Result<Option<Value>, ProbeError> {
        match self.get(path).await {
            Ok(value) => Ok(value),
            Err(error) if error.means_down() => Err(error),
            Err(error) => {
                tracing::debug!(%error, path, "Duplicati : lecture facultative indisponible");
                Ok(None)
            }
        }
    }
}

#[async_trait]
impl Collector for DuplicatiCollector {
    fn kind(&self) -> &str {
        "duplicati"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let mut session = self.session(target)?;
        session.login(false).await?;
        let backups = session.get(BACKUPS).await?.ok_or_else(|| {
            ProbeError::Auth("Duplicati refused to list the backups (403).".to_string())
        })?;
        let server = session.optional(SERVER_STATE).await?;
        let progress = session.optional(PROGRESS_STATE).await?;
        let mut results = HashMap::new();
        for id in backup_entries(&backups).filter_map(backup_id).take(MAX_LOGS) {
            let path = format!("/api/v1/backup/{id}/log?pagesize=5");
            if let Some(log) = session.optional(&path).await?
                && let Some(result) = last_result(&log)
            {
                results.insert(id, result);
            }
        }
        Ok(samples(&Replies { backups, server, progress, results }, now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let mut session = self.session(target)?;
        session.login(false).await?;
        session.get(BACKUPS).await?;
        Ok(Some("duplicati".to_string()))
    }
}

/// Résultat de la dernière exécution d'une tâche, lu dans son journal.
#[derive(Debug, Clone, PartialEq)]
pub struct LastResult {
    /// `Success`, `Warning`, `Error` ou `Fatal`.
    pub result: String,
    pub warnings: f64,
    pub errors: f64,
}

pub struct Replies {
    /// Réponse de `/api/v1/backups`.
    pub backups: Value,
    pub server: Option<Value>,
    pub progress: Option<Value>,
    /// Dernier résultat connu, par identifiant de tâche.
    pub results: HashMap<String, LastResult>,
}

/// Le premier journal de la page dont le message est un résultat d'opération :
/// Duplicati y met le résultat sérialisé en JSON, `ParsedResult` en tête.
pub fn last_result(log: &Value) -> Option<LastResult> {
    log.as_array()?.iter().find_map(|entry| {
        let message = entry.get("Message").and_then(Value::as_str)?;
        let result: Value = serde_json::from_str(message).ok()?;
        let parsed = text(&result, "ParsedResult")?;
        Some(LastResult {
            result: parsed.to_string(),
            warnings: number(result.get("WarningsActualLength")).unwrap_or(0.0),
            errors: number(result.get("ErrorsActualLength")).unwrap_or(0.0),
        })
    })
}

fn backup_entries(backups: &Value) -> impl Iterator<Item = &Value> {
    backups.as_array().into_iter().flatten()
}

/// La description d'une tâche : l'objet `Backup` d'une entrée `{Backup, Schedule}`.
fn backup_of(entry: &Value) -> &Value {
    entry.get("Backup").unwrap_or(entry)
}

/// Identifiant d'une entrée de la liste (texte ou nombre selon la version).
fn backup_id(entry: &Value) -> Option<String> {
    match backup_of(entry).get("ID")? {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Identifiant de la tâche en cours, d'après `serverstate`.
fn active_backup(server: &Value) -> Option<String> {
    let task = server.get("ActiveTask")?;
    match task.get("Item2").or_else(|| task.get("BackupID"))? {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Date de métadonnée Duplicati (`20261025T020512Z`, ou RFC 3339) en
/// millisecondes Unix.
fn metadata_ms(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(raw, "%Y%m%dT%H%M%SZ") {
        return Some(parsed.and_utc().timestamp_millis());
    }
    chrono::DateTime::parse_from_rfc3339(raw).ok().map(|dt| dt.timestamp_millis())
}

/// Durée `hh:mm:ss.fffffff` ou `j.hh:mm:ss`, en secondes.
fn timespan_seconds(raw: &str) -> Option<f64> {
    let mut parts = raw.trim().split(':');
    let (hours, minutes, seconds) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() {
        return None;
    }
    let (days, hours) = match hours.split_once('.') {
        Some((days, hours)) => (days.parse::<f64>().ok()?, hours.parse::<f64>().ok()?),
        None => (0.0, hours.parse::<f64>().ok()?),
    };
    let total = days * 86_400.0
        + hours * 3600.0
        + minutes.parse::<f64>().ok()? * 60.0
        + seconds.parse::<f64>().ok()?;
    (total >= 0.0 && total.is_finite()).then_some(total)
}

pub fn samples(r: &Replies, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut out = Vec::new();

    if let Some(server) = &r.server {
        let paused = text(server, "ProgramState").is_some_and(|s| s.eq_ignore_ascii_case("Paused"));
        out.push(flag("duplicati_server_paused", paused, ts_ms));
    }
    let active = r.server.as_ref().and_then(active_backup);

    let (mut total, mut failed, mut running) = (0, 0, 0);
    for entry in backup_entries(&r.backups) {
        let backup = backup_of(entry);
        let Some(name) = text(backup, "Name") else { continue };
        let id = backup_id(entry).unwrap_or_default();
        total += 1;
        let meta = backup.get("Metadata");
        let field = |key: &str| meta.and_then(|m| text(m, key));
        let date = |key: &str| field(key).and_then(metadata_ms);

        let last_date = date("LastBackupDate");
        let finished = date("LastBackupFinished");
        let error_at = date("LastErrorDate");
        // Une erreur postérieure (à quelques secondes près) à la fin de la dernière
        // exécution : la dernière exécution a échoué. Une réussite ultérieure
        // l'emporte, même si Duplicati garde l'ancienne erreur dans ses métadonnées.
        let metadata_failed = error_at.is_some_and(|e| {
            finished.or(last_date).is_none_or(|run| e + 5_000 >= run)
        });
        let known = r.results.get(&id);
        let is_failed = match known {
            Some(k) => {
                k.result.eq_ignore_ascii_case("Error") || k.result.eq_ignore_ascii_case("Fatal")
            }
            None => metadata_failed,
        };
        let is_running = !id.is_empty() && active.as_deref() == Some(id.as_str());
        failed += usize::from(is_failed);
        running += usize::from(is_running);
        if total > MAX_NAMED {
            continue;
        }
        let named = |sample: Sample| {
            sample.with_label("backup", name).with_label("backup_id", id.as_str())
        };
        out.push(named(flag("duplicati_backup_failed", is_failed, ts_ms)));
        out.push(named(flag("duplicati_backup_running", is_running, ts_ms)));
        let scheduled = entry.get("Schedule").is_some_and(|s| text(s, "Time").is_some());
        out.push(named(flag("duplicati_backup_scheduled", scheduled, ts_ms)));
        // La date de la dernière réussite : `LastBackupDate`, ou à défaut la fin de la
        // dernière exécution quand elle n'a pas échoué. Absente d'une tâche jamais réussie.
        let success = last_date.or(if is_failed { None } else { finished });
        if let Some(success) = success {
            out.push(named(g(
                "duplicati_backup_last_success_age_seconds",
                ((ts_ms - success) as f64 / 1000.0).max(0.0),
            )));
        }
        if let Some(duration) = field("LastBackupDuration").and_then(timespan_seconds) {
            out.push(named(g("duplicati_backup_last_duration_seconds", duration)));
        }
        for (key, metric) in [
            ("SourceFilesSize", "duplicati_backup_source_bytes"),
            ("SourceFilesCount", "duplicati_backup_source_files"),
            ("TargetFilesSize", "duplicati_backup_target_bytes"),
            ("BackupListCount", "duplicati_backup_versions"),
        ] {
            if let Some(value) = meta.and_then(|m| number(m.get(key))) {
                out.push(named(g(metric, value)));
            }
        }
        if let Some(k) = known {
            out.push(named(g("duplicati_backup_last_result_info", 1.0).with_label(
                "result",
                k.result.to_ascii_lowercase(),
            )));
            out.push(named(g("duplicati_backup_last_warnings", k.warnings)));
            out.push(named(g("duplicati_backup_last_errors", k.errors)));
        }
        if is_failed && let Some(message) = field("LastErrorMessage") {
            let message: String = message.chars().take(MAX_MESSAGE).collect();
            let info = g("duplicati_backup_last_error_info", 1.0).with_label("message", message);
            out.push(named(info));
        }
        if is_running
            && let Some(progress) = r.progress.as_ref().filter(|p| {
                backup_id_of_progress(p).as_deref() == Some(id.as_str())
            })
            && let Some(fraction) = number(progress.get("OverallProgress"))
        {
            let percent = (fraction * 100.0).clamp(0.0, 100.0);
            out.push(named(g("duplicati_backup_progress_percent", percent)));
        }
    }
    out.push(g("duplicati_backups", total as f64));
    out.push(g("duplicati_backups_failed", failed as f64));
    out.push(g("duplicati_backups_running", running as f64));
    out
}

/// Identifiant de tâche d'un `progressstate` (`BackupID`, texte ou nombre).
fn backup_id_of_progress(progress: &Value) -> Option<String> {
    match progress.get("BackupID")? {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::Router;
    use axum::extract::Json;
    use axum::http::{HeaderMap, StatusCode, Uri};
    use axum::routing::{get, post};

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    // Réponses construites d'après l'API du serveur Duplicati 2.1 : « Documents »
    // sauvegardé cette nuit (avec deux avertissements) et en cours d'exécution,
    // « Photos » en échec depuis le 24 (hôte introuvable) après sa dernière
    // réussite du 18, « Laptop » sans planning ni exécution.
    const LOGIN: &str = include_str!("testdata/login.json");
    const LOGIN_REFUSED: &str = include_str!("testdata/login_refused.json");
    const BACKUPS_JSON: &str = include_str!("testdata/backups.json");
    const IDLE: &str = include_str!("testdata/serverstate_idle.json");
    const RUNNING: &str = include_str!("testdata/serverstate_running.json");
    const PROGRESS: &str = include_str!("testdata/progressstate.json");
    const LOG_1: &str = include_str!("testdata/log_1.json");
    const LOG_2: &str = include_str!("testdata/log_2.json");
    const TOKEN: &str = "Bearer eyJhbGciOiJIUzI1NiJ9.dummy.signature";

    fn json(raw: &str) -> Value {
        serde_json::from_str(raw).unwrap()
    }

    fn now() -> i64 {
        chrono::DateTime::parse_from_rfc3339("2026-10-26T01:00:00Z").unwrap().timestamp_millis()
    }

    fn replies(server: &str, with_results: bool) -> Replies {
        let mut results = HashMap::new();
        if with_results {
            results.insert("1".to_string(), last_result(&json(LOG_1)).unwrap());
            results.insert("2".to_string(), last_result(&json(LOG_2)).unwrap());
        }
        Replies {
            backups: json(BACKUPS_JSON),
            server: Some(json(server)),
            progress: Some(json(PROGRESS)),
            results,
        }
    }

    #[test]
    fn taches_dernier_resultat_et_execution_en_cours() {
        let s = samples(&replies(RUNNING, true), now());
        assert_eq!(value(&s, "duplicati_backups", &[]), 3.0);
        assert_eq!(value(&s, "duplicati_backups_failed", &[]), 1.0);
        assert_eq!(value(&s, "duplicati_backups_running", &[]), 1.0);
        assert_eq!(value(&s, "duplicati_server_paused", &[]), 0.0);

        let documents = [("backup", "Documents to B2")];
        let failed = value(&s, "duplicati_backup_failed", &documents);
        assert_eq!(failed, 0.0, "a warning is not a failure");
        assert_eq!(value(&s, "duplicati_backup_running", &documents), 1.0);
        assert_eq!(value(&s, "duplicati_backup_last_success_age_seconds", &documents), 82_488.0);
        let duration = value(&s, "duplicati_backup_last_duration_seconds", &documents);
        assert!((duration - 312.3456789).abs() < 1e-6, "{duration}");
        assert_eq!(value(&s, "duplicati_backup_source_bytes", &documents), 5_368_709_120.0);
        assert_eq!(value(&s, "duplicati_backup_versions", &documents), 12.0);
        assert_eq!(value(&s, "duplicati_backup_last_warnings", &documents), 2.0);
        assert_eq!(
            find(&s, "duplicati_backup_last_result_info", &documents).unwrap().labels["result"],
            "warning"
        );
        assert_eq!(value(&s, "duplicati_backup_progress_percent", &documents), 50.0);
        assert_eq!(value(&s, "duplicati_backup_scheduled", &documents), 1.0);

        let photos = [("backup", "Photos to NAS")];
        assert_eq!(value(&s, "duplicati_backup_failed", &photos), 1.0);
        assert_eq!(value(&s, "duplicati_backup_last_success_age_seconds", &photos), 683_760.0);
        assert_eq!(value(&s, "duplicati_backup_last_errors", &photos), 1.0);
        assert_eq!(
            find(&s, "duplicati_backup_last_error_info", &photos).unwrap().labels["message"],
            "Failed to connect: No such host is known"
        );
        assert!(find(&s, "duplicati_backup_progress_percent", &photos).is_none());

        let laptop = [("backup", "Laptop manual")];
        assert_eq!(value(&s, "duplicati_backup_scheduled", &laptop), 0.0);
        assert_eq!(value(&s, "duplicati_backup_failed", &laptop), 0.0);
        assert!(find(&s, "duplicati_backup_last_success_age_seconds", &laptop).is_none());
    }

    #[test]
    fn sans_journal_l_echec_se_deduit_des_metadonnees() {
        let s = samples(&replies(IDLE, false), now());
        assert_eq!(value(&s, "duplicati_backups_failed", &[]), 1.0);
        assert_eq!(value(&s, "duplicati_backups_running", &[]), 0.0);
        assert_eq!(value(&s, "duplicati_backup_failed", &[("backup", "Photos to NAS")]), 1.0);
        assert_eq!(value(&s, "duplicati_backup_failed", &[("backup", "Documents to B2")]), 0.0);
        assert!(find(&s, "duplicati_backup_last_result_info", &[]).is_none());
        assert!(find(&s, "duplicati_backup_progress_percent", &[]).is_none());
    }

    #[test]
    fn les_dates_et_les_durees_de_duplicati() {
        assert_eq!(metadata_ms("20261025T020512Z"), metadata_ms("2026-10-25T02:05:12Z"));
        assert!(metadata_ms("hier").is_none());
        assert_eq!(timespan_seconds("00:05:12.5"), Some(312.5));
        assert_eq!(timespan_seconds("1.02:00:00"), Some(93_600.0));
        assert_eq!(timespan_seconds("12"), None);
    }

    #[tokio::test]
    async fn interrogation_complete() {
        let logins = Arc::new(AtomicUsize::new(0));
        let counted = logins.clone();
        let app = Router::new()
            .route(
                LOGIN_PATH,
                post(move |Json(body): Json<Value>| {
                    let counted = counted.clone();
                    async move {
                        if body.get("Password").and_then(Value::as_str) != Some("s3cret") {
                            return (StatusCode::UNAUTHORIZED, LOGIN_REFUSED);
                        }
                        counted.fetch_add(1, Ordering::SeqCst);
                        (StatusCode::OK, LOGIN)
                    }
                }),
            )
            .fallback(get(|headers: HeaderMap, uri: Uri| async move {
                if headers.get("authorization").and_then(|v| v.to_str().ok()) != Some(TOKEN) {
                    return (StatusCode::UNAUTHORIZED, "");
                }
                match uri.path() {
                    "/api/v1/backups" => (StatusCode::OK, BACKUPS_JSON),
                    "/api/v1/serverstate" => (StatusCode::OK, RUNNING),
                    "/api/v1/progressstate" => (StatusCode::OK, PROGRESS),
                    "/api/v1/backup/1/log" => (StatusCode::OK, LOG_1),
                    "/api/v1/backup/2/log" => (StatusCode::OK, LOG_2),
                    // Une lecture facultative qui échoue ne casse rien.
                    _ => (StatusCode::NOT_FOUND, ""),
                }
            }));
        let address = serve(app).await;
        let collector = DuplicatiCollector::new();
        let mut target = cible("duplicati", &format!("http://{address}"), &[]);
        target.credential = Credential::ApiToken { token: "s3cret".into() };
        let s = collector.probe(&target).await.unwrap();
        assert_eq!(value(&s, "duplicati_backups_failed", &[]), 1.0);
        let documents = [("backup", "Documents to B2")];
        assert_eq!(value(&s, "duplicati_backup_progress_percent", &documents), 50.0);
        collector.probe(&target).await.unwrap();
        assert_eq!(logins.load(Ordering::SeqCst), 1, "the token is kept");

        target.credential = Credential::ApiToken { token: "wrong".into() };
        let target = Target { id: 2, ..target };
        let error = collector.probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());
    }

    #[tokio::test]
    async fn un_serveur_2_0_demande_une_mise_a_jour_sans_etre_en_panne() {
        let app = Router::new().fallback(|| async { (StatusCode::NOT_FOUND, "") });
        let address = serve(app).await;
        let collector = DuplicatiCollector::new();
        let mut target = cible("duplicati", &format!("http://{address}"), &[]);
        target.credential = Credential::ApiToken { token: "s3cret".into() };
        let error = collector.probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
        assert!(!error.means_down());

        target.credential = Credential::None;
        let error = collector.probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }
}
