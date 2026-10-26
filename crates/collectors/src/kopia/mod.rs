//! Kopia, par l'API REST de son serveur (`kopia server start`, port 51515).
//!
//! Deux lectures, toutes deux avec les identifiants du serveur (authentification
//! HTTP Basic, `--server-username` / `--server-password`) :
//!
//! * `GET /api/v1/repo/status` : le dépôt est-il connecté, son stockage, son
//!   chiffrement. Lecture obligatoire : c'est elle qui valide les identifiants ;
//! * `GET /api/v1/sources` : les sources de sauvegarde connues du serveur, chacune
//!   avec son état (`IDLE`, `PENDING`, `UPLOADING`, `PAUSED`, `REMOTE`), sa
//!   planification et son dernier instantané (`lastSnapshot`, avec les
//!   statistiques `stats`). Lue seulement si le dépôt est connecté : Kopia
//!   répond par une erreur sinon.
//!
//! `GET /api/v1/snapshots?…` (tous les instantanés d'une source) n'est pas lu :
//! le dernier instantané de chaque source figure déjà dans `/api/v1/sources`, et
//! interroger l'historique de chaque source multiplierait les appels sans rien
//! apporter aux alertes (échec, instantané trop ancien).
//!
//! # Ce que la sauvegarde « en échec » veut dire pour Kopia
//!
//! Kopia enregistre un instantané même quand certains fichiers n'ont pas pu être
//! lus : `errorCount` (ou `numFailed` du résumé du dossier racine) dit combien.
//! Une source dont la planification n'a pas pu s'exécuter n'a, elle, tout
//! simplement pas de nouvel instantané : c'est l'âge qui la trahit. Les deux
//! signaux sont donc publiés, et c'est la règle d'âge qui ne s'applique qu'aux
//! sources planifiées (une source manuelle n'a pas à être récente).
//!
//! Les sources `REMOTE` sont celles d'autres machines connues du dépôt : ce
//! serveur ne les sauvegarde pas, elles sont comptées mais jamais décrites.
//!
//! # Réglages, portés par les étiquettes de la cible
//!
//! | Étiquette | Défaut | Rôle |
//! |---|---|---|
//! | `scheme` | `https` | Protocole de l'API (`http` si le serveur tourne avec `--insecure`). |
//! | `port` | `51515` | Port de l'API. |
//! | `insecure_tls` | `false` | Accepte le certificat auto-signé de `--tls-generate-cert`. |
//! | `request_timeout_seconds` | `15` | Délai par requête HTTP. |

use async_trait::async_trait;
use base64::Engine;
use dumbmonit_proto::{Collector, Credential, ProbeError, Sample, Target};
use serde_json::Value;

use crate::rest::{MAX_NAMED, RestClient, age_seconds, flag, gauge, now_ms, number, text};

pub const DEFAULT_PORT: u16 = 51515;
const REPO_STATUS: &str = "/api/v1/repo/status";
const SOURCES: &str = "/api/v1/sources";

#[derive(Default)]
pub struct KopiaCollector;

impl KopiaCollector {
    pub fn new() -> Self {
        Self
    }

    fn client(&self, target: &Target) -> Result<RestClient, ProbeError> {
        let client = RestClient::for_target(target, "https", DEFAULT_PORT, "Kopia")?;
        match &target.credential {
            // Un serveur lancé avec `--without-password` n'attend rien.
            Credential::None => Ok(client),
            Credential::UsernamePassword { username, password } if !username.trim().is_empty() => {
                let pair = format!("{}:{password}", username.trim());
                let encoded = base64::engine::general_purpose::STANDARD.encode(pair.as_bytes());
                Ok(client.with_header("Authorization", format!("Basic {encoded}")))
            }
            other => Err(ProbeError::Config(format!(
                "Kopia expects the server user name and password, or nothing, configured: {other}"
            ))),
        }
    }
}

#[async_trait]
impl Collector for KopiaCollector {
    fn kind(&self) -> &str {
        "kopia"
    }

    async fn probe(&self, target: &Target) -> Result<Vec<Sample>, ProbeError> {
        let client = self.client(target)?;
        let status: Value = client.get_json(REPO_STATUS).await?;
        let connected = status.get("connected").and_then(Value::as_bool).unwrap_or(false);
        let sources = if connected { Some(client.get_json::<Value>(SOURCES).await?) } else { None };
        Ok(samples(&Replies { status, sources }, now_ms()))
    }

    async fn discover(&self, target: &Target) -> Result<Option<String>, ProbeError> {
        let client = self.client(target)?;
        let _: Value = client.get_json(REPO_STATUS).await?;
        Ok(Some("kopia".to_string()))
    }
}

pub struct Replies {
    pub status: Value,
    /// `None` quand le dépôt n'est pas connecté.
    pub sources: Option<Value>,
}

/// Un instant RFC 3339 (Kopia écrit les nanosecondes) en millisecondes Unix.
fn instant_ms(raw: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(raw.trim()).ok().map(|dt| dt.timestamp_millis())
}

/// Un champ présent et non vide, qu'il soit un texte ou une liste.
fn present(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Array(items)) => !items.is_empty(),
        Some(Value::String(s)) => !s.trim().is_empty(),
        _ => false,
    }
}

/// Vrai si Kopia déclenche lui-même les instantanés de cette source.
fn is_scheduled(entry: &Value) -> bool {
    let schedule = entry.get("schedule");
    let manual =
        schedule.and_then(|s| s.get("manual")).and_then(Value::as_bool).unwrap_or(false);
    if manual {
        return false;
    }
    present(entry.get("nextSnapshotTime"))
        || schedule.is_some_and(|s| {
            number(s.get("intervalSeconds")).is_some_and(|v| v > 0.0)
                || present(s.get("timeOfDay"))
                || present(s.get("cron"))
        })
}

/// `utilisateur@machine:/chemin`, la forme que Kopia affiche.
fn source_name(entry: &Value) -> Option<String> {
    let source = entry.get("source")?;
    let path = text(source, "path")?;
    let user = text(source, "userName").unwrap_or("?");
    let host = text(source, "host").unwrap_or("?");
    Some(format!("{user}@{host}:{path}"))
}

pub fn samples(r: &Replies, ts_ms: i64) -> Vec<Sample> {
    let g = |name: &str, value: f64| gauge(name, value, ts_ms);
    let mut out = Vec::new();

    let connected = r.status.get("connected").and_then(Value::as_bool).unwrap_or(false);
    out.push(flag("kopia_repo_connected", connected, ts_ms));
    if connected {
        let readonly = r.status.get("readonly").and_then(Value::as_bool).unwrap_or(false);
        out.push(flag("kopia_repo_readonly", readonly, ts_ms));
        out.push(
            g("kopia_repo_info", 1.0)
                .with_label("storage", text(&r.status, "storage").unwrap_or(""))
                .with_label("hash", text(&r.status, "hash").unwrap_or(""))
                .with_label("encryption", text(&r.status, "encryption").unwrap_or("")),
        );
    }

    let Some(sources) = &r.sources else { return out };
    let entries: Vec<&Value> =
        sources.get("sources").and_then(Value::as_array).into_iter().flatten().collect();
    let (mut total, mut remote, mut without, mut with_errors, mut uploading) = (0, 0, 0, 0, 0);
    for entry in entries {
        let status = text(entry, "status").unwrap_or("UNKNOWN");
        if status.eq_ignore_ascii_case("REMOTE") {
            remote += 1;
            continue;
        }
        let Some(name) = source_name(entry) else { continue };
        total += 1;
        let last = entry.get("lastSnapshot").filter(|v| v.is_object());
        let errors = last
            .map(|l| {
                number(l.pointer("/stats/errorCount"))
                    .or_else(|| number(l.pointer("/rootEntry/summ/numFailed")))
                    .unwrap_or(0.0)
            })
            .unwrap_or(0.0);
        let is_uploading = status.eq_ignore_ascii_case("UPLOADING");
        without += usize::from(last.is_none());
        with_errors += usize::from(errors > 0.0);
        uploading += usize::from(is_uploading);
        if total > MAX_NAMED {
            continue;
        }
        let named = |sample: Sample| sample.with_label("source", name.as_str());
        out.push(named(flag("kopia_source_has_snapshot", last.is_some(), ts_ms)));
        out.push(named(flag("kopia_source_scheduled", is_scheduled(entry), ts_ms)));
        out.push(named(flag("kopia_source_uploading", is_uploading, ts_ms)));
        out.push(named(g("kopia_source_status_info", 1.0).with_label("status", status)));
        let Some(last) = last else { continue };
        let start = text(last, "startTime");
        let end = text(last, "endTime");
        if let Some(age) = end.or(start).and_then(|t| age_seconds(t, ts_ms)) {
            out.push(named(g("kopia_source_last_snapshot_age_seconds", age)));
        }
        if let (Some(start), Some(end)) =
            (start.and_then(instant_ms), end.and_then(instant_ms))
            && end >= start
        {
            out.push(named(g(
                "kopia_source_last_snapshot_duration_seconds",
                (end - start) as f64 / 1000.0,
            )));
        }
        out.push(named(g("kopia_source_last_snapshot_errors", errors)));
        if let Some(size) = number(last.pointer("/stats/totalSize"))
            .or_else(|| number(last.pointer("/rootEntry/summ/size")))
        {
            out.push(named(g("kopia_source_last_snapshot_bytes", size)));
        }
        if let Some(files) = number(last.pointer("/stats/fileCount"))
            .or_else(|| number(last.pointer("/rootEntry/summ/files")))
        {
            out.push(named(g("kopia_source_last_snapshot_files", files)));
        }
    }
    out.push(g("kopia_sources", total as f64));
    out.push(g("kopia_sources_without_snapshot", without as f64));
    out.push(g("kopia_sources_with_errors", with_errors as f64));
    out.push(g("kopia_sources_uploading", uploading as f64));
    out.push(g("kopia_sources_remote", remote as f64));
    out
}

#[cfg(test)]
mod tests {
    use axum::Router;
    use axum::http::{HeaderMap, StatusCode, Uri};
    use axum::routing::get;

    use super::*;
    use crate::rest::test_support::{find, serve, value};
    use crate::uptime::tags::test_support::cible;

    // Réponses construites d'après les structures de l'API du serveur Kopia
    // (`serverapi.StatusResponse`, `serverapi.SourceStatus`, `snapshot.Manifest`) :
    // des photos sauvegardées hier soir, des documents en cours de téléversement
    // dont le dernier instantané a trois fichiers en erreur, une archive sans
    // instantané depuis six jours, une source manuelle sans instantané, et une
    // source d'une autre machine.
    const STATUS: &str = include_str!("testdata/repo_status.json");
    const DISCONNECTED: &str = include_str!("testdata/repo_status_disconnected.json");
    const SOURCES_JSON: &str = include_str!("testdata/sources.json");

    fn json(raw: &str) -> Value {
        serde_json::from_str(raw).unwrap()
    }

    fn now() -> i64 {
        chrono::DateTime::parse_from_rfc3339("2026-10-26T01:00:00Z").unwrap().timestamp_millis()
    }

    #[test]
    fn sources_instantanes_erreurs() {
        let replies = Replies { status: json(STATUS), sources: Some(json(SOURCES_JSON)) };
        let s = samples(&replies, now());
        assert_eq!(value(&s, "kopia_repo_connected", &[]), 1.0);
        assert_eq!(
            find(&s, "kopia_repo_info", &[]).unwrap().labels["storage"],
            "filesystem"
        );
        assert_eq!(value(&s, "kopia_sources", &[]), 4.0, "the remote source is not counted");
        assert_eq!(value(&s, "kopia_sources_remote", &[]), 1.0);
        assert_eq!(value(&s, "kopia_sources_without_snapshot", &[]), 1.0);
        assert_eq!(value(&s, "kopia_sources_with_errors", &[]), 1.0);
        assert_eq!(value(&s, "kopia_sources_uploading", &[]), 1.0);

        let photos = [("source", "root@nas:/data/photos")];
        let age = value(&s, "kopia_source_last_snapshot_age_seconds", &photos);
        assert!((age - 82_338.89).abs() < 0.5, "{age}");
        let duration = value(&s, "kopia_source_last_snapshot_duration_seconds", &photos);
        assert!((duration - 457.63).abs() < 0.01, "{duration}");
        assert_eq!(value(&s, "kopia_source_last_snapshot_bytes", &photos), 48_318_382_080.0);
        assert_eq!(value(&s, "kopia_source_last_snapshot_files", &photos), 15_234.0);
        assert_eq!(value(&s, "kopia_source_last_snapshot_errors", &photos), 0.0);
        assert_eq!(value(&s, "kopia_source_scheduled", &photos), 1.0);

        let documents = [("source", "root@nas:/data/documents")];
        assert_eq!(value(&s, "kopia_source_last_snapshot_errors", &documents), 3.0);
        assert_eq!(value(&s, "kopia_source_uploading", &documents), 1.0);
        assert_eq!(value(&s, "kopia_source_scheduled", &documents), 1.0, "timeOfDay");

        let archive = [("source", "root@nas:/data/archive")];
        assert_eq!(value(&s, "kopia_source_last_snapshot_age_seconds", &archive), 517_200.0);

        let scratch = [("source", "root@nas:/data/scratch")];
        assert_eq!(value(&s, "kopia_source_has_snapshot", &scratch), 0.0);
        assert_eq!(value(&s, "kopia_source_scheduled", &scratch), 0.0, "manual");
        assert!(find(&s, "kopia_source_last_snapshot_age_seconds", &scratch).is_none());
        assert!(find(&s, "kopia_source_has_snapshot", &[("source", "anna@laptop:/home/anna")])
            .is_none());
    }

    #[test]
    fn un_depot_deconnecte_ne_publie_pas_de_sources() {
        let replies = Replies { status: json(DISCONNECTED), sources: None };
        let s = samples(&replies, now());
        assert_eq!(value(&s, "kopia_repo_connected", &[]), 0.0);
        assert!(find(&s, "kopia_sources", &[]).is_none());
        assert!(find(&s, "kopia_repo_info", &[]).is_none());
    }

    #[tokio::test]
    async fn interrogation_complete() {
        let expected = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(b"kopia:s3cret")
        );
        let app = Router::new().fallback(get(move |headers: HeaderMap, uri: Uri| {
            let expected = expected.clone();
            async move {
                if headers.get("authorization").and_then(|v| v.to_str().ok())
                    != Some(expected.as_str())
                {
                    return (StatusCode::UNAUTHORIZED, "");
                }
                match uri.path() {
                    "/api/v1/repo/status" => (StatusCode::OK, STATUS),
                    "/api/v1/sources" => (StatusCode::OK, SOURCES_JSON),
                    _ => (StatusCode::NOT_FOUND, ""),
                }
            }
        }));
        let address = serve(app).await;
        let collector = KopiaCollector::new();
        let mut target = cible("kopia", &format!("http://{address}"), &[]);
        target.credential = Credential::UsernamePassword {
            username: "kopia".into(),
            password: "s3cret".into(),
        };
        let s = collector.probe(&target).await.unwrap();
        assert_eq!(value(&s, "kopia_sources", &[]), 4.0);

        target.credential = Credential::UsernamePassword {
            username: "kopia".into(),
            password: "wrong".into(),
        };
        let error = collector.probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Auth(_)), "{error}");
        assert!(!error.means_down());

        target.credential = Credential::ApiToken { token: "x".into() };
        let error = collector.probe(&target).await.unwrap_err();
        assert!(matches!(error, ProbeError::Config(_)), "{error}");
    }

    #[tokio::test]
    async fn un_serveur_injoignable_est_en_panne() {
        let collector = KopiaCollector::new();
        let target = cible("kopia", "http://127.0.0.1:1", &[]);
        let error = collector.probe(&target).await.unwrap_err();
        assert!(error.means_down(), "{error}");
    }
}
